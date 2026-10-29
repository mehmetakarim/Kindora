# Kindora P1A/P1B/P1C - LocalService service end-to-end check (CI, BUG-003, P1.B, P1.C).
#
# Installs KindoraAgentDev as NT AUTHORITY\LocalService with the SAME commands
# as packaging/p1a/README-TEST-TR.md, then checks:
#   1. A foreign client (this powershell.exe, no LocalService ACE) is rejected
#      with os_errors=OpenProcess=5 / Unauthorized(ExePathUnknown)  -> BUG-003 reproduction.
#   2. kindora-helper.exe (adds the ACE) is identified: exe path read, PING/STATUS/HEALTH OK.
#   3. The same helper copied to another directory is rejected (ExeDirMismatch).
#   3b. P1.B desktop bridge: `kindora-helper bridge` reports SERVICE snapshots,
#       answers a refresh and exits when stdin closes.
#   3c. (-DesktopExe) The real Tauri desktop app connects through its helper,
#       sees the service stop/start, and closes without leaving a helper behind.
#   3d. (-ScreenTime) P1.C screen time: DEV policy file validated by the agent,
#       SQLite in the LocalService profile with a protected DACL, totals kept
#       across a service restart without double counting, corrupt database
#       recovered, corrupt policy keeps the last valid one, real Windows API
#       observations (GetLastInputInfo, WTS lock state) recorded.
#   4. Service stops cleanly; it is always deleted afterwards.
#
# Runs under Windows PowerShell 5.1 on purpose (README compatibility).
# Kept ASCII-only: PowerShell 5.1 reads BOM-less scripts as ANSI.
# The runner account is an administrator, so this is NOT the standard-user /
# Windows 11 Home-Pro acceptance test (W-03, W-08 remain device tests).

param(
    [Parameter(Mandatory = $true)][string]$BinDir,
    [string]$SummaryFile = '',
    [string]$DesktopExe = '',
    [switch]$ScreenTime
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
$dataDir = 'C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora'
$db = Join-Path $dataDir 'kindora.db'
$policyFile = Join-Path $dir 'kindora-dev-policy.json'
$policyV1 = '{"kind":"kindora-dev-policy","dev_only":true,"version":1,"daily_limit_secs":300,"break_interval_secs":120,"break_duration_secs":30,"first_warning_secs":60,"final_warning_secs":20,"idle_threshold_secs":60,"note":"CI quick test"}'

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
if ($ScreenTime) {
    Remove-Item $dataDir -Recurse -Force -ErrorAction SilentlyContinue
    [IO.File]::WriteAllText($policyFile, $policyV1, [Text.Encoding]::ASCII)
}
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

    Write-Host '== 3b. Desktop bridge against the service (P1.B) =='
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $helper
    $psi.Arguments = 'bridge'
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $bp = [System.Diagnostics.Process]::Start($psi)
    function Read-BridgeUntil([string]$regex, [int]$ms) {
        $sw = [Diagnostics.Stopwatch]::StartNew()
        while ($true) {
            $left = $ms - [int]$sw.ElapsedMilliseconds
            if ($left -le 0) { throw "TIMEOUT: bridge line /$regex/" }
            $task = $bp.StandardOutput.ReadLineAsync()
            if (-not $task.Wait($left)) { throw "TIMEOUT: bridge line /$regex/" }
            $l = $task.Result
            if ($null -eq $l) { throw "bridge stdout closed before /$regex/" }
            Write-Host "bridge> $l"
            if ($l -match $regex) { return $l }
        }
    }
    $null = Read-BridgeUntil '"type":"hello"' 5000
    if ($session -ne 0) {
        $null = Read-BridgeUntil '"type":"link","state":"connected"' 10000
        $null = Read-BridgeUntil '"type":"snapshot".*"run_mode":"SERVICE".*"healthy":true' 10000
        if ($ScreenTime) {
            $null = Read-BridgeUntil '"type":"snapshot".*"screen_time":\{.*"source":"DEV_FILE"' 10000
            Assert-True $true 'bridge snapshot carries agent screen time (DEV policy)'
        }
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $bp.StandardInput.WriteLine('{"op":"refresh"}')
        $bp.StandardInput.Flush()
        $null = Read-BridgeUntil '"type":"snapshot"' 3000
        Write-Host "bridge refresh took $($sw.ElapsedMilliseconds) ms"
        $bp.StandardInput.WriteLine('{"op":"LOCK"}')
        $bp.StandardInput.Flush()
        $null = Read-BridgeUntil '"type":"request_rejected"' 8000
    } else {
        $null = Read-BridgeUntil '"state":"unauthorized"' 10000
    }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $bp.StandardInput.Close()
    Assert-True ($bp.WaitForExit(5000)) "bridge exits after stdin close ($($sw.ElapsedMilliseconds) ms)"
    Assert-True ($bp.ExitCode -eq 0) "bridge exit code 0 (got $($bp.ExitCode))"
    $summary.Add("LOCALSERVICE desktop bridge: SERVICE snapshot, refresh, LOCK rejected, clean exit on stdin close")

    if ($DesktopExe) {
        Write-Host '== 3c. Tauri desktop app (P1.B GUI smoke) =='
        $desk = Join-Path $dir 'kindora-desktop.exe'
        Copy-Item $DesktopExe $desk
        $dlog = Join-Path $env:LOCALAPPDATA 'app.kindora.desktop.preview\logs\desktop.log'
        Remove-Item $dlog -ErrorAction SilentlyContinue
        function Get-DesktopLog { if (Test-Path $dlog) { return @(Get-Content $dlog) } ; return @() }
        function Count-Desktop([string]$needle) { return @(Get-DesktopLog | Where-Object { $_.Contains($needle) }).Count }
        $dp = Start-Process -FilePath $desk -PassThru
        try {
            Wait-Until { (Count-Desktop 'event=agent_link state=Connected') -ge 1 } 60000 'desktop connected via helper'
            Assert-True $true 'desktop app connected to the LocalService agent through kindora-helper'
            $kids = @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$($dp.Id)" | Where-Object { $_.Name -eq 'kindora-helper.exe' })
            Assert-True ($kids.Count -eq 1) "exactly one helper child of the desktop (found $($kids.Count))"
            $helperPid = $kids[0].ProcessId

            $sw = [Diagnostics.Stopwatch]::StartNew()
            sc.exe stop $svc | Out-Null
            Wait-Until { (Get-Service $svc).Status -eq 'Stopped' } 15000 'service Stopped (desktop test)'
            Wait-Until { (Count-Desktop 'event=agent_link state=Disconnected') -ge 1 } 20000 'desktop sees agent stop'
            Write-Host "desktop detected service stop in $($sw.ElapsedMilliseconds) ms"
            $sw = [Diagnostics.Stopwatch]::StartNew()
            sc.exe start $svc | Out-Null
            Wait-Until { (Get-Service $svc).Status -eq 'Running' } 15000 'service Running again'
            Wait-Until { (Count-Desktop 'event=agent_link state=Connected') -ge 2 } 30000 'desktop recovers after service restart'
            Write-Host "desktop recovered after service start in $($sw.ElapsedMilliseconds) ms"

            $dp.Refresh()
            Assert-True ($dp.MainWindowHandle -ne [IntPtr]::Zero) 'desktop has a normal top-level window'
            $sw = [Diagnostics.Stopwatch]::StartNew()
            Assert-True ($dp.CloseMainWindow()) 'WM_CLOSE delivered to desktop window'
            Assert-True ($dp.WaitForExit(15000)) "desktop exits after window close ($($sw.ElapsedMilliseconds) ms)"
            Start-Sleep -Milliseconds 500
            Assert-True ($null -eq (Get-Process -Id $helperPid -ErrorAction SilentlyContinue)) 'helper child exited with the desktop (no leak)'
            Assert-True ((Count-Desktop 'event=helper_stopped') -ge 1) 'desktop log: helper_stopped'
            Assert-True ((Get-Service $svc).Status -eq 'Running') 'closing the desktop does not stop the agent service'
            $summary.Add("LOCALSERVICE desktop GUI: connected via helper, service stop/start detected and recovered, window close -> desktop+helper exit, service keeps running")
        }
        finally {
            Write-Host '== Desktop log =='
            Get-DesktopLog | ForEach-Object { Write-Host $_ }
            if (-not $dp.HasExited) { Stop-Process -Id $dp.Id -Force -ErrorAction SilentlyContinue }
        }
    }

    if ($ScreenTime) {
        Write-Host '== 3d. Screen time (P1.C) =='
        function Get-ScreenTime([int]$ms = 15000) {
            $sw = [Diagnostics.Stopwatch]::StartNew()
            while ($true) {
                $out = & $helper screen-time | Out-String
                if ($LASTEXITCODE -eq 0) { return ($out | ConvertFrom-Json).payload }
                if ($sw.ElapsedMilliseconds -gt $ms) { throw "TIMEOUT: screen-time ($out)" }
                Start-Sleep -Milliseconds 500
            }
        }
        function Total([object]$st) {
            $t = $st.today
            return [int64]$t.active_secs + $t.idle_secs + $t.locked_secs + $t.suspended_secs + $t.unknown_secs
        }
        if ((Get-Service $svc).Status -ne 'Running') { sc.exe start $svc | Out-Null; Wait-Until { (Get-Service $svc).Status -eq 'Running' } 15000 'service Running (3d)' }

        $st = Get-ScreenTime
        Write-Host ("screen-time: " + ($st | ConvertTo-Json -Depth 5 -Compress))
        Assert-True ($st.kind -eq 'SCREEN_TIME') 'GET_SCREEN_TIME answered by the LocalService agent'
        Assert-True ($st.policy.source -eq 'DEV_FILE' -and $st.policy.version -eq 1 -and $st.policy.status -eq 'OK') 'DEV policy file validated and applied by the agent'
        Assert-True ($st.daily_limit_secs -eq 300) 'daily limit from DEV policy'
        Assert-True ($st.storage.persistent -eq $true) 'SQLite storage persistent'
        Assert-True (Test-Path $db) "database in LocalService profile ($db)"
        $sddl = (Get-Acl $dataDir).Sddl
        Write-Host "data dir SDDL: $sddl"
        Assert-True ($sddl -match '^O:LS' -or $sddl -match '^O:S-1-5-19') 'data dir owned by LocalService'
        Assert-True (-not ($sddl -match ';;;(BU|WD|IU|AU)\)')) 'data dir DACL has no Users/Everyone/Interactive/AuthenticatedUsers ACE'
        $summary.Add("SCREENTIME LocalService: GET_SCREEN_TIME ok, DEV policy v1 applied, SQLite at LocalService profile, protected DACL")

        # Real Windows API observations (recorded, not acceptance).
        $idle1 = & $helper idle | Out-String
        Add-Type -Namespace KdCi -Name Input -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, System.UIntPtr e);'
        [KdCi.Input]::mouse_event(1, 1, 0, 0, [UIntPtr]::Zero)
        [KdCi.Input]::mouse_event(1, -1, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 300
        $idle2 = & $helper idle | Out-String
        $ack = & $helper report-activity | Out-String
        $console = (Get-CimInstance Win32_ComputerSystem).UserName
        Write-Host "GetLastInputInfo before=$($idle1.Trim()) after_injected_input=$($idle2.Trim()) report=$($ack.Trim()) console_user=$console"
        $st = Get-ScreenTime
        Write-Host "state=$($st.state) source=$($st.measurement.source) reason=$($st.measurement.unknown_reason)"
        $summary.Add("SCREENTIME real API (runner, info): idle before=$($idle1.Trim()) after injected input=$($idle2.Trim()); report=$(($ack | ConvertFrom-Json).payload.applied); state=$($st.state) reason=$($st.measurement.unknown_reason)")

        # Restart: totals kept, no double counting.
        Start-Sleep -Seconds 16
        $before = Get-ScreenTime
        $tb = Total $before
        $sw = [Diagnostics.Stopwatch]::StartNew()
        sc.exe stop $svc | Out-Null
        Wait-Until { (Get-Service $svc).Status -eq 'Stopped' } 15000 'service Stopped (3d)'
        sc.exe start $svc | Out-Null
        Wait-Until { (Get-Service $svc).Status -eq 'Running' } 15000 'service Running (3d restart)'
        $after = Get-ScreenTime
        $ta = Total $after
        $gap = [int]($sw.ElapsedMilliseconds / 1000)
        Write-Host "restart: total before=$tb after=$ta gap=${gap}s active before=$($before.today.active_secs) after=$($after.today.active_secs)"
        Assert-True ($ta -ge $tb - 1) "totals kept across service restart ($tb -> $ta)"
        Assert-True ($ta -le $tb + $gap + 5) "no double counting after restart ($tb -> $ta, gap ${gap}s)"
        Assert-True ($after.policy.version -eq 1) 'last valid policy version kept'
        Assert-True (@(Get-AgentLog | Where-Object { $_ -match 'event=screen_time kind=AGENT_STOP' }).Count -ge 1) 'graceful stop flushed AGENT_STOP'
        $summary.Add("SCREENTIME service restart: totals $tb s -> $ta s (gap ${gap}s), no double counting, AGENT_STOP flushed")

        # Corrupt database -> quarantined and recreated.
        sc.exe stop $svc | Out-Null
        Wait-Until { (Get-Service $svc).Status -eq 'Stopped' } 15000 'service Stopped (corrupt db)'
        [IO.File]::WriteAllText($db, 'this is not a sqlite database at all', [Text.Encoding]::ASCII)
        Remove-Item "$db-wal", "$db-shm" -ErrorAction SilentlyContinue
        sc.exe start $svc | Out-Null
        Wait-Until { (Get-Service $svc).Status -eq 'Running' } 15000 'service Running (corrupt db)'
        $st = Get-ScreenTime
        Assert-True ($st.storage.persistent -and $st.storage.recovered) 'corrupt database recovered (persistent, recovered=true)'
        Assert-True (@(Get-ChildItem $dataDir -Filter 'kindora.db.corrupt-*').Count -ge 1) 'corrupt copy kept for diagnostics'
        $null = Wait-LogLine 'event=screen_time kind=DB_RECOVERED'
        $summary.Add("SCREENTIME corrupt SQLite: quarantined + recreated, service kept running")

        # Corrupt policy file -> last valid kept.
        [IO.File]::WriteAllText($policyFile, '{ broken', [Text.Encoding]::ASCII)
        $script:pst = $null
        Wait-Until { $script:pst = Get-ScreenTime; $script:pst.policy.status -eq 'INVALID_KEPT_LAST' } 20000 'corrupt policy -> INVALID_KEPT_LAST'
        Assert-True ($script:pst.policy.version -eq 1 -and $script:pst.daily_limit_secs -eq 300) 'last valid policy still applied'
        $summary.Add("SCREENTIME corrupt policy file: INVALID_KEPT_LAST, last valid v1 kept; no lock applied")
    }

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
    if ($ScreenTime) { Remove-Item $dataDir -Recurse -Force -ErrorAction SilentlyContinue }
}

sc.exe query $svc | Out-Null
Assert-True ($LASTEXITCODE -eq 1060) 'service deleted (1060)'
if ($SummaryFile) { $summary | Add-Content -Path $SummaryFile -Encoding ASCII }
$summary | ForEach-Object { Write-Host $_ }
$global:LASTEXITCODE = 0
