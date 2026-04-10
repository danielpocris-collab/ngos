use super::*;

#[inline(never)]
pub(crate) fn run_native_network_hardware_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let device = match runtime.inspect_device("/dev/net0") {
        Ok(record) => record,
        Err(_) => return 520,
    };
    if device.class != 6 || device.link_up == 0 || device.submitted_requests == 0 {
        return 521;
    }
    let driver = match runtime.inspect_driver("/drv/net0") {
        Ok(record) => record,
        Err(_) => return 522,
    };
    if driver.bound_device_count != 1
        || driver.completed_requests > device.submitted_requests
        || driver.completed_requests != device.completed_requests
    {
        return 523;
    }
    if write_line(
        runtime,
        &format!(
            "network.hw.device path=/dev/net0 class={} link={} queue-depth={} queue-capacity={} submitted={} completed={} last-terminal-state={}",
            device.class,
            if device.link_up != 0 { "up" } else { "down" },
            device.queue_depth,
            device.queue_capacity,
            device.submitted_requests,
            device.completed_requests,
            device.last_terminal_state
        ),
    )
    .is_err()
    {
        return 524;
    }
    if write_line(
        runtime,
        &format!(
            "network.hw.driver path=/drv/net0 bound={} queued={} inflight={} completed={} last-terminal-state={}",
            driver.bound_device_count,
            driver.queued_requests,
            driver.in_flight_requests,
            driver.completed_requests,
            driver.last_terminal_state
        ),
    )
    .is_err()
    {
        return 525;
    }
    if write_line(runtime, "network-hardware-smoke-ok").is_err() {
        return 526;
    }
    0
}
