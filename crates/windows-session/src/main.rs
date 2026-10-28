use ipc_protocol::{CommandType, ResponseStatus};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use windows_session::{log_event, run_helper, Client, HelperConfig};

const USAGE: &str = "\
Kindora Session Helper (P1.A prototip — kısıtlama arayüzü yok)

Kullanım:
  kindora-helper run [--heartbeat-ms N] [--socket PATH]
      Agent'a bağlanır, kalp atışı gönderir; Ctrl+C ile kapanır.
  kindora-helper ping|status|health [--socket PATH]
      Tek istek gönderir, yanıtı JSON olarak yazar.
      Çıkış kodu: 0 OK, 1 agent hata yanıtı, 3 bağlantı/iletişim hatası.
  --socket yalnız Unix geliştirme taşımasında geçerlidir.
";

fn parse(args: &[String]) -> Result<HelperConfig, String> {
    let mut cfg = HelperConfig::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} için değer eksik"));
        match a.as_str() {
            "--heartbeat-ms" => {
                let ms: u64 = value()?.parse().map_err(|_| "geçersiz --heartbeat-ms")?;
                cfg.heartbeat_interval = Duration::from_millis(ms.max(50));
            }
            #[cfg(unix)]
            "--socket" => cfg.socket_path = value()?.into(),
            other => return Err(format!("bilinmeyen argüman: {other}")),
        }
    }
    Ok(cfg)
}

fn one_shot(cfg: &HelperConfig, command: CommandType) -> ExitCode {
    let result = Client::connect(cfg).and_then(|mut c| c.request(command));
    match result {
        Ok(resp) => {
            println!("{}", serde_json::to_string(&resp).unwrap_or_default());
            if resp.status == ResponseStatus::Ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("hata: {e}");
            ExitCode::from(3)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().map(String::as_str) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let cfg = match parse(&args[1..]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    // BUG-003: LocalService agent'ın bu sürecin exe yolunu okuyabilmesi için.
    // Başarısızlık helper'ı durdurmaz; agent yolu okuyamazsa zaten reddeder.
    #[cfg(windows)]
    if let Err(e) = ipc_protocol::transport::windows_pipe::grant_agent_process_query() {
        log_event("process_query_grant_failed", &[("error", e.to_string())]);
    }
    match cmd {
        "run" => {
            let shutdown = Arc::new(AtomicBool::new(false));
            let flag = shutdown.clone();
            if let Err(e) = ctrlc::set_handler(move || {
                log_event("shutdown_requested", &[("source", "ctrl_c".into())]);
                flag.store(true, Ordering::SeqCst);
            }) {
                eprintln!("Ctrl+C işleyicisi kurulamadı: {e}");
                return ExitCode::FAILURE;
            }
            run_helper(&cfg, &shutdown);
            ExitCode::SUCCESS
        }
        "ping" => one_shot(&cfg, CommandType::Ping),
        "status" => one_shot(&cfg, CommandType::GetStatus),
        "health" => one_shot(&cfg, CommandType::HealthCheck),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
