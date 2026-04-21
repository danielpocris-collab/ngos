Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$bootNetworkRuntime = Join-Path $RepoRoot "boot-x86_64\src\boot_network_runtime.rs"
$virtioNetBoot = Join-Path $RepoRoot "boot-x86_64\src\virtio_net_boot.rs"
$proveScript = Join-Path $PSScriptRoot "prove-qemu-device-runtime-smoke.ps1"
$serialLog = Join-Path $RepoRoot "target\qemu\serial-device-runtime.log"

foreach ($path in @($bootNetworkRuntime, $virtioNetBoot, $proveScript, $serialLog)) {
    if (!(Test-Path -LiteralPath $path)) {
        throw "Required path not found: $path"
    }
}

$bootRuntimeText = Get-Content -LiteralPath $bootNetworkRuntime -Raw
$virtioText = Get-Content -LiteralPath $virtioNetBoot -Raw
$proveText = Get-Content -LiteralPath $proveScript -Raw
$serialText = Get-Content -LiteralPath $serialLog -Raw

if (-not $bootRuntimeText.Contains('pub const NETWORK_DEVICE_PATH: &str = "/dev/net0";')) {
    throw "boot_network_runtime no longer exposes a single fixed /dev/net0 path."
}
if (-not $bootRuntimeText.Contains('pub const NETWORK_DRIVER_PATH: &str = "/drv/net0";')) {
    throw "boot_network_runtime no longer exposes a single fixed /drv/net0 path."
}
if (-not $virtioText.Contains('NETWORK_DEVICE_PATH') -or -not $virtioText.Contains('NETWORK_DRIVER_PATH')) {
    throw "virtio_net_boot no longer appears anchored to the single-interface boot path."
}

$netdevMatches = [regex]::Matches($proveText, '-netdev')
if ($netdevMatches.Count -ne 0) {
    throw "Expected device-runtime proof script to rely on shared single-interface staging rather than declaring multiple explicit -netdev entries; found $($netdevMatches.Count)."
}

$virtioMatches = [regex]::Matches($proveText, 'virtio-net-pci')
if ($virtioMatches.Count -ne 0) {
    throw "Expected device-runtime proof script to avoid inline multi-interface qemu wiring; found $($virtioMatches.Count) explicit virtio-net-pci entries."
}

$deviceMarkers = [regex]::Matches($serialText, 'device\.runtime\.smoke\.[^\r\n]+')
if ($deviceMarkers.Count -eq 0) {
    throw "serial-device-runtime.log does not contain device.runtime.smoke markers."
}
if ($serialText.Contains('/dev/net1') -or $serialText.Contains('/drv/net1')) {
    throw "serial-device-runtime.log already shows a second interface path."
}

Write-Host "Device runtime single-interface blocker verified."
Write-Host "boot owner: /dev/net0 + /drv/net0"
Write-Host "QEMU harness: no explicit multi-interface wiring in prove-qemu-device-runtime-smoke.ps1"
Write-Host "QEMU evidence: no /dev/net1 marker in serial-device-runtime.log"
