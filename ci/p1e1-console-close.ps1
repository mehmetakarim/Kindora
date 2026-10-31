# Kindora P1E.1 - OBS-011: does the background helper exit cleanly when its
# console is closed (CTRL_CLOSE_EVENT)?
#
# Real Windows event, no simulation: the helper runs under its own conhost
# console window (like the logon task, which uses conhost --headless); the
# console window receives WM_CLOSE, conhost delivers CTRL_CLOSE_EVENT to the
# helper and Windows terminates the helper after its handler returns (or after
# the system timeout).
#
# A clean exit means: "event=helper_stopped" logged and the run marker
# (<log>.running) removed. Without -ExpectClean the script only REPORTS the
# outcome (reproduction step); with -ExpectClean it fails on an unclean exit.
# No agent is needed: the helper's reconnect loop is enough.
# Kept ASCII-only for Windows PowerShell 5.1.

param(
    [Parameter(Mandatory = $true)][string]$Helper,
    [switch]$ExpectClean,
    [int]$Rounds = 3,
    [string]$SummaryFile = ''
)

$ErrorActionPreference = 'Stop'
$summary = New-Object System.Collections.Generic.List[string]
$helperPath = (Resolve-Path $Helper).Path
$conhost = Join-Path $env:SystemRoot 'System32\conhost.exe'
$clean = 0

for ($r = 1; $r -le $Rounds; $r++) {
    $dir = Join-Path $env:RUNNER_TEMP "obs011-$r"
    Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory $dir | Out-Null
    $log = Join-Path $dir 'activity.log'
    $marker = "$log.running"

    # conhost owns the visible console window; the helper is its client.
    $host_ = Start-Process -FilePath $conhost -ArgumentList "`"$helperPath`" activity --launch manual --log-file `"$log`"" -PassThru
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path $log) -and (Get-Content $log | Where-Object { $_.Contains('event=helper_started') }))) {
        if ($sw.ElapsedMilliseconds -gt 15000) { throw "helper did not start (round $r)" }
        Start-Sleep -Milliseconds 100
    }
    $h = @(Get-CimInstance Win32_Process -Filter "Name='kindora-helper.exe'" | Where-Object { $_.CommandLine -match [regex]::Escape($log) })
    if ($h.Count -ne 1) { throw "expected one helper for round $r, found $($h.Count)" }
    $hp = Get-Process -Id $h[0].ProcessId
    Start-Sleep -Milliseconds 1500
    Write-Host "round $r : helper pid=$($hp.Id) parent(conhost)=$($host_.Id) marker_exists=$(Test-Path $marker)"

    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($host_.MainWindowHandle -eq [IntPtr]::Zero -and $sw.ElapsedMilliseconds -lt 10000) { Start-Sleep -Milliseconds 100; $host_.Refresh() }
    if ($host_.MainWindowHandle -eq [IntPtr]::Zero) { throw "conhost has no console window (round $r)" }

    $sw = [Diagnostics.Stopwatch]::StartNew()
    $null = $host_.CloseMainWindow()   # WM_CLOSE -> CTRL_CLOSE_EVENT to the helper
    if (-not $hp.WaitForExit(10000)) { throw "helper still running 10 s after console close (round $r)" }
    $exitMs = $sw.ElapsedMilliseconds
    Start-Sleep -Milliseconds 300
    $lines = @(Get-Content $log)
    $stopped = @($lines | Where-Object { $_.Contains('event=helper_stopped') }).Count -ge 1
    $requested = ($lines | Where-Object { $_.Contains('event=shutdown_requested') } | Select-Object -First 1)
    $markerLeft = Test-Path $marker
    $ok = $stopped -and -not $markerLeft
    if ($ok) { $clean++ }
    Write-Host "round $r : exit after $exitMs ms, helper_stopped=$stopped marker_left=$markerLeft shutdown_requested=[$requested]"
    $lines | ForEach-Object { Write-Host "  log> $_" }
    $summary.Add("P1E1 console close round $r : exit $exitMs ms, helper_stopped=$stopped, marker_left=$markerLeft -> $(if ($ok) { 'CLEAN' } else { 'UNCLEAN' })")
    if (-not $host_.HasExited) { Stop-Process -Id $host_.Id -Force -ErrorAction SilentlyContinue }
}

$summary.Add("P1E1 console close: $clean/$Rounds clean exits (ExpectClean=$ExpectClean)")
if ($SummaryFile) { $summary | Add-Content -Path $SummaryFile -Encoding ASCII }
$summary | ForEach-Object { Write-Host $_ }
if ($ExpectClean -and $clean -ne $Rounds) { throw "FAILED: $($Rounds - $clean) of $Rounds console closes were unclean" }
$global:LASTEXITCODE = 0
