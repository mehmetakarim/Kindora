use ipc_protocol::{CommandType, ReporterKind, ResponseStatus};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use windows_session::{
    default_activity_log, instance, log_event, run_activity, run_bridge, set_log_file,
    write_bridge_event, Client, HelperConfig, RunMarker,
};

const USAGE: &str = "\
Kindora Session Helper (P1.D önizleme — kısıtlama arayüzü yok)

Kullanım:
  kindora-helper activity [--launch logon-task|manual] [--interval-ms N]
                          [--log-file PATH] [--socket PATH]
      Bağımsız arka plan raporlayıcısı: bu oturumun son girdiden bu yana
      geçen süresini agent'a bildirir. Oturum başına tek örnek; ikincisi
      hemen çıkar (kod 0). Masaüstü uygulamasından bağımsızdır. Ctrl+C veya
      Windows oturumunun kapanmasıyla temiz çıkar. `run` eski adıdır.
      Windows kaydı: %LOCALAPPDATA%\\Kindora\\helper\\activity.log
  kindora-helper bridge [--socket PATH]
      Masaüstü uygulamasının alt süreci: stdin'den JSON istek satırları
      (yalnız op=refresh) okur, stdout'a agent durumunu JSON satırları olarak yazar; stdin
      kapanınca çıkar. Elle kullanım için değildir.
  kindora-helper ping|status|health|screen-time [--socket PATH]
      Tek istek gönderir, yanıtı JSON olarak yazar.
  kindora-helper report-activity [--socket PATH]
      Bu oturumun son girdiden bu yana geçen süresini bir kez bildirir.
  kindora-helper idle
      Yalnız yerel ölçüm: son girdiden bu yana geçen süreyi (ms) yazar.
      Çıkış kodu: 0 OK, 1 agent hata yanıtı, 3 bağlantı/iletişim hatası.
  --socket yalnız Unix geliştirme taşımasında geçerlidir.
";

struct Args {
    cfg: HelperConfig,
    log_file: Option<PathBuf>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut cfg = HelperConfig::default();
    let mut log_file = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} için değer eksik"));
        match a.as_str() {
            "--heartbeat-ms" | "--interval-ms" => {
                let ms: u64 = value()?.parse().map_err(|_| format!("geçersiz {a}"))?;
                cfg.heartbeat_interval = Duration::from_millis(ms.max(50));
            }
            "--launch" => {
                cfg.reporter_kind = match value()?.as_str() {
                    "logon-task" => ReporterKind::BackgroundLogonTask,
                    "manual" => ReporterKind::BackgroundManual,
                    other => return Err(format!("geçersiz --launch: {other}")),
                }
            }
            "--log-file" => log_file = Some(PathBuf::from(value()?)),
            #[cfg(unix)]
            "--socket" => cfg.socket_path = value()?.into(),
            other => return Err(format!("bilinmeyen argüman: {other}")),
        }
    }
    Ok(Args { cfg, log_file })
}

/// Arka plan raporlayıcısı. Çıkış: 0 (temiz / zaten çalışıyor), 1 (kilit hatası).
fn activity(mut cfg: HelperConfig, log_file: Option<PathBuf>) -> ExitCode {
    if !ReporterKind::is_background(Some(cfg.reporter_kind)) {
        cfg.reporter_kind = ReporterKind::BackgroundManual;
    }
    let log_file = log_file.or_else(default_activity_log);
    if let Some(path) = &log_file {
        if let Err(e) = set_log_file(path) {
            eprintln!("kayıt dosyası açılamadı ({}): {e}", path.display());
        }
    }
    let _guard = match instance::acquire(&cfg) {
        Ok(Some(g)) => g,
        Ok(None) => {
            log_event(
                "already_running",
                &[
                    ("pid", std::process::id().to_string()),
                    ("kind", cfg.reporter_kind.as_str().into()),
                ],
            );
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            log_event("instance_lock_failed", &[("error", e.to_string())]);
            return ExitCode::from(1);
        }
    };
    let (marker, previous) = match &log_file {
        Some(p) => RunMarker::create(p),
        None => (None, None),
    };
    if let Some(prev) = previous {
        // Önceki örnek sonlandırıldı veya çöktü (temiz çıkış izini silemedi).
        log_event(
            "previous_exit",
            &[("state", "unclean".into()), ("previous", prev)],
        );
    }

    let shutdown = Arc::new(AtomicBool::new(false));
    let stopped = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        log_event("shutdown_requested", &[("source", "ctrl_c".into())]);
        flag.store(true, Ordering::SeqCst);
    }) {
        log_event("ctrl_c_handler_failed", &[("error", e.to_string())]);
    }
    #[cfg(windows)]
    if let Err(e) = windows_session::session_end::watch(shutdown.clone(), stopped.clone()) {
        log_event("session_end_watch_failed", &[("error", e.to_string())]);
    }
    run_activity(&cfg, &shutdown);
    if let Some(m) = marker {
        m.clear();
    }
    stopped.store(true, Ordering::SeqCst);
    ExitCode::SUCCESS
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

/// Masaüstü köprüsü: stdin satırları → istek kanalı; olaylar → stdout.
fn bridge(cfg: &HelperConfig) -> ExitCode {
    use ipc_protocol::bridge::{decode_bridge_request, BridgeEvent, MAX_BRIDGE_LINE_BYTES};
    use std::io::{BufRead, Read};
    use std::sync::mpsc;

    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    // Okuyucu iş parçacığı stdin EOF'unda kanalı bırakır → köprü kapanır.
    // Reddedilen satır hemen bildirilir (stdout kilidiyle, olaylar karışmaz).
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        let mut line = String::new();
        loop {
            line.clear();
            // Satır sınırı: sınırsız bellek kullanımı yok.
            let mut limited = (&mut input).take(MAX_BRIDGE_LINE_BYTES as u64 + 1);
            match limited.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let too_long = !line.ends_with('\n') && line.len() > MAX_BRIDGE_LINE_BYTES;
            let result = if too_long {
                Err("line too long".to_string())
            } else {
                decode_bridge_request(&line).map_err(|e| e.to_string())
            };
            match result {
                Ok(req) => {
                    if tx.send(req).is_err() {
                        break;
                    }
                }
                Err(reason) => {
                    let event = BridgeEvent::RequestRejected { reason };
                    if !write_bridge_event(&mut std::io::stdout().lock(), &event) || too_long {
                        break;
                    }
                }
            }
        }
    });

    let mut emit = |e: &BridgeEvent| write_bridge_event(&mut std::io::stdout().lock(), e);
    run_bridge(cfg, &rx, &shutdown, &mut emit);
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().map(String::as_str) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let Args { mut cfg, log_file } = match parse(&args[1..]) {
        Ok(a) => a,
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
        "activity" | "run" => activity(cfg, log_file),
        "bridge" => {
            // Köprü yalnız yedek raporlayıcıdır; agent arka plan helper'ını önceler.
            cfg.reporter_kind = ReporterKind::DesktopBridge;
            bridge(&cfg)
        }
        "ping" => one_shot(&cfg, CommandType::Ping),
        "status" => one_shot(&cfg, CommandType::GetStatus),
        "health" => one_shot(&cfg, CommandType::HealthCheck),
        "screen-time" => one_shot(&cfg, CommandType::GetScreenTime),
        "report-activity" => {
            let idle = windows_session::activity::idle_ms();
            match Client::connect(&cfg).and_then(|mut c| c.report_activity(idle)) {
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
        "idle" => match windows_session::activity::idle_ms() {
            Some(ms) => {
                println!("{{\"idle_ms\":{ms}}}");
                ExitCode::SUCCESS
            }
            None => {
                println!("{{\"idle_ms\":null}}");
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
