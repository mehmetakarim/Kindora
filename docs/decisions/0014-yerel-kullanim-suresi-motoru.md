# ADR-0014 — Yerel kullanım süresi motoru: ölçüm agent'ta, sinyal helper'dan, kayıt SQLite'ta

- **Tarih:** 2026-10-29
- **Durum:** Önerildi (P1.C'de uygulandı; Lenovo temel kullanım süresi kabulü PASS, 2026-10-30). "Yalnız masaüstü açıkken ölçüm" sınırlaması [ADR-0015](0015-bagimsiz-session-helper-ve-oturum-acilisi.md) ile kaldırıldı; ölçüm modeli değişmedi.
- **Karar türü:** Teknik uygulama yöntemi
- **İlgili belgeler:** [P1.C raporu](../screen-time/p1c-screen-time.md), [ADR-0002](0002-supabase-sqlite-gorev-ayrimi.md), [ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [ADR-0012](0012-mevcut-windows-hesabinda-calisma.md), [ADR-0013](0013-masaustu-kabugu-helper-koprusu.md), [T0-D](../feasibility/T0-D-offline-policy.md)

## Bağlam ve problem

P1.C'de Kindora gerçek kullanım süresini ölçmeli, yerel kuralları uygulamalı ve mola hatırlatmalıdır. Kilit veya engelleme yoktur. Sınırlar:

- Agent `LocalService` olarak Session 0'da çalışır. Kullanıcının klavye/fare etkinliği (`GetLastInputInfo`) yalnız kullanıcının oturumundan okunabilir.
- Gereksiz kullanıcı verisi toplanmamalıdır (tuş içeriği, koordinat, ekran, pencere, geçmiş yok).
- Uyku, kilit, servis yeniden başlatması, saat değişikliği ve gün dönümü süreleri bozmamalı; belirsiz süre aktif sayılmamalıdır.
- Kurallar agent'ta doğrulanmalı; çocuk arayüzü değiştirememelidir. Ebeveyn kimlik doğrulaması henüz yoktur.

## Değerlendirilen seçenekler

1. **Ölçüm masaüstü uygulamasında.** Reddedildi: arayüz kapatılınca ölçüm biter ve kural mantığı kullanıcı sürecine taşınır (AGENTS.md §5.1).
2. **Agent `WTSQuerySessionInformation` `LastInputTime` ile ölçer.** Reddedildi: alan konsol oturumlarında güvenilir değildir [Çıkarım; PoC'de doğrulanmadı].
3. **Seçilen:** Helper kendi oturumunda yalnız "son girdiden bu yana geçen süre"yi ölçer ve `REPORT_ACTIVITY` ile bildirir. Agent kilit/oturum durumunu WTS'den, uykuyu servis güç olaylarından alır; sınıflandırma, kurallar ve kayıt agent'tadır.

## Seçilen yaklaşım

- **Yeni crate `screen-time`** (platformdan bağımsız, benzetilmiş saatle test edilir): `accounting`, `day`, `breaks`, `policy`, `engine`, `store`.
- **Ölçüm modeli ("girdi sınırı"):**
  - Süreler yalnız monoton saatle ölçülür; duvar saati yalnız hangi güne yazılacağını belirler.
  - İki gözlenen girdi arası, boşta kalma eşiğinden kısaysa `ACTIVE`, değilse `IDLE` sayılır. Eşik dolunca son girdiden itibaren geriye dönük `IDLE` yazılır.
  - Kilit açılışı, uyanma veya başlangıçtan ilk girdiye kadarki süre ve doğrulanmamış kuyruk `IDLE` sayılır.
  - Kilit → `SESSION_LOCKED`; güç olayı → `SUSPENDED`; helper raporu yok/eski, API hatası veya kilit durumu okunamazsa → `UNKNOWN`. Bu süreler aktif sayılmaz.
  - İki ölçüm arası 5 sn'yi aşarsa (uyku, duraklama) aralık `SUSPENDED` veya `UNKNOWN` olur.
  - Yalnız **etkin konsol oturumundan** gelen raporlar sayılır.
- **Gün ve saat:**
  - Gün ileri giderse yeni gün başlar; saat değişikliğiyle olduysa gün `clock_adjusted` işaretlenir.
  - En fazla 1 gün geri gidişte mevcut gün korunur (kullanım geçmiş güne saklanamaz). Daha büyük geri alma bir düzeltme sayılır.
  - Saat atlaması ve saat dilimi değişikliği yaşam döngüsü olayı olarak kaydedilir.
- **Kurallar:** Doğrulama ve sürümleme agent'tadır. IPC'de kural yazma komutu yoktur.
  - Giriş yalnız agent klasöründeki DEV test dosyasıdır (`kindora-dev-policy.json`, `kind` + `dev_only: true` zorunlu, bilinmeyen alan reddedilir).
  - Bozuk dosya → son geçerli politika. Düşük veya eşit sürüm farklı içerik → reddedilir.
  - Dosya yoksa test varsayılanları kullanılır. Varsayılanlar ebeveyn tercihi olarak kaydedilmez.
  - **Üretimde** kural değişikliği ebeveyn kimliğiyle yetkilendirilmiş sunucu kararından gelmelidir (ADR-0002, ADR-0009); DEV dosyası bunun yerine geçmez.
- **Mola:** Molaya kalan süre yalnız aktif kullanımla azalır. Uyarılar ilk/son uyarı sürelerinde verilir. Mola zamanında geri sayım başlar. Mola süresi kadar kesintisiz boşta/kilit/uyku doğal mola sayılır. Hiçbir aşama kilit veya engelleme uygulamaz.
- **Bildirimler:** Agent her olay için kararlı kimlik üretir (`b{seq}-w1`, `-w2`, `-due`, `-done`, `{gün}-limit`). Masaüstü aynı kimliği bir kez gösterir; kapatılan kimlik tekrar gösterilmez.
- **SQLite (`rusqlite` 0.41.1, gömülü SQLite):**
  - Yol: servis hesabının `LocalAppData\Kindora\kindora.db` dosyası. LocalService için `C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora`.
  - Klasör korumalı DACL ile oluşturulur: SYSTEM, Administrators, sahip.
  - `PRAGMA user_version` migration, WAL, tek transaction'lı kayıt (15 sn'de bir veya olayda).
  - Bozuk dosya kenara alınır (`kindora.db.corrupt-<ms>`, en çok 3 kopya) ve yeniden oluşturulur. Daha yeni şema bulunursa dosyaya dokunulmaz, bellek içi çalışılır.
  - Saklama: parçalar 14 gün, günlük özetler 90 gün, olaylar son 5000.

## Sonuçlar ve ödünleşimler

- **Artılar:** Ölçüm arayüzden bağımsızdır. Toplanan veri en aza indirilmiştir. Belirsiz süre hiçbir koşulda aktif sayılmaz. Kayıtlar yeniden başlatmada korunur ve iki kez sayılmaz.
- **Eksiler / riskler:**
  - Bu sürümde helper yalnız masaüstü uygulaması (veya `kindora-helper run`) açıkken çalışır. Kapalıyken süre `UNKNOWN` kaydedilir; aktif kullanım **eksik** sayılabilir. Helper'ın oturum açılışında başlatılması P1.D'nin işidir (ADR-0006).
  - Aynı kullanıcı helper'ı sonlandırarak ölçümü durdurabilir; süre `UNKNOWN` olarak görünür (tespit edilir, engellenmez; ADR-0012).
  - Saatin ileri alınması günlük sayacı yeni güne taşır. Standart kullanıcı saati değiştiremez, saat dilimini değiştirebilir; her ikisi de kaydedilir. Güvenilir zaman kaynağı (sunucu) P1.D+ işidir.
  - Kayıt aralığı nedeniyle beklenmedik sonlanmada son ≤15 sn kaybolabilir (eksik sayım yönünde).
  - Bozuk veritabanı kurtarmasında o günün sayacı sıfırdan başlar; olay kaydedilir.

## Bilinmeyenler / doğrulama gereken noktalar

- Gerçek Windows 11 cihazında: `GetLastInputInfo`, WTS kilit durumu, güç olayları, uyku/uyanma (Lenovo C-01…C-11).
- Monoton saatin (QPC) uyku sırasında ilerleyip ilerlemediği; model her iki durumda da uykuyu aktif saymaz.
- Hızlı kullanıcı değiştirme ve uzak masaüstü oturumları.

## İlişkili kararlar

[ADR-0002](0002-supabase-sqlite-gorev-ayrimi.md), [ADR-0006](0006-agent-surec-ve-hesap-modeli.md), [ADR-0007](0007-servis-helper-ipc-named-pipe.md), [ADR-0012](0012-mevcut-windows-hesabinda-calisma.md), [ADR-0013](0013-masaustu-kabugu-helper-koprusu.md).
