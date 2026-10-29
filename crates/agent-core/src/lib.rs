//! Kindora agent çekirdeği — işletim sisteminden bağımsız.
//!
//! - [`auth`]: işletim sisteminin bildirdiği eş kimliğine göre yetkilendirme.
//! - [`handler`]: istek doğrulama, tekrar kimliği tespiti, komut yanıtları.
//! - [`connection`]: tek bir bağlantının yaşam döngüsü (zaman aşımı, hata
//!   bütçesi, kapanma), herhangi bir `Read + Write` akışı üzerinde.
//! - [`ScreenTimeService`]: kullanım süresi motoruna (P1.C) dar arayüz.

pub mod auth;
pub mod connection;
pub mod handler;

use ipc_protocol::{ActivityReport, HealthCheckResult, RunMode, ScreenTimeStatus};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Kullanım süresi motoruna agent çekirdeğinden erişim (P1.C, ADR-0014).
///
/// Yalnız iki işlem vardır: helper'ın etkinlik raporunu iletmek ve durumu
/// okumak. Kural değiştirme veya kısıtlama uygulama işlemi **yoktur**.
pub trait ScreenTimeService: Send + Sync + std::fmt::Debug {
    /// `client_session`: raporu gönderen sürecin Windows oturumu (işletim
    /// sisteminden). Dönüş: rapor sayıma girdi mi.
    fn report_activity(&self, client_session: Option<u32>, report: ActivityReport) -> bool;
    fn status(&self) -> Option<ScreenTimeStatus>;
    /// Sağlık denetimine eklenecek satırlar (motor, depolama).
    fn health(&self) -> Vec<HealthCheckResult>;
}

/// Agent süreci genelinde paylaşılan durum.
#[derive(Debug)]
pub struct AgentState {
    pub run_mode: RunMode,
    started: Instant,
    connected: AtomicU32,
    screen_time: Option<Arc<dyn ScreenTimeService>>,
}

impl AgentState {
    pub fn new(run_mode: RunMode) -> Self {
        AgentState {
            run_mode,
            started: Instant::now(),
            connected: AtomicU32::new(0),
            screen_time: None,
        }
    }

    pub fn with_screen_time(mut self, service: Arc<dyn ScreenTimeService>) -> Self {
        self.screen_time = Some(service);
        self
    }

    pub fn screen_time(&self) -> Option<&dyn ScreenTimeService> {
        self.screen_time.as_deref()
    }

    pub fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn connected_clients(&self) -> u32 {
        self.connected.load(Ordering::SeqCst)
    }

    /// Yetkilendirilmiş bağlantı sayacı; `Drop` ile azalır.
    pub fn register_client(&self) -> ClientGuard<'_> {
        self.connected.fetch_add(1, Ordering::SeqCst);
        ClientGuard(self)
    }
}

pub struct ClientGuard<'a>(&'a AgentState);

impl Drop for ClientGuard<'_> {
    fn drop(&mut self) {
        self.0.connected.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Bağlantı başına sınırlar.
#[derive(Debug, Clone)]
pub struct ConnectionLimits {
    pub max_message_bytes: usize,
    /// Bu süre boyunca hiç çerçeve gelmezse bağlantı kapatılır.
    pub idle_timeout: Duration,
    /// Bu kadar hatalı/reddedilen istekten sonra bağlantı kapatılır.
    pub max_errors: u32,
    /// Tekrar tespiti için hatırlanan son `message_id` sayısı.
    pub dedupe_window: usize,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        ConnectionLimits {
            max_message_bytes: ipc_protocol::MAX_MESSAGE_BYTES,
            idle_timeout: Duration::from_secs(30),
            max_errors: 5,
            dedupe_window: 256,
        }
    }
}
