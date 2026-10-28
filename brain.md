# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-28 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-005 (P1.A.1 Windows CI, test paketi)

## Git durumu (son kontrol)

- **Branch:** `main` · Prerelease commit'i: `17195b5` (etiket `p1a-windows-test.1`). Bu belge onu izleyen `docs(p1a)` commit'indedir; güncel HEAD için `git log -1`.
- **Commit'ler:** `3c60d42` P1.A + CI · `36e657c` BUG-001 düzeltmesi · `17195b5` BUG-002 düzeltmesi · ardından sonuç belgeleri. `target/` ignore'da.
- **Kimlik (depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>`; AI attribution kapalı.

## Aktif hedef

**Aktif görev yok — kullanıcının gerçek Windows cihaz testi bekleniyor.** TASK-005 (P1.A.1) tamamlandı: Windows CI ([workflow](.github/workflows/windows-p1a.yml)) PASS (32 test); prerelease [`p1a-windows-test.1`](https://github.com/mehmetakarim/Kindora/releases/tag/p1a-windows-test.1) yayımlandı (Not for Production); rehber [packaging/p1a/README-TEST-TR.md](packaging/p1a/README-TEST-TR.md).
**Politika:** yerel Windows VM kurulmayacak; gerçek Windows kabulü kullanıcının kendi Windows bilgisayarında yapılır. CI ≠ Windows 11 Home/Pro kabulü. TASK-004 (P1.A) Windows kabulü bekliyor. P1.B başlamadı.

## Gerçek durum (kısa)

- Rust workspace: `ipc-protocol`, `agent-core`, `windows-agent` (`kindora-agent`), `windows-session` (`kindora-helper`). Yalnız `PING`/`GET_STATUS`/`HEALTH_CHECK`. Kilit, politika, SQLite, Supabase, UI yok.
- Ayrıntı ve komutlar: [docs/agent/p1a-windows-agent.md](docs/agent/p1a-windows-agent.md).
- Ortam: macOS 27 arm64, Rust 1.94; `x86_64-pc-windows-msvc` std hedefi (yalnız check/clippy). Yerel Windows VM **kurulmayacak**; Windows derleme/test GitHub-hosted runner'da, kabul testi kullanıcının Windows bilgisayarında.

## Son doğrulamalar (2026-10-28)

- macOS: `cargo build` + `cargo test --workspace` (32 birim + 6 Unix-dev e2e), `clippy -D warnings` — PASS.
- **Windows CI** (Windows Server 2022 runner, yönetici, oturum 2): 32 test (pipe smoke 7, süreç e2e 1 dahil), clippy `-D warnings`, release EXE, paket/hash doğrulaması — PASS. Release dosyaları macOS'ta indirilip SHA-256 ile doğrulandı.
- Ölçümler (CI): pipe kopma tespiti 0 ms; `TerminateProcess` sonrası ≤ 3–5 ms; ikinci agent / ad gaspı → hata 5.

## Çalıştırılmamış testler

**Gerçek cihazda W-01…W-12 NOT RUN** (CI ile kısmen sınananlar: W-01, W-02, W-04 kısmen, W-06, W-09 — ayrıntı [P1.A raporu](docs/agent/p1a-windows-agent.md)). CI'ın sınayamadıkları: W-03 standart kullanıcı DACL, W-05, W-07/W-08/W-11 servis, **W-10 LocalService süreç sorgusu**, W-12. Önceki T0 testleri (PoC-A, Supabase, SQLite, Flutter) NOT RUN.

## Açık kararlar

- ADR-0006 hesap modeli — yeni girdi: LocalService süreç yolu sorgusu (W-10).
- ADR-0007/0011 pipe modeli (Interactive Users vs logon SID) — Önerildi.
- ADR-0008…0010, K-04 fail politikası, K-06 push — açık.
- Açık P0 riskleri: [R-01…R-04](docs/feasibility/risk-register.md) — kapatılmadı.

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. Kullanıcı, prerelease paketini kendi Windows bilgisayarında [README-TEST-TR.md](packaging/p1a/README-TEST-TR.md) ile test eder; **W-10 sonucu** ADR-0006'ya girdi olur. Sonuç tablosunu `docs/tasks.md`'ye işle.
2. Ancak bundan sonra P1.B (kısıtlama yüzeyi / PoC-A) kullanıcı onayıyla.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/feasibility/](docs/feasibility/README.md) · [docs/agent/p1a-windows-agent.md](docs/agent/p1a-windows-agent.md) · [docs/solutions.md](docs/solutions.md)
