//! Canonical subsystem role:
//! - subsystem: native control / semantic userland
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: control and explainability surface over kernel truth
//!
//! Canonical contract families consumed here:
//! - runtime snapshot contracts
//! - verified-core contracts
//! - scheduler fairness contracts
//! - process/network/device inspection contracts
//!
//! This module is an operational consumer.
//! It may explain, score, and act on canonical system truth, but it must not
//! become the semantic owner of scheduler, VFS, VM, networking, or other
//! kernel subsystems.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_semantic::SemanticEntityEpoch;
use ngos_user_abi::{
    Errno, ExitCode, NativeEventRecord, NativeNetworkInterfaceRecord, NativeSchedulerClass,
    NativeSystemSnapshotRecord, POLLIN, POLLPRI, SyscallBackend,
};
use ngos_user_runtime::{
    Runtime,
    system_control::{
        AdaptiveState, AdaptiveStateSnapshot, BusEndpointEntity, CapabilityToken, DeviceHandle,
        EventFilter, EventSemantic, EventStream, PressureState, ProcessAction, ProcessEntity,
        SemanticActionRecord, SemanticContext, SemanticEntity, SemanticVerdict, SystemController,
        SystemFact, SystemPressureMetrics, cpu_mask_for, load_percent, pressure_channel_name,
        select_cpu, semantic_capabilities_csv, semantic_class_name, semantic_entity_kind_name,
        semantic_verdict_name,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextMindDecisionReport {
    pub trigger: PressureState,
    pub before: SystemPressureMetrics,
    pub after: SystemPressureMetrics,
    pub semantic: EventSemantic,
    pub observation: ngos_user_runtime::system_control::SemanticObservation,
    pub adaptive: AdaptiveStateSnapshot,
    pub actions: Vec<SemanticActionRecord>,
    pub verdict: SemanticVerdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextMindAutoState {
    pub enabled: bool,
    pub streams: Vec<EventStream>,
}

pub struct NextMindAgentState<'a> {
    pub last_snapshot: &'a mut Option<NativeSystemSnapshotRecord>,
    pub adaptive_state: &'a mut AdaptiveState,
    pub context: &'a mut SemanticContext,
    pub entity_epochs: &'a mut Vec<SemanticEntityEpoch>,
    pub auto_state: &'a mut NextMindAutoState,
    pub last_report: &'a mut Option<NextMindDecisionReport>,
    pub last_status: &'a mut i32,
}

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub fn nextmind_pressure_state_label(state: PressureState) -> &'static str {
    match state {
        PressureState::Stable => "stable",
        PressureState::HighSchedulerPressure => "high-scheduler-pressure",
        PressureState::NetworkBackpressure => "network-backpressure",
        PressureState::MixedPressure => "mixed-pressure",
    }
}

pub fn nextmind_channel_for_metrics(
    metrics: &SystemPressureMetrics,
    state: PressureState,
) -> String {
    if metrics.verified_core_ok {
        pressure_channel_name(state).to_string()
    } else {
        String::from("kernel::verified-core")
    }
}

pub fn nextmind_auto_summary(report: &NextMindDecisionReport) -> String {
    format!(
        "nextmind.auto trigger={} channel={} class={} verdict={} verified-core={} violations={}",
        nextmind_pressure_state_label(report.trigger),
        nextmind_channel_for_metrics(&report.before, report.trigger),
        semantic_class_name(report.semantic.class),
        semantic_verdict_name(report.verdict),
        report.before.verified_core_ok,
        report.before.verified_core_violation_count,
    )
}

pub fn nextmind_metrics_score(metrics: &SystemPressureMetrics) -> u64 {
    metrics.run_queue_total.saturating_mul(100)
        + metrics.run_queue_urgent_total().saturating_mul(80)
        + metrics.scheduler_lag_debt_total.max(0) as u64 * 25
        + metrics.scheduler_dispatch_total.saturating_mul(5)
        + metrics.scheduler_runtime_ticks_total.saturating_mul(5)
        + metrics.scheduler_runtime_imbalance.saturating_mul(20)
        + metrics.scheduler_cpu_load_imbalance.saturating_mul(40)
        + metrics.bus_endpoint_count.saturating_mul(5)
        + metrics.saturated_bus_endpoint_count.saturating_mul(100)
        + metrics.bus_queue_depth_total.saturating_mul(15)
        + metrics.bus_overflow_total.saturating_mul(120)
        + u64::from(metrics.bus_pressure_pct)
        + metrics.snapshot.saturated_socket_count.saturating_mul(100)
        + u64::from(metrics.cpu_utilization_pct)
        + u64::from(metrics.socket_pressure_pct)
        + u64::from(metrics.event_queue_pressure_pct)
        + metrics.snapshot.blocked_process_count.saturating_mul(5)
        + if metrics.scheduler_starved { 250 } else { 0 }
        + if metrics.verified_core_ok {
            0
        } else {
            500 + metrics.verified_core_violation_count.saturating_mul(50)
        }
}

pub fn nextmind_render_metrics<B: SyscallBackend>(
    runtime: &Runtime<B>,
    label: &str,
    metrics: &SystemPressureMetrics,
    state: PressureState,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "nextmind.metrics label={label} state={} runq={} urgent={} lag={} dispatches={} runtime-ticks={} imbalance={} sched-cpus={} running-cpu={} cpu-load-imbalance={} starved={} classes={}/{}/{}/{} cpu={} active={} blocked={} bus-endpoints={} bus-depth={}/{} bus-pressure={} bus-overflows={} sockets={}/{} socket-pressure={} event-pressure={} drops={}/{} drop-delta={}/{} verified-core={} violations={} busy={}/{}",
            nextmind_pressure_state_label(state),
            metrics.run_queue_total,
            metrics.run_queue_urgent_total(),
            metrics.scheduler_lag_debt_total,
            metrics.scheduler_dispatch_total,
            metrics.scheduler_runtime_ticks_total,
            metrics.scheduler_runtime_imbalance,
            metrics.scheduler_cpu_count,
            metrics
                .scheduler_running_cpu
                .map(|cpu| cpu.to_string())
                .unwrap_or_else(|| String::from("-")),
            metrics.scheduler_cpu_load_imbalance,
            metrics.scheduler_starved,
            metrics.run_queue_latency_critical,
            metrics.run_queue_interactive,
            metrics.run_queue_normal,
            metrics.run_queue_background,
            metrics.cpu_utilization_pct,
            metrics.snapshot.active_process_count,
            metrics.snapshot.blocked_process_count,
            metrics.bus_endpoint_count,
            metrics.bus_queue_depth_total,
            metrics.bus_queue_capacity_total,
            metrics.bus_pressure_pct,
            metrics.bus_overflow_total,
            metrics.snapshot.total_socket_rx_depth,
            metrics.snapshot.total_socket_rx_limit,
            metrics.socket_pressure_pct,
            metrics.event_queue_pressure_pct,
            metrics.snapshot.total_network_tx_dropped,
            metrics.snapshot.total_network_rx_dropped,
            metrics.tx_drop_delta,
            metrics.rx_drop_delta,
            metrics.verified_core_ok,
            metrics.verified_core_violation_count,
            metrics.snapshot.busy_ticks,
            metrics.snapshot.current_tick,
        ),
    )
}

pub fn test_nextmind_metrics(
    verified_core_ok: bool,
    violation_count: u64,
) -> SystemPressureMetrics {
    SystemPressureMetrics {
        snapshot: NativeSystemSnapshotRecord {
            current_tick: 100,
            busy_ticks: 80,
            process_count: 3,
            active_process_count: 3,
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
            lag_debt_interactive: 3,
            lag_debt_normal: 0,
            lag_debt_background: 0,
            dispatch_count_latency_critical: 0,
            dispatch_count_interactive: 2,
            dispatch_count_normal: 1,
            dispatch_count_background: 0,
            runtime_ticks_latency_critical: 0,
            runtime_ticks_interactive: 2,
            runtime_ticks_normal: 1,
            runtime_ticks_background: 0,
            scheduler_cpu_count: 2,
            scheduler_running_cpu: 0,
            scheduler_cpu_load_imbalance: 1,
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
            total_socket_rx_limit: 16,
            max_socket_rx_depth: 1,
            total_network_tx_dropped: 0,
            total_network_rx_dropped: 0,
            running_pid: 1,
            reserved0: if verified_core_ok {
                NativeSystemSnapshotRecord::VERIFIED_CORE_OK_TRUE
            } else {
                NativeSystemSnapshotRecord::VERIFIED_CORE_OK_FALSE
            },
            reserved1: violation_count,
        },
        verified_core_ok,
        verified_core_violation_count: violation_count,
        cpu_utilization_pct: 55,
        run_queue_total: 2,
        run_queue_latency_critical: 0,
        run_queue_interactive: 1,
        run_queue_normal: 1,
        run_queue_background: 0,
        run_queue_urgent_latency_critical: 0,
        run_queue_urgent_interactive: 0,
        run_queue_urgent_normal: 0,
        run_queue_urgent_background: 0,
        scheduler_lag_debt_total: 3,
        scheduler_dispatch_total: 3,
        scheduler_runtime_ticks_total: 3,
        scheduler_runtime_imbalance: 2,
        scheduler_cpu_count: 2,
        scheduler_running_cpu: Some(0),
        scheduler_cpu_load_imbalance: 1,
        scheduler_starved: false,
        bus_endpoint_count: 0,
        saturated_bus_endpoint_count: 0,
        bus_queue_depth_total: 0,
        bus_queue_capacity_total: 0,
        bus_pressure_pct: 0,
        bus_overflow_total: 0,
        socket_pressure_pct: 6,
        event_queue_pressure_pct: 1,
        tx_drop_delta: 0,
        rx_drop_delta: 0,
    }
}

fn nextmind_collect_process_entities(facts: &[SystemFact]) -> Vec<ProcessEntity> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Process(process) => Some(process.clone()),
            _ => None,
        })
        .collect()
}

fn nextmind_collect_device_entities(
    facts: &[SystemFact],
) -> Vec<(DeviceHandle, NativeNetworkInterfaceRecord)> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Device(device) => {
                device.record.map(|record| (device.handle.clone(), record))
            }
            _ => None,
        })
        .collect()
}

fn nextmind_collect_bus_endpoints(facts: &[SystemFact]) -> Vec<BusEndpointEntity> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::BusEndpoint(endpoint) => Some(endpoint.clone()),
            _ => None,
        })
        .collect()
}

fn nextmind_protected_process(process: &ProcessEntity) -> bool {
    matches!(
        NativeSchedulerClass::from_raw(process.record.scheduler_class),
        Some(NativeSchedulerClass::LatencyCritical | NativeSchedulerClass::Interactive)
    ) || process.handle.pid == 1
}

fn nextmind_candidate_processes(processes: &[ProcessEntity]) -> Vec<ProcessEntity> {
    let mut candidates = processes
        .iter()
        .filter(|process| {
            !nextmind_protected_process(process) && matches!(process.record.state, 1 | 2)
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .record
            .cpu_runtime_ticks
            .cmp(&left.record.cpu_runtime_ticks)
            .then(
                right
                    .record
                    .scheduler_budget
                    .cmp(&left.record.scheduler_budget),
            )
            .then(left.handle.pid.cmp(&right.handle.pid))
    });
    candidates
}

pub fn nextmind_subscribe_auto_streams<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<Vec<EventStream>, ExitCode> {
    let controller = SystemController::new(runtime);
    let facts = controller.collect_facts().map_err(|_| 265)?;
    let mut streams = Vec::new();
    for process in nextmind_collect_process_entities(&facts) {
        streams.push(
            controller
                .subscribe(EventFilter::Process {
                    pid: process.handle.pid,
                    token: CapabilityToken {
                        value: process.handle.pid,
                    },
                    exited: true,
                    reaped: true,
                    poll_events: POLLPRI,
                })
                .map_err(|_| 265)?,
        );
    }
    for (handle, _) in nextmind_collect_device_entities(&facts) {
        streams.push(
            controller
                .subscribe(EventFilter::Network {
                    interface_path: handle.path,
                    socket_path: None,
                    token: CapabilityToken { value: 1 },
                    link_changed: true,
                    rx_ready: true,
                    tx_drained: true,
                    poll_events: POLLPRI,
                })
                .map_err(|_| 265)?,
        );
    }
    for endpoint in facts.iter().filter_map(|fact| match fact {
        SystemFact::BusEndpoint(endpoint) => Some(endpoint),
        _ => None,
    }) {
        streams.push(
            controller
                .subscribe(EventFilter::Bus {
                    endpoint: endpoint.id,
                    token: CapabilityToken {
                        value: endpoint.id as u64,
                    },
                    attached: true,
                    detached: true,
                    published: true,
                    received: true,
                    poll_events: POLLPRI,
                })
                .map_err(|_| 265)?,
        );
    }
    Ok(streams)
}

pub fn nextmind_explain_last<B: SyscallBackend>(
    runtime: &Runtime<B>,
    adaptive_state: &AdaptiveState,
    context: &SemanticContext,
    last_report: &Option<NextMindDecisionReport>,
) -> Result<(), ExitCode> {
    let Some(report) = last_report else {
        return write_line(runtime, "nextmind.explain last=none");
    };
    let diagnostics = SystemController::new(runtime).semantic_diagnostics(adaptive_state, context);
    write_line(
        runtime,
        &format!(
            "nextmind.explain trigger={} verdict={} thresholds=runq>3,cpu>=75,socket>=80,event>=75 channel={} class={} caps={} tier={:?} mode={:?} budget={} verified-core={} violations={}",
            nextmind_pressure_state_label(report.trigger),
            semantic_verdict_name(report.verdict),
            nextmind_channel_for_metrics(&report.before, report.trigger),
            semantic_class_name(report.semantic.class),
            semantic_capabilities_csv(&report.semantic),
            report.adaptive.tier,
            report.adaptive.compute_mode,
            report.adaptive.budget_points,
            report.before.verified_core_ok,
            report.before.verified_core_violation_count,
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "nextmind.observation cpu={} mem={} anomaly={} thermal={} stress={} focus={}",
            report.observation.cpu_load,
            report.observation.mem_pressure,
            report.observation.anomaly_score,
            report.observation.thermal_c,
            report.adaptive.stress,
            report.adaptive.focus
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "nextmind.diagnostics stress={} focus={} tier={:?} mode={:?} budget={} events={}",
            diagnostics.stress,
            diagnostics.focus,
            diagnostics.tier,
            diagnostics.compute_mode,
            diagnostics.budget_points,
            diagnostics.event_count
        ),
    )?;
    if !diagnostics.context_tail.is_empty() {
        for line in diagnostics.context_tail.lines() {
            write_line(runtime, &format!("nextmind.context {line}"))?;
        }
    }
    nextmind_render_metrics(runtime, "before", &report.before, report.trigger)?;
    nextmind_render_metrics(
        runtime,
        "after",
        &report.after,
        SystemController::new(runtime).classify_pressure(&report.after),
    )?;
    if report.actions.is_empty() {
        write_line(
            runtime,
            "nextmind.action reason=none detail=no-direct-adjustment-required",
        )?;
    } else {
        for action in &report.actions {
            write_line(
                runtime,
                &format!(
                    "nextmind.action reason={} detail={}",
                    action.reason, action.detail
                ),
            )?;
        }
    }
    Ok(())
}

fn nextmind_update_entity_epochs(
    epochs: &mut Vec<SemanticEntityEpoch>,
    entities: &[SemanticEntity],
) -> Vec<(SemanticEntity, u32)> {
    let mut resolved = Vec::new();
    for entity in entities {
        if let Some(entry) = epochs
            .iter_mut()
            .find(|entry| entry.subject == entity.subject)
        {
            if entry.policy_fingerprint != entity.policy.policy_fingerprint {
                entry.policy_fingerprint = entity.policy.policy_fingerprint;
                let next = entry.policy_epoch.wrapping_add(1);
                entry.policy_epoch = if next == 0 { 1 } else { next };
            }
            resolved.push((entity.clone(), entry.policy_epoch));
            continue;
        }
        epochs.push(SemanticEntityEpoch {
            subject: entity.subject.clone(),
            policy_fingerprint: entity.policy.policy_fingerprint,
            policy_epoch: 1,
        });
        resolved.push((entity.clone(), 1));
    }
    resolved
}

fn nextmind_optimize_system<B: SyscallBackend>(
    runtime: &Runtime<B>,
    last_snapshot: &mut Option<NativeSystemSnapshotRecord>,
    adaptive_state: &mut AdaptiveState,
) -> Result<NextMindDecisionReport, ExitCode> {
    let controller = SystemController::new(runtime);
    let plan = controller
        .plan_pressure_response(last_snapshot.as_ref(), adaptive_state)
        .map_err(|_| 266)?;
    let before = plan.before.clone();
    let trigger = plan.trigger;
    let facts = controller.collect_facts().map_err(|_| 266)?;
    let processes = nextmind_collect_process_entities(&facts);
    let devices = nextmind_collect_device_entities(&facts);
    let bus_endpoints = nextmind_collect_bus_endpoints(&facts);
    let mut actions = plan.actions.clone();
    let mut original_net_admin = Vec::<(DeviceHandle, NativeNetworkInterfaceRecord)>::new();

    if matches!(
        trigger,
        PressureState::HighSchedulerPressure | PressureState::MixedPressure
    ) {
        for process in nextmind_candidate_processes(&processes).into_iter().take(2) {
            if process.record.scheduler_class == NativeSchedulerClass::Background as u32
                && process.record.scheduler_budget <= 1
            {
                continue;
            }
            let reason = String::from("scheduler-pressure");
            if !actions.iter().any(|action| {
                action.reason == reason
                    && action
                        .detail
                        .contains(&format!("pid={}", process.handle.pid))
            }) {
                continue;
            }
            controller
                .act_on_process(
                    process.handle,
                    ProcessAction::Renice {
                        class: NativeSchedulerClass::Background,
                        budget: 1,
                    },
                )
                .map_err(|_| 266)?;
        }
    }

    if matches!(
        trigger,
        PressureState::NetworkBackpressure | PressureState::MixedPressure
    ) {
        for (handle, record) in devices {
            let socket_pressure = if before.snapshot.total_socket_rx_limit == 0 {
                0
            } else {
                before.socket_pressure_pct
            };
            if record.rx_dropped == 0
                && record.tx_dropped == 0
                && socket_pressure < 80
                && record.tx_inflight_depth < record.tx_inflight_limit
            {
                continue;
            }
            if !actions.iter().any(|action| {
                action.reason == "network-backpressure"
                    && action.detail.contains(&format!("iface={}", handle.path))
            }) {
                continue;
            }
            original_net_admin.push((handle.clone(), record));
            let new_tx_capacity = (record.tx_capacity as usize)
                .saturating_add((record.tx_capacity as usize / 2).max(1));
            let new_rx_capacity = (record.rx_capacity as usize)
                .saturating_add((record.rx_capacity as usize / 2).max(1));
            let new_tx_inflight_limit = (record.tx_inflight_limit as usize)
                .saturating_add((record.tx_inflight_limit as usize / 2).max(1))
                .min(new_tx_capacity.max(1));
            controller
                .configure_interface_admin(
                    &handle,
                    record.mtu as usize,
                    new_tx_capacity,
                    new_rx_capacity,
                    new_tx_inflight_limit,
                    record.admin_up != 0,
                    false,
                )
                .map_err(|_| 266)?;
        }
    }

    if matches!(
        trigger,
        PressureState::NetworkBackpressure | PressureState::MixedPressure
    ) {
        for endpoint in bus_endpoints {
            if endpoint.record.queue_depth == 0 || endpoint.record.last_peer == 0 {
                continue;
            }
            if !actions.iter().any(|action| {
                action.reason == "bus-backpressure"
                    && action.detail.contains(&format!("endpoint={}", endpoint.id))
            }) {
                continue;
            }
            let mut buffer = [0u8; 256];
            let drained = runtime
                .receive_bus_message(endpoint.record.last_peer as usize, endpoint.id, &mut buffer)
                .map_err(|_| 266)?;
            actions.push(SemanticActionRecord {
                reason: String::from("bus-drain"),
                detail: format!(
                    "endpoint={} peer={} drained-bytes={} remaining-queue>=0",
                    endpoint.id, endpoint.record.last_peer, drained
                ),
            });
        }
    }

    let mut after = controller
        .observe_pressure(Some(&before.snapshot))
        .map_err(|_| 266)?;
    let mut verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
        core::cmp::Ordering::Less => SemanticVerdict::Improved,
        core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
        core::cmp::Ordering::Greater => SemanticVerdict::Worse,
    };

    if !matches!(verdict, SemanticVerdict::Improved)
        && matches!(
            trigger,
            PressureState::MixedPressure | PressureState::HighSchedulerPressure
        )
        && let Some(process) = nextmind_candidate_processes(&processes).into_iter().next()
    {
        if !actions
            .iter()
            .any(|action| action.reason == "fallback-throttle")
        {
            actions.push(SemanticActionRecord {
                reason: String::from("fallback-throttle"),
                detail: format!(
                    "pause pid={} name={} cpu_ticks={}",
                    process.handle.pid, process.name, process.record.cpu_runtime_ticks
                ),
            });
        }
        controller
            .act_on_process(process.handle, ProcessAction::Pause)
            .map_err(|_| 266)?;
        after = controller
            .observe_pressure(Some(&before.snapshot))
            .map_err(|_| 266)?;
        verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
            core::cmp::Ordering::Less => SemanticVerdict::Improved,
            core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
            core::cmp::Ordering::Greater => SemanticVerdict::Worse,
        };
        if matches!(verdict, SemanticVerdict::Worse) {
            controller
                .act_on_process(process.handle, ProcessAction::Resume)
                .map_err(|_| 266)?;
            actions.push(SemanticActionRecord {
                reason: String::from("rollback"),
                detail: format!("resume pid={} after worse outcome", process.handle.pid),
            });
            after = controller
                .observe_pressure(Some(&before.snapshot))
                .map_err(|_| 266)?;
            verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
                core::cmp::Ordering::Less => SemanticVerdict::Improved,
                core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
                core::cmp::Ordering::Greater => SemanticVerdict::Worse,
            };
        }
    }

    if matches!(verdict, SemanticVerdict::Worse) {
        for (handle, record) in original_net_admin {
            controller
                .configure_interface_admin(
                    &handle,
                    record.mtu as usize,
                    record.tx_capacity as usize,
                    record.rx_capacity as usize,
                    record.tx_inflight_limit as usize,
                    record.admin_up != 0,
                    record.promiscuous != 0,
                )
                .map_err(|_| 266)?;
            actions.push(SemanticActionRecord {
                reason: String::from("rollback"),
                detail: format!("restore iface={} admin profile", handle.path),
            });
        }
        after = controller
            .observe_pressure(Some(&before.snapshot))
            .map_err(|_| 266)?;
        verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
            core::cmp::Ordering::Less => SemanticVerdict::Improved,
            core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
            core::cmp::Ordering::Greater => SemanticVerdict::Worse,
        };
    }

    *last_snapshot = Some(after.snapshot);
    Ok(NextMindDecisionReport {
        trigger,
        before,
        after,
        semantic: plan.semantic,
        observation: plan.observation,
        adaptive: plan.adaptive,
        actions,
        verdict,
    })
}

pub fn nextmind_drain_auto_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    auto_state: &NextMindAutoState,
    last_snapshot: &mut Option<NativeSystemSnapshotRecord>,
    adaptive_state: &mut AdaptiveState,
    last_report: &mut Option<NextMindDecisionReport>,
) -> Result<(), ExitCode> {
    if nextmind_auto_triggered(runtime, auto_state)? {
        let report = nextmind_optimize_system(runtime, last_snapshot, adaptive_state)?;
        *last_report = Some(report.clone());
        write_line(runtime, &nextmind_auto_summary(&report))?;
    }
    Ok(())
}

pub fn nextmind_auto_triggered<B: SyscallBackend>(
    runtime: &Runtime<B>,
    auto_state: &NextMindAutoState,
) -> Result<bool, ExitCode> {
    if !auto_state.enabled {
        return Ok(false);
    }
    let mut triggered = false;
    for stream in &auto_state.streams {
        let ready = runtime
            .poll(stream.queue_fd, POLLIN | POLLPRI)
            .map_err(|_| 267)?;
        if ready == 0 {
            continue;
        }
        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 8];
        let count = match runtime.wait_event_queue(stream.queue_fd, &mut events) {
            Ok(count) => count,
            Err(Errno::Again) => continue,
            Err(_) => return Err(267),
        };
        if count != 0 {
            triggered = true;
        }
    }
    Ok(triggered)
}

pub fn try_handle_nextmind_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    state: &mut NextMindAgentState<'_>,
) -> Option<Result<(), ExitCode>> {
    if line == "nextmind.observe" {
        let controller = SystemController::new(runtime);
        let semantic_state = match controller
            .observe_semantic_state(state.last_snapshot.as_ref(), state.adaptive_state)
        {
            Ok(semantic_state) => semantic_state,
            Err(_) => return Some(Err(266)),
        };
        *state.last_snapshot = Some(semantic_state.metrics.snapshot);
        if nextmind_render_metrics(
            runtime,
            "current",
            &semantic_state.metrics,
            semantic_state.pressure,
        )
        .is_err()
        {
            return Some(Err(266));
        }
        if write_line(
            runtime,
            &format!(
                "nextmind.semantic channel={} class={} caps={} tier={:?} mode={:?} budget={} stress={} focus={} obs={}/{}/{}/{} verified-core={} violations={}",
                semantic_state.channel,
                semantic_class_name(semantic_state.semantic.class),
                semantic_capabilities_csv(&semantic_state.semantic),
                semantic_state.adaptive.tier,
                semantic_state.adaptive.compute_mode,
                semantic_state.adaptive.budget_points,
                semantic_state.adaptive.stress,
                semantic_state.adaptive.focus,
                semantic_state.observation.cpu_load,
                semantic_state.observation.mem_pressure,
                semantic_state.observation.anomaly_score,
                semantic_state.observation.thermal_c,
                semantic_state.metrics.verified_core_ok,
                semantic_state.metrics.verified_core_violation_count,
            ),
        )
        .is_err()
        {
            return Some(Err(266));
        }
        state.context.push(
            &semantic_state.channel,
            &semantic_state.semantic,
            &format!(
                "pressure={} runq={} urgent={} lag={} imbalance={} starved={} cpu={} socket={} event={} bus-pressure={} bus-depth={} bus-overflows={} verified-core={} violations={}",
                nextmind_pressure_state_label(semantic_state.pressure),
                semantic_state.metrics.run_queue_total,
                semantic_state.metrics.run_queue_urgent_total(),
                semantic_state.metrics.scheduler_lag_debt_total,
                semantic_state.metrics.scheduler_runtime_imbalance,
                semantic_state.metrics.scheduler_starved,
                semantic_state.metrics.cpu_utilization_pct,
                semantic_state.metrics.socket_pressure_pct,
                semantic_state.metrics.event_queue_pressure_pct,
                semantic_state.metrics.bus_pressure_pct,
                semantic_state.metrics.bus_queue_depth_total,
                semantic_state.metrics.bus_overflow_total,
                semantic_state.metrics.verified_core_ok,
                semantic_state.metrics.verified_core_violation_count
            ),
            &[],
        );
        let entities = match controller.collect_semantic_entities() {
            Ok(entities) => entities,
            Err(_) => return Some(Err(266)),
        };
        for (entity, epoch) in nextmind_update_entity_epochs(state.entity_epochs, &entities) {
            if write_line(
                runtime,
                &format!(
                    "nextmind.entity kind={} subject={} class={} caps={} cpu-mask=0x{:x} policy-epoch={}",
                    semantic_entity_kind_name(entity.kind),
                    entity.subject,
                    semantic_class_name(entity.semantic.class),
                    semantic_capabilities_csv(&entity.semantic),
                    entity.policy.cpu_mask,
                    epoch,
                ),
            )
            .is_err()
            {
                return Some(Err(266));
            }
        }
        let topology = match controller.observe_topology(state.last_snapshot.as_ref()) {
            Ok(topology) => topology,
            Err(_) => return Some(Err(266)),
        };
        let loads = topology
            .entries
            .iter()
            .map(|entry| entry.load)
            .collect::<Vec<_>>();
        let selected_cpu = select_cpu(&loads, topology.online_cpus, &[]).unwrap_or(0);
        for entry in &topology.entries {
            if write_line(
                runtime,
                &format!(
                    "nextmind.cpu cpu={} apic={} online={} launched={} load={} mask=0x{:x} selected={}",
                    entry.cpu_index,
                    entry.apic_id,
                    entry.online,
                    entry.launched,
                    load_percent(&entry.load),
                    cpu_mask_for(entry.cpu_index),
                    entry.cpu_index == selected_cpu,
                ),
            )
            .is_err()
            {
                return Some(Err(266));
            }
        }
        return Some(Ok(()));
    }
    if line == "nextmind.optimize" {
        match nextmind_optimize_system(runtime, state.last_snapshot, state.adaptive_state) {
            Ok(report) => {
                if nextmind_render_metrics(runtime, "before", &report.before, report.trigger)
                    .is_err()
                    || nextmind_render_metrics(
                        runtime,
                        "after",
                        &report.after,
                        SystemController::new(runtime).classify_pressure(&report.after),
                    )
                    .is_err()
                {
                    return Some(Err(266));
                }
                if write_line(
                    runtime,
                    &format!(
                        "nextmind.semantic channel={} class={} caps={} tier={:?} mode={:?} budget={} stress={} focus={} obs={}/{}/{}/{} verified-core={} violations={}",
                        nextmind_channel_for_metrics(&report.before, report.trigger),
                        semantic_class_name(report.semantic.class),
                        semantic_capabilities_csv(&report.semantic),
                        report.adaptive.tier,
                        report.adaptive.compute_mode,
                        report.adaptive.budget_points,
                        report.adaptive.stress,
                        report.adaptive.focus,
                        report.observation.cpu_load,
                        report.observation.mem_pressure,
                        report.observation.anomaly_score,
                        report.observation.thermal_c,
                        report.before.verified_core_ok,
                        report.before.verified_core_violation_count,
                    ),
                )
                .is_err()
                {
                    return Some(Err(266));
                }
                if report.actions.is_empty() {
                    if write_line(
                        runtime,
                        "nextmind.action reason=none detail=no-direct-adjustment-required",
                    )
                    .is_err()
                    {
                        return Some(Err(266));
                    }
                } else {
                    for action in &report.actions {
                        if write_line(
                            runtime,
                            &format!(
                                "nextmind.action reason={} detail={}",
                                action.reason, action.detail
                            ),
                        )
                        .is_err()
                        {
                            return Some(Err(266));
                        }
                    }
                }
                if write_line(
                    runtime,
                    &format!("nextmind.verdict={}", semantic_verdict_name(report.verdict)),
                )
                .is_err()
                {
                    return Some(Err(266));
                }
                state.context.push(
                    &nextmind_channel_for_metrics(&report.before, report.trigger),
                    &report.semantic,
                    &format!(
                        "verdict={} before-runq={} after-runq={} before-urgent={} after-urgent={} before-lag={} after-lag={} before-imbalance={} after-imbalance={} before-bus-pressure={} after-bus-pressure={} before-bus-depth={} after-bus-depth={} before-bus-overflows={} after-bus-overflows={} starved={} verified-core={} violations={}",
                        semantic_verdict_name(report.verdict),
                        report.before.run_queue_total,
                        report.after.run_queue_total,
                        report.before.run_queue_urgent_total(),
                        report.after.run_queue_urgent_total(),
                        report.before.scheduler_lag_debt_total,
                        report.after.scheduler_lag_debt_total,
                        report.before.scheduler_runtime_imbalance,
                        report.after.scheduler_runtime_imbalance,
                        report.before.bus_pressure_pct,
                        report.after.bus_pressure_pct,
                        report.before.bus_queue_depth_total,
                        report.after.bus_queue_depth_total,
                        report.before.bus_overflow_total,
                        report.after.bus_overflow_total,
                        report.before.scheduler_starved || report.after.scheduler_starved,
                        report.before.verified_core_ok,
                        report.before.verified_core_violation_count
                    ),
                    &report.actions,
                );
                *state.last_report = Some(report);
                *state.last_status = 0;
            }
            Err(code) => {
                *state.last_status = code;
            }
        }
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("nextmind.auto ") {
        match rest.trim() {
            "on" => match nextmind_subscribe_auto_streams(runtime) {
                Ok(streams) => {
                    state.auto_state.enabled = true;
                    state.auto_state.streams = streams;
                    if write_line(
                        runtime,
                        &format!(
                            "nextmind.auto=on streams={}",
                            state.auto_state.streams.len()
                        ),
                    )
                    .is_err()
                    {
                        return Some(Err(195));
                    }
                    *state.last_status = 0;
                }
                Err(code) => *state.last_status = code,
            },
            "off" => {
                state.auto_state.enabled = false;
                state.auto_state.streams.clear();
                if write_line(runtime, "nextmind.auto=off").is_err() {
                    return Some(Err(195));
                }
                *state.last_status = 0;
            }
            _ => {
                *state.last_status = 2;
                if write_line(runtime, "usage: nextmind.auto <on|off>").is_err() {
                    return Some(Err(199));
                }
            }
        }
        return Some(Ok(()));
    }
    if line == "nextmind.explain last" {
        *state.last_status = match nextmind_explain_last(
            runtime,
            state.adaptive_state,
            state.context,
            state.last_report,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    None
}
