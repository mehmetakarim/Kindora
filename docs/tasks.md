# Kindora — Görevler

Görevlerin yürütülme, doğrulama ve yayın durumu. Ürün gereksinim envanteri (MVP-01…MVP-18) ve teknik kapılar (GATE A–E) [KINDORA_PROJECT_CONTEXT.md §16–§17](../design/KINDORA_PROJECT_CONTEXT.md#16-mvp-işlevsel-kabul-kapsamı) içindedir; bunlar görev **değildir**, kullanıcı onayıyla görevlere dönüştürülür.

**Durum değerleri:** Bekliyor · Devam ediyor · Engellendi · Tamamlandı · İptal
**Doğrulama değerleri:** PASS · CONDITIONAL PASS · FAIL · NOT RUN (`NOT RUN` asla `PASS` sayılmaz)

## Görev şablonu

```markdown
### TASK-NNN — Başlık
- **Öncelik:** Yüksek | Orta | Düşük
- **Durum:**
- **Amaç ve kapsam:**
- **Kabul kriterleri:**
- **Bağımlılıklar:**
- **İlgili dosyalar:**
- **Doğrulama sonucu:**
- **Yayın durumu:**
```

---

## Aktif / son görevler

### TASK-010 — P1.E: Yerel Kindora kullanım kısıtlaması PoC'si (ölçüm odaklı)

- **Öncelik:** Yüksek
- **Durum:** Planlandı. Kapsam kararları K-1…K-10 kullanıcı tarafından verildi (2026-10-31). **Ürün kodu başlamadı.**
- **Amaç ve kapsam:**
  - ADR-0001 Kindora kısıtlamasının yerel, geri alınabilir ve ölçülebilir ilk sürümü.
  - Kısıtlama yalnız DEV dosyasıyla uygulanır; varsayılan 10 dk, mutlak sınır 30 dk; fail-open; geçici Win32 kısıtlama ekranı.
  - Klavye kancası, LocalSystem, Windows oturum kilidi, Supabase ve uzaktan kaldırma yok.
  - Ayrıntı: [plans/p1e-proposal.md](plans/p1e-proposal.md).
- **Kabul kriterleri:** E.0'da kesinleşir. Kesinleşmeden kısıtlama kodu (E.2+) başlamaz. Taslak: plan §3.2, §9, §10, §11.
- **Alt görevler:**
  - [ ] E.0 kabul kriterleri + geri alma yöntemi + ADR-0016 (belge; kullanıcı onayı)
  - [ ] **E.1 OBS-011 sağlamlaştırma** (ilk uygulanabilir alt görev; yalnız helper)
  - [ ] E.2…E.10 (plan §8)
- **Bağımlılıklar:** TASK-009.
- **İlgili dosyalar:** `docs/plans/p1e-proposal.md`, [OBS-011](solutions.md).
- **Doğrulama sonucu:** NOT RUN.
- **Yayın durumu:** Yok.

### TASK-009 — P1.D: Bağımsız Session Helper ve güvenilir arka plan kullanım süresi

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı — **Lenovo Acceptance PASS** (D-01…D-10: 10/10, kullanıcı raporu, 2026-10-31). Bu bir geliştirme/test kabulüdür; **üretim yayını değildir**.
- **Amaç ve kapsam:**
  - Masaüstü kapalıyken de mevcut Windows oturumunda çalışan bağımsız helper ile kullanım süresi ölçümü.
  - Oturum açılışında kontrollü başlatma; güvenli kurulum ve kaldırma.
  - Kapsam dışı: zorunlu kilit, site engelleme, uygulama sonlandırma, uzaktan ebeveyn kontrolü.
- **Kabul kriterleri:**
  - [x] Helper masaüstünden bağımsız çalışıyor (`kindora-helper activity`; macOS e2e + CI).
  - [x] Masaüstü kapalıyken kullanım süresi artıyor (macOS e2e; CI: enjekte girdiyle aktif 0 → 24 sn).
  - [x] Masaüstü tekrar açıldığında gerçek toplam gösteriliyor (agent'tan; CI).
  - [x] Tek bir etkinlik raporlayıcısı kabul ediliyor:
    - agent kiralaması; köprü, tek seferlik ve ikinci arka plan helper'ı reddedilir;
    - oturum başına tek örnek.
  - [x] Agent/helper yeniden bağlantısı çalışıyor (agent yeniden başlatma, helper sonlandırma ve görevle yeniden başlatma).
  - [x] SQLite süreleri korunuyor (şema v1 değişmedi; yeniden kurulum ve kaldırma veriyi korur).
  - [x] Windows oturum başlangıcıyla helper kontrollü açılıyor. Kullanıcı başına Zamanlanmış Görev; CI'da görev elle tetiklendi; **gerçek oturum açılışı NOT RUN**.
  - [x] Kurulum ve kaldırma güvenli: idempotent; yalnız Kindora kayıtları; kendini yükseltmez; veri yalnız onayla silinir; 1060 doğrulaması.
  - [x] P1.A/B/C testleri bozulmuyor (`windows-p1a/b/c` + P1A-C betiği p1d iş akışında).
  - [x] IPC güvenliği korunuyor (yeni komut yok; helper politikası ve BUG-003 ACE düzeltmesi değişmedi).
  - [x] Windows CI başarılı.
  - [x] Prerelease ve checksum yayımlandı.
  - [x] **Lenovo arka plan ölçümü kabulü (D-01…D-10)** — 10/10 PASS (`p1d-windows-test.1`, mevcut Lenovo hesabı, kullanıcı raporu, 2026-10-31). Kabul tablosu aşağıda.
    - **D-05 geçmişi (P1.D.1):**
      - İlk Lenovo denemesinde yeniden başlama gözlenmedi. Kontrolün `NextRunTime`'dan sonra yapıldığı doğrulanmadı.
      - P1.D.1 araştırması: CI'da gerçek 5 dk'lık zaman tetikleyicisi helper'ı yeniden başlattı (olay 107/129). `0x800710E0` = "örnek zaten çalışıyor" reddi (olay 322). Yazılım hatası bulunamadı ([NOTE-010](solutions.md)).
      - CI kanıtı `2e340d3`:
        - [p1d 41100377217](https://github.com/mehmetakarim/Kindora/actions/runs/41100377217) — PASS; p1a/p1b/p1c regresyonu da PASS.
        - Akış: kill 17:53:48 → olay 107 17:58:13 → yeni helper. Olay 129'daki süreç yeni helper'ın üst süreci; olay 110 yok.
        - Raporlayıcı `DESKTOP_BRIDGE` → `BACKGROUND_LOGON_TASK`. Günlük toplam 362 sn, kurulumdan beri geçen 364 sn'yi aşmadı.
      - Tekrar rehberi: [p1d-d05-retest-tr.md](background/p1d-d05-retest-tr.md). Yeni prerelease gerekmedi.
      - Tekrar testinde D-05 PASS (aşağıdaki tablo).
- **Lenovo kabul tablosu (gerçek cihaz; CI kanıtından ayrıdır):**

  | Test | Sonuç | Kanıt (kullanıcı raporu) |
  |---|---|---|
  | D-01 | PASS | Agent LocalService / Auto / Running; bağımsız helper ve Zamanlanmış Görev |
  | D-02 | PASS | Masaüstü, arka plan raporlayıcısını ve tanılama bilgilerini gösterdi |
  | D-03 | PASS | Masaüstü kapalıyken kullanım toplamı arttı |
  | D-04 | PASS | `BACKGROUND_LOGON_TASK` önceliği; ikinci raporlar reddedildi |
  | D-05 | PASS | Helper PID 32380 sonlandırıldı. Planlı tetikleme `NextRunTime` 31.10.2026 00:29:04; `LastRunTime` 00:29:05'te yeni PID 22152 başladı; raporlayıcı `BACKGROUND_LOGON_TASK`. Günlükte `previous_exit state=unclean previous="pid=32380 …"`, `helper_started pid=22152 kind=BACKGROUND_LOGON_TASK`, `connected`, `reporter_active`. |
  | D-06 | PASS | Agent PID 26112 → 34656; helper PID 22152 korundu, `reconnects=1`; SQLite süreleri korundu |
  | D-07 | PASS | Kilitli süre 0 → 152 sn; agent ve helper çalışmaya devam etti |
  | D-08 | PASS | Oturum kapatılıp mevcut Lenovo hesabına giriş yapıldı; yeni helper PID 20380, oturum 2, `BACKGROUND_LOGON_TASK` |
  | D-09 | PASS | Windows yeniden başlatıldı; agent PID 6952, helper PID 11408; kullanım verileri korundu |
  | D-10 | PASS | Görev ve görev klasörü, program klasörü kaldırıldı; `sc.exe query KindoraAgentDev` → 1060; Kindora süreci yok; SQLite ve günlükler korundu |
  | D-11 (uyku, isteğe bağlı) | NOT RUN | Raporlanmadı |
  | D-12 (kullanıcı değiştirme, isteğe bağlı) | NOT RUN | Raporlanmadı |

- **Açık gözlem (hata olarak sınıflandırılmadı):** D-08 ve D-09'da günlükte `event=session_ending`, ardından bir sonraki açılışta `event=previous_exit state=unclean` görüldü. Kabulü engellemedi; nedeni açıklanmadı ([OBS-011](solutions.md)).
- **Bağımlılıklar:** TASK-008.
- **İlgili dosyalar:**
  - `crates/windows-session/src/{lib,main,instance,session_end}.rs`
  - `crates/windows-agent/src/{screen_time,service}.rs`, `crates/windows-agent/tests/background_helper_e2e.rs`
  - `crates/agent-core/src/{lib,handler,connection}.rs`, `crates/ipc-protocol/src/message.rs`
  - `apps/desktop/src/` (raporlayıcı tanılaması)
  - `packaging/p1d/`, `ci/p1d-background-e2e.ps1`, `.github/workflows/windows-p1d.yml`
  - Rapor: [background/p1d-background-helper.md](background/p1d-background-helper.md); karar: [ADR-0015](decisions/0015-bagimsiz-session-helper-ve-oturum-acilisi.md)
- **Doğrulama sonucu:**
  - **macOS:** kök Rust testleri, masaüstü Rust 14, Vitest 42; fmt, clippy (macOS + SQLite'sız crate'ler için Windows hedefi), ESLint, Prettier, vue-tsc — PASS. Ayrıntı: [rapor §6](background/p1d-background-helper.md#6-macos-testleri-2026-10-30).
  - **Windows CI `293e06a`:**
    - Çalıştırmalar: [p1d 41049107378](https://github.com/mehmetakarim/Kindora/actions/runs/41049107378), [p1c 41049107529](https://github.com/mehmetakarim/Kindora/actions/runs/41049107529), [p1b 41049107532](https://github.com/mehmetakarim/Kindora/actions/runs/41049107532), [p1a 41049107436](https://github.com/mehmetakarim/Kindora/actions/runs/41049107436) — PASS.
    - Windows'ta `session_end` testi gerçek `WM_ENDSESSION` ile, `background_helper_e2e` de konsol agent'la geçti.
    - **P1D betiği (paket betikleriyle, LocalService):**
      - Kurulum: servis `AUTO_START`, 3 kademeli kurtarma eylemi.
      - Görev: oturum açılışı tetikleyicisi + 1 dk zaman tetikleyicisi; `Limited`/`Interactive`; pilde çalışır; `PT0S`; `IgnoreNew`; `conhost --headless`.
      - Görev helper'ı oturum 2'de başlattı; raporlayıcı `BACKGROUND_LOGON_TASK`.
      - Masaüstü açıkken köprü reddedildi. Masaüstü kapalıyken helper çalışmayı sürdürdü (enjekte girdiyle aktif 0 → 24 sn); yeniden açınca toplam korundu.
      - İkinci örnek 53 ms'de çıktı; tek seferlik rapor `applied=false`.
      - Helper sonlandırılınca kiralama 339 ms'de boşaldı; görev helper'ı 23 sn'de yeniden başlattı (`previous_exit=unclean`).
      - Agent yeniden başlatması: 58 → 61 sn, çift sayım yok.
      - Yeniden kurulum idempotent; veritabanı korundu.
      - Kaldırma: `1060`, görev/süreç/klasör kaldırıldı, veri korundu. `-DeleteData -Yes` veriyi sildi.
    - **CI'da bulunan ve düzeltilen:** [NOTE-009](solutions.md) (görev denetimi + zaman tetikleyicisi), [BUG-006](solutions.md) (zamanlamaya bağlı test), PowerShell tek elemanlı dizi sayımı.
    - **NOT RUN:** gerçek oturum açılışı, oturum kapatma, kullanıcı değiştirme, Windows yeniden başlatma, uyku, standart kullanıcı, Windows Home.
- **Yayın durumu:**
  - Prerelease [`p1d-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1d-windows-test.1).
  - [Çalıştırma 41051052140](https://github.com/mehmetakarim/Kindora/actions/runs/41051052140); hedef `293e06a`; `latest` değil.
  - ZIP SHA-256 `1bcfa4d903c96ad169a23b210803e2879d52b85e3fca2d6f956dfbeccf768ca9` (5 034 660 bayt); macOS'ta indirilip doğrulandı.

### TASK-008 — P1.C: Yerel kullanım süresi motoru ve Filiz mola deneyimi

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı. **Kabul edildi — Lenovo temel kullanım süresi kabulü PASS** (kullanıcı raporu, P1.D mesajı, 2026-10-30).
- **Amaç ve kapsam:** Gerçek kullanım süresi ölçümü (ACTIVE/IDLE/SESSION_LOCKED/SUSPENDED/UNKNOWN), yerel SQLite, agent tarafında doğrulanan yerel kurallar (DEV test dosyası), Filiz mola hatırlatmaları. Kapsam dışı: kilit, program kapatma, internet/erişim engelleme, ebeveyn yetkilendirmesi, ayrı widget penceresi.
- **Kabul kriterleri:**
  - [x] Aktif ve boşta kalma ayrımı çalışıyor (benzetim testleri; CI'da gerçek `GetLastInputInfo` → `ACTIVE`).
  - [x] Uyku süresi aktif kullanıma eklenmiyor (benzetim; gerçek uyku CI'da yapılamaz).
  - [x] Günlük toplam tutarlı (gün bölme, saat değişikliği testleri).
  - [x] SQLite kayıtları yeniden başlatmada korunuyor (macOS e2e; CI LocalService 21 sn → 21 sn).
  - [x] Yerel politikalar doğrulanıyor (bozuk → son geçerli, eski sürüm reddi; CI'da LocalService ile).
  - [x] Kalan süre gerçek veriden hesaplanıyor (agent `GET_SCREEN_TIME`).
  - [x] Filiz doğru zamanda mola hatırlatıyor (benzetim + Vitest; tek seferlik bildirim kimlikleri).
  - [x] Limit dolunca zorunlu kilit yok (yalnız `LIMIT_REACHED` bildirimi).
  - [x] Agent bağlantı kaybı doğru yönetiliyor (bağlantı yokken kullanım verisi gösterilmez).
  - [x] Mevcut IPC güvenliği korunuyor (helper politikası değişmedi; yeni komutlar sınırlı ve test edildi).
  - [x] P1.A ve P1.B regresyonu geçiyor (`windows-p1a`, `windows-p1b` PASS).
  - [x] Windows CI başarılı.
  - [x] Prerelease yayımlandı.
  - [x] **Lenovo kullanım süresi kabulü** (`p1c-windows-test.1`, kullanıcı raporu):
    - C-01…C-10 temel kabul PASS.
    - C-11 (isteğe bağlı, bozuk kural) NOT RUN.
    - Windows uyku testinde uyku sayacı 0 kaldı, **ölçülemeyen** süre arttı; aktif süre artmadı. README C-08 bunu kabul ediyor ("uyku veya ölçülemeyen"). Neden doğrulanmadı: güç olayı gelmemiş veya helper raporu eskimiş olabilir. P1.D'de bütün güç olayları agent kaydına yazılır.
    - Agent yeniden başlatmasından sonra SQLite süreleri korundu.
    - Agent servisi STOPPED; `sc delete` sonrası `1060` doğrulandı.
    - Ayrı sonuç olarak raporlanmayanlar: %125/%150 ölçek.
- **Bağımlılıklar:** TASK-007.
- **İlgili dosyalar:** `crates/screen-time/`, `crates/windows-agent/src/{platform,screen_time,service}.rs`, `crates/windows-session/src/activity.rs`, `crates/ipc-protocol/src/{message,bridge}.rs`, `crates/agent-core/src/{lib,handler}.rs`, `apps/desktop/src/`, `ci/p1a-localservice-e2e.ps1` (`-ScreenTime`), `.github/workflows/windows-p1c.yml`, `packaging/p1c/README-TEST-TR.md`. Rapor: [screen-time/p1c-screen-time.md](screen-time/p1c-screen-time.md); karar: [ADR-0014](decisions/0014-yerel-kullanim-suresi-motoru.md).
- **Doğrulama sonucu:**
  - **macOS:** kök Rust 82 test (benzetim senaryoları 1–10, SQLite migration/kurtarma, süreç e2e), masaüstü Rust 14, Vitest 39; fmt, clippy, ESLint, Prettier, vue-tsc, build. Windows hedefli clippy yalnız SQLite'sız crate'lerde ([NOTE-006](solutions.md)). Gerçek Tauri penceresi agent'a bağlandı; görsel ekran görüntüsü NOT RUN.
  - **Windows CI** [p1c 40926272301](https://github.com/mehmetakarim/Kindora/actions/runs/40926272301) / [p1b 40926272192](https://github.com/mehmetakarim/Kindora/actions/runs/40926272192) / [p1a 40926272444](https://github.com/mehmetakarim/Kindora/actions/runs/40926272444) — PASS. Ayrıntı: [rapor §9](screen-time/p1c-screen-time.md#9-windows-ci-sonuçları).
- **Yayın durumu:** Prerelease [`p1c-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1c-windows-test.1); [çalıştırma 40928512931](https://github.com/mehmetakarim/Kindora/actions/runs/40928512931); hedef `584f4d1`; `latest` değil. ZIP SHA-256 `91c8e88035e331b21bcfbcde863c26f2a0847411a87c8662a16b531ac950868a`.

### TASK-007 — P1.B: Windows masaüstü kabuğu ve oturum deneyimi

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı. **Kabul edildi — Lenovo GUI Acceptance PASS** (kullanıcı raporu, P1.C mesajı, 2026-10-29).
- **Amaç ve kapsam:**
  - Mevcut Windows hesabında çalışan, gerçek agent bağlantısına sahip ilk Kindora masaüstü uygulaması (Tauri 2 + Vue 3).
  - Kapsam dışı: kilit, engelleme, filtreleme, süre sınırı.
- **Kabul kriterleri:**
  - [x] Windows masaüstü uygulaması derleniyor (CI: `kindora-desktop.exe` + NSIS).
  - [x] Vue arayüzü gerçek Tauri uygulamasında açılıyor (macOS'ta elle; Windows CI'da gerçek pencere, `MainWindowHandle` ≠ 0).
  - [x] Agent durumu gerçek IPC verisinden geliyor (helper köprüsü; demo yalnız Tauri dışında ve etiketli).
  - [x] LocalService güvenlik politikası korunuyor. Politika kodu değişmedi; farklı klasördeki helper reddediliyor; `OpenProcess=5` düzeltmesi CI'da yeniden doğrulandı.
  - [x] Agent kapalıyken arayüz kontrollü davranıyor ("Bağlantı yok", geri çekilme, yoğun döngü yok).
  - [x] Agent yeniden başlayınca bağlantı kurtarılıyor (CI servis: 438 ms; konsol: 242 ms).
  - [x] Desktop normal şekilde kapanıyor (CI: `WM_CLOSE` → 880 ms; helper sızıntısı yok; servis çalışmaya devam ediyor).
  - [x] Engelleyici ekran veya kullanım kilidi yok.
  - [x] Önceki P1.A testleri bozulmadı (`windows-p1a` PASS).
  - [x] CI başarılı.
  - [x] Prerelease ve checksum yayımlandı.
  - [x] Proje hafızası ve belgeler güncellendi.
  - [x] **Lenovo GUI kabulü** — PASS (kullanıcı raporu, `p1b-windows-test.1`): Windows GUI açıldı; LocalService agent bağlantısı çalıştı; gerçek agent bilgileri göründü; servis durunca "Bağlantı yok", yeniden başlayınca otomatik bağlantı; Filiz'in görsel durumu değişti; X ile kapatınca desktop ve helper süreçleri sonlandı.
  - Operasyonel not: son `sc.exe delete` isteği başarıyla kabul edildi; ardından `1060` doğrulaması yapılmadığı için servisin silindiği ayrıca kanıtlanmış sayılmaz. Kullanıcının raporladığı maddeler dışındaki B tablosu satırları (ör. %125/%150 ölçek, Görev Yöneticisi ile helper sonlandırma) ayrı sonuç olarak raporlanmadı.
- **Bağımlılıklar:** TASK-006.
- **İlgili dosyalar:** `apps/desktop/`, `crates/ipc-protocol/src/bridge.rs`, `crates/windows-session` (`bridge`), `crates/windows-agent/tests/bridge_lifecycle.rs`, `ci/p1a-localservice-e2e.ps1`, `.github/workflows/windows-p1b.yml`, `packaging/p1b/README-TEST-TR.md`, `.gitattributes`. Rapor: [desktop/p1b-desktop-shell.md](desktop/p1b-desktop-shell.md); karar: [ADR-0013](decisions/0013-masaustu-kabugu-helper-koprusu.md).
- **Doğrulama sonucu:**
  - **macOS:** kök 42 test; masaüstü Rust 13 test; Vitest 22 test; lint, typecheck, build, clippy (macOS + Windows hedefi); gerçek Tauri penceresiyle elle yaşam döngüsü — PASS.
  - **Windows CI** [40754107950](https://github.com/mehmetakarim/Kindora/actions/runs/40754107950): 37 kök + 8 masaüstü + 22 ön yüz testi, Tauri paketi, LocalService + köprü + GUI duman testi — PASS.
  - **Bulunan ve düzeltilen hatalar:** [BUG-004](solutions.md) (CRLF checkout), [BUG-005](solutions.md) (.NET BOM).
- **Yayın durumu:**
  - Prerelease [`p1b-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1b-windows-test.1).
  - [Çalıştırma 40755951911](https://github.com/mehmetakarim/Kindora/actions/runs/40755951911); hedef `2fb82cf`; `latest` değil.
  - ZIP SHA-256 `ab55cb4afcb5132ddc921650932263ea6727a184ba70b401d4b9d86eb460beae`.

### TASK-006 — P1.A.2: LocalService IPC kimlik doğrulama düzeltmesi (BUG-003)

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı (2026-10-28). CI'da ve kullanıcının Lenovo cihazında (W-10) doğrulandı.
- **Amaç ve kapsam:**
  - Kullanıcının cihazında görülen `os_errors=OpenProcess=5` / `Unauthorized(ExePathUnknown)` hatasını, LocalSystem'e geçmeden ve kimlik doğrulamayı gevşetmeden çözmek.
  - Mevcut Windows hesabı ürün kararını kaydetmek.
  - Yeni komut eklenmedi; P1.B kapsam dışı.
- **Kabul kriterleri:**
  - [x] Kök neden: kullanıcı süreçlerinin varsayılan DACL'i LocalService'e izin vermiyor ([BUG-003](solutions.md)).
  - [x] En az yetkili düzeltme: helper kendi DACL'ine LocalService için yalnız `PROCESS_QUERY_LIMITED_INFORMATION` ekliyor. Protokol, komutlar ve politika değişmedi.
  - [x] Ürün kararı: mevcut Windows hesabı, ayrı çocuk hesabı zorunlu değil ([ADR-0012](decisions/0012-mevcut-windows-hesabinda-calisma.md)). AGENTS.md ve ADR-0006/0011 güncellendi.
  - [x] macOS testleri: 38 test, clippy macOS ve Windows hedefi PASS.
  - [x] Windows CI [40720517755](https://github.com/mehmetakarim/Kindora/actions/runs/40720517755) (push, `a031ab7`):
    - 33 test PASS, clippy, release derleme.
    - LocalService servis uçtan uca testi PASS ([ci/p1a-localservice-e2e.ps1](../ci/p1a-localservice-e2e.ps1)).
  - [x] PowerShell uyumu: servis kurma/başlatma/durdurma/silme komutları (`sc.exe --%`) CI'da Windows PowerShell 5.1 ile çalıştırıldı.
  - [x] Prerelease [`p1a-windows-test.2`](https://github.com/mehmetakarim/Kindora/releases/tag/p1a-windows-test.2):
    - [Çalıştırma 40720830232](https://github.com/mehmetakarim/Kindora/actions/runs/40720830232); hedef `a031ab7`.
    - `latest` değil. macOS'ta indirilip SHA-256 doğrulandı.
  - [x] Test rehberi mevcut hesaba göre sadeleştirildi; başına "Hızlı test" bölümü eklendi.
  - [x] Kullanıcının Lenovo cihazında `p1a-windows-test.2` testi (kullanıcı raporu, P1.B mesajı): ZIP SHA-256, servis kaydı, LocalService başlatma, RUNNING, named pipe iletişimi, helper PING, exe yolu doğrulaması, `os_errors=-`, `outcome=PeerClosed requests=1 errors=0`, servis STOPPED ve silme — **PASS**. **W-10 cihazda PASS.**
- **Bağımlılıklar:** TASK-005; kullanıcının `test.1` sonucu.
- **İlgili dosyalar:**
  - `crates/ipc-protocol/src/transport/windows_pipe.rs`
  - `crates/windows-session/src/main.rs`
  - `crates/windows-agent/tests/windows_pipe_smoke.rs`
  - `ci/p1a-localservice-e2e.ps1`
  - `.github/workflows/windows-p1a.yml`
  - `packaging/p1a/README-TEST-TR.md`
- **Doğrulama sonucu:** CI'daki LocalService servis testi (runneradmin, oturum 2, PowerShell 5.1.20348):
  - ACE'siz istemci: `OpenProcess=5` → `Unauthorized(ExePathUnknown)`. Cihaz hatasının yeniden üretimi.
  - Helper PING/STATUS/HEALTH: `exit=0`, `os_errors=-`, `run_mode=SERVICE`.
  - Farklı dizindeki kopya: `ExeDirMismatch`.
  - Servis durdu ve silindi (1060).
  - Helper DACL'i önce `D:(A;;0x1fffff;;;LA)(A;;0x1fffff;;;BA)(A;;0x1fffff;;;SY)`, sonra başına `(A;;0x1000;;;LS)` eklendi.
  - **CI, cihaz kabulü değildir.**
- **Yayın durumu:**
  - Prerelease — Not for Production.
  - ZIP SHA-256 `db0ad4dd8e2ef4cf10b57d1da5aea74aaf9bdce83df20135ea7a8d7ab6c15489`.

### TASK-005 — P1.A.1: Windows CI derlemesi, test paketi, prerelease hazırlığı

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı (2026-10-28) — Windows CI ve prerelease başarılı. **Gerçek cihaz testleri (W-01…W-12) kullanıcıda bekliyor**; bu görev Windows kabulü anlamına gelmez.
- **Amaç ve kapsam:** macOS'ta yerel Windows VM kurmadan GitHub-hosted `windows-2022` üzerinde derleme/test, taşınabilir test paketi, kontrollü prerelease. P1.B (kullanım kilidi) kapsam dışı.
- **Kabul kriterleri:**
  - [x] [`.github/workflows/windows-p1a.yml`](../.github/workflows/windows-p1a.yml): push(main) + workflow_dispatch, `contents: read`, resmi eylemler commit SHA ile sabit, `--locked`.
  - [x] fmt / check / test / clippy `-D warnings` / release build / EXE varlığı / paket ve SHA-256 doğrulaması.
  - [x] Windows'a özgü güvenli smoke testleri (yönetici/servis gerektirmez) ve CI'ın neyi doğrulayamadığı belgelendi.
  - [x] Test paketi rehberi: [packaging/p1a/README-TEST-TR.md](../packaging/p1a/README-TEST-TR.md).
  - [x] Prerelease yalnız elle tetiklenir, aynı çalıştırmanın artifact'ını kullanır, `--latest=false`, `contents: write` yalnız o işte.
  - [x] Gerçek CI çalıştırması başarılı: push [40667341733](https://github.com/mehmetakarim/Kindora/actions/runs/40667341733) ve elle tetiklenen [40667562920](https://github.com/mehmetakarim/Kindora/actions/runs/40667562920) (commit `17195b5`). Önceki iki başarısız çalıştırma gerçek hatalarla düzeltildi: [BUG-001](solutions.md#bug-001--süreç-düzeyi-testler-kindora-helper-ikilisini-bulamıyor-temiz-derleme), [BUG-002](solutions.md#bug-002--ci-hash-dosyaları-crlf-ile-yazılıyor-linuxmacos-sha256sum--c-başarısız).
  - [x] Prerelease [`p1a-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1a-windows-test.1) — prerelease, `latest` değil, hedef `17195b5`; dosyalar macOS'ta indirilip SHA-256 ile doğrulandı.
  - [ ] Kullanıcının gerçek Windows cihazında W-01…W-12 — **bekliyor**.
- **Bağımlılıklar:** TASK-004
- **İlgili dosyalar:** workflow, `packaging/p1a/`, `crates/windows-agent/tests/windows_*.rs`, [agent/p1a-windows-agent.md §5a](agent/p1a-windows-agent.md)
- **Doğrulama sonucu:** Windows Server 2022 runner (`runneradmin`, oturum 2): fmt, check, **32 test** (agent-core 11, ipc-protocol 13, pipe smoke 7, süreç e2e 1), clippy `-D warnings`, release derleme, paket doğrulaması — PASS. Ölçümler: pipe kopma tespiti 0 ms, boşta kapanma 441–445 ms (400 ms ayar), `TerminateProcess` sonrası tespit ≤ 3–5 ms, ad gaspı/ikinci agent → `ERROR_ACCESS_DENIED (5)`. **CI, Windows 11 Home/Pro etkileşimli cihaz kabulünün yerine geçmez.**
- **Yayın durumu:** Prerelease — Not for Production. ZIP SHA-256 `8432f367adf62c62aa35a0cf2974e652bad122fe4f1abdf08e1d2fa0a84bcb66`.

### TASK-004 — P1.A: Windows Agent & Session Helper ilk prototipi

- **Öncelik:** Yüksek
- **Durum:** Devam ediyor — kod, macOS testleri ve Windows CI hazırlığı (TASK-005) tamam; **gerçek Windows cihaz kabulü yapılmadı**.
- **Amaç ve kapsam:** Rust workspace; servis yaşam döngüsü olan agent; kullanıcı oturumunda helper; erişim kısıtlı yerel IPC; yalnız `PING`/`GET_STATUS`/`HEALTH_CHECK`. Kullanım kilidi, politika motoru, Supabase/Flutter/Tauri, otomatik kurulum kapsam dışı.
- **Kabul kriterleri:**
  - [x] Rust workspace (`Cargo.toml`, `crates/` altında 4 crate).
  - [x] Agent (`kindora-agent`) ve helper (`kindora-helper`) ayrı çalıştırılabilir ikililer (macOS'ta çalıştırıldı).
  - [x] IPC sözleşmesi tanımlı ([P1.A raporu §3](agent/p1a-windows-agent.md)).
  - [x] Güvenli komut sınırı: yalnız 3 komut; `LOCK`/`UNLOCK`/`PROCESS_KILL` → `UNKNOWN_COMMAND` (test edildi).
  - [x] Mevcut ortamda mümkün testler çalıştırıldı: 32 birim + 6 süreç düzeyi e2e (Unix dev taşıması, 5 tekrar), clippy macOS + Windows hedefi.
  - [x] Windows'a özgü doğrulanmayan davranışlar W-01…W-12 olarak listelendi.
  - [x] Kullanıcı dosyaları korundu (tasarım dosyaları değişmedi).
  - [x] Hafıza belgeleri güncellendi.
  - [ ] Windows kabulü (W-01…W-12) — **NOT RUN / BLOCKED**.
- **Bağımlılıklar:** TASK-003; Windows test ortamı.
- **İlgili dosyalar:** [Cargo.toml](../Cargo.toml), `crates/`, [agent/p1a-windows-agent.md](agent/p1a-windows-agent.md), [ADR-0011](decisions/0011-p1a-crate-yapisi-ve-gelistirme-tasimasi.md)
- **Doğrulama sonucu:** macOS: build/test/clippy PASS; Windows hedefi: `cargo check`/`clippy` PASS (yalnız derleme). Windows çalıştırma testleri: NOT RUN; MSVC bağlama: BLOCKED. Ayrıntı: [P1.A raporu §5](agent/p1a-windows-agent.md).
- **Yayın durumu:** Kod `feat(agent): add P1A Windows IPC prototype and CI build` commit'iyle `origin/main`'e gönderilir; Windows kabulü bekliyor.

### TASK-003 — Phase T0: teknik uygulanabilirlik ve uygulama planı

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı (2026-10-28) — **yalnızca teknik araştırma ve dokümantasyon**. Uygulama ve prototip başlatılmadı.
- **Amaç ve kapsam:** Kindora kilidi, Windows Agent, Supabase güvenliği, çevrimdışı politika ve istemci entegrasyonu için araştırma, mimari öneri, risk kaydı ve faz planı. Kod, paket kurulumu ve Supabase projesi kapsam dışı; T0 belgelerinin commit/push'u kullanıcı onayıyla yapıldı.
- **Kabul kriterleri:**
  - [x] [docs/feasibility/](feasibility/README.md) altında 8 belge (T0-A…E, risk kaydı, yol haritası, README).
  - [x] ADR-0006…0010 **Önerildi** durumunda; hiçbiri kabul edilmiş sayılmadı.
  - [x] Önemli API/kota/sürüm iddiaları resmi kaynak bağlantılı; iddialar [Resmi kaynak]/[Çıkarım]/[PoC gerekli] etiketli.
  - [x] Belgeler arası bağlantılar geçerli.
  - [x] Kullanıcı kabulü: T0 dokümantasyon kararı **PASS** (GATE T0-A…E, yalnız plan kabulü; 2026-10-28).
- **Teknik araştırma (yapıldı):** Microsoft Learn (Session 0, `WTSQueryUserToken`, `LockWorkStation`, servis DACL'leri, named pipe güvenliği, low-level hook, Assigned Access), Supabase (pricing, Realtime authorization, JWT/Auth Hooks), Tauri (WebView2), SQLite (WAL), Bootstrap 6 resmi blog.
- **Prototip / entegrasyon doğrulaması (yapılmadı):**
  - Windows prototipleri (kullanım kilidi A-01…A-12, servis/helper, DPI/WebView2): **NOT RUN**
  - Supabase testleri (eşleştirme, RLS, komut kuyruğu): **NOT RUN**
  - Flutter entegrasyon testleri: **NOT RUN**
  - SQLite dayanıklılık testleri: **NOT RUN**
  - Araştırma macOS üzerinde yapıldı. [Risk kaydındaki](feasibility/risk-register.md) hiçbir risk kapatılmadı.
- **Bağımlılıklar:** TASK-002
- **İlgili dosyalar:** [feasibility/](feasibility/README.md), [risk-register.md](feasibility/risk-register.md), [implementation-roadmap.md](feasibility/implementation-roadmap.md), [decisions/](decisions/README.md)
- **Doğrulama sonucu:** Dokümantasyon PASS (8 feasibility belgesi, 5 ADR önerisi, göreli bağlantılar 0 kırık, tasarım dosyaları değişmedi, gizli bilgi yok). Teknik uygulama: NOT RUN.
- **Yayın durumu:** `docs: complete Kindora T0 feasibility and architecture assessment` commit'i ile `origin/main`'e gönderilir (SHA sonraki belge güncellemesinde/`git log` ile doğrulanır).

### TASK-002 — Git depo temeli ve yazar (attribution) politikası (Phase M0.1)

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı (2026-10-28)
- **Amaç ve kapsam:** Yerel Git deposu, `origin` remote, depo-yerel Git kimliği, Claude Code attribution ayarları, `.gitignore`, Git katkıcı politikası ve onaylı ürün kararlarının (A–D) belgelere işlenmesi. Commit/push ve uygulama geliştirme kapsam dışı.
- **Kabul kriterleri:**
  - [x] `git init -b main`; var olan depo yeniden başlatılmadı (önceden depo yoktu).
  - [x] `origin` = `https://github.com/mehmetakarim/Kindora.git`; uzak depo boş olduğu doğrulandı (geçmiş yok).
  - [x] Yerel kimlik `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>` (GitHub noreply; ID 11380904 `gh api user` ile hesapla eşleşti — Phase M0.2'de kullanıcı kararıyla gmail adresinden değiştirildi); global kimlik değiştirilmedi.
  - [x] `.claude/settings.json`: `attribution.commit` / `attribution.pr` boş + `includeCoAuthoredBy: false`.
  - [x] `.gitignore`: hafıza ve tasarım dosyaları izleniyor, yerel/gizli dosyalar dışlanıyor (`git check-ignore` ile doğrulandı).
  - [x] AGENTS.md §6 Git katkıcı politikası.
  - [x] Kararlar A–D: ADR-0001/0003 güncellendi, ADR-0004/0005 eklendi.
  - [x] İlk commit `64a05ac251cbe9737d4ec2642dd09ad24c6f1dd7` — `chore: initialize Kindora project memory and design foundation`, 38 dosya.
  - [x] İlk push `origin/main` — `git push -u origin main` başarılı; yerel `main` = `origin/main` = GitHub API `commits/main` SHA'sı.
  - [x] Depo görünürlüğü: kullanıcı kararıyla **Public** kalır.
- **Bağımlılıklar:** TASK-001
- **İlgili dosyalar:** [.gitignore](../.gitignore), [.claude/settings.json](../.claude/settings.json), [AGENTS.md](../AGENTS.md), [decisions/](decisions/README.md), [architecture.md](architecture.md), [brain.md](../brain.md)
- **Doğrulama sonucu:** PASS — author/committer `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>` (GitHub `mehmetakarim` hesabına bağlı), commit mesajında AI attribution yok, GitHub dosya listesi yerelle aynı (38). Not: commit imzasız (GitHub "Unverified" — imzalama kurulmadı); `attribution.sessionUrl` Claude Code 2.1.71 şemasında yok.
- **Yayın durumu:** GitHub `origin/main` üzerinde (public).

### TASK-001 — Kindora proje hafızası ve AI çalışma protokolü kurulumu

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı (2026-10-27)
- **Amaç ve kapsam:** Phase M0. AI araçları (Claude Code, Codex vb.) arasında bağlamı koruyan, Git ile sürümlenebilir hafıza yapısını kurmak. Uygulama kodu, bağımlılık kurulumu ve tasarım dosyası değişikliği kapsam dışı.
- **Kabul kriterleri:**
  - [x] `AGENTS.md`, `CLAUDE.md`, `brain.md`, `docs/architecture.md`, `docs/tasks.md`, `docs/decisions/README.md`, `docs/solutions.md` oluşturuldu.
  - [x] `CLAUDE.md` → `@AGENTS.md` referansı; `AGENTS.md` CLAUDE.md'yi içe aktarmıyor (döngü yok).
  - [x] Mimari belgede planlanan mimari ile gerçek durum ayrıldı; uygulanmamış bileşenler "Henüz uygulanmadı".
  - [x] Belge–tasarım çelişkileri kaydedildi, kendiliğinden kapatılmadı.
  - [x] Göreli Markdown bağlantıları geçerli.
  - [x] Tasarım dosyaları değiştirilmedi (SHA-256 öncesi/sonrası karşılaştırması).
  - [x] Hafıza dosyalarında gizli bilgi yok.
- **Bağımlılıklar:** Yok
- **İlgili dosyalar:** [AGENTS.md](../AGENTS.md), [CLAUDE.md](../CLAUDE.md), [brain.md](../brain.md), [architecture.md](architecture.md), [decisions/](decisions/README.md), [solutions.md](solutions.md)
- **Doğrulama sonucu:** CONDITIONAL PASS — dosya, bağlantı, döngü, gizli bilgi ve tasarım bütünlüğü kontrolleri geçti. Koşul: dizin git deposu değildi; Git/`.gitignore` kontrolü TASK-002'ye devredildi.
- **Yayın durumu:** Yayın yok (belge görevi; commit yapılmadı).

---

## Kayıtlı açık bulgular (görev değil — karar veya göreve dönüştürülmeyi bekliyor)

Kaynak: [Kindora Acceptance Report](../design/design_handoff_kindora/design_files/Kindora%20Acceptance%20Report.dc.html) (tasarım/teknik kabul bulguları; uygulama hatası değildir).

| ID | Bulgu | Önem | Bağlantı |
|---|---|---|---|
| OBS-01 | 1280×800 @%150 (etkin ~853 px) panel dar düzeni 972 px istiyor; yatay kaydırma kalıyor. Destek kararı gerekli. | Yüksek | GATE D, AGENTS.md §5.5 |
| OBS-02 | %125/%150 yalnızca CSS hesabıyla kontrol edildi; gerçek WebView2 ölçeklemesi test edilmedi. | Yüksek | GATE D |
| OBS-03 | 7 durumlu komut modeli prototipte yalnızca kilit komutunda; mola/ek süre/mod/kural kaydı da aynı modeli kullanmalı. | Orta | GATE B |
| OBS-04 | Çocuk uygulaması ve widget için dark tema kararı yok (öneri: MVP'de light). | Orta | — |
| OBS-05 | NVDA/Narrator ekran okuyucu testi yapılmadı. | Orta | GATE D |
| OBS-06 | Always-on-top widget'a klavye odağının nasıl geçeceği doğrulanmadı. | Orta | GATE D |
| OBS-07 | Mobil prototip eski kilit metinlerini içeriyor (birleşik kilit); mobil artık MVP kapsamında ve kilitler ayrı komut ([ADR-0001](decisions/0001-kindora-kullanim-kilidi-ana-kontrol.md), [ADR-0003](decisions/0003-flutter-ebeveyn-mobil-mvp.md)). | Orta | GATE E |
| OBS-08 | Filiz konsept çizim; nihai illüstrasyon ayrı kabul kapısında. | Düşük | Mascot Review |
