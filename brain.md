# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-30 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-008 (P1.C kullanım süresi motoru)

## Git durumu (son kontrol)

- **Branch:** `main`.
- **Prerelease'ler:**
  - `a031ab7` → `p1a-windows-test.2`
  - `2fb82cf` → `p1b-windows-test.1`
  - `584f4d1` → `p1c-windows-test.1`
- Bu belge onları izleyen `docs(p1c)` commit'indedir; güncel HEAD için `git log -1`.
- **Kimlik (depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>`; AI attribution kapalı.

## Aktif hedef

**TASK-008 (P1.C): CONDITIONAL PASS — Pending Lenovo Screen Time Acceptance.**

- Kullanıcı [`p1c-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1c-windows-test.1) paketini [packaging/p1c/README-TEST-TR.md](packaging/p1c/README-TEST-TR.md) ile dener (C-01…C-11, DEV hızlı kurallar).
- TASK-007 (P1.B): **Lenovo GUI Acceptance PASS** (kullanıcı raporu). Son `sc delete` kabul edildi; `1060` doğrulanmadı.
- **P1.D başlamadı.**

## Gerçek durum (kısa)

- **Rust workspace:** `ipc-protocol`, `agent-core`, `screen-time` (yeni), `windows-agent`, `windows-session`.
  - Agent komutları: `PING`, `GET_STATUS`, `HEALTH_CHECK`, `REPORT_ACTIVITY` (yalnız `idle_ms`), `GET_SCREEN_TIME` (salt okunur). Kilit/kural yazma yok. Helper politikası değişmedi.
  - Ölçüm agent'ta (1 sn döngü, monoton saat, "girdi sınırı" modeli); helper `GetLastInputInfo` bildirir; kilit WTS, uyku `POWER_EVENT`.
  - SQLite (`rusqlite` 0.41.1 gömülü) LocalService profilinde, korumalı DACL, migration v1, bozuk DB kurtarma.
  - Kurallar: test varsayılanları veya agent klasöründeki `kindora-dev-policy.json` (DEV; üretim yetkilendirmesi değil).
- **Masaüstü:** [apps/desktop](apps/desktop) — Ana görünüm (Filiz, bugünkü kullanım, kalan süre, sıradaki mola, kapatılabilir bildirimler, sessiz mod) + Teknik tanılama.
- Ayrıntı: [docs/screen-time/p1c-screen-time.md](docs/screen-time/p1c-screen-time.md), [ADR-0014](docs/decisions/0014-yerel-kullanim-suresi-motoru.md).
- **Kapsam dışı:** kilit, engelleme, program kapatma, overlay, otomatik başlatma, yetki yükseltme, Supabase, Flutter.
- **Ortam:** macOS 27 arm64, Rust 1.94 (workspace `rust-version` 1.89), Node 24 (`~/.nvm`). Windows derleme/test GitHub `windows-2022`.

## Son doğrulamalar (2026-10-29)

- **macOS:** kök 82, masaüstü Rust 14, Vitest 39; lint/typecheck/build/clippy.
- **Windows CI** [p1c 40926272301](https://github.com/mehmetakarim/Kindora/actions/runs/40926272301), p1b, p1a — PASS.
  - LocalService: DEV politika, SQLite + DACL, gerçek `GetLastInputInfo` → `ACTIVE`, yeniden başlatmada 21→21 sn, bozuk DB kurtarma, bozuk politika → son geçerli.
  - GUI regresyonu: durdurma 1,5 sn'de algılandı, 0,2 sn'de kurtarma, kapanış 0,97 sn.

## Çalıştırılmamış testler

- **Lenovo C-01…C-11 — NOT RUN** (gerçek etkinlik, Win+L, uyku, mola akışı, limit, ölçek).
- macOS'ta P1.C pencere ekran görüntüsü — NOT RUN.
- Windows hedefli clippy macOS'ta SQLite'lı crate'ler için çalışmaz ([NOTE-006](docs/solutions.md)).

## Açık kararlar ve riskler

- Helper yalnız masaüstü açıkken ölçer → kapalıyken `UNKNOWN` (P1.D: oturumda helper başlatma, ADR-0006).
- Ebeveyn yetkilendirmesi yok; DEV dosyası tek kural kaynağı.
- Saat/saat dilimi değişikliği kaydedilir ama engellenmez; güvenilir zaman yok.
- ADR-0006/0007/0011/0013/0014 Önerildi; ADR-0008…0010 açık. Kurucu/imzalama açık. R-14 (Bootstrap 6 alfa).

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. Kullanıcıdan C-01…C-11 tablosunu al; TASK-008'e işle. Kabul gelirse durumu "Kabul edildi" yap.
2. Ancak bundan sonra, kullanıcı onayıyla P1.D.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/screen-time/p1c-screen-time.md](docs/screen-time/p1c-screen-time.md) · [docs/desktop/p1b-desktop-shell.md](docs/desktop/p1b-desktop-shell.md) · [docs/solutions.md](docs/solutions.md)
