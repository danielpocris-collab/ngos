param(
    [Parameter(Mandatory = $true)]
    [string]$EspRoot,
    [Parameter(Mandatory = $true)]
    [string]$BackupRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-NormalizedFullPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    if ($Path.StartsWith("\\?\")) {
        return $Path.TrimEnd('\') + '\'
    }
    return [System.IO.Path]::GetFullPath($Path)
}

function Join-NativePath {
    param(
        [Parameter(Mandatory = $true)][string]$Base,
        [Parameter(Mandatory = $true)][string]$Relative
    )

    $trimmedBase = $Base.TrimEnd('\')
    $trimmedRelative = ($Relative -replace '/', '\').TrimStart('\')
    if ([string]::IsNullOrWhiteSpace($trimmedRelative)) {
        return $trimmedBase + '\'
    }
    return $trimmedBase + '\' + $trimmedRelative
}

function Ensure-Directory {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (!(Test-Path -LiteralPath $Path)) {
        [System.IO.Directory]::CreateDirectory($Path) | Out-Null
    }
}

$espFull = Get-NormalizedFullPath -Path $EspRoot
$backupFull = Get-NormalizedFullPath -Path $BackupRoot

if (!(Test-Path -LiteralPath $espFull)) {
    throw "ESP root not found: $espFull"
}
if (!(Test-Path -LiteralPath $backupFull)) {
    throw "Backup root not found: $backupFull"
}

$backupBootDir = Join-NativePath -Base $backupFull -Relative "EFI\BOOT"
if (!(Test-Path -LiteralPath $backupBootDir)) {
    throw "Backup fallback directory not found: $backupBootDir"
}

$targetBootDir = Join-NativePath -Base $espFull -Relative "EFI\BOOT"
Ensure-Directory -Path $targetBootDir

$existingItems = @(Get-ChildItem -LiteralPath $targetBootDir -Force)
foreach ($item in $existingItems) {
    Remove-Item -LiteralPath $item.FullName -Force -Recurse:$item.PSIsContainer
}

$backupItems = @(Get-ChildItem -LiteralPath $backupBootDir -Force)
foreach ($item in $backupItems) {
    Copy-Item -LiteralPath $item.FullName -Destination (Join-NativePath -Base $targetBootDir -Relative $item.Name) -Recurse -Force
}

Write-Host "Hardware fallback restore completed."
Write-Host "ESP: $espFull"
Write-Host "Restored from: $backupFull"
