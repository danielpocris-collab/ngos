param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$content = Get-Content -Path $LogPath -Raw
$markers = @(
    "boot.proof=compat-loader",
    "compat.loader.smoke.plan slug=nova-strike api=directx11",
    "preloads=2",
    "dll-overrides=2",
    "env-overrides=2",
    "translation=compat-to-vulkan",
    "compat.loader.smoke.success pid=",
    "route=compat-game-runtime",
    "mode=compat-shim",
    "entry=dx-to-vulkan-entry",
    "bootstrap=shim-heavy",
    "entrypoint=/compat/bin/game-entry",
    "requires-shims=1",
    "slug=nova-strike",
    "preloads=/compat/nova/preload/d3d11.ngm;/compat/nova/preload/xaudio2.ngm",
    "dll-overrides=d3d11=builtin;xaudio2=native",
    "env-overrides=DXVK_HUD=1;WINEDEBUG=-all",
    "compat.loader.smoke.refusal path=/games/bad.manifest",
    "reason=loader-overrides-invalid",
    "outcome=expected",
    "compat.loader.smoke.relaunch.stopped pid=",
    "compat.loader.smoke.recovery pid=",
    "api=vulkan",
    "translation=native-vulkan",
    "route=native-app-runtime",
    "mode=native-direct",
    "entry=native-vulkan-entry",
    "bootstrap=env-overlay",
    "entrypoint=/compat/bin/app-entry",
    "requires-shims=0",
    "env-overrides=NGOS_COMPAT_RECOVERY=1",
    "running=1 stopped=1",
    "compat.loader.smoke.matrix pid=",
    "target=tool",
    "slug=nova-tool",
    "api=webgpu",
    "route=compat-tool-runtime",
    "mode=compat-shim",
    "entry=webgpu-to-vulkan-entry",
    "bootstrap=bootstrap-light",
    "entrypoint=/compat/bin/tool-entry",
    "requires-shims=1",
    "preloads=0",
    "dll-overrides=0",
    "env-overrides=0",
    "target=other",
    "slug=nova-service",
    "route=native-other-runtime",
    "mode=native-direct",
    "entry=native-vulkan-entry",
    "bootstrap=shim-heavy",
    "entrypoint=/compat/bin/other-entry",
    "preloads=1",
    "dll-overrides=1",
    "compat.loader.smoke.cleanup pid=",
    "running=0 stopped=4",
    "compat-loader-smoke-ok"
)

foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        throw "Missing loader compat log marker: $marker"
    }
}

Write-Host "QEMU compat loader log markers verified."
Write-Host "Log: $LogPath"
