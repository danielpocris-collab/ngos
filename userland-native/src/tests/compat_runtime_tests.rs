use super::*;

#[test]
fn native_shell_compat_loader_session_profile() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/nova\nmkfile-path /games/nova.manifest\nappend-line /games/nova.manifest title=Nova Strike\nappend-line /games/nova.manifest slug=nova-strike\nappend-line /games/nova.manifest exec=/bin/worker\nappend-line /games/nova.manifest cwd=/games/nova\nappend-line /games/nova.manifest arg=--fullscreen\nappend-line /games/nova.manifest arg=--vsync\nappend-line /games/nova.manifest gfx.api=directx11\nappend-line /games/nova.manifest gfx.backend=vulkan\nappend-line /games/nova.manifest gfx.profile=latency-opt\nappend-line /games/nova.manifest audio.backend=native-mixer\nappend-line /games/nova.manifest audio.profile=stereo-hifi\nappend-line /games/nova.manifest input.backend=native-input\nappend-line /games/nova.manifest input.profile=kbm-first\nappend-line /games/nova.manifest shim.prefix=/compat/nova\nappend-line /games/nova.manifest shim.saves=/saves/nova\nappend-line /games/nova.manifest shim.cache=/cache/nova\ngame-launch /games/nova.manifest\ngame-session-profile 77\ngame-abi-status 77\ngame-loader-status\nexit 0\n",
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
    assert!(stdout.contains("game.launched pid=77 title=Nova Strike slug=nova-strike"));
    assert!(stdout.contains("game.session.profile pid=77 slug=nova-strike state=running"));
    assert!(stdout.contains("game.session.profile.gfx pid=77 api=directx11 profile=latency-opt"));
    assert!(stdout.contains("translation=compat-to-vulkan"));
    assert!(stdout.contains("game.session.profile.audio pid=77 profile=stereo-hifi"));
    assert!(stdout.contains("game.session.profile.input pid=77 profile=kbm-first"));
    assert!(stdout.contains("game.session.profile.paths pid=77 cwd=/games/nova"));
    assert!(stdout.contains("prefix=/compat/nova saves=/saves/nova cache=/cache/nova"));
    assert!(stdout.contains("game.session.profile.abi pid=77 route=compat-game-abi"));
    assert!(stdout.contains("game.abi.status pid=77 target=game route=compat-game-abi"));
    assert!(stdout.contains("game.session.profile.loader pid=77 route=compat-game-runtime"));
    assert!(stdout.contains("game.loader.status sessions=1 running=1 stopped=0"));
    assert!(stdout.contains("game.loader.session pid=77 slug=nova-strike state=running"));
    assert!(stdout.contains("preloads=0 dll-overrides=0 env-overrides=0"));
}

#[test]
fn native_shell_compat_loader_relaunch() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/nova\nmkfile-path /games/nova.manifest\nappend-line /games/nova.manifest title=Nova Strike\nappend-line /games/nova.manifest slug=nova-strike\nappend-line /games/nova.manifest exec=/bin/worker\nappend-line /games/nova.manifest cwd=/games/nova\nappend-line /games/nova.manifest gfx.api=directx11\nappend-line /games/nova.manifest gfx.backend=vulkan\nappend-line /games/nova.manifest gfx.profile=latency-opt\nappend-line /games/nova.manifest audio.backend=native-mixer\nappend-line /games/nova.manifest audio.profile=stereo-hifi\nappend-line /games/nova.manifest input.backend=native-input\nappend-line /games/nova.manifest input.profile=kbm-first\nappend-line /games/nova.manifest shim.prefix=/compat/nova\nappend-line /games/nova.manifest shim.saves=/saves/nova\nappend-line /games/nova.manifest shim.cache=/cache/nova\ngame-launch /games/nova.manifest\ngame-loader-status\ngame-relaunch /games/nova.manifest\ngame-loader-status\nexit 0\n",
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
    assert!(stdout.contains("game.launched pid=77 title=Nova Strike slug=nova-strike"));
    assert!(stdout.contains("game.loader.status sessions=1 running=1 stopped=0"));
    assert!(stdout.contains("route=compat-game-runtime mode=compat-shim entry=dx-to-vulkan-entry bootstrap=bootstrap-light"));
    assert!(stdout.contains("preloads=0 dll-overrides=0 env-overrides=0"));
    assert!(stdout.contains("game.relaunch.stopped pid=77 slug=nova-strike"));
    assert!(stdout.contains("game.relaunched"));
    assert!(stdout.contains("slug=nova-strike"));
    assert!(stdout.contains("game.loader.status sessions=2 running=1 stopped=1"));
}

#[test]
fn native_shell_compat_loader_invalid_manifest_refused() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkfile-path /games/bad.manifest\nappend-line /games/bad.manifest title=Bad\nappend-line /games/bad.manifest slug=bad\nappend-line /games/bad.manifest exec=bad-no-slash\nappend-line /games/bad.manifest cwd=/games\nappend-line /games/bad.manifest gfx.backend=vulkan\nappend-line /games/bad.manifest gfx.profile=compat\nappend-line /games/bad.manifest audio.backend=native-mixer\nappend-line /games/bad.manifest audio.profile=mono\nappend-line /games/bad.manifest input.backend=native-input\nappend-line /games/bad.manifest input.profile=kbm\nappend-line /games/bad.manifest shim.prefix=/compat/bad\nappend-line /games/bad.manifest shim.saves=/saves/bad\nappend-line /games/bad.manifest shim.cache=/cache/bad\ngame-launch /games/bad.manifest\nlast-status\ngame-loader-status\nexit 0\n",
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
    assert!(stdout.contains("last-status="));
    assert!(stdout.contains("game.loader.status sessions=0"));
}

#[test]
fn native_shell_compat_loader_exit_state_after_stop() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/nova\nmkfile-path /games/nova.manifest\nappend-line /games/nova.manifest title=Nova Strike\nappend-line /games/nova.manifest slug=nova-strike\nappend-line /games/nova.manifest exec=/bin/worker\nappend-line /games/nova.manifest cwd=/games/nova\nappend-line /games/nova.manifest gfx.api=directx11\nappend-line /games/nova.manifest gfx.backend=vulkan\nappend-line /games/nova.manifest gfx.profile=latency-opt\nappend-line /games/nova.manifest audio.backend=native-mixer\nappend-line /games/nova.manifest audio.profile=stereo-hifi\nappend-line /games/nova.manifest input.backend=native-input\nappend-line /games/nova.manifest input.profile=kbm-first\nappend-line /games/nova.manifest shim.prefix=/compat/nova\nappend-line /games/nova.manifest shim.saves=/saves/nova\nappend-line /games/nova.manifest shim.cache=/cache/nova\ngame-launch /games/nova.manifest\ngame-stop 77\ngame-session-profile 77\ngame-loader-status\nexit 0\n",
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
    assert!(stdout.contains("game.session.profile pid=77 slug=nova-strike state=stopped"));
    assert!(stdout.contains("exit-code="));
    assert!(stdout.contains("game.loader.status sessions=1 running=0 stopped=1"));
    assert!(stdout.contains("game.loader.session pid=77 slug=nova-strike state=stopped"));
}

#[test]
fn native_shell_compat_audio_translate_xaudio2_lifecycle() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/nova\nmkfile-path /games/nova.manifest\nappend-line /games/nova.manifest title=Nova Strike\nappend-line /games/nova.manifest slug=nova-strike\nappend-line /games/nova.manifest exec=/bin/worker\nappend-line /games/nova.manifest cwd=/games/nova\nappend-line /games/nova.manifest gfx.backend=vulkan\nappend-line /games/nova.manifest gfx.profile=latency-opt\nappend-line /games/nova.manifest audio.backend=native-mixer\nappend-line /games/nova.manifest audio.profile=stereo-hifi\nappend-line /games/nova.manifest input.backend=native-input\nappend-line /games/nova.manifest input.profile=kbm-first\nappend-line /games/nova.manifest shim.prefix=/compat/nova\nappend-line /games/nova.manifest shim.saves=/saves/nova\nappend-line /games/nova.manifest shim.cache=/cache/nova\nmkfile-path /games/nova.mix\nappend-line /games/nova.mix rate=48000\nappend-line /games/nova.mix channels=2\nappend-line /games/nova.mix stream=nova-intro\nappend-line /games/nova.mix route=music\nappend-line /games/nova.mix latency-mode=interactive\nappend-line /games/nova.mix spatialization=stereo\nappend-line /games/nova.mix completion=fire-and-forget\nappend-line /games/nova.mix tone=main,440,120,0.800,-0.250,sine\ngame-launch /games/nova.manifest\ngame-audio-translate 77 xaudio2 /games/nova.mix\ngame-audio-status 77\nexit 0\n",
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
    assert!(stdout.contains("game.audio.translate pid=77 api=xaudio2 translation=compat-to-mixer"));
    assert!(stdout.contains("stream=nova-intro ops=1 submitted=1"));
    assert!(stdout.contains("game.audio.status pid=77"));
    assert!(stdout.contains("submitted=1"));
}

#[test]
fn native_shell_compat_audio_translate_refuses_other_api() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/nova\nmkfile-path /games/nova.manifest\nappend-line /games/nova.manifest title=Nova Strike\nappend-line /games/nova.manifest slug=nova-strike\nappend-line /games/nova.manifest exec=/bin/worker\nappend-line /games/nova.manifest cwd=/games/nova\nappend-line /games/nova.manifest gfx.backend=vulkan\nappend-line /games/nova.manifest gfx.profile=latency-opt\nappend-line /games/nova.manifest audio.backend=native-mixer\nappend-line /games/nova.manifest audio.profile=stereo-hifi\nappend-line /games/nova.manifest input.backend=native-input\nappend-line /games/nova.manifest input.profile=kbm-first\nappend-line /games/nova.manifest shim.prefix=/compat/nova\nappend-line /games/nova.manifest shim.saves=/saves/nova\nappend-line /games/nova.manifest shim.cache=/cache/nova\nmkfile-path /games/nova.mix\nappend-line /games/nova.mix rate=48000\nappend-line /games/nova.mix channels=2\nappend-line /games/nova.mix stream=nova-intro\nappend-line /games/nova.mix route=music\nappend-line /games/nova.mix latency-mode=interactive\nappend-line /games/nova.mix spatialization=stereo\nappend-line /games/nova.mix completion=fire-and-forget\nappend-line /games/nova.mix tone=main,440,120,0.800,-0.250,sine\ngame-launch /games/nova.manifest\ngame-audio-translate 77 other /games/nova.mix\nlast-status\nexit 0\n",
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
        "game.audio.translate.refused pid=77 api=other reason=unsupported audio api=other"
    ));
    assert!(stdout.contains("last-status=296"));
}

#[test]
fn native_shell_compat_input_translate_xinput_lifecycle() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/nova\nmkfile-path /games/nova.manifest\nappend-line /games/nova.manifest title=Nova Strike\nappend-line /games/nova.manifest slug=nova-strike\nappend-line /games/nova.manifest exec=/bin/worker\nappend-line /games/nova.manifest cwd=/games/nova\nappend-line /games/nova.manifest gfx.backend=vulkan\nappend-line /games/nova.manifest gfx.profile=latency-opt\nappend-line /games/nova.manifest audio.backend=native-mixer\nappend-line /games/nova.manifest audio.profile=stereo-hifi\nappend-line /games/nova.manifest input.backend=native-input\nappend-line /games/nova.manifest input.profile=kbm-first\nappend-line /games/nova.manifest shim.prefix=/compat/nova\nappend-line /games/nova.manifest shim.saves=/saves/nova\nappend-line /games/nova.manifest shim.cache=/cache/nova\nmkfile-path /games/nova.input\nappend-line /games/nova.input device=gamepad\nappend-line /games/nova.input family=xbox\nappend-line /games/nova.input frame=input-001\nappend-line /games/nova.input layout=gamepad-standard\nappend-line /games/nova.input key-table=us-game\nappend-line /games/nova.input pointer-capture=none\nappend-line /games/nova.input delivery=immediate\nappend-line /games/nova.input button=a,press\nappend-line /games/nova.input axis=left-x,750\ngame-launch /games/nova.manifest\ngame-input-translate 77 xinput /games/nova.input\ngame-input-status 77\nexit 0\n",
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
    assert!(stdout.contains("game.input.translate pid=77 api=xinput translation=compat-to-input"));
    assert!(stdout.contains("frame=input-001 ops=2 submitted=1"));
    assert!(stdout.contains("game.input.status pid=77"));
    assert!(stdout.contains("submitted=1"));
}

#[test]
fn native_shell_compat_input_translate_refuses_other_api() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/nova\nmkfile-path /games/nova.manifest\nappend-line /games/nova.manifest title=Nova Strike\nappend-line /games/nova.manifest slug=nova-strike\nappend-line /games/nova.manifest exec=/bin/worker\nappend-line /games/nova.manifest cwd=/games/nova\nappend-line /games/nova.manifest gfx.backend=vulkan\nappend-line /games/nova.manifest gfx.profile=latency-opt\nappend-line /games/nova.manifest audio.backend=native-mixer\nappend-line /games/nova.manifest audio.profile=stereo-hifi\nappend-line /games/nova.manifest input.backend=native-input\nappend-line /games/nova.manifest input.profile=kbm-first\nappend-line /games/nova.manifest shim.prefix=/compat/nova\nappend-line /games/nova.manifest shim.saves=/saves/nova\nappend-line /games/nova.manifest shim.cache=/cache/nova\nmkfile-path /games/nova.input\nappend-line /games/nova.input device=gamepad\nappend-line /games/nova.input family=generic\nappend-line /games/nova.input frame=input-001\nappend-line /games/nova.input layout=gamepad-standard\nappend-line /games/nova.input key-table=us-game\nappend-line /games/nova.input pointer-capture=none\nappend-line /games/nova.input delivery=immediate\nappend-line /games/nova.input button=a,press\ngame-launch /games/nova.manifest\ngame-input-translate 77 other /games/nova.input\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("last-status="));
    assert!(stdout.contains("refused"));
}
