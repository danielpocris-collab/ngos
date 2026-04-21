use super::*;

#[test]
fn native_shell_runs_compat_gfx_smoke_command_and_reports_success_refusal_and_recovery() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b"compat-gfx-smoke\nexit 0\n"));
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
    assert!(stdout.contains("compat.gfx.smoke.success request="));
    assert!(stdout.contains("frame=qemu-compat-001"));
    assert!(stdout.contains("api=directx12"));
    assert!(stdout.contains("translation=compat-to-vulkan"));
    assert!(stdout.contains("deep-ops=clear,gradient-rect,flip-region"));
    assert!(
        stdout.contains("compat.gfx.smoke.refusal request=missing errno=ENOENT outcome=expected")
    );
    assert!(stdout.contains("compat.gfx.smoke.recovery request="));
    assert!(stdout.contains("frame=qemu-compat-002"));
    assert!(stdout.contains("api=opengl"));
    assert!(stdout.contains("deep-ops=clear,set-clip,clear-clip,flip-region"));
    assert!(stdout.contains("compat-gfx-smoke-ok"));
}

#[test]
fn native_shell_runs_compat_audio_smoke_command_and_reports_success_refusal_and_recovery() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-audio-smoke\nexit 0\n",
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
    assert!(stdout.contains("compat.audio.smoke.success request="));
    assert!(stdout.contains("stream=qemu-audio-001"));
    assert!(stdout.contains("api=xaudio2"));
    assert!(stdout.contains("translation=compat-to-mixer"));
    assert!(
        stdout.contains("compat.audio.smoke.refusal request=missing errno=ENOENT outcome=expected")
    );
    assert!(stdout.contains("compat.audio.smoke.recovery request="));
    assert!(stdout.contains("stream=qemu-audio-002"));
    assert!(stdout.contains("api=webaudio"));
    assert!(stdout.contains("translation=native-mixer"));
    assert!(stdout.contains("compat-audio-smoke-ok"));
}

#[test]
fn native_shell_runs_compat_input_smoke_command_and_reports_success_refusal_and_recovery() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-input-smoke\nexit 0\n",
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
    assert!(stdout.contains("compat.input.smoke.success request="));
    assert!(stdout.contains("frame=qemu-input-001"));
    assert!(stdout.contains("api=xinput"));
    assert!(stdout.contains("translation=compat-to-input"));
    assert!(
        stdout.contains("compat.input.smoke.refusal request=missing errno=ENOENT outcome=expected")
    );
    assert!(stdout.contains("compat.input.smoke.recovery request="));
    assert!(stdout.contains("frame=qemu-input-002"));
    assert!(stdout.contains("api=evdev"));
    assert!(stdout.contains("translation=native-input"));
    assert!(stdout.contains("compat-input-smoke-ok"));
}

#[test]
fn native_shell_runs_compat_loader_smoke_command_and_reports_success_refusal_and_recovery() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-loader-smoke\nexit 0\n",
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
    assert!(stdout.contains("compat.loader.smoke.plan slug=nova-strike api=directx11"));
    assert!(stdout.contains("preloads=2"));
    assert!(stdout.contains("dll-overrides=2"));
    assert!(stdout.contains("env-overrides=2"));
    assert!(stdout.contains("translation=compat-to-vulkan"));
    assert!(stdout.contains("compat.loader.smoke.success pid="));
    assert!(stdout.contains("route=compat-game-runtime mode=compat-shim entry=dx-to-vulkan-entry bootstrap=shim-heavy entrypoint=/compat/bin/game-entry requires-shims=1"));
    assert!(stdout.contains("slug=nova-strike"));
    assert!(
        stdout.contains("preloads=/compat/nova/preload/d3d11.ngm;/compat/nova/preload/xaudio2.ngm")
    );
    assert!(stdout.contains("dll-overrides=d3d11=builtin;xaudio2=native"));
    assert!(stdout.contains("env-overrides=DXVK_HUD=1;WINEDEBUG=-all"));
    assert!(stdout.contains("compat.loader.smoke.refusal path=/games/bad.manifest"));
    assert!(stdout.contains("reason=loader-overrides-invalid"));
    assert!(stdout.contains("outcome=expected"));
    assert!(stdout.contains("compat.loader.smoke.relaunch.stopped pid="));
    assert!(stdout.contains("compat.loader.smoke.recovery pid="));
    assert!(stdout.contains("api=vulkan"));
    assert!(stdout.contains("translation=native-vulkan"));
    assert!(stdout.contains("route=native-app-runtime mode=native-direct entry=native-vulkan-entry bootstrap=env-overlay entrypoint=/compat/bin/app-entry requires-shims=0"));
    assert!(stdout.contains("env-overrides=NGOS_COMPAT_RECOVERY=1"));
    assert!(stdout.contains("running=1 stopped=1"));
    assert!(stdout.contains("compat.loader.smoke.matrix pid="));
    assert!(stdout.contains("target=tool slug=nova-tool api=webgpu translation=compat-to-vulkan route=compat-tool-runtime mode=compat-shim entry=webgpu-to-vulkan-entry bootstrap=bootstrap-light entrypoint=/compat/bin/tool-entry requires-shims=1 preloads=0 dll-overrides=0 env-overrides=0"));
    assert!(stdout.contains("target=other slug=nova-service api=vulkan translation=native-vulkan route=native-other-runtime mode=native-direct entry=native-vulkan-entry bootstrap=shim-heavy entrypoint=/compat/bin/other-entry requires-shims=1 preloads=1 dll-overrides=1 env-overrides=0"));
    assert!(stdout.contains("compat.loader.smoke.cleanup pid="));
    assert!(stdout.contains("running=0 stopped=4"));
    assert!(stdout.contains("compat-loader-smoke-ok"));
}

#[test]
fn native_shell_runs_compat_abi_smoke_command_and_reports_success_refusal_and_recovery() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b"compat-abi-smoke\nexit 0\n"));
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

    let result = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(result, 0, "compat-abi smoke stdout:\n{stdout}");
    assert!(
        stdout.contains(
            "compat.abi.smoke.handle.success id=1 dup=2 kind=domain object-id=1001 open=2"
        )
    );
    assert!(stdout.contains("compat.abi.smoke.path.success unix=/compat/root/games/nova/config.toml relative=/compat/root/profiles/player-one.cfg"));
    assert!(
        stdout.contains("compat.abi.smoke.sched.success win32=latency-critical posix=best-effort")
    );
    assert!(stdout.contains("compat.abi.smoke.sync.success mutex-id=1 state=locked owner=1000 event-id=1 event-state=signaled"));
    assert!(stdout.contains("compat.abi.smoke.timer.success oneshot-id=1 oneshot-fires=1 oneshot-state=idle periodic-id=2 periodic-fires=2 periodic-due=150 periodic-state=armed"));
    assert!(stdout.contains("compat.abi.smoke.module.success id=1 name=nova.renderer path=/compat/root/modules/nova-renderer.ngm base=0x400000 size=0x20000 state=loaded retain=2 release=1"));
    assert!(stdout.contains("compat.abi.smoke.refusal"));
    assert!(stdout.contains("timer=timer invalid interval id=2"));
    assert!(stdout.contains("module=module already unloaded id=1"));
    assert!(stdout.contains("outcome=expected"));
    assert!(stdout.contains("compat.abi.smoke.proc.success pid="));
    assert!(stdout.contains("fd-count="));
    assert!(stdout.contains("fd0=present fd1=present fd2=present"));
    assert!(stdout.contains("compat.abi.smoke.proc.refusal pid="));
    assert!(stdout.contains("path=/proc/"));
    assert!(stdout.contains("/fd/9999 outcome=expected"));
    assert!(stdout.contains("compat.abi.smoke.proc.recovery pid="));
    assert!(stdout.contains("fd-list=ok outcome=ok"));
    assert!(stdout.contains("compat.abi.smoke.recovery handles-open=0 mutex-state=unlocked event-state=unsignaled path=/compat/root/restored/session.ok sched=background timer-state=idle timer-fires=2 module-id=2 module-name=nova.runtime module-state=loaded module-ref-count=1 outcome=ok"));
    assert!(stdout.contains("compat.abi.smoke.route pid="));
    assert!(stdout.contains("target=game route=compat-game-abi handles=win32-game-handles"));
    assert!(stdout.contains("target=app route=compat-app-abi handles=win32-app-handles"));
    assert!(stdout.contains("target=tool route=compat-tool-abi handles=utility-handles"));
    assert!(stdout.contains("target=other route=compat-other-abi handles=service-handles"));
    assert!(stdout.contains("compat.abi.smoke.cleanup running=0 stopped=4 outcome=ok"));
    assert!(stdout.contains("compat-abi-smoke-ok"));
}
