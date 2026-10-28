# Kindora — Doğrulanmış Teknik Sorunlar ve Çözümler

Yalnızca **gerçekte yaşanmış ve doğrulanmış** teknik sorunlar kaydedilir. Tasarım veya kabul bulguları buraya değil, [tasks.md](tasks.md) içindeki "Kayıtlı açık bulgular" tablosuna yazılır.

## Şablon

```markdown
### BUG-NNN — Kısa başlık
- **Belirti:**
- **Ortaya çıktığı koşul:** (OS/sürüm, cihaz, adımlar)
- **Kök neden:**
- **İşe yaramayan önemli denemeler:**
- **Uygulanan çözüm:**
- **Doğrulama yöntemi:**
- **İlgili görev, dosya veya commit:**
```

## Kayıtlar

### BUG-001 — Süreç düzeyi testler `kindora-helper` ikilisini bulamıyor (temiz derleme)

- **Belirti:** GitHub Actions `windows-2022` çalıştırması [40666717224](https://github.com/mehmetakarim/Kindora/actions/runs/40666717224): `windows_process_e2e` testi `helper_bin()` doğrulamasında panik (`windows_process_e2e.rs:32`), `cargo test` çıkış kodu 101.
- **Ortaya çıktığı koşul:** Temiz `target/` dizini (CI). `windows-agent` paketinin entegrasyon testleri, **başka bir paketin** (`windows-session`) ikilisini `target/<profil>/kindora-helper[.exe]` yolunda arıyor.
- **Kök neden:** Cargo, `cargo test` sırasında yalnız aynı paketin ikililerini entegrasyon testleri için derler (`CARGO_BIN_EXE_*`). Diğer paketin `[[bin]]` hedefi derlenmez; macOS'ta geçmesinin nedeni önceden çalıştırılmış `cargo build`'in bıraktığı ikiliydi (bayat ikili riski).
- **İşe yaramayan önemli denemeler:** Yok (ilk analizde tespit edildi).
- **Uygulanan çözüm:** CI'da `cargo test`'ten önce `cargo build --workspace --locked` adımı; test hata mesajı "önce `cargo build --workspace`" diye düzeltildi. Yerel komut sırası: `cargo build --workspace && cargo test --workspace`.
- **Doğrulama yöntemi:** macOS'ta `target/debug/kindora-helper` silinip `cargo test` → aynı panik yeniden üretildi; `cargo build` sonrası 38/38 test geçti. Windows doğrulaması: düzeltme commit'inin CI çalıştırması.
- **İlgili görev, dosya veya commit:** TASK-005; `.github/workflows/windows-p1a.yml`, `crates/windows-agent/tests/{dev_transport_e2e,windows_process_e2e}.rs`; hata commit'i `3c60d42`.
- **Not:** İleride `windows-agent` testlerinin helper'ı Cargo'nun artifact dependency özelliğiyle (kararlı hâle gelirse) derletmesi değerlendirilebilir.
