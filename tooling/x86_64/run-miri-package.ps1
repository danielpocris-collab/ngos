param(
    [Parameter(Mandatory = $true)]
    [string]$Package,
    [string]$TestName
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$EnvScript = Join-Path $PSScriptRoot "enable-dev-acceleration.ps1"

. $EnvScript

$cargoMiri = Get-Command cargo-miri -ErrorAction SilentlyContinue
if (-not $cargoMiri) {
    throw "cargo-miri was not found in PATH."
}

Push-Location $RepoRoot
try {
    $args = @("miri", "test", "-p", $Package)
    if ($TestName) {
        $args += $TestName
    }

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "cargo miri failed."
    }
}
finally {
    Pop-Location
}
