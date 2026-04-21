use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, summarize_graphics_deep_ops, write_line};

fn write_next_graphics<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
) -> Result<bool, ExitCode> {
    if session.pending_graphics_frames.is_empty() {
        return Ok(false);
    }
    let encoded = session.pending_graphics_frames.remove(0);
    write_line(
        runtime,
        &format!(
            "game.next pid={} kind=graphics tag={} remaining[gfx={};audio={};input={}] deep-ops={} payload={}",
            session.pid,
            encoded.frame_tag,
            session.pending_graphics_frames.len(),
            session.pending_audio_batches.len(),
            session.pending_input_batches.len(),
            summarize_graphics_deep_ops(&encoded.payload),
            encoded.payload
        ),
    )?;
    Ok(true)
}

fn write_next_audio<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
) -> Result<bool, ExitCode> {
    if session.pending_audio_batches.is_empty() {
        return Ok(false);
    }
    let encoded = session.pending_audio_batches.remove(0);
    write_line(
        runtime,
        &format!(
            "game.next pid={} kind=audio tag={} remaining[gfx={};audio={};input={}] payload={}",
            session.pid,
            encoded.stream_tag,
            session.pending_graphics_frames.len(),
            session.pending_audio_batches.len(),
            session.pending_input_batches.len(),
            encoded.payload
        ),
    )?;
    Ok(true)
}

fn write_next_input<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
) -> Result<bool, ExitCode> {
    if session.pending_input_batches.is_empty() {
        return Ok(false);
    }
    let encoded = session.pending_input_batches.remove(0);
    write_line(
        runtime,
        &format!(
            "game.next pid={} kind=input tag={} remaining[gfx={};audio={};input={}] payload={}",
            session.pid,
            encoded.frame_tag,
            session.pending_graphics_frames.len(),
            session.pending_audio_batches.len(),
            session.pending_input_batches.len(),
            encoded.payload
        ),
    )?;
    Ok(true)
}

pub fn game_next_payload<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
) -> Result<(), ExitCode> {
    if write_next_graphics(runtime, session)? {
        return Ok(());
    }
    if write_next_audio(runtime, session)? {
        return Ok(());
    }
    if write_next_input(runtime, session)? {
        return Ok(());
    }
    Err(299)
}
