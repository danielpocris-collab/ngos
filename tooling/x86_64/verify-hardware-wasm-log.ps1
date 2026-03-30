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
    "boot.proof=wasm",
    "ngos/x86_64: boot report handled status=0 stage=2 code=0",
    "wasm.smoke.start component=semantic-observer pid=1 artifact=boot-proof",
    "wasm.smoke.refusal component=semantic-observer missing=observe-system-process-count outcome=expected",
    "wasm.smoke.grants component=semantic-observer grants=observe-process-capability-count,observe-system-process-count",
    "wasm.smoke.observe component=semantic-observer pid=1 capabilities=",
    "processes=",
    "wasm.smoke.recovery component=semantic-observer refusal=observe-system-process-count recovered=yes verdict=",
    "wasm.smoke.result component=semantic-observer verdict=",
    "wasm.smoke.start component=process-identity pid=1 artifact=process-identity",
    "wasm.smoke.refusal component=process-identity missing=observe-process-cwd-root outcome=expected",
    "wasm.smoke.grants component=process-identity grants=observe-process-status-bytes,observe-process-cwd-root",
    "wasm.smoke.observe component=process-identity pid=1 status-bytes=",
    "cwd-root=",
    "wasm.smoke.recovery component=process-identity refusal=observe-process-cwd-root recovered=yes verdict=",
    "wasm.smoke.result component=process-identity verdict=",
    "wasm-smoke-ok"
)

$missing = @()
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        $missing += $marker
    }
}

if ($missing.Count -ne 0) {
    throw ("Missing hardware WASM markers: " + ($missing -join " | "))
}

Write-Host "Hardware WASM log markers verified."
Write-Host "Log: $LogPath"
