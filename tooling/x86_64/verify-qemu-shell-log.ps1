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
    "boot.proof=shell",
    "shell.smoke.session protocol=kernel-launch cwd=/ outcome=ok",
    "shell.smoke.ux suggest=pro apropos=mount explain=identity-of unknown=feedback outcome=ok",
    "shell.smoke.ergonomics topic=pipeline examples=identity-of repeat=yes rerun=yes recent=yes next=review outcome=ok",
    "shell.smoke.scripting path=/shell-proof/note bytes=14 source=yes outcome=ok",
    "shell.smoke.lang return=shell-proof-lang argc=1 outcome=ok",
    "shell.smoke.match result=matched value=shell-proof-lang outcome=ok",
    "shell.smoke.values type=record path=src/lib.rs outcome=ok",
    "shell.smoke.pipeline path=src/lib.rs type=string outcome=ok",
    "shell.smoke.pipeline-real source=session outcome=ok",
    "shell.smoke.pipeline-system pid=1 outcome=ok",
    "shell.smoke.pipeline-list count=",
    "shell.smoke.pipeline-waiters count=",
    "shell.smoke.pipeline-mount path=/shell-proof-mount",
    "shell.smoke.pipeline-storage generation=",
    "shell.smoke.pipeline-storage-prepare generation=",
    "shell.smoke.pipeline-storage-recover generation=",
    "shell.smoke.pipeline-storage-repair generation=",
    "shell.smoke.pipeline-storage-volume generation=",
    "shell.smoke.pipeline-storage-range count=",
    "shell.smoke.pipeline-storage-tail count=",
    "storage-prepare path=/dev/storage0 generation=",
    "storage-recover path=/dev/storage0 generation=",
    "storage-repair path=/dev/storage0 generation=",
    "storage-volume path=/dev/storage0 valid=",
    "pipeline-source stage=storage-volume path=/dev/storage0 type=record fields=",
    "storage-history-range path=/dev/storage0 start=0 count=3",
    "pipeline-source stage=storage-history-range-of path=/dev/storage0 start=0 count=3 type=list items=",
    "storage-history-tail path=/dev/storage0 count=3",
    "pipeline-source stage=storage-history-tail-of path=/dev/storage0 count=3 type=list items=",
    "storage-lineage path=/dev/storage0 generation=",
    "storage-history path=/dev/storage0 count=",
    "storage-history-entry path=/dev/storage0 index=0 generation=",
    "shell.smoke.pipeline-filter count=1 outcome=ok",
    "shell.smoke.pipeline-inventory domains=1 resources=1 contracts=1 outcome=ok",
    "shell.smoke.pipeline-netif path=/dev/net0 addr=10.1.0.2 admin=up outcome=ok",
    "shell.smoke.pipeline-netsock path=/run/net0.sock port=4020 connected=yes outcome=ok",
    "shell.smoke.pipeline-queues epoll=1 kqueue=1 outcome=ok",
    "shell.smoke.pipeline-fd source=list kind=File outcome=ok",
    "shell.smoke.pipeline-maps pid=1 source=list outcome=ok",
    "shell.smoke.pipeline-vm objects=",
    "shell.smoke.pipeline-ops queue-suffix=",
    "shell.smoke.pipeline-query key=",
    "shell.smoke.pipeline-bool contains=true starts=true ends=true not=false empty=true outcome=ok",
    "shell.smoke.pipeline-caps count=",
    "shell.smoke.pipeline-signals pending=",
    "shell.smoke.pipeline-interop record-count=",
    "shell.smoke.pipeline-recordops owner=",
    "shell.smoke.pipeline-process capability-count=",
    "shell.smoke.pipeline-compat route=",
    "shell.smoke.pipeline-identity uid=",
    "shell.smoke.pipeline-recordpredicates identity=true compat=true outcome=ok",
    "shell.smoke.pipeline-auxv count=",
    "shell.smoke.pipeline-procfs status=",
    "shell.smoke.pipeline-vfsstats nodes=",
    "shell.smoke.pipeline-jobs count=",
    "shell.smoke.pipeline-mounts count=1 outcome=ok",
    "shell.smoke.pipeline-listfields mount-device=1 mount-mode=2 auxv-exec=",
    "shell.smoke.pipeline-listpredicates any=true all=true outcome=ok",
    "shell.smoke.coding build=/shell-proof/build.log test=/shell-proof/test.log outcome=ok",
    "shell.smoke.review left=/shell-proof/review.before right=/shell-proof/review.after outcome=ok",
    "process-spawned pid=",
    "job-info pid=",
    "foreground-complete pid=",
    "shell.smoke.jobs pid=",
    "shell.smoke.observe pid=1 procfs=stat-open outcome=ok",
    "shell.smoke.refusal pid=1 command=missing-command outcome=expected",
    "shell.smoke.recovery pid=1 guard=or outcome=ok",
    "shell.smoke.state pid=1 cwd=/ note=/shell-proof/note outcome=ok",
    "shell-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing shell marker: $marker"
    }
}

Write-Host "QEMU shell log markers verified."
