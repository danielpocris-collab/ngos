param(
    [ValidateSet("record", "replay")]
    [string]$Mode = "record",
    [switch]$Release,
    [string]$ReplayFile = "target\\qemu\\ngos-replay.bin",
    [string]$SnapshotName = "ngos-init"
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
$Vars = Join-Path $RepoRoot "target\qemu\edk2-x86_64-vars-replay.fd"
$SerialLog = Join-Path $RepoRoot ("target\\qemu\\serial-replay-" + $Mode + ".log")
$DebugconLog = Join-Path $RepoRoot ("target\\qemu\\debugcon-replay-" + $Mode + ".log")
$OverlayImage = Join-Path $RepoRoot "target\qemu\limine-uefi-replay.qcow2"
$ReplayPath = Join-Path $RepoRoot $ReplayFile
$BaseEspImage = Join-Path $RepoRoot "target\qemu\limine-uefi.img"

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

if (!(Test-Path $BaseEspImage)) {
    throw "ESP image not found at $BaseEspImage"
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $ReplayPath) | Out-Null
if ($Mode -eq "record") {
    Remove-Item $ReplayPath, $OverlayImage -ErrorAction SilentlyContinue
    & qemu-img create -f qcow2 -F raw -b $BaseEspImage $OverlayImage | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to create replay overlay image."
    }
}
elseif (!(Test-Path $ReplayPath)) {
    throw "Replay log not found at $ReplayPath"
}
elseif (!(Test-Path $OverlayImage)) {
    throw "Replay overlay image not found at $OverlayImage"
}

$RrOption = ("shift=auto,rr=" + $Mode + ",rrfile=" + $ReplayPath + ",rrsnapshot=" + $SnapshotName)

& $QemuExe `
    -machine pc `
    -m 512M `
    -cpu qemu64 `
    -icount $RrOption `
    -drive "if=pflash,format=raw,readonly=on,file=$Firmware" `
    -drive "if=pflash,format=raw,file=$Vars" `
    -drive "file=$OverlayImage,if=none,id=img-direct" `
    -drive "driver=blkreplay,if=none,image=img-direct,id=img-blkreplay" `
    -device "virtio-blk-pci,drive=img-blkreplay,bootindex=1" `
    -serial "file:$SerialLog" `
    -debugcon "file:$DebugconLog" `
    -global "isa-debugcon.iobase=0xe9" `
    -display none `
    -monitor none `
    -no-reboot `
    -no-shutdown
