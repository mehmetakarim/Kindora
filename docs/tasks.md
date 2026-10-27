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

### TASK-002 — Git depo temeli ve yazar (attribution) politikası (Phase M0.1)

- **Öncelik:** Yüksek
- **Durum:** Devam ediyor — kullanıcı ilk commit ve push'u onayladı (Phase M0.2, 2026-10-28); push sonrası doğrulama bu commit'te kaydedilemez, sonraki belge güncellemesinde işlenecek
- **Amaç ve kapsam:** Yerel Git deposu, `origin` remote, depo-yerel Git kimliği, Claude Code attribution ayarları, `.gitignore`, Git katkıcı politikası ve onaylı ürün kararlarının (A–D) belgelere işlenmesi. Commit/push ve uygulama geliştirme kapsam dışı.
- **Kabul kriterleri:**
  - [x] `git init -b main`; var olan depo yeniden başlatılmadı (önceden depo yoktu).
  - [x] `origin` = `https://github.com/mehmetakarim/Kindora.git`; uzak depo boş olduğu doğrulandı (geçmiş yok).
  - [x] Yerel kimlik `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>` (GitHub noreply; ID 11380904 `gh api user` ile hesapla eşleşti — Phase M0.2'de kullanıcı kararıyla gmail adresinden değiştirildi); global kimlik değiştirilmedi.
  - [x] `.claude/settings.json`: `attribution.commit` / `attribution.pr` boş + `includeCoAuthoredBy: false`.
  - [x] `.gitignore`: hafıza ve tasarım dosyaları izleniyor, yerel/gizli dosyalar dışlanıyor (`git check-ignore` ile doğrulandı).
  - [x] AGENTS.md §6 Git katkıcı politikası.
  - [x] Kararlar A–D: ADR-0001/0003 güncellendi, ADR-0004/0005 eklendi.
  - [ ] İlk commit (onaylandı; bu belgeyi içeren commit — SHA sonraki belge güncellemesinde).
  - [ ] İlk push `origin/main` (onaylandı; sonuç sonraki belge güncellemesinde).
  - [x] Depo görünürlüğü: kullanıcı kararıyla **Public** kalır.
- **Bağımlılıklar:** TASK-001
- **İlgili dosyalar:** [.gitignore](../.gitignore), [.claude/settings.json](../.claude/settings.json), [AGENTS.md](../AGENTS.md), [decisions/](decisions/README.md), [architecture.md](architecture.md), [brain.md](../brain.md)
- **Doğrulama sonucu:** CONDITIONAL PASS — commit öncesi denetimler geçti; koşullar: `attribution.sessionUrl` anahtarı Claude Code 2.1.71 şemasında yok (boş `attribution.commit` şemaya göre trailer dahil tüm commit attribution metnini gizler), commit e-postası GitHub noreply adresi (hesap ID'si doğrulandı), push sonrası doğrulama henüz kayıtlı değil.
- **Yayın durumu:** Yayın yok.

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
