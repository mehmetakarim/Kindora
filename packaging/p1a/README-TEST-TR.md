# Kindora P1A — Windows Test Paketi (ÜRETİM İÇİN DEĞİLDİR)

Bu paket **yalnızca IPC ve servis yaşam döngüsü prototipidir.**

- Kindora kullanım kilidi **yoktur**. Ekran kilitlenmez, uygulama kapatılmaz, internet engellenmez.
- Komutlar yalnızca `PING`, `GET_STATUS`, `HEALTH_CHECK`.
- Hiçbir şey otomatik kurulmaz, açılışta başlamaz veya kendini kalıcı hâle getirmez.
- EXE dosyaları **imzasızdır** (kod imzalama sertifikası yok).
- Gerçek çocuk verisi kullanmayın; test için ayrı Windows hesapları açın.

Paket içeriği: `kindora-agent.exe`, `kindora-helper.exe`, `README-TEST-TR.md`, `BUILD-INFO.txt` (commit ve CI çalıştırma bilgisi), `SHA256SUMS.txt`.

Sonuçları en sondaki **Sonuç tablosuna** yazın; agent ve helper çıktılarını (konsol satırları) saklayın.

---

## 0. Ortam tespiti

Yönetici PowerShell açın:

```powershell
Get-ComputerInfo -Property WindowsProductName, WindowsVersion, OsBuildNumber, OsArchitecture
whoami
whoami /groups | Select-String "S-1-5-32-544"
```

- `WindowsProductName` Home mu Pro mu? `OsArchitecture` **64 bit** olmalı (paket yalnız x64; ARM64 cihazda öykünmeyle çalışabilir ama sonuçları ayrı not edin).
- Son komut bir satır döndürüyorsa hesap **yöneticidir**.

İki hesap gerekir:

| Rol | Örnek ad | Tür |
|---|---|---|
| Ebeveyn / operatör | mevcut yönetici hesabınız | Yönetici |
| Test çocuk hesabı | `kindora-test` | **Standart kullanıcı** |

Standart hesap yoksa (yönetici PowerShell):

```powershell
$pw = Read-Host -AsSecureString "kindora-test parolası"
New-LocalUser -Name "kindora-test" -Password $pw -FullName "Kindora Test" -Description "Kindora P1A test hesabi"
Add-LocalGroupMember -Group "Users" -Member "kindora-test"
```

Testler bitince bu hesabı silebilirsiniz (bkz. §7).

## 1. Güvenli hazırlık

### 1.1 ZIP bütünlüğü

Release sayfasındaki `Kindora-P1A-Windows-x64.zip.sha256` dosyasındaki değeri karşılaştırın:

```powershell
(Get-FileHash .\Kindora-P1A-Windows-x64.zip -Algorithm SHA256).Hash.ToLower()
```

Eşleşmiyorsa **devam etmeyin**.

### 1.2 Test klasörü

```powershell
New-Item -ItemType Directory C:\KindoraTest -Force
Expand-Archive .\Kindora-P1A-Windows-x64.zip -DestinationPath C:\KindoraTest -Force
Get-ChildItem C:\KindoraTest -File | ForEach-Object { "{0}  {1}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower(), $_.Name }
Get-Content C:\KindoraTest\SHA256SUMS.txt
```

İki listeyi karşılaştırın (`SHA256SUMS.txt` kendi satırını içermez). Hash'ler eşleştikten sonra, internetten indirildiği için engellenen dosyaların engelini kaldırabilirsiniz:

```powershell
Get-ChildItem C:\KindoraTest\*.exe | Unblock-File
```

SmartScreen "Windows bilgisayarınızı korudu" uyarısı gösterirse bu imzasız prototip için beklenir; yalnız hash doğrulandıysa "Yine de çalıştır" seçin.

### 1.3 Mevcut sistem kontrolü (değişiklik yapmaz)

```powershell
sc.exe query KindoraAgentDev
Get-ChildItem "\\.\pipe\" | Where-Object Name -like "kindora*"
```

Beklenen: `1060` (servis yok) ve boş pipe listesi. Aksi hâlde önce §7 temizliğini yapın. Bu testler **yalnız** `KindoraAgentDev` adlı servisi ve `C:\KindoraTest`, `C:\Program Files\KindoraDev` klasörlerini kullanır; başka hiçbir servise dokunmayın.

## 2. İlk test — konsol modu (W-02)

Aynı (yönetici olmayan da olabilir) kullanıcı ile iki PowerShell penceresi:

**Pencere A — agent:**

```powershell
C:\KindoraTest\kindora-agent.exe console
```

Beklenen satırlar: `event=agent_started ... mode=Console`, `event=listening transport=named-pipe path=\\.\pipe\kindora-agent-v1`.

**Pencere B — helper:**

```powershell
C:\KindoraTest\kindora-helper.exe ping;   "exit=$LASTEXITCODE"
C:\KindoraTest\kindora-helper.exe status; "exit=$LASTEXITCODE"
C:\KindoraTest\kindora-helper.exe health; "exit=$LASTEXITCODE"
```

Beklenen: JSON yanıtta sırasıyla `"kind":"PONG"`, `"kind":"STATUS"`, `"kind":"HEALTH"` ve `exit=0`. Pencere A'da her biri için `event=client_connected ... session=<N> exe=C:\KindoraTest\kindora-helper.exe os_errors=-` ve `event=client_disconnected ... outcome=PeerClosed`.

Kalıcı helper:

```powershell
C:\KindoraTest\kindora-helper.exe run
```

Beklenen: `event=helper_started`, `event=connected`, `event=agent_status`. 5 sn'de bir sessiz PING. **Ctrl+C** ile kapatın → `event=helper_stopped`.

Agent'ı Pencere A'da **Ctrl+C** ile kapatın → `event=shutdown_requested`, `event=agent_stopped`.

## 3. Negatif testler (konsol modu)

Agent'ı Pencere A'da yeniden başlatın (`kindora-agent.exe console`).

| ID | Adım | Beklenen |
|---|---|---|
| W-09 | Pencere B: `kindora-helper.exe run`. Görev Yöneticisi → Ayrıntılar → `kindora-helper.exe` → **Görevi sonlandır**. Saati not edin. | Pencere A: `event=client_disconnected ... outcome=PeerClosed` hemen. `ts_ms` farkını yazın. |
| Agent kapanması | `kindora-helper.exe run` çalışırken Pencere A'da Ctrl+C. Sonra agent'ı tekrar başlatın. | Helper: `event=connection_lost`, ardından `connect_failed` denemeleri; agent dönünce yeniden `event=connected`. |
| W-04a | `Copy-Item C:\KindoraTest\kindora-helper.exe C:\KindoraTest\baska-ad.exe` → `C:\KindoraTest\baska-ad.exe ping` | `hata: unauthorized`, `exit=3`. Agent: `outcome=Unauthorized(ExeNameMismatch)` |
| W-04b | `New-Item -ItemType Directory C:\KindoraTest\farkli -Force; Copy-Item C:\KindoraTest\kindora-helper.exe C:\KindoraTest\farkli\` → `C:\KindoraTest\farkli\kindora-helper.exe ping` | `exit=3`. Agent: `Unauthorized(ExeDirMismatch)` |
| W-06 | Agent çalışırken üçüncü pencerede ikinci kez `kindora-agent.exe console` | İkinci agent hemen çıkar: `event=agent_failed error=...` (Erişim engellendi / 5). İlk agent etkilenmez. |
| Geçersiz komut / tekrar kimlik | Paket bu testler için ayrı istemci içermez; CI'da ve birim testlerinde doğrulandı (`UNKNOWN_COMMAND`, `DUPLICATE_MESSAGE_ID`). | — (gerçek cihazda **kapsam dışı**) |

Bitince helper ve agent'ı Ctrl+C ile kapatın.

## 4. Servis testleri (yönetici PowerShell, yalnız açıkça çalıştırdığınız komutlar)

### 4.1 Kurulum (W-07)

Servisin okuyabileceği, yalnız yöneticinin yazabildiği bir klasöre kopyalayın:

```powershell
New-Item -ItemType Directory "C:\Program Files\KindoraDev" -Force
Copy-Item C:\KindoraTest\kindora-agent.exe, C:\KindoraTest\kindora-helper.exe "C:\Program Files\KindoraDev\"
sc.exe create KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= demand obj= "NT AUTHORITY\LocalService" DisplayName= "Kindora Agent (DEV TEST)"
sc.exe qc KindoraAgentDev
```

`start= demand`: Windows açılışında **başlamaz**. `obj= NT AUTHORITY\LocalService`: LocalSystem **değil**.

### 4.2 Başlatma ve durum

```powershell
sc.exe start KindoraAgentDev
sc.exe query KindoraAgentDev
```

Beklenen: `STATE : 4 RUNNING`. Servis kaydı (yönetici okuyabilir):

```powershell
Get-Content "C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log" -Tail 20
```

Dosya orada değilse `Get-ChildItem C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp -Filter kindora*` ile arayın ve bulduğunuz yolu not edin.

### 4.3 Kritik W-10 ve W-03 — LocalService helper kimliğini okuyabiliyor mu?

1. Windows'tan çıkış yapmadan **Kullanıcı değiştir** ile `kindora-test` (standart) hesabına girin.
2. `kindora-test` oturumunda normal PowerShell:

   ```powershell
   & "C:\Program Files\KindoraDev\kindora-helper.exe" ping;   "exit=$LASTEXITCODE"
   & "C:\Program Files\KindoraDev\kindora-helper.exe" status; "exit=$LASTEXITCODE"
   ```

3. Yönetici hesabına dönüp servis kaydındaki son `client_connected` / `client_disconnected` satırlarını **aynen** kopyalayın.

Yorumlama:

| Helper çıktısı | Kayıt | Anlam |
|---|---|---|
| `PONG`, `exit=0` | `session=<≠0> exe=C:\Program Files\KindoraDev\kindora-helper.exe os_errors=-` | **W-03 PASS** (standart kullanıcı DACL ile bağlanabildi) + **W-10 PASS** (LocalService pid/oturum/exe okuyabildi) |
| `hata: unauthorized`, `exit=3` | `exe=- os_errors=OpenProcess=5` (veya başka kod) | **W-10 FAIL**: LocalService helper sürecini açamadı; Windows hata kodunu yazın. ADR-0006'yı etkiler. |
| `connect failed` / `Access is denied`, `exit=3` | kayıtta bağlantı yok | **W-03 FAIL**: pipe DACL'i standart kullanıcıyı engelledi; hata metnini yazın. |
| Başka bir şey | — | Tüm çıktıyı ve kayıt satırlarını not edin. |

### 4.4 Standart kullanıcı servisi durduramamalı (W-08)

`kindora-test` oturumunda:

```powershell
sc.exe stop KindoraAgentDev
```

Beklenen: `[SC] OpenService FAILED 5: Access is denied.`

### 4.5 Servis durdurulurken bağlı helper (W-11)

1. `kindora-test` oturumunda: `& "C:\Program Files\KindoraDev\kindora-helper.exe" run` (W-10 PASS ise bağlanır).
2. Yönetici oturumunda: `sc.exe stop KindoraAgentDev` → `STATE : 1 STOPPED` olmalı.
3. Helper penceresinde `event=connection_lost` ve ardından `connect_failed` görülmeli.
4. `sc.exe start KindoraAgentDev` → helper yeniden `event=connected`.
5. Helper'ı Ctrl+C ile kapatın.

### 4.6 Kontrollü durdurma ve tam kaldırma

```powershell
sc.exe stop KindoraAgentDev
sc.exe delete KindoraAgentDev
sc.exe query KindoraAgentDev        # Beklenen: 1060 (servis yok)
Remove-Item "C:\Program Files\KindoraDev" -Recurse -Force
```

## 5. Kapsam dışı / bu paketle yapılamayanlar

- **W-05 uzak istemci reddi:** ikinci bilgisayar ve ağ yapılandırması gerekir; isteğe bağlı.
- **W-12 Session 0 istemcisi:** ek araç (ör. PsExec) gerektirir; bu pakette istenmez.
- Geçersiz/tekrar mesaj testleri yalnız otomatik testlerde.
- Kullanım kilidi, mola ekranı, web engelleme: **bu sürümde yok**.

## 6. Her testten sonra kurtarma

1. Helper pencerelerinde **Ctrl+C** (veya Görev Yöneticisi → `kindora-helper.exe` → Görevi sonlandır).
2. Agent konsolunda **Ctrl+C**.
3. Servis kurduysanız §4.6.
4. Kontrol: `Get-Process kindora-* -ErrorAction SilentlyContinue` boş, `sc.exe query KindoraAgentDev` → 1060.

Bu prototip ekranı kilitlemez; bir şey takılırsa yönetici hesabıyla Görev Yöneticisi'nden süreci sonlandırmanız ve §4.6'yı uygulamanız yeterlidir.

## 7. Tam temizlik

```powershell
Get-Process kindora-* -ErrorAction SilentlyContinue | Stop-Process -Force
sc.exe stop KindoraAgentDev 2>$null; sc.exe delete KindoraAgentDev 2>$null
Remove-Item "C:\Program Files\KindoraDev" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item C:\KindoraTest -Recurse -Force -ErrorAction SilentlyContinue
# Test hesabını da silmek isterseniz:
Remove-LocalUser -Name "kindora-test"
```

## 8. Sonuç tablosu

Windows sürümü / Home-Pro / build / mimari: ______________________

| ID | Test | Sonuç (PASS / FAIL / BLOCKED / NOT RUN) | Not, süre, Windows hata kodu |
|---|---|---|---|
| W-01 | EXE'ler çalıştı (MSVC derlemesi) | | |
| W-02 | Konsol: PING / STATUS / HEALTH | | |
| W-03 | Standart kullanıcı pipe'a bağlanabildi (servis) | | |
| W-04 | Farklı ad / dizindeki exe reddedildi | | |
| W-05 | Uzak istemci reddi | | |
| W-06 | İkinci agent başlayamadı | | |
| W-07 | Servis kur / başlat / sorgula / durdur / sil | | |
| W-08 | Standart kullanıcı `sc stop` → erişim reddi | | |
| W-09 | Görev Yöneticisi ile helper sonlandırma tespit edildi | | |
| W-10 | LocalService helper pid/oturum/exe okuyabildi | | |
| W-11 | Servis durunca helper kaybı algıladı, yeniden bağlandı | | |
| W-12 | Session 0 istemcisi reddi | | |

Bu tabloyu ve kayıt satırlarını geliştiriciye iletin. Tablo doldurulmadan Windows kabulü **verilmiş sayılmaz**.
