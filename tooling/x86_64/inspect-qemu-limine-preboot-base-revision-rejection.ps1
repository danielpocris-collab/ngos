param(
    [switch]$Release,
    [ValidateSet("missing-base-revision", "unsupported-base-revision")]
    [string]$Mode = "missing-base-revision",
    [int]$TimeoutSeconds = 180
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BuildScript = Join-Path $PSScriptRoot "build-limine-uefi.ps1"
$VerifyScript = Join-Path $PSScriptRoot "verify-qemu-limine-preboot-base-revision-rejection-log.ps1"
$QemuExe = "C:\Program Files\qemu\qemu-system-x86_64.exe"
$FirmwareSource = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$VarsSource = "C:\Program Files\qemu\share\edk2-i386-vars.fd"
$Firmware = Join-Path $RepoRoot "target\qemu\edk2-x86_64-code.fd"
$RunId = Get-Date -Format "yyyyMMdd-HHmmss"
$Vars = Join-Path $RepoRoot ("target\qemu\edk2-x86_64-vars-limine-preboot-base-revision-$Mode-$RunId.fd")
$StageName = "limine-uefi-preboot-base-revision-$Mode-$RunId"
$BuildImageName = "limine-uefi-preboot-base-revision-$Mode-$RunId.img"
$ProofImageName = "limine-uefi-preboot-base-revision-$Mode-$RunId-proof.img"
$EspImage = Join-Path $RepoRoot ("target\qemu\" + $ProofImageName)
$SerialLog = Join-Path $RepoRoot ("target\qemu\serial-limine-preboot-$Mode.log")
$DebugconLog = Join-Path $RepoRoot ("target\qemu\debugcon-limine-preboot-$Mode.log")
$StageDir = Join-Path $RepoRoot ("target\qemu\" + $StageName)
$KernelPath = Join-Path $StageDir "kernel\ngos-boot-x86_64"
$MakeEspScript = Join-Path $PSScriptRoot "make_esp_image.py"

$baseRevisionPattern = [byte[]](
    0xC8, 0xA6, 0x95, 0x5C, 0x2D, 0x2B, 0x56, 0xF9,
    0xDC, 0x6B, 0x53, 0x44, 0x49, 0x38, 0x7B, 0x6A
)

function Find-BytePatternOffset {
    param(
        [byte[]]$Haystack,
        [byte[]]$Needle
    )

    for ($i = 0; $i -le $Haystack.Length - $Needle.Length; $i++) {
        $matched = $true
        for ($j = 0; $j -lt $Needle.Length; $j++) {
            if ($Haystack[$i + $j] -ne $Needle[$j]) {
                $matched = $false
                break
            }
        }
        if ($matched) {
            return $i
        }
    }

    return -1
}

& $BuildScript -Release:$Release -StageName $StageName -ImageName $BuildImageName
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build Limine UEFI base-revision preboot image."
}

Copy-Item $FirmwareSource $Firmware -Force
Copy-Item $VarsSource $Vars -Force
Remove-Item $SerialLog, $DebugconLog -ErrorAction SilentlyContinue

$kernelBytes = [System.IO.File]::ReadAllBytes($KernelPath)
$baseRevisionOffset = Find-BytePatternOffset -Haystack $kernelBytes -Needle $baseRevisionPattern
if ($baseRevisionOffset -lt 0) {
    throw "Failed to locate Limine BaseRevision tag in staged kernel image: $KernelPath"
}

switch ($Mode) {
    "missing-base-revision" {
        [byte[]]$replacement = 0, 0, 0, 0, 0, 0, 0, 0
        [Array]::Copy($replacement, 0, $kernelBytes, $baseRevisionOffset, 8)
    }
    "unsupported-base-revision" {
        $revisionOffset = $baseRevisionOffset + 16
        [byte[]]$replacement = 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F
        [Array]::Copy($replacement, 0, $kernelBytes, $revisionOffset, 8)
    }
}

[System.IO.File]::WriteAllBytes($KernelPath, $kernelBytes)

& python $MakeEspScript --source $StageDir --output $EspImage --size-mib 128
if ($LASTEXITCODE -ne 0) {
    throw "Failed to rebuild ESP image with base-revision preboot mutation."
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
                & $VerifyScript -LogPath $SerialLog -Mode $Mode | Out-Null
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
            & $VerifyScript -LogPath $SerialLog -Mode $Mode
        }
        else {
            throw "Serial log was not produced: $SerialLog"
        }
    }
}
finally {
    if ($qemuProcess -and -not $qemuProcess.HasExited) {
        Stop-Process -Id $qemuProcess.Id -Force
    }
}

Write-Host "Limine preboot base-revision rejection inspection completed."
Write-Host "Mode: $Mode"
Write-Host "Serial log: $SerialLog"
