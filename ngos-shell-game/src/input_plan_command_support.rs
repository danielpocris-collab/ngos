use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session, game_encode_input, game_load_input_script,
    parse_game_pid_script_args, write_line,
};

pub fn handle_game_input_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-input-plan <pid> <input-script>",
    )?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let script = game_load_input_script(runtime, &resolved)?;
    let encoded = game_encode_input(session, &script)?;
    write_line(
        runtime,
        &format!(
            "game.input.plan pid={} frame={} ops={} bytes={} profile={} family={} layout={} key-table={} pointer-capture={} delivery={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.input_profile,
            encoded.device_family,
            encoded.layout,
            encoded.key_table,
            encoded.pointer_capture,
            encoded.delivery
        ),
    )
}
