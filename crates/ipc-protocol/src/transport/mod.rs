//! Yerel taşıma katmanı.
//!
//! - Windows (hedef): erişim listesi kısıtlanmış named pipe — [`windows_pipe`].
//! - Unix (yalnız geliştirme/test): sahibine özel dizinde Unix domain socket —
//!   [`unix_dev`]. Üretim taşıması değildir.
//!
//! Eş kimliği ([`PeerIdentity`]) her zaman **işletim sisteminden** alınır;
//! mesaj içindeki hiçbir alan kimlik olarak kullanılmaz.

use std::path::PathBuf;

/// İşletim sisteminin bildirdiği bağlanan süreç kimliği. Alınamayan alanlar `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeerIdentity {
    pub pid: Option<u32>,
    /// Unix kullanıcı kimliği (yalnız Unix geliştirme taşıması).
    pub uid: Option<u32>,
    /// Windows oturum kimliği (Session 0 = servisler).
    pub session_id: Option<u32>,
    /// İstemci sürecin çalıştırılabilir dosya yolu.
    pub exe_path: Option<PathBuf>,
    /// Kimlik alanları alınamadığında işletim sistemi hata kodları
    /// (ör. `OpenProcess=5`). Yalnız kayıt/teşhis içindir; yetki kararı vermez.
    pub diagnostics: Vec<String>,
}

#[cfg(unix)]
pub mod unix_dev;

#[cfg(windows)]
pub mod windows_pipe;
