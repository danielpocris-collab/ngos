use super::*;
use ngos_user_abi::{
    Errno, NativeContractKind, NativeContractRecord, NativeContractState, NativeDomainRecord,
    NativeResourceArbitrationPolicy, NativeResourceCancelRecord, NativeResourceClaimRecord,
    NativeResourceContractPolicy, NativeResourceGovernanceMode, NativeResourceIssuerPolicy,
    NativeResourceKind, NativeResourceRecord, NativeResourceReleaseRecord, NativeResourceState,
    SYS_ACQUIRE_RESOURCE, SYS_BIND_PROCESS_CONTRACT, SYS_CANCEL_RESOURCE_CLAIM, SYS_CLAIM_RESOURCE,
    SYS_CLOSE, SYS_CREATE_CONTRACT, SYS_CREATE_DOMAIN, SYS_CREATE_RESOURCE, SYS_DUP, SYS_EXIT,
    SYS_FCNTL, SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME, SYS_GET_RESOURCE_NAME,
    SYS_INSPECT_CONTRACT, SYS_INSPECT_DOMAIN, SYS_INSPECT_RESOURCE, SYS_INVOKE_CONTRACT,
    SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS, SYS_LIST_RESOURCE_WAITERS, SYS_LIST_RESOURCES, SYS_POLL,
    SYS_READ, SYS_RELEASE_CLAIMED_RESOURCE, SYS_RELEASE_RESOURCE, SYS_SET_CONTRACT_STATE,
    SYS_SET_RESOURCE_CONTRACT_POLICY, SYS_SET_RESOURCE_GOVERNANCE, SYS_SET_RESOURCE_ISSUER_POLICY,
    SYS_SET_RESOURCE_POLICY, SYS_SET_RESOURCE_STATE, SYS_TRANSFER_RESOURCE, SYS_WRITE,
    SyscallFrame, SyscallReturn,
};

const IOPOLL_READABLE: u32 = ngos_user_abi::POLLIN;
const IOPOLL_PRIORITY: u32 = ngos_user_abi::POLLPRI;
const IOPOLL_WRITABLE: u32 = ngos_user_abi::POLLOUT;
const IOPOLL_HANGUP: u32 = 1 << 3;

#[path = "user_syscall_runtime/dispatch_basic.rs"]
mod dispatch_basic;
#[path = "user_syscall_runtime/dispatch_native.rs"]
mod dispatch_native;
#[path = "user_syscall_runtime/helpers.rs"]
mod helpers;

use helpers::*;

#[cfg(test)]
mod tests;
