use alloc::vec::Vec;

use ngos_game_compat_runtime::CompatLanePlan;
use ngos_user_abi::{ExitCode, NativeContractState, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatLaneRuntime, game_apply_resource_policy, game_plan_contract_kind,
    game_plan_resource_kind, pending_game_lane_record,
    session_launch_rollback_support::rollback_partial_game_session,
};

pub fn create_game_lane_resource<B: SyscallBackend>(
    runtime: &Runtime<B>,
    domain_id: usize,
    spec: &CompatLanePlan,
    existing_lanes: Vec<GameCompatLaneRuntime>,
) -> Result<(Vec<GameCompatLaneRuntime>, usize), ExitCode> {
    let resource_id = match runtime.create_resource(
        domain_id,
        game_plan_resource_kind(spec.kind),
        &spec.resource_name,
    ) {
        Ok(resource_id) => resource_id,
        Err(_) => {
            let mut pending = existing_lanes;
            rollback_partial_game_session(runtime, None, &mut pending);
            return Err(284);
        }
    };
    if game_apply_resource_policy(runtime, resource_id, spec.kind).is_err() {
        let mut pending = existing_lanes;
        rollback_partial_game_session(runtime, None, &mut pending);
        return Err(284);
    }
    Ok((existing_lanes, resource_id))
}

pub fn create_game_lane_contract<B: SyscallBackend>(
    runtime: &Runtime<B>,
    domain_id: usize,
    spec: &CompatLanePlan,
    resource_id: usize,
    existing_lanes: Vec<GameCompatLaneRuntime>,
) -> Result<(Vec<GameCompatLaneRuntime>, usize), ExitCode> {
    match runtime.create_contract(
        domain_id,
        resource_id,
        game_plan_contract_kind(spec.kind),
        &spec.contract_label,
    ) {
        Ok(contract_id) => Ok((existing_lanes, contract_id)),
        Err(_) => {
            let mut pending = existing_lanes;
            pending.push(pending_game_lane_record(spec, resource_id, 0, false));
            rollback_partial_game_session(runtime, None, &mut pending);
            Err(284)
        }
    }
}

pub fn activate_and_claim_game_lane<B: SyscallBackend>(
    runtime: &Runtime<B>,
    spec: &CompatLanePlan,
    resource_id: usize,
    contract_id: usize,
    existing_lanes: Vec<GameCompatLaneRuntime>,
) -> Result<(Vec<GameCompatLaneRuntime>, GameCompatLaneRuntime), ExitCode> {
    if runtime
        .set_contract_state(contract_id, NativeContractState::Active)
        .is_err()
    {
        let mut pending = existing_lanes;
        pending.push(pending_game_lane_record(
            spec,
            resource_id,
            contract_id,
            false,
        ));
        rollback_partial_game_session(runtime, None, &mut pending);
        return Err(284);
    }
    if runtime.acquire_resource(contract_id).is_err() {
        let mut pending = existing_lanes;
        pending.push(pending_game_lane_record(
            spec,
            resource_id,
            contract_id,
            false,
        ));
        rollback_partial_game_session(runtime, None, &mut pending);
        return Err(284);
    }
    Ok((
        existing_lanes,
        pending_game_lane_record(spec, resource_id, contract_id, true),
    ))
}
