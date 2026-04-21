param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath,
    [Parameter(Mandatory = $true)]
    [string]$Mode,
    [Parameter(Mandatory = $true)]
    [string]$Detail,
    [Parameter(Mandatory = $true)]
    [string]$StatusHex
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Boot post-handoff refusal serial log not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$markers = @(
    "ngos/x86_64: stage0 entered",
    "ngos/x86_64: post-handoff corruption applied mode=$Mode",
    "ngos/x86_64: limine handoff refusal detail=$Detail status=$StatusHex",
    "family=limine detail=$Detail cause=Limine(",
    "ngos/x86_64: boot locator stage=Limine checkpoint=0x2ff name=limine/contract-refusal payload0=Status:$StatusHex"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing QEMU post-handoff refusal markers: " + ($missing -join " | "))
}

Write-Host "QEMU boot post-handoff refusal log markers verified."
Write-Host "Mode: $Mode"
Write-Host "Detail: $Detail"
Write-Host "Status: $StatusHex"
Write-Host "Log: $LogPath"
