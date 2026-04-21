use super::*;

#[test]
fn native_shell_runs_game_simulate_agent_and_renders_quality_report() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-simulate /games/orbit 2\nexit 0\n",
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
    assert!(stdout.contains("game-simulate starting slug=orbit-runner frames=2 pid="));
    assert!(stdout.contains("== GAME QUALITY REPORT =="));
    assert!(stdout.contains("title: Orbit Runner"));
    assert!(stdout.contains("frames_submitted: 2"));
    assert!(stdout.contains("quality_score:"));
}

#[test]
fn native_shell_runs_game_gfx_translate_lifecycle() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.api=directx12\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\nmkfile-path /games/orbit.foreign\nappend-line /games/orbit.foreign surface=1280x720\nappend-line /games/orbit.foreign frame=dx12-001\nappend-line /games/orbit.foreign queue=graphics\nappend-line /games/orbit.foreign present-mode=mailbox\nappend-line /games/orbit.foreign completion=fire-and-forget\nappend-line /games/orbit.foreign clear=000000ff\nappend-line /games/orbit.foreign fill-rect=0,0,1280,720,112233ff\nappend-line /games/orbit.foreign draw-sprite=ship,400,200,96,96\nappend-line /games/orbit.foreign present=0,0,1280,720\ngame-gfx-translate 77 /games/orbit.foreign\ngame-gfx-status 77\ngame-gfx-driver-read 77\ngame-gfx-request 77\ngame-gfx-next 77\nexit 0\n",
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
    assert!(stdout.contains("game.gfx.translate pid=77 frame=dx12-001 ops=4 bytes="));
    assert!(stdout.contains(
        "deep-ops=clear,rect,sprite,flip-region api=directx12 translation=compat-to-vulkan"
    ));
    assert!(stdout.contains("api=directx12 translation=compat-to-vulkan"));
    assert!(stdout.contains("queue=graphics present-mode=mailbox"));
    assert!(stdout.contains("gpu-submit device=/dev/gpu0 bytes="));
    assert!(stdout.contains("source-api=directx12 translation=compat-to-vulkan"));
    assert!(stdout.contains("game.gfx.status pid=77"));
    assert!(stdout.contains("api=directx12 backend=vulkan translation=compat-to-vulkan"));
    assert!(stdout.contains("deep-ops=clear,rect,sprite,flip-region"));
    assert!(stdout.contains("claimed=true"));
    assert!(stdout.contains("device-queue="));
    assert!(stdout.contains("driver-queued="));
    assert!(stdout.contains("submitted=1"));
    assert!(stdout.contains("game.gfx.driver-read pid=77 driver=/drv/gpu0 api=directx12 translation=compat-to-vulkan outcome=empty"));
    assert!(stdout.contains("game.gfx.request pid=77 driver=/drv/gpu0 api=directx12 translation=compat-to-vulkan outcome=empty"));
    assert!(stdout.contains(
        "game.gfx.next pid=77 frame=dx12-001 api=directx12 translation=compat-to-vulkan"
    ));
    assert!(
        stdout.contains("deep-ops=clear,rect,sprite,flip-region payload=ngos-gfx-translate/v1")
    );
    assert!(stdout.contains("source-api=directx12"));
    assert!(stdout.contains("translation=compat-to-vulkan"));
}

#[test]
fn native_shell_game_gfx_driver_read_reports_empty_after_queue_is_drained() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.api=directx12\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\nmkfile-path /games/orbit.foreign\nappend-line /games/orbit.foreign surface=1280x720\nappend-line /games/orbit.foreign frame=dx12-001\nappend-line /games/orbit.foreign queue=graphics\nappend-line /games/orbit.foreign present-mode=mailbox\nappend-line /games/orbit.foreign completion=fire-and-forget\nappend-line /games/orbit.foreign clear=000000ff\nappend-line /games/orbit.foreign fill-rect=0,0,1280,720,112233ff\nappend-line /games/orbit.foreign draw-sprite=ship,400,200,96,96\nappend-line /games/orbit.foreign present=0,0,1280,720\ngame-gfx-translate 77 /games/orbit.foreign\ngame-gfx-driver-read 77\ngame-gfx-driver-read 77\nexit 0\n",
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
    assert!(stdout.contains("game.gfx.driver-read pid=77 driver=/drv/gpu0 api=directx12 translation=compat-to-vulkan outcome=empty"));
    assert!(stdout.contains("game.gfx.driver-read pid=77 driver=/drv/gpu0 api=directx12 translation=compat-to-vulkan outcome=empty"));
}

#[test]
fn native_shell_refuses_game_gfx_translate_for_unsupported_api() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/bad\nmkfile-path /games/bad.manifest\nappend-line /games/bad.manifest title=Bad App\nappend-line /games/bad.manifest slug=bad-app\nappend-line /games/bad.manifest exec=/bin/worker\nappend-line /games/bad.manifest cwd=/games/bad\nappend-line /games/bad.manifest gfx.api=other\nappend-line /games/bad.manifest gfx.backend=vulkan\nappend-line /games/bad.manifest gfx.profile=compat\nappend-line /games/bad.manifest audio.backend=native-mixer\nappend-line /games/bad.manifest audio.profile=mono\nappend-line /games/bad.manifest input.backend=native-input\nappend-line /games/bad.manifest input.profile=kbm\nappend-line /games/bad.manifest shim.prefix=/compat/bad\nappend-line /games/bad.manifest shim.saves=/saves/bad\nappend-line /games/bad.manifest shim.cache=/cache/bad\ngame-launch /games/bad.manifest\nmkfile-path /games/bad.foreign\nappend-line /games/bad.foreign surface=640x480\nappend-line /games/bad.foreign frame=bad-001\nappend-line /games/bad.foreign queue=graphics\nappend-line /games/bad.foreign present-mode=fifo\nappend-line /games/bad.foreign completion=fire-and-forget\nappend-line /games/bad.foreign clear=ff0000ff\nappend-line /games/bad.foreign present=0,0,640,480\ngame-gfx-translate 77 /games/bad.foreign\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("game.gfx.translate.refused pid=77 api=other reason=unsupported-api"));
    assert!(stdout.contains("last-status=294"));
}
