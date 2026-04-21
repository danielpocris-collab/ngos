use alloc::boxed::Box;
use alloc::string::String;

use ngos_game_compat_runtime::GameCompatManifest;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatAbiSessionObservation, GameCompatAbiSessionObservationError, GameCompatSession,
    game_compat_observe_abi_session,
};

pub struct GameCompatLaunchedAbiObservation {
    pub session: Box<GameCompatSession>,
    pub observation: GameCompatAbiSessionObservation,
}

pub enum GameCompatLaunchAbiObservationError {
    Launch(ExitCode),
    Observe(Box<GameCompatSession>, GameCompatAbiSessionObservationError),
}

pub fn game_launch_and_observe_abi_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    manifest: &GameCompatManifest,
) -> Result<GameCompatLaunchedAbiObservation, GameCompatLaunchAbiObservationError> {
    let session = crate::game_launch_session(runtime, current_cwd, manifest)
        .map_err(GameCompatLaunchAbiObservationError::Launch)?;
    let observation = match game_compat_observe_abi_session(runtime, &session, manifest) {
        Ok(observation) => observation,
        Err(error) => return Err(GameCompatLaunchAbiObservationError::Observe(session, error)),
    };
    Ok(GameCompatLaunchedAbiObservation {
        session,
        observation,
    })
}
