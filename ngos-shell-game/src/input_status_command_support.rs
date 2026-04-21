use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session, game_session_lane, parse_game_pid_arg,
    render_game_input_status, write_line,
};

pub fn handle_game_input_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-input-status <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let lane = game_session_lane(session, CompatLaneKind::Input)?;
    let device = runtime
        .inspect_device(&session.input_device_path)
        .map_err(|_| 246)?;
    let driver = runtime
        .inspect_driver(&session.input_driver_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &render_game_input_status(pid, session, lane.claim_acquired, &device, &driver),
    )
}
