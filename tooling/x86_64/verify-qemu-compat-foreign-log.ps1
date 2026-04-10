param(
    [Parameter(Mandatory = $true)]
    [string] $LogPath
)

$markers = @(
    "boot.proof=compat-foreign",
    "compat.loader.smoke.plan slug=nova-strike api=directx11",
    "compat.loader.smoke.success pid=",
    "compat.loader.smoke.refusal path=/games/bad.manifest",
    "compat.loader.smoke.relaunch.stopped pid=",
    "compat.loader.smoke.recovery pid=",
    "compat.loader.smoke.matrix pid=",
    "compat.loader.smoke.cleanup pid=",
    "compat-loader-smoke-ok",
    "compat.abi.smoke.handle.success id=1 dup=2 kind=domain object-id=1001 open=2",
    "compat.abi.smoke.path.success unix=/compat/root/games/nova/config.toml relative=/compat/root/profiles/player-one.cfg",
    "compat.abi.smoke.sched.success win32=latency-critical posix=best-effort",
    "compat.abi.smoke.sync.success mutex-id=1 state=locked owner=1000 event-id=1 event-state=signaled",
    "compat.abi.smoke.timer.success oneshot-id=1 oneshot-fires=1 oneshot-state=idle periodic-id=2 periodic-fires=2 periodic-due=150 periodic-state=armed",
    "compat.abi.smoke.module.success id=1 name=nova.renderer path=/compat/root/modules/nova-renderer.ngm base=0x400000 size=0x20000 state=loaded retain=2 release=1",
    "compat.abi.smoke.proc.success pid=",
    "fd-count=",
    "fd0=present fd1=present fd2=present",
    "cmdline=present",
    "compat.abi.smoke.proc.environ pid=",
    "outcome=ok marker=NGOS_COMPAT_TARGET=game",
    "compat.abi.smoke.proc.refusal pid=",
    "/fd/9999 outcome=expected",
    "compat.abi.smoke.proc.recovery pid=",
    "fd-list=ok outcome=ok",
    "compat.abi.smoke.recovery handles-open=0 mutex-state=unlocked event-state=unsignaled path=/compat/root/restored/session.ok sched=background timer-state=idle timer-fires=2 module-id=2 module-name=nova.runtime module-state=loaded module-ref-count=1 outcome=ok",
    "compat.abi.smoke.route pid=",
    "target=game route=compat-game-abi handles=win32-game-handles",
    "target=app route=compat-app-abi handles=win32-app-handles",
    "target=tool route=compat-tool-abi handles=utility-handles",
    "target=other route=compat-other-abi handles=service-handles",
    "compat.abi.smoke.cleanup running=0 stopped=4 outcome=ok",
    "compat-abi-smoke-ok",
    "compat-foreign-smoke-ok"
)

$content = Get-Content -Path $LogPath -Raw
foreach ($marker in $markers) {
    if (-not $content.Contains($marker)) {
        throw "Missing QEMU compat foreign marker: $marker"
    }
}

Write-Host "QEMU compat foreign log markers verified."
Write-Host "Log: $LogPath"
