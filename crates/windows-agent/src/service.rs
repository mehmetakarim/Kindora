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

use crate::{log, run_agent, AgentConfig};
use ipc_protocol::RunMode;
use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
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
    let handler = move |control| match control {
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
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        ServiceExitCode::Win32(0),
    ))?;

    let result = run_agent(RunMode::Service, AgentConfig::default(), shutdown);

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
