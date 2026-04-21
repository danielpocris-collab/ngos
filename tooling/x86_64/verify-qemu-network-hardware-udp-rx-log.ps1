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
    "boot.proof=network-hardware-udp-rx",
    "network.hw.udp-rx.send-complete socket=/run/net0.sock",
    "network.hw.udp-rx.success socket=/run/net0.sock",
    "network.hw.udp-rx.refusal socket=/run/net0.sock state=link-down errno=EACCES outcome=expected",
    "network.hw.udp-rx.recovery socket=/run/net0.sock",
    "network-hardware-udp-rx-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing network hardware UDP RX marker: $marker"
    }
}

Write-Host "QEMU network hardware UDP RX log markers verified."
Write-Host "Log: $LogPath"
