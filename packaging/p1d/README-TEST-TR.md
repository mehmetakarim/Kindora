# Kindora P1D — Arka Plan Ölçümü Test Paketi (ÜRETİM İÇİN DEĞİLDİR)

Bu paket, Kindora penceresi **kapalıyken** de kullanım süresinin ölçüldüğünü dener.

- Ölçümü yapan küçük yardımcı (`kindora-helper.exe activity`) Windows oturumunuz açılınca kendiliğinden başlar. Başlatma, yalnız sizin hesabınız için kurulan bir **Zamanlanmış Görev** ile yapılır. Yardımcı yönetici yetkisi olmadan çalışır ve pencere açmaz.
- **Kısıtlama yoktur.** Ekran kilitlenmez, program kapatılmaz, site engellenmez.
- Ölçülen tek şey "son klavye/fare girdisinden bu yana geçen süre"dir. Tuşlar, ekran görüntüsü, pencereler ve tarayıcı geçmişi toplanmaz. Veriler bu bilgisayarda kalır.
- Mevcut Windows hesabınızla test edilir. Yeni hesap veya sanal makine gerekmez.
- EXE dosyaları imzasızdır; SmartScreen uyarısı beklenir.

**Paket içeriği:**

| Dosya | Ne işe yarar |
|---|---|
| `kindora-agent.exe`, `kindora-helper.exe`, `kindora-desktop.exe` | Uygulama dosyaları |
| `install-kindora-test.ps1` | Kurar veya günceller. Tekrar çalıştırılabilir. |
| `uninstall-kindora-test.ps1` | Kaldırır. Veriler korunur; silmek için ayrıca onay ister. |
| `status-kindora-test.ps1` | Durumu gösterir, hiçbir şeyi değiştirmez. |
| `README-TEST-TR.md`, `BUILD-INFO.txt`, `SHA256SUMS.txt` | Bu rehber, derleme bilgisi, sağlama değerleri |

Betikler yalnız Kindora kayıtlarını değiştirir:

- `KindoraAgentDev` servisi
- `\Kindora\` görev klasörü
- `C:\Program Files\KindoraDev` klasörü

Betikler kendini yönetici yetkisine **yükseltmez**. Yönetici gerekiyorsa uyarı verip durur.

**PowerShell pencereleri:**

| Ad | Nasıl açılır |
|---|---|
| **Yönetici PowerShell** | Başlat → "PowerShell" → sağ tık → *Yönetici olarak çalıştır* |
| **Normal PowerShell** | Başlat → "PowerShell" (yönetici **değil**) |

---

## 1. Kurulum (yaklaşık 3 dk) — Yönetici PowerShell

ZIP'i örneğin `İndirilenler` klasörüne indirin ve o klasörde çalıştırın:

```powershell
cd $HOME\Downloads
(Get-FileHash .\Kindora-P1D-Windows-x64.zip -Algorithm SHA256).Hash.ToLower()   # release'teki .sha256 ile aynı olmalı
Expand-Archive .\Kindora-P1D-Windows-x64.zip -DestinationPath .\Kindora-P1D -Force
Get-ChildItem .\Kindora-P1D | Unblock-File
powershell -ExecutionPolicy Bypass -File .\Kindora-P1D\install-kindora-test.ps1 -AgentStartMode Auto -EnableHelperAutostart
```

İki parametre **açık onayınızdır**:

- `-AgentStartMode Auto`: agent servisi Windows açılışında kendiliğinden başlar. Servis çökerse Windows onu 5 sn, 60 sn ve sonra 5 dk aralıklarla yeniden başlatır.
- `-EnableHelperAutostart`: yardımcı her oturum açılışında başlar. Görev 5 dk'da bir yardımcının çalıştığını da denetler; çalışıyorsa ikinci kopya açmaz.

Bu iki parametre verilmezse servis yalnız elle başlar ve oturum açılışı görevi kurulmaz. `-ExecutionPolicy Bypass` yalnız bu komut için geçerlidir; sistem ayarını değiştirmez.

**Beklenen çıktı:**

- `Servis: KindoraAgentDev calisiyor`
- `Oturum acilisi gorevi: \Kindora\SessionHelper-...`
- `Arka plan yardimcisi calisiyor: pid=...`

> Betik, ekrandaki oturumun hesabıyla yönetici PowerShell'in hesabı farklıysa durur. Böyle bir durumda doğru hesabı `-User "BILGISAYAR\kullanici"` ile verin.
>
> Önceki testten kalan kurulum varsa betik onu günceller. Kullanım süresi veritabanı **silinmez**.

## 2. Durum

Durumu istediğiniz zaman Normal veya Yönetici PowerShell'de görebilirsiniz:

```powershell
powershell -ExecutionPolicy Bypass -File "C:\Program Files\KindoraDev\status-kindora-test.ps1"
```

**Beklenen:**

- Agent servisi: `durum=Running baslatma=Auto`
- Görev satırı
- `rol=ARKA PLAN yardimcisi`
- `raporlayici=BACKGROUND_LOGON_TASK`

## 3. Kabul kontrolleri (yaklaşık 25 dk)

Masaüstü uygulaması: `C:\Program Files\KindoraDev\kindora-desktop.exe`. Yönetici olarak **açmayın**.

| ID | Adım | Beklenen |
|---|---|---|
| D-01 | Kurulumdan sonra §2 durumunu çalıştırın | Görev var. **ARKA PLAN yardimcisi** çalışıyor. `raporlayici=BACKGROUND_LOGON_TASK`. Ekranda siyah komut penceresi **yok**. |
| D-02 | Masaüstü uygulamasını açın | Ana görünümde **"Arka plan ölçümü açık"**. Teknik tanılama → **Etkinlik raporlayıcısı:** "Arka plan yardımcısı (oturum açılışı görevi) · PID … · oturum …". |
| D-03 | "Bugün kullanılan" değerini not edin, pencereyi **kapatın**. 5 dk normal kullanın (tarayıcı, yazı). Sonra uygulamayı yeniden açın. | "Bugün kullanılan" yaklaşık 5 dk **artmış**. §2'de yardımcının PID'i aynı. |
| D-04 | Uygulama açıkken Teknik tanılama → **Reddedilen ikinci raporlayıcı** satırına bakın | Sayı artar ("yalnız biri sayılır"). Raporlayıcı yine arka plan yardımcısıdır. |
| D-05 | Görev Yöneticisi → Ayrıntılar → `kindora-helper.exe`. İki satır olabilir: biri pencerenin köprüsüdür. §2'de yazan **arka plan** PID'ini seçip *Görevi sonlandır* deyin. 6 dk bekleyin. | Önce Teknik tanılama → raporlayıcı **Masaüstü köprüsü** veya **Yok**. 6 dk içinde görev yardımcıyı yeniden başlatır ve raporlayıcı yine arka plan yardımcısı olur. §2 kaydında `previous_exit state=unclean`. |
| D-06 | Yönetici PowerShell: `sc.exe stop KindoraAgentDev`, 10 sn sonra `sc.exe start KindoraAgentDev` | Durdurulunca "Bağlantı yok". Başlayınca kullanım değeri **korunur**, iki katına çıkmaz. Yardımcı kendiliğinden yeniden bağlanır. |
| D-07 | Win+L ile kilitleyin, 1 dk sonra açın | Teknik tanılama → Bugünün dağılımı: **kilitli** artar, **aktif** artmaz. |
| D-08 | Başlat → hesap → **Oturumu kapat**. Tekrar giriş yapın. Uygulamayı **açmadan** 3 dk kullanın, sonra §2. | `raporlayici=BACKGROUND_LOGON_TASK`, yeni PID, aktif süre artmış. |
| D-09 | Windows'u **yeniden başlatın**. Giriş yapın, uygulamayı açmadan 3 dk kullanın, sonra §2. | Servis `Running` (Auto). Yardımcı çalışıyor. Aktif süre artmış. Önceki kayıtlar korunmuş. |
| D-10 | **Kaldırma** (§5, verileri koruyarak) | `1060 ... servis silindi`. §2: görev yok, süreç yok. |

**İsteğe bağlı:**

- **D-11 — Uyku:** Başlat → Güç → Uyku; 2 dk sonra uyandırın.
  - Beklenen: aktif artmaz; uyku veya ölçülemeyen artar.
  - Yönetici PowerShell'de `Select-String power_event "C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log"` çıktısını iletin. Bu, P1.C'deki "uyku 0, ölçülemeyen arttı" bulgusunun tanısı içindir.
- **D-12 — Kullanıcı değiştirme:** Bilgisayarda zaten başka bir hesap varsa hızlı kullanıcı değiştirme deneyin.
  - Beklenen: yalnız ekrandaki oturumun süresi sayılır.
  - Yeni hesap **oluşturmayın**.

## 4. Kayıtlar (sorun olursa)

```powershell
# Yardımcı (Normal PowerShell; sizin profiliniz)
Get-Content "$env:LOCALAPPDATA\Kindora\helper\activity.log" -Tail 30
# Masaüstü
Get-Content "$env:LOCALAPPDATA\app.kindora.desktop.preview\logs\desktop.log" -Tail 30
# Agent (Yönetici PowerShell)
Get-Content "C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log" -Tail 40
```

## 5. Kaldırma — Yönetici PowerShell

```powershell
cd $HOME\Downloads
powershell -ExecutionPolicy Bypass -File .\Kindora-P1D\uninstall-kindora-test.ps1
```

**Kaldırılanlar (veriler korunur):**

- Oturum açılışı görevi
- Çalışan Kindora süreçleri
- `KindoraAgentDev` servisi (sonunda `sc.exe query` → `1060` doğrulanır)
- `C:\Program Files\KindoraDev` klasörü

**Verileri de silmek** için `-DeleteData` ekleyin. Betik onay için `SIL` yazmanızı ister. Silinenler:

- `C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora` — kullanım süresi veritabanı (P1.C'den kalan dahil)
- Kendi profilinizdeki yardımcı kaydı ve masaüstü ayarları

Betik tekrar çalıştırılabilir; kurulum yoksa hata vermez.

## 6. Sonuç tablosu

Windows sürümü / Home-Pro / build / ekran ölçeği: ______________________

| ID | Sonuç (PASS / FAIL / NOT RUN) | Not (süre, ekran görüntüsü, hata metni) |
|---|---|---|
| D-01 | | |
| D-02 | | |
| D-03 | | |
| D-04 | | |
| D-05 | | |
| D-06 | | |
| D-07 | | |
| D-08 | | |
| D-09 | | |
| D-10 | | |
| D-11 (isteğe bağlı) | | |
| D-12 (isteğe bağlı) | | |

Bu tabloyu ve §2 çıktısını geliştiriciye iletin. Tablo doldurulmadan P1.D kabul edilmiş sayılmaz.
