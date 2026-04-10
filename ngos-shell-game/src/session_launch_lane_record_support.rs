use ngos_game_compat_runtime::CompatLanePlan;

use crate::GameCompatLaneRuntime;

pub fn pending_game_lane_record(
    spec: &CompatLanePlan,
    resource_id: usize,
    contract_id: usize,
    claim_acquired: bool,
) -> GameCompatLaneRuntime {
    GameCompatLaneRuntime {
        kind: spec.kind,
        resource_id,
        resource_name: spec.resource_name.clone(),
        contract_id,
        contract_label: spec.contract_label.clone(),
        claim_acquired,
        invoke_token: None,
        watch_queue_fd: None,
        watch_token: None,
    }
}
