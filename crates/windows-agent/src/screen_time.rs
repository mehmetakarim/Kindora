//! Kullanım süresi motorunun agent içindeki çalışma zamanı (P1.C, ADR-0014).
//!
//! Ayrı bir iş parçacığı saniyede bir:
//! 1. DEV politika dosyasını (en çok 5 sn'de bir) denetler,
//! 2. platform sinyallerinden ve helper'ın son raporundan bağlamı kurar,
//! 3. motoru ilerletir, 15 sn'de bir (veya olay olduğunda) SQLite'a yazar,
//! 4. `GET_SCREEN_TIME` için anlık durumu günceller.
//!
//! P1.D (ADR-0015): agent aynı anda **tek** etkinlik raporlayıcısı kabul eder
//! ([`Lease`]). Arka plan helper'ı masaüstü köprüsünden önceliklidir; kopan
//! veya susan raporlayıcının yeri boşalır. Süre yalnız agent'ın kendi
//! döngüsünde sayıldığı için raporların tekrarı süreyi iki kez yazamaz.
//!
//! Hiçbir adım kilit, engelleme, program kapatma veya ağ işlemi yapmaz.

use crate::log;
use crate::platform::{self, Signals};
use agent_core::{ClientInfo, ScreenTimeService};
use ipc_protocol::{
    now_ms, ActivityReport, HealthCheckResult, MeasurementInfo, MeasurementSource, ReporterInfo,
    ReporterKind, ScreenTimeStatus, UsageState,
};
use screen_time::accounting::Ctx;
use screen_time::engine::{Engine, LifecycleEvent, Piece, TickInput};
use screen_time::policy::{self, DevFileRead, Resolution, StoredPolicy, MAX_POLICY_FILE_BYTES};
use screen_time::store::Store;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    /// Raporu gönderen sürecin oturumu; konsol oturumu değişince rapor atılır.
    session: Option<u32>,
}

/// Kabul edilen tek raporlayıcı.
#[derive(Debug, Clone, Copy)]
struct Lease {
    client: ClientInfo,
    kind: Option<ReporterKind>,
    /// Kabul anı (Unix ms) — tanılamada "son (yeniden) bağlantı".
    since_ms: u64,
    last_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeaseDecision {
    /// Aynı bağlantı: süre uzatılır.
    Refresh,
    /// Yer boş veya önceki raporlayıcı sustu: yeni raporlayıcı.
    Acquire,
    /// Daha öncelikli raporlayıcı (arka plan helper'ı köprünün yerini alır).
    Takeover,
    /// Başka geçerli raporlayıcı var: rapor sayılmaz.
    Reject,
}

/// Tek raporlayıcı kuralı. Öncelik eşitse ilk gelen korunur; böylece iki
/// helper sırayla raporlayıp ölçüm kaynağını titretemez.
fn decide(
    lease: Option<&Lease>,
    client: &ClientInfo,
    kind: Option<ReporterKind>,
    now: Instant,
    fresh_for: Duration,
) -> LeaseDecision {
    let Some(l) = lease else {
        return LeaseDecision::Acquire;
    };
    if l.client.connection_id == client.connection_id {
        return LeaseDecision::Refresh;
    }
    if now.saturating_duration_since(l.last_at) > fresh_for
        || l.client.session_id != client.session_id
    {
        return LeaseDecision::Acquire;
    }
    if ReporterKind::priority(kind) > ReporterKind::priority(l.kind) {
        return LeaseDecision::Takeover;
    }
    LeaseDecision::Reject
}

fn describe(client: &ClientInfo, kind: Option<ReporterKind>) -> String {
    let opt = |v: Option<u32>| v.map_or("-".to_string(), |v| v.to_string());
    format!(
        "kind={} session={} pid={} conn={}",
        kind.map_or("UNSPECIFIED", ReporterKind::as_str),
        opt(client.session_id),
        opt(client.pid),
        client.connection_id
    )
}

/// Bellekte bekleyen raporlayıcı olayları için üst sınır (taşma: en eskiler atılır).
const MAX_QUEUED_EVENTS: usize = 64;

#[derive(Debug)]
pub struct ScreenTimeRuntime {
    started: Instant,
    signals: Arc<dyn Signals>,
    report_fresh_for: Duration,
    report: Mutex<Option<Report>>,
    lease: Mutex<Option<Lease>>,
    competing_reports: AtomicU64,
    /// IPC iş parçacıklarından motor döngüsüne aktarılacak yaşam döngüsü olayları.
    queued_events: Mutex<Vec<LifecycleEvent>>,
    status: Mutex<Option<ScreenTimeStatus>>,
    last_tick: Mutex<Option<Instant>>,
    db_path: Mutex<Option<String>>,
}

impl ScreenTimeService for ScreenTimeRuntime {
    fn report_activity(&self, client: &ClientInfo, report: ActivityReport) -> bool {
        if !self.signals.accepts_report_from(client.session_id) {
            return false;
        }
        let now = Instant::now();
        {
            let Ok(mut lease) = self.lease.lock() else {
                return false;
            };
            let decision = decide(
                lease.as_ref(),
                client,
                report.kind,
                now,
                self.report_fresh_for,
            );
            let event = match decision {
                LeaseDecision::Reject => {
                    self.competing_reports.fetch_add(1, Ordering::Relaxed);
                    return false;
                }
                LeaseDecision::Refresh => None,
                LeaseDecision::Acquire => {
                    Some(("REPORTER_ACQUIRED", describe(client, report.kind)))
                }
                LeaseDecision::Takeover => Some((
                    "REPORTER_TAKEOVER",
                    format!(
                        "{} previous_{}",
                        describe(client, report.kind),
                        lease
                            .as_ref()
                            .map(|l| describe(&l.client, l.kind))
                            .unwrap_or_default()
                    ),
                )),
            };
            match (decision, lease.as_mut()) {
                (LeaseDecision::Refresh, Some(l)) => l.last_at = now,
                _ => {
                    *lease = Some(Lease {
                        client: *client,
                        kind: report.kind,
                        since_ms: now_ms(),
                        last_at: now,
                    })
                }
            }
            if let Some((kind, detail)) = event {
                self.queue_event(kind, detail);
            }
        }
        if let Ok(mut r) = self.report.lock() {
            *r = Some(Report {
                idle_ms: report.idle_ms,
                at: now,
                session: client.session_id,
            });
        }
        true
    }

    fn connection_closed(&self, connection_id: u64) {
        let released = self
            .lease
            .lock()
            .ok()
            .and_then(|mut l| l.take_if(|l| l.client.connection_id == connection_id));
        if let Some(l) = released {
            // Son rapor bir ölçüm noktasıdır; tazeliği bitince ölçüm UNKNOWN olur.
            // Kopuk süre için etkinlik uydurulmaz (girdi sınırı modeli).
            self.queue_event(
                "REPORTER_RELEASED",
                format!("reason=disconnected {}", describe(&l.client, l.kind)),
            );
        }
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

    fn queue_event(&self, kind: &'static str, detail: String) {
        if let Ok(mut q) = self.queued_events.lock() {
            if q.len() >= MAX_QUEUED_EVENTS {
                q.remove(0);
            }
            q.push(LifecycleEvent {
                wall_ms: now_ms(),
                kind,
                detail,
            });
        }
    }

    /// Motor döngüsünden: susan raporlayıcının ve başka oturuma ait raporun
    /// yerini boşaltır, bekleyen olayları verir.
    fn housekeeping(&self, now: Instant) -> Vec<LifecycleEvent> {
        let mut released = None;
        if let Ok(mut lease) = self.lease.lock() {
            let reason = lease.as_ref().and_then(|l| {
                if !self.signals.accepts_report_from(l.client.session_id) {
                    Some("console_session_changed")
                } else if now.saturating_duration_since(l.last_at) > self.report_fresh_for {
                    Some("stale")
                } else {
                    None
                }
            });
            if let Some(reason) = reason {
                released = lease.take().map(|l| (reason, l));
            }
        }
        if let Some((reason, l)) = released {
            self.queue_event(
                "REPORTER_RELEASED",
                format!("reason={reason} {}", describe(&l.client, l.kind)),
            );
        }
        if let Ok(mut r) = self.report.lock() {
            if r.is_some_and(|r| !self.signals.accepts_report_from(r.session)) {
                // Önceki konsol oturumunun raporu yeni oturuma yazılmaz.
                *r = None;
            }
        }
        self.queued_events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default()
    }

    fn reporter_info(&self) -> Option<ReporterInfo> {
        let l = (*self.lease.lock().ok()?)?;
        Some(ReporterInfo {
            kind: l.kind,
            session_id: l.client.session_id,
            pid: l.client.pid,
            since_ms: l.since_ms,
        })
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
                reporter: None,
                competing_reports: 0,
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
        reporter: None,
        competing_reports: 0,
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
            ..
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
        report_fresh_for: cfg.report_fresh_for,
        report: Mutex::new(None),
        lease: Mutex::new(None),
        competing_reports: AtomicU64::new(0),
        queued_events: Mutex::new(Vec::new()),
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

        pending_events.extend(rt.housekeeping(now));
        let report = rt.report.lock().ok().and_then(|r| *r);
        let (ctx, mut measurement) = measure(
            rt.signals.as_ref(),
            report,
            rt.started,
            cfg.report_fresh_for,
        );
        measurement.reporter = rt.reporter_info();
        measurement.competing_reports = rt.competing_reports.load(Ordering::Relaxed);
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
        /// Konsol oturumu 1 → 2 (hızlı kullanıcı değiştirme benzetimi).
        switched: AtomicBool,
    }

    fn client(conn: u64, session: Option<u32>) -> ClientInfo {
        ClientInfo {
            connection_id: conn,
            session_id: session,
            pid: Some(1000 + conn as u32),
        }
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
            s == Some(if self.switched.load(Ordering::SeqCst) {
                2
            } else {
                1
            })
        }
    }

    #[test]
    fn lease_decisions() {
        use LeaseDecision::*;
        let now = Instant::now();
        let fresh = Duration::from_secs(15);
        let lease = |conn, kind, age_s| Lease {
            client: client(conn, Some(1)),
            kind,
            since_ms: 0,
            last_at: now - Duration::from_secs(age_s),
        };
        let bg = Some(ReporterKind::BackgroundLogonTask);
        let bridge = Some(ReporterKind::DesktopBridge);
        let c2 = client(2, Some(1));
        assert_eq!(decide(None, &c2, bridge, now, fresh), Acquire);
        assert_eq!(
            decide(Some(&lease(2, bridge, 1)), &c2, bridge, now, fresh),
            Refresh
        );
        // Eşit öncelik: ilk gelen korunur (iki arka plan helper'ı titreşmez).
        assert_eq!(decide(Some(&lease(1, bg, 1)), &c2, bg, now, fresh), Reject);
        assert_eq!(
            decide(
                Some(&lease(1, None, 1)),
                &c2,
                Some(ReporterKind::OneShot),
                now,
                fresh
            ),
            Reject
        );
        // Arka plan helper'ı köprünün yerini alır; tersi olmaz.
        assert_eq!(
            decide(Some(&lease(1, bridge, 1)), &c2, bg, now, fresh),
            Takeover
        );
        assert_eq!(
            decide(Some(&lease(1, bg, 1)), &c2, bridge, now, fresh),
            Reject
        );
        // Susan raporlayıcının yeri boşalmış sayılır.
        assert_eq!(
            decide(Some(&lease(1, bg, 16)), &c2, bridge, now, fresh),
            Acquire
        );
        // Başka oturumun raporlayıcısı (kullanıcı değişti) geçerli değildir.
        let other = Lease {
            client: client(1, Some(2)),
            ..lease(1, bg, 1)
        };
        assert_eq!(decide(Some(&other), &c2, bridge, now, fresh), Acquire);
    }

    fn total(s: &ScreenTimeStatus) -> u64 {
        let t = &s.today;
        t.active_secs + t.idle_secs + t.locked_secs + t.suspended_secs + t.unknown_secs
    }

    #[test]
    fn single_reporter_lifecycle_and_no_double_counting() {
        let dir =
            std::env::temp_dir().join(format!("kd-lease-{}-{}", std::process::id(), now_ms()));
        let mut cfg = ScreenTimeConfig::new(Some(dir.clone()), None);
        cfg.tick = Duration::from_millis(100);
        cfg.flush_every = Duration::from_millis(200);
        cfg.report_fresh_for = Duration::from_millis(600);
        let sig = Arc::new(FakeSignals::default());
        cfg.signals = sig.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let (rt, h) = start(cfg, shutdown.clone());
        let started = Instant::now();
        let rep = |kind| ActivityReport {
            idle_ms: Some(0),
            kind: Some(kind),
        };
        let reporter = || rt.status().and_then(|s| s.measurement.reporter);
        let wait = |what: &str, f: &dyn Fn() -> bool| {
            let t = Instant::now();
            while !f() {
                assert!(t.elapsed() < Duration::from_secs(5), "{what}");
                thread::sleep(Duration::from_millis(20));
            }
        };

        // Köprü yer alır; arka plan helper'ı onun yerini alır; köprü artık reddedilir.
        let bridge = client(1, Some(1));
        let bg = client(2, Some(1));
        assert!(rt.report_activity(&bridge, rep(ReporterKind::DesktopBridge)));
        assert!(rt.report_activity(&bg, rep(ReporterKind::BackgroundLogonTask)));
        assert!(!rt.report_activity(&bridge, rep(ReporterKind::DesktopBridge)));
        // İkinci bir arka plan helper'ı (ör. eski sürüm) da reddedilir.
        assert!(!rt.report_activity(&client(3, Some(1)), rep(ReporterKind::BackgroundManual)));
        wait("reporter=bg", &|| {
            reporter().is_some_and(|r| {
                r.kind == Some(ReporterKind::BackgroundLogonTask) && r.pid == Some(1002)
            })
        });
        assert_eq!(rt.status().unwrap().measurement.competing_reports, 2);

        // Aynı anda iki raporlayıcı rapor yağdırsa da süre bir kez sayılır.
        let flood_end = Instant::now() + Duration::from_millis(500);
        while Instant::now() < flood_end {
            rt.report_activity(&bg, rep(ReporterKind::BackgroundLogonTask));
            rt.report_activity(&bridge, rep(ReporterKind::DesktopBridge));
            thread::sleep(Duration::from_millis(5));
        }

        // Arka plan helper'ı koptu: yer boşalır, köprü devralır.
        rt.connection_closed(bg.connection_id);
        assert!(reporter_released_now(&rt));
        assert!(rt.report_activity(&bridge, rep(ReporterKind::DesktopBridge)));
        wait("reporter=bridge", &|| {
            reporter().is_some_and(|r| r.kind == Some(ReporterKind::DesktopBridge))
        });

        // Rapor kesildi: tazelik bitince yer boşalır, ölçüm UNKNOWN (uydurma yok).
        wait("stale release", &|| {
            rt.status()
                .is_some_and(|s| s.measurement.reporter.is_none() && s.state == UsageState::Unknown)
        });

        // Konsol oturumu değişti: eski oturumun raporu ve raporlayıcısı atılır.
        assert!(rt.report_activity(&bg, rep(ReporterKind::BackgroundLogonTask)));
        sig.switched.store(true, Ordering::SeqCst);
        wait("session switch", &|| {
            rt.status().is_some_and(|s| {
                s.measurement.reporter.is_none()
                    && s.measurement.unknown_reason.as_deref() == Some("NO_HELPER")
            })
        });
        assert!(!rt.report_activity(&bg, rep(ReporterKind::BackgroundLogonTask)));
        assert!(rt.report_activity(&client(4, Some(2)), rep(ReporterKind::BackgroundLogonTask)));

        let elapsed = started.elapsed().as_secs();
        let s = rt.status().unwrap();
        shutdown.store(true, Ordering::SeqCst);
        h.join().unwrap();
        assert!(
            total(&s) <= elapsed + 1,
            "süre iki kez sayıldı: {} > {elapsed}",
            total(&s)
        );

        let store = Store::open(Some(&dir), "check", now_ms());
        assert_eq!(store.count_events("REPORTER_ACQUIRED"), 4);
        assert_eq!(store.count_events("REPORTER_TAKEOVER"), 1);
        assert_eq!(store.count_events("REPORTER_RELEASED"), 3);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn reporter_released_now(rt: &ScreenTimeRuntime) -> bool {
        rt.lease.lock().unwrap().is_none()
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
                session: Some(1),
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
            session: Some(1),
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
        let report = ActivityReport {
            idle_ms: Some(0),
            kind: None,
        };
        assert!(!rt.report_activity(&client(1, Some(2)), report));
        assert!(rt.report_activity(&client(1, Some(1)), report));
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
