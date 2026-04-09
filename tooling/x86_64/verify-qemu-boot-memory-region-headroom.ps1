param(
    [string]$LogPath = "target/qemu/serial-boot.log",
    [int]$Capacity = 256
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not [System.IO.Path]::IsPathRooted($LogPath)) {
    $LogPath = Join-Path $RepoRoot $LogPath
}

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "QEMU boot serial log not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$match = [regex]::Match($content, 'memory_regions=(\d+)')
if (-not $match.Success) {
    throw "Could not find memory_regions marker in $LogPath"
}

$count = [int]$match.Groups[1].Value
if ($count -ge $Capacity) {
    throw "Observed memory_regions=$count meets or exceeds capacity=$Capacity; headroom assumption is invalid."
}

$headroom = $Capacity - $count
Write-Host "QEMU boot memory-region headroom verified."
Write-Host "Observed regions: $count"
Write-Host "Configured capacity: $Capacity"
Write-Host "Remaining headroom: $headroom"
Write-Host "Log: $LogPath"
