param(
    [string]$LogPath = "target/qemu/serial-boot.log"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not [System.IO.Path]::IsPathRooted($LogPath)) {
    $LogPath = Join-Path $RepoRoot $LogPath
}

if (!(Test-Path $LogPath)) {
    throw "Kernel span log not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$match = [regex]::Match($content, 'kernel image span = 0x([0-9a-fA-F]+)')
if (-not $match.Success) {
    throw "Kernel image span marker was not found in $LogPath"
}

$span = [Convert]::ToUInt64($match.Groups[1].Value, 16)
$pageSize = 4096
if (($span % $pageSize) -ne 0) {
    throw ("Observed kernel image span 0x{0:x} is not page-aligned." -f $span)
}

Write-Host "QEMU boot kernel span alignment verified."
Write-Host ("Observed span: 0x{0:x}" -f $span)
Write-Host ("Alignment: 0x{0:x}" -f $pageSize)
Write-Host "Log: $LogPath"
