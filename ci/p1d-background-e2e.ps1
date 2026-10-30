# Kindora P1D - independent background helper end-to-end check (CI).
#
# Uses the SAME packaging scripts as the Lenovo test (packaging/p1d):
#   1. install-kindora-test.ps1 -AgentStartMode Auto -EnableHelperAutostart
#      -> service AUTO_START + recovery actions, per-user logon task with the
#         expected settings (LeastPrivilege, InteractiveToken, battery-safe,
#         no time limit, IgnoreNew), helper started by the task in the runner
#         session, agent reporter = BACKGROUND_LOGON_TASK.
#   2. Desktop open: its bridge is NOT accepted as a second reporter.
#      Desktop closed: bridge exits, background helper keeps running and
#      measurement continues (totals grow, report stays fresh).
#      Desktop reopened: totals kept.
#   3. Second "activity" instance exits (single instance per session);
#      one-shot report rejected.
#   4. Helper killed: reporter released; the logon task re-check restarts it;
#      the new helper logs previous_exit=unclean.
#   5. Agent service restart: helper reconnects; no double counting.
#   6. Re-running the installer is idempotent and keeps data.
#   7. Uninstall keeps data (1060, no task, no process, no files);
#      uninstall -DeleteData -Yes removes it.
# NOT covered (device tests): a real logon trigger, logoff, fast user
# switching, Windows restart, sleep, standard-user / Windows 11 Home.
# Kept ASCII-only for Windows PowerShell 5.1.

param(
    [Parameter(Mandatory = $true)][string]$PackageDir,
    [string]$SummaryFile = ''
)

$ErrorActionPreference = 'Stop'
$svc = 'KindoraAgentDev'
$dir = 'C:\Program Files\KindoraDev'
$helper = Join-Path $dir 'kindora-helper.exe'
$data = 'C:\Windows\ServiceProfiles\LocalService\AppData\Local\Kindora'
$db = Join-Path $data 'kindora.db'
$helperLog = Join-Path $env:LOCALAPPDATA 'Kindora\helper\activity.log'
$agentLog = 'C:\Windows\ServiceProfiles\LocalService\AppData\Local\Temp\kindora-agent-dev.log'
$summary = New-Object System.Collections.Generic.List[string]

function Assert-True($cond, [string]$msg) {
    if (-not $cond) { throw "FAILED: $msg" }
    Write-Host "OK: $msg"
}

function Wait-Until([scriptblock]$cond, [int]$ms, [string]$what) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $ms) {
        if (& $cond) { return }
        Start-Sleep -Milliseconds 300
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

function Get-Helpers([string]$role) {
    return @(Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe'" | Where-Object { $_.CommandLine -match " $role" })
}

function Get-HelperLog { if (Test-Path $helperLog) { return @(Get-Content $helperLog) } ; return @() }
function Count-HelperLog([string]$needle) { return @(Get-HelperLog | Where-Object { $_.Contains($needle) }).Count }

function Invoke-Script([string]$name, [string[]]$scriptArgs) {
    $path = Join-Path $PackageDir $name
    $out = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $path @scriptArgs 2>&1 | Out-String
    $code = $LASTEXITCODE
    Write-Host $out
    return @{ Code = $code; Out = $out }
}

$session = (Get-Process -Id $PID).SessionId
$me = "$env:USERDOMAIN\$env:USERNAME"
Write-Host "runner: $me session=$session psversion=$($PSVersionTable.PSVersion)"
Remove-Item $helperLog, "$helperLog.running" -ErrorAction SilentlyContinue
$taskName = $null

try {
    Write-Host '== 1. install (Auto + logon task, 1 min re-check) =='
    $r = Invoke-Script 'install-kindora-test.ps1' @('-AgentStartMode', 'Auto', '-EnableHelperAutostart', '-User', $me, '-HelperRepeatMinutes', '1')
    Assert-True ($r.Code -eq 0) "install exit 0 (got $($r.Code))"
    $qc = sc.exe qc $svc | Out-String
    Assert-True ($qc -match 'AUTO_START') 'service start= auto'
    Assert-True ($qc -match 'NT AUTHORITY\\LocalService') 'service account LocalService'
    $qf = sc.exe qfailure $svc | Out-String
    Write-Host $qf
    Assert-True (([regex]::Matches($qf, 'RESTART')).Count -ge 3) 'service recovery: bounded restarts'
    $task = @(Get-ScheduledTask -TaskPath '\Kindora\')
    Assert-True ($task.Count -eq 1) 'one Kindora logon task'
    $taskName = $task[0].TaskName
    $x = [xml](Export-ScheduledTask -TaskPath '\Kindora\' -TaskName $taskName)
    $set = $x.Task.Settings
    $pr = $x.Task.Principals.Principal
    Assert-True ($pr.RunLevel -eq 'LeastPrivilege' -and $pr.LogonType -eq 'InteractiveToken') 'task: LeastPrivilege + InteractiveToken (no admin, no password)'
    Assert-True ($set.DisallowStartIfOnBatteries -eq 'false' -and $set.StopIfGoingOnBatteries -eq 'false') 'task: runs on battery'
    Assert-True ($set.ExecutionTimeLimit -eq 'PT0S' -and $set.MultipleInstancesPolicy -eq 'IgnoreNew') 'task: no time limit, IgnoreNew'
    Assert-True ($x.Task.Triggers.LogonTrigger.Repetition.Interval -eq 'PT1M') 'task: logon trigger + 1 min re-check'
    Assert-True ($x.Task.Actions.Exec.Arguments -match 'activity --launch logon-task') 'task action: helper activity --launch logon-task'
    $summary.Add("P1D install: service AUTO_START + recovery restarts; logon task LeastPrivilege/InteractiveToken, battery-safe, PT0S, IgnoreNew")

    Wait-Until { (Get-Helpers 'activity').Count -eq 1 } 30000 'task started the background helper'
    $bg = (Get-Helpers 'activity')[0]
    Write-Host "background helper pid=$($bg.ProcessId) session=$($bg.SessionId) parent=$($bg.ParentProcessId)"
    Assert-True ($bg.SessionId -eq $session) 'helper runs in the user session (not session 0)'
    $script:st = $null
    Wait-Until { $script:st = Get-ScreenTime; $script:st.measurement.reporter.kind -eq 'BACKGROUND_LOGON_TASK' } 30000 'agent reporter = BACKGROUND_LOGON_TASK'
    Assert-True ([int]$script:st.measurement.reporter.pid -eq [int]$bg.ProcessId) 'reporter pid (from OS) = background helper'
    $summary.Add("P1D logon task (started on demand): helper in session $session, reporter BACKGROUND_LOGON_TASK pid=$($bg.ProcessId)")

    Write-Host '== 2. Desktop open / close / reopen =='
    $desk = Join-Path $dir 'kindora-desktop.exe'
    $dlog = Join-Path $env:LOCALAPPDATA 'app.kindora.desktop.preview\logs\desktop.log'
    Remove-Item $dlog -ErrorAction SilentlyContinue
    function Count-Desktop([string]$needle) { if (Test-Path $dlog) { return @(Get-Content $dlog | Where-Object { $_.Contains($needle) }).Count } ; return 0 }
    $dp = Start-Process -FilePath $desk -PassThru
    Wait-Until { (Count-Desktop 'event=agent_link state=Connected') -ge 1 } 60000 'desktop connected'
    $c0 = [int](Get-ScreenTime).measurement.competing_reports
    Wait-Until { [int](Get-ScreenTime).measurement.competing_reports -gt $c0 } 20000 'desktop bridge report rejected (competing_reports grows)'
    $st = Get-ScreenTime
    Assert-True ($st.measurement.reporter.kind -eq 'BACKGROUND_LOGON_TASK') 'desktop open: background helper stays the only reporter'
    $bridges = @(Get-Helpers 'bridge')
    Assert-True ($bridges.Count -eq 1) 'exactly one bridge helper (desktop child)'
    $dp.Refresh()
    Assert-True ($dp.CloseMainWindow()) 'WM_CLOSE to desktop'
    Assert-True ($dp.WaitForExit(15000)) 'desktop exits'
    Start-Sleep -Milliseconds 800
    Assert-True ((Get-Helpers 'bridge').Count -eq 0) 'bridge helper exited with the desktop'
    Assert-True ($null -ne (Get-Process -Id $bg.ProcessId -ErrorAction SilentlyContinue)) 'background helper still running after desktop close'
    # Simulated user input while the desktop is closed (CI has no keyboard):
    # only the background helper can report it.
    Add-Type -Namespace KdCi -Name Input -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, System.UIntPtr e);'
    $s1 = Get-ScreenTime
    for ($i = 0; $i -lt 15; $i++) {
        [KdCi.Input]::mouse_event(1, 1, 0, 0, [UIntPtr]::Zero)
        [KdCi.Input]::mouse_event(1, -1, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Seconds 2
    }
    $st = Get-ScreenTime
    $t1 = Total $s1; $t2 = Total $st
    $a1 = [int64]$s1.today.active_secs; $a2 = [int64]$st.today.active_secs
    Write-Host "desktop closed: total $t1 -> $t2 s, active $a1 -> $a2 s, report age $($st.measurement.last_report_age_ms) ms, state=$($st.state)"
    Assert-True ($st.measurement.reporter.kind -eq 'BACKGROUND_LOGON_TASK' -and [int]$st.measurement.last_report_age_ms -lt 12000) 'desktop closed: background reports continue'
    Assert-True ($a2 -ge $a1 + 10) "desktop closed: active use measured by the background helper ($a1 -> $a2 s)"
    $dp = Start-Process -FilePath $desk -PassThru
    Wait-Until { (Count-Desktop 'event=agent_link state=Connected') -ge 2 } 60000 'desktop reconnected'
    $t3 = Total (Get-ScreenTime)
    Assert-True ($t3 -ge $t2) "desktop reopened: totals kept ($t2 -> $t3)"
    $dp.Refresh(); $null = $dp.CloseMainWindow(); $null = $dp.WaitForExit(15000)
    $summary.Add("P1D desktop: bridge rejected as 2nd reporter; closed -> helper alive, injected input: active $a1 -> $a2 s; reopened -> total $t3 s kept")

    Write-Host '== 3. Single reporter / single instance =='
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p2 = Start-Process -FilePath $helper -ArgumentList 'activity', '--launch', 'manual' -PassThru -WindowStyle Hidden
    Assert-True ($p2.WaitForExit(10000)) "second activity instance exits ($($sw.ElapsedMilliseconds) ms)"
    Assert-True ($p2.ExitCode -eq 0) 'second instance exit code 0'
    Wait-Until { (Count-HelperLog 'event=already_running') -ge 1 } 5000 'helper log: already_running'
    $ack = (& $helper report-activity | Out-String | ConvertFrom-Json).payload
    Assert-True ($ack.applied -eq $false) 'one-shot report rejected while background helper reports'
    $summary.Add("P1D single instance: 2nd activity exit 0 (already_running); one-shot report applied=false")

    Write-Host '== 4. Helper killed -> released -> task restarts it =='
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Stop-Process -Id $bg.ProcessId -Force
    Wait-Until { $null -eq (Get-ScreenTime).measurement.reporter } 15000 'reporter released after kill'
    Write-Host "reporter released in $($sw.ElapsedMilliseconds) ms"
    Wait-Until { @(Get-Helpers 'activity' | Where-Object { $_.ProcessId -ne $bg.ProcessId }).Count -eq 1 } 180000 'logon task re-check restarted the helper'
    $restartMs = $sw.ElapsedMilliseconds
    $bg = (Get-Helpers 'activity')[0]
    Wait-Until { $s = Get-ScreenTime; $s.measurement.reporter.kind -eq 'BACKGROUND_LOGON_TASK' -and [int]$s.measurement.reporter.pid -eq [int]$bg.ProcessId } 30000 'restarted helper is the reporter'
    Wait-Until { (Count-HelperLog 'event=previous_exit state=unclean') -ge 1 } 10000 'helper log: previous_exit=unclean'
    $summary.Add("P1D helper kill: reporter released, task restarted helper in $([int]($restartMs/1000)) s (re-check 1 min), previous_exit=unclean logged")

    Write-Host '== 5. Agent service restart =='
    Start-Sleep -Seconds 16
    $before = Get-ScreenTime
    $connBefore = Count-HelperLog 'event=connected'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    sc.exe stop $svc | Out-Null
    Wait-Until { (Get-Service $svc).Status -eq 'Stopped' } 20000 'service stopped'
    sc.exe start $svc | Out-Null
    Wait-Until { (Get-Service $svc).Status -eq 'Running' } 20000 'service running'
    Wait-Until { (Count-HelperLog 'event=connected') -gt $connBefore } 30000 'helper reconnected'
    $after = $null
    Wait-Until { $script:after = Get-ScreenTime; $script:after.measurement.reporter.kind -eq 'BACKGROUND_LOGON_TASK' } 30000 'reporter back after restart'
    $gap = [int]($sw.ElapsedMilliseconds / 1000)
    $tb = Total $before; $ta = Total $script:after
    Write-Host "agent restart: total $tb -> $ta (gap $gap s)"
    Assert-True ($ta -ge $tb - 1) 'totals kept across agent restart'
    Assert-True ($ta -le $tb + $gap + 5) 'no double counting after agent restart'
    $summary.Add("P1D agent restart: helper reconnected in $gap s, totals $tb -> $ta s, no double counting")

    Write-Host '== 6. Idempotent re-install =='
    $r = Invoke-Script 'install-kindora-test.ps1' @('-AgentStartMode', 'Auto', '-EnableHelperAutostart', '-User', $me, '-HelperRepeatMinutes', '1')
    Assert-True ($r.Code -eq 0) 're-install exit 0'
    Assert-True (@(Get-ScheduledTask -TaskPath '\Kindora\').Count -eq 1) 're-install: still one task'
    Wait-Until { (Get-Helpers 'activity').Count -eq 1 } 30000 're-install: one background helper'
    $st = Get-ScreenTime
    Assert-True ((Total $st) -ge $ta - 16) 're-install keeps recorded totals'
    Assert-True ($st.storage.persistent -and -not $st.storage.recovered) 're-install: same SQLite database'
    $summary.Add("P1D re-install: idempotent (1 task, 1 helper), database kept")

    Write-Host '== 7. Uninstall (keep data), then delete data =='
    $r = Invoke-Script 'uninstall-kindora-test.ps1' @()
    Assert-True ($r.Code -eq 0) 'uninstall exit 0'
    sc.exe query $svc | Out-Null
    Assert-True ($LASTEXITCODE -eq 1060) 'service deleted (1060)'
    Assert-True (@(Get-ScheduledTask -TaskPath '\Kindora\' -ErrorAction SilentlyContinue).Count -eq 0) 'logon task removed'
    Assert-True ((Get-Helpers '').Count -eq 0) 'no kindora-helper process left'
    Assert-True (-not (Test-Path $dir)) 'program folder removed'
    Assert-True (Test-Path $db) 'database kept without -DeleteData'
    $r = Invoke-Script 'uninstall-kindora-test.ps1' @('-DeleteData', '-Yes')
    Assert-True ($r.Code -eq 0) 'second uninstall (idempotent) with -DeleteData -Yes'
    Assert-True (-not (Test-Path $data)) 'data removed only with -DeleteData'
    $summary.Add("P1D uninstall: 1060, task/process/files removed, data kept; -DeleteData -Yes removes data; idempotent")
}
finally {
    $ErrorActionPreference = 'Continue'
    Write-Host '== Helper log =='
    Get-HelperLog | ForEach-Object { Write-Host $_ }
    Write-Host '== Agent log (screen_time / power / reporter) =='
    if (Test-Path $agentLog) { Get-Content $agentLog | Where-Object { $_ -match 'REPORTER|power_event|agent_started|agent_stopped' } | ForEach-Object { Write-Host $_ } }
    if ($taskName) {
        $i = Get-ScheduledTaskInfo -TaskPath '\Kindora\' -TaskName $taskName -ErrorAction SilentlyContinue
        if ($i) { Write-Host ("task last result=0x{0:X} last run={1}" -f $i.LastTaskResult, $i.LastRunTime) }
    }
    if (Test-Path $dir) { & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PackageDir 'uninstall-kindora-test.ps1') -DeleteData -Yes | Out-Host }
}

if ($SummaryFile) { $summary | Add-Content -Path $SummaryFile -Encoding ASCII }
$summary | ForEach-Object { Write-Host $_ }
$global:LASTEXITCODE = 0
