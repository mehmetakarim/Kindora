# ADR-0016 — Yerel Kindora kısıtlaması PoC'si: durum/ekran ayrımı, zaman modeli, fail-open ve kurtarma

- **Tarih:** 2026-10-31
- **Durum:** **Taslak — kullanıcı onayı bekliyor.** Onaylanmadan P1.E.2 ve sonrası (kısıtlama kodu) başlamaz.
- **Karar türü:** Teknik uygulama yöntemi (PoC)
- **İlgili belgeler:** [P1.E planı](../plans/p1e-proposal.md), [ADR-0001](0001-kindora-kullanim-kilidi-ana-kontrol.md), [ADR-0012](0012-mevcut-windows-hesabinda-calisma.md), [ADR-0014](0014-yerel-kullanim-suresi-motoru.md), [ADR-0015](0015-bagimsiz-session-helper-ve-oturum-acilisi.md), [T0-A](../feasibility/T0-A-kindora-lock.md)

## 1. Bağlam

- P1.E, ADR-0001'deki Kindora kullanım kısıtlamasının yerel ve ölçüm odaklı ilk sürümüdür.
- Kapsam kararları K-1…K-10 verildi (2026-10-31).
- Bu ADR, uygulamaya başlamadan önce güvenlik ve kabul ön kapısını (E.0) tanımlar:
  - durum modeli ve zaman modeli
  - fail-open matrisi
  - kurtarma yolları
  - tehdit modeli
  - test ve kabul planı

## 2. Kararlar (taslak)

| No | Karar |
|---|---|
| D-1 | **Masaüstü köprüsü yalnız durumu okur;** kısıtlama ekranı oluşturmaz. Ekranın tek kaynağı arka plan helper'ıdır. |
| D-2 | **Kısıtlama durumu ve görünür ekran bağımsızdır.** Durum agent'ta (LocalService) ve SQLite'tadır. Ekran helper'da yalnız durumu gösterir; ekranın kapanması durumu değiştirmez. |
| D-3 | Süre **varsayılan 10 dk**, **mutlak üst sınır 30 dk**. Daha uzun istek reddedilir (kısaltılmaz). |
| D-4 | Kısıtlama **yalnız DEV denetim dosyasıyla** etkinleştirilir. IPC'de, masaüstünde veya helper'da etkinleştirme yolu yok. Limit dolunca otomatik kısıtlama varsayılan kapalı (DEV politika bayrağı). |
| D-5 | **Agent kaybında ekran fail-open:** helper agent'tan geçerli durum alamazsa (bağlantı yok, yanıt eskidi, hata) en geç **10 sn** içinde ekranı kaldırır. |
| D-6 | **Zaman modeli** (§3): süre monoton saatle tüketilir; yeniden başlatmada duvar saatine dayalı mutlak bitiş tavanı uzatılamaz. |
| D-7 | **Saat sapması eşiği 60 sn (başlangıç).** Aşılırsa kısıtlama serbest bırakılır (fail-open). Değer Lenovo ölçümüyle gözden geçirilir. |
| D-8 | Kurtarma yolları bağımsızlık ve kalıcılığa göre sınıflandırılır (§5). **R-5 (helper sonlandırma) tek başına kalıcı kurtarma sayılmaz.** |
| D-9 | SQLite **v1 → v2** yalnız tablo ekler. v1 tablolarına dokunulmaz. Eski sürüme dönüş veri silmez (§6). |
| D-10 | Klavye kancası, LocalSystem, Windows oturum kilidi, program sonlandırma ve ağ kesme **yok** (K-5, ADR-0001, ADR-0012). |

## 3. Zaman modeli (D-6, D-7)

**Kayıtlı alanlar** (SQLite `restriction_state`, tek satır):

| Alan | Anlamı |
|---|---|
| `active` | Kısıtlama geçerli mi |
| `reason` | Neden: `dev_file` veya `daily_limit` |
| `requested_secs` | İstenen süre; ≤ 1800 |
| `consumed_ms` | Tüketilen süre; monoton saatle biriktirilir, flush'ta yazılır |
| `applied_wall_ms` | Uygulandığı duvar saati |
| `hard_deadline_wall_ms` | `applied_wall_ms + requested_secs` — **asla artmaz** |
| `dev_version` | Uygulayan DEV dosyası sürümü |

**Kurallar:**

1. **Çalışırken:** Kalan süre `requested − consumed` ile hesaplanır. `consumed` yalnız monoton saatle ilerler; duvar saati değişimi süreyi uzatamaz.
2. **Uyku:** Monoton saatin uykuda ilerleyip ilerlemediği belirsizdir (ADR-0014 bilinmeyeni). Bu yüzden uyanmada duvar saati de denetlenir: `now_wall ≥ hard_deadline_wall_ms` ise kısıtlama serbest bırakılır.
   - Uyku süresi kısıtlamayı **kısaltabilir**, hiçbir zaman **uzatamaz**.
3. **Yeniden başlatma ve oturum kapanışı:** Yükleme sırasında kısıtlama yalnız şu koşulların **hepsi** sağlanırsa sürer:
   - `applied_wall_ms ≤ now_wall < hard_deadline_wall_ms`
   - `consumed_ms < requested_secs × 1000`
   - kayıt tutarlı
   - Kalan süre `min(requested − consumed, hard_deadline − now_wall)` olur.
   - Aksi halde `FREE` (`reason=restart_expired` / `restart_inconsistent`).
4. **Saat sapması (D-7):** Her döngüde `|Δwall − Δmono| > 60 sn` ise `FREE` (`reason=clock_changed`). P1.C `CLOCK_JUMP` ile aynı ölçüdür.
   - **Analiz [Çıkarım; Lenovo'da ölçülecek]:**
     - Normal NTP düzeltmeleri saniye altı ya da birkaç saniyedir; eşiği aşmaz.
     - Yaz saati ve saat dilimi değişimi UTC'yi değiştirmez. Model UTC ms kullanır; yanlış serbest bırakma beklenmez.
     - Uyku/uyanma: uyanmada monoton saat ilerlemediyse büyük sapma görülür ve kısıtlama serbest kalır. Bu güvenli yöndür; uyku sonrası kısıtlamanın kalkması kabul edilmiş bir PoC davranışıdır.
     - Bilerek saat değiştirmek yönetici yetkisi ister ve serbest bırakma ile sonuçlanır (fail-open).
5. **Duvar saati geri gitti** (`now_wall < applied_wall_ms`) → `FREE` (`reason=clock_backwards`).

## 4. Fail-open matrisi

| Olay | Durum | Ekran | Olay kaydı |
|---|---|---|---|
| Süre doldu (monoton) | `FREE` | Kalkar (≤ 5 sn hedef) | `RESTRICTION_RELEASED reason=expired` |
| DEV `release` / dosya silindi | `FREE` | Kalkar | `reason=dev_release` |
| Bozuk veya ACL'i beklenmeyen DEV dosyası | Değişmez; uygulama **reddedilir**; etkin kısıtlama varsa `FREE` | — / kalkar | `RESTRICTION_DEV_REJECTED` |
| Saat sapması > 60 sn | `FREE` | Kalkar | `reason=clock_changed` |
| Duvar saati geri gitti | `FREE` | Kalkar | `reason=clock_backwards` |
| Servis çöktü veya durduruldu | Kayıt kalır | Agent yok → **≤ 10 sn'de kalkar** (D-5) | Helper günlüğü; servis açılınca `AGENT_RESTARTED_DURING_RESTRICTION` |
| Bozuk SQLite / kurtarma / bellek içi çalışma | `FREE` | Kalkar | `reason=storage_recovered` |
| Yeniden başlatma | §3 kural 3 | Oturum açılınca yeniden oluşturulur (ölçülür) | `reason=restart_*` |
| Oturum kapanışı | §3 kural 3 | Yeni oturumda yeniden oluşturulur | `reason=logoff_*` |
| Uyku/uyanma | §3 kural 2 | Uyanınca yeniden çizilir veya kalkar | `reason=resume_expired` |
| Konsol oturumu değişti | Sürer (cihaz düzeyi) | Yalnız etkin konsol oturumunun helper'ı | `SURFACE_SESSION_CHANGED` |
| Kaldırma betiği | Önce `FREE` | Kalkar | `reason=uninstall` |
| Agent iç hatası / bilinmeyen durum | `FREE` | Kalkar | `reason=fail_open` |
| Helper sonlandırıldı | **Değişmez** | Yok; görev en çok 5 dk'da yeniden oluşturur (ölçülür) | `SURFACE_LOST` / `SURFACE_RESTORED` |

**Değişmez kural:** Hiçbir yol 30 dk'yı veya `hard_deadline_wall_ms`'i aşan kısıtlama üretmez.

## 5. Kurtarma yolları — sınıflandırma (D-8)

| # | Yol | Kim | Bağımsız mı? | Kalıcı mı? | Not |
|---|---|---|---|---|---|
| R-1 | DEV `release` dosyası (yardımcı betik) | Yönetici | Agent çalışıyorsa evet | **Evet** (durum `FREE` yazılır) | Agent çalışmıyorsa etkisi açılışta |
| R-2 | Süre dolması (≤ 30 dk) | Otomatik | Evet (kullanıcı eylemi yok) | **Evet** | Mutlak tavan duvar saatine bağlı |
| R-3 | Servisi durdurmak (`sc.exe stop`) | Yönetici | Evet | **Hayır** — ekran kalkar ama durum kalır; servis yeniden başlarsa §3 kural 3 uygulanır | Kalıcılık için R-1/R-2/R-4 ile birlikte |
| R-4 | Kaldırma betiği (önce `FREE`, sonra P1.D sırası) | Yönetici | Evet | **Evet** | Veri korunur |
| R-5 | Helper'ı sonlandırmak | Kullanıcı / yönetici | Evet | **Hayır** — durum değişmez; görev ekranı en çok 5 dk'da geri getirir | **Tek başına kurtarma sayılmaz** |
| R-6 | Güvenli Mod (servis ve görev başlamaz) | Yönetici | Evet | Oturum boyunca; kalıcılık için R-4 | Belge; Lenovo'da isteğe bağlı |

- **Kalıcı ve bağımsız kurtarma:** R-1, R-2, R-4.
- **Geçici:** R-3, R-5, R-6.
- Hiçbiri Windows parolası, oturum kilidi veya ağ gerektirmez.
- **Kabul ölçütü:** her kalıcı yol (R-1, R-2, R-4) CI'da ve Lenovo'da en az bir kez doğrulanmalıdır. Biri başarısız olursa P1.E **FAIL**.

## 6. SQLite v1 → v2 ve geri dönüş (D-9)

- **Migration:** tek transaction'dır ve yalnız `CREATE TABLE restriction_state` ile `PRAGMA user_version = 2` yapar. v1 tablolarına (`daily_usage`, `usage_segments`, `policy_state`, `break_state`, `lifecycle_events`) dokunulmaz. Hata olursa transaction geri alınır; v1 kalır.
- **Doğrulama testi:**
  - P1.D sürümüyle üretilmiş v1 fixture'ı kullanılır.
  - Migration öncesi ve sonrası bütün v1 tablolarının satır sayısı ve içerik özeti (sıralı satırların karması) eşit olmalıdır.
  - Migration ortasında hata benzetimi → dosya v1 ve özet aynı kalır.
- **Eski sürüme dönüş:**
  - P1.D kodu `user_version=2`'yi "daha yeni şema" sayar ve P1.C kuralıyla **bellek içi** çalışır. Dosyaya dokunmaz, veri silinmez.
  - Ödünleşim: o sürede ölçüm kalıcı olmaz.
  - Testi: P1.D sürümü v2 dosyayı açar → `persistent=false`, dosya karması değişmez.
- **v2 → v1 geri migration yok.** Gerekirse yönetici P1.E'ye geri döner; veri v2 dosyasında korunmuştur.

## 7. Tehdit modeli

**Aktörler:**

- Çocuk: mevcut hesap; Lenovo'da yönetici olabilir
- Yönetici
- Aynı oturumdaki herhangi bir süreç
- Hata (yazılım veya ortam)

| Tehdit | Beklenen | Garanti |
|---|---|---|
| Alt+Tab, Win+Tab, Win+D, Win | Ekran öne gelmeyi dener; süreler ölçülür | Yok [PoC gerekli] |
| Helper'ı sonlandırma | Durum değişmez; olay; ekran ≤ 5 dk'da geri gelir | Tespit evet, önleme hayır |
| Masaüstünü kapatma | Etkisiz (D-1) | — |
| Sahte uygulama/kaldırma IPC'si | IPC'de yazma komutu yok; mevcut yetkilendirme | Tasarım |
| DEV dosyasını standart kullanıcının yazması | ACL reddeder; agent ACL'i doğrular ve reddeder | Test edilir |
| Yönetici: dosya, servis, görev, saat | Yetkili yol veya fail-open | Önlenemez (ADR-0012) |
| Saat ileri/geri | Süre uzamaz; sapmada `FREE` | Tasarım |
| Kurtarılamaz kilit | 30 dk tavan + fail-open + R-1/R-2/R-4 | Tasarım; test edilir |
| Tam ekran oyun | Görünmeyebilir; oyun kapatılmaz | Yok |
| Ekranın girdi okuması | Ekran girdi okumaz, kaydetmez; kanca yok | Tasarım |

**Kesin sınırlar:**

- Ctrl+Alt+Del ve güvenli masaüstü engellenmez.
- Windows parolası yok.
- Keylogger veya ekran kaydı yok.
- Tam ekran güvenlik garantisi değildir.
- Ekranın veya sürecin kapatılabilmesi "geçilmez engel" olarak sunulmaz.

## 8. Kesin kabul kriterleri (P1.E PoC)

**CI ile doğrulanacaklar (hepsi PASS olmalı):**

- **C-1:** Kısıtlama yalnız DEV dosyasıyla uygulanır. Bozuk dosya, eski sürüm, bilinmeyen alan veya > 30 dk reddedilir.
- **C-2:** ACL:
  - Düşük yetkili belirteçle DEV dosyası yazılamaz.
  - Yazılabilir ACL'li veya kurulum klasörü dışındaki dosya reddedilir.
- **C-3:** Süre dolması (kısaltılmış DEV süresiyle) → `FREE`, ekran ≤ 5 sn'de kalkar.
- **C-4:** §4 matrisinin her satırı birim/benzetim testiyle doğrulanır. Saat, uyku ve yeniden başlatma satırları benzetim saatiyle test edilir.
- **C-5:** Servis durdurulunca ekran ≤ 10 sn'de kalkar (D-5).
- **C-6:** Servis yeniden başlatmada §3 kural 3; mutlak tavan aşılmaz.
- **C-7:** R-1, R-2 ve R-4 çalışır; kaldırma kısıtlama sırasında cihazı serbest bırakır, `1060` döner, veri korunur.
- **C-8:** v1 → v2 kayıpsız; eski sürüm v2 dosyayı bozmaz (§6).
- **C-9:** Helper sonlandırma → görevle ekranın yeniden oluşması; olay 107 kanıtıyla, süre ölçülür (PASS/FAIL değil).
- **C-10:** P1.A–P1.D ve P1.E.1 regresyonu PASS.

**Lenovo ile doğrulanacaklar (mevcut hesap; yöneticiyse etiketlenir):**

- E-01 … E-12 ([plan §10](../plans/p1e-proposal.md)).
- Geçme ölçütü: E-01, E-07, E-08 ve E-10 (kalıcı kurtarma yolları) **PASS** zorunludur. E-02, E-03 ve E-09 ölçüm kaydıdır.
- Hiçbir senaryoda cihaz 30 dk'dan uzun kısıtlı kalmaz.

## 9. Windows CI test planı (`windows-p1e`)

1. P1.A–P1.D iş akışları ve P1.E.1 konsol kapanışı testi.
2. LocalService + paket betikleri:
   - DEV dosyasıyla uygula → `GET_RESTRICTION` → ekran penceresi (`EnumWindows`, topmost, monitör kaplaması)
   - R-1 ile kaldır.
3. Kısaltılmış süre (ör. 60 sn) → otomatik serbest bırakma.
4. `sc.exe stop` → ekran ≤ 10 sn'de kalkar → `sc.exe start` → kalan süre kuralı.
5. Bozuk SQLite ve saat sapması benzetimi (DEV tanılama kancası değil; birim testleri ve dosya bozma).
6. ACL: düşük yetkili belirteçle yazma girişimi; ACL'i gevşetilmiş dosyanın reddi.
7. Helper sonlandırma → görev → ekranın yeniden oluşması (olay 107/129).
8. Kısıtlama sırasında kaldırma → `1060`, ekran yok, veri korunur.
9. v1 fixture → v2 migration; P1.D ikilisiyle v2 dosya açma.

**CI'da yapılamayanlar (NOT RUN):** gerçek kısayollar, oturum kapatma, yeniden başlatma, uyku, çoklu monitör, DPI.

## 10. Lenovo kabul planı

- Mevcut Lenovo hesabı kullanılır; yeni hesap gerekmez (K-10).
- Kurulum, P1.D betiklerinin genişletilmiş hâliyle yapılır. Mevcut veritabanı korunur (v2 migration).
- Sıra: E-01 → E-07 → E-08 → E-10 (zorunlu kurtarma yolları), ardından ölçüm senaryoları.
- Her senaryo öncesi kurtarma yolu hatırlatılır. Kısıtlama süresi DEV ile kısa tutulur (2–5 dk).

## 11. Açık kalan mimari kararlar

- **Ekranın yeniden oluşma süresi:** şu an görev aralığına bağlıdır (≤ 5 dk). Kısaltma (ek tetikleyici, kısıtlama sırasında daha kısa aralık) PoC ölçümünden sonra karara bağlanır.
- **Saat sapması eşiği 60 sn:** Lenovo'daki uyku ve NTP gözlemiyle gözden geçirilir.
- **Uyku sonrası kısıtlama:** kalkması (güvenli yön) PoC davranışı olarak kabul edilir. Ürün davranışı ayrı karardır.
- **Nihai ekran teknolojisi** (Tauri/Filiz tasarımı): K-2'ye göre geçicidir.
- **Ebeveyn yetkili kaldırma** (Supabase, P4/P5) ve DEV dosyasının üretimdeki karşılığı.
- **Standart kullanıcı hesabıyla ölçüm** (A-10): mevcut hesap yöneticiyse NOT RUN.

## 12. Sonuçlar ve ödünleşimler

- **Artılar:**
  - Kurtarılamaz kilit riski tasarım düzeyinde sınırlı: 30 dk tavan, fail-open ve üç kalıcı kurtarma yolu.
  - Durum ile ekranın ayrılması, kurcalama ile kurtarmayı birbirinden ayırır.
  - Mevcut yetkilendirme ve ölçüm modeli değişmez.
- **Eksiler:**
  - Fail-open nedeniyle kısıtlama yönetici, saat ve servis müdahalesiyle kolayca kalkar. Bu bilinçli bir PoC tercihidir; ürün güvenlik garantisi değildir.
  - Ekran sonlandırılabilir.
  - Uyku sonrası kısıtlama kalkabilir.
