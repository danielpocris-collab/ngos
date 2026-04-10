param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Log file not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$markers = @(
    "boot.proof=bus",
    "bus.smoke.refusal path=/proc/system/bus contract=observe outcome=expected",
    "bus.smoke.observe path=/proc/system/bus",
    "capacity=64 outcome=ok",
    "bus.smoke.attach peer=",
    "kind=attached outcome=ok",
    "bus.smoke.success peer=",
    "payload=hello-qemu outcome=ok",
    "bus.smoke.overflow peer=",
    "errno=Again",
    "peak=64 overflows=1 outcome=ok",
    "bus.smoke.detach peer=",
    "bus.smoke.refusal peer=",
    "errno=Inval outcome=expected",
    "bus.smoke.recovery peer=",
    "payload=recovered-qemu outcome=ok",
    "bus.smoke.state peer=",
    "attached=1 depth=0 publishes=67 receives=67 peak=64 overflows=1 outcome=ok",
    "bus-smoke-ok",
    "ngos/x86_64: boot report handled status=0 stage=2 code=0"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing hardware bus markers: " + ($missing -join " | "))
}

Write-Host "Hardware bus log markers verified."
Write-Host "Log: $LogPath"
