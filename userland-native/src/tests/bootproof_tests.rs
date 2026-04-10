use super::*;

#[test]
fn native_program_runs_process_exec_bootproof_and_reports_process_exec_markers() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = [
        "NGOS_BOOT=1",
        "NGOS_BOOT_PROTOCOL=limine",
        "NGOS_BOOT_MODULE=ngos-userland-native",
        "NGOS_BOOT_MODULE_LEN=12288",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_BOOT_MODULE_PHYS_START=0x200000",
        "NGOS_BOOT_MODULE_PHYS_END=0x203000",
        "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_FRAMEBUFFER_PRESENT=1",
        "NGOS_FRAMEBUFFER_WIDTH=1920",
        "NGOS_FRAMEBUFFER_HEIGHT=1080",
        "NGOS_FRAMEBUFFER_PITCH=7680",
        "NGOS_FRAMEBUFFER_BPP=32",
        "NGOS_MEMORY_REGION_COUNT=2",
        "NGOS_USABLE_MEMORY_BYTES=8388608",
        "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
        "NGOS_RSDP=0xdeadbeef",
        "NGOS_KERNEL_PHYS_START=0x100000",
        "NGOS_KERNEL_PHYS_END=0x101000",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
        "NGOS_BOOT_PROOF=process-exec",
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
    assert_eq!(result, 0, "process-exec stdout:\n{stdout}");
    assert!(stdout.contains("boot.proof=process-exec"));
    assert!(
        stdout.contains("process.exec.smoke.refusal pid=")
            || stdout.contains("process.exec.smoke.observe pid=")
    );
    assert!(stdout.contains("mode=metadata-only"));
    assert!(stdout.contains("process.exec.smoke.recovery pid="));
    assert!(stdout.contains("process.exec.smoke.spawn pid="));
    assert!(stdout.contains("process.exec.smoke.success pid="));
    assert!(stdout.contains("process.exec.smoke.state pid="));
    assert!(stdout.contains("process-exec-smoke-ok"));
}

#[test]
fn native_program_runs_compat_foreign_bootproof_and_reports_unified_family_markers() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = [
        "NGOS_BOOT=1",
        "NGOS_BOOT_PROTOCOL=limine",
        "NGOS_BOOT_MODULE=ngos-userland-native",
        "NGOS_BOOT_MODULE_LEN=12288",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_BOOT_MODULE_PHYS_START=0x200000",
        "NGOS_BOOT_MODULE_PHYS_END=0x203000",
        "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_FRAMEBUFFER_PRESENT=1",
        "NGOS_FRAMEBUFFER_WIDTH=1920",
        "NGOS_FRAMEBUFFER_HEIGHT=1080",
        "NGOS_FRAMEBUFFER_PITCH=7680",
        "NGOS_FRAMEBUFFER_BPP=32",
        "NGOS_MEMORY_REGION_COUNT=2",
        "NGOS_USABLE_MEMORY_BYTES=8388608",
        "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
        "NGOS_RSDP=0xdeadbeef",
        "NGOS_KERNEL_PHYS_START=0x100000",
        "NGOS_KERNEL_PHYS_END=0x101000",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
        "NGOS_BOOT_PROOF=compat-foreign",
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
    assert_eq!(result, 0, "compat-foreign stdout:\n{stdout}");
    assert!(
        stdout.contains(
            "boot.cpu xsave=true save_area=4096 xcr0=0xe7 seed=0x12345678 hw_provider=true"
        )
    );
    assert!(stdout.contains("boot.proof=compat-foreign"));
    assert!(stdout.contains("compat-loader-foreign-smoke-ok"));
    assert!(stdout.contains("compat-abi-smoke-ok"));
    assert!(stdout.contains("compat-foreign-smoke-ok"));
    assert!(stdout.contains("compat.loader.foreign.success pid="));
    assert!(stdout.contains("compat.abi.smoke.proc.success pid="));
}

#[test]
fn native_program_runs_shell_bootproof_and_reports_shell_markers() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = [
        "NGOS_BOOT=1",
        "NGOS_BOOT_PROTOCOL=limine",
        "NGOS_BOOT_MODULE=ngos-userland-native",
        "NGOS_BOOT_MODULE_LEN=12288",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_BOOT_MODULE_PHYS_START=0x200000",
        "NGOS_BOOT_MODULE_PHYS_END=0x203000",
        "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_FRAMEBUFFER_PRESENT=1",
        "NGOS_FRAMEBUFFER_WIDTH=1920",
        "NGOS_FRAMEBUFFER_HEIGHT=1080",
        "NGOS_FRAMEBUFFER_PITCH=7680",
        "NGOS_FRAMEBUFFER_BPP=32",
        "NGOS_MEMORY_REGION_COUNT=2",
        "NGOS_USABLE_MEMORY_BYTES=8388608",
        "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
        "NGOS_RSDP=0xdeadbeef",
        "NGOS_KERNEL_PHYS_START=0x100000",
        "NGOS_KERNEL_PHYS_END=0x101000",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
        "NGOS_BOOT_PROOF=shell",
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
    assert_eq!(result, 0, "shell stdout:\n{stdout}");
    assert!(
        stdout.contains(
            "boot.cpu xsave=true save_area=4096 xcr0=0xe7 seed=0x12345678 hw_provider=true"
        )
    );
    assert!(stdout.contains("boot.proof=shell"));
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
}

#[test]
fn native_program_runs_device_runtime_bootproof_and_reports_unified_markers() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = [
        "NGOS_BOOT=1",
        "NGOS_BOOT_PROTOCOL=limine",
        "NGOS_BOOT_MODULE=ngos-userland-native",
        "NGOS_BOOT_MODULE_LEN=12288",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_BOOT_MODULE_PHYS_START=0x200000",
        "NGOS_BOOT_MODULE_PHYS_END=0x203000",
        "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_FRAMEBUFFER_PRESENT=1",
        "NGOS_FRAMEBUFFER_WIDTH=1920",
        "NGOS_FRAMEBUFFER_HEIGHT=1080",
        "NGOS_FRAMEBUFFER_PITCH=7680",
        "NGOS_FRAMEBUFFER_BPP=32",
        "NGOS_MEMORY_REGION_COUNT=2",
        "NGOS_USABLE_MEMORY_BYTES=8388608",
        "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
        "NGOS_RSDP=0xdeadbeef",
        "NGOS_KERNEL_PHYS_START=0x100000",
        "NGOS_KERNEL_PHYS_END=0x101000",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
        "NGOS_BOOT_PROOF=device-runtime",
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
    assert_eq!(result, 0, "device-runtime stdout:\n{stdout}");
    assert!(
        stdout.contains(
            "boot.cpu xsave=true save_area=4096 xcr0=0xe7 seed=0x12345678 hw_provider=true"
        )
    );
    assert!(stdout.contains("boot.proof=device-runtime"));
    assert!(stdout.contains("device.runtime.smoke.graphics device=/dev/gpu0"));
    assert!(stdout.contains("device.runtime.smoke.audio device=/dev/audio0"));
    assert!(stdout.contains("device.runtime.smoke.input device=/dev/input0"));
    assert!(stdout.contains("network.smoke.success"));
    assert!(stdout.contains("network.smoke.teardown socket=/run/net1.sock"));
    assert!(stdout.contains("network.smoke.rebind socket=/run/net1.sock"));
    assert!(stdout.contains("network.smoke.recovery local=10.1.0.2:4000"));
    assert!(stdout.contains("storage.smoke.mount.commit mount=/persist"));
    assert!(stdout.contains("device.runtime.smoke.storage device=/dev/storage0"));
    assert!(stdout.contains("device-runtime-smoke-ok"));
}

#[test]
fn native_program_runs_bus_bootproof_and_reports_bus_markers() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = [
        "NGOS_BOOT=1",
        "NGOS_BOOT_PROTOCOL=limine",
        "NGOS_BOOT_MODULE=ngos-userland-native",
        "NGOS_BOOT_MODULE_LEN=12288",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_BOOT_MODULE_PHYS_START=0x200000",
        "NGOS_BOOT_MODULE_PHYS_END=0x203000",
        "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_FRAMEBUFFER_PRESENT=1",
        "NGOS_FRAMEBUFFER_WIDTH=1920",
        "NGOS_FRAMEBUFFER_HEIGHT=1080",
        "NGOS_FRAMEBUFFER_PITCH=7680",
        "NGOS_FRAMEBUFFER_BPP=32",
        "NGOS_MEMORY_REGION_COUNT=2",
        "NGOS_USABLE_MEMORY_BYTES=8388608",
        "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
        "NGOS_RSDP=0xdeadbeef",
        "NGOS_KERNEL_PHYS_START=0x100000",
        "NGOS_KERNEL_PHYS_END=0x101000",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
        "NGOS_BOOT_PROOF=bus",
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
    assert_eq!(result, 0, "bus stdout:\n{stdout}");
    assert!(
        stdout.contains(
            "boot.cpu xsave=true save_area=4096 xcr0=0xe7 seed=0x12345678 hw_provider=true"
        )
    );
    assert!(stdout.contains("boot.proof=bus"));
    assert!(
        stdout
            .contains("bus.smoke.refusal path=/proc/system/bus contract=observe outcome=expected")
    );
    assert!(stdout.contains("bus.smoke.observe path=/proc/system/bus"));
    assert!(stdout.contains("path=/ipc/render capacity=64 outcome=ok"));
    assert!(stdout.contains("bus.smoke.attach peer="));
    assert!(stdout.contains("kind=attached outcome=ok"));
    assert!(stdout.contains("bus.smoke.success peer="));
    assert!(stdout.contains("payload=hello-qemu outcome=ok"));
    assert!(stdout.contains("bus.smoke.overflow peer="));
    assert!(stdout.contains("errno=Again"));
    assert!(stdout.contains("peak=64 overflows=1 outcome=ok"));
    assert!(stdout.contains("bus.smoke.detach peer="));
    assert!(stdout.contains("bus.smoke.refusal peer="));
    assert!(stdout.contains("outcome=expected"));
    assert!(stdout.contains("bus.smoke.recovery peer="));
    assert!(stdout.contains("payload=recovered-qemu outcome=ok"));
    assert!(stdout.contains("bus.smoke.state peer="));
    assert!(stdout.contains("publishes=67 receives=67"));
    assert!(stdout.contains("bus-smoke-ok"));
}

#[test]
fn native_program_runs_vm_bootproof_and_reports_vm_and_vfs_markers() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = [
        "NGOS_BOOT=1",
        "NGOS_BOOT_PROTOCOL=limine",
        "NGOS_BOOT_MODULE=ngos-userland-native",
        "NGOS_BOOT_MODULE_LEN=12288",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_BOOT_MODULE_PHYS_START=0x200000",
        "NGOS_BOOT_MODULE_PHYS_END=0x203000",
        "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_FRAMEBUFFER_PRESENT=1",
        "NGOS_FRAMEBUFFER_WIDTH=1920",
        "NGOS_FRAMEBUFFER_HEIGHT=1080",
        "NGOS_FRAMEBUFFER_PITCH=7680",
        "NGOS_FRAMEBUFFER_BPP=32",
        "NGOS_MEMORY_REGION_COUNT=2",
        "NGOS_USABLE_MEMORY_BYTES=8388608",
        "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
        "NGOS_RSDP=0xdeadbeef",
        "NGOS_KERNEL_PHYS_START=0x100000",
        "NGOS_KERNEL_PHYS_END=0x101000",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
        "NGOS_BOOT_PROOF=vm",
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
    assert_eq!(result, 0, "vm stdout:\n{stdout}");
    assert!(stdout.contains("boot.proof=vm"));
    assert!(stdout.contains(
        "vm.smoke.production pid=1 stress=yes pressure=yes global-pressure=yes advise=yes quarantine=yes policy=yes workloads=anon,cow,file,heap,region outcome=ok"
    ));
    assert!(stdout.contains("vfs.smoke.mount pid=1 path=/vfs"));
    assert!(stdout.contains("vfs.smoke.create pid=1 path=/vfs/bin/app"));
    assert!(stdout.contains("vfs.smoke.symlink pid=1 path=/vfs/link target=/vfs/bin/app"));
    assert!(stdout.contains("vfs.smoke.rename pid=1 from=/vfs/bin/app to=/vfs/bin/app2"));
    assert!(stdout.contains("vfs.smoke.unlink pid=1 path=/vfs/link after-unlink=missing"));
    assert!(stdout.contains(
        "vfs.smoke.coherence pid=1 descriptor=open-path-open readlink=stable statfs=ok outcome=ok"
    ));
}
