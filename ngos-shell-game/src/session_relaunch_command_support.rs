use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ngos_game_compat_runtime::CompatLaunchProfile;
use ngos_shell_types::{ShellJob, resolve_shell_path};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, game_launch_session, game_manifest_load, game_stop_session, write_line,
};

pub fn handle_game_relaunch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    path: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    jobs: &mut Vec<ShellJob>,
    last_spawned_pid: &mut Option<u64>,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(current_cwd, path);
    let manifest = game_manifest_load(runtime, &resolved).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!("game.relaunch.refused path={resolved} reason=manifest-load-failed"),
        );
        e
    })?;
    for session in game_sessions.iter_mut() {
        if session.slug == manifest.slug && !session.stopped {
            let _ = game_stop_session(runtime, session);
            if let Some(job) = jobs.iter_mut().find(|job| job.pid == session.pid) {
                job.reaped_exit = session.exit_code;
            }
            write_line(
                runtime,
                &format!(
                    "game.relaunch.stopped pid={} slug={} exit-code={}",
                    session.pid,
                    session.slug,
                    session.exit_code.unwrap_or(-1),
                ),
            )?;
        }
    }
    let session = game_launch_session(runtime, current_cwd, &manifest).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "game.relaunch.refused slug={} reason=launch-failed",
                manifest.slug
            ),
        );
        e
    })?;
    let profile = CompatLaunchProfile::from_manifest(&manifest);
    *last_spawned_pid = Some(session.pid);
    jobs.push(ShellJob {
        pid: session.pid,
        name: session.process_name.clone(),
        path: session.executable_path.clone(),
        reaped_exit: None,
        signal_count: 0,
    });
    write_line(
        runtime,
        &format!(
            "game.relaunched pid={} slug={} {}",
            session.pid,
            session.slug,
            profile.describe(),
        ),
    )?;
    game_sessions.push(*session);
    Ok(())
}
