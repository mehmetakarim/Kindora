//! Mola hatırlatma durumu.
//!
//! - Molaya kalan süre yalnız **aktif kullanımla** azalır.
//! - `first_warning` / `final_warning` kala uyarı, aralık dolunca mola zamanı
//!   (`BREAK_DUE`) ve `break_duration` boyunca geri sayım.
//! - Kullanıcı en az `break_duration` boyunca boşta/kilitli/uykuda kalırsa bu
//!   doğal bir mola sayılır ve sayaç sıfırlanır. `UNKNOWN` mola sayılmaz.
//! - Hiçbir aşama kilit, engelleme veya program kapatma yapmaz; yalnız
//!   bilgilendirme durumudur.

use crate::policy::Policy;
use ipc_protocol::{BreakPhase, UsageState};

/// "Mola tamamlandı" mesajının görünür kaldığı süre.
pub const COMPLETE_NOTICE_MS: u64 = 60_000;

/// Kalıcı mola durumu (SQLite `break_state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakState {
    pub active_since_break_ms: u64,
    pub phase: BreakPhase,
    /// `BREAK_DUE` başlangıcından bu yana geçen monoton süre.
    pub due_elapsed_ms: u64,
    /// `BREAK_COMPLETE` başlangıcından bu yana geçen süre.
    pub complete_elapsed_ms: u64,
    /// Mola sırasındaki aktif kullanım (yalnız kayıt; yaptırım yok).
    pub active_during_break_ms: u64,
    /// Kesintisiz boşta/kilitli/uyku süresi.
    pub inactive_run_ms: u64,
    /// Başlatılan mola sayacı; bildirim kimliklerinde kullanılır.
    pub seq: u64,
}

impl Default for BreakState {
    fn default() -> Self {
        BreakState {
            active_since_break_ms: 0,
            phase: BreakPhase::Normal,
            due_elapsed_ms: 0,
            complete_elapsed_ms: 0,
            active_during_break_ms: 0,
            inactive_run_ms: 0,
            seq: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakEvent {
    Started { seq: u64 },
    Completed { seq: u64, active_during_ms: u64 },
    NaturalBreak { after_active_ms: u64 },
}

impl BreakState {
    pub fn on_segment(
        &mut self,
        state: UsageState,
        ms: u64,
        policy: &Policy,
        out: &mut Vec<BreakEvent>,
    ) {
        match state {
            UsageState::Active => {
                self.inactive_run_ms = 0;
                if self.phase == BreakPhase::BreakDue {
                    self.active_during_break_ms += ms;
                } else {
                    self.active_since_break_ms += ms;
                }
            }
            UsageState::Idle | UsageState::SessionLocked | UsageState::Suspended => {
                self.inactive_run_ms += ms;
                let duration = u64::from(policy.break_duration_secs) * 1000;
                let waiting = matches!(
                    self.phase,
                    BreakPhase::Normal | BreakPhase::FirstWarning | BreakPhase::FinalWarning
                );
                if waiting && self.active_since_break_ms > 0 && self.inactive_run_ms >= duration {
                    out.push(BreakEvent::NaturalBreak {
                        after_active_ms: self.active_since_break_ms,
                    });
                    self.active_since_break_ms = 0;
                    self.phase = BreakPhase::Normal;
                }
            }
            UsageState::Unknown => {}
        }
    }

    /// Geçen monoton süreyle geri sayımları ilerletir ve aşamayı yeniden hesaplar.
    pub fn advance(&mut self, elapsed_ms: u64, policy: &Policy, out: &mut Vec<BreakEvent>) {
        let interval = u64::from(policy.break_interval_secs) * 1000;
        let duration = u64::from(policy.break_duration_secs) * 1000;
        match self.phase {
            BreakPhase::BreakDue => {
                self.due_elapsed_ms += elapsed_ms;
                if self.due_elapsed_ms >= duration {
                    out.push(BreakEvent::Completed {
                        seq: self.seq,
                        active_during_ms: self.active_during_break_ms,
                    });
                    self.phase = BreakPhase::BreakComplete;
                    self.complete_elapsed_ms = 0;
                    self.active_since_break_ms = 0;
                }
                return;
            }
            BreakPhase::BreakComplete => {
                self.complete_elapsed_ms += elapsed_ms;
                if self.complete_elapsed_ms >= COMPLETE_NOTICE_MS {
                    self.phase = BreakPhase::Normal;
                }
            }
            _ => {}
        }
        if self.active_since_break_ms >= interval {
            self.seq += 1;
            self.phase = BreakPhase::BreakDue;
            self.due_elapsed_ms = 0;
            self.active_during_break_ms = 0;
            out.push(BreakEvent::Started { seq: self.seq });
            return;
        }
        let remaining = interval - self.active_since_break_ms;
        let warning = if remaining <= u64::from(policy.final_warning_secs) * 1000 {
            Some(BreakPhase::FinalWarning)
        } else if remaining <= u64::from(policy.first_warning_secs) * 1000 {
            Some(BreakPhase::FirstWarning)
        } else {
            None
        };
        match (warning, self.phase) {
            (Some(w), _) => self.phase = w,
            (None, BreakPhase::BreakComplete) => {}
            (None, _) => self.phase = BreakPhase::Normal,
        }
    }

    /// Sonraki molaya kalan aktif süre (mola sırasında `None`).
    pub fn next_break_in_ms(&self, policy: &Policy) -> Option<u64> {
        (self.phase != BreakPhase::BreakDue).then(|| {
            (u64::from(policy.break_interval_secs) * 1000)
                .saturating_sub(self.active_since_break_ms)
        })
    }

    pub fn break_remaining_ms(&self, policy: &Policy) -> Option<u64> {
        (self.phase == BreakPhase::BreakDue).then(|| {
            (u64::from(policy.break_duration_secs) * 1000).saturating_sub(self.due_elapsed_ms)
        })
    }

    /// Agent yeniden başlarken: kapalı kalınan süre (duvar saatiyle, 0 ile
    /// kalan süre arasında sınırlanmış) mola geri sayımından düşülür.
    pub fn restore_after_downtime(mut self, downtime_ms: u64, policy: &Policy) -> Self {
        match self.phase {
            BreakPhase::BreakDue => {
                let duration = u64::from(policy.break_duration_secs) * 1000;
                self.due_elapsed_ms = (self.due_elapsed_ms + downtime_ms).min(duration);
            }
            BreakPhase::BreakComplete => {
                self.complete_elapsed_ms =
                    (self.complete_elapsed_ms + downtime_ms).min(COMPLETE_NOTICE_MS);
            }
            _ => {}
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::BUILTIN_DEFAULT;
    use ipc_protocol::UsageState::*;

    const MIN: u64 = 60_000;

    /// `minutes` dakika boyunca saniyede bir `state`.
    fn feed(
        b: &mut BreakState,
        p: &Policy,
        state: UsageState,
        minutes: u64,
        ev: &mut Vec<BreakEvent>,
    ) {
        for _ in 0..minutes * 60 {
            b.on_segment(state, 1000, p, ev);
            b.advance(1000, p, ev);
        }
    }

    #[test]
    fn warnings_then_break_then_completion() {
        let p = BUILTIN_DEFAULT; // 30 dk aralık, 5 dk mola, 5/1 dk uyarı
        let mut b = BreakState::default();
        let mut ev = vec![];
        feed(&mut b, &p, Active, 24, &mut ev);
        assert_eq!(b.phase, BreakPhase::Normal);
        assert_eq!(b.next_break_in_ms(&p), Some(6 * MIN));
        feed(&mut b, &p, Active, 1, &mut ev);
        assert_eq!(b.phase, BreakPhase::FirstWarning);
        feed(&mut b, &p, Active, 4, &mut ev);
        assert_eq!(b.phase, BreakPhase::FinalWarning);
        feed(&mut b, &p, Active, 1, &mut ev);
        assert_eq!(b.phase, BreakPhase::BreakDue);
        assert_eq!(ev, vec![BreakEvent::Started { seq: 1 }]);
        assert_eq!(b.break_remaining_ms(&p), Some(5 * MIN));
        assert_eq!(b.next_break_in_ms(&p), None);
        // Mola sırasında kullanıcı aktif kalırsa engellenmez; yalnız kaydedilir.
        feed(&mut b, &p, Active, 5, &mut ev);
        assert_eq!(b.phase, BreakPhase::BreakComplete);
        assert!(
            matches!(ev[1], BreakEvent::Completed { seq: 1, active_during_ms } if active_during_ms >= 4 * MIN)
        );
        assert_eq!(b.active_since_break_ms, 0);
        feed(&mut b, &p, Idle, 1, &mut ev);
        assert_eq!(b.phase, BreakPhase::Normal);
    }

    #[test]
    fn idle_does_not_advance_break_and_long_idle_is_a_natural_break() {
        let p = BUILTIN_DEFAULT;
        let mut b = BreakState::default();
        let mut ev = vec![];
        feed(&mut b, &p, Active, 26, &mut ev);
        assert_eq!(b.phase, BreakPhase::FirstWarning);
        feed(&mut b, &p, Idle, 4, &mut ev);
        assert_eq!(
            b.active_since_break_ms,
            26 * MIN,
            "boşta süre molayı yaklaştırmaz"
        );
        assert_eq!(b.phase, BreakPhase::FirstWarning);
        feed(&mut b, &p, SessionLocked, 1, &mut ev);
        assert_eq!(
            ev,
            vec![BreakEvent::NaturalBreak {
                after_active_ms: 26 * MIN
            }]
        );
        assert_eq!(b.active_since_break_ms, 0);
        assert_eq!(b.phase, BreakPhase::Normal);
    }

    #[test]
    fn unknown_neither_counts_nor_resets() {
        let p = BUILTIN_DEFAULT;
        let mut b = BreakState::default();
        let mut ev = vec![];
        feed(&mut b, &p, Active, 10, &mut ev);
        feed(&mut b, &p, Unknown, 30, &mut ev);
        assert_eq!(b.active_since_break_ms, 10 * MIN);
        assert!(ev.is_empty());
    }

    #[test]
    fn restore_after_downtime_is_bounded() {
        let p = BUILTIN_DEFAULT;
        let due = BreakState {
            phase: BreakPhase::BreakDue,
            due_elapsed_ms: MIN,
            ..Default::default()
        };
        assert_eq!(
            due.restore_after_downtime(2 * MIN, &p)
                .break_remaining_ms(&p),
            Some(2 * MIN)
        );
        assert_eq!(
            due.restore_after_downtime(u64::MAX / 4, &p)
                .break_remaining_ms(&p),
            Some(0)
        );
        let normal = BreakState {
            active_since_break_ms: 7 * MIN,
            ..Default::default()
        };
        assert_eq!(normal.restore_after_downtime(10 * MIN, &p), normal);
    }
}
