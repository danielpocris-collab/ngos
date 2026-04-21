use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session, game_encode_frame, game_load_frame_script,
    parse_game_pid_script_args, write_line,
};

pub fn handle_game_gfx_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-gfx-plan <pid> <frame-script>",
    )?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let script = game_load_frame_script(runtime, &resolved)?;
    let encoded = game_encode_frame(session, &script)?;
    write_line(
        runtime,
        &format!(
            "game.gfx.plan pid={} frame={} ops={} bytes={} device={} profile={} queue={} present-mode={} completion={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.graphics_device_path,
            session.graphics_profile,
            encoded.queue,
            encoded.present_mode,
            encoded.completion
        ),
    )
}
