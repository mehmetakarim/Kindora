//! Kullanım süresi motoru: ölçüm, gün, mola ve politika parçalarını birleştirir.
//!
//! Motor saf bir durum makinesidir: platform katmanı saniyede bir
//! [`TickInput`] verir, motor sınıflandırılmış parçaları ([`Piece`]) ve yaşam
//! döngüsü olaylarını döndürür; bunlar [`crate::store`] ile kalıcılaşır.

use crate::accounting::{Accountant, Ctx, Segment};
use crate::breaks::{BreakEvent, BreakState};
use crate::calendar::LocalDate;
use crate::day::{ClockEvent, DayTracker};
use crate::policy::{Policy, Resolution};
use ipc_protocol::{
    BreakPhase, DayUsage, MeasurementInfo, MeasurementSource, Notice, NoticeKind, PolicyInfo,
    ScreenTimeStatus, StorageInfo, UsageState,
};

/// İki ölçüm arası bundan uzunsa aralık uyku/duraklama sayılır.
pub const MAX_TICK_GAP_MS: u64 = 5_000;

/// Gün toplamları (ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DayUsageMs {
    pub active_ms: u64,
    pub idle_ms: u64,
    pub locked_ms: u64,
    pub suspended_ms: u64,
    pub unknown_ms: u64,
}

impl DayUsageMs {
    pub fn add(&mut self, state: UsageState, ms: u64) {
        let slot = match state {
            UsageState::Active => &mut self.active_ms,
            UsageState::Idle => &mut self.idle_ms,
            UsageState::SessionLocked => &mut self.locked_ms,
            UsageState::Suspended => &mut self.suspended_ms,
            UsageState::Unknown => &mut self.unknown_ms,
        };
        *slot = slot.saturating_add(ms);
    }

    pub fn to_secs(self) -> DayUsage {
        DayUsage {
            active_secs: self.active_ms / 1000,
            idle_secs: self.idle_ms / 1000,
            locked_secs: self.locked_ms / 1000,
            suspended_secs: self.suspended_ms / 1000,
            unknown_secs: self.unknown_ms / 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TickInput {
    pub mono_ms: u64,
    pub wall_ms: u64,
    pub local: LocalDate,
    pub utc_offset_min: i32,
    pub ctx: Ctx,
    /// Uzun aralığın durumu: güç olayı uyku bildirdiyse `SUSPENDED`, aksi `UNKNOWN`.
    pub gap_state: UsageState,
    /// Son ölçümden bu yana uyku/uyanma olayı görüldü.
    pub suspended_since_last: bool,
    pub measurement: MeasurementInfo,
}

/// Kalıcılaşacak sınıflandırılmış süre parçası.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub day: LocalDate,
    pub state: UsageState,
    pub start_wall_ms: u64,
    pub end_wall_ms: u64,
    pub ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleEvent {
    pub wall_ms: u64,
    pub kind: &'static str,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct TickOutput {
    pub pieces: Vec<Piece>,
    pub events: Vec<LifecycleEvent>,
    /// Gün değişti (yeni gün anahtarı).
    pub day_changed: Option<LocalDate>,
}

pub struct Engine {
    policy: Policy,
    policy_info: PolicyInfo,
    acct: Accountant,
    day: DayTracker,
    brk: BreakState,
    today: DayUsageMs,
    /// Gün değişiminin monoton anı; öncesindeki parçalar önceki güne yazılır.
    boundary: Option<(u64, LocalDate)>,
    last_mono: Option<u64>,
    state: UsageState,
    measurement: MeasurementInfo,
    storage: StorageInfo,
    limit_announced: bool,
}

fn blocking(s: UsageState) -> bool {
    matches!(
        s,
        UsageState::SessionLocked | UsageState::Suspended | UsageState::Unknown
    )
}

impl Engine {
    pub fn new(
        policy: &Resolution,
        storage: StorageInfo,
        day: LocalDate,
        today: DayUsageMs,
        brk: BreakState,
    ) -> Self {
        let threshold = u64::from(policy.active.idle_threshold_secs) * 1000;
        let limit_ms = u64::from(policy.active.daily_limit_secs) * 1000;
        Engine {
            policy: policy.active,
            policy_info: policy.info.clone(),
            acct: Accountant::new(threshold, MAX_TICK_GAP_MS),
            day: DayTracker::new(day),
            brk,
            today,
            boundary: None,
            last_mono: None,
            state: UsageState::Unknown,
            measurement: MeasurementInfo {
                source: MeasurementSource::None,
                last_report_age_ms: None,
                unknown_reason: Some("STARTING".into()),
                reporter: None,
                competing_reports: 0,
            },
            storage,
            limit_announced: today.active_ms >= limit_ms,
        }
    }

    pub fn set_policy(&mut self, policy: &Resolution) {
        self.policy = policy.active;
        self.policy_info = policy.info.clone();
        self.acct
            .set_threshold(u64::from(policy.active.idle_threshold_secs) * 1000);
        let limit_ms = u64::from(policy.active.daily_limit_secs) * 1000;
        if self.today.active_ms < limit_ms {
            self.limit_announced = false;
        }
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    pub fn break_state(&self) -> BreakState {
        self.brk
    }

    pub fn today(&self) -> (LocalDate, DayUsageMs) {
        (self.day.key(), self.today)
    }

    pub fn clock_adjusted_today(&self) -> bool {
        self.day.adjusted_today()
    }

    pub fn state(&self) -> UsageState {
        self.state
    }

    pub fn set_storage(&mut self, storage: StorageInfo) {
        self.storage = storage;
    }

    pub fn tick(
        &mut self,
        input: &TickInput,
        load_day: &mut dyn FnMut(LocalDate) -> DayUsageMs,
    ) -> TickOutput {
        let mut out = TickOutput::default();
        let now = input.mono_ms;
        let ev = |kind: &'static str, detail: String| LifecycleEvent {
            wall_ms: input.wall_ms,
            kind,
            detail,
        };

        // 1. Gün ve saat.
        let gap = self.last_mono.map(|l| now.saturating_sub(l)).unwrap_or(0);
        let suspended = input.suspended_since_last || gap > MAX_TICK_GAP_MS;
        let old_key = self.day.key();
        let (clock_events, changed) = self.day.observe(
            input.local,
            input.wall_ms,
            now,
            input.utc_offset_min,
            suspended,
        );
        for c in clock_events {
            out.events.push(match c {
                ClockEvent::ClockJump { skew_ms } => ev("CLOCK_JUMP", format!("skew_ms={skew_ms}")),
                ClockEvent::TimezoneChanged { from_min, to_min } => ev(
                    "TIMEZONE_CHANGED",
                    format!("from_min={from_min} to_min={to_min}"),
                ),
                ClockEvent::DayRollover { from, to } => {
                    ev("DAY_ROLLOVER", format!("from={from} to={to}"))
                }
                ClockEvent::ClockBehind { local, kept } => {
                    ev("CLOCK_BEHIND_DAY", format!("local={local} kept={kept}"))
                }
                ClockEvent::DayCorrected { from, to } => {
                    ev("DAY_CORRECTED", format!("from={from} to={to}"))
                }
            });
        }
        if let Some(new_day) = changed {
            self.boundary = Some((now, old_key));
            self.today = load_day(new_day);
            let limit_ms = u64::from(self.policy.daily_limit_secs) * 1000;
            self.limit_announced = self.today.active_ms >= limit_ms;
            out.day_changed = Some(new_day);
        }
        let key = self.day.key();

        // 2. Ölçüm.
        let mut segs: Vec<Segment> = vec![];
        self.acct.tick(now, input.ctx, input.gap_state, &mut segs);
        let to_wall = |m: u64| input.wall_ms.saturating_sub(now.saturating_sub(m));
        let mut break_events = vec![];
        for s in segs {
            let parts: Vec<(LocalDate, u64, u64)> = match self.boundary {
                Some((b, prev)) if s.start_ms < b && s.end_ms > b => {
                    vec![(prev, s.start_ms, b), (key, b, s.end_ms)]
                }
                Some((b, prev)) if s.end_ms <= b => vec![(prev, s.start_ms, s.end_ms)],
                _ => vec![(key, s.start_ms, s.end_ms)],
            };
            for (day, a, b) in parts {
                let ms = b - a;
                if day == key {
                    self.today.add(s.state, ms);
                }
                self.brk
                    .on_segment(s.state, ms, &self.policy, &mut break_events);
                out.pieces.push(Piece {
                    day,
                    state: s.state,
                    start_wall_ms: to_wall(a),
                    end_wall_ms: to_wall(b),
                    ms,
                });
            }
        }

        // 3. Mola.
        let elapsed = self.last_mono.map(|l| now.saturating_sub(l)).unwrap_or(0);
        self.brk.advance(elapsed, &self.policy, &mut break_events);
        for b in break_events {
            out.events.push(match b {
                BreakEvent::Started { seq } => ev("BREAK_STARTED", format!("seq={seq}")),
                BreakEvent::Completed {
                    seq,
                    active_during_ms,
                } => ev(
                    "BREAK_COMPLETED",
                    format!("seq={seq} active_during_ms={active_during_ms}"),
                ),
                BreakEvent::NaturalBreak { after_active_ms } => ev(
                    "BREAK_NATURAL",
                    format!("after_active_ms={after_active_ms}"),
                ),
            });
        }

        // 4. Anlık durum; kilit/uyku/bilinmeyen geçişleri kayda geçer.
        let state = self.acct.current_state(now, input.ctx);
        if state != self.state && (blocking(state) || blocking(self.state)) {
            let mut detail = format!("from={} to={}", self.state.as_str(), state.as_str());
            if state == UsageState::Unknown {
                if let Some(r) = &input.measurement.unknown_reason {
                    detail.push_str(&format!(" reason={r}"));
                }
            }
            out.events.push(ev("USAGE_STATE", detail));
        }
        self.state = state;
        self.measurement = input.measurement.clone();

        // 5. Günlük limit: yalnız bilgilendirme.
        let limit_ms = u64::from(self.policy.daily_limit_secs) * 1000;
        if !self.limit_announced && self.today.active_ms >= limit_ms {
            self.limit_announced = true;
            out.events.push(ev(
                "LIMIT_REACHED",
                format!("day={key} active_ms={}", self.today.active_ms),
            ));
        }

        self.last_mono = Some(now);
        out
    }

    pub fn status(&self, wall_ms: u64) -> ScreenTimeStatus {
        let key = self.day.key();
        let limit_secs = u64::from(self.policy.daily_limit_secs);
        let today = self.today.to_secs();
        let limit_reached = today.active_secs >= limit_secs;
        let mut notices = vec![];
        let upcoming = self.brk.seq + 1;
        match self.brk.phase {
            BreakPhase::FirstWarning => notices.push(Notice {
                id: format!("b{upcoming}-w1"),
                kind: NoticeKind::BreakFirstWarning,
            }),
            BreakPhase::FinalWarning => notices.push(Notice {
                id: format!("b{upcoming}-w2"),
                kind: NoticeKind::BreakFinalWarning,
            }),
            BreakPhase::BreakDue => notices.push(Notice {
                id: format!("b{}-due", self.brk.seq),
                kind: NoticeKind::BreakStarted,
            }),
            BreakPhase::BreakComplete => notices.push(Notice {
                id: format!("b{}-done", self.brk.seq),
                kind: NoticeKind::BreakCompleted,
            }),
            BreakPhase::Normal => {}
        }
        if limit_reached {
            notices.push(Notice {
                id: format!("{key}-limit"),
                kind: NoticeKind::LimitReached,
            });
        }
        ScreenTimeStatus {
            day: key.to_string(),
            state: self.state,
            today,
            daily_limit_secs: limit_secs,
            remaining_secs: limit_secs.saturating_sub(today.active_secs),
            limit_reached,
            break_phase: self.brk.phase,
            next_break_in_secs: self
                .brk
                .next_break_in_ms(&self.policy)
                .map(|ms| ms.div_ceil(1000)),
            break_remaining_secs: self
                .brk
                .break_remaining_ms(&self.policy)
                .map(|ms| ms.div_ceil(1000)),
            notices,
            policy: self.policy_info.clone(),
            measurement: self.measurement.clone(),
            storage: self.storage.clone(),
            clock_adjusted_today: self.day.adjusted_today(),
            computed_at_ms: wall_ms,
        }
    }
}
