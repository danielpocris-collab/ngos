use alloc::format;

use ngos_shell_proc::fixed_text_field;
use ngos_user_abi::NativeDriverRecord;

use crate::gpu_request_state_name;

pub fn render_game_gfx_driver_status(driver: &NativeDriverRecord) -> alloc::string::String {
    format!(
        "driver-queued={} driver-inflight={} driver-completed={} driver-last-request={} driver-last-frame={} driver-last-api={} driver-last-translation={} driver-last-terminal-request={} driver-last-terminal-state={} driver-last-terminal-frame={} driver-last-terminal-api={} driver-last-terminal-translation={}",
        driver.queued_requests,
        driver.in_flight_requests,
        driver.completed_requests,
        driver.last_completed_request_id,
        fixed_text_field(&driver.last_completed_frame_tag),
        fixed_text_field(&driver.last_completed_source_api_name),
        fixed_text_field(&driver.last_completed_translation_label),
        driver.last_terminal_request_id,
        gpu_request_state_name(driver.last_terminal_state),
        fixed_text_field(&driver.last_terminal_frame_tag),
        fixed_text_field(&driver.last_terminal_source_api_name),
        fixed_text_field(&driver.last_terminal_translation_label)
    )
}
