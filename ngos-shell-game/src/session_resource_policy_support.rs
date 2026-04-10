use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::{
    ExitCode, NativeContractKind, NativeResourceArbitrationPolicy, NativeResourceContractPolicy,
    NativeResourceGovernanceMode, NativeResourceIssuerPolicy, NativeResourceKind, SyscallBackend,
};
use ngos_user_runtime::Runtime;

pub fn game_plan_resource_kind(kind: CompatLaneKind) -> NativeResourceKind {
    match kind {
        CompatLaneKind::Graphics => NativeResourceKind::Surface,
        CompatLaneKind::Audio => NativeResourceKind::Channel,
        CompatLaneKind::Input => NativeResourceKind::Device,
    }
}

pub fn game_plan_contract_kind(kind: CompatLaneKind) -> NativeContractKind {
    match kind {
        CompatLaneKind::Graphics => NativeContractKind::Display,
        CompatLaneKind::Audio => NativeContractKind::Io,
        CompatLaneKind::Input => NativeContractKind::Observe,
    }
}

pub fn game_apply_resource_policy<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource_id: usize,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    runtime
        .set_resource_arbitration_policy(resource_id, NativeResourceArbitrationPolicy::Fifo)
        .map_err(|_| 284)?;
    let governance = match kind {
        CompatLaneKind::Audio => NativeResourceGovernanceMode::Queueing,
        CompatLaneKind::Graphics | CompatLaneKind::Input => {
            NativeResourceGovernanceMode::ExclusiveLease
        }
    };
    runtime
        .set_resource_governance_mode(resource_id, governance)
        .map_err(|_| 284)?;
    let contract_policy = match kind {
        CompatLaneKind::Graphics => NativeResourceContractPolicy::Display,
        CompatLaneKind::Audio => NativeResourceContractPolicy::Io,
        CompatLaneKind::Input => NativeResourceContractPolicy::Observe,
    };
    runtime
        .set_resource_contract_policy(resource_id, contract_policy)
        .map_err(|_| 284)?;
    runtime
        .set_resource_issuer_policy(resource_id, NativeResourceIssuerPolicy::CreatorOnly)
        .map_err(|_| 284)?;
    Ok(())
}
