use alloc::string::String;

use ngos_gfx_translate::EncodedFrame;

use crate::{GameCompatSession, summarize_graphics_deep_ops};

pub fn game_record_submitted_frame(
    session: &mut GameCompatSession,
    encoded: &EncodedFrame,
    presented: bool,
    completion_observed: &str,
) {
    session.last_frame_tag = Some(encoded.frame_tag.clone());
    session.last_graphics_queue = Some(encoded.queue.clone());
    session.last_present_mode = Some(encoded.present_mode.clone());
    session.last_completion_mode = Some(encoded.completion.clone());
    session.last_completion_observed = Some(String::from(completion_observed));
    session.last_frame_op_count = encoded.op_count;
    session.last_frame_payload_bytes = encoded.payload.len();
    session.last_graphics_deep_ops = Some(summarize_graphics_deep_ops(&encoded.payload));
    session.submitted_frames = session.submitted_frames.saturating_add(1);
    if presented {
        session.presented_frames = session.presented_frames.saturating_add(1);
    }
    session.last_presented = presented;
    session.pending_graphics_frames.push(encoded.clone());
}
