use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session_mut, game_encode_mix, game_load_mix_script,
    game_submit_mix, parse_game_pid_script_args, write_line,
};

pub fn handle_game_audio_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-audio-submit <pid> <mix-script>",
    )?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    let script = game_load_mix_script(runtime, &resolved)?;
    let encoded = game_encode_mix(session, &script)?;
    let (token, completion_observed) = game_submit_mix(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.audio.submit pid={} stream={} ops={} bytes={} batches={} token={} route={} latency-mode={} spatialization={} completion={} completion-observed={}",
            pid,
            encoded.stream_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.submitted_audio_batches,
            token,
            encoded.route,
            encoded.latency_mode,
            encoded.spatialization,
            encoded.completion,
            completion_observed
        ),
    )
}
