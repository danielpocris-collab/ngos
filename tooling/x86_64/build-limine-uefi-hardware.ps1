param(
    [switch]$Release
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"

& $BuildScript -Release:$Release -StageName "limine-uefi-hardware" -ImageName "limine-uefi-hardware.img"

