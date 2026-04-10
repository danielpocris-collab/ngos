use alloc::string::String;

use ngos_audio_translate::EncodedMix;
use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::ExitCode;

use crate::{GameCompatSession, game_session_lane_mut};

pub fn game_record_submitted_mix(
    session: &mut GameCompatSession,
    encoded: &EncodedMix,
    token: usize,
    completion_observed: &'static str,
) -> Result<(), ExitCode> {
    let lane = game_session_lane_mut(session, CompatLaneKind::Audio)?;
    lane.invoke_token = Some(token);
    session.last_audio_stream_tag = Some(encoded.stream_tag.clone());
    session.last_audio_route = Some(encoded.route.clone());
    session.last_audio_latency_mode = Some(encoded.latency_mode.clone());
    session.last_audio_spatialization = Some(encoded.spatialization.clone());
    session.last_audio_completion_mode = Some(encoded.completion.clone());
    session.last_audio_completion_observed = Some(String::from(completion_observed));
    session.last_audio_op_count = encoded.op_count;
    session.last_audio_payload_bytes = encoded.payload.len();
    session.submitted_audio_batches = session.submitted_audio_batches.saturating_add(1);
    session.last_audio_invoke_token = Some(token);
    session.pending_audio_batches.push(encoded.clone());
    Ok(())
}
