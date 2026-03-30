param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path $LogPath)) {
    throw "Log file not found: $LogPath"
}

$text = Get-Content -Path $LogPath -Raw
$required = @(
    "boot.proof=vfs",
    "vfs.smoke.mount pid=1 path=/vfs",
    "vfs.smoke.create pid=1 path=/vfs/bin/app",
    "vfs.smoke.symlink pid=1 path=/vfs/link target=/vfs/bin/app",
    "vfs.smoke.rename pid=1 from=/vfs/bin/app to=/vfs/bin/app2",
    "vfs.smoke.unlink pid=1 path=/vfs/link after-unlink=missing",
    "vfs.smoke.coherence pid=1 descriptor=open-path-open readlink=stable statfs=ok outcome=ok",
    "vfs-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing VFS marker: $marker"
    }
}

Write-Host "QEMU VFS log markers verified."
