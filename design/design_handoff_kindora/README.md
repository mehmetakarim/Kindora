# Handoff: Kindora — Dijital Ebeveyn Asistanı

## Overview
Kindora, 8–13 yaş arası çocukların Windows bilgisayar kullanımını **yasaklayıcı değil destekleyici** biçimde yöneten bir ebeveyn asistanıdır. Ürün üç yüzeyden oluşur:

1. **Ebeveyn masaüstü paneli** — Tauri 2 + Vue 3 + TypeScript + Bootstrap (1280×800 ve üzeri, 1366×768'de tam kullanılabilir)
2. **Çocuk uygulaması + masaüstü widget'ı** — Windows, çerçevesiz / always-on-top widget, maskot "Filiz"
3. **Ebeveyn mobil uygulaması** — *sonraki sürüm adayı, MVP dışı.* Tasarım dosyaları korunuyor; Flutter bir öneridir, onaylanmış teknoloji kararı değildir.

> **Durum:** Tasarım kabulü kullanıcı onayı bekliyor. Bu paket henüz geliştirmeye aktarılmadı. Güncel durum, kabul matrisi ve açık kararlar: `Kindora Acceptance Report.dc.html`.

**İlk MVP:** yalnızca Windows masaüstü (1 ve 2).

Ürün ilkesi: *"Kontrol etmek için değil, güvenle büyümelerine yardımcı olmak için."*

## About the Design Files
Bu paketteki `.dc.html` dosyaları **HTML ile hazırlanmış tasarım referanslarıdır** — amaçlanan görünümü ve davranışı gösteren prototiplerdir, doğrudan kopyalanacak üretim kodu değildir. Görev, bu tasarımları hedef ortamlarda (Vue 3 + Bootstrap, Flutter) o ortamın kalıplarıyla **yeniden oluşturmaktır**. Dosyalar bir tarayıcıda doğrudan açılır (`support.js` aynı klasörde olmalı). Prototiplerdeki tüm backend davranışları (cihaz yanıtı, ebeveyn onayı) zamanlayıcıyla **simüle edilmiştir** ve ekranda "Demo" olarak işaretlidir.

## Fidelity
**High-fidelity.** Renkler, tipografi, boşluklar, köşe yarıçapları, durumlar ve metinler tasarım olarak tamamdır ve kullanıcı onayı bekler. **Maskot Filiz konsepttir, nihai değildir**; ayrı kabul kapısı `Kindora Mascot Review.dc.html`, son illüstrasyon için `mascot_brief/`. Windows davranışları (oturum kilidi, alan adı engelleme, always-on-top widget) görsel olarak hazırdır, teknik olarak doğrulanmamıştır.

## Değişmez ürün kuralları (her yüzeyde)
- **Belirsiz durum başarı değildir.** Her komut 7 durumdan birindedir: Hazır · Gönderiliyor · Sırada (cihaz çevrimdışı) · Cihaz yanıtı bekleniyor · Uygulandı · Başarısız (cihaz hata bildirdi, kırmızı) · Doğrulanamadı (zaman aşımı, amber). **Zaman aşımı başarısızlık sayılmaz.**
- **Tekrar güvenliği.** Her komutun kimliği vardır (`K-1042`). İşlenirken tekrar tıklama yeni komut üretmez; Doğrulanamadı'da düğme durumu sorgular (yeni komut yok); Başarısız'da yeniden gönderme aynı kimlikle yapılır ve cihaz komutu bir kez uygular.
- **Windows oturumu ≠ Kindora kısıtlaması.** Kilit, 1 dk kaydetme süresinden sonra Windows oturumunu kilitler ve Kindora kısıtlamasını başlatır. "Kısıtlamayı kaldır" Windows oturumunu açmaz; çocuk kendi Windows şifresi/PIN'iyle girer. (Teknik doğrulama ve kullanıcı onayı bekliyor.)
- **Onaysız ek süre limiti uzatmaz.** Çocuk istek gönderdiğinde süre değişmez; mola zamanı gelirse mola başlar.
- **Ani kilit yok.** Kilitten önce çocuğa 1 dk kaydetme süresi; molaya 5 ve 1 dk kala uyarı. Bu ayarlar kapatılamaz.
- **Çocuk tarafında danger (mercan) rengi kullanılmaz.** Ret mesajları yargısız, ebeveyn notu ile.
- **Gözetim yok.** Ekran görüntüsü, tuş kaydı, mesaj içeriği yok. Raporlar süre + uygulama kategorisi + alan adı kuralı.
- **Web kuralları yalnızca alan adı düzeyinde.** Tam URL girilirse alan adına indirgenir ve uyarı gösterilir. Kategori / URL / video düzeyi "Planlanıyor".
- **Durum asla yalnızca renkle anlatılmaz:** ikon + kelime + renk.
- **Hata bildirimleri kapatılana kadar kalır**; bilgi/başarı 3,6–4 sn.
- Tüm arayüz metinleri Türkçe; bileşen adları İngilizce.

## Screens / Views

### Ebeveyn masaüstü (`Kindora Phase 5 - Management.dc.html` — en güncel, Phase 2'yi kapsar)
Kabuk: `grid-template-columns: 240px 1fr`; sidebar beyaz, `border-right 1px #E4E8F1`, padding `22px 14px`. Topbar 72px, beyaz, alt çizgi. İçerik `padding 24px 28px 40px`, `max-width 1320px`, `min-width 1040px`, kartlar arası `gap 18px`.

| Ekran | Amaç | Önemli bileşenler |
|---|---|---|
| Genel Bakış | Çocuğun durumu ve hızlı işlemler | ChildDeviceCard (avatar 48, durum çipi, ModeSelector 4'lü), UsageCard (32/700 sayı, 8px bar), NextBreakCard, QuickActions 3×2 (min-height 96), PendingRequests, ActivityList, CategoryBar |
| Cihazlar | Eşleştirilmiş PC'ler | DeviceCard (6 bilgi alanı, Test/Yeniden adlandır/Eşleştirmeyi kaldır), boş "Başka bilgisayar ekle" kartı, PairingModal (4 adım: çocuk seç → 6 haneli kod + bekleme → cihaz doğrula + ad → başarı) |
| Ekran Süresi | Limit ve program | Hafta içi/sonu stepper (±15 dk, 30–360), mola sıklığı 30/45/60 + süre 5/10/15, **WeeklySchedule** 7×16 saat ızgarası (07–23; fırça: Serbest #2BBFA7 / Ders #5265D8 / Kapalı #C9CFDD), bugün için +15/+30/+60, sticky kaydet çubuğu |
| Web Kuralları | Alan adı kuralları | Bilgi bandı (alan adı düzeyi), sekmeler Engellenenler/İzinliler/Ders modu/Geçici, tablo (alan adı, ne zaman, ekleyen, düzenle/sil), varsayılan davranış seçimi, Gelecek özellikler kartı, **RuleEditorModal** |
| İzin Talepleri | Ek süre / site isteklerine yanıt | RequestCard (neden, süre seçici, hazır not çipleri + serbest not, Onayla/Reddet), Geçmiş listesi |
| Raporlar | Haftalık özet | Hafta seçici, 4 KPI, yığılmış günlük çubuk + kesik limit çizgisi, kategoriler, önceki haftayla tablo, kural değişiklikleri |
| Etkinlik Geçmişi | Olay günlüğü | Filtre çipleri, gün gruplu zaman çizelgesi (hata/bekleme olayları dahil) |
| Ayarlar | 6 bölüm | Genel kurallar, Bildirimler, Görünüm ve dil (Light varsayılan), Hesap, Çocuk profilleri → **Çocuk profili** detay, Güvenlik (ebeveyn PIN'i, kapatılamaz) |

Dark varyant: `Kindora Parent Panel - Dark.dc.html` (aynı ekranlar, dark token'lar).

### Ebeveyn onboarding (`Kindora Onboarding.dc.html` → Ebeveyn)
1080 genişlik kart, sol 340px indigo panel (adım listesi) + sağ form. Adımlar: Hoş geldiniz (3 ilke) → Hesap (ad, e-posta doğrulama, şifre gücü 4 bar, KVKK onayı zorunlu) → Çocuk profili (ad, yaş 6–15 çipleri, renk) → Başlangıç planı (Dengeli [önerilen] / Okul dönemi / Kendim) → Cihaz eşleştirme (kod 482 913, "Sonra eşleştir") → Hazır (özet + çocukla konuşma önerileri).

### Çocuk uygulaması (`Kindora Phase 3 - Child Experience.dc.html`)
Arka plan `#F6F5FE`, kartlar beyaz `radius 28px`, başlıklar Fredoka 600. Üst sekmeler (pill): Ana Sayfa / Hedefler / Rozetler / Filiz.
- **Ana Sayfa:** Filiz + konuşma balonu (tıklanınca ipucu döngüsü), kalan süre halkası (200px, r=84, stroke 16; <%15 amber), sonraki mola, günlük hedefler, son rozet, "Kullanım planım" (kilitli, salt okunur), bugünün akışı.
- **Ek süre / Site iste:** neden **zorunlu** (≥5 karakter), 10/15/30 dk; bekleme → onay/ret (ebeveyn notu).
- **Rozetler:** 5 rozet; kazanılmamış = kesik halka + ilerleme. Daha uzun kullanım rozet kazandırmaz; seri/kaybetme mekaniği yok.
- **Filiz:** ad (varsayılan Filiz, max 14), 4 renk, konuşma Sessiz/Normal/Konuşkan, hareketi azalt.

### Çocuk onboarding (`Kindora Onboarding.dc.html` → Çocuk)
0) Ebeveyn kod girer (hata durumu) → 1) Filiz karşılama → 2) 3 yardım + "Seni izlemem" → 3) Kullanım planı (salt okunur) → 4) Filiz'e ad/renk (veya "Filiz kalsın") → 5) Widget köşesi → 6) "Harika Başlangıç" rozeti.

### Masaüstü widget + mola (`Kindora Phase 4 - Desktop Companion.dc.html`)
- **Compact** 224×58 pill · **Expanded** 300 genişlik, radius 22 · **Mascot Only** 92px + süre çipi.
- Sürükle-bırak → en yakın köşeye 16px boşlukla yapışır (alt köşelerde görev çubuğu + 12px). Şeffaf mod: `rgba(255,255,255,.72)` + `blur(14px)`. Tam ekran uygulamada gizlenir, uyarılar üstte bildirim. Gizlenince tepside kalır; kurallar çalışır.
- **Mola akışı:** Normal → 5 dk kala balon → 1 dk kala ("Kaydettim" / "Ek süre iste", kapanmaz) → Mola zamanı tam ekranı ("Molaya hazırım" / "Ek süre iste") → Mola (10:00 geri sayım, öneri çipleri) → Mola bitti → Yeniden başlatma (süre cihaz kaydından yüklenir).
- Balon 6 sn sonra kapanır; eylemli balonlar yanıt bekler.

### Mobil ebeveyn — sonraki sürüm adayı, MVP dışı (`Kindora Mobile Parent - Prototype.dc.html`, `... - Flutter.dc.html`)
412×892 referans. NavigationBar 80dp, 5 sekme: Ana sayfa / Talepler (rozet) / Süre / Siteler / Daha fazla. Kritik eylemler `showModalBottomSheet` (radius 28, drag handle 32×4). Bildirimden onay biyometrik doğrulama ister. Tema: Açık / Koyu / Sistem. Tema kodu: `flutter/kindora_theme.dart`.

## Interactions & Behavior
- **Komut yaşam döngüsü** (kilit, mola, mod, ek süre, kural kaydı): `ready → (confirm) → sending → pending → applied | failed | unverified`; çevrimdışıysa `sending → queued → pending`. Cihazdan ACK beklenir; 30 sn içinde gelmezse `unverified` (failed değil). `unverified` → "Durumu kontrol et" aynı kimlikle sorgular. Referans uygulama: Phase 5 kilit komutu (Tweaks › lockResult: Uygulandı / Başarısız / Doğrulanamadı). Mola, ek süre ve mod prototipte henüz kısa yolu gösterir; Vue'da hepsi aynı `useCommandStore` modelini kullanmalı.
- **Kilit:** onay modali (“1 dk sonra kilitle”, danger #C9475A) → Gönderiliyor → Cihaz yanıtı bekleniyor (amber, nabız 1,4 sn) → Kilit uygulandı (teal) → tekrar tıklayınca "Kindora kısıtlaması kaldırılsın mı?" onayı → Kısıtlama kaldırılıyor → Hazır.
- **Dar ekran:** pencere genişliği < 1240 px'de sidebar 72 px ikon rayı, içerik min 900 px (1366×768 ve 1280×800 için %125'te taşma yok; %150 açık karar).
- **Hata ve Doğrulanamadı bildirimleri** kapatılana kadar kalır.
- **Mod değiştirme:** seçilen segment amber kesik çerçeve + kum saati; onayda aktif.
- **Ebeveyn bağlantısı yok:** tüm komutlar kapalı (#F4F6FA zemin, #5D6680 metin, "Bağlantı gelince kullanılabilir"), veriler "10:42 itibarıyla" etiketiyle.
- **RuleEditor doğrulama:** `^[a-z0-9-]+(\.[a-z0-9-]+)+$`; protokol, `www.`, yol/sorgu atılır; mükerrer alan adı engellenir; zaman modu "Belirli saatler" seçiliyse en az bir gün.
- **Animasyon:** kart/balon girişi 200ms `translateY(6–8px)→0` + opacity; bottom sheet 250ms `cubic-bezier(.2,.8,.2,1)`; maskot durumları `mascot_brief/` içinde. `prefers-reduced-motion` ve uygulama içi "Hareketi azalt" tüm maskot hareketini durdurur.
- **Focus:** 3px `#8C80E8` halka, 2px offset. Tüm etkileşimler klavyeyle erişilebilir; modallar odağı tutar, Esc kapatır.

## State Management (öneri)
Masaüstü (Pinia): `useDeviceStore` (status: online|offline|unknown, lastSeen, syncAt), `useCommandStore` (komut kuyruğu: id, type, state, sentAt, ackAt, error), `useRequestStore` (open/history, dur, note), `useScheduleStore` (limits, breakEvery, breakLen, grid[7][16], dirty), `useWebRulesStore` (rules[], defaultBehavior), `useUiStore` (theme, toast). Çocuk tarafı: kalan süre ve plan **cihazdaki SQLite** kaydından okunur; uygulama yeniden başlatılınca sıfırlanmaz. Mobil: aynı alanlar, durum yönetimi tercihe bağlı (Riverpod/Bloc).

## Design Tokens
Tam liste: `tokens/_kindora-tokens.scss` (Bootstrap override'ları + light/dark CSS değişkenleri) ve `flutter/kindora_theme.dart`.

| Rol | Light | Dark |
|---|---|---|
| Primary | #5265D8 (hover #4657C8, active #3F50B8) | aynı; metin olarak #8C9BFF |
| Secondary | #8C80E8 | — |
| Accent / success | #2BBFA7 · tint #E3F7F3 · text #127A6A | text #5FDCC6 |
| Warning | #E7A83E · tint #FDF4E3 · text #8A5A0C | text #F3C77A |
| Danger | #D95B68 · solid #C9475A · tint #FCEBED · text #A8364A | text #F29AA6 |
| Zemin / yüzey | #F7F9FC / #FFFFFF / #F1F3F9 | #111827 / #1A2233 / #232D42 |
| Çizgi | #E4E8F1 · güçlü #D5DAE6 | #2E3850 · #3A4560 |
| Metin | #1A2033 · #4A5370 · yardımcı #5D6680 | #E9ECF5 · #A9B1C7 · #8D96AE |

- **Tipografi:** Onest 400/500/600/700 (tüm arayüz). Fredoka 500/600 yalnızca çocuk başlıkları, balonlar, sayaçlar. Ölçek: 12 · 13 · 14 · 15 · 16 · 20 · 26 · 32 · 40. Minimum 12px.
- **Boşluk:** 4px taban (4, 8, 12, 16, 20, 24, 28, 32, 40, 48).
- **Radius:** 8 kontrol · 10 kutu · 12 ebeveyn kartı · 16 modal/mobil kart · 22 widget · 28 çocuk kartı & bottom sheet · 999 pill.
- **Gölge:** kart `0 1px 2px rgba(26,32,51,.04), 0 12px 32px rgba(26,32,51,.06)`; modal `0 24px 64px rgba(17,24,39,.25)`; widget `0 10px 28px rgba(0,0,0,.28)`.

## Assets
- **İkonlar:** Phosphor Icons v2.1 (regular + fill) — Vue: `@phosphor-icons/vue`; Flutter: `phosphor_flutter` (temadaki `Icons.*` yer tutucudur, Phosphor eşdeğerleriyle değiştirin).
- **Fontlar:** Onest, Fredoka (Google Fonts, OFL).
- **Logo:** filiz işaretli çakıl (SVG, prototiplerde `logo()` fonksiyonu).
- **Maskot:** konsept SVG `mascot()` fonksiyonu (Phase 1/3/4). Son çizim illüstratörden — `mascot_brief/`.
- Görsel/fotoğraf yok.

## Files
`design_files/` içinde:
- `Kindora Phase 1 - Visual Foundation.dc.html` — marka, palet, tipografi, maskot konsepti
- `Kindora Phase 2 - Parent Experience.dc.html` — *arşiv*, Phase 5 geçersiz kılar
- `Kindora Phase 3 - Child Experience.dc.html` — çocuk uygulaması
- `Kindora Phase 4 - Desktop Companion.dc.html` — widget, mola akışı, animasyon tablosu
- `Kindora Phase 5 - Management.dc.html` — **ebeveyn panelinin tamamı (kanonik)**
- `Kindora Parent Panel - Dark.dc.html` — dark tema
- `Kindora Onboarding.dc.html` — ebeveyn + çocuk onboarding
- `Kindora Phase 6 - Design Review.dc.html` — *arşiv* (17 Ekim bulguları, bileşen durumları)
- `Kindora Acceptance Report.dc.html` — **kabul raporu, ekran matrisi, açık kararlar (güncel)**
- `Kindora Mascot Review.dc.html` — maskot kabul kapısı (konsept, 6 inceleme)
- `Kindora Mobile Parent - Prototype.dc.html` — etkileşimli mobil (light/dark) · *MVP dışı*
- `Kindora Mobile Parent - Flutter.dc.html` — mobil ekran panosu · *MVP dışı*
- `Kindora Presentation.dc.html` — 12 slaytlık sunum (canlı prototip slaytları, konuşmacı notları)
- `support.js`, `android-frame.jsx`, `deck-stage.js` — prototip çalışma dosyaları

`mascot_brief/` içinde:
- `Kindora Mascot Brief.dc.html` — illüstratör brifi (A4, PDF'e yazdırılabilir): karakter, silüet, renk, özgünlük kuralları, 10 durum, boyutlar, animasyon, teslimatlar, kabul ölçütleri

Diğer: `components.md` (bileşen envanteri ve durumları), `tokens/_kindora-tokens.scss`, `flutter/kindora_theme.dart`, `mascot_brief/`.
