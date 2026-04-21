use alloc::format;
use alloc::string::{String, ToString};

use ngos_game_compat_runtime::lane_name;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn render_session_lanes<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.session.gfx-queue pid={} depth={}",
            session.pid,
            session.pending_graphics_frames.len()
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.audio-queue pid={} depth={}",
            session.pid,
            session.pending_audio_batches.len()
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.input-queue pid={} depth={}",
            session.pid,
            session.pending_input_batches.len()
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.runtime-channel pid={} path={}",
            session.pid, session.runtime_channel_path
        ),
    )?;
    for lane in &session.lanes {
        write_line(
            runtime,
            &format!(
                "game.session.lane kind={} resource-id={} resource={} contract-id={} contract={} claimed={} token={}",
                lane_name(lane.kind),
                lane.resource_id,
                lane.resource_name,
                lane.contract_id,
                lane.contract_label,
                lane.claim_acquired,
                lane.invoke_token
                    .map(|token| token.to_string())
                    .unwrap_or_else(|| String::from("pending"))
            ),
        )?;
        write_line(
            runtime,
            &format!(
                "game.session.watch kind={} queue={} token={}",
                lane_name(lane.kind),
                lane.watch_queue_fd
                    .map(|fd| fd.to_string())
                    .unwrap_or_else(|| String::from("inactive")),
                lane.watch_token
                    .map(|token| token.to_string())
                    .unwrap_or_else(|| String::from("inactive"))
            ),
        )?;
    }
    Ok(())
}
