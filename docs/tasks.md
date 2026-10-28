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

### TASK-006 — P1.A.2: LocalService IPC kimlik doğrulama düzeltmesi (BUG-003)

- **Öncelik:** Yüksek
- **Durum:** Tamamlandı (2026-10-28). Düzeltme CI'da gerçek LocalService servisiyle doğrulandı ve prerelease yayımlandı. **Gerçek Windows cihazında yeniden test yapılmadı.**
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
  - [ ] Kullanıcının cihazında W-10 (ve W-01…W-12) yeniden testi — **bekliyor**.
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
