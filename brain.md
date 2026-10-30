# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-30 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-009 (P1.D bağımsız Session Helper)

## Git durumu (son kontrol)

- **Branch:** `main`.
- **Prerelease'ler:**
  - `a031ab7` → `p1a-windows-test.2`
  - `2fb82cf` → `p1b-windows-test.1`
  - `584f4d1` → `p1c-windows-test.1`
  - `293e06a` → `p1d-windows-test.1`
- Güncel HEAD için `git log -1`.
- **Kimlik (depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>`; AI attribution kapalı.

## Aktif hedef

**TASK-009 (P1.D): CONDITIONAL PASS — Pending Lenovo Background Measurement Acceptance.**

- Kullanıcı `p1d-windows-test.1` paketini [packaging/p1d/README-TEST-TR.md](packaging/p1d/README-TEST-TR.md) ile dener: `install-kindora-test.ps1 -AgentStartMode Auto -EnableHelperAutostart`, D-01…D-10.
- **TASK-008 (P1.C):** Lenovo temel kabul PASS.
  - C-01…C-10 PASS; C-11 NOT RUN.
  - Uyku 0, ölçülemeyen arttı ([NOTE-008](docs/solutions.md)).
  - Kaldırmada `1060` doğrulandı.
- **P1.D.1 (D-05 kurtarma araştırması):**
  - CI'da gerçek 5 dk zaman tetikleyicisiyle kurtarma kanıtlandı. `0x800710E0` = `IgnoreNew` reddi (olay 322).
  - Yazılım hatası yok. `RestartOnFailure`'ın etkisiz olduğu belgeye işlendi ([NOTE-010](docs/solutions.md)).
  - Yeni prerelease yok. Kullanıcı D-05'i [tekrar rehberiyle](docs/background/p1d-d05-retest-tr.md) yeniden dener.
- **P1.E başlamadı.**

## Gerçek durum (kısa)

- **Helper rolleri ([ADR-0015](docs/decisions/0015-bagimsiz-session-helper-ve-oturum-acilisi.md)):**
  - `kindora-helper activity`: arka plan raporlayıcısı; oturum başına tek örnek.
    - Kayıt: `%LOCALAPPDATA%\Kindora\helper\activity.log`.
    - Temiz olmayan çıkış izi; `WM_ENDSESSION` ile temiz kapanış.
  - `bridge`: masaüstü alt süreci; yalnız yedek raporlayıcı.
- **Agent:**
  - Tek raporlayıcı kiralaması: bağlantı kimliği, arka plan > köprü; kopma, susma veya konsol oturumu değişince bırakılır.
  - Raporlayıcı olayları SQLite'a yazılır; şema v1 değişmedi.
  - Bütün güç olayları kaydedilir.
- **Başlatma:**
  - Kullanıcı başına Zamanlanmış Görev `\Kindora\SessionHelper-<SID>`: `LeastPrivilege`, `InteractiveToken`, `conhost --headless`, N dk yeniden denetim, pilde çalışır.
  - Agent servisi `-AgentStartMode Auto` ile `auto`; her durumda kurtarma eylemleri kurulur.
  - Kurulum, kaldırma ve durum: `packaging/p1d/*.ps1`.
- **IPC:** yeni komut yok. `REPORT_ACTIVITY` isteğe bağlı `kind` aldı; `GET_SCREEN_TIME` `measurement.reporter` ve `competing_reports` aldı.
- **Masaüstü:**
  - Ana görünümde "Arka plan ölçümü açık / yalnız pencere açıkken / yok".
  - Teknik tanılamada raporlayıcı ayrıntıları.
  - Masaüstü arka plan helper'ını başlatmaz ve durdurmaz.
- **Kapsam dışı:** kilit, engelleme, program kapatma, uzaktan kontrol, ebeveyn yetkilendirmesi, Supabase, Flutter.
- **Ortam:** macOS 27 arm64, Rust 1.94 (workspace 1.89), Node 24 (`~/.nvm`). Windows: GitHub `windows-2022`.

## Son doğrulamalar (2026-10-30)

- **macOS:** kök Rust testleri (yeni: kiralama, tek raporlayıcı, bağlantı kimliği, tek örnek, `background_helper_e2e`), masaüstü Rust 14, Vitest 42; fmt, clippy (+ SQLite'sız crate'ler için Windows hedefi), ESLint, Prettier, vue-tsc — PASS.
- **Windows CI `293e06a`:** p1a/p1b/p1c/p1d PASS. P1D betiği gerçek LocalService + paket betikleriyle çalıştı: görev → helper, masaüstü kapalıyken ölçüm, tek raporlayıcı, sonlandırma → 23 sn'de yeniden başlatma, agent yeniden başlatma, idempotent kurulum, kaldırma `1060`. Ayrıntı: [TASK-009](docs/tasks.md).

## Çalıştırılmamış testler

- **Lenovo D-01…D-12 — NOT RUN.**
- CI'da doğrulanamayanlar: gerçek oturum açılışı, oturum kapatma, kullanıcı değiştirme, Windows yeniden başlatma, uyku, standart kullanıcı, Windows Home, ekran ölçeği.

## Açık kararlar ve riskler

- Aynı kullanıcı görevi devre dışı bırakabilir veya helper'ı sonlandırabilir. Bu tespit edilir, engellenmez (ADR-0012).
- `conhost --headless` belgelenmemiş bir parametredir.
- Kurtarma gecikmesi en çok 5 dk.
- Kullanıcı başına süre yok.
- Uyku nedeni doğrulanmadı ([NOTE-008](docs/solutions.md)).
- ADR-0006/0007/0011/0013/0014/0015 Önerildi; ADR-0008…0010 açık.
- Kurucu ve imzalama açık. R-14 (Bootstrap 6 alfa).

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. Kullanıcıdan D-05 tekrar sonucunu (NextRunTime sonrası yeni PID) ve kalan D tablosunu al; TASK-009'a işle.
2. D-11 yapıldıysa `power_event` satırlarıyla NOTE-008'i güncelle.
3. Kabul gelirse durumu "Kabul edildi" yap. P1.E yalnız kullanıcı onayıyla başlar.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/background/p1d-background-helper.md](docs/background/p1d-background-helper.md) · [docs/screen-time/p1c-screen-time.md](docs/screen-time/p1c-screen-time.md) · [docs/solutions.md](docs/solutions.md)
