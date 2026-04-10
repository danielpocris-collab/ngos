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
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$BackupScript = Join-Path $PSScriptRoot "backup-hardware-fallback.ps1"
$SyncScript = Join-Path $PSScriptRoot "sync-limine-uefi-stage.ps1"
$StageDir = Join-Path $RepoRoot "target\qemu\limine-uefi-hardware-bus"
$StageConfig = Join-Path $StageDir "limine.conf"
$BootConfig = Join-Path $StageDir "EFI\BOOT\limine.conf"

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

& $BuildScript -Release:$Release -StageName "limine-uefi-hardware-bus" -ImageName "limine-uefi-hardware-bus.img"

$ProofConfig = @"
timeout: 0
verbose: yes
serial: yes

/ngos_bus
    protocol: limine
    path: boot():/kernel/ngos-boot-x86_64
    module_path: boot():/kernel/ngos-userland-native
    cmdline: console=ttyS0 earlyprintk=serial ngos.boot.proof=bus
"@
Set-Content -Path $StageConfig -Value $ProofConfig -Encoding ascii
Set-Content -Path $BootConfig -Value $ProofConfig -Encoding ascii

if ($BackupFallback) {
    & $BackupScript -EspRoot $EspRoot -BackupRoot $BackupRoot
}
& $SyncScript -StagePath $StageDir -EspRoot $EspRoot -Prune:$Prune

Write-Host "Hardware bus deploy completed."
Write-Host "Stage: $StageDir"
Write-Host "ESP:   $EspRoot"
if ($BackupFallback) {
    Write-Host "Fallback backup: enabled"
}
