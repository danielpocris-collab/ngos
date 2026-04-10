use alloc::string::String;

use ngos_game_compat_runtime::{GameCompatManifest, GameSessionPlan};

use crate::{GameCompatSession, build_loader_dll_overrides, build_loader_env_overrides};

pub struct SessionRuntimePaths {
    pub runtime_env_path: String,
    pub runtime_argv_path: String,
    pub runtime_channel_path: String,
    pub runtime_loader_path: String,
    pub runtime_abi_path: String,
}

pub fn build_game_session_runtime(
    session: &mut GameCompatSession,
    manifest: &GameCompatManifest,
    _plan: &GameSessionPlan,
    paths: SessionRuntimePaths,
) {
    session.runtime_env_path = paths.runtime_env_path;
    session.runtime_argv_path = paths.runtime_argv_path;
    session.runtime_channel_path = paths.runtime_channel_path;
    session.runtime_loader_path = paths.runtime_loader_path;
    session.runtime_abi_path = paths.runtime_abi_path;
    session.loader_preloads = manifest.shim_preloads.clone();
    session.loader_dll_overrides = build_loader_dll_overrides(manifest);
    session.loader_env_overrides = build_loader_env_overrides(manifest);
    session.loader_routing = manifest.loader_routing_plan();
    session.abi_routing = manifest.abi_routing_plan();
}
