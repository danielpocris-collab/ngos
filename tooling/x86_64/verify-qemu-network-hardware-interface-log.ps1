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
    "boot.proof=network-hardware-interface",
    "network.hw.iface.inspect path=/dev/net0",
    "network.hw.iface.refusal path=/dev/net0 state=link-down errno=EACCES outcome=expected",
    "network.hw.iface.success path=/dev/net0",
    "network-hardware-interface-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing network hardware interface marker: $marker"
    }
}

Write-Host "QEMU network hardware interface log markers verified."
Write-Host "Log: $LogPath"
