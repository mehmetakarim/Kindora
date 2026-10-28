//! Windows süreç düzeyi smoke testi — gerçek `kindora-agent.exe` (console) ve
//! `kindora-helper.exe`, üretim pipe adı üzerinden. Yönetici, servis kurulumu
//! veya ikinci kullanıcı gerektirmez.
//!
//! Üretim pipe adı paylaşıldığı için tüm adımlar **tek test** içinde sırayla
//! çalışır. Beklenen sonuç, testin çalıştığı Windows oturumuna göre değişir:
//! - oturum ≠ 0 → helper yetkilidir (PONG beklenir);
//! - oturum = 0 (ör. servis olarak çalışan CI) → politika gereği reddedilir.
//!
//! Her iki durumda da beklenen davranış açıkça doğrulanır; hiçbir adım atlanmaz.
//!
//! DOĞRULAMAZ: Windows servisi (SCM), LocalService hesabı, standart kullanıcı
//! DACL erişimi, başka kullanıcı/oturum, Görev Yöneticisi'nden sonlandırma.
#![cfg(windows)]

use ipc_protocol::frame::{read_frame, FrameError};
use ipc_protocol::transport::windows_pipe::connect;
use ipc_protocol::{decode_response, ErrorCode};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn agent_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_kindora-agent"))
}

fn helper_bin() -> PathBuf {
    let p = agent_bin().with_file_name("kindora-helper.exe");
    assert!(p.exists(), "kindora-helper.exe yok veya bayat olabilir ({}); önce `cargo build --workspace` çalıştırın", p.display());
    p
}

fn own_session_id() -> u32 {
    let mut s = 0u32;
    // SAFETY: geçerli PID ve çıktı işaretçisi.
    let ok = unsafe {
        windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId(std::process::id(), &mut s)
    };
    assert_ne!(ok, 0);
    s
}

struct Proc {
    child: Child,
    lines: Arc<Mutex<Vec<String>>>,
}

impl Proc {
    fn spawn(bin: &PathBuf, args: &[&str]) -> Proc {
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
            for l in BufReader::new(stderr).lines().map_while(Result::ok) {
                sink.lock().unwrap().push(l);
            }
        });
        Proc { child, lines }
    }
    fn wait_for(&self, needle: &str, n: usize, timeout: Duration) -> bool {
        let t = Instant::now();
        while t.elapsed() < timeout {
            if self
                .lines
                .lock()
                .unwrap()
                .iter()
                .filter(|l| l.contains(needle))
                .count()
                >= n
            {
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
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

fn helper(cmd: &str) -> Output {
    Command::new(helper_bin()).arg(cmd).output().unwrap()
}

fn connected_clients_via_helper() -> Option<u64> {
    let out = helper("status");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    v["payload"]["connected_clients"].as_u64()
}

#[test]
fn windows_agent_and_helper_processes_over_production_pipe() {
    let session = own_session_id();
    eprintln!("MEASURE e2e_session_id={session}");
    let interactive = session != 0;

    let agent = Proc::spawn(&agent_bin(), &["console"]);
    assert!(
        agent.wait_for("event=listening", 1, Duration::from_secs(10)),
        "agent başlamadı:\n{}",
        agent.dump()
    );

    // 1) Helper CLI: PING / GET_STATUS / HEALTH_CHECK
    for (cmd, needle) in [
        ("ping", "\"PONG\""),
        ("status", "\"STATUS\""),
        ("health", "\"HEALTH\""),
    ] {
        let out = helper(cmd);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if interactive {
            assert!(
                out.status.success() && stdout.contains(needle),
                "{cmd}: {stdout} {stderr}\n{}",
                agent.dump()
            );
        } else {
            assert_eq!(out.status.code(), Some(3), "{cmd}: {stdout} {stderr}");
            assert!(stderr.contains("unauthorized"), "{cmd}: {stderr}");
        }
    }

    // 2) Yetkisiz istemci: bu test ikilisi kindora-helper.exe değildir.
    let mut raw = connect(Duration::from_secs(3), Duration::from_secs(3)).expect("raw connect");
    let r =
        decode_response(&read_frame(&mut raw, 1 << 20).expect("unauthorized response")).unwrap();
    assert_eq!(r.error_code, Some(ErrorCode::Unauthorized));
    assert!(matches!(
        read_frame(&mut raw, 1 << 20),
        Err(FrameError::Closed)
    ));
    drop(raw);
    assert!(
        agent.wait_for("outcome=Unauthorized(", 1, Duration::from_secs(3)),
        "{}",
        agent.dump()
    );

    // 3) Helper süreci sonlandırılınca (TerminateProcess) agent bağlantı kaybını görür.
    if interactive {
        let mut run = Proc::spawn(&helper_bin(), &["run", "--heartbeat-ms", "200"]);
        assert!(
            run.wait_for("event=connected", 1, Duration::from_secs(5)),
            "{}",
            run.dump()
        );
        let t = Instant::now();
        while connected_clients_via_helper() != Some(2) {
            assert!(t.elapsed() < Duration::from_secs(5), "{}", agent.dump());
            thread::sleep(Duration::from_millis(50));
        }
        run.child.kill().unwrap();
        let killed = Instant::now();
        run.child.wait().unwrap();
        while connected_clients_via_helper() != Some(1) {
            assert!(
                killed.elapsed() < Duration::from_secs(5),
                "{}",
                agent.dump()
            );
            thread::sleep(Duration::from_millis(20));
        }
        eprintln!(
            "MEASURE win_helper_terminate_detect_upper_ms={}",
            killed.elapsed().as_millis()
        );
    }

    // 4) İkinci agent aynı pipe adını alamaz (FILE_FLAG_FIRST_PIPE_INSTANCE).
    let mut second = Proc::spawn(&agent_bin(), &["console"]);
    let t = Instant::now();
    let status = loop {
        if let Some(s) = second.child.try_wait().unwrap() {
            break s;
        }
        assert!(
            t.elapsed() < Duration::from_secs(10),
            "second agent did not exit:\n{}",
            second.dump()
        );
        thread::sleep(Duration::from_millis(20));
    };
    assert!(!status.success());
    assert!(
        second.wait_for("event=agent_failed", 1, Duration::from_secs(2)),
        "{}",
        second.dump()
    );
    eprintln!(
        "MEASURE second_agent_failed_log={:?}",
        second.dump().lines().last()
    );

    // Agent loglarında kimlik teşhis alanı yer almalı (W-10 için kanıt biçimi).
    assert!(
        agent.wait_for("os_errors=", 1, Duration::from_secs(1)),
        "{}",
        agent.dump()
    );
    eprintln!("AGENT LOG:\n{}", agent.dump());
}
