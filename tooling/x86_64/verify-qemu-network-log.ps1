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
    "boot.proof=network",
    "network.smoke.success",
    "network.smoke.multi iface0=10.1.0.2 iface1=10.2.0.2",
    "network.smoke.rx remote=10.1.0.9:5000 bytes=10 payload=reply-qemu",
    "network.smoke.refusal interface=/dev/net1 state=link-down errno=EACCES outcome=expected",
    "network.smoke.teardown socket=/run/net0.sock attached-sockets=0 sibling-sockets=1 outcome=ok",
    "network.smoke.teardown socket=/run/net1.sock attached-sockets=0 sibling-sockets=0 outcome=ok",
    "network.smoke.rebind socket=/run/net0.sock local=10.1.0.2:4010 attached-sockets=1 outcome=ok",
    "network.smoke.rebind socket=/run/net1.sock local=10.2.0.2:4110 attached-sockets=1 outcome=ok",
    "network.smoke.recovery local=10.1.0.2:4000",
    "network.smoke.recovery local=10.2.0.2:4100",
    "network-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing network marker: $marker"
    }
}

Write-Host "QEMU network log markers verified."
Write-Host "Log: $LogPath"
