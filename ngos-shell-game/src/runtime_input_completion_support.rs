use ngos_game_compat_runtime::CompatLaneKind;
use ngos_input_translate::EncodedInput;
use ngos_user_abi::{ExitCode, NativeEventQueueMode, POLLPRI, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_session_lane, shell_wait_event_queue};

pub fn game_submit_input_completion<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    encoded: &EncodedInput,
) -> Result<(usize, &'static str), ExitCode> {
    let lane = game_session_lane(session, CompatLaneKind::Input)?;
    let token = runtime.invoke_contract(lane.contract_id).map_err(|_| 297)?;
    let delivery_observed = match encoded.delivery.as_str() {
        "immediate" => "submitted",
        "wait-batch" | "wait-frame" => {
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
            if encoded.delivery == "wait-frame" {
                "frame-delivered"
            } else {
                "batch-delivered"
            }
        }
        _ => return Err(296),
    };
    Ok((token, delivery_observed))
}
