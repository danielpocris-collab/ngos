use ngos_game_compat_runtime::{
    CompatLoaderArtifactSnapshot, CompatLoaderSessionSnapshot, GameCompatManifest,
    compat_loader_session_snapshot,
};
use ngos_shell_vfs::shell_read_file_bytes;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;

pub fn game_compat_loader_session_snapshot(
    session: &GameCompatSession,
    manifest: &GameCompatManifest,
) -> CompatLoaderSessionSnapshot {
    compat_loader_session_snapshot(
        session.pid,
        manifest,
        &session.loader_preloads,
        &session.loader_dll_overrides,
        &session.loader_env_overrides,
    )
}

pub fn game_compat_loader_artifact_snapshot<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<CompatLoaderArtifactSnapshot, ExitCode> {
    Ok(CompatLoaderArtifactSnapshot {
        env_payload: shell_read_file_bytes(runtime, &session.runtime_env_path)?,
        loader_payload: shell_read_file_bytes(runtime, &session.runtime_loader_path)?,
    })
}
