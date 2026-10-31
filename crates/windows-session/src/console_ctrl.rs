//! Arka plan helper'ının Windows konsol olayları (P1.E.1, OBS-011).
//!
//! `CTRL_CLOSE_EVENT` / `CTRL_LOGOFF_EVENT` / `CTRL_SHUTDOWN_EVENT` olaylarında
//! Windows, işleyici döndüğü anda süreci sonlandırır. `ctrlc` işleyicisi bu
//! olaylarda da hemen döndüğü için helper temiz çıkış izini silemiyordu
//! (CI'da konsol kapatılınca 2–8 ms'de sonlanma, iz kaldı). Bu işleyici
//! kapanma bayrağını ayarlar ve ana döngünün bitmesini **en çok**
//! [`CLOSE_WAIT`] kadar bekler; Windows kapanışını veya oturum kapatmayı
//! engellemez (süre dolunca yine döner). Ctrl+C / Ctrl+Break eskisi gibi hemen
//! döner.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::core::BOOL;
use windows_sys::Win32::System::Console::{
    SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT, CTRL_LOGOFF_EVENT,
    CTRL_SHUTDOWN_EVENT,
};

/// Kapanış olaylarında ana döngüyü bekleme üst sınırı.
pub const CLOSE_WAIT: Duration = Duration::from_secs(2);

struct Flags {
    shutdown: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
}

static FLAGS: OnceLock<Flags> = OnceLock::new();

fn source(ctrl: u32) -> Option<(&'static str, bool)> {
    match ctrl {
        CTRL_C_EVENT => Some(("ctrl_c", false)),
        CTRL_BREAK_EVENT => Some(("ctrl_break", false)),
        CTRL_CLOSE_EVENT => Some(("console_close", true)),
        CTRL_LOGOFF_EVENT => Some(("logoff", true)),
        CTRL_SHUTDOWN_EVENT => Some(("shutdown", true)),
        _ => None,
    }
}

unsafe extern "system" fn handler(ctrl: u32) -> BOOL {
    let (Some(f), Some((name, wait))) = (FLAGS.get(), source(ctrl)) else {
        return 0;
    };
    crate::log_event("shutdown_requested", &[("source", name.into())]);
    f.shutdown.store(true, Ordering::SeqCst);
    if wait {
        let end = Instant::now() + CLOSE_WAIT;
        while !f.stopped.load(Ordering::SeqCst) && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    1
}

/// İşleyiciyi kurar. `shutdown`: olayda ayarlanır. `stopped`: ana döngü
/// bitip iz silinince çağıran ayarlar. Süreçte bir kez çağrılır.
pub fn install(shutdown: Arc<AtomicBool>, stopped: Arc<AtomicBool>) -> std::io::Result<()> {
    if FLAGS.set(Flags { shutdown, stopped }).is_err() {
        return Err(std::io::Error::other("console handler already installed"));
    }
    // SAFETY: statik ömürlü işleyici; FLAGS önce ayarlandı.
    if unsafe { SetConsoleCtrlHandler(Some(handler), 1) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_waits_for_the_main_loop_but_never_longer_than_the_cap() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        install(shutdown.clone(), stopped.clone()).unwrap();

        // Ctrl+C: hemen döner (davranış değişmedi).
        let t = Instant::now();
        // SAFETY: işleyici doğrudan çağrılır; Windows'un çağırdığı biçimle aynı.
        assert_eq!(unsafe { handler(CTRL_C_EVENT) }, 1);
        assert!(shutdown.load(Ordering::SeqCst));
        assert!(t.elapsed() < Duration::from_millis(200));

        // Konsol kapanışı: ana döngü izi silene kadar bekler.
        let st = stopped.clone();
        let main_loop = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            st.store(true, Ordering::SeqCst);
        });
        let t = Instant::now();
        // SAFETY: aynı.
        assert_eq!(unsafe { handler(CTRL_CLOSE_EVENT) }, 1);
        let waited = t.elapsed();
        main_loop.join().unwrap();
        assert!(waited >= Duration::from_millis(140), "{waited:?}");
        assert!(waited < Duration::from_secs(1), "{waited:?}");

        // Ana döngü bitmezse kapanış yine en çok CLOSE_WAIT kadar gecikir.
        stopped.store(false, Ordering::SeqCst);
        let t = Instant::now();
        // SAFETY: aynı.
        assert_eq!(unsafe { handler(CTRL_SHUTDOWN_EVENT) }, 1);
        let waited = t.elapsed();
        assert!(
            waited >= CLOSE_WAIT && waited < CLOSE_WAIT + Duration::from_millis(500),
            "{waited:?}"
        );

        // Tanınmayan olay: varsayılan işleyiciye bırakılır.
        // SAFETY: aynı.
        assert_eq!(unsafe { handler(99) }, 0);
    }
}
