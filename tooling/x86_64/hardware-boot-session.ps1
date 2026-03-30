param(
    [Parameter(Mandatory = $true)]
    [string]$EspRoot,
    [string]$PortName = "COM1",
    [switch]$Release,
    [switch]$Prune,
    [switch]$BackupFallback,
    [string]$BackupRoot = "",
    [int]$BaudRate = 38400,
    [int]$CaptureSeconds = 20,
    [int]$WaitForFirstByteSeconds = 120,
    [string]$LogPath = "",
    [switch]$SkipAutoElevation
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$CommonScript = Join-Path $PSScriptRoot "hardware-boot-common.ps1"
. $CommonScript

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$PreflightScript = Join-Path $PSScriptRoot "preflight-hardware-boot.ps1"
$DeployScript = Join-Path $PSScriptRoot "deploy-limine-uefi-hardware.ps1"
$InspectEspScript = Join-Path $PSScriptRoot "inspect-hardware-esp.ps1"
$CaptureScript = Join-Path $PSScriptRoot "capture-hardware-serial.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-hardware-boot-log.ps1"

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $LogPath = Join-Path $RepoRoot "target\hardware\serial-$($PortName.ToLowerInvariant()).log"
}

if (-not $SkipAutoElevation -and -not (Test-IsAdministrator) -and (Test-Path -LiteralPath $EspRoot) -and -not (Test-EspWritable -Root $EspRoot)) {
    Write-Host "ESP is not writable from the current PowerShell token. Relaunching hardware boot session elevated..."
    $argumentList = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-EspRoot", $EspRoot,
        "-PortName", $PortName,
        "-BaudRate", $BaudRate.ToString(),
        "-CaptureSeconds", $CaptureSeconds.ToString(),
        "-WaitForFirstByteSeconds", $WaitForFirstByteSeconds.ToString(),
        "-SkipAutoElevation"
    )
    if (-not [string]::IsNullOrWhiteSpace($LogPath)) {
        $argumentList += @("-LogPath", $LogPath)
    }
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

& $PreflightScript -EspRoot $EspRoot -PortName $PortName -CaptureSeconds $CaptureSeconds -WaitForFirstByteSeconds $WaitForFirstByteSeconds -LogPath $LogPath
& $DeployScript -EspRoot $EspRoot -Release:$Release -Prune:$Prune -BackupFallback:$BackupFallback -BackupRoot $BackupRoot -SkipAutoElevation
& $InspectEspScript -EspRoot $EspRoot

Write-Host "Boot the target hardware now."
if ($WaitForFirstByteSeconds -gt 0) {
    Write-Host "Waiting up to $WaitForFirstByteSeconds seconds for first serial byte on $PortName..."
}
Write-Host "Capturing $CaptureSeconds seconds after first serial byte."

& $CaptureScript -PortName $PortName -BaudRate $BaudRate -DurationSeconds $CaptureSeconds -WaitForFirstByteSeconds $WaitForFirstByteSeconds -LogPath $LogPath -RequireData
& $VerifyScript -LogPath $LogPath

Write-Host "Hardware boot session complete."
Write-Host "ESP: $EspRoot"
Write-Host "Serial log: $LogPath"
