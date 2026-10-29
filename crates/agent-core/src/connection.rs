//! Tek bir IPC bağlantısının yaşam döngüsü.
//!
//! Taşıma katmanı akışa kısa bir okuma zaman aşımı (yoklama aralığı) koyar;
//! bu döngü bu aralıkla `shutdown` bayrağını ve boşta kalma süresini denetler.

use crate::auth::{Authorizer, DenyReason};
use crate::handler::{Disposition, SessionHandler};
use crate::{AgentState, ConnectionLimits};
use ipc_protocol::frame::{read_frame, write_frame, FrameError};
use ipc_protocol::transport::PeerIdentity;
use ipc_protocol::{ErrorCode, Response};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Bağlantının neden sona erdiği (kayıt ve ölçüm için).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionOutcome {
    Unauthorized(DenyReason),
    /// İstemci çerçeve sınırında bağlantıyı kapattı (süreç çıkışı dahil).
    PeerClosed,
    /// Çerçeve yarıda kaldı veya G/Ç hatası.
    Broken(String),
    IdleTimeout,
    MessageTooLarge,
    TooManyErrors,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionReport {
    pub outcome: ConnectionOutcome,
    pub requests: u64,
    pub errors: u32,
}

fn write_response<S: Write>(stream: &mut S, r: &Response) -> Result<(), FrameError> {
    write_frame(stream, &r.to_bytes())
}

pub fn serve_connection<S: Read + Write>(
    stream: &mut S,
    peer: &PeerIdentity,
    state: &AgentState,
    authorizer: &dyn Authorizer,
    limits: &ConnectionLimits,
    shutdown: &AtomicBool,
) -> ConnectionReport {
    if let Err(reason) = authorizer.authorize(peer) {
        // Gerekçe istemciye bildirilmez; yalnız genel kod.
        let _ = write_response(stream, &Response::error(None, ErrorCode::Unauthorized));
        return ConnectionReport {
            outcome: ConnectionOutcome::Unauthorized(reason),
            requests: 0,
            errors: 0,
        };
    }

    let _guard = state.register_client();
    let mut handler = SessionHandler::new(limits.clone()).with_client_session(peer.session_id);
    let mut requests = 0u64;
    let mut last_activity = Instant::now();

    let outcome = loop {
        if shutdown.load(Ordering::SeqCst) {
            break ConnectionOutcome::Shutdown;
        }
        match read_frame(stream, limits.max_message_bytes) {
            Ok(body) => {
                last_activity = Instant::now();
                requests += 1;
                let (resp, disposition) = handler.handle(state, &body);
                if let Err(e) = write_response(stream, &resp) {
                    break ConnectionOutcome::Broken(e.to_string());
                }
                if disposition == Disposition::Close {
                    break ConnectionOutcome::TooManyErrors;
                }
            }
            Err(FrameError::Idle) => {
                if last_activity.elapsed() >= limits.idle_timeout {
                    break ConnectionOutcome::IdleTimeout;
                }
            }
            Err(FrameError::Closed) => break ConnectionOutcome::PeerClosed,
            Err(FrameError::TooLarge(_)) => {
                // Gövde okunmadığı için akış senkronu kayboldu: yanıtla ve kapat.
                let r = handler.reject_oversized();
                let _ = write_response(stream, &r);
                break ConnectionOutcome::MessageTooLarge;
            }
            Err(e @ (FrameError::Truncated | FrameError::Io(_))) => {
                break ConnectionOutcome::Broken(e.to_string())
            }
        }
    };

    ConnectionReport {
        outcome,
        requests,
        errors: handler.errors(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::auth::HelperPolicy;
    use ipc_protocol::{decode_response, CommandType, Request, ResponseStatus, RunMode};
    use std::os::unix::net::UnixStream;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    const POLL: Duration = Duration::from_millis(50);

    fn limits() -> ConnectionLimits {
        ConnectionLimits {
            idle_timeout: Duration::from_millis(300),
            max_errors: 3,
            ..Default::default()
        }
    }

    struct Harness {
        client: UnixStream,
        server: thread::JoinHandle<ConnectionReport>,
        shutdown: Arc<AtomicBool>,
        state: Arc<AgentState>,
    }

    fn start(peer: PeerIdentity, policy: HelperPolicy) -> Harness {
        let (client, mut server) = UnixStream::pair().unwrap();
        server.set_read_timeout(Some(POLL)).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let state = Arc::new(AgentState::new(RunMode::Console));
        let (sd, st) = (shutdown.clone(), state.clone());
        let server = thread::spawn(move || {
            serve_connection(&mut server, &peer, &st, &policy, &limits(), &sd)
        });
        Harness {
            client,
            server,
            shutdown,
            state,
        }
    }

    fn uid_policy(uid: u32) -> HelperPolicy {
        HelperPolicy {
            required_uid: Some(uid),
            ..Default::default()
        }
    }

    fn peer(uid: u32) -> PeerIdentity {
        PeerIdentity {
            uid: Some(uid),
            ..Default::default()
        }
    }

    fn send(c: &mut UnixStream, body: &[u8]) -> Response {
        write_frame(c, body).unwrap();
        decode_response(&read_frame(c, 1 << 20).unwrap()).unwrap()
    }

    fn json(r: &Request) -> Vec<u8> {
        serde_json::to_vec(r).unwrap()
    }

    #[test]
    fn ping_over_stream_then_peer_close() {
        let mut h = start(peer(7), uid_policy(7));
        let r = send(&mut h.client, &json(&Request::new("a", CommandType::Ping)));
        assert_eq!(r.status, ResponseStatus::Ok);
        drop(h.client);
        let rep = h.server.join().unwrap();
        assert_eq!(rep.outcome, ConnectionOutcome::PeerClosed);
        assert_eq!(rep.requests, 1);
        assert_eq!(h.state.connected_clients(), 0);
    }

    #[test]
    fn unauthorized_peer_gets_generic_error_and_is_dropped() {
        let mut h = start(peer(8), uid_policy(7));
        let r = decode_response(&read_frame(&mut h.client, 1 << 20).unwrap()).unwrap();
        assert_eq!(r.error_code, Some(ErrorCode::Unauthorized));
        assert_eq!(
            h.server.join().unwrap().outcome,
            ConnectionOutcome::Unauthorized(DenyReason::UidMismatch)
        );
        // Sunucu kapandı; istemci EOF görür.
        assert!(matches!(
            read_frame(&mut h.client, 1 << 20),
            Err(FrameError::Closed)
        ));
        assert_eq!(h.state.connected_clients(), 0);
    }

    #[test]
    fn identity_claim_in_message_is_rejected_not_trusted() {
        // Yetkilendirme yalnız `PeerIdentity`'ye dayanır; gövdeye kimlik iddiası
        // eklemek bilinmeyen alan olduğundan INVALID_MESSAGE ile reddedilir.
        let mut h = start(peer(7), uid_policy(7));
        let body = serde_json::to_vec(&serde_json::json!({
            "protocol_version": 1, "message_id": "c-1", "command": "PING",
            "timestamp_ms": 1, "client_uid": 0
        }))
        .unwrap();
        let r = send(&mut h.client, &body);
        assert_eq!(r.error_code, Some(ErrorCode::InvalidMessage));
    }

    #[test]
    fn oversized_frame_closes_connection() {
        let mut h = start(peer(7), uid_policy(7));
        use std::io::Write as _;
        h.client.write_all(&(1_000_000u32).to_le_bytes()).unwrap();
        let r = decode_response(&read_frame(&mut h.client, 1 << 20).unwrap()).unwrap();
        assert_eq!(r.error_code, Some(ErrorCode::MessageTooLarge));
        assert_eq!(
            h.server.join().unwrap().outcome,
            ConnectionOutcome::MessageTooLarge
        );
    }

    #[test]
    fn idle_connection_times_out() {
        let h = start(peer(7), uid_policy(7));
        let t = Instant::now();
        let rep = h.server.join().unwrap();
        assert_eq!(rep.outcome, ConnectionOutcome::IdleTimeout);
        assert!(t.elapsed() >= Duration::from_millis(300));
        assert!(t.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn truncated_frame_breaks_connection() {
        let mut h = start(peer(7), uid_policy(7));
        use std::io::Write as _;
        h.client.write_all(&10u32.to_le_bytes()).unwrap();
        h.client.write_all(b"abc").unwrap();
        // Gövdenin geri kalanı hiç gelmez → çerçeve ortasında zaman aşımı.
        assert!(matches!(
            h.server.join().unwrap().outcome,
            ConnectionOutcome::Broken(_)
        ));
    }

    #[test]
    fn too_many_errors_close_connection() {
        let mut h = start(peer(7), uid_policy(7));
        for _ in 0..3 {
            let r = send(&mut h.client, b"{bad");
            assert_eq!(r.error_code, Some(ErrorCode::InvalidMessage));
        }
        assert_eq!(
            h.server.join().unwrap().outcome,
            ConnectionOutcome::TooManyErrors
        );
    }

    #[test]
    fn shutdown_flag_ends_connection_promptly() {
        let h = start(peer(7), uid_policy(7));
        thread::sleep(Duration::from_millis(60));
        let t = Instant::now();
        h.shutdown.store(true, Ordering::SeqCst);
        assert_eq!(
            h.server.join().unwrap().outcome,
            ConnectionOutcome::Shutdown
        );
        assert!(t.elapsed() < Duration::from_millis(500));
    }
}
