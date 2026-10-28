//! Session Helper köprüsünün (`kindora-helper bridge`) yaşam döngüsü.
//!
//! - Helper, masaüstü uygulamasıyla **aynı klasörden** başlatılır; yol ön yüzden
//!   veya yapılandırmadan seçilmez. Agent yalnız kendi klasöründeki helper'ı
//!   yetkilendirir, bu yüzden üç EXE aynı klasörde durmalıdır (ADR-0013).
//! - Helper kapanırsa (çökme, sonlandırma) üstel geri çekilmeyle (1 → 30 sn)
//!   yeniden başlatılır.
//! - Kapanışta önce stdin kapatılır (helper temiz çıkar), süre aşılırsa
//!   süreç sonlandırılır; arkada helper süreci bırakılmaz.

use crate::logging::Logger;
use crate::view::AgentView;
use ipc_protocol::bridge::{
    decode_bridge_event, encode_bridge_line, BridgeEvent, BridgeRequest, LinkState,
    MAX_BRIDGE_LINE_BYTES,
};
use ipc_protocol::now_ms;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const RESTART_MIN: Duration = Duration::from_secs(1);
const RESTART_MAX: Duration = Duration::from_secs(30);
/// Bu süreden uzun çalışmış helper'ın çıkışı geri çekilmeyi sıfırlar.
const STABLE_RUN: Duration = Duration::from_secs(60);
/// Ön yüzden gelen yenileme istekleri en sık bu aralıkla iletilir.
const REFRESH_GAP: Duration = Duration::from_millis(1000);

/// Başlatılacak helper komutu.
#[derive(Debug, Clone)]
pub struct HelperCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl HelperCommand {
    /// Üretim: bu EXE'nin klasöründeki `kindora-helper[.exe] bridge`.
    pub fn beside_current_exe() -> std::io::Result<HelperCommand> {
        let exe = std::env::current_exe()?;
        let dir = exe
            .parent()
            .ok_or_else(|| std::io::Error::other("executable has no parent directory"))?;
        Ok(HelperCommand {
            program: dir.join(format!("kindora-helper{}", std::env::consts::EXE_SUFFIX)),
            args: vec!["bridge".into()],
        })
    }
}

type Notify = Box<dyn Fn(&AgentView) + Send + Sync>;

struct Inner {
    cmd: HelperCommand,
    log: Arc<Logger>,
    notify: Notify,
    view: Mutex<AgentView>,
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Option<Child>>,
    stopping: AtomicBool,
    /// Kapanışta geri çekilme uykusunu hemen bitirmek için.
    wake: (Mutex<()>, Condvar),
    last_refresh: Mutex<Option<Instant>>,
}

#[derive(Clone)]
pub struct Supervisor {
    inner: Arc<Inner>,
    thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Inner {
    fn update(&self, f: impl FnOnce(&mut AgentView) -> bool) {
        let snapshot = {
            let Ok(mut v) = self.view.lock() else { return };
            if !f(&mut v) {
                return;
            }
            v.clone()
        };
        (self.notify)(&snapshot);
    }

    fn stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }

    /// `d` kadar bekler; kapanış istenirse hemen döner.
    fn sleep(&self, d: Duration) {
        let (lock, cv) = &self.wake;
        let deadline = Instant::now() + d;
        let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        while !self.stopping() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            guard = cv
                .wait_timeout(guard, left)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    fn spawn(&self) -> std::io::Result<(Child, ChildStdin, impl Read, impl Read)> {
        let mut cmd = Command::new(&self.cmd.program);
        cmd.args(&self.cmd.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // GUI uygulamasından başlatılan konsol helper'ı için pencere açılmaz.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        Ok((child, stdin, stdout, stderr))
    }

    /// Helper'ın bitmesini bekler (zaten çıktıysa hemen döner).
    fn reap(&self) -> String {
        let Some(mut child) = self.child.lock().ok().and_then(|mut c| c.take()) else {
            return "stopped".into();
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.to_string(),
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return "killed after stdout closed".into();
                }
            }
        }
    }

    fn run(self: &Arc<Self>) {
        let mut backoff = RESTART_MIN;
        let mut last_link: Option<LinkState> = None;
        while !self.stopping() {
            if !self.cmd.program.is_file() {
                let p = self.cmd.program.display().to_string();
                self.log.event("helper_missing", &[("path", p.clone())]);
                self.update(|v| {
                    v.helper_missing(p, now_ms());
                    true
                });
                self.sleep(Duration::from_secs(10));
                continue;
            }

            let (child, stdin, stdout, stderr) = match self.spawn() {
                Ok(parts) => parts,
                Err(e) => {
                    self.log
                        .event("helper_spawn_failed", &[("error", e.to_string())]);
                    self.update(|v| {
                        v.helper_exited(format!("spawn failed: {e}"), now_ms());
                        true
                    });
                    self.sleep(backoff);
                    backoff = (backoff * 2).min(RESTART_MAX);
                    continue;
                }
            };
            let started = Instant::now();
            self.log
                .event("helper_spawned", &[("pid", child.id().to_string())]);
            if let Ok(mut c) = self.child.lock() {
                *c = Some(child);
            }
            if let Ok(mut s) = self.stdin.lock() {
                *s = Some(stdin);
            }
            self.update(|v| {
                v.helper_spawned(now_ms());
                true
            });

            // Helper'ın stderr'i yalnız durum geçişlerini içerir; kayda aktarılır.
            // İş parçacığı boru kapanınca kendiliğinden biter; beklenmez (borunun
            // başka bir süreçte açık kalması yeniden başlatmayı geciktirmesin).
            let log = self.log.clone();
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    log.event("helper_log", &[("line", line)]);
                }
            });

            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                let mut limited = (&mut reader).take(MAX_BRIDGE_LINE_BYTES as u64 + 1);
                match limited.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                match decode_bridge_event(&line) {
                    Ok(event) => {
                        if let BridgeEvent::Link { state, reason, .. } = &event {
                            // Kayıt yalnız yerleşik durum değişince: her yeniden
                            // denemedeki "connecting" satırı yazılmaz.
                            if *state != LinkState::Connecting && last_link != Some(*state) {
                                last_link = Some(*state);
                                self.log.event(
                                    "agent_link",
                                    &[
                                        ("state", format!("{state:?}")),
                                        ("reason", reason.clone().unwrap_or_default()),
                                    ],
                                );
                            }
                        }
                        if let BridgeEvent::RequestRejected { reason } = &event {
                            self.log
                                .event("bridge_request_rejected", &[("reason", reason.clone())]);
                        }
                        self.update(|v| v.apply(event, now_ms()));
                    }
                    Err(e) => {
                        // Beklenmeyen çıktı: helper'a güvenilmez, yeniden başlatılır.
                        self.log
                            .event("bridge_bad_line", &[("error", e.to_string())]);
                        break;
                    }
                }
            }

            if let Ok(mut s) = self.stdin.lock() {
                s.take();
            }
            let status = self.reap();
            last_link = None;
            if self.stopping() {
                self.log.event("helper_stopped", &[("status", status)]);
                break;
            }
            self.log.event(
                "helper_exited",
                &[
                    ("status", status.clone()),
                    ("ran_ms", started.elapsed().as_millis().to_string()),
                    ("restart_in_ms", backoff.as_millis().to_string()),
                ],
            );
            self.update(|v| {
                v.helper_exited(status, now_ms());
                true
            });
            if started.elapsed() >= STABLE_RUN {
                backoff = RESTART_MIN;
            }
            self.sleep(backoff);
            backoff = (backoff * 2).min(RESTART_MAX);
        }
    }
}

impl Supervisor {
    pub fn start(cmd: HelperCommand, log: Arc<Logger>, notify: Notify) -> Supervisor {
        let inner = Arc::new(Inner {
            cmd,
            log,
            notify,
            view: Mutex::new(AgentView::new(now_ms())),
            stdin: Mutex::new(None),
            child: Mutex::new(None),
            stopping: AtomicBool::new(false),
            wake: (Mutex::new(()), Condvar::new()),
            last_refresh: Mutex::new(None),
        });
        let worker = inner.clone();
        let handle = thread::Builder::new()
            .name("helper-supervisor".into())
            .spawn(move || worker.run())
            .expect("spawn supervisor thread");
        Supervisor {
            inner,
            thread: Arc::new(Mutex::new(Some(handle))),
        }
    }

    pub fn view(&self) -> AgentView {
        self.inner
            .view
            .lock()
            .map(|v| v.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// Helper'dan agent durumunu hemen yeniden okumasını ister (en sık 1 sn'de bir).
    pub fn refresh(&self) -> Result<(), String> {
        let mut guard = self.inner.stdin.lock().map_err(|e| e.to_string())?;
        let Some(stdin) = guard.as_mut() else {
            return Err("helper is not running".into());
        };
        {
            let mut last = self.inner.last_refresh.lock().map_err(|e| e.to_string())?;
            if last.is_some_and(|t| t.elapsed() < REFRESH_GAP) {
                return Ok(());
            }
            *last = Some(Instant::now());
        }
        let mut line = encode_bridge_line(&BridgeRequest::Refresh);
        line.push('\n');
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|e| e.to_string())
    }

    pub fn set_closing(&self) {
        self.inner.update(|v| {
            v.set_closing(now_ms());
            true
        });
    }

    /// Helper'ı kapatır ve gözetim iş parçacığını bekler. Birden çok kez çağrılabilir.
    pub fn shutdown(&self, timeout: Duration) {
        if self.inner.stopping.swap(true, Ordering::SeqCst) {
            // Zaten kapanıyor; yalnız iş parçacığının bitmesini bekle.
        } else {
            self.inner.log.event("shutdown_requested", &[]);
        }
        self.inner.wake.1.notify_all();
        // stdin EOF → helper temiz çıkar.
        if let Ok(mut s) = self.inner.stdin.lock() {
            s.take();
        }
        let deadline = Instant::now() + timeout;
        loop {
            let exited = match self.inner.child.lock() {
                Ok(mut c) => match c.as_mut() {
                    None => true,
                    Some(child) => match child.try_wait() {
                        Ok(Some(_)) => true,
                        Ok(None) if Instant::now() < deadline => false,
                        _ => {
                            let _ = child.kill();
                            let _ = child.wait();
                            self.inner.log.event("helper_killed", &[]);
                            true
                        }
                    },
                },
                Err(_) => true,
            };
            if exited {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        if let Some(h) = self.thread.lock().ok().and_then(|mut t| t.take()) {
            let _ = h.join();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::view::HelperState;

    fn sh(script: &str) -> HelperCommand {
        HelperCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
        }
    }

    fn quiet() -> Arc<Logger> {
        Arc::new(Logger::open_in(None))
    }

    fn wait_until(sup: &Supervisor, what: &str, f: impl Fn(&AgentView) -> bool) -> AgentView {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(5) {
            let v = sup.view();
            if f(&v) {
                return v;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("{what}: {:?}", sup.view());
    }

    const HELLO: &str = r#"{"type":"hello","bridge_version":1,"helper_version":"t","pid":1}"#;
    const SNAP: &str = r#"{"type":"snapshot","agent_version":"0.1.0","run_mode":"SERVICE","uptime_secs":1,"connected_clients":1,"healthy":true,"checks":[],"received_at_ms":5,"round_trip_ms":1}"#;

    #[test]
    fn missing_helper_is_reported() {
        let sup = Supervisor::start(
            HelperCommand {
                program: "/nonexistent/kindora-helper".into(),
                args: vec![],
            },
            quiet(),
            Box::new(|_| {}),
        );
        let v = wait_until(&sup, "missing", |v| v.helper == HelperState::Missing);
        assert_eq!(v.link, LinkState::Disconnected);
        let t = Instant::now();
        sup.shutdown(Duration::from_secs(1));
        assert!(
            t.elapsed() < Duration::from_secs(1),
            "kapanış uykuyu beklememeli"
        );
    }

    #[test]
    fn events_update_view_and_crash_triggers_restart() {
        let notified = Arc::new(Mutex::new(0u32));
        let n = notified.clone();
        let sup = Supervisor::start(
            sh(&format!("echo '{HELLO}'; echo '{SNAP}'; sleep 0.3; exit 3")),
            quiet(),
            Box::new(move |_| *n.lock().unwrap() += 1),
        );
        wait_until(&sup, "connected", |v| {
            v.link == LinkState::Connected && v.agent.is_some()
        });
        let v = wait_until(&sup, "exited", |v| v.helper_restarts >= 1);
        assert!(v.agent.is_none(), "helper çıkınca agent verisi gösterilmez");
        // 1 sn geri çekilmeden sonra yeniden başlar.
        wait_until(&sup, "restarted", |v| {
            v.agent.is_some() && v.helper_restarts >= 1
        });
        assert!(*notified.lock().unwrap() >= 4);
        sup.shutdown(Duration::from_secs(2));
    }

    #[test]
    fn garbage_output_restarts_helper() {
        let sup = Supervisor::start(sh("echo 'not json'; sleep 5"), quiet(), Box::new(|_| {}));
        wait_until(&sup, "restart on bad line", |v| v.helper_restarts >= 1);
        sup.shutdown(Duration::from_secs(2));
    }

    #[test]
    fn shutdown_closes_stdin_then_kills_stubborn_helper() {
        // stdin EOF'unda çıkan helper → hızlı, temiz kapanış.
        let polite = Supervisor::start(
            sh(&format!("echo '{HELLO}'; cat >/dev/null")),
            quiet(),
            Box::new(|_| {}),
        );
        wait_until(&polite, "running", |v| v.helper == HelperState::Running);
        let t = Instant::now();
        polite.shutdown(Duration::from_secs(2));
        assert!(t.elapsed() < Duration::from_secs(1));

        // EOF'u yok sayan helper → süre dolunca sonlandırılır, sızıntı yok.
        let stubborn = Supervisor::start(
            sh(&format!(
                "trap '' INT; echo '{HELLO}'; exec sleep 30 </dev/null"
            )),
            quiet(),
            Box::new(|_| {}),
        );
        wait_until(&stubborn, "running", |v| v.helper == HelperState::Running);
        let t = Instant::now();
        stubborn.shutdown(Duration::from_millis(300));
        assert!(t.elapsed() < Duration::from_secs(2));
        assert!(stubborn.inner.child.lock().unwrap().is_none());
    }

    #[test]
    fn refresh_writes_whitelisted_line_and_is_throttled() {
        let sup = Supervisor::start(
            sh(&format!("echo '{HELLO}'; cat >/dev/null")),
            quiet(),
            Box::new(|_| {}),
        );
        wait_until(&sup, "running", |v| v.helper == HelperState::Running);
        assert!(sup.refresh().is_ok());
        assert!(
            sup.refresh().is_ok(),
            "kısma: ikinci istek sessizce yutulur"
        );
        sup.shutdown(Duration::from_secs(2));
        assert!(sup.refresh().is_err(), "kapanıştan sonra helper yok");
    }
}
