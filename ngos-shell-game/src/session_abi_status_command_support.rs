use alloc::format;

use ngos_game_compat_runtime::compat_target_name;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, find_game_session, parse_game_pid_arg, write_line};

pub fn handle_game_abi_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-abi-status <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    write_line(
        runtime,
        &format!(
            "game.abi.status pid={pid} target={} route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={} abi-file={}",
            compat_target_name(session.target),
            session.abi_routing.route_class,
            session.abi_routing.handle_profile,
            session.abi_routing.path_profile,
            session.abi_routing.scheduler_profile,
            session.abi_routing.sync_profile,
            session.abi_routing.timer_profile,
            session.abi_routing.module_profile,
            session.abi_routing.event_profile,
            if session.abi_routing.requires_kernel_abi_shims {
                1
            } else {
                0
            },
            session.runtime_abi_path,
        ),
    )
}
