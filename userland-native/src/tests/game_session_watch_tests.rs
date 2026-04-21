use super::*;

#[test]
fn native_shell_writes_runtime_bootstrap_files_for_game_session() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest arg=--vsync\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-status\nproc 77 environ\nproc 77 cmdline\nproc 77 cwd\nexit 0\n",
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
    assert!(
        stdout.contains(
            "env-file=/compat/orbit/session.env argv-file=/compat/orbit/session.argv channel-file=/compat/orbit/session.chan loader-file=/compat/orbit/session.loader abi-file=/compat/orbit/session.abi"
        )
    );
    assert!(stdout.contains("NGOS_GAME_TITLE=Orbit Runner"));
    assert!(stdout.contains("NGOS_GFX_BACKEND=vulkan"));
    assert!(stdout.contains("NGOS_COMPAT_PREFIX=/compat/orbit"));
    assert!(stdout.contains("NGOS_COMPAT_ROUTE_CLASS=native-game-runtime"));
    assert!(stdout.contains("NGOS_COMPAT_ABI_ROUTE_CLASS=compat-game-abi"));
    assert!(stdout.contains("NGOS_COMPAT_LAUNCH_MODE=native-direct"));
    assert!(stdout.contains("NGOS_GAME_CHANNEL=/compat/orbit/session.chan"));
    assert!(stdout.contains("NGOS_AUDIO_BACKEND=native-mixer"));
    assert!(stdout.contains("game.session.abi pid=77 route=compat-game-abi handles=win32-game-handles paths=prefix-overlay-paths scheduler=latency-game-scheduler sync=event-heavy-sync timer=frame-budget-timers module=game-module-registry event=game-window-events requires-shims=1"));
    assert!(stdout.contains("game.session.loader pid=77 route=native-game-runtime mode=native-direct entry=native-vulkan-entry bootstrap=bootstrap-light entrypoint=/compat/bin/game-entry requires-shims=0"));
    assert!(stdout.contains("/bin/worker"));
    assert!(stdout.contains("--fullscreen"));
    assert!(stdout.contains("--vsync"));
    assert!(stdout.contains("/games/orbit"));
}

#[test]
fn native_shell_tracks_game_lane_watch_lifecycle() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-sessions\ngame-watch-status-all\ngame-watch-wait 77 audio\nlast-status\ngame-watch-start 77 audio\ngame-watch-status 77 audio\ngame-watch-start 77 input\ngame-watch-status-all\ngame-watch-wait 77 audio\ngame-watch-wait 77 input\ngame-watch-stop 77 audio\ngame-watch-stop 77 input\ngame-watch-status-all\ngame-status\nexit 0\n",
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
    assert!(stdout.contains("game.watch.start pid=77 kind=audio queue="));
    assert!(stdout.contains("game.watch.status pid=77 kind=audio queue="));
    assert!(stdout.contains("game.session.summary pid=77 slug=orbit-runner title=Orbit Runner"));
    assert!(stdout.contains(
        "game.watch.summary pid=77 slug=orbit-runner kind=graphics queue=inactive token=inactive"
    ));
    assert!(stdout.contains("last-status=299"));
    assert!(stdout.contains("queue-event queue="));
    assert!(stdout.contains("game.watch.start pid=77 kind=input queue="));
    assert!(stdout.contains("game.watch.stop pid=77 kind=audio"));
    assert!(stdout.contains("game.watch.stop pid=77 kind=input"));
    assert!(stdout.contains(
        "game.watch.summary pid=77 slug=orbit-runner kind=input queue=inactive token=inactive"
    ));
    assert!(stdout.contains("game.session.watch kind=audio queue=inactive token=inactive"));
    assert!(stdout.contains("game.session.watch kind=input queue=inactive token=inactive"));
}

#[test]
fn native_shell_rejects_duplicate_game_watch_start_without_replacing_active_watch() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-watch-start 77 audio\ngame-watch-status 77 audio\ngame-watch-start 77 audio\nlast-status\ngame-watch-status 77 audio\ngame-watch-stop 77 audio\ngame-watch-status 77 audio\nexit 0\n",
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
    assert!(stdout.contains("game.watch.start pid=77 kind=audio queue="));
    assert!(stdout.contains("last-status=298"));
    assert!(stdout.contains("game.watch.stop pid=77 kind=audio"));
    assert!(stdout.contains("game.watch.status pid=77 kind=audio queue=inactive token=inactive"));
}

#[test]
fn native_shell_rejects_game_watch_start_after_session_stop() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-stop 77\ngame-watch-start 77 audio\nlast-status\ngame-watch-status 77 audio\ngame-status\nexit 0\n",
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
    assert!(stdout.contains("last-status=295"));
    assert!(stdout.contains("game.watch.status pid=77 kind=audio queue=inactive token=inactive"));
    assert!(
        stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner target=game")
    );
    assert!(stdout.contains("stopped=true exit="));
}

#[test]
fn native_shell_closes_watch_queue_fds_on_watch_stop_and_session_stop() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-watch-start 77 audio\ngame-watch-start 77 input\ngame-watch-stop 77 audio\ngame-stop 77\nexit 0\n",
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
    let frames = runtime.backend().frames.borrow();
    let close_count = frames
        .iter()
        .filter(|frame| frame.number == SYS_CLOSE)
        .count();
    assert!(close_count >= 2);
}

#[test]
fn native_shell_tracks_multiple_game_sessions_with_distinct_pids() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /games/comet\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/comet.manifest\nappend-line /games/comet.manifest title=Comet Arena\nappend-line /games/comet.manifest slug=comet-arena\nappend-line /games/comet.manifest exec=/bin/worker\nappend-line /games/comet.manifest cwd=/games/comet\nappend-line /games/comet.manifest arg=--windowed\nappend-line /games/comet.manifest gfx.backend=vulkan\nappend-line /games/comet.manifest gfx.profile=frame-pace\nappend-line /games/comet.manifest audio.backend=native-mixer\nappend-line /games/comet.manifest audio.profile=arena-mix\nappend-line /games/comet.manifest input.backend=native-input\nappend-line /games/comet.manifest input.profile=kbm-first\nappend-line /games/comet.manifest shim.prefix=/compat/comet\nappend-line /games/comet.manifest shim.saves=/saves/comet\nappend-line /games/comet.manifest shim.cache=/cache/comet\ngame-launch /games/orbit.manifest\ngame-launch /games/comet.manifest\ngame-watch-poll-all\nlast-status\ngame-watch-start 77 audio\ngame-watch-start 78 input\ngame-sessions\ngame-watch-status-all\ngame-watch-poll-all\ngame-status\ngame-stop 77\ngame-stop 78\ngame-sessions\ngame-watch-status-all\nexit 0\n",
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
    assert!(
        stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner target=game")
    );
    assert!(stdout.contains("game.session pid=78 title=Comet Arena slug=comet-arena target=game"));
    assert!(stdout.contains("game.watch.start pid=77 kind=audio queue="));
    assert!(stdout.contains("game.watch.start pid=78 kind=input queue="));
    assert!(stdout.contains("last-status=299"));
    assert!(stdout.contains("game.session.summary pid=77 slug=orbit-runner title=Orbit Runner"));
    assert!(stdout.contains("game.session.summary pid=78 slug=comet-arena title=Comet Arena"));
    assert!(stdout.contains("game.watch.summary pid=77 slug=orbit-runner kind=audio queue="));
    assert!(stdout.contains("game.watch.summary pid=78 slug=comet-arena kind=input queue="));
    assert!(stdout.contains("queue-event queue="));
    assert!(stdout.contains("game.watch.event pid=77 slug=orbit-runner kind=audio queue="));
    assert!(stdout.contains("game.watch.event pid=78 slug=comet-arena kind=input queue="));
    assert!(stdout.contains("game.watch.poll count=2"));
    assert!(
        stdout.contains(
            "game.session pid=77 title=Orbit Runner slug=orbit-runner target=game domain="
        )
    );
    assert!(
        stdout
            .contains("game.session pid=78 title=Comet Arena slug=comet-arena target=game domain=")
    );
    assert!(stdout.contains(
        "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=false exit=- lanes=3 watches=1"
    ));
    assert!(stdout.contains(
        "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=false exit=- lanes=3 watches=1"
    ));
    assert!(stdout.contains(
        "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=true exit="
    ));
    assert!(stdout.contains(
        "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=true exit="
    ));
}

#[test]
fn native_shell_preserves_other_game_session_activity_after_partial_stop() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /games/comet\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/comet.manifest\nappend-line /games/comet.manifest title=Comet Arena\nappend-line /games/comet.manifest slug=comet-arena\nappend-line /games/comet.manifest exec=/bin/worker\nappend-line /games/comet.manifest cwd=/games/comet\nappend-line /games/comet.manifest arg=--windowed\nappend-line /games/comet.manifest gfx.backend=vulkan\nappend-line /games/comet.manifest gfx.profile=frame-pace\nappend-line /games/comet.manifest audio.backend=native-mixer\nappend-line /games/comet.manifest audio.profile=arena-mix\nappend-line /games/comet.manifest input.backend=native-input\nappend-line /games/comet.manifest input.profile=kbm-first\nappend-line /games/comet.manifest shim.prefix=/compat/comet\nappend-line /games/comet.manifest shim.saves=/saves/comet\nappend-line /games/comet.manifest shim.cache=/cache/comet\ngame-launch /games/orbit.manifest\ngame-launch /games/comet.manifest\ngame-watch-start 77 audio\ngame-watch-start 78 input\ngame-stop 77\ngame-sessions\ngame-watch-status-all\ngame-watch-poll-all\nlast-status\ngame-status\nexit 0\n",
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
        "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=true exit="
    ));
    assert!(stdout.contains(
        "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=false exit=- lanes=3 watches=1"
    ));
    assert!(stdout.contains(
        "game.watch.summary pid=77 slug=orbit-runner kind=audio queue=inactive token=inactive"
    ));
    assert!(stdout.contains("game.watch.summary pid=78 slug=comet-arena kind=input queue="));
    assert!(stdout.contains("game.watch.event pid=78 slug=comet-arena kind=input queue="));
    assert!(stdout.contains("game.watch.poll count=1"));
    assert!(stdout.contains("last-status=0"));
    assert!(stdout.contains("game.session pid=78 title=Comet Arena slug=comet-arena target=game"));
}

#[test]
fn native_shell_rejects_repeated_game_stop_after_session_is_already_stopped() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-stop 77\ngame-stop 77\nlast-status\ngame-status\nexit 0\n",
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
    assert!(stdout.contains("last-status=295"));
    assert!(
        stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner target=game")
    );
    assert!(stdout.contains("stopped=true exit="));
}
