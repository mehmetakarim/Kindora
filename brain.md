# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-29 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-007 (P1.B Windows masaüstü kabuğu)

## Git durumu (son kontrol)

- **Branch:** `main`.
- **Prerelease'ler:**
  - `a031ab7` → `p1a-windows-test.2`
  - `2fb82cf` → `p1b-windows-test.1`
- Bu belge onları izleyen `docs(p1b)` commit'indedir; güncel HEAD için `git log -1`.
- **Kimlik (depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>`; AI attribution kapalı.

## Aktif hedef

**TASK-007 (P1.B): CONDITIONAL PASS — Lenovo Windows GUI kabulü bekleniyor.**

- Kullanıcı [`p1b-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1b-windows-test.1) paketini [packaging/p1b/README-TEST-TR.md](packaging/p1b/README-TEST-TR.md) ile dener (B-01…B-11).
- P1.A.2 (TASK-006): Lenovo'da W-10 **PASS** (kullanıcı raporu).
- **P1.C başlamadı.**

## Gerçek durum (kısa)

- **Rust workspace:**
  - `ipc-protocol` (artık `bridge` sözleşmesi dahil), `agent-core`, `windows-agent` (`kindora-agent`), `windows-session` (`kindora-helper`: `run`/`bridge`/`ping`/`status`/`health`).
  - Agent komutları yine yalnız `PING`/`GET_STATUS`/`HEALTH_CHECK`. Politika değişmedi.
- **Masaüstü:**
  - [apps/desktop](apps/desktop) — Tauri 2.12 + Vue 3.5 + TS strict + Bootstrap 5.3.8 (tasarım token'ları `design/` klasöründen değiştirilmeden).
  - Agent'a doğrudan bağlanmaz. Yanındaki helper'ı `bridge` modunda stdio ile kullanır ([ADR-0013](docs/decisions/0013-masaustu-kabugu-helper-koprusu.md)).
  - Üç EXE aynı klasörde olmalı.
  - Ayrıntı: [docs/desktop/p1b-desktop-shell.md](docs/desktop/p1b-desktop-shell.md).
- **Kapsam dışı:** kilit, engelleme, overlay, otomatik başlatma, yetki yükseltme, servis kurulumu, Supabase, Flutter.
- **Ortam:**
  - macOS 27 arm64, Rust 1.94, Node 24 (`~/.nvm`; vitest 5 için Node ≥ 22.12 gerekir).
  - Windows derleme ve testleri GitHub-hosted `windows-2022` üzerinde.

## Son doğrulamalar (2026-10-28)

- **macOS:**
  - Kök 42 test, masaüstü Rust 13, Vitest 22.
  - Lint, typecheck, build, clippy.
  - Gerçek Tauri penceresi agent'a karşı elle denendi: bağlı → kopma → kurtarma; helper öldürülünce yeniden başladı; kapanışta sızıntı yok.
- **Windows CI** [p1b 40754107950](https://github.com/mehmetakarim/Kindora/actions/runs/40754107950) / [p1a 40754107939](https://github.com/mehmetakarim/Kindora/actions/runs/40754107939) — PASS:
  - 37 + 8 + 22 test, Tauri EXE + NSIS paketi.
  - LocalService + köprü testi.
  - **Gerçek pencere GUI duman testi:** servis durunca 1,9 sn'de fark etti, yeniden başlayınca 0,4 sn'de kurtardı, `WM_CLOSE` ile 0,9 sn'de çıktı, servis çalışmaya devam etti.
- **Hatalar:** [BUG-004](docs/solutions.md) (CRLF → `.gitattributes`), [BUG-005](docs/solutions.md) (.NET BOM).

## Çalıştırılmamış testler

- **Lenovo GUI kabulü B-01…B-11 — NOT RUN.** Kapsam: görsel doğruluk, %125/%150 ölçek, yönetici olmayan pencere, Görev Yöneticisi ile helper sonlandırma, klavye/fare serbestliği.
- P1A W-03/W-08/W-09/W-11 cihazda henüz tam tablo olarak raporlanmadı.

## Açık kararlar ve riskler

- ADR-0006 (M1/M2 helper başlatma), ADR-0007/0011, ADR-0013 — Önerildi. ADR-0008…0010 açık.
- Kurucu dağıtımı ve imzalama — açık (NSIS yalnız CI doğrulaması).
- Yönetici hesapta kurcalama engellenemez (ADR-0012).
- R-14 (Bootstrap 6 alfa) açık; 5.3.8 kullanıcı onayıyla kullanılıyor.
- Rust ↔ TS tipleri elle eşleniyor (ADR-0010).

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. Kullanıcıdan B-01…B-11 sonuç tablosunu al ve `docs/tasks.md` TASK-007'ye işle. Kabul gelirse durumu "Kabul edildi" yap.
2. Ancak bundan sonra, kullanıcı onayıyla P1.C.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/desktop/p1b-desktop-shell.md](docs/desktop/p1b-desktop-shell.md) · [docs/agent/p1a-windows-agent.md](docs/agent/p1a-windows-agent.md) · [docs/solutions.md](docs/solutions.md)
