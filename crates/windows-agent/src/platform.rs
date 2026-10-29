//! Kullanım süresi için işletim sistemi sinyalleri (P1.C, ADR-0014).
//!
//! Windows (servis Session 0'da, LocalService):
//! - Oturum kilidi: etkin konsol oturumu `WTSGetActiveConsoleSessionId` +
//!   `WTSQuerySessionInformationW(WTSSessionInfoEx)` (`SessionFlags`,
//!   `SessionState`). Saniyede bir sorgulanır; olay kaçırılsa da durum düzelir.
//! - Uyku/uyanma: servis denetleyicisine gelen `SERVICE_CONTROL_POWEREVENT`
//!   ([`WindowsSignals::on_power_event`]). Konsol modunda olay yoktur; uzun
//!   ölçüm boşluğu uyku/duraklama sayılır.
//! - Kullanıcı girdisi Session 0'dan okunamaz; Session Helper kendi
//!   oturumunda `GetLastInputInfo` ile ölçüp `REPORT_ACTIVITY` gönderir.
//!   Yalnız **etkin konsol oturumundan** gelen rapor sayılır.
//!
//! Unix (yalnız geliştirme): kilit/uyku sinyali yoktur; helper raporu sayılır.

use ipc_protocol::UsageState;
use std::path::PathBuf;

pub trait Signals: Send + Sync + std::fmt::Debug {
    /// Sistem şu an uykuya geçiyor/uykuda (güç olayı).
    fn suspended(&self) -> bool;
    /// Son çağrıdan bu yana uyanma oldu mu (bir kez `true` döner).
    fn take_resumed(&self) -> bool;
    /// Kullanıcının olmadığı veya ölçülemeyen oturum durumu.
    /// `Ok(None)`: oturum açık ve kilitsiz; ölçüm helper raporuna bağlı.
    fn session_block(&self) -> Result<Option<UsageState>, &'static str>;
    /// Bu oturumdan gelen etkinlik raporu sayılabilir mi?
    fn accepts_report_from(&self, client_session: Option<u32>) -> bool;
}

/// Yerel takvim günü ve UTC farkı (dakika).
pub fn local_stamp() -> (screen_time::calendar::LocalDate, i32) {
    imp::local_stamp()
}

pub fn default_signals() -> std::sync::Arc<dyn Signals> {
    imp::default_signals()
}

/// Servis hesabının yerel uygulama verisi klasörü altındaki Kindora klasörü.
/// LocalService için: `C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora`
/// (varsayılan olarak yalnız SYSTEM, Administrators ve LocalService erişir).
pub fn default_data_dir(console: bool) -> Option<PathBuf> {
    imp::local_app_data().map(|d| {
        let d = d.join("Kindora");
        if console {
            d.join("dev-console")
        } else {
            d
        }
    })
}

/// Veri klasörünü hazırlar. Windows: yoksa korumalı DACL ile oluşturur
/// (SYSTEM, Administrators ve sahibi; Users/Interactive erişemez).
pub fn prepare_data_dir(dir: &std::path::Path) -> std::io::Result<()> {
    imp::prepare_data_dir(dir)
}

#[cfg(windows)]
mod imp {
    use super::Signals;
    use ipc_protocol::UsageState;
    use screen_time::calendar::LocalDate;
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use windows_sys::Win32::Foundation::{LocalFree, FILETIME, SYSTEMTIME};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_sys::Win32::Storage::FileSystem::CreateDirectoryW;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::System::RemoteDesktop::{
        WTSActive, WTSFreeMemory, WTSGetActiveConsoleSessionId, WTSQuerySessionInformationW,
        WTSSessionInfoEx, WTSINFOEXW, WTS_CURRENT_SERVER_HANDLE, WTS_SESSIONSTATE_LOCK,
        WTS_SESSIONSTATE_UNLOCK,
    };
    use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};
    use windows_sys::Win32::System::Time::SystemTimeToFileTime;
    use windows_sys::Win32::UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath};

    /// Korumalı DACL: SYSTEM, Administrators ve nesnenin sahibi (agent hesabı).
    const DATA_DIR_SDDL: &str = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;OW)";

    #[derive(Debug, Default)]
    pub struct WindowsSignals {
        suspended: AtomicBool,
        resumed: AtomicBool,
    }

    impl WindowsSignals {
        /// Servis denetleyicisinden: `true` uyku, `false` uyanma.
        pub fn on_power_event(&self, suspending: bool) {
            self.suspended.store(suspending, Ordering::SeqCst);
            if !suspending {
                self.resumed.store(true, Ordering::SeqCst);
            }
        }
    }

    fn console_session() -> Option<u32> {
        // SAFETY: parametresiz.
        let id = unsafe { WTSGetActiveConsoleSessionId() };
        (id != u32::MAX).then_some(id)
    }

    impl Signals for WindowsSignals {
        fn suspended(&self) -> bool {
            self.suspended.load(Ordering::SeqCst)
        }

        fn take_resumed(&self) -> bool {
            self.resumed.swap(false, Ordering::SeqCst)
        }

        fn session_block(&self) -> Result<Option<UsageState>, &'static str> {
            let Some(session) = console_session() else {
                return Err("NO_CONSOLE_SESSION");
            };
            let mut buf: *mut u16 = std::ptr::null_mut();
            let mut bytes = 0u32;
            // SAFETY: çıktı işaretçileri geçerli; tampon WTSFreeMemory ile bırakılır.
            let ok = unsafe {
                WTSQuerySessionInformationW(
                    WTS_CURRENT_SERVER_HANDLE,
                    session,
                    WTSSessionInfoEx,
                    &mut buf,
                    &mut bytes,
                )
            };
            if ok == 0 || buf.is_null() || (bytes as usize) < std::mem::size_of::<WTSINFOEXW>() {
                return Err("LOCK_STATE_QUERY_FAILED");
            }
            // SAFETY: API WTSINFOEXW döndürdü (boyut denetlendi); Level 1 okunur.
            let (level, state, flags) = unsafe {
                let info = &*(buf as *const WTSINFOEXW);
                let l1 = info.Data.WTSInfoExLevel1;
                (info.Level, l1.SessionState, l1.SessionFlags)
            };
            // SAFETY: WTS tarafından ayrılan tampon.
            unsafe { WTSFreeMemory(buf.cast()) };
            if level != 1 {
                return Err("LOCK_STATE_QUERY_FAILED");
            }
            if state != WTSActive {
                // Konsolda oturum açık değil / bağlantısı kesik (kullanıcı değişimi).
                return Ok(Some(UsageState::SessionLocked));
            }
            match flags as u32 {
                WTS_SESSIONSTATE_LOCK => Ok(Some(UsageState::SessionLocked)),
                WTS_SESSIONSTATE_UNLOCK => Ok(None),
                _ => Err("LOCK_STATE_UNKNOWN"),
            }
        }

        fn accepts_report_from(&self, client_session: Option<u32>) -> bool {
            client_session.is_some() && client_session == console_session()
        }
    }

    pub fn default_signals() -> Arc<dyn Signals> {
        Arc::new(WindowsSignals::default())
    }

    fn filetime(st: &SYSTEMTIME) -> Option<i64> {
        let mut ft = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        // SAFETY: geçerli giriş/çıkış işaretçileri.
        let ok = unsafe { SystemTimeToFileTime(st, &mut ft) };
        (ok != 0).then(|| (i64::from(ft.dwHighDateTime) << 32) | i64::from(ft.dwLowDateTime))
    }

    pub fn local_stamp() -> (LocalDate, i32) {
        // SAFETY: SYSTEMTIME düz C yapısı; API'ler yalnız yazar.
        let (local, utc) = unsafe {
            let mut l: SYSTEMTIME = std::mem::zeroed();
            let mut u: SYSTEMTIME = std::mem::zeroed();
            GetSystemTime(&mut u);
            GetLocalTime(&mut l);
            (l, u)
        };
        let offset_min = match (filetime(&local), filetime(&utc)) {
            // 100 ns birimleri → en yakın dakika.
            (Some(l), Some(u)) => ((l - u) as f64 / 600_000_000.0).round() as i32,
            _ => 0,
        };
        let date = LocalDate::new(
            i32::from(local.wYear),
            u32::from(local.wMonth),
            u32::from(local.wDay),
        )
        .unwrap_or_else(|| LocalDate::from_days(0));
        (date, offset_min)
    }

    pub fn local_app_data() -> Option<PathBuf> {
        let mut p: windows_sys::core::PWSTR = std::ptr::null_mut();
        // SAFETY: bilinen klasör kimliği; çıktı CoTaskMemFree ile bırakılır.
        let hr = unsafe {
            SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, std::ptr::null_mut(), &mut p)
        };
        if hr < 0 || p.is_null() {
            return std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        }
        // SAFETY: API sıfır sonlu geniş dize döndürdü.
        let path = unsafe {
            let len = (0..).take_while(|&i| *p.add(i) != 0).count();
            let s = OsString::from_wide(std::slice::from_raw_parts(p, len));
            CoTaskMemFree(p as *const core::ffi::c_void);
            s
        };
        Some(PathBuf::from(path))
    }

    pub fn prepare_data_dir(dir: &Path) -> std::io::Result<()> {
        if dir.is_dir() {
            return Ok(());
        }
        if let Some(parent) = dir.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let sddl: Vec<u16> = std::ffi::OsStr::new(DATA_DIR_SDDL)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: sıfır sonlu SDDL; tanımlayıcı LocalFree ile bırakılır.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };
        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: geçerli yol ve güvenlik öznitelikleri.
        let created = unsafe { CreateDirectoryW(wide.as_ptr(), &sa) };
        let err = std::io::Error::last_os_error();
        // SAFETY: Convert... tarafından ayrıldı.
        unsafe { LocalFree(sd) };
        if created == 0 && !dir.is_dir() {
            return Err(err);
        }
        Ok(())
    }
}

#[cfg(unix)]
mod imp {
    use super::Signals;
    use ipc_protocol::UsageState;
    use screen_time::calendar::LocalDate;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// Geliştirme: kilit/uyku sinyali yok.
    #[derive(Debug, Default)]
    pub struct DevSignals;

    impl Signals for DevSignals {
        fn suspended(&self) -> bool {
            false
        }
        fn take_resumed(&self) -> bool {
            false
        }
        fn session_block(&self) -> Result<Option<UsageState>, &'static str> {
            Ok(None)
        }
        fn accepts_report_from(&self, _client_session: Option<u32>) -> bool {
            true
        }
    }

    pub fn default_signals() -> Arc<dyn Signals> {
        Arc::new(DevSignals)
    }

    pub fn local_stamp() -> (LocalDate, i32) {
        // SAFETY: localtime_r yalnız verilen yapıya yazar.
        unsafe {
            let t = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            if libc::localtime_r(&t, &mut tm).is_null() {
                return (LocalDate::from_days(t as i64 / 86_400), 0);
            }
            let date = LocalDate::new(tm.tm_year + 1900, (tm.tm_mon + 1) as u32, tm.tm_mday as u32)
                .unwrap_or_else(|| LocalDate::from_days(t as i64 / 86_400));
            (date, (tm.tm_gmtoff / 60) as i32)
        }
    }

    pub fn local_app_data() -> Option<PathBuf> {
        None
    }

    pub fn prepare_data_dir(dir: &Path) -> std::io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(dir)?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
    }
}

#[cfg(windows)]
pub use imp::WindowsSignals;
