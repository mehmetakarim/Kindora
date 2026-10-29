//! Bağlantı başına istek işleyici.

use crate::{AgentState, ConnectionLimits, AGENT_VERSION};
use ipc_protocol::{
    decode_request, CommandType, ErrorCode, HealthCheckResult, Payload, Request, Response,
};
use std::time::{Duration, Instant};

/// Bir bağlantıdan `REPORT_ACTIVITY` en sık bu aralıkla uygulanır.
pub const MIN_ACTIVITY_REPORT_GAP: Duration = Duration::from_millis(250);
use std::collections::{HashSet, VecDeque};

/// Yanıt yazıldıktan sonra bağlantının ne olacağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Continue,
    /// Hata bütçesi tükendi; bağlantı kapatılmalı.
    Close,
}

/// Tek bir bağlantının durumu: son mesaj kimlikleri ve hata sayacı.
pub struct SessionHandler {
    limits: ConnectionLimits,
    seen_order: VecDeque<String>,
    seen: HashSet<String>,
    errors: u32,
    /// İstemcinin Windows oturumu (işletim sisteminden; mesajdan değil).
    client_session: Option<u32>,
    last_activity_report: Option<Instant>,
}

impl SessionHandler {
    pub fn new(limits: ConnectionLimits) -> Self {
        SessionHandler {
            limits,
            seen_order: VecDeque::new(),
            seen: HashSet::new(),
            errors: 0,
            client_session: None,
            last_activity_report: None,
        }
    }

    pub fn with_client_session(mut self, session: Option<u32>) -> Self {
        self.client_session = session;
        self
    }

    pub fn errors(&self) -> u32 {
        self.errors
    }

    fn fail(&mut self, message_id: Option<String>, code: ErrorCode) -> (Response, Disposition) {
        self.errors += 1;
        let disposition = if self.errors >= self.limits.max_errors {
            Disposition::Close
        } else {
            Disposition::Continue
        };
        (Response::error(message_id, code), disposition)
    }

    /// `true`: kimlik yeni (kaydedildi). `false`: bu bağlantıda daha önce görüldü.
    fn remember(&mut self, id: &str) -> bool {
        if self.seen.contains(id) {
            return false;
        }
        if self.seen_order.len() >= self.limits.dedupe_window {
            if let Some(old) = self.seen_order.pop_front() {
                self.seen.remove(&old);
            }
        }
        self.seen_order.push_back(id.to_owned());
        self.seen.insert(id.to_owned());
        true
    }

    pub fn handle(&mut self, state: &AgentState, body: &[u8]) -> (Response, Disposition) {
        let req: Request = match decode_request(body) {
            Ok(r) => r,
            Err(e) => return self.fail(e.message_id, e.code),
        };
        if !self.remember(&req.message_id) {
            return self.fail(Some(req.message_id), ErrorCode::DuplicateMessageId);
        }
        let payload = match req.command {
            CommandType::Ping => Payload::Pong,
            CommandType::GetStatus => Payload::Status {
                agent_version: AGENT_VERSION.to_owned(),
                run_mode: state.run_mode,
                uptime_secs: state.uptime().as_secs(),
                connected_clients: state.connected_clients(),
            },
            CommandType::HealthCheck => health(state),
            CommandType::ReportActivity => {
                let Some(service) = state.screen_time() else {
                    return self.fail(Some(req.message_id), ErrorCode::Unavailable);
                };
                // decode_request bu komutta `activity` alanını zorunlu kılar.
                let Some(report) = req.activity else {
                    return self.fail(Some(req.message_id), ErrorCode::InvalidMessage);
                };
                let now = Instant::now();
                let throttled = self
                    .last_activity_report
                    .is_some_and(|t| now.duration_since(t) < MIN_ACTIVITY_REPORT_GAP);
                let applied = !throttled && service.report_activity(self.client_session, report);
                if !throttled {
                    self.last_activity_report = Some(now);
                }
                Payload::ActivityAck { applied }
            }
            CommandType::GetScreenTime => match state.screen_time().and_then(|s| s.status()) {
                Some(status) => Payload::ScreenTime(Box::new(status)),
                None => return self.fail(Some(req.message_id), ErrorCode::Unavailable),
            },
        };
        (
            Response::ok(&req.message_id, payload),
            Disposition::Continue,
        )
    }

    pub fn reject_oversized(&mut self) -> Response {
        self.errors += 1;
        Response::error(None, ErrorCode::MessageTooLarge)
    }
}

/// Süreç içi durum + (varsa) kullanım süresi motoru ve depolama. Bulut
/// bağlantısı henüz yok.
fn health(state: &AgentState) -> Payload {
    let mut checks = vec![
        HealthCheckResult {
            name: "ipc_listener".into(),
            ok: true,
            detail: Some("accepting connections".into()),
        },
        HealthCheckResult {
            name: "connected_clients".into(),
            ok: true,
            detail: Some(state.connected_clients().to_string()),
        },
    ];
    if let Some(st) = state.screen_time() {
        checks.extend(st.health());
    }
    Payload::Health {
        healthy: checks.iter().all(|c| c.ok),
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScreenTimeService;
    use ipc_protocol::{ActivityReport, ResponseStatus, RunMode, ScreenTimeStatus};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct FakeScreenTime {
        reports: Mutex<Vec<(Option<u32>, ActivityReport)>>,
    }

    impl ScreenTimeService for FakeScreenTime {
        fn report_activity(&self, s: Option<u32>, r: ActivityReport) -> bool {
            self.reports.lock().unwrap().push((s, r));
            s == Some(1)
        }
        fn status(&self) -> Option<ScreenTimeStatus> {
            None
        }
        fn health(&self) -> Vec<HealthCheckResult> {
            vec![HealthCheckResult {
                name: "screen_time".into(),
                ok: false,
                detail: Some("fake".into()),
            }]
        }
    }

    fn activity(id: &str, idle: Option<u64>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "protocol_version": 1, "message_id": id, "command": "REPORT_ACTIVITY",
            "timestamp_ms": 1, "activity": {"idle_ms": idle}
        }))
        .unwrap()
    }

    #[test]
    fn screen_time_commands_without_engine_are_unavailable() {
        let (s, mut h) = setup();
        let (r, _) = h.handle(&s, &activity("a-1", Some(5)));
        assert_eq!(r.error_code, Some(ErrorCode::Unavailable));
        let (r, _) = h.handle(&s, &req("g-1", "GET_SCREEN_TIME"));
        assert_eq!(r.error_code, Some(ErrorCode::Unavailable));
        assert_eq!(r.message_id.as_deref(), Some("g-1"));
    }

    #[test]
    fn activity_report_carries_os_session_and_is_throttled() {
        let fake = Arc::new(FakeScreenTime::default());
        let s = AgentState::new(RunMode::Service).with_screen_time(fake.clone());
        let mut h = SessionHandler::new(ConnectionLimits::default()).with_client_session(Some(1));
        let (r, _) = h.handle(&s, &activity("a-1", Some(1500)));
        assert_eq!(r.payload, Some(Payload::ActivityAck { applied: true }));
        // Aynı bağlantıdan hemen gelen ikinci rapor uygulanmaz (hata değil).
        let (r, _) = h.handle(&s, &activity("a-2", None));
        assert_eq!(r.payload, Some(Payload::ActivityAck { applied: false }));
        assert_eq!(fake.reports.lock().unwrap().len(), 1);
        assert_eq!(
            fake.reports.lock().unwrap()[0],
            (
                Some(1),
                ActivityReport {
                    idle_ms: Some(1500)
                }
            )
        );

        // Başka oturumdan gelen rapor sayılmaz.
        let mut other =
            SessionHandler::new(ConnectionLimits::default()).with_client_session(Some(2));
        let (r, _) = other.handle(&s, &activity("a-3", Some(1)));
        assert_eq!(r.payload, Some(Payload::ActivityAck { applied: false }));

        // Sağlık denetimi motor satırını içerir; motor sağlıksızsa genel sonuç da.
        let (r, _) = h.handle(&s, &req("h-1", "HEALTH_CHECK"));
        match r.payload {
            Some(Payload::Health { healthy, checks }) => {
                assert!(!healthy);
                assert!(checks.iter().any(|c| c.name == "screen_time"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    fn req(id: &str, cmd: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "protocol_version": 1, "message_id": id, "command": cmd, "timestamp_ms": 1
        }))
        .unwrap()
    }

    fn setup() -> (AgentState, SessionHandler) {
        (
            AgentState::new(RunMode::Console),
            SessionHandler::new(ConnectionLimits::default()),
        )
    }

    #[test]
    fn ping_returns_pong_and_echoes_id() {
        let (s, mut h) = setup();
        let (r, d) = h.handle(&s, &req("p-1", "PING"));
        assert_eq!(r.status, ResponseStatus::Ok);
        assert_eq!(r.message_id.as_deref(), Some("p-1"));
        assert_eq!(r.payload, Some(Payload::Pong));
        assert_eq!(d, Disposition::Continue);
    }

    #[test]
    fn status_and_health() {
        let (s, mut h) = setup();
        let _g = s.register_client();
        let (r, _) = h.handle(&s, &req("s-1", "GET_STATUS"));
        match r.payload {
            Some(Payload::Status {
                connected_clients,
                run_mode,
                ..
            }) => {
                assert_eq!(connected_clients, 1);
                assert_eq!(run_mode, RunMode::Console);
            }
            other => panic!("unexpected {other:?}"),
        }
        let (r, _) = h.handle(&s, &req("h-1", "HEALTH_CHECK"));
        assert!(matches!(
            r.payload,
            Some(Payload::Health { healthy: true, .. })
        ));
    }

    #[test]
    fn unknown_and_invalid_are_rejected() {
        let (s, mut h) = setup();
        let (r, _) = h.handle(&s, &req("u-1", "LOCK"));
        assert_eq!(r.error_code, Some(ErrorCode::UnknownCommand));
        let (r, _) = h.handle(&s, b"{garbage");
        assert_eq!(r.error_code, Some(ErrorCode::InvalidMessage));
        assert_eq!(r.message_id, None);
    }

    #[test]
    fn duplicate_message_id_is_rejected() {
        let (s, mut h) = setup();
        assert_eq!(
            h.handle(&s, &req("d-1", "PING")).0.status,
            ResponseStatus::Ok
        );
        let (r, _) = h.handle(&s, &req("d-1", "PING"));
        assert_eq!(r.error_code, Some(ErrorCode::DuplicateMessageId));
        assert_eq!(r.message_id.as_deref(), Some("d-1"));
    }

    #[test]
    fn dedupe_window_is_bounded() {
        let s = AgentState::new(RunMode::Console);
        let mut h = SessionHandler::new(ConnectionLimits {
            dedupe_window: 2,
            ..Default::default()
        });
        for id in ["a", "b", "c"] {
            assert_eq!(h.handle(&s, &req(id, "PING")).0.status, ResponseStatus::Ok);
        }
        // "a" pencereden düştü; yeniden kabul edilir.
        assert_eq!(h.handle(&s, &req("a", "PING")).0.status, ResponseStatus::Ok);
        assert_eq!(
            h.handle(&s, &req("c", "PING")).0.error_code,
            Some(ErrorCode::DuplicateMessageId)
        );
    }

    #[test]
    fn error_budget_closes_connection() {
        let s = AgentState::new(RunMode::Console);
        let mut h = SessionHandler::new(ConnectionLimits {
            max_errors: 3,
            ..Default::default()
        });
        assert_eq!(h.handle(&s, b"x").1, Disposition::Continue);
        assert_eq!(h.handle(&s, b"x").1, Disposition::Continue);
        assert_eq!(h.handle(&s, b"x").1, Disposition::Close);
    }

    #[test]
    fn client_counter_follows_guards() {
        let s = AgentState::new(RunMode::Service);
        {
            let _a = s.register_client();
            let _b = s.register_client();
            assert_eq!(s.connected_clients(), 2);
        }
        assert_eq!(s.connected_clients(), 0);
    }
}
