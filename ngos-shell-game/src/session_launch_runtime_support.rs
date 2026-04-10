use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ngos_game_compat_runtime::GameCompatManifest;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::session_launch_builder_support::build_game_session;
use crate::session_launch_lane_support::{launch_session_lanes, rollback_partial_game_session};
use crate::{GameCompatSession, game_write_runtime_bootstrap};

pub fn game_launch_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    _current_cwd: &mut String,
    manifest: &GameCompatManifest,
) -> Result<Box<GameCompatSession>, ExitCode> {
    let plan = manifest.session_plan();
    let (
        runtime_env_path,
        runtime_argv_path,
        runtime_channel_path,
        runtime_loader_path,
        runtime_abi_path,
    ) = game_write_runtime_bootstrap(runtime, &plan, manifest)?;
    let domain_id = runtime
        .create_domain(None, &plan.domain_name)
        .map_err(|_| 284)?;
    let mut lanes = launch_session_lanes(runtime, domain_id, &plan)?;
    let process_argv = core::iter::once(plan.executable_path.as_str())
        .chain(plan.argv.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let process_env = plan
        .env_shims
        .iter()
        .map(|shim| format!("{}={}", shim.key, shim.value))
        .chain(core::iter::once(format!(
            "NGOS_GAME_CHANNEL={runtime_channel_path}"
        )))
        .collect::<Vec<_>>();
    let process_env_refs = process_env.iter().map(String::as_str).collect::<Vec<_>>();
    let pid = match runtime.spawn_configured_process(
        &plan.process_name,
        &plan.executable_path,
        &plan.working_dir,
        &process_argv,
        &process_env_refs,
    ) {
        Ok(pid) => pid,
        Err(_) => {
            rollback_partial_game_session(runtime, None, &mut lanes);
            return Err(288);
        }
    };

    Ok(build_game_session(
        manifest,
        plan,
        pid,
        domain_id,
        lanes,
        runtime_env_path,
        runtime_argv_path,
        runtime_channel_path,
        runtime_loader_path,
        runtime_abi_path,
    ))
}
