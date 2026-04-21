use super::*;

fn build_hardware_interface_frame() -> [u8; 60] {
    let mut frame = [0u8; 60];
    frame[0..6].copy_from_slice(&[0xff; 6]);
    frame[6..12].copy_from_slice(&[0; 6]);
    frame[12..14].copy_from_slice(&[0x88, 0xb5]);
    frame[14..28].copy_from_slice(b"NGOS-HW-IFACE!");
    frame
}

#[inline(never)]
pub(crate) fn run_native_network_hardware_interface_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    if runtime
        .configure_network_interface_ipv4(
            "/dev/net0",
            [10, 0, 2, 15],
            [255, 255, 255, 0],
            [10, 0, 2, 2],
        )
        .is_err()
    {
        return 570;
    }
    if runtime
        .configure_network_interface_admin("/dev/net0", 1500, 8, 8, 4, true, false)
        .is_err()
    {
        return 571;
    }
    let iface = match runtime.inspect_network_interface("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 572,
    };
    if iface.admin_up == 0
        || iface.link_up == 0
        || iface.mtu != 1500
        || iface.tx_capacity != 8
        || iface.rx_capacity != 8
        || iface.tx_inflight_limit != 4
        || iface.ipv4_addr != [10, 0, 2, 15]
        || iface.ipv4_gateway != [10, 0, 2, 2]
    {
        return 573;
    }
    if write_line(
        runtime,
        &format!(
            "network.hw.iface.inspect path=/dev/net0 admin={} link={} mtu={} addr={}.{}.{}.{} gw={}.{}.{}.{} tx-cap={} rx-cap={} inflight-limit={}",
            iface.admin_up,
            iface.link_up,
            iface.mtu,
            iface.ipv4_addr[0],
            iface.ipv4_addr[1],
            iface.ipv4_addr[2],
            iface.ipv4_addr[3],
            iface.ipv4_gateway[0],
            iface.ipv4_gateway[1],
            iface.ipv4_gateway[2],
            iface.ipv4_gateway[3],
            iface.tx_capacity,
            iface.rx_capacity,
            iface.tx_inflight_limit
        ),
    )
    .is_err()
    {
        return 574;
    }

    let queue_fd = match runtime.create_event_queue(NativeEventQueueMode::Epoll) {
        Ok(fd) => fd,
        Err(_) => return 575,
    };
    if runtime
        .watch_network_events(
            queue_fd,
            "/dev/net0",
            None,
            901,
            true,
            false,
            false,
            POLLPRI,
        )
        .is_err()
    {
        return 576;
    }
    let fd = match runtime.open_path("/dev/net0") {
        Ok(fd) => fd,
        Err(_) => return 577,
    };
    if runtime
        .set_network_interface_link_state("/dev/net0", false)
        .is_err()
    {
        return 578;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged).is_err() {
        return 579;
    }
    match runtime.write(fd, &build_hardware_interface_frame()) {
        Err(Errno::Access) => {}
        _ => return 580,
    }
    if write_line(
        runtime,
        "network.hw.iface.refusal path=/dev/net0 state=link-down errno=EACCES outcome=expected",
    )
    .is_err()
    {
        return 581;
    }
    if runtime
        .set_network_interface_link_state("/dev/net0", true)
        .is_err()
    {
        return 582;
    }
    if wait_for_network_event(runtime, queue_fd, NativeNetworkEventKind::LinkChanged).is_err() {
        return 583;
    }
    if runtime
        .write(fd, &build_hardware_interface_frame())
        .is_err()
    {
        return 584;
    }

    let mut final_iface = iface;
    let mut observed = false;
    for _ in 0..20_000 {
        final_iface = match runtime.inspect_network_interface("/dev/net0") {
            Ok(record) => record,
            Err(_) => return 585,
        };
        if final_iface.link_up != 0 && final_iface.tx_packets >= iface.tx_packets.saturating_add(1)
        {
            observed = true;
            break;
        }
        core::hint::spin_loop();
    }
    if !observed {
        return 586;
    }
    if write_line(
        runtime,
        &format!(
            "network.hw.iface.success path=/dev/net0 admin={} link={} tx-packets={} tx-completions={} tx-inflight-depth={} tx-ring={} rx-ring={}",
            final_iface.admin_up,
            final_iface.link_up,
            final_iface.tx_packets,
            final_iface.tx_completions,
            final_iface.tx_inflight_depth,
            final_iface.tx_ring_depth,
            final_iface.rx_ring_depth
        ),
    )
    .is_err()
    {
        return 587;
    }
    if write_line(runtime, "network-hardware-interface-smoke-ok").is_err() {
        return 588;
    }
    0
}
