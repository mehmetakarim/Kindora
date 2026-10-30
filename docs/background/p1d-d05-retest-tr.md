# D-05 tekrar testi — arka plan yardımcısının kendiliğinden yeniden başlaması (yaklaşık 8 dk)

Mevcut `p1d-windows-test.1` kurulumu kullanılır. Yeni paket, yeniden kurulum, Windows yeniden başlatma veya yeni hesap **gerekmez**.

**Neden tekrar?** Yardımcı sonlandırılınca Windows onu hemen değil, görevin bir sonraki planlı zamanında (`NextRunTime`, en geç 5 dk) yeniden başlatır. İlk denemede kontrol bu zamandan önce yapılmış olabilir.

Bu durumda `0x800710E0` sonucu **normaldir**: "yardımcı zaten çalışıyor, ikinci kopya açılmadı" demektir.

## Adımlar — Normal PowerShell

**1. Görevin durumunu ve çalışan yardımcıyı görün:**

```powershell
$t = Get-ScheduledTask -TaskPath '\Kindora\'; Get-ScheduledTaskInfo -InputObject $t | Format-List LastRunTime, NextRunTime, LastTaskResult
Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe'" | Where-Object CommandLine -match ' activity' | Select-Object ProcessId, SessionId
```

**Beklenen:** bir `ProcessId` görünür. Bunu not edin.

**2. Yardımcıyı sonlandırın ve bir sonraki çalışma zamanını not edin:**

```powershell
Stop-Process -Id <1. adımdaki ProcessId>
Start-Sleep 5; Get-ScheduledTaskInfo -InputObject $t | Format-List LastRunTime, NextRunTime, LastTaskResult
```

`NextRunTime` saatini not edin.

**3. O saat geçtikten sonra 1 dk daha bekleyin.** Saatin geçtiğinden emin olmak için `Get-Date` yazabilirsiniz. Sonra 1. adımdaki iki komutu tekrar çalıştırın.

## Beklenen sonuç (D-05 PASS)

- Yeni ve **farklı** bir `ProcessId`.
- `LastRunTime`, 2. adımda not ettiğiniz `NextRunTime` ile aynı dakikada.
- Durum betiğinde `raporlayici=BACKGROUND_LOGON_TASK` ve yeni PID:

  ```powershell
  powershell -ExecutionPolicy Bypass -File "C:\Program Files\KindoraDev\status-kindora-test.ps1"
  ```

- Yardımcı kaydının son satırlarında `event=previous_exit state=unclean`.

Masaüstü penceresi açıksa, aradaki sürede Teknik tanılama → **Etkinlik raporlayıcısı** "Masaüstü köprüsü" gösterir. Bu normaldir.

## Sonucu iletin

1. Üç adımın çıktıları (özellikle 2. adımdaki `NextRunTime` ve 3. adımdaki yeni `ProcessId`).
2. **İsteğe bağlı:** hâlâ yeniden başlamıyorsa yönetici PowerShell'de aşağıdaki komutu çalıştırıp çıktısını iletin. Bu komut yalnız okuma yapar; Görev Zamanlayıcısı geçmişi kapalıysa boş dönebilir.

   ```powershell
   Get-WinEvent -LogName Microsoft-Windows-TaskScheduler/Operational -MaxEvents 30 -ErrorAction SilentlyContinue | Where-Object Message -like '*Kindora*' | Format-Table TimeCreated, Id -AutoSize
   ```
