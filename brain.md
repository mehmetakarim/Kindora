# brain.md — Kindora güncel session durumu

> Kısa devralma notu. Konuşma dökümü değildir. Kurallar: [AGENTS.md](AGENTS.md).

## Son güncelleme

- **Tarih:** 2026-10-28 (+03, Europe/Istanbul)
- **Güncelleyen:** Claude Code — TASK-003 kapanışı (Phase T0 final closeout)

## Git durumu (son kontrol)

- **Branch:** `main` · **Önceki referans:** `64a05ac` (ilk commit) → üzerine T0 commit'i.
- **Çalışma ağacı:** T0 belgeleri kullanıcı onayıyla `docs: complete Kindora T0 feasibility and architecture assessment` commit'inde. Bu dosya o commit'in parçası olduğundan SHA burada yazılı değil — `git log -1` ile doğrula.
- **Remote:** `origin` → `https://github.com/mehmetakarim/Kindora.git` (public).
- **Kimlik (depo-yerel):** `Mehmet Akar <11380904+mehmetakarim@users.noreply.github.com>`; AI attribution kapalı ([AGENTS.md §6](AGENTS.md#6-git-katkıcı-contributor-politikası)).

## Aktif hedef

**Aktif görev yok.** TASK-003 (Phase T0) **tamamlandı**: dokümantasyon kullanıcı tarafından kabul edildi (PASS, 2026-10-28) — yalnız plan kabulü, teknik doğrulama değil.

Sıradaki önerilen aşama: **P1 — Windows kullanım kilidi prototipi (PoC-A). Henüz başlamadı**; açık kullanıcı onayı ve Windows test ortamı gerekir.

## Proje gerçeği (kısa)

- Uygulama kodu, paket, build/test komutu **yok**. Depo: tasarım + hafıza + T0 planlama belgeleri.
- Ürün bağlamı: [design/KINDORA_PROJECT_CONTEXT.md](design/KINDORA_PROJECT_CONTEXT.md). Kabul edilmiş kararlar: ADR-0001…0005.

## Tamamlanan teknik araştırma (T0)

- [docs/feasibility/](docs/feasibility/README.md): T0-A kilit, T0-B agent, T0-C Supabase, T0-D çevrimdışı, T0-E istemci, risk kaydı (17 risk, 4×P0), 10 fazlı yol haritası.
- Önemli bulgular: helper çocuk tarafından sonlandırılabilir [Çıkarım]; `WTSQueryUserToken` yalnız servisin helper'ı kendisinin başlattığı modelde LocalSystem gerektirir; Assigned Access Home'da yok; Supabase Free 1 hafta hareketsizlikte duraklar; Bootstrap 6 = `6.0.0-alpha.1` (resmi, 8 Eki 2026).

## Açık P0 riskleri

[R-01 kilidin atlatılması, R-02 servis/helper kapanması, R-03 yetkisiz uzaktan komut, R-04 aileler arası sızıntı](docs/feasibility/risk-register.md) — hiçbiri kapatılmadı. P1 PoC-A, R-01/R-02 için ilk kanıtı üretir.

## Açık teknik/ürün kararları (kullanıcı onayı bekliyor)

- **ADR-0006** agent süreç/hesap modeli (M1 / M2 / bölünmüş) — PoC-A/B ölçümüne bağlı.
- **ADR-0007** named pipe IPC · **ADR-0008** cihaz kimliği/eşleştirme · **ADR-0009** kalıcı komut kuyruğu · **ADR-0010** JSON Schema sözleşmesi — hepsi **Önerildi**.
- **K-04** fail politikası ("son ebeveyn kuralı" + güvenli varsayılan) — [T0-D §4](docs/feasibility/T0-D-offline-policy.md).
- **K-06** push sağlayıcısı (FCM/APNs) — [T0-E §5](docs/feasibility/T0-E-client-integration.md).
- Kurcalama yedek tepkisi (helper sonlandırılınca oturum kilidi?) ebeveyn ayarı olarak mı — ADR-0006.
- Windows destek matrisi (Win11 birincil, Win10 ESU ikincil), Supabase bölgesi, kod imzalama sertifikası, PoC hedef değerleri.
- Önceki açıklar: 1280×800 @%150, 30 sn timeout, onboarding adım sayısı, çocuk dark tema ([tasks.md](docs/tasks.md#kayıtlı-açık-bulgular-görev-değil--karar-veya-göreve-dönüştürülmeyi-bekliyor)).

## Çalıştırılmamış testler (NOT RUN — PASS sayılmaz)

Gerçek Windows kullanım kilidi (PoC-A A-01…A-12) · Windows Service/Helper entegrasyonu · Supabase cihaz eşleştirme ve RLS · SQLite çevrimdışı dayanıklılık · Flutter uçtan uca komut · WebView2 DPI. Araştırma macOS'ta yapıldı.

## Son doğrulamalar (2026-10-28)

T0 belgeleri (8/8), ADR-0006…0010 (Önerildi), göreli Markdown bağlantıları, tasarım dosyası bütünlüğü (SHA-256), uygulama kodu olmaması, gizli bilgi taraması — PASS (doküman kontrolleri). Teknik testler NOT RUN.

## Sonraki somut adım (öneri — otomatik başlatma yetkisi değildir)

Yeni session ilk adımı: `git status` + `git log -2` ile T0 commit'inin `origin/main` ile eşit olduğunu doğrula; ardından kullanıcıyla **P1 hazırlığını** netleştir — Windows test ortamı (fiziksel cihaz/VM, Win11 Home + Pro, standart çocuk + yönetici ebeveyn hesabı), Rust toolchain'in nerede kurulacağı, PoC-A hedef değerleri ([T0-A §6](docs/feasibility/T0-A-kindora-lock.md)). P1 kodu kullanıcının açık onayı olmadan yazılmaz.

## İlgili belgeler

[AGENTS.md](AGENTS.md) · [CLAUDE.md](CLAUDE.md) · [docs/architecture.md](docs/architecture.md) · [docs/tasks.md](docs/tasks.md) · [docs/decisions/](docs/decisions/README.md) · [docs/feasibility/](docs/feasibility/README.md) · [docs/solutions.md](docs/solutions.md)
