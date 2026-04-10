use alloc::format;

use crate::GameCompatSession;

pub fn render_session_abi_line(session: &GameCompatSession) -> alloc::string::String {
    format!(
        "game.session.abi pid={} route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={}",
        session.pid,
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
    )
}
