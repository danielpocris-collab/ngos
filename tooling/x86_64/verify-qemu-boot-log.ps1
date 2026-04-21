param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "QEMU boot serial log not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$markers = @(
    "ngos/x86_64: stage0 entered",
    "ngos/x86_64: bootloader = Limine ",
    "ngos/x86_64: early_kernel_main reached",
    "ngos/x86_64: framebuffer console online",
    "ngos/x86_64: protocol=Limine hhdm=",
    "ngos/x86_64: post-paging handoff regions=",
    'ngos/x86_64: entering user mode module="/kernel/ngos-userland-native"',
    "ngos/x86_64: boot report handled status=0 stage=2 code=0",
    "FIRST USER PROCESS REPORT",
    "ngos/x86_64: first user process report disposition=exited outcome=success",
    "ngos/x86_64: first user process boot outcome policy=RequireZeroExit outcome=success action=halt-success exit_code=0"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing QEMU boot markers: " + ($missing -join " | "))
}

Write-Host "QEMU boot log markers verified."
Write-Host "Log: $LogPath"
