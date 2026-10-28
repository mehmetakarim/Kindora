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
