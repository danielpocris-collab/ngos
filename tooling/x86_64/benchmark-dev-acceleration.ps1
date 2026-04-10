Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$EnvScript = Join-Path $PSScriptRoot "enable-dev-acceleration.ps1"
$BenchRoot = Join-Path $RepoRoot "target\bench-dev"
$ResultsJson = Join-Path $BenchRoot "results.json"
$ResultsMd = Join-Path $RepoRoot "docs\dev-acceleration-benchmark.md"

. $EnvScript | Out-Null

function Invoke-MeasuredCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Label,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Script
    )

    Write-Host ("[bench] " + $Label)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $null = & $Script
    $sw.Stop()
    [pscustomobject]@{
        label = $Label
        seconds = [math]::Round($sw.Elapsed.TotalSeconds, 2)
    }
}

function Remove-PathIfExists {
    param([string]$Path)
    if (Test-Path $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Get-SccacheStatsText {
    $sccache = Get-Command sccache -ErrorAction SilentlyContinue
    if (-not $sccache) {
        return "sccache not installed"
    }
    return (& $sccache.Source --show-stats | Out-String).Trim()
}

function Set-BuildEnvironment {
    param(
        [string]$TargetDir,
        [switch]$UseSccache
    )

    $env:CARGO_TARGET_DIR = $TargetDir
    if ($UseSccache) {
        $sccache = Get-Command sccache -ErrorAction SilentlyContinue
        if (-not $sccache) {
            throw "sccache was not found in PATH."
        }
        $env:RUSTC_WRAPPER = $sccache.Source
    }
    else {
        Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue
    }
}

function Invoke-CargoBuildBench {
    param(
        [string]$TargetDir,
        [switch]$UseSccache
    )

    Set-BuildEnvironment -TargetDir $TargetDir -UseSccache:$UseSccache
    Push-Location $RepoRoot
    try {
        & cargo test -p ngos-userland-native -p ngos-boot-x86_64 --lib --no-run
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build benchmark failed."
        }
    }
    finally {
        Pop-Location
    }
}

function Invoke-CargoRunnerBench {
    param([string]$TargetDir)

    Set-BuildEnvironment -TargetDir $TargetDir
    Push-Location $RepoRoot
    try {
        & cargo test -p ngos-userland-native --lib
        if ($LASTEXITCODE -ne 0) {
            throw "cargo test benchmark failed."
        }
    }
    finally {
        Pop-Location
    }
}

function Invoke-NextestRunnerBench {
    param([string]$TargetDir)

    Set-BuildEnvironment -TargetDir $TargetDir
    Push-Location $RepoRoot
    try {
        & cargo nextest run -p ngos-userland-native --lib --hide-progress-bar
        if ($LASTEXITCODE -ne 0) {
            throw "cargo nextest benchmark failed."
        }
    }
    finally {
        Pop-Location
    }
}

New-Item -ItemType Directory -Force -Path $BenchRoot | Out-Null

$buildBaselineDir = Join-Path $BenchRoot "build-baseline"
$buildSccacheColdDir = Join-Path $BenchRoot "build-sccache-cold"
$buildSccacheWarmDir = Join-Path $BenchRoot "build-sccache-warm"
$runnerDir = Join-Path $BenchRoot "runner"

Remove-PathIfExists $buildBaselineDir
Remove-PathIfExists $buildSccacheColdDir
Remove-PathIfExists $buildSccacheWarmDir
Remove-PathIfExists $runnerDir

$sccache = Get-Command sccache -ErrorAction SilentlyContinue
if ($sccache) {
    & $sccache.Source --stop-server 2>$null | Out-Null
    & $sccache.Source --zero-stats | Out-Null
}

$buildBaseline = Invoke-MeasuredCommand -Label "cargo build baseline" -Script {
    Invoke-CargoBuildBench -TargetDir $buildBaselineDir
}

$sccacheAfterBaseline = Get-SccacheStatsText

if ($sccache) {
    & $sccache.Source --stop-server 2>$null | Out-Null
    & $sccache.Source --zero-stats | Out-Null
}

$buildSccacheCold = Invoke-MeasuredCommand -Label "cargo build with sccache cold" -Script {
    Invoke-CargoBuildBench -TargetDir $buildSccacheColdDir -UseSccache
}
$sccacheColdStats = Get-SccacheStatsText

$buildSccacheWarm = Invoke-MeasuredCommand -Label "cargo build with sccache warm" -Script {
    Invoke-CargoBuildBench -TargetDir $buildSccacheWarmDir -UseSccache
}
$sccacheWarmStats = Get-SccacheStatsText

Set-BuildEnvironment -TargetDir $runnerDir
Push-Location $RepoRoot
try {
    & cargo test -p ngos-userland-native --lib --no-run
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to prebuild runner benchmark target."
    }
}
finally {
    Pop-Location
}

$cargoTestRun = Invoke-MeasuredCommand -Label "cargo test runner" -Script {
    Invoke-CargoRunnerBench -TargetDir $runnerDir
}

$nextestRun = Invoke-MeasuredCommand -Label "cargo nextest runner" -Script {
    Invoke-NextestRunnerBench -TargetDir $runnerDir
}

$buildWarmGainPct = if ($buildBaseline.seconds -gt 0) {
    [math]::Round((1.0 - ($buildSccacheWarm.seconds / $buildBaseline.seconds)) * 100.0, 1)
} else { 0.0 }

$runnerGainPct = if ($cargoTestRun.seconds -gt 0) {
    [math]::Round((1.0 - ($nextestRun.seconds / $cargoTestRun.seconds)) * 100.0, 1)
} else { 0.0 }

$result = [pscustomobject]@{
    generated_at = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss zzz")
    cpu_jobs = $env:CARGO_BUILD_JOBS
    build = [pscustomobject]@{
        baseline_seconds = $buildBaseline.seconds
        sccache_cold_seconds = $buildSccacheCold.seconds
        sccache_warm_seconds = $buildSccacheWarm.seconds
        warm_gain_percent = $buildWarmGainPct
        baseline_sccache_stats = $sccacheAfterBaseline
        cold_sccache_stats = $sccacheColdStats
        warm_sccache_stats = $sccacheWarmStats
    }
    tests = [pscustomobject]@{
        cargo_test_seconds = $cargoTestRun.seconds
        cargo_nextest_seconds = $nextestRun.seconds
        nextest_gain_percent = $runnerGainPct
    }
}

$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ResultsJson -Encoding UTF8

$md = @"
## Dev Acceleration Benchmark

Generat la: $($result.generated_at)

### Config

- `CARGO_BUILD_JOBS=$($result.cpu_jobs)`
- benchmark build:
  - `cargo test -p ngos-userland-native -p ngos-boot-x86_64 --lib --no-run`
- benchmark test runner:
  - `cargo test -p ngos-userland-native --lib`
  - `cargo nextest run -p ngos-userland-native --lib`

### Rezultate

#### Build

- baseline fara `sccache`: $($result.build.baseline_seconds)s
- `sccache` cold: $($result.build.sccache_cold_seconds)s
- `sccache` warm: $($result.build.sccache_warm_seconds)s
- castig build warm vs baseline: $($result.build.warm_gain_percent)%

#### Test runner

- `cargo test`: $($result.tests.cargo_test_seconds)s
- `cargo nextest`: $($result.tests.cargo_nextest_seconds)s
- castig `nextest` vs `cargo test`: $($result.tests.nextest_gain_percent)%

### Observatii

- `sccache` ajuta mai ales pe rebuild-uri sau build-uri in target dir diferit dupa ce cache-ul s-a incalzit.
- `nextest` accelereaza executia testelor; nu schimba regula ca closure ramane pe path-ul real si pe proof-urile `QEMU`.
- `QEMU gdbstub` si `QEMU record/replay` reduc timpul de debug, nu throughput-ul brut de build/test.

### Artefacte

- [results.json](/C:/Users/pocri/OneDrive/Desktop/experiment/target/bench-dev/results.json)
"@

Set-Content -LiteralPath $ResultsMd -Value $md -Encoding UTF8

Write-Host "Benchmark results written to:"
Write-Host "  $ResultsJson"
Write-Host "  $ResultsMd"
Write-Host ""
Write-Host "Build baseline      : $($buildBaseline.seconds)s"
Write-Host "Build sccache cold  : $($buildSccacheCold.seconds)s"
Write-Host "Build sccache warm  : $($buildSccacheWarm.seconds)s"
Write-Host "Runner cargo test   : $($cargoTestRun.seconds)s"
Write-Host "Runner cargo nextest: $($nextestRun.seconds)s"
