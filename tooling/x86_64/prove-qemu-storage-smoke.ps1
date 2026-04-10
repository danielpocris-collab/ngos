param(
    [switch]$Release,
    [int]$TimeoutSeconds = 300
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-storage-log.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$RunId = Get-Date -Format "yyyyMMdd-HHmmss"
$StageName = "limine-uefi-storage-" + $RunId
$BuildImageName = "limine-uefi-storage-" + $RunId + ".img"
$StorageImage = Join-Path $RepoRoot "target\qemu\storage-proof.img"
$SerialLog = Join-Path $RepoRoot ("target\qemu\serial-storage-" + $RunId + ".log")
$DebugconLog = Join-Path $RepoRoot ("target\qemu\debugcon-storage-" + $RunId + ".log")
$CommitSerialLog = Join-Path $RepoRoot ("target\qemu\serial-storage-" + $RunId + "-commit.log")
$RecoverSerialLog = Join-Path $RepoRoot ("target\qemu\serial-storage-" + $RunId + "-recover.log")
$CorruptSerialLog = Join-Path $RepoRoot ("target\qemu\serial-storage-" + $RunId + "-corrupt.log")
$CommitDebugconLog = Join-Path $RepoRoot ("target\qemu\debugcon-storage-" + $RunId + "-commit.log")
$RecoverDebugconLog = Join-Path $RepoRoot ("target\qemu\debugcon-storage-" + $RunId + "-recover.log")
$CorruptDebugconLog = Join-Path $RepoRoot ("target\qemu\debugcon-storage-" + $RunId + "-corrupt.log")
$StageDir = Join-Path $RepoRoot ("target\qemu\" + $StageName)
$StageConfig = Join-Path $StageDir "limine.conf"
$BootConfig = Join-Path $StageDir "EFI\BOOT\limine.conf"
$MakeEspScript = Join-Path $PSScriptRoot "make_esp_image.py"

if (!(Test-Path $QemuExe)) { throw "QEMU executable not found at $QemuExe" }
if (!(Test-Path $FirmwareSource)) { throw "UEFI firmware not found at $FirmwareSource" }
if (!(Test-Path $VarsSource)) { throw "UEFI variable store not found at $VarsSource" }

& $BuildScript -Release:$Release -StageName $StageName -ImageName $BuildImageName
if ($LASTEXITCODE -ne 0) { throw "Failed to build Limine UEFI storage image." }

Copy-Item $FirmwareSource $Firmware -Force
Remove-Item $SerialLog, $DebugconLog, $CommitSerialLog, $RecoverSerialLog, $CorruptSerialLog, $CommitDebugconLog, $RecoverDebugconLog, $CorruptDebugconLog -ErrorAction SilentlyContinue

& python -c "from pathlib import Path; p=Path(r'$StorageImage'); p.parent.mkdir(parents=True, exist_ok=True); f=p.open('wb'); f.truncate(64*1024*1024); f.close()"
if ($LASTEXITCODE -ne 0) { throw "Failed to create storage proof disk." }

function Set-ProofConfig([string]$ProofName) {
    $PhaseEspImage = Join-Path $RepoRoot ("target\qemu\limine-uefi-storage-" + $RunId + "-" + $ProofName + ".img")
    $ProofConfig = @"
timeout: 0
verbose: yes
serial: yes

/ngos_storage
    protocol: limine
    path: boot():/kernel/ngos-boot-x86_64
    module_path: boot():/kernel/ngos-userland-native
    cmdline: console=ttyS0 earlyprintk=serial ngos.boot.proof=$ProofName
"@
    Set-Content -Path $StageConfig -Value $ProofConfig -Encoding ascii
    Set-Content -Path $BootConfig -Value $ProofConfig -Encoding ascii
    & python $MakeEspScript --source $StageDir --output $PhaseEspImage --size-mib 128
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to rebuild ESP image with storage proof config."
    }
    return $PhaseEspImage
}

function Invoke-QemuProof([string]$ProofName) {
    $PhaseEspImage = Set-ProofConfig $ProofName
    $PhaseVars = Join-Path $RepoRoot ("target\qemu\edk2-x86_64-vars-storage-" + $RunId + "-" + $ProofName + ".fd")
    $PhaseSerialLog = if ($ProofName -eq "storage-commit") { $CommitSerialLog } elseif ($ProofName -eq "storage-recover") { $RecoverSerialLog } else { $CorruptSerialLog }
    $PhaseDebugconLog = if ($ProofName -eq "storage-commit") { $CommitDebugconLog } elseif ($ProofName -eq "storage-recover") { $RecoverDebugconLog } else { $CorruptDebugconLog }
    Copy-Item $VarsSource $PhaseVars -Force
    $arguments = @(
        "-machine", "pc",
        "-m", "512M",
        "-cpu", "qemu64",
        "-drive", "if=pflash,format=raw,readonly=on,file=$Firmware",
        "-drive", "if=pflash,format=raw,file=$PhaseVars",
        "-drive", "if=none,id=storage,format=raw,file=$StorageImage",
        "-device", "virtio-blk-pci,drive=storage",
        "-drive", "if=none,id=esp,format=raw,file=$PhaseEspImage",
        "-device", "virtio-blk-pci,drive=esp,bootindex=1",
        "-serial", "file:$PhaseSerialLog",
        "-debugcon", "file:$PhaseDebugconLog",
        "-global", "isa-debugcon.iobase=0xe9",
        "-display", "none",
        "-monitor", "none",
        "-no-reboot",
        "-no-shutdown"
    )

    $process = $null
    try {
        $process = Start-Process -FilePath $QemuExe -ArgumentList $arguments -PassThru
        $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
        $marker = if ($ProofName -eq "storage-commit") { "storage-commit-smoke-ok" } elseif ($ProofName -eq "storage-recover") { "storage-recover-smoke-ok" } else { "storage-corrupt-smoke-ok" }
        while ((Get-Date) -lt $deadline) {
            Start-Sleep -Milliseconds 500
            if (Test-Path $PhaseSerialLog) {
                try {
                    if (Select-String -Path $PhaseSerialLog -SimpleMatch $marker -Quiet) {
                        return
                    }
                } catch {
                }
            }
            if ($process.HasExited) { break }
        }
        if (Test-Path $PhaseSerialLog) {
            try {
                if (Select-String -Path $PhaseSerialLog -SimpleMatch $marker -Quiet) {
                    return
                }
            } catch {
            }
        }
        throw "Storage proof phase did not complete: $ProofName"
    }
    finally {
        if ($process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }
}

Invoke-QemuProof "storage-commit"
Invoke-QemuProof "storage-recover"
Invoke-QemuProof "storage-corrupt"
@(
    if (Test-Path $CommitSerialLog) { Get-Content -Path $CommitSerialLog -Raw }
    if (Test-Path $RecoverSerialLog) { Get-Content -Path $RecoverSerialLog -Raw }
    if (Test-Path $CorruptSerialLog) { Get-Content -Path $CorruptSerialLog -Raw }
) | Set-Content -Path $SerialLog -Encoding ascii
& $VerifyScript -LogPath $SerialLog | Out-Null

Write-Host "QEMU storage proof completed."
Write-Host "Serial log: $SerialLog"
