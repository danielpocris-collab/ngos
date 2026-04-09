param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Push-Location $RepoRoot
try {
    & cargo test -p ngos-platform-x86_64 handoff_rejects_overlapping_or_misaligned_memory_regions
    if ($LASTEXITCODE -ne 0) {
        throw "Targeted platform InvalidBootInfo overlap/misalignment test failed."
    }

    & cargo test -p ngos-platform-x86_64 handoff_rejects_non_kernel_image_or_unaligned_physical_offset
    if ($LASTEXITCODE -ne 0) {
        throw "Targeted platform InvalidBootInfo kernel-range/offset test failed."
    }

}
finally {
    Pop-Location
}

Write-Host "Boot InvalidBootInfo local coverage verified."
Write-Host "Platform owner invariants remain covered."
Write-Host "Boot main.rs failure-summary tests stay source-owned but are cfg-gated off host cargo test."
