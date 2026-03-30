param(
    [int]$DurationSeconds = 18
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$Vars = Join-Path $RepoRoot "target\qemu\edk2-x86_64-vars-debug.fd"
$EspImage = Join-Path $RepoRoot "target\qemu\limine-uefi.img"
$SerialLog = Join-Path $RepoRoot "target\qemu\serial.log"
$DebugconLog = Join-Path $RepoRoot "target\qemu\debugcon.log"
$HostNetLog = Join-Path $RepoRoot "target\qemu\virtio-net-host.log"
$HostHelper = Join-Path $PSScriptRoot "virtio_net_host.py"

if (!(Test-Path $QemuExe)) {
    throw "QEMU executable not found at $QemuExe"
}
if (!(Test-Path $FirmwareSource)) {
    throw "UEFI firmware not found at $FirmwareSource"
}
if (!(Test-Path $VarsSource)) {
    throw "UEFI variable store not found at $VarsSource"
}
if (!(Test-Path $EspImage)) {
    throw "ESP image not found at $EspImage"
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

$HostProcess = Start-Process -FilePath "python" -ArgumentList @(
    $HostHelper,
    "--duration", [string]$DurationSeconds,
    "--log", $HostNetLog
) -PassThru

$Arguments = @(
    "-machine", "pc",
    "-m", "512M",
    "-cpu", "qemu64",
    "-drive", "if=pflash,format=raw,readonly=on,file=$Firmware",
    "-drive", "if=pflash,format=raw,file=$Vars",
    "-drive", "if=none,id=esp,format=raw,file=$EspImage",
    "-device", "virtio-blk-pci,drive=esp,bootindex=1",
    "-netdev", "socket,id=hostnet0,udp=127.0.0.1:10001,localaddr=127.0.0.1:10000",
    "-device", "virtio-net-pci,netdev=hostnet0",
    "-serial", "file:$SerialLog",
    "-debugcon", "file:$DebugconLog",
    "-global", "isa-debugcon.iobase=0xe9",
    "-display", "none",
    "-monitor", "none",
    "-no-reboot",
    "-no-shutdown"
)

$Process = Start-Process -FilePath $QemuExe -ArgumentList $Arguments -PassThru
Start-Sleep -Seconds $DurationSeconds
if (-not $Process.HasExited) {
    Stop-Process -Id $Process.Id -Force
}
if (-not $HostProcess.HasExited) {
    Stop-Process -Id $HostProcess.Id -Force
}
Start-Sleep -Seconds 1

Write-Output "---DEBUGCON---"
if (Test-Path $DebugconLog) {
    Get-Content $DebugconLog
}
Write-Output "---SERIAL---"
if (Test-Path $SerialLog) {
    Get-Content $SerialLog
}
Write-Output "---VIRTIO-NET-HOST---"
if (Test-Path $HostNetLog) {
    Get-Content $HostNetLog
}
