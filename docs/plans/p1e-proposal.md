# P1.E planı — Yerel Kindora kullanım kısıtlaması PoC'si (ölçüm odaklı)

- **Tarih:** 2026-10-31 (öneri), 2026-10-31 (kapsam kararları)
- **Durum:** **Kapsam onaylandı (K-1…K-10); uygulama başlamadı.**
  - Ürün kodu yazmak için önce **E.0 kapısı** geçilmelidir (§8): PoC kabul kriterleri ve geri alma yöntemi kesinleşir, ADR-0016 kullanıcı tarafından onaylanır.
  - Görev: [TASK-010](../tasks.md).
- **Dayanak:**
  - [KINDORA_PROJECT_CONTEXT.md](../../design/KINDORA_PROJECT_CONTEXT.md) §5.3, §8.2, §9, §11, §16–17
  - [Uygulama yol haritası](../feasibility/implementation-roadmap.md) (P1 = PoC-A)
  - [T0-A](../feasibility/T0-A-kindora-lock.md)
  - [ADR-0001](../decisions/0001-kindora-kullanim-kilidi-ana-kontrol.md), [ADR-0006](../decisions/0006-agent-surec-ve-hesap-modeli.md), [ADR-0012](../decisions/0012-mevcut-windows-hesabinda-calisma.md), [ADR-0013](../decisions/0013-masaustu-kabugu-helper-koprusu.md), [ADR-0014](../decisions/0014-yerel-kullanim-suresi-motoru.md), [ADR-0015](../decisions/0015-bagimsiz-session-helper-ve-oturum-acilisi.md)

**Etiketler:**

- **[Onaylı]:** kullanıcı kararı veya kabul edilmiş ADR
- **[Öneri]:** onay bekleyen teknik ayrıntı; E.0'da kesinleşir
- **[Varsayım]:** doğrulanmamış kabul
- **[PoC gerekli]:** işletim sistemi davranışı ölçülmeden iddia edilemez

## 0. Kullanıcı kararları (2026-10-31) [Onaylı]

| No | Karar |
|---|---|
| K-1 | P1.E = **Seçenek A**: yerel Kindora kullanım kısıtlaması PoC'si |
| K-2 | Kısıtlama ekranı geçici olarak Session Helper içindeki **Win32 penceresi**. Nihai tasarım kararı **değildir**. |
| K-3 | Uygulama ve kaldırma kaynağı **yalnız DEV dosyası**; limit dolunca otomatik kısıtlama **varsayılan kapalı** |
| K-4 | Varsayılan süre **10 dk**, mutlak sınır **30 dk**; fail-open; güvenli kaldırma |
| K-5 | **Klavye kancası yok** |
| K-6 | Helper yeniden başlatma ve ekranın yeniden oluşturulması **ölçülür**. Masaüstü köprüsünün rolü güvenlik modeliyle **ayrıca doğrulanır**; varsayılmaz. |
| K-7 | SQLite **v2**; v1 verilerinin **kayıpsız** korunması zorunlu |
| K-8 | ADR-0013/0014/0015 **Kabul edildi**; açık riskler korunur |
| K-9 | OBS-011, küçük, izole ve doğrulanabilir **ilk sağlamlaştırma görevi** |
| K-10 | Mevcut Lenovo hesabıyla test; yeni kullanıcı hesabı gerekmez |

**Ek koşullar [Onaylı]:**

1. Kısıtlama **durumu** ile kısıtlama **ekranı** ayrı kavramlardır (§3.1).
2. Bir süreci veya pencereyi kapatmak Windows güvenliği açısından geçilmez engel olarak sunulmaz.
3. Şu durumların her biri için güvenli serbest bırakma tasarlanır (§3.2):
   - saat manipülasyonu
   - servis çökmesi
   - bozuk SQLite
   - yeniden başlatma
   - oturum kapanışı
4. Belgelenmiş ve **doğrulanmış** yönetici kurtarma mekanizması olmalıdır (§11).
5. DEV dosyasının yetkilendirmesi ve ACL güvenliği test edilir (§9).
6. P1.A–P1.D regresyon testleri korunur.
7. Gerçek kısıtlama uygulaması başlamadan önce PoC kabul kriterleri ve geri alma yöntemi kesinleşir (E.0, §8).

## 1. Fazın amacı

Kindora kullanım kısıtlamasının (ADR-0001) mevcut mimari üzerinde **yerel**, **geri alınabilir** ve **ölçülebilir** ilk sürümünü kurmak. Gerçek Windows davranışını senaryo başına ölçmek: "neyi yapabilir, neyi yapamaz" tablosu (GATE A, T0-A §6 PoC-A).

- Sonuç "kırılamaz kilit" değildir. Senaryo başına **tespit + tepki süresi** ölçülür.
- Ebeveyn uzaktan onayı, ek süre talebi ve bulut bu fazda **yoktur**.

## 2. Onaylı gereksinimlerle ilişkisi

| Gereksinim | İlişki | Bu fazda |
|---|---|---|
| ADR-0001 [Onaylı] | Doğrudan | Yalnız Kindora kısıtlaması. `LOCK_WINDOWS_SESSION` **yok**. |
| MVP-11 | Kısmen | Yerel kısıtlama ve yerel kaldırma. Uzaktan kaldırma P4/P5'e bağlı. |
| §5.3 (süre bitince kısıtlama ekranı) | Kısmen | Yalnız DEV bayrağıyla, varsayılan kapalı (K-3). Ek süre isteği yok. |
| §8.2 PoC maddeleri 3, 5, 6 | Doğrudan | Ölçülür |
| GATE A | Doğrudan | Kısmi kanıt. Standart hesap senaryoları NOT RUN (K-10). |
| MVP-09 | Destekler | Kısıtlama durumu SQLite'ta tutulur; ağ gerektirmez. |
| ADR-0012 [Onaylı] | Sınır | Lenovo hesabı yönetici olabilir; sonuçlar bu koşulla etiketlenir. |
| AGENTS §5.2 / §5.4 [Onaylı] | Sınır | Tam ekran güvenlik iddiası yok. Geri döndürülemez kilit yok. Parola yok. |

## 3. Kapsam

### Kapsam içi

1. **Kısıtlama durumu (agent, yetkili kaynak):** durum makinesi, kalıcılık, güvenlik ağları, olay kaydı (§3.1, §3.2).
2. **Uygulama ve kaldırma yalnız DEV denetim dosyasından** (K-3): `kindora-dev-restriction.json`.
   - Agent klasöründe, yalnız yöneticinin yazabildiği yerde durur. DEV politika dosyası (ADR-0014) ile aynı desen: `kind` / `dev_only` / `version`, bilinmeyen alan reddedilir.
   - İstenen süre ≤ 30 dk; varsayılan 10 dk.
   - IPC'de uygulama veya kaldırma komutu **yok**.
3. **Salt okunur IPC `GET_RESTRICTION`:** helper ve köprü kısıtlama durumunu okur. Yanıt boyutu sınırlıdır; yetkilendirme mevcut helper politikasıyla aynıdır.
4. **Kısıtlama ekranı (geçici Win32, K-2):** helper `activity` rolünde, her monitörde tam ekran, en üstte duran, çerçevesiz pencere.
   - Türkçe açıklayıcı metin gösterir.
   - Girdi okumaz veya kaydetmez. Klavye kancası yok (K-5).
   - Odak kaybında öne gelmeyi dener; bu davranış **ölçülür**.
5. **Limit dolunca otomatik kısıtlama:** DEV politika bayrağıyla, **varsayılan kapalı** (K-3).
6. **Tanılama:** masaüstü ve durum betiği kısıtlama durumunu, nedenini, kalan süreyi ve ekranın görünür olup olmadığını gösterir. **Kaldır düğmesi yok.**
7. **Ölçüm olayları:** ekran sürecinin kaybı, ekranın yeniden oluşturulma süresi, konsol oturumu değişimi, serbest bırakma nedeni → SQLite olayları.

### Kapsam dışı

- Ebeveyn uzaktan onayı veya kaldırması, ek süre talebi, eşleştirme, Supabase (P4/P5).
- `LOCK_WINDOWS_SESSION`, `WTSDisconnectSession`, oturum kapatma, program sonlandırma, internet kesme.
- LocalSystem'e yükseltme, M2/M3 helper modelleri (ADR-0006), oturuma süreç enjeksiyonu.
- Klavye kancası ve kısayol yutma (K-5).
- Nihai Filiz/Tauri kısıtlama tasarımı, widget, ses.
- Assigned Access / kiosk, Görev Yöneticisi'ni kapatan politikalar, Ctrl+Alt+Del müdahalesi.
- Web filtreleme, uygulama kategorileri.

### 3.1 Durum ile ekran ayrımı (ek koşul 1)

| Kavram | Sahibi | Yetki | Kalıcılık | Anlamı |
|---|---|---|---|---|
| **Kısıtlama durumu** (`RestrictionState`) | Agent (LocalService) | Yalnız DEV dosyası ve güvenlik ağları değiştirir | SQLite v2 | "Şu an Kindora kısıtlaması geçerli mi, neden, en geç ne zamana kadar?" |
| **Kısıtlama ekranı** (`RestrictionSurface`) | Session Helper (kullanıcı oturumu) | Durumu yalnız **okur** (`GET_RESTRICTION`); hiçbir şeyi değiştirmez | Yok | Durumun kullanıcıya gösterimi; görünür veya görünmez |

- Ekranın kapanması, sürecin sonlandırılması veya masaüstü köprüsünün kapanması durumu **değiştirmez**. Yalnız ölçüm olayı üretir (`SURFACE_LOST`, `SURFACE_RESTORED` + süre).
- Ekranın kapatılabilmesi bir güvenlik açığı olarak değil, **ölçülen davranış** olarak raporlanır (ek koşul 2). Kısıtlama "geçilmez" diye sunulmaz.
- Agent'a ulaşılamazsa ekran **kendini kaldırır** (fail-open). Ekran agent yokken kendi başına kısıtlama sürdürmez.

### 3.2 Güvenli serbest bırakma matrisi (ek koşul 3) [Öneri — E.0'da kesinleşir]

| Durum | Kısıtlama durumu | Ekran | Olay |
|---|---|---|---|
| Süre doldu (varsayılan 10 dk / istenen ≤ 30 dk) | `FREE` | Kalkar | `RESTRICTION_RELEASED reason=expired` |
| DEV dosyası `release` veya dosya silindi | `FREE` | Kalkar | `reason=dev_release` |
| **Saat ileri/geri alındı** | Süre **monoton saatle** ölçülür; duvar saati değişimi süreyi uzatamaz. Duvar–monoton sapması > 60 sn ise `FREE`. | Kalkar | `reason=clock_changed` |
| **Servis çöktü / durdu** | Durum SQLite'ta kalır, sınır aşağıdaki "yeniden başlatma" kuralıdır | Agent'a ulaşılamaz → **ekran kendini kaldırır** | Helper günlüğü; servis açılınca `AGENT_RESTARTED_DURING_RESTRICTION` |
| **Bozuk SQLite** (kurtarma veya bellek içi çalışma) | `FREE` | Kalkar | `reason=storage_recovered` |
| **Yeniden başlatma** | Sürer, ancak mutlak sınır duvar saatiyle (`applied_at_wall + ≤30 dk`) denetlenir. Sınır geçti, duvar saati `applied_at`'ten geride veya kayıt tutarsızsa `FREE`. | Oturum açılınca yeniden oluşturulur (ölçülür) | `reason=restart_*` |
| **Oturum kapanışı** | Yeniden başlatmayla aynı kural; mutlak sınır korunur | Yeni oturumda yeniden oluşturulur | `reason=logoff_*` |
| Konsol oturumu değişti (başka kullanıcı) | Sürer (cihaz düzeyi) | Yalnız etkin konsol oturumunun helper'ı gösterir | `SURFACE_SESSION_CHANGED` |
| Kaldırma betiği | Önce `FREE` yazılır; servis ve görev kaldırılır | Kalkar | `reason=uninstall` |
| Agent iç hatası, bilinmeyen durum | `FREE` | Kalkar | `reason=fail_open` |

**İlke:** Hiçbir durum 30 dk'yı aşan veya yönetici kurtarmasıyla kaldırılamayan bir kısıtlama üretmez.

## 4. Gerekli mimari değişiklikler

| Bileşen | Değişiklik |
|---|---|
| Yeni `restriction` modülü (platformdan bağımsız) | `RestrictionState`, güvenlik zaman aşımı, §3.2 matrisi; benzetim saatiyle test edilir |
| SQLite | **Şema v2 migration (K-7):** `restriction_state` + olay türleri. v1 tabloları ve verileri **dokunulmadan** korunur. Migration tek transaction'dır; başarısızsa v1 kalır. |
| `ipc-protocol` | `GET_RESTRICTION` (salt okunur), `Payload::Restriction`. Yazma komutu yok. |
| `windows-agent` | DEV denetim dosyası okuyucu (5 sn), ACL denetimi, durum yayını, olaylar |
| `windows-session` | `RestrictionSurface` (Win32, ayrı UI iş parçacığı). Agent yokken fail-open. OBS-011 düzeltmesi (E.1). |
| Masaüstü | Salt okunur tanılama. Köprünün ekrandaki rolü E.0'da güvenlik modeliyle belirlenir (K-6). |
| `packaging/p1e` | P1.D betikleri + yönetici gerektiren DEV `apply`/`release` yardımcıları; kaldırma önce serbest bırakır |
| ADR | **ADR-0016** (E.0): kısıtlama durumu/ekran modeli, yetki kaynağı, §3.2 matrisi, kurtarma |

**Değişmeyenler:** helper yetkilendirme politikası, LocalService, tek raporlayıcı, ölçüm modeli.

## 5. Önceden çözülmesi gereken belirsizlikler (E.0'da kapanır)

1. **Masaüstü köprüsünün rolü (K-6):** Köprü kısıtlama ekranını göstermeli mi?
   - Köprü masaüstünün alt sürecidir ve çocuk tarafından kapatılabilir.
   - Gösterirse ikinci bir ekran kaynağı olur ve "iki ekran" sorunu doğar; göstermezse helper kaybında ekran yok.
   - E.0'da tehdit modeliyle karar verilir; varsayılan öneri: **köprü yalnız durumu gösterir, ekran oluşturmaz**.
2. **Kurtarma süresi:** Helper sonlandırılırsa ekran görev aralığında (≤5 dk) geri gelir.
   - Bu fazda **ölçülür**; kısaltma (ek tetikleyici) ayrı karar.
3. **Saat sapması eşiği:** 60 sn önerisi. Uyku, saat dilimi ve NTP düzeltmelerinde yanlış serbest bırakma oranı ölçülür.
4. **ACL doğrulaması:** DEV dosyası klasörünün `C:\Program Files\KindoraDev` ACL'i; standart kullanıcının ve `Users` grubunun yazamadığı test edilir. Lenovo hesabı yöneticiyse bu sınır o hesap için geçerli değildir (ADR-0012).
5. **OBS-011:** E.1'de kök neden doğrulanır ve düzeltilir. Ekran kaybı ölçümleri temiz kapanışla karışmasın.

## 6. Tehdit modeli ve güvenlik sınırları

**Varlıklar:** kısıtlama durumu, DEV denetim dosyası, SQLite, kullanıcının çalışması, cihazın kullanılabilirliği.

| Tehdit | Aktör | Beklenen davranış | Garanti? |
|---|---|---|---|
| Alt+Tab, Win+Tab, Win+D, Win key | Çocuk | Ekran öne gelmeyi dener; ölçülür | **Hayır** [PoC gerekli] |
| Görev Yöneticisi ile helper'ı sonlandırma | Çocuk | Durum değişmez; olay; görev ekranı yeniden oluşturur (ölçülür) | Tespit evet, önleme hayır |
| Masaüstünü kapatma | Çocuk | Durum değişmez (K-6) | — |
| Win+L, oturum kapatma, yeniden başlatma | Çocuk | §3.2 matrisi | [PoC gerekli] |
| Saat değiştirme | Yönetici | Süre uzamaz; sapmada fail-open | Kasıtlı fail-open |
| DEV dosyasını değiştirme | Standart kullanıcı | ACL reddeder (test edilir) | ACL'e bağlı |
| DEV dosyasını değiştirme | Yönetici | Yetkili yol | — |
| Servisi durdurma veya kaldırma | Yönetici | Kısıtlama biter; olay kaydedilir | Önlenemez (ADR-0012) |
| Sahte uygulama/kaldırma IPC'si | Herhangi bir süreç | IPC'de yazma komutu yok; mevcut yetkilendirme | Evet (tasarım) |
| Kurtarılamaz kilit | Hata | 30 dk mutlak sınır + fail-open + yönetici kurtarması | Tasarım hedefi; test edilir |
| Tam ekran oyun | Çocuk | Ekran görünmeyebilir; oyun kapatılmaz | Hayır |

**Kesin sınırlar [Onaylı]:**

- Windows parolası toplanmaz, saklanmaz, atlatılmaz.
- Ctrl+Alt+Del ve güvenli masaüstü engellenmez.
- Keylogger, ekran kaydı veya mesaj izleme yok.
- Tam ekran veya always-on-top güvenlik garantisi değildir.
- IPC kimlik doğrulaması gevşetilmez.
- Ayrı Windows hesabı zorunlu değildir.

## 7. PoC gerektiren işletim sistemi davranışları

| ID (T0-A) | Davranış | Ölçüm |
|---|---|---|
| A-01 | Alt+Tab, Win+Tab, Win+D, Win+M, Win | Görünmez kalma süresi; geri gelme süresi |
| A-02/A-03 | Görev Yöneticisi ile helper sonlandırma | Tespit süresi; ekranın yeniden oluşturulma süresi (K-6) |
| A-04 | Win+L, oturum kapat/aç | §3.2 kuralı doğru mu |
| A-05 | Yeniden başlatma, hızlı başlangıç | Aynı; mutlak sınır |
| A-06 | Uyku/uyanma | Süre ve saat sapması kuralı; ekran yeniden çiziliyor mu |
| A-07 | İkinci monitör, DPI %125/%150 | Kaplama, taşma (isteğe bağlı) |
| A-08 | Tam ekran video/oyun | Görünürlük |
| A-10 | Standart kullanıcının servisi durdurma girişimi | Mevcut hesap yönetici ise **NOT RUN** (K-10) |
| A-11 | Yönetici kurtarması | **Her zaman çalışmalı**; başarısızlık = FAIL |
| A-12 | `LockWorkStation` | **Kapsam dışı** |

## 8. Alt görevler ve uygulama sırası

| Sıra | Alt görev | Çıktı | Kapı |
|---|---|---|---|
| **E.0** | **Kabul kriterleri + geri alma yöntemi + ADR-0016** (yalnız belge) | §3.2, §10 ve §11 kesin değerler; köprü rolü kararı (K-6); ADR-0016 taslağı | **Kullanıcı onayı olmadan E.2+ başlamaz** (ek koşul 7) |
| **E.1** | **OBS-011 sağlamlaştırma** (K-9): oturum sonu / konsol kapanışında temiz çıkış izi | Küçük kod değişikliği + Windows CI testi + NOTE güncellemesi | E.0'dan bağımsız; ürün davranışı değişmez |
| E.2 | `restriction` durum makinesi + §3.2 matrisi (saf modül) | Birim/benzetim testleri | E.0 |
| E.3 | SQLite v2 migration | v1 → v2 kayıpsız migration testleri, v1 fixture | E.2 |
| E.4 | DEV denetim dosyası + ACL denetimi | Birim + Windows ACL testleri | E.2 |
| E.5 | `GET_RESTRICTION` IPC + köprü snapshot | Protokol testleri, satır sınırı | E.2 |
| E.6 | Helper `RestrictionSurface` (Win32) + agent yokken fail-open | Windows CI pencere testleri | E.5 |
| E.7 | Masaüstü salt okunur tanılama | Vitest | E.5 |
| E.8 | `packaging/p1e` + Türkçe ölçüm rehberi + yönetici kurtarma rehberi | Paket | E.4, E.6 |
| E.9 | `windows-p1e` CI uçtan uca + P1.A–D regresyonu | CI | E.8 |
| E.10 | Lenovo ölçüm turu | Ölçüm tablosu | E.9 + kullanıcı onayı |

### İlk uygulanabilir alt görev: E.1 — OBS-011

- **Amaç:** Windows oturumu kapanırken veya `conhost` konsolu kapatırken arka plan helper'ı temiz çıkış izini silebilsin. Temiz kapanış `previous_exit state=unclean` olarak görünmesin.
- **Hipotez (H1):** `ctrlc` 3.5.2 işleyicisi `CTRL_CLOSE` / `LOGOFF` / `SHUTDOWN` olaylarında hemen `TRUE` döner; Windows süreci beklemeden sonlandırır.
- **Adımlar:**
  1. Windows CI'da yeniden üret: `conhost --headless` altındaki helper'a `CTRL_CLOSE` gönder (konsolu kapat) → iz dosyası kalıyor mu, `helper_stopped` yazılıyor mu.
  2. H1 doğrulanırsa: helper kendi konsol işleyicisini kullanır. Bu üç olayda kapanma bayrağını ayarlar ve ana döngünün izi silmesini **≤2 sn** bekleyip döner. Ctrl+C davranışı değişmez.
  3. Testler:
     - Windows CI: `CTRL_CLOSE` sonrası iz yok ve `helper_stopped` var.
     - Mevcut `session_end` testi geçer.
     - P1.A–D regresyonu geçer.
  4. H1 doğrulanmazsa kod değişmez; bulgular OBS-011'e eklenir.
- **Kapsam:** yalnız `crates/windows-session` (helper), test, `docs/solutions.md`. Agent, IPC, SQLite ve betikler değişmez.
- **Kabul:** Windows CI PASS; isteğe bağlı Lenovo doğrulaması (oturumu kapat/aç → `previous_exit` yok). Yeni prerelease ayrı karardır.

## 9. Testler

**Birim / benzetim (macOS):**

- Durum geçişleri; 10/30 dk sınırları; monoton süre; yeniden başlatmada mutlak sınır.
- Saat ileri/geri → süre uzamaz veya fail-open.
- Bozuk DB → `FREE`.
- DEV dosyası doğrulaması: bozuk, eski sürüm, bilinmeyen alan, > 30 dk.
- Otomatik kısıtlama bayrağı varsayılan kapalı.

**v1 → v2 migration (K-7):**

- P1.C/P1.D şemasıyla oluşturulmuş v1 veritabanı fixture'ı. Migration sonrası bütün v1 tablo satırları birebir korunur (satır sayısı + içerik karşılaştırması).
- Migration ortasında hata → v1 korunur.
- v2'yi gören eski sürüm → bellek içi çalışır, dosyaya dokunmaz.

**Entegrasyon (macOS + Windows konsol agent):**

- Uygula → `GET_RESTRICTION` → kaldır.
- Yeniden başlatmada sınır.
- Helper kaybında olay; agent yokken ekran fail-open.
- Tek raporlayıcı ve ölçüm regresyonu.

**Windows CI (`windows-2022`):**

- P1.A–D regresyonu.
- LocalService ile DEV dosyası uygulama ve kaldırma.
- **ACL testi (ek koşul 5):**
  - Standart kullanıcı benzeri düşük yetkili belirteçle DEV dosyasına yazma girişimi **reddedilir**.
  - Dosya kurulum klasörü dışına konursa agent okumaz.
  - Agent dosyanın sahibini ve ACL'ini doğrular; beklenmeyen yazılabilirlikte uygulamayı reddeder.
- **Ekran:** `EnumWindows` ile varlık, topmost ve monitör kaplaması; serbest bırakmada kaybolma.
- **Güvenli serbest bırakma:** süre dolması, bozuk DB, saat değişimi, servis durdurma → ekranın kalkması.
- Helper sonlandırma → görevle ekranın yeniden oluşması (olay 107 kanıtı; süre ölçülür).
- Kısıtlama sırasında kaldırma betiği çalışır ve cihaz serbest kalır.
- **CI'da yapılamayanlar (NOT RUN):** gerçek kısayollar, oturum kapatma, yeniden başlatma, uyku, çoklu monitör, DPI.

## 10. Gerçek Lenovo kabul senaryoları (taslak; E.0'da kesinleşir)

| ID | Senaryo | Geçme ölçütü |
|---|---|---|
| E-01 | DEV uygula | Ekran ≤ 5 sn'de tüm monitörlerde |
| E-02 | Alt+Tab, Win+Tab, Win+D, Win | Ölçüm kaydı (süreler; PASS/FAIL değil) |
| E-03 | Görev Yöneticisi ile helper sonlandır | Durum değişmez; olay; yeniden oluşma süresi ölçülür |
| E-04 | Win+L → aç | §3.2'ye uygun |
| E-05 | Oturumu kapat → aç | §3.2'ye uygun; 30 dk sınırı aşılmaz |
| E-06 | Yeniden başlat | §3.2'ye uygun; kullanım verisi korunur |
| E-07 | DEV kaldır | ≤ 5 sn'de ekran kalkar |
| E-08 | Süre dolması | Kendiliğinden kalkar |
| E-09 | Saat değiştirme (yönetici) | Süre uzamaz; gerekirse serbest bırakır |
| E-10 | Yönetici kurtarması (§11) | Her adım çalışır; cihaz serbest kalır |
| E-11 | İsteğe bağlı: otomatik limit bayrağı | Kısıtlama başlar; süre sınırıyla biter |
| E-12 | İsteğe bağlı: ikinci monitör, %125/%150 | Taşma yok |

Hesap yöneticiyse rapor bunu belirtir. Standart hesap senaryoları **NOT RUN** (K-10).

## 11. Geri alma ve yönetici kurtarması (ek koşul 4)

**Kurtarma sırası** (her biri tek başına yeterli; belge + test):

| # | Yol | Kim | Doğrulama |
|---|---|---|---|
| R-1 | DEV `release` dosyası (yardımcı betik) | Yönetici | CI + Lenovo E-07 |
| R-2 | Süre dolması (≤30 dk) | Otomatik | CI + Lenovo E-08 |
| R-3 | Servisi durdurma (`sc.exe stop`) → ekran fail-open ile kalkar | Yönetici | CI + Lenovo E-10 |
| R-4 | Kaldırma betiği (önce `FREE`, sonra P1.D sırası; veri korunur) | Yönetici | CI + Lenovo E-10 |
| R-5 | Görev Yöneticisi ile helper'ı sonlandırma (ekran kalkar; görev geri getirebilir; R-1/R-3 ile birlikte) | Kullanıcı / yönetici | CI |
| R-6 | Güvenli Mod (Kindora servisi ve görevi başlamaz) | Yönetici | **Belge**; Lenovo'da isteğe bağlı |

- Hiçbir yol Windows parolası, oturum kilidi veya ağ gerektirmez.
- **Şema geri dönüşü:** v2 → v1 migration yoktur. Eski sürüm v2'yi görürse bellek içi çalışır; veri silinmez. P1.D sürümüne dönmek için kaldır + P1.D kur; veriler korunur ama v1 kodu v2 dosyasını yazmaz. Bu sınır ADR-0016'da açıkça yazılır.

## 12. Tahmini karmaşıklık ve risk

| Alan | Karmaşıklık | Risk |
|---|---|---|
| E.1 OBS-011 | Düşük | Düşük |
| Durum makinesi + §3.2 + SQLite v2 | Orta | Orta (migration kayıpsızlığı) |
| DEV denetim yolu + ACL | Düşük–Orta | Orta |
| Win32 ekran + çoklu monitör + odak | **Yüksek** | **Yüksek** |
| Genel | Orta–Yüksek; E.0–E.10 | Yüksek; 30 dk sınırı, fail-open ve R-1…R-6 ile yönetilir |

## 13. Açık karar noktaları (E.0'da kullanıcıya sunulacak)

- Masaüstü köprüsünün ekran rolü (öneri: ekran oluşturmaz).
- Saat sapması eşiği (öneri: 60 sn).
- §10 ölçütlerinin kesin hedef değerleri (ör. ≤ 5 sn).
- E.1'in Lenovo doğrulaması için ara prerelease yapılıp yapılmayacağı.

## Alternatifler (değerlendirildi, seçilmedi)

| Seçenek | İçerik | Neden şimdi değil |
|---|---|---|
| B | P4: Supabase Auth + eşleştirme + RLS (GATE B) | Büyük ve bulut kararları gerektiriyor; A ile paralel yürütülmesi ayrı karar |
| C | GATE D: Filiz widget, DPI, çoklu monitör | Kilit sorusunu erteler |
| D | P1.D.2 sağlamlaştırma | Yalnız OBS-011 kısmı E.1 olarak alındı |
