use alloc::boxed::Box;
use alloc::vec::Vec;

use ngos_game_compat_runtime::{GameCompatManifest, GameSessionPlan};

use crate::{GameCompatLaneRuntime, GameCompatSession};

pub fn build_game_session(
    manifest: &GameCompatManifest,
    plan: GameSessionPlan,
    pid: u64,
    domain_id: usize,
    lanes: Vec<GameCompatLaneRuntime>,
    runtime_env_path: alloc::string::String,
    runtime_argv_path: alloc::string::String,
    runtime_channel_path: alloc::string::String,
    runtime_loader_path: alloc::string::String,
    runtime_abi_path: alloc::string::String,
) -> Box<GameCompatSession> {
    let mut session = crate::new_empty_game_session(manifest);
    crate::build_game_session_identity(&mut session, manifest, &plan, pid, domain_id);
    crate::build_game_session_runtime(
        &mut session,
        manifest,
        &plan,
        crate::SessionRuntimePaths {
            runtime_env_path,
            runtime_argv_path,
            runtime_channel_path,
            runtime_loader_path,
            runtime_abi_path,
        },
    );
    crate::build_game_session_media(&mut session, manifest, &plan, lanes);
    Box::new(session)
}
