use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::{ExitCode, NativeEventQueueMode, POLLPRI, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, game_session_lane, game_session_lane_mut, game_watch_token,
    shell_wait_event_queue,
};

pub fn game_start_resource_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(usize, u64), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    if game_session_lane(session, kind)?.watch_queue_fd.is_some() {
        return Err(298);
    }
    let token = {
        let lane = game_session_lane(session, kind)?;
        game_watch_token(session, lane)
    };
    let queue_fd = runtime
        .create_event_queue(NativeEventQueueMode::Kqueue)
        .map_err(|_| 298)?;
    let lane = game_session_lane(session, kind)?;
    runtime
        .watch_resource_events(
            queue_fd,
            lane.resource_id,
            token,
            true,
            true,
            true,
            true,
            true,
            true,
            POLLPRI,
        )
        .map_err(|_| 298)?;
    let lane = game_session_lane_mut(session, kind)?;
    lane.watch_queue_fd = Some(queue_fd);
    lane.watch_token = Some(token);
    Ok((queue_fd, token))
}

pub fn game_stop_resource_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    let (queue_fd, token, resource_id) = {
        let lane = game_session_lane(session, kind)?;
        (
            lane.watch_queue_fd.ok_or(299)?,
            lane.watch_token.ok_or(299)?,
            lane.resource_id,
        )
    };
    runtime
        .remove_resource_events(queue_fd, resource_id, token)
        .map_err(|_| 299)?;
    runtime.close(queue_fd).map_err(|_| 240)?;
    let lane = game_session_lane_mut(session, kind)?;
    lane.watch_queue_fd = None;
    lane.watch_token = None;
    Ok(())
}

pub fn game_wait_resource_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    let lane = game_session_lane(session, kind)?;
    let queue_fd = lane.watch_queue_fd.ok_or(299)?;
    shell_wait_event_queue(runtime, queue_fd)
}
