use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session, game_encode_mix, game_load_mix_script,
    parse_game_pid_script_args, write_line,
};

pub fn handle_game_audio_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-audio-plan <pid> <mix-script>",
    )?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let script = game_load_mix_script(runtime, &resolved)?;
    let encoded = game_encode_mix(session, &script)?;
    write_line(
        runtime,
        &format!(
            "game.audio.plan pid={} stream={} ops={} bytes={} profile={} route={} latency-mode={} spatialization={}",
            pid,
            encoded.stream_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.audio_profile,
            encoded.route,
            encoded.latency_mode,
            encoded.spatialization
        ),
    )
}
