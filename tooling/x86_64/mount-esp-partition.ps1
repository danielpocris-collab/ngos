param(
    [Parameter(Mandatory = $true)]
    [int]$DiskNumber,
    [Parameter(Mandatory = $true)]
    [int]$PartitionNumber,
    [string]$DriveLetter = "S"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($DriveLetter.Length -ne 1 -or $DriveLetter -notmatch "^[A-Za-z]$") {
    throw "DriveLetter must be a single ASCII letter."
}

$targetLetter = $DriveLetter.ToUpperInvariant()
$partition = Get-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber

if ($partition.GptType -notmatch "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" -and $partition.Type -ne "System") {
    throw "Partition $DiskNumber/$PartitionNumber is not an ESP/System partition."
}

$existingVolume = $null
try {
    $existingVolume = $partition | Get-Volume -ErrorAction Stop
}
catch {
}

if ($existingVolume -and $existingVolume.DriveLetter) {
    Write-Host "ESP already mounted."
    Write-Host ("Path: " + $existingVolume.DriveLetter + ":\")
    exit 0
}

$targetPath = $targetLetter + ":\"
$usedLetters = @(Get-Volume | Where-Object { $_.DriveLetter } | ForEach-Object { $_.DriveLetter.ToString().ToUpperInvariant() })
if ($usedLetters -contains $targetLetter) {
    throw "Drive letter already in use: $targetLetter"
}

Add-PartitionAccessPath -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber -AccessPath $targetPath

Write-Host "ESP mounted."
Write-Host "Disk: $DiskNumber"
Write-Host "Partition: $PartitionNumber"
Write-Host "Path: $targetPath"
