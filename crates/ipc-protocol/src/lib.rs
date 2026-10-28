//! Kindora yerel IPC sözleşmesi (P1.A).
//!
//! - [`message`]: istek/yanıt modelleri ve doğrulama.
//! - [`frame`]: 4 bayt uzunluk önekli çerçeveleme ve boyut sınırı.
//! - [`transport`]: işletim sistemine özgü yerel taşıma (Windows named pipe,
//!   yalnız geliştirme amaçlı Unix domain socket) ve işletim sisteminden alınan
//!   eş (peer) kimliği.
//! - [`bridge`]: masaüstü uygulaması ile Session Helper arasındaki stdio
//!   köprüsünün sözleşmesi (P1.B). Agent protokolüne yeni komut eklemez.
//!
//! P1.A kapsamındaki komutlar yalnızca `PING`, `GET_STATUS`, `HEALTH_CHECK`'tir.
//! Kilit, süreç sonlandırma veya sistem yetkisi gerektiren komut yoktur.

pub mod bridge;
pub mod frame;
pub mod message;
pub mod transport;

pub use message::*;
