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
    "boot.proof=network-hardware",
    "virtio-net online mac=",
    "virtio-net summary tx_completions=",
    "network.hw.device path=/dev/net0",
    "submitted=",
    "completed=",
    "network.hw.driver path=/drv/net0",
    "network-hardware-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing network hardware marker: $marker"
    }
}

Write-Host "QEMU network hardware log markers verified."
Write-Host "Log: $LogPath"
