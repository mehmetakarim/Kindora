use ipc_protocol::RunMode;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use windows_agent::screen_time::{dev_policy_beside_exe, ScreenTimeConfig};
use windows_agent::{log, run_agent, AgentConfig};

const USAGE: &str = "\
Kindora Agent (P1.C önizleme — kullanım süresi ölçer; kilit/engelleme yok)

Kullanım:
  kindora-agent console [--idle-timeout-ms N] [--socket PATH]
                        [--data-dir DIR | --no-store] [--dev-policy FILE]
      Ön planda çalışır. Ctrl+C ile güvenli kapanır.
      --socket yalnız Unix geliştirme taşımasında geçerlidir.
      --data-dir: SQLite klasörü (varsayılan: Windows %LOCALAPPDATA%\\Kindora\\dev-console,
        Unix soket klasörü/data). --no-store: yalnız bellek.
      --dev-policy: DEV test politika dosyası (varsayılan: agent klasöründeki
        kindora-dev-policy.json). Üretim yetkilendirmesi değildir.
  kindora-agent service
      Yalnız Windows Service Control Manager tarafından çağrılır.
      Kurulum otomatik değildir; bkz. docs/agent/p1a-windows-agent.md
";

fn parse_console(args: &[String]) -> Result<AgentConfig, String> {
    let mut cfg = AgentConfig::default();
    let mut data_dir: Option<std::path::PathBuf> = None;
    let mut no_store = false;
    let mut dev_policy = dev_policy_beside_exe();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} için değer eksik"));
        match a.as_str() {
            "--idle-timeout-ms" => {
                let ms: u64 = value()?.parse().map_err(|_| "geçersiz --idle-timeout-ms")?;
                cfg.limits.idle_timeout = Duration::from_millis(ms);
            }
            #[cfg(unix)]
            "--socket" => cfg.socket_path = value()?.into(),
            "--data-dir" => data_dir = Some(value()?.into()),
            "--no-store" => no_store = true,
            "--dev-policy" => dev_policy = Some(value()?.into()),
            other => return Err(format!("bilinmeyen argüman: {other}")),
        }
    }
    let default_dir = || {
        #[cfg(unix)]
        {
            cfg.socket_path.parent().map(|d| d.join("data"))
        }
        #[cfg(windows)]
        {
            windows_agent::platform::default_data_dir(true)
        }
    };
    let dir = if no_store {
        None
    } else {
        data_dir.or_else(default_dir)
    };
    cfg.screen_time = Some(ScreenTimeConfig::new(dir, dev_policy));
    Ok(cfg)
}

fn run_console(args: &[String]) -> ExitCode {
    let cfg = match parse_console(args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        log::event("shutdown_requested", &[("source", "ctrl_c".into())]);
        flag.store(true, Ordering::SeqCst);
    }) {
        eprintln!("Ctrl+C işleyicisi kurulamadı: {e}");
        return ExitCode::FAILURE;
    }
    match run_agent(RunMode::Console, cfg, shutdown) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("console") => run_console(&args[1..]),
        #[cfg(windows)]
        Some("service") => match windows_agent::service::run_dispatcher() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("servis dağıtıcısı başlatılamadı (yalnız SCM çağırabilir): {e}");
                ExitCode::FAILURE
            }
        },
        #[cfg(not(windows))]
        Some("service") => {
            eprintln!("service modu yalnız Windows'ta desteklenir");
            ExitCode::from(2)
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
