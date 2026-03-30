param(
    [switch]$Release
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"

& $BuildScript -Release:$Release -StageName "limine-uefi-vm" -ImageName "limine-uefi-vm.img"
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build VM-specific Limine UEFI image."
}
