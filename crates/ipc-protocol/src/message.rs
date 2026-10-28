use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// Desteklenen tek protokol sürümü.
pub const PROTOCOL_VERSION: u16 = 1;
/// Tek bir çerçevenin (istek veya yanıt) en büyük JSON gövde boyutu.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024;
/// `message_id` en büyük uzunluğu.
pub const MAX_MESSAGE_ID_LEN: usize = 64;

/// P1.A'da izin verilen komutların tam listesi (beyaz liste).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandType {
    Ping,
    GetStatus,
    HealthCheck,
}

impl CommandType {
    pub const ALL: [CommandType; 3] = [
        CommandType::Ping,
        CommandType::GetStatus,
        CommandType::HealthCheck,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            CommandType::Ping => "PING",
            CommandType::GetStatus => "GET_STATUS",
            CommandType::HealthCheck => "HEALTH_CHECK",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol_version: u16,
    pub message_id: String,
    pub command: CommandType,
    /// Gönderenin duvar saati (Unix ms). Yalnız bilgi amaçlı; yetkilendirme
    /// veya tekrar tespiti için kullanılmaz (saat manipüle edilebilir).
    pub timestamp_ms: u64,
}

impl Request {
    pub fn new(message_id: impl Into<String>, command: CommandType) -> Self {
        Request {
            protocol_version: PROTOCOL_VERSION,
            message_id: message_id.into(),
            command,
            timestamp_ms: now_ms(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    UnsupportedVersion,
    UnknownCommand,
    InvalidMessage,
    MessageTooLarge,
    DuplicateMessageId,
    Unauthorized,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunMode {
    Console,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub name: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Payload {
    Pong,
    Status {
        agent_version: String,
        run_mode: RunMode,
        uptime_secs: u64,
        connected_clients: u32,
    },
    Health {
        healthy: bool,
        checks: Vec<HealthCheckResult>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub protocol_version: u16,
    /// İsteğin kimliği; istek ayrıştırılamadıysa `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    pub status: ResponseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    pub timestamp_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Payload>,
}

impl Response {
    pub fn ok(message_id: &str, payload: Payload) -> Self {
        Response {
            protocol_version: PROTOCOL_VERSION,
            message_id: Some(message_id.to_owned()),
            status: ResponseStatus::Ok,
            error_code: None,
            timestamp_ms: now_ms(),
            payload: Some(payload),
        }
    }

    pub fn error(message_id: Option<String>, code: ErrorCode) -> Self {
        Response {
            protocol_version: PROTOCOL_VERSION,
            message_id,
            status: ResponseStatus::Error,
            error_code: Some(code),
            timestamp_ms: now_ms(),
            payload: None,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Yanıt türleri her zaman serileştirilebilir; hata yolu yok.
        serde_json::to_vec(self).expect("response serialization")
    }
}

/// İstek çözümlenemediğinde dönen hata; mümkünse yankılanacak `message_id` taşır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestError {
    pub code: ErrorCode,
    pub message_id: Option<String>,
}

pub fn is_valid_message_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_MESSAGE_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Bir istek gövdesini katı biçimde çözer.
///
/// Sıra: JSON nesnesi mi → sürüm → komut beyaz listede mi → alanların tamamı
/// ve fazlası (deny_unknown_fields) → `message_id` biçimi.
pub fn decode_request(bytes: &[u8]) -> Result<Request, RequestError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(RequestError {
            code: ErrorCode::MessageTooLarge,
            message_id: None,
        });
    }
    let invalid = |message_id| RequestError {
        code: ErrorCode::InvalidMessage,
        message_id,
    };

    let value: Value = serde_json::from_slice(bytes).map_err(|_| invalid(None))?;
    let obj = value.as_object().ok_or_else(|| invalid(None))?;

    // Yalnızca geçerli biçimdeki kimlik yankılanır; aksi halde yanıtta kimlik yok.
    let echo_id = obj
        .get("message_id")
        .and_then(Value::as_str)
        .filter(|id| is_valid_message_id(id))
        .map(str::to_owned);

    match obj.get("protocol_version").and_then(Value::as_u64) {
        Some(v) if v == u64::from(PROTOCOL_VERSION) => {}
        Some(_) => {
            return Err(RequestError {
                code: ErrorCode::UnsupportedVersion,
                message_id: echo_id,
            })
        }
        None => return Err(invalid(echo_id)),
    }

    match obj.get("command").and_then(Value::as_str) {
        Some(c) if CommandType::parse(c).is_some() => {}
        Some(_) => {
            return Err(RequestError {
                code: ErrorCode::UnknownCommand,
                message_id: echo_id,
            })
        }
        None => return Err(invalid(echo_id)),
    }

    let req: Request = serde_json::from_value(value).map_err(|_| invalid(echo_id.clone()))?;
    if !is_valid_message_id(&req.message_id) {
        return Err(invalid(None));
    }
    Ok(req)
}

pub fn decode_response(bytes: &[u8]) -> Result<Response, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(v: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&v).unwrap()
    }

    #[test]
    fn valid_ping_round_trips() {
        let req = Request::new("m-1", CommandType::Ping);
        let bytes = serde_json::to_vec(&req).unwrap();
        assert_eq!(decode_request(&bytes).unwrap(), req);
    }

    #[test]
    fn all_whitelisted_commands_decode() {
        for c in CommandType::ALL {
            let b = body(serde_json::json!({
                "protocol_version": 1, "message_id": "a", "command": c.as_str(), "timestamp_ms": 1
            }));
            assert_eq!(decode_request(&b).unwrap().command, c);
        }
    }

    #[test]
    fn unknown_command_is_rejected_with_echoed_id() {
        for cmd in ["LOCK", "UNLOCK", "PROCESS_KILL", "ping", ""] {
            let b = body(serde_json::json!({
                "protocol_version": 1, "message_id": "x-1", "command": cmd, "timestamp_ms": 1
            }));
            let e = decode_request(&b).unwrap_err();
            assert_eq!(e.code, ErrorCode::UnknownCommand, "command {cmd}");
            assert_eq!(e.message_id.as_deref(), Some("x-1"));
        }
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let b = body(serde_json::json!({
            "protocol_version": 2, "message_id": "v", "command": "PING", "timestamp_ms": 1
        }));
        assert_eq!(
            decode_request(&b).unwrap_err().code,
            ErrorCode::UnsupportedVersion
        );
    }

    #[test]
    fn malformed_inputs_are_invalid() {
        let cases: Vec<Vec<u8>> = vec![
            b"not json".to_vec(),
            b"[]".to_vec(),
            b"{}".to_vec(),
            b"".to_vec(),
            body(serde_json::json!({"protocol_version": 1, "command": "PING", "timestamp_ms": 1})),
            body(serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "PING"})),
            body(
                serde_json::json!({"protocol_version": "1", "message_id": "a", "command": "PING", "timestamp_ms": 1}),
            ),
            body(
                serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "PING", "timestamp_ms": -5}),
            ),
            body(
                serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "PING", "timestamp_ms": 1, "extra": true}),
            ),
            body(
                serde_json::json!({"protocol_version": 1, "message_id": "", "command": "PING", "timestamp_ms": 1}),
            ),
            body(
                serde_json::json!({"protocol_version": 1, "message_id": "bad id/..", "command": "PING", "timestamp_ms": 1}),
            ),
            body(
                serde_json::json!({"protocol_version": 1, "message_id": "a".repeat(65), "command": "PING", "timestamp_ms": 1}),
            ),
            body(
                serde_json::json!({"protocol_version": 1, "message_id": 7, "command": "PING", "timestamp_ms": 1}),
            ),
        ];
        for (i, c) in cases.iter().enumerate() {
            assert_eq!(
                decode_request(c).unwrap_err().code,
                ErrorCode::InvalidMessage,
                "case {i}"
            );
        }
    }

    #[test]
    fn invalid_message_id_is_never_echoed() {
        let b = body(serde_json::json!({
            "protocol_version": 1, "message_id": "<script>", "command": "NOPE", "timestamp_ms": 1
        }));
        let e = decode_request(&b).unwrap_err();
        assert_eq!(e.code, ErrorCode::UnknownCommand);
        assert_eq!(e.message_id, None);
    }

    #[test]
    fn oversized_body_is_rejected() {
        let big = vec![b' '; MAX_MESSAGE_BYTES + 1];
        assert_eq!(
            decode_request(&big).unwrap_err().code,
            ErrorCode::MessageTooLarge
        );
    }

    #[test]
    fn response_round_trips() {
        let r = Response::ok(
            "m",
            Payload::Status {
                agent_version: "0.1.0".into(),
                run_mode: RunMode::Console,
                uptime_secs: 3,
                connected_clients: 1,
            },
        );
        assert_eq!(decode_response(&r.to_bytes()).unwrap(), r);
        let e = Response::error(None, ErrorCode::Unauthorized);
        let json = String::from_utf8(e.to_bytes()).unwrap();
        assert!(json.contains("\"error_code\":\"UNAUTHORIZED\""));
        assert!(!json.contains("message_id"));
    }
}
