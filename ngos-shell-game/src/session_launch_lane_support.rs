use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatLaneRuntime;
use crate::session_launch_lane_build_support::launch_session_lane;

pub fn launch_session_lanes<B: SyscallBackend>(
    runtime: &Runtime<B>,
    domain_id: usize,
    plan: &ngos_game_compat_runtime::GameSessionPlan,
) -> Result<Vec<GameCompatLaneRuntime>, ExitCode> {
    let mut lanes = Vec::new();
    for lane in &plan.lanes {
        let existing = core::mem::take(&mut lanes);
        match launch_session_lane(runtime, domain_id, lane, existing) {
            Ok((mut existing, built_lane)) => {
                existing.push(built_lane);
                lanes = existing;
            }
            Err(code) => return Err(code),
        }
    }
    Ok(lanes)
}

pub use crate::session_launch_rollback_support::rollback_partial_game_session;
