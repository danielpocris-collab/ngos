use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::gfx_request_common_support::{
    parse_driver_request_id, retained_gfx_request_id, write_retained_gfx_request,
};
use crate::{GameCompatSession, find_game_session, parse_game_pid_arg, write_line};

pub fn handle_game_gfx_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-gfx-request <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    if let Some((request_id, _)) = retained_gfx_request_id(runtime, session)? {
        return write_retained_gfx_request(runtime, pid, session, request_id);
    }
    let fd = runtime
        .open_path(&session.graphics_driver_path)
        .map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    if count == 0 {
        return write_line(
            runtime,
            &format!(
                "game.gfx.request pid={} driver={} api={} translation={} outcome=empty",
                pid,
                session.graphics_driver_path,
                session.graphics_translation.source_api_name,
                session.graphics_translation.translation
            ),
        );
    }
    let prefix_len = buffer[..count]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(count);
    let header = core::str::from_utf8(&buffer[..prefix_len]).map_err(|_| 239)?;
    let request_id = parse_driver_request_id(header.trim_end()).ok_or(239)?;
    write_retained_gfx_request(runtime, pid, session, request_id)
}
