use super::*;

#[test]
fn native_shell_runs_vfs_smoke_command_and_reports_vfs_markers() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b"vfs-smoke\nexit 0\n"));
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
    assert_eq!(result, 0, "stdout:\n{stdout}");
    assert!(stdout.contains("vfs.smoke.mount pid=1 path=/vfs"));
    assert!(stdout.contains("vfs.smoke.create pid=1 path=/vfs/bin/app"));
    assert!(stdout.contains("vfs.smoke.symlink pid=1 path=/vfs/link target=/vfs/bin/app"));
    assert!(stdout.contains("vfs.smoke.rename pid=1 from=/vfs/bin/app to=/vfs/bin/app2"));
    assert!(stdout.contains("vfs.smoke.unlink pid=1 path=/vfs/link after-unlink=missing"));
    assert!(
        stdout.contains(
            "vfs.smoke.recovery pid=1 path=/vfs/link target=/vfs/bin/app rename-restored=yes readlink=stable outcome=ok"
        ),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains(
        "vfs.smoke.refusal pid=1 create-missing-parent=yes unlink-nonempty-dir=yes outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.symlink-loop pid=1 refusal=loop-detected yes recovery=unlink outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.file pid=1 path=/vfs/bin/app copy=/vfs/bin/app-copy bytes=16 append=yes copy-match=yes outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.link pid=1 source=/vfs/bin/app link=/vfs/bin/app-link shared-inode=yes shared-write=yes links-before=2 links-after=1 unlink-released=yes outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.truncate pid=1 path=/vfs/bin/app-copy shrink=5 extend=8 zero-fill=yes outcome=ok"
    ));
    assert!(stdout.contains("vfs.smoke.unlink-open pid=1 path=/vfs/bin/live fd="));
    assert!(stdout.contains(
        "vfs.smoke.vm-file pid=1 path=/vfs/bin/vm-file sync=yes truncate-reflects=yes unlink-survives=yes unmap=yes outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.permissions pid=1 dir=/vfs/secure file=/vfs/secure/secret.txt list-blocked=yes traverse-blocked=yes rename-blocked=yes unlink-blocked=yes file-read-blocked=yes recovery=yes outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.replace pid=1 source=/vfs/bin/replace-src target=/vfs/bin/replace-dst file-replaced=yes open-target-survives=yes nonempty-dir-refusal=yes empty-dir-replaced=yes kind-mismatch-refusal=yes outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.tree pid=1 source=/vfs/tree-src copy=/vfs/tree-dst mirror=/vfs/tree-mirror refusal=self-nest yes symlink=stable pruned=yes outcome=ok"
    ));
    assert!(stdout.contains(
        "vfs.smoke.mount-propagation pid=1 shared=/vfs/mount-shared peer=/vfs/mount-peer child=/vfs/mount-shared/child clone=/vfs/mount-peer/child"
    ));
    assert!(stdout.contains(
        "cross-mount-rename=blocked cross-mount-link=blocked parent-unmount-blocked=yes recovery=yes outcome=ok"
    ));
    assert!(
        stdout
            .contains("vfs.smoke.list pid=1 path=/vfs/bin entries=2 names=app,app-copy outcome=ok")
    );
    assert!(stdout.contains("vfs.smoke.fd pid=1 fd="));
    assert!(stdout.contains("vfs.smoke.dup pid=1 fd="));
    assert!(stdout.contains("vfs.smoke.fcntl pid=1 fd="));
    assert!(stdout.contains("vfs.smoke.lock pid=1 primary-fd="));
    assert!(stdout.contains("shared=yes shared-refusal=busy mutation-blocked=yes mutation-recovery=yes shared-recovery=yes"));
    assert!(stdout.contains("vfs.smoke.coherence pid=1 descriptor=open-path-open"));
    assert!(stdout.contains("vfs-smoke-ok"));
}

#[test]
fn native_shell_runs_shell_smoke_command_and_reports_shell_markers() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b"shell-smoke\nexit 0\n"));
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
    assert!(stdout.contains("shell.smoke.session protocol=kernel-launch cwd=/ outcome=ok"));
    assert!(stdout.contains(
        "shell.smoke.ux suggest=pro apropos=mount explain=identity-of unknown=feedback outcome=ok"
    ));
    assert!(stdout.contains(
        "shell.smoke.ergonomics topic=pipeline examples=identity-of repeat=yes rerun=yes recent=yes next=review outcome=ok"
    ));
    assert!(
        stdout.contains(
            "shell.smoke.scripting path=/shell-proof/note bytes=14 source=yes outcome=ok"
        )
    );
    assert!(stdout.contains("shell.smoke.lang return=shell-proof-lang argc=1 outcome=ok"));
    assert!(stdout.contains("shell.smoke.match result=matched value=shell-proof-lang outcome=ok"));
    assert!(stdout.contains("shell.smoke.values type=record path=src/lib.rs outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline path=src/lib.rs type=string outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-real source=session outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-system pid=1 outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-list count="));
    assert!(stdout.contains("shell.smoke.pipeline-waiters count="));
    assert!(stdout.contains("shell.smoke.pipeline-mount path=/shell-proof-mount"));
    assert!(stdout.contains("shell.smoke.pipeline-mounts count=1 outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-filter count=1 outcome=ok"));
    assert!(
        stdout.contains(
            "shell.smoke.pipeline-inventory domains=1 resources=1 contracts=1 outcome=ok"
        )
    );
    assert!(stdout.contains("shell.smoke.pipeline-queues epoll=1 kqueue=1 outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-fd source=list kind=File outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-maps pid=1 source=list outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-vm objects="));
    assert!(stdout.contains("shell.smoke.pipeline-bool contains=true starts=true ends=true not=false empty=true outcome=ok"));
    assert!(stdout.contains("shell.smoke.pipeline-caps count="));
    assert!(stdout.contains("shell.smoke.pipeline-recordops owner="));
    assert!(stdout.contains("shell.smoke.pipeline-jobs count="));
    assert!(stdout.contains("shell.smoke.pipeline-compat route="));
    assert!(stdout.contains("shell.smoke.pipeline-identity uid="));
    assert!(
        stdout
            .contains("shell.smoke.pipeline-recordpredicates identity=true compat=true outcome=ok")
    );
    assert!(stdout.contains("shell.smoke.pipeline-auxv count="));
    assert!(stdout.contains("shell.smoke.pipeline-procfs status="));
    assert!(stdout.contains("shell.smoke.pipeline-vfsstats nodes="));
    assert!(
        stdout.contains("shell.smoke.pipeline-listfields mount-device=1 mount-mode=2 auxv-exec=")
    );
    assert!(stdout.contains("shell.smoke.pipeline-listpredicates any=true all=true outcome=ok"));
    assert!(stdout.contains(
        "shell.smoke.coding build=/shell-proof/build.log test=/shell-proof/test.log outcome=ok"
    ));
    assert!(stdout.contains(
        "shell.smoke.review left=/shell-proof/review.before right=/shell-proof/review.after outcome=ok"
    ));
    assert!(stdout.contains("process-spawned pid="));
    assert!(stdout.contains("job-info pid="));
    assert!(stdout.contains("foreground-complete pid="));
    assert!(stdout.contains("shell.smoke.jobs pid="));
    assert!(stdout.contains("shell.smoke.observe pid=1 procfs=stat-open outcome=ok"));
    assert!(stdout.contains("shell.smoke.refusal pid=1 command=missing-command outcome=expected"));
    assert!(stdout.contains("recovered"));
    assert!(stdout.contains("shell.smoke.recovery pid=1 guard=or outcome=ok"));
    assert!(stdout.contains("shell-proof"));
    assert!(stdout.contains("shell.smoke.state pid=1 cwd=/ note=/shell-proof/note outcome=ok"));
    assert!(stdout.contains("shell-smoke-ok"));
}
