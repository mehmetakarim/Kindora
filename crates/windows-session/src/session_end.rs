//! Windows oturumu kapanırken (oturum kapatma, yeniden başlatma, kapatma)
//! arka plan helper'ının temiz çıkışı (P1.D).
//!
//! `user32` kullanan konsol süreçlerine `CTRL_LOGOFF_EVENT` gelmez; Windows bu
//! süreçlere oturum sonunu gizli bir üst düzey pencereye gönderdiği
//! `WM_QUERYENDSESSION` / `WM_ENDSESSION` iletileriyle bildirir. Pencere
//! görünmez, odak almaz ve girdi okumaz; yalnız bu iki iletiyi işler.
//!
//! Helper'ın kalıcı durumu yoktur: oturum sonunda yapılacak tek iş bağlantıyı
//! kapatıp "temiz çıkış" iz dosyasını silmektir. Süre agent'ta tutulur.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    TranslateMessage, MSG, WM_ENDSESSION, WM_QUERYENDSESSION, WNDCLASSW,
};

pub const WINDOW_CLASS: &str = "KindoraSessionHelperEndSession";

struct Flags {
    shutdown: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
}

static FLAGS: OnceLock<Flags> = OnceLock::new();

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // Oturum kapanışını engellemez.
        WM_QUERYENDSESSION => 1,
        WM_ENDSESSION => {
            if wparam != 0 {
                if let Some(f) = FLAGS.get() {
                    crate::log_event("session_ending", &[("lparam", format!("{lparam:#x}"))]);
                    f.shutdown.store(true, Ordering::SeqCst);
                    // Ana döngünün bağlantıyı kapatıp iz dosyasını silmesi için
                    // kısa süre tanınır; Windows bu iletiden sonra süreci sonlandırır.
                    let end = Instant::now() + Duration::from_secs(2);
                    while !f.stopped.load(Ordering::SeqCst) && Instant::now() < end {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            }
            0
        }
        // SAFETY: Windows'un verdiği parametrelerle varsayılan işleyici.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Gizli pencereyi ayrı bir iş parçacığında açar. `shutdown`: oturum biterken
/// ayarlanır. `stopped`: ana döngü bitince çağıran ayarlar. Süreçte bir kez çağrılır.
pub fn watch(shutdown: Arc<AtomicBool>, stopped: Arc<AtomicBool>) -> std::io::Result<()> {
    if FLAGS.set(Flags { shutdown, stopped }).is_err() {
        return Err(std::io::Error::other("session end watcher already running"));
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("session-end".into())
        .spawn(move || {
            let class = wide(WINDOW_CLASS);
            // SAFETY: geçerli sınıf yapısı; dizeler bu iş parçacığı boyunca yaşar.
            let hwnd = unsafe {
                let instance = GetModuleHandleW(std::ptr::null());
                let wc = WNDCLASSW {
                    style: 0,
                    lpfnWndProc: Some(wndproc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: instance,
                    hIcon: std::ptr::null_mut(),
                    hCursor: std::ptr::null_mut(),
                    hbrBackground: std::ptr::null_mut(),
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class.as_ptr(),
                };
                if RegisterClassW(&wc) == 0 {
                    let _ = tx.send(Err(std::io::Error::last_os_error()));
                    return;
                }
                // Üst düzey, hiç gösterilmeyen pencere (WS_VISIBLE yok). İleti
                // yalnız pencereleri (HWND_MESSAGE) oturum sonu yayınını almaz.
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    class.as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null(),
                )
            };
            if hwnd.is_null() {
                let _ = tx.send(Err(std::io::Error::last_os_error()));
                return;
            }
            let _ = tx.send(Ok(()));
            // SAFETY: standart ileti döngüsü; MSG yalnız bu iş parçacığında.
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        })?;
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|_| std::io::Error::other("session end watcher did not start"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, SendMessageW};

    #[test]
    fn end_session_message_sets_shutdown_and_waits_for_stop() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        watch(shutdown.clone(), stopped.clone()).unwrap();
        let class = wide(WINDOW_CLASS);
        // SAFETY: sınıf adıyla bu süreçteki gizli pencere aranır.
        let hwnd = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
        assert!(!hwnd.is_null());
        // Kapanışı engellemez.
        // SAFETY: geçerli pencereye senkron ileti.
        assert_eq!(unsafe { SendMessageW(hwnd, WM_QUERYENDSESSION, 0, 0) }, 1);
        assert!(!shutdown.load(Ordering::SeqCst));
        let s2 = shutdown.clone();
        let st2 = stopped.clone();
        let main_loop = std::thread::spawn(move || {
            while !s2.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
            st2.store(true, Ordering::SeqCst);
        });
        let t = Instant::now();
        // SAFETY: geçerli pencereye senkron ileti (wParam=TRUE: oturum bitiyor).
        unsafe { SendMessageW(hwnd, WM_ENDSESSION, 1, 0) };
        assert!(shutdown.load(Ordering::SeqCst));
        assert!(stopped.load(Ordering::SeqCst), "ana döngü beklenmedi");
        assert!(t.elapsed() < Duration::from_secs(2));
        main_loop.join().unwrap();
    }
}
