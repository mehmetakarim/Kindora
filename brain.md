# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-28 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-005 (P1.A.1 Windows CI, test paketi)

## Git durumu (son kontrol)

- **Branch:** `main` · **HEAD:** `00764ee` (T0) = `origin/main`.
- **Çalışma ağacı:** P1.A + P1.A.1, kullanıcı onayıyla `feat(agent): add P1A Windows IPC prototype and CI build` commit'inde (bu dosya o commit'in parçası; SHA için `git log`). `target/` ignore'da.
- **Kimlik (depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>`; AI attribution kapalı.

## Aktif hedef

**TASK-005 — P1.A.1**: Windows derleme/testleri GitHub-hosted `windows-2022` üzerinde ([workflow](.github/workflows/windows-p1a.yml)); test paketi rehberi [packaging/p1a/README-TEST-TR.md](packaging/p1a/README-TEST-TR.md); prerelease `p1a-windows-test.1` yalnız elle tetiklenir. CI sonucu: GitHub Actions'da kontrol et (`gh run list --workflow windows-p1a.yml`).
**Politika:** yerel Windows VM kurulmayacak; gerçek Windows kabulü kullanıcının kendi Windows bilgisayarında yapılır. CI ≠ Windows 11 Home/Pro kabulü. TASK-004 (P1.A) Windows kabulü bekliyor. P1.B başlamadı.

## Gerçek durum (kısa)

- Rust workspace: `ipc-protocol`, `agent-core`, `windows-agent` (`kindora-agent`), `windows-session` (`kindora-helper`). Yalnız `PING`/`GET_STATUS`/`HEALTH_CHECK`. Kilit, politika, SQLite, Supabase, UI yok.
- Ayrıntı ve komutlar: [docs/agent/p1a-windows-agent.md](docs/agent/p1a-windows-agent.md).
- Ortam: macOS 27 arm64, Rust 1.94; `x86_64-pc-windows-msvc` std hedefi eklendi (yalnız check/clippy). Windows cihaz/VM yok.

## Son doğrulamalar (2026-10-28)

- macOS: `cargo build`, `cargo test --workspace` (32 birim + 6 e2e, e2e 5 tekrar), `clippy` — PASS.
- Windows hedefi: `cargo check` + `clippy --all-targets` — PASS (**yalnız derleme**).
- Ölçümler (Unix dev taşıması, Windows değil): helper SIGKILL tespiti ~12 ms; agent düzgün kapanma 12–202 ms; helper yeniden bağlanma ~240 ms.

## Çalıştırılmamış testler

Windows W-01 (MSVC bağlama) BLOCKED; W-02…W-12 (named pipe, DACL, yetkisiz exe/oturum, SCM yaşam döngüsü, standart kullanıcı `sc stop`, Görev Yöneticisi, LocalService süreç sorgusu) NOT RUN. Önceki T0 testleri (PoC-A A-01…A-12, Supabase, SQLite, Flutter) NOT RUN.

## Açık kararlar

- ADR-0006 hesap modeli — yeni girdi: LocalService süreç yolu sorgusu (W-10).
- ADR-0007/0011 pipe modeli (Interactive Users vs logon SID) — Önerildi.
- ADR-0008…0010, K-04 fail politikası, K-06 push — açık.
- Açık P0 riskleri: [R-01…R-04](docs/feasibility/risk-register.md) — kapatılmadı.

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. Son CI çalıştırmasının ve prerelease'in durumunu doğrula; SHA/URL'leri `docs/tasks.md` TASK-005'e işle.
2. Kullanıcı, prerelease paketini kendi Windows bilgisayarında [README-TEST-TR.md](packaging/p1a/README-TEST-TR.md) ile test eder; **W-10 sonucu** ADR-0006'ya girdi olur. Sonuç tablosunu `docs/tasks.md`'ye işle.
3. Ancak bundan sonra P1.B (kısıtlama yüzeyi / PoC-A) kullanıcı onayıyla.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/feasibility/](docs/feasibility/README.md) · [docs/agent/p1a-windows-agent.md](docs/agent/p1a-windows-agent.md) · [docs/solutions.md](docs/solutions.md)
