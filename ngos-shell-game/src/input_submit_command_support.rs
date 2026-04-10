use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session_mut, game_encode_input, game_load_input_script,
    game_submit_input, parse_game_pid_script_args, write_line,
};

pub fn handle_game_input_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-input-submit <pid> <input-script>",
    )?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    let script = game_load_input_script(runtime, &resolved)?;
    let encoded = game_encode_input(session, &script)?;
    let (token, delivery_observed) = game_submit_input(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.input.submit pid={} frame={} ops={} bytes={} batches={} token={} delivery={} delivery-observed={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.submitted_input_batches,
            token,
            encoded.delivery,
            delivery_observed
        ),
    )
}
