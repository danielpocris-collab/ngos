use alloc::boxed::Box;
use ngos_game_compat_runtime::{CompatLoaderArtifactMismatch, CompatLoaderSessionSnapshot};
use ngos_user_abi::ExitCode;

use crate::GameCompatSession;

pub struct GameCompatLoaderSessionObservation {
    pub snapshot: CompatLoaderSessionSnapshot,
}

pub struct GameCompatLaunchedLoaderObservation {
    pub session: Box<GameCompatSession>,
    pub observation: GameCompatLoaderSessionObservation,
}

pub enum GameCompatLoaderSessionObservationError {
    Render,
    ArtifactRead(ExitCode),
    ArtifactMismatch(CompatLoaderArtifactMismatch),
}

pub enum GameCompatLaunchLoaderObservationError {
    Launch(ExitCode),
    Observe(
        Box<GameCompatSession>,
        GameCompatLoaderSessionObservationError,
    ),
}

pub use crate::compat_loader_artifact_support::{
    game_compat_loader_artifact_snapshot, game_compat_loader_session_snapshot,
};
pub use crate::compat_loader_launch_support::game_launch_and_observe_loader_session;
pub use crate::compat_loader_observe_support::game_compat_observe_loader_session;
