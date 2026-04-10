use alloc::string::String;
use alloc::vec::Vec;

use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, try_handle_game_session_lifecycle_command,
    try_handle_game_session_manifest_command, try_handle_game_session_simulation_command,
    try_handle_game_session_status_command,
};

pub fn try_handle_game_session_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    jobs: &mut Vec<ShellJob>,
    last_spawned_pid: &mut Option<u64>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    try_handle_game_session_manifest_command(
        runtime,
        current_cwd,
        line,
        game_sessions,
        jobs,
        last_spawned_pid,
        last_status,
    )
    .or_else(|| {
        try_handle_game_session_lifecycle_command(
            runtime,
            current_cwd,
            line,
            game_sessions,
            jobs,
            last_spawned_pid,
            last_status,
        )
    })
    .or_else(|| {
        try_handle_game_session_status_command(
            runtime,
            current_cwd,
            line,
            game_sessions,
            jobs,
            last_spawned_pid,
            last_status,
        )
    })
    .or_else(|| {
        try_handle_game_session_simulation_command(
            runtime,
            current_cwd,
            line,
            game_sessions,
            jobs,
            last_spawned_pid,
            last_status,
        )
    })
}
