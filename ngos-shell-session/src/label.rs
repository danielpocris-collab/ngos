//! Label/name helpers for ABI enum values.

use ngos_user_abi::{
    NativeContractKind, NativeContractState, NativeMountPropagationMode,
    NativeResourceArbitrationPolicy, NativeResourceGovernanceMode, NativeResourceKind,
    NativeResourceState,
};

pub(crate) fn contract_state_name(raw: u32) -> &'static str {
    match NativeContractState::from_raw(raw) {
        Some(NativeContractState::Active) => "active",
        Some(NativeContractState::Suspended) => "suspended",
        Some(NativeContractState::Revoked) => "revoked",
        None => "unknown",
    }
}

pub(crate) fn resource_state_name(raw: u32) -> &'static str {
    match NativeResourceState::from_raw(raw) {
        Some(NativeResourceState::Active) => "active",
        Some(NativeResourceState::Suspended) => "suspended",
        Some(NativeResourceState::Retired) => "retired",
        None => "unknown",
    }
}

pub(crate) fn resource_kind_name(raw: u32) -> &'static str {
    match NativeResourceKind::from_raw(raw) {
        Some(NativeResourceKind::Memory) => "memory",
        Some(NativeResourceKind::Storage) => "storage",
        Some(NativeResourceKind::Channel) => "channel",
        Some(NativeResourceKind::Device) => "device",
        Some(NativeResourceKind::Namespace) => "namespace",
        Some(NativeResourceKind::Surface) => "surface",
        None => "unknown",
    }
}

pub(crate) fn contract_kind_name(raw: u32) -> &'static str {
    match NativeContractKind::from_raw(raw) {
        Some(NativeContractKind::Execution) => "execution",
        Some(NativeContractKind::Memory) => "memory",
        Some(NativeContractKind::Io) => "io",
        Some(NativeContractKind::Device) => "device",
        Some(NativeContractKind::Display) => "display",
        Some(NativeContractKind::Observe) => "observe",
        None => "unknown",
    }
}

pub(crate) fn resource_arbitration_name(raw: u32) -> &'static str {
    match NativeResourceArbitrationPolicy::from_raw(raw) {
        Some(NativeResourceArbitrationPolicy::Fifo) => "fifo",
        Some(NativeResourceArbitrationPolicy::Lifo) => "lifo",
        None => "unknown",
    }
}

pub(crate) fn resource_governance_name(raw: u32) -> &'static str {
    match NativeResourceGovernanceMode::from_raw(raw) {
        Some(NativeResourceGovernanceMode::Queueing) => "queueing",
        Some(NativeResourceGovernanceMode::ExclusiveLease) => "exclusive-lease",
        None => "unknown",
    }
}

pub(crate) fn mount_propagation_name(mode: u32) -> &'static str {
    match NativeMountPropagationMode::from_raw(mode) {
        Some(NativeMountPropagationMode::Private) => "private",
        Some(NativeMountPropagationMode::Shared) => "shared",
        Some(NativeMountPropagationMode::Slave) => "slave",
        None => "unknown",
    }
}
