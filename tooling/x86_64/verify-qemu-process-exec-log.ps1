param(
    [Parameter(Mandatory = $true)]
    [string] $LogPath
)

$markers = @(
    "boot.proof=process-exec",
    "process.exec.smoke.recovery pid=",
    "outcome=ok",
    "process.exec.smoke.spawn pid=",
    "mode=same-image-blocking outcome=ok",
    "compat.abi.smoke.proc.success pid=",
    "fd-count=",
    "fd0=present fd1=present fd2=present",
    "cmdline=present",
    "compat.abi.smoke.proc.environ pid=",
    "outcome=ok marker=NGOS_COMPAT_TARGET=process-exec",
    "compat.abi.smoke.proc.refusal pid=",
    "/fd/9999 outcome=expected",
    "compat.abi.smoke.proc.recovery pid=",
    "fd-list=ok outcome=ok",
    "process.exec.smoke.success pid=",
    "exit=0 outcome=ok",
    "process.exec.smoke.state pid=",
    "present=no outcome=ok",
    "process-exec-smoke-ok"
)

$content = Get-Content -Path $LogPath -Raw
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        throw "Missing QEMU process exec marker: $marker"
    }
}

$hasMetadataRefusal = $content.Contains("process.exec.smoke.refusal pid=") -and
    $content.Contains("mode=metadata-only outcome=expected")
$hasMetadataObserve = $content.Contains("process.exec.smoke.observe pid=") -and
    $content.Contains("mode=metadata-only") -and
    $content.Contains("outcome=ok")

if (-not ($hasMetadataRefusal -or $hasMetadataObserve)) {
    throw "Missing QEMU process exec metadata marker family: refusal or observe."
}

Write-Host "QEMU process exec log markers verified."
Write-Host "Log: $LogPath"
