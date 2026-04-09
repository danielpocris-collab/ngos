param(
    [string]$SuccessLog = "target/qemu/serial-boot.log",
    [string]$InvalidCmdlineLog = "target/qemu/serial-boot-refusal-invalid-cmdline.log",
    [string]$TooManyModulesLog = "target/qemu/serial-boot-refusal.log",
    [string]$MissingMemoryMapLog = "target/qemu/serial-boot-refusal-missing-memory-map.log",
    [string]$MissingHhdmLog = "target/qemu/serial-boot-refusal-missing-hhdm.log",
    [string]$PrebootModulePathLog = "target/qemu/serial-limine-preboot-invalid-module-path.log",
    [string]$PrebootExecutablePathLog = "target/qemu/serial-limine-preboot-invalid-executable-path.log",
    [string]$PrebootMissingBaseRevisionLog = "target/qemu/serial-limine-preboot-missing-base-revision.log",
    [string]$PrebootUnsupportedBaseRevisionLog = "target/qemu/serial-limine-preboot-unsupported-base-revision.log",
    [string]$TooManyMemoryRegionsLog = "target/qemu/serial-boot-post-handoff-too-many-memory-regions.log",
    [string]$InvalidHhdmOffsetLog = "target/qemu/serial-boot-post-handoff-invalid-hhdm-offset.log",
    [string]$InvalidKernelRangeKindLog = "target/qemu/serial-boot-post-handoff-invalid-kernel-range-kind.log",
    [string]$InvalidKernelRangeAlignmentLog = "target/qemu/serial-boot-post-handoff-invalid-kernel-range-alignment.log",
    [string]$EmptyKernelRangeLog = "target/qemu/serial-boot-post-handoff-empty-kernel-range.log",
    [string]$InvalidMemoryRegionAlignmentLog = "target/qemu/serial-boot-post-handoff-invalid-memory-region-alignment.log",
    [string]$EmptyMemoryRegionLog = "target/qemu/serial-boot-post-handoff-empty-memory-region.log",
    [string]$OverlappingMemoryRegionsLog = "target/qemu/serial-boot-post-handoff-overlapping-memory-regions.log"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$VerifyBoot = Join-Path $PSScriptRoot "verify-qemu-boot-log.ps1"
$VerifyRefusal = Join-Path $PSScriptRoot "verify-qemu-boot-refusal-log.ps1"
$VerifyPreboot = Join-Path $PSScriptRoot "verify-qemu-limine-preboot-module-path-rejection-log.ps1"
$VerifyPrebootExecutable = Join-Path $PSScriptRoot "verify-qemu-limine-preboot-executable-path-rejection-log.ps1"
$VerifyPrebootBaseRevision = Join-Path $PSScriptRoot "verify-qemu-limine-preboot-base-revision-rejection-log.ps1"
$VerifyPostHandoff = Join-Path $PSScriptRoot "verify-qemu-boot-post-handoff-refusal-log.ps1"
$VerifyOpenFamilyBlockers = Join-Path $PSScriptRoot "verify-boot-open-family-blockers.ps1"

function Resolve-RepoPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $RepoRoot $Path)
}

$resolvedSuccess = Resolve-RepoPath $SuccessLog
$resolvedInvalidCmdline = Resolve-RepoPath $InvalidCmdlineLog
$resolvedTooManyModules = Resolve-RepoPath $TooManyModulesLog
$resolvedMissingMemoryMap = Resolve-RepoPath $MissingMemoryMapLog
$resolvedMissingHhdm = Resolve-RepoPath $MissingHhdmLog
$resolvedPrebootModulePath = Resolve-RepoPath $PrebootModulePathLog
$resolvedPrebootExecutablePath = Resolve-RepoPath $PrebootExecutablePathLog
$resolvedPrebootMissingBaseRevision = Resolve-RepoPath $PrebootMissingBaseRevisionLog
$resolvedPrebootUnsupportedBaseRevision = Resolve-RepoPath $PrebootUnsupportedBaseRevisionLog
$resolvedTooManyMemoryRegions = Resolve-RepoPath $TooManyMemoryRegionsLog
$resolvedInvalidHhdmOffset = Resolve-RepoPath $InvalidHhdmOffsetLog
$resolvedInvalidKernelRangeKind = Resolve-RepoPath $InvalidKernelRangeKindLog
$resolvedInvalidKernelRangeAlignment = Resolve-RepoPath $InvalidKernelRangeAlignmentLog
$resolvedEmptyKernelRange = Resolve-RepoPath $EmptyKernelRangeLog
$resolvedInvalidMemoryRegionAlignment = Resolve-RepoPath $InvalidMemoryRegionAlignmentLog
$resolvedEmptyMemoryRegion = Resolve-RepoPath $EmptyMemoryRegionLog
$resolvedOverlappingMemoryRegions = Resolve-RepoPath $OverlappingMemoryRegionsLog

& $VerifyBoot -LogPath $resolvedSuccess | Out-Null
& $VerifyRefusal -LogPath $resolvedInvalidCmdline -Detail "invalid-command-line-utf8" -StatusHex "0x30" | Out-Null
& $VerifyRefusal -LogPath $resolvedTooManyModules -Detail "too-many-modules" -StatusHex "0x21" | Out-Null
& $VerifyRefusal -LogPath $resolvedMissingMemoryMap -Detail "missing-memory-map" -StatusHex "0x10" | Out-Null
& $VerifyRefusal -LogPath $resolvedMissingHhdm -Detail "missing-hhdm" -StatusHex "0x11" | Out-Null
& $VerifyPreboot -LogPath $resolvedPrebootModulePath | Out-Null
& $VerifyPrebootExecutable -LogPath $resolvedPrebootExecutablePath | Out-Null
& $VerifyPrebootBaseRevision -LogPath $resolvedPrebootMissingBaseRevision -Mode "missing-base-revision" | Out-Null
& $VerifyPrebootBaseRevision -LogPath $resolvedPrebootUnsupportedBaseRevision -Mode "unsupported-base-revision" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedTooManyMemoryRegions -Mode "too-many-memory-regions" -Detail "too-many-memory-regions" -StatusHex "0x20" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedInvalidHhdmOffset -Mode "invalid-hhdm-offset" -Detail "invalid-hhdm-offset" -StatusHex "0x40" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedInvalidKernelRangeKind -Mode "invalid-kernel-range-kind" -Detail "invalid-kernel-range-kind" -StatusHex "0x41" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedInvalidKernelRangeAlignment -Mode "invalid-kernel-range-alignment" -Detail "invalid-kernel-range-alignment" -StatusHex "0x42" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedEmptyKernelRange -Mode "empty-kernel-range" -Detail "empty-kernel-range" -StatusHex "0x43" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedInvalidMemoryRegionAlignment -Mode "invalid-memory-region-alignment" -Detail "invalid-memory-region-alignment" -StatusHex "0x44" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedEmptyMemoryRegion -Mode "empty-memory-region" -Detail "empty-memory-region" -StatusHex "0x45" | Out-Null
& $VerifyPostHandoff -LogPath $resolvedOverlappingMemoryRegions -Mode "overlapping-memory-regions" -Detail "overlapping-memory-regions" -StatusHex "0x46" | Out-Null
& $VerifyOpenFamilyBlockers | Out-Null

Write-Host "Boot QEMU evidence verified."
Write-Host "Success log: $resolvedSuccess"
Write-Host "Invalid cmdline refusal log: $resolvedInvalidCmdline"
Write-Host "Too-many-modules refusal log: $resolvedTooManyModules"
Write-Host "Missing-memory-map refusal log: $resolvedMissingMemoryMap"
Write-Host "Missing-hhdm refusal log: $resolvedMissingHhdm"
Write-Host "Preboot module-path rejection log: $resolvedPrebootModulePath"
Write-Host "Preboot executable-path rejection log: $resolvedPrebootExecutablePath"
Write-Host "Preboot missing-base-revision log: $resolvedPrebootMissingBaseRevision"
Write-Host "Preboot unsupported-base-revision log: $resolvedPrebootUnsupportedBaseRevision"
Write-Host "Post-handoff too-many-memory-regions log: $resolvedTooManyMemoryRegions"
Write-Host "Post-handoff invalid-hhdm-offset log: $resolvedInvalidHhdmOffset"
Write-Host "Post-handoff invalid-kernel-range-kind log: $resolvedInvalidKernelRangeKind"
Write-Host "Post-handoff invalid-kernel-range-alignment log: $resolvedInvalidKernelRangeAlignment"
Write-Host "Post-handoff empty-kernel-range log: $resolvedEmptyKernelRange"
Write-Host "Post-handoff invalid-memory-region-alignment log: $resolvedInvalidMemoryRegionAlignment"
Write-Host "Post-handoff empty-memory-region log: $resolvedEmptyMemoryRegion"
Write-Host "Post-handoff overlapping-memory-regions log: $resolvedOverlappingMemoryRegions"
Write-Host "Post-handoff proof surface: verified"
