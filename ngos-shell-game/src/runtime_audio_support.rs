use ngos_audio_translate::EncodedMix;
use ngos_shell_vfs::shell_write_all;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_publish_runtime_payload};

pub fn game_submit_mix<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    encoded: &EncodedMix,
) -> Result<(usize, &'static str), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    let (token, completion_observed) =
        crate::game_submit_mix_completion(runtime, session, encoded)?;
    crate::game_record_submitted_mix(session, encoded, token, completion_observed)?;
    let audio_fd = runtime
        .open_path(&session.audio_device_path)
        .map_err(|_| 234)?;
    shell_write_all(runtime, audio_fd, encoded.payload.as_bytes())?;
    runtime.close(audio_fd).map_err(|_| 240)?;
    game_publish_runtime_payload(
        runtime,
        &session.runtime_channel_path,
        "audio",
        &encoded.stream_tag,
        encoded.payload.as_bytes(),
    )?;
    Ok((token, completion_observed))
}
