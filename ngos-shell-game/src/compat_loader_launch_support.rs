use ngos_game_compat_runtime::GameCompatManifest;
use ngos_user_abi::SyscallBackend;
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatLaunchLoaderObservationError, GameCompatLaunchedLoaderObservation,
    game_compat_observe_loader_session,
};

pub fn game_launch_and_observe_loader_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut alloc::string::String,
    manifest: &GameCompatManifest,
) -> Result<GameCompatLaunchedLoaderObservation, GameCompatLaunchLoaderObservationError> {
    let session = crate::game_launch_session(runtime, current_cwd, manifest)
        .map_err(GameCompatLaunchLoaderObservationError::Launch)?;
    let observation = match game_compat_observe_loader_session(runtime, &session, manifest) {
        Ok(observation) => observation,
        Err(error) => {
            return Err(GameCompatLaunchLoaderObservationError::Observe(
                session, error,
            ));
        }
    };
    Ok(GameCompatLaunchedLoaderObservation {
        session,
        observation,
    })
}
