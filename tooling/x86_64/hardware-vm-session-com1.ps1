param(
    [Parameter(Mandatory = $true)]
    [string]$EspRoot,
    [switch]$Release,
    [switch]$Prune,
    [switch]$BackupFallback,
    [string]$BackupRoot = "",
    [int]$BaudRate = 38400,
    [int]$CaptureSeconds = 20,
    [int]$WaitForFirstByteSeconds = 120,
    [string]$LogPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$SessionScript = Join-Path $PSScriptRoot "hardware-boot-session.ps1"
$VerifyVmScript = Join-Path $PSScriptRoot "verify-hardware-vm-log.ps1"

& $SessionScript `
    -EspRoot $EspRoot `
    -PortName "COM1" `
    -Release:$Release `
    -Prune:$Prune `
    -BackupFallback:$BackupFallback `
    -BackupRoot $BackupRoot `
    -BaudRate $BaudRate `
    -CaptureSeconds $CaptureSeconds `
    -WaitForFirstByteSeconds $WaitForFirstByteSeconds `
    -LogPath $LogPath

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $LogPath = Join-Path $RepoRoot "target\hardware\serial-com1.log"
}

& $VerifyVmScript -LogPath $LogPath

Write-Host "Hardware VM session complete."
Write-Host "ESP: $EspRoot"
Write-Host "Serial log: $LogPath"
