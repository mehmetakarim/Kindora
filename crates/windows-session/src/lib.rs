//! Kindora Session Helper — P1.A prototipi.
//!
//! Kullanıcı oturumunda çalışır, agent'a yerel IPC ile bağlanır, periyodik
//! `PING` (kalp atışı) gönderir, yaşam döngüsü olaylarını raporlar, bağlantı
//! kaybını algılayıp yeniden bağlanır. **Kısıtlama/tam ekran arayüzü yok;
//! kendini kalıcılaştırmaz** (otomatik başlatma kaydı oluşturmaz).
//!
//! [`run_bridge`] (P1.B, ADR-0013): masaüstü uygulamasının alt süreci olarak
//! agent durumunu stdio üzerinden JSON satırlarıyla aktarır; agent'a salt
//! okunur `GET_STATUS` / `HEALTH_CHECK` / `GET_SCREEN_TIME` gönderir.
//!
//! P1.C (ADR-0014): helper her döngüde kendi oturumundaki "son girdiden bu
//! yana geçen süre"yi [`activity::idle_ms`] ile ölçüp `REPORT_ACTIVITY` olarak
//! bildirir. Başka hiçbir kullanıcı verisi okunmaz.
//!
//! P1.D (ADR-0015): iki ayrı rol vardır.
//! - [`run_activity`] (`kindora-helper activity`): masaüstünden **bağımsız**
//!   arka plan raporlayıcısı. Oturum başına tek örnek ([`instance`]), oturum
//!   açılışında Zamanlanmış Görev ile başlar; masaüstü açılıp kapanınca durmaz.
//! - [`run_bridge`] (`kindora-helper bridge`): masaüstünün alt süreci; durum
//!   okur ve yalnız yedek raporlayıcıdır (agent arka plan helper'ını önceler).
//!
//! Raporlar anlık ölçümdür: bağlantı yokken biriktirilmez, yeniden bağlanınca
//! geçmiş için rapor gönderilmez.

pub mod activity;
pub mod instance;
#[cfg(windows)]
pub mod session_end;

use ipc_protocol::bridge::{
    encode_bridge_line, AgentSnapshot, BridgeEvent, BridgeRequest, LinkState, BRIDGE_VERSION,
};
use ipc_protocol::frame::{read_frame, write_frame, FrameError};
use ipc_protocol::{
    decode_response, now_ms, ActivityReport, CommandType, ErrorCode, Payload, ReporterKind,
    Request, Response, MAX_MESSAGE_BYTES,
};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

#[derive(Debug, Clone)]
pub struct HelperConfig {
    /// `activity` modunda rapor aralığı (eski adı: kalp atışı).
    pub heartbeat_interval: Duration,
    /// Bu süreç hangi rolle rapor gönderir (agent önceliklendirmesi için).
    pub reporter_kind: ReporterKind,
    pub response_timeout: Duration,
    pub reconnect_min: Duration,
    pub reconnect_max: Duration,
    /// Köprü modunda bağlıyken agent durumunu yenileme aralığı.
    pub bridge_poll_interval: Duration,
    /// Köprü modunda agent yetkisiz dediğinde yeniden deneme aralığı.
    pub unauthorized_retry: Duration,
    #[cfg(unix)]
    pub socket_path: std::path::PathBuf,
}

impl Default for HelperConfig {
    fn default() -> Self {
        HelperConfig {
            heartbeat_interval: Duration::from_secs(5),
            reporter_kind: ReporterKind::OneShot,
            response_timeout: Duration::from_secs(3),
            reconnect_min: Duration::from_millis(250),
            reconnect_max: Duration::from_secs(5),
            bridge_poll_interval: Duration::from_secs(2),
            unauthorized_retry: Duration::from_secs(30),
            #[cfg(unix)]
            socket_path: ipc_protocol::transport::unix_dev::default_socket_path(),
        }
    }
}

#[derive(Debug)]
pub enum ClientError {
    Connect(io::Error),
    Unauthorized,
    ConnectionClosed,
    Timeout,
    Io(String),
    Protocol(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Connect(e) => write!(f, "connect failed: {e}"),
            ClientError::Unauthorized => write!(f, "unauthorized"),
            ClientError::ConnectionClosed => write!(f, "connection closed by agent"),
            ClientError::Timeout => write!(f, "response timeout"),
            ClientError::Io(e) => write!(f, "io: {e}"),
            ClientError::Protocol(e) => write!(f, "protocol: {e}"),
        }
    }
}

static LOG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

/// Bu boyutu aşan kayıt dosyası açılışta `.1` olarak kenara alınır (tek kopya).
pub const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Kayıtları stderr'e ek olarak dosyaya da yazar (arka plan helper'ında stderr
/// görünmez). Süreçte bir kez çağrılır.
pub fn set_log_file(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
        let mut old = path.as_os_str().to_owned();
        old.push(".1");
        let _ = std::fs::rename(path, PathBuf::from(old));
    }
    let f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    LOG_FILE
        .set(Mutex::new(f))
        .map_err(|_| io::Error::other("log file already set"))
}

/// Windows: `%LOCALAPPDATA%\Kindora\helper\activity.log` (kullanıcının kendi
/// profili; tuş, ekran veya içerik içermez). Diğer: yok.
pub fn default_activity_log() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| {
            PathBuf::from(d)
                .join("Kindora")
                .join("helper")
                .join("activity.log")
        })
    } else {
        None
    }
}

pub fn log_event(name: &str, fields: &[(&str, String)]) {
    let mut line = format!("ts_ms={} event={}", now_ms(), name);
    for (k, v) in fields {
        if v.contains(char::is_whitespace) || v.is_empty() {
            line.push_str(&format!(" {k}={v:?}"));
        } else {
            line.push_str(&format!(" {k}={v}"));
        }
    }
    eprintln!("{line}");
    if let Some(f) = LOG_FILE.get() {
        if let Ok(mut f) = f.lock() {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Önceki çalıştırmanın temiz bitip bitmediğini izleyen dosya
/// (`<kayıt>.running`). Süreç sonlandırılır veya çökerse dosya kalır; bir
/// sonraki açılış bunu `previous_exit=unclean` olarak kaydeder.
pub struct RunMarker(PathBuf);

impl RunMarker {
    /// Dönüş: (iz, önceki çalıştırma temiz bitmediyse onun içeriği).
    pub fn create(log_file: &Path) -> (Option<RunMarker>, Option<String>) {
        let mut p = log_file.as_os_str().to_owned();
        p.push(".running");
        let path = PathBuf::from(p);
        let previous = std::fs::read_to_string(&path)
            .ok()
            .map(|s| s.trim().chars().take(80).collect());
        let content = format!("pid={} started_ms={}", std::process::id(), now_ms());
        let marker = std::fs::write(&path, content).ok().map(|_| RunMarker(path));
        (marker, previous)
    }

    /// Temiz çıkış: iz silinir.
    pub fn clear(self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn connect_stream(cfg: &HelperConfig) -> io::Result<(Box<dyn Stream>, Option<u32>)> {
    #[cfg(unix)]
    {
        let s = ipc_protocol::transport::unix_dev::connect(&cfg.socket_path, cfg.response_timeout)?;
        Ok((Box::new(s), None))
    }
    #[cfg(windows)]
    {
        let s = ipc_protocol::transport::windows_pipe::connect(
            cfg.response_timeout,
            cfg.response_timeout,
        )?;
        let server_pid = s.server_process_id();
        Ok((Box::new(s), server_pid))
    }
}

pub struct Client {
    stream: Box<dyn Stream>,
    id_prefix: String,
    counter: u64,
    /// Windows: pipe sunucusunun PID'i (kayıt için).
    pub server_pid: Option<u32>,
    /// Agent kullanım süresi komutlarını reddettiyse (eski sürüm / motor yok)
    /// bu bağlantıda bir daha gönderilmez; hata bütçesi tüketilmez.
    screen_time_supported: bool,
    reporter_kind: ReporterKind,
}

impl Client {
    pub fn connect(cfg: &HelperConfig) -> Result<Self, ClientError> {
        let (stream, server_pid) = connect_stream(cfg).map_err(ClientError::Connect)?;
        Ok(Client {
            stream,
            id_prefix: format!("h{}-{}", std::process::id(), now_ms()),
            counter: 0,
            server_pid,
            screen_time_supported: true,
            reporter_kind: cfg.reporter_kind,
        })
    }

    pub fn request(&mut self, command: CommandType) -> Result<Response, ClientError> {
        self.counter += 1;
        let id = format!("{}-{}", self.id_prefix, self.counter);
        self.send(Request::new(id, command))
    }

    /// Bu oturumun etkinlik sinyalini bildirir (yalnız son girdiden bu yana geçen süre).
    pub fn report_activity(&mut self, idle_ms: Option<u64>) -> Result<Response, ClientError> {
        self.counter += 1;
        let id = format!("{}-{}", self.id_prefix, self.counter);
        self.send(Request::report_activity(
            id,
            ActivityReport {
                idle_ms,
                kind: Some(self.reporter_kind),
            },
        ))
    }

    pub fn screen_time_supported(&self) -> bool {
        self.screen_time_supported
    }

    /// Etkinlik raporu + kullanım süresi durumu. Agent desteklemiyorsa `None`.
    pub fn screen_time_cycle(
        &mut self,
    ) -> Result<Option<Box<ipc_protocol::ScreenTimeStatus>>, ClientError> {
        if !self.screen_time_supported {
            return Ok(None);
        }
        if self
            .report_activity(activity::idle_ms())?
            .error_code
            .is_some()
        {
            self.screen_time_supported = false;
            return Ok(None);
        }
        let r = self.request(CommandType::GetScreenTime)?;
        match r.payload {
            Some(Payload::ScreenTime(s)) => Ok(Some(s)),
            _ => {
                self.screen_time_supported = false;
                Ok(None)
            }
        }
    }

    fn send(&mut self, req: Request) -> Result<Response, ClientError> {
        let id = req.message_id.clone();
        let body = serde_json::to_vec(&req).map_err(|e| ClientError::Protocol(e.to_string()))?;
        if let Err(write_err) = write_frame(&mut self.stream, &body) {
            // Agent yetkisiz istemciye UNAUTHORIZED yazıp bağlantıyı kapatmış
            // olabilir; yazma hatasından önce tamponda bekleyen yanıtı oku.
            if let Ok(b) = read_frame(&mut self.stream, MAX_MESSAGE_BYTES) {
                if let Ok(r) = decode_response(&b) {
                    if r.error_code == Some(ErrorCode::Unauthorized) {
                        return Err(ClientError::Unauthorized);
                    }
                }
            }
            return Err(map_frame_error(write_err));
        }
        let resp_body = read_frame(&mut self.stream, MAX_MESSAGE_BYTES).map_err(map_frame_error)?;
        let resp = decode_response(&resp_body).map_err(|e| ClientError::Protocol(e.to_string()))?;
        if resp.error_code == Some(ErrorCode::Unauthorized) {
            return Err(ClientError::Unauthorized);
        }
        if resp.message_id.as_deref() != Some(id.as_str()) {
            return Err(ClientError::Protocol("response message_id mismatch".into()));
        }
        Ok(resp)
    }
}

fn map_frame_error(e: FrameError) -> ClientError {
    match e {
        FrameError::Closed => ClientError::ConnectionClosed,
        FrameError::Idle => ClientError::Timeout,
        FrameError::Io(e) if e.kind() == io::ErrorKind::BrokenPipe => ClientError::ConnectionClosed,
        other => ClientError::Io(other.to_string()),
    }
}

/// `shutdown` ayarlanana kadar `d` süresi bekler. `true`: kapanma istendi.
fn sleep_or_shutdown(d: Duration, shutdown: &AtomicBool) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if shutdown.load(Ordering::SeqCst) {
            return true;
        }
        thread::sleep(Duration::from_millis(25).min(end - Instant::now()));
    }
    shutdown.load(Ordering::SeqCst)
}

/// Arka plan helper'ında yeniden bağlanma aralığının üst sınırı (agent
/// kapalıyken düşük kaynak kullanımı).
pub const ACTIVITY_RECONNECT_MAX: Duration = Duration::from_secs(10);

/// Bağımsız arka plan raporlayıcısı (`activity`; eski adı `run`).
///
/// Bağlan → `GET_STATUS` → her aralıkta yalnız `REPORT_ACTIVITY` → kopunca
/// üstel geri çekilmeyle (en çok [`ACTIVITY_RECONNECT_MAX`]) yeniden bağlan.
/// Agent yetkisiz derse `unauthorized_retry` kadar bekler. Kayıt yalnız durum
/// geçişlerinde yazılır. Agent kullanım süresini desteklemiyorsa `PING` gönderir.
pub fn run_activity(cfg: &HelperConfig, shutdown: &AtomicBool) {
    log_event(
        "helper_started",
        &[
            ("pid", std::process::id().to_string()),
            ("kind", cfg.reporter_kind.as_str().into()),
            ("version", env!("CARGO_PKG_VERSION").into()),
            (
                "interval_ms",
                cfg.heartbeat_interval.as_millis().to_string(),
            ),
        ],
    );
    let reconnect_max = cfg.reconnect_max.max(ACTIVITY_RECONNECT_MAX);
    let mut backoff = cfg.reconnect_min;
    let mut connects: u64 = 0;
    let mut failing = false;

    'outer: while !shutdown.load(Ordering::SeqCst) {
        let mut client = match Client::connect(cfg) {
            Ok(c) => c,
            Err(e) => {
                // Agent kapalıyken her denemede satır yazılmaz.
                if !failing {
                    failing = true;
                    log_event(
                        "connect_failed",
                        &[
                            ("error", e.to_string()),
                            ("retry_ms", backoff.as_millis().to_string()),
                        ],
                    );
                }
                if sleep_or_shutdown(backoff, shutdown) {
                    break;
                }
                backoff = (backoff * 2).min(reconnect_max);
                continue;
            }
        };
        failing = false;
        backoff = cfg.reconnect_min;
        connects += 1;
        let connected_at = Instant::now();
        let mut fields = vec![("reconnects", (connects - 1).to_string())];
        if let Some(pid) = client.server_pid {
            fields.push(("server_pid", pid.to_string()));
        }
        log_event("connected", &fields);

        let lost = |e: &ClientError, phase: &str, last_ok: Instant| {
            log_event(
                "connection_lost",
                &[
                    ("reason", e.to_string()),
                    ("phase", phase.into()),
                    (
                        "since_last_ok_ms",
                        last_ok.elapsed().as_millis().to_string(),
                    ),
                    (
                        "connected_ms",
                        connected_at.elapsed().as_millis().to_string(),
                    ),
                ],
            );
        };
        match client.request(CommandType::GetStatus) {
            Ok(r) => log_event(
                "agent_status",
                &[(
                    "payload",
                    serde_json::to_string(&r.payload).unwrap_or_default(),
                )],
            ),
            Err(e) => {
                lost(&e, "status", connected_at);
                if matches!(e, ClientError::Unauthorized)
                    && sleep_or_shutdown(cfg.unauthorized_retry, shutdown)
                {
                    break 'outer;
                }
                continue 'outer;
            }
        }

        let mut last_ok = Instant::now();
        let mut accepted: Option<bool> = None;
        let mut first = true;
        loop {
            // İlk rapor bağlanır bağlanmaz; sonrakiler aralıkla.
            if !first && sleep_or_shutdown(cfg.heartbeat_interval, shutdown) {
                break 'outer;
            }
            first = false;
            let beat = if client.screen_time_supported() {
                client.report_activity(activity::idle_ms()).map(|r| {
                    match r.payload {
                        Some(Payload::ActivityAck { applied }) => Some(applied),
                        _ => {
                            // Kullanım süresi motoru yok: bundan sonra PING.
                            client.screen_time_supported = false;
                            None
                        }
                    }
                })
            } else {
                client.request(CommandType::Ping).map(|_| None)
            };
            match beat {
                Ok(applied) => {
                    last_ok = Instant::now();
                    if applied.is_some() && applied != accepted {
                        // Tek raporlayıcı kuralı: başka raporlayıcı varsa beklemede.
                        log_event(
                            if applied == Some(true) {
                                "reporter_active"
                            } else {
                                "reporter_standby"
                            },
                            &[],
                        );
                        accepted = applied;
                    }
                }
                Err(e) => {
                    lost(&e, "report", last_ok);
                    if matches!(e, ClientError::Unauthorized)
                        && sleep_or_shutdown(cfg.unauthorized_retry, shutdown)
                    {
                        break 'outer;
                    }
                    continue 'outer;
                }
            }
        }
    }
    log_event("helper_stopped", &[]);
}

/// Köprü bekleme sonucu.
enum Wake {
    Timeout,
    Refresh,
    Shutdown,
}

/// Masaüstünden gelen `refresh` istekleri en sık bu aralıkla uygulanır.
const MIN_REFRESH_GAP: Duration = Duration::from_millis(500);

/// `d` kadar bekler; masaüstünden `refresh` gelirse (en erken
/// [`MIN_REFRESH_GAP`] sonra) veya kapanma istenirse erken döner.
fn wait_bridge(d: Duration, requests: &Receiver<BridgeRequest>, shutdown: &AtomicBool) -> Wake {
    let start = Instant::now();
    let end = start + d;
    let mut refresh = false;
    loop {
        if shutdown.load(Ordering::SeqCst) {
            return Wake::Shutdown;
        }
        let now = Instant::now();
        if refresh && now >= start + MIN_REFRESH_GAP {
            return Wake::Refresh;
        }
        let left = end.saturating_duration_since(now);
        if left.is_zero() {
            return Wake::Timeout;
        }
        match requests.recv_timeout(left.min(Duration::from_millis(100))) {
            Ok(BridgeRequest::Refresh) => refresh = true,
            Err(RecvTimeoutError::Timeout) => {}
            // stdin kapandı: masaüstü gitti.
            Err(RecvTimeoutError::Disconnected) => {
                shutdown.store(true, Ordering::SeqCst);
                return Wake::Shutdown;
            }
        }
    }
}

/// `REPORT_ACTIVITY` + `GET_STATUS` + `HEALTH_CHECK` + `GET_SCREEN_TIME` →
/// tek anlık görüntü. Kullanım süresi alınamazsa (eski agent, motor yok)
/// `screen_time: None` olur; masaüstü bunu ölçüm yok olarak gösterir.
pub fn poll_snapshot(client: &mut Client) -> Result<AgentSnapshot, ClientError> {
    let started = Instant::now();
    let screen_time = client.screen_time_cycle()?;
    let status = client.request(CommandType::GetStatus)?;
    let health = client.request(CommandType::HealthCheck)?;
    match (status.payload, health.payload) {
        (
            Some(Payload::Status {
                agent_version,
                run_mode,
                uptime_secs,
                connected_clients,
            }),
            Some(Payload::Health { healthy, checks }),
        ) => Ok(AgentSnapshot {
            agent_version,
            run_mode,
            uptime_secs,
            connected_clients,
            healthy,
            checks,
            received_at_ms: now_ms(),
            round_trip_ms: started.elapsed().as_millis() as u64,
            screen_time,
        }),
        _ => Err(ClientError::Protocol("unexpected payload".into())),
    }
}

/// Köprü ana döngüsü. `emit` false dönerse (çıkış borusu kapandı) döngü biter.
///
/// - Bağlantı denemeleri üstel geri çekilmeli (`reconnect_min` → `reconnect_max`);
///   yetkisiz yanıtta `unauthorized_retry`. Yoğun döngü yok.
/// - Kayıt (stderr) yalnız durum geçişlerinde yazılır; satır sayısı sınırlıdır.
pub fn run_bridge(
    cfg: &HelperConfig,
    requests: &Receiver<BridgeRequest>,
    shutdown: &AtomicBool,
    emit: &mut dyn FnMut(&BridgeEvent) -> bool,
) {
    let mut send = |e: BridgeEvent| {
        if !emit(&e) {
            shutdown.store(true, Ordering::SeqCst);
        }
    };
    send(BridgeEvent::Hello {
        bridge_version: BRIDGE_VERSION,
        helper_version: env!("CARGO_PKG_VERSION").into(),
        pid: std::process::id(),
    });
    log_event("bridge_started", &[("pid", std::process::id().to_string())]);
    send(BridgeEvent::Link {
        state: LinkState::Connecting,
        attempt: 0,
        retry_in_ms: None,
        reason: None,
    });

    let mut backoff = cfg.reconnect_min;
    let mut attempt: u32 = 0;
    let mut last_logged: Option<LinkState> = None;
    let mut log_transition = |state: LinkState, reason: &str| {
        if last_logged != Some(state) {
            log_event(
                "bridge_link",
                &[
                    ("state", format!("{state:?}")),
                    ("reason", reason.to_string()),
                ],
            );
            last_logged = Some(state);
        }
    };

    'outer: while !shutdown.load(Ordering::SeqCst) {
        let failure = match Client::connect(cfg) {
            Err(e) => e,
            Ok(mut client) => {
                let mut connected = false;
                loop {
                    match poll_snapshot(&mut client) {
                        Ok(snap) => {
                            if !connected {
                                connected = true;
                                attempt = 0;
                                backoff = cfg.reconnect_min;
                                log_transition(LinkState::Connected, "");
                                send(BridgeEvent::Link {
                                    state: LinkState::Connected,
                                    attempt: 0,
                                    retry_in_ms: None,
                                    reason: None,
                                });
                            }
                            send(BridgeEvent::Snapshot(snap));
                        }
                        Err(e) => break e,
                    }
                    if let Wake::Shutdown =
                        wait_bridge(cfg.bridge_poll_interval, requests, shutdown)
                    {
                        break 'outer;
                    }
                }
            }
        };

        attempt = attempt.saturating_add(1);
        let (state, retry) = if matches!(failure, ClientError::Unauthorized) {
            (LinkState::Unauthorized, cfg.unauthorized_retry)
        } else {
            (LinkState::Disconnected, backoff)
        };
        let reason = failure.to_string();
        log_transition(state, &reason);
        send(BridgeEvent::Link {
            state,
            attempt,
            retry_in_ms: Some(retry.as_millis() as u64),
            reason: Some(reason),
        });
        if state == LinkState::Disconnected {
            backoff = (backoff * 2).min(cfg.reconnect_max);
        }
        match wait_bridge(retry, requests, shutdown) {
            Wake::Shutdown => break,
            Wake::Timeout | Wake::Refresh => {}
        }
        send(BridgeEvent::Link {
            state: LinkState::Connecting,
            attempt,
            retry_in_ms: None,
            reason: None,
        });
    }
    log_event("bridge_stopped", &[]);
}

/// Satır biçiminde bir köprü olayı yazar.
pub fn write_bridge_event(out: &mut dyn Write, event: &BridgeEvent) -> bool {
    let mut line = encode_bridge_line(event);
    line.push('\n');
    out.write_all(line.as_bytes())
        .and_then(|_| out.flush())
        .is_ok()
}
