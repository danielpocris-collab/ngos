use alloc::string::String;

use ngos_game_compat_runtime::CompatLaneKind;
use ngos_input_translate::EncodedInput;
use ngos_user_abi::ExitCode;

use crate::{GameCompatSession, game_session_lane_mut};

pub fn game_record_submitted_input(
    session: &mut GameCompatSession,
    encoded: &EncodedInput,
    token: usize,
    delivery_observed: &'static str,
) -> Result<(), ExitCode> {
    let lane = game_session_lane_mut(session, CompatLaneKind::Input)?;
    lane.invoke_token = Some(token);
    session.last_input_frame_tag = Some(encoded.frame_tag.clone());
    session.last_input_family = Some(encoded.device_family.clone());
    session.last_input_layout = Some(encoded.layout.clone());
    session.last_input_key_table = Some(encoded.key_table.clone());
    session.last_pointer_capture = Some(encoded.pointer_capture.clone());
    session.last_input_delivery_mode = Some(encoded.delivery.clone());
    session.last_input_delivery_observed = Some(String::from(delivery_observed));
    session.last_input_op_count = encoded.op_count;
    session.last_input_payload_bytes = encoded.payload.len();
    session.submitted_input_batches = session.submitted_input_batches.saturating_add(1);
    session.last_input_invoke_token = Some(token);
    session.pending_input_batches.push(encoded.clone());
    Ok(())
}
