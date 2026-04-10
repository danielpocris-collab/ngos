use super::*;

#[test]
fn native_shell_can_watch_and_unwatch_resource_events_through_queue_interface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"queue-create epoll\nresource-watch $LAST_QUEUE_FD 42 900 all\nclaim 43\nqueue-wait $LAST_QUEUE_FD\nresource-unwatch $LAST_QUEUE_FD 42 900\nresource-watch $LAST_QUEUE_FD 42 901 queued\nclaim 44\nqueue-wait $LAST_QUEUE_FD\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("queue-created fd="));
    assert!(stdout.contains("resource-watch queue="));
    assert!(stdout.contains("token=900 kinds=all"));
    assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
    assert!(stdout.contains("queue-event queue="));
    assert!(stdout.contains("source=resource id=42 contract=43 kind=claimed"));
    assert!(stdout.contains("resource-unwatch queue="));
    assert!(stdout.contains("token=901 kinds=queued"));
    assert!(stdout.contains("claim-queued contract=44 resource=42 holder=43 position=1"));
    assert!(stdout.contains("source=resource id=42 contract=44 kind=queued"));
    assert!(stdout.contains("last-status=0"));
}

#[test]
fn native_shell_rejects_invalid_resource_watch_kind_list() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"queue-create epoll\nresource-watch $LAST_QUEUE_FD 42 900 invalid-kind\nlast-status\nexit 0\n",
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

    assert_eq!(main(&runtime, &bootstrap), 199);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("queue-created fd="));
    assert!(stdout.contains(
        "usage: resource-watch <queue-fd> <resource> <token> [all|claimed,queued,canceled,released,handed-off,revoked]"
    ));
}

#[test]
fn native_shell_controls_fd_flags_and_observes_empty_resource_queue_nonblocking() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"queue-create epoll\nfcntl-getfl $LAST_QUEUE_FD\nfcntl-getfd $LAST_QUEUE_FD\nnonblock-fd $LAST_QUEUE_FD on\nfcntl-getfl $LAST_QUEUE_FD\ncloexec-fd $LAST_QUEUE_FD on\nfcntl-getfd $LAST_QUEUE_FD\nresource-watch $LAST_QUEUE_FD 42 900 queued\nresource-unwatch $LAST_QUEUE_FD 42 900\nqueue-wait $LAST_QUEUE_FD\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("fcntl-getfl fd="));
    assert!(stdout.contains("flags=0x0"));
    assert!(stdout.contains("nonblock-fd fd="));
    assert!(stdout.contains("nonblock=1"));
    assert!(stdout.contains("flags=0x2"));
    assert!(stdout.contains("cloexec-fd fd="));
    assert!(stdout.contains("cloexec=1"));
    assert!(stdout.contains("resource-watch queue="));
    assert!(stdout.contains("resource-unwatch queue="));
    assert!(stdout.contains("last-status=246"));
}
