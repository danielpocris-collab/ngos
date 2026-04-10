use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;

pub fn try_handle_game_graphics_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    crate::try_handle_game_graphics_submit_command(
        runtime,
        current_cwd,
        line,
        game_sessions,
        last_status,
    )
    .or_else(|| {
        crate::try_handle_game_graphics_observe_command(runtime, line, game_sessions, last_status)
    })
}
