//! Windows named pipe smoke testleri — süreç içi, yönetici/servis gerektirmez.
//!
//! Her test benzersiz bir pipe adı kullanır; üretim adına (`kindora-agent-v1`)
//! dokunmaz. Sunucu ve istemci **aynı süreç ve aynı kullanıcıdır**; bu yüzden
//! aşağıdakileri DOĞRULAMAZ: standart kullanıcının DACL ile erişimi, başka
//! kullanıcı/oturumdan bağlantı, LocalService ile süreç sorgusu (W-03, W-10, W-12).
#![cfg(windows)]

use agent_core::auth::{Authorizer, DenyReason};
use agent_core::connection::{serve_connection, ConnectionOutcome, ConnectionReport};
use agent_core::{AgentState, ConnectionLimits};
use ipc_protocol::frame::{read_frame, write_frame, FrameError};
use ipc_protocol::transport::windows_pipe::{connect_named, PipeServer, PipeStream};
use ipc_protocol::transport::PeerIdentity;
use ipc_protocol::{decode_response, CommandType, ErrorCode, Payload, Request, Response, RunMode};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn unique_name() -> String {
    format!(
        r"\\.\pipe\kindora-test-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    )
}

fn own_session_id() -> u32 {
    let mut s = 0u32;
    // SAFETY: geçerli PID ve çıktı işaretçisi.
    let ok = unsafe {
        windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId(std::process::id(), &mut s)
    };
    assert_ne!(ok, 0, "ProcessIdToSessionId failed");
    s
}

struct AllowAll;
impl Authorizer for AllowAll {
    fn authorize(&self, _peer: &PeerIdentity) -> Result<(), DenyReason> {
        Ok(())
    }
}

struct Served {
    handle: JoinHandle<(PeerIdentity, ConnectionReport)>,
    shutdown: Arc<AtomicBool>,
}

/// Tek bağlantı kabul eden sunucu iş parçacığı.
fn serve_once(name: &str, limits: ConnectionLimits, authorizer: Box<dyn Authorizer>) -> Served {
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();
    let name = name.to_owned();
    let handle = thread::spawn(move || {
        let mut server = PipeServer::with_name(&name).expect("server");
        let (mut stream, peer) = server
            .accept(Duration::from_millis(100), &flag)
            .expect("accept")
            .expect("client");
        let state = AgentState::new(RunMode::Console);
        let rep = serve_connection(
            &mut stream,
            &peer,
            &state,
            authorizer.as_ref(),
            &limits,
            &flag,
        );
        (peer, rep)
    });
    Served { handle, shutdown }
}

fn client(name: &str) -> PipeStream {
    let t = Instant::now();
    loop {
        match connect_named(name, Duration::from_secs(2), Duration::from_secs(3)) {
            Ok(s) => return s,
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    && t.elapsed() < Duration::from_secs(5) =>
            {
                thread::sleep(Duration::from_millis(20))
            }
            Err(e) => panic!("connect: {e}"),
        }
    }
}

fn send(c: &mut PipeStream, body: &[u8]) -> Response {
    write_frame(c, body).unwrap();
    decode_response(&read_frame(c, 1 << 20).unwrap()).unwrap()
}

fn req(id: &str, cmd: CommandType) -> Vec<u8> {
    serde_json::to_vec(&Request::new(id, cmd)).unwrap()
}

#[test]
fn pipe_round_trip_and_os_reported_identity() {
    let name = unique_name();
    let srv = serve_once(&name, ConnectionLimits::default(), Box::new(AllowAll));
    let mut c = client(&name);
    assert_eq!(c.server_process_id(), Some(std::process::id()));
    assert_eq!(
        send(&mut c, &req("p1", CommandType::Ping)).payload,
        Some(Payload::Pong)
    );
    assert!(matches!(
        send(&mut c, &req("s1", CommandType::GetStatus)).payload,
        Some(Payload::Status { .. })
    ));
    drop(c);
    let (peer, rep) = srv.handle.join().unwrap();
    assert_eq!(rep.outcome, ConnectionOutcome::PeerClosed);
    assert_eq!(peer.pid, Some(std::process::id()));
    assert_eq!(peer.session_id, Some(own_session_id()));
    let exe = std::env::current_exe().unwrap();
    assert_eq!(
        peer.exe_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_lowercase()),
        Some(exe.to_string_lossy().to_lowercase()),
        "diagnostics: {:?}",
        peer.diagnostics
    );
    eprintln!("MEASURE ci_session_id={}", own_session_id());
}

#[test]
fn pipe_protocol_rejections() {
    let name = unique_name();
    let srv = serve_once(&name, ConnectionLimits::default(), Box::new(AllowAll));
    let mut c = client(&name);
    let unknown = serde_json::to_vec(&serde_json::json!({
        "protocol_version": 1, "message_id": "u1", "command": "LOCK", "timestamp_ms": 1
    }))
    .unwrap();
    assert_eq!(
        send(&mut c, &unknown).error_code,
        Some(ErrorCode::UnknownCommand)
    );
    assert_eq!(
        send(&mut c, b"{bad").error_code,
        Some(ErrorCode::InvalidMessage)
    );
    assert_eq!(send(&mut c, &req("d1", CommandType::Ping)).error_code, None);
    assert_eq!(
        send(&mut c, &req("d1", CommandType::Ping)).error_code,
        Some(ErrorCode::DuplicateMessageId)
    );
    c.write_all(&(1_000_000u32).to_le_bytes()).unwrap();
    let r = decode_response(&read_frame(&mut c, 1 << 20).unwrap()).unwrap();
    assert_eq!(r.error_code, Some(ErrorCode::MessageTooLarge));
    assert!(matches!(
        read_frame(&mut c, 1 << 20),
        Err(FrameError::Closed)
    ));
    assert_eq!(
        srv.handle.join().unwrap().1.outcome,
        ConnectionOutcome::MessageTooLarge
    );
}

#[test]
fn pipe_idle_timeout_uses_overlapped_cancel() {
    let name = unique_name();
    let limits = ConnectionLimits {
        idle_timeout: Duration::from_millis(400),
        ..Default::default()
    };
    let srv = serve_once(&name, limits, Box::new(AllowAll));
    let mut c = client(&name);
    let t = Instant::now();
    assert!(matches!(
        read_frame(&mut c, 1 << 20),
        Err(FrameError::Closed)
    ));
    eprintln!("MEASURE pipe_idle_close_ms={}", t.elapsed().as_millis());
    assert!(t.elapsed() < Duration::from_secs(3));
    assert_eq!(
        srv.handle.join().unwrap().1.outcome,
        ConnectionOutcome::IdleTimeout
    );
}

#[test]
fn pipe_client_disconnect_is_detected() {
    let name = unique_name();
    let srv = serve_once(&name, ConnectionLimits::default(), Box::new(AllowAll));
    let mut c = client(&name);
    assert_eq!(
        send(&mut c, &req("a", CommandType::Ping)).payload,
        Some(Payload::Pong)
    );
    let t = Instant::now();
    drop(c);
    let rep = srv.handle.join().unwrap().1;
    eprintln!(
        "MEASURE pipe_disconnect_detect_ms={}",
        t.elapsed().as_millis()
    );
    assert_eq!(rep.outcome, ConnectionOutcome::PeerClosed);
    assert!(t.elapsed() < Duration::from_secs(2));
}

#[test]
fn pipe_accept_honours_shutdown() {
    let name = unique_name();
    let flag = Arc::new(AtomicBool::new(false));
    let f2 = flag.clone();
    let n2 = name.clone();
    let h = thread::spawn(move || {
        let mut s = PipeServer::with_name(&n2).unwrap();
        s.accept(Duration::from_millis(100), &f2)
            .map(|o| o.is_none())
    });
    thread::sleep(Duration::from_millis(300));
    let t = Instant::now();
    flag.store(true, Ordering::SeqCst);
    assert!(
        h.join().unwrap().expect("accept error"),
        "expected Ok(None)"
    );
    assert!(t.elapsed() < Duration::from_secs(2));
}

#[test]
fn first_pipe_instance_blocks_squatting() {
    let name = unique_name();
    let srv = serve_once(&name, ConnectionLimits::default(), Box::new(AllowAll));
    // İlk sunucunun pipe örneğini oluşturmasına zaman tanı (yarış önlemi).
    thread::sleep(Duration::from_millis(500));
    let mut second = PipeServer::with_name(&name).unwrap();
    let stop_now = AtomicBool::new(true);
    let err = match second.accept(Duration::from_millis(50), &stop_now) {
        Err(e) => e,
        Ok(_) => panic!("second server instance was allowed (pipe squatting not blocked)"),
    };
    eprintln!("MEASURE squat_error={err} raw={:?}", err.raw_os_error());
    assert_eq!(err.raw_os_error(), Some(5), "expected ERROR_ACCESS_DENIED");
    srv.shutdown.store(true, Ordering::SeqCst);
    // Sunucuyu serbest bırakmak için bağlan ve kapat.
    drop(client(&name));
    let _ = srv.handle.join();
}

#[test]
fn production_helper_policy_rejects_this_test_executable() {
    // Test ikilisi `kindora-helper.exe` değildir → gerçek politika reddetmeli.
    let name = unique_name();
    let srv = serve_once(
        &name,
        ConnectionLimits::default(),
        Box::new(windows_agent::helper_policy()),
    );
    let mut c = client(&name);
    let r = decode_response(&read_frame(&mut c, 1 << 20).unwrap()).unwrap();
    assert_eq!(r.error_code, Some(ErrorCode::Unauthorized));
    let (peer, rep) = srv.handle.join().unwrap();
    eprintln!(
        "MEASURE policy_reject_reason={:?} session={:?}",
        rep.outcome, peer.session_id
    );
    assert!(matches!(rep.outcome, ConnectionOutcome::Unauthorized(_)));
}

/// BUG-003: helper'ın süreç DACL'ine eklediği tek ACE — LocalService (`LS`)
/// için yalnız `PROCESS_QUERY_LIMITED_INFORMATION` (0x1000). Gerçek LocalService
/// servisiyle uçtan uca doğrulama CI iş akışının servis adımındadır.
#[test]
fn helper_grants_only_query_limited_to_local_service() {
    use ipc_protocol::transport::windows_pipe::{
        current_process_dacl_sddl, grant_agent_process_query,
    };
    let before = current_process_dacl_sddl().expect("dacl before");
    assert!(!before.contains(";;;LS)"), "beklenmeyen LS ACE: {before}");

    grant_agent_process_query().expect("grant");
    grant_agent_process_query().expect("grant is idempotent");

    let after = current_process_dacl_sddl().expect("dacl after");
    eprintln!("dacl_before={before}\ndacl_after={after}");
    let ls: Vec<&str> = after
        .split('(')
        .filter(|ace| ace.ends_with(";;;LS)"))
        .collect();
    assert_eq!(ls, vec!["A;;0x1000;;;LS)"], "DACL: {after}");
}
