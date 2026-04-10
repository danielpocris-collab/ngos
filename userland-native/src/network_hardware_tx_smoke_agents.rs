use super::*;

fn build_hardware_tx_frame() -> [u8; 60] {
    let mut frame = [0u8; 60];
    frame[0..6].copy_from_slice(&[0xff; 6]);
    frame[6..12].copy_from_slice(&[0x02, 0x47, 0x4f, 0x53, 0x54, 0x58]);
    frame[12..14].copy_from_slice(&[0x88, 0xb5]);
    frame[14..25].copy_from_slice(b"NGOS-HW-TX!");
    frame
}

#[inline(never)]
pub(crate) fn run_native_network_hardware_tx_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let baseline = match runtime.inspect_device("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 530,
    };
    let baseline_driver = match runtime.inspect_driver("/drv/net0") {
        Ok(record) => record,
        Err(_) => return 531,
    };
    let fd = match runtime.open_path("/dev/net0") {
        Ok(fd) => fd,
        Err(_) => return 532,
    };
    let ready = match runtime.poll(fd, POLLIN | POLLOUT) {
        Ok(value) => value,
        Err(_) => return 533,
    };
    if (ready & POLLOUT) == 0 {
        return 534;
    }
    match runtime.write(fd, b"short-frame") {
        Err(Errno::Inval) => {}
        _ => return 535,
    }
    if write_line(
        runtime,
        "network.hw.tx.refusal path=/dev/net0 errno=EINVAL outcome=expected",
    )
    .is_err()
    {
        return 536;
    }
    let frame = build_hardware_tx_frame();
    if runtime.write(fd, &frame).is_err() {
        return 537;
    }
    let target_submitted = baseline.submitted_requests.saturating_add(1);
    let target_completed = baseline.completed_requests.saturating_add(1);
    let mut device = baseline;
    let mut driver = baseline_driver;
    let mut observed = false;
    for _ in 0..20_000 {
        device = match runtime.inspect_device("/dev/net0") {
            Ok(record) => record,
            Err(_) => return 538,
        };
        driver = match runtime.inspect_driver("/drv/net0") {
            Ok(record) => record,
            Err(_) => return 539,
        };
        if device.submitted_requests >= target_submitted
            && device.completed_requests >= target_completed
            && driver.completed_requests >= target_completed
            && device.queue_depth == 0
        {
            observed = true;
            break;
        }
        core::hint::spin_loop();
    }
    if !observed {
        return 540;
    }
    let final_ready = match runtime.poll(fd, POLLIN | POLLOUT) {
        Ok(value) => value,
        Err(_) => return 541,
    };
    if (final_ready & POLLOUT) == 0 {
        return 542;
    }
    if write_line(
        runtime,
        &format!(
            "network.hw.tx.success path=/dev/net0 bytes={} submitted={} completed={} driver-completed={} queue-depth={} ready={:#x}",
            frame.len(),
            device.submitted_requests,
            device.completed_requests,
            driver.completed_requests,
            device.queue_depth,
            final_ready
        ),
    )
    .is_err()
    {
        return 543;
    }
    if write_line(runtime, "network-hardware-tx-smoke-ok").is_err() {
        return 544;
    }
    0
}
