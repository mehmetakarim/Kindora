# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-28 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-006 (P1.A.2 LocalService IPC kimlik doğrulama düzeltmesi)

## Git durumu (son kontrol)

- **Branch:** `main`.
- **Prerelease commit'leri:** `17195b5` → `p1a-windows-test.1` (BUG-003'ü içerir); `a031ab7` → `p1a-windows-test.2` (BUG-003 düzeltmesi). Bu belge onu izleyen `docs(p1a)` commit'indedir; güncel HEAD için `git log -1`.
- **Kimlik (depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>`; AI attribution kapalı.

## Aktif hedef

**Aktif görev yok — kullanıcının `p1a-windows-test.2` ile gerçek Windows testi bekleniyor.**

- TASK-006 (P1.A.2) tamamlandı. Kullanıcının cihazında görülen `OpenProcess=5` / `Unauthorized(ExePathUnknown)` hatasının nedeni: helper süreç DACL'i LocalService'e izin vermiyordu ([BUG-003](docs/solutions.md)).
- Düzeltme: helper, kendi DACL'ine LocalService için yalnız `PROCESS_QUERY_LIMITED_INFORMATION` ekler. LocalSystem'e geçilmedi.
- Doğrulama: CI'da gerçek LocalService servisiyle PASS.
- Prerelease: [`p1a-windows-test.2`](https://github.com/mehmetakarim/Kindora/releases/tag/p1a-windows-test.2).

**Ürün kararı ([ADR-0012](docs/decisions/0012-mevcut-windows-hesabinda-calisma.md)):**

- Kindora mevcut Windows hesabında çalışır; ayrı çocuk hesabı zorunlu değil.
- Mevcut Lenovo hesabı gerçek test hesabıdır.
- Yönetici hesapta kurcalama engellenemez, yalnız tespit edilebilir (açık risk).

**Politika:**

- Yerel Windows VM kurulmayacak.
- CI ≠ Windows 11 Home/Pro kabulü.
- TASK-004 (P1.A) Windows kabulü bekliyor. P1.B başlamadı.

## Gerçek durum (kısa)

- Rust workspace: `ipc-protocol`, `agent-core`, `windows-agent` (`kindora-agent`), `windows-session` (`kindora-helper`).
- Yalnız `PING`/`GET_STATUS`/`HEALTH_CHECK`. Kilit, politika, SQLite, Supabase ve UI yok.
- Ayrıntı ve komutlar: [docs/agent/p1a-windows-agent.md](docs/agent/p1a-windows-agent.md); test rehberi: [packaging/p1a/README-TEST-TR.md](packaging/p1a/README-TEST-TR.md).
- Ortam: macOS 27 arm64, Rust 1.94. Windows derleme/test GitHub-hosted runner'da; kabul testi kullanıcının Windows bilgisayarında.

## Son doğrulamalar (2026-10-28)

- **macOS:** `cargo build` + `cargo test --workspace` (38 test), `clippy -D warnings` (macOS + Windows hedefi) — PASS.
- **Windows CI** [40720517755](https://github.com/mehmetakarim/Kindora/actions/runs/40720517755) — PASS:
  - 33 test, clippy, release EXE, paket/hash doğrulaması.
  - **LocalService servis uçtan uca testi** ([ci/p1a-localservice-e2e.ps1](ci/p1a-localservice-e2e.ps1), PowerShell 5.1):
    - ACE'siz istemci → `OpenProcess=5` ile reddedildi (cihaz hatasının yeniden üretimi).
    - Helper PING/STATUS/HEALTH → `exit=0`.
    - Farklı dizin → `ExeDirMismatch`.
    - Servis kuruldu, durduruldu ve silindi.
- **Prerelease:** [40720830232](https://github.com/mehmetakarim/Kindora/actions/runs/40720830232). Dosyalar macOS'ta SHA-256 ile doğrulandı. ZIP: `db0ad4dd…5489`.

## Çalıştırılmamış testler

- **Gerçek cihazda `test.2` ile W-01…W-12 NOT RUN.** `test.1` ile W-10 FAIL (BUG-003, kullanıcı raporu).
- CI'ın sınayamadıkları: W-03 (yönetici olmayan pencereden pipe), W-08 (`sc stop` reddi), W-09 (Görev Yöneticisi), W-11 (yeniden bağlanma), W-05, W-12.
- T0 PoC'leri NOT RUN.

## Açık kararlar

- ADR-0006 hesap modeli — **Önerildi**. W-10 engeli giderildi; M1/M2 helper başlatma kararı açık. LocalService izni servis SID'iyle daraltılabilir.
- ADR-0007/0011 pipe modeli — Önerildi.
- ADR-0008…0010, K-04, K-06 — açık.
- Yönetici hesapta koruma düzeyi ve bildirim akışı (ADR-0012 sonucu) — açık.
- P0 riskleri: [R-01…R-04](docs/feasibility/risk-register.md) — kapatılmadı.

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. Kullanıcı `p1a-windows-test.2` paketini Lenovo'da README'deki **"Hızlı test"** ile dener. Beklenen kayıt satırı: `os_errors=- outcome=PeerClosed`. Ardından §2–§4 ile tablo doldurulur ve `docs/tasks.md` güncellenir.
2. W-10 cihazda PASS olursa ADR-0006 güncellenir. P1.B ancak kullanıcı onayıyla başlar.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/feasibility/](docs/feasibility/README.md) · [docs/agent/p1a-windows-agent.md](docs/agent/p1a-windows-agent.md) · [docs/solutions.md](docs/solutions.md)
