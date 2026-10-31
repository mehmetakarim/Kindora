# P1.D — Bağımsız Session Helper ve arka plan kullanım süresi ölçümü

- **Görev:** [TASK-009](../tasks.md)
- **Karar:** [ADR-0015](../decisions/0015-bagimsiz-session-helper-ve-oturum-acilisi.md)
- **Önceki aşama:** [P1.C](../screen-time/p1c-screen-time.md)
- **Durum:** Tamamlandı — **Lenovo Acceptance PASS** (D-01…D-10, 2026-10-31). Prerelease `p1d-windows-test.1`. Geliştirme/test kabulüdür, üretim yayını değildir.
- **Kapsam dışı:** zorunlu kilit, site engelleme, program sonlandırma, uzaktan ebeveyn kontrolü, ebeveyn yetkilendirmesi.

## 1. Mimari ve helper yaşam döngüsü

```text
Windows oturum açılışı ──► Zamanlanmış Görev \Kindora\SessionHelper-<SID>
 (+ N dk zaman tetikleyicisi)  (LeastPrivilege, InteractiveToken, IgnoreNew)
                                     │ conhost --headless
                                     ▼
 kindora-helper activity  ──REPORT_ACTIVITY(idle_ms, kind=BACKGROUND_*)──►  kindora-agent (LocalService)
   (oturum başına tek örnek)                                                  tek raporlayıcı kiralaması
                                                                              1 sn döngü → SQLite
 kindora-desktop ── stdio ──► kindora-helper bridge ──GET_STATUS/GET_SCREEN_TIME──►  (aynı agent)
                                    (yalnız yedek raporlayıcı: kind=DESKTOP_BRIDGE)
```

**Arka plan helper'ı (`activity`):**

1. Kayıt dosyasını açar (`%LOCALAPPDATA%\Kindora\helper\activity.log`, 1 MB'ta `.1`).
2. Tek örnek kilidini alır. Alamazsa `already_running` kaydedip 0 koduyla çıkar.
3. Önceki çalıştırmanın izi (`activity.log.running`) varsa `previous_exit state=unclean` yazar.
4. Oturum sonu için gizli pencere açar (`WM_ENDSESSION`). Ctrl+C işleyicisini kurar.
5. **Bağlantı döngüsü:**
   - Agent'a bağlanır ve `GET_STATUS` ister.
   - Her 5 sn'de bir `REPORT_ACTIVITY` gönderir.
   - Kabul durumu değişirse `reporter_active` / `reporter_standby` kaydeder.
   - Bağlantı koparsa geri çekilmeyle (250 ms → 10 sn) yeniden bağlanır.
   - Agent yetkisiz derse 30 sn bekler.
6. Temiz çıkışta iz dosyasını siler. Kayıt yalnız durum geçişlerinde yazılır.

**Masaüstü ile ilişki:**

- Masaüstü arka plan helper'ını **başlatmaz ve durdurmaz**.
- Kapanınca yalnız kendi köprüsünü kapatır. Arka plan helper'ı ve agent çalışmaya devam eder.
- Açılınca agent'tan güncel süreyi ve raporlayıcı bilgisini okur:
  - Ana görünüm: "Arka plan ölçümü açık" / "… yalnız bu pencere açıkken" / "Ölçüm yapan yardımcı yok".
  - Teknik tanılama: raporlayıcı rolü, PID ve oturum; son (yeniden) bağlantı; son rapor; reddedilen ikinci raporlayıcı sayısı; bugünkü ölçülemeyen süre.

## 2. Windows başlangıç mekanizması

[ADR-0015](../decisions/0015-bagimsiz-session-helper-ve-oturum-acilisi.md) seçenekleri karşılaştırır. Seçilen yöntem kullanıcı başına Zamanlanmış Görev'dir.

| Ayar | Değer | Neden |
|---|---|---|
| Tetikleyici | Oturum açılışı (yalnız hedef SID) + kurulumdan 1 dk sonra başlayan, `PT{N}M` aralıklı, süresiz zaman tetikleyicisi | Başlatma ve çökme sonrası sınırlı kurtarma. Oturum açılışı tetikleyicisinin tekrarı ilk gerçek oturum açılışına kadar başlamadığı için ayrı zaman tetikleyicisi kullanılır. Kullanıcı oturumda değilken çalışmaz (`InteractiveToken`). |
| Çalıştıran | Hedef kullanıcı, `InteractiveToken`, `LeastPrivilege` | Kullanıcının oturumunda; yönetici yok, parola saklanmaz |
| Birden çok örnek | `IgnoreNew` (+ helper mutex'i) | Çalışan helper çoğaltılmaz |
| Pil | `DisallowStartIfOnBatteries=false`, `StopIfGoingOnBatteries=false` | Dizüstünde (Lenovo) varsayılanlar helper'ı durdururdu |
| Süre sınırı | `PT0S` | Varsayılan 72 sa sınırı helper'ı sonlandırırdu |
| Öncelik | 7 (normalin altı) | Düşük kaynak |
| Hata durumunda | 1 dk arayla 3 kez | **Etkisiz** (P1.D.1): sonlandırılan helper'da `conhost` 0 koduyla çıkar, görev başarılı sayılır. Kurtarma yalnız zaman tetikleyicisiyle olur ([NOTE-010](../solutions.md)). |
| Eylem | `conhost.exe --headless "…\kindora-helper.exe" activity --launch logon-task` | Konsol penceresi görünmez |

**Agent:**

- Varsayılan `start= demand`.
- `-AgentStartMode Auto` ile `start= auto`.
- Kurtarma eylemleri her zaman kurulur: `restart/5000/restart/60000/restart/300000`, sıfırlama 86400 sn.

**Üretim bağımlılıkları:** agent `auto` + kurtarma; helper oturum açılışı görevi. İkisi birbirini başlatmaz. Hangisi önce açılırsa diğerini beklerken geçen süre `UNKNOWN` sayılır.

## 3. Süre ölçümü ve tekil raporlama

- P1.C ölçüm motoru ([ADR-0014](../decisions/0014-yerel-kullanim-suresi-motoru.md)) **değişmedi**: girdi sınırı modeli, monoton saat, `ACTIVE` / `IDLE` / `SESSION_LOCKED` / `SUSPENDED` / `UNKNOWN`.
- Helper yoksa, raporu eskiyse veya oturum değiştiyse süre `UNKNOWN` sayılır. Bağlantı kesintisi sonradan aktif kullanıma dönüştürülmez.
- **Tek raporlayıcı kiralaması:** ayrıntılar ADR-0015'tedir. Özet:

| Durum | Sonuç |
|---|---|
| Kiralama boş | Raporlayıcı olur (`REPORTER_ACQUIRED`) |
| Aynı bağlantı | Yenilenir |
| Kiralayan 15 sn sustu veya başka oturuma ait | Yeni raporlayıcı alır |
| Arka plan helper'ı köprünün / tek seferliğin yerine | Devralır (`REPORTER_TAKEOVER`) |
| Diğer her durum | Reddedilir (`applied=false`, `competing_reports`+1) |
| Kiralayan bağlantı kapandı / sustu / konsol oturumu değişti | Bırakılır (`REPORTER_RELEASED reason=…`) |

- **Neden iki kez sayılamaz:**
  - Süre yalnız agent döngüsünde sayılır. Rapor bir ölçüm noktasıdır, aralık değildir.
  - Tekrarlanan, geciken veya eşzamanlı raporlar noktayı yalnız günceller.
  - Bu, birim testinde iki raporlayıcının yoğun raporlarıyla ölçülür: toplam ≤ geçen süre + 1 sn.
- **Gecikmiş veya eski mesajlar:**
  - Bir bağlantıdaki mesajlar sıralıdır; `message_id` tekrarı reddedilir.
  - Helper rapor biriktirmez ve yeniden bağlanınca geçmiş göndermez.
  - Agent zamanı alış anına göre damgalar. Helper'ın saati kullanılmaz.

## 4. Güvenlik modeli

- **IPC (dar kapsam):**
  - Yeni komut yok.
  - `REPORT_ACTIVITY.activity.kind` isteğe bağlı bir enum'dur. Bilinmeyen değer ve ek alan reddedilir (`deny_unknown_fields`, testli).
  - `GET_SCREEN_TIME.measurement` yeni alanlar aldı: `reporter{kind, session_id, pid, since_ms}` ve `competing_reports`.
  - Köprü satırı 4096 bayt sınırında kalır (en büyük değerlerle testli).
- **Yetkilendirme değişmedi:**
  - Helper aynı klasördeki `kindora-helper.exe` olmalı ve Session 0'da olmamalıdır.
  - Exe yolu işletim sisteminden okunur (BUG-003 ACE düzeltmesi korunur).
  - Kimlik PID veya EXE adından alınmaz; kiralama agent'ın bağlantı kimliğine bağlıdır.
  - Rol beyanı yetki vermez; yalnız tek raporlayıcı önceliğidir.
- **Kapalı kalanlar:** kilit, açma, program kapatma, kural yazma komutu yok. Servis hesabı `LocalService` olarak kalır.
- **Kurulum betikleri:**
  - Kendini yükseltmez.
  - Yalnız Kindora servisini, `\Kindora\` görevlerini ve `C:\Program Files\KindoraDev` klasörünü değiştirir.
  - Otomatik başlatma yalnız `-EnableHelperAutostart` / `-AgentStartMode Auto` ile kurulur.
  - Veri silmek `-DeleteData` + `SIL` onayı ister.
- **Gizlilik:**
  - Toplanan tek sinyal son girdiden bu yana geçen süredir.
  - Helper kaydı yalnız olay satırları içerir: başlatma, bağlantı, kabul/beklemede, çıkış.
  - Tuş, ekran, pencere, dosya, mesaj, tarayıcı geçmişi, kamera veya mikrofon verisi yoktur.

## 5. SQLite uyumluluğu

- **Şema değişmedi (v1):** P1.C veritabanı olduğu gibi açılır; migration gerekmedi.
- Raporlayıcı olayları mevcut `lifecycle_events` tablosuna yeni `kind` değerleriyle yazılır:
  - `REPORTER_ACQUIRED`
  - `REPORTER_TAKEOVER`
  - `REPORTER_RELEASED`
- Yeniden kurulum veritabanına dokunmaz (CI: aynı veritabanı, `recovered=false`). Kaldırma veriyi korur.
- Yeniden başlatmada iki kez sayılmama P1.C mekanizmasıyla sağlanır: tek transaction'lı kayıt, ardından yalnız yeni parçalar. P1.D'de agent ve helper yeniden başlatmasıyla tekrar test edildi.

## 6. macOS testleri (2026-10-30)

| Komut | Sonuç |
|---|---|
| `cargo test --workspace` | **PASS** (ayrıntı aşağıda) |
| `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo clippy -p ipc-protocol -p agent-core -p windows-session --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | PASS (Windows'a özgü `instance.rs` / `session_end.rs` dahil) |
| Masaüstü: `cargo test` 14, `cargo clippy` | PASS |
| Ön yüz: Vitest **42**, ESLint, Prettier, vue-tsc | PASS |

`cargo test --workspace` ayrıntısı:

- `agent-core` 22 (yeni: bağlantı kimliği benzersizliği ve kapanış bildirimi)
- `ipc-protocol` 18 (rol alanı doğrulaması dahil)
- `screen-time` 21 + 12 senaryo
- `windows-agent` birim 4 (yeni: kiralama kararları; tek raporlayıcı yaşam döngüsü + çift sayım yok + konsol oturumu değişimi)
- `windows-session` 1 (tek örnek kilidi)
- e2e: `background_helper_e2e` 1 (yeni), `bridge_lifecycle` 1, `dev_transport_e2e` 6, `screen_time_e2e` 1

Yeni uçtan uca test (`crates/windows-agent/tests/background_helper_e2e.rs`) gerçek süreçlerle şunları sınar:

1. Arka plan helper'ı masaüstü olmadan raporlayıcı olur.
2. İkinci örnek 24 ms'de çıkar (`already_running`).
3. Köprü ve tek seferlik rapor reddedilir; köprü kapanınca raporlayıcı değişmez.
4. Helper sonlandırılınca kiralama yaklaşık 1,1 sn'de boşalır ve köprü devralır.
5. Yeniden başlayan helper önceki çıkışın temiz olmadığını kaydeder ve köprünün yerini alır.
6. Agent yeniden başlayınca helper 322 ms'de yeniden bağlanır. Süreler korunur, iki kez sayılmaz.

Mock / benzetim testleri gerçek Windows kabulü değildir.

## 7. Windows CI

[windows-p1d.yml](../../.github/workflows/windows-p1d.yml) şunları çalıştırır:

1. Derleme ve testler (Windows'ta `session_end` testi gerçek `WM_ENDSESSION` iletisiyle).
2. P1A–C LocalService regresyon betiği.
3. Paket oluşturma.
4. **Paketteki betiklerle** [ci/p1d-background-e2e.ps1](../../ci/p1d-background-e2e.ps1).

P1.A/B/C iş akışları regresyon olarak her push'ta çalışmaya devam eder.

Sonuçlar §9'da.

**CI'da doğrulanamayan (NOT RUN):**

- Gerçek oturum açılışı tetikleyicisi (görev elle tetiklenir)
- Oturum kapatma, hızlı kullanıcı değiştirme, Windows yeniden başlatma
- Uyku/uyanma, gerçek klavye/fare (CI'da `mouse_event` enjeksiyonu)
- Standart kullanıcı hesabı, Windows 11 Home/Pro, ekran ölçeği

## 8. Lenovo'da en kısa kabul testi

Rehber: [packaging/p1d/README-TEST-TR.md](../../packaging/p1d/README-TEST-TR.md) (D-01…D-10, isteğe bağlı D-11 uyku, D-12 kullanıcı değiştirme). Yaklaşık 25 dk.

1. Yönetici PowerShell'de üç komut: SHA-256 denetimi, ZIP'i açma, `install-kindora-test.ps1 -AgentStartMode Auto -EnableHelperAutostart`.
2. Pencere kapalıyken 5 dk kullanım.
3. Helper'ı Görev Yöneticisi'nden sonlandırma.
4. Agent'ı yeniden başlatma.
5. Oturumu kapatıp açma ve Windows'u yeniden başlatma.
6. `uninstall-kindora-test.ps1`.

## 9. Sonuçlar, yayın ve riskler

CI sonuçları, prerelease ve SHA-256 değerleri [tasks.md TASK-009](../tasks.md) kaydındadır.

**Açık riskler / sonraki faz bağımlılıkları:**

- **Kurcalama:** Aynı kullanıcı görevi devre dışı bırakabilir veya helper'ı sonlandırabilir. Bu tespit edilir (`UNKNOWN`, `REPORTER_RELEASED`, tanılama), engellenmez. Yönetici hesapta her şey değiştirilebilir (ADR-0012).
- **Görünmez pencere:** `conhost --headless` belgelenmemiş bir parametredir. Lenovo D-01 bunu doğrulamalı.
- **Kurtarma gecikmesi:** en çok N dk (5). Bu sürede süre `UNKNOWN` sayılır.
- **Kullanıcı başına süre yok:** süre cihaz düzeyindedir. Çok kullanıcılı aile senaryosu ebeveyn/eşleştirme fazına bağlıdır.
- **Uyku:** P1.C C-08 bulgusu (uyku 0, ölçülemeyen arttı) açık. Güç olayları artık kaydediliyor; doğrulanamayan uyku `SUSPENDED` sayılmaz.
- **Kurulum:** ebeveyn yetkilendirmesi, imzalı EXE ve kurucu (MSI/NSIS + görev) yok. DEV kurulum betikleri üretim kurucusu değildir.
- **Teknik borç:** TS ↔ Rust tip eşlemesi elle yapılıyor (ADR-0010).

## 10. Kapanış (2026-10-31)

**Gerçek Lenovo kabulü:** D-01…D-10, 10/10 PASS.

- Mevcut Lenovo hesabı ve `p1d-windows-test.1` kullanıldı; kullanıcı raporudur.
- D-11 (uyku) ve D-12 (kullanıcı değiştirme) isteğe bağlıydı; **NOT RUN**.
- Ayrıntılı kanıt tablosu: [TASK-009](../tasks.md).

**CI kanıtları (gerçek cihaz kabulü değildir):**

- `windows-p1a/b/c/d` iş akışları.
- P1D paket betikleri uçtan uca testi.
- P1.D.1 kurtarma yeniden üretimi: [41100377217](https://github.com/mehmetakarim/Kindora/actions/runs/41100377217). Olay 107/129 kanıtı, [NOTE-010](../solutions.md).

**P1.D.1 sonucu korunur:**

- Kurtarma bir sonraki zaman tetikleyicisinde (en çok 5 dk) gerçekleşir.
- `0x800710E0`, `IgnoreNew` reddidir.
- `RestartOnFailure` sonlandırmada etkisizdir.
- Lenovo D-05 tekrarında planlı 00:29:04 tetiklemesinden sonra 00:29:05'te yeni helper başladı.

**Açık riskler (kapanışta açık bırakıldı):**

- **Yönetici hesap:** Kullanıcının hesabı yöneticiyse servisi, görevi ve dosyaları değiştirebilir veya kaldırabilir. Bu tespit edilebilir, engellenemez ([ADR-0012](../decisions/0012-mevcut-windows-hesabinda-calisma.md)).
- **`conhost --headless` bağımlılığı:** Belgelenmemiş bir parametredir. Lenovo'da (Windows 11) ve Windows Server 2022 CI'da çalıştı; diğer sürümler doğrulanmadı.
- **Kurtarma aralığı:** Kurtarma zamanlanmış denetim aralığına bağlıdır (en çok 5 dk). Bu sürede masaüstü açıksa köprü raporlar; değilse süre `UNKNOWN` sayılır.
- **Oturum kapanışı:** `previous_exit state=unclean` gözlemi açıklanmadı ([OBS-011](../solutions.md)). Hata olarak sınıflandırılmadı.
- **Doğrulanmamış kapsamlar:** Windows Home, ekran ölçeklendirmesi (%125/%150), uyku (D-11; [NOTE-008](../solutions.md)), hızlı kullanıcı değiştirme (D-12), standart kullanıcı hesabı.
- **Teslim biçimi:** İmzalı EXE, kurucu ve ebeveyn yetkilendirmesi yok. DEV betikleri üretim kurucusu değildir.
