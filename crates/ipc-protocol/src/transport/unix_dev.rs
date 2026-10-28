//! YALNIZ GELİŞTİRME: macOS/Linux üzerinde agent ve helper'ı uçtan uca
//! çalıştırabilmek için Unix domain socket taşıması. Windows davranışını
//! temsil etmez.
//!
//! Erişim kısıtı: soket, sahibine özel (0700) bir dizinde 0600 izinle açılır;
//! eş kullanıcı kimliği çekirdekten (`getpeereid` / `SO_PEERCRED`) alınır.

use super::PeerIdentity;
use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn default_socket_path() -> PathBuf {
    std::env::var_os("KINDORA_DEV_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("kindora-dev").join("agent.sock"))
}

pub fn current_uid() -> u32 {
    // SAFETY: getuid her zaman başarılıdır.
    unsafe { libc::getuid() }
}

pub struct DevListener {
    inner: UnixListener,
    path: PathBuf,
}

impl DevListener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let dir = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "socket path has no parent")
        })?;
        fs::create_dir_all(dir)?;
        let meta = fs::metadata(dir)?;
        if meta.uid() != current_uid() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "socket directory is owned by another user",
            ));
        }
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;

        // Yalnız bayat bir *soket* dosyası silinir; başka dosya türlerine dokunulmaz.
        if let Ok(m) = fs::symlink_metadata(path) {
            if m.file_type().is_socket() {
                fs::remove_file(path)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "socket path exists and is not a socket",
                ));
            }
        }
        let inner = UnixListener::bind(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        inner.set_nonblocking(true)?;
        Ok(DevListener {
            inner,
            path: path.to_owned(),
        })
    }

    /// Bloklamayan kabul. Bekleyen bağlantı yoksa `Ok(None)`.
    pub fn accept(&self, read_timeout: Duration) -> io::Result<Option<(UnixStream, PeerIdentity)>> {
        match self.inner.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false)?;
                stream.set_read_timeout(Some(read_timeout))?;
                stream.set_write_timeout(Some(read_timeout))?;
                let peer = peer_identity(&stream);
                Ok(Some((stream, peer)))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for DevListener {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn connect(path: &Path, io_timeout: Duration) -> io::Result<UnixStream> {
    let s = UnixStream::connect(path)?;
    s.set_read_timeout(Some(io_timeout))?;
    s.set_write_timeout(Some(io_timeout))?;
    Ok(s)
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn peer_identity(stream: &UnixStream) -> PeerIdentity {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: geçerli soket tanımlayıcısı ve geçerli çıktı işaretçileri.
    let rc = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    PeerIdentity {
        uid: (rc == 0).then_some(uid),
        ..Default::default()
    }
}

#[cfg(target_os = "linux")]
fn peer_identity(stream: &UnixStream) -> PeerIdentity {
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: SO_PEERCRED için doğru boyutta çıktı yapısı.
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };
    if rc != 0 {
        return PeerIdentity::default();
    }
    PeerIdentity {
        pid: u32::try_from(cred.pid).ok(),
        uid: Some(cred.uid),
        ..Default::default()
    }
}

/// Desteklenmeyen Unix türevlerinde kimlik alınamaz → yetkilendirme reddeder.
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "linux"
)))]
fn peer_identity(_stream: &UnixStream) -> PeerIdentity {
    PeerIdentity::default()
}
