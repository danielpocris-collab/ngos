use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_cleanup_session_artifacts, game_stop_session};

pub fn game_stop_sessions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    sessions: &mut [&mut GameCompatSession],
) -> Result<(), ExitCode> {
    for session in sessions {
        game_stop_session(runtime, session)?;
    }
    Ok(())
}

pub fn game_cleanup_sessions_and_paths<B: SyscallBackend>(
    runtime: &Runtime<B>,
    sessions: &[&GameCompatSession],
    generated_paths: &[&str],
) {
    for session in sessions {
        game_cleanup_session_artifacts(runtime, session);
    }
    game_cleanup_generated_paths(runtime, generated_paths);
}

pub fn game_cleanup_generated_paths<B: SyscallBackend>(runtime: &Runtime<B>, paths: &[&str]) {
    for path in paths {
        let _ = runtime.unlink_path(path);
    }
}
