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
    "ngos/x86_64: stage0 entered",
    "ngos/x86_64: early_kernel_main reached",
    "ngos/x86_64: framebuffer console online",
    "ngos/x86_64: boot report handled status=0 stage=2 code=0",
    "ngos/x86_64: exit syscall handled code=0",
    "ngos/x86_64: process exit propagated code=0 exited=true"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing hardware boot markers: " + ($missing -join " | "))
}

Write-Host "Hardware boot log markers verified."
Write-Host "Log: $LogPath"
