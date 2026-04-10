use super::*;

fn build_hardware_arp_request() -> [u8; 60] {
    let mut frame = [0u8; 60];
    frame[0..6].copy_from_slice(&[0xff; 6]);
    frame[6..12].copy_from_slice(&[0; 6]);
    frame[12..14].copy_from_slice(&[0x08, 0x06]);
    frame[14..16].copy_from_slice(&[0x00, 0x01]);
    frame[16..18].copy_from_slice(&[0x08, 0x00]);
    frame[18] = 6;
    frame[19] = 4;
    frame[20..22].copy_from_slice(&[0x00, 0x01]);
    frame[22..28].copy_from_slice(&[0; 6]);
    frame[28..32].copy_from_slice(&[10, 0, 2, 15]);
    frame[32..38].copy_from_slice(&[0; 6]);
    frame[38..42].copy_from_slice(&[10, 0, 2, 2]);
    frame
}

fn matches_gateway_arp_reply(frame: &[u8]) -> bool {
    frame.len() >= 42
        && frame[12] == 0x08
        && frame[13] == 0x06
        && frame[20] == 0x00
        && frame[21] == 0x02
        && frame[28..32] == [10, 0, 2, 2]
}

#[inline(never)]
pub(crate) fn run_native_network_hardware_rx_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let baseline = match runtime.inspect_device("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 550,
    };
    let fd = match runtime.open_path("/dev/net0") {
        Ok(fd) => fd,
        Err(_) => return 551,
    };
    if runtime
        .fcntl(fd, FcntlCmd::SetFl { nonblock: true })
        .is_err()
    {
        return 552;
    }
    let mut empty = [0u8; 1514];
    match runtime.read(fd, &mut empty) {
        Err(Errno::Again) => {}
        _ => return 553,
    }
    if write_line(
        runtime,
        "network.hw.rx.refusal path=/dev/net0 errno=EAGAIN outcome=expected",
    )
    .is_err()
    {
        return 554;
    }

    let request = build_hardware_arp_request();
    if runtime.write(fd, &request).is_err() {
        return 555;
    }

    let mut ready = 0u32;
    let mut buffer = [0u8; 1514];
    let mut matched_len = 0usize;
    let mut matched_ethertype = 0u16;
    let mut observed = false;
    for _ in 0..20_000 {
        ready = match runtime.poll(fd, POLLIN | POLLOUT) {
            Ok(value) => value,
            Err(_) => return 556,
        };
        if (ready & POLLIN) == 0 {
            core::hint::spin_loop();
            continue;
        }
        loop {
            match runtime.read(fd, &mut buffer) {
                Ok(count) => {
                    if count == 0 {
                        break;
                    }
                    matched_len = count;
                    matched_ethertype = u16::from_be_bytes([buffer[12], buffer[13]]);
                    if matches_gateway_arp_reply(&buffer[..count]) {
                        observed = true;
                        break;
                    }
                }
                Err(Errno::Again) => break,
                Err(_) => return 557,
            }
        }
        if observed {
            break;
        }
    }
    if !observed {
        return 558;
    }

    let final_ready = match runtime.poll(fd, POLLIN | POLLOUT) {
        Ok(value) => value,
        Err(_) => return 559,
    };
    let device = match runtime.inspect_device("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 560,
    };
    if device.submitted_requests < baseline.submitted_requests.saturating_add(1)
        || device.completed_requests < baseline.completed_requests.saturating_add(1)
    {
        return 561;
    }
    if write_line(
        runtime,
        &format!(
            "network.hw.rx.success path=/dev/net0 bytes={} ethertype=0x{:04x} submitted={} completed={} ready-before={:#x} ready-after={:#x}",
            matched_len,
            matched_ethertype,
            device.submitted_requests,
            device.completed_requests,
            ready,
            final_ready
        ),
    )
    .is_err()
    {
        return 562;
    }
    if write_line(runtime, "network-hardware-rx-smoke-ok").is_err() {
        return 563;
    }
    0
}
