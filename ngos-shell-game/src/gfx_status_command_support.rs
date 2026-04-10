use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session, parse_game_pid_arg, render_game_gfx_device_status,
    render_game_gfx_driver_status, render_game_gfx_session_status, write_line,
};

pub fn handle_game_gfx_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-gfx-status <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let device = runtime
        .inspect_device(&session.graphics_device_path)
        .map_err(|_| 246)?;
    let driver = runtime
        .inspect_driver(&session.graphics_driver_path)
        .map_err(|_| 246)?;
    let session_text = render_game_gfx_session_status(session)?;
    let device_text = render_game_gfx_device_status(&device);
    let driver_text = render_game_gfx_driver_status(&driver);
    write_line(
        runtime,
        &format!(
            "game.gfx.status {} {} {}",
            session_text, device_text, driver_text
        ),
    )
}
