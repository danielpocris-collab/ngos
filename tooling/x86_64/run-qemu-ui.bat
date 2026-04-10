@echo off
echo ========================================
echo   NGOS UI - QEMU Test
echo ========================================
echo.

cd /d "%~dp0.."

for /f %%I in ('powershell -NoProfile -Command "(Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors"') do set "NGOS_LOGICAL_CPUS=%%I"
set /a NGOS_CARGO_JOBS=(NGOS_LOGICAL_CPUS*80)/100
if %NGOS_CARGO_JOBS% LSS 1 set "NGOS_CARGO_JOBS=1"
set "CARGO_BUILD_JOBS=%NGOS_CARGO_JOBS%"

echo [1/3] Building kernel...
cargo build -p ngos-boot-x86_64
if %ERRORLEVEL% neq 0 (
    echo ERROR: Build failed!
    pause
    exit /b 1
)
echo      Build OK!
echo.

echo [2/3] Checking files...
if not exist "target\x86_64-ngos-kernel\debug\ngos-boot-x86_64" (
    echo ERROR: Kernel binary not found!
    pause
    exit /b 1
)
echo      Kernel OK!
echo.

echo [3/3] Starting QEMU...
echo.
echo ========================================
echo   QEMU is starting...
echo   You should see NGOS boot screen
echo ========================================
echo.
echo Controls:
echo   - Close QEMU: Ctrl+A then X
echo.

"C:\Program Files\qemu\qemu-system-x86_64.exe" ^
    -machine pc ^
    -m 512M ^
    -cpu qemu64 ^
    -device virtio-vga ^
    -drive if=pflash,format=raw,readonly=on,file="target\qemu\edk2-x86_64-code.fd" ^
    -drive if=pflash,format=raw,file="target\qemu\edk2-x86_64-vars-ui.fd" ^
    -drive if=none,id=esp,format=raw,file="target\qemu\limine-uefi-vm.img" ^
    -device virtio-blk-pci,drive=esp,bootindex=1 ^
    -serial file:"target\qemu\serial-ui.log" ^
    -debugcon file:"target\qemu\debugcon-ui.log" ^
    -global isa-debugcon.iobase=0xe9 ^
    -display gtk,gl=off ^
    -monitor none ^
    -no-reboot ^
    -no-shutdown

echo.
echo QEMU closed.
pause
