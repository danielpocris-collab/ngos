param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Log file not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$markers = @(
    "vm.smoke.map pid=1",
    "vm.smoke.protect pid=1",
    "vm.smoke.heap pid=1 kind=heap grew=yes shrank=yes",
    "vm.smoke.region pid=1 kind=region protected=yes unmapped=yes",
    "vm.smoke.cow.observe pid=2 source=1 object=[cow] depth=1 kind=fault cow=yes",
    "vm.smoke.cow pid=2 source=1",
    "vm.smoke.unmap pid=1",
    "vm.smoke.stress pid=1 cycles=12 refusals=12 outcome=ok",
    "vm.smoke.pressure pid=1 target-pages=3",
    "restored=yes outcome=ok",
    "vm.smoke.pressure.global pid=1 child=",
    "victim=libvm-global-a survivor=libvm-global-b outcome=ok",
    "vm.smoke.advise pid=1 path=/lib/libvm-advise.so advised=yes outcome=ok",
    "vm.smoke.quarantine pid=1 path=/lib/libvm-quarantine.so blocked=yes released=yes outcome=ok",
    "vm.smoke.policy pid=1 contract=",
    "blocked=yes resumed=yes outcome=ok",
    "vm.smoke.production pid=1 stress=yes pressure=yes global-pressure=yes advise=yes quarantine=yes policy=yes workloads=anon,cow,file,heap,region outcome=ok",
    "vfs.smoke.mount pid=1 path=/vfs mounts=",
    "vfs.smoke.create pid=1 path=/vfs/bin/app",
    "vfs.smoke.symlink pid=1 path=/vfs/link target=/vfs/bin/app",
    "vfs.smoke.rename pid=1 from=/vfs/bin/app to=/vfs/bin/app2 refusal=invalid-subtree yes outcome=ok",
    "vfs.smoke.unlink pid=1 path=/vfs/link after-unlink=missing outcome=ok",
    "vfs.smoke.coherence pid=1 descriptor=open-path-open readlink=stable statfs=ok outcome=ok"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing QEMU VM markers: " + ($missing -join " | "))
}

Write-Host "QEMU VM log markers verified."
Write-Host "Log: $LogPath"
