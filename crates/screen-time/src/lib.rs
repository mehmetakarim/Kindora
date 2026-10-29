//! Kindora yerel kullanım süresi motoru (P1.C, ADR-0014).
//!
//! İşletim sisteminden bağımsızdır; Windows sinyalleri (oturum kilidi, güç
//! olayları, helper raporları) `windows-agent` içinde toplanıp [`engine`]'e
//! [`engine::TickInput`] olarak verilir. Bu sayede ölçüm kuralları macOS'ta
//! benzetilmiş saatle test edilebilir — **bu testler Windows kabulü değildir**.
//!
//! - [`calendar`]: yerel takvim günü.
//! - [`policy`]: yerel ebeveyn kuralları, doğrulama ve DEV test dosyası.
//! - [`accounting`]: monoton saatle aktif / boşta / kilitli / uyku / bilinmeyen ayrımı.
//! - [`day`]: gün değişimi ve saat/saat dilimi değişikliği tespiti.
//! - [`breaks`]: mola hatırlatma durumu.
//! - [`engine`]: hepsini birleştiren motor ve `GET_SCREEN_TIME` durumu.
//! - [`store`]: SQLite kalıcılığı, migration ve bozuk veritabanı kurtarma.
//!
//! Toplanmayan veriler: tuş içeriği, fare koordinatı, ekran görüntüsü, pencere
//! veya uygulama adı, belge içeriği, tarayıcı geçmişi, mesaj, kamera/mikrofon.
//! Tek girdi sinyali "son girdiden bu yana geçen süre"dir.

pub mod accounting;
pub mod breaks;
pub mod calendar;
pub mod day;
pub mod engine;
pub mod policy;
pub mod store;
