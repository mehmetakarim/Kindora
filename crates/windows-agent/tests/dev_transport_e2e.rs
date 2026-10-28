//! Süreç düzeyi uçtan uca testler — YALNIZ Unix geliştirme taşıması.
//!
//! Gerçek `kindora-agent` ve `kindora-helper` ikililerini ayrı süreçler olarak
//! çalıştırır. Windows named pipe, servis (SCM) ve oturum davranışını
//! **doğrulamaz**; bunlar Windows test ortamında ayrıca çalıştırılmalıdır.
#![cfg(unix)]

use ipc_protocol::frame::{read_frame, write_frame, FrameError};
use ipc_protocol::transport::unix_dev;
use ipc_protocol::{
    decode_response, CommandType, ErrorCode, Payload, Request, Response, ResponseStatus,
};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn agent_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_kindora-agent"))
}

fn helper_bin() -> PathBuf {
    let p = agent_bin().with_file_name("kindora-helper");
    assert!(
        p.exists(),
        "kindora-helper bulunamadı ({}); `cargo test --workspace` ile çalıştırın",
        p.display()
    );
    p
}

fn socket_path(tag: &str) -> PathBuf {
    // macOS'ta soket yolu ~104 bayt ile sınırlı; kısa tutulur.
    std::env::temp_dir()
        .join(format!(
            "kd-{}-{}-{tag}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ))
        .join("a.sock")
}

/// stderr satırlarını toplayan süreç sarmalayıcı.
struct Proc {
    child: Child,
    lines: Arc<Mutex<Vec<String>>>,
}

impl Proc {
    fn spawn(bin: &Path, args: &[&str]) -> Proc {
        let mut child = Command::new(bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = lines.clone();
        let stderr = child.stderr.take().unwrap();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                sink.lock().unwrap().push(line);
            }
        });
        Proc { child, lines }
    }

    fn count(&self, needle: &str) -> usize {
        self.lines
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.contains(needle))
            .count()
    }

    fn wait_for(&self, needle: &str, n: usize, timeout: Duration) -> Option<Duration> {
        let t = Instant::now();
        while t.elapsed() < timeout {
            if self.count(needle) >= n {
                return Some(t.elapsed());
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }

    fn signal(&self, sig: i32) {
        // SAFETY: kendi başlattığımız çocuk sürece sinyal.
        unsafe { libc::kill(self.child.id() as i32, sig) };
    }

    fn wait_exit(&mut self, timeout: Duration) -> Option<std::process::ExitStatus> {
        let t = Instant::now();
        while t.elapsed() < timeout {
            if let Some(s) = self.child.try_wait().unwrap() {
                return Some(s);
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }

    fn dump(&self) -> String {
        self.lines.lock().unwrap().join("\n")
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_agent(sock: &Path, extra: &[&str]) -> Proc {
    let mut args = vec!["console", "--socket", sock.to_str().unwrap()];
    args.extend_from_slice(extra);
    let p = Proc::spawn(&agent_bin(), &args);
    assert!(
        p.wait_for("event=listening", 1, Duration::from_secs(10))
            .is_some(),
        "agent başlamadı:\n{}",
        p.dump()
    );
    p
}

struct Raw {
    s: UnixStream,
    n: u32,
}

impl Raw {
    fn connect(sock: &Path) -> Raw {
        Raw {
            s: unix_dev::connect(sock, Duration::from_secs(3)).unwrap(),
            n: 0,
        }
    }
    fn send_body(&mut self, body: &[u8]) -> Result<Response, FrameError> {
        write_frame(&mut self.s, body)?;
        Ok(decode_response(&read_frame(&mut self.s, 1 << 20)?).unwrap())
    }
    fn send(&mut self, cmd: CommandType) -> Response {
        self.n += 1;
        let req = Request::new(format!("raw-{}", self.n), cmd);
        self.send_body(&serde_json::to_vec(&req).unwrap()).unwrap()
    }
    fn connected_clients(&mut self) -> u32 {
        match self.send(CommandType::GetStatus).payload {
            Some(Payload::Status {
                connected_clients, ..
            }) => connected_clients,
            other => panic!("unexpected {other:?}"),
        }
    }
}

fn helper_cli(sock: &Path, cmd: &str) -> (bool, String) {
    let out = Command::new(helper_bin())
        .args([cmd, "--socket", sock.to_str().unwrap()])
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn e2e_helper_cli_ping_status_health() {
    let sock = socket_path("cli");
    let _agent = start_agent(&sock, &[]);
    let (ok, out) = helper_cli(&sock, "ping");
    assert!(ok && out.contains("\"PONG\""), "{out}");
    let (ok, out) = helper_cli(&sock, "status");
    assert!(
        ok && out.contains("\"STATUS\"") && out.contains("\"CONSOLE\""),
        "{out}"
    );
    let (ok, out) = helper_cli(&sock, "health");
    assert!(ok && out.contains("\"healthy\":true"), "{out}");
}

#[test]
fn e2e_protocol_rejections_over_real_socket() {
    let sock = socket_path("rej");
    let _agent = start_agent(&sock, &[]);
    let mut c = Raw::connect(&sock);

    let unknown = serde_json::to_vec(&serde_json::json!({
        "protocol_version": 1, "message_id": "u1", "command": "PROCESS_KILL", "timestamp_ms": 1
    }))
    .unwrap();
    assert_eq!(
        c.send_body(&unknown).unwrap().error_code,
        Some(ErrorCode::UnknownCommand)
    );
    assert_eq!(
        c.send_body(b"{oops").unwrap().error_code,
        Some(ErrorCode::InvalidMessage)
    );

    let dup = serde_json::to_vec(&Request::new("dup-1", CommandType::Ping)).unwrap();
    assert_eq!(c.send_body(&dup).unwrap().status, ResponseStatus::Ok);
    assert_eq!(
        c.send_body(&dup).unwrap().error_code,
        Some(ErrorCode::DuplicateMessageId)
    );

    // Boyut sınırı: başlık 1 MB bildirir → hata yanıtı ve bağlantı kapanır.
    let mut big = Raw::connect(&sock);
    big.s.write_all(&(1_000_000u32).to_le_bytes()).unwrap();
    let r = decode_response(&read_frame(&mut big.s, 1 << 20).unwrap()).unwrap();
    assert_eq!(r.error_code, Some(ErrorCode::MessageTooLarge));
    assert!(matches!(
        read_frame(&mut big.s, 1 << 20),
        Err(FrameError::Closed)
    ));
}

#[test]
fn e2e_idle_connection_is_closed_by_agent() {
    let sock = socket_path("idle");
    let _agent = start_agent(&sock, &["--idle-timeout-ms", "600"]);
    let mut c = Raw::connect(&sock);
    c.s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let t = Instant::now();
    assert!(matches!(
        read_frame(&mut c.s, 1 << 20),
        Err(FrameError::Closed)
    ));
    let elapsed = t.elapsed();
    eprintln!("MEASURE idle_close_ms={}", elapsed.as_millis());
    assert!(elapsed >= Duration::from_millis(500) && elapsed < Duration::from_secs(3));
}

#[test]
fn e2e_helper_kill_is_detected_by_agent() {
    let sock = socket_path("kill");
    let agent = start_agent(&sock, &[]);
    let mut helper = Proc::spawn(
        &helper_bin(),
        &[
            "run",
            "--heartbeat-ms",
            "200",
            "--socket",
            sock.to_str().unwrap(),
        ],
    );
    assert!(
        helper
            .wait_for("event=connected", 1, Duration::from_secs(5))
            .is_some(),
        "{}",
        helper.dump()
    );

    let mut probe = Raw::connect(&sock);
    let t = Instant::now();
    while probe.connected_clients() < 2 {
        assert!(t.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(20));
    }

    // Kontrolsüz sonlandırma (Görev Yöneticisi'nden "görevi sonlandır" benzeri).
    helper.signal(libc::SIGKILL);
    let killed_at = Instant::now();
    helper
        .wait_exit(Duration::from_secs(5))
        .expect("helper did not exit");
    while probe.connected_clients() > 1 {
        assert!(
            killed_at.elapsed() < Duration::from_secs(5),
            "agent kaybı fark etmedi:\n{}",
            agent.dump()
        );
        thread::sleep(Duration::from_millis(5));
    }
    let detect = killed_at.elapsed();
    eprintln!("MEASURE helper_kill_detect_ms={}", detect.as_millis());
    assert!(detect < Duration::from_secs(2));
    assert!(
        agent
            .wait_for("outcome=PeerClosed", 1, Duration::from_secs(2))
            .is_some(),
        "{}",
        agent.dump()
    );
}

#[test]
fn e2e_agent_restart_and_helper_reconnect_then_graceful_stops() {
    let sock = socket_path("rst");
    let mut agent = start_agent(&sock, &[]);
    let mut helper = Proc::spawn(
        &helper_bin(),
        &[
            "run",
            "--heartbeat-ms",
            "200",
            "--socket",
            sock.to_str().unwrap(),
        ],
    );
    assert!(
        helper
            .wait_for("event=connected", 1, Duration::from_secs(5))
            .is_some(),
        "{}",
        helper.dump()
    );

    // Agent'ı Ctrl+C eşdeğeriyle durdur: düzgün kapanmalı ve soketi kaldırmalı.
    agent.signal(libc::SIGINT);
    let stop_at = Instant::now();
    let status = agent
        .wait_exit(Duration::from_secs(5))
        .expect("agent did not stop");
    eprintln!(
        "MEASURE agent_graceful_stop_ms={}",
        stop_at.elapsed().as_millis()
    );
    assert!(status.success(), "{}", agent.dump());
    assert!(agent.count("event=agent_stopped") == 1, "{}", agent.dump());
    assert!(!sock.exists(), "soket dosyası kaldırılmadı");

    helper
        .wait_for("event=connection_lost", 1, Duration::from_secs(5))
        .expect("helper did not detect loss");
    // Üst sınır: SIGINT'ten helper'ın kaybı kaydetmesine kadar geçen süre
    // (kalp atışı aralığı 200 ms; helper kaybı bir sonraki PING'de fark eder).
    eprintln!(
        "MEASURE helper_detects_agent_stop_upper_bound_ms={}",
        stop_at.elapsed().as_millis()
    );

    // Agent yeniden başlar → helper kendiliğinden yeniden bağlanır.
    let _agent2 = start_agent(&sock, &[]);
    let restarted = Instant::now();
    assert!(
        helper
            .wait_for("event=connected", 2, Duration::from_secs(10))
            .is_some(),
        "{}",
        helper.dump()
    );
    eprintln!(
        "MEASURE helper_reconnect_after_restart_ms={}",
        restarted.elapsed().as_millis()
    );

    // Helper'ı kontrollü kapat.
    helper.signal(libc::SIGINT);
    let st = helper
        .wait_exit(Duration::from_secs(5))
        .expect("helper did not stop");
    assert!(st.success());
    assert_eq!(helper.count("event=helper_stopped"), 1, "{}", helper.dump());
}

#[test]
fn e2e_second_agent_cannot_start_on_live_socket_path_safely() {
    // Canlı soket yolu başka bir dosya türüyle değiştirilmişse agent başlamaz (dosya silinmez).
    let sock = socket_path("file");
    std::fs::create_dir_all(sock.parent().unwrap()).unwrap();
    std::fs::write(&sock, b"not a socket").unwrap();
    let mut p = Proc::spawn(
        &agent_bin(),
        &["console", "--socket", sock.to_str().unwrap()],
    );
    let st = p
        .wait_exit(Duration::from_secs(5))
        .expect("agent should exit");
    assert!(!st.success());
    assert_eq!(std::fs::read(&sock).unwrap(), b"not a socket");
}
