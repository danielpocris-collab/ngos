param(
    [switch]$Release
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$SuccessScript = Join-Path $PSScriptRoot "prove-qemu-boot.ps1"
$InvalidCmdlineScript = Join-Path $PSScriptRoot "prove-qemu-boot-refusal-invalid-command-line-utf8.ps1"
$TooManyModulesScript = Join-Path $PSScriptRoot "prove-qemu-boot-refusal-too-many-modules.ps1"
$MissingLoaderResponseScript = Join-Path $PSScriptRoot "prove-qemu-boot-refusal-missing-loader-response.ps1"
$PrebootModulePathScript = Join-Path $PSScriptRoot "inspect-qemu-limine-preboot-module-path-rejection.ps1"
$PrebootExecutablePathScript = Join-Path $PSScriptRoot "inspect-qemu-limine-preboot-executable-path-rejection.ps1"
$PrebootBaseRevisionScript = Join-Path $PSScriptRoot "inspect-qemu-limine-preboot-base-revision-rejection.ps1"
$PostHandoffScript = Join-Path $PSScriptRoot "prove-qemu-boot-refusal-post-handoff-corruption.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-boot-evidence.ps1"

& $SuccessScript -Release:$Release
& $InvalidCmdlineScript -Release:$Release
& $TooManyModulesScript -Release:$Release
& $MissingLoaderResponseScript -Release:$Release -Mode "missing-memory-map"
& $MissingLoaderResponseScript -Release:$Release -Mode "missing-hhdm"
& $PrebootModulePathScript -Release:$Release
& $PrebootExecutablePathScript -Release:$Release
& $PrebootBaseRevisionScript -Release:$Release -Mode "missing-base-revision"
& $PrebootBaseRevisionScript -Release:$Release -Mode "unsupported-base-revision"
foreach ($mode in @(
    "too-many-memory-regions",
    "invalid-hhdm-offset",
    "invalid-kernel-range-kind",
    "invalid-kernel-range-alignment",
    "empty-kernel-range",
    "invalid-memory-region-alignment",
    "empty-memory-region",
    "overlapping-memory-regions"
)) {
    & $PostHandoffScript -Release:$Release -Mode $mode
}
& $VerifyScript

Write-Host "Boot QEMU evidence proof completed."
