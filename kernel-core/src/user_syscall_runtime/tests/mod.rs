use super::*;
use ngos_user_abi::{
    BootSessionStage, BootSessionStatus, NativeBusEndpointRecord, NativeBusPeerRecord,
    NativeContractKind, NativeContractRecord, NativeContractState, NativeDomainRecord,
    NativeFileStatusRecord, NativeFileSystemStatusRecord, NativeNetworkInterfaceConfig,
    NativeNetworkInterfaceRecord, NativeNetworkSocketRecord, NativeObjectKind, NativeProcessRecord,
    NativeResourceCancelRecord, NativeResourceContractPolicy, NativeResourceGovernanceMode,
    NativeResourceIssuerPolicy, NativeResourceKind, NativeResourceRecord, NativeResourceState,
    NativeUdpBindConfig, SYS_ACQUIRE_RESOURCE, SYS_ATTACH_BUS_PEER, SYS_BIND_PROCESS_CONTRACT,
    SYS_BIND_UDP_SOCKET, SYS_BOOT_REPORT, SYS_CANCEL_RESOURCE_CLAIM, SYS_CHDIR_PATH,
    SYS_CLAIM_RESOURCE, SYS_CLOSE, SYS_CONFIGURE_NETIF_IPV4, SYS_CREATE_BUS_ENDPOINT,
    SYS_CREATE_BUS_PEER, SYS_CREATE_CONTRACT, SYS_CREATE_DOMAIN, SYS_CREATE_RESOURCE,
    SYS_DETACH_BUS_PEER, SYS_DUP, SYS_EXIT, SYS_FCNTL, SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME,
    SYS_GET_PROCESS_CWD, SYS_GET_PROCESS_IMAGE_PATH, SYS_GET_PROCESS_NAME, SYS_GET_RESOURCE_NAME,
    SYS_INSPECT_BUS_ENDPOINT, SYS_INSPECT_BUS_PEER, SYS_INSPECT_CONTRACT, SYS_INSPECT_DOMAIN,
    SYS_INSPECT_NETIF, SYS_INSPECT_NETSOCK, SYS_INSPECT_PROCESS, SYS_INSPECT_RESOURCE,
    SYS_INVOKE_CONTRACT, SYS_LIST_BUS_ENDPOINTS, SYS_LIST_BUS_PEERS, SYS_LIST_CONTRACTS,
    SYS_LIST_DOMAINS, SYS_LIST_PATH, SYS_LIST_PROCESSES, SYS_LIST_RESOURCE_WAITERS,
    SYS_LIST_RESOURCES, SYS_LSTAT_PATH, SYS_MKCHAN_PATH, SYS_MKDIR_PATH, SYS_MKFILE_PATH,
    SYS_MKSOCK_PATH, SYS_OPEN_PATH, SYS_PENDING_SIGNALS, SYS_POLL, SYS_PUBLISH_BUS_MESSAGE,
    SYS_READ, SYS_READLINK_PATH, SYS_READV, SYS_REAP_PROCESS, SYS_RECEIVE_BUS_MESSAGE,
    SYS_RELEASE_CLAIMED_RESOURCE, SYS_RELEASE_RESOURCE, SYS_RENAME_PATH, SYS_SEND_SIGNAL,
    SYS_SET_CONTRACT_STATE, SYS_SET_RESOURCE_CONTRACT_POLICY, SYS_SET_RESOURCE_GOVERNANCE,
    SYS_SET_RESOURCE_ISSUER_POLICY, SYS_SET_RESOURCE_STATE, SYS_SPAWN_PATH_PROCESS, SYS_STAT_PATH,
    SYS_STATFS_PATH, SYS_SYMLINK_PATH, SYS_TRANSFER_RESOURCE, SYS_UNLINK_PATH, SYS_WRITE,
    SYS_WRITEV, SyscallFrame, UserIoVec,
};

fn setup_runtime_with_user_process() -> (KernelRuntime, ProcessId, u64) {
    let mut runtime = KernelRuntime::host_runtime_default();
    let pid = runtime
        .spawn_process("user", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "user-syscall")
        .unwrap();
    (runtime, pid, mapped)
}

fn open_file_descriptor(runtime: &mut KernelRuntime, pid: ProcessId, name: &str) -> Descriptor {
    let cap = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "rw-cap",
        )
        .unwrap();
    runtime
        .open_descriptor(pid, cap, ObjectKind::File, name)
        .unwrap()
}

mod basic;
mod native_model;
