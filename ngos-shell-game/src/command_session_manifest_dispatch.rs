use alloc::string::String;
use alloc::vec::Vec;

use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{
    GameCompatSession, handle_game_manifest, handle_game_plan, handle_game_session_profile,
};

pub fn try_handle_game_session_manifest_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    _jobs: &mut Vec<ShellJob>,
    _last_spawned_pid: &mut Option<u64>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(path) = line.strip_prefix("game-manifest ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_manifest(runtime, current_cwd, path.trim()),
        ));
    }
    if let Some(path) = line.strip_prefix("game-plan ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_plan(runtime, current_cwd, path.trim()),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-session-profile ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_session_profile(runtime, rest.trim(), game_sessions),
        ));
    }
    None
}
