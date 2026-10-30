# Kindora P1D.1 - background helper recovery by the REAL scheduled trigger (CI).
#
# Reproduces the Lenovo D-05 scenario with the DEFAULT 5 minute re-check
# (same as the Lenovo installation) and proves recovery from Task Scheduler
# evidence, not from a manual start:
#   1. install (Auto + logon task, default interval) -> helper started on demand
#      (Task Scheduler event 110 = "triggered by user").
#   2. Wait for the first time-trigger firing while the helper runs:
#      expect a refused launch (event 322) and LastTaskResult 0x800710E0.
#   3. Desktop bridge running (fallback reporter, like the Lenovo test).
#   4. Stop-Process the helper (same command as on Lenovo); record task state,
#      LastTaskResult, NextRunTime.
#   5. Wait past NextRunTime: a NEW helper must appear, launched by the time
#      trigger (event 107 + 129 whose process id is the new helper's parent),
#      with NO manual launch (event 110) after the kill.
#   6. Agent reporter returns to BACKGROUND_LOGON_TASK (takeover from the
#      bridge), helper logs previous_exit=unclean, totals are not double counted.
# The Task Scheduler Operational log is enabled for this CI run only and its
# previous state is restored. Kept ASCII-only for Windows PowerShell 5.1.

param(
    [Parameter(Mandatory = $true)][string]$PackageDir,
    [string]$SummaryFile = ''
)

$ErrorActionPreference = 'Stop'
$svc = 'KindoraAgentDev'
$dir = 'C:\Program Files\KindoraDev'
$helper = Join-Path $dir 'kindora-helper.exe'
$helperLog = Join-Path $env:LOCALAPPDATA 'Kindora\helper\activity.log'
$agentLog = 'C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log'
$opLog = 'Microsoft-Windows-TaskScheduler/Operational'
$summary = New-Object System.Collections.Generic.List[string]

function Assert-True($cond, [string]$msg) {
    if (-not $cond) { throw "FAILED: $msg" }
    Write-Host "OK: $msg"
}

function Wait-Until([scriptblock]$cond, [int]$ms, [string]$what) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $ms) {
        if (& $cond) { return }
        Start-Sleep -Milliseconds 500
    }
    throw "TIMEOUT: $what"
}

function Get-ScreenTime {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($true) {
        $out = & $helper screen-time 2>$null | Out-String
        if ($LASTEXITCODE -eq 0) { return ($out | ConvertFrom-Json).payload }
        if ($sw.ElapsedMilliseconds -gt 20000) { throw "TIMEOUT: screen-time ($out)" }
        Start-Sleep -Milliseconds 500
    }
}

function Total([object]$st) {
    $t = $st.today
    return [int64]$t.active_secs + $t.idle_secs + $t.locked_secs + $t.suspended_secs + $t.unknown_secs
}

function Get-Activity {
    return @(Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe'" | Where-Object { $_.CommandLine -match ' activity' })
}

function Hex([object]$v) { return ('0x{0:X8}' -f ([uint32]([int64]$v -band 0xFFFFFFFF))) }

function Task-Info([string]$label) {
    $t = Get-ScheduledTask -TaskPath '\Kindora\' -TaskName $script:taskName
    $i = Get-ScheduledTaskInfo -TaskPath '\Kindora\' -TaskName $script:taskName
    $line = "{0}: state={1} last_run={2:HH:mm:ss} next_run={3:HH:mm:ss} last_result={4}" -f $label, $t.State, $i.LastRunTime, $i.NextRunTime, (Hex $i.LastTaskResult)
    Write-Host $line
    return $i
}

# Task Scheduler events of our task after $since: id, time, process id (129).
function Get-TaskEvents([datetime]$since) {
    $ev = @(Get-WinEvent -FilterHashtable @{ LogName = $opLog; StartTime = $since } -ErrorAction SilentlyContinue |
        Where-Object { $_.Message -like "*$($script:taskName)*" } | Sort-Object TimeCreated)
    return $ev
}

function Show-Events([object[]]$ev) {
    foreach ($e in $ev) {
        $m = ($e.Message -replace '\s+', ' ')
        if ($m.Length -gt 200) { $m = $m.Substring(0, 200) }
        Write-Host ("  event {0} {1:HH:mm:ss.fff} {2}" -f $e.Id, $e.TimeCreated, $m)
    }
}

$me = "$env:USERDOMAIN\$env:USERNAME"
$wasEnabled = (Get-WinEvent -ListLog $opLog).IsEnabled
Write-Host "Task Scheduler Operational log enabled before: $wasEnabled"
if (-not $wasEnabled) { wevtutil.exe sl $opLog /e:true | Out-Null }
$script:taskName = $null
$bp = $null

try {
    Write-Host '== 1. install with the default 5 min re-check (as on Lenovo) =='
    $installAt = Get-Date
    $out = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PackageDir 'install-kindora-test.ps1') -AgentStartMode Auto -EnableHelperAutostart -User $me 2>&1 | Out-String
    Write-Host $out
    Assert-True ($LASTEXITCODE -eq 0) 'install exit 0'
    $script:taskName = (@(Get-ScheduledTask -TaskPath '\Kindora\'))[0].TaskName
    $timer = @((Get-ScheduledTask -TaskPath '\Kindora\' -TaskName $script:taskName).Triggers | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_TaskTimeTrigger' })[0]
    Write-Host "time trigger: start=$($timer.StartBoundary) interval=$($timer.Repetition.Interval)"
    Assert-True ($timer.Repetition.Interval -eq 'PT5M') 'default re-check interval 5 min'
    Wait-Until { @(Get-Activity).Count -eq 1 } 30000 'helper started'
    $old = @(Get-Activity)[0]
    $null = Task-Info 'after install'
    $ev = Get-TaskEvents $installAt
    Show-Events $ev
    Assert-True (@($ev | Where-Object { $_.Id -eq 110 }).Count -ge 1) 'install start = manual launch (event 110)'

    Write-Host '== 2. first time-trigger firing while the helper runs =='
    $first = [datetime]::Parse($timer.StartBoundary)
    Wait-Until { (Get-Date) -gt $first.AddSeconds(20) } 150000 'first time trigger due'
    $i = Task-Info 'after first trigger'
    $ev = Get-TaskEvents $first.AddSeconds(-5)
    Show-Events $ev
    $refused = @($ev | Where-Object { $_.Id -eq 322 })
    Assert-True ($refused.Count -ge 1) 'time trigger fired and the launch was refused (event 322: instance already running)'
    Assert-True ((Hex $i.LastTaskResult) -eq '0x800710E0') "LastTaskResult 0x800710E0 while the instance runs (got $(Hex $i.LastTaskResult))"
    Assert-True (@(Get-Activity).Count -eq 1 -and @(Get-Activity)[0].ProcessId -eq $old.ProcessId) 'refused launch: still exactly the same single helper'
    $summary.Add("P1D1 refused trigger: time trigger at $($first.ToString('HH:mm:ss')) -> event 322, LastTaskResult 0x800710E0, helper unchanged (pid $($old.ProcessId))")

    Write-Host '== 3. desktop bridge as fallback (like the Lenovo test) =='
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $helper
    $psi.Arguments = 'bridge'
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $bp = [System.Diagnostics.Process]::Start($psi)
    # Drain both pipes so the bridge never blocks on a full pipe buffer.
    $bp.BeginOutputReadLine()
    $bp.BeginErrorReadLine()

    Write-Host '== 4. Stop-Process (same command as Lenovo D-05) =='
    Start-Sleep -Seconds 16
    $before = Get-ScreenTime
    $killAt = Get-Date
    Stop-Process -Id $old.ProcessId
    Wait-Until { @(Get-Activity).Count -eq 0 } 10000 'helper gone'
    Wait-Until { (Get-ScheduledTask -TaskPath '\Kindora\' -TaskName $script:taskName).State -eq 'Ready' } 15000 'task instance ended (Ready)'
    $i = Task-Info 'after kill'
    $next = $i.NextRunTime
    $killResult = Hex $i.LastTaskResult
    Wait-Until { (Get-ScreenTime).measurement.reporter.kind -eq 'DESKTOP_BRIDGE' } 20000 'bridge takes over while the helper is down'
    Assert-True (@(Get-Activity).Count -eq 0) 'no background helper until the trigger fires'
    $summary.Add("P1D1 kill: task Ready, LastTaskResult $killResult, NextRunTime $($next.ToString('HH:mm:ss')), reporter DESKTOP_BRIDGE (fallback)")

    Write-Host "== 5. wait past NextRunTime ($($next.ToString('HH:mm:ss'))) for the scheduled restart =="
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Wait-Until { @(Get-Activity).Count -eq 1 } ([int](($next - (Get-Date)).TotalMilliseconds) + 120000) 'helper restarted by the scheduled trigger'
    $new = @(Get-Activity)[0]
    $restartAt = Get-Date
    Write-Host "new helper pid=$($new.ProcessId) parent=$($new.ParentProcessId) after $([int]($restartAt - $killAt).TotalSeconds) s (next run was $($next.ToString('HH:mm:ss')))"
    $null = Task-Info 'after restart'
    $ev = Get-TaskEvents $killAt
    Show-Events $ev
    Assert-True ($new.ProcessId -ne $old.ProcessId) 'new helper pid'
    Assert-True (@($ev | Where-Object { $_.Id -eq 110 }).Count -eq 0) 'no manual launch (event 110) after the kill'
    Assert-True (@($ev | Where-Object { $_.Id -eq 107 }).Count -ge 1) 'launch triggered by the time trigger (event 107)'
    $launch = @($ev | Where-Object { $_.Id -eq 129 })
    Assert-True ($launch.Count -ge 1) 'task process created (event 129)'
    $launchPid = [int]([regex]::Match($launch[-1].Message, 'process ID (\d+)').Groups[1].Value)
    Write-Host "event 129 process id=$launchPid"
    Assert-True ($launchPid -eq [int]$new.ParentProcessId) 'new helper is the child of the process created by the trigger (conhost)'
    # Get-ScheduledTaskInfo.NextRunTime was observed ~30 s off the real firing
    # (event 107) on the runner; the event log is authoritative.
    $fired = @($ev | Where-Object { $_.Id -eq 107 })[-1].TimeCreated
    Write-Host "time trigger fired at $($fired.ToString('HH:mm:ss')) (NextRunTime reported $($next.ToString('HH:mm:ss')))"
    Assert-True ($fired -gt $killAt -and [math]::Abs(($fired - $next).TotalSeconds) -le 60) 'restart by the scheduled firing that followed the kill (within 60 s of NextRunTime)'

    Write-Host '== 6. reporter, unclean exit, no double counting =='
    $script:after = $null
    Wait-Until { $script:after = Get-ScreenTime; $script:after.measurement.reporter.kind -eq 'BACKGROUND_LOGON_TASK' -and [int]$script:after.measurement.reporter.pid -eq [int]$new.ProcessId } 30000 'reporter back to BACKGROUND_LOGON_TASK (new pid)'
    Wait-Until { @(Get-Content $helperLog | Where-Object { $_.Contains('event=previous_exit state=unclean') }).Count -ge 1 } 10000 'helper log: previous_exit=unclean'
    Assert-True (@(Get-Content $agentLog | Where-Object { $_ -match 'REPORTER_TAKEOVER .*kind=BACKGROUND_LOGON_TASK' }).Count -ge 1) 'agent log: background helper took over from the bridge'
    # Time still pending at the kill (input frontier) is classified later, so
    # "after - before <= outage" is not a valid bound. The day total can never
    # exceed the wall time since the agent (fresh database) started.
    $elapsed = [int]((Get-Date) - $killAt).TotalSeconds
    $sinceInstall = [int]((Get-Date) - $installAt).TotalSeconds
    $tb = Total $before; $ta = Total $script:after
    Write-Host "totals: $tb -> $ta s (outage $elapsed s, since install $sinceInstall s)"
    Assert-True ($ta -le $sinceInstall + 2) "no double counting: day total $ta s <= wall time since install $sinceInstall s"
    Assert-True ($ta -ge $tb) 'totals kept'
    $summary.Add("P1D1 scheduled recovery: kill $($killAt.ToString('HH:mm:ss')) -> new helper $($restartAt.ToString('HH:mm:ss')) (next run $($next.ToString('HH:mm:ss'))), events 107+129 (pid=$launchPid = parent), no event 110, reporter BACKGROUND_LOGON_TASK pid $($new.ProcessId), day total $ta s <= $sinceInstall s since install")
}
finally {
    $ErrorActionPreference = 'Continue'
    if ($bp -and -not $bp.HasExited) { $bp.StandardInput.Close(); $null = $bp.WaitForExit(5000) }
    if ($script:taskName) {
        Write-Host '== All Task Scheduler events for the task =='
        Show-Events (Get-TaskEvents $installAt.AddMinutes(-1))
    }
    Write-Host '== Helper log =='
    if (Test-Path $helperLog) { Get-Content $helperLog | ForEach-Object { Write-Host $_ } }
    Write-Host '== Agent log (reporter) =='
    if (Test-Path $agentLog) { Get-Content $agentLog | Where-Object { $_ -match 'REPORTER|agent_started' } | ForEach-Object { Write-Host $_ } }
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PackageDir 'uninstall-kindora-test.ps1') -DeleteData -Yes | Out-Host
    if (-not $wasEnabled) { wevtutil.exe sl $opLog /e:false | Out-Null }
}

if ($SummaryFile) { $summary | Add-Content -Path $SummaryFile -Encoding ASCII }
$summary | ForEach-Object { Write-Host $_ }
$global:LASTEXITCODE = 0
