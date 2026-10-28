//! Kindora Session Helper — P1.A prototipi.
//!
//! Kullanıcı oturumunda çalışır, agent'a yerel IPC ile bağlanır, periyodik
//! `PING` (kalp atışı) gönderir, yaşam döngüsü olaylarını raporlar, bağlantı
//! kaybını algılayıp yeniden bağlanır. **Kısıtlama/tam ekran arayüzü yok;
//! kendini kalıcılaştırmaz** (otomatik başlatma kaydı oluşturmaz).
//!
//! [`run_bridge`] (P1.B, ADR-0013): masaüstü uygulamasının alt süreci olarak
//! agent durumunu stdio üzerinden JSON satırlarıyla aktarır; agent'a yalnız
//! salt okunur `GET_STATUS` / `HEALTH_CHECK` gönderir.

use ipc_protocol::bridge::{
    encode_bridge_line, AgentSnapshot, BridgeEvent, BridgeRequest, LinkState, BRIDGE_VERSION,
};
use ipc_protocol::frame::{read_frame, write_frame, FrameError};
use ipc_protocol::{
    decode_response, now_ms, CommandType, ErrorCode, Payload, Request, Response, MAX_MESSAGE_BYTES,
};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

pub trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

#[derive(Debug, Clone)]
pub struct HelperConfig {
    pub heartbeat_interval: Duration,
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
            response_timeout: Duration::from_secs(3),
            reconnect_min: Duration::from_millis(250),
            reconnect_max: Duration::from_secs(5),
            bridge_poll_interval: Duration::from_secs(3),
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
}

impl Client {
    pub fn connect(cfg: &HelperConfig) -> Result<Self, ClientError> {
        let (stream, server_pid) = connect_stream(cfg).map_err(ClientError::Connect)?;
        Ok(Client {
            stream,
            id_prefix: format!("h{}-{}", std::process::id(), now_ms()),
            counter: 0,
            server_pid,
        })
    }

    pub fn request(&mut self, command: CommandType) -> Result<Response, ClientError> {
        self.counter += 1;
        let id = format!("{}-{}", self.id_prefix, self.counter);
        let body = serde_json::to_vec(&Request::new(id.clone(), command))
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
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

/// Helper ana döngüsü: bağlan → durum al → kalp atışı → kopunca yeniden bağlan.
pub fn run_helper(cfg: &HelperConfig, shutdown: &AtomicBool) {
    log_event("helper_started", &[("pid", std::process::id().to_string())]);
    let mut backoff = cfg.reconnect_min;

    'outer: while !shutdown.load(Ordering::SeqCst) {
        let mut client = match Client::connect(cfg) {
            Ok(c) => c,
            Err(e) => {
                log_event(
                    "connect_failed",
                    &[
                        ("error", e.to_string()),
                        ("retry_ms", backoff.as_millis().to_string()),
                    ],
                );
                if sleep_or_shutdown(backoff, shutdown) {
                    break;
                }
                backoff = (backoff * 2).min(cfg.reconnect_max);
                continue;
            }
        };
        backoff = cfg.reconnect_min;
        let connected_at = Instant::now();
        let mut fields = vec![];
        if let Some(pid) = client.server_pid {
            fields.push(("server_pid", pid.to_string()));
        }
        log_event("connected", &fields);

        match client.request(CommandType::GetStatus) {
            Ok(r) => log_event(
                "agent_status",
                &[(
                    "payload",
                    serde_json::to_string(&r.payload).unwrap_or_default(),
                )],
            ),
            Err(e) => {
                log_event(
                    "connection_lost",
                    &[("reason", e.to_string()), ("phase", "status".into())],
                );
                continue 'outer;
            }
        }

        let mut last_ok = Instant::now();
        loop {
            if sleep_or_shutdown(cfg.heartbeat_interval, shutdown) {
                break 'outer;
            }
            match client.request(CommandType::Ping) {
                Ok(_) => last_ok = Instant::now(),
                Err(e) => {
                    log_event(
                        "connection_lost",
                        &[
                            ("reason", e.to_string()),
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
                    if matches!(e, ClientError::Unauthorized) {
                        // Yetkisiz helper'ın agent'ı yoklamaya devam etmesi anlamsız.
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

/// `GET_STATUS` + `HEALTH_CHECK` → tek anlık görüntü.
pub fn poll_snapshot(client: &mut Client) -> Result<AgentSnapshot, ClientError> {
    let started = Instant::now();
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
