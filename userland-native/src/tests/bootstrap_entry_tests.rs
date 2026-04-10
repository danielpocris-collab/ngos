use super::*;

fn standard_boot_env(image_path: &'static str, outcome_policy: &'static str) -> [&'static str; 33] {
    [
        "NGOS_BOOT=1",
        "NGOS_BOOT_PROTOCOL=limine",
        "NGOS_BOOT_MODULE=ngos-userland-native",
        "NGOS_BOOT_MODULE_LEN=12288",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_BOOT_MODULE_PHYS_START=0x200000",
        "NGOS_BOOT_MODULE_PHYS_END=0x203000",
        image_path,
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
        outcome_policy,
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ]
}

fn standard_boot_auxv() -> [ngos_user_abi::AuxvEntry; 2] {
    [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ]
}

#[test]
fn native_program_accepts_basic_bootstrap_and_emits_descriptor_syscalls_then_write() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = ["TERM=dumb"];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &[]);
    assert_eq!(main(&runtime, &bootstrap), 0);
    let frames = runtime.backend().frames.borrow();
    let frame_numbers = frames.iter().map(|frame| frame.number).collect::<Vec<_>>();
    let expected_prefix = vec![
        SYS_FCNTL,
        SYS_POLL,
        SYS_DUP,
        SYS_FCNTL,
        SYS_FCNTL,
        SYS_CLOSE,
        SYS_INSPECT_DEVICE,
        SYS_INSPECT_DRIVER,
        SYS_OPEN_PATH,
        SYS_WRITE,
        SYS_OPEN_PATH,
        SYS_POLL,
        SYS_READ,
        SYS_WRITE,
        SYS_POLL,
        SYS_READ,
        SYS_CLOSE,
        SYS_CLOSE,
        SYS_CREATE_DOMAIN,
        SYS_CREATE_RESOURCE,
        SYS_CREATE_CONTRACT,
        SYS_CREATE_CONTRACT,
        SYS_CREATE_CONTRACT,
        SYS_LIST_DOMAINS,
        SYS_LIST_RESOURCES,
        SYS_LIST_CONTRACTS,
        SYS_INSPECT_DOMAIN,
        SYS_INSPECT_RESOURCE,
        SYS_INSPECT_CONTRACT,
        SYS_SET_CONTRACT_STATE,
        SYS_INSPECT_CONTRACT,
        SYS_INVOKE_CONTRACT,
        SYS_SET_CONTRACT_STATE,
        SYS_INSPECT_CONTRACT,
        SYS_INVOKE_CONTRACT,
        SYS_SET_RESOURCE_POLICY,
        SYS_CLAIM_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_LIST_RESOURCE_WAITERS,
        SYS_CANCEL_RESOURCE_CLAIM,
        SYS_LIST_RESOURCE_WAITERS,
        SYS_INSPECT_RESOURCE,
        SYS_RELEASE_CLAIMED_RESOURCE,
        SYS_INSPECT_RESOURCE,
        SYS_TRANSFER_RESOURCE,
        SYS_LIST_RESOURCE_WAITERS,
        SYS_RELEASE_RESOURCE,
        SYS_INSPECT_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_SET_CONTRACT_STATE,
        SYS_LIST_RESOURCE_WAITERS,
        SYS_INSPECT_RESOURCE,
        SYS_RELEASE_CLAIMED_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_SET_CONTRACT_STATE,
        SYS_CLAIM_RESOURCE,
        SYS_SET_CONTRACT_STATE,
        SYS_INSPECT_RESOURCE,
        SYS_SET_RESOURCE_GOVERNANCE,
        SYS_INSPECT_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_INSPECT_RESOURCE,
        SYS_RELEASE_CLAIMED_RESOURCE,
        SYS_CREATE_CONTRACT,
        SYS_SET_RESOURCE_CONTRACT_POLICY,
        SYS_INSPECT_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_RELEASE_CLAIMED_RESOURCE,
        SYS_CREATE_CONTRACT,
        SYS_SET_RESOURCE_ISSUER_POLICY,
        SYS_INSPECT_RESOURCE,
        SYS_CREATE_CONTRACT,
        SYS_SET_RESOURCE_STATE,
        SYS_INSPECT_RESOURCE,
        SYS_CLAIM_RESOURCE,
        SYS_INVOKE_CONTRACT,
        SYS_CREATE_CONTRACT,
        SYS_SET_RESOURCE_STATE,
        SYS_CREATE_CONTRACT,
        SYS_SET_RESOURCE_STATE,
        SYS_INSPECT_RESOURCE,
        SYS_INSPECT_CONTRACT,
        SYS_CREATE_CONTRACT,
        SYS_GET_DOMAIN_NAME,
        SYS_GET_RESOURCE_NAME,
        SYS_GET_CONTRACT_LABEL,
        SYS_WRITE,
    ];
    let mut cursor = 0usize;
    for expected in expected_prefix {
        let Some(found) = frame_numbers[cursor..]
            .iter()
            .position(|number| *number == expected)
        else {
            panic!("missing expected syscall {expected} after position {cursor}");
        };
        cursor += found + 1;
    }
    assert!(frame_numbers.contains(&SYS_MAP_ANONYMOUS_MEMORY));
    assert!(frame_numbers.contains(&SYS_MAP_FILE_MEMORY));
    assert!(frame_numbers.contains(&SYS_SPAWN_PROCESS_COPY_VM));
    assert!(frame_numbers.ends_with(&[SYS_WRITE]));
    let first_poll = frames
        .iter()
        .find(|frame| frame.number == SYS_POLL)
        .unwrap();
    assert_eq!(first_poll.arg1 as u32, POLLOUT);
    let first_open = frames
        .iter()
        .find(|frame| frame.number == SYS_OPEN_PATH)
        .unwrap();
    assert_eq!(first_open.arg1, "/dev/storage0".len());
    let stdout_write = frames
        .iter()
        .find(|frame| frame.number == SYS_WRITE && frame.arg0 == 1)
        .unwrap();
    assert!(stdout_write.arg2 > 0);
}

#[test]
fn native_program_accepts_bootstrap_with_framebuffer_metadata() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = standard_boot_env(
        "NGOS_IMAGE_PATH=ngos-userland-native",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
    );
    let auxv = standard_boot_auxv();
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
}

#[test]
fn native_program_accepts_bootstrap_with_allow_any_exit_policy() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = standard_boot_env(
        "NGOS_IMAGE_PATH=ngos-userland-native",
        "NGOS_BOOT_OUTCOME_POLICY=allow-any-exit",
    );
    let auxv = standard_boot_auxv();
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
}

#[test]
fn native_program_accepts_bootstrap_with_kernel_image_path() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native", "--boot"];
    let envp = standard_boot_env(
        "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
        "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
    );
    let auxv = standard_boot_auxv();
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
}
