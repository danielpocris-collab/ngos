use alloc::string::String;

use ngos_game_compat_runtime::CompatLaneKind;

pub struct GameCompatLaneRuntime {
    pub kind: CompatLaneKind,
    pub resource_id: usize,
    pub resource_name: String,
    pub contract_id: usize,
    pub contract_label: String,
    pub claim_acquired: bool,
    pub invoke_token: Option<usize>,
    pub watch_queue_fd: Option<usize>,
    pub watch_token: Option<u64>,
}
