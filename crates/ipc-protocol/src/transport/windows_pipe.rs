//! Windows named pipe taşıması (ADR-0007, Önerildi).
//!
//! DURUM: GitHub Actions `windows-2022` üzerinde test edildi (P1.A.1);
//! gerçek Windows 11 Home/Pro kabulü bekleniyor.
//!
//! Güvenlik özellikleri:
//! - Açık DACL (SDDL): SYSTEM, Administrators ve pipe sahibi tam yetki;
//!   Interactive Users yalnız okuma/yazma — `FILE_CREATE_PIPE_INSTANCE`
//!   (= `FILE_APPEND_DATA`) verilmez, böylece etkileşimli kullanıcı ikinci bir
//!   sunucu örneği açamaz.
//! - `PIPE_REJECT_REMOTE_CLIENTS`: ağ üzerinden bağlantı yok.
//! - `FILE_FLAG_FIRST_PIPE_INSTANCE`: ad önceden başkası tarafından alınmışsa
//!   (pipe squatting) agent başlamaz.
//! - İstemci `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION` ile bağlanır:
//!   sahte bir sunucu helper kullanıcısını taklit (impersonate) edemez.
//! - Eş kimliği: `GetNamedPipeClientProcessId`, `GetNamedPipeClientSessionId`,
//!   `QueryFullProcessImageNameW` — mesaj alanlarına güvenilmez.
//! - LocalService agent'ın helper sürecini açabilmesi için helper kendi süreç
//!   DACL'ine yalnız sorgu hakkı ekler: [`grant_agent_process_query`] (BUG-003).
//! - Zaman aşımı: overlapped G/Ç + `WaitForSingleObject` + `CancelIoEx`.

use super::PeerIdentity;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_BROKEN_PIPE, ERROR_FILE_NOT_FOUND,
    ERROR_IO_PENDING, ERROR_NO_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
    ERROR_PIPE_NOT_CONNECTED, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW, GetSecurityInfo,
    SetEntriesInAclW, SetSecurityInfo, EXPLICIT_ACCESS_W, GRANT_ACCESS, NO_MULTIPLE_TRUSTEE,
    SDDL_REVISION_1, SE_KERNEL_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, NO_INHERITANCE, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED,
    FILE_GENERIC_READ, FILE_WRITE_DATA, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION,
    SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, GetNamedPipeClientSessionId,
    GetNamedPipeServerProcessId, WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, OpenProcess, QueryFullProcessImageNameW, ResetEvent,
    WaitForSingleObject, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

pub const PIPE_NAME: &str = r"\\.\pipe\kindora-agent-v1";

/// D:P korumalı DACL.
/// - SY, BA, OW: GENERIC_ALL
/// - IU (Interactive Users): 0x0012008B =
///   FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_EA | FILE_READ_ATTRIBUTES |
///   READ_CONTROL | SYNCHRONIZE  (FILE_CREATE_PIPE_INSTANCE yok)
pub const PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012008B;;;IU)";

/// `NT AUTHORITY\LocalService` — P1.A servisinin hesabı (ADR-0006).
pub const LOCAL_SERVICE_SID: &str = "S-1-5-19";

const BUFFER_BYTES: u32 = (crate::MAX_MESSAGE_BYTES as u32) + 4;
const MAX_INSTANCES: u32 = 8;

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

fn last_error() -> u32 {
    // SAFETY: yan etkisiz.
    unsafe { GetLastError() }
}

struct OwnedHandle(HANDLE);
// SAFETY: Windows çekirdek tanıtıcıları iş parçacıkları arasında taşınabilir.
unsafe impl Send for OwnedHandle {}
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            // SAFETY: tanıtıcının sahibi biziz ve yalnız bir kez kapatılır.
            unsafe { CloseHandle(self.0) };
        }
    }
}

fn new_event() -> io::Result<OwnedHandle> {
    // Manuel sıfırlamalı, başlangıçta sinyalsiz olay.
    // SAFETY: tüm işaretçi parametreleri null (varsayılan güvenlik, adsız).
    let h = unsafe { CreateEventW(null(), 1, 0, null()) };
    if h.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(OwnedHandle(h))
}

/// Overlapped modda açılmış, zaman aşımlı okuma/yazma yapan pipe ucu.
///
/// Sunucu ucu kapanırken `DisconnectNamedPipe` **çağrılmaz**: bu çağrı
/// istemcinin henüz okumadığı veriyi siler (ör. `UNAUTHORIZED` yanıtı).
/// Her bağlantı için yeni bir pipe örneği oluşturulduğundan örnek yeniden
/// kullanılmaz; tanıtıcıyı kapatmak yeterlidir. [Windows'ta doğrulanacak]
pub struct PipeStream {
    handle: OwnedHandle,
    event: OwnedHandle,
    timeout: Option<Duration>,
}

impl PipeStream {
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

    /// Sunucu sürecinin PID'i (istemci tarafında sunucuyu kaydetmek için).
    pub fn server_process_id(&self) -> Option<u32> {
        let mut pid = 0u32;
        // SAFETY: geçerli pipe tanıtıcısı ve çıktı işaretçisi.
        let ok = unsafe { GetNamedPipeServerProcessId(self.handle.0, &mut pid) };
        (ok != 0).then_some(pid)
    }

    fn overlapped_io(
        &mut self,
        op: impl FnOnce(HANDLE, *mut OVERLAPPED) -> i32,
    ) -> io::Result<usize> {
        // SAFETY: OVERLAPPED düz bir C yapısıdır; sıfırlanmış hali geçerlidir.
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        ov.hEvent = self.event.0;
        // SAFETY: geçerli olay tanıtıcısı.
        unsafe { ResetEvent(self.event.0) };

        let started = op(self.handle.0, &mut ov);
        if started == 0 {
            let err = last_error();
            if err != ERROR_IO_PENDING {
                return Err(map_error(err));
            }
            let wait_ms = match self.timeout {
                Some(d) => d.as_millis().min(u128::from(u32::MAX - 1)) as u32,
                None => u32::MAX, // INFINITE
            };
            // SAFETY: geçerli olay tanıtıcısı.
            let w = unsafe { WaitForSingleObject(self.event.0, wait_ms) };
            if w == WAIT_TIMEOUT {
                let mut n = 0u32;
                // `ov` yığında olduğu için iptal tamamlanana kadar beklenmelidir.
                // SAFETY: aynı tanıtıcı ve aynı OVERLAPPED.
                unsafe {
                    CancelIoEx(self.handle.0, &ov);
                    GetOverlappedResult(self.handle.0, &ov, &mut n, 1);
                }
                return Err(io::Error::from(io::ErrorKind::TimedOut));
            }
            if w != WAIT_OBJECT_0 {
                return Err(io::Error::last_os_error());
            }
        }
        let mut n = 0u32;
        // SAFETY: işlem tamamlandı; sonuç okunuyor.
        let ok = unsafe { GetOverlappedResult(self.handle.0, &ov, &mut n, 0) };
        if ok == 0 {
            return Err(map_error(last_error()));
        }
        Ok(n as usize)
    }
}

fn map_error(err: u32) -> io::Error {
    match err {
        ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_NO_DATA => {
            io::Error::from(io::ErrorKind::BrokenPipe)
        }
        other => io::Error::from_raw_os_error(other as i32),
    }
}

impl Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len().min(u32::MAX as usize) as u32;
        let ptr = buf.as_mut_ptr();
        match self.overlapped_io(|h, ov| unsafe { ReadFile(h, ptr, len, null_mut(), ov) }) {
            // Karşı taraf kapandı: Read sözleşmesindeki EOF.
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(0),
            other => other,
        }
    }
}

impl Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len().min(u32::MAX as usize) as u32;
        let ptr = buf.as_ptr();
        self.overlapped_io(|h, ov| unsafe { WriteFile(h, ptr, len, null_mut(), ov) })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// `LocalAlloc` ile ayrılmış bellek (SID, ACL, SDDL dizesi); LocalFree ile bırakılır.
struct LocalMem(*mut core::ffi::c_void);
impl Drop for LocalMem {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: Win32 API'sinin LocalAlloc ile döndürdüğü bellek.
            unsafe { LocalFree(self.0 as _) };
        }
    }
}

/// Güvenlik tanımlayıcısı (LocalFree ile serbest bırakılır).
struct SecurityDescriptor(PSECURITY_DESCRIPTOR);
unsafe impl Send for SecurityDescriptor {}
impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: ConvertStringSecurityDescriptorToSecurityDescriptorW ile ayrıldı.
        unsafe { LocalFree(self.0 as _) };
    }
}

pub struct PipeServer {
    name: Vec<u16>,
    sd: SecurityDescriptor,
    first_instance: bool,
}

impl PipeServer {
    /// Üretim pipe adı ([`PIPE_NAME`]) ile sunucu.
    pub fn new() -> io::Result<Self> {
        Self::with_name(PIPE_NAME)
    }

    /// Belirli bir adla sunucu (testlerde gerçek agent adıyla çakışmamak için).
    pub fn with_name(pipe_name: &str) -> io::Result<Self> {
        let sddl = wide(PIPE_SDDL);
        let mut sd: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: geçerli SDDL dizesi ve çıktı işaretçisi.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(PipeServer {
            name: wide(pipe_name),
            sd: SecurityDescriptor(sd),
            first_instance: true,
        })
    }

    fn create_instance(&mut self) -> io::Result<OwnedHandle> {
        let mut open_mode = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED;
        if self.first_instance {
            open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
        }
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.sd.0,
            bInheritHandle: 0,
        };
        // SAFETY: geçerli ad, güvenlik öznitelikleri ve sabitler.
        let h = unsafe {
            CreateNamedPipeW(
                self.name.as_ptr(),
                open_mode,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                MAX_INSTANCES,
                BUFFER_BYTES,
                BUFFER_BYTES,
                0,
                &sa,
            )
        };
        if h == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        self.first_instance = false;
        Ok(OwnedHandle(h))
    }

    /// Bir istemci bekler. `shutdown` ayarlanırsa `Ok(None)` döner.
    pub fn accept(
        &mut self,
        io_timeout: Duration,
        shutdown: &AtomicBool,
    ) -> io::Result<Option<(PipeStream, PeerIdentity)>> {
        let handle = self.create_instance()?;
        let event = new_event()?;
        // SAFETY: sıfırlanmış OVERLAPPED geçerlidir.
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        ov.hEvent = event.0;

        // SAFETY: geçerli tanıtıcı ve OVERLAPPED.
        let ok = unsafe { ConnectNamedPipe(handle.0, &mut ov) };
        if ok == 0 {
            match last_error() {
                ERROR_PIPE_CONNECTED => {}
                ERROR_IO_PENDING => loop {
                    // SAFETY: geçerli olay tanıtıcısı.
                    let w = unsafe { WaitForSingleObject(event.0, 250) };
                    if w == WAIT_OBJECT_0 {
                        let mut n = 0u32;
                        // SAFETY: tamamlanan işlem.
                        if unsafe { GetOverlappedResult(handle.0, &ov, &mut n, 0) } == 0 {
                            return Err(io::Error::last_os_error());
                        }
                        break;
                    }
                    if shutdown.load(Ordering::SeqCst) {
                        let mut n = 0u32;
                        // SAFETY: bekleyen bağlantı isteğini iptal edip tamamlanmasını bekler.
                        unsafe {
                            CancelIoEx(handle.0, &ov);
                            GetOverlappedResult(handle.0, &ov, &mut n, 1);
                        }
                        return Ok(None);
                    }
                },
                other => return Err(io::Error::from_raw_os_error(other as i32)),
            }
        }

        let peer = client_identity(handle.0);
        Ok(Some((
            PipeStream {
                handle,
                event: new_event()?,
                timeout: Some(io_timeout),
            },
            peer,
        )))
    }
}

fn client_identity(pipe: HANDLE) -> PeerIdentity {
    let mut diagnostics = Vec::new();
    let mut pid_out = 0u32;
    let mut session_out = 0u32;
    // SAFETY: geçerli pipe tanıtıcısı ve çıktı işaretçileri.
    let pid = if unsafe { GetNamedPipeClientProcessId(pipe, &mut pid_out) } != 0 {
        Some(pid_out)
    } else {
        diagnostics.push(format!("GetNamedPipeClientProcessId={}", last_error()));
        None
    };
    // SAFETY: aynı.
    let session_id = if unsafe { GetNamedPipeClientSessionId(pipe, &mut session_out) } != 0 {
        Some(session_out)
    } else {
        diagnostics.push(format!("GetNamedPipeClientSessionId={}", last_error()));
        None
    };
    let exe_path = match pid {
        Some(pid) => match process_image_path(pid) {
            Ok(p) => Some(p),
            Err(d) => {
                diagnostics.push(d);
                None
            }
        },
        None => None,
    };
    PeerIdentity {
        pid,
        uid: None,
        session_id,
        exe_path,
        diagnostics,
    }
}

/// İstemcinin tam exe yolu. LocalService agent, helper süreci
/// [`grant_agent_process_query`] ile izin vermemişse `OpenProcess=5` alır;
/// yol bilinmeyince politika reddeder (fail closed).
/// Hata durumunda `"<API>=<GetLastError>"` biçiminde teşhis döner.
fn process_image_path(pid: u32) -> Result<PathBuf, String> {
    // SAFETY: yalnız sorgu hakkı istenir.
    let proc = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if proc.is_null() {
        return Err(format!("OpenProcess={}", last_error()));
    }
    let proc = OwnedHandle(proc);
    let mut buf = vec![0u16; 32 * 1024];
    let mut len = buf.len() as u32;
    // SAFETY: tampon ve uzunluk tutarlı.
    let ok = unsafe {
        QueryFullProcessImageNameW(proc.0, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
    };
    if ok == 0 {
        return Err(format!("QueryFullProcessImageNameW={}", last_error()));
    }
    buf.truncate(len as usize);
    Ok(PathBuf::from(OsString::from_wide(&buf)))
}

/// Helper tarafı (BUG-003): kendi süreç DACL'ine LocalService için **yalnız**
/// `PROCESS_QUERY_LIMITED_INFORMATION` ekler.
///
/// Kullanıcı süreçlerinin varsayılan DACL'i yalnız kullanıcıya, SYSTEM'e ve
/// oturum SID'ine izin verir; LocalService agent bu yüzden helper'ın exe yolunu
/// okuyamaz. Bu izin bellek okuma/yazma, sonlandırma veya token erişimi vermez;
/// kimlik doğrulaması değişmez — exe yolu yine işletim sisteminden okunur ve
/// agent politikası (oturum, ad, dizin) uygulanır. İzni vermeyen istemci yine
/// reddedilir.
pub fn grant_agent_process_query() -> io::Result<()> {
    let sid_text = wide(LOCAL_SERVICE_SID);
    let mut sid: PSID = null_mut();
    // SAFETY: geçerli SID dizesi ve çıktı işaretçisi.
    if unsafe { ConvertStringSidToSidW(sid_text.as_ptr(), &mut sid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let sid = LocalMem(sid);

    // SAFETY: sözde tanıtıcı; kapatılmaz.
    let process = unsafe { GetCurrentProcess() };
    let mut dacl: *mut ACL = null_mut();
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: çıktı işaretçileri geçerli; DACL `sd` içinde yaşar.
    let err = unsafe {
        GetSecurityInfo(
            process,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut sd,
        )
    };
    if err != 0 {
        return Err(io::Error::from_raw_os_error(err as i32));
    }
    let _sd = LocalMem(sd);

    let access = EXPLICIT_ACCESS_W {
        grfAccessPermissions: PROCESS_QUERY_LIMITED_INFORMATION,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
            ptstrName: sid.0 as _,
        },
    };
    let mut new_dacl: *mut ACL = null_mut();
    // SAFETY: tek girişli dizi, mevcut DACL ve çıktı işaretçisi.
    let err = unsafe { SetEntriesInAclW(1, &access, dacl, &mut new_dacl) };
    if err != 0 {
        return Err(io::Error::from_raw_os_error(err as i32));
    }
    let new_dacl = LocalMem(new_dacl as _);
    // SAFETY: kendi sürecimiz; yalnız DACL değişir.
    let err = unsafe {
        SetSecurityInfo(
            process,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            new_dacl.0 as *const ACL,
            null(),
        )
    };
    if err != 0 {
        return Err(io::Error::from_raw_os_error(err as i32));
    }
    Ok(())
}

/// Geçerli sürecin DACL'i SDDL biçiminde (teşhis ve testler için).
pub fn current_process_dacl_sddl() -> io::Result<String> {
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: sözde tanıtıcı ve çıktı işaretçisi.
    let err = unsafe {
        GetSecurityInfo(
            GetCurrentProcess(),
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sd,
        )
    };
    if err != 0 {
        return Err(io::Error::from_raw_os_error(err as i32));
    }
    let sd = LocalMem(sd);
    let mut text: *mut u16 = null_mut();
    let mut len = 0u32;
    // SAFETY: geçerli güvenlik tanımlayıcısı ve çıktı işaretçileri.
    let ok = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            sd.0,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut text,
            &mut len,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let text = LocalMem(text as _);
    // SAFETY: API null ile biten `len` uzunluklu dize döndürdü.
    let units = unsafe { std::slice::from_raw_parts(text.0 as *const u16, len as usize) };
    let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
    Ok(String::from_utf16_lossy(&units[..end]))
}

/// Helper tarafı: üretim pipe'ına bağlanır; meşgulse `connect_timeout` süresince bekler.
pub fn connect(connect_timeout: Duration, io_timeout: Duration) -> io::Result<PipeStream> {
    connect_named(PIPE_NAME, connect_timeout, io_timeout)
}

/// Belirli bir pipe adına bağlanır (testler için).
pub fn connect_named(
    pipe_name: &str,
    connect_timeout: Duration,
    io_timeout: Duration,
) -> io::Result<PipeStream> {
    let name = wide(pipe_name);
    let deadline = Instant::now() + connect_timeout;
    loop {
        // SAFETY: geçerli ad; null güvenlik öznitelikleri; şablon yok.
        let h = unsafe {
            CreateFileW(
                name.as_ptr(),
                FILE_GENERIC_READ | FILE_WRITE_DATA,
                0,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                null_mut(),
            )
        };
        if h != INVALID_HANDLE_VALUE {
            return Ok(PipeStream {
                handle: OwnedHandle(h),
                event: new_event()?,
                timeout: Some(io_timeout),
            });
        }
        let err = last_error();
        let remaining = deadline.saturating_duration_since(Instant::now());
        match err {
            ERROR_PIPE_BUSY if !remaining.is_zero() => {
                // SAFETY: geçerli ad.
                unsafe { WaitNamedPipeW(name.as_ptr(), remaining.as_millis().max(1) as u32) };
            }
            ERROR_FILE_NOT_FOUND => return Err(io::Error::from(io::ErrorKind::NotFound)),
            other => return Err(io::Error::from_raw_os_error(other as i32)),
        }
    }
}
