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
    "boot.proof=device-runtime",
    "device.runtime.smoke.graphics device=/dev/gpu0",
    "device.runtime.smoke.audio device=/dev/audio0",
    "device.runtime.smoke.input device=/dev/input0",
    "network.smoke.success",
    "network.smoke.multi iface0=10.1.0.2 iface1=10.2.0.2",
    "network.smoke.refusal interface=/dev/net1 state=link-down errno=EACCES outcome=expected",
    "network.smoke.recovery local=10.1.0.2:4000",
    "sibling-sockets=1 outcome=ok",
    "storage.smoke.mount.commit mount=/persist",
    "storage-commit-smoke-ok",
    "device.runtime.smoke.storage device=/dev/storage0",
    "device-runtime-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing device-runtime marker: $marker"
    }
}

Write-Host "QEMU device-runtime log markers verified."
Write-Host "Log: $LogPath"
