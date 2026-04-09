param(
    [switch]$Release,
    [int]$TimeoutSeconds = 180,
    [int]$DimmCount = 8,
    [int]$DimmSizeMiB = 16
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$VerifyHeadroomScript = Join-Path $PSScriptRoot "verify-qemu-boot-memory-region-headroom.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$RunId = Get-Date -Format "yyyyMMdd-HHmmss"
$Vars = Join-Path $RepoRoot ("target\qemu\edk2-x86_64-vars-memory-scaling-$RunId.fd")
$StageName = "limine-uefi-memory-scaling-$RunId"
$ImageName = "limine-uefi-memory-scaling-$RunId.img"
$EspImage = Join-Path $RepoRoot ("target\qemu\" + $ImageName)
$SerialLog = Join-Path $RepoRoot "target\qemu\serial-boot-memory-region-scaling.log"
$DebugconLog = Join-Path $RepoRoot "target\qemu\debugcon-boot-memory-region-scaling.log"

& $BuildScript -Release:$Release -StageName $StageName -ImageName $ImageName
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build Limine UEFI memory-scaling image."
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

$maxMemMiB = 512 + ($DimmCount * $DimmSizeMiB) + 256
$arguments = @(
    "-machine", "pc",
    "-m", ("512M,slots=64,maxmem={0}M" -f $maxMemMiB),
    "-cpu", "qemu64",
    "-drive", "if=pflash,format=raw,readonly=on,file=$Firmware",
    "-drive", "if=pflash,format=raw,file=$Vars",
    "-drive", "if=none,id=esp,format=raw,file=$EspImage",
    "-device", "virtio-blk-pci,drive=esp,bootindex=1",
    "-serial", "file:$SerialLog",
    "-debugcon", "file:$DebugconLog",
    "-global", "isa-debugcon.iobase=0xe9",
    "-display", "none",
    "-monitor", "none",
    "-no-reboot",
    "-no-shutdown"
)

for ($index = 0; $index -lt $DimmCount; $index++) {
    $memoryId = "mem$index"
    $deviceId = "dimm$index"
    $arguments += "-object"
    $arguments += ("memory-backend-ram,id={0},size={1}M" -f $memoryId, $DimmSizeMiB)
    $arguments += "-device"
    $arguments += ("pc-dimm,id={0},memdev={1}" -f $deviceId, $memoryId)
}

$qemuProcess = $null
try {
    $qemuProcess = Start-Process -FilePath $QemuExe -ArgumentList $arguments -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $verified = $false

    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if (Test-Path $SerialLog) {
            try {
                & $VerifyHeadroomScript -LogPath $SerialLog | Out-Null
                $verified = $true
                break
            }
            catch {
            }
        }
        if ($qemuProcess.HasExited) {
            break
        }
    }

    if (-not $verified) {
        if (Test-Path $SerialLog) {
            & $VerifyHeadroomScript -LogPath $SerialLog
        }
        else {
            throw "Serial log was not produced: $SerialLog"
        }
    }
}
finally {
    if ($qemuProcess -and -not $qemuProcess.HasExited) {
        Stop-Process -Id $qemuProcess.Id -Force
    }
}

Write-Host "QEMU boot memory-region scaling inspection completed."
Write-Host "DIMMs: $DimmCount"
Write-Host "DIMM size MiB: $DimmSizeMiB"
Write-Host "Serial log: $SerialLog"
