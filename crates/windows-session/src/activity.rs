//! Kullanıcı oturumunda tek etkinlik sinyali: son klavye/fare girdisinden bu
//! yana geçen süre (P1.C, ADR-0014).
//!
//! - Windows: `GetLastInputInfo` (oturum geneli son girdi zamanı) ve
//!   `GetTickCount`. Hangi tuşa basıldığı, fare konumu, pencere veya uygulama
//!   bilgisi **okunmaz** ve gönderilmez.
//! - macOS (yalnız geliştirme): `CGEventSourceSecondsSinceLastEventType`.
//! - Diğer: ölçüm yok (`None` → agent `UNKNOWN` sayar).

/// Son girdiden bu yana geçen süre (ms). `None`: ölçülemedi.
pub fn idle_ms() -> Option<u64> {
    imp::idle_ms()
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    pub fn idle_ms() -> Option<u64> {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        // SAFETY: cbSize ayarlı yapı; API yalnız yazar.
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            return None;
        }
        // SAFETY: parametresiz. Her iki değer de 32 bit tick; 49,7 günde bir
        // sarar, bu yüzden fark sarmalı aritmetikle alınır.
        let now = unsafe { GetTickCount() };
        Some(u64::from(now.wrapping_sub(info.dwTime)))
    }
}

#[cfg(target_os = "macos")]
mod imp {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
    }
    /// kCGEventSourceStateHIDSystemState
    const HID_SYSTEM_STATE: i32 = 1;
    /// kCGAnyInputEventType
    const ANY_INPUT: u32 = u32::MAX;

    pub fn idle_ms() -> Option<u64> {
        // SAFETY: saf sorgu işlevi; işaretçi yok.
        let secs = unsafe { CGEventSourceSecondsSinceLastEventType(HID_SYSTEM_STATE, ANY_INPUT) };
        (secs.is_finite() && secs >= 0.0).then_some((secs * 1000.0) as u64)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    pub fn idle_ms() -> Option<u64> {
        None
    }
}
