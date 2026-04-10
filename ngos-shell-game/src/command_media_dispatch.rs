use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, try_handle_game_audio_command, try_handle_game_input_command};

pub fn try_handle_game_media_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    try_handle_game_audio_command(runtime, current_cwd, line, game_sessions, last_status).or_else(
        || try_handle_game_input_command(runtime, current_cwd, line, game_sessions, last_status),
    )
}
