use alloc::format;

use ngos_shell_proc::fixed_text_field;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, gpu_request_state_name, write_line};

pub fn write_retained_gfx_driver_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    let device = runtime
        .inspect_device(&session.graphics_device_path)
        .map_err(|_| 246)?;
    let driver = runtime
        .inspect_driver(&session.graphics_driver_path)
        .map_err(|_| 246)?;
    let retained_request_id = if driver.last_terminal_request_id != 0 {
        driver.last_terminal_request_id
    } else {
        device.last_terminal_request_id
    };
    write_line(
        runtime,
        &format!(
            "game.gfx.driver-read pid={} driver={} api={} translation={} outcome=retained request={} state={} frame={} request-api={} request-translation={}",
            pid,
            session.graphics_driver_path,
            session.graphics_translation.source_api_name,
            session.graphics_translation.translation,
            retained_request_id,
            if driver.last_terminal_request_id != 0 {
                gpu_request_state_name(driver.last_terminal_state)
            } else {
                gpu_request_state_name(device.last_terminal_state)
            },
            if driver.last_terminal_request_id != 0 {
                fixed_text_field(&driver.last_terminal_frame_tag)
            } else {
                fixed_text_field(&device.last_terminal_frame_tag)
            },
            if driver.last_terminal_request_id != 0 {
                fixed_text_field(&driver.last_terminal_source_api_name)
            } else {
                fixed_text_field(&device.last_terminal_source_api_name)
            },
            if driver.last_terminal_request_id != 0 {
                fixed_text_field(&driver.last_terminal_translation_label)
            } else {
                fixed_text_field(&device.last_terminal_translation_label)
            }
        ),
    )
}
