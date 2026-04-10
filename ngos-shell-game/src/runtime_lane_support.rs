use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::ExitCode;

use crate::{GameCompatLaneRuntime, GameCompatSession};

pub fn game_session_lane(
    session: &GameCompatSession,
    kind: CompatLaneKind,
) -> Result<&GameCompatLaneRuntime, ExitCode> {
    session
        .lanes
        .iter()
        .find(|lane| lane.kind == kind)
        .ok_or(293)
}

pub fn game_session_lane_mut(
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<&mut GameCompatLaneRuntime, ExitCode> {
    session
        .lanes
        .iter_mut()
        .find(|lane| lane.kind == kind)
        .ok_or(293)
}
