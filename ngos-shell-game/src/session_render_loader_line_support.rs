use alloc::format;
use alloc::string::String;

use crate::GameCompatSession;

pub fn render_session_loader_line(session: &GameCompatSession) -> String {
    format!(
        "game.session.loader pid={} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={} preloads={} dll-overrides={} env-overrides={}",
        session.pid,
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
    )
}
