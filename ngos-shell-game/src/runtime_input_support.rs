use ngos_input_translate::EncodedInput;
use ngos_shell_vfs::shell_write_all;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_publish_runtime_payload};

pub fn game_submit_input<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    encoded: &EncodedInput,
) -> Result<(usize, &'static str), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    let (token, delivery_observed) =
        crate::game_submit_input_completion(runtime, session, encoded)?;
    crate::game_record_submitted_input(session, encoded, token, delivery_observed)?;
    let input_fd = runtime
        .open_path(&session.input_device_path)
        .map_err(|_| 234)?;
    shell_write_all(runtime, input_fd, encoded.payload.as_bytes())?;
    runtime.close(input_fd).map_err(|_| 240)?;
    game_publish_runtime_payload(
        runtime,
        &session.runtime_channel_path,
        "input",
        &encoded.frame_tag,
        encoded.payload.as_bytes(),
    )?;
    Ok((token, delivery_observed))
}
