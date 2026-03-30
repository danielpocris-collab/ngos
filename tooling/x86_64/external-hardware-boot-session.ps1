param(
    [Parameter(Mandatory = $true)]
    [int]$DiskNumber,
    [Parameter(Mandatory = $true)]
    [int]$PartitionNumber,
    [string]$MountPath = "",
    [string]$PortName = "COM1",
    [int]$BaudRate = 38400,
    [int[]]$ScanBaudRates = @(38400, 115200, 57600, 19200, 9600),
    [int]$CaptureSeconds = 20,
    [int]$WaitForFirstByteSeconds = 120,
    [int]$ScanWaitPerBaudSeconds = 20,
    [int]$ScanCaptureAfterFirstByteSeconds = 5,
    [string]$LogPath = "",
    [switch]$Release,
    [switch]$Prune,
    [switch]$BackupFallback,
    [switch]$SkipSerialScanOnFailure
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$CommonScript = Join-Path $PSScriptRoot "hardware-boot-common.ps1"
. $CommonScript

$DeployScript = Join-Path $PSScriptRoot "deploy-limine-uefi-hardware.ps1"
$PreflightScript = Join-Path $PSScriptRoot "preflight-hardware-boot.ps1"
$InspectEspScript = Join-Path $PSScriptRoot "inspect-hardware-esp.ps1"
$CaptureScript = Join-Path $PSScriptRoot "capture-hardware-serial.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-hardware-boot-log.ps1"
$ScanScript = Join-Path $PSScriptRoot "scan-hardware-serial.ps1"
$InspectSerialScript = Join-Path $PSScriptRoot "inspect-hardware-serial.ps1"

if ([string]::IsNullOrWhiteSpace($MountPath)) {
    $MountPath = Join-Path $RepoRoot "target\hardware\external-esp-mount-disk$DiskNumber-part$PartitionNumber"
}
if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $LogPath = Join-Path $RepoRoot "target\hardware\external-disk$DiskNumber-part$PartitionNumber-$($PortName.ToLowerInvariant()).log"
}

$mountFull = [System.IO.Path]::GetFullPath($MountPath)
$logFull = [System.IO.Path]::GetFullPath($LogPath)
$scanOutputDir = Join-Path (Split-Path -Parent $logFull) ("serial-scan-" + $PortName.ToLowerInvariant())

if (-not (Test-IsAdministrator)) {
    throw "Administrator privileges are required for external ESP mount operations."
}

function Ensure-EmptyDirectory {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (Test-Path -LiteralPath $Path) {
        $existing = Get-ChildItem -LiteralPath $Path -Force
        if ($existing.Count -ne 0) {
            throw "MountPath must be empty before mounting the ESP: $Path"
        }
    } else {
        New-Item -ItemType Directory -Path $Path -Force | Out-Null
    }
}

function Get-ExternalEspVolumePath {
    param(
        [Parameter(Mandatory = $true)][int]$DiskNumber,
        [Parameter(Mandatory = $true)][int]$PartitionNumber
    )

    $partition = Get-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber -ErrorAction Stop
    if ($partition.GptType -ne "{c12a7328-f81f-11d2-ba4b-00a0c93ec93b}") {
        throw "Partition $DiskNumber/$PartitionNumber is not an EFI System Partition."
    }

    $volume = $partition | Get-Volume -ErrorAction Stop
    if ([string]::IsNullOrWhiteSpace($volume.UniqueId)) {
        throw "Partition $DiskNumber/$PartitionNumber has no volume GUID."
    }
    return $volume.UniqueId
}

function Mount-ExternalEsp {
    param(
        [Parameter(Mandatory = $true)][string]$VolumePath,
        [Parameter(Mandatory = $true)][string]$MountPath
    )

    Ensure-EmptyDirectory -Path $MountPath
    & mountvol $MountPath $VolumePath
    if ($LASTEXITCODE -ne 0) {
        throw "mountvol failed for $VolumePath -> $MountPath"
    }
}

function Dismount-ExternalEsp {
    param([Parameter(Mandatory = $true)][string]$MountPath)

    if (Test-Path -LiteralPath $MountPath) {
        & mountvol $MountPath /D
        if ($LASTEXITCODE -ne 0) {
            throw "mountvol /D failed for $MountPath"
        }
    }
}

$volumePath = $null
$mountedHere = $false
$serialVerified = $false

try {
    Set-Disk -Number $DiskNumber -IsOffline $false
    $volumePath = Get-ExternalEspVolumePath -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber
    Mount-ExternalEsp -VolumePath $volumePath -MountPath $mountFull
    $mountedHere = $true

    & $PreflightScript -EspRoot $mountFull -PortName $PortName -CaptureSeconds $CaptureSeconds -WaitForFirstByteSeconds $WaitForFirstByteSeconds -LogPath $logFull
    & $DeployScript -EspRoot $mountFull -Release:$Release -Prune:$Prune -BackupFallback:$BackupFallback
    & $InspectEspScript -EspRoot $mountFull
    & $InspectSerialScript -PortName $PortName -BaudRate $BaudRate

    Write-Host "Boot the target hardware now."
    if ($WaitForFirstByteSeconds -gt 0) {
        Write-Host "Waiting up to $WaitForFirstByteSeconds seconds for first serial byte on $PortName..."
    }
    Write-Host "Capturing $CaptureSeconds seconds after first serial byte."

    & $CaptureScript -PortName $PortName -BaudRate $BaudRate -DurationSeconds $CaptureSeconds -WaitForFirstByteSeconds $WaitForFirstByteSeconds -LogPath $logFull -RequireData
    & $VerifyScript -LogPath $logFull
    $serialVerified = $true
}
catch {
    $failure = $_
    Write-Warning ("External hardware boot session failed: " + $failure.Exception.Message)

    if (-not $SkipSerialScanOnFailure) {
        try {
            Write-Host "Running serial baud scan for diagnostics..."
            & $ScanScript `
                -PortName $PortName `
                -BaudRates $ScanBaudRates `
                -WaitPerBaudSeconds $ScanWaitPerBaudSeconds `
                -CaptureAfterFirstByteSeconds $ScanCaptureAfterFirstByteSeconds `
                -OutputDirectory $scanOutputDir
        }
        catch {
            Write-Warning ("Serial baud scan also failed: " + $_.Exception.Message)
        }
    }

    throw $failure
}
finally {
    if ($mountedHere) {
        Dismount-ExternalEsp -MountPath $mountFull
    }
}

Write-Host "External hardware boot session complete."
Write-Host "Disk: $DiskNumber"
Write-Host "Partition: $PartitionNumber"
Write-Host "ESP mount path: $mountFull"
Write-Host "Serial verified: $serialVerified"
Write-Host "Serial log: $logFull"
