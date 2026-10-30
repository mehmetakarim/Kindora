# Kindora P1D - DEV test installation (NOT FOR PRODUCTION).
#
# Installs or updates (idempotent):
#   - C:\Program Files\KindoraDev  (kindora-agent.exe, kindora-helper.exe,
#     kindora-desktop.exe and these scripts)
#   - Windows service KindoraAgentDev as NT AUTHORITY\LocalService
#     (start mode: -AgentStartMode Demand|Auto; recovery: restart after 5 s,
#     60 s, then every 5 min)
#   - ONLY with -EnableHelperAutostart: a per-user Task Scheduler task
#     \Kindora\SessionHelper-<SID> that starts "kindora-helper.exe activity"
#     at logon of that user, without administrator rights (LeastPrivilege),
#     hidden (conhost --headless), and re-checks every N minutes with a time
#     trigger while the user is logged on (a running helper is never
#     duplicated: IgnoreNew + per-session single instance).
#
# Never touches: the screen time database (LocalService profile), the DEV
# policy file, other services, other tasks, Windows accounts or security
# settings. Does not elevate itself: run it from an administrator PowerShell.
# Kept ASCII-only: Windows PowerShell 5.1 reads BOM-less scripts as ANSI.

param(
    [ValidateSet('Demand', 'Auto')][string]$AgentStartMode = 'Demand',
    [switch]$EnableHelperAutostart,
    # Windows account whose logon starts the helper (default: the account
    # running this script). Must be the account that uses the computer.
    [string]$User = '',
    [ValidateRange(1, 60)][int]$HelperRepeatMinutes = 5,
    # Start the helper task right away (default) so no logoff is needed.
    [switch]$NoStartNow
)

$ErrorActionPreference = 'Stop'
$svc = 'KindoraAgentDev'
$dir = 'C:\Program Files\KindoraDev'
$taskPath = '\Kindora\'
$files = @('kindora-agent.exe', 'kindora-helper.exe', 'kindora-desktop.exe')
$extras = @('install-kindora-test.ps1', 'uninstall-kindora-test.ps1', 'status-kindora-test.ps1', 'README-TEST-TR.md', 'BUILD-INFO.txt', 'SHA256SUMS.txt')

function Say([string]$m) { Write-Host "[kindora] $m" }
function Fail([string]$m) { Write-Host "[kindora] HATA: $m" -ForegroundColor Yellow; exit 1 }

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Fail 'Bu betik yonetici PowerShell gerektirir. Kendini yukseltmez: Baslat > PowerShell > sag tik > Yonetici olarak calistir.'
}

$src = $PSScriptRoot
foreach ($f in $files) {
    if (-not (Test-Path (Join-Path $src $f))) { Fail "Paket eksik: $f ($src)" }
}

# --- Target user (only for the logon task) ---------------------------------
if ($User -eq '') { $User = "$env:USERDOMAIN\$env:USERNAME" }
try {
    $sid = (New-Object Security.Principal.NTAccount($User)).Translate([Security.Principal.SecurityIdentifier]).Value
}
catch { Fail "Windows hesabi bulunamadi: $User" }
if ($EnableHelperAutostart) {
    $console = (Get-CimInstance Win32_ComputerSystem).UserName
    if ($console -and ($console -ne $User)) {
        Fail "Ekrandaki oturum '$console', hedef hesap '$User'. Dogru hesabi -User ile acikca verin (or. -User '$console')."
    }
}

# --- Stop running Kindora processes from this install dir only ---------------
# Stop the task first so its host process (conhost) does not keep the task
# "running" and block the restart below (IgnoreNew).
Get-ScheduledTask -TaskPath $taskPath -ErrorAction SilentlyContinue | Stop-ScheduledTask -ErrorAction SilentlyContinue
$running = @(Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe' OR Name='kindora-desktop.exe'" |
    Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($dir + '\', [StringComparison]::OrdinalIgnoreCase) })
foreach ($p in $running) {
    Say "Durduruluyor: $($p.Name) pid=$($p.ProcessId) oturum=$($p.SessionId)"
    Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
}
sc.exe query $svc | Out-Null
$exists = ($LASTEXITCODE -eq 0)
if ($exists) {
    sc.exe stop $svc | Out-Null
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ((Get-Service $svc).Status -ne 'Stopped' -and $sw.ElapsedMilliseconds -lt 20000) { Start-Sleep -Milliseconds 250 }
    if ((Get-Service $svc).Status -ne 'Stopped') { Fail "Servis durmadi: $svc" }
}

# --- Files (overwrite only Kindora files; DEV policy file is kept) -----------
New-Item -ItemType Directory $dir -Force | Out-Null
if ((Resolve-Path $src).Path.TrimEnd('\') -ne $dir) {
    foreach ($f in ($files + $extras)) {
        $p = Join-Path $src $f
        if (Test-Path $p) { Copy-Item $p $dir -Force }
    }
}
Get-ChildItem (Join-Path $dir '*.exe') | Unblock-File
Say "Dosyalar: $dir"

# --- Service ------------------------------------------------------------------
if (-not $exists) {
    if ($AgentStartMode -eq 'Auto') {
        sc.exe --% create KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= auto obj= "NT AUTHORITY\LocalService" DisplayName= "Kindora Agent (DEV TEST)"
    }
    else {
        sc.exe --% create KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= demand obj= "NT AUTHORITY\LocalService" DisplayName= "Kindora Agent (DEV TEST)"
    }
    if ($LASTEXITCODE -ne 0) { Fail "sc.exe create (kod $LASTEXITCODE)" }
}
else {
    if ($AgentStartMode -eq 'Auto') {
        sc.exe --% config KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= auto
    }
    else {
        sc.exe --% config KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= demand
    }
    if ($LASTEXITCODE -ne 0) { Fail "sc.exe config (kod $LASTEXITCODE)" }
}
sc.exe --% description KindoraAgentDev "Kindora DEV test agent (P1D). Not for production. Screen time only; no lock or blocking."
# Windows service recovery: bounded restarts, never a tight loop.
sc.exe --% failure KindoraAgentDev reset= 86400 actions= restart/5000/restart/60000/restart/300000
if ($LASTEXITCODE -ne 0) { Fail "sc.exe failure (kod $LASTEXITCODE)" }
sc.exe start $svc | Out-Null
$sw = [Diagnostics.Stopwatch]::StartNew()
while ((Get-Service $svc).Status -ne 'Running' -and $sw.ElapsedMilliseconds -lt 20000) { Start-Sleep -Milliseconds 250 }
if ((Get-Service $svc).Status -ne 'Running') { Fail "Servis baslamadi: $svc" }
Say "Servis: $svc calisiyor (baslatma: $AgentStartMode, hesap: NT AUTHORITY\LocalService)"

# --- Helper logon task (explicit opt-in only) --------------------------------
$taskName = "SessionHelper-$sid"
if ($EnableHelperAutostart) {
    $conhost = Join-Path $env:SystemRoot 'System32\conhost.exe'
    $esc = [Security.SecurityElement]
    # Re-check trigger: a time trigger repeats on its own schedule, so a crashed
    # helper is restarted even before the next logon (a logon trigger's
    # repetition only starts after a real logon). Runs only while the user is
    # logged on (InteractiveToken); IgnoreNew + the helper mutex prevent copies.
    $start = (Get-Date).AddMinutes(1).ToString('yyyy-MM-ddTHH:mm:ss')
    $xml = @"
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>Kindora DEV TEST</Author>
    <Description>Kindora P1D test: starts the Kindora session helper (screen time activity signal only) at logon of $($esc::Escape($User)). Not for production. Remove with uninstall-kindora-test.ps1.</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>$sid</UserId>
    </LogonTrigger>
    <TimeTrigger>
      <Enabled>true</Enabled>
      <StartBoundary>$start</StartBoundary>
      <Repetition>
        <Interval>PT$($HelperRepeatMinutes)M</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
    </TimeTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>$sid</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>3</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>$conhost</Command>
      <Arguments>--headless "$dir\kindora-helper.exe" activity --launch logon-task</Arguments>
    </Exec>
  </Actions>
</Task>
"@
    Register-ScheduledTask -TaskPath $taskPath -TaskName $taskName -Xml $xml -Force | Out-Null
    Say "Oturum acilisi gorevi: $taskPath$taskName (hesap: $User, yeniden deneme: $HelperRepeatMinutes dk)"
    if (-not $NoStartNow) {
        Start-ScheduledTask -TaskPath $taskPath -TaskName $taskName
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $h = $null
        while (-not $h -and $sw.ElapsedMilliseconds -lt 20000) {
            Start-Sleep -Milliseconds 500
            $h = Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe'" |
                Where-Object { $_.CommandLine -match ' activity ' } | Select-Object -First 1
        }
        if ($h) { Say "Arka plan yardimcisi calisiyor: pid=$($h.ProcessId) oturum=$($h.SessionId)" }
        else { Say 'UYARI: arka plan yardimcisi 20 sn icinde gorunmedi; status-kindora-test.ps1 ile kontrol edin.' }
    }
}
else {
    $old = Get-ScheduledTask -TaskPath $taskPath -ErrorAction SilentlyContinue
    if ($old) { Say "Mevcut oturum acilisi gorev(ler)i degistirilmedi: $(($old | ForEach-Object { $_.TaskName }) -join ', ')" }
    else { Say 'Oturum acilisi gorevi KURULMADI (-EnableHelperAutostart verilmedi).' }
}

$data = 'C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora'
Say "Veriler (degistirilmedi): $data"
Say 'Tamamlandi. Durum: status-kindora-test.ps1   Kaldirma: uninstall-kindora-test.ps1'
exit 0
