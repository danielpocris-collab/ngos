param(
    [Parameter(Mandatory = $true)]
    [int]$DiskNumber,
    [Parameter(Mandatory = $true)]
    [int]$PartitionNumber,
    [string]$DriveLetter = "X",
    [string]$Label = "NGOS_BOOT",
    [switch]$BackupFallback
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$CommonScript = Join-Path $PSScriptRoot "hardware-boot-common.ps1"
. $CommonScript

$DeployScript = Join-Path $PSScriptRoot "deploy-limine-uefi-hardware.ps1"
$espGuid = "{c12a7328-f81f-11d2-ba4b-00a0c93ec93b}"
$basicDataGuid = "{ebd0a0a2-b9e5-4433-87c0-68b6b72699c7}"

function Get-ArgList {
    $argList = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-DiskNumber", $DiskNumber.ToString(),
        "-PartitionNumber", $PartitionNumber.ToString(),
        "-DriveLetter", $DriveLetter,
        "-Label", $Label
    )
    if ($BackupFallback) {
        $argList += "-BackupFallback"
    }
    return $argList
}

if (-not (Test-IsAdministrator)) {
    Write-Host "Administrator privileges are required. Relaunching elevated..."
    Invoke-ScriptElevated -ScriptPath $PSCommandPath -ArgumentList (Get-ArgList)
    return
}

if ($DriveLetter.Length -ne 1 -or $DriveLetter -notmatch "^[A-Za-z]$") {
    throw "DriveLetter must be a single ASCII letter."
}

$targetLetter = $DriveLetter.ToUpperInvariant()
$accessPath = $targetLetter + ":\"
$partition = Get-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber -ErrorAction Stop
$disk = Get-Disk -Number $DiskNumber -ErrorAction Stop

if ($disk.PartitionStyle -ne "GPT") {
    throw "Disk $DiskNumber is not GPT."
}

if ($partition.Size -lt 128MB) {
    throw "Partition $DiskNumber/$PartitionNumber is unexpectedly small for an ESP."
}

Write-Host "Target partition:"
$partition | Format-Table DiskNumber, PartitionNumber, Type, GptType, Size

$restoreEspAtEnd = $partition.GptType -ne $espGuid
$mountedHere = $false

try {
    if ($partition.GptType -ne $basicDataGuid) {
        Write-Host "Temporarily switching partition to Basic Data for access-path assignment..."
        Set-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber -GptType $basicDataGuid
    }

    $volume = $null
    try {
        $volume = Get-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber | Get-Volume -ErrorAction Stop
    }
    catch {
    }

    if (-not $volume -or $volume.FileSystem -ne "FAT32" -or $volume.FileSystemLabel -ne $Label) {
        Write-Host "Formatting partition as FAT32 with label $Label..."
        Get-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber |
            Format-Volume -FileSystem FAT32 -NewFileSystemLabel $Label -Confirm:$false | Out-Null
    }

    $currentPartition = Get-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber
    $currentVolume = $null
    try {
        $currentVolume = $currentPartition | Get-Volume -ErrorAction Stop
    }
    catch {
    }

    $usedLetter = Get-Volume -DriveLetter $targetLetter -ErrorAction SilentlyContinue
    if ($usedLetter -and (-not $currentVolume -or $currentVolume.UniqueId -ne $usedLetter.UniqueId)) {
        throw "Drive letter $targetLetter is already in use."
    }

    if ($currentVolume -and $currentVolume.DriveLetter -and $currentVolume.DriveLetter.ToString().ToUpperInvariant() -eq $targetLetter) {
        Write-Host "Partition already mounted at $accessPath."
    } else {
        Write-Host "Mounting partition at $accessPath..."
        Add-PartitionAccessPath -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber -AccessPath $accessPath
        $mountedHere = $true
    }

    & $DeployScript -EspRoot $accessPath -BackupFallback:$BackupFallback
}
finally {
    if ($restoreEspAtEnd) {
        Write-Host "Restoring partition GPT type to EFI System Partition..."
        Set-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber -GptType $espGuid
    }
}

Write-Host "External ESP finalized."
Write-Host "Disk: $DiskNumber"
Write-Host "Partition: $PartitionNumber"
Write-Host "ESP: $accessPath"
Write-Host "Mounted in this session: $mountedHere"
