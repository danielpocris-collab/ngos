param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path $LogPath)) {
    throw "Storage serial log not found: $LogPath"
}

$text = Get-Content -Path $LogPath -Raw
$required = @(
    "boot.proof=storage-commit",
    "storage.smoke.success generation=",
    "storage.smoke.mapping.refusal op=unmount errno=E2BIG outcome=expected",
    "storage.smoke.mount.commit mount=/persist",
    "files=8",
    "dirs=2",
    "symlinks=1",
    "alloc-total=",
    "storage.smoke.refusal op=prepare errno=E2BIG outcome=expected",
    "storage.smoke.mount.refusal op=unmount errno=ENOENT outcome=expected",
    "storage-commit-smoke-ok",
    "boot.proof=storage-recover",
    "storage.smoke.prepared generation=",
    "storage.smoke.recovery generation=",
    "state=recovered",
    "storage.smoke.mount.recovery mount=/persist",
    "payload=persist:qemu-vfs-session-001",
    "asset-bytes=900",
    "alloc-total=",
    "alloc-used=10",
    "extents=10",
    "storage-recover-smoke-ok",
    "boot.proof=storage-corrupt",
    "storage.smoke.corruption sector=",
    "storage.smoke.corruption.refusal op=mount errno=EINVAL outcome=expected",
    "storage.smoke.corruption.repair generation=",
    "alloc-total=",
    "files=8",
    "dirs=2",
    "symlinks=1",
    "extents=10",
    "storage.smoke.corruption.recovery mount=/persist",
    "storage-corrupt-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing storage proof marker: $marker"
    }
}

Write-Host "QEMU storage proof verified."
