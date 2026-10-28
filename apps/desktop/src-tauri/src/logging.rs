//! Satır tabanlı, boyutu sınırlı olay kaydı: `ts_ms=<..> event=<ad> anahtar=değer ...`
//! (agent ve helper ile aynı biçim).
//!
//! - Dosya [`MAX_FILE_BYTES`]'ı aşınca `desktop.log.1`'e döner; en fazla iki dosya.
//! - Yalnız süreç ve bağlantı geçişleri yazılır; çocuğa ait içerik, ekran veya
//!   tuş bilgisi kaydedilmez.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_VALUE_CHARS: usize = 400;

pub struct Logger {
    path: Option<PathBuf>,
    file: Mutex<Option<(File, u64)>>,
}

fn now_ms() -> u64 {
    ipc_protocol::now_ms()
}

fn open(path: &Path) -> Option<(File, u64)> {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size >= MAX_FILE_BYTES {
        let _ = fs::rename(path, path.with_extension("log.1"));
        return open(path);
    }
    let f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()?;
    Some((f, size))
}

pub fn format_line(ts_ms: u64, name: &str, fields: &[(&str, String)]) -> String {
    let mut line = format!("ts_ms={ts_ms} event={name}");
    for (k, v) in fields {
        let v: String = v
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .take(MAX_VALUE_CHARS)
            .collect();
        if v.is_empty() || v.contains(char::is_whitespace) || v.contains('"') {
            line.push_str(&format!(" {k}={v:?}"));
        } else {
            line.push_str(&format!(" {k}={v}"));
        }
    }
    line
}

impl Logger {
    /// Kayıt dizini oluşturulamazsa yalnız stderr'e yazar (uygulama durmaz).
    pub fn open_in(dir: Option<PathBuf>) -> Logger {
        let path = dir.and_then(|d| {
            fs::create_dir_all(&d).ok()?;
            Some(d.join("desktop.log"))
        });
        let file = path.as_deref().and_then(open);
        Logger {
            path,
            file: Mutex::new(file),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn event(&self, name: &str, fields: &[(&str, String)]) {
        let line = format_line(now_ms(), name, fields);
        #[cfg(debug_assertions)]
        eprintln!("{line}");
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        if let Some((f, size)) = guard.as_mut() {
            if writeln!(f, "{line}").is_ok() {
                *size += line.len() as u64 + 1;
            }
            if *size >= MAX_FILE_BYTES {
                *guard = self.path.as_deref().and_then(open);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_single_line_and_bounded() {
        let l = format_line(
            1,
            "x",
            &[
                ("a", "plain".into()),
                ("b", "two words".into()),
                ("c", "line1\nline2".into()),
                ("d", "y".repeat(1000)),
                ("e", String::new()),
            ],
        );
        assert!(!l.contains('\n'));
        assert!(l.starts_with("ts_ms=1 event=x a=plain b=\"two words\" c=\"line1 line2\""));
        assert!(l.len() < 600);
        assert!(l.ends_with(" e=\"\""));
    }

    #[test]
    fn rotates_when_full() {
        let dir = std::env::temp_dir().join(format!("kd-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("desktop.log"), vec![b'x'; MAX_FILE_BYTES as usize]).unwrap();
        let log = Logger::open_in(Some(dir.clone()));
        log.event("after_rotate", &[]);
        assert!(dir.join("desktop.log.1").exists());
        let fresh = fs::read_to_string(dir.join("desktop.log")).unwrap();
        assert!(fresh.contains("event=after_rotate"));
        assert!(fresh.len() < 200);
        let _ = fs::remove_dir_all(&dir);
    }
}
