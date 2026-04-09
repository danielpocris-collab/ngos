param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath,
    [string]$Detail = "too-many-modules",
    [string]$StatusHex = "0x21"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Boot refusal serial log not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$markers = @(
    "ngos/x86_64: stage0 entered",
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
    throw ("Missing QEMU boot refusal markers: " + ($missing -join " | "))
}

Write-Host "QEMU boot refusal log markers verified."
Write-Host "Detail: $Detail"
Write-Host "Status: $StatusHex"
Write-Host "Log: $LogPath"
