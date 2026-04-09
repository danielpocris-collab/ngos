param(
    [switch]$IncludeQemu
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BootEvidenceScript = Join-Path $PSScriptRoot "verify-qemu-boot-evidence.ps1"
$InvalidBootInfoLocalScript = Join-Path $PSScriptRoot "verify-boot-invalid-boot-info-local-coverage.ps1"
$InvalidBootInfoBlockersScript = Join-Path $PSScriptRoot "explain-boot-invalid-boot-info-qemu-blockers.ps1"
$OpenFamiliesStateScript = Join-Path $PSScriptRoot "verify-boot-open-families-state.ps1"

Push-Location $RepoRoot
try {
    cargo test -p ngos-platform-x86_64
    if ($LASTEXITCODE -ne 0) {
        throw "ngos-platform-x86_64 tests failed."
    }

    cargo test -p ngos-boot-x86_64 --lib
    if ($LASTEXITCODE -ne 0) {
        throw "ngos-boot-x86_64 library tests failed."
    }

    & $InvalidBootInfoLocalScript
    if ($LASTEXITCODE -ne 0) {
        throw "Boot InvalidBootInfo local coverage verification failed."
    }

    & $InvalidBootInfoBlockersScript
    if ($LASTEXITCODE -ne 0) {
        throw "Boot InvalidBootInfo blocker explanation failed."
    }

    & $OpenFamiliesStateScript
    if ($LASTEXITCODE -ne 0) {
        throw "Boot open-families state verification failed."
    }

    if ($IncludeQemu) {
        & $BootEvidenceScript
        if ($LASTEXITCODE -ne 0) {
            throw "QEMU boot evidence verification failed."
        }
    }
}
finally {
    Pop-Location
}

Write-Host "Boot subsystem state verified."
Write-Host "Platform owner coverage: ngos-platform-x86_64"
Write-Host "Boot owner coverage: ngos-boot-x86_64 --lib"
Write-Host "InvalidBootInfo local coverage: verified"
Write-Host "InvalidBootInfo QEMU mechanism: verified"
Write-Host "Open boot families state: verified"
if ($IncludeQemu) {
    Write-Host "QEMU evidence: verified"
} else {
    Write-Host "QEMU evidence: skipped"
}
