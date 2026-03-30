param(
    [switch]$Release,
    [string]$StageName = "limine-uefi",
    [string]$ImageName = "limine-uefi.img"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-WithStageMutex {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Script
    )

    $createdNew = $false
    $mutex = New-Object System.Threading.Mutex($false, ("Global\ngos-" + $Name), [ref]$createdNew)
    try {
        if (-not $mutex.WaitOne([TimeSpan]::FromMinutes(5))) {
            throw "Timed out waiting for stage mutex: $Name"
        }
        & $Script
    }
    finally {
        try {
            $mutex.ReleaseMutex() | Out-Null
        }
        catch {
        }
        $mutex.Dispose()
    }
}

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$TargetJson = Join-Path $RepoRoot "platform-x86_64\targets\x86_64-ngos-kernel.json"
$UserTargetJson = Join-Path $RepoRoot "platform-x86_64\targets\x86_64-ngos-user.json"
$BuildProfile = if ($Release) { "release" } else { "debug" }
$KernelBinary = Join-Path $RepoRoot "target\x86_64-ngos-kernel\$BuildProfile\ngos-boot-x86_64"
$UserBinary = Join-Path $RepoRoot "target\x86_64-ngos-user\$BuildProfile\ngos-userland-native"
$DownloadsDir = Join-Path $RepoRoot "target\downloads"
$LimineTarball = Join-Path $DownloadsDir "limine-11.0.0-binary.tar.gz"
$LimineExtractDir = Join-Path $RepoRoot "target\limine\Limine-11.0.0-binary"
$StageDir = Join-Path $RepoRoot ("target\qemu\" + $StageName)
$EspImage = Join-Path $RepoRoot ("target\qemu\" + $ImageName)
$EfiBootDir = Join-Path $StageDir "EFI\BOOT"
$KernelDir = Join-Path $StageDir "kernel"
$LimineConfig = Join-Path $PSScriptRoot "limine.conf"
$StartupScript = Join-Path $PSScriptRoot "startup.nsh"
$MakeEspScript = Join-Path $PSScriptRoot "make_esp_image.py"

if (!(Test-Path $LimineTarball)) {
    throw "Limine binary tarball not found at $LimineTarball"
}

$CargoArgs = @(
    "+nightly",
    "-Z",
    "build-std=core,alloc",
    "-Z",
    "json-target-spec",
    "build",
    "-p",
    "ngos-boot-x86_64",
    "--bin",
    "ngos-boot-x86_64",
    "--target",
    $TargetJson
)
$UserCargoArgs = @(
    "+nightly",
    "-Z",
    "build-std=core,alloc",
    "-Z",
    "json-target-spec",
    "build",
    "-p",
    "ngos-userland-native",
    "--bin",
    "ngos-userland-native",
    "--target",
    $UserTargetJson
)
if ($Release) {
    $CargoArgs += "--release"
    $UserCargoArgs += "--release"
}

Invoke-WithStageMutex -Name ("limine-stage-" + $StageName) -Script {
    Push-Location $RepoRoot
    try {
        & cargo @CargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed."
        }
        & cargo @UserCargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "Userland cargo build failed."
        }
    }
    finally {
        Pop-Location
    }

    if (!(Test-Path $KernelBinary)) {
        throw "Freestanding kernel binary not found at $KernelBinary"
    }
    if (!(Test-Path $UserBinary)) {
        throw "Freestanding user binary not found at $UserBinary"
    }

    if (!(Test-Path (Join-Path $LimineExtractDir "BOOTX64.EFI"))) {
        $LimineExtractRoot = Split-Path -Parent $LimineExtractDir
        New-Item -ItemType Directory -Force -Path $LimineExtractRoot | Out-Null
        tar -xf $LimineTarball -C $LimineExtractRoot
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to extract Limine binary archive."
        }
    }

    if (Test-Path $StageDir) {
        Remove-Item -Recurse -Force $StageDir
    }

    New-Item -ItemType Directory -Force -Path $EfiBootDir, $KernelDir | Out-Null
    Copy-Item (Join-Path $LimineExtractDir "BOOTX64.EFI") (Join-Path $EfiBootDir "BOOTX64.EFI")
    Copy-Item $KernelBinary (Join-Path $KernelDir "ngos-boot-x86_64")
    Copy-Item $UserBinary (Join-Path $KernelDir "ngos-userland-native")
    Copy-Item $LimineConfig (Join-Path $StageDir "limine.conf")
    Copy-Item $LimineConfig (Join-Path $EfiBootDir "limine.conf")
    Copy-Item $StartupScript (Join-Path $StageDir "startup.nsh")

    & python $MakeEspScript --source $StageDir --output $EspImage --size-mib 128
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build FAT ESP image."
    }
}

Write-Host "Staged Limine UEFI directory: $StageDir"
Write-Host "ESP image: $EspImage"
Write-Host "Kernel binary: $KernelBinary"
Write-Host "User binary: $UserBinary"
Write-Host "UEFI loader: $(Join-Path $EfiBootDir 'BOOTX64.EFI')"
