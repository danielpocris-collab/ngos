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
    "boot.proof=network-hardware-tx",
    "virtio-net online mac=",
    "network.hw.tx.refusal path=/dev/net0 errno=EINVAL outcome=expected",
    "virtio-net raw tx queued len=",
    "network.hw.tx.success path=/dev/net0",
    "network-hardware-tx-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing network hardware tx marker: $marker"
    }
}

Write-Host "QEMU network hardware tx log markers verified."
Write-Host "Log: $LogPath"
