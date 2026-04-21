use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_render_session, game_stop_session, parse_game_pid_arg};

pub fn handle_game_stop<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
    jobs: &mut [ShellJob],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-stop <pid>")?;
    let Some(session) = game_sessions.iter_mut().find(|session| session.pid == pid) else {
        return crate::game_session_missing(runtime, pid);
    };
    game_stop_session(runtime, session)?;
    if let Some(job) = jobs.iter_mut().find(|job| job.pid == pid) {
        job.reaped_exit = session.exit_code;
    }
    game_render_session(runtime, session)
}
