Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Format-Nullable {
    param($Value)
    if ($null -eq $Value -or $Value -eq "") {
        return "-"
    }
    return [string]$Value
}

$volumes = @()
try {
    $volumes = @(Get-Volume)
}
catch {
}

$partitions = @(Get-Partition | Sort-Object DiskNumber, PartitionNumber)
$results = @()

foreach ($partition in $partitions) {
    $volume = $null
    try {
        $volume = $partition | Get-Volume -ErrorAction Stop
    }
    catch {
    }

    $gptType = Format-Nullable $partition.GptType
    $type = Format-Nullable $partition.Type
    $driveLetter = if ($volume -and $volume.DriveLetter) { [string]$volume.DriveLetter } else { "-" }
    $fileSystem = if ($volume) { Format-Nullable $volume.FileSystem } else { "-" }
    $label = if ($volume) { Format-Nullable $volume.FileSystemLabel } else { "-" }
    $sizeBytes = if ($partition.Size) { [uint64]$partition.Size } else { 0 }
    $isEsp = ($gptType -match "c12a7328-f81f-11d2-ba4b-00a0c93ec93b") -or ($type -eq "System")
    $isFat = $fileSystem -match "^FAT"
    $isMounted = $driveLetter -ne "-"

    if ($isEsp -or $isFat) {
        $results += [PSCustomObject]@{
            Disk = $partition.DiskNumber
            Partition = $partition.PartitionNumber
            DriveLetter = $driveLetter
            FileSystem = $fileSystem
            Label = $label
            GptType = $gptType
            Type = $type
            SizeMiB = [Math]::Round($sizeBytes / 1MB, 2)
            MountedPath = if ($isMounted) { ($driveLetter + ":\") } else { "-" }
            CandidateReason = if ($isEsp -and $isFat) { "gpt-system+fat" } elseif ($isEsp) { "gpt-system" } else { "fat-volume" }
        }
    }
}

if ($results.Count -eq 0) {
    Write-Host "No ESP candidates found."
    exit 0
}

$results | Format-Table Disk, Partition, DriveLetter, FileSystem, Label, SizeMiB, CandidateReason, MountedPath -AutoSize
