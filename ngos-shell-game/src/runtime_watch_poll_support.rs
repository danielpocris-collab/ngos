use alloc::format;

use ngos_game_compat_runtime::lane_name;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, shell_wait_event_queue, write_line};

pub fn game_poll_all_watches<B: SyscallBackend>(
    runtime: &Runtime<B>,
    sessions: &[GameCompatSession],
) -> Result<usize, ExitCode> {
    let mut polled = 0usize;
    for session in sessions {
        for lane in &session.lanes {
            let (Some(queue_fd), Some(token)) = (lane.watch_queue_fd, lane.watch_token) else {
                continue;
            };
            shell_wait_event_queue(runtime, queue_fd)?;
            write_line(
                runtime,
                &format!(
                    "game.watch.event pid={} slug={} kind={} queue={} token={}",
                    session.pid,
                    session.slug,
                    lane_name(lane.kind),
                    queue_fd,
                    token
                ),
            )?;
            polled = polled.saturating_add(1);
        }
    }
    if polled == 0 {
        return Err(299);
    }
    write_line(runtime, &format!("game.watch.poll count={polled}"))?;
    Ok(polled)
}
