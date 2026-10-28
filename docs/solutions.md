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

### BUG-002 — CI hash dosyaları CRLF ile yazılıyor; Linux/macOS `sha256sum -c` başarısız

- **Belirti:** Başarılı CI çalıştırması [40667002377](https://github.com/mehmetakarim/Kindora/actions/runs/40667002377) artifact'ında `shasum -a 256 -c Kindora-P1A-Windows-x64.zip.sha256` → `No such file or directory`. `od -c` dosya adının `...zip\r\n` ile bittiğini gösterdi. Aynı sorun `SHA256SUMS.txt` için de geçerliydi.
- **Ortaya çıktığı koşul:** Hash dosyaları Windows runner'da PowerShell `Set-Content` ile üretilip Linux/macOS'ta doğrulandığında (ör. prerelease işinin `sha256sum -c` adımı).
- **Kök neden:** Windows'ta `Set-Content` satır sonu olarak CRLF yazar; GNU/BSD `sha256sum -c` `\r`'yi dosya adının parçası sayar.
- **İşe yaramayan önemli denemeler:** Yok.
- **Uygulanan çözüm:** Paketleme adımında metin dosyaları `[IO.File]::WriteAllText` ile LF satır sonuyla yazılır; doğrulama adımı CR içeren hash dosyasında CI'ı başarısız kılar.
- **Doğrulama yöntemi:** Hatalı artifact macOS'ta `tr -d '\r'` ile doğrulandığında tüm hash'ler eşleşti (içerik sağlam, yalnız biçim hatalı). Düzeltme sonrası CI artifact'ı macOS'ta doğrudan `shasum -a 256 -c` ile doğrulanacak.
- **İlgili görev, dosya veya commit:** TASK-005; `.github/workflows/windows-p1a.yml`; hata commit'i `36e657c`.
