param(
    [Parameter(Mandatory = $true)]
    [int]$DiskNumber,
    [char]$EspDriveLetter = 'X',
    [string]$Label = 'NGOS_BOOT'
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
        "-EspDriveLetter", $EspDriveLetter.ToString(),
        "-Label", $Label
    )
}

if (-not (Test-IsAdministrator)) {
    Write-Host "Administrator privileges are required. Relaunching elevated..."
    Invoke-ScriptElevated -ScriptPath $PSCommandPath -ArgumentList (Get-ArgList)
    return
}

$disk = Get-Disk -Number $DiskNumber -ErrorAction Stop
if ($disk.PartitionStyle -ne 'GPT') {
    throw "Disk $DiskNumber is not GPT."
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

Write-Host "Creating EFI System Partition from free space on Disk $DiskNumber..."
$newPartition = New-Partition -DiskNumber $DiskNumber -UseMaximumSize -DriveLetter $EspDriveLetter
Set-Partition -DiskNumber $DiskNumber -PartitionNumber $newPartition.PartitionNumber -GptType '{c12a7328-f81f-11d2-ba4b-00a0c93ec93b}'
Format-Volume -DriveLetter $EspDriveLetter -FileSystem FAT32 -NewFileSystemLabel $Label -Confirm:$false

$espRoot = "$EspDriveLetter`:\"
Ensure-Directory -Path (Join-NativePath -Base $espRoot -Relative "EFI\BOOT")

Write-Host "External ESP created successfully."
Write-Host "Disk: $DiskNumber"
Write-Host "ESP partition: $($newPartition.PartitionNumber)"
Write-Host "ESP root: $espRoot"
