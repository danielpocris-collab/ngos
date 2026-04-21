use alloc::string::String;
use alloc::vec::Vec;

use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{
    GameCompatSession, handle_game_abi_status, handle_game_loader_status, handle_game_sessions,
    handle_game_status,
};

pub fn try_handle_game_session_status_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    _current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    _jobs: &mut Vec<ShellJob>,
    _last_spawned_pid: &mut Option<u64>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if line == "game-status" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_status(runtime, game_sessions),
        ));
    }
    if line == "game-sessions" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_sessions(runtime, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-abi-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_abi_status(runtime, rest.trim(), game_sessions),
        ));
    }
    if line == "game-loader-status" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_loader_status(runtime, game_sessions),
        ));
    }
    None
}
