use super::*;

#[test]
fn native_shell_compat_path_normalize() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-path-prefix /compat/root\ncompat-path-normalize win-abs C:\\Users\\Player\\save.dat\ncompat-path-normalize wine-drive Z:\\home\\user\\file.cfg\ncompat-path-normalize unix-abs /home/user/data.bin\ncompat-path-normalize win-abs C:\\..\\secret\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("compat.path.normalize flavor=windows-absolute"));
    assert!(stdout.contains("output=/compat/root/Users/Player/save.dat"));
    assert!(stdout.contains("compat.path.normalize flavor=wine-drive"));
    assert!(stdout.contains("output=/compat/root/home/user/file.cfg"));
    assert!(stdout.contains("compat.path.normalize flavor=unix-absolute"));
    assert!(stdout.contains("output=/compat/root/home/user/data.bin"));
    assert!(stdout.contains("compat.path.normalize.refused"));
    assert!(stdout.contains("reason=path traversal refused"));
    assert!(stdout.contains("last-status=302"));
}

#[test]
fn native_shell_compat_sched_mapping() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-sched-win32 15\ncompat-sched-win32 0\ncompat-sched-win32 -20\ncompat-sched-posix -15\ncompat-sched-posix 5\ncompat-sched-posix 20\ncompat-sched-class latency-critical\ncompat-sched-class best-effort\ncompat-sched-class unknown-xyz\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("source=win32 priority=15 class=latency-critical"));
    assert!(stdout.contains("source=win32 priority=0 class=best-effort"));
    assert!(stdout.contains("source=win32 priority=-20 class=background"));
    assert!(stdout.contains("source=posix nice=-15 class=latency-critical"));
    assert!(stdout.contains("source=posix nice=5 class=best-effort"));
    assert!(stdout.contains("source=posix nice=20 class=background"));
    assert!(stdout.contains("source=class name=latency-critical class=latency-critical"));
    assert!(stdout.contains("source=class name=best-effort class=best-effort"));
    assert!(stdout.contains("compat.sched.map.refused name=unknown-xyz reason=unknown-class"));
    assert!(stdout.contains("last-status=303"));
}

#[test]
fn native_shell_compat_mutex_lock_unlock_contended() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-mutex-create\ncompat-mutex-status 1\ncompat-mutex-lock 1 1000\ncompat-mutex-status 1\ncompat-mutex-lock 1 2000\nlast-status\ncompat-mutex-unlock 1 999\nlast-status\ncompat-mutex-unlock 1 1000\ncompat-mutex-status 1\nexit 0\n",
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
    assert!(stdout.contains("compat.mutex.create id=1 state=unlocked"));
    assert!(stdout.contains("compat.mutex.status id=1 state=unlocked owner=-"));
    assert!(stdout.contains("compat.mutex.lock id=1 pid=1000 state=locked"));
    assert!(stdout.contains("compat.mutex.status id=1 state=locked owner=1000"));
    assert!(stdout.contains("compat.mutex.lock.refused id=1 pid=2000 reason="));
    assert!(stdout.contains("last-status=303"));
    assert!(stdout.contains("compat.mutex.unlock.refused id=1 pid=999 reason="));
    assert!(stdout.contains("compat.mutex.unlock id=1 pid=1000 state=unlocked"));
}

#[test]
fn native_shell_compat_event_signal_reset_autoreset() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-event-create\ncompat-event-status 1\ncompat-event-signal 1\ncompat-event-status 1\ncompat-event-reset 1\ncompat-event-status 1\ncompat-event-create --auto-reset\ncompat-event-signal 2\ncompat-event-status 2\nexit 0\n",
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
    assert!(stdout.contains("compat.event.create id=1 auto-reset=false state=unsignaled"));
    assert!(stdout.contains("compat.event.status id=1 state=unsignaled auto-reset=false"));
    assert!(stdout.contains("compat.event.signal id=1 state=signaled"));
    assert!(stdout.contains("compat.event.status id=1 state=signaled auto-reset=false"));
    assert!(stdout.contains("compat.event.reset id=1 state=unsignaled"));
    assert!(stdout.contains("compat.event.create id=2 auto-reset=true state=unsignaled"));
    assert!(stdout.contains("compat.event.signal id=2 state=signaled"));
    assert!(stdout.contains("compat.event.status id=2 state=signaled auto-reset=true"));
}
