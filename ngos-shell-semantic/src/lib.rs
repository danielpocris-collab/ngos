//! Canonical subsystem role:
//! - subsystem: native semantic action control
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing semantic actions over canonical process
//!   and runtime signals
//!
//! Canonical contract families handled here:
//! - semantic action command contracts
//! - intent routing contracts
//! - semantic feedback learning contracts
//!
//! This module may trigger semantic actions and record userland learning, but
//! it must not redefine kernel truth, verified-core truth, or scheduler truth.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::String;

use ngos_shell_proc::{native_process_state_label, scheduler_class_label};
use ngos_shell_types::{ShellMode, parse_u64_arg, parse_usize_arg, resolve_shell_path};
use ngos_user_abi::{
    ExitCode, NativeEventRecord, NativeResourceState, NativeSchedulerClass, POLLPRI, SyscallBackend,
};
use ngos_user_runtime::{
    Runtime,
    system_control::{
        CapabilityToken, DeviceHandle, EventFilter, ProcessAction, ProcessHandle, ResourceContract,
        ResourceUpdate, SemanticFeedbackStore, SystemController, event_source_name,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEntityEpoch {
    pub subject: String,
    pub policy_fingerprint: u64,
    pub policy_epoch: u32,
}

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

fn parse_scheduler_class(token: &str) -> Option<NativeSchedulerClass> {
    match token {
        "latency-critical" | "critical" => Some(NativeSchedulerClass::LatencyCritical),
        "interactive" => Some(NativeSchedulerClass::Interactive),
        "best-effort" | "besteffort" => Some(NativeSchedulerClass::BestEffort),
        "background" => Some(NativeSchedulerClass::Background),
        _ => None,
    }
}

fn parse_cpu_mask_arg(text: Option<&str>) -> Option<u64> {
    let value = text?.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else if let Some(bits) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        u64::from_str_radix(bits, 2).ok()
    } else {
        value.parse::<u64>().ok()
    }
}

fn resource_state_name(raw: u32) -> &'static str {
    match NativeResourceState::from_raw(raw) {
        Some(NativeResourceState::Active) => "active",
        Some(NativeResourceState::Suspended) => "suspended",
        Some(NativeResourceState::Retired) => "retired",
        None => "unknown",
    }
}

pub fn shell_semantic_watch_event<B: SyscallBackend>(
    runtime: &Runtime<B>,
    filter: EventFilter,
) -> Result<usize, ExitCode> {
    let controller = SystemController::new(runtime);
    controller
        .subscribe(filter)
        .map(|stream| stream.queue_fd)
        .map_err(|_| 261)
}

pub fn shell_semantic_wait_event<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
) -> Result<(), ExitCode> {
    let mut events = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 16];
    let count = runtime
        .wait_event_queue(queue_fd, &mut events)
        .map_err(|_| 262)?;
    for event in events.into_iter().take(count) {
        write_line(
            runtime,
            &format!(
                "semantic-event queue={} token={} source={} arg0={} arg1={} arg2={} detail0={} detail1={}",
                queue_fd,
                event.token,
                event_source_name(&event),
                event.source_arg0,
                event.source_arg1,
                event.source_arg2,
                event.detail0,
                event.detail1
            ),
        )?;
    }
    Ok(())
}

pub fn shell_semantic_process_action<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    action: ProcessAction,
) -> Result<(), ExitCode> {
    let controller = SystemController::new(runtime);
    let affinity_suffix = match action {
        ProcessAction::SetAffinity { cpu_mask } => format!(" cpu-mask=0x{cpu_mask:x}"),
        _ => String::new(),
    };
    controller
        .act_on_process(ProcessHandle { pid }, action)
        .map_err(|_| 263)?;
    let record = runtime.inspect_process(pid).map_err(|_| 263)?;
    write_line(
        runtime,
        &format!(
            "process-control pid={} state={} class={} budget={}{}",
            pid,
            native_process_state_label(record.state),
            scheduler_class_label(record.scheduler_class),
            record.scheduler_budget,
            affinity_suffix
        ),
    )
}

pub fn shell_semantic_resource_update<B: SyscallBackend>(
    runtime: &Runtime<B>,
    contract: usize,
    action: ResourceUpdate,
) -> Result<(), ExitCode> {
    let controller = SystemController::new(runtime);
    controller
        .update_resource(ResourceContract { id: contract }, action)
        .map_err(|_| 264)?;
    let contract_record = runtime.inspect_contract(contract).map_err(|_| 264)?;
    let resource = runtime
        .inspect_resource(contract_record.resource as usize)
        .map_err(|_| 264)?;
    write_line(
        runtime,
        &format!(
            "resource-control contract={} resource={} state={}",
            contract,
            contract_record.resource,
            resource_state_name(resource.state)
        ),
    )
}

pub fn shell_record_learning(
    learning: &mut SemanticFeedbackStore,
    epochs: &[SemanticEntityEpoch],
    subject: &str,
    action: &str,
    success: bool,
) {
    let policy_epoch = epochs
        .iter()
        .find(|entry| entry.subject == subject)
        .map(|entry| entry.policy_epoch)
        .unwrap_or(1);
    learning.record(subject, action, policy_epoch, success);
}

pub fn try_handle_semantic_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
    last_status: &mut i32,
    semantic_learning: &mut SemanticFeedbackStore,
    nextmind_entity_epochs: &[SemanticEntityEpoch],
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("pause ") {
        let pid = match parse_u64_arg(Some(rest.trim())) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: pause <pid>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_process_action(runtime, pid, ProcessAction::Pause) {
            Ok(()) => 0,
            Err(code) => code,
        };
        shell_record_learning(
            semantic_learning,
            nextmind_entity_epochs,
            &format!("process:{pid}"),
            "pause",
            *last_status == 0,
        );
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("resume ") {
        let pid = match parse_u64_arg(Some(rest.trim())) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: resume <pid>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_process_action(runtime, pid, ProcessAction::Resume) {
            Ok(()) => 0,
            Err(code) => code,
        };
        shell_record_learning(
            semantic_learning,
            nextmind_entity_epochs,
            &format!("process:{pid}"),
            "resume",
            *last_status == 0,
        );
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("renice ") {
        let mut parts = rest.split_whitespace();
        let pid = match parse_u64_arg(parts.next()) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: renice <pid> <class> <budget>");
                return Some(Err(2));
            }
        };
        let class = match parts.next().and_then(parse_scheduler_class) {
            Some(class) => class,
            None => {
                let _ = write_line(runtime, "usage: renice <pid> <class> <budget>");
                return Some(Err(2));
            }
        };
        let budget = match parts.next().and_then(|value| value.parse::<u32>().ok()) {
            Some(budget) if budget > 0 => budget,
            _ => {
                let _ = write_line(runtime, "usage: renice <pid> <class> <budget>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_process_action(
            runtime,
            pid,
            ProcessAction::Renice { class, budget },
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        shell_record_learning(
            semantic_learning,
            nextmind_entity_epochs,
            &format!("process:{pid}"),
            "renice",
            *last_status == 0,
        );
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("affinity ") {
        let mut parts = rest.split_whitespace();
        let pid = match parse_u64_arg(parts.next()) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: affinity <pid> <cpu-mask>");
                return Some(Err(2));
            }
        };
        let cpu_mask = match parse_cpu_mask_arg(parts.next()) {
            Some(mask) => mask,
            None => {
                let _ = write_line(runtime, "usage: affinity <pid> <cpu-mask>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_process_action(
            runtime,
            pid,
            ProcessAction::SetAffinity { cpu_mask },
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        shell_record_learning(
            semantic_learning,
            nextmind_entity_epochs,
            &format!("process:{pid}"),
            "affinity",
            *last_status == 0,
        );
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("semantic-watch ") {
        let mut parts = rest.split_whitespace();
        let target = parts.next().unwrap_or("");
        let result = match target {
            "process" => match parse_u64_arg(parts.next()) {
                Some(pid) => shell_semantic_watch_event(
                    runtime,
                    EventFilter::Process {
                        pid,
                        token: CapabilityToken { value: pid },
                        exited: true,
                        reaped: true,
                        poll_events: POLLPRI,
                    },
                ),
                None => Err(261),
            },
            "resource" => match parse_usize_arg(parts.next()) {
                Some(resource) => shell_semantic_watch_event(
                    runtime,
                    EventFilter::Resource {
                        resource,
                        token: CapabilityToken {
                            value: resource as u64,
                        },
                        claimed: true,
                        queued: true,
                        canceled: true,
                        released: true,
                        handed_off: true,
                        revoked: true,
                        poll_events: POLLPRI,
                    },
                ),
                None => Err(261),
            },
            "network" => match parts.next() {
                Some(path) => shell_semantic_watch_event(
                    runtime,
                    EventFilter::Network {
                        interface_path: resolve_shell_path(cwd, path),
                        socket_path: None,
                        token: CapabilityToken { value: 1 },
                        link_changed: true,
                        rx_ready: true,
                        tx_drained: true,
                        poll_events: POLLPRI,
                    },
                ),
                None => Err(261),
            },
            _ => Err(261),
        };
        match result {
            Ok(queue_fd) => {
                *last_status = 0;
                return Some(
                    write_line(
                        runtime,
                        &format!("semantic-watch fd={queue_fd} target={target}"),
                    )
                    .map_err(|_| 199),
                );
            }
            Err(code) => {
                *last_status = code;
                return Some(Ok(()));
            }
        }
    }
    if let Some(rest) = line.strip_prefix("semantic-wait ") {
        let queue_fd = match parse_usize_arg(Some(rest.trim())) {
            Some(queue_fd) => queue_fd,
            None => {
                let _ = write_line(runtime, "usage: semantic-wait <queue-fd>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_wait_event(runtime, queue_fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    None
}

pub fn try_handle_intent_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    shell_mode: ShellMode,
    line: &str,
    last_status: &mut i32,
    semantic_learning: &mut SemanticFeedbackStore,
    nextmind_entity_epochs: &[SemanticEntityEpoch],
) -> Option<Result<(), ExitCode>> {
    let rest = line.strip_prefix("intent ")?;
    let mut parts = rest.split_whitespace();
    let kind = parts.next().unwrap_or("");
    let subject = parts.next().unwrap_or("");
    match (kind, subject) {
        ("optimize", "process") => {
            let Some(pid) = parse_u64_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent optimize process <pid>");
                return Some(Err(2));
            };
            *last_status = match shell_semantic_process_action(
                runtime,
                pid,
                ProcessAction::Renice {
                    class: NativeSchedulerClass::LatencyCritical,
                    budget: 4,
                },
            ) {
                Ok(()) => 0,
                Err(code) => code,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("process:{pid}"),
                "optimize",
                *last_status == 0,
            );
        }
        ("throttle", "process") => {
            let Some(pid) = parse_u64_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent throttle process <pid>");
                return Some(Err(2));
            };
            *last_status = match shell_semantic_process_action(
                runtime,
                pid,
                ProcessAction::Renice {
                    class: NativeSchedulerClass::Background,
                    budget: 1,
                },
            ) {
                Ok(()) => 0,
                Err(code) => code,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("process:{pid}"),
                "throttle",
                *last_status == 0,
            );
        }
        ("restart", "process") => {
            let Some(pid) = parse_u64_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent restart process <pid>");
                return Some(Err(2));
            };
            *last_status = match shell_semantic_process_action(
                runtime,
                pid,
                ProcessAction::Kill { signal: 9 },
            ) {
                Ok(()) => 0,
                Err(code) => code,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("process:{pid}"),
                "restart",
                *last_status == 0,
            );
        }
        ("activate", "resource") => {
            let Some(contract) = parse_usize_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent activate resource <contract>");
                return Some(Err(2));
            };
            *last_status =
                match shell_semantic_resource_update(runtime, contract, ResourceUpdate::Activate) {
                    Ok(()) => 0,
                    Err(code) => code,
                };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("resource:{contract}"),
                "activate",
                *last_status == 0,
            );
        }
        ("stabilize", "network") => {
            let Some(path) = parts.next() else {
                let _ = write_line(runtime, "usage: intent stabilize network <device>");
                return Some(Err(2));
            };
            let device = DeviceHandle {
                path: resolve_shell_path(cwd, path),
            };
            let controller = SystemController::new(runtime);
            *last_status = match controller.device_stats(&device) {
                Ok(stats) => match stats.record {
                    Some(record) => match controller.configure_interface_admin(
                        &device,
                        record.mtu as usize,
                        record.tx_capacity.max(record.tx_inflight_limit + 1) as usize,
                        record.rx_capacity as usize,
                        record.tx_inflight_limit.max(2) as usize,
                        true,
                        false,
                    ) {
                        Ok(()) => 0,
                        Err(_) => 265,
                    },
                    None => 265,
                },
                Err(_) => 265,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("device:{}", device.path),
                "stabilize",
                *last_status == 0,
            );
        }
        _ if matches!(shell_mode, ShellMode::Semantic) => {
            let _ = write_line(
                runtime,
                "usage: intent <optimize|throttle|restart|activate|stabilize> <process|resource|network> ...",
            );
            return Some(Err(2));
        }
        _ => {}
    }
    Some(Ok(()))
}
