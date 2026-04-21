param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$OpenFamilyBlockersScript = Join-Path $PSScriptRoot "verify-boot-open-family-blockers.ps1"
$TooManyRegionsLog = Join-Path $RepoRoot "target\qemu\serial-boot-post-handoff-too-many-memory-regions.log"
$InvalidHhdmLog = Join-Path $RepoRoot "target\qemu\serial-boot-post-handoff-invalid-hhdm-offset.log"

& $OpenFamilyBlockersScript

foreach ($path in @($TooManyRegionsLog, $InvalidHhdmLog)) {
    if (!(Test-Path $path)) {
        throw "Expected final-family proof log is missing: $path"
    }
}

Write-Host "Boot open families state verified."
Write-Host "Still open: none"
Write-Host "Reason: post-handoff proof mechanism now covers too-many-memory-regions and InvalidBootInfo(...) on QEMU"
