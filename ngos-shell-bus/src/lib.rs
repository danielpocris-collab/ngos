//! Canonical subsystem role:
//! - subsystem: native bus control surface
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-bus`
//! - truth path role: operator-facing bus watch control over canonical bus and
//!   event queue contracts
//!
//! Canonical contract families handled here:
//! - bus lifecycle command contracts
//! - bus message flow command contracts
//! - bus event watch command contracts
//! - queue-driven bus observation contracts
//!
//! This module may issue and render bus watch operations, but it must not
//! redefine bus truth, event queue truth, or ownership from lower layers.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::{
    ShellVariable, parse_u64_arg, parse_usize_arg, resolve_shell_path, shell_set_variable,
};
use ngos_user_abi::{
    BlockRightsMask, ExitCode, NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind,
    NativeResourceKind, POLLPRI, SyscallBackend,
};
use ngos_user_runtime::Runtime;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

fn read_procfs_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<u8>, ExitCode> {
    let mut data = Vec::new();
    data.resize(1024, 0);
    loop {
        let count = runtime.read_procfs(path, &mut data).map_err(|_| 520)?;
        if count == 0 {
            data.truncate(0);
            return Ok(data);
        }
        if count < data.len() {
            data.truncate(count);
            return Ok(data);
        }
        data.resize(data.len() * 2, 0);
    }
}

fn procfs_text_contains_all_markers(text: &str, markers: &[&str]) -> bool {
    markers.iter().all(|marker| text.contains(marker))
}

fn bus_procfs_contains_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    markers: &[&str],
) -> Result<bool, ExitCode> {
    let bus = read_procfs_all(runtime, "/proc/system/bus")?;
    let text = core::str::from_utf8(&bus).map_err(|_| 520)?;
    Ok(procfs_text_contains_all_markers(text, markers))
}

fn bus_event_matches(
    record: &NativeEventRecord,
    expected_kind: u32,
    expected_peer: usize,
    expected_endpoint: usize,
) -> bool {
    record.source_kind == NativeEventSourceKind::Bus as u32
        && record.source_arg0 == expected_peer as u64
        && record.source_arg1 == expected_endpoint as u64
        && record.detail0 == expected_kind
}

fn wait_for_bus_event<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    expected_kind: u32,
    expected_peer: usize,
    expected_endpoint: usize,
) -> Result<NativeEventRecord, ExitCode> {
    for _attempt in 0..4 {
        let mut records = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 8];
        let count = runtime
            .wait_event_queue(queue_fd, &mut records)
            .map_err(|_| 521)?;
        if let Some(record) = records[..count].iter().copied().find(|record| {
            bus_event_matches(record, expected_kind, expected_peer, expected_endpoint)
        }) {
            return Ok(record);
        }
    }
    Err(522)
}

#[inline(never)]
pub fn run_bus_boot_smoke<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    bind_observe_contract: F,
) -> ExitCode
where
    F: FnOnce(&Runtime<B>) -> Result<(), ExitCode>,
{
    let mut probe = [0u8; 64];
    if runtime.read_procfs("/proc/system/bus", &mut probe).is_ok() {
        return 523;
    }
    if write_line(
        runtime,
        "bus.smoke.refusal path=/proc/system/bus contract=observe outcome=expected",
    )
    .is_err()
    {
        return 524;
    }

    if let Err(code) = bind_observe_contract(runtime) {
        return code;
    }

    if runtime.mkdir_path("/ipc").is_err() {
        return 525;
    }
    if runtime.mkchan_path("/ipc/render").is_err() {
        return 526;
    }

    let domain = match runtime.create_domain(None, "bus-proof") {
        Ok(id) => id,
        Err(_) => return 527,
    };
    let resource = match runtime.create_resource(domain, NativeResourceKind::Channel, "render-bus")
    {
        Ok(id) => id,
        Err(_) => return 528,
    };
    let peer = match runtime.create_bus_peer(domain, "renderer") {
        Ok(id) => id,
        Err(_) => return 529,
    };
    let endpoint = match runtime.create_bus_endpoint(domain, resource, "/ipc/render") {
        Ok(id) => id,
        Err(_) => return 530,
    };

    let observe_markers = [
        "bus-peers:\t1",
        "bus-endpoints:\t1",
        "path=/ipc/render",
        "queue-capacity=64",
        "queue-peak=0",
        "overflows=0",
    ];
    let observe_ok = match bus_procfs_contains_all(runtime, &observe_markers) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !observe_ok {
        return 531;
    }
    if write_line(
        runtime,
        &format!(
            "bus.smoke.observe path=/proc/system/bus peer={} endpoint={} path=/ipc/render capacity=64 outcome=ok",
            peer, endpoint
        ),
    )
    .is_err()
    {
        return 532;
    }

    let queue_fd = match runtime.create_event_queue(NativeEventQueueMode::Epoll) {
        Ok(fd) => fd,
        Err(_) => return 533,
    };
    if runtime
        .watch_bus_events(queue_fd, endpoint, 910, true, true, true, true, POLLPRI)
        .is_err()
    {
        return 534;
    }

    if runtime.attach_bus_peer(peer, endpoint).is_err() {
        return 535;
    }
    let attached = match wait_for_bus_event(runtime, queue_fd, 0, peer, endpoint) {
        Ok(record) => record,
        Err(code) => return code,
    };
    if write_line(
        runtime,
        &format!(
            "bus.smoke.attach peer={} endpoint={} token={} kind=attached outcome=ok",
            peer, endpoint, attached.token
        ),
    )
    .is_err()
    {
        return 536;
    }

    let first_payload = b"hello-qemu";
    let first_bytes = match runtime.publish_bus_message(peer, endpoint, first_payload) {
        Ok(bytes) => bytes,
        Err(_) => return 537,
    };
    let published = match wait_for_bus_event(runtime, queue_fd, 2, peer, endpoint) {
        Ok(record) => record,
        Err(code) => return code,
    };
    let mut receive_buffer = [0u8; 64];
    let received_bytes = match runtime.receive_bus_message(peer, endpoint, &mut receive_buffer) {
        Ok(bytes) => bytes,
        Err(_) => return 538,
    };
    let received = match wait_for_bus_event(runtime, queue_fd, 3, peer, endpoint) {
        Ok(record) => record,
        Err(code) => return code,
    };
    if &receive_buffer[..received_bytes] != first_payload {
        return 539;
    }
    if write_line(
        runtime,
        &format!(
            "bus.smoke.success peer={} endpoint={} published={} received={} token-pub={} token-recv={} payload=hello-qemu outcome=ok",
            peer, endpoint, first_bytes, received_bytes, published.token, received.token
        ),
    )
    .is_err()
    {
        return 540;
    }

    if runtime.remove_bus_events(queue_fd, endpoint, 910).is_err() {
        return 541;
    }
    for _ in 0..64 {
        if runtime.publish_bus_message(peer, endpoint, b"q").is_err() {
            return 542;
        }
    }
    let overflow_errno = match runtime.publish_bus_message(peer, endpoint, b"overflow-qemu") {
        Ok(_) => return 543,
        Err(errno) => errno,
    };
    let drained_bytes = match runtime.receive_bus_message(peer, endpoint, &mut receive_buffer) {
        Ok(bytes) => bytes,
        Err(_) => return 544,
    };
    if drained_bytes != 1 || receive_buffer[0] != b'q' {
        return 545;
    }
    let overflow_recovery_bytes = match runtime.publish_bus_message(peer, endpoint, b"r") {
        Ok(bytes) => bytes,
        Err(_) => return 546,
    };
    if overflow_recovery_bytes != 1 {
        return 547;
    }
    for _ in 0..64 {
        if runtime
            .receive_bus_message(peer, endpoint, &mut receive_buffer)
            .is_err()
        {
            return 548;
        }
    }
    if runtime
        .watch_bus_events(queue_fd, endpoint, 911, true, true, true, true, POLLPRI)
        .is_err()
    {
        return 549;
    }
    if write_line(
        runtime,
        &format!(
            "bus.smoke.overflow peer={} endpoint={} errno={:?} peak=64 overflows=1 outcome=ok",
            peer, endpoint, overflow_errno
        ),
    )
    .is_err()
    {
        return 550;
    }

    if runtime.detach_bus_peer(peer, endpoint).is_err() {
        return 551;
    }
    let detached = match wait_for_bus_event(runtime, queue_fd, 1, peer, endpoint) {
        Ok(record) => record,
        Err(code) => return code,
    };
    let refusal_errno = match runtime.publish_bus_message(peer, endpoint, b"detached-qemu") {
        Ok(_) => return 552,
        Err(errno) => errno,
    };
    if write_line(
        runtime,
        &format!(
            "bus.smoke.detach peer={} endpoint={} token={} outcome=ok",
            peer, endpoint, detached.token
        ),
    )
    .is_err()
    {
        return 553;
    }
    if write_line(
        runtime,
        &format!(
            "bus.smoke.refusal peer={} endpoint={} errno={:?} outcome=expected",
            peer, endpoint, refusal_errno
        ),
    )
    .is_err()
    {
        return 554;
    }

    if runtime.attach_bus_peer(peer, endpoint).is_err() {
        return 555;
    }
    let reattached = match wait_for_bus_event(runtime, queue_fd, 0, peer, endpoint) {
        Ok(record) => record,
        Err(code) => return code,
    };
    let recovery_payload = b"recovered-qemu";
    let recovery_published = match runtime.publish_bus_message(peer, endpoint, recovery_payload) {
        Ok(bytes) => bytes,
        Err(_) => return 556,
    };
    let republished = match wait_for_bus_event(runtime, queue_fd, 2, peer, endpoint) {
        Ok(record) => record,
        Err(code) => return code,
    };
    let recovery_received = match runtime.receive_bus_message(peer, endpoint, &mut receive_buffer) {
        Ok(bytes) => bytes,
        Err(_) => return 557,
    };
    let reconsumed = match wait_for_bus_event(runtime, queue_fd, 3, peer, endpoint) {
        Ok(record) => record,
        Err(code) => return code,
    };
    if &receive_buffer[..recovery_received] != recovery_payload {
        return 558;
    }
    if write_line(
        runtime,
        &format!(
            "bus.smoke.recovery peer={} endpoint={} attach-token={} publish-token={} receive-token={} bytes={} payload=recovered-qemu outcome=ok",
            peer,
            endpoint,
            reattached.token,
            republished.token,
            reconsumed.token,
            recovery_published
        ),
    )
    .is_err()
    {
        return 559;
    }

    let endpoint_record = match runtime.inspect_bus_endpoint(endpoint) {
        Ok(record) => record,
        Err(_) => return 550,
    };
    if endpoint_record.queue_depth != 0
        || endpoint_record.publish_count != 67
        || endpoint_record.receive_count != 67
        || endpoint_record.peak_queue_depth != 64
        || endpoint_record.overflow_count != 1
        || endpoint_record.attached_peer_count != 1
    {
        return 560;
    }
    let state_markers = [
        "path=/ipc/render",
        "queue-depth=0",
        "queue-capacity=64",
        "queue-peak=64",
        "overflows=1",
        "publishes=67",
        "receives=67",
    ];
    let state_ok = match bus_procfs_contains_all(runtime, &state_markers) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !state_ok {
        return 561;
    }
    if write_line(
        runtime,
        &format!(
            "bus.smoke.state peer={} endpoint={} attached={} depth={} publishes={} receives={} peak={} overflows={} outcome=ok",
            peer,
            endpoint,
            endpoint_record.attached_peer_count,
            endpoint_record.queue_depth,
            endpoint_record.publish_count,
            endpoint_record.receive_count,
            endpoint_record.peak_queue_depth,
            endpoint_record.overflow_count
        ),
    )
    .is_err()
    {
        return 562;
    }
    if write_line(runtime, "bus-smoke-ok").is_err() {
        return 563;
    }
    0
}

pub fn try_handle_bus_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
    variables: &mut Vec<ShellVariable>,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if line == "bus-peers" {
        return Some(shell_render_bus_peers(runtime).map_err(|_| 246));
    }
    if let Some(rest) = line.strip_prefix("bus-peer ") {
        let Some(id) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: bus-peer <id>");
            return Some(Err(2));
        };
        return Some(shell_render_bus_peer_detail(runtime, id).map_err(|_| 246));
    }
    if line == "bus-endpoints" {
        return Some(shell_render_bus_endpoints(runtime).map_err(|_| 246));
    }
    if let Some(rest) = line.strip_prefix("bus-endpoint ") {
        let Some(id) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: bus-endpoint <id>");
            return Some(Err(2));
        };
        return Some(shell_render_bus_endpoint_detail(runtime, id).map_err(|_| 246));
    }
    if let Some(rest) = line.strip_prefix("mkbuspeer ") {
        let mut parts = rest.split_whitespace();
        let Some(domain) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: mkbuspeer <domain> <name>");
            return Some(Err(2));
        };
        let name = parts.collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            let _ = write_line(runtime, "usage: mkbuspeer <domain> <name>");
            return Some(Err(2));
        }
        return Some(
            match shell_create_bus_peer(runtime, variables, domain, &name) {
                Ok(()) => Ok(()),
                Err(code) => {
                    *last_status = code;
                    Ok(())
                }
            },
        );
    }
    if let Some(rest) = line.strip_prefix("mkbusendpoint ") {
        let mut parts = rest.split_whitespace();
        let Some(domain) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: mkbusendpoint <domain> <resource> <path>");
            return Some(Err(2));
        };
        let Some(resource) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: mkbusendpoint <domain> <resource> <path>");
            return Some(Err(2));
        };
        let path = parts.collect::<Vec<_>>().join(" ");
        if path.is_empty() {
            let _ = write_line(runtime, "usage: mkbusendpoint <domain> <resource> <path>");
            return Some(Err(2));
        }
        let resolved = resolve_shell_path(cwd, &path);
        return Some(
            match shell_create_bus_endpoint(runtime, variables, domain, resource, &resolved) {
                Ok(()) => Ok(()),
                Err(code) => {
                    *last_status = code;
                    Ok(())
                }
            },
        );
    }
    if let Some(rest) = line.strip_prefix("bus-attach ") {
        let mut parts = rest.split_whitespace();
        let Some(peer) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-attach <peer> <endpoint>");
            return Some(Err(2));
        };
        let Some(endpoint) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-attach <peer> <endpoint>");
            return Some(Err(2));
        };
        *last_status = match shell_attach_bus_peer(runtime, peer, endpoint) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("bus-attach-rights ") {
        let mut parts = rest.split_whitespace();
        let Some(peer) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: bus-attach-rights <peer> <endpoint> <read|write|readwrite>",
            );
            return Some(Err(2));
        };
        let Some(endpoint) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: bus-attach-rights <peer> <endpoint> <read|write|readwrite>",
            );
            return Some(Err(2));
        };
        let Some(rights) = parts.next().and_then(parse_bus_rights) else {
            let _ = write_line(
                runtime,
                "usage: bus-attach-rights <peer> <endpoint> <read|write|readwrite>",
            );
            return Some(Err(2));
        };
        *last_status = match shell_attach_bus_peer_with_rights(runtime, peer, endpoint, rights) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("bus-detach ") {
        let mut parts = rest.split_whitespace();
        let Some(peer) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-detach <peer> <endpoint>");
            return Some(Err(2));
        };
        let Some(endpoint) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-detach <peer> <endpoint>");
            return Some(Err(2));
        };
        *last_status = match shell_detach_bus_peer(runtime, peer, endpoint) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("bus-send ") {
        let mut parts = rest.split_whitespace();
        let Some(peer) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-send <peer> <endpoint> <payload>");
            return Some(Err(2));
        };
        let Some(endpoint) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-send <peer> <endpoint> <payload>");
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(runtime, "usage: bus-send <peer> <endpoint> <payload>");
            return Some(Err(2));
        }
        *last_status = match shell_publish_bus_message(runtime, peer, endpoint, &payload) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("bus-recv ") {
        let mut parts = rest.split_whitespace();
        let Some(peer) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-recv <peer> <endpoint>");
            return Some(Err(2));
        };
        let Some(endpoint) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-recv <peer> <endpoint>");
            return Some(Err(2));
        };
        *last_status = match shell_receive_bus_message(runtime, variables, peer, endpoint) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }

    if let Some(rest) = line.strip_prefix("bus-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: bus-watch <queue-fd> <endpoint> <token> [all|attached,detached,published,received]",
            );
            return Some(Err(2));
        };
        let Some(endpoint) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: bus-watch <queue-fd> <endpoint> <token> [all|attached,detached,published,received]",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: bus-watch <queue-fd> <endpoint> <token> [all|attached,detached,published,received]",
            );
            return Some(Err(2));
        };
        let Some((attached, detached, published, received, kinds_label)) =
            parse_bus_watch_kinds(parts.next())
        else {
            let _ = write_line(
                runtime,
                "usage: bus-watch <queue-fd> <endpoint> <token> [all|attached,detached,published,received]",
            );
            return Some(Err(2));
        };
        *last_status = match shell_watch_bus_events(
            runtime,
            queue_fd,
            endpoint,
            token,
            attached,
            detached,
            published,
            received,
            &kinds_label,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }

    if let Some(rest) = line.strip_prefix("bus-unwatch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-unwatch <queue-fd> <endpoint> <token>");
            return Some(Err(2));
        };
        let Some(endpoint) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-unwatch <queue-fd> <endpoint> <token>");
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: bus-unwatch <queue-fd> <endpoint> <token>");
            return Some(Err(2));
        };
        *last_status = match shell_remove_bus_watch(runtime, queue_fd, endpoint, token) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }

    None
}

fn parse_bus_watch_kinds(raw: Option<&str>) -> Option<(bool, bool, bool, bool, String)> {
    let Some(raw) = raw else {
        return Some((true, true, true, true, String::from("all")));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "all" {
        return Some((true, true, true, true, String::from("all")));
    }
    let mut attached = false;
    let mut detached = false;
    let mut published = false;
    let mut received = false;
    for token in trimmed.split(',') {
        match token.trim() {
            "attached" => attached = true,
            "detached" => detached = true,
            "published" => published = true,
            "received" => received = true,
            _ => return None,
        }
    }
    Some((attached, detached, published, received, trimmed.to_string()))
}

fn parse_bus_rights(token: &str) -> Option<BlockRightsMask> {
    match token {
        "read" => Some(BlockRightsMask::READ),
        "write" => Some(BlockRightsMask::WRITE),
        "readwrite" => Some(BlockRightsMask::READ.union(BlockRightsMask::WRITE)),
        _ => None,
    }
}

fn bus_rights_label(rights: BlockRightsMask) -> &'static str {
    match (
        rights.contains(BlockRightsMask::READ),
        rights.contains(BlockRightsMask::WRITE),
    ) {
        (true, true) => "readwrite",
        (true, false) => "read",
        (false, true) => "write",
        (false, false) => "none",
    }
}

fn shell_watch_bus_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    endpoint: usize,
    token: u64,
    attached: bool,
    detached: bool,
    published: bool,
    received: bool,
    kinds_label: &str,
) -> Result<(), ExitCode> {
    runtime
        .watch_bus_events(
            queue_fd, endpoint, token, attached, detached, published, received, POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "bus-watch queue={} endpoint={} token={} kinds={}",
            queue_fd, endpoint, token, kinds_label
        ),
    )
}

fn shell_remove_bus_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    endpoint: usize,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_bus_events(queue_fd, endpoint, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "bus-unwatch queue={} endpoint={} token={}",
            queue_fd, endpoint, token
        ),
    )
}

fn shell_render_bus_peers<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = [0u64; 64];
    let count = runtime.list_bus_peers(&mut ids).map_err(|_| 246)?;
    write_line(runtime, &format!("bus-peers count={count}"))?;
    for id in ids.into_iter().take(count) {
        let record = runtime.inspect_bus_peer(id as usize).map_err(|_| 246)?;
        write_line(
            runtime,
            &format!(
                "bus-peer id={} owner={} domain={} attached={} readable={} writable={} publishes={} receives={} last-endpoint={}",
                record.id,
                record.owner,
                record.domain,
                record.attached_endpoint_count,
                record.readable_endpoint_count,
                record.writable_endpoint_count,
                record.publish_count,
                record.receive_count,
                record.last_endpoint,
            ),
        )?;
    }
    Ok(())
}

fn shell_render_bus_peer_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_bus_peer(id).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "bus-peer-detail id={} owner={} domain={} attached={} readable={} writable={} publishes={} receives={} last-endpoint={}",
            record.id,
            record.owner,
            record.domain,
            record.attached_endpoint_count,
            record.readable_endpoint_count,
            record.writable_endpoint_count,
            record.publish_count,
            record.receive_count,
            record.last_endpoint,
        ),
    )
}

fn shell_render_bus_endpoints<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = [0u64; 64];
    let count = runtime.list_bus_endpoints(&mut ids).map_err(|_| 246)?;
    write_line(runtime, &format!("bus-endpoints count={count}"))?;
    for id in ids.into_iter().take(count) {
        let record = runtime.inspect_bus_endpoint(id as usize).map_err(|_| 246)?;
        write_line(
            runtime,
            &format!(
                "bus-endpoint id={} domain={} resource={} kind={} attached={} readers={} writers={} publishes={} receives={} bytes={} depth={} capacity={} peak={} overflows={} last-peer={}",
                record.id,
                record.domain,
                record.resource,
                record.kind,
                record.attached_peer_count,
                record.readable_peer_count,
                record.writable_peer_count,
                record.publish_count,
                record.receive_count,
                record.byte_count,
                record.queue_depth,
                record.queue_capacity,
                record.peak_queue_depth,
                record.overflow_count,
                record.last_peer,
            ),
        )?;
    }
    Ok(())
}

fn shell_render_bus_endpoint_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_bus_endpoint(id).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "bus-endpoint-detail id={} domain={} resource={} kind={} attached={} readers={} writers={} publishes={} receives={} bytes={} depth={} capacity={} peak={} overflows={} last-peer={}",
            record.id,
            record.domain,
            record.resource,
            record.kind,
            record.attached_peer_count,
            record.readable_peer_count,
            record.writable_peer_count,
            record.publish_count,
            record.receive_count,
            record.byte_count,
            record.queue_depth,
            record.queue_capacity,
            record.peak_queue_depth,
            record.overflow_count,
            record.last_peer,
        ),
    )
}

fn shell_create_bus_peer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    variables: &mut Vec<ShellVariable>,
    domain: usize,
    name: &str,
) -> Result<(), i32> {
    let id = runtime.create_bus_peer(domain, name).map_err(|_| 246)?;
    shell_set_variable(variables, "LAST_BUS_PEER_ID", id.to_string());
    shell_set_variable(variables, "LAST_CREATED_ID", id.to_string());
    write_line(
        runtime,
        &format!("bus-peer-created id={} domain={} name={}", id, domain, name),
    )
    .map_err(|_| 246)
}

fn shell_create_bus_endpoint<B: SyscallBackend>(
    runtime: &Runtime<B>,
    variables: &mut Vec<ShellVariable>,
    domain: usize,
    resource: usize,
    path: &str,
) -> Result<(), i32> {
    let id = runtime
        .create_bus_endpoint(domain, resource, path)
        .map_err(|_| 246)?;
    shell_set_variable(variables, "LAST_BUS_ENDPOINT_ID", id.to_string());
    shell_set_variable(variables, "LAST_CREATED_ID", id.to_string());
    write_line(
        runtime,
        &format!(
            "bus-endpoint-created id={} domain={} resource={} path={}",
            id, domain, resource, path
        ),
    )
    .map_err(|_| 246)
}

fn shell_attach_bus_peer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    peer: usize,
    endpoint: usize,
) -> Result<(), i32> {
    shell_attach_bus_peer_with_rights(
        runtime,
        peer,
        endpoint,
        BlockRightsMask::READ.union(BlockRightsMask::WRITE),
    )
}

fn shell_attach_bus_peer_with_rights<B: SyscallBackend>(
    runtime: &Runtime<B>,
    peer: usize,
    endpoint: usize,
    rights: BlockRightsMask,
) -> Result<(), i32> {
    runtime
        .attach_bus_peer_with_rights(peer, endpoint, rights)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "bus-attached peer={} endpoint={} rights={}",
            peer,
            endpoint,
            bus_rights_label(rights)
        ),
    )
    .map_err(|_| 246)
}

fn shell_detach_bus_peer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    peer: usize,
    endpoint: usize,
) -> Result<(), i32> {
    runtime.detach_bus_peer(peer, endpoint).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("bus-detached peer={} endpoint={}", peer, endpoint),
    )
    .map_err(|_| 246)
}

fn shell_publish_bus_message<B: SyscallBackend>(
    runtime: &Runtime<B>,
    peer: usize,
    endpoint: usize,
    payload: &str,
) -> Result<(), i32> {
    let bytes = runtime
        .publish_bus_message(peer, endpoint, payload.as_bytes())
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "bus-published peer={} endpoint={} bytes={} payload={}",
            peer, endpoint, bytes, payload
        ),
    )
    .map_err(|_| 246)
}

fn shell_receive_bus_message<B: SyscallBackend>(
    runtime: &Runtime<B>,
    variables: &mut Vec<ShellVariable>,
    peer: usize,
    endpoint: usize,
) -> Result<(), i32> {
    let mut buffer = [0u8; 256];
    let bytes = runtime
        .receive_bus_message(peer, endpoint, &mut buffer)
        .map_err(|_| 246)?;
    let payload = String::from_utf8_lossy(&buffer[..bytes]).into_owned();
    shell_set_variable(variables, "LAST_BUS_MESSAGE", payload.clone());
    write_line(
        runtime,
        &format!(
            "bus-received peer={} endpoint={} bytes={} payload={}",
            peer, endpoint, bytes, payload
        ),
    )
    .map_err(|_| 246)
}

#[cfg(test)]
mod tests {
    use super::{
        NativeEventRecord, NativeEventSourceKind, bus_event_matches,
        procfs_text_contains_all_markers,
    };
    use ngos_user_abi::POLLPRI;

    #[test]
    fn bus_event_matching_stays_scoped_to_expected_kind_and_endpoint() {
        let record = NativeEventRecord {
            token: 910,
            events: POLLPRI,
            source_kind: NativeEventSourceKind::Bus as u32,
            source_arg0: 11,
            source_arg1: 22,
            source_arg2: 0,
            detail0: 2,
            detail1: 0,
        };

        assert!(bus_event_matches(&record, 2, 11, 22));
        assert!(!bus_event_matches(&record, 3, 11, 22));
        assert!(!bus_event_matches(&record, 2, 12, 22));
        assert!(!bus_event_matches(&record, 2, 11, 23));
    }

    #[test]
    fn procfs_marker_scan_requires_every_expected_bus_marker() {
        let text = "bus-peers:\t1\npath=/ipc/render\nqueue-capacity=64\n";

        assert!(procfs_text_contains_all_markers(
            text,
            &["bus-peers:\t1", "path=/ipc/render"]
        ));
        assert!(!procfs_text_contains_all_markers(
            text,
            &["bus-peers:\t1", "overflows=0"]
        ));
    }
}
