# Kindora P1D - remove the DEV test installation (idempotent).
#
# Removes ONLY Kindora entries:
#   - Task Scheduler tasks in \Kindora\ (and the empty folder)
#   - kindora-helper.exe / kindora-desktop.exe processes started from
#     C:\Program Files\KindoraDev
#   - service KindoraAgentDev (verified with "sc.exe query" -> 1060)
#   - C:\Program Files\KindoraDev
# The screen time database is KEPT unless -DeleteData is given AND confirmed
# by typing SIL (or -Yes for unattended CI runs).
# Does not elevate itself. Kept ASCII-only for Windows PowerShell 5.1.

param(
    [switch]$DeleteData,
    [switch]$Yes
)

$ErrorActionPreference = 'Stop'
$svc = 'KindoraAgentDev'
$dir = 'C:\Program Files\KindoraDev'
$taskPath = '\Kindora\'
$data = 'C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora'
$userData = @(
    (Join-Path $env:LOCALAPPDATA 'Kindora'),
    (Join-Path $env:LOCALAPPDATA 'app.kindora.desktop.preview')
)

function Say([string]$m) { Write-Host "[kindora] $m" }
function Fail([string]$m) { Write-Host "[kindora] HATA: $m" -ForegroundColor Yellow; exit 1 }

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Fail 'Bu betik yonetici PowerShell gerektirir. Kendini yukseltmez.'
}

if ($DeleteData -and -not $Yes) {
    Write-Host "Kullanim suresi kayitlari kalici olarak silinecek:`n  $data`n  $($userData -join "`n  ")"
    $answer = Read-Host 'Onaylamak icin SIL yazin (baska bir sey: veriler korunur)'
    if ($answer -cne 'SIL') {
        Say 'Veri silme onaylanmadi; veriler korunacak.'
        $DeleteData = $false
    }
}

# 1. Logon tasks (first, so nothing restarts the helper).
$tasks = @(Get-ScheduledTask -TaskPath $taskPath -ErrorAction SilentlyContinue)
foreach ($t in $tasks) {
    Unregister-ScheduledTask -TaskPath $taskPath -TaskName $t.TaskName -Confirm:$false
    Say "Gorev kaldirildi: $taskPath$($t.TaskName)"
}
try {
    $sch = New-Object -ComObject Schedule.Service
    $sch.Connect()
    $sch.GetFolder('\').DeleteFolder('Kindora', 0)
    Say 'Gorev klasoru kaldirildi: \Kindora'
}
catch { }
if (@(Get-ScheduledTask -TaskPath $taskPath -ErrorAction SilentlyContinue).Count -ne 0) { Fail 'Kindora gorevleri kaldirilamadi.' }

# 2. Kindora processes from the install dir (all sessions).
$procs = @(Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe' OR Name='kindora-desktop.exe'" |
    Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($dir + '\', [StringComparison]::OrdinalIgnoreCase) })
foreach ($p in $procs) {
    Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
    Say "Durduruldu: $($p.Name) pid=$($p.ProcessId) oturum=$($p.SessionId)"
}

# 3. Service.
sc.exe query $svc | Out-Null
if ($LASTEXITCODE -eq 0) {
    sc.exe stop $svc | Out-Null
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt 20000) {
        $state = (Get-Service $svc -ErrorAction SilentlyContinue).Status
        if ($null -eq $state -or $state -eq 'Stopped') { break }
        Start-Sleep -Milliseconds 250
    }
    sc.exe delete $svc | Out-Null
}
$sw = [Diagnostics.Stopwatch]::StartNew()
do {
    sc.exe query $svc | Out-Null
    $code = $LASTEXITCODE
    if ($code -eq 1060) { break }
    Start-Sleep -Milliseconds 500
} while ($sw.ElapsedMilliseconds -lt 20000)
if ($code -ne 1060) { Fail "Servis silinemedi (sc.exe query kodu $code). Acik bir Hizmetler penceresi varsa kapatip tekrar deneyin." }
Say "Servis silindi: $svc (sc.exe query -> 1060)"

# 4. Program files (includes the DEV policy file).
if (Test-Path $dir) {
    Remove-Item $dir -Recurse -Force
    Say "Klasor kaldirildi: $dir"
}

# 5. Data.
if ($DeleteData) {
    foreach ($p in @($data) + $userData) {
        if (Test-Path $p) {
            Remove-Item $p -Recurse -Force
            Say "Veri silindi: $p"
        }
    }
}
else {
    Say "Veriler korundu: $data"
    Say "Yardimci kaydi korundu: $($userData[0])"
}
Say 'Kaldirma tamamlandi.'
exit 0
