//! Çerçeveleme: `u32` little-endian gövde uzunluğu + JSON gövde.
//!
//! Okuma zaman aşımları taşıma katmanında kısa bir yoklama aralığı olarak
//! ayarlanır. Çerçeve **başlamadan** oluşan zaman aşımı [`FrameError::Idle`]
//! döner (bağlantı sağlam, yalnız sessiz). Çerçevenin **ortasında** oluşan zaman
//! aşımı senkron kaybı demektir ve [`FrameError::Truncated`] döner; çağıran
//! bağlantıyı kapatmalıdır.

use crate::MAX_MESSAGE_BYTES;
use std::io::{self, Read, Write};

#[derive(Debug)]
pub enum FrameError {
    /// Bildirilen gövde boyutu sınırı aşıyor; gövde okunmadı.
    TooLarge(usize),
    /// Karşı taraf, çerçeve sınırında bağlantıyı kapattı.
    Closed,
    /// Hiç bayt gelmeden okuma zaman aşımına uğradı.
    Idle,
    /// Çerçeve yarıda kaldı (kapanma veya zaman aşımı).
    Truncated,
    Io(io::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::TooLarge(n) => write!(f, "frame too large ({n} bytes)"),
            FrameError::Closed => write!(f, "peer closed connection"),
            FrameError::Idle => write!(f, "read timed out (idle)"),
            FrameError::Truncated => write!(f, "frame truncated"),
            FrameError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

fn is_timeout(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

/// `buf`'ı doldurur. `Ok(false)`: hiç bayt okunmadan temiz EOF.
fn fill<R: Read>(r: &mut R, buf: &mut [u8], at_frame_start: bool) -> Result<bool, FrameError> {
    let mut read = 0;
    while read < buf.len() {
        match r.read(&mut buf[read..]) {
            Ok(0) if read == 0 && at_frame_start => return Ok(false),
            Ok(0) => return Err(FrameError::Truncated),
            Ok(n) => read += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) if is_timeout(&e) && read == 0 && at_frame_start => {
                return Err(FrameError::Idle)
            }
            Err(e) if is_timeout(&e) => return Err(FrameError::Truncated),
            Err(e) => return Err(FrameError::Io(e)),
        }
    }
    Ok(true)
}

pub fn read_frame<R: Read>(r: &mut R, max_bytes: usize) -> Result<Vec<u8>, FrameError> {
    let mut header = [0u8; 4];
    if !fill(r, &mut header, true)? {
        return Err(FrameError::Closed);
    }
    let len = u32::from_le_bytes(header) as usize;
    if len > max_bytes {
        return Err(FrameError::TooLarge(len));
    }
    let mut body = vec![0u8; len];
    fill(r, &mut body, false)?;
    Ok(body)
}

pub fn write_frame<W: Write>(w: &mut W, body: &[u8]) -> Result<(), FrameError> {
    if body.len() > MAX_MESSAGE_BYTES {
        return Err(FrameError::TooLarge(body.len()));
    }
    let mut buf = Vec::with_capacity(4 + body.len());
    buf.extend_from_slice(&(body.len() as u32).to_le_bytes());
    buf.extend_from_slice(body);
    w.write_all(&buf).map_err(FrameError::Io)?;
    w.flush().map_err(FrameError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"{\"a\":1}").unwrap();
        let got = read_frame(&mut Cursor::new(buf), MAX_MESSAGE_BYTES).unwrap();
        assert_eq!(got, b"{\"a\":1}");
    }

    #[test]
    fn clean_eof_is_closed() {
        assert!(matches!(
            read_frame(&mut Cursor::new(Vec::<u8>::new()), MAX_MESSAGE_BYTES),
            Err(FrameError::Closed)
        ));
    }

    #[test]
    fn partial_header_or_body_is_truncated() {
        assert!(matches!(
            read_frame(&mut Cursor::new(vec![5u8, 0]), MAX_MESSAGE_BYTES),
            Err(FrameError::Truncated)
        ));
        let mut buf = 10u32.to_le_bytes().to_vec();
        buf.extend_from_slice(b"abc");
        assert!(matches!(
            read_frame(&mut Cursor::new(buf), MAX_MESSAGE_BYTES),
            Err(FrameError::Truncated)
        ));
    }

    #[test]
    fn oversized_header_is_rejected_without_reading_body() {
        let buf = (1_000_000u32).to_le_bytes().to_vec();
        assert!(matches!(
            read_frame(&mut Cursor::new(buf), MAX_MESSAGE_BYTES),
            Err(FrameError::TooLarge(1_000_000))
        ));
    }

    #[test]
    fn writer_refuses_oversized_body() {
        let mut out = Vec::new();
        let big = vec![b'x'; MAX_MESSAGE_BYTES + 1];
        assert!(matches!(
            write_frame(&mut out, &big),
            Err(FrameError::TooLarge(_))
        ));
        assert!(out.is_empty());
    }
}
