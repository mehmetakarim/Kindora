# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-28 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-002 (Phase M0.2, ilk commit hazırlığı)

## Git durumu (son kontrol)

- **Depo:** Yerel depo mevcut (`git init -b main`). Bu dosya **ilk commit**in parçasıdır (kullanıcı onayı: Phase M0.2). Commit SHA'sı ve push sonucu bu commit'e yazılamadığı için `git log -1` ve `git status -sb` ile doğrulanmalı.
- **Branch:** `main` · **Referans commit:** ilk commit (`chore: initialize Kindora project memory and design foundation`).
- **Remote:** `origin` → `https://github.com/mehmetakarim/Kindora.git` — **public** (kullanıcı kararı); ilk push onaylandı (`git push -u origin main`).
- **Kimlik (yalnız depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>` (GitHub noreply); global yapılandırma değiştirilmedi.
- **AI attribution:** [.claude/settings.json](.claude/settings.json) ile kapalı; politika [AGENTS.md §6](AGENTS.md#6-git-katkıcı-contributor-politikası).

## Aktif hedef

TASK-002 — Git depo temeli ve yazar politikası. İlk commit + push onaylandı; **sonraki belge güncellemesinde SHA ve push doğrulaması işlenip görev kapatılacak.**
Uygulama geliştirme görevi yok; yeni görev yalnızca kullanıcının açık onayıyla başlar.

## Proje gerçeği (kısa)

- Proje yalnızca **tasarım ve dokümantasyondan** oluşuyor; uygulama kodu, paket, build/test komutu yok.
- Ürün bağlamı: [design/KINDORA_PROJECT_CONTEXT.md](design/KINDORA_PROJECT_CONTEXT.md) (v1.0).
- Tasarım: [design/design_handoff_kindora/](design/design_handoff_kindora/README.md) — kanonik ebeveyn paneli Phase 5 (+ Dark), çocuk Phase 3, widget/mola Phase 4.
- Teknoloji tercihleri onaylı ama **hiçbiri uygulanmadı/test edilmedi**.

## Onaylı kararlar (2026-10-27, Phase M0.1)

- **A — Flutter:** ebeveyn mobil (iOS + Android) MVP kapsamında → [ADR-0003](docs/decisions/0003-flutter-ebeveyn-mobil-mvp.md).
- **B — Kilit:** Kindora kullanım kilidi ana yöntem; Windows oturum kilidi bağımsız ek komut → [ADR-0001](docs/decisions/0001-kindora-kullanim-kilidi-ana-kontrol.md).
- **C — Claude Design:** genel UI/UX yönü kabul; Filiz nihai animasyonları, Windows entegrasyonları, erişilebilirlik testleri doğrulanmadı → [ADR-0004](docs/decisions/0004-claude-design-tasarim-yonu-kabulu.md).
- **D — Bootstrap 6:** hedef; sürüm doğrulanacak, 5.3'e geçiş kullanıcı onayı ister → [ADR-0005](docs/decisions/0005-bootstrap-6-hedef.md).

Tasarım dosyalarının tarihsel içeriği değiştirilmedi; çelişkiler bu ADR'lerle çözüldü.

## Son tamamlanan anlamlı aşama

TASK-002 hazırlığı: `.gitignore`, `.claude/settings.json`, AGENTS.md §6, ADR-0004/0005, ADR-0001/0003 güncellemeleri.

## Engeller ve açık sorunlar

- Depo **public** (kullanıcı kararı): tüm belgeler herkese açık; commit e-postası noreply.
- Açık tasarım kararları: 1280×800 @%150 desteği, 30 sn komut zaman aşımı, onboarding adım sayısı, çocuk dark tema → [docs/tasks.md açık bulgular](docs/tasks.md#kayıtlı-açık-bulgular-görev-değil--karar-veya-göreve-dönüştürülmeyi-bekliyor).

## Son doğrulamalar (2026-10-27)

- Tasarım dosyaları değişmedi (24 dosya SHA-256) — PASS
- Göreli Markdown bağlantıları — PASS
- `.gitignore` izleme/dışlama kontrolü (`git check-ignore`) — PASS
- Gizli bilgi ve büyük dosya taraması — PASS
- Commit/push — onaylandı; sonucu bu dosyada değil, `git log`/GitHub üzerinden doğrulanır

## Henüz doğrulanmamış noktalar

- GATE A–E (Windows kilidi, komut/kimlik güvenliği, web filtreleme, WebView2/DPI/widget, Flutter push/dağıtım).
- Bootstrap 6 yayın durumu; Windows sürüm hedefi; Supabase bölgesi; push sağlayıcısı.

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. İlk commit SHA'sını ve push doğrulamasını `brain.md` / `docs/tasks.md`'ye işleyip TASK-002'yi kapatmak (yeni commit kullanıcı onayı ister).
2. Teknik uygulanabilirlik ve MVP geliştirme planı (Windows kullanım kilidi, cihaz eşleştirme, çevrimdışı kural riskleri ve PoC kapsamları).

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [CLAUDE.md](CLAUDE.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/solutions.md](docs/solutions.md)
