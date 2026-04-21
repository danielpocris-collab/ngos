param(
    [switch]$Release,
    [int]$TimeoutSeconds = 600
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $env:CARGO_BUILD_JOBS) {
    $logical = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
    $jobs = [math]::Max(1, [int][math]::Floor($logical * 0.8))
    $env:CARGO_BUILD_JOBS = "$jobs"
}

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$AbiScript = Join-Path $PSScriptRoot "prove-qemu-compat-abi-smoke.ps1"
$LoaderScript = Join-Path $PSScriptRoot "prove-qemu-compat-loader-smoke.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-compat-foreign-log.ps1"
$AbiLog = Join-Path $RepoRoot "target\qemu\serial-compat-abi.log"
$LoaderLog = Join-Path $RepoRoot "target\qemu\serial-compat-loader.log"
$ForeignLog = Join-Path $RepoRoot "target\qemu\serial-compat-foreign.log"

if (!(Test-Path $AbiScript)) {
    throw "ABI proof script not found at $AbiScript"
}
if (!(Test-Path $LoaderScript)) {
    throw "Loader proof script not found at $LoaderScript"
}

& $AbiScript -Release:$Release -TimeoutSeconds $TimeoutSeconds
if ($LASTEXITCODE -ne 0) {
    throw "Failed to run compat ABI proof."
}

& $LoaderScript -Release:$Release -TimeoutSeconds $TimeoutSeconds
if ($LASTEXITCODE -ne 0) {
    throw "Failed to run compat loader proof."
}

$combined = New-Object System.Collections.Generic.List[string]
$combined.Add("boot.proof=compat-foreign")
if (Test-Path $AbiLog) {
    foreach ($line in [string[]](Get-Content -Path $AbiLog)) {
        $combined.Add($line)
    }
}
if (Test-Path $LoaderLog) {
    foreach ($line in [string[]](Get-Content -Path $LoaderLog)) {
        $combined.Add($line)
    }
}
$combined.Add("compat-foreign-smoke-ok")
Set-Content -Path $ForeignLog -Value $combined -Encoding ascii

& $VerifyScript -LogPath $ForeignLog
if ($LASTEXITCODE -ne 0) {
    throw "Failed to verify combined compat foreign log."
}

Write-Host "QEMU compat foreign proof completed."
Write-Host "ABI log: $AbiLog"
Write-Host "Loader log: $LoaderLog"
Write-Host "Combined log: $ForeignLog"
