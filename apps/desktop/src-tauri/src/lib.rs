//! Kindora Desktop — P1.B masaüstü kabuğu.
//!
//! Güven sınırı (ADR-0013): bu süreç agent'a **bağlanmaz**. Agent durumunu
//! yanındaki `kindora-helper.exe`'yi `bridge` modunda alt süreç olarak çalıştırıp
//! stdio üzerinden okur. Ön yüz yalnız iki komut çağırabilir
//! (`get_agent_state`, `refresh_agent`) ve `agent-state` olayını dinleyebilir.
//!
//! Bu sürüm hiçbir kısıtlama uygulamaz: tam ekran, always-on-top, giriş
//! engelleme, otomatik başlatma, yetki yükseltme veya servis kurulumu yoktur.
//! Pencereyi kapatmak yalnız masaüstü uygulamasını ve onun helper'ını kapatır;
//! Windows servisi (agent) SCM tarafından yönetilir ve çalışmaya devam eder.

pub mod logging;
pub mod supervisor;
pub mod view;

use logging::Logger;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use supervisor::{HelperCommand, Supervisor};
use tauri::{Emitter, Manager, RunEvent, State, WindowEvent};
use view::AgentView;

pub const AGENT_STATE_EVENT: &str = "agent-state";
/// Maskotun "güvenli kapanış" mesajının görünmesi için kısa süre.
const CLOSE_MESSAGE_DELAY: Duration = Duration::from_millis(700);
const HELPER_STOP_TIMEOUT: Duration = Duration::from_secs(2);

struct AppState {
    supervisor: Supervisor,
    closing: AtomicBool,
}

#[tauri::command]
fn get_agent_state(state: State<'_, AppState>) -> AgentView {
    state.supervisor.view()
}

#[tauri::command]
fn refresh_agent(state: State<'_, AppState>) -> Result<(), String> {
    state.supervisor.refresh()
}

fn helper_command() -> std::io::Result<HelperCommand> {
    // Yalnız geliştirme derlemesi: `tauri dev` sırasında helper başka klasördedir.
    #[cfg(debug_assertions)]
    if let Some(p) = std::env::var_os("KINDORA_HELPER_PATH") {
        return Ok(HelperCommand {
            program: p.into(),
            args: vec!["bridge".into()],
        });
    }
    HelperCommand::beside_current_exe()
}

pub fn run() {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_agent_state, refresh_agent])
        .setup(|app| {
            let log = Arc::new(Logger::open_in(app.path().app_log_dir().ok()));
            let cmd = helper_command()?;
            log.event(
                "app_started",
                &[
                    ("version", env!("CARGO_PKG_VERSION").into()),
                    ("pid", std::process::id().to_string()),
                    ("helper", cmd.program.display().to_string()),
                    (
                        "log",
                        log.path()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default(),
                    ),
                ],
            );
            let handle = app.handle().clone();
            let supervisor = Supervisor::start(
                cmd,
                log,
                Box::new(move |v: &AgentView| {
                    let _ = handle.emit(AGENT_STATE_EVENT, v);
                }),
            );
            app.manage(AppState {
                supervisor,
                closing: AtomicBool::new(false),
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.closing.swap(true, Ordering::SeqCst) {
                    return; // ikinci kapatma isteği: beklemeden kapan
                }
                api.prevent_close();
                state.supervisor.set_closing();
                let supervisor = state.supervisor.clone();
                let app = window.app_handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(CLOSE_MESSAGE_DELAY);
                    supervisor.shutdown(HELPER_STOP_TIMEOUT);
                    app.exit(0);
                });
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to build Kindora Desktop");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.supervisor.shutdown(HELPER_STOP_TIMEOUT);
            }
        }
    });
}
