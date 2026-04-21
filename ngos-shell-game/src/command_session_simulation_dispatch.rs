use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{GameCompatSession, handle_game_next, handle_game_simulate};

pub fn try_handle_game_session_simulation_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    _jobs: &mut Vec<ShellJob>,
    _last_spawned_pid: &mut Option<u64>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("game-next ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_next(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-simulate ") {
        let mut parts = rest.split_whitespace();
        let slug = parts.next()?.to_string();
        let frame_count = parts
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(60);
        return Some(settle_game_command_status(
            last_status,
            handle_game_simulate(runtime, current_cwd, game_sessions, &slug, frame_count),
        ));
    }
    None
}
