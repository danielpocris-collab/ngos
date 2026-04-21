param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Log file not found: $LogPath"
}

$text = Get-Content -LiteralPath $LogPath -Raw
$required = @(
    "boot.proof=bus",
    "bus.smoke.refusal path=/proc/system/bus contract=observe outcome=expected",
    "bus.smoke.observe path=/proc/system/bus",
    "path=/ipc/render capacity=64 outcome=ok",
    "bus.smoke.attach peer=",
    "kind=attached outcome=ok",
    "bus.smoke.success peer=",
    "payload=hello-qemu outcome=ok",
    "bus.smoke.overflow peer=",
    "errno=Again",
    "peak=64 overflows=1 outcome=ok",
    "bus.smoke.detach peer=",
    "bus.smoke.refusal peer=",
    "outcome=expected",
    "bus.smoke.recovery peer=",
    "payload=recovered-qemu outcome=ok",
    "bus.smoke.state peer=",
    "publishes=67 receives=67",
    "bus-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing bus marker: $marker"
    }
}

Write-Host "QEMU bus log markers verified."
Write-Host "Log: $LogPath"
