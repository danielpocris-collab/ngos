use alloc::format;
use alloc::string::{String, ToString};

use ngos_user_abi::{NativeDeviceRecord, NativeDriverRecord};

use crate::GameCompatSession;

pub fn render_game_audio_status(
    pid: u64,
    session: &GameCompatSession,
    claimed: bool,
    device: &NativeDeviceRecord,
    driver: &NativeDriverRecord,
) -> alloc::string::String {
    format!(
        "game.audio.status pid={} device={} driver={} profile={} claimed={} token={} batches={} stream={} route={} latency-mode={} spatialization={} completion={} completion-observed={} ops={} bytes={} device-queue={}/{} device-submitted={} device-completed={} driver-queued={} driver-inflight={} driver-completed={}",
        pid,
        session.audio_device_path,
        session.audio_driver_path,
        session.audio_profile,
        claimed,
        session
            .last_audio_invoke_token
            .map(|token| token.to_string())
            .unwrap_or_else(|| String::from("pending")),
        session.submitted_audio_batches,
        session
            .last_audio_stream_tag
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_audio_route
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_audio_latency_mode
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_audio_spatialization
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_audio_completion_mode
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_audio_completion_observed
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session.last_audio_op_count,
        session.last_audio_payload_bytes,
        device.queue_depth,
        device.queue_capacity,
        device.submitted_requests,
        device.completed_requests,
        driver.queued_requests,
        driver.in_flight_requests,
        driver.completed_requests
    )
}
