param(
    [Parameter(Mandatory = $true)]
    [string]$EspRoot,
    [string]$BackupRoot = ""
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
if (!(Test-Path -LiteralPath $espFull)) {
    throw "ESP root not found: $espFull"
}

$bootDir = Join-NativePath -Base $espFull -Relative "EFI\BOOT"
if (!(Test-Path -LiteralPath $bootDir)) {
    throw "EFI fallback directory not found: $bootDir"
}

if ([string]::IsNullOrWhiteSpace($BackupRoot)) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $BackupRoot = Join-Path $RepoRoot ("target\hardware-backup\esp-fallback-" + $timestamp)
}

$backupFull = Get-NormalizedFullPath -Path $BackupRoot
$fallbackBackup = Join-NativePath -Base $backupFull -Relative "EFI\BOOT"
Ensure-Directory -Path $fallbackBackup

$items = @(Get-ChildItem -LiteralPath $bootDir -Force)
foreach ($item in $items) {
    Copy-Item -LiteralPath $item.FullName -Destination (Join-NativePath -Base $fallbackBackup -Relative $item.Name) -Recurse -Force
}

$manifestPath = Join-NativePath -Base $backupFull -Relative "backup-manifest.txt"
$manifest = @(
    "esp-root=$espFull"
    "fallback-source=$bootDir"
    "backup-created=$(Get-Date -Format o)"
)
Set-Content -LiteralPath $manifestPath -Value $manifest -Encoding ascii

Write-Host "Hardware fallback backup completed."
Write-Host "ESP: $espFull"
Write-Host "Source: $bootDir"
Write-Host "Backup: $backupFull"
