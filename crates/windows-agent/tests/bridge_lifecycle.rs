//! Masaüstü köprüsü (`kindora-helper bridge`, P1.B) bağlantı yaşam döngüsü —
//! gerçek `kindora-agent` (console) ve `kindora-helper` süreçleriyle.
//!
//! Masaüstü uygulamasının helper'ı nasıl kullandığını taklit eder: helper
//! stdin/stdout boruları bağlı alt süreç olarak başlatılır, olaylar satır satır
//! okunur. Unix'te yalnız geliştirme soketi, Windows'ta üretim pipe adı
//! kullanılır; üretim adı paylaşıldığı için tüm adımlar **tek test** içindedir.
//!
//! Sınanan senaryolar: agent kapalıyken açılış, agent'ın sonradan başlaması,
//! `refresh`, agent'ın durması ve yeniden başlaması, izinsiz istek satırının
//! reddi, stdin kapanınca temiz çıkış (süreç sızıntısı yok) ve — Windows'ta —
//! farklı dizindeki helper kopyasının reddedilmesi.
//!
//! DOĞRULAMAZ: Tauri penceresi, WebView2, LocalService servisi (CI betiği
//! `ci/p1a-localservice-e2e.ps1` köprüyü servis ile ayrıca dener).

use ipc_protocol::bridge::{decode_bridge_event, BridgeEvent, LinkState};
use ipc_protocol::RunMode;
use std::io::{BufRead, BufReader, Write};
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
        "kindora-helper yok veya bayat olabilir ({}); önce `cargo build --workspace` çalıştırın",
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

#[cfg(windows)]
fn own_session_id() -> u32 {
    let mut s = 0u32;
    // SAFETY: geçerli PID ve çıktı işaretçisi.
    let ok = unsafe {
        windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId(std::process::id(), &mut s)
    };
    assert_ne!(ok, 0);
    s
}

struct Agent(Child);

impl Agent {
    fn start(dir: &Path) -> Agent {
        let mut args = vec!["console".to_string()];
        args.extend(transport_args(dir));
        let child = Command::new(agent_bin())
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("agent spawn");
        Agent(child)
    }

    /// Unix: SIGINT ile düzgün kapanış. Windows: konsol Ctrl+C gönderilemediği
    /// için süreç sonlandırılır (servis durdurma CI betiğinde ayrıca sınanır).
    fn stop(mut self) {
        #[cfg(unix)]
        {
            // SAFETY: kendi alt sürecimize sinyal.
            unsafe { libc::kill(self.0.id() as i32, libc::SIGINT) };
        }
        #[cfg(windows)]
        {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

struct Bridge {
    child: Child,
    stdin: Option<ChildStdin>,
    events: Arc<Mutex<Vec<(Instant, BridgeEvent)>>>,
}

impl Bridge {
    fn spawn(bin: &Path, dir: &Path) -> Bridge {
        let mut args = vec!["bridge".to_string()];
        args.extend(transport_args(dir));
        let mut child = Command::new(bin)
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
                let e = decode_bridge_event(&line).expect("bridge stdout must be valid events");
                sink.lock().unwrap().push((Instant::now(), e));
            }
        });
        Bridge {
            child,
            stdin,
            events,
        }
    }

    fn send(&mut self, line: &str) {
        let s = self.stdin.as_mut().expect("stdin open");
        s.write_all(line.as_bytes()).unwrap();
        s.write_all(b"\n").unwrap();
        s.flush().unwrap();
    }

    fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// `from` indeksinden sonra koşulu sağlayan ilk olayın zamanını bekler.
    fn wait(&self, from: usize, timeout: Duration, pred: impl Fn(&BridgeEvent) -> bool) -> Instant {
        let t = Instant::now();
        while t.elapsed() < timeout {
            if let Some((at, _)) = self.events.lock().unwrap()[from..]
                .iter()
                .find(|(_, e)| pred(e))
            {
                return *at;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "olay gelmedi ({timeout:?}); olaylar: {:#?}",
            self.events.lock().unwrap()[from..]
                .iter()
                .map(|(_, e)| e)
                .collect::<Vec<_>>()
        );
    }

    fn snapshots(&self) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, e)| matches!(e, BridgeEvent::Snapshot(_)))
            .count()
    }

    /// stdin'i kapatır (masaüstü kapanışı) ve çıkışı bekler.
    fn close(mut self) -> (Duration, Option<i32>) {
        drop(self.stdin.take());
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(5) {
            if let Some(status) = self.child.try_wait().unwrap() {
                return (t.elapsed(), status.code());
            }
            thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        panic!("köprü stdin kapandıktan sonra 5 sn içinde çıkmadı (süreç sızıntısı)");
    }
}

fn link(state: LinkState) -> impl Fn(&BridgeEvent) -> bool {
    move |e| matches!(e, BridgeEvent::Link { state: s, .. } if *s == state)
}

fn is_snapshot(e: &BridgeEvent) -> bool {
    matches!(e, BridgeEvent::Snapshot(s) if s.run_mode == RunMode::Console && s.healthy)
}

fn temp_dir(tag: &str) -> PathBuf {
    // macOS'ta soket yolu ~104 bayt ile sınırlı; kısa tutulur.
    let d = std::env::temp_dir().join(format!("kb-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn bridge_connection_lifecycle() {
    #[cfg(windows)]
    let authorized = own_session_id() != 0;
    #[cfg(unix)]
    let authorized = true;
    let dir = temp_dir("life");

    // 1. Agent kapalıyken köprü açılır → "bağlantı yok", yoğun döngü yok.
    let mut bridge = Bridge::spawn(&helper_bin(), &dir);
    bridge.wait(0, Duration::from_secs(3), |e| {
        matches!(e, BridgeEvent::Hello { .. })
    });
    bridge.wait(0, Duration::from_secs(3), link(LinkState::Disconnected));
    thread::sleep(Duration::from_millis(1200));
    let attempts = bridge
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, e)| {
            matches!(
                e,
                BridgeEvent::Link {
                    state: LinkState::Disconnected,
                    ..
                }
            )
        })
        .count();
    eprintln!("MEASURE bridge_attempts_first_1500ms={attempts}");
    assert!(attempts <= 4, "geri çekilme yok: {attempts} deneme");

    if !authorized {
        // Oturum 0 (servis olarak çalışan CI): politika köprüyü reddetmeli.
        let agent = Agent::start(&dir);
        bridge.wait(0, Duration::from_secs(10), link(LinkState::Unauthorized));
        agent.stop();
        bridge.close();
        return;
    }

    // 2. Agent sonradan başlar → otomatik bağlantı ve gerçek anlık görüntü.
    let mark = bridge.len();
    let started = Instant::now();
    let agent = Agent::start(&dir);
    bridge.wait(mark, Duration::from_secs(10), link(LinkState::Connected));
    let at = bridge.wait(mark, Duration::from_secs(10), is_snapshot);
    eprintln!(
        "MEASURE bridge_connect_after_agent_start_ms={}",
        at.duration_since(started).as_millis()
    );

    // 3. refresh → yoklama aralığını (3 sn) beklemeden yeni anlık görüntü.
    let before = bridge.snapshots();
    let mark = bridge.len();
    let t = Instant::now();
    bridge.send(r#"{"op":"refresh"}"#);
    bridge.wait(mark, Duration::from_millis(2500), |e| {
        matches!(e, BridgeEvent::Snapshot(_))
    });
    assert!(bridge.snapshots() > before);
    eprintln!("MEASURE bridge_refresh_ms={}", t.elapsed().as_millis());

    // 4. İzinsiz istek satırı reddedilir; köprü çalışmaya devam eder.
    let mark = bridge.len();
    bridge.send(r#"{"op":"LOCK"}"#);
    bridge.send(r#"{"op":"refresh","command":"PROCESS_KILL"}"#);
    bridge.wait(mark, Duration::from_secs(5), |e| {
        matches!(e, BridgeEvent::RequestRejected { .. })
    });

    // 5. Agent durur → köprü bağlantı kaybını bildirir.
    let mark = bridge.len();
    let stopped = Instant::now();
    agent.stop();
    let at = bridge.wait(mark, Duration::from_secs(10), link(LinkState::Disconnected));
    eprintln!(
        "MEASURE bridge_detect_agent_stop_ms={}",
        at.duration_since(stopped).as_millis()
    );

    // 6. Agent yeniden başlar → bağlantı kurtarılır.
    let mark = bridge.len();
    let restarted = Instant::now();
    let agent = Agent::start(&dir);
    bridge.wait(mark, Duration::from_secs(12), link(LinkState::Connected));
    let at = bridge.wait(mark, Duration::from_secs(12), is_snapshot);
    eprintln!(
        "MEASURE bridge_recover_after_agent_restart_ms={}",
        at.duration_since(restarted).as_millis()
    );

    // 7. Windows: farklı dizindeki helper kopyası yetkisiz kalır (politika gevşemez).
    #[cfg(windows)]
    {
        let other = temp_dir("other");
        let copy = other.join("kindora-helper.exe");
        std::fs::copy(helper_bin(), &copy).unwrap();
        let rogue = Bridge::spawn(&copy, &dir);
        rogue.wait(0, Duration::from_secs(10), link(LinkState::Unauthorized));
        let (_, code) = rogue.close();
        assert_eq!(code, Some(0));
    }

    // 8. Masaüstü kapanışı: stdin kapanır → köprü temiz çıkar.
    let (took, code) = bridge.close();
    eprintln!(
        "MEASURE bridge_exit_after_stdin_close_ms={}",
        took.as_millis()
    );
    assert_eq!(code, Some(0));
    agent.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
