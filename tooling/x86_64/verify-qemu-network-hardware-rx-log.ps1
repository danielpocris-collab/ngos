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
    "boot.proof=network-hardware-rx",
    "virtio-net online mac=",
    "network.hw.rx.refusal path=/dev/net0 errno=EAGAIN outcome=expected",
    "virtio-net raw tx queued len=",
    "virtio-net raw read copied=",
    "network.hw.rx.success path=/dev/net0",
    "network-hardware-rx-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing network hardware rx marker: $marker"
    }
}

Write-Host "QEMU network hardware rx log markers verified."
Write-Host "Log: $LogPath"
