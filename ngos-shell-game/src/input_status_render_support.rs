use alloc::format;
use alloc::string::{String, ToString};

use ngos_user_abi::{NativeDeviceRecord, NativeDriverRecord};

use crate::GameCompatSession;

pub fn render_game_input_status(
    pid: u64,
    session: &GameCompatSession,
    claimed: bool,
    device: &NativeDeviceRecord,
    driver: &NativeDriverRecord,
) -> alloc::string::String {
    format!(
        "game.input.status pid={} device={} driver={} profile={} claimed={} token={} batches={} frame={} family={} layout={} key-table={} pointer-capture={} delivery={} delivery-observed={} ops={} bytes={} device-queue={}/{} device-submitted={} device-completed={} driver-queued={} driver-inflight={} driver-completed={}",
        pid,
        session.input_device_path,
        session.input_driver_path,
        session.input_profile,
        claimed,
        session
            .last_input_invoke_token
            .map(|token| token.to_string())
            .unwrap_or_else(|| String::from("pending")),
        session.submitted_input_batches,
        session
            .last_input_frame_tag
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_input_family
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_input_layout
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_input_key_table
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_pointer_capture
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_input_delivery_mode
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_input_delivery_observed
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session.last_input_op_count,
        session.last_input_payload_bytes,
        device.queue_depth,
        device.queue_capacity,
        device.submitted_requests,
        device.completed_requests,
        driver.queued_requests,
        driver.in_flight_requests,
        driver.completed_requests
    )
}
