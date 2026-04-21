param(
    [switch]$Release
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$RunGfx = Join-Path $PSScriptRoot "prove-qemu-compat-gfx-smoke.ps1"
$RunAudio = Join-Path $PSScriptRoot "prove-qemu-compat-audio-smoke.ps1"
$RunInput = Join-Path $PSScriptRoot "prove-qemu-compat-input-smoke.ps1"
$RunForeign = Join-Path $PSScriptRoot "prove-qemu-compat-foreign-smoke.ps1"

Write-Host "[1/4] QEMU compat graphics"
& $RunGfx -Release:$Release
if ($LASTEXITCODE -ne 0) {
    throw "QEMU compat graphics proof failed."
}

Write-Host "[2/4] QEMU compat audio"
& $RunAudio -Release:$Release
if ($LASTEXITCODE -ne 0) {
    throw "QEMU compat audio proof failed."
}

Write-Host "[3/4] QEMU compat input"
& $RunInput -Release:$Release
if ($LASTEXITCODE -ne 0) {
    throw "QEMU compat input proof failed."
}

Write-Host "[4/4] QEMU compat foreign"
& $RunForeign -Release:$Release
if ($LASTEXITCODE -ne 0) {
    throw "QEMU compat foreign proof failed."
}

Write-Host "QEMU unified compat suite completed."
