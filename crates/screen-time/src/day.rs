//! Gün değişimi ve saat değişikliği tespiti.
//!
//! - Süreler monoton saatle ölçülür; duvar saati yalnız **hangi güne**
//!   yazılacağını belirler.
//! - Yerel tarih ileri giderse yeni gün başlar (gece yarısı, uyku sonrası veya
//!   saatin ileri alınması). İleri alma kayda geçer ve gün `clock_adjusted`
//!   olarak işaretlenir.
//! - Yerel tarih en fazla 1 gün geri giderse (saat dilimi değişikliği, küçük
//!   düzeltme) mevcut gün korunur: bugünün kullanımı geçmiş bir güne yazılarak
//!   gizlenemez. Daha büyük geri alma bir saat düzeltmesi kabul edilir ve o
//!   güne geçilir (ileri yanlış ayarlanmış bir saat sonsuza dek takılı kalmasın).
//! - Monoton ve duvar saati farkı [`CLOCK_JUMP_TOLERANCE_MS`]'i aşarsa (uyku
//!   dışında) saat değişikliği olarak kaydedilir.

use crate::calendar::LocalDate;

pub const CLOCK_JUMP_TOLERANCE_MS: i64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockEvent {
    ClockJump {
        skew_ms: i64,
    },
    TimezoneChanged {
        from_min: i32,
        to_min: i32,
    },
    DayRollover {
        from: LocalDate,
        to: LocalDate,
    },
    /// Yerel tarih mevcut günün gerisinde; mevcut gün korunuyor.
    ClockBehind {
        local: LocalDate,
        kept: LocalDate,
    },
    DayCorrected {
        from: LocalDate,
        to: LocalDate,
    },
}

#[derive(Debug, Clone, Copy)]
struct Last {
    wall_ms: u64,
    mono_ms: u64,
    offset_min: i32,
}

#[derive(Debug, Clone)]
pub struct DayTracker {
    key: LocalDate,
    last: Option<Last>,
    behind: bool,
    adjusted: bool,
}

impl DayTracker {
    pub fn new(key: LocalDate) -> Self {
        DayTracker {
            key,
            last: None,
            behind: false,
            adjusted: false,
        }
    }

    pub fn key(&self) -> LocalDate {
        self.key
    }

    /// Bugün saat veya saat dilimi değişikliği görüldü mü?
    pub fn adjusted_today(&self) -> bool {
        self.adjusted
    }

    pub fn mark_adjusted(&mut self) {
        self.adjusted = true;
    }

    /// `suspended`: son gözlemden bu yana uyku/duraklama oldu (duvar saati
    /// monoton saatten hızlı ilerlemiş olabilir; saat değişikliği sayılmaz).
    /// Dönüş: olaylar ve gün değiştiyse yeni gün.
    pub fn observe(
        &mut self,
        local: LocalDate,
        wall_ms: u64,
        mono_ms: u64,
        offset_min: i32,
        suspended: bool,
    ) -> (Vec<ClockEvent>, Option<LocalDate>) {
        let mut events = vec![];
        let mut jumped = false;
        if let Some(last) = self.last {
            if !suspended {
                let dw = wall_ms as i64 - last.wall_ms as i64;
                let dm = mono_ms as i64 - last.mono_ms as i64;
                let skew = dw - dm;
                if skew.abs() > CLOCK_JUMP_TOLERANCE_MS {
                    events.push(ClockEvent::ClockJump { skew_ms: skew });
                    jumped = true;
                }
            }
            if offset_min != last.offset_min {
                events.push(ClockEvent::TimezoneChanged {
                    from_min: last.offset_min,
                    to_min: offset_min,
                });
                jumped = true;
            }
        }
        self.last = Some(Last {
            wall_ms,
            mono_ms,
            offset_min,
        });
        if jumped {
            self.adjusted = true;
        }

        let mut changed = None;
        let diff = self.key.days_until(local);
        if diff > 0 {
            events.push(ClockEvent::DayRollover {
                from: self.key,
                to: local,
            });
            self.key = local;
            self.adjusted = jumped;
            self.behind = false;
            changed = Some(local);
        } else if diff < -1 {
            events.push(ClockEvent::DayCorrected {
                from: self.key,
                to: local,
            });
            self.key = local;
            self.adjusted = true;
            self.behind = false;
            changed = Some(local);
        } else if diff == -1 {
            if !self.behind {
                events.push(ClockEvent::ClockBehind {
                    local,
                    kept: self.key,
                });
                self.adjusted = true;
            }
            self.behind = true;
        } else {
            self.behind = false;
        }
        (events, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> LocalDate {
        LocalDate::parse(s).unwrap()
    }

    #[test]
    fn midnight_rollover_without_clock_change() {
        let mut t = DayTracker::new(d("2026-10-29"));
        assert!(t
            .observe(d("2026-10-29"), 1_000_000, 0, 180, false)
            .1
            .is_none());
        let (ev, ch) = t.observe(d("2026-10-30"), 1_001_000, 1000, 180, false);
        assert_eq!(ch, Some(d("2026-10-30")));
        assert_eq!(
            ev,
            vec![ClockEvent::DayRollover {
                from: d("2026-10-29"),
                to: d("2026-10-30")
            }]
        );
        assert!(!t.adjusted_today());
    }

    #[test]
    fn forward_clock_change_starts_new_day_but_is_flagged() {
        let mut t = DayTracker::new(d("2026-10-29"));
        t.observe(d("2026-10-29"), 1_000_000, 0, 180, false);
        // Monoton 1 sn ilerledi, duvar saati 1 gün ileri alındı.
        let (ev, ch) = t.observe(d("2026-10-30"), 1_000_000 + 86_400_000, 1000, 180, false);
        assert_eq!(ch, Some(d("2026-10-30")));
        assert!(matches!(ev[0], ClockEvent::ClockJump { skew_ms } if skew_ms > 86_000_000));
        assert!(t.adjusted_today());
    }

    #[test]
    fn backward_one_day_keeps_current_day() {
        let mut t = DayTracker::new(d("2026-10-29"));
        t.observe(d("2026-10-29"), 2_000_000_000, 0, 180, false);
        let (ev, ch) = t.observe(d("2026-10-28"), 2_000_000_000 - 3_600_000, 1000, 180, false);
        assert!(ch.is_none());
        assert_eq!(t.key(), d("2026-10-29"));
        assert!(ev.iter().any(|e| matches!(e, ClockEvent::ClockJump { .. })));
        assert!(ev
            .iter()
            .any(|e| matches!(e, ClockEvent::ClockBehind { .. })));
        // Tekrar eden gözlemde yeni olay üretilmez.
        let (ev, _) = t.observe(d("2026-10-28"), 2_000_000_000 - 3_599_000, 2000, 180, false);
        assert!(ev.is_empty());
        // Gerçek tarih yetişince aynı gün sürer, sonra normal ilerler.
        let (_, ch) = t.observe(d("2026-10-29"), 2_000_000_000 + 1000, 3_602_000, 180, false);
        assert!(ch.is_none());
    }

    #[test]
    fn large_backward_correction_is_accepted() {
        let mut t = DayTracker::new(d("2030-01-01"));
        t.observe(d("2030-01-01"), 1_900_000_000_000, 0, 180, false);
        let (ev, ch) = t.observe(d("2026-10-29"), 1_761_700_000_000, 1000, 180, false);
        assert_eq!(ch, Some(d("2026-10-29")));
        assert!(ev
            .iter()
            .any(|e| matches!(e, ClockEvent::DayCorrected { .. })));
    }

    #[test]
    fn timezone_change_is_recorded_and_sleep_is_not_a_clock_jump() {
        let mut t = DayTracker::new(d("2026-10-29"));
        t.observe(d("2026-10-29"), 1_000_000, 0, 180, false);
        let (ev, _) = t.observe(d("2026-10-29"), 1_001_000, 1000, 60, false);
        assert_eq!(
            ev,
            vec![ClockEvent::TimezoneChanged {
                from_min: 180,
                to_min: 60
            }]
        );
        // Uyku: monoton 1 sn, duvar 2 saat — uyku işaretliyse saat değişikliği değil.
        let mut t = DayTracker::new(d("2026-10-29"));
        t.observe(d("2026-10-29"), 1_000_000, 0, 180, false);
        let (ev, _) = t.observe(d("2026-10-29"), 1_000_000 + 7_200_000, 1000, 180, true);
        assert!(ev.is_empty());
        assert!(!t.adjusted_today());
    }
}
