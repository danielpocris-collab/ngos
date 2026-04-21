use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, find_game_session_mut, parse_game_pid_arg, write_line};

pub fn handle_game_audio_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-audio-next <pid>")?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    if session.pending_audio_batches.is_empty() {
        write_line(
            runtime,
            &format!("game.audio.queue pid={} depth=0", session.pid),
        )?;
        return Err(299);
    }
    let encoded = session.pending_audio_batches.remove(0);
    write_line(
        runtime,
        &format!(
            "game.audio.next pid={} stream={} route={} latency-mode={} spatialization={} completion={} remaining={} payload={}",
            session.pid,
            encoded.stream_tag,
            encoded.route,
            encoded.latency_mode,
            encoded.spatialization,
            encoded.completion,
            session.pending_audio_batches.len(),
            encoded.payload
        ),
    )
}
