//! Kindora Session Helper — P1.A prototipi.
//!
//! Kullanıcı oturumunda çalışır, agent'a yerel IPC ile bağlanır, periyodik
//! `PING` (kalp atışı) gönderir, yaşam döngüsü olaylarını raporlar, bağlantı
//! kaybını algılayıp yeniden bağlanır. **Kısıtlama/tam ekran arayüzü yok;
//! kendini kalıcılaştırmaz** (otomatik başlatma kaydı oluşturmaz).

use ipc_protocol::frame::{read_frame, write_frame, FrameError};
use ipc_protocol::{
    decode_response, now_ms, CommandType, ErrorCode, Request, Response, MAX_MESSAGE_BYTES,
};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
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
