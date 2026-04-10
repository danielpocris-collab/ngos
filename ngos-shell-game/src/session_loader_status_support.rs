use alloc::format;
use alloc::string::String;

use ngos_game_compat_runtime::graphics_api_name;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn handle_game_loader_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let running = game_sessions.iter().filter(|s| !s.stopped).count();
    let stopped = game_sessions.iter().filter(|s| s.stopped).count();
    write_line(
        runtime,
        &format!(
            "game.loader.status sessions={} running={running} stopped={stopped}",
            game_sessions.len()
        ),
    )?;
    for session in game_sessions {
        let state = if session.stopped {
            "stopped"
        } else {
            "running"
        };
        let exit_str = session
            .exit_code
            .map(|c| format!("{c}"))
            .unwrap_or_else(|| String::from("-"));
        write_line(
            runtime,
            &format!(
                "game.loader.session pid={} slug={} state={state} exit-code={exit_str} route={} mode={} entry={} bootstrap={} gfx={} audio={} input={} preloads={} dll-overrides={} env-overrides={}",
                session.pid,
                session.slug,
                session.loader_routing.route_class,
                session.loader_routing.launch_mode,
                session.loader_routing.entry_profile,
                session.loader_routing.bootstrap_profile,
                graphics_api_name(session.graphics_source_api),
                session.audio_profile,
                session.input_profile,
                session.loader_preloads.len(),
                session.loader_dll_overrides.len(),
                session.loader_env_overrides.len(),
            ),
        )?;
    }
    Ok(())
}
