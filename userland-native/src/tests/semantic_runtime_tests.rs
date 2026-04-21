use super::*;

#[test]
fn native_shell_nextmind_optimize_drains_bus_backpressure() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
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
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
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
    let session = parse_session_context(&bootstrap).unwrap();
    runtime.mkdir_path("/ipc").unwrap();
    runtime.mkchan_path("/ipc/render").unwrap();
    assert_eq!(
        run_session_shell_script(
            &runtime,
            &session,
            "mkdomain render\nset BUS_DOMAIN $LAST_DOMAIN_ID\nmkresource $BUS_DOMAIN channel render-bus\nset BUS_RESOURCE $LAST_RESOURCE_ID\nmkbuspeer $BUS_DOMAIN renderer\nset BUS_PEER $LAST_BUS_PEER_ID\nmkbusendpoint $BUS_DOMAIN $BUS_RESOURCE /ipc/render\nset BUS_ENDPOINT $LAST_BUS_ENDPOINT_ID\nbus-attach $BUS_PEER $BUS_ENDPOINT\nrepeat 64 bus-send $BUS_PEER $BUS_ENDPOINT q\nbus-send $BUS_PEER $BUS_ENDPOINT overflow || true\nmode semantic\nnextmind.optimize\nbus-endpoint $BUS_ENDPOINT\nexit 0\n",
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("nextmind.metrics label=before state=mixed-pressure"));
    assert!(stdout.contains("bus-pressure=100"));
    assert!(stdout.contains("bus-overflows=1"));
    assert!(stdout.contains("nextmind.action reason=bus-backpressure detail=drain endpoint="));
    assert!(stdout.contains("nextmind.action reason=bus-drain detail=endpoint="));
    assert!(stdout.contains("nextmind.verdict=improved"));
    assert!(stdout.contains("bus-endpoint-detail id="));
    assert!(stdout.contains("depth=63"));
    assert!(stdout.contains("capacity=64"));
    assert!(stdout.contains("overflows=1"));
    assert!(stdout.contains("receives=1"));
}

#[test]
fn native_shell_nextmind_explain_reports_verified_core_state() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
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
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
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
    let session = parse_session_context(&bootstrap).unwrap();
    assert_eq!(
        run_session_shell_script(
            &runtime,
            &session,
            "mode semantic\nnextmind.optimize\nnextmind.explain last\nexit 0\n",
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("nextmind.explain trigger="));
    assert!(stdout.contains("channel=proc::"));
    assert!(stdout.contains("verified-core=true"));
    assert!(stdout.contains("violations=0"));
}

#[test]
fn native_shell_repair_system_repairs_pressure_and_reports_final_state() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
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
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
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
    let session = parse_session_context(&bootstrap).unwrap();
    assert_eq!(
        run_session_shell_script(&runtime, &session, "repair-system\nexit 0\n"),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("system.repair.before.metrics"));
    assert!(
        stdout.contains("repair.action kind=renice")
            || stdout.contains("repair.action kind=net-admin")
    );
    assert!(stdout.contains("repair.action kind=vm-reclaim"));
    assert!(stdout.contains("system.repair.after.metrics"));
    assert!(
        stdout.contains("system.repair.verdict=improved")
            || stdout.contains("system.repair.verdict=no-change")
    );
    assert!(stdout.contains("verified-core=true"));
    assert!(stdout.contains("violations=0"));
}

#[test]
fn native_shell_modernize_system_applies_modernization_and_reports_final_state() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
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
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
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
    let session = parse_session_context(&bootstrap).unwrap();
    assert_eq!(
        run_session_shell_script(&runtime, &session, "modernize-system\nexit 0\n"),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("system.modernize.before.metrics"));
    assert!(stdout.contains("modernize.action kind=net-profile"));
    assert!(stdout.contains("modernize.action kind=vm-reclaim"));
    assert!(stdout.contains("system.modernize.after.metrics"));
    assert!(stdout.contains("system.modernize.verdict="));
    assert!(stdout.contains("verified-core=true"));
}

#[test]
fn native_shell_repair_system_refuses_when_verified_core_is_degraded() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    runtime
        .backend()
        .set_system_snapshot_override(NativeSystemSnapshotRecord {
            current_tick: 200,
            busy_ticks: 150,
            process_count: 2,
            active_process_count: 2,
            blocked_process_count: 0,
            queued_processes: 2,
            queued_latency_critical: 0,
            queued_interactive: 1,
            queued_normal: 1,
            queued_background: 0,
            queued_urgent_latency_critical: 0,
            queued_urgent_interactive: 0,
            queued_urgent_normal: 0,
            queued_urgent_background: 0,
            lag_debt_latency_critical: 0,
            lag_debt_interactive: 0,
            lag_debt_normal: 0,
            lag_debt_background: 0,
            dispatch_count_latency_critical: 0,
            dispatch_count_interactive: 0,
            dispatch_count_normal: 0,
            dispatch_count_background: 0,
            runtime_ticks_latency_critical: 0,
            runtime_ticks_interactive: 0,
            runtime_ticks_normal: 0,
            runtime_ticks_background: 0,
            scheduler_cpu_count: 1,
            scheduler_running_cpu: 0,
            scheduler_cpu_load_imbalance: 0,
            starved_latency_critical: NativeSystemSnapshotRecord::SCHEDULER_POLICY_FALSE,
            starved_interactive: NativeSystemSnapshotRecord::SCHEDULER_POLICY_FALSE,
            starved_normal: NativeSystemSnapshotRecord::SCHEDULER_POLICY_FALSE,
            starved_background: NativeSystemSnapshotRecord::SCHEDULER_POLICY_FALSE,
            deferred_task_count: 0,
            sleeping_processes: 0,
            total_event_queue_count: 1,
            total_event_queue_pending: 1,
            total_event_queue_waiters: 0,
            total_socket_count: 1,
            saturated_socket_count: 0,
            total_socket_rx_depth: 1,
            total_socket_rx_limit: 8,
            max_socket_rx_depth: 1,
            total_network_tx_dropped: 0,
            total_network_rx_dropped: 0,
            running_pid: 1,
            reserved0: NativeSystemSnapshotRecord::VERIFIED_CORE_OK_FALSE,
            reserved1: 2,
        });
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
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
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
    let session = parse_session_context(&bootstrap).unwrap();
    assert_eq!(
        run_session_shell_script(&runtime, &session, "repair-system\nlast-status\nexit 0\n"),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains(
        "system.repair.refusal reason=verified-core-degraded action=manual-kernel-repair-required"
    ));
    assert!(stdout.contains("last-status=1"));
}
