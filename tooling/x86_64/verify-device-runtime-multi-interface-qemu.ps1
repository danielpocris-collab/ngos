Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$bootNetworkRuntime = Join-Path $RepoRoot "boot-x86_64\src\boot_network_runtime.rs"
$serialLog = Join-Path $RepoRoot "target\qemu\serial-device-runtime.log"

foreach ($path in @($bootNetworkRuntime, $serialLog)) {
    if (!(Test-Path -LiteralPath $path)) {
        throw "Required path not found: $path"
    }
}

$bootRuntimeText = Get-Content -LiteralPath $bootNetworkRuntime -Raw
$serialText = Get-Content -LiteralPath $serialLog -Raw

if (-not $bootRuntimeText.Contains('path.strip_prefix("/dev/net")')) {
    throw "boot_network_runtime does not expose generic /dev/netN parsing."
}
if (-not $bootRuntimeText.Contains('path.strip_prefix("/drv/net")')) {
    throw "boot_network_runtime does not expose generic /drv/netN parsing."
}

$required = @(
    "network.smoke.multi iface0=10.1.0.2 iface1=10.2.0.2",
    "network.smoke.refusal interface=/dev/net1 state=link-down errno=EACCES outcome=expected",
    "network.smoke.teardown socket=/run/net0.sock attached-sockets=0 sibling-sockets=1 outcome=ok"
)

foreach ($marker in $required) {
    if (-not $serialText.Contains($marker)) {
        throw "Missing multi-interface QEMU marker: $marker"
    }
}

Write-Host "Device runtime multi-interface QEMU evidence verified."
Write-Host "Boot owner: generic /dev/netN + /drv/netN paths"
Write-Host "Serial log: $serialLog"
