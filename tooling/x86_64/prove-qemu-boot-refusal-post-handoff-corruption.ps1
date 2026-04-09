param(
    [switch]$Release,
    [ValidateSet(
        "too-many-memory-regions",
        "invalid-hhdm-offset",
        "invalid-kernel-range-kind",
        "invalid-kernel-range-alignment",
        "empty-kernel-range",
        "invalid-memory-region-alignment",
        "empty-memory-region",
        "overlapping-memory-regions"
    )]
    [string]$Mode = "invalid-hhdm-offset",
    [int]$TimeoutSeconds = 180
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-boot-post-handoff-refusal-log.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$RunId = Get-Date -Format "yyyyMMdd-HHmmss"
$Vars = Join-Path $RepoRoot ("target\qemu\edk2-x86_64-vars-post-handoff-$Mode-$RunId.fd")
$StageName = "limine-uefi-post-handoff-$Mode-$RunId"
$BuildImageName = "limine-uefi-post-handoff-$Mode-$RunId.img"
$ProofImageName = "limine-uefi-post-handoff-$Mode-$RunId-proof.img"
$EspImage = Join-Path $RepoRoot ("target\qemu\" + $ProofImageName)
$SerialLog = Join-Path $RepoRoot ("target\qemu\serial-boot-post-handoff-$Mode.log")
$DebugconLog = Join-Path $RepoRoot ("target\qemu\debugcon-boot-post-handoff-$Mode.log")
$StageDir = Join-Path $RepoRoot ("target\qemu\" + $StageName)
$StageConfig = Join-Path $StageDir "limine.conf"
$BootConfig = Join-Path $StageDir "EFI\BOOT\limine.conf"
$MakeEspScript = Join-Path $PSScriptRoot "make_esp_image.py"

$detail = switch ($Mode) {
    "too-many-memory-regions" { "too-many-memory-regions" }
    "invalid-hhdm-offset" { "invalid-hhdm-offset" }
    "invalid-kernel-range-kind" { "invalid-kernel-range-kind" }
    "invalid-kernel-range-alignment" { "invalid-kernel-range-alignment" }
    "empty-kernel-range" { "empty-kernel-range" }
    "invalid-memory-region-alignment" { "invalid-memory-region-alignment" }
    "empty-memory-region" { "empty-memory-region" }
    "overlapping-memory-regions" { "overlapping-memory-regions" }
}
$statusHex = switch ($Mode) {
    "too-many-memory-regions" { "0x20" }
    "invalid-hhdm-offset" { "0x40" }
    "invalid-kernel-range-kind" { "0x41" }
    "invalid-kernel-range-alignment" { "0x42" }
    "empty-kernel-range" { "0x43" }
    "invalid-memory-region-alignment" { "0x44" }
    "empty-memory-region" { "0x45" }
    "overlapping-memory-regions" { "0x46" }
}

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
    throw "Failed to build Limine UEFI post-handoff corruption image."
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

$ProofConfig = @"
timeout: 0
verbose: yes
serial: yes

/ngos_boot_post_handoff_corruption
    protocol: limine
    path: boot():/kernel/ngos-boot-x86_64
    cmdline: console=ttyS0 earlyprintk=serial ngos.boot.handoff_corrupt=$Mode
"@
Set-Content -Path $StageConfig -Value $ProofConfig -Encoding ascii
Set-Content -Path $BootConfig -Value $ProofConfig -Encoding ascii
& python $MakeEspScript --source $StageDir --output $EspImage --size-mib 128
if ($LASTEXITCODE -ne 0) {
    throw "Failed to rebuild ESP image with post-handoff corruption config."
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
                & $VerifyScript -LogPath $SerialLog -Mode $Mode -Detail $detail -StatusHex $statusHex | Out-Null
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
            & $VerifyScript -LogPath $SerialLog -Mode $Mode -Detail $detail -StatusHex $statusHex
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

Write-Host "QEMU boot post-handoff refusal proof completed."
Write-Host "Mode: $Mode"
Write-Host "Detail: $detail"
Write-Host "Serial log: $SerialLog"
