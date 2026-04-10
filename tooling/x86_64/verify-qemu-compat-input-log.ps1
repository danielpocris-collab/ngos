param(
    [Parameter(Mandatory = $true)]
    [string] $LogPath
)

$markers = @(
    "boot.proof=compat-input",
    "compat.input.smoke.success request=",
    "frame=qemu-input-001",
    "api=xinput",
    "translation=compat-to-input",
    "compat.input.smoke.refusal request=missing errno=ENOENT outcome=expected",
    "compat.input.smoke.recovery request=",
    "frame=qemu-input-002",
    "api=evdev",
    "translation=native-input",
    "compat-input-smoke-ok"
)

$content = Get-Content -Path $LogPath -Raw
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        throw "Missing QEMU compat input marker: $marker"
    }
}

Write-Host "QEMU compat input log markers verified."
Write-Host "Log: $LogPath"
