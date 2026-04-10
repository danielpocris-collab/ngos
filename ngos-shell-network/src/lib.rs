//! Canonical subsystem role:
//! - subsystem: native networking control surface
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing networking actions over canonical network
//!   and device contracts

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_support::{build_udp_ipv4_frame, parse_ipv4, render_ipv4};
use ngos_shell_types::{
    ShellVariable, parse_u16_arg, parse_u64_arg, parse_usize_arg, resolve_shell_path,
    shell_set_variable,
};
use ngos_shell_vfs::{shell_write_all, write_line};
use ngos_user_abi::{
    Errno, ExitCode, NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind,
    NativeGraphicsEventKind, NativeNetworkEventKind, NativeNetworkInterfaceRecord,
    NativeNetworkSocketRecord, NativeUdpRecvMeta, POLLIN, POLLOUT, POLLPRI, SyscallBackend,
};
use ngos_user_runtime::Runtime;

fn network_event_matches(record: &NativeEventRecord, expected: NativeNetworkEventKind) -> bool {
    record.source_kind == NativeEventSourceKind::Network as u32
        && NativeNetworkEventKind::from_raw(record.detail1) == Some(expected)
}

pub fn wait_for_network_event<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    expected: NativeNetworkEventKind,
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
            .map_err(|_| 469)?;
        if let Some(record) = records[..count]
            .iter()
            .copied()
            .find(|record| network_event_matches(record, expected))
        {
            return Ok(record);
        }
    }
    Err(470)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSmokeReport {
    pub success_line: String,
    pub multi_line: String,
    pub rx_line: String,
    pub refusal_line: &'static str,
    pub teardown_line: String,
    pub teardown_net1_line: String,
    pub rebind_line: String,
    pub rebind_net1_line: String,
    pub recovery_line: String,
    pub recovery_net1_line: String,
}

pub fn run_network_smoke_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<NetworkSmokeReport, ExitCode> {
    if runtime.mkdir_path("/run").is_err() {
        return Err(471);
    }
    if runtime.mksock_path("/run/net0.sock").is_err() {
        return Err(472);
    }
    if runtime.mksock_path("/run/net1.sock").is_err() {
        return Err(512);
    }
    if runtime
        .configure_network_interface_ipv4(
            "/dev/net0",
            [10, 1, 0, 2],
            [255, 255, 255, 0],
            [10, 1, 0, 1],
        )
        .is_err()
    {
        return Err(473);
    }
    if runtime
        .configure_network_interface_admin("/dev/net0", 1500, 4, 4, 2, true, true)
        .is_err()
    {
        return Err(474);
    }
    if runtime
        .configure_network_interface_ipv4(
            "/dev/net1",
            [10, 2, 0, 2],
            [255, 255, 255, 0],
            [10, 2, 0, 1],
        )
        .is_err()
    {
        return Err(513);
    }
    if runtime
        .configure_network_interface_admin("/dev/net1", 1500, 4, 4, 2, true, false)
        .is_err()
    {
        return Err(514);
    }
    if runtime
        .bind_udp_socket("/run/net0.sock", "/dev/net0", 4000, [0, 0, 0, 0], 0)
        .is_err()
    {
        return Err(475);
    }
    if runtime
        .bind_udp_socket("/run/net1.sock", "/dev/net1", 4100, [0, 0, 0, 0], 0)
        .is_err()
    {
        return Err(515);
    }
    let queue_fd = runtime
        .create_event_queue(NativeEventQueueMode::Epoll)
        .map_err(|_| 476)?;
    if runtime
        .watch_network_events(queue_fd, "/dev/net0", None, 700, true, true, true, POLLPRI)
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return Err(477);
    }
    if runtime
        .watch_network_events(queue_fd, "/dev/net1", None, 701, true, true, true, POLLPRI)
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return Err(516);
    }

    let report = (|| {
        let tx_bytes = runtime
            .send_udp_to("/run/net0.sock", [10, 1, 0, 9], 5000, b"hello-qemu")
            .map_err(|_| 478)?;
        let driver_fd = runtime.open_path("/drv/net0").map_err(|_| 479)?;

        let result = (|| {
            let mut driver_request = [0u8; 512];
            let driver_len = runtime
                .read(driver_fd, &mut driver_request)
                .map_err(|_| 480)?;
            if driver_len == 0 {
                return Err(481);
            }
            let completed = runtime
                .complete_network_tx("/drv/net0", 1)
                .map_err(|_| 482)?;
            let tx_event =
                wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::TxDrained)?;
            let interface = runtime
                .inspect_network_interface("/dev/net0")
                .map_err(|_| 483)?;
            runtime
                .send_udp_to("/run/net1.sock", [10, 2, 0, 9], 5100, b"hello-net1")
                .map_err(|_| 517)?;
            let net1_completed = runtime
                .complete_network_tx("/drv/net1", 1)
                .map_err(|_| 518)?;
            let tx_event_net1 =
                wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::TxDrained)?;
            let interface1 = runtime
                .inspect_network_interface("/dev/net1")
                .map_err(|_| 519)?;
            let success_line = format!(
                "network.smoke.success bytes={} tx-completed={} iface={} queue={} link={} tx-packets={} tx-completions={} token={}",
                tx_bytes,
                completed,
                render_ipv4(interface.ipv4_addr),
                interface.tx_ring_depth,
                if interface.link_up != 0 { "up" } else { "down" },
                interface.tx_packets,
                interface.tx_completions,
                tx_event.token
            );
            let multi_line = format!(
                "network.smoke.multi iface0={} iface1={} sockets0={} sockets1={} tx0={} tx1={} completed1={} token1={} outcome=ok",
                render_ipv4(interface.ipv4_addr),
                render_ipv4(interface1.ipv4_addr),
                interface.attached_socket_count,
                interface1.attached_socket_count,
                interface.tx_packets,
                interface1.tx_packets,
                net1_completed,
                tx_event_net1.token
            );

            let rx_frame = build_udp_ipv4_frame(
                [0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee],
                [0x02, 0x11, 0x22, 0x33, 0x44, 0x55],
                [10, 1, 0, 9],
                [10, 1, 0, 2],
                5000,
                4000,
                b"reply-qemu",
            );
            runtime.write(driver_fd, &rx_frame).map_err(|_| 485)?;
            let rx_event =
                wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::RxReady)?;
            let mut recv_buffer = [0u8; 128];
            let (recv_count, recv_meta): (usize, NativeUdpRecvMeta) = runtime
                .recv_udp_from("/run/net0.sock", &mut recv_buffer)
                .map_err(|_| 486)?;
            if &recv_buffer[..recv_count] != b"reply-qemu" {
                return Err(487);
            }
            let rx_line = format!(
                "network.smoke.rx remote={}:{} bytes={} payload=reply-qemu token={}",
                render_ipv4(recv_meta.remote_ipv4),
                recv_meta.remote_port,
                recv_count,
                rx_event.token
            );

            runtime
                .set_network_interface_link_state("/dev/net1", false)
                .map_err(|_| 489)?;
            wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged)
                .map_err(|_| 498)?;
            match runtime.send_udp_to("/run/net1.sock", [10, 2, 0, 10], 5101, b"down-link") {
                Err(Errno::Access) => {}
                _ => return Err(490),
            }
            runtime
                .send_udp_to("/run/net0.sock", [10, 1, 0, 10], 5001, b"net0-still-up")
                .map_err(|_| 520)?;
            runtime
                .complete_network_tx("/drv/net0", 1)
                .map_err(|_| 521)?;
            wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::TxDrained)
                .map_err(|_| 522)?;

            runtime
                .set_network_interface_link_state("/dev/net1", true)
                .map_err(|_| 492)?;
            wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged)
                .map_err(|_| 499)?;
            runtime
                .send_udp_to("/run/net1.sock", [10, 2, 0, 10], 5101, b"recovered-qemu")
                .map_err(|_| 493)?;
            runtime
                .complete_network_tx("/drv/net1", 1)
                .map_err(|_| 494)?;
            wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::TxDrained)
                .map_err(|_| 500)?;

            let socket = runtime
                .inspect_network_socket("/run/net0.sock")
                .map_err(|_| 495)?;
            runtime.unlink_path("/run/net0.sock").map_err(|_| 501)?;
            match runtime.inspect_network_socket("/run/net0.sock") {
                Err(Errno::NoEnt) => {}
                _ => return Err(502),
            }
            let torn_down = runtime
                .inspect_network_interface("/dev/net0")
                .map_err(|_| 503)?;
            if torn_down.attached_socket_count != 0 {
                return Err(504);
            }
            let torn_down_net1 = runtime
                .inspect_network_interface("/dev/net1")
                .map_err(|_| 523)?;
            if torn_down_net1.attached_socket_count != 1 {
                return Err(524);
            }
            let teardown_line = format!(
                "network.smoke.teardown socket=/run/net0.sock attached-sockets={} sibling-sockets={} outcome=ok",
                torn_down.attached_socket_count, torn_down_net1.attached_socket_count
            );
            runtime.unlink_path("/run/net1.sock").map_err(|_| 525)?;
            match runtime.inspect_network_socket("/run/net1.sock") {
                Err(Errno::NoEnt) => {}
                _ => return Err(526),
            }
            let torn_down_net1 = runtime
                .inspect_network_interface("/dev/net1")
                .map_err(|_| 527)?;
            if torn_down_net1.attached_socket_count != 0 {
                return Err(528);
            }
            let teardown_net1_line = format!(
                "network.smoke.teardown socket=/run/net1.sock attached-sockets={} sibling-sockets={} outcome=ok",
                torn_down_net1.attached_socket_count, torn_down.attached_socket_count
            );

            runtime.mksock_path("/run/net0.sock").map_err(|_| 506)?;
            runtime
                .bind_udp_socket("/run/net0.sock", "/dev/net0", 4010, [0, 0, 0, 0], 0)
                .map_err(|_| 507)?;
            let rebound = runtime
                .inspect_network_socket("/run/net0.sock")
                .map_err(|_| 508)?;
            let rebound_interface = runtime
                .inspect_network_interface("/dev/net0")
                .map_err(|_| 509)?;
            if rebound.local_port != 4010 || rebound_interface.attached_socket_count != 1 {
                return Err(510);
            }
            let rebind_line = format!(
                "network.smoke.rebind socket=/run/net0.sock local={}:{} attached-sockets={} outcome=ok",
                render_ipv4(rebound.local_ipv4),
                rebound.local_port,
                rebound_interface.attached_socket_count
            );
            runtime.mksock_path("/run/net1.sock").map_err(|_| 529)?;
            runtime
                .bind_udp_socket("/run/net1.sock", "/dev/net1", 4110, [0, 0, 0, 0], 0)
                .map_err(|_| 530)?;
            let rebound_net1 = runtime
                .inspect_network_socket("/run/net1.sock")
                .map_err(|_| 531)?;
            let rebound_interface_net1 = runtime
                .inspect_network_interface("/dev/net1")
                .map_err(|_| 532)?;
            if rebound_net1.local_port != 4110 || rebound_interface_net1.attached_socket_count != 1
            {
                return Err(533);
            }
            let rebind_net1_line = format!(
                "network.smoke.rebind socket=/run/net1.sock local={}:{} attached-sockets={} outcome=ok",
                render_ipv4(rebound_net1.local_ipv4),
                rebound_net1.local_port,
                rebound_interface_net1.attached_socket_count
            );

            let recovery_line = format!(
                "network.smoke.recovery local={}:{} rx-depth={} tx-packets={} rx-packets={} outcome=ok",
                render_ipv4(socket.local_ipv4),
                socket.local_port,
                socket.rx_depth,
                socket.tx_packets,
                socket.rx_packets
            );
            let socket_net1 = runtime
                .inspect_network_socket("/run/net1.sock")
                .map_err(|_| 534)?;
            let recovery_net1_line = format!(
                "network.smoke.recovery local={}:{} rx-depth={} tx-packets={} rx-packets={} outcome=ok",
                render_ipv4(socket_net1.local_ipv4),
                socket_net1.local_port,
                socket_net1.rx_depth,
                socket_net1.tx_packets,
                socket_net1.rx_packets
            );

            Ok(NetworkSmokeReport {
                success_line,
                multi_line,
                rx_line,
                refusal_line: "network.smoke.refusal interface=/dev/net1 state=link-down errno=EACCES outcome=expected",
                teardown_line,
                teardown_net1_line,
                rebind_line,
                rebind_net1_line,
                recovery_line,
                recovery_net1_line,
            })
        })();

        let _ = runtime.close(driver_fd);
        result
    })();

    let _ = runtime.remove_network_events(queue_fd, "/dev/net0", None, 700);
    let _ = runtime.remove_network_events(queue_fd, "/dev/net1", None, 701);
    let _ = runtime.close(queue_fd);
    report
}

fn emit_network_smoke_report<E>(
    report: &NetworkSmokeReport,
    mut emit: impl FnMut(&str) -> Result<(), E>,
) -> Result<(), E> {
    for line in [
        report.success_line.as_str(),
        report.multi_line.as_str(),
        report.rx_line.as_str(),
        report.refusal_line,
        report.teardown_line.as_str(),
        report.teardown_net1_line.as_str(),
        report.rebind_line.as_str(),
        report.rebind_net1_line.as_str(),
        report.recovery_line.as_str(),
        report.recovery_net1_line.as_str(),
        "network-smoke-ok",
    ] {
        emit(line)?;
    }
    Ok(())
}

pub fn run_network_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let report = match run_network_smoke_report(runtime) {
        Ok(report) => report,
        Err(code) => return code,
    };
    if emit_network_smoke_report(&report, |line| write_line(runtime, line)).is_err() {
        return 511;
    }
    0
}

fn shell_render_network_interface<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeNetworkInterfaceRecord = runtime
        .inspect_network_interface(device_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netif path={} admin={} link={} promisc={} mtu={} tx-cap={} rx-cap={} inflight-limit={} inflight={} free-buffers={} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} addr={} netmask={} gateway={} rx-depth={} tx-depth={} rx-packets={} tx-packets={} tx-completions={} tx-dropped={} rx-dropped={} sockets={}",
            device_path,
            if record.admin_up != 0 { "up" } else { "down" },
            if record.link_up != 0 { "up" } else { "down" },
            if record.promiscuous != 0 { "on" } else { "off" },
            record.mtu,
            record.tx_capacity,
            record.rx_capacity,
            record.tx_inflight_limit,
            record.tx_inflight_depth,
            record.free_buffer_count,
            record.mac[0],
            record.mac[1],
            record.mac[2],
            record.mac[3],
            record.mac[4],
            record.mac[5],
            render_ipv4(record.ipv4_addr),
            render_ipv4(record.ipv4_netmask),
            render_ipv4(record.ipv4_gateway),
            record.rx_ring_depth,
            record.tx_ring_depth,
            record.rx_packets,
            record.tx_packets,
            record.tx_completions,
            record.tx_dropped,
            record.rx_dropped,
            record.attached_socket_count
        ),
    )
}

fn shell_render_network_socket<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeNetworkSocketRecord = runtime
        .inspect_network_socket(socket_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netsock path={} local={}:{} remote={}:{} connected={} rx-depth={} rx-limit={} rx-packets={} tx-packets={} dropped={}",
            socket_path,
            render_ipv4(record.local_ipv4),
            record.local_port,
            render_ipv4(record.remote_ipv4),
            record.remote_port,
            if record.connected != 0 { "yes" } else { "no" },
            record.rx_depth,
            record.rx_queue_limit,
            record.rx_packets,
            record.tx_packets,
            record.dropped_packets
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn shell_net_admin<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    mtu: usize,
    tx_capacity: usize,
    rx_capacity: usize,
    tx_inflight_limit: usize,
    admin_up: bool,
    promiscuous: bool,
) -> Result<(), ExitCode> {
    runtime
        .configure_network_interface_admin(
            device_path,
            mtu,
            tx_capacity,
            rx_capacity,
            tx_inflight_limit,
            admin_up,
            promiscuous,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-admin path={} mtu={} tx-cap={} rx-cap={} inflight-limit={} admin={} promisc={}",
            device_path,
            mtu,
            tx_capacity,
            rx_capacity,
            tx_inflight_limit,
            if admin_up { "up" } else { "down" },
            if promiscuous { "on" } else { "off" }
        ),
    )
}

fn shell_net_config<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    addr: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
) -> Result<(), ExitCode> {
    runtime
        .configure_network_interface_ipv4(device_path, addr, netmask, gateway)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netif-configured path={} addr={} netmask={} gateway={}",
            device_path,
            render_ipv4(addr),
            render_ipv4(netmask),
            render_ipv4(gateway)
        ),
    )
}

fn shell_udp_bind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    device_path: &str,
    local_port: u16,
    remote_ipv4: [u8; 4],
    remote_port: u16,
) -> Result<(), ExitCode> {
    runtime
        .bind_udp_socket(
            socket_path,
            device_path,
            local_port,
            remote_ipv4,
            remote_port,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "udp-bound socket={} device={} local-port={} remote={}:{}",
            socket_path,
            device_path,
            local_port,
            render_ipv4(remote_ipv4),
            remote_port
        ),
    )
}

fn shell_poll_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    interest: u32,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 234)?;
    let events = runtime.poll(fd, interest).map_err(|_| 234)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "poll path={} interest=0x{:x} ready=0x{:x}",
            path, interest, events
        ),
    )
}

fn shell_net_send<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(socket_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, payload.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("net-send socket={} bytes={}", socket_path, payload.len()),
    )
}

fn shell_net_sendto<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    remote_ip: [u8; 4],
    remote_port: u16,
    payload: &str,
) -> Result<(), ExitCode> {
    let written = runtime
        .send_udp_to(socket_path, remote_ip, remote_port, payload.as_bytes())
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-sendto socket={} remote={}:{} bytes={}",
            socket_path,
            render_ipv4(remote_ip),
            remote_port,
            written
        ),
    )
}

fn shell_net_recv<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(socket_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
    write_line(
        runtime,
        &format!(
            "net-recv socket={} bytes={} payload={}",
            socket_path, count, text
        ),
    )
}

fn shell_net_recvfrom<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let mut buffer = [0u8; 512];
    let (count, meta) = runtime
        .recv_udp_from(socket_path, &mut buffer)
        .map_err(|_| 246)?;
    let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
    write_line(
        runtime,
        &format!(
            "net-recvfrom socket={} remote={}:{} bytes={} payload={}",
            socket_path,
            render_ipv4(meta.remote_ipv4),
            meta.remote_port,
            count,
            text
        ),
    )
}

fn shell_driver_inject_udp<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    src_ip: [u8; 4],
    src_port: u16,
    dst_ip: [u8; 4],
    dst_port: u16,
    payload: &str,
) -> Result<(), ExitCode> {
    let frame = build_udp_ipv4_frame(
        [0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee],
        [0x02, 0x11, 0x22, 0x33, 0x44, 0x55],
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        payload.as_bytes(),
    );
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, &frame)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "net-inject driver={} src={}:{} dst={}:{} bytes={}",
            driver_path,
            render_ipv4(src_ip),
            src_port,
            render_ipv4(dst_ip),
            dst_port,
            payload.len()
        ),
    )
}

fn shell_tcp_listen<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    device_path: &str,
    local_port: u16,
    backlog: usize,
) -> Result<(), ExitCode> {
    runtime
        .tcp_listen(socket_path, device_path, local_port, backlog)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "tcp-listen socket={} device={} port={} backlog={}",
            socket_path, device_path, local_port, backlog
        ),
    )
}

fn shell_tcp_connect<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    remote_ipv4: [u8; 4],
    remote_port: u16,
) -> Result<(), ExitCode> {
    runtime
        .tcp_connect(socket_path, remote_ipv4, remote_port)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "tcp-connect socket={} remote={}:{}",
            socket_path,
            render_ipv4(remote_ipv4),
            remote_port
        ),
    )
}

fn shell_tcp_accept<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let (accepted_path, remote_ipv4, remote_port) = runtime
        .tcp_accept(socket_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "tcp-accept socket={} accepted={} remote={}:{}",
            socket_path,
            accepted_path,
            render_ipv4(remote_ipv4),
            remote_port
        ),
    )
}

fn shell_tcp_send<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let written = runtime.tcp_send(socket_path, payload.as_bytes()).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "tcp-send socket={} bytes={}",
            socket_path, written
        ),
    )
}

fn shell_tcp_recv<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let mut buffer = [0u8; 512];
    let count = runtime.tcp_recv(socket_path, &mut buffer).map_err(|_| 246)?;
    let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
    write_line(
        runtime,
        &format!(
            "tcp-recv socket={} bytes={} payload={}",
            socket_path,
            count,
            text
        ),
    )
}

fn shell_tcp_close<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    runtime.tcp_close(socket_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("tcp-close socket={}", socket_path),
    )
}

fn shell_tcp_reset<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    runtime.tcp_reset(socket_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("tcp-reset socket={}", socket_path),
    )
}

fn shell_set_net_link<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    link_up: bool,
) -> Result<(), ExitCode> {
    runtime
        .set_network_interface_link_state(device_path, link_up)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netif-link path={} state={}",
            device_path,
            if link_up { "up" } else { "down" }
        ),
    )
}

fn shell_udp_connect<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    remote_ipv4: [u8; 4],
    remote_port: u16,
) -> Result<(), ExitCode> {
    runtime
        .connect_udp_socket(socket_path, remote_ipv4, remote_port)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "udp-connected socket={} remote={}:{}",
            socket_path,
            render_ipv4(remote_ipv4),
            remote_port
        ),
    )
}

fn shell_complete_net_tx<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    completions: usize,
) -> Result<(), ExitCode> {
    let completed = runtime
        .complete_network_tx(driver_path, completions)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-complete driver={} completed={}",
            driver_path, completed
        ),
    )
}

fn shell_create_event_queue<B: SyscallBackend>(
    runtime: &Runtime<B>,
    mode: NativeEventQueueMode,
) -> Result<usize, ExitCode> {
    let fd = runtime.create_event_queue(mode).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "queue-created fd={} mode={}",
            fd,
            match mode {
                NativeEventQueueMode::Kqueue => "kqueue",
                NativeEventQueueMode::Epoll => "epoll",
            }
        ),
    )?;
    Ok(fd)
}

fn shell_watch_network_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    socket_path: Option<&str>,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .watch_network_events(
            queue_fd,
            device_path,
            socket_path,
            token,
            true,
            true,
            true,
            POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-watch queue={} device={} socket={} token={}",
            queue_fd,
            device_path,
            socket_path.unwrap_or("-"),
            token
        ),
    )
}

fn shell_remove_network_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    socket_path: Option<&str>,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_network_events(queue_fd, device_path, socket_path, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-unwatch queue={} device={} socket={} token={}",
            queue_fd,
            device_path,
            socket_path.unwrap_or("-"),
            token
        ),
    )
}

pub fn shell_wait_event_queue<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
) -> Result<(), ExitCode> {
    let mut records = [NativeEventRecord {
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
        .wait_event_queue(queue_fd, &mut records)
        .map_err(|_| 246)?;
    for record in &records[..count] {
        let source = match NativeEventSourceKind::from_raw(record.source_kind) {
            Some(NativeEventSourceKind::Resource) => {
                let kind = match record.detail0 {
                    0 => "claimed",
                    1 => "queued",
                    2 => "canceled",
                    3 => "released",
                    4 => "handed-off",
                    5 => "revoked",
                    _ => "unknown",
                };
                format!(
                    "resource id={} contract={} kind={}",
                    record.source_arg0, record.source_arg1, kind
                )
            }
            Some(NativeEventSourceKind::Network) => {
                let kind = match NativeNetworkEventKind::from_raw(record.detail1) {
                    Some(NativeNetworkEventKind::LinkChanged) => "link-changed",
                    Some(NativeNetworkEventKind::RxReady) => "rx-ready",
                    Some(NativeNetworkEventKind::TxDrained) => "tx-drained",
                    None => "unknown",
                };
                format!(
                    "network iface={} socket={} kind={}",
                    record.source_arg0,
                    if record.detail0 != 0 {
                        record.source_arg1.to_string()
                    } else {
                        "-".to_string()
                    },
                    kind
                )
            }
            Some(NativeEventSourceKind::Graphics) => {
                match NativeGraphicsEventKind::from_raw(record.detail1) {
                    Some(NativeGraphicsEventKind::Submitted) => format!(
                        "graphics device={} request={} kind=submitted",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Completed) => format!(
                        "graphics device={} request={} kind=completed",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Failed) => format!(
                        "graphics device={} request={} kind=failed",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Drained) => format!(
                        "graphics device={} request={} kind=drained",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Canceled) => format!(
                        "graphics device={} request={} kind=canceled",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Faulted) => format!(
                        "graphics device={} token={} kind=faulted",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Recovered) => format!(
                        "graphics device={} token={} kind=recovered",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Retired) => format!(
                        "graphics device={} token={} kind=retired",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::LeaseReleased) => format!(
                        "graphics device={} contract={} kind=lease-released",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::LeaseAcquired) => format!(
                        "graphics device={} contract={} kind=lease-acquired",
                        record.source_arg0, record.source_arg1
                    ),
                    None => format!(
                        "graphics device={} token={} kind=unknown",
                        record.source_arg0, record.source_arg1
                    ),
                }
            }
            Some(NativeEventSourceKind::Bus) => {
                let kind = match record.detail0 {
                    0 => "attached",
                    1 => "detached",
                    2 => "published",
                    3 => "received",
                    _ => "unknown",
                };
                format!(
                    "bus peer={} endpoint={} kind={}",
                    record.source_arg0, record.source_arg1, kind
                )
            }
            Some(kind) => format!("other:{kind:?}"),
            None => "unknown".to_string(),
        };
        write_line(
            runtime,
            &format!(
                "queue-event queue={} token={} events=0x{:x} source={}",
                queue_fd, record.token, record.events, source
            ),
        )?;
    }
    Ok(())
}

pub fn try_handle_network_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    variables: &mut Vec<ShellVariable>,
    line: &str,
    last_status: &mut i32,
    driver_read: fn(&Runtime<B>, &str) -> Result<(), ExitCode>,
) -> Option<Result<(), ExitCode>> {
    if let Some(path) = line.strip_prefix("netif ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_network_interface(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-config ") {
        let mut parts = rest.split_whitespace();
        let path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        let addr = match parts.next().and_then(parse_ipv4) {
            Some(addr) => addr,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        let netmask = match parts.next().and_then(parse_ipv4) {
            Some(netmask) => netmask,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        let gateway = match parts.next().and_then(parse_ipv4) {
            Some(gateway) => gateway,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-config <device> <addr> <netmask> <gateway>",
                );
                return Some(Err(2));
            }
        };
        return Some(shell_net_config(runtime, &path, addr, netmask, gateway).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-link ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next().map(|path| resolve_shell_path(cwd, path)) else {
            let _ = write_line(runtime, "usage: net-link <device> <up|down>");
            return Some(Err(2));
        };
        let Some(state) = parts.next() else {
            let _ = write_line(runtime, "usage: net-link <device> <up|down>");
            return Some(Err(2));
        };
        let link_up = match state {
            "up" => true,
            "down" => false,
            _ => {
                let _ = write_line(runtime, "usage: net-link <device> <up|down>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_set_net_link(runtime, &device_path, link_up) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-admin ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next().map(|path| resolve_shell_path(cwd, path)) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(mtu) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(tx_cap) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(rx_cap) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(inflight) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(admin_raw) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let Some(promisc_raw) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
            );
            return Some(Err(2));
        };
        let admin_up = match admin_raw {
            "up" => true,
            "down" => false,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
                );
                return Some(Err(2));
            }
        };
        let promiscuous = match promisc_raw {
            "promisc" => true,
            "nopromisc" => false,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-admin <device> <mtu> <tx-cap> <rx-cap> <inflight-limit> <up|down> <promisc|nopromisc>",
                );
                return Some(Err(2));
            }
        };
        *last_status = match shell_net_admin(
            runtime,
            &device_path,
            mtu,
            tx_cap,
            rx_cap,
            inflight,
            admin_up,
            promiscuous,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("udp-bind ") {
        let mut parts = rest.split_whitespace();
        let socket_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let device_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let local_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_ip = match parts.next().and_then(parse_ipv4) {
            Some(addr) => addr,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-bind <socket> <device> <local-port> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        return Some(
            shell_udp_bind(
                runtime,
                &socket_path,
                &device_path,
                local_port,
                remote_ip,
                remote_port,
            )
            .map_err(|_| 205),
        );
    }
    if let Some(rest) = line.strip_prefix("udp-connect ") {
        let mut parts = rest.split_whitespace();
        let socket_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-connect <socket> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-connect <socket> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        let remote_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: udp-connect <socket> <remote-ip> <remote-port>",
                );
                return Some(Err(2));
            }
        };
        *last_status = match shell_udp_connect(runtime, &socket_path, remote_ip, remote_port) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("tcp-listen ") {
        let mut parts = rest.split_whitespace();
        let socket_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-listen <socket> <device> <port> [backlog]");
                return Some(Err(2));
            }
        };
        let device_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-listen <socket> <device> <port> [backlog]");
                return Some(Err(2));
            }
        };
        let local_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(runtime, "usage: tcp-listen <socket> <device> <port> [backlog]");
                return Some(Err(2));
            }
        };
        let backlog = parts.next().map(|s| parse_usize_arg(Some(s))).unwrap_or(Some(128)).unwrap_or(128);
        *last_status = match shell_tcp_listen(runtime, &socket_path, &device_path, local_port, backlog) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("tcp-connect ") {
        let mut parts = rest.split_whitespace();
        let socket_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-connect <socket> <remote-ip> <remote-port>");
                return Some(Err(2));
            }
        };
        let remote_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(runtime, "usage: tcp-connect <socket> <remote-ip> <remote-port>");
                return Some(Err(2));
            }
        };
        let remote_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(runtime, "usage: tcp-connect <socket> <remote-ip> <remote-port>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_tcp_connect(runtime, &socket_path, remote_ip, remote_port) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("tcp-accept ") {
        let socket_path = match rest.split_whitespace().next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-accept <socket>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_tcp_accept(runtime, &socket_path) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("tcp-send ") {
        let mut parts = rest.split_whitespace();
        let socket_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-send <socket> <payload>");
                return Some(Err(2));
            }
        };
        let payload = parts.next().unwrap_or("");
        *last_status = match shell_tcp_send(runtime, &socket_path, payload) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("tcp-recv ") {
        let socket_path = match rest.split_whitespace().next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-recv <socket>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_tcp_recv(runtime, &socket_path) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("tcp-close ") {
        let socket_path = match rest.split_whitespace().next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-close <socket>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_tcp_close(runtime, &socket_path) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("tcp-reset ") {
        let socket_path = match rest.split_whitespace().next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: tcp-reset <socket>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_tcp_reset(runtime, &socket_path) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("netsock ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_render_network_socket(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("queue-create ") {
        let mode = match rest.trim() {
            "kqueue" => Some(NativeEventQueueMode::Kqueue),
            "epoll" => Some(NativeEventQueueMode::Epoll),
            _ => None,
        };
        let Some(mode) = mode else {
            let _ = write_line(runtime, "usage: queue-create <kqueue|epoll>");
            return Some(Err(2));
        };
        *last_status = match shell_create_event_queue(runtime, mode) {
            Ok(fd) => {
                shell_set_variable(variables, "LAST_QUEUE_FD", fd.to_string());
                0
            }
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-watch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(device_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-watch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-watch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let device_path = resolve_shell_path(cwd, device_path);
        let socket_path = parts.next().map(|path| resolve_shell_path(cwd, path));
        *last_status = match shell_watch_network_events(
            runtime,
            queue_fd,
            &device_path,
            socket_path.as_deref(),
            token,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-unwatch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-unwatch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(device_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: net-unwatch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: net-unwatch <queue-fd> <device> <token> [socket]",
            );
            return Some(Err(2));
        };
        let device_path = resolve_shell_path(cwd, device_path);
        let socket_path = parts.next().map(|path| resolve_shell_path(cwd, path));
        *last_status = match shell_remove_network_watch(
            runtime,
            queue_fd,
            &device_path,
            socket_path.as_deref(),
            token,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("queue-wait ") {
        let Some(queue_fd) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: queue-wait <queue-fd>");
            return Some(Err(2));
        };
        *last_status = match shell_wait_event_queue(runtime, queue_fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-send ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let socket = match parts.next() {
            Some(path) if !path.is_empty() => resolve_shell_path(cwd, path),
            _ => {
                let _ = write_line(runtime, "usage: net-send <socket> <payload>");
                return Some(Err(2));
            }
        };
        let payload = match parts.next() {
            Some(payload) if !payload.trim_start().is_empty() => payload.trim_start(),
            _ => {
                let _ = write_line(runtime, "usage: net-send <socket> <payload>");
                return Some(Err(2));
            }
        };
        return Some(shell_net_send(runtime, &socket, payload).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-sendto ") {
        let mut parts = rest.splitn(4, char::is_whitespace);
        let socket = match parts.next() {
            Some(path) if !path.is_empty() => resolve_shell_path(cwd, path),
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let remote_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let remote_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let payload = match parts.next() {
            Some(payload) if !payload.trim_start().is_empty() => payload.trim_start(),
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-sendto <socket> <remote-ip> <remote-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        *last_status = match shell_net_sendto(runtime, &socket, remote_ip, remote_port, payload) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("net-recv ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_net_recv(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("net-recvfrom ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_net_recvfrom(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("net-driver-read ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(driver_read(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("net-complete ") {
        let mut parts = rest.split_whitespace();
        let driver_path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: net-complete <driver> <count>");
                return Some(Err(2));
            }
        };
        let Some(count) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: net-complete <driver> <count>");
            return Some(Err(2));
        };
        *last_status = match shell_complete_net_tx(runtime, &driver_path, count) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("net-inject-udp ") {
        let mut parts = rest.splitn(6, char::is_whitespace);
        let driver_path = match parts.next() {
            Some(path) if !path.is_empty() => resolve_shell_path(cwd, path),
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let src_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let src_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let dst_ip = match parts.next().and_then(parse_ipv4) {
            Some(ip) => ip,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let dst_port = match parse_u16_arg(parts.next()) {
            Some(port) => port,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        let payload = match parts.next() {
            Some(payload) if !payload.is_empty() => payload,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: net-inject-udp <driver> <src-ip> <src-port> <dst-ip> <dst-port> <payload>",
                );
                return Some(Err(2));
            }
        };
        return Some(
            shell_driver_inject_udp(
                runtime,
                &driver_path,
                src_ip,
                src_port,
                dst_ip,
                dst_port,
                payload,
            )
            .map_err(|_| 205),
        );
    }
    if let Some(rest) = line.strip_prefix("poll-path ") {
        let mut parts = rest.split_whitespace();
        let path = match parts.next() {
            Some(path) => resolve_shell_path(cwd, path),
            None => {
                let _ = write_line(runtime, "usage: poll-path <path> <read|write|readwrite>");
                return Some(Err(2));
            }
        };
        let interest = match parts.next() {
            Some("read") => POLLIN,
            Some("write") => POLLOUT,
            Some("readwrite") => POLLIN | POLLOUT,
            _ => {
                let _ = write_line(runtime, "usage: poll-path <path> <read|write|readwrite>");
                return Some(Err(2));
            }
        };
        return Some(shell_poll_path(runtime, &path, interest).map_err(|_| 205));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        NativeEventRecord, NativeEventSourceKind, NativeNetworkEventKind, NetworkSmokeReport,
        emit_network_smoke_report, network_event_matches,
    };
    use alloc::string::{String, ToString};
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn network_event_matching_stays_scoped_to_network_kind_and_expected_detail() {
        let record = NativeEventRecord {
            token: 700,
            events: 0,
            source_kind: NativeEventSourceKind::Network as u32,
            source_arg0: 1,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: NativeNetworkEventKind::RxReady as u32,
        };

        assert!(network_event_matches(
            &record,
            NativeNetworkEventKind::RxReady
        ));
        assert!(!network_event_matches(
            &record,
            NativeNetworkEventKind::TxDrained
        ));
    }

    #[test]
    fn network_smoke_report_emits_expected_lines_in_order() {
        let report = NetworkSmokeReport {
            success_line: String::from("network.smoke.success bytes=10"),
            multi_line: String::from("network.smoke.multi iface0=10.1.0.2 iface1=10.2.0.2"),
            rx_line: String::from("network.smoke.rx remote=10.1.0.9:5000"),
            refusal_line: "network.smoke.refusal interface=/dev/net1 state=link-down errno=EACCES outcome=expected",
            teardown_line: String::from("network.smoke.teardown socket=/run/net0.sock"),
            teardown_net1_line: String::from("network.smoke.teardown socket=/run/net1.sock"),
            rebind_line: String::from("network.smoke.rebind socket=/run/net0.sock"),
            rebind_net1_line: String::from("network.smoke.rebind socket=/run/net1.sock"),
            recovery_line: String::from("network.smoke.recovery local=10.1.0.2:4000"),
            recovery_net1_line: String::from("network.smoke.recovery local=10.2.0.2:4100"),
        };
        let mut lines = Vec::new();

        emit_network_smoke_report(&report, |line| {
            lines.push(line.to_string());
            Ok::<(), ()>(())
        })
        .unwrap();

        assert_eq!(
            lines,
            vec![
                String::from("network.smoke.success bytes=10"),
                String::from("network.smoke.multi iface0=10.1.0.2 iface1=10.2.0.2"),
                String::from("network.smoke.rx remote=10.1.0.9:5000"),
                String::from(
                    "network.smoke.refusal interface=/dev/net1 state=link-down errno=EACCES outcome=expected"
                ),
                String::from("network.smoke.teardown socket=/run/net0.sock"),
                String::from("network.smoke.teardown socket=/run/net1.sock"),
                String::from("network.smoke.rebind socket=/run/net0.sock"),
                String::from("network.smoke.rebind socket=/run/net1.sock"),
                String::from("network.smoke.recovery local=10.1.0.2:4000"),
                String::from("network.smoke.recovery local=10.2.0.2:4100"),
                String::from("network-smoke-ok"),
            ]
        );
    }
}
