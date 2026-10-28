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
//!   yalnız salt okunur `GET_STATUS` / `HEALTH_CHECK` gönderir.
//! - Helper → desktop: [`BridgeEvent`] (bağlantı durumu ve agent anlık görüntüsü).
//! - Kapanış: desktop stdin'i kapatır (EOF) → helper temiz çıkar. Desktop
//!   çökerse işletim sistemi boruyu kapatır; helper sahipsiz kalmaz.

use crate::message::{HealthCheckResult, RunMode};
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
    /// İki isteğin toplam gidiş-dönüş süresi.
    pub round_trip_ms: u64,
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
    Ok(line.trim_end_matches(['\r', '\n']))
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
