use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session_mut, game_encode_frame, game_load_frame_script,
    game_submit_frame, parse_game_pid_script_args, write_line,
};

pub fn handle_game_gfx_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-gfx-submit <pid> <frame-script>",
    )?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    let script = game_load_frame_script(runtime, &resolved)?;
    let encoded = game_encode_frame(session, &script)?;
    let (presented, completion_observed) = game_submit_frame(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.gfx.submit pid={} frame={} ops={} bytes={} submitted={} presented={} present-ok={} queue={} present-mode={} completion={} completion-observed={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.submitted_frames,
            session.presented_frames,
            presented,
            encoded.queue,
            encoded.present_mode,
            encoded.completion,
            completion_observed
        ),
    )
}
