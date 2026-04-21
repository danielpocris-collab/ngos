use super::*;

#[test]
fn native_shell_translates_and_submits_game_audio_mix() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.mix\nappend-line /games/orbit.mix rate=48000\nappend-line /games/orbit.mix channels=2\nappend-line /games/orbit.mix stream=orbit-intro\nappend-line /games/orbit.mix route=music\nappend-line /games/orbit.mix latency-mode=interactive\nappend-line /games/orbit.mix spatialization=world-3d\nappend-line /games/orbit.mix completion=wait-drain\nappend-line /games/orbit.mix tone=lead,440,120,0.800,-0.250,sine\nappend-line /games/orbit.mix clip=ambience,hangar-loop,2,0.650,0.100\ngame-launch /games/orbit.manifest\ngame-audio-plan 77 /games/orbit.mix\ngame-audio-submit 77 /games/orbit.mix\ngame-status\ngame-audio-status 77\ngame-audio-next 77\ngame-audio-next 77\nlast-status\ngame-stop 77\ngame-audio-submit 77 /games/orbit.mix\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("game.audio.plan pid=77 stream=orbit-intro ops=2"));
    assert!(stdout.contains("route=music latency-mode=interactive spatialization=world-3d"));
    assert!(stdout.contains("completion=wait-drain"));
    assert!(stdout.contains("game.audio.submit pid=77 stream=orbit-intro ops=2"));
    assert!(stdout.contains("batches=1 token="));
    assert!(stdout.contains("completion-observed=resource-drained"));
    assert!(stdout.contains("queue-event queue="));
    assert!(stdout.contains(
        "game.audio.status pid=77 device=/dev/audio0 driver=/drv/audio0 profile=spatial-mix claimed="
    ));
    assert!(stdout.contains(" token="));
    assert!(stdout.contains(
        "stream=orbit-intro route=music latency-mode=interactive spatialization=world-3d completion=wait-drain completion-observed=resource-drained ops=2 bytes="
    ));
    assert!(stdout.contains("device-queue="));
    assert!(stdout.contains("device-submitted="));
    assert!(stdout.contains("device-completed="));
    assert!(stdout.contains("driver-queued="));
    assert!(stdout.contains("driver-inflight="));
    assert!(stdout.contains("driver-completed="));
    assert!(stdout.contains("game.session.audio-queue pid=77 depth=1"));
    assert!(stdout.contains("game.audio.next pid=77 stream=orbit-intro route=music latency-mode=interactive spatialization=world-3d completion=wait-drain remaining=0 payload=ngos-audio-translate/v1"));
    assert!(stdout.contains("game.audio.queue pid=77 depth=0"));
    assert!(stdout.contains("last-status=295"));
}

#[test]
fn native_shell_translates_and_submits_game_input_batch() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.api=directx12\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.input\nappend-line /games/orbit.input device=gamepad\nappend-line /games/orbit.input family=dualshock\nappend-line /games/orbit.input frame=input-001\nappend-line /games/orbit.input layout=gamepad-standard\nappend-line /games/orbit.input key-table=us-game\nappend-line /games/orbit.input pointer-capture=relative-lock\nappend-line /games/orbit.input delivery=wait-frame\nappend-line /games/orbit.input button=cross,press\nappend-line /games/orbit.input axis=left-x,750\nappend-line /games/orbit.input pointer=4,-2\ngame-launch /games/orbit.manifest\ngame-input-plan 77 /games/orbit.input\ngame-input-submit 77 /games/orbit.input\ngame-status\ngame-input-status 77\ngame-input-next 77\ngame-input-next 77\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("game.session.input-queue pid=77 depth=0"));
    assert!(stdout.contains(
        "game.input.status pid=77 device=/dev/input0 driver=/drv/input0 profile=gamepad-first claimed="
    ));
    assert!(stdout.contains("frame=input-001 family=dualshock layout=gamepad-standard"));
    assert!(stdout.contains("pointer-capture=relative-lock delivery=wait-frame"));
    assert!(stdout.contains("ops=3"));
    assert!(stdout.contains("device-queue=1/64 device-submitted=1 device-completed=0"));
    assert!(stdout.contains("driver-queued=1 driver-inflight=1 driver-completed=0"));
    assert!(stdout.contains("game.session.input-queue pid=77 depth=0"));
    assert!(stdout.contains(
        "game.input.next pid=77 frame=input-001 family=dualshock layout=gamepad-standard delivery=wait-frame remaining=0 payload=ngos-input-translate/v1"
    ));
    assert!(stdout.contains("game.input.queue pid=77 depth=0"));
    assert!(stdout.contains("last-status=299"));
}

#[test]
fn native_shell_consumes_unified_game_payload_queue() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.frame\nappend-line /games/orbit.frame surface=1280x720\nappend-line /games/orbit.frame frame=orbit-001\nappend-line /games/orbit.frame queue=graphics\nappend-line /games/orbit.frame present-mode=mailbox\nappend-line /games/orbit.frame completion=wait-complete\nappend-line /games/orbit.frame clear=#112233\nappend-line /games/orbit.frame rect=10,20,200,100,#ff8800ff\nmkfile-path /games/orbit.mix\nappend-line /games/orbit.mix rate=48000\nappend-line /games/orbit.mix channels=2\nappend-line /games/orbit.mix stream=orbit-intro\nappend-line /games/orbit.mix route=music\nappend-line /games/orbit.mix latency-mode=interactive\nappend-line /games/orbit.mix spatialization=world-3d\nappend-line /games/orbit.mix completion=wait-drain\nappend-line /games/orbit.mix tone=lead,440,120,0.800,-0.250,sine\nmkfile-path /games/orbit.input\nappend-line /games/orbit.input device=gamepad\nappend-line /games/orbit.input family=dualshock\nappend-line /games/orbit.input frame=input-001\nappend-line /games/orbit.input layout=gamepad-standard\nappend-line /games/orbit.input key-table=us-game\nappend-line /games/orbit.input pointer-capture=relative-lock\nappend-line /games/orbit.input delivery=wait-frame\nappend-line /games/orbit.input button=cross,press\ngame-launch /games/orbit.manifest\ngame-gfx-submit 77 /games/orbit.frame\ngame-audio-submit 77 /games/orbit.mix\ngame-input-submit 77 /games/orbit.input\ngame-next 77\ngame-next 77\ngame-next 77\ngame-next 77\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("game.next pid=77 kind=graphics tag=orbit-001 remaining[gfx=0;audio=1;input=1] deep-ops=clear,rect payload=ngos-gfx-translate/v1"));
    assert!(stdout.contains("game.next pid=77 kind=audio tag=orbit-intro remaining[gfx=0;audio=0;input=1] payload=ngos-audio-translate/v1"));
    assert!(stdout.contains("game.next pid=77 kind=input tag=input-001 remaining[gfx=0;audio=0;input=0] payload=ngos-input-translate/v1"));
    assert!(stdout.contains("game.next pid=77 depth[gfx=0;audio=0;input=0]"));
    assert!(stdout.contains("last-status=299"));
}
