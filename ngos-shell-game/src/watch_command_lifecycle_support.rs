use alloc::format;

use ngos_game_compat_runtime::lane_name;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::watch_command_args_support::parse_game_pid_lane_args;
use crate::{
    GameCompatSession, game_session_missing, game_start_watch, game_stop_watch, game_wait_watch,
    write_line,
};

pub fn handle_game_watch_start<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-start <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter_mut().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    let (queue_fd, token) = game_start_watch(runtime, session, kind)?;
    write_line(
        runtime,
        &format!(
            "game.watch.start pid={} kind={} queue={} token={}",
            pid,
            lane_name(kind),
            queue_fd,
            token
        ),
    )
}

pub fn handle_game_watch_wait<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-wait <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    game_wait_watch(runtime, session, kind)
}

pub fn handle_game_watch_stop<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-stop <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter_mut().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    game_stop_watch(runtime, session, kind)?;
    write_line(
        runtime,
        &format!("game.watch.stop pid={} kind={}", pid, lane_name(kind)),
    )
}
