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
    "boot.proof=compat-audio",
    "compat.audio.smoke.success request=",
    "stream=qemu-audio-001",
    "api=xaudio2",
    "translation=compat-to-mixer",
    "compat.audio.smoke.refusal request=missing errno=ENOENT outcome=expected",
    "compat.audio.smoke.recovery request=",
    "stream=qemu-audio-002",
    "api=webaudio",
    "translation=native-mixer",
    "compat-audio-smoke-ok"
)

foreach ($marker in $required) {
    if (-not $text.Contains($marker)) {
        throw "Missing compat audio marker: $marker"
    }
}

Write-Host "QEMU compat audio log markers verified."
Write-Host "Log: $LogPath"
