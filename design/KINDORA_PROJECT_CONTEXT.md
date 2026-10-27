# KINDORA — Ürün Vizyonu, Gereksinimler ve Teknik Mimari Bağlamı

> **Belge türü:** Kalıcı ürün ve mimari bağlam referansı  
> **Sürüm:** 1.0 — İlk geliştirme öncesi  
> **Belge tarihi:** 2026-10-27 (Europe/Istanbul)  
> **Önerilen konum:** `design/KINDORA_PROJECT_CONTEXT.md`  
> **Durum:** Ürün kararları kayıtlı; üretim kodu, sistem entegrasyonları ve teknik kabul **henüz doğrulanmadı**.  
> **Amaç:** Claude Code, Codex ve diğer geliştirme araçlarında yeni oturum başladığında projenin hedef, kapsam, karar ve sınırlarını korumak.

---

## 1. Bu belge nasıl kullanılmalı?

Bu dosya **ürün niyetinin ve başlangıç teknik yaklaşımının ana kaynağıdır**. Mevcut kodun gerçekten bunları uyguladığının kanıtı değildir. Yeni bir oturumda önce `brain.md` ve proje talimatları okunmalı; ürün kapsamı için bu belgeye, UI ayrıntıları için `design/design_handoff_kindora/` altındaki **gerçekten mevcut** tasarım teslimatına başvurulmalıdır. Kod, test ve Git durumu ayrı doğrulanmalıdır.

Terimler:
- **Onaylandı (ürün kararı):** Kullanıcıyla kararlaştırılan hedef/tercih.
- **Önerilen tasarım:** Teknik doğrulama öncesi plan; PoC sonucuna göre uyarlanabilir.
- **Doğrulanmadı:** Çalışan uygulama veya kabul testiyle henüz kanıtlanmamış.
- **MVP:** İlk aile kullanımında gerçek uçtan uca çalışan minimum ürün; tam ticari sürüm değil.

**Belge yönetimi:** Bu dosya sohbet dökümü değildir. Onaylı ürün gereksinimleri burada; yaşayan durum `brain.md` içinde, görevler `docs/tasks.md` içinde, onaylı mimari değişiklikler `docs/decisions/` altında ADR olarak, tekrarlanabilir teknik sorunlar `docs/solutions.md` içinde tutulur. Bu dosya ile kod veya güncel ADR çelişirse çelişki açıkça raporlanır; kullanıcı onayı olmadan kapsam sessizce değiştirilmez.

## 2. Ürün özeti

**Ürün adı: Kindora.** Kindora, başlangıçta 10 yaşındaki bir çocuğun Windows bilgisayarını güvenli ve dengeli kullanmasını desteklemek için tasarlanan, ileride başka ailelere de sunulması hedeflenen **dijital ebeveyn asistanı**dır.

Temel yaklaşım: **Çocuğu sürekli gözetlemek veya cezalandırmak değil; dijital becerileri, yapay zekâ okuryazarlığını, ders–oyun–dinlenme dengesini ve öz düzenlemeyi desteklemek.** Ebeveyn gerektiğinde müdahale edebilir, çocuk kalan süreyi görebilir, mola zamanı önceden bildirilir ve ek süre/erişim izni isteyebilir.

Örnek ürün ilkesi: *“Kontrol etmek için değil, güvenle büyümelerine yardımcı olmak için.”* Bu bir marka sloganı adayıdır; hukuki veya pazarlama yönünden nihai onay değildir.

### 2.1 Kullanıcılar ve cihazlar

1. **Çocuk:** Windows bilgisayarda Kindora çocuk uygulamasını ve isteğe bağlı masaüstü maskot widget'ını kullanır; ebeveyn kurallarını değiştirme yetkisi yoktur.
2. **Anne/baba veya yetkili ebeveyn:** Windows/macOS masaüstü ebeveyn uygulaması veya Android/iOS Flutter mobil uygulamasından cihazı ve kuralları yönetir.
3. **Aile:** Başlangıçta tek aile ile test edilir, ancak hesap–aile–çocuk–cihaz ilişkileri ileride çoklu aile ürününe dönüşebilecek şekilde tasarlanır. Yetkilendirme sınırları aileler arası veri erişimini kesin biçimde ayırmalıdır.

### 2.2 Ana hedefler

- Manuel ve zamanlanmış kullanım kontrolünü birleştirmek (**hibrit sistem**).
- Ebeveyn bilgisayar başında olmasa dahi mobil uygulamayla yönetim.
- Çocuğun ekranında sürekli ancak rahatsız etmeyen kalan süre bilgisi.
- Mola öncesi bilgilendirme; süre sonunda anlaşılır Kindora kısıtlama deneyimi.
- Chrome, Edge vb. tarayıcılardan bağımsız **alan adı düzeyinde** web erişim kuralı hedefi.
- Ebeveyn onaylı süre ve site erişim istisnaları.
- İnternet yokken önceden alınmış kuralları yerelde uygulama.
- Hassas çocuk verilerini en aza indiren kullanım raporları.
- Sıfıra yakın başlangıç altyapı maliyeti; erken aşamada ayrı PHP/VPS zorunluluğu yok.
- Oyunlaştırma ile dengeli kullanımı teşvik; daha uzun ekran süresini ödüllendirmeme.

### 2.3 Bilerek kapsam dışı bırakılanlar

- Gizli izleme, tuş kaydı, mesaj okuma, mikrofon/kamera dinleme, sürekli ekran görüntüsü.
- Windows parolasını depolama, Windows kimlik doğrulamasını atlatma veya uzak parola girişi otomasyonu.
- İlk MVP'de gerçek generatif AI sohbet asistanı.
- Çocuğun tüm HTTPS URL'lerini, videoları veya YouTube Shorts gibi alt içerikleri her tarayıcıda kesin filtreleme iddiası.
- İlk sürümde ödeme, abonelik ekranı, lisans sunucusu ve ticari pazaryeri entegrasyonları.

## 3. Kesinleşen ürün/teknoloji tercihleri

| Bileşen | Teknoloji / yaklaşım | Karar niteliği |
|---|---|---|
| Çocuk Windows uygulaması | **Tauri 2 + Vue 3 + TypeScript + Bootstrap 6 hedefi** | Onaylı tercih; Bootstrap 6 sürüm uyumluluğu kontrol edilecek |
| Ebeveyn masaüstü uygulaması | **Tauri 2 + Vue 3 + TypeScript + Bootstrap 6 hedefi** | Onaylı tercih |
| Ebeveyn mobil uygulaması | **Flutter (Android/iOS)** | Onaylı, **MVP kapsamına dahil** |
| Windows arka plan denetimi | **Rust tabanlı Windows servisi ve oturum yardımcısı** | Onaylı yön; çalışma modeli PoC ile doğrulanacak |
| Bulut veritabanı, kimlik, olaylar | **Supabase** | Onaylı tercih |
| Çevrimdışı kurallar ve yerel olaylar | **SQLite** | Onaylı tercih |
| Çocuk masaüstü asistanı | **Filiz** özgün maskot konsepti | Konsept yönü onaylı; nihai görsel/animasyon teknik kabulü ayrı |
| Ana kullanım kısıtlaması | **Kindora kullanım kilidi** | Onaylı ürün yaklaşımı; zorlayıcı uygulama yöntemi doğrulanacak |
| Ek kontrol | Windows'un yerleşik oturum kilidi | İkincil işlev; Kindora kısıtlamasından farklı |
| Web filtreleme | Tarayıcıdan bağımsız alan adı kuralları | Hedef onaylı; sağlamlık ve bypass direnci doğrulanacak |
| UI metinleri / belgeler | Türkçe; kod sembolleri İngilizce olabilir | Onaylı |
| Stil kütüphanesi | Bootstrap, **Tailwind kullanılmaz** | Onaylı |

**Önemli:** Teknoloji seçimi, çalışan entegrasyon veya platform uyumluluk testi tamamlandı demek değildir. Bootstrap 6 alfa/kararlı sürüm durumu geliştirmeye başlanırken resmi kaynaktan yeniden doğrulanmalı; gerekiyorsa uyumluluk yaklaşımı bir ADR ile kararlaştırılmalıdır.

## 4. Sistem bütününde mimari

```text
                   EBEVEYN ALANLARI
         ┌─────────────────────────────────┐
         │ Tauri + Vue masaüstü ebeveyn    │
         │ Flutter Android/iOS ebeveyn     │
         └───────────────┬─────────────────┘
                         │ TLS / Auth
                         ▼
               ┌─────────────────────┐
               │ SUPABASE            │
               │ Auth + Postgres RLS │
               │ Edge Functions      │
               │ Realtime / olaylar  │
               └─────────┬───────────┘
                         │ yetkilendirilmiş komutlar
                         ▼
          ┌────────────────────────────────┐
          │ ÇOCUK WINDOWS BİLGİSAYARI      │
          │ Tauri + Vue UI                 │
          │ Filiz widget + mola ekranı     │
          │ Oturum yardımcısı (user mode)  │
          │ Rust Windows Service           │
          │ Yerel policy engine + SQLite   │
          │ Web filtreleme adaptörü        │
          └────────────────────────────────┘
```

**Prensipler:** Ebeveyn arayüzleri yönetim istemcileridir; Windows kısıtlamaları **çocuğun cihazında** uygulanır. İnternet kesilse de mevcut kurallar sürer. Bulut, nihai yerel cihaz eylemi yerine geçmez. Realtime bildirimi bir **uyandırma/yenileme sinyali** olabilir; komut gerçeği ve kesin durum için dayanıklı kayıt/ack protokolü gerekir. Aile sınırları RLS ve sunucu tarafı yetki kontrolleriyle korunur.

## 5. Çocuk Windows uygulaması — Tauri + Vue + Bootstrap 6

### 5.1 Görev ve ekranlar

Çocuğun gördüğü arayüz: ilk kurulum ve ebeveynle eşleştirme, güncel kalan süre, günlük limit, sıradaki mola, ders/oyun/serbest mod, erişim isteği, ek süre isteği, yanıt bekleme, ret/onay durumları, başarı rozetleri, yardım ve erişilebilirlik ayarları. Çocuk ebeveynin tanımladığı politika değerlerini değiştiremez.

Önerilen teknik UI bileşenleri: Vue 3 Composition API + TypeScript, Pinia state, Vue Router; Bootstrap tabanlı ortak bileşen kütüphanesi; Tauri WebView2 (Windows) kabuğu. UI ile Rust köprüsü en dar yetki setiyle sınırlandırılmalı; her frontend komutu otomatik işletim sistemi ayrıcalığına dönüşmemelidir.

### 5.2 Masaüstü Filiz widget'ı

Kindora'nın Clippy'nin **etkileşim fikrinden** ilham alan ancak Clippy'yi kopyalamayan özgün karakteri **Filiz** kullanılır. Görsel hedef: dostça, dikkat dağıtmayan, 8–13 yaşa uygun, yaş aldıkça da çocukça gelmeyecek kadar sade.

Widget biçimleri:
- **Compact:** Maskot simgesi + kalan süre + durum rengi.
- **Expanded:** Kalan süre, sonraki mola, aktif mod, ek süre talebi, ana uygulamaya geçiş.
- **Mascot Only:** Çok küçük karakter, konuşma balonu gerektiğinde açılır.

Teknik hedefler: çerçevesiz Tauri pencere, always-on-top, sürüklenebilir konum, tercihe göre şeffaflık, çoklu monitör/DPI davranışı, isteğe bağlı tam ekran uygulamalarda gizleme. Always-on-top **güvenlik sınırı değildir**; yalnızca sunum davranışıdır. Kullanıcı widget'ı kapatabilse de politika motoru çalışmaya devam eder.

Konuşma durumları: `idle`, `greeting`, `reminder`, `break`, `happy`, `thinking`, `sleeping`, `permission_pending`, `permission_approved`, `encouragement`. Sık animasyon, sesli uyarı veya ödül baskısı yok. Azaltılmış hareket ve sessiz mod bulunur. İlk MVP'de sabit olay mesajları kullanılır; LLM zorunlu değildir.

### 5.3 Süre ve mola deneyimi

- Süre sayımı cihazda yerel olarak yapılır; widget her saniye Supabase çağırmaz.
- Günlük kullanım limiti, aktif oturum, boşta kalma ve sistem uyku/uyanma olaylarının nasıl hesaplandığı test edilebilir kurallarla tanımlanır.
- Molaya örn. 5 dakika ve 1 dakika kala ön uyarı; çalışmayı kaydetme fırsatı.
- Süre bitince Kindora mola/kısıtlama ekranı görünür; **mola başlat** ve **ek süre iste** seçenekleri sunulur.
- Talep onaylanmamışken limit kendiliğinden uzamaz. Çevrimdışı talep durumu açıkça belirtilir.
- Uzaktan onay geldiyse yeni kural yerel motora işlenir; masaüstü erişimi uygun biçimde yeniden sağlanır.

### 5.4 Kuralların kapsamı

Serbest zaman, ders/odak modu, mola, gecelik program, uygulama kategorileri ve alan adı kuralları. Uygulama kategorilerinin güvenilir eşleştirilmesi MVP sırasında gerçek Windows süreçleri ile doğrulanmalıdır; isimle eşleştirme tek başına güvenlik garantisi değildir.

## 6. Ebeveyn masaüstü uygulaması — Tauri + Vue + Bootstrap 6

### 6.1 Amaç

Çocuğun cihazındaki durumu bir bakışta görmek, hızlı müdahale etmek, plan oluşturmak ve talepleri yönetmek. Daha önce beğenilen **sade sol menü + özet kartlar + hızlı kontrol** tasarımı esastır.

### 6.2 Ekranlar

- Dashboard: cihaz çevrimiçi/çevrimdışı, bugünkü kullanım, limit, sonraki mola, aktif mod, bekleyen talepler.
- Çocuk profili ve cihazlar: eşleştirme, cihaz son görülme, yetkiler, kaldırma.
- Süre yönetimi: gün/gün limit, hafta içi/hafta sonu, ders/uyku programları.
- Web kuralları: engelle/izin listeleri, geçici izinler, etkin tarih/saat aralıkları.
- Talepler: süre veya site izni isteme, neden, onay/ret, süre aşımı, işlem geçmişi.
- Kullanım raporları: günlük/haftalık toplam, eğitim/oyun oranı, mola hedefleri.
- Ayarlar: hesap, aile üyeleri/roller, bildirimler, güvenlik, tema ve dil.

### 6.3 Hızlı komutların durumu

Komutu göndermek, uygulanmasıyla eşdeğer değildir. Arayüz `Hazır`, `Gönderiliyor`, `Sırada`, `Cihaz yanıtı bekleniyor`, `Uygulandı`, `Başarısız`, `Doğrulanamadı` gibi açık durumlar göstermelidir. Timeout “kesin uygulanmadı” anlamına gelmez. Başarı yalnız cihazdan güvenilir durum/ack geldikten sonra gösterilmelidir.

### 6.4 Pencere ve DPI

Masaüstü ebeveyn paneli varsayılan olarak **büyütülmüş / ekranı dolduran pencere** olabilir; çocuğun widget'ı ise küçük ve bağımsızdır. Tam ekran, taşmanın otomatik çözümü değildir. Windows %100/%125/%150 ölçeklendirme; 1280×800 ve 1366×768 gibi görünür alanlar test edilir. Dar genişlikte sidebar ikon moduna düşer, grid tek sütuna iner, dikey kaydırma mümkündür, yatay taşma engellenir. `%150` DPI'da piksel boyutları ile CSS mantıksal piksel alanının farklı olduğu unutulmamalıdır.

## 7. Ebeveyn mobil uygulaması — Flutter

### 7.1 Neden MVP'de?

Anne veya baba bilgisayar başında olmayabilir. Mobil uygulama ilk sürümden itibaren cihaz durumu görme, anlık kısıtlama, ek süre verme ve gelen talepleri yanıtlama aracı olarak kabul edildi.

### 7.2 Kapsam

- Android/iOS oturum açma, hesap/ebeveyn yetkilendirmesi.
- Aile/çocuk seçimi, cihaz çevrimiçi durumu, son görülme zamanı.
- Kullanım süresi, kalan süre, aktif kural özeti.
- Kindora kilidi uygula/kaldır, mola başlat, ek süre ver.
- Ek süre ve web erişim taleplerini onayla/reddet.
- Temel bildirimler ve işlem geçmişi.
- İleride tam program editörü, ayrıntılı analitik, çoklu aile gelişmiş yönetimi eklenebilir.

### 7.3 Ortak backend şartı

Flutter, masaüstü ebeveyn uygulamasından farklı/gevşek yetki kuralları kullanmamalı. Her iki istemci de aynı aile kimlik modeli, sunucu yetkilendirme fonksiyonları, cihaz komut şeması ve durum makinesini kullanır. İstemci tarafındaki kullanıcı rolü göstergesi tek başına yetkilendirme değildir.

### 7.4 Mobil bildirimler

Uygulama kapalıyken talepleri göstermek için Android/iOS push bildirim altyapısı (ör. FCM/APNs köprüsü) gerekebilir; **yalnız Supabase Realtime, arka planda güvenilir push teslimi garantisi değildir**. Push sağlayıcısı, kimlik bilgileri, maliyet/kota, veri minimizasyonu ve mağaza izinleri PoC ile doğrulanır. Push içeriğinde hassas çocuk bilgisi tutulmaz; bildirime tıklayınca gerçek sunucu durumu yenilenir.

### 7.5 Dağıtım

Flutter teknik hedef olarak onaylı olsa da Android/iOS kod imzalama, Apple Developer ve Google Play hesapları, uygulama mağazası incelemeleri veya platform gereklilikleri **henüz doğrulanmadı**. MVP ilk testleri geliştirici cihazlarında yapılabilir. Mağaza maliyeti $0 kabul edilmez.

## 8. Rust Windows servisi ve oturum yardımcısı

### 8.1 İş bölümü

**Rust Windows Service (arka plan):** cihaz açılışında çalışma, yerel kuralları yükleme, durum/komut senkronizasyonu, policy engine, SQLite, ağ filtreleme bileşenini yönetme ve denetim kayıtlarını tutma. Servis, etkileşimli masaüstüyle doğrudan kullanıcı arayüzü göstermemelidir.

**Oturum yardımcısı (user session):** kullanıcının açık Windows oturumunda UI bildirimleri, gerçek oturum kimliğine bağlı işlemler, Kindora ekranıyla güvenli IPC. `LockWorkStation()` gibi etkileşimli oturum gerektiren çağrıların uygun oturumdan yapılması gerekir.

**Tauri UI:** yalnızca sunum, izin isteme, sonuçları gösterme. Kullanıcı UI sürecini sonlandırırsa denetim mekanizması bundan etkilenmemeli; servis iyileştirme veya güvenli yeniden başlatma davranışı test edilmeli.

### 8.2 İlk teknik PoC gereksinimleri

1. Servis + oturum yardımcısı Windows yeniden başlatmada doğru başlıyor mu?
2. Standart Windows çocuk hesabı ile yöneticisiz kullanımda hangi yetkiler gerekir?
3. Tam ekran Kindora kısıtlama ekranı Alt+Tab/Görev Yöneticisi/yeni oturum karşısında neyi yapabilir, neyi **yapamaz**?
4. Kullanım kısıtlaması Windows oturum güvenliğini atlatmadan gerçekten nasıl uygulanır?
5. Bağlantı yokken limit dolması, sistem saati değişimi, uyku/uyanma, yeniden başlatma, oturum kapatma/açma nasıl işlenir?
6. Yönetici tarafından güvenli kaldırma/güncelleme ve kayıp ebeveyn erişimi için kurtarma akışı nasıl çalışır?

Bu maddeler tamamlanmadan **“kapatılamaz/kesin engeller”** iddiası üretilmez. Çocuk standart Windows hesabı, ebeveyn yönetici hesabı varsayımı açıkça doğrulanmalıdır.

## 9. Kindora kullanım kilidi ve Windows kilidi

### 9.1 Ana tercih: Kindora kullanım kilidi

İnternet kafedeki masa açma/kapama mantığından esinlenen kullanıcı deneyimi: Windows oturumu teknik olarak açıkken Kindora mola/kısıtlama ekranını gösterir ve uygun sistem denetimleriyle kullanımı politika gereği sınırlar. Ebeveyn izin verdiğinde Windows parolası gerekmeden **Kindora'nın kendi kısıtlaması** kaldırılır.

Bu yaklaşımın amacı ebeveyn mobil uygulamasıyla kolay yönetim ve çocuk için açıklayıcı mola deneyimidir. **Tam ekran, always-on-top pencere güvenlik mekanizması değildir**. Güvenilir kısıtlama için Windows'un desteklediği gerçek sistem sınırları/standart kullanıcı ilkeleri araştırılmalı ve PoC ile doğrulanmalıdır. Ctrl+Alt+Del gibi güvenli Windows dizilerini engelleme vaadinde bulunulmaz.

### 9.2 İkincil seçenek: Windows oturumunu kilitleme

`LockWorkStation()` Windows oturumunu kilitler. Yeniden açma Windows kimlik doğrulamasına tabidir. Ebeveynin Kindora'da “izin ver” demesi, Windows parolasını atlayarak oturum açılması anlamına gelmez. İki işlev UI ve komut protokolünde ayrı adlandırılır (`APPLY_KINDORA_RESTRICTION`, `RELEASE_KINDORA_RESTRICTION`, `LOCK_WINDOWS_SESSION` gibi).

### 9.3 Güvenlik ve acil durumlar

- Ebeveyn erişimi kaybolduğunda cihazı süresiz ve kurtarılamaz biçimde kilitleme riski önlenir.
- Donanım/Windows onarımını veya acil durumda gerekli erişimi engellemeye çalışma.
- Kurtarma yerel ebeveyn/yönetici kimlik doğrulaması ve denetlenebilir kayıt ile tanımlanır.
- Komutlar rastgele shell komutu çalıştırmaz; doğrulanmış beyaz listedeki işlemlerden oluşur.

## 10. Merkezi backend — Supabase

### 10.1 Tercihin gerekçesi

Ayrı bir PHP/VPS sunucusu kurmadan başlangıç yapabilmek; PostgreSQL ilişkisel model, Auth, Edge Functions ve Realtime imkanlarından yararlanmak. Başlangıç kullanımının ücretsiz kota içinde kalması **hedef**; ücret ve kota değişebileceği için lansman öncesi yeniden kontrol edilir.

### 10.2 Kavramsal veri modeli — taslak (nihai şema değil)

| Varlık | Sorumluluk |
|---|---|
| `families` | Aile/tenant sınırı |
| `family_members` | Ebeveyn/çocuk üyelikleri ve roller |
| `child_profiles` | Asgari çocuk profili bilgisi |
| `devices` | Cihaz kimliği, aile aidiyeti, son görülme |
| `device_credentials` | Cihaz kimliği/rotasyonu; sırlar güvenli biçimde tutulur, açıkça istemciye verilmez |
| `policy_sets` / `policy_versions` | Onaylı kullanım kuralları ve sürümleme |
| `schedule_rules` | Gün/saat/mola/uyku/ders zamanları |
| `web_rules` | Alan adı izin/engelleme kuralları |
| `commands` | Komut kimliği, aktör, hedef cihaz, durum, sona erme |
| `command_receipts` | Cihaz kabul/uygulama sonucu ve hata kanıtı |
| `permission_requests` | Çocuk süre/site izin talepleri |
| `usage_summaries` | Özetlenmiş ekran süresi/uygulama kategorileri |
| `audit_events` | Yetkili işlemlerin güvenli, asgari denetim kaydı |

Tablo ve alan adları birer öneridir. Tam SQL şeması, veri saklama süresi, indeksler, RLS politikaları ve migration stratejisi teknik tasarımda kesinleşir.

### 10.3 Yetki modeli

- Ebeveyn yalnız kendi ailesindeki yetkili cihazları yönetebilir.
- Çocuk, kendi profilinin kısıtlanmış verisini okuyabilir ve talep açabilir; **kendisine izin veremez**, kural/komut yetkisini yükseltemez.
- Cihaz yalnız kendi komutlarını okuyabilir/işleyebilir ve yalnız kendi raporlarını gönderebilir.
- Çoklu aile izolasyonu, her tablonun RLS ve sunucu API testlerinin zorunlu kabul kriteridir.
- Supabase `service_role` veya yönetici sırları masaüstü/mobil uygulamaya gömülmez. İstemci anahtarları tek başına güvenlik sağlamaz.
- Kritik komutlar Edge Functions veya eşdeğer güvenli sunucu sınırından geçer; aktör, aile, cihaz, niyet, yetki ve hız sınırlaması kontrol edilir.

### 10.4 Komut iletimi

**Önerilen akış:** Ebeveyn uygulaması yetkili fonksiyona `command_id` ve komut tipini gönderir → sunucuda dayanıklı komut kaydı oluşturulur → Realtime cihazı uyandırır → cihaz kayıtlı komutu kimlik doğrulayarak alır → aynı `command_id` daha önce uygulandıysa tekrar etmez → sonuç/receipt döner → arayüz son durumu okur.

Durumlar kullanıcıya: Hazır, Gönderiliyor, Sırada, Cihaz yanıtı bekleniyor, Uygulandı, Başarısız, Doğrulanamadı. `Doğrulanamadı`, timeout veya ağ kesintisinde **gerçek uygulama sonucu bilinmiyor** demektir. Tekrar deneme idempotent olmalı; teslimat, uygulama ve onay ayrı izlenir. Komutlar hedef/TTL/policy_version ve gerekli durum önkoşullarını içerir. Komutun teslim edildiği iddiası ağ ACK'i ile, **uygulandığı** iddiası cihazın gerçek uygulama sonucuyla temellendirilir.

### 10.5 Realtime ve push

Realtime, online masaüstü cihaz için uygun bildirim yolu olabilir. Mobil uygulama arka plandayken yeni talebi güvenilir şekilde göstermek için APNs/FCM gibi push teslim mekanizmaları ayrıca ele alınır. Push mesajı komutun asıl veri kaynağı değildir; uygulama açıldığında sunucu yeniden sorgulanır.

### 10.6 Maliyet ve gizlilik

Ücretsiz plan kaynakları (DB, Realtime, Edge Functions, Auth) gerçek kullanımda izlenir. Aktif cihaz sayısı, mesaj/senkronizasyon sıklığı, analitik saklama süresi ve push maliyeti proje ölçeklendikçe yeniden değerlendirilir. Çocuğun hassas içerik verileri gönderilmez; saklama politikası ve veri silme hakkı için ticari aşamada hukuki uygunluk araştırılır.

## 11. Yerel çevrimdışı kurallar — SQLite

### 11.1 Ne tutar?

- Son **geçerli ve imzası/versiyonu doğrulanmış** politika kopyası.
- Uygulama süre sayaçları, gün devri ve zaman dilimi/yerel oturum bilgisi.
- Son işlenen komut kimlikleri ve idempotency kayıtları.
- Sunucuya gönderilmeyi bekleyen asgari kullanım özetleri/audit olayları.
- Cihaz konfigürasyonu, veri şeması sürümü, hata ve kurtarma bilgileri.

### 11.2 İşleyiş

Politika motoru internete bağlı olmadan SQLite'taki en son geçerli kuralları uygular. Yeni ebeveyn kararı çevrimdışıyken cihazda **hemen uygulanmış sayılmaz**. Çocuk isteği, mevcut bağlantı durumuna göre `Gönderilecek`, `Gönderildi`, `Yanıt bekliyor` şeklinde gösterilir; ebeveyn onayı olmadan otomatik kabul edilmez.

### 11.3 Veri tutarlılığı

- Atomik transaction ve migration yönetimi.
- SQLite dosyasına sadece yetkili yerel süreçlerin erişimi (ACL); sırları düz metin tutmama.
- Politika sürümü ve zaman damgası; eski politikanın yeni sürümü yanlışlıkla ezmesini önleme.
- Süre hesaplamada duvar saati + monoton zaman + son güvenilir senkronizasyonun birlikte değerlendirilmesi; yeniden başlatma/uyku gibi durumlar test edilmeli.
- Çevrimdışı üretilen olaylar sıra/kimlik ile biriktirilir, bağlantıda bir kez senkronize edilir; olası tekrarlar sunucu tarafında ayıklanır.
- Bozulmuş DB, disk doluluğu ve yeniden kurulum senaryolarında açık kurtarma politikası belirlenir.

### 11.4 Lokal–bulut otoritesi

**Politika değişikliği yetkisi:** Ebeveynin sunucuda yetkilendirilmiş kararı.  
**Fiili cihaz durumu ve geçerli kullanım sayacı:** Yerel ajan; sunucu sonuç bildirimlerini ve özetleri alır.  
**Çelişki çözümü:** Versiyon ve yetki tabanlı; çocuğun istemci değerleri ebeveyn kuralını geçersiz kılamaz.

## 12. Tarayıcıdan bağımsız web sitesi kontrolü

### 12.1 Ürün gereksinimi

Ebeveyn `youtube.com` vb. alan adlarını, ders saatlerine veya geçici izinlere göre engelleyebilmeli. Chrome, Edge ve diğer tarayıcılarda **aynı yerel ağ kuralının uygulanması** hedeflenir. Geliştirme testlerinde tüm desteklenen tarayıcılar ve ağ koşulları ayrı doğrulanır.

### 12.2 Teknik seçenekler (kesinleşmiş yöntem değil)

1. `hosts` dosyası: hızlı prototip, kolay aşılabilir ve geniş site kümelerinde kırılgan.
2. DNS temelli filtreleme ve bypass denetimi: MVP için **aday yaklaşım**, DoH/DoT, VPN, DNS cache ve yerel yönetici hesabı senaryolarıyla test edilmeli.
3. Windows Filtering Platform (WFP) veya uygun Windows ağ denetimi: daha güçlü ama geliştirme/bakım/test maliyeti yüksek; HTTPS URL içeriği veya YouTube video türünü kendiliğinden ayırt etmez.
4. Tarayıcı eklentileri: belirli URL/video türü istenirse alternatif; saf tarayıcı bağımsız çözüm sayılmaz.

### 12.3 Kabul sınırı

Başlangıç kabulü **alan adı bazında**: HTTP/HTTPS, alt alan adları, çeşitli DNS yolları, Chromium tarayıcılar, yeniden başlatma ve çevrimdışı kural uygulanması. `youtube.com` engeli tek başına tüm YouTube CDN/embedded link kaçaklarını kapatmayabilir; başarılı iddia ancak test kapsamıyla söylenir. HTTPS şifrelemesini kırarak çocuk trafiğini toplama varsayılan yöntem değildir.

## 13. Oyunlaştırma ve Filiz deneyimi

- Rozet örnekleri: Mola Ustası, Dijital Denge, Odak Kahramanı, Meraklı Kaşif.
- Başarı göstergesi daha çok ekran süresi değil; zamanında mola, kendi hedefi, planına uyma gibi olumlu davranışlar.
- Aşırı streak, kayıp korkusu, manipülatif geri sayım ve dikkat bağımlılığı yaratabilecek bildirimler kullanılmaz.
- Mesajlar Türkçe, kısa, destekleyici ve yargılamayan dilde.
- Çocuk maskot görünümünü/uygun animasyonları kişiselleştirebilir ama denetim kurallarını değil.
- Nihai Filiz illüstrasyonu, 24/32/48/96 px okunabilirliği, açık/koyu arka plan ve animasyon performansı ayrı tasarım/teknik kabulü gerektirir.

## 14. Güvenlik, veri koruma ve ürün etiği

- Çocuğun Windows kullanıcısı **standart kullanıcı**, ebeveyn/kurulum hesabı gerektiği ölçüde yönetici olmalı; bu kurulum gereksinimi doğrulanır.
- Cihaz eşleştirme tek kullanımlık/kısa ömürlü onay kodu veya QR + ebeveyn kimlik doğrulaması; yeniden eşleştirme ve cihaz silme güvenli olmalı.
- Her cihaz benzersiz kimliğe, döndürülebilir anahtara ve iptal edilebilir oturuma sahip olmalı.
- IPC kanalları yalnız izinli işlemleri kabul eder; Windows servisinde keyfi komut yürütme bulunmaz.
- Güncelleme ve kaldırma süreçleri imzalama, ayrıcalık sınırları ve kurtarma ihtiyacı açısından tasarlanır.
- Açıkça gösterilen veri toplama: toplam süre, sınırlı uygulama kategorisi, ebeveyn izinleri ve cihaz durumları.
- Gereksiz tam URL, kişisel iletişim ve içerik geçmişi saklanmaz.
- Çocukların kişisel verilerine ilişkin Türkiye KVKK ve ileride hizmet verilecek ülkelerin gereksinimleri ticari yayından önce uzman incelemesine ihtiyaç duyar; bu belge hukuki uygunluk onayı değildir.
- Anne/baba yetki değişikliği, ayrılan aile üyesinin erişiminin kaldırılması, çalıntı cihaz ve hesap ele geçirilmesi senaryoları düşünülür.
- Kayıp bağlantı, bozuk kural veya saat sapması gibi belirsizliklerde cihazı geri döndürülemez şekilde kilitleyen tasarım yapılmaz.

## 15. Önerilen monorepo organizasyonu (henüz oluşturulmuş değil)

```text
kindora/
├── AGENTS.md                  # Ortak AI geliştirme protokolü
├── CLAUDE.md                  # Claude Code giriş talimatları
├── brain.md                   # Güncel oturum devri
├── README.md
├── design/
│   ├── KINDORA_PROJECT_CONTEXT.md  # BU BELGE: ürün bağlamı
│   └── design_handoff_kindora/     # Claude Design teslim dosyaları (mevcutsa)
├── docs/
│   ├── architecture.md       # KODLA DOĞRULANMIŞ teknik durum
│   ├── tasks.md              # Görevler ve kabul kanıtları
│   ├── solutions.md          # Tekrarlanabilir hata/çözüm
│   └── decisions/            # ADR kayıtları
├── apps/
│   ├── child-desktop/        # Tauri + Vue
│   ├── parent-desktop/       # Tauri + Vue
│   └── parent-mobile/        # Flutter
├── crates/
│   ├── windows-agent/
│   ├── windows-session/
│   ├── policy-engine/
│   └── network-filter/
├── packages/
│   ├── ui/
│   └── shared-contracts/
└── supabase/
    ├── migrations/
    └── functions/
```

**Uyarı:** Bu ağaç öneri niteliğindedir. Gerçekte hangi klasörlerin, paketlerin veya komutların mevcut olduğu proje incelenmeden iddia edilmez. Flutter/Dart ile TypeScript arasında tip eşitliği otomatik değildir; ortak API sözleşmesi için OpenAPI/JSON Schema veya eşdeğer üretilebilir sözleşme yaklaşımı değerlendirilebilir.

## 16. MVP işlevsel kabul kapsamı

Kullanıcının onayladığı ilk 10 gereksinim korunur; genişleyen kararlar ayrıca eklenir:

| ID | Kabul hedefi | Doğrulama durumu |
|---|---|---|
| MVP-01 | Çocuk bilgisayarına Tauri uygulaması kurulabilir | Bekliyor |
| MVP-02 | Ebeveyn–çocuk cihazları güvenli eşleştirilir | Bekliyor |
| MVP-03 | Çevrimiçi durum ve son görülme doğru gösterilir | Bekliyor |
| MVP-04 | Uzaktan Windows oturum kilidi komutu uygulanabilir | Bekliyor |
| MVP-05 | Günlük ekran süresi yerelde ölçülür | Bekliyor |
| MVP-06 | Süre dolmadan 5 dk uyarı gösterilir | Bekliyor |
| MVP-07 | Çocuk ek süre talebi gönderebilir | Bekliyor |
| MVP-08 | Ebeveyn talebi onaylayabilir/reddedebilir | Bekliyor |
| MVP-09 | İnternet kesildiğinde son geçerli kurallar sürer | Bekliyor |
| MVP-10 | Komut/kilit ve sonuç geçmişi görülebilir | Bekliyor |
| MVP-11 | **Kindora kullanım kilidi** ve uzaktan kaldırma akışı | Bekliyor; güvenlik PoC gerekli |
| MVP-12 | Compact/Expanded/Mascot Only kalan süre widget'ı | Bekliyor |
| MVP-13 | Alan adı izin/engelleme ve ders modu kuralları | Bekliyor; filtreleme PoC gerekli |
| MVP-14 | Flutter mobilde cihaz durumu ve kritik komutlar | Bekliyor |
| MVP-15 | Mobilde izin taleplerini yanıtlama/bildirim | Bekliyor; push PoC gerekli |
| MVP-16 | Temel mola hedefi ve olumlu oyunlaştırma | Bekliyor |
| MVP-17 | Ebeveyn dashboard ve temel kullanım raporu | Bekliyor |
| MVP-18 | Standart Windows hesabı, çoklu DPI, çevrimdışı, tekrar komut, kurtarma testleri | Bekliyor |

Bu tablo **ürün gereksinimi envanteridir**, ayrıntılı görev listesi değildir. Görev kimliği, öncelik, bağımlılık, test sonucu ve yayın durumu `docs/tasks.md` içinde açılır. Birçok gereksinim MVP'de kademeli teslim edilebilir; kapsam küçültme veya taşıma kararı kullanıcı onayı/ADR gerektirir.

## 17. Başlangıçtaki teknik doğrulama kapıları (uygulamaya geçmeden)

### GATE A — Yerel ajan ve Kindora kilidi

Standart kullanıcı hesabı, servis kurulumu/başlangıcı, oturum yardımcısı ve farklı Windows olaylarında güvenilir kural uygulama. Fullscreen pencerenin tek başına kilit sayılmadığı açıkça test edilir. Güvenli kurtarma yolu gösterilir.

### GATE B — Komut ve kimlik güvenliği

Supabase RLS, aile izolasyonu, cihaz eşleştirme, TTL/idempotency, offline komut, yetki iptali ve yanlış komut tekrarı test edilir.

### GATE C — Ağ filtreleme

Chrome/Edge, DoH, DNS önbelleği, VPN/standart hesap, internet bağlantısı kesilmesi, sistem yeniden başlatma senaryolarıyla desteklenen sınırlar belirlenir.

### GATE D — Masaüstü UI ve widget

WebView2, %100/%125/%150 DPI, küçük ekran, birden fazla monitör, uyku/uyanma, tam ekran oyun, klavye ve azaltılmış hareket testleri.

### GATE E — Flutter mobil

Android/iOS cihazda Supabase Auth, komut onayı ve gerçek push/yeniden açma akışı; arka plan teslimatı varsayılmadan test edilir.

Her kapı `PASS / CONDITIONAL PASS / FAIL / NOT RUN` şeklinde, hangi cihaz/OS/sürümle test edildiği belirtilerek kaydedilir. `NOT RUN` asla `PASS` sayılmaz.

## 18. Kullanıcı deneyimi ilkeleri ve kabul koşulları

- Ebeveyn dashboard'u ilk bakışta süre, çevrimiçi durum ve hızlı komutları gösterir.
- Çocuk mesajları bilgilendirici ve saygılıdır; keyfi, ani kısıtlama yerine önceden bildirir.
- Kısıtlama kaldırma, geçici izin, Windows oturum kilidi farklı fiillerdir; UI metinleri açık.
- Her ağ işlemi `bekliyor / hata / doğrulanamadı / başarı` gerçeğini yansıtır.
- Kullanıcı çalışmasını kaydedebilsin diye uyarılar; işletim sistemi veya kritik işler kontrolsüz kesilmez.
- Kullanım raporları yeterince özet; sürekli gözetim ve ayrıntılı davranış izleme yok.
- Açık/koyu tema tercihi ebeveyn tasarımında korunur; çocuk MVP açık temayla başlayabilir.
- Ebeveyn masaüstü paneli ekranı doldurabilir; responsive düzen yine de zorunludur.

## 19. Ticari ürün vizyonu (ileride)

Hedef yalnız aile içi araç değil, ileride başka aileler için güvenilir bir ürün olabilmek. Değerlendirilecek lisans modelleri:

- **SaaS:** Cihaz başına veya aile başına aylık/yıllık abonelik; bulut/izleme/bildirim maliyetlerini karşılaması daha kolay.
- **Ömür boyu lisans:** Satın alım sonrasında kalıcı yerel uygulama hakkı; sürekli bulut, push ve desteğin finansmanı ayrı analiz edilmeli.
- **Hibrit:** Yerel çekirdek için tek seferlik lisans + isteğe bağlı çevrimiçi aile servisi.

Bu seçenekler **ticari araştırma konusu**; ödeme altyapısı, fiyat, hak sınırlamaları ve mağaza modeli onaylanmamış. Erken mimari kararlar zorunlu abonelik duvarı üretmemeli, ama sunucu maliyetlerini de sınırsız ücretsiz varsaymamalı.

## 20. Tasarım referansları ve durumları

Claude Design üzerinden üretilen referans paketinde şu teslim başlıkları bulunur (bu belgeyi okuyan ajan dosyaların proje içinde gerçekten bulunduğunu ayrıca doğrulamalıdır):

- `design_handoff_kindora/README.md`
- `design_handoff_kindora/components.md`
- `design_handoff_kindora/tokens/_kindora-tokens.scss`
- `design_handoff_kindora/design_files/Kindora Phase 1 - Visual Foundation.dc.html`
- `design_handoff_kindora/design_files/Kindora Phase 2 - Parent Experience.dc.html`
- `design_handoff_kindora/design_files/Kindora Phase 3 - Child Experience.dc.html`
- `design_handoff_kindora/design_files/Kindora Phase 4 - Desktop Companion.dc.html`
- `design_handoff_kindora/design_files/Kindora Phase 5 - Management.dc.html`
- `design_handoff_kindora/design_files/Kindora Phase 6 - Design Review.dc.html`
- `design_handoff_kindora/design_files/Kindora Acceptance Report.dc.html`
- `design_handoff_kindora/mascot_brief/Kindora Mascot Brief.dc.html`
- `design_handoff_kindora/mascot_brief/Kindora Mascot Review.dc.html`
- `design_handoff_kindora/design_files/Kindora Mobile Parent - Flutter.dc.html`

**Kullanıcının mevcut tasarım kararı:** Claude Design genel tasarım yönü başarılı bulundu ve geliştirme referansı olarak kabul edildi. Flutter ebeveyn uygulaması **MVP kapsamına eklendi**; önceki tasarım raporlarında “sonraki sürüm” denmesi artık güncel değildir. Filiz konsepti benimsenmiştir ancak nihai üretim asset'leri ve animasyonları otomatik olarak onaylı sayılmaz.

Görsel kimlik referansları: ana renk `#5265D8`, olumlu vurgu `#2BBFA7`, uyarı `#E7A83E`; Onest genel UI, Fredoka çocuk dostu metin/mascot için tasarım referansı. **Gerçek token değerleri ve kullanım kuralları** daima teslim edilen `_kindora-tokens.scss` ile kontrol edilmeli; çelişki varsa açıklanmalı.

## 21. Bilinmeyenler, riskler ve varsayımlar

| Konu | Mevcut durum |
|---|---|
| Repository/branch/commit, çalışan proje kodu | Bu belgede doğrulanmadı; başlangıçta incelenecek |
| Tauri/Flutter/Bootstrap kesin bağımlılık sürümleri | Başlangıçta kilit dosyalarından veya resmi kaynaktan belirlenecek |
| Windows sürüm hedefi (10/11, mimari) | Henüz kesinleşmedi |
| Windows servis/IPC ve kısıtlama mekanizması | PoC gerekli |
| Network filtreleme yöntemi, VPN/DoH davranışı | PoC gerekli |
| Supabase bölgesi ve ücretli kota politikası | Henüz belirlenmedi |
| Mobil push sağlayıcısı ve maliyeti | Henüz belirlenmedi |
| CI, otomatik test komutları ve release kanalı | Repo incelemesi sonrası belirlenecek |
| Çocuk verileri için saklama, silme ve hukuki süreç | Ürün ve hukuki tasarımda netleştirilecek |
| Çoklu aile / çoklu ebeveyn yetki şeması | Temel izolasyon hedefi var; şema kararı bekliyor |
| Nihai Filiz asset, sprite/Lottie/Rive/vektör uygulaması | Henüz belirlenmedi |
| Ticari lisans modeli | Henüz belirlenmedi |

## 22. Claude Code / Codex yeni oturum başlangıcı

Yeni oturumda kullanılabilecek kısa mesaj:

> Kindora projesinde çalışıyoruz. Önce `AGENTS.md`, `CLAUDE.md` (varsa), `brain.md` ve `design/KINDORA_PROJECT_CONTEXT.md` dosyalarını oku; ilgili mevcut tasarım teslimatını incele. Git branch/status ve mevcut kodla belgelerdeki durum iddialarını doğrula. Ürün kararlarını kodlanmış özellik zannetme. Eksikleri ve çelişkileri açıkla. Mevcut görev ve kabul kriterlerine göre ilerle; yetkisiz commit/push/deploy yapma. Önce güvenlik açısından kritik Windows kullanım kilidi, Supabase komut yetkilendirme ve ağ filtreleme belirsizliklerinin test edilebilir PoC kapsamını çıkar. Aktif görev yoksa kendiliğinden uygulama kodu yazma.

### İlk hafıza kurulumu promptuna eklenecek zorunlu referans

> `design/KINDORA_PROJECT_CONTEXT.md` dosyası Kindora'nın ana ürün bağlamı ve kullanıcı onaylı başlangıç kararlarının kaynağıdır. İçeriğini incele, `docs/architecture.md` içinde **planlanan mimari** ile **kodda doğrulanan mimariyi** ayır; `docs/tasks.md` içerisinde belgeye bağlantı ver. Belgeyi kopyalayarak tekrarlama. `design/design_handoff_kindora/` varsa görsel tasarımın kaynak dosyalarını ayrıca incele. Önce bu bağlamı koruyan hafıza iskeletini oluştur; uygulama koduna müdahale etme.

---

## 23. Bu belgenin ileride güncellenmesi

Değişiklik olduğunda yalnız ilgili bölüm güncellenir, tarih/sürüm revize edilir. Önemli teknik kararlar ADR olarak kaydedilir ve buradan bağlantı verilir. Tamamlanan test sonuçları `docs/tasks.md` veya test raporlarında kanıtlanır; bu belgede yalnız güncel ürün kararının kısa özeti tutulur. Özellikle **tasarlandı / kodlandı / test edildi / yayınlandı** ifadeleri hiçbir zaman birbiri yerine kullanılmaz.
