param(
    [switch]$SkipProof
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$ProofScript = Join-Path $PSScriptRoot "prove-qemu-compat-gfx-smoke.ps1"
$BenchRoot = Join-Path $RepoRoot "target\bench-qemu"
$ResultsJson = Join-Path $BenchRoot "results.json"
$ResultsMd = Join-Path $RepoRoot "docs\dev-acceleration-qemu-benchmark.md"
$sccache = Get-Command sccache -ErrorAction SilentlyContinue
$LimineExtractDir = Join-Path $RepoRoot "target\limine\Limine-11.0.0-binary"

function Set-DefaultBuildJobs {
    if (-not $env:CARGO_BUILD_JOBS) {
        $logical = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
        $jobs = [math]::Max(1, [int][math]::Floor($logical * 0.8))
        $env:CARGO_BUILD_JOBS = "$jobs"
    }
}

function Invoke-MeasuredCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Label,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Script
    )

    Write-Host ("[bench-qemu] " + $Label)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $null = & $Script
    $sw.Stop()
    [pscustomobject]@{
        label = $Label
        seconds = [math]::Round($sw.Elapsed.TotalSeconds, 2)
    }
}

function Reset-BuildOutputs {
    $paths = @(
        (Join-Path $RepoRoot "target\x86_64-ngos-kernel"),
        (Join-Path $RepoRoot "target\x86_64-ngos-user"),
        (Join-Path $RepoRoot "target\qemu\limine-uefi"),
        (Join-Path $RepoRoot "target\qemu\limine-uefi.img"),
        (Join-Path $RepoRoot "target\qemu\serial-compat-gfx.log"),
        (Join-Path $RepoRoot "target\qemu\debugcon-compat-gfx.log")
    )

    foreach ($path in $paths) {
        if (Test-Path $path) {
            Remove-Item -LiteralPath $path -Recurse -Force
        }
    }
}

function Ensure-LimineInputs {
    if (!(Test-Path (Join-Path $LimineExtractDir "BOOTX64.EFI"))) {
        & powershell -ExecutionPolicy Bypass -File $BuildScript
        if ($LASTEXITCODE -ne 0) {
            throw "Could not materialize exact Limine BOOTX64.EFI through build-limine-uefi.ps1."
        }
    }
}

function Reset-SccacheStats {
    if ($sccache) {
        & $sccache.Source --stop-server 2>$null | Out-Null
        & $sccache.Source --zero-stats | Out-Null
    }
}

function Get-SccacheStatsText {
    if (-not $sccache) {
        return "sccache not installed"
    }
    return (& $sccache.Source --show-stats | Out-String).Trim()
}

function Invoke-RealPathScript {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ScriptPath,
        [switch]$UseSccache
    )

    Set-DefaultBuildJobs
    if ($UseSccache) {
        if (-not $sccache) {
            throw "sccache was not found in PATH."
        }
        $env:RUSTC_WRAPPER = $sccache.Source
    }
    else {
        Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue
    }

    & powershell -ExecutionPolicy Bypass -File $ScriptPath
    if ($LASTEXITCODE -ne 0) {
        throw ("Failed while running " + $ScriptPath)
    }
}

function Measure-Scenario {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$ScriptPath,
        [switch]$UseSccache,
        [switch]$ResetCache
    )

    Reset-BuildOutputs
    if ($ResetCache) {
        Reset-SccacheStats
    }

    try {
        $measurement = Invoke-MeasuredCommand -Label $Name -Script {
            Invoke-RealPathScript -ScriptPath $ScriptPath -UseSccache:$UseSccache
        }

        return [pscustomobject]@{
            name = $Name
            success = $true
            seconds = $measurement.seconds
            sccache_stats = Get-SccacheStatsText
            error = ""
        }
    }
    catch {
        return [pscustomobject]@{
            name = $Name
            success = $false
            seconds = $null
            sccache_stats = Get-SccacheStatsText
            error = $_.Exception.Message
        }
    }
}

New-Item -ItemType Directory -Force -Path $BenchRoot | Out-Null
Ensure-LimineInputs

$buildBaseline = Measure-Scenario -Name "build-limine baseline" -ScriptPath $BuildScript
$buildCold = Measure-Scenario -Name "build-limine sccache cold" -ScriptPath $BuildScript -UseSccache -ResetCache
$buildWarm = Measure-Scenario -Name "build-limine sccache warm" -ScriptPath $BuildScript -UseSccache

$proofBaseline = $null
$proofCold = $null
$proofWarm = $null
if (-not $SkipProof) {
    $proofBaseline = Measure-Scenario -Name "qemu proof baseline" -ScriptPath $ProofScript
    $proofCold = Measure-Scenario -Name "qemu proof sccache cold" -ScriptPath $ProofScript -UseSccache -ResetCache
    $proofWarm = Measure-Scenario -Name "qemu proof sccache warm" -ScriptPath $ProofScript -UseSccache
}

$buildWarmGainPct = if ($buildBaseline.seconds -gt 0) {
    [math]::Round((1.0 - ($buildWarm.seconds / $buildBaseline.seconds)) * 100.0, 1)
} else { 0.0 }

$proofWarmGainPct = if ($proofBaseline -and $proofBaseline.success -and $proofBaseline.seconds -gt 0 -and $proofWarm.success) {
    [math]::Round((1.0 - ($proofWarm.seconds / $proofBaseline.seconds)) * 100.0, 1)
} else { 0.0 }

$result = [pscustomobject]@{
    generated_at = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss zzz")
    cpu_jobs = $env:CARGO_BUILD_JOBS
    build_limine = [pscustomobject]@{
        baseline_seconds = $buildBaseline.seconds
        sccache_cold_seconds = $buildCold.seconds
        sccache_warm_seconds = $buildWarm.seconds
        warm_gain_percent = $buildWarmGainPct
        baseline_sccache_stats = $buildBaseline.sccache_stats
        cold_sccache_stats = $buildCold.sccache_stats
        warm_sccache_stats = $buildWarm.sccache_stats
    }
    qemu_proof = [pscustomobject]@{
        baseline_success = $(if ($proofBaseline) { $proofBaseline.success } else { $false })
        baseline_seconds = $(if ($proofBaseline) { $proofBaseline.seconds } else { $null })
        baseline_error = $(if ($proofBaseline) { $proofBaseline.error } else { "skipped" })
        sccache_cold_success = $(if ($proofCold) { $proofCold.success } else { $false })
        sccache_cold_seconds = $(if ($proofCold) { $proofCold.seconds } else { $null })
        sccache_cold_error = $(if ($proofCold) { $proofCold.error } else { "skipped" })
        sccache_warm_success = $(if ($proofWarm) { $proofWarm.success } else { $false })
        sccache_warm_seconds = $(if ($proofWarm) { $proofWarm.seconds } else { $null })
        sccache_warm_error = $(if ($proofWarm) { $proofWarm.error } else { "skipped" })
        warm_gain_percent = $proofWarmGainPct
        baseline_sccache_stats = $(if ($proofBaseline) { $proofBaseline.sccache_stats } else { "" })
        cold_sccache_stats = $(if ($proofCold) { $proofCold.sccache_stats } else { "" })
        warm_sccache_stats = $(if ($proofWarm) { $proofWarm.sccache_stats } else { "" })
    }
}

$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ResultsJson -Encoding UTF8

$md = @"
## QEMU Path Acceleration Benchmark

Generat la: $($result.generated_at)

### Config

- `CARGO_BUILD_JOBS=$($result.cpu_jobs)`
- benchmark build real:
  - `tooling/x86_64/build-limine-uefi.ps1`
- benchmark proof real:
  - `tooling/x86_64/prove-qemu-compat-gfx-smoke.ps1`

### Rezultate

#### Build Limine UEFI

- baseline fara `sccache`: $($result.build_limine.baseline_seconds)s
- `sccache` cold: $($result.build_limine.sccache_cold_seconds)s
- `sccache` warm: $($result.build_limine.sccache_warm_seconds)s
- castig warm vs baseline: $($result.build_limine.warm_gain_percent)%

#### QEMU Proof

- baseline fara `sccache`: $($result.qemu_proof.baseline_seconds)s
- baseline status: $($result.qemu_proof.baseline_success)
- baseline error: $($result.qemu_proof.baseline_error)
- `sccache` cold: $($result.qemu_proof.sccache_cold_seconds)s
- `sccache` cold status: $($result.qemu_proof.sccache_cold_success)
- `sccache` cold error: $($result.qemu_proof.sccache_cold_error)
- `sccache` warm: $($result.qemu_proof.sccache_warm_seconds)s
- `sccache` warm status: $($result.qemu_proof.sccache_warm_success)
- `sccache` warm error: $($result.qemu_proof.sccache_warm_error)
- castig warm vs baseline: $($result.qemu_proof.warm_gain_percent)%

### Observatii

- pe build-ul real, `sccache` reduce timpul total de compilare daca rebuild-ul porneste de la zero.
- pe proof-ul real, castigul este mai mic decat pe build pur, pentru ca o parte din timp este consumata de `QEMU`, boot si verificarea logurilor.
- daca artefactul exact de bootloader Limine lipseste, scriptul raporteaza blocajul explicit in loc sa opreasca tot benchmark-ul.
- `QEMU gdbstub` si `QEMU record/replay` ajuta la viteza de debug, nu la throughput brut; aici ele nu sunt incluse in procentul de accelerare.

### Artefacte

- [results.json](/C:/Users/pocri/OneDrive/Desktop/experiment/target/bench-qemu/results.json)
"@

Set-Content -LiteralPath $ResultsMd -Value $md -Encoding UTF8

Write-Host "QEMU path benchmark results written to:"
Write-Host "  $ResultsJson"
Write-Host "  $ResultsMd"
Write-Host ""
Write-Host "Build baseline      : $($buildBaseline.seconds)s"
Write-Host "Build sccache cold  : $($buildCold.seconds)s"
Write-Host "Build sccache warm  : $($buildWarm.seconds)s"
Write-Host "Proof baseline      : $($result.qemu_proof.baseline_seconds)s"
Write-Host "Proof baseline ok   : $($result.qemu_proof.baseline_success)"
Write-Host "Proof sccache cold  : $($result.qemu_proof.sccache_cold_seconds)s"
Write-Host "Proof sccache cold ok: $($result.qemu_proof.sccache_cold_success)"
Write-Host "Proof sccache warm  : $($result.qemu_proof.sccache_warm_seconds)s"
Write-Host "Proof sccache warm ok: $($result.qemu_proof.sccache_warm_success)"
