Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Set-DefaultBuildJobs {
    if (-not $env:CARGO_BUILD_JOBS) {
        $logical = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
        $jobs = [math]::Max(1, [int][math]::Floor($logical * 0.8))
        $env:CARGO_BUILD_JOBS = "$jobs"
    }
}

Set-DefaultBuildJobs

$sccache = Get-Command sccache -ErrorAction SilentlyContinue
if ($sccache -and -not $env:RUSTC_WRAPPER) {
    $env:RUSTC_WRAPPER = $sccache.Source
}

$cargoNextest = Get-Command cargo-nextest -ErrorAction SilentlyContinue
$cargoLlvmCov = Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue
$cargoMiri = Get-Command cargo-miri -ErrorAction SilentlyContinue

Write-Host "NGOS dev acceleration environment"
Write-Host ("  CARGO_BUILD_JOBS : " + $env:CARGO_BUILD_JOBS)
Write-Host ("  RUSTC_WRAPPER    : " + ($(if ($env:RUSTC_WRAPPER) { $env:RUSTC_WRAPPER } else { "<unset>" })))
Write-Host ("  sccache          : " + ($(if ($sccache) { $sccache.Source } else { "<missing>" })))
Write-Host ("  cargo-nextest    : " + ($(if ($cargoNextest) { $cargoNextest.Source } else { "<missing>" })))
Write-Host ("  cargo-llvm-cov   : " + ($(if ($cargoLlvmCov) { $cargoLlvmCov.Source } else { "<missing>" })))
Write-Host ("  cargo-miri       : " + ($(if ($cargoMiri) { $cargoMiri.Source } else { "<missing>" })))
