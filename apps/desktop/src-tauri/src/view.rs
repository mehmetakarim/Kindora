//! Ön yüze gönderilen agent durumu ve köprü olaylarından türetilmesi.
//!
//! Tek doğruluk kaynağı helper köprüsünden gelen gerçek olaylardır; bağlantı
//! yokken agent bilgisi ve kullanım süresi **gösterilmez** (`agent: None`,
//! `screen_time: None`), yalnız son başarılı iletişim zamanı korunur. Masaüstü
//! kullanım süresini kendisi hesaplamaz; agent'ın hesapladığını gösterir.

use ipc_protocol::bridge::{AgentSnapshot, BridgeEvent, LinkState};
use ipc_protocol::{HealthCheckResult, RunMode, ScreenTimeStatus};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperState {
    /// Helper başlatılıyor / ilk olay bekleniyor.
    Starting,
    Running,
    /// `kindora-helper.exe` masaüstü uygulamasının yanında yok.
    Missing,
    /// Helper beklenmedik biçimde kapandı; yeniden başlatılacak.
    Exited,
    /// Uygulama kapanıyor.
    Stopping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentInfo {
    pub version: String,
    pub run_mode: RunMode,
    pub uptime_secs: u64,
    pub connected_clients: u32,
    pub healthy: bool,
    pub checks: Vec<HealthCheckResult>,
    pub round_trip_ms: u64,
}

impl From<AgentSnapshot> for AgentInfo {
    fn from(s: AgentSnapshot) -> Self {
        AgentInfo {
            version: s.agent_version,
            run_mode: s.run_mode,
            uptime_secs: s.uptime_secs,
            connected_clients: s.connected_clients,
            healthy: s.healthy,
            checks: s.checks,
            round_trip_ms: s.round_trip_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentView {
    pub helper: HelperState,
    pub helper_restarts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helper_detail: Option<String>,
    pub link: LinkState,
    pub attempt: u32,
    pub retry_in_ms: Option<u64>,
    pub link_reason: Option<String>,
    pub agent: Option<AgentInfo>,
    /// Agent'ın hesapladığı kullanım süresi (P1.C). Bağlantı yokken `None`.
    pub screen_time: Option<ScreenTimeStatus>,
    pub last_success_ms: Option<u64>,
    pub closing: bool,
    pub updated_ms: u64,
}

impl AgentView {
    pub fn new(now_ms: u64) -> Self {
        AgentView {
            helper: HelperState::Starting,
            helper_restarts: 0,
            helper_detail: None,
            link: LinkState::Connecting,
            attempt: 0,
            retry_in_ms: None,
            link_reason: None,
            agent: None,
            screen_time: None,
            last_success_ms: None,
            closing: false,
            updated_ms: now_ms,
        }
    }

    fn lose_link(&mut self, state: LinkState, reason: Option<String>) {
        self.link = state;
        self.agent = None;
        self.screen_time = None;
        self.retry_in_ms = None;
        self.link_reason = reason;
    }

    /// Bir köprü olayını uygular. Ön yüze bildirim gerekiyorsa `true`.
    pub fn apply(&mut self, event: BridgeEvent, now_ms: u64) -> bool {
        match event {
            BridgeEvent::Hello { .. } => {
                self.helper = HelperState::Running;
                self.helper_detail = None;
            }
            BridgeEvent::Link {
                state,
                attempt,
                retry_in_ms,
                reason,
            } => {
                // Bağlantı yokken her yeniden deneme "connecting" yayınlar; ekranın
                // her denemede gidip gelmemesi için kopuk durum korunur.
                if state == LinkState::Connecting
                    && matches!(self.link, LinkState::Disconnected | LinkState::Unauthorized)
                {
                    return false;
                }
                if state != LinkState::Connected {
                    self.agent = None;
                    self.screen_time = None;
                }
                self.link = state;
                self.attempt = attempt;
                self.retry_in_ms = retry_in_ms;
                self.link_reason = reason;
            }
            BridgeEvent::Snapshot(s) => {
                self.link = LinkState::Connected;
                self.attempt = 0;
                self.retry_in_ms = None;
                self.link_reason = None;
                self.last_success_ms = Some(s.received_at_ms);
                self.screen_time = s.screen_time.clone().map(|b| *b);
                self.agent = Some(s.into());
            }
            // Masaüstü yalnız izinli istek gönderir; ret yalnız kayda geçer.
            BridgeEvent::RequestRejected { .. } => return false,
        }
        self.updated_ms = now_ms;
        true
    }

    pub fn helper_spawned(&mut self, now_ms: u64) {
        self.helper = HelperState::Starting;
        self.helper_detail = None;
        self.updated_ms = now_ms;
    }

    pub fn helper_exited(&mut self, detail: String, now_ms: u64) {
        self.helper = HelperState::Exited;
        self.helper_restarts = self.helper_restarts.saturating_add(1);
        self.helper_detail = Some(detail);
        self.lose_link(
            LinkState::Disconnected,
            Some("helper process exited".into()),
        );
        self.updated_ms = now_ms;
    }

    pub fn helper_missing(&mut self, detail: String, now_ms: u64) {
        self.helper = HelperState::Missing;
        self.helper_detail = Some(detail);
        self.lose_link(LinkState::Disconnected, Some("helper not found".into()));
        self.updated_ms = now_ms;
    }

    pub fn set_closing(&mut self, now_ms: u64) {
        self.closing = true;
        self.helper = HelperState::Stopping;
        self.updated_ms = now_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(at: u64) -> BridgeEvent {
        BridgeEvent::Snapshot(AgentSnapshot {
            agent_version: "0.1.0".into(),
            run_mode: RunMode::Service,
            uptime_secs: 5,
            connected_clients: 1,
            healthy: true,
            checks: vec![],
            received_at_ms: at,
            round_trip_ms: 3,
            screen_time: None,
        })
    }

    fn link(state: LinkState, attempt: u32, retry: Option<u64>) -> BridgeEvent {
        BridgeEvent::Link {
            state,
            attempt,
            retry_in_ms: retry,
            reason: Some("x".into()),
        }
    }

    #[test]
    fn starts_without_agent_data() {
        let v = AgentView::new(1);
        assert_eq!(v.link, LinkState::Connecting);
        assert!(v.agent.is_none());
        assert!(v.last_success_ms.is_none());
    }

    #[test]
    fn snapshot_connects_and_disconnect_clears_agent_but_keeps_last_success() {
        let mut v = AgentView::new(0);
        v.apply(
            BridgeEvent::Hello {
                bridge_version: 1,
                helper_version: "0.1.0".into(),
                pid: 7,
            },
            1,
        );
        assert_eq!(v.helper, HelperState::Running);
        v.apply(snapshot(100), 2);
        assert_eq!(v.link, LinkState::Connected);
        assert_eq!(v.agent.as_ref().unwrap().run_mode, RunMode::Service);
        assert_eq!(v.last_success_ms, Some(100));

        v.apply(link(LinkState::Disconnected, 1, Some(250)), 3);
        assert_eq!(v.link, LinkState::Disconnected);
        assert!(
            v.agent.is_none(),
            "bağlantı yokken agent verisi gösterilmez"
        );
        assert_eq!(v.last_success_ms, Some(100));
        assert_eq!(v.retry_in_ms, Some(250));

        assert!(
            !v.apply(link(LinkState::Connecting, 1, None), 4),
            "kopukken yeniden deneme ekranı değiştirmez"
        );
        assert_eq!(v.link, LinkState::Disconnected);
        assert_eq!(v.retry_in_ms, Some(250));
        v.apply(snapshot(200), 5);
        assert_eq!(v.attempt, 0);
        assert_eq!(v.last_success_ms, Some(200));
    }

    #[test]
    fn unauthorized_is_reported_without_agent_data() {
        let mut v = AgentView::new(0);
        v.apply(snapshot(1), 1);
        v.apply(link(LinkState::Unauthorized, 1, Some(30_000)), 2);
        assert_eq!(v.link, LinkState::Unauthorized);
        assert!(v.agent.is_none());
    }

    #[test]
    fn helper_exit_and_missing_drop_the_link() {
        let mut v = AgentView::new(0);
        v.apply(snapshot(1), 1);
        v.helper_exited("exit code 1".into(), 2);
        assert_eq!(v.helper, HelperState::Exited);
        assert_eq!(v.helper_restarts, 1);
        assert_eq!(v.link, LinkState::Disconnected);
        assert!(v.agent.is_none());
        v.helper_spawned(3);
        assert_eq!(v.helper, HelperState::Starting);
        v.helper_missing("C:\\x\\kindora-helper.exe".into(), 4);
        assert_eq!(v.helper, HelperState::Missing);
        assert_eq!(v.link, LinkState::Disconnected);
    }

    #[test]
    fn rejected_request_does_not_change_view() {
        let mut v = AgentView::new(0);
        let before = v.clone();
        assert!(!v.apply(BridgeEvent::RequestRejected { reason: "x".into() }, 9));
        assert_eq!(v, before);
    }

    #[test]
    fn screen_time_is_shown_only_while_connected() {
        use ipc_protocol::*;
        let st = ScreenTimeStatus {
            day: "2026-10-29".into(),
            state: UsageState::Active,
            today: DayUsage {
                active_secs: 600,
                ..Default::default()
            },
            daily_limit_secs: 7200,
            remaining_secs: 6600,
            limit_reached: false,
            break_phase: BreakPhase::Normal,
            next_break_in_secs: Some(1200),
            break_remaining_secs: None,
            notices: vec![],
            policy: PolicyInfo {
                version: 0,
                source: PolicySource::BuiltinDefault,
                status: PolicyStatus::Ok,
                daily_limit_secs: 7200,
                break_interval_secs: 1800,
                break_duration_secs: 300,
                first_warning_secs: 300,
                final_warning_secs: 60,
                idle_threshold_secs: 300,
            },
            measurement: MeasurementInfo {
                source: MeasurementSource::SessionHelper,
                last_report_age_ms: Some(100),
                unknown_reason: None,
                reporter: None,
                competing_reports: 0,
            },
            storage: StorageInfo {
                persistent: true,
                schema_version: 1,
                recovered: false,
            },
            clock_adjusted_today: false,
            computed_at_ms: 5,
        };
        let mut v = AgentView::new(0);
        let BridgeEvent::Snapshot(mut snap) = snapshot(10) else {
            unreachable!()
        };
        snap.screen_time = Some(Box::new(st.clone()));
        v.apply(BridgeEvent::Snapshot(snap), 1);
        assert_eq!(v.screen_time.as_ref(), Some(&st));
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["screen_time"]["state"], "ACTIVE");
        assert_eq!(json["screen_time"]["today"]["active_secs"], 600);

        v.apply(link(LinkState::Disconnected, 1, Some(250)), 2);
        assert!(
            v.screen_time.is_none(),
            "agent yokken ölçüm varmış gibi gösterilmez"
        );
        v.helper_exited("x".into(), 3);
        assert!(v.screen_time.is_none());
    }

    #[test]
    fn serializes_for_the_frontend() {
        let mut v = AgentView::new(0);
        v.apply(snapshot(10), 1);
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["link"], "connected");
        assert_eq!(json["helper"], "starting");
        assert_eq!(json["agent"]["run_mode"], "SERVICE");
        assert_eq!(json["last_success_ms"], 10);
    }
}
