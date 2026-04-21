use alloc::format;
use alloc::string::String;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn render_game_session_profile_loader<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    let pid = session.pid;
    write_line(
        runtime,
        &format!(
            "game.session.profile.loader pid={pid} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={} file={} preloads={} dll-overrides={} env-overrides={}",
            session.loader_routing.route_class,
            session.loader_routing.launch_mode,
            session.loader_routing.entry_profile,
            session.loader_routing.bootstrap_profile,
            session.loader_routing.entrypoint,
            if session.loader_routing.requires_compat_shims {
                1
            } else {
                0
            },
            session.runtime_loader_path,
            if session.loader_preloads.is_empty() {
                String::from("-")
            } else {
                session.loader_preloads.join(";")
            },
            if session.loader_dll_overrides.is_empty() {
                String::from("-")
            } else {
                session.loader_dll_overrides.join(";")
            },
            if session.loader_env_overrides.is_empty() {
                String::from("-")
            } else {
                session.loader_env_overrides.join(";")
            },
        ),
    )
}
