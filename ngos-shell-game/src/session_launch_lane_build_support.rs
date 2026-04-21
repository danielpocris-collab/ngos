use alloc::vec::Vec;

use ngos_game_compat_runtime::CompatLanePlan;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatLaneRuntime, activate_and_claim_game_lane, create_game_lane_contract,
    create_game_lane_resource,
};

pub fn launch_session_lane<B: SyscallBackend>(
    runtime: &Runtime<B>,
    domain_id: usize,
    spec: &CompatLanePlan,
    existing_lanes: Vec<GameCompatLaneRuntime>,
) -> Result<(Vec<GameCompatLaneRuntime>, GameCompatLaneRuntime), ExitCode> {
    let (existing_lanes, resource_id) =
        create_game_lane_resource(runtime, domain_id, spec, existing_lanes)?;
    let (existing_lanes, contract_id) =
        create_game_lane_contract(runtime, domain_id, spec, resource_id, existing_lanes)?;
    activate_and_claim_game_lane(runtime, spec, resource_id, contract_id, existing_lanes)
}
