use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{
    GameCompatSession, handle_game_gfx_driver_read, handle_game_gfx_next, handle_game_gfx_request,
    handle_game_gfx_status,
};

pub fn try_handle_game_graphics_observe_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("game-gfx-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_status(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-driver-read ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_driver_read(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-request ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_request(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-next ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_next(runtime, rest.trim(), game_sessions),
        ));
    }
    None
}
