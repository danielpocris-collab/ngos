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
    "vfs.smoke.recovery pid=1 path=/vfs/link target=/vfs/bin/app rename-restored=yes readlink=stable outcome=ok",
    "vfs.smoke.refusal pid=1 create-missing-parent=yes unlink-nonempty-dir=yes outcome=ok",
    "vfs.smoke.symlink-loop pid=1 refusal=loop-detected yes recovery=unlink outcome=ok",
    "vfs.smoke.file pid=1 path=/vfs/bin/app copy=/vfs/bin/app-copy bytes=16 append=yes copy-match=yes outcome=ok",
    "vfs.smoke.link pid=1 source=/vfs/bin/app link=/vfs/bin/app-link shared-inode=yes shared-write=yes links-before=2 links-after=1 unlink-released=yes outcome=ok",
    "vfs.smoke.truncate pid=1 path=/vfs/bin/app-copy shrink=5 extend=8 zero-fill=yes outcome=ok",
    "vfs.smoke.unlink-open pid=1 path=/vfs/bin/live fd=",
    "vfs.smoke.vm-file pid=1 path=/vfs/bin/vm-file sync=yes truncate-reflects=yes unlink-survives=yes unmap=yes outcome=ok",
    "vfs.smoke.permissions pid=1 dir=/vfs/secure file=/vfs/secure/secret.txt list-blocked=yes traverse-blocked=yes rename-blocked=yes unlink-blocked=yes file-read-blocked=yes recovery=yes outcome=ok",
    "vfs.smoke.replace pid=1 source=/vfs/bin/replace-src target=/vfs/bin/replace-dst file-replaced=yes open-target-survives=yes nonempty-dir-refusal=yes empty-dir-replaced=yes kind-mismatch-refusal=yes outcome=ok",
    "vfs.smoke.tree pid=1 source=/vfs/tree-src copy=/vfs/tree-dst mirror=/vfs/tree-mirror refusal=self-nest yes symlink=stable pruned=yes outcome=ok",
    "vfs.smoke.mount-propagation pid=1 shared=/vfs/mount-shared peer=/vfs/mount-peer child=/vfs/mount-shared/child clone=/vfs/mount-peer/child",
    "cross-mount-rename=blocked cross-mount-link=blocked parent-unmount-blocked=yes recovery=yes outcome=ok",
    "vfs.smoke.list pid=1 path=/vfs/bin entries=2 names=app,app-copy outcome=ok",
    "vfs.smoke.fd pid=1 fd=",
    "vfs.smoke.dup pid=1 fd=",
    "vfs.smoke.fcntl pid=1 fd=",
    "vfs.smoke.lock pid=1 primary-fd=",
    "shared=yes shared-refusal=busy mutation-blocked=yes mutation-recovery=yes shared-recovery=yes",
    "vfs.smoke.coherence pid=1 descriptor=open-path-open readlink=stable statfs=ok outcome=ok",
    "vfs-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing VFS marker: $marker"
    }
}

Write-Host "QEMU VFS log markers verified."
