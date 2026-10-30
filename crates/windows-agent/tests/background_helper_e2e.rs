//! Bağımsız arka plan helper'ı (P1.D, ADR-0015) — gerçek `kindora-agent`
//! (console) ve `kindora-helper` süreçleriyle uçtan uca.
//!
//! Sınanan: `activity` modunun masaüstünden bağımsız raporlaması, oturum
//! başına tek örnek, agent'ın tek raporlayıcı kuralı (köprü ve tek seferlik
//! rapor reddedilir), köprünün yedek raporlayıcılığı ve arka plan helper'ının
//! onu devralması, helper'ın sonlandırılması ve yeniden başlaması (temiz
//! olmayan çıkış izi), agent yeniden başlatmasında yeniden bağlanma ve
//! sürelerin iki kez sayılmaması.
//!
//! DOĞRULAMAZ: Windows oturum açılışı görevi, oturum kapatma, kullanıcı
//! değiştirme, uyku (CI betiği ve Lenovo kabulü).

use ipc_protocol::bridge::{decode_bridge_event, BridgeEvent};
use ipc_protocol::{decode_response, Payload, ReporterKind, ScreenTimeStatus};
use std::io::{BufRead, BufReader, Read};
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

/// stderr'i arka planda toplanan süreç.
struct Proc {
    child: Child,
    stdin: Option<ChildStdin>,
    err: Arc<Mutex<String>>,
    events: Arc<Mutex<Vec<BridgeEvent>>>,
}

impl Proc {
    fn spawn(bin: &Path, args: &[String], bridge: bool) -> Proc {
        let mut child = Command::new(bin)
            .args(args)
            .stdin(if bridge {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(if bridge {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let err = Arc::new(Mutex::new(String::new()));
        let sink = err.clone();
        let mut stderr = child.stderr.take().unwrap();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut buf) {
                if n == 0 {
                    break;
                }
                sink.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
        let events = Arc::new(Mutex::new(Vec::new()));
        if bridge {
            let out = child.stdout.take().unwrap();
            let sink = events.clone();
            thread::spawn(move || {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    sink.lock()
                        .unwrap()
                        .push(decode_bridge_event(&line).expect("event"));
                }
            });
        }
        let stdin = child.stdin.take();
        Proc {
            child,
            stdin,
            err,
            events,
        }
    }

    fn count(&self, needle: &str) -> usize {
        self.err.lock().unwrap().matches(needle).count()
    }

    fn wait_for(&self, needle: &str, n: usize, timeout: Duration) {
        let t = Instant::now();
        while self.count(needle) < n {
            assert!(
                t.elapsed() < timeout,
                "'{needle}' x{n} bekleniyordu:\n{}",
                self.err.lock().unwrap()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn wait_exit(&mut self, timeout: Duration) -> std::process::ExitStatus {
        let t = Instant::now();
        loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                return s;
            }
            assert!(t.elapsed() < timeout, "süreç çıkmadı");
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Görev Yöneticisi'nden "görevi sonlandır" eşdeğeri (temiz çıkış yok).
    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        drop(self.stdin.take());
        if self.child.try_wait().ok().flatten().is_none() {
            self.kill();
        }
    }
}

fn start_agent(dir: &Path) -> Proc {
    let mut args = vec!["console".to_string()];
    args.extend(transport_args(dir));
    args.extend(["--data-dir".into(), dir.join("data").display().to_string()]);
    let p = Proc::spawn(&agent_bin(), &args, false);
    p.wait_for("event=listening", 1, Duration::from_secs(10));
    p
}

fn start_activity(dir: &Path, log: &Path) -> Proc {
    let mut args = vec![
        "activity".to_string(),
        "--launch".into(),
        "manual".into(),
        "--interval-ms".into(),
        "200".into(),
        "--log-file".into(),
        log.display().to_string(),
    ];
    args.extend(transport_args(dir));
    Proc::spawn(&helper_bin(), &args, false)
}

fn one_shot(dir: &Path, cmd: &str) -> Option<Payload> {
    let mut args = vec![cmd.to_string()];
    args.extend(transport_args(dir));
    let out = Command::new(helper_bin()).args(&args).output().unwrap();
    decode_response(&out.stdout).ok().and_then(|r| r.payload)
}

fn screen_time(dir: &Path) -> ScreenTimeStatus {
    let t = Instant::now();
    loop {
        if let Some(Payload::ScreenTime(s)) = one_shot(dir, "screen-time") {
            return *s;
        }
        assert!(t.elapsed() < Duration::from_secs(15), "agent yanıt vermedi");
        thread::sleep(Duration::from_millis(200));
    }
}

fn wait_status(
    dir: &Path,
    what: &str,
    pred: impl Fn(&ScreenTimeStatus) -> bool,
) -> ScreenTimeStatus {
    let t = Instant::now();
    loop {
        let s = screen_time(dir);
        if pred(&s) {
            return s;
        }
        assert!(
            t.elapsed() < Duration::from_secs(15),
            "{what}: son durum {:#?}",
            s.measurement
        );
        thread::sleep(Duration::from_millis(150));
    }
}

fn reporter_kind(s: &ScreenTimeStatus) -> Option<ReporterKind> {
    s.measurement.reporter.as_ref().and_then(|r| r.kind)
}

fn reporter_pid(s: &ScreenTimeStatus) -> Option<u32> {
    s.measurement.reporter.as_ref().and_then(|r| r.pid)
}

/// Windows'ta PID işletim sisteminden gelir; Unix geliştirme taşıması PID vermez.
fn assert_reporter_pid(s: &ScreenTimeStatus, pid: u32) {
    if cfg!(windows) {
        assert_eq!(reporter_pid(s), Some(pid));
    } else {
        assert!(reporter_pid(s).is_none_or(|p| p == pid));
    }
}

fn total(s: &ScreenTimeStatus) -> u64 {
    let t = &s.today;
    t.active_secs + t.idle_secs + t.locked_secs + t.suspended_secs + t.unknown_secs
}

#[test]
fn background_helper_single_reporter_crash_and_agent_restart() {
    let dir = std::env::temp_dir().join(format!("kbg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("helper").join("activity.log");
    let agent = start_agent(&dir);

    // 1. Arka plan helper'ı masaüstü olmadan raporlar ve tek raporlayıcı olur.
    let mut bg = start_activity(&dir, &log);
    bg.wait_for("event=reporter_active", 1, Duration::from_secs(10));
    let bg_pid = bg.child.id();
    let st = wait_status(&dir, "arka plan raporlayıcı", |s| {
        reporter_kind(s) == Some(ReporterKind::BackgroundManual)
    });
    assert_reporter_pid(&st, bg_pid);
    eprintln!(
        "MEASURE bg_reporter source={:?} state={:?} reason={:?}",
        st.measurement.source, st.state, st.measurement.unknown_reason
    );

    // 2. Aynı oturumda ikinci arka plan helper'ı hemen çıkar (kod 0).
    let mut second = start_activity(&dir, &dir.join("second.log"));
    let t = Instant::now();
    assert!(second.wait_exit(Duration::from_secs(10)).success());
    eprintln!(
        "MEASURE second_instance_exit_ms={}",
        t.elapsed().as_millis()
    );
    assert_eq!(second.count("event=already_running"), 1);

    // 3. Masaüstü köprüsü açılır: raporu reddedilir, ölçüm arka plandan sürer.
    let mut bargs = vec!["bridge".to_string()];
    bargs.extend(transport_args(&dir));
    let bridge = Proc::spawn(&helper_bin(), &bargs, true);
    let st = wait_status(&dir, "köprü reddi", |s| {
        s.measurement.competing_reports >= 1
    });
    assert_eq!(reporter_kind(&st), Some(ReporterKind::BackgroundManual));
    // Tek seferlik rapor da ikinci raporlayıcıdır.
    match one_shot(&dir, "report-activity") {
        Some(Payload::ActivityAck { applied }) => assert!(!applied),
        other => panic!("{other:?}"),
    }
    // Köprü snapshot'ı raporlayıcıyı masaüstüne taşır.
    let t = Instant::now();
    loop {
        let seen = bridge.events.lock().unwrap().iter().any(|e| match e {
            BridgeEvent::Snapshot(s) => s.screen_time.as_ref().is_some_and(|st| {
                st.measurement.reporter.as_ref().and_then(|r| r.kind)
                    == Some(ReporterKind::BackgroundManual)
            }),
            _ => false,
        });
        if seen {
            break;
        }
        assert!(
            t.elapsed() < Duration::from_secs(10),
            "köprü raporlayıcıyı göstermedi"
        );
        thread::sleep(Duration::from_millis(50));
    }

    // 4. Masaüstü kapanır (köprünün stdin'i kapanır): arka plan ölçümü sürer.
    let mut bridge = bridge;
    drop(bridge.stdin.take());
    assert!(bridge.wait_exit(Duration::from_secs(5)).success());
    thread::sleep(Duration::from_millis(800));
    let st = screen_time(&dir);
    assert_eq!(reporter_kind(&st), Some(ReporterKind::BackgroundManual));
    assert_reporter_pid(&st, bg_pid);
    assert!(st.measurement.last_report_age_ms.is_some_and(|a| a < 2_000));

    // 5. Arka plan helper'ı sonlandırılır: yer hemen boşalır.
    bg.kill();
    let t = Instant::now();
    wait_status(&dir, "raporlayıcı boşaldı", |s| {
        s.measurement.reporter.is_none()
    });
    eprintln!(
        "MEASURE reporter_release_after_kill_ms={}",
        t.elapsed().as_millis()
    );

    // 6. Arka plan helper'ı yokken köprü yedek raporlayıcıdır …
    let bridge2 = Proc::spawn(&helper_bin(), &bargs, true);
    wait_status(&dir, "köprü yedek", |s| {
        reporter_kind(s) == Some(ReporterKind::DesktopBridge)
    });
    // … yeniden başlayan arka plan helper'ı devralır ve önceki çıkışı kaydeder.
    let bg2 = start_activity(&dir, &log);
    bg2.wait_for(
        "event=previous_exit state=unclean",
        1,
        Duration::from_secs(10),
    );
    let st = wait_status(&dir, "devralma", |s| {
        reporter_kind(s) == Some(ReporterKind::BackgroundManual)
    });
    assert_reporter_pid(&st, bg2.child.id());
    drop(bridge2);

    // 7. Agent yeniden başlar: helper yeniden bağlanır; süreler korunur, iki kez sayılmaz.
    thread::sleep(Duration::from_secs(16)); // kayıt aralığı (15 sn) dolsun
    let before = screen_time(&dir);
    let mut agent = agent;
    let stop = Instant::now();
    agent.kill();
    bg2.wait_for("event=connection_lost", 1, Duration::from_secs(10));
    let _agent2 = start_agent(&dir);
    bg2.wait_for("event=connected", 2, Duration::from_secs(20));
    eprintln!(
        "MEASURE helper_reconnect_after_agent_restart_ms={}",
        stop.elapsed().as_millis()
    );
    let after = wait_status(&dir, "yeniden bağlantı", |s| {
        reporter_kind(s) == Some(ReporterKind::BackgroundManual)
    });
    let gap = stop.elapsed().as_secs();
    let (b, a) = (total(&before), total(&after));
    eprintln!("MEASURE agent_restart totals before={b} after={a} gap={gap}");
    assert!(a + 16 >= b, "kayıtlar kayboldu: {b} -> {a}");
    assert!(
        a <= b + gap + 5,
        "süre iki kez sayıldı: {b} -> {a} (ara {gap} sn)"
    );

    // 8. Kayıt dosyası helper'ın kendi profilinde; içerik yalnız olay satırları.
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(text.contains("event=helper_started"));
    assert!(text.contains("kind=BACKGROUND_MANUAL"));
    let _ = std::fs::remove_dir_all(&dir);
}
