use ngos_gfx_translate::EncodedFrame;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, game_publish_runtime_payload, game_record_submitted_frame,
    game_submit_frame_completion, shell_gpu_submit,
};

pub fn game_submit_frame<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    encoded: &EncodedFrame,
) -> Result<(bool, &'static str), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    shell_gpu_submit(runtime, &session.graphics_device_path, &encoded.payload)?;
    let (presented, completion_observed) = game_submit_frame_completion(runtime, session, encoded)?;
    game_record_submitted_frame(session, encoded, presented, completion_observed);
    game_publish_runtime_payload(
        runtime,
        &session.runtime_channel_path,
        "graphics",
        &encoded.frame_tag,
        encoded.payload.as_bytes(),
    )?;
    Ok((presented, completion_observed))
}
