use alloc::format;

use ngos_game_compat_runtime::graphics_api_name;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn render_game_session_profile_runtime<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    let pid = session.pid;
    write_line(
        runtime,
        &format!(
            "game.session.profile.gfx pid={pid} api={} profile={} backend={} translation={}",
            graphics_api_name(session.graphics_source_api),
            session.graphics_profile,
            session.graphics_translation.backend_name,
            session.graphics_translation.translation,
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.profile.audio pid={pid} profile={}",
            session.audio_profile
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.profile.input pid={pid} profile={}",
            session.input_profile
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.profile.paths pid={pid} cwd={} exec={} prefix={} saves={} cache={}",
            session.working_dir,
            session.executable_path,
            session.prefix_path,
            session.saves_path,
            session.cache_path,
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.profile.abi pid={pid} route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={} file={}",
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
