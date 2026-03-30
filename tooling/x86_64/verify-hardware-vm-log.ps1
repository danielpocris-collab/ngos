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
    "ngos/x86_64: boot report handled status=0 stage=2 code=0",
    "vm.smoke.map pid=1",
    "vm.smoke.protect pid=1",
    "vm.smoke.heap pid=1 kind=heap grew=yes shrank=yes",
    "vm.smoke.region pid=1 kind=region protected=yes unmapped=yes",
    "vm.smoke.cow.observe pid=2 source=1 object=[cow] depth=1 kind=fault cow=yes",
    "vm.smoke.cow pid=2 source=1",
    "vm.smoke.unmap pid=1"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing hardware VM markers: " + ($missing -join " | "))
}

Write-Host "Hardware VM log markers verified."
Write-Host "Log: $LogPath"
