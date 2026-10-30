use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// Desteklenen tek protokol sürümü.
pub const PROTOCOL_VERSION: u16 = 1;
/// Tek bir çerçevenin (istek veya yanıt) en büyük JSON gövde boyutu.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024;
/// `message_id` en büyük uzunluğu.
pub const MAX_MESSAGE_ID_LEN: usize = 64;

/// İzin verilen komutların tam listesi (beyaz liste).
///
/// - P1.A: `PING`, `GET_STATUS`, `HEALTH_CHECK` (salt okunur).
/// - P1.C: `REPORT_ACTIVITY` (helper → agent: yalnız son girdiden bu yana
///   geçen süre; içerik yok) ve `GET_SCREEN_TIME` (salt okunur).
///
/// Kilit, süreç sonlandırma veya kural değiştirme komutu **yoktur**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandType {
    Ping,
    GetStatus,
    HealthCheck,
    ReportActivity,
    GetScreenTime,
}

impl CommandType {
    pub const ALL: [CommandType; 5] = [
        CommandType::Ping,
        CommandType::GetStatus,
        CommandType::HealthCheck,
        CommandType::ReportActivity,
        CommandType::GetScreenTime,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            CommandType::Ping => "PING",
            CommandType::GetStatus => "GET_STATUS",
            CommandType::HealthCheck => "HEALTH_CHECK",
            CommandType::ReportActivity => "REPORT_ACTIVITY",
            CommandType::GetScreenTime => "GET_SCREEN_TIME",
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
    /// Yalnız `REPORT_ACTIVITY` ile ve onunla zorunlu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<ActivityReport>,
}

impl Request {
    pub fn new(message_id: impl Into<String>, command: CommandType) -> Self {
        Request {
            protocol_version: PROTOCOL_VERSION,
            message_id: message_id.into(),
            command,
            timestamp_ms: now_ms(),
            activity: None,
        }
    }

    pub fn report_activity(message_id: impl Into<String>, report: ActivityReport) -> Self {
        Request {
            activity: Some(report),
            ..Request::new(message_id, CommandType::ReportActivity)
        }
    }
}

/// `idle_ms` için üst sınır (31 gün). Daha büyük değer geçersiz mesajdır.
pub const MAX_IDLE_MS: u64 = 31 * 24 * 60 * 60 * 1000;

/// Session Helper'ın kullanıcı oturumundan bildirdiği **tek** etkinlik sinyali:
/// son klavye/fare girdisinden bu yana geçen süre. Tuş, koordinat, pencere,
/// uygulama veya içerik bilgisi taşınmaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityReport {
    /// `None`: helper ölçemedi (API hatası / desteklenmeyen platform).
    /// Alan zorunludur; ölçüm yoksa açıkça `null` gönderilir.
    #[serde(deserialize_with = "required_option")]
    pub idle_ms: Option<u64>,
    /// Raporlayıcının rolü (P1.D, ADR-0015). Yetki **vermez**: agent'ı
    /// yalnız aynı oturumdaki yetkili helper'lar arasında tek raporlayıcı
    /// seçerken önceliklendirmede kullanır. Eski helper'lar göndermez.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ReporterKind>,
}

/// Etkinlik raporlayıcısının rolü ve başlatılma yolu (P1.D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReporterKind {
    /// Bağımsız arka plan helper'ı; Windows oturum açılışı görevi başlattı.
    BackgroundLogonTask,
    /// Bağımsız arka plan helper'ı; elle başlatıldı (`kindora-helper activity`).
    BackgroundManual,
    /// Masaüstü köprüsü (`bridge`): yalnız arka plan helper'ı yokken yedek.
    DesktopBridge,
    /// Tek seferlik `report-activity` (tanılama / test).
    OneShot,
}

impl ReporterKind {
    /// Tek raporlayıcı seçiminde öncelik: arka plan helper'ı köprüden önce gelir.
    pub fn priority(kind: Option<ReporterKind>) -> u8 {
        match kind {
            Some(ReporterKind::BackgroundLogonTask | ReporterKind::BackgroundManual) => 2,
            _ => 1,
        }
    }

    pub fn is_background(kind: Option<ReporterKind>) -> bool {
        Self::priority(kind) == 2
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ReporterKind::BackgroundLogonTask => "BACKGROUND_LOGON_TASK",
            ReporterKind::BackgroundManual => "BACKGROUND_MANUAL",
            ReporterKind::DesktopBridge => "DESKTOP_BRIDGE",
            ReporterKind::OneShot => "ONE_SHOT",
        }
    }
}

fn required_option<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
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
    /// İstenen alt sistem (ör. kullanım süresi motoru) bu agent'ta çalışmıyor.
    Unavailable,
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
    /// `REPORT_ACTIVITY` yanıtı. `applied=false`: rapor kabul edildi ama
    /// sayıma girmedi (ör. etkin konsol oturumundan gelmiyor).
    ActivityAck {
        applied: bool,
    },
    ScreenTime(Box<ScreenTimeStatus>),
}

/// Kullanım durumu (P1.C, ADR-0014). Yalnız `ACTIVE` kullanım süresine eklenir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UsageState {
    Active,
    Idle,
    SessionLocked,
    Suspended,
    /// Ölçüm yok veya belirsiz (helper yok, API hatası, süreç duraklaması).
    Unknown,
}

impl UsageState {
    pub const ALL: [UsageState; 5] = [
        UsageState::Active,
        UsageState::Idle,
        UsageState::SessionLocked,
        UsageState::Suspended,
        UsageState::Unknown,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            UsageState::Active => "ACTIVE",
            UsageState::Idle => "IDLE",
            UsageState::SessionLocked => "SESSION_LOCKED",
            UsageState::Suspended => "SUSPENDED",
            UsageState::Unknown => "UNKNOWN",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicySource {
    /// Koddaki test varsayılanları; ebeveyn tercihi değildir.
    BuiltinDefault,
    /// Yönetici yazabilen klasördeki DEV test dosyası; üretim yetkilendirmesi değildir.
    DevFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyStatus {
    Ok,
    /// Politika dosyası bozuk; son geçerli politika kullanılıyor.
    InvalidKeptLast,
    /// Politika dosyası bozuk ve önceki geçerli politika yok; varsayılanlar.
    InvalidUsingDefault,
    /// Sürüm, son geçerli sürümden düşük/eşit (geri alma reddedildi).
    VersionRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyInfo {
    pub version: u32,
    pub source: PolicySource,
    pub status: PolicyStatus,
    pub daily_limit_secs: u32,
    pub break_interval_secs: u32,
    pub break_duration_secs: u32,
    pub first_warning_secs: u32,
    pub final_warning_secs: u32,
    pub idle_threshold_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BreakPhase {
    Normal,
    FirstWarning,
    FinalWarning,
    /// Mola zamanı; geri sayım sürüyor.
    BreakDue,
    /// Mola süresi doldu (kısa süre gösterilir).
    BreakComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NoticeKind {
    BreakFirstWarning,
    BreakFinalWarning,
    BreakStarted,
    BreakCompleted,
    /// Günlük süre doldu: yalnız bilgilendirme, kilit yok.
    LimitReached,
}

/// Masaüstünde gösterilecek tek seferlik bildirim. Aynı olay aynı `id`'yi taşır;
/// masaüstü bir `id`'yi bir kez gösterir ve kapatılabilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub id: String,
    pub kind: NoticeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MeasurementSource {
    /// Etkin konsol oturumundaki Session Helper (`GetLastInputInfo`).
    SessionHelper,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementInfo {
    pub source: MeasurementSource,
    /// Son geçerli helper raporunun yaşı (agent'ın monoton saatiyle).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_report_age_ms: Option<u64>,
    /// `UNKNOWN` durumunun nedeni (ör. `NO_HELPER`, `IDLE_API_FAILED`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown_reason: Option<String>,
    /// Agent'ın kabul ettiği tek etkinlik raporlayıcısı (P1.D). `None`: yok.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporter: Option<ReporterInfo>,
    /// Agent başladığından beri reddedilen ikinci raporlayıcı raporları.
    #[serde(default)]
    pub competing_reports: u64,
}

/// Kabul edilen raporlayıcının tanılama bilgisi. Kimlik bilgileri işletim
/// sisteminden (oturum, PID); `kind` helper'ın kendi beyanıdır.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReporterInfo {
    /// `None`: eski helper rolünü bildirmedi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ReporterKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Bu raporlayıcının kabul edildiği an (Unix ms; son (yeniden) bağlantı).
    pub since_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageInfo {
    /// `false`: SQLite açılamadı; sayaçlar yalnız bellekte (yeniden başlatmada kaybolur).
    pub persistent: bool,
    pub schema_version: u32,
    /// Bu çalıştırmada bozuk veritabanı kenara alınıp yeniden oluşturuldu.
    pub recovered: bool,
}

/// Bugünün süreleri (ms değil saniye; yerel takvim günü).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DayUsage {
    pub active_secs: u64,
    pub idle_secs: u64,
    pub locked_secs: u64,
    pub suspended_secs: u64,
    pub unknown_secs: u64,
}

/// `GET_SCREEN_TIME` yanıtı: agent'ın hesapladığı gerçek kullanım durumu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenTimeStatus {
    /// Yerel takvim günü `YYYY-MM-DD`.
    pub day: String,
    pub state: UsageState,
    pub today: DayUsage,
    pub daily_limit_secs: u64,
    pub remaining_secs: u64,
    pub limit_reached: bool,
    pub break_phase: BreakPhase,
    /// Sonraki molaya kalan **aktif kullanım** süresi (mola sırasında `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_break_in_secs: Option<u64>,
    /// Mola geri sayımı (yalnız `BREAK_DUE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_remaining_secs: Option<u64>,
    pub notices: Vec<Notice>,
    pub policy: PolicyInfo,
    pub measurement: MeasurementInfo,
    pub storage: StorageInfo,
    /// Bugün saat/saat dilimi değişikliği algılandı (kayıtlarda işaretli).
    pub clock_adjusted_today: bool,
    /// Agent'ın bu durumu hesapladığı an (Unix ms).
    pub computed_at_ms: u64,
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
    // `activity` yalnız REPORT_ACTIVITY'de ve orada zorunlu; değer sınırlı.
    let activity_ok = match (req.command, &req.activity) {
        (CommandType::ReportActivity, Some(a)) => a.idle_ms.is_none_or(|ms| ms <= MAX_IDLE_MS),
        (CommandType::ReportActivity, None) => false,
        (_, Some(_)) => false,
        (_, None) => true,
    };
    if !activity_ok {
        return Err(invalid(echo_id));
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
        for c in CommandType::ALL
            .into_iter()
            .filter(|c| *c != CommandType::ReportActivity)
        {
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
    fn report_activity_requires_bounded_activity_and_nothing_else() {
        let ok = Request::report_activity(
            "r-1",
            ActivityReport {
                idle_ms: Some(1500),
                kind: Some(ReporterKind::BackgroundLogonTask),
            },
        );
        assert_eq!(
            decode_request(&serde_json::to_vec(&ok).unwrap()).unwrap(),
            ok
        );
        let unmeasured = Request::report_activity(
            "r-2",
            ActivityReport {
                idle_ms: None,
                kind: None,
            },
        );
        let json = serde_json::to_vec(&unmeasured).unwrap();
        assert_eq!(
            decode_request(&json).unwrap().activity,
            Some(ActivityReport {
                idle_ms: None,
                kind: None
            })
        );

        let bad = [
            // activity eksik
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "REPORT_ACTIVITY", "timestamp_ms": 1}),
            // activity başka komutta
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "PING", "timestamp_ms": 1, "activity": {"idle_ms": 1}}),
            // içerik taşımaya çalışan ek alan
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "REPORT_ACTIVITY", "timestamp_ms": 1, "activity": {"idle_ms": 1, "keys": "abc"}}),
            // bilinmeyen rol / rol yerine serbest metin
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "REPORT_ACTIVITY", "timestamp_ms": 1, "activity": {"idle_ms": 1, "kind": "ADMIN"}}),
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "REPORT_ACTIVITY", "timestamp_ms": 1, "activity": {"idle_ms": 1, "kind": {"role": "x"}}}),
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "REPORT_ACTIVITY", "timestamp_ms": 1, "activity": {"idle_ms": -1}}),
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "REPORT_ACTIVITY", "timestamp_ms": 1, "activity": {"idle_ms": MAX_IDLE_MS + 1}}),
            serde_json::json!({"protocol_version": 1, "message_id": "a", "command": "REPORT_ACTIVITY", "timestamp_ms": 1, "activity": {}}),
        ];
        for (i, b) in bad.iter().enumerate() {
            let e = decode_request(&body(b.clone())).unwrap_err();
            assert_eq!(e.code, ErrorCode::InvalidMessage, "case {i}");
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
