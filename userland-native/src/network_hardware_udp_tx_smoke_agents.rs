use super::*;
use ngos_user_abi::{NativeDriverRecord, NativeNetworkInterfaceRecord};

#[inline(never)]
pub(crate) fn run_native_network_hardware_udp_tx_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    if runtime.mkdir_path("/run").is_err() {
        return 610;
    }
    if runtime.mksock_path("/run/net0.sock").is_err() {
        return 611;
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
        return 612;
    }
    if runtime
        .configure_network_interface_admin("/dev/net0", 1500, 8, 8, 4, true, false)
        .is_err()
    {
        return 613;
    }
    if runtime
        .bind_udp_socket("/run/net0.sock", "/dev/net0", 4010, [0, 0, 0, 0], 0)
        .is_err()
    {
        return 614;
    }
    let queue_fd = match runtime.create_event_queue(NativeEventQueueMode::Epoll) {
        Ok(fd) => fd,
        Err(_) => return 615,
    };
    if runtime
        .watch_network_events(queue_fd, "/dev/net0", None, 902, true, false, true, POLLPRI)
        .is_err()
    {
        return 616;
    }
    let interface = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 617,
    };
    if interface.attached_socket_count != 1 {
        return 618;
    }
    let baseline_device = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 619,
    };
    let baseline_driver = match runtime.inspect_driver("/drv/net0") {
        Ok(record) => record,
        Err(_) => return 620,
    };
    if write_line(runtime, "network.hw.udp.send-start socket=/run/net0.sock").is_err() {
        return 621;
    }
    let tx_bytes = {
        let mut attempt = 0usize;
        loop {
            attempt = attempt.saturating_add(1);
            match runtime.send_udp_to("/run/net0.sock", [10, 0, 2, 2], 5000, b"hello-qemu-hw-udp") {
                Ok(count) => break count,
                Err(Errno::Again) if attempt < 20_000 => core::hint::spin_loop(),
                Err(_) => return 619,
            }
        }
    };
    if write_line(
        runtime,
        &format!(
            "network.hw.udp.send-complete socket=/run/net0.sock bytes={} outcome=ok",
            tx_bytes
        ),
    )
    .is_err()
    {
        return 622;
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
    let socket = match runtime.inspect_network_socket("/run/net0.sock") {
        Ok(record) => record,
        Err(_) => return 623,
    };
    let interface = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 624,
    };
    if write_line(
        runtime,
        &format!(
            "network.hw.udp.success socket=/run/net0.sock bytes={} completed={} token={} local={}:{} socket-tx={} iface-tx={} iface-completions={}",
            tx_bytes,
            completed,
            tx_event.token,
            render_ipv4(socket.local_ipv4),
            socket.local_port,
            socket.tx_packets,
            interface.tx_packets,
            interface.tx_completions
        ),
    )
    .is_err()
    {
        return 625;
    }

    if runtime
        .set_network_interface_link_state("/dev/net0", false)
        .is_err()
    {
        return 626;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged).is_err() {
        return 627;
    }
    match runtime.send_udp_to("/run/net0.sock", [10, 0, 2, 2], 5001, b"link-down-hw-udp") {
        Err(Errno::Access) => {}
        _ => return 628,
    }
    if write_line(
        runtime,
        "network.hw.udp.refusal socket=/run/net0.sock state=link-down errno=EACCES outcome=expected",
    )
    .is_err()
    {
        return 629;
    }

    if runtime
        .set_network_interface_link_state("/dev/net0", true)
        .is_err()
    {
        return 630;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged).is_err() {
        return 631;
    }
    let baseline_device = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 633,
    };
    let baseline_driver = match runtime.inspect_driver("/drv/net0") {
        Ok(record) => record,
        Err(_) => return 634,
    };
    let mut recovered_attempt = 0usize;
    loop {
        recovered_attempt = recovered_attempt.saturating_add(1);
        match runtime.send_udp_to("/run/net0.sock", [10, 0, 2, 2], 5002, b"recovered-hw-udp") {
            Ok(_) => break,
            Err(Errno::Again) if recovered_attempt < 20_000 => core::hint::spin_loop(),
            Err(_) => return 632,
        }
    }
    if wait_for_udp_tx_completion(runtime, &baseline_device, &baseline_driver).is_err() {
        return 635;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::TxDrained).is_err() {
        return 636;
    }
    let socket = match runtime.inspect_network_socket("/run/net0.sock") {
        Ok(record) => record,
        Err(_) => return 637,
    };
    if write_line(
        runtime,
        &format!(
            "network.hw.udp.recovery socket=/run/net0.sock local={}:{} socket-tx={} outcome=ok",
            render_ipv4(socket.local_ipv4),
            socket.local_port,
            socket.tx_packets
        ),
    )
    .is_err()
    {
        return 638;
    }
    let _ = runtime.remove_network_events(queue_fd, "/dev/net0", None, 902);
    let _ = runtime.close(queue_fd);
    if write_line(runtime, "network-hardware-udp-tx-smoke-ok").is_err() {
        return 639;
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
            .map_err(|_| 640)?;
        let driver = runtime.inspect_driver("/drv/net0").map_err(|_| 641)?;
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
                Err(_) => Err(640),
            };
        }
        core::hint::spin_loop();
    }
    Err(642)
}
