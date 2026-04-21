param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Log file not found: $LogPath"
}

$text = Get-Content -LiteralPath $LogPath -Raw
$required = @(
    "boot.proof=compat-gfx",
    "compat.gfx.smoke.success request=",
    "frame=qemu-compat-001",
    "api=directx12",
    "translation=compat-to-vulkan",
    "deep-ops=clear,gradient-rect,flip-region",
    "compat.gfx.smoke.refusal request=missing errno=ENOENT outcome=expected",
    "compat.gfx.smoke.recovery request=",
    "frame=qemu-compat-002",
    "api=opengl",
    "deep-ops=clear,set-clip,clear-clip,flip-region",
    "compat-gfx-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing compat graphics marker: $marker"
    }
}

Write-Host "QEMU compat graphics log markers verified."
Write-Host "Log: $LogPath"
