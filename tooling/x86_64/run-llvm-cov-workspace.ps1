param(
    [switch]$OpenReport
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$EnvScript = Join-Path $PSScriptRoot "enable-dev-acceleration.ps1"

. $EnvScript

$cargoLlvmCov = Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue
if (-not $cargoLlvmCov) {
    throw "cargo-llvm-cov was not found in PATH."
}

Push-Location $RepoRoot
try {
    $args = @("llvm-cov", "--workspace", "--all-targets", "--html")
    if ($OpenReport) {
        $args += "--open"
    }

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "cargo llvm-cov failed."
    }
}
finally {
    Pop-Location
}
