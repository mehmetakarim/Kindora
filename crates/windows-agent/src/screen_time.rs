//! Kullanım süresi motorunun agent içindeki çalışma zamanı (P1.C, ADR-0014).
//!
//! Ayrı bir iş parçacığı saniyede bir:
//! 1. DEV politika dosyasını (en çok 5 sn'de bir) denetler,
//! 2. platform sinyallerinden ve helper'ın son raporundan bağlamı kurar,
//! 3. motoru ilerletir, 15 sn'de bir (veya olay olduğunda) SQLite'a yazar,
//! 4. `GET_SCREEN_TIME` için anlık durumu günceller.
//!
//! Hiçbir adım kilit, engelleme, program kapatma veya ağ işlemi yapmaz.

use crate::log;
use crate::platform::{self, Signals};
use agent_core::ScreenTimeService;
use ipc_protocol::{
    now_ms, ActivityReport, HealthCheckResult, MeasurementInfo, MeasurementSource,
    ScreenTimeStatus, UsageState,
};
use screen_time::accounting::Ctx;
use screen_time::engine::{Engine, LifecycleEvent, Piece, TickInput};
use screen_time::policy::{self, DevFileRead, Resolution, StoredPolicy, MAX_POLICY_FILE_BYTES};
use screen_time::store::Store;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ScreenTimeConfig {
    /// `None`: kalıcı depo yok (bellek içi).
    pub data_dir: Option<PathBuf>,
    /// DEV test politikası dosyası (agent klasöründe `kindora-dev-policy.json`).
    pub dev_policy_path: Option<PathBuf>,
    pub tick: Duration,
    pub flush_every: Duration,
    pub policy_check_every: Duration,
    /// Helper raporu bu süreden eskiyse ölçüm `UNKNOWN` olur.
    pub report_fresh_for: Duration,
    pub signals: Arc<dyn Signals>,
}

impl ScreenTimeConfig {
    pub fn new(data_dir: Option<PathBuf>, dev_policy_path: Option<PathBuf>) -> Self {
        ScreenTimeConfig {
            data_dir,
            dev_policy_path,
            tick: Duration::from_secs(1),
            flush_every: Duration::from_secs(15),
            policy_check_every: Duration::from_secs(5),
            report_fresh_for: Duration::from_secs(15),
            signals: platform::default_signals(),
        }
    }
}

/// Agent klasöründeki DEV politika dosyası yolu.
pub fn dev_policy_beside_exe() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(policy::DEV_POLICY_FILE_NAME)))
}

#[derive(Debug, Clone, Copy)]
struct Report {
    idle_ms: Option<u64>,
    at: Instant,
}

#[derive(Debug)]
pub struct ScreenTimeRuntime {
    started: Instant,
    signals: Arc<dyn Signals>,
    report: Mutex<Option<Report>>,
    status: Mutex<Option<ScreenTimeStatus>>,
    last_tick: Mutex<Option<Instant>>,
    db_path: Mutex<Option<String>>,
}

impl ScreenTimeService for ScreenTimeRuntime {
    fn report_activity(&self, client_session: Option<u32>, report: ActivityReport) -> bool {
        if !self.signals.accepts_report_from(client_session) {
            return false;
        }
        if let Ok(mut r) = self.report.lock() {
            *r = Some(Report {
                idle_ms: report.idle_ms,
                at: Instant::now(),
            });
        }
        true
    }

    fn status(&self) -> Option<ScreenTimeStatus> {
        self.status.lock().ok().and_then(|s| s.clone())
    }

    fn health(&self) -> Vec<HealthCheckResult> {
        let fresh = self
            .last_tick
            .lock()
            .ok()
            .and_then(|t| *t)
            .is_some_and(|t| t.elapsed() < Duration::from_secs(5));
        let status = self.status();
        let engine_detail = status.as_ref().map(|s| {
            format!(
                "state={} policy={:?} v{}",
                s.state.as_str(),
                s.policy.source,
                s.policy.version
            )
        });
        let storage = status.as_ref().map(|s| s.storage.clone());
        vec![
            HealthCheckResult {
                name: "screen_time_engine".into(),
                ok: fresh,
                detail: engine_detail.or(Some("not running".into())),
            },
            HealthCheckResult {
                name: "local_storage".into(),
                ok: storage.as_ref().is_some_and(|s| s.persistent),
                detail: Some(match storage {
                    Some(s) if s.persistent => format!(
                        "sqlite schema v{}{}",
                        s.schema_version,
                        if s.recovered { " (recovered)" } else { "" }
                    ),
                    Some(s) => format!("memory only (schema v{})", s.schema_version),
                    None => "not open".into(),
                }),
            },
        ]
    }
}

impl ScreenTimeRuntime {
    pub fn db_path(&self) -> Option<String> {
        self.db_path.lock().ok().and_then(|p| p.clone())
    }
}

fn read_dev_file(path: Option<&Path>) -> DevFileRead {
    let Some(path) = path else {
        return DevFileRead::Missing;
    };
    match std::fs::File::open(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => DevFileRead::Missing,
        Err(e) => DevFileRead::Unreadable(e.kind().to_string()),
        Ok(f) => {
            let mut buf = Vec::new();
            match f
                .take(MAX_POLICY_FILE_BYTES as u64 + 1)
                .read_to_end(&mut buf)
            {
                Ok(_) => DevFileRead::Content(buf),
                Err(e) => DevFileRead::Unreadable(e.kind().to_string()),
            }
        }
    }
}

fn policy_event(r: &Resolution, wall_ms: u64) -> Option<LifecycleEvent> {
    use ipc_protocol::PolicyStatus::*;
    let detail = r.detail.clone()?;
    let kind = match r.info.status {
        Ok if r.store.is_some() => "POLICY_APPLIED",
        Ok => return None,
        InvalidKeptLast | InvalidUsingDefault => "POLICY_INVALID",
        VersionRejected => "POLICY_VERSION_REJECTED",
    };
    Some(LifecycleEvent {
        wall_ms,
        kind,
        detail: format!(
            "status={:?} version={} {detail}",
            r.info.status, r.active.version
        ),
    })
}

/// Bağlamı kurar: uyku → kilit/oturum → helper raporu.
fn measure(
    signals: &dyn Signals,
    report: Option<Report>,
    started: Instant,
    fresh_for: Duration,
) -> (Ctx, MeasurementInfo) {
    let unknown = |source, age: Option<u64>, reason: &str| {
        (
            Ctx::Blocked(UsageState::Unknown),
            MeasurementInfo {
                source,
                last_report_age_ms: age,
                unknown_reason: Some(reason.into()),
            },
        )
    };
    let age = report.map(|r| r.at.elapsed().as_millis() as u64);
    let fresh = report.filter(|r| r.at.elapsed() <= fresh_for);
    let source = if fresh.is_some() {
        MeasurementSource::SessionHelper
    } else {
        MeasurementSource::None
    };
    let info = MeasurementInfo {
        source,
        last_report_age_ms: age,
        unknown_reason: None,
    };
    if signals.suspended() {
        return (Ctx::Blocked(UsageState::Suspended), info);
    }
    match signals.session_block() {
        Ok(Some(state)) => return (Ctx::Blocked(state), info),
        Err(code) => return unknown(source, age, code),
        Ok(None) => {}
    }
    match fresh {
        None if report.is_none() => unknown(MeasurementSource::None, None, "NO_HELPER"),
        None => unknown(MeasurementSource::None, age, "STALE_REPORT"),
        Some(Report { idle_ms: None, .. }) => unknown(source, age, "IDLE_API_FAILED"),
        Some(Report {
            idle_ms: Some(idle),
            at,
        }) => {
            let at_mono = at.saturating_duration_since(started).as_millis() as u64;
            (
                Ctx::Observing {
                    last_input_ms: Some(at_mono.saturating_sub(idle)),
                },
                info,
            )
        }
    }
}

fn log_lifecycle(e: &LifecycleEvent) {
    log::event(
        "screen_time",
        &[("kind", e.kind.to_string()), ("detail", e.detail.clone())],
    );
}

/// Motoru başlatır. Dönen iş parçacığı `shutdown` ile durur ve son kaydı yapar.
pub fn start(
    cfg: ScreenTimeConfig,
    shutdown: Arc<AtomicBool>,
) -> (Arc<ScreenTimeRuntime>, JoinHandle<()>) {
    let started = Instant::now();
    let rt = Arc::new(ScreenTimeRuntime {
        started,
        signals: cfg.signals.clone(),
        report: Mutex::new(None),
        status: Mutex::new(None),
        last_tick: Mutex::new(None),
        db_path: Mutex::new(None),
    });
    let shared = rt.clone();
    let handle = thread::spawn(move || run(cfg, &shared, &shutdown));
    (rt, handle)
}

fn run(cfg: ScreenTimeConfig, rt: &ScreenTimeRuntime, shutdown: &AtomicBool) {
    let run_id = format!("a{}-{}", std::process::id(), now_ms());
    let wall = now_ms();
    if let Some(dir) = &cfg.data_dir {
        if let Err(e) = platform::prepare_data_dir(dir) {
            log::event("screen_time_storage_error", &[("error", e.to_string())]);
        }
    }
    let mut store = Store::open(cfg.data_dir.as_deref(), &run_id, wall);
    if let Ok(mut p) = rt.db_path.lock() {
        *p = store.path().map(|p| p.display().to_string());
    }
    let mut pending_events: Vec<LifecycleEvent> = store
        .take_startup_notes()
        .into_iter()
        .map(|(kind, detail)| LifecycleEvent {
            wall_ms: wall,
            kind,
            detail,
        })
        .collect();
    pending_events.push(LifecycleEvent {
        wall_ms: wall,
        kind: "AGENT_START",
        detail: format!(
            "run_id={run_id} persistent={} db={}",
            store.info().persistent,
            rt.db_path().unwrap_or_else(|| "-".into())
        ),
    });

    let mut stored: Option<StoredPolicy> = store.load_policy();
    let mut last_read = read_dev_file(cfg.dev_policy_path.as_deref());
    let mut resolution = policy::resolve(&last_read, stored.as_ref());
    if let Some(sp) = &resolution.store {
        if store.save_policy(sp, wall).is_ok() {
            stored = Some(sp.clone());
        }
    }
    pending_events.extend(policy_event(&resolution, wall));

    let (day, _) = platform::local_stamp();
    let brk = store
        .load_break()
        .map(|(b, saved)| b.restore_after_downtime(wall.saturating_sub(saved), &resolution.active))
        .unwrap_or_default();
    let mut engine = Engine::new(
        &resolution,
        store.info().clone(),
        day,
        store.load_day(day),
        brk,
    );

    let mut pending_pieces: Vec<Piece> = vec![];
    let mut last_flush = Instant::now();
    let mut last_policy_check = Instant::now();
    let mut next_tick = Instant::now();

    loop {
        let stopping = shutdown.load(Ordering::SeqCst);
        if !stopping {
            while Instant::now() < next_tick {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            next_tick += cfg.tick;
            if next_tick < Instant::now() {
                next_tick = Instant::now() + cfg.tick;
            }
        }
        let now = Instant::now();
        let wall = now_ms();

        if last_policy_check.elapsed() >= cfg.policy_check_every {
            last_policy_check = now;
            let read = read_dev_file(cfg.dev_policy_path.as_deref());
            if read != last_read {
                last_read = read;
                let r = policy::resolve(&last_read, stored.as_ref());
                if let Some(sp) = &r.store {
                    if store.save_policy(sp, wall).is_ok() {
                        stored = Some(sp.clone());
                    }
                }
                pending_events.extend(policy_event(&r, wall));
                if r.info != resolution.info {
                    engine.set_policy(&r);
                    resolution = r;
                }
            }
        }

        let report = rt.report.lock().ok().and_then(|r| *r);
        let (ctx, measurement) = measure(
            rt.signals.as_ref(),
            report,
            rt.started,
            cfg.report_fresh_for,
        );
        let resumed = rt.signals.take_resumed();
        let (local, offset) = platform::local_stamp();
        let input = TickInput {
            mono_ms: now.saturating_duration_since(rt.started).as_millis() as u64,
            wall_ms: wall,
            local,
            utc_offset_min: offset,
            ctx,
            gap_state: if resumed || rt.signals.suspended() {
                UsageState::Suspended
            } else {
                UsageState::Unknown
            },
            suspended_since_last: resumed,
            measurement,
        };
        let out = engine.tick(&input, &mut |d| store.load_day(d));
        pending_pieces.extend(out.pieces);
        pending_events.extend(out.events);

        let must_flush = stopping
            || out.day_changed.is_some()
            || !pending_events.is_empty()
            || last_flush.elapsed() >= cfg.flush_every;
        if must_flush {
            if stopping {
                pending_events.push(LifecycleEvent {
                    wall_ms: wall,
                    kind: "AGENT_STOP",
                    detail: format!("run_id={run_id}"),
                });
            }
            pending_events.iter().for_each(log_lifecycle);
            let adjusted = engine.clock_adjusted_today().then(|| engine.today().0);
            match store.flush(
                &pending_pieces,
                &pending_events,
                &engine.break_state(),
                adjusted,
                wall,
            ) {
                Ok(()) => {
                    pending_pieces.clear();
                    pending_events.clear();
                }
                Err(e) => {
                    // Kayıt başarısız: veriler bellekte kalır, sonraki denemede yazılır.
                    // Olay listesi sınırsız büyümesin.
                    pending_events.clear();
                    log::event("screen_time_flush_failed", &[("error", e.to_string())]);
                }
            }
            if out.day_changed.is_some() {
                let _ = store.prune(Some(engine.today().0));
            }
            last_flush = now;
        }

        if let Ok(mut s) = rt.status.lock() {
            *s = Some(engine.status(wall));
        }
        if let Ok(mut t) = rt.last_tick.lock() {
            *t = Some(now);
        }
        if stopping {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_protocol::PolicySource;

    #[derive(Debug, Default)]
    struct FakeSignals {
        suspended: AtomicBool,
        locked: AtomicBool,
        fail: AtomicBool,
    }

    impl Signals for FakeSignals {
        fn suspended(&self) -> bool {
            self.suspended.load(Ordering::SeqCst)
        }
        fn take_resumed(&self) -> bool {
            false
        }
        fn session_block(&self) -> Result<Option<UsageState>, &'static str> {
            if self.fail.load(Ordering::SeqCst) {
                Err("LOCK_STATE_QUERY_FAILED")
            } else if self.locked.load(Ordering::SeqCst) {
                Ok(Some(UsageState::SessionLocked))
            } else {
                Ok(None)
            }
        }
        fn accepts_report_from(&self, s: Option<u32>) -> bool {
            s == Some(1)
        }
    }

    #[test]
    fn measurement_context_priorities() {
        let sig = FakeSignals::default();
        let started = Instant::now() - Duration::from_secs(100);
        let fresh = Duration::from_secs(15);
        let report = |idle| {
            Some(Report {
                idle_ms: idle,
                at: Instant::now(),
            })
        };

        let (ctx, info) = measure(&sig, None, started, fresh);
        assert_eq!(ctx, Ctx::Blocked(UsageState::Unknown));
        assert_eq!(info.unknown_reason.as_deref(), Some("NO_HELPER"));

        let (ctx, info) = measure(&sig, report(None), started, fresh);
        assert_eq!(ctx, Ctx::Blocked(UsageState::Unknown));
        assert_eq!(info.unknown_reason.as_deref(), Some("IDLE_API_FAILED"));

        let (ctx, info) = measure(&sig, report(Some(4000)), started, fresh);
        match ctx {
            Ctx::Observing {
                last_input_ms: Some(t),
            } => assert!((95_000..=96_500).contains(&t), "{t}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(info.source, MeasurementSource::SessionHelper);

        let stale = Some(Report {
            idle_ms: Some(0),
            at: Instant::now() - Duration::from_secs(30),
        });
        let (_, info) = measure(&sig, stale, started, fresh);
        assert_eq!(info.unknown_reason.as_deref(), Some("STALE_REPORT"));

        sig.locked.store(true, Ordering::SeqCst);
        assert_eq!(
            measure(&sig, report(Some(0)), started, fresh).0,
            Ctx::Blocked(UsageState::SessionLocked)
        );
        sig.suspended.store(true, Ordering::SeqCst);
        assert_eq!(
            measure(&sig, report(Some(0)), started, fresh).0,
            Ctx::Blocked(UsageState::Suspended)
        );
        sig.suspended.store(false, Ordering::SeqCst);
        sig.fail.store(true, Ordering::SeqCst);
        let (ctx, info) = measure(&sig, report(Some(0)), started, fresh);
        assert_eq!(ctx, Ctx::Blocked(UsageState::Unknown));
        assert_eq!(
            info.unknown_reason.as_deref(),
            Some("LOCK_STATE_QUERY_FAILED")
        );
    }

    #[test]
    fn runtime_persists_and_reports_only_from_console_session() {
        let dir = std::env::temp_dir().join(format!("kd-rt-{}-{}", std::process::id(), now_ms()));
        let mut cfg = ScreenTimeConfig::new(Some(dir.clone()), None);
        cfg.tick = Duration::from_millis(100);
        cfg.signals = Arc::new(FakeSignals::default());
        let shutdown = Arc::new(AtomicBool::new(false));
        let (rt, h) = start(cfg, shutdown.clone());
        assert!(!rt.report_activity(Some(2), ActivityReport { idle_ms: Some(0) }));
        assert!(rt.report_activity(Some(1), ActivityReport { idle_ms: Some(0) }));
        thread::sleep(Duration::from_millis(400));
        let s = rt.status().expect("status");
        assert!(s.storage.persistent);
        assert_eq!(s.policy.source, PolicySource::BuiltinDefault);
        assert!(rt.health().iter().all(|c| c.ok), "{:?}", rt.health());
        shutdown.store(true, Ordering::SeqCst);
        h.join().unwrap();
        assert!(dir.join(screen_time::store::DB_FILE_NAME).exists());
        let store = Store::open(Some(&dir), "check", now_ms());
        assert_eq!(store.count_events("AGENT_START"), 1);
        assert_eq!(store.count_events("AGENT_STOP"), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
