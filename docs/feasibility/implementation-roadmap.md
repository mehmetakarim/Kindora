# Kindora — Önerilen Uygulama Yol Haritası

> Durum: **Öneri — uygulama izni vermez.** Her faz ayrı kullanıcı onayı ister. Kapsam daraltılmadı, yeni büyük modül eklenmedi.

| Faz | Hedef | Modüller | Bağımlılık | Kabul kriterleri (özet) | Test senaryoları | Ana riskler | Teslim |
|---|---|---|---|---|---|---|---|
| P1 | Windows kullanım kilidi prototipi (PoC-A) | Minimal Rust servis + helper | T0 kabulü | [T0-A §6](T0-A-kindora-lock.md) kriterleri; ölçüm tablosu | A-01..A-12 | R-01, R-02, R-11 | PoC raporu + ADR-0006 kararı |
| P2 | Rust Service + Session Helper | `windows-agent`, `windows-session`, IPC | P1 | Otomatik başlangıç, watchdog, güvenli pipe, kurtarma | A-03, A-05, A-10, A-11, IPC yetkisiz bağlantı | R-02 | Kurulabilir agent (yerel komut) |
| P3 | Yerel politika motoru + SQLite | `policy-engine`, persistence | P2 | Çevrimdışı limit/mola, yeniden başlatma, saat sapması | [T0-D §3](T0-D-offline-policy.md) 10 senaryo | R-05, R-07, R-11 | Birim + entegrasyon testleri |
| P4 | Supabase Auth + cihaz eşleştirme | Şema, RLS, Edge Functions | P3 (paralel başlayabilir) | Eşleştirme akışı, RLS çapraz-aile testleri | R-03/R-04/R-08 negatif testleri | R-03, R-04, R-08 | Migration + RLS test paketi |
| P5 | Uçtan uca uzaktan komut | Komut kuyruğu, receipt, Realtime sinyali | P3, P4 | 7 durumlu akış, idempotency, offline kuyruk | Offline/timeout/tekrar | R-06 | Komut CLI/test istemcisi ile E2E |
| P6 | Ebeveyn Tauri/Vue arayüzü | Parent desktop, shared-contracts | P5, Bootstrap 6 doğrulaması | Phase 5 ekranları, DPI | GATE D | R-12, R-14 | Ebeveyn masaüstü MVP |
| P7 | Çocuk arayüzü + Filiz widget | Child desktop | P2, P5 | Phase 3/4 akışları, widget modları | GATE D, mola akışı | R-12 | Çocuk uygulaması MVP |
| P8 | Flutter ebeveyn uygulaması | Parent mobile | P5 (P6 ile paralel olabilir) | Aynı sözleşme/durum makinesi, push | GATE E | R-16 | Geliştirici cihazında çalışan app |
| P9 | Web sitesi engelleme | `network-filter` | P2, P3 | Alan adı engelleme Chrome/Edge, DoH/VPN sınırları belgeli | GATE C | Yeni risk kaydı gerekir | Filtre PoC + seçilen yöntem ADR |
| P10 | Raporlama, mola akışları, oyunlaştırma | Tüm istemciler | P5–P8 | MVP-16/17 | Kullanım özetleri, rozet kuralları | R-10 | MVP tamamlama |

Not: P4 bulut tarafı, P1–P3 ile paralel yürüyebilir (farklı dosyalar); paralellik kullanıcı kararıdır.
