# Kindora P1D - read-only status of the DEV test installation.
# Changes nothing. Works without administrator rights (some service details
# need them). Kept ASCII-only for Windows PowerShell 5.1.

$ErrorActionPreference = 'Continue'
$svc = 'KindoraAgentDev'
$dir = 'C:\Program Files\KindoraDev'
$helper = Join-Path $dir 'kindora-helper.exe'

function Head([string]$m) { Write-Host "`n== $m ==" }

Head 'Agent servisi'
$s = Get-CimInstance Win32_Service -Filter "Name='$svc'" -ErrorAction SilentlyContinue
if ($s) {
    Write-Host "durum=$($s.State) baslatma=$($s.StartMode) hesap=$($s.StartName) pid=$($s.ProcessId)"
    sc.exe qfailure $svc | Select-String 'RESTART|RESET' | ForEach-Object { Write-Host $_.Line.Trim() }
}
else { Write-Host 'Servis kurulu degil.' }

Head 'Oturum acilisi gorevi (\Kindora\)'
$tasks = @(Get-ScheduledTask -TaskPath '\Kindora\' -ErrorAction SilentlyContinue)
if ($tasks.Count -eq 0) { Write-Host 'Gorev yok (arka plan yardimcisi oturum acilisinda baslamaz).' }
foreach ($t in $tasks) {
    $i = Get-ScheduledTaskInfo -TaskPath '\Kindora\' -TaskName $t.TaskName
    Write-Host ("{0} durum={1} son_calisma={2} son_sonuc=0x{3:X} kullanici={4}" -f $t.TaskName, $t.State, $i.LastRunTime, $i.LastTaskResult, $t.Principal.UserId)
}

Head 'Kindora surecleri'
$procs = @(Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe' OR Name='kindora-desktop.exe'")
if ($procs.Count -eq 0) { Write-Host 'Calisan surec yok.' }
foreach ($p in $procs) {
    $role = if ($p.Name -eq 'kindora-desktop.exe') { 'masaustu' }
    elseif ($p.CommandLine -match ' bridge') { 'masaustu koprusu' }
    elseif ($p.CommandLine -match ' (activity|run)') { 'ARKA PLAN yardimcisi' }
    else { 'tek seferlik' }
    Write-Host "pid=$($p.ProcessId) oturum=$($p.SessionId) rol=$role"
}

Head 'Kullanim suresi (agent)'
if (Test-Path $helper) {
    $out = & $helper screen-time 2>$null | Out-String
    if ($LASTEXITCODE -eq 0) {
        $st = ($out | ConvertFrom-Json).payload
        $t = $st.today
        Write-Host "gun=$($st.day) durum=$($st.state) aktif=$($t.active_secs)s bosta=$($t.idle_secs)s kilitli=$($t.locked_secs)s uyku=$($t.suspended_secs)s olculemeyen=$($t.unknown_secs)s"
        $r = $st.measurement.reporter
        if ($r) {
            $since = [DateTimeOffset]::FromUnixTimeMilliseconds([int64]$r.since_ms).LocalDateTime
            Write-Host "raporlayici=$($r.kind) pid=$($r.pid) oturum=$($r.session_id) baglanti=$since son_rapor=$($st.measurement.last_report_age_ms)ms reddedilen_ikinci=$($st.measurement.competing_reports)"
        }
        else { Write-Host "raporlayici=YOK neden=$($st.measurement.unknown_reason)" }
    }
    else { Write-Host "Agent'a ulasilamadi (kod $LASTEXITCODE)." }
}
else { Write-Host "Bulunamadi: $helper" }

Head 'Arka plan yardimcisi kaydi (bu kullanici, son 12 satir)'
$log = Join-Path $env:LOCALAPPDATA 'Kindora\helper\activity.log'
if (Test-Path $log) { Get-Content $log -Tail 12 } else { Write-Host "Kayit yok: $log" }
