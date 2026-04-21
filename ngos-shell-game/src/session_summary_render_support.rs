use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, StackLineBuffer, write_line};
use alloc::format;
use alloc::string::{String, ToString};
use ngos_game_compat_runtime::lane_name;

pub fn game_render_session_summary<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    let active_watches = session
        .lanes
        .iter()
        .filter(|lane| lane.watch_queue_fd.is_some())
        .count();
    let mut line = StackLineBuffer::<384>::new();
    line.push_str("game.session.summary pid=")?;
    line.push_u64(session.pid)?;
    line.push_str(" slug=")?;
    line.push_str(&session.slug)?;
    line.push_str(" title=")?;
    line.push_str(&session.title)?;
    line.push_str(" stopped=")?;
    line.push_bool(session.stopped)?;
    line.push_str(" exit=")?;
    if let Some(code) = session.exit_code {
        line.push_i32(code)?;
    } else {
        line.push_byte(b'-')?;
    }
    line.push_str(" lanes=")?;
    line.push_usize(session.lanes.len())?;
    line.push_str(" watches=")?;
    line.push_usize(active_watches)?;
    line.push_str(" pending[gfx=")?;
    line.push_usize(session.pending_graphics_frames.len())?;
    line.push_str(";audio=")?;
    line.push_usize(session.pending_audio_batches.len())?;
    line.push_str(";input=")?;
    line.push_usize(session.pending_input_batches.len())?;
    line.push_str("] submitted[gfx=")?;
    line.push_u64(session.submitted_frames)?;
    line.push_str(";audio=")?;
    line.push_u64(session.submitted_audio_batches)?;
    line.push_str(";input=")?;
    line.push_u64(session.submitted_input_batches)?;
    line.push_byte(b']')?;
    runtime
        .writev(1, &[line.as_bytes(), b"\n"])
        .map_err(|_| 190)?;
    Ok(())
}

pub fn game_render_watch_summary<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    for lane in &session.lanes {
        write_line(
            runtime,
            &format!(
                "game.watch.summary pid={} slug={} kind={} queue={} token={} claimed={}",
                session.pid,
                session.slug,
                lane_name(lane.kind),
                lane.watch_queue_fd
                    .map(|fd| fd.to_string())
                    .unwrap_or_else(|| String::from("inactive")),
                lane.watch_token
                    .map(|token| token.to_string())
                    .unwrap_or_else(|| String::from("inactive")),
                lane.claim_acquired
            ),
        )?;
    }
    Ok(())
}
