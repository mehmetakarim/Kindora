//! Kindora Windows Agent.
//!
//! Kapsam: servis yaşam döngüsü (start/stop/status/health/graceful shutdown),
//! Session Helper ile yerel IPC (P1.A) ve yerel kullanım süresi ölçümü + mola
//! hatırlatma durumu (P1.C, [`screen_time`]). **Kullanım kilidi, engelleme,
//! program kapatma, bulut senkronu yok.** Servis otomatik kurulmaz/başlatılmaz.
//!
//! Servis hesabı ADR-0006 kapsamında açıktır; kod LocalSystem varsaymaz.

pub mod log;
pub mod platform;
pub mod screen_time;
#[cfg(windows)]
pub mod service;

use agent_core::auth::{Authorizer, HelperPolicy};
use agent_core::connection::serve_connection;
use agent_core::{AgentState, ConnectionLimits, AGENT_VERSION};
use ipc_protocol::transport::PeerIdentity;
use ipc_protocol::RunMode;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub limits: ConnectionLimits,
    /// Okuma yoklama aralığı: kapanma bayrağı en geç bu sürede fark edilir.
    pub poll_interval: Duration,
    pub max_connections: usize,
    #[cfg(unix)]
    pub socket_path: std::path::PathBuf,
    /// `None`: kullanım süresi motoru çalışmaz (`GET_SCREEN_TIME` → UNAVAILABLE).
    pub screen_time: Option<screen_time::ScreenTimeConfig>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            limits: ConnectionLimits::default(),
            poll_interval: Duration::from_millis(500),
            max_connections: 8,
            #[cfg(unix)]
            socket_path: ipc_protocol::transport::unix_dev::default_socket_path(),
            screen_time: None,
        }
    }
}

/// Windows: istemci etkileşimli oturumda, agent ile aynı dizindeki
/// `kindora-helper.exe` olmalı. Unix geliştirme: aynı kullanıcı kimliği.
pub fn helper_policy() -> HelperPolicy {
    #[cfg(windows)]
    {
        HelperPolicy {
            required_uid: None,
            require_interactive_session: true,
            required_exe_name: Some("kindora-helper.exe".into()),
            required_exe_dir: std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf())),
        }
    }
    #[cfg(unix)]
    {
        HelperPolicy {
            required_uid: Some(ipc_protocol::transport::unix_dev::current_uid()),
            ..Default::default()
        }
    }
}

fn peer_fields(peer: &PeerIdentity) -> Vec<(&'static str, String)> {
    let opt = |v: Option<u32>| v.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
    vec![
        ("pid", opt(peer.pid)),
        ("uid", opt(peer.uid)),
        ("session", opt(peer.session_id)),
        (
            "exe",
            peer.exe_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".into()),
        ),
        (
            "os_errors",
            if peer.diagnostics.is_empty() {
                "-".into()
            } else {
                peer.diagnostics.join(",")
            },
        ),
    ]
}

struct Ctx {
    state: Arc<AgentState>,
    policy: Arc<dyn Authorizer>,
    limits: ConnectionLimits,
    shutdown: Arc<AtomicBool>,
    active: Arc<AtomicUsize>,
    max_connections: usize,
}

fn spawn_connection<S>(ctx: &Ctx, mut stream: S, peer: PeerIdentity) -> Option<JoinHandle<()>>
where
    S: Read + Write + Send + 'static,
{
    if ctx.active.load(Ordering::SeqCst) >= ctx.max_connections {
        log::event("connection_rejected", &[("reason", "capacity".into())]);
        return None;
    }
    ctx.active.fetch_add(1, Ordering::SeqCst);
    let (state, policy, limits, shutdown, active) = (
        ctx.state.clone(),
        ctx.policy.clone(),
        ctx.limits.clone(),
        ctx.shutdown.clone(),
        ctx.active.clone(),
    );
    Some(thread::spawn(move || {
        let started = Instant::now();
        log::event("client_connected", &peer_fields(&peer));
        let rep = serve_connection(
            &mut stream,
            &peer,
            &state,
            policy.as_ref(),
            &limits,
            &shutdown,
        );
        drop(stream);
        let mut fields = peer_fields(&peer);
        fields.push(("outcome", format!("{:?}", rep.outcome)));
        fields.push(("requests", rep.requests.to_string()));
        fields.push(("errors", rep.errors.to_string()));
        fields.push(("duration_ms", started.elapsed().as_millis().to_string()));
        log::event("client_disconnected", &fields);
        active.fetch_sub(1, Ordering::SeqCst);
    }))
}

/// Agent'ı `shutdown` ayarlanana kadar çalıştırır; tüm bağlantılar kapanınca döner.
pub fn run_agent(mode: RunMode, cfg: AgentConfig, shutdown: Arc<AtomicBool>) -> io::Result<()> {
    let mut state = AgentState::new(mode);
    let mut screen_time_thread = None;
    if let Some(st_cfg) = cfg.screen_time.clone() {
        let (rt, handle) = screen_time::start(st_cfg, shutdown.clone());
        state = state.with_screen_time(rt);
        screen_time_thread = Some(handle);
    }
    let ctx = Ctx {
        state: Arc::new(state),
        policy: Arc::new(helper_policy()),
        limits: cfg.limits.clone(),
        shutdown: shutdown.clone(),
        active: Arc::new(AtomicUsize::new(0)),
        max_connections: cfg.max_connections,
    };
    log::event(
        "agent_started",
        &[
            ("version", AGENT_VERSION.into()),
            ("mode", format!("{mode:?}")),
            ("pid", std::process::id().to_string()),
        ],
    );

    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    let result = accept_loop(&ctx, &cfg, &mut workers);

    shutdown.store(true, Ordering::SeqCst);
    for w in workers {
        let _ = w.join();
    }
    if let Some(h) = screen_time_thread {
        let _ = h.join();
    }
    match &result {
        Ok(()) => log::event("agent_stopped", &[]),
        Err(e) => log::event("agent_failed", &[("error", e.to_string())]),
    }
    result
}

#[cfg(unix)]
fn accept_loop(ctx: &Ctx, cfg: &AgentConfig, workers: &mut Vec<JoinHandle<()>>) -> io::Result<()> {
    use ipc_protocol::transport::unix_dev::DevListener;
    let listener = DevListener::bind(&cfg.socket_path)?;
    log::event(
        "listening",
        &[
            ("transport", "unix-dev".into()),
            ("path", listener.path().display().to_string()),
        ],
    );
    while !ctx.shutdown.load(Ordering::SeqCst) {
        match listener.accept(cfg.poll_interval)? {
            Some((stream, peer)) => workers.extend(spawn_connection(ctx, stream, peer)),
            None => thread::sleep(Duration::from_millis(25)),
        }
        workers.retain(|w| !w.is_finished());
    }
    Ok(())
}

#[cfg(windows)]
fn accept_loop(ctx: &Ctx, cfg: &AgentConfig, workers: &mut Vec<JoinHandle<()>>) -> io::Result<()> {
    use ipc_protocol::transport::windows_pipe::{PipeServer, PIPE_NAME};
    let mut server = PipeServer::new()?;
    log::event(
        "listening",
        &[
            ("transport", "named-pipe".into()),
            ("path", PIPE_NAME.into()),
        ],
    );
    while !ctx.shutdown.load(Ordering::SeqCst) {
        match server.accept(cfg.poll_interval, &ctx.shutdown)? {
            Some((stream, peer)) => workers.extend(spawn_connection(ctx, stream, peer)),
            None => break,
        }
        workers.retain(|w| !w.is_finished());
    }
    Ok(())
}
