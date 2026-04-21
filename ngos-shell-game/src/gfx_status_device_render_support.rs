use alloc::format;

use ngos_shell_proc::fixed_text_field;
use ngos_user_abi::NativeDeviceRecord;

use crate::gpu_request_state_name;

pub fn render_game_gfx_device_status(device: &NativeDeviceRecord) -> alloc::string::String {
    format!(
        "device-queue={}/{} device-submitted={} device-completed={} device-last-request={} device-last-frame={} device-last-api={} device-last-translation={} device-last-terminal-request={} device-last-terminal-state={} device-last-terminal-frame={} device-last-terminal-api={} device-last-terminal-translation={}",
        device.queue_depth,
        device.queue_capacity,
        device.submitted_requests,
        device.completed_requests,
        device.last_completed_request_id,
        fixed_text_field(&device.last_completed_frame_tag),
        fixed_text_field(&device.last_completed_source_api_name),
        fixed_text_field(&device.last_completed_translation_label),
        device.last_terminal_request_id,
        gpu_request_state_name(device.last_terminal_state),
        fixed_text_field(&device.last_terminal_frame_tag),
        fixed_text_field(&device.last_terminal_source_api_name),
        fixed_text_field(&device.last_terminal_translation_label)
    )
}
