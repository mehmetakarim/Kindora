//! Motor + gerçek SQLite ile benzetilmiş saat senaryoları (P1.C §2 maddeleri 1–10).
//!
//! Windows API'leri burada **taklit edilir** (kilit, uyku, helper raporu); bu
//! testler gerçek Windows kabulü değildir. Gerçek API'ler Windows CI ve Lenovo
//! testinde ayrıca doğrulanır.

use ipc_protocol::{
    BreakPhase, MeasurementInfo, MeasurementSource, NoticeKind, PolicySource, PolicyStatus,
    UsageState,
};
use screen_time::accounting::Ctx;
use screen_time::calendar::LocalDate;
use screen_time::engine::{Engine, LifecycleEvent, Piece, TickInput};
use screen_time::policy::{resolve, DevFileRead, Resolution};
use screen_time::store::{Store, DB_FILE_NAME, SCHEMA_VERSION};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

const S: u64 = 1000;
/// 2026-10-29 10:00:00 UTC.
const T0_WALL: u64 = 1_761_732_000_000;
const OFFSET_MIN: i32 = 180;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "kd-st-{}-{}-{tag}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn local_date(wall_ms: u64, offset_min: i32) -> LocalDate {
    let secs = wall_ms as i64 / 1000 + i64::from(offset_min) * 60;
    LocalDate::from_days(secs.div_euclid(86_400))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum User {
    /// Her 2 sn'de bir girdi.
    Busy,
    /// Girdi yok.
    Away,
    Locked,
    /// Helper ölçemedi (API hatası).
    ApiError,
    /// Helper yok / rapor yok.
    NoHelper,
}

/// Agent çalışma zamanının benzetimi: saniyede bir ölçüm, 15 sn'de bir kayıt.
struct Sim {
    dir: Option<PathBuf>,
    store: Store,
    engine: Engine,
    policy: Resolution,
    mono: u64,
    wall: u64,
    offset: i32,
    last_input: u64,
    pending_pieces: Vec<Piece>,
    pending_events: Vec<LifecycleEvent>,
    since_flush: u64,
    resumed: bool,
}

impl Sim {
    fn start(dir: Option<&Path>, policy: Resolution, wall: u64) -> Sim {
        let mut store = Store::open(dir, "test-run", wall);
        let today = local_date(wall, OFFSET_MIN);
        let base = store.load_day(today);
        let brk = store
            .load_break()
            .map(|(b, saved)| b.restore_after_downtime(wall.saturating_sub(saved), &policy.active))
            .unwrap_or_default();
        let engine = Engine::new(&policy, store.info().clone(), today, base, brk);
        let notes = store.take_startup_notes();
        let mut sim = Sim {
            dir: dir.map(Path::to_path_buf),
            store,
            engine,
            policy,
            mono: 10_000,
            wall,
            offset: OFFSET_MIN,
            last_input: 0,
            pending_pieces: vec![],
            pending_events: notes
                .into_iter()
                .map(|(kind, detail)| LifecycleEvent {
                    wall_ms: wall,
                    kind,
                    detail,
                })
                .collect(),
            since_flush: 0,
            resumed: false,
        };
        sim.step(User::NoHelper);
        sim
    }

    fn measurement(user: User) -> (Ctx, MeasurementInfo) {
        let helper = |reason: Option<&str>| MeasurementInfo {
            source: if reason == Some("NO_HELPER") {
                MeasurementSource::None
            } else {
                MeasurementSource::SessionHelper
            },
            last_report_age_ms: Some(500),
            unknown_reason: reason.map(str::to_owned),
        };
        match user {
            User::Busy | User::Away => (
                Ctx::Observing {
                    last_input_ms: None,
                },
                helper(None),
            ),
            User::Locked => (Ctx::Blocked(UsageState::SessionLocked), helper(None)),
            User::ApiError => (
                Ctx::Blocked(UsageState::Unknown),
                helper(Some("IDLE_API_FAILED")),
            ),
            User::NoHelper => (Ctx::Blocked(UsageState::Unknown), helper(Some("NO_HELPER"))),
        }
    }

    fn step(&mut self, user: User) {
        if user == User::Busy && self.mono.is_multiple_of(2 * S) {
            self.last_input = self.mono;
        }
        let (mut ctx, measurement) = Self::measurement(user);
        if let Ctx::Observing { last_input_ms } = &mut ctx {
            *last_input_ms = (self.last_input > 0).then_some(self.last_input);
        }
        let input = TickInput {
            mono_ms: self.mono,
            wall_ms: self.wall,
            local: local_date(self.wall, self.offset),
            utc_offset_min: self.offset,
            ctx,
            gap_state: if self.resumed {
                UsageState::Suspended
            } else {
                UsageState::Unknown
            },
            suspended_since_last: std::mem::take(&mut self.resumed),
            measurement,
        };
        let store = &self.store;
        let out = self.engine.tick(&input, &mut |d| store.load_day(d));
        self.pending_pieces.extend(out.pieces);
        self.pending_events.extend(out.events);
        self.since_flush += 1;
        if self.since_flush >= 15 || out.day_changed.is_some() {
            self.flush();
        }
    }

    fn flush(&mut self) {
        let adjusted = self
            .engine
            .clock_adjusted_today()
            .then(|| self.engine.today().0);
        self.store
            .flush(
                &self.pending_pieces,
                &self.pending_events,
                &self.engine.break_state(),
                adjusted,
                self.wall,
            )
            .expect("flush");
        self.pending_pieces.clear();
        self.pending_events.clear();
        self.since_flush = 0;
    }

    fn run(&mut self, user: User, secs: u64) {
        for _ in 0..secs {
            self.mono += S;
            self.wall += S;
            self.step(user);
        }
    }

    /// Süreç durmadan uyku: monoton saat (Windows QPC/macOS) uyku süresince
    /// ilerlemeyebilir; burada en kötü durum olarak ilerlediği varsayılır.
    fn sleep(&mut self, secs: u64, power_event: bool) {
        self.mono += secs * S;
        self.wall += secs * S;
        self.resumed = power_event;
        self.step(User::Away);
    }

    fn status(&self) -> ipc_protocol::ScreenTimeStatus {
        self.engine.status(self.wall)
    }

    fn active_secs(&self) -> u64 {
        self.status().today.active_secs
    }

    /// Agent'ı "kapatıp" aynı veritabanıyla yeniden başlatır.
    fn restart(mut self, downtime_secs: u64) -> Sim {
        self.flush();
        let dir = self.dir.clone();
        let wall = self.wall + downtime_secs * S;
        let policy = self.policy.clone();
        drop(self);
        Sim::start(dir.as_deref(), policy, wall)
    }
}

fn default_policy() -> Resolution {
    resolve(&DevFileRead::Missing, None)
}

fn quick_policy() -> Resolution {
    // DEV hızlı test: 10 dk limit, 3 dk'da bir 1 dk mola, 60/20 sn uyarı, 60 sn boşta eşiği.
    let file = br#"{"kind":"kindora-dev-policy","dev_only":true,"version":1,
        "daily_limit_secs":600,"break_interval_secs":180,"break_duration_secs":60,
        "first_warning_secs":60,"final_warning_secs":20,"idle_threshold_secs":60}"#;
    resolve(&DevFileRead::Content(file.to_vec()), None)
}

#[test]
fn s1_s2_s3_active_idle_and_return() {
    let mut sim = Sim::start(None, default_policy(), T0_WALL);
    sim.run(User::Busy, 600);
    let a1 = sim.active_secs();
    assert!((595..=600).contains(&a1), "{a1}");
    assert_eq!(sim.status().state, UsageState::Active);

    // 20 dk uzakta: eşik (5 dk) dolunca geriye dönük IDLE; aktif artmaz.
    sim.run(User::Away, 1200);
    assert_eq!(sim.status().state, UsageState::Idle);
    assert!(sim.active_secs() <= a1 + 2, "boşta süre aktif sayılmadı");
    assert!(sim.status().today.idle_secs >= 1195);

    sim.run(User::Busy, 300);
    assert!(sim.active_secs() >= a1 + 290);
    assert_eq!(
        sim.status().measurement.source,
        MeasurementSource::SessionHelper
    );
}

#[test]
fn s4_session_lock_is_not_usage() {
    let mut sim = Sim::start(None, default_policy(), T0_WALL);
    sim.run(User::Busy, 120);
    let before = sim.active_secs();
    sim.run(User::Locked, 900);
    assert_eq!(sim.status().state, UsageState::SessionLocked);
    assert!(sim.active_secs() <= before + 2);
    assert!(sim.status().today.locked_secs >= 895);
    sim.run(User::Busy, 60);
    assert!(sim.active_secs() >= before + 55);
}

#[test]
fn s5_s6_sleep_and_wake_are_not_usage() {
    let mut sim = Sim::start(None, default_policy(), T0_WALL);
    sim.run(User::Busy, 300);
    let before = sim.active_secs();
    // Güç olayıyla uyku (servis modu) ve olaysız uyku (konsol / olay kaçırıldı).
    sim.sleep(3600, true);
    assert_eq!(sim.status().today.suspended_secs, 3600);
    sim.sleep(1800, false);
    assert!(sim.status().today.unknown_secs >= 1800);
    assert!(sim.active_secs() <= before + 2, "uyku aktif sayılmadı");
    assert!(
        !sim.engine.clock_adjusted_today(),
        "uyku saat değişikliği sayılmaz"
    );
    // Uyandıktan sonra kullanım devam eder.
    sim.run(User::Busy, 120);
    assert!(sim.active_secs() >= before + 110);
}

#[test]
fn s7_restart_keeps_totals_without_double_counting() {
    let dir = temp_dir("restart");
    let mut sim = Sim::start(Some(&dir), quick_policy(), T0_WALL);
    assert!(sim.status().storage.persistent);
    sim.run(User::Busy, 150);
    let before = sim.active_secs();
    let brk_before = sim.engine.break_state();
    assert!(before >= 145);

    // Yeniden başlatma (ör. sc stop/start), 30 sn kesinti.
    let mut sim = sim.restart(30);
    assert_eq!(
        sim.active_secs(),
        before,
        "kayıtlar korunur, iki kez sayılmaz"
    );
    assert_eq!(
        sim.engine.break_state().active_since_break_ms,
        brk_before.active_since_break_ms
    );
    // Kesinti süresi aktif sayılmaz; yeniden başlatma ve kayıt tekrarlanırsa da değişmez.
    let mut sim = {
        sim.run(User::NoHelper, 5);
        sim.restart(0)
    };
    assert_eq!(sim.active_secs(), before);
    sim.run(User::Busy, 60);
    assert!(sim.active_secs() >= before + 55);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn s8_clock_changes_do_not_create_or_hide_usage() {
    let dir = temp_dir("clock");
    let mut sim = Sim::start(Some(&dir), default_policy(), T0_WALL);
    sim.run(User::Busy, 300);
    let today = sim.status().day.clone();
    let before = sim.active_secs();

    // Saat 1 gün geri alınır: bugünün kullanımı dünün kaydına yazılamaz.
    sim.wall -= 86_400 * S;
    sim.run(User::Busy, 60);
    assert_eq!(sim.status().day, today, "gün geri gitmedi");
    assert!(sim.active_secs() >= before + 55);
    assert!(sim.status().clock_adjusted_today);
    sim.wall += 86_400 * S; // düzeltildi
    sim.run(User::Busy, 10);

    // Saat 1 gün ileri: yeni gün başlar ama işaretlenir; önceki gün kaybolmaz.
    sim.wall += 86_400 * S;
    sim.run(User::Busy, 30);
    let s = sim.status();
    assert_ne!(s.day, today);
    assert!(s.clock_adjusted_today);
    assert!(s.today.active_secs <= 31);
    sim.flush();
    let prev = sim.store.load_day(LocalDate::parse(&today).unwrap());
    assert!(prev.active_ms / 1000 >= before + 60);
    assert!(sim
        .store
        .day_clock_adjusted(LocalDate::parse(&today).unwrap()));
    assert!(sim.store.count_events("CLOCK_JUMP") >= 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn s9_day_rollover_splits_usage_at_midnight() {
    let dir = temp_dir("rollover");
    // Yerel 23:58:00 (UTC+3).
    let wall = T0_WALL + (10 * 3600 + 58 * 60) * S;
    let mut sim = Sim::start(Some(&dir), default_policy(), wall);
    let day1 = sim.status().day.clone();
    sim.run(User::Busy, 240); // 00:02'ye kadar
    let s = sim.status();
    assert_ne!(s.day, day1);
    assert!(
        !s.clock_adjusted_today,
        "gece yarısı saat değişikliği değildir"
    );
    assert!((118..=121).contains(&s.today.active_secs), "{:?}", s.today);
    sim.flush();
    let d1 = sim.store.load_day(LocalDate::parse(&day1).unwrap());
    assert!((115..=120).contains(&(d1.active_ms / 1000)), "{d1:?}");
    assert_eq!(sim.store.count_events("DAY_ROLLOVER"), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn s10_measurement_errors_are_unknown_not_active() {
    let mut sim = Sim::start(None, default_policy(), T0_WALL);
    sim.run(User::Busy, 60);
    let before = sim.active_secs();
    sim.run(User::ApiError, 300);
    let s = sim.status();
    assert_eq!(s.state, UsageState::Unknown);
    assert_eq!(
        s.measurement.unknown_reason.as_deref(),
        Some("IDLE_API_FAILED")
    );
    sim.run(User::NoHelper, 300);
    let s = sim.status();
    assert_eq!(s.measurement.source, MeasurementSource::None);
    assert!(s.today.active_secs <= before + 2);
    assert!(s.today.unknown_secs >= 599);
}

#[test]
fn break_reminders_fire_once_each_and_limit_is_informational() {
    let mut sim = Sim::start(None, quick_policy(), T0_WALL);
    assert_eq!(sim.status().policy.source, PolicySource::DevFile);
    let mut seen: Vec<(String, NoticeKind)> = vec![];
    let mut phases = vec![];
    for _ in 0..900 {
        sim.run(User::Busy, 1);
        let s = sim.status();
        if phases.last() != Some(&s.break_phase) {
            phases.push(s.break_phase);
        }
        for n in s.notices {
            if !seen.iter().any(|(id, _)| *id == n.id) {
                seen.push((n.id, n.kind));
            }
        }
    }
    use BreakPhase::*;
    assert_eq!(
        &phases[..6],
        &[
            Normal,
            FirstWarning,
            FinalWarning,
            BreakDue,
            BreakComplete,
            Normal
        ]
    );
    let kinds: Vec<NoticeKind> = seen.iter().map(|(_, k)| *k).collect();
    assert_eq!(
        &kinds[..4],
        &[
            NoticeKind::BreakFirstWarning,
            NoticeKind::BreakFinalWarning,
            NoticeKind::BreakStarted,
            NoticeKind::BreakCompleted
        ]
    );
    // Her olay benzersiz kimlikle bir kez; ikinci döngü yeni kimlikler alır.
    let ids: std::collections::HashSet<_> = seen.iter().map(|(id, _)| id.clone()).collect();
    assert_eq!(ids.len(), seen.len());
    assert!(seen.iter().any(|(id, _)| id == "b2-due"));
    // 10 dk limit doldu: yalnız bilgilendirme durumu.
    let s = sim.status();
    assert!(s.limit_reached);
    assert_eq!(s.remaining_secs, 0);
    assert!(kinds.contains(&NoticeKind::LimitReached));
    // Limit sonrası ölçüm sürer (kilit yok, sayım durmaz).
    let before = s.today.active_secs;
    sim.run(User::Busy, 30);
    assert!(sim.active_secs() > before);
}

#[test]
fn break_countdown_survives_restart() {
    let dir = temp_dir("break-restart");
    let mut sim = Sim::start(Some(&dir), quick_policy(), T0_WALL);
    sim.run(User::Busy, 200);
    assert_eq!(sim.status().break_phase, BreakPhase::BreakDue);
    let left = sim.status().break_remaining_secs.unwrap();
    let sim = sim.restart(10);
    let s = sim.status();
    assert_eq!(s.break_phase, BreakPhase::BreakDue);
    let after = s.break_remaining_secs.unwrap();
    assert!(after <= left - 9 && after + 12 >= left, "{left} -> {after}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn sqlite_migration_recovery_and_newer_schema() {
    let dir = temp_dir("db");
    // Yeni veritabanı: şema v1, kalıcı.
    let store = Store::open(Some(&dir), "r1", T0_WALL);
    assert_eq!(store.info().schema_version, SCHEMA_VERSION);
    assert!(store.info().persistent);
    assert!(!store.info().recovered);
    drop(store);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join(DB_FILE_NAME))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "veritabanı yalnız sahibine açık");
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0);
    }

    // Bozuk dosya: kenara alınır, yeni veritabanı açılır, olay kaydedilir.
    std::fs::write(
        dir.join(DB_FILE_NAME),
        b"this is not a sqlite database at all.....",
    )
    .unwrap();
    let _ = std::fs::remove_file(dir.join(format!("{DB_FILE_NAME}-wal")));
    let _ = std::fs::remove_file(dir.join(format!("{DB_FILE_NAME}-shm")));
    let mut sim = Sim::start(Some(&dir), default_policy(), T0_WALL);
    assert!(sim.status().storage.persistent);
    assert!(sim.status().storage.recovered);
    sim.flush();
    assert_eq!(sim.store.count_events("DB_RECOVERED"), 1);
    let copies = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("kindora.db.corrupt-")
        })
        .count();
    assert!(copies >= 1);
    drop(sim);

    // Agent'tan yeni şema: dosyaya dokunulmaz, bellek içi çalışılır.
    {
        let conn = rusqlite::Connection::open(dir.join(DB_FILE_NAME)).unwrap();
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();
    }
    let store = Store::open(Some(&dir), "r3", T0_WALL);
    assert!(!store.info().persistent);
    assert_eq!(store.info().schema_version, 99);
    drop(store);
    let conn = rusqlite::Connection::open(dir.join(DB_FILE_NAME)).unwrap();
    let v: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 99, "yeni şema bozulmadı");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn policy_is_versioned_in_sqlite_and_corrupt_file_keeps_last_valid() {
    let dir = temp_dir("policy");
    let mut store = Store::open(Some(&dir), "p", T0_WALL);
    let applied = quick_policy();
    store
        .save_policy(applied.store.as_ref().unwrap(), T0_WALL)
        .unwrap();
    drop(store);
    let store = Store::open(Some(&dir), "p2", T0_WALL);
    let stored = store.load_policy().expect("son geçerli politika saklandı");
    assert_eq!(stored.policy.break_interval_secs, 180);
    let r = resolve(&DevFileRead::Content(b"{broken".to_vec()), Some(&stored));
    assert_eq!(r.info.status, PolicyStatus::InvalidKeptLast);
    assert_eq!(r.active.break_interval_secs, 180);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn engine_status_fits_ipc_message_limit() {
    let mut sim = Sim::start(None, quick_policy(), T0_WALL);
    sim.run(User::Busy, 700);
    let mut s = sim.status();
    s.measurement.unknown_reason = Some("X".repeat(64));
    let bytes = ipc_protocol::Response::ok("m-1", ipc_protocol::Payload::ScreenTime(Box::new(s)))
        .to_bytes();
    assert!(bytes.len() < 2048, "{}", bytes.len());
}
