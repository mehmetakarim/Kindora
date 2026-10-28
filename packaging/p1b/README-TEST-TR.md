# Kindora P1B — Masaüstü Test Paketi (ÜRETİM İÇİN DEĞİLDİR)

Bu paket Kindora masaüstü uygulamasının ilk önizlemesidir. Uygulama, Kindora Agent'ın gerçek durumunu gösterir.

- **Kısıtlama yoktur.** Ekran kilitlenmez; uygulama, site veya süre engellenmez. Tam ekran ya da kapatılamayan pencere yoktur.
- Uygulama normal bir penceredir; görev çubuğunda görünür ve **X** ile kapanır.
- Hiçbir şey otomatik kurulmaz, açılışta başlamaz veya yönetici yetkisi istemez. Servis kurulumu aşağıdaki komutlarla **sizin tarafınızdan** yapılır.
- Mevcut Windows hesabınızla test edilir. Ayrı hesap veya sanal makine gerekmez.
- EXE dosyaları imzasızdır; SmartScreen uyarısı beklenir.

Paket içeriği: `kindora-desktop.exe` (masaüstü uygulaması), `kindora-agent.exe` (servis), `kindora-helper.exe` (oturum yardımcısı), `README-TEST-TR.md`, `BUILD-INFO.txt`, `SHA256SUMS.txt`.

> **Önemli:** Üç EXE **aynı klasörde** olmalıdır. Masaüstü uygulaması agent'a doğrudan bağlanmaz; yanındaki `kindora-helper.exe`'yi başlatır. Agent yalnız kendi klasöründeki helper'ı kabul eder.

İki tür PowerShell penceresi kullanılır:

| Ad | Nasıl açılır |
|---|---|
| **Yönetici PowerShell** | Başlat → "PowerShell" → sağ tık → *Yönetici olarak çalıştır* |
| **Normal PowerShell** | Başlat → "PowerShell" (yönetici **değil**) |

PowerShell'de `sc` yerine her zaman `sc.exe` yazın. Önceki P1A testinden kalan servis veya klasör varsa önce §5'teki temizliği yapın.

---

## 1. Kurulum (yaklaşık 3 dk) — Yönetici PowerShell

ZIP'in indirildiği klasörde çalıştırın. Bu komutlar sistemde **şu değişiklikleri** yapar: `C:\Program Files\KindoraDev` klasörünü oluşturur ve `KindoraAgentDev` servisini **elle başlatılan** (`start= demand`) ve `NT AUTHORITY\LocalService` hesabıyla çalışan biçimde kaydeder. Başka bir değişiklik yapılmaz.

```powershell
(Get-FileHash .\Kindora-P1B-Windows-x64.zip -Algorithm SHA256).Hash.ToLower()   # release'teki .sha256 ile aynı olmalı
Expand-Archive .\Kindora-P1B-Windows-x64.zip -DestinationPath "C:\Program Files\KindoraDev" -Force
Get-ChildItem "C:\Program Files\KindoraDev\*.exe" | Unblock-File
sc.exe --% create KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= demand obj= "NT AUTHORITY\LocalService" DisplayName= "Kindora Agent (DEV TEST)"
sc.exe start KindoraAgentDev
```

Beklenen: `[SC] CreateService SUCCESS` ve `STATE : 4 RUNNING` (veya `2 START_PENDING`).

## 2. Masaüstü uygulamasını açın — normal kullanıcı olarak

Dosya Gezgini'nde `C:\Program Files\KindoraDev\kindora-desktop.exe` dosyasına çift tıklayın. Alternatif olarak **Normal PowerShell**'de:

```powershell
& "C:\Program Files\KindoraDev\kindora-desktop.exe"
```

**Yönetici olarak çalıştırmayın.** SmartScreen uyarı verirse ve hash doğrulandıysa "Yine de çalıştır" seçin.

## 3. Kabul kontrolleri

| ID | Adım | Beklenen |
|---|---|---|
| B-01 | Uygulama açılır | Kindora penceresi açılır, görev çubuğunda görünür. Filiz "Merhaba!" der, ardından "Kindora Agent'a bağlandım" mesajını verir. |
| B-02 | Durum kartı | **Bağlı** rozeti; Çalışma modu **Windows servisi**; Agent sürümü **0.1.0**; Agent sağlığı **Sağlıklı**; "Son başarılı iletişim" birkaç saniyede bir güncellenir. |
| B-03 | **Şimdi yenile** düğmesi | "Son başarılı iletişim" hemen "az önce" olur. |
| B-04 | Yönetici PowerShell: `sc.exe stop KindoraAgentDev` | En geç ~5 sn içinde sarı **Bağlantı yok** bandı görünür. Alanlar "Bağlantı yok" yazar. Filiz ulaşamadığını, sonra yeniden denediğini söyler. Uygulama donmaz. |
| B-05 | Yönetici PowerShell: `sc.exe start KindoraAgentDev` | En geç ~10 sn içinde kendiliğinden yeniden **Bağlı** olur. |
| B-06 | Görev Yöneticisi → Ayrıntılar → `kindora-helper.exe` → **Görevi sonlandır** | Kısa süre "Bağlantı yok" görünür. Yaklaşık 1–2 sn sonra helper yeniden başlar ve **Bağlı** olur. |
| B-07 | Pencereyi farklı boyutlara getirin, ekranı %100 / %125 / %150 ölçekte deneyin | Yatay kaydırma çubuğu veya taşan metin yok. Dar pencerede kartlar alt alta dizilir. |
| B-08 | Pencereyi **X** ile kapatın | Filiz "güvenli şekilde kapanıyor" der, pencere 1–2 sn içinde kapanır. |
| B-09 | Kapattıktan sonra Normal PowerShell: `Get-Process kindora-desktop, kindora-helper -ErrorAction SilentlyContinue` | **Boş** (artık süreç yok). `sc.exe query KindoraAgentDev` → servis hâlâ `RUNNING` (masaüstünü kapatmak servisi durdurmaz). |
| B-10 | Klavye ve fare | Uygulama açıkken Alt+Tab, Görev Yöneticisi, Başlat menüsü normal çalışır. Hiçbir giriş engellenmez. |

Yetkilendirme (isteğe bağlı) — B-11: `kindora-desktop.exe` ve `kindora-helper.exe`'yi başka bir klasöre (ör. `C:\KindoraTest\farkli`) kopyalayıp oradan açın. Beklenen: **Yetkilendirilmedi** rozeti ve Filiz'in "Agent bu bağlantıyı tanımadı" mesajı. Agent'ın güvenlik politikası gevşetilmemiştir.

## 4. Sorun olursa — kayıtlar

- Masaüstü kaydı: `%LOCALAPPDATA%\app.kindora.desktop.preview\logs\desktop.log`

  ```powershell
  Get-Content "$env:LOCALAPPDATA\app.kindora.desktop.preview\logs\desktop.log" -Tail 30
  ```

- Agent servis kaydı (Yönetici PowerShell):

  ```powershell
  Get-Content "C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log" -Tail 20
  ```

- Pencere hiç açılmıyorsa: Windows'ta **Microsoft Edge WebView2 Runtime** gerekir. Windows 11'de yüklü gelir; Windows 10'da eksikse Microsoft'un resmi sayfasından kurulabilir. Bu durumu not edin.

## 5. Kaldırma — Yönetici PowerShell

```powershell
Get-Process kindora-desktop, kindora-helper -ErrorAction SilentlyContinue | Stop-Process -Force
sc.exe stop KindoraAgentDev
sc.exe delete KindoraAgentDev
Remove-Item "C:\Program Files\KindoraDev" -Recurse -Force
Remove-Item "$env:LOCALAPPDATA\app.kindora.desktop.preview" -Recurse -Force -ErrorAction SilentlyContinue
```

Servis yoksa `sc.exe` satırları `1060` / `1062` yazar; bu normaldir. Son satır yalnız masaüstü kayıtlarını ve WebView önbelleğini siler.

## 6. Sonuç tablosu

Windows sürümü / Home-Pro / build / ekran ölçeği: ______________________

| ID | Sonuç (PASS / FAIL / NOT RUN) | Not (süre, ekran görüntüsü, hata metni) |
|---|---|---|
| B-01 | | |
| B-02 | | |
| B-03 | | |
| B-04 | | |
| B-05 | | |
| B-06 | | |
| B-07 | | |
| B-08 | | |
| B-09 | | |
| B-10 | | |
| B-11 (isteğe bağlı) | | |

Bu tabloyu ve gerekiyorsa kayıt satırlarını geliştiriciye iletin. Tablo doldurulmadan P1.B kabul edilmiş sayılmaz.
