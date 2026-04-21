use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn find_game_session<'a, B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &'a [GameCompatSession],
    pid: u64,
) -> Result<&'a GameCompatSession, ExitCode> {
    game_sessions
        .iter()
        .find(|session| session.pid == pid)
        .ok_or_else(|| {
            let _ = write_line(runtime, &format!("game.session-missing pid={pid}"));
            2
        })
}

pub fn find_game_session_mut<'a, B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &'a mut [GameCompatSession],
    pid: u64,
) -> Result<&'a mut GameCompatSession, ExitCode> {
    game_sessions
        .iter_mut()
        .find(|session| session.pid == pid)
        .ok_or_else(|| {
            let _ = write_line(runtime, &format!("game.session-missing pid={pid}"));
            2
        })
}

pub fn game_session_missing<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    let _ = write_line(runtime, &format!("game.session-missing pid={pid}"));
    Err(2)
}
