//! Desktop ↔ Session Helper yerel köprüsü (P1.B, ADR-0013).
//!
//! Masaüstü uygulaması agent'a **doğrudan bağlanmaz**: agent yalnız kendi
//! dizinindeki `kindora-helper.exe`'yi kabul eder (ADR-0011, BUG-003). Desktop,
//! helper'ı `kindora-helper bridge` ile alt süreç olarak başlatır ve onunla
//! yalnız bu iki süreç arasındaki anonim stdio boruları üzerinden konuşur:
//!
//! - Taşıma: satır başına bir JSON nesnesi (JSON Lines), en fazla
//!   [`MAX_BRIDGE_LINE_BYTES`] bayt. Yeni bir ağ ucu veya adlandırılmış pipe yok.
//! - Desktop → helper: yalnız [`BridgeRequest`] (bugün tek işlem: `refresh`).
//!   Agent komutu adı, argüman veya keyfî metin taşınmaz; helper agent'a
//!   salt okunur `GET_STATUS` / `HEALTH_CHECK` / `GET_SCREEN_TIME` ve kendi
//!   ölçtüğü `REPORT_ACTIVITY` gönderir (P1.C). Masaüstü kural değiştiremez.
//! - Helper → desktop: [`BridgeEvent`] (bağlantı durumu ve agent anlık görüntüsü).
//! - Kapanış: desktop stdin'i kapatır (EOF) → helper temiz çıkar. Desktop
//!   çökerse işletim sistemi boruyu kapatır; helper sahipsiz kalmaz.

use crate::message::{HealthCheckResult, RunMode, ScreenTimeStatus};
use serde::{Deserialize, Serialize};

pub const BRIDGE_VERSION: u16 = 1;
pub const MAX_BRIDGE_LINE_BYTES: usize = 4096;

/// Desktop'tan helper'a izin verilen istekler (beyaz liste).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BridgeRequest {
    /// Agent durumunu hemen yeniden oku; bağlantı yoksa beklemeden yeniden dene.
    Refresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    Connecting,
    Connected,
    Disconnected,
    /// Agent bu helper'ı yetkilendirmedi (ör. farklı dizin). Desktop politikayı
    /// aşmaya çalışmaz; yalnız durumu gösterir.
    Unauthorized,
}

/// Gerçek agent yanıtlarından (`GET_STATUS` + `HEALTH_CHECK`) oluşturulur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub agent_version: String,
    pub run_mode: RunMode,
    pub uptime_secs: u64,
    pub connected_clients: u32,
    pub healthy: bool,
    pub checks: Vec<HealthCheckResult>,
    /// Helper'ın yanıtı aldığı an (Unix ms).
    pub received_at_ms: u64,
    /// Döngüdeki isteklerin toplam gidiş-dönüş süresi.
    pub round_trip_ms: u64,
    /// `GET_SCREEN_TIME` yanıtı (P1.C). Agent sağlamazsa `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_time: Option<Box<ScreenTimeStatus>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeEvent {
    Hello {
        bridge_version: u16,
        helper_version: String,
        pid: u32,
    },
    Link {
        state: LinkState,
        /// Art arda başarısız bağlantı denemesi sayısı (bağlanınca 0).
        attempt: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_in_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Snapshot(AgentSnapshot),
    /// Desktop'tan gelen satır reddedildi (ayrıştırılamadı / izinli değil).
    RequestRejected {
        reason: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeLineError {
    TooLong,
    Invalid(String),
}

impl std::fmt::Display for BridgeLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeLineError::TooLong => write!(f, "line too long"),
            BridgeLineError::Invalid(e) => write!(f, "invalid line: {e}"),
        }
    }
}

fn check_len(line: &str) -> Result<&str, BridgeLineError> {
    if line.len() > MAX_BRIDGE_LINE_BYTES {
        return Err(BridgeLineError::TooLong);
    }
    // Windows'ta .NET `Process.StandardInput` ilk satırın başına UTF-8 BOM
    // ekleyebilir (BUG-005); yalnız bu görünmez önek atılır.
    Ok(line
        .trim_start_matches('\u{feff}')
        .trim_end_matches(['\r', '\n']))
}

/// Katı ayrıştırma: yalnız `{"op":"<izinli>"}`. Serde'nin etiketli enum'larında
/// `deny_unknown_fields` ek alanları reddetmediği için ara yapı kullanılır.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequest {
    op: String,
}

pub fn decode_bridge_request(line: &str) -> Result<BridgeRequest, BridgeLineError> {
    let raw: RawRequest = serde_json::from_str(check_len(line)?)
        .map_err(|e| BridgeLineError::Invalid(e.to_string()))?;
    match raw.op.as_str() {
        "refresh" => Ok(BridgeRequest::Refresh),
        other => Err(BridgeLineError::Invalid(format!("unsupported op: {other}"))),
    }
}

pub fn decode_bridge_event(line: &str) -> Result<BridgeEvent, BridgeLineError> {
    serde_json::from_str(check_len(line)?).map_err(|e| BridgeLineError::Invalid(e.to_string()))
}

/// Tek satırlık JSON (sonunda `\n` yok).
pub fn encode_bridge_line<T: Serialize>(value: &T) -> String {
    // Bu modüldeki türler her zaman serileştirilebilir.
    serde_json::to_string(value).expect("bridge types serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_is_the_only_accepted_request() {
        assert_eq!(
            decode_bridge_request(r#"{"op":"refresh"}"#),
            Ok(BridgeRequest::Refresh)
        );
        assert_eq!(
            decode_bridge_request("{\"op\":\"refresh\"}\r\n"),
            Ok(BridgeRequest::Refresh)
        );
        assert_eq!(
            decode_bridge_request("\u{feff}{\"op\":\"refresh\"}\r\n"),
            Ok(BridgeRequest::Refresh),
            ".NET StandardInput BOM"
        );
        for bad in [
            r#"{"op":"LOCK"}"#,
            r#"{"op":"refresh","command":"PROCESS_KILL"}"#,
            r#"{"op":"exec","args":["cmd.exe"]}"#,
            r#"{"command":"PING"}"#,
            "refresh",
            "",
        ] {
            assert!(
                matches!(decode_bridge_request(bad), Err(BridgeLineError::Invalid(_))),
                "{bad}"
            );
        }
        let long = format!(r#"{{"op":"refresh","pad":"{}"}}"#, "x".repeat(5000));
        assert_eq!(decode_bridge_request(&long), Err(BridgeLineError::TooLong));
    }

    #[test]
    fn largest_snapshot_line_fits_the_bridge_limit() {
        use crate::message::*;
        let checks = (0..6)
            .map(|i| HealthCheckResult {
                name: format!("check_{i}_with_a_long_name"),
                ok: false,
                detail: Some("x".repeat(64)),
            })
            .collect();
        let policy = PolicyInfo {
            version: 1_000_000,
            source: PolicySource::DevFile,
            status: PolicyStatus::InvalidKeptLast,
            daily_limit_secs: 86_400,
            break_interval_secs: 14_400,
            break_duration_secs: 3_600,
            first_warning_secs: 1_800,
            final_warning_secs: 600,
            idle_threshold_secs: 3_600,
        };
        let st = ScreenTimeStatus {
            day: "2026-10-29".into(),
            state: UsageState::SessionLocked,
            today: DayUsage {
                active_secs: u64::from(u32::MAX),
                idle_secs: u64::from(u32::MAX),
                locked_secs: u64::from(u32::MAX),
                suspended_secs: u64::from(u32::MAX),
                unknown_secs: u64::from(u32::MAX),
            },
            daily_limit_secs: 86_400,
            remaining_secs: 86_400,
            limit_reached: true,
            break_phase: BreakPhase::BreakComplete,
            next_break_in_secs: Some(14_400),
            break_remaining_secs: Some(3_600),
            notices: vec![
                Notice {
                    id: format!("b{}-done", u64::MAX),
                    kind: NoticeKind::BreakCompleted,
                },
                Notice {
                    id: "2026-10-29-limit".into(),
                    kind: NoticeKind::LimitReached,
                },
            ],
            policy,
            measurement: MeasurementInfo {
                source: MeasurementSource::SessionHelper,
                last_report_age_ms: Some(u64::MAX),
                unknown_reason: Some("LOCK_STATE_QUERY_FAILED".into()),
            },
            storage: StorageInfo {
                persistent: true,
                schema_version: 99,
                recovered: true,
            },
            clock_adjusted_today: true,
            computed_at_ms: u64::MAX,
        };
        let line = encode_bridge_line(&BridgeEvent::Snapshot(AgentSnapshot {
            agent_version: "0.1.0-dev+long".into(),
            run_mode: RunMode::Service,
            uptime_secs: u64::MAX,
            connected_clients: u32::MAX,
            healthy: false,
            checks,
            received_at_ms: u64::MAX,
            round_trip_ms: u64::MAX,
            screen_time: Some(Box::new(st)),
        }));
        assert!(line.len() < MAX_BRIDGE_LINE_BYTES, "{}", line.len());
        assert!(decode_bridge_event(&line).is_ok());
    }

    #[test]
    fn request_encoding_matches_decoder() {
        let line = encode_bridge_line(&BridgeRequest::Refresh);
        assert_eq!(line, r#"{"op":"refresh"}"#);
        assert_eq!(decode_bridge_request(&line), Ok(BridgeRequest::Refresh));
    }

    #[test]
    fn events_round_trip_as_tagged_json_lines() {
        let events = vec![
            BridgeEvent::Hello {
                bridge_version: BRIDGE_VERSION,
                helper_version: "0.1.0".into(),
                pid: 42,
            },
            BridgeEvent::Link {
                state: LinkState::Disconnected,
                attempt: 3,
                retry_in_ms: Some(2000),
                reason: Some("connect failed".into()),
            },
            BridgeEvent::Snapshot(AgentSnapshot {
                agent_version: "0.1.0".into(),
                run_mode: RunMode::Service,
                uptime_secs: 12,
                connected_clients: 1,
                healthy: true,
                checks: vec![HealthCheckResult {
                    name: "ipc_listener".into(),
                    ok: true,
                    detail: Some("accepting connections".into()),
                }],
                received_at_ms: 1,
                round_trip_ms: 2,
                screen_time: None,
            }),
        ];
        for e in events {
            let line = encode_bridge_line(&e);
            assert!(!line.contains('\n'));
            assert_eq!(decode_bridge_event(&line), Ok(e));
        }
        let link = encode_bridge_line(&BridgeEvent::Link {
            state: LinkState::Connected,
            attempt: 0,
            retry_in_ms: None,
            reason: None,
        });
        assert_eq!(link, r#"{"type":"link","state":"connected","attempt":0}"#);
    }
}
