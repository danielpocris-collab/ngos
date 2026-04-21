param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $LogPath)) {
    throw "Log file not found: $LogPath"
}

$content = Get-Content -LiteralPath $LogPath -Raw
$markers = @(
    "render3d.smoke.init renderer=640x480",
    "render3d.smoke.mesh registered id=1 vertices=3",
    "render3d.smoke.material registered id=1",
    "render3d.smoke.light added type=directional",
    "render3d.smoke.pass created id=1 type=geometry",
    "render3d.smoke.render triangles=1 pixels=",
    "render3d.smoke.pixel x=320 y=240 r=255 g=0 b=0",
    "render3d.smoke.depth depth=0.5",
    "render3d.smoke.complete outcome=ok"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing QEMU 3D markers: " + ($missing -join " | "))
}

Write-Host "QEMU 3D render log markers verified."
Write-Host "Log: $LogPath"
