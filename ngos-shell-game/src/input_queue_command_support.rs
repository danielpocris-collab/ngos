use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, find_game_session_mut, parse_game_pid_arg, write_line};

pub fn handle_game_input_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-input-next <pid>")?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    if session.pending_input_batches.is_empty() {
        write_line(runtime, &format!("game.input.queue pid={} depth=0", pid))?;
        return Err(299);
    }
    let encoded = session.pending_input_batches.remove(0);
    write_line(
        runtime,
        &format!(
            "game.input.next pid={} frame={} family={} layout={} delivery={} remaining={} payload={}",
            pid,
            encoded.frame_tag,
            encoded.device_family,
            encoded.layout,
            encoded.delivery,
            session.pending_input_batches.len(),
            encoded.payload
        ),
    )
}
