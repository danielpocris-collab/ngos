use ngos_audio_translate::EncodedMix;
use ngos_user_abi::{ExitCode, NativeEventQueueMode, POLLPRI, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_session_lane, shell_wait_event_queue};

pub fn game_submit_mix_completion<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    encoded: &EncodedMix,
) -> Result<(usize, &'static str), ExitCode> {
    let lane = game_session_lane(session, ngos_game_compat_runtime::CompatLaneKind::Audio)?;
    let token = runtime.invoke_contract(lane.contract_id).map_err(|_| 294)?;
    let completion_observed = match encoded.completion.as_str() {
        "fire-and-forget" => "submitted",
        "wait-batch" | "wait-drain" => {
            let queue_fd = runtime
                .create_event_queue(NativeEventQueueMode::Kqueue)
                .map_err(|_| 298)?;
            let watch_token = ((session.pid & 0xffff_ffff) << 32) | (lane.resource_id as u64);
            runtime
                .watch_resource_events(
                    queue_fd,
                    lane.resource_id,
                    watch_token,
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    POLLPRI,
                )
                .map_err(|_| 298)?;
            shell_wait_event_queue(runtime, queue_fd)?;
            runtime
                .remove_resource_events(queue_fd, lane.resource_id, watch_token)
                .map_err(|_| 299)?;
            if encoded.completion == "wait-drain" {
                "resource-drained"
            } else {
                "batch-waited"
            }
        }
        _ => return Err(292),
    };
    Ok((token, completion_observed))
}
