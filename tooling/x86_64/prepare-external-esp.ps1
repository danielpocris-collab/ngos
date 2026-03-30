param(
    [Parameter(Mandatory = $true)]
    [int]$DiskNumber,
    [Parameter(Mandatory = $true)]
    [int]$DataPartitionNumber,
    [int]$EspSizeMiB = 512,
    [char]$EspDriveLetter = 'X'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$CommonScript = Join-Path $PSScriptRoot "hardware-boot-common.ps1"
. $CommonScript

function Get-ArgList {
    return @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $PSCommandPath,
        "-DiskNumber", $DiskNumber.ToString(),
        "-DataPartitionNumber", $DataPartitionNumber.ToString(),
        "-EspSizeMiB", $EspSizeMiB.ToString(),
        "-EspDriveLetter", $EspDriveLetter.ToString()
    )
}

if (-not (Test-IsAdministrator)) {
    Write-Host "Administrator privileges are required. Relaunching elevated..."
    Invoke-ScriptElevated -ScriptPath $PSCommandPath -ArgumentList (Get-ArgList)
    return
}

$disk = Get-Disk -Number $DiskNumber -ErrorAction Stop
$partition = Get-Partition -DiskNumber $DiskNumber -PartitionNumber $DataPartitionNumber -ErrorAction Stop
$volume = if ($partition.DriveLetter) {
    Get-Volume -DriveLetter $partition.DriveLetter -ErrorAction Stop
} else {
    throw "Data partition Disk $DiskNumber / Partition $DataPartitionNumber has no drive letter."
}

if ($disk.PartitionStyle -ne 'GPT') {
    throw "Disk $DiskNumber is not GPT."
}
if ($volume.FileSystem -ne 'NTFS') {
    throw "Data partition must be NTFS. Found $($volume.FileSystem)."
}

$espBytes = [uint64]$EspSizeMiB * 1MB
$supported = Get-PartitionSupportedSize -DiskNumber $DiskNumber -PartitionNumber $DataPartitionNumber
$currentSize = [uint64]$partition.Size
$newSize = $currentSize - $espBytes

if ($newSize -lt $supported.SizeMin) {
    $minMiB = [math]::Ceiling($supported.SizeMin / 1MB)
    throw "Partition cannot be shrunk enough. Minimum supported size is ${minMiB} MiB."
}

$existingEsp = Get-Partition -DiskNumber $DiskNumber | Where-Object {
    $_.GptType -eq '{c12a7328-f81f-11d2-ba4b-00a0c93ec93b}'
}
if ($existingEsp) {
    throw "Disk $DiskNumber already has an EFI System Partition."
}

$existingLetter = Get-Volume -DriveLetter $EspDriveLetter -ErrorAction SilentlyContinue
if ($existingLetter) {
    throw "Drive letter $EspDriveLetter is already in use."
}

Write-Host "Shrinking Disk $DiskNumber Partition $DataPartitionNumber from $([math]::Floor($currentSize / 1MB)) MiB to $([math]::Floor($newSize / 1MB)) MiB..."
Resize-Partition -DiskNumber $DiskNumber -PartitionNumber $DataPartitionNumber -Size $newSize

Write-Host "Creating ${EspSizeMiB} MiB EFI System Partition on Disk $DiskNumber..."
$newPartition = New-Partition -DiskNumber $DiskNumber -Size $espBytes -DriveLetter $EspDriveLetter
Set-Partition -DiskNumber $DiskNumber -PartitionNumber $newPartition.PartitionNumber -GptType '{c12a7328-f81f-11d2-ba4b-00a0c93ec93b}'

Write-Host "Formatting $EspDriveLetter`: as FAT32..."
Format-Volume -DriveLetter $EspDriveLetter -FileSystem FAT32 -NewFileSystemLabel "NGOS_BOOT" -Confirm:$false

$espRoot = "$EspDriveLetter`:\"
Ensure-Directory -Path (Join-NativePath -Base $espRoot -Relative "EFI\BOOT")

Write-Host "External ESP prepared successfully."
Write-Host "Disk: $DiskNumber"
Write-Host "Data partition: $DataPartitionNumber"
Write-Host "ESP partition: $($newPartition.PartitionNumber)"
Write-Host "ESP root: $espRoot"
