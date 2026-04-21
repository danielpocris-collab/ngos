param(
    [switch]$Release,
    [int]$TimeoutSeconds = 300
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi-vm.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-vm-log.ps1"
$HostHelper = Join-Path $PSScriptRoot "virtio_net_host.py"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$Vars = Join-Path $RepoRoot "target\qemu\edk2-x86_64-vars-vm.fd"
$EspImage = Join-Path $RepoRoot "target\qemu\limine-uefi-vm.img"
$SerialLog = Join-Path $RepoRoot "target\qemu\serial-vm.log"
$DebugconLog = Join-Path $RepoRoot "target\qemu\debugcon-vm.log"
$HostNetLog = Join-Path $RepoRoot "target\qemu\virtio-net-host.log"
$StageDir = Join-Path $RepoRoot "target\qemu\limine-uefi-vm"
$StageConfig = Join-Path $StageDir "limine.conf"
$BootConfig = Join-Path $StageDir "EFI\BOOT\limine.conf"
$MakeEspScript = Join-Path $PSScriptRoot "make_esp_image.py"

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
    throw "Failed to build Limine UEFI VM image."
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog, $HostNetLog -ErrorAction SilentlyContinue

$ProofConfig = @"
timeout: 0
verbose: yes
serial: yes

/ngos_vm
    protocol: limine
    path: boot():/kernel/ngos-boot-x86_64
    module_path: boot():/kernel/ngos-userland-native
    cmdline: console=ttyS0 earlyprintk=serial ngos.boot.proof=vm
"@
Set-Content -Path $StageConfig -Value $ProofConfig -Encoding ascii
Set-Content -Path $BootConfig -Value $ProofConfig -Encoding ascii
& python $MakeEspScript --source $StageDir --output $EspImage --size-mib 128
if ($LASTEXITCODE -ne 0) {
    throw "Failed to rebuild ESP image with vm proof config."
}

$hostProcess = Start-Process -FilePath "python" -ArgumentList @(
    $HostHelper,
    "--duration", [string]$TimeoutSeconds,
    "--log", $HostNetLog
) -PassThru

$arguments = @(
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

$qemuProcess = $null
try {
    $qemuProcess = Start-Process -FilePath $QemuExe -ArgumentList $arguments -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $verified = $false

    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if (Test-Path $SerialLog) {
            try {
                & $VerifyScript -LogPath $SerialLog | Out-Null
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
            & $VerifyScript -LogPath $SerialLog
            $verified = $true
        } else {
            throw "Serial log was not produced: $SerialLog"
        }
    }
}
finally {
    if ($qemuProcess -and -not $qemuProcess.HasExited) {
        Stop-Process -Id $qemuProcess.Id -Force
    }
    if ($hostProcess -and -not $hostProcess.HasExited) {
        Stop-Process -Id $hostProcess.Id -Force
    }
}

Write-Host "QEMU VM smoke proof completed."
Write-Host "Serial log: $SerialLog"
