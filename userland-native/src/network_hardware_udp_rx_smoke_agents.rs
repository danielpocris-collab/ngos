use super::*;
use alloc::vec::Vec;
use ngos_user_abi::{NativeDriverRecord, NativeNetworkInterfaceRecord};

fn build_dns_query_payload(transaction_id: u16, name: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(64);
    payload.extend_from_slice(&transaction_id.to_be_bytes());
    payload.extend_from_slice(&0x0100u16.to_be_bytes());
    payload.extend_from_slice(&1u16.to_be_bytes());
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(&0u16.to_be_bytes());
    for label in name.split('.') {
        payload.push(label.len() as u8);
        payload.extend_from_slice(label.as_bytes());
    }
    payload.push(0);
    payload.extend_from_slice(&1u16.to_be_bytes());
    payload.extend_from_slice(&1u16.to_be_bytes());
    payload
}

fn dns_response_transaction_id(payload: &[u8]) -> Option<u16> {
    if payload.len() < 2 {
        return None;
    }
    Some(u16::from_be_bytes([payload[0], payload[1]]))
}

#[inline(never)]
pub(crate) fn run_native_network_hardware_udp_rx_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    if runtime.mkdir_path("/run").is_err() {
        return 640;
    }
    if runtime.mksock_path("/run/net0.sock").is_err() {
        return 641;
    }
    if runtime
        .configure_network_interface_ipv4(
            "/dev/net0",
            [10, 0, 2, 15],
            [255, 255, 255, 0],
            [10, 0, 2, 2],
        )
        .is_err()
    {
        return 642;
    }
    if runtime
        .configure_network_interface_admin("/dev/net0", 1500, 8, 8, 4, true, false)
        .is_err()
    {
        return 643;
    }
    if runtime
        .bind_udp_socket("/run/net0.sock", "/dev/net0", 4011, [0, 0, 0, 0], 0)
        .is_err()
    {
        return 644;
    }

    let queue_fd = match runtime.create_event_queue(NativeEventQueueMode::Epoll) {
        Ok(fd) => fd,
        Err(_) => return 645,
    };
    if runtime
        .watch_network_events(queue_fd, "/dev/net0", None, 904, true, true, true, POLLPRI)
        .is_err()
    {
        return 646;
    }

    let baseline_device = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 647,
    };
    let baseline_driver = match runtime.inspect_driver("/drv/net0") {
        Ok(record) => record,
        Err(_) => return 648,
    };
    let baseline_socket = match runtime.inspect_network_socket("/run/net0.sock") {
        Ok(record) => record,
        Err(_) => return 649,
    };
    if baseline_socket.rx_depth != 0 {
        return 650;
    }
    let interface = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 651,
    };
    if interface.attached_socket_count != 1 {
        return 652;
    }

    let request_id = 0x4e47;
    let query = build_dns_query_payload(request_id, "example.com");
    if write_line(
        runtime,
        "network.hw.udp-rx.send-start socket=/run/net0.sock remote=10.0.2.3:53",
    )
    .is_err()
    {
        return 653;
    }
    let tx_bytes = {
        let mut attempt = 0usize;
        loop {
            attempt = attempt.saturating_add(1);
            match runtime.send_udp_to("/run/net0.sock", [10, 0, 2, 3], 53, &query) {
                Ok(count) => break count,
                Err(Errno::Again) if attempt < 20_000 => core::hint::spin_loop(),
                Err(_) => return 654,
            }
        }
    };
    if write_line(
        runtime,
        &format!(
            "network.hw.udp-rx.send-complete socket=/run/net0.sock bytes={} outcome=ok",
            tx_bytes
        ),
    )
    .is_err()
    {
        return 655;
    }
    let completed = match wait_for_udp_tx_completion(runtime, &baseline_device, &baseline_driver) {
        Ok(count) => count,
        Err(code) => return code,
    };
    let tx_event =
        match wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::TxDrained) {
            Ok(record) => record,
            Err(code) => return code,
        };
    let rx_event = match wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::RxReady)
    {
        Ok(record) => record,
        Err(code) => return code,
    };

    let mut recv_buffer = [0u8; 512];
    let (received, recv_meta) = {
        let mut recv_attempt = 0usize;
        loop {
            recv_attempt = recv_attempt.saturating_add(1);
            match runtime.recv_udp_from("/run/net0.sock", &mut recv_buffer) {
                Ok((count, meta)) => break (count, meta),
                Err(Errno::Again) if recv_attempt < 20_000 => core::hint::spin_loop(),
                Err(_) => return 656,
            }
        }
    };
    if recv_meta.remote_ipv4 != [10, 0, 2, 3] || recv_meta.remote_port != 53 {
        return 657;
    }
    if dns_response_transaction_id(&recv_buffer[..received]) != Some(request_id) {
        return 658;
    }
    if received < 12 || (recv_buffer[2] & 0x80) == 0 {
        return 659;
    }
    let socket = match runtime.inspect_network_socket("/run/net0.sock") {
        Ok(record) => record,
        Err(_) => return 658,
    };
    let interface = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 659,
    };
    if socket.rx_packets < baseline_socket.rx_packets.saturating_add(1)
        || socket.rx_depth != 0
        || interface.rx_packets < baseline_device.rx_packets.saturating_add(1)
    {
        return 660;
    }
    if write_line(
        runtime,
        &format!(
            "network.hw.udp-rx.success socket=/run/net0.sock bytes={} remote={}:{} txid=0x{:04x} completed={} tx-token={} rx-token={} socket-rx={} iface-rx={} iface-tx={} iface-completions={}",
            received,
            render_ipv4(recv_meta.remote_ipv4),
            recv_meta.remote_port,
            request_id,
            completed,
            tx_event.token,
            rx_event.token,
            socket.rx_packets,
            interface.rx_packets,
            interface.tx_packets,
            interface.tx_completions
        ),
    )
    .is_err()
    {
        return 661;
    }

    if runtime
        .set_network_interface_link_state("/dev/net0", false)
        .is_err()
    {
        return 662;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged).is_err() {
        return 663;
    }
    match runtime.send_udp_to("/run/net0.sock", [10, 0, 2, 3], 53, &query) {
        Err(Errno::Access) => {}
        _ => return 664,
    }
    if write_line(
        runtime,
        "network.hw.udp-rx.refusal socket=/run/net0.sock state=link-down errno=EACCES outcome=expected",
    )
    .is_err()
    {
        return 665;
    }

    if runtime
        .set_network_interface_link_state("/dev/net0", true)
        .is_err()
    {
        return 666;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged).is_err() {
        return 667;
    }
    let baseline_device = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 668,
    };
    let baseline_driver = match runtime.inspect_driver("/drv/net0") {
        Ok(record) => record,
        Err(_) => return 669,
    };
    let recovery_id = request_id.wrapping_add(1);
    let recovery_query = build_dns_query_payload(recovery_id, "example.com");
    let mut recovery_attempt = 0usize;
    loop {
        recovery_attempt = recovery_attempt.saturating_add(1);
        match runtime.send_udp_to("/run/net0.sock", [10, 0, 2, 3], 53, &recovery_query) {
            Ok(_) => break,
            Err(Errno::Again) if recovery_attempt < 20_000 => core::hint::spin_loop(),
            Err(_) => return 670,
        }
    }
    if wait_for_udp_tx_completion(runtime, &baseline_device, &baseline_driver).is_err() {
        return 671;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::TxDrained).is_err() {
        return 672;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::RxReady).is_err() {
        return 673;
    }
    let mut recovery_buffer = [0u8; 512];
    let (recovery_count, recovery_meta) =
        match runtime.recv_udp_from("/run/net0.sock", &mut recovery_buffer) {
            Ok(value) => value,
            Err(_) => return 674,
        };
    if recovery_meta.remote_ipv4 != [10, 0, 2, 3]
        || recovery_meta.remote_port != 53
        || dns_response_transaction_id(&recovery_buffer[..recovery_count]) != Some(recovery_id)
    {
        return 675;
    }
    let socket = match runtime.inspect_network_socket("/run/net0.sock") {
        Ok(record) => record,
        Err(_) => return 676,
    };
    if write_line(
        runtime,
        &format!(
            "network.hw.udp-rx.recovery socket=/run/net0.sock bytes={} remote={}:{} txid=0x{:04x} socket-rx={} outcome=ok",
            recovery_count,
            render_ipv4(recovery_meta.remote_ipv4),
            recovery_meta.remote_port,
            recovery_id,
            socket.rx_packets
        ),
    )
    .is_err()
    {
        return 677;
    }

    if runtime.unlink_path("/run/net0.sock").is_err() {
        return 678;
    }
    match runtime.inspect_network_socket("/run/net0.sock") {
        Err(Errno::NoEnt) => {}
        _ => return 679,
    }
    let interface = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 680,
    };
    if interface.attached_socket_count != 0 {
        return 681;
    }
    let _ = runtime.remove_network_events(queue_fd, "/dev/net0", None, 904);
    let _ = runtime.close(queue_fd);
    if write_line(runtime, "network-hardware-udp-rx-smoke-ok").is_err() {
        return 682;
    }
    0
}

fn wait_for_udp_tx_completion<B: SyscallBackend>(
    runtime: &Runtime<B>,
    baseline_device: &NativeNetworkInterfaceRecord,
    baseline_driver: &NativeDriverRecord,
) -> Result<usize, ExitCode> {
    let target_completed = baseline_device.tx_completions.saturating_add(1);
    let target_driver_completed = baseline_driver.completed_requests.saturating_add(1);
    for _ in 0..50_000 {
        let device = runtime
            .inspect_network_interface("/dev/net0")
            .map_err(|_| 683)?;
        let driver = runtime.inspect_driver("/drv/net0").map_err(|_| 684)?;
        if device.tx_completions >= target_completed
            && driver.completed_requests >= target_driver_completed
            && device.tx_ring_depth == 0
        {
            return match runtime.complete_network_tx("/drv/net0", 1) {
                Ok(0) => {
                    core::hint::spin_loop();
                    continue;
                }
                Ok(count) => Ok(count),
                Err(_) => Err(683),
            };
        }
        core::hint::spin_loop();
    }
    Err(685)
}
