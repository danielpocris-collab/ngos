use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_render_session, game_render_session_summary, write_line};

pub fn handle_game_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    if game_sessions.is_empty() {
        write_line(runtime, "game.sessions=0")?;
        return Ok(());
    }
    for session in game_sessions {
        game_render_session(runtime, session)?;
    }
    Ok(())
}

pub fn handle_game_sessions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    if game_sessions.is_empty() {
        write_line(runtime, "game.sessions=0")?;
        return Ok(());
    }
    for session in game_sessions {
        game_render_session_summary(runtime, session)?;
    }
    Ok(())
}

pub fn handle_game_loader_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    crate::session_loader_status_support::handle_game_loader_status(runtime, game_sessions)
}
