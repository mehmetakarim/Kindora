//! Kullanım süresi motoru (P1.C) — gerçek `kindora-agent` (console) ve
//! `kindora-helper` süreçleriyle uçtan uca.
//!
//! Sınanan: helper köprüsünün `GET_SCREEN_TIME` verisini taşıması, DEV
//! politika dosyasının agent tarafında doğrulanması ve sürümlenmesi (geçerli /
//! bozuk / geri alma), SQLite kayıtlarının yeniden başlatmada korunması ve
//! sürelerin iki kez sayılmaması.
//!
//! DOĞRULAMAZ: gerçek kullanıcı etkinliği (CI'da klavye/fare yok), oturum
//! kilidi, uyku/uyanma, LocalService servisi (CI betiği ayrıca dener). Bu
//! durumlar `screen-time` crate'inin benzetim testlerinde ve Lenovo kabulünde.

use ipc_protocol::bridge::{decode_bridge_event, BridgeEvent};
use ipc_protocol::{decode_response, Payload, PolicySource, PolicyStatus, ScreenTimeStatus};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn agent_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_kindora-agent"))
}

fn helper_bin() -> PathBuf {
    let p = agent_bin().with_file_name(format!("kindora-helper{}", std::env::consts::EXE_SUFFIX));
    assert!(
        p.exists(),
        "önce `cargo build --workspace`: {}",
        p.display()
    );
    p
}

#[cfg(unix)]
fn transport_args(dir: &Path) -> Vec<String> {
    vec!["--socket".into(), dir.join("a.sock").display().to_string()]
}

#[cfg(windows)]
fn transport_args(_dir: &Path) -> Vec<String> {
    Vec::new()
}

fn policy_json(version: u32, interval: u32) -> String {
    format!(
        r#"{{"kind":"kindora-dev-policy","dev_only":true,"version":{version},
            "daily_limit_secs":600,"break_interval_secs":{interval},"break_duration_secs":30,
            "first_warning_secs":60,"final_warning_secs":20,"idle_threshold_secs":60,
            "note":"CI e2e"}}"#
    )
}

struct Agent(Child);

impl Agent {
    fn start(dir: &Path) -> Agent {
        let mut args = vec!["console".to_string()];
        args.extend(transport_args(dir));
        args.extend([
            "--data-dir".into(),
            dir.join("data").display().to_string(),
            "--dev-policy".into(),
            dir.join("policy.json").display().to_string(),
        ]);
        let child = Command::new(agent_bin())
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("agent spawn");
        Agent(child)
    }

    fn stop(mut self) {
        #[cfg(unix)]
        {
            // SAFETY: kendi alt sürecimize sinyal; SIGINT → düzgün kapanış + son kayıt.
            unsafe { libc::kill(self.0.id() as i32, libc::SIGINT) };
        }
        #[cfg(windows)]
        {
            // Konsola Ctrl+C gönderilemez; sonlandırma = beklenmedik kapanış
            // (son kayıttan sonraki ≤15 sn kaybolabilir, iki kez sayılmaz).
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

struct Bridge {
    child: Child,
    stdin: Option<ChildStdin>,
    events: Arc<Mutex<Vec<BridgeEvent>>>,
}

impl Bridge {
    fn spawn(dir: &Path) -> Bridge {
        let mut args = vec!["bridge".to_string()];
        args.extend(transport_args(dir));
        let mut child = Command::new(helper_bin())
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("bridge spawn");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                sink.lock()
                    .unwrap()
                    .push(decode_bridge_event(&line).expect("valid event"));
            }
        });
        Bridge {
            child,
            stdin,
            events,
        }
    }

    fn wait_screen_time(
        &self,
        what: &str,
        timeout: Duration,
        pred: impl Fn(&ScreenTimeStatus) -> bool,
    ) -> ScreenTimeStatus {
        let t = Instant::now();
        while t.elapsed() < timeout {
            let found = self
                .events
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find_map(|e| match e {
                    BridgeEvent::Snapshot(s) => s.screen_time.clone().filter(|st| pred(st)),
                    _ => None,
                });
            if let Some(s) = found {
                return *s;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let last = self
            .events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|e| match e {
                BridgeEvent::Snapshot(s) => Some(s.clone()),
                _ => None,
            });
        panic!("{what}: beklenen kullanım süresi durumu gelmedi; son: {last:#?}");
    }

    fn close(mut self) {
        drop(self.stdin.take());
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(5) {
            if self.child.try_wait().unwrap().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        panic!("köprü kapanmadı");
    }
}

fn one_shot_screen_time(dir: &Path) -> ScreenTimeStatus {
    let mut args = vec!["screen-time".to_string()];
    args.extend(transport_args(dir));
    let t = Instant::now();
    loop {
        let out = Command::new(helper_bin())
            .args(&args)
            .output()
            .expect("helper");
        if out.status.success() {
            let resp = decode_response(&out.stdout).expect("json yanıt");
            match resp.payload {
                Some(Payload::ScreenTime(s)) => return *s,
                other => panic!("beklenmeyen yanıt: {other:?}"),
            }
        }
        assert!(t.elapsed() < Duration::from_secs(15), "agent yanıt vermedi");
        thread::sleep(Duration::from_millis(200));
    }
}

fn total_secs(s: &ScreenTimeStatus) -> u64 {
    s.today.active_secs
        + s.today.idle_secs
        + s.today.locked_secs
        + s.today.suspended_secs
        + s.today.unknown_secs
}

fn count_events(db: &Path, kind: &str) -> i64 {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM lifecycle_events WHERE kind = ?1",
        [kind],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn screen_time_policy_persistence_and_restart() {
    let dir = std::env::temp_dir().join(format!("ks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let policy = dir.join("policy.json");
    std::fs::write(&policy, policy_json(1, 120)).unwrap();

    // 1. Agent + köprü: kullanım süresi verisi masaüstüne ulaşır.
    let agent = Agent::start(&dir);
    let bridge = Bridge::spawn(&dir);
    let st = bridge.wait_screen_time("başlangıç", Duration::from_secs(20), |s| {
        s.policy.source == PolicySource::DevFile && s.storage.persistent
    });
    eprintln!(
        "ölçüm kaynağı: {:?} durum: {:?} neden: {:?}",
        st.measurement.source, st.state, st.measurement.unknown_reason
    );
    assert_eq!(st.policy.version, 1);
    assert_eq!(st.policy.break_interval_secs, 120);
    assert_eq!(st.daily_limit_secs, 600);
    assert!(st.remaining_secs <= 600);
    assert!(!st.limit_reached || st.today.active_secs >= 600);

    // 2. Bozuk politika: son geçerli korunur.
    std::fs::write(&policy, b"{ bozuk").unwrap();
    let st = bridge.wait_screen_time("bozuk politika", Duration::from_secs(15), |s| {
        s.policy.status == PolicyStatus::InvalidKeptLast
    });
    assert_eq!(st.policy.break_interval_secs, 120);

    // 3. Aynı sürümde farklı içerik: geri alma reddedilir.
    std::fs::write(&policy, policy_json(1, 300)).unwrap();
    bridge.wait_screen_time("sürüm reddi", Duration::from_secs(15), |s| {
        s.policy.status == PolicyStatus::VersionRejected && s.policy.break_interval_secs == 120
    });

    // 4. Yeni sürüm uygulanır.
    std::fs::write(&policy, policy_json(2, 300)).unwrap();
    let st = bridge.wait_screen_time("yeni sürüm", Duration::from_secs(15), |s| {
        s.policy.status == PolicyStatus::Ok && s.policy.version == 2
    });
    assert_eq!(st.policy.break_interval_secs, 300);
    // Kayıt aralığından fazla bekle: süreler SQLite'a yazılsın.
    thread::sleep(Duration::from_secs(16));
    let before = one_shot_screen_time(&dir);
    bridge.close();
    agent.stop();

    let db = dir.join("data").join("kindora.db");
    assert!(db.exists(), "SQLite dosyası oluşmadı");
    assert_eq!(count_events(&db, "AGENT_START"), 1);
    assert_eq!(count_events(&db, "POLICY_APPLIED"), 2);
    assert!(count_events(&db, "POLICY_INVALID") >= 1);
    assert!(count_events(&db, "POLICY_VERSION_REJECTED") >= 1);

    // 5. Yeniden başlatma: kayıtlar korunur, iki kez sayılmaz, politika sürümü kalır.
    let agent = Agent::start(&dir);
    let after = one_shot_screen_time(&dir);
    assert_eq!(after.day, before.day);
    assert_eq!(after.policy.version, 2);
    assert!(after.storage.persistent);
    let (b, a) = (total_secs(&before), total_secs(&after));
    assert!(a + 16 >= b, "kayıtlar kayboldu: önce {b} sn, sonra {a} sn");
    assert!(
        a <= b + 10,
        "süre iki kez sayıldı: önce {b} sn, sonra {a} sn"
    );
    assert!(after.today.active_secs <= before.today.active_secs + 5);
    agent.stop();
    assert_eq!(count_events(&db, "AGENT_START"), 2);
    // Politika dosyası değişmediği için ikinci açılış yeniden uygulamaz.
    assert_eq!(count_events(&db, "POLICY_APPLIED"), 2);

    let _ = std::fs::remove_dir_all(&dir);
}
