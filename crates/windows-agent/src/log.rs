//! Satır tabanlı olay kaydı: `ts_ms=<..> event=<ad> anahtar=değer ...`
//!
//! Konsol modunda stderr'e, servis modunda ayrıca bir dosyaya yazar. Çocuğa ait
//! içerik veya kimlik bilgisi kaydedilmez; yalnız süreç/bağlantı olayları.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static FILE_SINK: OnceLock<Mutex<File>> = OnceLock::new();

pub fn init_file(path: &Path) {
    if let Ok(f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = FILE_SINK.set(Mutex::new(f));
    }
}

pub fn event(name: &str, fields: &[(&str, String)]) {
    let mut line = format!("ts_ms={} event={}", ipc_protocol::now_ms(), name);
    for (k, v) in fields {
        if v.contains(char::is_whitespace) || v.is_empty() {
            line.push_str(&format!(" {k}={v:?}"));
        } else {
            line.push_str(&format!(" {k}={v}"));
        }
    }
    eprintln!("{line}");
    if let Some(f) = FILE_SINK.get() {
        if let Ok(mut f) = f.lock() {
            let _ = writeln!(f, "{line}");
        }
    }
}
