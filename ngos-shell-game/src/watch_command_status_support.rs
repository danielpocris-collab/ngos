use alloc::format;
use alloc::string::{String, ToString};

use ngos_game_compat_runtime::lane_name;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::watch_command_args_support::parse_game_pid_lane_args;
use crate::{
    GameCompatSession, game_poll_all_watches, game_render_watch_summary, game_session_lane,
    game_session_missing, write_line,
};

pub fn handle_game_watch_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-status <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    let lane = game_session_lane(session, kind)?;
    write_line(
        runtime,
        &format!(
            "game.watch.status pid={} kind={} queue={} token={}",
            pid,
            lane_name(kind),
            lane.watch_queue_fd
                .map(|fd| fd.to_string())
                .unwrap_or_else(|| String::from("inactive")),
            lane.watch_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("inactive"))
        ),
    )
}

pub fn handle_game_watch_status_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    if game_sessions.is_empty() {
        write_line(runtime, "game.watch.sessions=0")?;
        return Ok(());
    }
    for session in game_sessions {
        game_render_watch_summary(runtime, session)?;
    }
    Ok(())
}

pub fn handle_game_watch_poll_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    game_poll_all_watches(runtime, game_sessions).map(|_| ())
}
