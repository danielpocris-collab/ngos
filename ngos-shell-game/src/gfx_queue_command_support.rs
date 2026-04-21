use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session_mut, parse_game_pid_arg, summarize_graphics_deep_ops,
    write_line,
};

pub fn handle_game_gfx_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-gfx-next <pid>")?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    if session.pending_graphics_frames.is_empty() {
        write_line(
            runtime,
            &format!("game.gfx.queue pid={} depth=0", session.pid),
        )?;
        return Err(299);
    }
    let encoded = session.pending_graphics_frames.remove(0);
    let source_api = encoded.source_api.as_deref().unwrap_or("-");
    let translation = encoded.translation_label.as_deref().unwrap_or("-");
    let deep_ops = summarize_graphics_deep_ops(&encoded.payload);
    write_line(
        runtime,
        &format!(
            "game.gfx.next pid={} frame={} api={} translation={} queue={} present-mode={} completion={} remaining={} deep-ops={} payload={}",
            session.pid,
            encoded.frame_tag,
            source_api,
            translation,
            encoded.queue,
            encoded.present_mode,
            encoded.completion,
            session.pending_graphics_frames.len(),
            deep_ops,
            encoded.payload
        ),
    )
}
