# Kindora P1C — Kullanım Süresi ve Mola Test Paketi (ÜRETİM İÇİN DEĞİLDİR)

Bu paket Kindora'nın yerel kullanım süresi ölçümünü ve Filiz'in mola hatırlatmalarını dener.

- **Kısıtlama yoktur.** Ekran kilitlenmez, program kapatılmaz, internet veya site engellenmez. Süre dolduğunda yalnız bilgi verilir.
- Ölçülen tek şey "son klavye/fare girdisinden bu yana geçen süre"dir. Tuşlar, ekran görüntüsü, açık pencereler veya tarayıcı geçmişi toplanmaz. Veriler bu bilgisayarda kalır.
- Kurallar **test** kurallarıdır. Ebeveyn yetkilendirmesi henüz yoktur.
- Mevcut Windows hesabınızla test edilir. Ayrı hesap veya sanal makine gerekmez.
- EXE dosyaları imzasızdır; SmartScreen uyarısı beklenir.

Paket içeriği: `kindora-desktop.exe`, `kindora-agent.exe`, `kindora-helper.exe`, `README-TEST-TR.md`, `BUILD-INFO.txt`, `SHA256SUMS.txt`. Üç EXE **aynı klasörde** olmalıdır.

| Ad | Nasıl açılır |
|---|---|
| **Yönetici PowerShell** | Başlat → "PowerShell" → sağ tık → *Yönetici olarak çalıştır* |
| **Normal PowerShell** | Başlat → "PowerShell" (yönetici **değil**) |

Önceki P1A/P1B testinden kalan servis veya klasör varsa önce §6'daki kaldırma adımlarını uygulayın.

---

## 1. Kurulum (yaklaşık 3 dk) — Yönetici PowerShell

ZIP'in indirildiği klasörde çalıştırın. Bu komutlar `C:\Program Files\KindoraDev` klasörünü oluşturur ve `KindoraAgentDev` servisini elle başlatılan, `NT AUTHORITY\LocalService` hesabıyla çalışan biçimde kaydeder.

```powershell
(Get-FileHash .\Kindora-P1C-Windows-x64.zip -Algorithm SHA256).Hash.ToLower()   # release'teki .sha256 ile aynı olmalı
Expand-Archive .\Kindora-P1C-Windows-x64.zip -DestinationPath "C:\Program Files\KindoraDev" -Force
Get-ChildItem "C:\Program Files\KindoraDev\*.exe" | Unblock-File
```

### 1.1 Hızlı test kuralları (DEV — yalnız bu test için)

Varsayılan test kuralları 120 dk limit ve 30 dk'da bir moladır; bunları beklemek uzun sürer. Aşağıdaki **DEV test dosyası** kuralları birkaç dakikaya indirir: 6 dk limit, 3 dk aktif kullanımda bir 1 dk mola, 60 ve 20 sn kala uyarı, 60 sn boşta kalma eşiği.

```powershell
Set-Content -Encoding ASCII "C:\Program Files\KindoraDev\kindora-dev-policy.json" '{"kind":"kindora-dev-policy","dev_only":true,"version":1,"daily_limit_secs":360,"break_interval_secs":180,"break_duration_secs":60,"first_warning_secs":60,"final_warning_secs":20,"idle_threshold_secs":60,"note":"Lenovo hizli test"}'
```

- Dosya yalnız yöneticinin yazabildiği klasördedir. Agent dosyayı doğrular; bozuk veya eski sürümlü dosyayı reddeder.
- Dosyayı silmek test varsayılanlarına döndürür. Üretim varsayılanlarını değiştirmez.

### 1.2 Servisi kaydedip başlatın

```powershell
sc.exe --% create KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= demand obj= "NT AUTHORITY\LocalService" DisplayName= "Kindora Agent (DEV TEST)"
sc.exe start KindoraAgentDev
```

Beklenen: `[SC] CreateService SUCCESS` ve `STATE : 4 RUNNING`.

## 2. Masaüstü uygulamasını açın — normal kullanıcı olarak

`C:\Program Files\KindoraDev\kindora-desktop.exe` dosyasına çift tıklayın. **Yönetici olarak çalıştırmayın.**

> **Önemli:** Bu sürümde kullanıcı etkinliğini ölçen oturum yardımcısı, masaüstü uygulaması açıkken çalışır. Uygulama kapalıyken geçen süre **"ölçülemeyen"** olarak kaydedilir; aktif sayılmaz.

## 3. Kabul kontrolleri (yaklaşık 15 dk)

"Ana görünüm" sekmesinde **Bugünkü kullanım**, **Sıradaki mola** ve Filiz görünür. Teknik ayrıntılar "Teknik tanılama" sekmesindedir.

| ID | Adım | Beklenen |
|---|---|---|
| C-01 | Uygulama açılır, birkaç saniye bekleyin | Durum **Kullanılıyor**. Kurallar satırında **DEV test politikası v1**. Günlük limit **6 dk**. Filiz: "Bugün güzel ilerliyorsun!" |
| C-02 | 1–2 dk normal kullanın (fareyi oynatın, yazı yazın) | "Bugün kullanılan" artar, "Kalan süre" azalır. Sıradaki mola "… kullanım sonra" azalır. |
| C-03 | Hiç dokunmadan 2 dk bekleyin | 1 dk sonra durum **Boşta** olur. Bu sürede kullanılan süre **artmaz**. Tekrar kullanınca **Kullanılıyor** olur. |
| C-04 | Toplam ~2 dk aktif kullanım | Bildirim: "Molaya yaklaşık 1 dk kaldı." Filiz: "Birazdan kısa bir mola veriyoruz." **Kapat** düğmesi çalışır; aynı bildirim tekrar açılmaz. |
| C-05 | Kullanmaya devam edin | Filiz: "Çalışmanı kaydetmek için iyi bir zaman." Ardından: "Gözlerini dinlendirmeye ne dersin?" ve **Mola zamanı · 00:59 kaldı** geri sayımı. Hiçbir şey kilitlenmez veya kapanmaz. |
| C-06 | Geri sayım bitince | Filiz: "Hazırsan devam edebiliriz!" |
| C-07 | **Win+L** ile kilitleyin, 1 dk sonra açın | Teknik tanılama → Bugünün dağılımı: **kilitli** süresi artmış, **aktif** artmamış. |
| C-08 | Başlat → Güç → **Uyku**; 1–2 dk sonra uyandırın | Bugünün dağılımı: **uyku** süresi artmış (veya ölçülemeyen); **aktif** artmamış. |
| C-09 | Yönetici PowerShell: `sc.exe stop KindoraAgentDev`, sonra `sc.exe start KindoraAgentDev` | Durdurulunca "Bağlantı yok"; kullanım sayıları **gösterilmez**. Başlayınca önceki "Bugün kullanılan" değeri **korunur**, sıfırlanmaz ve iki katına çıkmaz. |
| C-10 | 6 dk aktif kullanıma ulaşın | "Kalan süre" **Süre doldu**. Bilgilendirme: "Bilgisayar kilitlenmez." **Kapat** ile kapanır; bilgisayar normal kullanılmaya devam eder. |

İsteğe bağlı — C-11 (bozuk kural): Yönetici PowerShell'de `Set-Content "C:\Program Files\KindoraDev\kindora-dev-policy.json" "{ bozuk"` çalıştırın. En geç ~10 sn içinde Teknik tanılama → Kurallar: "Politika dosyası bozuk — son geçerli kurallar kullanılıyor". Hiçbir şey kilitlenmez.

## 4. Kayıtlar (sorun olursa)

```powershell
# Masaüstü (Normal PowerShell)
Get-Content "$env:LOCALAPPDATA\app.kindora.desktop.preview\logs\desktop.log" -Tail 30
# Agent (Yönetici PowerShell) — kullanım süresi olayları "event=screen_time" ile başlar
Get-Content "C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log" -Tail 40
```

## 5. Veriler nerede?

SQLite veritabanı: `C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora\kindora.db`. Klasöre yalnız SYSTEM, Administrators ve agent servis hesabı erişebilir. Veriler buluta gönderilmez.

## 6. Kaldırma — Yönetici PowerShell

```powershell
Get-Process kindora-desktop, kindora-helper -ErrorAction SilentlyContinue | Stop-Process -Force
sc.exe stop KindoraAgentDev
sc.exe delete KindoraAgentDev
sc.exe query KindoraAgentDev   # 1060 = servis silindi
Remove-Item "C:\Program Files\KindoraDev" -Recurse -Force
Remove-Item "C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item "$env:LOCALAPPDATA\app.kindora.desktop.preview" -Recurse -Force -ErrorAction SilentlyContinue
```

`sc.exe query` satırının `1060` yazması servisin silindiğini doğrular.

## 7. Sonuç tablosu

Windows sürümü / Home-Pro / build / ekran ölçeği: ______________________

| ID | Sonuç (PASS / FAIL / NOT RUN) | Not (süre, ekran görüntüsü, hata metni) |
|---|---|---|
| C-01 | | |
| C-02 | | |
| C-03 | | |
| C-04 | | |
| C-05 | | |
| C-06 | | |
| C-07 | | |
| C-08 | | |
| C-09 | | |
| C-10 | | |
| C-11 (isteğe bağlı) | | |

Bu tabloyu geliştiriciye iletin. Tablo doldurulmadan P1.C kabul edilmiş sayılmaz.
