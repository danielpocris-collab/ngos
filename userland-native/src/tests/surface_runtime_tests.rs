use super::*;

#[test]
fn native_shell_mount_commands_report_propagation_and_cloned_children() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"storage-mount /dev/storage0 /mnt/shared\nstorage-mount /dev/storage0 /mnt/peer\nmount-propagation /mnt/shared shared\nmount-propagation /mnt/peer slave\nstorage-mount /dev/storage0 /mnt/shared/child\nmount-info /mnt/shared\nmount-info /mnt/peer\nmount-info /mnt/shared/child\nmount-info /mnt/peer/child\nstorage-unmount /mnt/shared/child\nstorage-unmount /mnt/shared\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("storage-mount device=/dev/storage0 mount=/mnt/shared entries=0"));
    assert!(stdout.contains("storage-mount device=/dev/storage0 mount=/mnt/peer entries=0"));
    assert!(stdout.contains("mount-propagation path=/mnt/shared mode=shared"));
    assert!(stdout.contains("mount-propagation path=/mnt/peer mode=slave"));
    assert!(stdout.contains("mount-info path=/mnt/shared "));
    assert!(stdout.contains("mode=shared"));
    assert!(stdout.contains("mount-info path=/mnt/peer "));
    assert!(stdout.contains("mode=slave"));
    assert!(stdout.contains("mount-info path=/mnt/shared/child "));
    assert!(stdout.contains("mount-info path=/mnt/peer/child "));
    assert!(stdout.contains("storage-unmount mount=/mnt/shared/child generation=1"));
    assert!(stdout.contains("storage-unmount mount=/mnt/shared generation=1"));
    assert!(stdout.contains("last-status=0"));
}

#[test]
fn native_shell_tree_and_grep_commands_do_not_recurse_through_symlink_loops() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /loop\nmkdir-path /loop/dir\nmkfile-path /loop/dir/file.txt\nwrite-file /loop/dir/file.txt loop-data\nsymlink-path /loop/dir/back ..\ntree-path /loop 8\ngrep-tree /loop loop-data 8\nlast-status\nexit 0\n",
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
    assert!(stdout.contains("tree-root path=/loop depth=8"));
    assert!(stdout.contains("grep-match /loop/dir/file.txt:1 loop-data"));
    assert!(
        stdout
            .contains("grep-tree-summary path=/loop needle=loop-data depth=8 visited=4 matches=1")
    );
    assert!(stdout.contains("last-status=0"));
}

#[test]
fn native_shell_runs_wasm_smoke_command_and_reports_wasm_markers() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b"wasm-smoke\nexit 0\n"));
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
    assert!(stdout.contains("wasm.smoke.start component=semantic-observer pid=1"));
    assert!(stdout.contains("wasm.smoke.refusal component=semantic-observer missing=observe-system-process-count outcome=expected"));
    assert!(stdout.contains("wasm.smoke.grants component=semantic-observer grants=observe-process-capability-count,observe-system-process-count"));
    assert!(stdout.contains(
        "wasm.smoke.observe component=semantic-observer pid=1 capabilities=2 processes=2"
    ));
    assert!(stdout.contains("wasm.smoke.recovery component=semantic-observer refusal=observe-system-process-count recovered=yes verdict=ready"));
    assert!(
        stdout.contains("wasm.smoke.result component=semantic-observer verdict=ready outcome=ok")
    );
    assert!(stdout.contains("wasm.smoke.start component=process-identity pid=1"));
    assert!(stdout.contains("wasm.smoke.refusal component=process-identity missing=observe-process-cwd-root outcome=expected"));
    assert!(stdout.contains("wasm.smoke.grants component=process-identity grants=observe-process-status-bytes,observe-process-cwd-root"));
    assert!(stdout.contains("wasm.smoke.observe component=process-identity pid=1 status-bytes="));
    assert!(stdout.contains(
        "wasm.smoke.recovery component=process-identity refusal=observe-process-cwd-root recovered=yes verdict=ready"
    ));
    assert!(
        stdout.contains("wasm.smoke.result component=process-identity verdict=ready outcome=ok")
    );
    assert!(stdout.contains("wasm-smoke-ok"));
}
