use alloc::format;
use alloc::string::String;

use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::ExitCode;

use crate::{GameCompatSession, game_session_lane};

pub fn render_game_gfx_session_status(session: &GameCompatSession) -> Result<String, ExitCode> {
    let lane = game_session_lane(session, CompatLaneKind::Graphics)?;
    Ok(format!(
        "pid={} device={} driver={} api={} backend={} translation={} profile={} claimed={} submitted={} frames={} presented={} last-frame={} queue={} present-mode={} completion={} completion-observed={} deep-ops={} ops={} bytes={}",
        session.pid,
        session.graphics_device_path,
        session.graphics_driver_path,
        session.graphics_translation.source_api_name,
        session.graphics_translation.backend_name,
        session.graphics_translation.translation,
        session.graphics_profile,
        lane.claim_acquired,
        session.submitted_frames,
        session.presented_frames,
        session.last_presented,
        session
            .last_frame_tag
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_graphics_queue
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_present_mode
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_completion_mode
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_completion_observed
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session
            .last_graphics_deep_ops
            .clone()
            .unwrap_or_else(|| String::from("-")),
        session.last_frame_op_count,
        session.last_frame_payload_bytes
    ))
}
