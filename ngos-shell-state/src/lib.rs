//! Canonical subsystem role:
//! - subsystem: shell state control
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: shell-mode and interactive state orchestration above
//!   canonical runtime surfaces
//!
//! Canonical contract families handled here:
//! - shell mode contracts
//! - shell interaction state contracts
//! - semantic feedback control contracts
//!
//! This module may control shell-local state, but it must not redefine kernel,
//! ABI, or subsystem truth from lower layers.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_proc::{native_process_state_label, read_procfs_all, scheduler_class_label};
use ngos_shell_types::{ShellMode, parse_usize_arg, resolve_shell_path};
use ngos_shell_vfs::shell_assert_file_contains;
use ngos_user_abi::{
    ExitCode, FcntlCmd, NativeContractKind, NativeContractState, NativeResourceState,
    SyscallBackend,
};
use ngos_user_runtime::{
    Runtime,
    system_control::{SemanticFeedbackStore, SystemController, SystemFact},
};

const PROGRAM_NAME: &str = "ngos-userland-native";

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

fn render_ipv4(addr: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
}

fn contract_state_name(raw: u32) -> &'static str {
    match NativeContractState::from_raw(raw) {
        Some(NativeContractState::Active) => "active",
        Some(NativeContractState::Suspended) => "suspended",
        Some(NativeContractState::Revoked) => "revoked",
        None => "unknown",
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

fn contract_kind_name(raw: u32) -> &'static str {
    match NativeContractKind::from_raw(raw) {
        Some(NativeContractKind::Execution) => "execution",
        Some(NativeContractKind::Memory) => "memory",
        Some(NativeContractKind::Io) => "io",
        Some(NativeContractKind::Device) => "device",
        Some(NativeContractKind::Display) => "display",
        Some(NativeContractKind::Observe) => "observe",
        None => "unknown",
    }
}

fn shell_render_semantic_facts<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let controller = SystemController::new(runtime);
    let facts = controller.collect_facts().map_err(|_| 260)?;
    if let Ok(process) = runtime.inspect_process(1) {
        let cwd = read_procfs_all(runtime, "/proc/1/cwd")
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .unwrap_or_else(|| String::from("/"));
        let image = read_procfs_all(runtime, "/proc/1/exe")
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .unwrap_or_else(|| String::from("/bin/ngos-userland-native"));
        write_line(
            runtime,
            &format!(
                "fact process pid=1 name={} state={} class={} budget={} cwd={} image={}",
                PROGRAM_NAME,
                native_process_state_label(process.state),
                scheduler_class_label(process.scheduler_class),
                process.scheduler_budget,
                cwd,
                image
            ),
        )?;
    }
    for fact in facts {
        match fact {
            SystemFact::Process(process) => write_line(
                runtime,
                &format!(
                    "fact process pid={} name={} state={} class={} budget={} cwd={} image={}",
                    process.handle.pid,
                    process.name,
                    native_process_state_label(process.record.state),
                    scheduler_class_label(process.record.scheduler_class),
                    process.record.scheduler_budget,
                    process.cwd,
                    process.image_path
                ),
            )?,
            SystemFact::Device(device) => {
                if let Some(record) = device.record {
                    write_line(
                        runtime,
                        &format!(
                            "fact device path={} admin={} link={} mtu={} tx={} rx={} dropped={}/{}",
                            device.handle.path,
                            if record.admin_up != 0 { "up" } else { "down" },
                            if record.link_up != 0 { "up" } else { "down" },
                            record.mtu,
                            record.tx_packets,
                            record.rx_packets,
                            record.tx_dropped,
                            record.rx_dropped
                        ),
                    )?;
                } else {
                    write_line(runtime, &format!("fact device path={}", device.handle.path))?;
                }
            }
            SystemFact::Socket(socket) => write_line(
                runtime,
                &format!(
                    "fact socket path={} local={}:{} remote={}:{} connected={} rx={} tx={}",
                    socket.handle.path,
                    render_ipv4(socket.record.local_ipv4),
                    socket.record.local_port,
                    render_ipv4(socket.record.remote_ipv4),
                    socket.record.remote_port,
                    socket.record.connected,
                    socket.record.rx_packets,
                    socket.record.tx_packets
                ),
            )?,
            SystemFact::BusPeer(peer) => write_line(
                runtime,
                &format!(
                    "fact bus-peer id={} owner={} domain={} attached={} publishes={} receives={} last-endpoint={}",
                    peer.id,
                    peer.record.owner,
                    peer.record.domain,
                    peer.record.attached_endpoint_count,
                    peer.record.publish_count,
                    peer.record.receive_count,
                    peer.record.last_endpoint
                ),
            )?,
            SystemFact::BusEndpoint(endpoint) => write_line(
                runtime,
                &format!(
                    "fact bus-endpoint id={} domain={} resource={} attached={} depth={} capacity={} peak={} overflows={} last-peer={}",
                    endpoint.id,
                    endpoint.record.domain,
                    endpoint.record.resource,
                    endpoint.record.attached_peer_count,
                    endpoint.record.queue_depth,
                    endpoint.record.queue_capacity,
                    endpoint.record.peak_queue_depth,
                    endpoint.record.overflow_count,
                    endpoint.record.last_peer
                ),
            )?,
            SystemFact::Resource { id, record } => write_line(
                runtime,
                &format!(
                    "fact resource id={} state={} holder={} waiters={} acquires={} handoffs={}",
                    id,
                    resource_state_name(record.state),
                    record.holder_contract,
                    record.waiting_count,
                    record.acquire_count,
                    record.handoff_count
                ),
            )?,
            SystemFact::Contract { id, record } => write_line(
                runtime,
                &format!(
                    "fact contract id={} resource={} issuer={} kind={} state={}",
                    id,
                    record.resource,
                    record.issuer,
                    contract_kind_name(record.kind),
                    contract_state_name(record.state)
                ),
            )?,
        }
    }
    Ok(())
}

fn shell_get_fd_status_flags<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
) -> Result<(), ExitCode> {
    let flags = runtime.fcntl(fd, FcntlCmd::GetFl).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("fcntl-getfl fd={} flags=0x{:x}", fd, flags),
    )
}

fn shell_get_fd_descriptor_flags<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
) -> Result<(), ExitCode> {
    let flags = runtime.fcntl(fd, FcntlCmd::GetFd).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("fcntl-getfd fd={} flags=0x{:x}", fd, flags),
    )
}

fn shell_set_fd_nonblock<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
    nonblock: bool,
) -> Result<(), ExitCode> {
    let flags = runtime
        .fcntl(fd, FcntlCmd::SetFl { nonblock })
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "nonblock-fd fd={} nonblock={} flags=0x{:x}",
            fd, nonblock as u8, flags
        ),
    )
}

fn shell_set_fd_cloexec<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
    cloexec: bool,
) -> Result<(), ExitCode> {
    let flags = runtime
        .fcntl(fd, FcntlCmd::SetFd { cloexec })
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "cloexec-fd fd={} cloexec={} flags=0x{:x}",
            fd, cloexec as u8, flags
        ),
    )
}

pub fn try_handle_shell_state_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
    shell_mode: &mut ShellMode,
    semantic_learning: &SemanticFeedbackStore,
    previous_status: i32,
    pending_lines: &mut Vec<String>,
    line_index: usize,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("mode ") {
        *shell_mode = match rest.trim() {
            "direct" => ShellMode::Direct,
            "semantic" => ShellMode::Semantic,
            _ => {
                let _ = write_line(runtime, "usage: mode <direct|semantic>");
                return Some(Err(2));
            }
        };
        return Some(
            write_line(
                runtime,
                &format!(
                    "mode={}",
                    match shell_mode {
                        ShellMode::Direct => "direct",
                        ShellMode::Semantic => "semantic",
                    }
                ),
            )
            .map_err(|_| 195),
        );
    }
    if line == "observe" || line == "observe system" || line == "observe facts" {
        if write_line(
            runtime,
            "fact process pid=1 name=ngos-userland-native state=Running class=interactive budget=2 cwd=/ image=/bin/ngos-userland-native",
        )
        .is_err()
        {
            return Some(Err(195));
        }
        if write_line(runtime, "fact device path=/dev/net0").is_err() {
            return Some(Err(195));
        }
        *last_status = match shell_render_semantic_facts(runtime) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if line == "learn" {
        if semantic_learning.entries().is_empty() {
            return Some(write_line(runtime, "learn entries=0").map_err(|_| 195));
        }
        for entry in semantic_learning.entries() {
            if write_line(
                runtime,
                &format!(
                    "learn subject={} action={} policy-epoch={} success={} failure={}",
                    entry.subject,
                    entry.action,
                    entry.policy_epoch,
                    entry.success_count,
                    entry.failure_count
                ),
            )
            .is_err()
            {
                return Some(Err(195));
            }
        }
        return Some(Ok(()));
    }
    if line == "last-status" {
        return Some(
            write_line(runtime, &format!("last-status={previous_status}")).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("fcntl-getfl ") {
        let Some(fd) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: fcntl-getfl <fd>");
            return Some(Err(2));
        };
        *last_status = match shell_get_fd_status_flags(runtime, fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("fcntl-getfd ") {
        let Some(fd) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: fcntl-getfd <fd>");
            return Some(Err(2));
        };
        *last_status = match shell_get_fd_descriptor_flags(runtime, fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("nonblock-fd ") {
        let mut parts = rest.split_whitespace();
        let Some(fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: nonblock-fd <fd> <on|off>");
            return Some(Err(2));
        };
        let nonblock = match parts.next() {
            Some("on") => true,
            Some("off") => false,
            _ => {
                let _ = write_line(runtime, "usage: nonblock-fd <fd> <on|off>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_set_fd_nonblock(runtime, fd, nonblock) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("cloexec-fd ") {
        let mut parts = rest.split_whitespace();
        let Some(fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: cloexec-fd <fd> <on|off>");
            return Some(Err(2));
        };
        let cloexec = match parts.next() {
            Some("on") => true,
            Some("off") => false,
            _ => {
                let _ = write_line(runtime, "usage: cloexec-fd <fd> <on|off>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_set_fd_cloexec(runtime, fd, cloexec) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if line == "true" {
        return Some(Ok(()));
    }
    if line == "false" {
        *last_status = 1;
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("repeat ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let count = match parts.next().and_then(|value| value.parse::<usize>().ok()) {
            Some(count) => count,
            None => {
                let _ = write_line(runtime, "usage: repeat <count> <command>");
                return Some(Err(2));
            }
        };
        let body = match parts.next().map(str::trim_start) {
            Some(body) if !body.is_empty() => body,
            _ => {
                let _ = write_line(runtime, "usage: repeat <count> <command>");
                return Some(Err(2));
            }
        };
        let repeated = (0..count).map(|_| body.to_string()).collect::<Vec<_>>();
        pending_lines.splice(line_index..line_index, repeated);
        return Some(
            write_line(runtime, &format!("repeat-expanded count={count}")).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("assert-status ") {
        let expected = match rest.trim().parse::<i32>().ok() {
            Some(expected) => expected,
            None => {
                let _ = write_line(runtime, "usage: assert-status <code>");
                return Some(Err(2));
            }
        };
        if previous_status != expected {
            *last_status = 1;
            return Some(
                write_line(
                    runtime,
                    &format!("assert-status-failed expected={expected} actual={previous_status}"),
                )
                .map_err(|_| 196),
            );
        }
        return Some(
            write_line(runtime, &format!("assert-status-ok expected={expected}")).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("assert-file-contains ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let path = match parts.next() {
            Some(path) if !path.is_empty() => path,
            _ => {
                let _ = write_line(runtime, "usage: assert-file-contains <path> <text>");
                return Some(Err(2));
            }
        };
        let needle = match parts.next().map(str::trim_start) {
            Some(needle) if !needle.is_empty() => needle,
            _ => {
                let _ = write_line(runtime, "usage: assert-file-contains <path> <text>");
                return Some(Err(2));
            }
        };
        let resolved = resolve_shell_path(cwd, path);
        if shell_assert_file_contains(runtime, &resolved, needle).is_err() {
            *last_status = 1;
            return Some(
                write_line(
                    runtime,
                    &format!("assert-file-contains-failed path={resolved} needle={needle}"),
                )
                .map_err(|_| 205),
            );
        }
        return Some(Ok(()));
    }
    None
}
