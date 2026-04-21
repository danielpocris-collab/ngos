use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::command_dispatch_support::settle_game_command_status;
use crate::{
    GameCompatSession, handle_game_watch_poll_all, handle_game_watch_start,
    handle_game_watch_status, handle_game_watch_status_all, handle_game_watch_stop,
    handle_game_watch_wait,
};

pub fn try_handle_game_watch_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("game-watch-start ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_start(runtime, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-watch-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_status(runtime, rest, game_sessions),
        ));
    }
    if line == "game-watch-status-all" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_status_all(runtime, game_sessions),
        ));
    }
    if line == "game-watch-poll-all" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_poll_all(runtime, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-watch-wait ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_wait(runtime, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-watch-stop ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_stop(runtime, rest, game_sessions),
        ));
    }
    None
}
