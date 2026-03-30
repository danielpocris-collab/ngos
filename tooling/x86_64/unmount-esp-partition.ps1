param(
    [Parameter(Mandatory = $true)]
    [int]$DiskNumber,
    [Parameter(Mandatory = $true)]
    [int]$PartitionNumber,
    [string]$DriveLetter = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$partition = Get-Partition -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber
$volume = $null
try {
    $volume = $partition | Get-Volume -ErrorAction Stop
}
catch {
}

$targetLetter = ""
if ([string]::IsNullOrWhiteSpace($DriveLetter)) {
    if ($volume -and $volume.DriveLetter) {
        $targetLetter = $volume.DriveLetter.ToString().ToUpperInvariant()
    } else {
        throw "ESP is not mounted and no DriveLetter override was provided."
    }
} else {
    if ($DriveLetter.Length -ne 1 -or $DriveLetter -notmatch "^[A-Za-z]$") {
        throw "DriveLetter must be a single ASCII letter."
    }
    $targetLetter = $DriveLetter.ToUpperInvariant()
}

$accessPath = $targetLetter + ":\"
Remove-PartitionAccessPath -DiskNumber $DiskNumber -PartitionNumber $PartitionNumber -AccessPath $accessPath

Write-Host "ESP unmounted."
Write-Host "Disk: $DiskNumber"
Write-Host "Partition: $PartitionNumber"
Write-Host "Path: $accessPath"
