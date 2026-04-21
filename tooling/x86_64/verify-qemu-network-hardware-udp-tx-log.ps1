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
    "boot.proof=network-hardware-udp-tx",
    "network.hw.udp.success socket=/run/net0.sock",
    "network.hw.udp.refusal socket=/run/net0.sock state=link-down errno=EACCES outcome=expected",
    "network.hw.udp.recovery socket=/run/net0.sock",
    "network-hardware-udp-tx-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing network hardware UDP TX marker: $marker"
    }
}

Write-Host "QEMU network hardware UDP TX log markers verified."
Write-Host "Log: $LogPath"
