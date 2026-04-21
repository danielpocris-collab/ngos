param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$KernelSpanAlignmentScript = Join-Path $PSScriptRoot "verify-qemu-boot-kernel-span-alignment.ps1"
$PostHandoffSurfaceScript = Join-Path $PSScriptRoot "verify-qemu-boot-post-handoff-corruption-surface.ps1"
$InvalidBootInfoBlockersScript = Join-Path $PSScriptRoot "explain-boot-invalid-boot-info-qemu-blockers.ps1"

& $KernelSpanAlignmentScript
& $PostHandoffSurfaceScript
& $InvalidBootInfoBlockersScript

Write-Host "Boot post-handoff proof surface verified."
Write-Host "Kernel-span alignment baseline: verified"
Write-Host "Post-handoff corruption surface: verified"
Write-Host "InvalidBootInfo proof mechanism: verified"
