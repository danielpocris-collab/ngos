param(
    [Parameter(Mandatory = $true)]
    [string]$EspRoot,
    [switch]$Release,
    [switch]$Prune,
    [switch]$BackupFallback,
    [string]$BackupRoot = "",
    [switch]$SkipAutoElevation
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$CommonScript = Join-Path $PSScriptRoot "hardware-boot-common.ps1"
. $CommonScript

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi-hardware.ps1"
$BackupScript = Join-Path $PSScriptRoot "backup-hardware-fallback.ps1"
$SyncScript = Join-Path $PSScriptRoot "sync-limine-uefi-stage.ps1"
$StageDir = Join-Path $RepoRoot "target\qemu\limine-uefi-hardware"

if (-not $SkipAutoElevation -and -not (Test-IsAdministrator) -and (Test-Path -LiteralPath $EspRoot) -and -not (Test-EspWritable -Root $EspRoot)) {
    Write-Host "ESP is not writable from the current PowerShell token. Relaunching deploy elevated..."
    $argumentList = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-EspRoot", $EspRoot,
        "-SkipAutoElevation"
    )
    if ($Release) {
        $argumentList += "-Release"
    }
    if ($Prune) {
        $argumentList += "-Prune"
    }
    if ($BackupFallback) {
        $argumentList += "-BackupFallback"
    }
    if (-not [string]::IsNullOrWhiteSpace($BackupRoot)) {
        $argumentList += @("-BackupRoot", $BackupRoot)
    }

    Invoke-ScriptElevated -ScriptPath $PSCommandPath -ArgumentList $argumentList
    return
}

& $BuildScript -Release:$Release
if ($BackupFallback) {
    & $BackupScript -EspRoot $EspRoot -BackupRoot $BackupRoot
}
& $SyncScript -StagePath $StageDir -EspRoot $EspRoot -Prune:$Prune

Write-Host "Hardware deploy completed."
Write-Host "Stage: $StageDir"
Write-Host "ESP:   $EspRoot"
if ($BackupFallback) {
    Write-Host "Fallback backup: enabled"
}
