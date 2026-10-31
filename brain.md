# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-31 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — P1.D resmi kapanışı + P1.E keşfi (yalnız belgeler)

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

**TASK-010 (P1.E): Planlandı.** Kapsam onaylandı (K-1…K-10, 2026-10-31). Plan: [docs/plans/p1e-proposal.md](docs/plans/p1e-proposal.md).

- P1.E ürün kodu **başlamadı**.
- Kısıtlama kodu (E.2+) için önce **E.0 kapısı** gerekir: PoC kabul kriterleri, geri alma yöntemi ve ADR-0016 kullanıcı onayıyla kesinleşir.
- İlk uygulanabilir alt görev: **E.1 — OBS-011 sağlamlaştırma** (yalnız helper; E.0'dan bağımsız).

**Kapanan aşamalar (geliştirme/test kabulü; üretim yayını değildir):**

- **TASK-009 (P1.D): Tamamlandı — Lenovo Acceptance PASS.**
  - D-01…D-10: 10/10 PASS (2026-10-31). D-11/D-12 isteğe bağlıydı, NOT RUN.
  - P1.D.1: kurtarma CI'da olay 107/129 ile kanıtlandı ([NOTE-010](docs/solutions.md)). Lenovo D-05 tekrarı PASS (00:29:04 planlı → 00:29:05 yeni PID).
- **TASK-008 (P1.C):** Lenovo temel kabul PASS.
  - C-11 NOT RUN.
  - Uyku 0, ölçülemeyen arttı ([NOTE-008](docs/solutions.md)).

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

- P1.D isteğe bağlı: D-11 (uyku), D-12 (kullanıcı değiştirme) — NOT RUN.
- Hiçbir aşamada doğrulanmayanlar: Windows Home, %125/%150 ölçek, standart kullanıcı hesabı.

## Açık kararlar ve riskler

- **Yönetici hesap:** servis, görev ve dosyalar değiştirilebilir; tespit edilir, engellenmez (ADR-0012).
- **`conhost --headless`:** belgelenmemiş bir parametreye bağımlılık.
- **Kurtarma aralığı:** zamanlanmış denetime bağlı (en çok 5 dk).
- **OBS-011:** oturum kapanışında `previous_exit state=unclean`. Hipotez H1: `ctrlc` işleyicisi `CTRL_CLOSE` olayında hemen `TRUE` dönüyor. Doğrulanmadı; BUG değil ([solutions](docs/solutions.md)).
- **ADR durumları:**
  - ADR-0013/0014/0015 **Kabul edildi** (K-8, 2026-10-31); açık riskler korunur.
  - ADR-0006/0007/0011 Önerildi; ADR-0008…0010 açık.
- **Teslim:** kurucu ve imzalama yok. R-14 (Bootstrap 6 alfa).

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

1. Kullanıcı onayıyla **E.1 (OBS-011):**
   - Windows CI'da `conhost` altında `CTRL_CLOSE` ile yeniden üret.
   - H1 doğrulanırsa konsol işleyicisini düzelt (≤2 sn bekleme).
2. Paralel olarak **E.0:** §3.2 serbest bırakma matrisi, §10 ölçütleri, §11 kurtarma yolu ve köprü rolü (K-6) için ADR-0016 taslağı → kullanıcı onayı.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/background/p1d-background-helper.md](docs/background/p1d-background-helper.md) · [docs/plans/p1e-proposal.md](docs/plans/p1e-proposal.md) · [docs/screen-time/p1c-screen-time.md](docs/screen-time/p1c-screen-time.md) · [docs/solutions.md](docs/solutions.md)
