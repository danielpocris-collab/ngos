use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{
    GameCompatSession, handle_game_input_next, handle_game_input_plan, handle_game_input_status,
    handle_game_input_submit, handle_game_input_translate,
};

pub fn try_handle_game_input_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("game-input-plan ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_plan(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-submit ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_submit(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_status(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-next ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_next(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-translate ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_translate(runtime, current_cwd, rest, game_sessions),
        ));
    }
    None
}
