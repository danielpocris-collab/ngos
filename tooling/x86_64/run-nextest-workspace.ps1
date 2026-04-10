param(
    [string]$Profile = "default"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$EnvScript = Join-Path $PSScriptRoot "enable-dev-acceleration.ps1"

. $EnvScript

$cargoNextest = Get-Command cargo-nextest -ErrorAction SilentlyContinue
if (-not $cargoNextest) {
    throw "cargo-nextest was not found in PATH."
}

Push-Location $RepoRoot
try {
    & cargo nextest run --workspace --profile $Profile
    if ($LASTEXITCODE -ne 0) {
        throw "cargo nextest failed."
    }
}
finally {
    Pop-Location
}
