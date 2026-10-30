//! Arka plan etkinlik helper'ı için oturum başına tek örnek (P1.D, ADR-0015).
//!
//! - Windows: `Local\` ad alanında adlandırılmış mutex — her Windows oturumu
//!   kendi kopyasını görür; aynı oturumda ikinci `activity` süreci hemen çıkar.
//!   Mutex süreç sonlanınca (çökme dahil) işletim sistemi tarafından bırakılır.
//! - Unix (yalnız geliştirme): soket yolunun yanındaki kilit dosyasında `flock`.
//!
//! Kilit yetki **değildir**: agent raporları yine yalnız kendi tek raporlayıcı
//! kuralıyla kabul eder. Kilit yalnız gereksiz ikinci süreci önler.

/// Kilit tutulduğu sürece geçerli. Düşürülünce bırakılır.
pub struct InstanceGuard {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(unix)]
    _file: std::fs::File,
}

// SAFETY (Windows): mutex tanıtıcısı yalnız Drop'ta kapatılır; süreçler arası
// paylaşılan bir çekirdek nesnesidir, iş parçacığına bağlı değildir.
#[cfg(windows)]
unsafe impl Send for InstanceGuard {}

#[cfg(windows)]
pub const MUTEX_NAME: &str = r"Local\Kindora.SessionHelper.Activity";

/// `Ok(Some)`: bu süreç tek örnek. `Ok(None)`: aynı oturumda başka örnek çalışıyor.
#[cfg(windows)]
pub fn acquire(_cfg: &crate::HelperConfig) -> std::io::Result<Option<InstanceGuard>> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let name: Vec<u16> = MUTEX_NAME.encode_utf16().chain(Some(0)).collect();
    // SAFETY: güvenlik özniteliği yok (varsayılan: oluşturan kullanıcı), ad
    // sıfır sonlu UTF-16. Dönen tanıtıcı Drop'ta kapatılır.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: CreateMutexW'den hemen sonra; başka çağrı araya girmedi.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: geçerli tanıtıcı, yalnız burada kapatılır.
        unsafe { CloseHandle(handle) };
        return Ok(None);
    }
    Ok(Some(InstanceGuard { handle }))
}

#[cfg(windows)]
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::ReleaseMutex;
        // SAFETY: acquire() içinde bu iş parçacığı sahibi oldu; tanıtıcı geçerli.
        unsafe {
            ReleaseMutex(self.handle);
            CloseHandle(self.handle);
        }
    }
}

#[cfg(unix)]
pub fn lock_path(cfg: &crate::HelperConfig) -> std::path::PathBuf {
    let mut p = cfg.socket_path.clone().into_os_string();
    p.push(".activity.lock");
    p.into()
}

#[cfg(unix)]
pub fn acquire(cfg: &crate::HelperConfig) -> std::io::Result<Option<InstanceGuard>> {
    use std::os::unix::io::AsRawFd;
    let path = lock_path(cfg);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)?;
    // SAFETY: geçerli dosya tanımlayıcısı; engellemeyen özel kilit.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let e = std::io::Error::last_os_error();
        return if e.kind() == std::io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(e)
        };
    }
    Ok(Some(InstanceGuard { _file: file }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_is_refused_until_first_releases() {
        #[allow(unused_mut)]
        let mut cfg = crate::HelperConfig::default();
        #[cfg(unix)]
        {
            cfg.socket_path = std::env::temp_dir()
                .join(format!("kd-inst-{}", std::process::id()))
                .join("a.sock");
        }
        let first = acquire(&cfg).unwrap();
        assert!(first.is_some());
        // Unix: flock açık dosya başına; Windows: aynı adlı mutex zaten var.
        assert!(acquire(&cfg).unwrap().is_none());
        drop(first);
        assert!(acquire(&cfg).unwrap().is_some());
        #[cfg(unix)]
        let _ = std::fs::remove_dir_all(cfg.socket_path.parent().unwrap());
    }
}
