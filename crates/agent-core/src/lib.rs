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
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Raporu gönderen bağlantının işletim sisteminden ve agent'tan gelen kimliği.
/// Mesaj gövdesindeki hiçbir alan buraya girmez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClientInfo {
    /// Agent'ın bu süreç ömründe her bağlantıya verdiği benzersiz sayı (PID değil).
    pub connection_id: u64,
    /// İstemci sürecin Windows oturumu.
    pub session_id: Option<u32>,
    /// Yalnız tanılama; yetkilendirmede kullanılmaz.
    pub pid: Option<u32>,
}

/// Kullanım süresi motoruna agent çekirdeğinden erişim (P1.C ADR-0014, P1.D ADR-0015).
///
/// İşlemler: helper'ın etkinlik raporunu iletmek, bağlantının bittiğini
/// bildirmek ve durumu okumak. Kural değiştirme veya kısıtlama uygulama
/// işlemi **yoktur**.
pub trait ScreenTimeService: Send + Sync + std::fmt::Debug {
    /// Dönüş: rapor kabul edildi mi (etkin konsol oturumu ve tek raporlayıcı kuralı).
    fn report_activity(&self, client: &ClientInfo, report: ActivityReport) -> bool;
    /// Bağlantı kapandı; bu bağlantı raporlayıcıysa yeri boşalır.
    fn connection_closed(&self, _connection_id: u64) {}
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
    next_connection: AtomicU64,
    screen_time: Option<Arc<dyn ScreenTimeService>>,
}

impl AgentState {
    pub fn new(run_mode: RunMode) -> Self {
        AgentState {
            run_mode,
            started: Instant::now(),
            connected: AtomicU32::new(0),
            next_connection: AtomicU64::new(1),
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

    /// Yeni bağlantı kimliği (1'den başlar, tekrar etmez).
    pub fn next_connection_id(&self) -> u64 {
        self.next_connection.fetch_add(1, Ordering::SeqCst)
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
