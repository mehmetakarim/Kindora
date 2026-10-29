//! Aktif kullanım ölçümü — "girdi sınırı" (input frontier) modeli.
//!
//! Tüm süreler **monoton** milisaniyedir (agent sürecinin başlangıcından
//! itibaren); duvar saati süre hesabına girmez.
//!
//! Kurallar:
//! 1. Engelleyen durum (`SESSION_LOCKED`, `SUSPENDED`, `UNKNOWN`) o aralığı
//!    kendi adıyla kaydeder; aktif sayılmaz.
//! 2. Gözlemde (helper taze rapor veriyor) **iki gözlenen girdi** arasındaki
//!    süre, boşta kalma eşiğinden kısaysa `ACTIVE`, değilse `IDLE` olur. Kilit
//!    açılışı, uyanma veya ölçüm başlangıcından ilk girdiye kadarki süre
//!    girdiyle başlamadığı için `IDLE` sayılır.
//! 3. Son girdiden sonra eşik dolunca, son girdiden itibaren tüm aralık
//!    **geriye dönük** `IDLE` sayılır (eşik süresi aktif kullanıma eklenmez).
//! 4. Son girdiden sonraki, henüz doğrulanmamış kuyruk bir engelleyen duruma
//!    geçişte `IDLE` sayılır (tahmin aktif kullanım olarak kaydedilmez).
//! 5. İki ölçüm arası `max_gap_ms`'den uzunsa (uyku, süreç duraklaması) aralık
//!    `SUSPENDED` (biliniyorsa) veya `UNKNOWN` olur.

use ipc_protocol::UsageState;

/// Ölçüm anındaki bağlam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctx {
    /// Helper taze rapor veriyor. `last_input_ms`: son girdinin monoton zamanı.
    Observing { last_input_ms: Option<u64> },
    /// Ölçülemeyen veya kullanıcı yok durumu (`ACTIVE`/`IDLE` dışında).
    Blocked(UsageState),
}

/// Monoton zamanda sınıflandırılmış bir aralık.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub state: UsageState,
    pub start_ms: u64,
    pub end_ms: u64,
}

impl Segment {
    pub fn len_ms(&self) -> u64 {
        self.end_ms - self.start_ms
    }
}

#[derive(Debug, Clone)]
pub struct Accountant {
    threshold_ms: u64,
    max_gap_ms: u64,
    /// Bu ana kadar her şey sınıflandırıldı.
    cursor: u64,
    last_tick: Option<u64>,
    /// Son sınıflandırma eşik aşımıyla IDLE yapıldı (kullanıcı uzakta).
    in_idle: bool,
    /// `cursor` gözlenen bir girdide mi duruyor (yoksa başlangıç/kilit/uyanma)?
    anchored: bool,
}

impl Accountant {
    pub fn new(threshold_ms: u64, max_gap_ms: u64) -> Self {
        Accountant {
            threshold_ms: threshold_ms.max(1),
            max_gap_ms,
            cursor: 0,
            last_tick: None,
            in_idle: false,
            anchored: false,
        }
    }

    pub fn set_threshold(&mut self, threshold_ms: u64) {
        self.threshold_ms = threshold_ms.max(1);
    }

    fn push(out: &mut Vec<Segment>, state: UsageState, start_ms: u64, end_ms: u64) {
        if end_ms > start_ms {
            out.push(Segment {
                state,
                start_ms,
                end_ms,
            });
        }
    }

    /// Doğrulanmamış kuyruğu (son girdiden sonrası) `IDLE` olarak kapatır.
    fn close_tail(&mut self, until: u64, out: &mut Vec<Segment>) {
        if until > self.cursor {
            Self::push(out, UsageState::Idle, self.cursor, until);
            self.cursor = until;
        }
        self.anchored = false;
    }

    /// `now` anındaki bağlamla ilerler; sınıflandırılan aralıkları `out`'a ekler.
    /// `gap_state`: aralık çok uzunsa kullanılacak durum (`SUSPENDED`/`UNKNOWN`).
    pub fn tick(&mut self, now: u64, ctx: Ctx, gap_state: UsageState, out: &mut Vec<Segment>) {
        let Some(last) = self.last_tick else {
            self.last_tick = Some(now);
            self.cursor = now;
            return;
        };
        if now <= last {
            return;
        }
        self.last_tick = Some(now);

        if now - last > self.max_gap_ms {
            self.close_tail(last, out);
            Self::push(out, gap_state, last, now);
            self.cursor = now;
            self.in_idle = false;
            self.anchored = false;
            return;
        }

        match ctx {
            Ctx::Blocked(state) => {
                self.close_tail(last, out);
                Self::push(out, state, self.cursor, now);
                self.cursor = now;
                self.in_idle = false;
                self.anchored = false;
            }
            Ctx::Observing { last_input_ms } => {
                if let Some(input) = last_input_ms.map(|f| f.min(now)) {
                    if input > self.cursor {
                        let confirmed = self.anchored
                            && !self.in_idle
                            && input - self.cursor < self.threshold_ms;
                        let state = if confirmed {
                            UsageState::Active
                        } else {
                            UsageState::Idle
                        };
                        Self::push(out, state, self.cursor, input);
                        self.cursor = input;
                        self.in_idle = false;
                        self.anchored = true;
                    }
                }
                if self.in_idle || now - self.cursor >= self.threshold_ms {
                    Self::push(out, UsageState::Idle, self.cursor, now);
                    self.cursor = now;
                    self.in_idle = true;
                    self.anchored = false;
                }
            }
        }
    }

    /// Ekranda gösterilecek anlık durum (henüz kaydedilmemiş kuyruğu tahmin
    /// olarak değerlendirir; sayıma etkisi yoktur).
    pub fn current_state(&self, now: u64, ctx: Ctx) -> UsageState {
        match ctx {
            Ctx::Blocked(s) => s,
            Ctx::Observing { .. }
                if self.in_idle || now.saturating_sub(self.cursor) >= self.threshold_ms =>
            {
                UsageState::Idle
            }
            Ctx::Observing { .. } => UsageState::Active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_protocol::UsageState::*;

    const S: u64 = 1000;
    const TH: u64 = 300 * S;

    /// Saniyede bir ölçüm yapan benzetim. `input(t)`: t anında görülen son girdi.
    fn run(
        acct: &mut Accountant,
        from: u64,
        to: u64,
        ctx: impl Fn(u64) -> Ctx,
        out: &mut Vec<Segment>,
    ) {
        let mut t = from;
        while t <= to {
            acct.tick(t, ctx(t), Unknown, out);
            t += S;
        }
    }

    fn total(out: &[Segment], st: UsageState) -> u64 {
        out.iter()
            .filter(|s| s.state == st)
            .map(Segment::len_ms)
            .sum()
    }

    /// Kullanıcı her 2 sn'de bir girdi yapıyor.
    fn busy(t: u64) -> Ctx {
        Ctx::Observing {
            last_input_ms: Some(t - t % (2 * S)),
        }
    }

    #[test]
    fn active_use_counts_only_up_to_the_last_seen_input() {
        let mut a = Accountant::new(TH, 5 * S);
        let mut out = vec![];
        run(&mut a, 0, 600 * S, busy, &mut out);
        // İlk gözlenen girdiye (2. sn) kadar olan kısım doğrulanmadığı için IDLE.
        assert_eq!(total(&out, Active), 598 * S);
        assert_eq!(total(&out, Idle), 2 * S);
        assert_eq!(a.current_state(600 * S, busy(600 * S)), Active);
    }

    #[test]
    fn idle_period_is_never_counted_and_threshold_is_retroactive() {
        // 0–100 sn aktif; sonra 20 dk hiç girdi yok; sonra tekrar aktif.
        let mut a = Accountant::new(TH, 5 * S);
        let mut out = vec![];
        let last_input = |t: u64| -> u64 {
            if t <= 100 * S {
                t - t % (2 * S)
            } else if t < 1300 * S {
                100 * S
            } else {
                t - t % (2 * S)
            }
        };
        let ctx = |t| Ctx::Observing {
            last_input_ms: Some(last_input(t)),
        };
        run(&mut a, 0, 1200 * S, ctx, &mut out);
        // Eşik dolana kadar (100+300 sn) hiçbir şey aktif sayılmadı; eşik
        // dolunca 100. saniyeden itibaren tamamı IDLE oldu.
        assert_eq!(total(&out, Active), 98 * S);
        assert_eq!(total(&out, Idle), 2 * S + 1100 * S);
        assert_eq!(a.current_state(1200 * S, ctx(1200 * S)), Idle);

        // 3. senaryo: kullanıcı geri döndü (1300. sn).
        run(&mut a, 1201 * S, 1500 * S, ctx, &mut out);
        assert_eq!(total(&out, Active), 98 * S + 200 * S);
        assert_eq!(total(&out, Idle), 2 * S + 1200 * S);
    }

    #[test]
    fn short_pause_below_threshold_counts_as_active_once_confirmed() {
        let mut a = Accountant::new(TH, 5 * S);
        let mut out = vec![];
        // 10. sn ve 200. sn'de girdi; arası okuma (eşikten kısa).
        let ctx = |t: u64| Ctx::Observing {
            last_input_ms: match t {
                _ if t < 10 * S => None,
                _ if t < 200 * S => Some(10 * S),
                _ => Some(200 * S),
            },
        };
        run(&mut a, 0, 199 * S, ctx, &mut out);
        assert_eq!(total(&out, Active), 0, "doğrulanmadan sayılmaz");
        run(&mut a, 200 * S, 200 * S, ctx, &mut out);
        assert_eq!(total(&out, Active), 190 * S);
    }

    #[test]
    fn lock_and_unconfirmed_tail() {
        let mut a = Accountant::new(TH, 5 * S);
        let mut out = vec![];
        // 0–60 sn aktif (son girdi 60), 90. sn'de kilit, 300. sn'de açılış.
        let ctx = |t: u64| {
            if (90 * S..300 * S).contains(&t) {
                Ctx::Blocked(SessionLocked)
            } else {
                Ctx::Observing {
                    last_input_ms: Some(if t <= 60 * S {
                        t
                    } else if t < 300 * S {
                        60 * S
                    } else {
                        t
                    }),
                }
            }
        };
        run(&mut a, 0, 400 * S, ctx, &mut out);
        // 1–60 sn ve kilit açıldıktan sonra 300–400 sn.
        assert_eq!(total(&out, Active), 59 * S + 100 * S);
        // 0–1 başlangıç, 60–89 doğrulanmamış kuyruk, 299–300 açılış → IDLE.
        assert_eq!(total(&out, Idle), S + 29 * S + S);
        assert_eq!(total(&out, SessionLocked), 210 * S);
    }

    #[test]
    fn sleep_gap_is_suspended_not_active() {
        let mut a = Accountant::new(TH, 5 * S);
        let mut out = vec![];
        run(&mut a, 0, 30 * S, busy, &mut out);
        // Süreç 2 saat çalışmadı (uyku); uyanınca helper idle'ı büyük bildirir.
        let wake = 30 * S + 7200 * S;
        a.tick(
            wake,
            Ctx::Observing {
                last_input_ms: Some(30 * S),
            },
            Suspended,
            &mut out,
        );
        assert_eq!(total(&out, Suspended), 7200 * S);
        assert_eq!(total(&out, Active), 28 * S);
        // Uyandıktan sonra eski girdi zamanı yeniden sayılmaz.
        run(
            &mut a,
            wake + S,
            wake + 20 * S,
            |_| Ctx::Observing {
                last_input_ms: Some(30 * S),
            },
            &mut out,
        );
        assert_eq!(total(&out, Active), 28 * S);
        // Kullanıcı tekrar aktif olunca sayım sürer (ilk girdiden itibaren).
        run(&mut a, wake + 21 * S, wake + 40 * S, busy, &mut out);
        assert!(total(&out, Active) > 28 * S);
        assert_eq!(total(&out, Active), 28 * S + 20 * S);
    }

    #[test]
    fn unknown_measurement_is_not_active() {
        let mut a = Accountant::new(TH, 5 * S);
        let mut out = vec![];
        run(&mut a, 0, 100 * S, |_| Ctx::Blocked(Unknown), &mut out);
        assert_eq!(total(&out, Active), 0);
        assert_eq!(total(&out, Unknown), 100 * S);
    }

    #[test]
    fn segments_never_overlap_and_cover_time() {
        let mut a = Accountant::new(60 * S, 5 * S);
        let mut out = vec![];
        let ctx = |t: u64| match (t / S) % 400 {
            0..=99 => busy(t),
            100..=149 => Ctx::Blocked(SessionLocked),
            150..=299 => Ctx::Observing {
                last_input_ms: Some(0),
            },
            _ => busy(t),
        };
        run(&mut a, 0, 2000 * S, ctx, &mut out);
        for w in out.windows(2) {
            assert!(w[0].end_ms <= w[1].start_ms, "{w:?}");
        }
        let covered: u64 = out.iter().map(Segment::len_ms).sum();
        assert!(covered <= 2000 * S);
        assert!(
            covered >= 2000 * S - 60 * S,
            "en fazla bir eşik kadar bekleyen kuyruk"
        );
    }
}
