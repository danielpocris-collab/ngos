use super::*;

#[test]
fn native_shell_repair_ai_diagnoses_repairs_and_learns_memory() {
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
            "repair-ai.diagnose\nrepair-ai.repair\nrepair-ai.memory\nexit 0\n",
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("repair-ai.diagnose state="));
    assert!(stdout.contains("repair-ai.kernel verified="));
    assert!(stdout.contains("repair-ai.kernel-family family="));
    assert!(stdout.contains("repair-ai.graph nodes="));
    assert!(stdout.contains("repair-ai.node kind="));
    assert!(stdout.contains("repair-ai.hypothesis strategy="));
    assert!(stdout.contains("repair-ai.candidate strategy="));
    assert!(stdout.contains("repair-ai.critic winner="));
    assert!(stdout.contains("repair-ai.plan strategy="));
    assert!(stdout.contains("repair-ai.model nodes="));
    assert!(stdout.contains("candidates="));
    assert!(stdout.contains("critique="));
    assert!(stdout.contains("kernel-family="));
    assert!(stdout.contains("repair-ai.choice strategy="));
    assert!(stdout.contains("system.repair-ai.verdict="));
    assert!(stdout.contains("repair-ai.memory id="));
}

#[test]
fn native_shell_repair_ai_refuses_when_verified_core_is_degraded() {
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
            reserved0: 0,
            reserved1: 3,
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
        run_session_shell_script(
            &runtime,
            &session,
            "repair-ai.diagnose\nrepair-ai.repair\nlast-status\nexit 0\n",
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("repair-ai.diagnose state="));
    assert!(stdout.contains("repair-ai.kernel verified=false"));
    assert!(stdout.contains("repair-ai.kernel-family family="));
    assert!(stdout.contains("repair-ai.graph nodes="));
    assert!(stdout.contains("repair-ai.node kind=verified-core"));
    assert!(stdout.contains("repair-ai.hypothesis strategy="));
    assert!(stdout.contains("repair-ai.candidate strategy="));
    assert!(stdout.contains("repair-ai.critic winner="));
    assert!(stdout.contains(
        "repair-ai.refusal reason=verified-core-degraded action=manual-kernel-repair-required"
    ));
    assert!(stdout.contains("last-status=1"));
}

#[test]
fn native_shell_repair_ai_memory_persists_and_reloads_from_vfs() {
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
            "mkdir-path /persist\nrepair-ai.repair\nrepair-ai.save /persist/repair-ai.mem\nrepair-ai.load /persist/repair-ai.mem\nrepair-ai.memory\nexit 0\n",
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("repair-ai.memory.saved path=/persist/repair-ai.mem entries="));
    assert!(stdout.contains("repair-ai.memory.loaded path=/persist/repair-ai.mem entries="));
    assert!(stdout.contains("repair-ai.memory id="));
}

#[test]
fn nextmind_channel_uses_kernel_verified_core_when_snapshot_is_not_verified() {
    let metrics = test_nextmind_metrics(false, 3);
    assert_eq!(
        nextmind_channel_for_metrics(&metrics, PressureState::Stable),
        "kernel::verified-core"
    );
}

#[test]
fn nextmind_metrics_score_penalizes_verified_core_violations() {
    let clean = test_nextmind_metrics(true, 0);
    let broken = test_nextmind_metrics(false, 3);
    assert!(nextmind_metrics_score(&broken) > nextmind_metrics_score(&clean));
}

#[test]
fn nextmind_auto_summary_reports_kernel_verified_core_channel() {
    let before = test_nextmind_metrics(false, 2);
    let report = NextMindDecisionReport {
        trigger: PressureState::MixedPressure,
        before: before.clone(),
        after: before,
        semantic: semantic_for_channel("kernel::verified-core"),
        observation: SemanticObservation {
            cpu_load: 55,
            mem_pressure: 6,
            anomaly_score: 45,
            thermal_c: 62,
        },
        adaptive: AdaptiveState::new().snapshot(),
        actions: Vec::new(),
        verdict: SemanticVerdict::NoChange,
    };

    let line = nextmind_auto_summary(&report);
    assert!(line.contains("nextmind.auto trigger=mixed-pressure"));
    assert!(line.contains("channel=kernel::verified-core"));
    assert!(line.contains("verified-core=false"));
    assert!(line.contains("violations=2"));
}

#[test]
fn nextmind_explain_last_reports_kernel_verified_core_channel() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let before = test_nextmind_metrics(false, 2);
    let report = NextMindDecisionReport {
        trigger: PressureState::MixedPressure,
        before: before.clone(),
        after: before,
        semantic: semantic_for_channel("kernel::verified-core"),
        observation: SemanticObservation {
            cpu_load: 55,
            mem_pressure: 6,
            anomaly_score: 45,
            thermal_c: 62,
        },
        adaptive: AdaptiveState::new().snapshot(),
        actions: Vec::new(),
        verdict: SemanticVerdict::NoChange,
    };
    let adaptive = AdaptiveState::new();
    let mut context = SemanticContext::new();
    context.push(
        "kernel::verified-core",
        &report.semantic,
        "pressure=mixed-pressure runq=2 cpu=55 socket=6 event=1 verified-core=false violations=2",
        &[],
    );

    nextmind_explain_last(&runtime, &adaptive, &context, &Some(report)).unwrap();

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("nextmind.explain trigger=mixed-pressure"));
    assert!(stdout.contains("channel=kernel::verified-core"));
    assert!(stdout.contains("verified-core=false"));
    assert!(stdout.contains("violations=2"));
    assert!(stdout.contains("nextmind.context #1"));
}
