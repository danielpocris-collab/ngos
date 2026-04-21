use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ngos_shell_types::{ShellJob, resolve_shell_path};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_launch_session, game_manifest_load, write_line};

pub fn handle_game_launch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    path: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    jobs: &mut Vec<ShellJob>,
    last_spawned_pid: &mut Option<u64>,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(current_cwd, path);
    let manifest = game_manifest_load(runtime, &resolved)?;
    let session = game_launch_session(runtime, current_cwd, &manifest)?;
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
            "game.launched pid={} title={} slug={} cwd={} exec={}",
            session.pid, session.title, session.slug, session.working_dir, session.executable_path
        ),
    )?;
    game_sessions.push(*session);
    Ok(())
}
