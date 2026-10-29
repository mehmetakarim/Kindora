//! SQLite kalıcılığı (P1.C, ADR-0014).
//!
//! - Veritabanına **yalnız agent** erişir; helper ve masaüstü SQLite'ı açmaz,
//!   arayüz yalnız salt okunur `GET_SCREEN_TIME` ile okur.
//! - Klasör ve dosya izinleri çağıran tarafından hazırlanır (Windows: korumalı
//!   DACL; Unix geliştirme: 0700/0600).
//! - Sürümlü migration: `PRAGMA user_version`. Agent'tan yeni bir şema
//!   bulunursa dosyaya dokunulmaz; sayaçlar bellekte tutulur.
//! - Bozuk dosya (`NOTADB`/`CORRUPT`/`quick_check` hatası) kenara alınır
//!   (`kindora.db.corrupt-<ms>`, en çok 3 kopya) ve yeni veritabanı açılır.
//! - Tüm yazmalar tek transaction ile yapılır; bir parça yalnız bir kez
//!   eklenir, böylece yeniden başlatma süreyi iki kez saymaz. Son kayıttan
//!   sonraki en fazla bir kayıt aralığı kaybolabilir (eksik sayım yönünde).

use crate::breaks::BreakState;
use crate::calendar::LocalDate;
use crate::engine::{DayUsageMs, LifecycleEvent, Piece};
use crate::policy::{Policy, StoredPolicy};
use ipc_protocol::{BreakPhase, PolicySource, StorageInfo, UsageState};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const DB_FILE_NAME: &str = "kindora.db";
pub const SCHEMA_VERSION: u32 = 1;
const MAX_CORRUPT_COPIES: usize = 3;
/// Saklama: ayrıntılı parçalar 14 gün, günlük özetler 90 gün, olaylar son 5000.
pub const SEGMENT_RETENTION_DAYS: i64 = 14;
pub const DAILY_RETENTION_DAYS: i64 = 90;
pub const MAX_LIFECYCLE_ROWS: i64 = 5000;
/// Aynı durumdaki ardışık parçalar bu aralıktan yakınsa tek satırda birleşir.
const SEGMENT_MERGE_GAP_MS: u64 = 2_500;

const MIGRATIONS: [&str; 1] = [
    // v1
    "
    CREATE TABLE daily_usage (
        day TEXT PRIMARY KEY CHECK (length(day) = 10),
        active_ms INTEGER NOT NULL DEFAULT 0 CHECK (active_ms >= 0),
        idle_ms INTEGER NOT NULL DEFAULT 0 CHECK (idle_ms >= 0),
        locked_ms INTEGER NOT NULL DEFAULT 0 CHECK (locked_ms >= 0),
        suspended_ms INTEGER NOT NULL DEFAULT 0 CHECK (suspended_ms >= 0),
        unknown_ms INTEGER NOT NULL DEFAULT 0 CHECK (unknown_ms >= 0),
        clock_adjusted INTEGER NOT NULL DEFAULT 0 CHECK (clock_adjusted IN (0, 1)),
        updated_wall_ms INTEGER NOT NULL
    );
    CREATE TABLE usage_segments (
        id INTEGER PRIMARY KEY,
        run_id TEXT NOT NULL,
        day TEXT NOT NULL,
        state TEXT NOT NULL CHECK (state IN ('ACTIVE','IDLE','SESSION_LOCKED','SUSPENDED','UNKNOWN')),
        start_wall_ms INTEGER NOT NULL,
        end_wall_ms INTEGER NOT NULL,
        duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0)
    );
    CREATE INDEX usage_segments_day ON usage_segments(day);
    CREATE TABLE policy_state (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        version INTEGER NOT NULL,
        source TEXT NOT NULL,
        policy_json TEXT NOT NULL,
        applied_wall_ms INTEGER NOT NULL
    );
    CREATE TABLE break_state (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        phase TEXT NOT NULL,
        active_since_break_ms INTEGER NOT NULL,
        due_elapsed_ms INTEGER NOT NULL,
        complete_elapsed_ms INTEGER NOT NULL,
        active_during_break_ms INTEGER NOT NULL,
        inactive_run_ms INTEGER NOT NULL,
        seq INTEGER NOT NULL,
        saved_wall_ms INTEGER NOT NULL
    );
    CREATE TABLE lifecycle_events (
        id INTEGER PRIMARY KEY,
        wall_ms INTEGER NOT NULL,
        run_id TEXT NOT NULL,
        kind TEXT NOT NULL,
        detail TEXT NOT NULL DEFAULT ''
    );
    ",
];

#[derive(Debug)]
enum OpenError {
    Corrupt(String),
    NewerSchema(u32),
    Other(String),
}

fn classify(e: rusqlite::Error) -> OpenError {
    use rusqlite::ErrorCode::{DatabaseCorrupt, NotADatabase};
    match &e {
        rusqlite::Error::SqliteFailure(f, _)
            if matches!(f.code, NotADatabase | DatabaseCorrupt) =>
        {
            OpenError::Corrupt(e.to_string())
        }
        _ => OpenError::Other(e.to_string()),
    }
}

struct OpenSegment {
    rowid: i64,
    day: LocalDate,
    state: UsageState,
    end_wall_ms: u64,
}

pub struct Store {
    conn: Connection,
    info: StorageInfo,
    run_id: String,
    path: Option<PathBuf>,
    open_segment: Option<OpenSegment>,
    /// Açılış sırasında oluşan, kayda geçirilecek olaylar (ör. kurtarma).
    startup_notes: Vec<(&'static str, String)>,
}

fn phase_str(p: BreakPhase) -> &'static str {
    match p {
        BreakPhase::Normal => "NORMAL",
        BreakPhase::FirstWarning => "FIRST_WARNING",
        BreakPhase::FinalWarning => "FINAL_WARNING",
        BreakPhase::BreakDue => "BREAK_DUE",
        BreakPhase::BreakComplete => "BREAK_COMPLETE",
    }
}

fn parse_phase(s: &str) -> Option<BreakPhase> {
    Some(match s {
        "NORMAL" => BreakPhase::Normal,
        "FIRST_WARNING" => BreakPhase::FirstWarning,
        "FINAL_WARNING" => BreakPhase::FinalWarning,
        "BREAK_DUE" => BreakPhase::BreakDue,
        "BREAK_COMPLETE" => BreakPhase::BreakComplete,
        _ => return None,
    })
}

fn source_str(s: PolicySource) -> &'static str {
    match s {
        PolicySource::BuiltinDefault => "BUILTIN_DEFAULT",
        PolicySource::DevFile => "DEV_FILE",
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalPolicy {
    version: u32,
    daily_limit_secs: u32,
    break_interval_secs: u32,
    break_duration_secs: u32,
    first_warning_secs: u32,
    final_warning_secs: u32,
    idle_threshold_secs: u32,
}

fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

fn to_u64(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    // WAL: yazma sırasında çökme dosyayı bozmaz; NORMAL: tutarlı, son
    // işlem güç kaybında kaybolabilir (eksik sayım yönünde).
    let _: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
    Ok(())
}

fn migrate(conn: &mut Connection) -> Result<u32, OpenError> {
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(classify)?;
    if version > SCHEMA_VERSION {
        return Err(OpenError::NewerSchema(version));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let next = i as u32 + 1;
        let tx = conn.transaction().map_err(classify)?;
        tx.execute_batch(sql).map_err(classify)?;
        tx.execute_batch(&format!("PRAGMA user_version = {next};"))
            .map_err(classify)?;
        tx.commit().map_err(classify)?;
    }
    Ok(SCHEMA_VERSION)
}

fn try_open(path: &Path) -> Result<Connection, OpenError> {
    let mut conn = Connection::open(path).map_err(classify)?;
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(classify)?;
    if check != "ok" {
        return Err(OpenError::Corrupt(format!("quick_check: {check}")));
    }
    configure(&conn).map_err(classify)?;
    migrate(&mut conn)?;
    Ok(conn)
}

#[cfg(unix)]
fn restrict(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn restrict(_path: &Path, _mode: u32) {}

/// Bozuk dosyayı (ve WAL/SHM eşlerini) kenara alır; eski kopyaları sınırlar.
fn quarantine(path: &Path, wall_ms: u64) -> std::io::Result<PathBuf> {
    let target = path.with_file_name(format!("{DB_FILE_NAME}.corrupt-{wall_ms}"));
    std::fs::rename(path, &target)?;
    for suffix in ["-wal", "-shm"] {
        let side = path.with_file_name(format!("{DB_FILE_NAME}{suffix}"));
        if side.exists() {
            let _ = std::fs::rename(
                &side,
                target.with_file_name(format!("{DB_FILE_NAME}.corrupt-{wall_ms}{suffix}")),
            );
        }
    }
    if let Some(dir) = path.parent() {
        let mut copies: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&format!("{DB_FILE_NAME}.corrupt-")))
            })
            .collect();
        copies.sort();
        let excess = copies.len().saturating_sub(MAX_CORRUPT_COPIES * 3);
        for old in copies.into_iter().take(excess) {
            let _ = std::fs::remove_file(old);
        }
    }
    Ok(target)
}

impl Store {
    fn memory(run_id: &str, info: StorageInfo, notes: Vec<(&'static str, String)>) -> Store {
        let mut conn = Connection::open_in_memory().expect("in-memory sqlite");
        let _ = migrate(&mut conn);
        Store {
            conn,
            info,
            run_id: run_id.to_owned(),
            path: None,
            open_segment: None,
            startup_notes: notes,
        }
    }

    /// Kalıcı depo açar; açılamazsa bellek içi depoya düşer (asla panik/çıkış yok).
    /// `dir = None`: bilinçli olarak bellek içi (testler).
    pub fn open(dir: Option<&Path>, run_id: &str, wall_ms: u64) -> Store {
        let Some(dir) = dir else {
            return Store::memory(
                run_id,
                StorageInfo {
                    persistent: false,
                    schema_version: SCHEMA_VERSION,
                    recovered: false,
                },
                vec![],
            );
        };
        if let Err(e) = std::fs::create_dir_all(dir) {
            return Store::memory(
                run_id,
                StorageInfo {
                    persistent: false,
                    schema_version: SCHEMA_VERSION,
                    recovered: false,
                },
                vec![("DB_UNAVAILABLE", format!("create_dir: {e}"))],
            );
        }
        restrict(dir, 0o700);
        let path = dir.join(DB_FILE_NAME);
        let mut notes = vec![];
        let mut recovered = false;
        let result = match try_open(&path) {
            Err(OpenError::Corrupt(reason)) => match quarantine(&path, wall_ms) {
                Ok(moved) => {
                    recovered = true;
                    notes.push((
                        "DB_RECOVERED",
                        format!("reason={reason:?} moved_to={}", moved.display()),
                    ));
                    try_open(&path)
                }
                Err(e) => Err(OpenError::Other(format!("quarantine failed: {e}"))),
            },
            other => other,
        };
        match result {
            Ok(conn) => {
                restrict(&path, 0o600);
                let mut store = Store {
                    conn,
                    info: StorageInfo {
                        persistent: true,
                        schema_version: SCHEMA_VERSION,
                        recovered,
                    },
                    run_id: run_id.to_owned(),
                    path: Some(path),
                    open_segment: None,
                    startup_notes: notes,
                };
                let _ = store.prune(None);
                store
            }
            Err(OpenError::NewerSchema(v)) => {
                notes.push((
                    "DB_NEWER_SCHEMA",
                    format!("found={v} supported={SCHEMA_VERSION}"),
                ));
                Store::memory(
                    run_id,
                    StorageInfo {
                        persistent: false,
                        schema_version: v,
                        recovered,
                    },
                    notes,
                )
            }
            Err(OpenError::Corrupt(e) | OpenError::Other(e)) => {
                notes.push(("DB_UNAVAILABLE", e));
                Store::memory(
                    run_id,
                    StorageInfo {
                        persistent: false,
                        schema_version: SCHEMA_VERSION,
                        recovered,
                    },
                    notes,
                )
            }
        }
    }

    pub fn info(&self) -> &StorageInfo {
        &self.info
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Açılışta oluşan notlar (bir kez alınır).
    pub fn take_startup_notes(&mut self) -> Vec<(&'static str, String)> {
        std::mem::take(&mut self.startup_notes)
    }

    pub fn load_day(&self, day: LocalDate) -> DayUsageMs {
        self.conn
            .query_row(
                "SELECT active_ms, idle_ms, locked_ms, suspended_ms, unknown_ms FROM daily_usage WHERE day = ?1",
                [day.to_string()],
                |r| {
                    Ok(DayUsageMs {
                        active_ms: to_u64(r.get(0)?),
                        idle_ms: to_u64(r.get(1)?),
                        locked_ms: to_u64(r.get(2)?),
                        suspended_ms: to_u64(r.get(3)?),
                        unknown_ms: to_u64(r.get(4)?),
                    })
                },
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub fn day_clock_adjusted(&self, day: LocalDate) -> bool {
        self.conn
            .query_row(
                "SELECT clock_adjusted FROM daily_usage WHERE day = ?1",
                [day.to_string()],
                |r| r.get::<_, i64>(0),
            )
            .map(|v| v == 1)
            .unwrap_or(false)
    }

    /// Son mola durumu ve kaydedildiği duvar saati.
    pub fn load_break(&self) -> Option<(BreakState, u64)> {
        self.conn
            .query_row(
                "SELECT phase, active_since_break_ms, due_elapsed_ms, complete_elapsed_ms,
                        active_during_break_ms, inactive_run_ms, seq, saved_wall_ms
                 FROM break_state WHERE id = 1",
                [],
                |r| {
                    let phase: String = r.get(0)?;
                    Ok((
                        phase,
                        BreakState {
                            phase: BreakPhase::Normal,
                            active_since_break_ms: to_u64(r.get(1)?),
                            due_elapsed_ms: to_u64(r.get(2)?),
                            complete_elapsed_ms: to_u64(r.get(3)?),
                            active_during_break_ms: to_u64(r.get(4)?),
                            inactive_run_ms: to_u64(r.get(5)?),
                            seq: to_u64(r.get(6)?),
                        },
                        to_u64(r.get(7)?),
                    ))
                },
            )
            .optional()
            .ok()
            .flatten()
            .and_then(|(phase, mut b, saved)| {
                b.phase = parse_phase(&phase)?;
                Some((b, saved))
            })
    }

    /// Son geçerli politika. Saklanan kayıt geçersizse yok sayılır.
    pub fn load_policy(&self) -> Option<StoredPolicy> {
        let (json, source): (String, String) = self
            .conn
            .query_row(
                "SELECT policy_json, source FROM policy_state WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .ok()
            .flatten()?;
        let c: CanonicalPolicy = serde_json::from_str(&json).ok()?;
        let policy = Policy {
            version: c.version,
            daily_limit_secs: c.daily_limit_secs,
            break_interval_secs: c.break_interval_secs,
            break_duration_secs: c.break_duration_secs,
            first_warning_secs: c.first_warning_secs,
            final_warning_secs: c.final_warning_secs,
            idle_threshold_secs: c.idle_threshold_secs,
        };
        policy.validate().ok()?;
        let source = match source.as_str() {
            "DEV_FILE" => PolicySource::DevFile,
            _ => return None,
        };
        Some(StoredPolicy { policy, source })
    }

    pub fn save_policy(&mut self, p: &StoredPolicy, wall_ms: u64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO policy_state (id, version, source, policy_json, applied_wall_ms)
             VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET version = excluded.version, source = excluded.source,
                 policy_json = excluded.policy_json, applied_wall_ms = excluded.applied_wall_ms",
            params![
                p.policy.version,
                source_str(p.source),
                p.policy.canonical_json(),
                to_i64(wall_ms)
            ],
        )?;
        Ok(())
    }

    /// Parçaları, olayları ve mola durumunu tek transaction'da yazar.
    pub fn flush(
        &mut self,
        pieces: &[Piece],
        events: &[LifecycleEvent],
        brk: &BreakState,
        clock_adjusted_day: Option<LocalDate>,
        wall_ms: u64,
    ) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        let mut open = self.open_segment.take();
        for p in pieces {
            let col = match p.state {
                UsageState::Active => "active_ms",
                UsageState::Idle => "idle_ms",
                UsageState::SessionLocked => "locked_ms",
                UsageState::Suspended => "suspended_ms",
                UsageState::Unknown => "unknown_ms",
            };
            tx.execute(
                &format!(
                    "INSERT INTO daily_usage (day, {col}, updated_wall_ms) VALUES (?1, ?2, ?3)
                     ON CONFLICT(day) DO UPDATE SET {col} = {col} + excluded.{col},
                         updated_wall_ms = excluded.updated_wall_ms"
                ),
                params![p.day.to_string(), to_i64(p.ms), to_i64(wall_ms)],
            )?;
            let merge = open.as_ref().is_some_and(|o| {
                o.day == p.day
                    && o.state == p.state
                    && p.start_wall_ms.abs_diff(o.end_wall_ms) <= SEGMENT_MERGE_GAP_MS
            });
            if merge {
                let o = open.as_mut().expect("checked");
                tx.execute(
                    "UPDATE usage_segments SET end_wall_ms = ?1, duration_ms = duration_ms + ?2 WHERE id = ?3",
                    params![to_i64(p.end_wall_ms), to_i64(p.ms), o.rowid],
                )?;
                o.end_wall_ms = p.end_wall_ms;
            } else {
                tx.execute(
                    "INSERT INTO usage_segments (run_id, day, state, start_wall_ms, end_wall_ms, duration_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        self.run_id,
                        p.day.to_string(),
                        p.state.as_str(),
                        to_i64(p.start_wall_ms),
                        to_i64(p.end_wall_ms),
                        to_i64(p.ms)
                    ],
                )?;
                open = Some(OpenSegment {
                    rowid: tx.last_insert_rowid(),
                    day: p.day,
                    state: p.state,
                    end_wall_ms: p.end_wall_ms,
                });
            }
        }
        for e in events {
            tx.execute(
                "INSERT INTO lifecycle_events (wall_ms, run_id, kind, detail) VALUES (?1, ?2, ?3, ?4)",
                params![to_i64(e.wall_ms), self.run_id, e.kind, e.detail],
            )?;
        }
        if let Some(day) = clock_adjusted_day {
            tx.execute(
                "INSERT INTO daily_usage (day, clock_adjusted, updated_wall_ms) VALUES (?1, 1, ?2)
                 ON CONFLICT(day) DO UPDATE SET clock_adjusted = 1",
                params![day.to_string(), to_i64(wall_ms)],
            )?;
        }
        tx.execute(
            "INSERT INTO break_state (id, phase, active_since_break_ms, due_elapsed_ms, complete_elapsed_ms,
                 active_during_break_ms, inactive_run_ms, seq, saved_wall_ms)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET phase = excluded.phase,
                 active_since_break_ms = excluded.active_since_break_ms,
                 due_elapsed_ms = excluded.due_elapsed_ms,
                 complete_elapsed_ms = excluded.complete_elapsed_ms,
                 active_during_break_ms = excluded.active_during_break_ms,
                 inactive_run_ms = excluded.inactive_run_ms,
                 seq = excluded.seq, saved_wall_ms = excluded.saved_wall_ms",
            params![
                phase_str(brk.phase),
                to_i64(brk.active_since_break_ms),
                to_i64(brk.due_elapsed_ms),
                to_i64(brk.complete_elapsed_ms),
                to_i64(brk.active_during_break_ms),
                to_i64(brk.inactive_run_ms),
                to_i64(brk.seq),
                to_i64(wall_ms)
            ],
        )?;
        tx.commit()?;
        self.open_segment = open;
        Ok(())
    }

    /// Saklama süresi dolmuş kayıtları siler. `today = None`: yalnız olay sınırı.
    pub fn prune(&mut self, today: Option<LocalDate>) -> rusqlite::Result<usize> {
        let mut n = self.conn.execute(
            "DELETE FROM lifecycle_events WHERE id <= (SELECT MAX(id) FROM lifecycle_events) - ?1",
            [MAX_LIFECYCLE_ROWS],
        )?;
        if let Some(today) = today {
            n += self.conn.execute(
                "DELETE FROM usage_segments WHERE day < ?1",
                [today.add_days(-SEGMENT_RETENTION_DAYS).to_string()],
            )?;
            n += self.conn.execute(
                "DELETE FROM daily_usage WHERE day < ?1",
                [today.add_days(-DAILY_RETENTION_DAYS).to_string()],
            )?;
            self.open_segment = None;
        }
        Ok(n)
    }

    /// Teşhis/test: belirli türdeki olay sayısı.
    pub fn count_events(&self, kind: &str) -> u64 {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM lifecycle_events WHERE kind = ?1",
                [kind],
                |r| r.get::<_, i64>(0),
            )
            .map(to_u64)
            .unwrap_or(0)
    }

    pub fn count_segments(&self) -> u64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM usage_segments", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(to_u64)
            .unwrap_or(0)
    }
}
