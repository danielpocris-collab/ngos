param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath,
    [int]$MinimumCpuCount = 2
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Log file not found: $LogPath"
}

$text = Get-Content -LiteralPath $LogPath -Raw
$required = @(
    "boot.proof=scheduler",
    "scheduler.smoke.refusal path=/proc/system/scheduler contract=observe outcome=expected",
    "scheduler.smoke.observe path=/proc/system/scheduler tokens=yes wait-ticks=yes lag=yes fairness=yes decisions=yes running=yes cpu=yes cpu-topology=yes cpu-queue=yes rebalance=yes outcome=ok",
    "scheduler.smoke.spawn pid=",
    "scheduler.smoke.balance pid=",
    "scheduler.smoke.renice pid=",
    "scheduler.smoke.pause pid=",
    "scheduler.smoke.resume pid=",
    "scheduler.smoke.queue pid=",
    "scheduler.smoke.fairness dispatch=yes runtime=yes imbalance=yes outcome=ok",
    "scheduler.smoke.cpu count=",
    "scheduler.smoke.episodes affinity=yes dispatch=yes causal=yes outcome=ok",
    "scheduler.smoke.recovery pid=",
    "scheduler.smoke.state pid=",
    "scheduler-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing scheduler marker: $marker"
    }
}

$cpuSummaryMatch = [regex]::Match($text, 'scheduler\.smoke\.cpu count=(\d+) ')
if (-not $cpuSummaryMatch.Success) {
    throw "Missing scheduler smoke cpu count marker."
}

$observedCpuCount = [int]$cpuSummaryMatch.Groups[1].Value
if ($observedCpuCount -lt $MinimumCpuCount) {
    throw "Scheduler cpu-summary count $observedCpuCount is below required minimum $MinimumCpuCount."
}

$rebalanceMatch = [regex]::Match($text, 'scheduler\.smoke\.cpu count=\d+ .*?migrations=(\d+) last-rebalance=(\d+) outcome=ok')
if (-not $rebalanceMatch.Success) {
    throw "Missing scheduler smoke rebalance evidence marker."
}

$observedMigrations = [int]$rebalanceMatch.Groups[1].Value
$observedLastRebalance = [int]$rebalanceMatch.Groups[2].Value
if ($MinimumCpuCount -ge 2 -and ($observedMigrations -lt 1 -or $observedLastRebalance -lt 1)) {
    throw "Scheduler rebalance evidence is too weak: migrations=$observedMigrations last-rebalance=$observedLastRebalance."
}

Write-Host "QEMU scheduler log markers verified."
Write-Host "Log: $LogPath"
