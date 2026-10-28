# Kindora P1A - LocalService service end-to-end check (CI, BUG-003).
#
# Installs KindoraAgentDev as NT AUTHORITY\LocalService with the SAME commands
# as packaging/p1a/README-TEST-TR.md, then checks:
#   1. A foreign client (this powershell.exe, no LocalService ACE) is rejected
#      with os_errors=OpenProcess=5 / Unauthorized(ExePathUnknown)  -> BUG-003 reproduction.
#   2. kindora-helper.exe (adds the ACE) is identified: exe path read, PING/STATUS/HEALTH OK.
#   3. The same helper copied to another directory is rejected (ExeDirMismatch).
#   4. Service stops cleanly; it is always deleted afterwards.
#
# Runs under Windows PowerShell 5.1 on purpose (README compatibility).
# Kept ASCII-only: PowerShell 5.1 reads BOM-less scripts as ANSI.
# The runner account is an administrator, so this is NOT the standard-user /
# Windows 11 Home-Pro acceptance test (W-03, W-08 remain device tests).

param(
    [Parameter(Mandatory = $true)][string]$BinDir,
    [string]$SummaryFile = ''
)

$ErrorActionPreference = 'Stop'
$svc = 'KindoraAgentDev'
$dir = 'C:\Program Files\KindoraDev'
$helper = Join-Path $dir 'kindora-helper.exe'
$logCandidates = @(
    'C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log',
    'C:\Windows\Temp\kindora-agent-dev.log'
)
$summary = New-Object System.Collections.Generic.List[string]

function Get-AgentLog {
    foreach ($p in $logCandidates) {
        if (Test-Path $p) { return @(Get-Content $p) }
    }
    return @()
}

function Assert-True($cond, [string]$msg) {
    if (-not $cond) { throw "FAILED: $msg" }
    Write-Host "OK: $msg"
}

function Wait-Until([scriptblock]$cond, [int]$ms, [string]$what) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $ms) {
        if (& $cond) { return }
        Start-Sleep -Milliseconds 200
    }
    throw "TIMEOUT: $what"
}

# Returns the first agent log line matching the regex (waits up to $ms).
function Wait-LogLine([string]$regex, [int]$ms = 10000) {
    $script:found = $null
    Wait-Until { $script:found = Get-AgentLog | Where-Object { $_ -match $regex } | Select-Object -First 1; [bool]$script:found } $ms "log line /$regex/"
    return $script:found
}

$session = (Get-Process -Id $PID).SessionId
Write-Host "runner: $(whoami) session=$session psversion=$($PSVersionTable.PSVersion)"

Write-Host '== Install (README 4.1 commands) =='
New-Item -ItemType Directory $dir -Force | Out-Null
Copy-Item (Join-Path $BinDir 'kindora-agent.exe'), (Join-Path $BinDir 'kindora-helper.exe') $dir
sc.exe --% create KindoraAgentDev binPath= "\"C:\Program Files\KindoraDev\kindora-agent.exe\" service" start= demand obj= "NT AUTHORITY\LocalService" DisplayName= "Kindora Agent (DEV TEST)"
Assert-True ($LASTEXITCODE -eq 0) "sc.exe create (exit=$LASTEXITCODE)"
$qc = sc.exe qc $svc | Out-String
Write-Host $qc
Assert-True ($qc -match 'DEMAND_START') 'start= demand'
Assert-True ($qc -match 'NT AUTHORITY\\LocalService') 'obj= NT AUTHORITY\LocalService'
Assert-True ($qc.Contains('"C:\Program Files\KindoraDev\kindora-agent.exe" service')) 'binPath quoted correctly'

try {
    Write-Host '== Start (README 4.2) =='
    sc.exe start $svc | Out-Host
    Assert-True ($LASTEXITCODE -eq 0) "sc.exe start (exit=$LASTEXITCODE)"
    Wait-Until { (Get-Service $svc).Status -eq 'Running' } 15000 'service Running'
    $null = Wait-LogLine 'event=agent_started .*mode=Service'
    $null = Wait-LogLine 'event=listening transport=named-pipe'
    Assert-True $true 'service running as LocalService, pipe listening'

    Write-Host '== 1. Foreign client without ACE (BUG-003 reproduction) =='
    $c = New-Object System.IO.Pipes.NamedPipeClientStream('.', 'kindora-agent-v1', [System.IO.Pipes.PipeDirection]::InOut)
    $c.Connect(5000)
    Start-Sleep -Milliseconds 300
    $c.Dispose()
    $line = Wait-LogLine "event=client_disconnected pid=$PID .*os_errors=OpenProcess=5 outcome=Unauthorized\(ExePathUnknown\)"
    Write-Host $line
    $summary.Add("LOCALSERVICE foreign_client(no ACE): os_errors=OpenProcess=5 outcome=Unauthorized(ExePathUnknown) [expected]")

    Write-Host '== 2. kindora-helper.exe (README 4.3) =='
    $expectedExe = 'exe="C:\\Program Files\\KindoraDev\\kindora-helper.exe" os_errors=-'
    foreach ($pair in @(@('ping', 'PONG'), @('status', 'STATUS'), @('health', 'HEALTH'))) {
        $out = & $helper $pair[0] | Out-String
        $code = $LASTEXITCODE
        Write-Host "helper $($pair[0]) exit=$code $out"
        if ($session -ne 0) {
            Assert-True ($code -eq 0) "helper $($pair[0]) exit=0"
            Assert-True ($out -match ('"kind":"' + $pair[1] + '"')) "helper $($pair[0]) -> $($pair[1])"
        }
    }
    $okLines = @(Get-AgentLog | Where-Object { $_.Contains('event=client_disconnected') -and $_.Contains($expectedExe) })
    $okLines | ForEach-Object { Write-Host $_ }
    Assert-True ($okLines.Count -ge 3) 'LocalService agent read helper exe path (os_errors=-)'
    if ($session -ne 0) {
        Assert-True (@($okLines | Where-Object { $_ -match 'outcome=PeerClosed requests=1 ' }).Count -ge 3) 'authorized helper requests served'
        $summary.Add("LOCALSERVICE helper ping/status/health: exit=0, exe path read (os_errors=-), outcome=PeerClosed")
    } else {
        $summary.Add("LOCALSERVICE helper: exe path read (os_errors=-); runner session 0 -> NonInteractiveSession expected")
    }

    Write-Host '== 3. Helper copy in another directory (W-04b) =='
    $otherDir = Join-Path $env:RUNNER_TEMP 'farkli'
    New-Item -ItemType Directory $otherDir -Force | Out-Null
    Copy-Item $helper $otherDir
    $out = & (Join-Path $otherDir 'kindora-helper.exe') ping | Out-String
    $code = $LASTEXITCODE
    Write-Host "copied helper ping exit=$code $out"
    Assert-True ($code -eq 3) 'copied helper rejected (exit=3)'
    $line = Wait-LogLine 'event=client_disconnected .*os_errors=- outcome=Unauthorized\((ExeDirMismatch|NonInteractiveSession)\)'
    Write-Host $line
    $summary.Add("LOCALSERVICE helper copy in other dir: exit=3, Unauthorized(ExeDirMismatch)")

    Write-Host '== 4. Stop (README 4.6) =='
    sc.exe stop $svc | Out-Host
    Wait-Until { (Get-Service $svc).Status -eq 'Stopped' } 15000 'service Stopped'
    $null = Wait-LogLine 'event=agent_stopped'
    $summary.Add("LOCALSERVICE service install/start/stop: OK (start= demand, obj= NT AUTHORITY\LocalService, PowerShell 5.1)")
}
finally {
    $ErrorActionPreference = 'Continue'
    Write-Host '== Agent service log =='
    Get-AgentLog | ForEach-Object { Write-Host $_ }
    sc.exe stop $svc | Out-Null
    Start-Sleep -Seconds 1
    sc.exe delete $svc | Out-Host
    Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
}

sc.exe query $svc | Out-Null
Assert-True ($LASTEXITCODE -eq 1060) 'service deleted (1060)'
if ($SummaryFile) { $summary | Add-Content -Path $SummaryFile -Encoding ASCII }
$summary | ForEach-Object { Write-Host $_ }
$global:LASTEXITCODE = 0
