use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{
    GameCompatSession, handle_game_gfx_plan, handle_game_gfx_submit, handle_game_gfx_translate,
};

pub fn try_handle_game_graphics_submit_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("game-gfx-plan ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_plan(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-submit ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_submit(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-translate ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_translate(runtime, current_cwd, rest, game_sessions),
        ));
    }
    None
}
