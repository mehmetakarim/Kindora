//! Bağlanan istemcinin yetkilendirilmesi.
//!
//! Karar yalnız işletim sisteminin bildirdiği [`PeerIdentity`] üzerinden
//! verilir. Bir alan alınamadıysa ve politika o alanı gerektiriyorsa istek
//! **reddedilir** (fail closed).

use ipc_protocol::transport::PeerIdentity;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyReason {
    UidUnknown,
    UidMismatch,
    SessionUnknown,
    NonInteractiveSession,
    ExePathUnknown,
    ExeNameMismatch,
    ExeDirMismatch,
}

pub trait Authorizer: Send + Sync {
    fn authorize(&self, peer: &PeerIdentity) -> Result<(), DenyReason>;
}

/// Session Helper için yetkilendirme politikası.
#[derive(Debug, Clone, Default)]
pub struct HelperPolicy {
    /// Unix geliştirme taşıması: istemci bu kullanıcı kimliğiyle çalışmalı.
    pub required_uid: Option<u32>,
    /// Windows: istemci Session 0 dışında (etkileşimli oturumda) olmalı.
    pub require_interactive_session: bool,
    /// İstemci çalıştırılabilir dosya adı (büyük/küçük harf duyarsız).
    pub required_exe_name: Option<String>,
    /// İstemci çalıştırılabilir dosyasının bulunması gereken dizin.
    pub required_exe_dir: Option<PathBuf>,
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

impl Authorizer for HelperPolicy {
    fn authorize(&self, peer: &PeerIdentity) -> Result<(), DenyReason> {
        if let Some(uid) = self.required_uid {
            match peer.uid {
                None => return Err(DenyReason::UidUnknown),
                Some(u) if u != uid => return Err(DenyReason::UidMismatch),
                Some(_) => {}
            }
        }
        if self.require_interactive_session {
            match peer.session_id {
                None => return Err(DenyReason::SessionUnknown),
                Some(0) => return Err(DenyReason::NonInteractiveSession),
                Some(_) => {}
            }
        }
        if self.required_exe_name.is_some() || self.required_exe_dir.is_some() {
            let exe = peer.exe_path.as_deref().ok_or(DenyReason::ExePathUnknown)?;
            if let Some(name) = &self.required_exe_name {
                let ok = exe
                    .file_name()
                    .map(|f| f.to_string_lossy().eq_ignore_ascii_case(name))
                    .unwrap_or(false);
                if !ok {
                    return Err(DenyReason::ExeNameMismatch);
                }
            }
            if let Some(dir) = &self.required_exe_dir {
                if !exe.parent().map(|p| same_path(p, dir)).unwrap_or(false) {
                    return Err(DenyReason::ExeDirMismatch);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win_policy() -> HelperPolicy {
        HelperPolicy {
            required_uid: None,
            require_interactive_session: true,
            required_exe_name: Some("kindora-helper.exe".into()),
            required_exe_dir: Some(PathBuf::from(r"C:\Program Files\Kindora")),
        }
    }

    fn win_peer(session: Option<u32>, exe: Option<&str>) -> PeerIdentity {
        PeerIdentity {
            pid: Some(42),
            uid: None,
            session_id: session,
            exe_path: exe.map(PathBuf::from),
            ..Default::default()
        }
    }

    #[test]
    fn uid_policy() {
        let p = HelperPolicy {
            required_uid: Some(501),
            ..Default::default()
        };
        let peer = |uid| PeerIdentity {
            uid,
            ..Default::default()
        };
        assert_eq!(p.authorize(&peer(Some(501))), Ok(()));
        assert_eq!(p.authorize(&peer(Some(502))), Err(DenyReason::UidMismatch));
        assert_eq!(p.authorize(&peer(None)), Err(DenyReason::UidUnknown));
    }

    #[test]
    fn windows_policy_accepts_expected_helper() {
        // Windows yolları test makinesinde (macOS) da dize olarak karşılaştırılır.
        let p = HelperPolicy {
            required_exe_dir: None,
            ..win_policy()
        };
        let exe = if cfg!(windows) {
            r"C:\Program Files\Kindora\Kindora-Helper.exe"
        } else {
            "/opt/kindora/Kindora-Helper.exe"
        };
        assert_eq!(p.authorize(&win_peer(Some(1), Some(exe))), Ok(()));
    }

    #[test]
    fn windows_policy_rejects_unexpected_clients() {
        let p = HelperPolicy {
            required_exe_dir: None,
            ..win_policy()
        };
        assert_eq!(
            p.authorize(&win_peer(Some(0), Some("kindora-helper.exe"))),
            Err(DenyReason::NonInteractiveSession)
        );
        assert_eq!(
            p.authorize(&win_peer(None, Some("kindora-helper.exe"))),
            Err(DenyReason::SessionUnknown)
        );
        assert_eq!(
            p.authorize(&win_peer(Some(1), None)),
            Err(DenyReason::ExePathUnknown)
        );
        assert_eq!(
            p.authorize(&win_peer(Some(1), Some("powershell.exe"))),
            Err(DenyReason::ExeNameMismatch)
        );
    }

    #[test]
    fn exe_dir_must_match() {
        let p = HelperPolicy {
            required_uid: None,
            require_interactive_session: false,
            required_exe_name: Some("kindora-helper".into()),
            required_exe_dir: Some(PathBuf::from("/opt/kindora")),
        };
        let peer = |p: &str| PeerIdentity {
            exe_path: Some(PathBuf::from(p)),
            ..Default::default()
        };
        assert_eq!(p.authorize(&peer("/opt/kindora/kindora-helper")), Ok(()));
        assert_eq!(
            p.authorize(&peer("/tmp/kindora-helper")),
            Err(DenyReason::ExeDirMismatch)
        );
    }
}
