use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;

pub fn retained_gfx_request_id<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<Option<(u64, bool)>, ExitCode> {
    let device = runtime
        .inspect_device(&session.graphics_device_path)
        .map_err(|_| 246)?;
    let driver = runtime
        .inspect_driver(&session.graphics_driver_path)
        .map_err(|_| 246)?;
    if driver.queued_requests == 0 && driver.in_flight_requests == 0 {
        let from_driver = driver.last_terminal_request_id != 0;
        let retained = if from_driver {
            driver.last_terminal_request_id
        } else {
            device.last_terminal_request_id
        };
        if retained != 0 {
            return Ok(Some((retained, from_driver)));
        }
    }
    Ok(None)
}
