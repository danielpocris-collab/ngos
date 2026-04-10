use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_next_payload, parse_game_pid_arg, write_line};

pub fn handle_game_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-next <pid>")?;
    let session = crate::find_game_session_mut(runtime, game_sessions, pid)?;
    match game_next_payload(runtime, session) {
        Ok(()) => Ok(()),
        Err(code) => {
            if code == 299 {
                write_line(
                    runtime,
                    &format!(
                        "game.next pid={} depth[gfx={};audio={};input={}]",
                        session.pid,
                        session.pending_graphics_frames.len(),
                        session.pending_audio_batches.len(),
                        session.pending_input_batches.len()
                    ),
                )?;
            }
            Err(code)
        }
    }
}
