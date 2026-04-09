param(
    [switch]$Release,
    [int]$TimeoutSeconds = 180,
    [int]$ModuleCount = 33
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($ModuleCount -le 32) {
    throw "ModuleCount must exceed the boot contract capacity (32)."
}

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-boot-refusal-log.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$RunId = Get-Date -Format "yyyyMMdd-HHmmss"
$Vars = Join-Path $RepoRoot ("target\qemu\edk2-x86_64-vars-boot-refusal-$RunId.fd")
$StageName = "limine-uefi-boot-refusal-$RunId"
$BuildImageName = "limine-uefi-boot-refusal-$RunId.img"
$ProofImageName = "limine-uefi-boot-refusal-$RunId-proof.img"
$EspImage = Join-Path $RepoRoot ("target\qemu\" + $ProofImageName)
$SerialLog = Join-Path $RepoRoot "target\qemu\serial-boot-refusal.log"
$DebugconLog = Join-Path $RepoRoot "target\qemu\debugcon-boot-refusal.log"
$StageDir = Join-Path $RepoRoot ("target\qemu\" + $StageName)
$StageConfig = Join-Path $StageDir "limine.conf"
$BootConfig = Join-Path $StageDir "EFI\BOOT\limine.conf"
$MakeEspScript = Join-Path $PSScriptRoot "make_esp_image.py"
$ProofModuleDir = Join-Path $StageDir "proof\modules"

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
    throw "Failed to build Limine UEFI boot refusal image."
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

New-Item -ItemType Directory -Force -Path $ProofModuleDir | Out-Null
$moduleLines = for ($index = 1; $index -le $ModuleCount; $index++) {
    $moduleName = "module-{0:D2}.bin" -f $index
    $modulePath = Join-Path $ProofModuleDir $moduleName
    Set-Content -Path $modulePath -Value ("ngos-boot-refusal-module-" + $index) -Encoding ascii
    "    module_path: boot():/proof/modules/$moduleName"
}
$moduleBlock = ($moduleLines -join "`n")
$ProofConfig = @"
timeout: 0
verbose: yes
serial: yes

/ngos_boot_refusal_too_many_modules
    protocol: limine
    path: boot():/kernel/ngos-boot-x86_64
$moduleBlock
    cmdline: console=ttyS0 earlyprintk=serial
"@
Set-Content -Path $StageConfig -Value $ProofConfig -Encoding ascii
Set-Content -Path $BootConfig -Value $ProofConfig -Encoding ascii
& python $MakeEspScript --source $StageDir --output $EspImage --size-mib 128
if ($LASTEXITCODE -ne 0) {
    throw "Failed to rebuild ESP image with boot refusal config."
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
                & $VerifyScript -LogPath $SerialLog -Detail "too-many-modules" -StatusHex "0x21" | Out-Null
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
            & $VerifyScript -LogPath $SerialLog -Detail "too-many-modules" -StatusHex "0x21"
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
Write-Host "Refusal detail: too-many-modules"
Write-Host "Serial log: $SerialLog"
