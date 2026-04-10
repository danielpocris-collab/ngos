use alloc::string::String;
use alloc::vec::Vec;

use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, try_handle_game_graphics_command, try_handle_game_media_command,
    try_handle_game_session_command, try_handle_game_watch_command,
};

pub fn try_handle_game_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    jobs: &mut Vec<ShellJob>,
    last_spawned_pid: &mut Option<u64>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    try_handle_game_session_command(
        runtime,
        current_cwd,
        line,
        game_sessions,
        jobs,
        last_spawned_pid,
        last_status,
    )
    .or_else(|| {
        try_handle_game_graphics_command(runtime, current_cwd, line, game_sessions, last_status)
    })
    .or_else(|| {
        try_handle_game_media_command(runtime, current_cwd, line, game_sessions, last_status)
    })
    .or_else(|| try_handle_game_watch_command(runtime, line, game_sessions, last_status))
}

pub(crate) fn settle_game_command_status(
    last_status: &mut ExitCode,
    result: Result<(), ExitCode>,
) -> Result<(), ExitCode> {
    *last_status = match result {
        Ok(()) => 0,
        Err(code) => code,
    };
    Ok(())
}
