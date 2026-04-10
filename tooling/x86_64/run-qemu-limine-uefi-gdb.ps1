param(
    [switch]$Release,
    [int]$Port = 1234
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$EnvScript = Join-Path $PSScriptRoot "enable-dev-acceleration.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$Vars = Join-Path $RepoRoot "target\qemu\edk2-x86_64-vars-gdb.fd"
$EspImage = Join-Path $RepoRoot "target\qemu\limine-uefi.img"
$SerialLog = Join-Path $RepoRoot "target\qemu\serial-gdb.log"
$DebugconLog = Join-Path $RepoRoot "target\qemu\debugcon-gdb.log"

. $EnvScript

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
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

if (!(Test-Path $EspImage)) {
    throw "ESP image not found at $EspImage"
}

Write-Host ("QEMU gdbstub waiting on tcp::" + $Port)
Write-Host "Connect with: gdb <symbol-file> then 'target remote localhost:$Port'"

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
    -serial "file:$SerialLog" `
    -debugcon "file:$DebugconLog" `
    -global "isa-debugcon.iobase=0xe9" `
    -display none `
    -monitor none `
    -no-reboot `
    -no-shutdown `
    -gdb ("tcp::" + $Port) `
    -S
