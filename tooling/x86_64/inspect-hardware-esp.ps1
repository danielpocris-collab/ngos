param(
    [Parameter(Mandatory = $true)]
    [string]$EspRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-NormalizedEspPath {
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

$espFull = Get-NormalizedEspPath -Path $EspRoot
if (!(Test-Path -LiteralPath $espFull)) {
    throw "ESP root not found: $espFull"
}

$requiredFiles = @(
    "EFI/BOOT/BOOTX64.EFI",
    "EFI/BOOT/limine.conf",
    "kernel/ngos-boot-x86_64",
    "kernel/ngos-userland-native",
    "limine.conf",
    "startup.nsh"
)

$missing = @()
foreach ($relative in $requiredFiles) {
    $path = Join-NativePath -Base $espFull -Relative $relative
    if (!(Test-Path -LiteralPath $path)) {
        $missing += $relative
    }
}

if ($missing.Length -ne 0) {
    throw ("ESP missing required boot files: " + ($missing -join " | "))
}

Write-Host "Hardware ESP layout verified."
Write-Host "ESP: $espFull"
foreach ($relative in $requiredFiles) {
    Write-Host ("OK " + $relative)
}
