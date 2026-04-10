use ngos_game_compat_runtime::{GameCompatManifest, compat_loader_verify_artifacts};
use ngos_user_abi::SyscallBackend;
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatLoaderSessionObservation, GameCompatLoaderSessionObservationError, GameCompatSession,
    game_compat_loader_artifact_snapshot, game_compat_loader_session_snapshot, game_render_session,
};

pub fn game_compat_observe_loader_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    manifest: &GameCompatManifest,
) -> Result<GameCompatLoaderSessionObservation, GameCompatLoaderSessionObservationError> {
    game_render_session(runtime, session)
        .map_err(|_| GameCompatLoaderSessionObservationError::Render)?;
    let artifacts = game_compat_loader_artifact_snapshot(runtime, session)
        .map_err(GameCompatLoaderSessionObservationError::ArtifactRead)?;
    if let Err(mismatch) = compat_loader_verify_artifacts(manifest, &artifacts) {
        return Err(GameCompatLoaderSessionObservationError::ArtifactMismatch(
            mismatch,
        ));
    }
    Ok(GameCompatLoaderSessionObservation {
        snapshot: game_compat_loader_session_snapshot(session, manifest),
    })
}
