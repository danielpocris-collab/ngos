param(
    [switch]$Release,
    [int]$TimeoutSeconds = 180
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-boot-refusal-log.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$RunId = Get-Date -Format "yyyyMMdd-HHmmss"
$Vars = Join-Path $RepoRoot ("target\qemu\edk2-x86_64-vars-invalid-cmdline-$RunId.fd")
$StageName = "limine-uefi-invalid-cmdline-$RunId"
$BuildImageName = "limine-uefi-invalid-cmdline-$RunId.img"
$ProofImageName = "limine-uefi-invalid-cmdline-$RunId-proof.img"
$EspImage = Join-Path $RepoRoot ("target\qemu\" + $ProofImageName)
$SerialLog = Join-Path $RepoRoot "target\qemu\serial-boot-refusal-invalid-cmdline.log"
$DebugconLog = Join-Path $RepoRoot "target\qemu\debugcon-boot-refusal-invalid-cmdline.log"
$StageDir = Join-Path $RepoRoot ("target\qemu\" + $StageName)
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

& $BuildScript -Release:$Release -StageName $StageName -ImageName $BuildImageName
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build Limine UEFI invalid-cmdline image."
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

$configBytes = [System.Collections.Generic.List[byte]]::new()
foreach ($line in @(
    "timeout: 0",
    "verbose: yes",
    "serial: yes",
    "",
    "/ngos_boot_refusal_invalid_command_line_utf8",
    "    protocol: limine",
    "    path: boot():/kernel/ngos-boot-x86_64",
    "    module_path: boot():/kernel/ngos-userland-native"
)) {
    $configBytes.AddRange([System.Text.Encoding]::ASCII.GetBytes($line))
    $configBytes.Add(10)
}
$configBytes.AddRange([System.Text.Encoding]::ASCII.GetBytes(
        "    cmdline: console=ttyS0 earlyprintk=serial refusal="
    ))
$configBytes.AddRange([byte[]](0xC3, 0x28))
$configBytes.Add(10)
[System.IO.File]::WriteAllBytes($StageConfig, $configBytes.ToArray())
[System.IO.File]::WriteAllBytes($BootConfig, $configBytes.ToArray())

& python $MakeEspScript --source $StageDir --output $EspImage --size-mib 128
if ($LASTEXITCODE -ne 0) {
    throw "Failed to rebuild ESP image with invalid-cmdline config."
}

$arguments = @(
    "-machine", "pc",
    "-m", "512M",
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

$qemuProcess = $null
try {
    $qemuProcess = Start-Process -FilePath $QemuExe -ArgumentList $arguments -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $verified = $false

    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if (Test-Path $SerialLog) {
            try {
                & $VerifyScript -LogPath $SerialLog -Detail "invalid-command-line-utf8" -StatusHex "0x30" | Out-Null
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
            & $VerifyScript -LogPath $SerialLog -Detail "invalid-command-line-utf8" -StatusHex "0x30"
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
}

Write-Host "QEMU boot refusal proof completed."
Write-Host "Refusal detail: invalid-command-line-utf8"
Write-Host "Serial log: $SerialLog"
