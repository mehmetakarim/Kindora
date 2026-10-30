//! Windows Service Control Manager entegrasyonu (P1.A).
//!
//! DURUM: yalnız derleme denetimi; **Windows'ta çalıştırılmadı**.
//!
//! - Servis otomatik kurulmaz. Elle kurulum/kaldırma komutları:
//!   `docs/agent/p1a-windows-agent.md`.
//! - Servis hesabı kodda seçilmez; `sc.exe create ... obj=` ile verilir
//!   (ADR-0006 açık). Önerilen ilk deneme: `NT AUTHORITY\LocalService`.
//! - Stop/Shutdown kontrolü: kapanma bayrağı → bağlantılar en geç yoklama
//!   aralığında kapanır → StopPending → Stopped.
//! - P1.C: `POWER_EVENT` kabul edilir; uyku/uyanma kullanım süresi motoruna
//!   iletilir (uyku süresi aktif kullanım sayılmaz).

use crate::platform::{self, WindowsSignals};
use crate::screen_time::{dev_policy_beside_exe, ScreenTimeConfig};
use crate::{log, run_agent, AgentConfig};
use ipc_protocol::RunMode;
use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use windows_service::service::{
    PowerEventParam, ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
    ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{define_windows_service, service_dispatcher};

pub const SERVICE_NAME: &str = "KindoraAgentDev";

define_windows_service!(ffi_service_main, service_main);

/// SCM tarafından `kindora-agent.exe service` ile başlatıldığında çağrılır.
pub fn run_dispatcher() -> windows_service::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn service_main(_args: Vec<OsString>) {
    log::init_file(&std::env::temp_dir().join("kindora-agent-dev.log"));
    if let Err(e) = run_service() {
        log::event("service_error", &[("error", e.to_string())]);
    }
}

fn status(
    state: ServiceState,
    accept: ServiceControlAccept,
    exit: ServiceExitCode,
) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: accept,
        exit_code: exit,
        checkpoint: 0,
        wait_hint: Duration::from_secs(5),
        process_id: None,
    }
}

fn run_service() -> windows_service::Result<()> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();
    let signals = Arc::new(WindowsSignals::default());
    let power = signals.clone();
    let handler = move |control| match control {
        ServiceControl::PowerEvent(p) => {
            let suspending = match p {
                PowerEventParam::Suspend => Some(true),
                PowerEventParam::ResumeAutomatic | PowerEventParam::ResumeSuspend => Some(false),
                _ => None,
            };
            // P1.D tanı: Lenovo'da uyku "ölçülemeyen" sayıldı (P1.C C-08); hangi
            // güç olaylarının geldiği kayda geçsin. Yalnız üç olay durumu değiştirir.
            // Pil/şarj durumu değişimi sık gelir; kayıt şişmesin diye yazılmaz.
            if !matches!(p, PowerEventParam::PowerStatusChange) {
                log::event(
                    "power_event",
                    &[
                        ("event", format!("{p:?}")),
                        ("applied", suspending.is_some().to_string()),
                    ],
                );
            }
            if let Some(s) = suspending {
                power.on_power_event(s);
            }
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Stop | ServiceControl::Shutdown => {
            log::event(
                "service_stop_requested",
                &[("control", format!("{control:?}"))],
            );
            flag.store(true, Ordering::SeqCst);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let handle = service_control_handler::register(SERVICE_NAME, handler)?;
    handle.set_service_status(status(
        ServiceState::Running,
        ServiceControlAccept::STOP
            | ServiceControlAccept::SHUTDOWN
            | ServiceControlAccept::POWER_EVENT,
        ServiceExitCode::Win32(0),
    ))?;

    let mut st = ScreenTimeConfig::new(platform::default_data_dir(false), dev_policy_beside_exe());
    st.signals = signals;
    let cfg = AgentConfig {
        screen_time: Some(st),
        ..AgentConfig::default()
    };
    let result = run_agent(RunMode::Service, cfg, shutdown);

    handle.set_service_status(status(
        ServiceState::StopPending,
        ServiceControlAccept::empty(),
        ServiceExitCode::Win32(0),
    ))?;
    let exit = match result {
        Ok(()) => ServiceExitCode::Win32(0),
        Err(_) => ServiceExitCode::ServiceSpecific(1),
    };
    handle.set_service_status(status(
        ServiceState::Stopped,
        ServiceControlAccept::empty(),
        exit,
    ))
}
