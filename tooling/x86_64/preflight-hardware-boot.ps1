param(
    [Parameter(Mandatory = $true)]
    [string]$EspRoot,
    [string]$PortName = "COM1",
    [int]$CaptureSeconds = 20,
    [int]$WaitForFirstByteSeconds = 0,
    [string]$LogPath = ""
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

function Ensure-Directory {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (!(Test-Path -LiteralPath $Path)) {
        [System.IO.Directory]::CreateDirectory($Path) | Out-Null
    }
}

function Test-EspWritable {
    param([Parameter(Mandatory = $true)][string]$Root)

    $probeDir = Join-NativePath -Base $Root -Relative "EFI\BOOT"
    $probeFile = Join-NativePath -Base $probeDir -Relative ".ngos-write-probe.tmp"
    try {
        Ensure-Directory -Path $probeDir
        [System.IO.File]::WriteAllText($probeFile, "ngos-write-probe")
        return $true
    }
    catch [System.UnauthorizedAccessException] {
        return $false
    }
    finally {
        if (Test-Path -LiteralPath $probeFile) {
            Remove-Item -LiteralPath $probeFile -Force -ErrorAction SilentlyContinue
        }
    }
}

if (-not ("System.IO.Ports.SerialPort" -as [type])) {
    throw "System.IO.Ports.SerialPort type is not available in this PowerShell runtime."
}

if ($CaptureSeconds -le 0) {
    throw "CaptureSeconds must be greater than zero."
}
if ($WaitForFirstByteSeconds -lt 0) {
    throw "WaitForFirstByteSeconds must be zero or greater."
}

$espFull = Get-NormalizedEspPath -Path $EspRoot
if (!(Test-Path -LiteralPath $espFull)) {
    throw "ESP root not found: $espFull"
}

$espItem = Get-Item -LiteralPath $espFull
if (-not $espItem.PSIsContainer) {
    throw "ESP root is not a directory: $espFull"
}
if (-not (Test-EspWritable -Root $espFull)) {
    throw "ESP root is not writable from this PowerShell session: $espFull"
}

$availablePorts = @([System.IO.Ports.SerialPort]::GetPortNames() | Sort-Object)
if ($availablePorts -notcontains $PortName) {
    throw "Serial port not found: $PortName. Available ports: $($availablePorts -join ', ')"
}

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $LogPath = Join-Path $RepoRoot "target\hardware\serial-$($PortName.ToLowerInvariant()).log"
}

$logFull = [System.IO.Path]::GetFullPath($LogPath)
$logDir = Split-Path -Parent $logFull
if (!(Test-Path -LiteralPath $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

$probeFile = Join-Path $logDir ("preflight-" + [Guid]::NewGuid().ToString("N") + ".tmp")
try {
    Set-Content -LiteralPath $probeFile -Value "ngos-preflight" -Encoding ascii
}
finally {
    if (Test-Path -LiteralPath $probeFile) {
        Remove-Item -LiteralPath $probeFile -Force
    }
}

Write-Host "Hardware boot preflight passed."
Write-Host "ESP: $espFull"
Write-Host "Port: $PortName"
if ($WaitForFirstByteSeconds -gt 0) {
    Write-Host "WaitForFirstByteSeconds: $WaitForFirstByteSeconds"
}
Write-Host "Log: $logFull"
