param(
    [switch]$Release
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$EspImage = Join-Path $RepoRoot "target\qemu\limine-uefi.img"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$Vars = Join-Path $RepoRoot "target\qemu\edk2-x86_64-vars.fd"

if (!(Test-Path $QemuExe)) {
    throw "QEMU executable not found at $QemuExe"
}
if (!(Test-Path $FirmwareSource)) {
    throw "UEFI firmware not found at $FirmwareSource"
}
if (!(Test-Path $VarsSource)) {
    throw "UEFI variable store not found at $VarsSource"
}

& $BuildScript -Release:$Release
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build Limine UEFI image."
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force

if (!(Test-Path $EspImage)) {
    throw "ESP image not found at $EspImage"
}

& $QemuExe `
    -machine pc `
    -m 512M `
    -cpu qemu64 `
    -drive "if=pflash,format=raw,readonly=on,file=$Firmware" `
    -drive "if=pflash,format=raw,file=$Vars" `
    -drive "if=none,id=esp,format=raw,file=$EspImage" `
    -device "virtio-blk-pci,drive=esp,bootindex=1" `
    -netdev "socket,id=hostnet0,udp=127.0.0.1:10001,localaddr=127.0.0.1:10000" `
    -device "virtio-net-pci,netdev=hostnet0" `
    -serial stdio `
    -debugcon "file:$RepoRoot\\target\\qemu\\debugcon.log" `
    -global "isa-debugcon.iobase=0xe9" `
    -display none `
    -monitor none `
    -no-reboot `
    -no-shutdown
