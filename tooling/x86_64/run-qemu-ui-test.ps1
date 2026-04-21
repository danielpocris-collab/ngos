param(
    [switch]$Release,
    [string]$Display = "gtk,zoom-to-fit=on"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $env:CARGO_BUILD_JOBS) {
    $logical = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
    $jobs = [math]::Max(1, [int][math]::Floor($logical * 0.8))
    $env:CARGO_BUILD_JOBS = "$jobs"
}

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi-vm.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$Vars = Join-Path $RepoRoot "target\qemu\edk2-x86_64-vars-vm.fd"
$EspImage = Join-Path $RepoRoot "target\qemu\limine-uefi-vm.img"
$SerialLog = Join-Path $RepoRoot "target\qemu\serial-ui.log"
$DebugconLog = Join-Path $RepoRoot "target\qemu\debugcon-ui.log"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  NGOS UI - QEMU Test" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

if (!(Test-Path $QemuExe)) {
    Write-Host "ERROR: QEMU not found at $QemuExe" -ForegroundColor Red
    exit 1
}
if (!(Test-Path $FirmwareSource)) {
    Write-Host "ERROR: UEFI firmware not found at $FirmwareSource" -ForegroundColor Red
    exit 1
}
if (!(Test-Path $VarsSource)) {
    Write-Host "ERROR: UEFI variable store not found at $VarsSource" -ForegroundColor Red
    exit 1
}

Write-Host "[1/4] Building NGOS VM/UI image..." -ForegroundColor Green
& $BuildScript -Release:$Release
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: VM/UI image build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "      VM/UI image completed successfully!" -ForegroundColor Green
Write-Host ""

Write-Host "[2/4] Preparing UEFI firmware..." -ForegroundColor Green
Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Write-Host "      Firmware ready!" -ForegroundColor Green
Write-Host ""

Write-Host "[3/4] Verifying ESP image..." -ForegroundColor Green
if (!(Test-Path $EspImage)) {
    Write-Host "ERROR: ESP image not found at $EspImage" -ForegroundColor Red
    exit 1
}
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue
Write-Host "      ESP image ready!" -ForegroundColor Green
Write-Host ""

Write-Host "[4/4] Starting QEMU..." -ForegroundColor Green
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  QEMU is now running!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "What you should see:" -ForegroundColor Yellow
Write-Host "  1. NGOS Boot Screen with logo" -ForegroundColor White
Write-Host "  2. Progress bar and boot messages" -ForegroundColor White
Write-Host "  3. Desktop with taskbar/window manager" -ForegroundColor White
Write-Host ""
Write-Host "Controls:" -ForegroundColor Yellow
Write-Host "  - Mouse: Click in window to capture" -ForegroundColor White
Write-Host "  - Release mouse: Ctrl+Alt" -ForegroundColor White
Write-Host "  - Close QEMU: Close window or press Ctrl+A then X" -ForegroundColor White
Write-Host ""
Write-Host "Serial log: $SerialLog" -ForegroundColor Cyan
Write-Host ""

$arguments = @(
    "-machine", "pc",
    "-m", "512M",
    "-cpu", "qemu64",
    "-device", "virtio-vga",
    "-drive", "if=pflash,format=raw,readonly=on,file=$Firmware",
    "-drive", "if=pflash,format=raw,file=$Vars",
    "-drive", "if=none,id=esp,format=raw,file=$EspImage",
    "-device", "virtio-blk-pci,drive=esp,bootindex=1",
    "-serial", "file:$SerialLog",
    "-debugcon", "file:$DebugconLog",
    "-global", "isa-debugcon.iobase=0xe9",
    "-display", $Display,
    "-monitor", "none",
    "-no-reboot",
    "-no-shutdown"
)

Start-Process -FilePath $QemuExe -ArgumentList $arguments -Wait

Write-Host ""
Write-Host "QEMU closed." -ForegroundColor Green
Write-Host "Check serial log: $SerialLog" -ForegroundColor Cyan
