use super::*;

#[test]
fn native_shell_runs_game_compat_launch_lifecycle() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.api=directx12\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-manifest /games/orbit.manifest\ngame-plan /games/orbit.manifest\ngame-launch /games/orbit.manifest\ngame-status\ngame-stop 77\ngame-status\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("game.manifest path=/games/orbit.manifest target=game title=Orbit Runner slug=orbit-runner exec=/bin/worker cwd=/games/orbit argv=--fullscreen"));
    assert!(stdout.contains(
        "game.gfx backend=vulkan profile=frame-pace api=directx12 translation=compat-to-vulkan"
    ));
    assert!(stdout.contains("game.audio backend=native-mixer profile=spatial-mix"));
    assert!(stdout.contains("game.input backend=native-input profile=gamepad-first"));
    assert!(stdout.contains("game.plan domain=compat-game target=game process=compat-orbit-runner cwd=/games/orbit exec=/bin/worker"));
    assert!(stdout.contains("game.plan.loader route=compat-game-runtime mode=compat-shim entry=dx-to-vulkan-entry bootstrap=bootstrap-light entrypoint=/compat/bin/game-entry requires-shims=1"));
    assert!(stdout.contains("game.plan.abi route=compat-game-abi handles=win32-game-handles paths=prefix-overlay-paths scheduler=latency-game-scheduler sync=event-heavy-sync timer=frame-budget-timers module=game-module-registry event=game-window-events requires-shims=1"));
    assert!(stdout.contains(
        "game.plan.lane kind=graphics resource=orbit-runner-gfx contract=frame-pace-display"
    ));
    assert!(stdout.contains(
        "game.plan.lane kind=audio resource=orbit-runner-audio contract=spatial-mix-mix"
    ));
    assert!(stdout.contains(
        "game.plan.lane kind=input resource=orbit-runner-input contract=gamepad-first-capture"
    ));
    assert!(stdout.contains("game.plan.env NGOS_COMPAT_PREFIX=/compat/orbit"));
    assert!(stdout.contains("game.plan.env NGOS_COMPAT_ROUTE_CLASS=compat-game-runtime"));
    assert!(stdout.contains("game.plan.env NGOS_COMPAT_LAUNCH_MODE=compat-shim"));
    assert!(
        stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner target=game")
    );
    assert!(
        stdout.contains(
            "game.session.shim prefix=/compat/orbit saves=/saves/orbit cache=/cache/orbit"
        )
    );
    assert!(
        stdout.contains(
            "loader-file=/compat/orbit/session.loader abi-file=/compat/orbit/session.abi"
        )
    );
    assert!(stdout.contains("game.session.abi pid=77 route=compat-game-abi handles=win32-game-handles paths=prefix-overlay-paths scheduler=latency-game-scheduler sync=event-heavy-sync timer=frame-budget-timers module=game-module-registry event=game-window-events requires-shims=1"));
    assert!(stdout.contains("game.session.loader pid=77 route=compat-game-runtime mode=compat-shim entry=dx-to-vulkan-entry bootstrap=bootstrap-light entrypoint=/compat/bin/game-entry requires-shims=1"));
    assert!(stdout.contains("game.session.lane kind=graphics"));
    assert!(stdout.contains("game.session.lane kind=audio"));
    assert!(stdout.contains("game.session.lane kind=input"));
    assert!(
        stdout.contains(
            "game.session pid=77 title=Orbit Runner slug=orbit-runner target=game domain="
        )
    );
    assert!(stdout.contains("stopped=true exit="));
}

#[test]
fn native_shell_reports_graphics_api_translation_for_directx_and_opengl_and_metal() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/dx\nmkfile-path /games/dx.manifest\nappend-line /games/dx.manifest title=Dx Runner\nappend-line /games/dx.manifest slug=dx-runner\nappend-line /games/dx.manifest exec=/bin/worker\nappend-line /games/dx.manifest cwd=/games/dx\nappend-line /games/dx.manifest gfx.api=directx12\nappend-line /games/dx.manifest gfx.backend=vulkan\nappend-line /games/dx.manifest gfx.profile=frame-pace\nappend-line /games/dx.manifest audio.backend=native-mixer\nappend-line /games/dx.manifest audio.profile=spatial-mix\nappend-line /games/dx.manifest input.backend=native-input\nappend-line /games/dx.manifest input.profile=gamepad-first\nappend-line /games/dx.manifest shim.prefix=/compat/dx\nappend-line /games/dx.manifest shim.saves=/saves/dx\nappend-line /games/dx.manifest shim.cache=/cache/dx\ngame-manifest /games/dx.manifest\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains(
        "game.gfx backend=vulkan profile=frame-pace api=directx12 translation=compat-to-vulkan"
    ));
}

#[test]
fn native_shell_reports_graphics_translation_plan_from_manifest() {
    let manifest = GameCompatManifest::parse(
        "title=Metal Runner\nslug=metal-runner\nexec=/games/metal/run\ncwd=/games/metal\ngfx.api=metal\ngfx.backend=vulkan\ngfx.profile=frame-pace\naudio.backend=native-mixer\naudio.profile=spatial-mix\ninput.backend=native-input\ninput.profile=gamepad-first\nshim.prefix=/compat/metal\nshim.saves=/saves/metal\nshim.cache=/cache/metal\n",
    )
    .unwrap();
    let plan = manifest.graphics_translation_plan();

    assert_eq!(plan.source_api_name, "metal");
    assert_eq!(plan.backend_name, "vulkan");
    assert_eq!(plan.translation, "compat-to-vulkan");
}

#[test]
fn native_shell_translates_and_submits_game_graphics_frame() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.frame\nappend-line /games/orbit.frame surface=1280x720\nappend-line /games/orbit.frame frame=orbit-001\nappend-line /games/orbit.frame queue=graphics\nappend-line /games/orbit.frame present-mode=mailbox\nappend-line /games/orbit.frame completion=wait-complete\nappend-line /games/orbit.frame clear=#112233\nappend-line /games/orbit.frame line=0,0,1279,719,#44ccffff\nappend-line /games/orbit.frame rect=10,20,200,100,#ff8800ff\nappend-line /games/orbit.frame sprite=ship-main,400,220,96,96\nappend-line /games/orbit.frame blit=hud-overlay,0,0,1280,64\ngame-launch /games/orbit.manifest\ngame-gfx-plan 77 /games/orbit.frame\ngame-gfx-submit 77 /games/orbit.frame\ncat-file /compat/orbit/session.chan\ngame-status\ngame-gfx-status 77\ngame-gfx-next 77\ngame-gfx-next 77\nlast-status\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("game.gfx.plan pid=77 frame=orbit-001 ops=5"));
    assert!(stdout.contains("queue=graphics present-mode=mailbox completion=wait-complete"));
    assert!(stdout.contains("gpu-submit device=/dev/gpu0 bytes="));
    assert!(stdout.contains("source-api=- translation=-"));
    assert!(stdout.contains("queue-event queue="));
    assert!(stdout.contains("kind=graphics tag=orbit-001"));
    assert!(stdout.contains("game.gfx.submit pid=77 frame=orbit-001 ops=5"));
    assert!(stdout.contains(
        "game.gfx.status pid=77 device=/dev/gpu0 driver=/drv/gpu0 api=vulkan backend=vulkan translation=native-vulkan profile=frame-pace claimed=true submitted=1"
    ));
    assert!(stdout.contains(
        "last-frame=orbit-001 queue=graphics present-mode=mailbox completion=wait-complete completion-observed=graphics-event-complete deep-ops=clear,line,rect,sprite,blit ops=5"
    ));
    assert!(stdout.contains("device-queue="));
    assert!(stdout.contains("driver-queued="));
    assert!(stdout.contains("game.session.gfx-queue pid=77 depth=1"));
    assert!(stdout.contains("game.gfx.next pid=77 frame=orbit-001"));
    assert!(stdout.contains(
        "queue=graphics present-mode=mailbox completion=wait-complete remaining=0 deep-ops=clear,line,rect,sprite,blit payload=ngos-gfx-translate/v1"
    ));
    assert!(stdout.contains("game.gfx.queue pid=77 depth=0"));
    assert!(stdout.contains("last-status=299"));
}

#[test]
fn native_shell_rejects_stopped_game_graphics_submit_and_clears_pending_payloads() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.frame\nappend-line /games/orbit.frame surface=1280x720\nappend-line /games/orbit.frame frame=orbit-001\nappend-line /games/orbit.frame queue=graphics\nappend-line /games/orbit.frame present-mode=mailbox\nappend-line /games/orbit.frame completion=wait-complete\nappend-line /games/orbit.frame clear=#112233\nappend-line /games/orbit.frame rect=10,20,200,100,#ff8800ff\ngame-launch /games/orbit.manifest\ngame-gfx-submit 77 /games/orbit.frame\ngame-stop 77\ngame-gfx-submit 77 /games/orbit.frame\ngame-next 77\ngame-status\nlast-status\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("game.gfx.submit pid=77 frame=orbit-001 ops=2"));
    assert!(stdout.contains("game.next pid=77 depth[gfx=0;audio=0;input=0]"));
    assert!(stdout.contains("game.session.gfx-queue pid=77 depth=0"));
    assert!(stdout.contains("game.session.audio-queue pid=77 depth=0"));
    assert!(stdout.contains("game.session.input-queue pid=77 depth=0"));
    assert!(stdout.contains("stopped=true exit="));
    assert!(stdout.contains("last-status=0"));
}
