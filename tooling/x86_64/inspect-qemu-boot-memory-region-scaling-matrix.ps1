param(
    [switch]$Release,
    [int]$TimeoutSeconds = 180
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScalingScript = Join-Path $PSScriptRoot "inspect-qemu-boot-memory-region-scaling.ps1"
$VerifyHeadroomScript = Join-Path $PSScriptRoot "verify-qemu-boot-memory-region-headroom.ps1"
$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$SummaryPath = Join-Path $RepoRoot "target\qemu\boot-memory-region-scaling-matrix.txt"

$cases = @(
    @{ DimmCount = 8; DimmSizeMiB = 16 },
    @{ DimmCount = 16; DimmSizeMiB = 16 },
    @{ DimmCount = 16; DimmSizeMiB = 64 },
    @{ DimmCount = 32; DimmSizeMiB = 64 }
)

$results = New-Object System.Collections.Generic.List[string]
foreach ($case in $cases) {
    & $ScalingScript -Release:$Release -TimeoutSeconds $TimeoutSeconds -DimmCount $case.DimmCount -DimmSizeMiB $case.DimmSizeMiB | Out-Null

    $logPath = Join-Path $RepoRoot "target\qemu\serial-boot-memory-region-scaling.log"
    & $VerifyHeadroomScript -LogPath $logPath | Out-Null

    $content = Get-Content -LiteralPath $logPath -Raw
    $match = [regex]::Match($content, 'memory_regions=(\d+)')
    if (-not $match.Success) {
        throw "memory_regions marker was not found after scaling case $($case.DimmCount)x$($case.DimmSizeMiB)MiB"
    }

    $count = [int]$match.Groups[1].Value
    $line = "dimm_count={0} dimm_size_mib={1} observed_memory_regions={2}" -f $case.DimmCount, $case.DimmSizeMiB, $count
    $results.Add($line) | Out-Null
}

Set-Content -LiteralPath $SummaryPath -Value $results -Encoding ascii

Write-Host "QEMU boot memory-region scaling matrix completed."
Write-Host "Summary: $SummaryPath"
foreach ($line in $results) {
    Write-Host $line
}
