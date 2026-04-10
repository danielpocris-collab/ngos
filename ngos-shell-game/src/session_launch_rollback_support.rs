use ngos_user_abi::{NativeContractState, NativeResourceState, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatLaneRuntime;

pub fn rollback_partial_game_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: Option<u64>,
    lanes: &mut [GameCompatLaneRuntime],
) {
    if let Some(pid) = pid {
        let _ = runtime.send_signal(pid, 15);
        let _ = runtime.reap_process(pid);
    }
    for lane in lanes.iter().rev() {
        if lane.claim_acquired {
            let _ = runtime.release_resource(lane.contract_id);
        }
        let _ = runtime.set_contract_state(lane.contract_id, NativeContractState::Suspended);
        let _ = runtime.set_resource_state(lane.resource_id, NativeResourceState::Suspended);
    }
}
