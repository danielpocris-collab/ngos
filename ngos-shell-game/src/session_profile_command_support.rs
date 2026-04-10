use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::session_profile_render_support::render_game_session_profile;
use crate::{GameCompatSession, find_game_session, parse_game_pid_arg};

pub fn handle_game_session_profile<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-session-profile <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    render_game_session_profile(runtime, session)
}
