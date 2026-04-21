use alloc::string::String;
use alloc::vec::Vec;

use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{GameCompatSession, handle_game_launch, handle_game_relaunch, handle_game_stop};

pub fn try_handle_game_session_lifecycle_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    jobs: &mut Vec<ShellJob>,
    last_spawned_pid: &mut Option<u64>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(path) = line.strip_prefix("game-launch ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_launch(
                runtime,
                current_cwd,
                path.trim(),
                game_sessions,
                jobs,
                last_spawned_pid,
            ),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-stop ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_stop(runtime, rest.trim(), game_sessions, jobs),
        ));
    }
    if let Some(path) = line.strip_prefix("game-relaunch ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_relaunch(
                runtime,
                current_cwd,
                path.trim(),
                game_sessions,
                jobs,
                last_spawned_pid,
            ),
        ));
    }
    None
}
