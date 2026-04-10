#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: native user runtime
//! - owner layer: Layer 2
//! - semantic owner: `user-runtime`
//! - truth path role: syscall-facing runtime and canonical user-mode execution
//!   support on top of `user-abi`
//!
//! Canonical contract families implemented here:
//! - bootstrap execution contracts
//! - syscall invocation contracts
//! - system control and observation contracts
//! - user-mode runtime support contracts
//!
//! This crate may execute and expose canonical user-mode runtime behavior, but
//! it must not redefine kernel truth or replace the ABI contracts it consumes.

extern crate alloc;

pub mod bootstrap;
pub mod compat_abi;
pub mod system_control;
pub mod wasm;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::arch::asm;

use ngos_user_abi::{
    BlockRightsMask, BootSessionStage, BootSessionStatus, BootstrapArgs, CapabilityToken, ExitCode,
    FcntlCmd, IntegrityTag, NATIVE_STORAGE_LINEAGE_DEPTH, NativeBlockIoCompletion,
    NativeBlockIoRequest, NativeBusEndpointRecord, NativeBusEventWatchConfig, NativeBusPeerRecord,
    NativeContractKind, NativeContractRecord, NativeContractState, NativeDeviceRecord,
    NativeDeviceRequestRecord, NativeDomainRecord, NativeDriverRecord, NativeEventQueueMode,
    NativeEventRecord, NativeFileStatusRecord, NativeFileSystemStatusRecord,
    NativeGpuBindingRecord, NativeGpuBufferRecord, NativeGpuDisplayRecord, NativeGpuGspRecord,
    NativeGpuInterruptRecord, NativeGpuMediaRecord, NativeGpuNeuralRecord, NativeGpuPowerRecord,
    NativeGpuScanoutRecord, NativeGpuTensorRecord, NativeGpuVbiosRecord,
    NativeGraphicsEventWatchConfig, NativeMountPropagationMode, NativeMountRecord,
    NativeNetworkAdminConfig, NativeNetworkEventWatchConfig, NativeNetworkInterfaceConfig,
    NativeNetworkInterfaceRecord, NativeNetworkLinkStateConfig, NativeNetworkSocketRecord,
    NativeProcessCompatRecord, NativeProcessEventWatchConfig, NativeProcessIdentityRecord,
    NativeProcessRecord, NativeReadinessRecord, NativeResourceArbitrationPolicy,
    NativeResourceCancelRecord, NativeResourceClaimRecord, NativeResourceContractPolicy,
    NativeResourceEventWatchConfig, NativeResourceGovernanceMode, NativeResourceIssuerPolicy,
    NativeResourceKind, NativeResourceRecord, NativeResourceReleaseRecord, NativeResourceState,
    NativeSchedulerClass, NativeSpawnProcessConfig, NativeStorageLineageRecord,
    NativeStorageVolumeRecord, NativeSystemSnapshotRecord, NativeUdpBindConfig,
    NativeUdpConnectConfig, NativeUdpRecvMeta, NativeUdpSendToConfig, NativeVfsEventWatchConfig,
    ObjectSecurityContext, PollEvents, ProvenanceTag, SYS_ACQUIRE_RESOURCE,
    SYS_ADVISE_MEMORY_RANGE, SYS_ATTACH_BUS_PEER, SYS_BIND_DEVICE_DRIVER,
    SYS_BIND_PROCESS_CONTRACT, SYS_BIND_UDP_SOCKET, SYS_BLOCKED_PENDING_SIGNALS, SYS_BOOT_REPORT,
    SYS_CANCEL_RESOURCE_CLAIM, SYS_CHDIR_PATH, SYS_CHMOD_PATH, SYS_CHMOD_PATH_AT, SYS_CHOWN_PATH,
    SYS_CHOWN_PATH_AT, SYS_CLAIM_RESOURCE, SYS_CLOSE, SYS_COLLECT_READINESS,
    SYS_COMMIT_GPU_NEURAL_FRAME, SYS_COMPLETE_NET_TX, SYS_CONFIGURE_DEVICE_QUEUE,
    SYS_CONFIGURE_NETIF_ADMIN, SYS_CONFIGURE_NETIF_IPV4, SYS_CONNECT_UDP_SOCKET,
    SYS_CONTROL_DESCRIPTOR, SYS_CREATE_BUS_ENDPOINT, SYS_CREATE_BUS_PEER, SYS_CREATE_CONTRACT,
    SYS_CREATE_DOMAIN, SYS_CREATE_EVENT_QUEUE, SYS_CREATE_GPU_BUFFER, SYS_CREATE_RESOURCE,
    SYS_DETACH_BUS_PEER, SYS_DISPATCH_GPU_TENSOR_KERNEL, SYS_DUP, SYS_EXIT, SYS_FCNTL,
    SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME, SYS_GET_PROCESS_CWD, SYS_GET_PROCESS_IDENTITY,
    SYS_GET_PROCESS_IMAGE_PATH, SYS_GET_PROCESS_NAME, SYS_GET_PROCESS_ROOT,
    SYS_GET_PROCESS_SECURITY_LABEL, SYS_GET_RESOURCE_NAME, SYS_INJECT_GPU_NEURAL_SEMANTIC,
    SYS_INSPECT_BUS_ENDPOINT, SYS_INSPECT_BUS_PEER, SYS_INSPECT_CONTRACT, SYS_INSPECT_DEVICE,
    SYS_INSPECT_DEVICE_REQUEST, SYS_INSPECT_DOMAIN, SYS_INSPECT_DRIVER, SYS_INSPECT_GPU_BINDING,
    SYS_INSPECT_GPU_BUFFER, SYS_INSPECT_GPU_DISPLAY, SYS_INSPECT_GPU_GSP,
    SYS_INSPECT_GPU_INTERRUPT, SYS_INSPECT_GPU_MEDIA, SYS_INSPECT_GPU_NEURAL,
    SYS_INSPECT_GPU_POWER, SYS_INSPECT_GPU_SCANOUT, SYS_INSPECT_GPU_TENSOR, SYS_INSPECT_GPU_VBIOS,
    SYS_INSPECT_MOUNT, SYS_INSPECT_NETIF, SYS_INSPECT_NETSOCK, SYS_INSPECT_PATH_SECURITY_CONTEXT,
    SYS_INSPECT_PROCESS, SYS_INSPECT_PROCESS_COMPAT, SYS_INSPECT_RESOURCE,
    SYS_INSPECT_STORAGE_LINEAGE, SYS_INSPECT_STORAGE_VOLUME, SYS_INSPECT_SYSTEM_SNAPSHOT,
    SYS_INVOKE_CONTRACT, SYS_LINK_PATH, SYS_LINK_PATH_AT, SYS_LIST_BUS_ENDPOINTS,
    SYS_LIST_BUS_PEERS, SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS, SYS_LIST_PATH, SYS_LIST_PATH_AT,
    SYS_LIST_PROCESSES, SYS_LIST_RESOURCE_WAITERS, SYS_LIST_RESOURCES, SYS_LOAD_MEMORY_WORD,
    SYS_LSTAT_PATH, SYS_LSTAT_PATH_AT, SYS_MAP_ANONYMOUS_MEMORY, SYS_MAP_FILE_MEMORY,
    SYS_MKCHAN_PATH, SYS_MKDIR_PATH, SYS_MKDIR_PATH_AT, SYS_MKFILE_PATH, SYS_MKFILE_PATH_AT,
    SYS_MKSOCK_PATH, SYS_MOUNT_STORAGE_VOLUME, SYS_OPEN_PATH, SYS_OPEN_PATH_AT, SYS_PAUSE_PROCESS,
    SYS_PENDING_SIGNALS, SYS_POLL, SYS_PREPARE_STORAGE_COMMIT, SYS_PRESENT_GPU_FRAME,
    SYS_PROTECT_MEMORY_RANGE, SYS_PUBLISH_BUS_MESSAGE, SYS_QUARANTINE_VM_OBJECT, SYS_READ,
    SYS_READ_GPU_SCANOUT_FRAME, SYS_READ_PROCFS, SYS_READLINK_PATH, SYS_READLINK_PATH_AT,
    SYS_READV, SYS_REAP_PROCESS, SYS_RECEIVE_BUS_MESSAGE, SYS_RECLAIM_MEMORY_PRESSURE,
    SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL, SYS_RECOVER_STORAGE_VOLUME, SYS_RECVFROM_UDP_SOCKET,
    SYS_REGISTER_READINESS, SYS_RELEASE_CLAIMED_RESOURCE, SYS_RELEASE_RESOURCE,
    SYS_RELEASE_VM_OBJECT, SYS_REMOVE_BUS_EVENTS, SYS_REMOVE_GRAPHICS_EVENTS,
    SYS_REMOVE_NET_EVENTS, SYS_REMOVE_PROCESS_EVENTS, SYS_REMOVE_RESOURCE_EVENTS,
    SYS_REMOVE_VFS_EVENTS, SYS_REMOVE_VFS_EVENTS_AT, SYS_RENAME_PATH, SYS_RENAME_PATH_AT,
    SYS_RENICE_PROCESS, SYS_REPAIR_STORAGE_SNAPSHOT, SYS_RESUME_PROCESS, SYS_SEEK, SYS_SEND_SIGNAL,
    SYS_SENDTO_UDP_SOCKET, SYS_SET_CONTRACT_STATE, SYS_SET_FD_RIGHTS, SYS_SET_GPU_POWER_STATE,
    SYS_SET_MOUNT_PROPAGATION, SYS_SET_NETIF_LINK_STATE, SYS_SET_PATH_SECURITY_LABEL,
    SYS_SET_PROCESS_AFFINITY, SYS_SET_PROCESS_ARGS, SYS_SET_PROCESS_BREAK, SYS_SET_PROCESS_CWD,
    SYS_SET_PROCESS_ENV, SYS_SET_PROCESS_IDENTITY, SYS_SET_PROCESS_ROOT,
    SYS_SET_PROCESS_SECURITY_LABEL, SYS_SET_RESOURCE_CONTRACT_POLICY, SYS_SET_RESOURCE_GOVERNANCE,
    SYS_SET_RESOURCE_ISSUER_POLICY, SYS_SET_RESOURCE_POLICY, SYS_SET_RESOURCE_STATE,
    SYS_SPAWN_CONFIGURED_PROCESS, SYS_SPAWN_PATH_PROCESS, SYS_SPAWN_PROCESS_COPY_VM,
    SYS_START_GPU_MEDIA_SESSION, SYS_STAT_PATH, SYS_STAT_PATH_AT, SYS_STATFS_PATH,
    SYS_STORE_MEMORY_WORD, SYS_SUBMIT_GPU_BUFFER, SYS_SYMLINK_PATH, SYS_SYMLINK_PATH_AT,
    SYS_SYNC_MEMORY_RANGE, SYS_TRANSFER_RESOURCE, SYS_TCP_ACCEPT, SYS_TCP_CLOSE, SYS_TCP_CONNECT,
    SYS_TCP_LISTEN, SYS_TCP_RECV, SYS_TCP_RESET, SYS_TCP_SEND, SYS_TRUNCATE_PATH,
    SYS_TRUNCATE_PATH_AT,
    SYS_UNBIND_DEVICE_DRIVER, SYS_UNLINK_PATH, SYS_UNLINK_PATH_AT, SYS_UNMAP_MEMORY_RANGE,
    SYS_UNMOUNT_STORAGE_VOLUME, SYS_WAIT_EVENT_QUEUE, SYS_WATCH_BUS_EVENTS,
    SYS_WATCH_GRAPHICS_EVENTS, SYS_WATCH_NET_EVENTS, SYS_WATCH_PROCESS_EVENTS,
    SYS_WATCH_RESOURCE_EVENTS, SYS_WATCH_VFS_EVENTS, SYS_WATCH_VFS_EVENTS_AT, SYS_WRITE,
    SYS_WRITE_GPU_BUFFER, SYS_WRITEV, SecurityError, SecurityLabel, SeekWhence,
    SubjectSecurityContext, SyscallBackend, SyscallFrame, SyscallNumber, SyscallReturn, UserIoVec,
    check_capability as abi_check_capability, check_ifc_read as abi_check_ifc_read,
    check_ifc_write as abi_check_ifc_write, delegate_capability as abi_delegate_capability,
    derive_completion_provenance as abi_derive_completion_provenance,
    derive_effective_completion_label as abi_derive_effective_completion_label,
    derive_effective_request_label as abi_derive_effective_request_label,
    derive_request_provenance as abi_derive_request_provenance, join_labels as abi_join_labels,
    security_error_to_errno as abi_security_error_to_errno,
    validate_capability_token as abi_validate_capability_token,
    validate_delegation as abi_validate_delegation,
    validate_integrity_tag as abi_validate_integrity_tag,
    validate_label_transition as abi_validate_label_transition,
    validate_object_context as abi_validate_object_context,
    validate_provenance_tag as abi_validate_provenance_tag,
    validate_revocation as abi_validate_revocation, validate_rights as abi_validate_rights,
    validate_subject_context as abi_validate_subject_context,
    verify_integrity_tag as abi_verify_integrity_tag,
};

pub use ngos_user_abi::{
    BlockRightsMask as RuntimeBlockRightsMask, CapabilityToken as RuntimeCapabilityToken,
    CryptographicDna as RuntimeCryptographicDna, IntegrityTag as RuntimeIntegrityTag,
    ObjectSecurityContext as RuntimeObjectSecurityContext, ProvenanceTag as RuntimeProvenanceTag,
    SecurityError as RuntimeSecurityError, SecurityErrorCode as RuntimeSecurityErrorCode,
    SecurityLabel as RuntimeSecurityLabel, SubjectSecurityContext as RuntimeSubjectSecurityContext,
};
pub use wasm::{
    WASM_BOOT_PROOF_COMPONENT, WASM_PROCESS_IDENTITY_COMPONENT, WasmCapability, WasmExecutionError,
    WasmExecutionReport, WasmObservation, WasmVerdict, execute_wasm_component,
};

pub fn validate_rights(
    available: BlockRightsMask,
    required: BlockRightsMask,
) -> Result<(), SecurityError> {
    abi_validate_rights(available, required)
}

pub fn check_ifc_read(subject: SecurityLabel, object: SecurityLabel) -> Result<(), SecurityError> {
    abi_check_ifc_read(subject, object)
}

pub fn check_ifc_write(subject: SecurityLabel, object: SecurityLabel) -> Result<(), SecurityError> {
    abi_check_ifc_write(subject, object)
}

pub const fn join_labels(left: SecurityLabel, right: SecurityLabel) -> SecurityLabel {
    abi_join_labels(left, right)
}

pub fn verify_integrity_tag(
    expected: &IntegrityTag,
    candidate: &IntegrityTag,
) -> Result<(), SecurityError> {
    abi_verify_integrity_tag(expected, candidate)
}

pub fn check_capability(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
    required_rights: BlockRightsMask,
    request_integrity: &IntegrityTag,
) -> Result<(), SecurityError> {
    abi_check_capability(subject, object, token, required_rights, request_integrity)
}

pub fn derive_request_provenance(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
    request_integrity: IntegrityTag,
    edge_id: u64,
) -> ProvenanceTag {
    abi_derive_request_provenance(subject, object, token, request_integrity, edge_id)
}

pub fn derive_completion_provenance(
    request: &ProvenanceTag,
    device_origin_id: u64,
    completion_integrity: IntegrityTag,
    edge_id: u64,
) -> ProvenanceTag {
    abi_derive_completion_provenance(request, device_origin_id, completion_integrity, edge_id)
}

pub fn validate_block_request_security(
    request: &NativeBlockIoRequest,
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
) -> Result<(), SecurityError> {
    request.validate_security(subject, object)
}

pub fn validate_block_completion_security(
    completion: &NativeBlockIoCompletion,
    request: &NativeBlockIoRequest,
) -> Result<(), SecurityError> {
    completion.preserves_security(request)
}

pub fn required_block_rights_for_op(op: u16) -> Option<BlockRightsMask> {
    ngos_user_abi::required_block_rights_for_op(op)
}

#[allow(clippy::too_many_arguments)]
pub fn compose_block_request(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    capability: CapabilityToken,
    op: u16,
    sector: u64,
    sector_count: u32,
    block_size: u32,
    request_label: SecurityLabel,
    request_integrity: IntegrityTag,
    edge_id: u64,
) -> Result<NativeBlockIoRequest, SecurityError> {
    ngos_user_abi::compose_block_request(
        subject,
        object,
        capability,
        op,
        sector,
        sector_count,
        block_size,
        request_label,
        request_integrity,
        edge_id,
    )
}

pub fn compose_block_completion(
    request: &NativeBlockIoRequest,
    device_origin_id: u64,
    status: u32,
    bytes_transferred: u32,
    completion_label: SecurityLabel,
    completion_integrity: IntegrityTag,
    edge_id: u64,
) -> Result<NativeBlockIoCompletion, SecurityError> {
    ngos_user_abi::compose_block_completion(
        request,
        device_origin_id,
        status,
        bytes_transferred,
        completion_label,
        completion_integrity,
        edge_id,
    )
}

pub fn block_request_required_rights(
    request: &NativeBlockIoRequest,
) -> Result<BlockRightsMask, SecurityError> {
    request.required_rights()
}

pub fn validate_subject_context(subject: &SubjectSecurityContext) -> Result<(), SecurityError> {
    abi_validate_subject_context(subject)
}

pub fn validate_object_context(object: &ObjectSecurityContext) -> Result<(), SecurityError> {
    abi_validate_object_context(object)
}

pub fn validate_label_transition(
    from: SecurityLabel,
    to: SecurityLabel,
) -> Result<(), SecurityError> {
    abi_validate_label_transition(from, to)
}

pub const fn security_error_to_errno(error: RuntimeSecurityErrorCode) -> ngos_user_abi::Errno {
    abi_security_error_to_errno(error)
}

pub fn validate_provenance_tag(tag: &ProvenanceTag) -> Result<(), SecurityError> {
    abi_validate_provenance_tag(tag)
}

pub fn validate_integrity_tag(tag: &IntegrityTag) -> Result<(), SecurityError> {
    abi_validate_integrity_tag(tag)
}

pub fn validate_capability_token(
    token: &CapabilityToken,
    current_epoch: u64,
) -> Result<(), SecurityError> {
    abi_validate_capability_token(token, current_epoch)
}

pub fn validate_revocation(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
) -> Result<(), SecurityError> {
    abi_validate_revocation(subject, object, token)
}

pub fn validate_delegation(
    subject: &SubjectSecurityContext,
    object: &ObjectSecurityContext,
    token: &CapabilityToken,
    required_rights: BlockRightsMask,
) -> Result<(), SecurityError> {
    abi_validate_delegation(subject, object, token, required_rights)
}

pub fn delegate_capability(
    parent: &CapabilityToken,
    delegated_subject_id: u64,
    delegated_rights: BlockRightsMask,
    delegated_nonce: u64,
    expiry_epoch: u64,
    authenticator: IntegrityTag,
) -> Result<CapabilityToken, SecurityError> {
    abi_delegate_capability(
        parent,
        delegated_subject_id,
        delegated_rights,
        delegated_nonce,
        expiry_epoch,
        authenticator,
    )
}

pub const fn derive_effective_request_label(
    subject: SecurityLabel,
    object: SecurityLabel,
) -> SecurityLabel {
    abi_derive_effective_request_label(subject, object)
}

pub fn derive_effective_completion_label(
    request: SecurityLabel,
    completion: SecurityLabel,
) -> Result<SecurityLabel, SecurityError> {
    abi_derive_effective_completion_label(request, completion)
}

pub struct Runtime<B> {
    backend: B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceClaimOutcome {
    Acquired {
        resource: usize,
        acquire_count: u64,
    },
    Queued {
        resource: usize,
        holder_contract: usize,
        position: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceReleaseOutcome {
    Released {
        resource: usize,
    },
    HandedOff {
        resource: usize,
        contract: usize,
        acquire_count: u64,
        handoff_count: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceCancelOutcome {
    pub resource: usize,
    pub waiting_count: u64,
}

impl<B> Runtime<B> {
    pub const fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }
}

fn encode_string_table(values: &[&str]) -> Vec<u8> {
    if values.is_empty() {
        return Vec::new();
    }
    let total_len = values
        .iter()
        .fold(0usize, |acc, value| acc.saturating_add(value.len() + 1));
    let mut payload = Vec::with_capacity(total_len);
    for value in values {
        payload.extend_from_slice(value.as_bytes());
        payload.push(0);
    }
    payload
}

impl<B: SyscallBackend> Runtime<B> {
    pub fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
        unsafe { self.backend.syscall(frame) }
    }

    fn invoke(
        &self,
        number: SyscallNumber,
        args: [usize; 6],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.syscall(SyscallFrame::new(number, args)).into_result()
    }

    pub fn read(&self, fd: usize, buffer: &mut [u8]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_READ,
            [fd, buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0],
        )
    }

    pub fn write(&self, fd: usize, buffer: &[u8]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_WRITE,
            [fd, buffer.as_ptr() as usize, buffer.len(), 0, 0, 0],
        )
    }

    pub fn readv(
        &self,
        fd: usize,
        buffers: &mut [&mut [u8]],
    ) -> Result<usize, ngos_user_abi::Errno> {
        let iovecs = buffers
            .iter_mut()
            .map(|buffer| UserIoVec {
                base: buffer.as_mut_ptr() as usize,
                len: buffer.len(),
            })
            .collect::<Vec<_>>();
        self.invoke(
            SYS_READV,
            [fd, iovecs.as_ptr() as usize, iovecs.len(), 0, 0, 0],
        )
    }

    pub fn writev(&self, fd: usize, buffers: &[&[u8]]) -> Result<usize, ngos_user_abi::Errno> {
        let iovecs = buffers
            .iter()
            .map(|buffer| UserIoVec {
                base: buffer.as_ptr() as usize,
                len: buffer.len(),
            })
            .collect::<Vec<_>>();
        self.invoke(
            SYS_WRITEV,
            [fd, iovecs.as_ptr() as usize, iovecs.len(), 0, 0, 0],
        )
    }

    pub fn close(&self, fd: usize) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(SYS_CLOSE, [fd, 0, 0, 0, 0, 0]).map(|_| ())
    }

    pub fn dup(&self, fd: usize) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_DUP, [fd, 0, 0, 0, 0, 0])
    }

    pub fn seek(
        &self,
        fd: usize,
        offset: i64,
        whence: SeekWhence,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_SEEK, [fd, offset as usize, whence as usize, 0, 0, 0])
    }

    pub fn fcntl(&self, fd: usize, cmd: FcntlCmd) -> Result<usize, ngos_user_abi::Errno> {
        let encoded = match cmd {
            FcntlCmd::GetFl => 0,
            FcntlCmd::GetFd => 1,
            FcntlCmd::SetFl { nonblock } => 2 | ((nonblock as usize) << 8),
            FcntlCmd::SetFd { cloexec } => 3 | ((cloexec as usize) << 8),
            FcntlCmd::QueryLock => 4,
            FcntlCmd::TryLockExclusive { token } => 5 | ((token as usize) << 8),
            FcntlCmd::UnlockExclusive { token } => 6 | ((token as usize) << 8),
            FcntlCmd::TryLockShared { token } => 7 | ((token as usize) << 8),
            FcntlCmd::UnlockShared { token } => 8 | ((token as usize) << 8),
            FcntlCmd::UpgradeLockExclusive { token } => 9 | ((token as usize) << 8),
            FcntlCmd::DowngradeLockShared { token } => 10 | ((token as usize) << 8),
        };
        self.invoke(SYS_FCNTL, [fd, encoded, 0, 0, 0, 0])
    }

    pub fn set_fd_rights(
        &self,
        fd: usize,
        rights: BlockRightsMask,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_SET_FD_RIGHTS, [fd, rights.0 as usize, 0, 0, 0, 0])
    }

    pub fn poll(
        &self,
        fd: usize,
        interest: PollEvents,
    ) -> Result<PollEvents, ngos_user_abi::Errno> {
        self.invoke(SYS_POLL, [fd, interest as usize, 0, 0, 0, 0])
            .map(|value| value as PollEvents)
    }

    pub fn control(&self, fd: usize, opcode: u32) -> Result<u32, ngos_user_abi::Errno> {
        self.invoke(SYS_CONTROL_DESCRIPTOR, [fd, opcode as usize, 0, 0, 0, 0])
            .map(|value| value as u32)
    }

    pub fn register_readiness(
        &self,
        fd: usize,
        readable: bool,
        writable: bool,
        priority: bool,
    ) -> Result<(), ngos_user_abi::Errno> {
        let raw = (readable as usize) | ((writable as usize) << 1) | ((priority as usize) << 2);
        self.invoke(SYS_REGISTER_READINESS, [fd, raw, 0, 0, 0, 0])
            .map(|_| ())
    }

    pub fn collect_readiness(
        &self,
        buffer: &mut [NativeReadinessRecord],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_COLLECT_READINESS,
            [buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0],
        )
    }

    pub fn configure_device_queue(
        &self,
        device_path: &str,
        queue_capacity: usize,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_CONFIGURE_DEVICE_QUEUE,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                queue_capacity,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn bind_device_driver(
        &self,
        device_path: &str,
        driver_path: &str,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_BIND_DEVICE_DRIVER,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                driver_path.as_ptr() as usize,
                driver_path.len(),
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn unbind_device_driver(&self, device_path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_UNBIND_DEVICE_DRIVER,
            [device_path.as_ptr() as usize, device_path.len(), 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn create_gpu_buffer(&self, length: usize) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(SYS_CREATE_GPU_BUFFER, [length, 0, 0, 0, 0, 0])
            .map(|value| value as u64)
    }

    pub fn write_gpu_buffer(
        &self,
        buffer_id: u64,
        offset: usize,
        bytes: &[u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_WRITE_GPU_BUFFER,
            [
                buffer_id as usize,
                offset,
                bytes.as_ptr() as usize,
                bytes.len(),
                0,
                0,
            ],
        )
    }

    pub fn submit_gpu_buffer(
        &self,
        device_path: &str,
        buffer_id: u64,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_SUBMIT_GPU_BUFFER,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                buffer_id as usize,
                0,
                0,
                0,
            ],
        )
    }

    pub fn present_gpu_frame(
        &self,
        device_path: &str,
        frame: &[u8],
    ) -> Result<u32, ngos_user_abi::Errno> {
        self.invoke(
            SYS_PRESENT_GPU_FRAME,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                frame.as_ptr() as usize,
                frame.len(),
                0,
                0,
            ],
        )
        .map(|response| response as u32)
    }

    pub fn list_processes(&self, buffer: &mut [u64]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_PROCESSES,
            [buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0],
        )
    }

    pub fn send_signal(&self, pid: u64, signal: u8) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(SYS_SEND_SIGNAL, [pid as usize, signal as usize, 0, 0, 0, 0])
            .map(|_| ())
    }

    pub fn pending_signals(
        &self,
        pid: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_PENDING_SIGNALS,
            [
                pid as usize,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn blocked_pending_signals(
        &self,
        pid: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_BLOCKED_PENDING_SIGNALS,
            [
                pid as usize,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn spawn_path_process(&self, name: &str, path: &str) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(
            SYS_SPAWN_PATH_PROCESS,
            [
                name.as_ptr() as usize,
                name.len(),
                path.as_ptr() as usize,
                path.len(),
                0,
                0,
            ],
        )
        .map(|pid| pid as u64)
    }

    pub fn spawn_process_copy_vm(
        &self,
        name: &str,
        path: &str,
        source_pid: u64,
    ) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(
            SYS_SPAWN_PROCESS_COPY_VM,
            [
                name.as_ptr() as usize,
                name.len(),
                path.as_ptr() as usize,
                path.len(),
                source_pid as usize,
                0,
            ],
        )
        .map(|pid| pid as u64)
    }

    pub fn spawn_configured_process(
        &self,
        name: &str,
        path: &str,
        cwd: &str,
        argv: &[&str],
        envp: &[&str],
    ) -> Result<u64, ngos_user_abi::Errno> {
        let argv_payload = encode_string_table(argv);
        let envp_payload = encode_string_table(envp);
        let config = NativeSpawnProcessConfig {
            name_ptr: name.as_ptr() as usize,
            name_len: name.len(),
            path_ptr: path.as_ptr() as usize,
            path_len: path.len(),
            cwd_ptr: cwd.as_ptr() as usize,
            cwd_len: cwd.len(),
            argv_ptr: argv_payload.as_ptr() as usize,
            argv_len: argv_payload.len(),
            argv_count: argv.len(),
            envp_ptr: envp_payload.as_ptr() as usize,
            envp_len: envp_payload.len(),
            envp_count: envp.len(),
        };
        self.invoke(
            SYS_SPAWN_CONFIGURED_PROCESS,
            [
                (&config as *const NativeSpawnProcessConfig) as usize,
                0,
                0,
                0,
                0,
                0,
            ],
        )
        .map(|pid| pid as u64)
    }

    pub fn set_process_args(&self, pid: u64, argv: &[&str]) -> Result<(), ngos_user_abi::Errno> {
        let payload = encode_string_table(argv);
        self.invoke(
            SYS_SET_PROCESS_ARGS,
            [
                pid as usize,
                payload.as_ptr() as usize,
                payload.len(),
                argv.len(),
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn set_process_env(&self, pid: u64, envp: &[&str]) -> Result<(), ngos_user_abi::Errno> {
        let payload = encode_string_table(envp);
        self.invoke(
            SYS_SET_PROCESS_ENV,
            [
                pid as usize,
                payload.as_ptr() as usize,
                payload.len(),
                envp.len(),
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn set_process_cwd(&self, pid: u64, cwd: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_PROCESS_CWD,
            [pid as usize, cwd.as_ptr() as usize, cwd.len(), 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn set_process_root(&self, pid: u64, root: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_PROCESS_ROOT,
            [pid as usize, root.as_ptr() as usize, root.len(), 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn set_process_identity(
        &self,
        pid: u64,
        identity: &NativeProcessIdentityRecord,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_PROCESS_IDENTITY,
            [
                pid as usize,
                (identity as *const NativeProcessIdentityRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn set_process_security_label(
        &self,
        pid: u64,
        label: &SecurityLabel,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_PROCESS_SECURITY_LABEL,
            [
                pid as usize,
                (label as *const SecurityLabel) as usize,
                0,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn reap_process(&self, pid: u64) -> Result<i32, ngos_user_abi::Errno> {
        self.invoke(SYS_REAP_PROCESS, [pid as usize, 0, 0, 0, 0, 0])
            .map(|code| code as i32)
    }

    pub fn inspect_process(&self, pid: u64) -> Result<NativeProcessRecord, ngos_user_abi::Errno> {
        let mut record = NativeProcessRecord {
            pid: 0,
            parent: 0,
            address_space: 0,
            main_thread: 0,
            state: 0,
            exit_code: 0,
            descriptor_count: 0,
            capability_count: 0,
            environment_count: 0,
            memory_region_count: 0,
            thread_count: 0,
            pending_signal_count: 0,
            session_reported: 0,
            session_status: 0,
            session_stage: 0,
            scheduler_class: 0,
            scheduler_budget: 0,
            cpu_runtime_ticks: 0,
            execution_contract: 0,
            memory_contract: 0,
            io_contract: 0,
            observe_contract: 0,
            reserved: 0,
        };
        self.invoke(
            SYS_INSPECT_PROCESS,
            [
                pid as usize,
                (&mut record as *mut NativeProcessRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_process_compat(
        &self,
        pid: u64,
    ) -> Result<NativeProcessCompatRecord, ngos_user_abi::Errno> {
        let mut record = NativeProcessCompatRecord {
            pid: 0,
            target: [0; 16],
            route_class: [0; 32],
            handle_profile: [0; 32],
            path_profile: [0; 32],
            scheduler_profile: [0; 32],
            sync_profile: [0; 32],
            timer_profile: [0; 32],
            module_profile: [0; 32],
            event_profile: [0; 32],
            requires_kernel_abi_shims: 0,
            prefix: [0; 64],
            executable_path: [0; 64],
            working_dir: [0; 64],
            loader_route_class: [0; 32],
            loader_launch_mode: [0; 32],
            loader_entry_profile: [0; 32],
            loader_requires_compat_shims: 0,
        };
        self.invoke(
            SYS_INSPECT_PROCESS_COMPAT,
            [
                pid as usize,
                (&mut record as *mut NativeProcessCompatRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn bind_process_contract(&self, contract: usize) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(SYS_BIND_PROCESS_CONTRACT, [contract, 0, 0, 0, 0, 0])?;
        Ok(())
    }

    pub fn inspect_system_snapshot(
        &self,
    ) -> Result<NativeSystemSnapshotRecord, ngos_user_abi::Errno> {
        let mut record = NativeSystemSnapshotRecord {
            current_tick: 0,
            busy_ticks: 0,
            process_count: 0,
            active_process_count: 0,
            blocked_process_count: 0,
            queued_processes: 0,
            queued_latency_critical: 0,
            queued_interactive: 0,
            queued_normal: 0,
            queued_background: 0,
            queued_urgent_latency_critical: 0,
            queued_urgent_interactive: 0,
            queued_urgent_normal: 0,
            queued_urgent_background: 0,
            lag_debt_latency_critical: 0,
            lag_debt_interactive: 0,
            lag_debt_normal: 0,
            lag_debt_background: 0,
            dispatch_count_latency_critical: 0,
            dispatch_count_interactive: 0,
            dispatch_count_normal: 0,
            dispatch_count_background: 0,
            runtime_ticks_latency_critical: 0,
            runtime_ticks_interactive: 0,
            runtime_ticks_normal: 0,
            runtime_ticks_background: 0,
            scheduler_cpu_count: 1,
            scheduler_running_cpu: u64::MAX,
            scheduler_cpu_load_imbalance: 0,
            starved_latency_critical: 0,
            starved_interactive: 0,
            starved_normal: 0,
            starved_background: 0,
            deferred_task_count: 0,
            sleeping_processes: 0,
            total_event_queue_count: 0,
            total_event_queue_pending: 0,
            total_event_queue_waiters: 0,
            total_socket_count: 0,
            saturated_socket_count: 0,
            total_socket_rx_depth: 0,
            total_socket_rx_limit: 0,
            max_socket_rx_depth: 0,
            total_network_tx_dropped: 0,
            total_network_rx_dropped: 0,
            running_pid: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.invoke(
            SYS_INSPECT_SYSTEM_SNAPSHOT,
            [
                (&mut record as *mut NativeSystemSnapshotRecord) as usize,
                0,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn get_process_name(
        &self,
        pid: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_GET_PROCESS_NAME,
            [
                pid as usize,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn get_process_image_path(
        &self,
        pid: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_GET_PROCESS_IMAGE_PATH,
            [
                pid as usize,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn chdir_path(&self, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_CHDIR_PATH,
            [path.as_ptr() as usize, path.len(), 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn get_process_cwd(
        &self,
        pid: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_GET_PROCESS_CWD,
            [
                pid as usize,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn get_process_root(
        &self,
        pid: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_GET_PROCESS_ROOT,
            [
                pid as usize,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn get_process_identity(
        &self,
        pid: u64,
    ) -> Result<NativeProcessIdentityRecord, ngos_user_abi::Errno> {
        let mut record = NativeProcessIdentityRecord::default();
        self.invoke(
            SYS_GET_PROCESS_IDENTITY,
            [
                pid as usize,
                (&mut record as *mut NativeProcessIdentityRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn get_process_security_label(
        &self,
        pid: u64,
    ) -> Result<SecurityLabel, ngos_user_abi::Errno> {
        let mut label = SecurityLabel::new(
            ngos_user_abi::ConfidentialityLevel::Public,
            ngos_user_abi::IntegrityLevel::Verified,
        );
        self.invoke(
            SYS_GET_PROCESS_SECURITY_LABEL,
            [
                pid as usize,
                (&mut label as *mut SecurityLabel) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(label)
    }

    pub fn inspect_path_security_context(
        &self,
        path: &str,
    ) -> Result<ObjectSecurityContext, ngos_user_abi::Errno> {
        let mut record = ObjectSecurityContext::new(
            0,
            BlockRightsMask::NONE,
            SecurityLabel::new(
                ngos_user_abi::ConfidentialityLevel::Public,
                ngos_user_abi::IntegrityLevel::Verified,
            ),
            SecurityLabel::new(
                ngos_user_abi::ConfidentialityLevel::Public,
                ngos_user_abi::IntegrityLevel::Verified,
            ),
            ProvenanceTag::root(
                ngos_user_abi::ProvenanceOriginKind::Unknown,
                0,
                0,
                IntegrityTag::zeroed(ngos_user_abi::IntegrityTagKind::Blake3),
            ),
            IntegrityTag::zeroed(ngos_user_abi::IntegrityTagKind::Blake3),
            0,
            0,
        );
        self.invoke(
            SYS_INSPECT_PATH_SECURITY_CONTEXT,
            [
                path.as_ptr() as usize,
                path.len(),
                (&mut record as *mut ObjectSecurityContext) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn set_path_security_label(
        &self,
        path: &str,
        label: &SecurityLabel,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_PATH_SECURITY_LABEL,
            [
                path.as_ptr() as usize,
                path.len(),
                (label as *const SecurityLabel) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn read_procfs(
        &self,
        path: &str,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_READ_PROCFS,
            [
                path.as_ptr() as usize,
                path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
            ],
        )
    }

    pub fn stat_path(&self, path: &str) -> Result<NativeFileStatusRecord, ngos_user_abi::Errno> {
        let mut record = NativeFileStatusRecord {
            inode: 0,
            link_count: 0,
            size: 0,
            kind: 0,
            cloexec: 0,
            nonblock: 0,
            readable: 0,
            writable: 0,
            executable: 0,
            owner_uid: 0,
            group_gid: 0,
            mode: 0,
        };
        self.invoke(
            SYS_STAT_PATH,
            [
                path.as_ptr() as usize,
                path.len(),
                (&mut record as *mut NativeFileStatusRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn stat_path_at(
        &self,
        dir_fd: usize,
        path: &str,
    ) -> Result<NativeFileStatusRecord, ngos_user_abi::Errno> {
        let mut record = NativeFileStatusRecord {
            inode: 0,
            link_count: 0,
            size: 0,
            kind: 0,
            cloexec: 0,
            nonblock: 0,
            readable: 0,
            writable: 0,
            executable: 0,
            owner_uid: 0,
            group_gid: 0,
            mode: 0,
        };
        self.invoke(
            SYS_STAT_PATH_AT,
            [
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                (&mut record as *mut NativeFileStatusRecord) as usize,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn lstat_path(&self, path: &str) -> Result<NativeFileStatusRecord, ngos_user_abi::Errno> {
        let mut record = NativeFileStatusRecord {
            inode: 0,
            link_count: 0,
            size: 0,
            kind: 0,
            cloexec: 0,
            nonblock: 0,
            readable: 0,
            writable: 0,
            executable: 0,
            owner_uid: 0,
            group_gid: 0,
            mode: 0,
        };
        self.invoke(
            SYS_LSTAT_PATH,
            [
                path.as_ptr() as usize,
                path.len(),
                (&mut record as *mut NativeFileStatusRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn lstat_path_at(
        &self,
        dir_fd: usize,
        path: &str,
    ) -> Result<NativeFileStatusRecord, ngos_user_abi::Errno> {
        let mut record = NativeFileStatusRecord {
            inode: 0,
            link_count: 0,
            size: 0,
            kind: 0,
            cloexec: 0,
            nonblock: 0,
            readable: 0,
            writable: 0,
            executable: 0,
            owner_uid: 0,
            group_gid: 0,
            mode: 0,
        };
        self.invoke(
            SYS_LSTAT_PATH_AT,
            [
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                (&mut record as *mut NativeFileStatusRecord) as usize,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn statfs_path(
        &self,
        path: &str,
    ) -> Result<NativeFileSystemStatusRecord, ngos_user_abi::Errno> {
        let mut record = NativeFileSystemStatusRecord {
            mount_count: 0,
            node_count: 0,
            read_only: 0,
            reserved: 0,
        };
        self.invoke(
            SYS_STATFS_PATH,
            [
                path.as_ptr() as usize,
                path.len(),
                (&mut record as *mut NativeFileSystemStatusRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn open_path(&self, path: &str) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_OPEN_PATH,
            [path.as_ptr() as usize, path.len(), 0, 0, 0, 0],
        )
    }

    pub fn open_path_at(&self, dir_fd: usize, path: &str) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_OPEN_PATH_AT,
            [dir_fd, path.as_ptr() as usize, path.len(), 0, 0, 0],
        )
    }

    pub fn readlink_path(
        &self,
        path: &str,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_READLINK_PATH,
            [
                path.as_ptr() as usize,
                path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
            ],
        )
    }

    pub fn readlink_path_at(
        &self,
        dir_fd: usize,
        path: &str,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_READLINK_PATH_AT,
            [
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
            ],
        )
    }

    pub fn mkdir_path(&self, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_MKDIR_PATH,
            [path.as_ptr() as usize, path.len(), 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn mkdir_path_at(&self, dir_fd: usize, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_MKDIR_PATH_AT,
            [dir_fd, path.as_ptr() as usize, path.len(), 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn mkfile_path(&self, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_MKFILE_PATH,
            [path.as_ptr() as usize, path.len(), 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn mkfile_path_at(&self, dir_fd: usize, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_MKFILE_PATH_AT,
            [dir_fd, path.as_ptr() as usize, path.len(), 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn mksock_path(&self, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_MKSOCK_PATH,
            [path.as_ptr() as usize, path.len(), 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn mkchan_path(&self, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_MKCHAN_PATH,
            [path.as_ptr() as usize, path.len(), 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn symlink_path(&self, path: &str, target: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SYMLINK_PATH,
            [
                path.as_ptr() as usize,
                path.len(),
                target.as_ptr() as usize,
                target.len(),
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn symlink_path_at(
        &self,
        dir_fd: usize,
        path: &str,
        target: &str,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SYMLINK_PATH_AT,
            [
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                target.as_ptr() as usize,
                target.len(),
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn rename_path(&self, from: &str, to: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_RENAME_PATH,
            [
                from.as_ptr() as usize,
                from.len(),
                to.as_ptr() as usize,
                to.len(),
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn rename_path_at(
        &self,
        from_dir_fd: usize,
        from: &str,
        to_dir_fd: usize,
        to: &str,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_RENAME_PATH_AT,
            [
                from_dir_fd,
                from.as_ptr() as usize,
                from.len(),
                to_dir_fd,
                to.as_ptr() as usize,
                to.len(),
            ],
        )
        .map(|_| ())
    }

    pub fn unlink_path(&self, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_UNLINK_PATH,
            [path.as_ptr() as usize, path.len(), 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn unlink_path_at(&self, dir_fd: usize, path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_UNLINK_PATH_AT,
            [dir_fd, path.as_ptr() as usize, path.len(), 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn list_path(&self, path: &str, buffer: &mut [u8]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_PATH,
            [
                path.as_ptr() as usize,
                path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
            ],
        )
    }

    pub fn list_path_at(
        &self,
        dir_fd: usize,
        path: &str,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_PATH_AT,
            [
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
            ],
        )
    }

    pub fn truncate_path(&self, path: &str, size: usize) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_TRUNCATE_PATH,
            [path.as_ptr() as usize, path.len(), size, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn truncate_path_at(
        &self,
        dir_fd: usize,
        path: &str,
        size: usize,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_TRUNCATE_PATH_AT,
            [dir_fd, path.as_ptr() as usize, path.len(), size, 0, 0],
        )
        .map(|_| ())
    }

    pub fn link_path(&self, source: &str, destination: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_LINK_PATH,
            [
                source.as_ptr() as usize,
                source.len(),
                destination.as_ptr() as usize,
                destination.len(),
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn link_path_at(
        &self,
        source_dir_fd: usize,
        source: &str,
        destination_dir_fd: usize,
        destination: &str,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_LINK_PATH_AT,
            [
                source_dir_fd,
                source.as_ptr() as usize,
                source.len(),
                destination_dir_fd,
                destination.as_ptr() as usize,
                destination.len(),
            ],
        )
        .map(|_| ())
    }

    pub fn chmod_path_at(
        &self,
        dir_fd: usize,
        path: &str,
        mode: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_CHMOD_PATH_AT,
            [
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                mode as usize,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn chown_path_at(
        &self,
        dir_fd: usize,
        path: &str,
        owner_uid: u32,
        group_gid: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_CHOWN_PATH_AT,
            [
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                owner_uid as usize,
                group_gid as usize,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn configure_network_interface_ipv4(
        &self,
        device_path: &str,
        addr: [u8; 4],
        netmask: [u8; 4],
        gateway: [u8; 4],
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeNetworkInterfaceConfig {
            addr,
            netmask,
            gateway,
        };
        self.invoke(
            SYS_CONFIGURE_NETIF_IPV4,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&config as *const NativeNetworkInterfaceConfig) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn bind_udp_socket(
        &self,
        socket_path: &str,
        device_path: &str,
        local_port: u16,
        remote_ipv4: [u8; 4],
        remote_port: u16,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeUdpBindConfig {
            remote_ipv4,
            local_port,
            remote_port,
        };
        self.invoke(
            SYS_BIND_UDP_SOCKET,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                device_path.as_ptr() as usize,
                device_path.len(),
                (&config as *const NativeUdpBindConfig) as usize,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn inspect_network_interface(
        &self,
        device_path: &str,
    ) -> Result<NativeNetworkInterfaceRecord, ngos_user_abi::Errno> {
        let mut record = NativeNetworkInterfaceRecord {
            admin_up: 0,
            link_up: 0,
            promiscuous: 0,
            reserved: 0,
            mtu: 0,
            tx_capacity: 0,
            rx_capacity: 0,
            tx_inflight_limit: 0,
            tx_inflight_depth: 0,
            free_buffer_count: 0,
            mac: [0; 6],
            mac_reserved: [0; 2],
            ipv4_addr: [0; 4],
            ipv4_netmask: [0; 4],
            ipv4_gateway: [0; 4],
            ipv4_reserved: [0; 4],
            rx_ring_depth: 0,
            tx_ring_depth: 0,
            tx_packets: 0,
            rx_packets: 0,
            tx_completions: 0,
            tx_dropped: 0,
            rx_dropped: 0,
            attached_socket_count: 0,
        };
        self.invoke(
            SYS_INSPECT_NETIF,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeNetworkInterfaceRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_network_socket(
        &self,
        socket_path: &str,
    ) -> Result<NativeNetworkSocketRecord, ngos_user_abi::Errno> {
        let mut record = NativeNetworkSocketRecord {
            local_ipv4: [0; 4],
            remote_ipv4: [0; 4],
            local_port: 0,
            remote_port: 0,
            connected: 0,
            reserved: 0,
            rx_depth: 0,
            rx_queue_limit: 0,
            tx_packets: 0,
            rx_packets: 0,
            dropped_packets: 0,
        };
        self.invoke(
            SYS_INSPECT_NETSOCK,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                (&mut record as *mut NativeNetworkSocketRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_device(
        &self,
        device_path: &str,
    ) -> Result<NativeDeviceRecord, ngos_user_abi::Errno> {
        let mut record = NativeDeviceRecord {
            class: 0,
            state: 0,
            reserved0: 0,
            queue_depth: 0,
            queue_capacity: 0,
            submitted_requests: 0,
            completed_requests: 0,
            total_latency_ticks: 0,
            max_latency_ticks: 0,
            total_queue_wait_ticks: 0,
            max_queue_wait_ticks: 0,
            link_up: 0,
            reserved1: 0,
            block_size: 0,
            reserved2: 0,
            capacity_bytes: 0,
            last_completed_request_id: 0,
            last_completed_frame_tag: [0; 64],
            last_completed_source_api_name: [0; 24],
            last_completed_translation_label: [0; 32],
            last_terminal_request_id: 0,
            last_terminal_state: 0,
            reserved3: 0,
            last_terminal_frame_tag: [0; 64],
            last_terminal_source_api_name: [0; 24],
            last_terminal_translation_label: [0; 32],
        };
        self.invoke(
            SYS_INSPECT_DEVICE,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeDeviceRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_storage_volume(
        &self,
        device_path: &str,
    ) -> Result<NativeStorageVolumeRecord, ngos_user_abi::Errno> {
        let mut record = NativeStorageVolumeRecord {
            valid: 0,
            dirty: 0,
            payload_len: 0,
            generation: 0,
            parent_generation: 0,
            replay_generation: 0,
            payload_checksum: 0,
            superblock_sector: 0,
            journal_sector: 0,
            data_sector: 0,
            index_sector: 0,
            alloc_sector: 0,
            data_start_sector: 0,
            prepared_commit_count: 0,
            recovered_commit_count: 0,
            repaired_snapshot_count: 0,
            allocation_total_blocks: 0,
            allocation_used_blocks: 0,
            mapped_file_count: 0,
            mapped_extent_count: 0,
            mapped_directory_count: 0,
            mapped_symlink_count: 0,
            volume_id: [0; 32],
            state_label: [0; 32],
            last_commit_tag: [0; 32],
            payload_preview: [0; 32],
        };
        self.invoke(
            SYS_INSPECT_STORAGE_VOLUME,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeStorageVolumeRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_storage_lineage(
        &self,
        device_path: &str,
    ) -> Result<NativeStorageLineageRecord, ngos_user_abi::Errno> {
        let mut record = NativeStorageLineageRecord {
            valid: 0,
            lineage_contiguous: 0,
            count: 0,
            newest_generation: 0,
            oldest_generation: 0,
            entries: [ngos_user_abi::NativeStorageLineageEntry {
                generation: 0,
                parent_generation: 0,
                payload_checksum: 0,
                kind_label: [0; 16],
                state_label: [0; 16],
                tag_label: [0; 32],
            }; NATIVE_STORAGE_LINEAGE_DEPTH],
        };
        self.invoke(
            SYS_INSPECT_STORAGE_LINEAGE,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeStorageLineageRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn prepare_storage_commit(
        &self,
        device_path: &str,
        tag: &str,
        payload: &[u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_PREPARE_STORAGE_COMMIT,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                tag.as_ptr() as usize,
                tag.len(),
                payload.as_ptr() as usize,
                payload.len(),
            ],
        )
    }

    pub fn recover_storage_volume(&self, device_path: &str) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_RECOVER_STORAGE_VOLUME,
            [device_path.as_ptr() as usize, device_path.len(), 0, 0, 0, 0],
        )
    }

    pub fn mount_storage_volume(
        &self,
        device_path: &str,
        mount_path: &str,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_MOUNT_STORAGE_VOLUME,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                mount_path.as_ptr() as usize,
                mount_path.len(),
                0,
                0,
            ],
        )
    }

    pub fn unmount_storage_volume(&self, mount_path: &str) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_UNMOUNT_STORAGE_VOLUME,
            [mount_path.as_ptr() as usize, mount_path.len(), 0, 0, 0, 0],
        )
    }

    pub fn inspect_mount(
        &self,
        mount_path: &str,
    ) -> Result<NativeMountRecord, ngos_user_abi::Errno> {
        let mut record = NativeMountRecord {
            id: 0,
            parent_mount_id: 0,
            peer_group: 0,
            master_group: 0,
            layer: 0,
            entry_count: 0,
            propagation_mode: 0,
            created_mount_root: 0,
        };
        self.invoke(
            SYS_INSPECT_MOUNT,
            [
                mount_path.as_ptr() as usize,
                mount_path.len(),
                (&mut record as *mut NativeMountRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn set_mount_propagation(
        &self,
        mount_path: &str,
        mode: NativeMountPropagationMode,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_MOUNT_PROPAGATION,
            [
                mount_path.as_ptr() as usize,
                mount_path.len(),
                mode as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn repair_storage_snapshot(
        &self,
        device_path: &str,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_REPAIR_STORAGE_SNAPSHOT,
            [device_path.as_ptr() as usize, device_path.len(), 0, 0, 0, 0],
        )
    }

    pub fn inspect_gpu_binding(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuBindingRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuBindingRecord {
            present: 0,
            msi_supported: 0,
            msi_message_limit: 0,
            resizable_bar_enabled: 0,
            subsystem_id: 0,
            bar1_total_mib: 0,
            framebuffer_total_mib: 0,
            display_engine_confirmed: 0,
            architecture_name: [0; 32],
            product_name: [0; 64],
            die_name: [0; 16],
            bus_interface: [0; 32],
            inf_section: [0; 32],
            kernel_service: [0; 32],
            vbios_version: [0; 32],
            part_number: [0; 32],
            msi_source_name: [0; 32],
        };
        self.invoke(
            SYS_INSPECT_GPU_BINDING,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuBindingRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_gpu_vbios(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuVbiosRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuVbiosRecord {
            present: 0,
            enabled: 0,
            vendor_id: 0,
            rom_bar_raw: 0,
            device_id: 0,
            physical_base: 0,
            image_len: 0,
            header_len: 0,
            pcir_offset: 0,
            bit_offset: 0,
            nvfw_offset: 0,
            header: [0; 16],
            board_name: [0; 64],
            board_code: [0; 32],
            version: [0; 32],
        };
        self.invoke(
            SYS_INSPECT_GPU_VBIOS,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuVbiosRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_gpu_gsp(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuGspRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuGspRecord {
            present: 0,
            loopback_ready: 0,
            firmware_known: 0,
            blackwell_blob_present: 0,
            hardware_ready: 0,
            driver_model_wddm: 0,
            loopback_completions: 0,
            loopback_failures: 0,
            firmware_version: [0; 16],
            blob_summary: [0; 48],
            refusal_reason: [0; 48],
        };
        self.invoke(
            SYS_INSPECT_GPU_GSP,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuGspRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_gpu_interrupt(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuInterruptRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuInterruptRecord {
            present: 0,
            vector: 0,
            delivered_count: 0,
            msi_supported: 0,
            message_limit: 0,
            windows_interrupt_message_maximum: 0,
            hardware_servicing_confirmed: 0,
        };
        self.invoke(
            SYS_INSPECT_GPU_INTERRUPT,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuInterruptRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_gpu_display(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuDisplayRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuDisplayRecord {
            present: 0,
            active_pipes: 0,
            planned_frames: 0,
            last_present_offset: 0,
            last_present_len: 0,
            hardware_programming_confirmed: 0,
        };
        self.invoke(
            SYS_INSPECT_GPU_DISPLAY,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuDisplayRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_gpu_power(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuPowerRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuPowerRecord {
            present: 0,
            pstate: 0,
            graphics_clock_mhz: 0,
            memory_clock_mhz: 0,
            boost_clock_mhz: 0,
            hardware_power_management_confirmed: 0,
        };
        self.invoke(
            SYS_INSPECT_GPU_POWER,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuPowerRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn set_gpu_power_state(
        &self,
        device_path: &str,
        pstate: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_GPU_POWER_STATE,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                pstate as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(())
    }

    pub fn chmod_path(&self, path: &str, mode: u32) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_CHMOD_PATH,
            [path.as_ptr() as usize, path.len(), mode as usize, 0, 0, 0],
        )?;
        Ok(())
    }

    pub fn chown_path(
        &self,
        path: &str,
        owner_uid: u32,
        group_gid: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_CHOWN_PATH,
            [
                path.as_ptr() as usize,
                path.len(),
                owner_uid as usize,
                group_gid as usize,
                0,
                0,
            ],
        )?;
        Ok(())
    }

    pub fn inspect_gpu_media(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuMediaRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuMediaRecord::default();
        self.invoke(
            SYS_INSPECT_GPU_MEDIA,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuMediaRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn start_gpu_media_session(
        &self,
        device_path: &str,
        width: u32,
        height: u32,
        bitrate_kbps: u32,
        codec: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_START_GPU_MEDIA_SESSION,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                width as usize,
                height as usize,
                bitrate_kbps as usize,
                codec as usize,
            ],
        )?;
        Ok(())
    }

    pub fn inspect_gpu_neural(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuNeuralRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuNeuralRecord::default();
        self.invoke(
            SYS_INSPECT_GPU_NEURAL,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuNeuralRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inject_gpu_neural_semantic(
        &self,
        device_path: &str,
        semantic_label: &str,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_INJECT_GPU_NEURAL_SEMANTIC,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                semantic_label.as_ptr() as usize,
                semantic_label.len(),
                0,
                0,
            ],
        )?;
        Ok(())
    }

    pub fn commit_gpu_neural_frame(&self, device_path: &str) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_COMMIT_GPU_NEURAL_FRAME,
            [device_path.as_ptr() as usize, device_path.len(), 0, 0, 0, 0],
        )?;
        Ok(())
    }

    pub fn inspect_gpu_tensor(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuTensorRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuTensorRecord::default();
        self.invoke(
            SYS_INSPECT_GPU_TENSOR,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuTensorRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn dispatch_gpu_tensor_kernel(
        &self,
        device_path: &str,
        kernel_id: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_DISPATCH_GPU_TENSOR_KERNEL,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                kernel_id as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(())
    }

    pub fn inspect_driver(
        &self,
        driver_path: &str,
    ) -> Result<NativeDriverRecord, ngos_user_abi::Errno> {
        let mut record = NativeDriverRecord {
            state: 0,
            reserved: 0,
            bound_device_count: 0,
            queued_requests: 0,
            in_flight_requests: 0,
            completed_requests: 0,
            last_completed_request_id: 0,
            last_completed_frame_tag: [0; 64],
            last_completed_source_api_name: [0; 24],
            last_completed_translation_label: [0; 32],
            last_terminal_request_id: 0,
            last_terminal_state: 0,
            reserved1: 0,
            last_terminal_frame_tag: [0; 64],
            last_terminal_source_api_name: [0; 24],
            last_terminal_translation_label: [0; 32],
        };
        self.invoke(
            SYS_INSPECT_DRIVER,
            [
                driver_path.as_ptr() as usize,
                driver_path.len(),
                (&mut record as *mut NativeDriverRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_device_request(
        &self,
        request_id: u64,
    ) -> Result<NativeDeviceRequestRecord, ngos_user_abi::Errno> {
        let mut record = NativeDeviceRequestRecord {
            issuer: 0,
            kind: 0,
            state: 0,
            opcode: 0,
            buffer_id: 0,
            payload_len: 0,
            response_len: 0,
            submitted_tick: 0,
            started_tick: 0,
            completed_tick: 0,
            frame_tag: [0; 64],
            source_api_name: [0; 24],
            translation_label: [0; 32],
        };
        self.invoke(
            SYS_INSPECT_DEVICE_REQUEST,
            [
                request_id as usize,
                (&mut record as *mut NativeDeviceRequestRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_gpu_buffer(
        &self,
        buffer_id: u64,
    ) -> Result<NativeGpuBufferRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuBufferRecord {
            owner: 0,
            length: 0,
            used_len: 0,
            reserved: 0,
        };
        self.invoke(
            SYS_INSPECT_GPU_BUFFER,
            [
                buffer_id as usize,
                (&mut record as *mut NativeGpuBufferRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn inspect_gpu_scanout(
        &self,
        device_path: &str,
    ) -> Result<NativeGpuScanoutRecord, ngos_user_abi::Errno> {
        let mut record = NativeGpuScanoutRecord {
            presented_frames: 0,
            last_frame_len: 0,
            last_frame_tag: [0; 64],
            last_source_api_name: [0; 24],
            last_translation_label: [0; 32],
        };
        self.invoke(
            SYS_INSPECT_GPU_SCANOUT,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&mut record as *mut NativeGpuScanoutRecord) as usize,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn read_gpu_scanout_frame(
        &self,
        device_path: &str,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_READ_GPU_SCANOUT_FRAME,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
            ],
        )
    }

    pub fn set_network_interface_link_state(
        &self,
        device_path: &str,
        link_up: bool,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeNetworkLinkStateConfig {
            link_up: link_up as u32,
            reserved: 0,
        };
        self.invoke(
            SYS_SET_NETIF_LINK_STATE,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&config as *const NativeNetworkLinkStateConfig) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure_network_interface_admin(
        &self,
        device_path: &str,
        mtu: usize,
        tx_capacity: usize,
        rx_capacity: usize,
        tx_inflight_limit: usize,
        admin_up: bool,
        promiscuous: bool,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeNetworkAdminConfig {
            mtu: mtu as u64,
            tx_capacity: tx_capacity as u64,
            rx_capacity: rx_capacity as u64,
            tx_inflight_limit: tx_inflight_limit as u64,
            admin_up: admin_up as u32,
            promiscuous: promiscuous as u32,
            reserved0: 0,
            reserved1: 0,
        };
        self.invoke(
            SYS_CONFIGURE_NETIF_ADMIN,
            [
                device_path.as_ptr() as usize,
                device_path.len(),
                (&config as *const NativeNetworkAdminConfig) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn connect_udp_socket(
        &self,
        socket_path: &str,
        remote_ipv4: [u8; 4],
        remote_port: u16,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeUdpConnectConfig {
            remote_ipv4,
            remote_port,
            reserved: 0,
        };
        self.invoke(
            SYS_CONNECT_UDP_SOCKET,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                (&config as *const NativeUdpConnectConfig) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn send_udp_to(
        &self,
        socket_path: &str,
        remote_ipv4: [u8; 4],
        remote_port: u16,
        payload: &[u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        let config = NativeUdpSendToConfig {
            remote_ipv4,
            remote_port,
            reserved: 0,
        };
        self.invoke(
            SYS_SENDTO_UDP_SOCKET,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                (&config as *const NativeUdpSendToConfig) as usize,
                payload.as_ptr() as usize,
                payload.len(),
                0,
            ],
        )
    }

    pub fn recv_udp_from(
        &self,
        socket_path: &str,
        buffer: &mut [u8],
    ) -> Result<(usize, NativeUdpRecvMeta), ngos_user_abi::Errno> {
        let mut meta = NativeUdpRecvMeta {
            remote_ipv4: [0; 4],
            remote_port: 0,
            reserved: 0,
        };
        let count = self.invoke(
            SYS_RECVFROM_UDP_SOCKET,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                (&mut meta as *mut NativeUdpRecvMeta) as usize,
                0,
            ],
        )?;
        Ok((count, meta))
    }

    pub fn tcp_listen(
        &self,
        socket_path: &str,
        device_path: &str,
        local_port: u16,
        backlog: usize,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_TCP_LISTEN,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                device_path.as_ptr() as usize,
                device_path.len(),
                local_port as usize,
                backlog,
            ],
        )
        .map(|_| ())
    }

    pub fn tcp_connect(
        &self,
        socket_path: &str,
        remote_ipv4: [u8; 4],
        remote_port: u16,
    ) -> Result<(), ngos_user_abi::Errno> {
        let ipv4_u32 = u32::from_be_bytes(remote_ipv4);
        self.invoke(
            SYS_TCP_CONNECT,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                ipv4_u32 as usize,
                remote_port as usize,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn tcp_accept(
        &self,
        socket_path: &str,
    ) -> Result<(String, [u8; 4], u16), ngos_user_abi::Errno> {
        let mut out_path = [0u8; 256];
        let mut out_ipv4 = [0u8; 4];
        let mut out_port = 0u16;
        let len = self.invoke(
            SYS_TCP_ACCEPT,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                out_path.as_mut_ptr() as usize,
                out_path.len(),
                (&mut out_ipv4 as *mut [u8; 4]) as usize,
                (&mut out_port as *mut u16) as usize,
            ],
        )?;
        let path_str = core::str::from_utf8(&out_path[..len])
            .map_err(|_| ngos_user_abi::Errno::Inval)?
            .to_string();
        Ok((path_str, out_ipv4, out_port))
    }

    pub fn tcp_send(
        &self,
        socket_path: &str,
        payload: &[u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_TCP_SEND,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                payload.as_ptr() as usize,
                payload.len(),
                0,
                0,
            ],
        )
    }

    pub fn tcp_recv(
        &self,
        socket_path: &str,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_TCP_RECV,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
            ],
        )
    }

    pub fn tcp_close(
        &self,
        socket_path: &str,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_TCP_CLOSE,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                0,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn tcp_reset(
        &self,
        socket_path: &str,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_TCP_RESET,
            [
                socket_path.as_ptr() as usize,
                socket_path.len(),
                0,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn complete_network_tx(
        &self,
        driver_path: &str,
        completions: usize,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_COMPLETE_NET_TX,
            [
                driver_path.as_ptr() as usize,
                driver_path.len(),
                completions,
                0,
                0,
                0,
            ],
        )
    }

    pub fn create_event_queue(
        &self,
        mode: NativeEventQueueMode,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_CREATE_EVENT_QUEUE, [mode as usize, 0, 0, 0, 0, 0])
    }

    pub fn wait_event_queue(
        &self,
        queue_fd: usize,
        buffer: &mut [NativeEventRecord],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_WAIT_EVENT_QUEUE,
            [
                queue_fd,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn watch_process_events(
        &self,
        queue_fd: usize,
        pid: u64,
        token: u64,
        exited: bool,
        reaped: bool,
        poll_events: PollEvents,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeProcessEventWatchConfig {
            token,
            poll_events,
            exited: exited as u32,
            reaped: reaped as u32,
            reserved: 0,
        };
        self.invoke(
            SYS_WATCH_PROCESS_EVENTS,
            [
                queue_fd,
                pid as usize,
                (&config as *const NativeProcessEventWatchConfig) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn remove_process_events(
        &self,
        queue_fd: usize,
        pid: u64,
        token: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_REMOVE_PROCESS_EVENTS,
            [queue_fd, pid as usize, token as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn watch_resource_events(
        &self,
        queue_fd: usize,
        resource: usize,
        token: u64,
        claimed: bool,
        queued: bool,
        canceled: bool,
        released: bool,
        handed_off: bool,
        revoked: bool,
        poll_events: PollEvents,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeResourceEventWatchConfig {
            token,
            poll_events,
            claimed: claimed as u32,
            queued: queued as u32,
            canceled: canceled as u32,
            released: released as u32,
            handed_off: handed_off as u32,
            revoked: revoked as u32,
        };
        self.invoke(
            SYS_WATCH_RESOURCE_EVENTS,
            [
                queue_fd,
                resource,
                (&config as *const NativeResourceEventWatchConfig) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn remove_resource_events(
        &self,
        queue_fd: usize,
        resource: usize,
        token: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_REMOVE_RESOURCE_EVENTS,
            [queue_fd, resource, token as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn watch_network_events(
        &self,
        queue_fd: usize,
        interface_path: &str,
        socket_path: Option<&str>,
        token: u64,
        interest_link_changed: bool,
        interest_rx_ready: bool,
        interest_tx_drained: bool,
        poll_events: PollEvents,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeNetworkEventWatchConfig {
            token,
            poll_events,
            link_changed: interest_link_changed as u32,
            rx_ready: interest_rx_ready as u32,
            tx_drained: interest_tx_drained as u32,
            reserved: 0,
        };
        let (socket_ptr, socket_len) = socket_path
            .map(|path| (path.as_ptr() as usize, path.len()))
            .unwrap_or((0, 0));
        self.invoke(
            SYS_WATCH_NET_EVENTS,
            [
                queue_fd,
                interface_path.as_ptr() as usize,
                interface_path.len(),
                socket_ptr,
                socket_len,
                (&config as *const NativeNetworkEventWatchConfig) as usize,
            ],
        )
        .map(|_| ())
    }

    pub fn remove_network_events(
        &self,
        queue_fd: usize,
        interface_path: &str,
        socket_path: Option<&str>,
        token: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        let (socket_ptr, socket_len) = socket_path
            .map(|path| (path.as_ptr() as usize, path.len()))
            .unwrap_or((0, 0));
        self.invoke(
            SYS_REMOVE_NET_EVENTS,
            [
                queue_fd,
                interface_path.as_ptr() as usize,
                interface_path.len(),
                socket_ptr,
                socket_len,
                token as usize,
            ],
        )
        .map(|_| ())
    }

    pub fn watch_graphics_events(
        &self,
        queue_fd: usize,
        device_path: &str,
        token: u64,
        interest_submitted: bool,
        interest_completed: bool,
        interest_failed: bool,
        interest_drained: bool,
        interest_canceled: bool,
        interest_faulted: bool,
        interest_recovered: bool,
        interest_retired: bool,
        interest_lease_released: bool,
        interest_lease_acquired: bool,
        poll_events: PollEvents,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeGraphicsEventWatchConfig {
            token,
            poll_events,
            submitted: interest_submitted as u32,
            completed: interest_completed as u32,
            failed: interest_failed as u32,
            drained: interest_drained as u32,
            canceled: interest_canceled as u32,
            faulted: interest_faulted as u32,
            recovered: interest_recovered as u32,
            retired: interest_retired as u32,
            lease_released: interest_lease_released as u32,
            lease_acquired: interest_lease_acquired as u32,
        };
        self.invoke(
            SYS_WATCH_GRAPHICS_EVENTS,
            [
                queue_fd,
                device_path.as_ptr() as usize,
                device_path.len(),
                (&config as *const NativeGraphicsEventWatchConfig) as usize,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn remove_graphics_events(
        &self,
        queue_fd: usize,
        device_path: &str,
        token: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_REMOVE_GRAPHICS_EVENTS,
            [
                queue_fd,
                device_path.as_ptr() as usize,
                device_path.len(),
                token as usize,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn watch_vfs_events(
        &self,
        queue_fd: usize,
        path: &str,
        token: u64,
        subtree: bool,
        interest_created: bool,
        interest_opened: bool,
        interest_closed: bool,
        interest_written: bool,
        interest_renamed: bool,
        interest_unlinked: bool,
        interest_mounted: bool,
        interest_unmounted: bool,
        interest_lock_acquired: bool,
        interest_lock_refused: bool,
        interest_permission_refused: bool,
        interest_truncated: bool,
        interest_linked: bool,
        poll_events: PollEvents,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeVfsEventWatchConfig {
            token,
            poll_events,
            subtree: subtree as u32,
            created: interest_created as u32,
            opened: interest_opened as u32,
            closed: interest_closed as u32,
            written: interest_written as u32,
            renamed: interest_renamed as u32,
            unlinked: interest_unlinked as u32,
            mounted: interest_mounted as u32,
            unmounted: interest_unmounted as u32,
            lock_acquired: interest_lock_acquired as u32,
            lock_refused: interest_lock_refused as u32,
            permission_refused: interest_permission_refused as u32,
            truncated: interest_truncated as u32,
            linked: interest_linked as u32,
        };
        self.invoke(
            SYS_WATCH_VFS_EVENTS,
            [
                queue_fd,
                path.as_ptr() as usize,
                path.len(),
                (&config as *const NativeVfsEventWatchConfig) as usize,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn watch_vfs_events_at(
        &self,
        queue_fd: usize,
        dir_fd: usize,
        path: &str,
        token: u64,
        subtree: bool,
        interest_created: bool,
        interest_opened: bool,
        interest_closed: bool,
        interest_written: bool,
        interest_renamed: bool,
        interest_unlinked: bool,
        interest_mounted: bool,
        interest_unmounted: bool,
        interest_lock_acquired: bool,
        interest_lock_refused: bool,
        interest_permission_refused: bool,
        interest_truncated: bool,
        interest_linked: bool,
        poll_events: PollEvents,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeVfsEventWatchConfig {
            token,
            poll_events,
            subtree: subtree as u32,
            created: interest_created as u32,
            opened: interest_opened as u32,
            closed: interest_closed as u32,
            written: interest_written as u32,
            renamed: interest_renamed as u32,
            unlinked: interest_unlinked as u32,
            mounted: interest_mounted as u32,
            unmounted: interest_unmounted as u32,
            lock_acquired: interest_lock_acquired as u32,
            lock_refused: interest_lock_refused as u32,
            permission_refused: interest_permission_refused as u32,
            truncated: interest_truncated as u32,
            linked: interest_linked as u32,
        };
        self.invoke(
            SYS_WATCH_VFS_EVENTS_AT,
            [
                queue_fd,
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                (&config as *const NativeVfsEventWatchConfig) as usize,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn remove_vfs_events(
        &self,
        queue_fd: usize,
        path: &str,
        token: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_REMOVE_VFS_EVENTS,
            [
                queue_fd,
                path.as_ptr() as usize,
                path.len(),
                token as usize,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn remove_vfs_events_at(
        &self,
        queue_fd: usize,
        dir_fd: usize,
        path: &str,
        token: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_REMOVE_VFS_EVENTS_AT,
            [
                queue_fd,
                dir_fd,
                path.as_ptr() as usize,
                path.len(),
                token as usize,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn pause_process(&self, pid: u64) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(SYS_PAUSE_PROCESS, [pid as usize, 0, 0, 0, 0, 0])
            .map(|_| ())
    }

    pub fn resume_process(&self, pid: u64) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(SYS_RESUME_PROCESS, [pid as usize, 0, 0, 0, 0, 0])
            .map(|_| ())
    }

    pub fn load_memory_word(&self, pid: u64, addr: u64) -> Result<u32, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LOAD_MEMORY_WORD,
            [pid as usize, addr as usize, 0, 0, 0, 0],
        )
        .map(|value| value as u32)
    }

    pub fn store_memory_word(
        &self,
        pid: u64,
        addr: u64,
        value: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_STORE_MEMORY_WORD,
            [pid as usize, addr as usize, value as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn quarantine_vm_object(
        &self,
        pid: u64,
        vm_object_id: u64,
        reason: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_QUARANTINE_VM_OBJECT,
            [
                pid as usize,
                vm_object_id as usize,
                reason as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn release_vm_object(
        &self,
        pid: u64,
        vm_object_id: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_RELEASE_VM_OBJECT,
            [pid as usize, vm_object_id as usize, 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn sync_memory_range(
        &self,
        pid: u64,
        start: u64,
        length: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SYNC_MEMORY_RANGE,
            [pid as usize, start as usize, length as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn advise_memory_range(
        &self,
        pid: u64,
        start: u64,
        length: u64,
        advice: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_ADVISE_MEMORY_RANGE,
            [
                pid as usize,
                start as usize,
                length as usize,
                advice as usize,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn protect_memory_range(
        &self,
        pid: u64,
        start: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_PROTECT_MEMORY_RANGE,
            [
                pid as usize,
                start as usize,
                length as usize,
                readable as usize,
                writable as usize,
                executable as usize,
            ],
        )
        .map(|_| ())
    }

    pub fn map_anonymous_memory(
        &self,
        pid: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: &str,
    ) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(
            SYS_MAP_ANONYMOUS_MEMORY,
            [
                pid as usize,
                length as usize,
                readable as usize | ((writable as usize) << 1) | ((executable as usize) << 2),
                label.as_ptr() as usize,
                label.len(),
                0,
            ],
        )
        .map(|value| value as u64)
    }

    pub fn map_file_memory(
        &self,
        pid: u64,
        path: &str,
        length: u64,
        file_offset: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        private: bool,
    ) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(
            SYS_MAP_FILE_MEMORY,
            [
                pid as usize,
                path.as_ptr() as usize,
                path.len(),
                length as usize,
                file_offset as usize,
                readable as usize
                    | ((writable as usize) << 1)
                    | ((executable as usize) << 2)
                    | ((private as usize) << 3),
            ],
        )
        .map(|value| value as u64)
    }

    pub fn unmap_memory_range(
        &self,
        pid: u64,
        start: u64,
        length: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_UNMAP_MEMORY_RANGE,
            [pid as usize, start as usize, length as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn set_process_break(&self, pid: u64, new_end: u64) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_PROCESS_BREAK,
            [pid as usize, new_end as usize, 0, 0, 0, 0],
        )
        .map(|value| value as u64)
    }

    pub fn reclaim_memory_pressure(
        &self,
        pid: u64,
        target_pages: u64,
    ) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(
            SYS_RECLAIM_MEMORY_PRESSURE,
            [pid as usize, target_pages as usize, 0, 0, 0, 0],
        )
        .map(|value| value as u64)
    }

    pub fn reclaim_memory_pressure_global(
        &self,
        target_pages: u64,
    ) -> Result<u64, ngos_user_abi::Errno> {
        self.invoke(
            SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL,
            [target_pages as usize, 0, 0, 0, 0, 0],
        )
        .map(|value| value as u64)
    }

    pub fn renice_process(
        &self,
        pid: u64,
        class: NativeSchedulerClass,
        budget: u32,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_RENICE_PROCESS,
            [pid as usize, class as usize, budget as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn set_process_affinity(
        &self,
        pid: u64,
        cpu_mask: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_PROCESS_AFFINITY,
            [pid as usize, cpu_mask as usize, 0, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn create_domain(
        &self,
        parent: Option<usize>,
        name: &str,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_CREATE_DOMAIN,
            [
                parent.unwrap_or(0),
                name.as_ptr() as usize,
                name.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn create_resource(
        &self,
        domain: usize,
        kind: NativeResourceKind,
        name: &str,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_CREATE_RESOURCE,
            [
                domain,
                kind as usize,
                name.as_ptr() as usize,
                name.len(),
                0,
                0,
            ],
        )
    }

    pub fn create_contract(
        &self,
        domain: usize,
        resource: usize,
        kind: NativeContractKind,
        label: &str,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_CREATE_CONTRACT,
            [
                domain,
                resource,
                kind as usize,
                label.as_ptr() as usize,
                label.len(),
                0,
            ],
        )
    }

    pub fn create_bus_peer(
        &self,
        domain: usize,
        name: &str,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_CREATE_BUS_PEER,
            [domain, name.as_ptr() as usize, name.len(), 0, 0, 0],
        )
    }

    pub fn create_bus_endpoint(
        &self,
        domain: usize,
        resource: usize,
        path: &str,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_CREATE_BUS_ENDPOINT,
            [domain, resource, path.as_ptr() as usize, path.len(), 0, 0],
        )
    }

    pub fn attach_bus_peer(
        &self,
        peer: usize,
        endpoint: usize,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.attach_bus_peer_with_rights(
            peer,
            endpoint,
            BlockRightsMask::READ.union(BlockRightsMask::WRITE),
        )
    }

    pub fn attach_bus_peer_with_rights(
        &self,
        peer: usize,
        endpoint: usize,
        rights: BlockRightsMask,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_ATTACH_BUS_PEER,
            [peer, endpoint, rights.0 as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn detach_bus_peer(
        &self,
        peer: usize,
        endpoint: usize,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(SYS_DETACH_BUS_PEER, [peer, endpoint, 0, 0, 0, 0])
            .map(|_| ())
    }

    pub fn list_domains(&self, buffer: &mut [u64]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_DOMAINS,
            [buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0],
        )
    }

    pub fn inspect_domain(&self, id: usize) -> Result<NativeDomainRecord, ngos_user_abi::Errno> {
        let mut record = NativeDomainRecord {
            id: 0,
            owner: 0,
            parent: 0,
            resource_count: 0,
            contract_count: 0,
        };
        self.invoke(
            SYS_INSPECT_DOMAIN,
            [
                id,
                (&mut record as *mut NativeDomainRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn list_resources(&self, buffer: &mut [u64]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_RESOURCES,
            [buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0],
        )
    }

    pub fn inspect_resource(
        &self,
        id: usize,
    ) -> Result<NativeResourceRecord, ngos_user_abi::Errno> {
        let mut record = NativeResourceRecord {
            id: 0,
            domain: 0,
            creator: 0,
            holder_contract: 0,
            kind: 0,
            state: 0,
            arbitration: 0,
            governance: 0,
            contract_policy: 0,
            issuer_policy: 0,
            waiting_count: 0,
            acquire_count: 0,
            handoff_count: 0,
        };
        self.invoke(
            SYS_INSPECT_RESOURCE,
            [
                id,
                (&mut record as *mut NativeResourceRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn list_bus_peers(&self, buffer: &mut [u64]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_BUS_PEERS,
            [buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0],
        )
    }

    pub fn inspect_bus_peer(&self, id: usize) -> Result<NativeBusPeerRecord, ngos_user_abi::Errno> {
        let mut record = NativeBusPeerRecord {
            id: 0,
            owner: 0,
            domain: 0,
            attached_endpoint_count: 0,
            readable_endpoint_count: 0,
            writable_endpoint_count: 0,
            publish_count: 0,
            receive_count: 0,
            last_endpoint: 0,
        };
        self.invoke(
            SYS_INSPECT_BUS_PEER,
            [
                id,
                (&mut record as *mut NativeBusPeerRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn list_bus_endpoints(&self, buffer: &mut [u64]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_BUS_ENDPOINTS,
            [buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0],
        )
    }

    pub fn inspect_bus_endpoint(
        &self,
        id: usize,
    ) -> Result<NativeBusEndpointRecord, ngos_user_abi::Errno> {
        let mut record = NativeBusEndpointRecord {
            id: 0,
            domain: 0,
            resource: 0,
            kind: 0,
            reserved: 0,
            attached_peer_count: 0,
            readable_peer_count: 0,
            writable_peer_count: 0,
            publish_count: 0,
            receive_count: 0,
            byte_count: 0,
            queue_depth: 0,
            queue_capacity: 0,
            peak_queue_depth: 0,
            overflow_count: 0,
            last_peer: 0,
        };
        self.invoke(
            SYS_INSPECT_BUS_ENDPOINT,
            [
                id,
                (&mut record as *mut NativeBusEndpointRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn publish_bus_message(
        &self,
        peer: usize,
        endpoint: usize,
        bytes: &[u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_PUBLISH_BUS_MESSAGE,
            [peer, endpoint, bytes.as_ptr() as usize, bytes.len(), 0, 0],
        )
    }

    pub fn receive_bus_message(
        &self,
        peer: usize,
        endpoint: usize,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_RECEIVE_BUS_MESSAGE,
            [
                peer,
                endpoint,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
            ],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn watch_bus_events(
        &self,
        queue_fd: usize,
        endpoint: usize,
        token: u64,
        attached: bool,
        detached: bool,
        published: bool,
        received: bool,
        poll_events: PollEvents,
    ) -> Result<(), ngos_user_abi::Errno> {
        let config = NativeBusEventWatchConfig {
            token,
            poll_events,
            attached: attached as u32,
            detached: detached as u32,
            published: published as u32,
            received: received as u32,
            reserved: 0,
        };
        self.invoke(
            SYS_WATCH_BUS_EVENTS,
            [
                queue_fd,
                endpoint,
                (&config as *const NativeBusEventWatchConfig) as usize,
                0,
                0,
                0,
            ],
        )
        .map(|_| ())
    }

    pub fn remove_bus_events(
        &self,
        queue_fd: usize,
        endpoint: usize,
        token: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_REMOVE_BUS_EVENTS,
            [queue_fd, endpoint, token as usize, 0, 0, 0],
        )
        .map(|_| ())
    }

    pub fn list_resource_waiters(
        &self,
        resource: usize,
        buffer: &mut [u64],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_RESOURCE_WAITERS,
            [
                resource,
                buffer.as_mut_ptr() as usize,
                buffer.len(),
                0,
                0,
                0,
            ],
        )
    }

    pub fn list_contracts(&self, buffer: &mut [u64]) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_LIST_CONTRACTS,
            [buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0, 0],
        )
    }

    pub fn inspect_contract(
        &self,
        id: usize,
    ) -> Result<NativeContractRecord, ngos_user_abi::Errno> {
        let mut record = NativeContractRecord {
            id: 0,
            domain: 0,
            resource: 0,
            issuer: 0,
            kind: 0,
            state: 0,
        };
        self.invoke(
            SYS_INSPECT_CONTRACT,
            [
                id,
                (&mut record as *mut NativeContractRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(record)
    }

    pub fn get_domain_name(
        &self,
        id: usize,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_GET_DOMAIN_NAME,
            [id, buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0],
        )
    }

    pub fn get_resource_name(
        &self,
        id: usize,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_GET_RESOURCE_NAME,
            [id, buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0],
        )
    }

    pub fn get_contract_label(
        &self,
        id: usize,
        buffer: &mut [u8],
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(
            SYS_GET_CONTRACT_LABEL,
            [id, buffer.as_mut_ptr() as usize, buffer.len(), 0, 0, 0],
        )
    }

    pub fn set_contract_state(
        &self,
        id: usize,
        state: NativeContractState,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(SYS_SET_CONTRACT_STATE, [id, state as usize, 0, 0, 0, 0])?;
        Ok(())
    }

    pub fn invoke_contract(&self, id: usize) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_INVOKE_CONTRACT, [id, 0, 0, 0, 0, 0])
    }

    pub fn acquire_resource(&self, contract: usize) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_ACQUIRE_RESOURCE, [contract, 0, 0, 0, 0, 0])
    }

    pub fn release_resource(&self, contract: usize) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_RELEASE_RESOURCE, [contract, 0, 0, 0, 0, 0])
    }

    pub fn transfer_resource(
        &self,
        source: usize,
        target: usize,
    ) -> Result<usize, ngos_user_abi::Errno> {
        self.invoke(SYS_TRANSFER_RESOURCE, [source, target, 0, 0, 0, 0])
    }

    pub fn set_resource_arbitration_policy(
        &self,
        resource: usize,
        policy: NativeResourceArbitrationPolicy,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_RESOURCE_POLICY,
            [resource, policy as usize, 0, 0, 0, 0],
        )?;
        Ok(())
    }

    pub fn set_resource_governance_mode(
        &self,
        resource: usize,
        mode: NativeResourceGovernanceMode,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_RESOURCE_GOVERNANCE,
            [resource, mode as usize, 0, 0, 0, 0],
        )?;
        Ok(())
    }

    pub fn set_resource_contract_policy(
        &self,
        resource: usize,
        policy: NativeResourceContractPolicy,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_RESOURCE_CONTRACT_POLICY,
            [resource, policy as usize, 0, 0, 0, 0],
        )?;
        Ok(())
    }

    pub fn set_resource_issuer_policy(
        &self,
        resource: usize,
        policy: NativeResourceIssuerPolicy,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_RESOURCE_ISSUER_POLICY,
            [resource, policy as usize, 0, 0, 0, 0],
        )?;
        Ok(())
    }

    pub fn set_resource_state(
        &self,
        resource: usize,
        state: NativeResourceState,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_SET_RESOURCE_STATE,
            [resource, state as usize, 0, 0, 0, 0],
        )?;
        Ok(())
    }

    pub fn claim_resource(
        &self,
        contract: usize,
    ) -> Result<ResourceClaimOutcome, ngos_user_abi::Errno> {
        let mut record = NativeResourceClaimRecord {
            resource: 0,
            holder_contract: 0,
            acquire_count: 0,
            position: 0,
            queued: 0,
            reserved: 0,
        };
        self.invoke(
            SYS_CLAIM_RESOURCE,
            [
                contract,
                (&mut record as *mut NativeResourceClaimRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(if record.queued == 0 {
            ResourceClaimOutcome::Acquired {
                resource: record.resource as usize,
                acquire_count: record.acquire_count,
            }
        } else {
            ResourceClaimOutcome::Queued {
                resource: record.resource as usize,
                holder_contract: record.holder_contract as usize,
                position: record.position,
            }
        })
    }

    pub fn release_claimed_resource(
        &self,
        contract: usize,
    ) -> Result<ResourceReleaseOutcome, ngos_user_abi::Errno> {
        let mut record = NativeResourceReleaseRecord {
            resource: 0,
            handoff_contract: 0,
            acquire_count: 0,
            handoff_count: 0,
            handed_off: 0,
            reserved: 0,
        };
        self.invoke(
            SYS_RELEASE_CLAIMED_RESOURCE,
            [
                contract,
                (&mut record as *mut NativeResourceReleaseRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(if record.handed_off == 0 {
            ResourceReleaseOutcome::Released {
                resource: record.resource as usize,
            }
        } else {
            ResourceReleaseOutcome::HandedOff {
                resource: record.resource as usize,
                contract: record.handoff_contract as usize,
                acquire_count: record.acquire_count,
                handoff_count: record.handoff_count,
            }
        })
    }

    pub fn cancel_resource_claim(
        &self,
        contract: usize,
    ) -> Result<ResourceCancelOutcome, ngos_user_abi::Errno> {
        let mut record = NativeResourceCancelRecord {
            resource: 0,
            waiting_count: 0,
        };
        self.invoke(
            SYS_CANCEL_RESOURCE_CLAIM,
            [
                contract,
                (&mut record as *mut NativeResourceCancelRecord) as usize,
                0,
                0,
                0,
                0,
            ],
        )?;
        Ok(ResourceCancelOutcome {
            resource: record.resource as usize,
            waiting_count: record.waiting_count,
        })
    }

    pub fn exit(&self, code: ExitCode) -> ! {
        let _ = self.syscall(SyscallFrame::new(SYS_EXIT, [code as usize, 0, 0, 0, 0, 0]));
        loop {
            core::hint::spin_loop();
        }
    }

    pub fn report_boot_session(
        &self,
        status: BootSessionStatus,
        stage: BootSessionStage,
        code: i32,
        detail: u64,
    ) -> Result<(), ngos_user_abi::Errno> {
        self.invoke(
            SYS_BOOT_REPORT,
            [
                status as usize,
                stage as usize,
                code as usize,
                detail as usize,
                0,
                0,
            ],
        )?;
        Ok(())
    }

    pub fn start<F>(self, bootstrap: &BootstrapArgs<'_>, main: F) -> !
    where
        F: FnOnce(&Self, &BootstrapArgs<'_>) -> ExitCode,
    {
        let code = main(&self, bootstrap);
        self.exit(code)
    }
}

#[cfg(target_arch = "x86_64")]
pub struct Amd64SyscallBackend;

#[cfg(target_arch = "x86_64")]
impl SyscallBackend for Amd64SyscallBackend {
    unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
        let mut rax = frame.number as usize;
        let rdi = frame.arg0;
        let rsi = frame.arg1;
        let rdx = frame.arg2;
        let r10 = frame.arg3;
        let r8 = frame.arg4;
        let r9 = frame.arg5;

        unsafe {
            asm!(
                "syscall",
                inlateout("rax") rax,
                in("rdi") rdi,
                in("rsi") rsi,
                in("rdx") rdx,
                in("r10") r10,
                in("r8") r8,
                in("r9") r9,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack),
            );
        }

        SyscallReturn::from_raw(rax as isize)
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub struct Amd64SyscallBackend;

#[cfg(not(target_arch = "x86_64"))]
impl SyscallBackend for Amd64SyscallBackend {
    unsafe fn syscall(&self, _frame: SyscallFrame) -> SyscallReturn {
        panic!("amd64 syscall backend is only available on x86_64")
    }
}

pub fn _start<B, F>(runtime: &Runtime<B>, bootstrap: &BootstrapArgs<'_>, main: F) -> !
where
    B: SyscallBackend,
    F: FnOnce(&Runtime<B>, &BootstrapArgs<'_>) -> ExitCode,
{
    let code = main(runtime, bootstrap);
    runtime.exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::{Cell, RefCell};
    use ngos_user_abi::{
        AuxvEntry, BootSessionStage, BootSessionStatus, Errno, NativeContractKind,
        NativeContractState, NativeResourceArbitrationPolicy, NativeResourceContractPolicy,
        NativeResourceGovernanceMode, NativeResourceIssuerPolicy, NativeResourceKind,
        NativeResourceState, SYS_ACQUIRE_RESOURCE, SYS_BIND_UDP_SOCKET, SYS_BOOT_REPORT,
        SYS_CLAIM_RESOURCE, SYS_CONFIGURE_NETIF_IPV4, SYS_CREATE_CONTRACT, SYS_CREATE_DOMAIN,
        SYS_CREATE_RESOURCE, SYS_EXIT, SYS_FCNTL, SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME,
        SYS_GET_RESOURCE_NAME, SYS_INSPECT_CONTRACT, SYS_INSPECT_DEVICE, SYS_INSPECT_DOMAIN,
        SYS_INSPECT_DRIVER, SYS_INSPECT_NETIF, SYS_INSPECT_NETSOCK, SYS_INSPECT_RESOURCE,
        SYS_INVOKE_CONTRACT, SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS, SYS_LIST_PROCESSES,
        SYS_LIST_RESOURCES, SYS_LSTAT_PATH, SYS_MKSOCK_PATH, SYS_OPEN_PATH, SYS_POLL,
        SYS_READ_PROCFS, SYS_READLINK_PATH, SYS_RELEASE_CLAIMED_RESOURCE, SYS_RELEASE_RESOURCE,
        SYS_SET_CONTRACT_STATE, SYS_SET_RESOURCE_CONTRACT_POLICY, SYS_SET_RESOURCE_GOVERNANCE,
        SYS_SET_RESOURCE_ISSUER_POLICY, SYS_SET_RESOURCE_POLICY, SYS_SET_RESOURCE_STATE,
        SYS_STAT_PATH, SYS_STATFS_PATH, SYS_TRANSFER_RESOURCE, SYS_WRITE,
    };
    use ngos_user_abi::{
        BlockRightsMask, CapabilityToken, ConfidentialityLevel, IntegrityLevel, IntegrityTag,
        IntegrityTagKind, ObjectSecurityContext, ProvenanceOriginKind, ProvenanceTag,
        SecurityErrorCode, SecurityLabel, SubjectSecurityContext,
    };

    #[derive(Default)]
    struct DummyBackend;

    impl SyscallBackend for DummyBackend {
        unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
            if frame.number == SYS_WRITE {
                SyscallReturn::ok(frame.arg2)
            } else {
                SyscallReturn::err(ngos_user_abi::Errno::Inval)
            }
        }
    }

    #[test]
    fn bootstrap_builder_emits_expected_stack_layout() {
        let bootstrap = BootstrapArgs::new(
            &["prog", "arg1"],
            &["A=1", "B=2"],
            &[AuxvEntry { key: 7, value: 11 }],
        );
        let image = bootstrap::build_initial_stack(0x8000, &bootstrap).unwrap();
        assert_eq!(image.argc, 2);
        assert_eq!(image.argv_addrs.len(), 2);
        assert_eq!(image.envp_addrs.len(), 2);
        assert!(image.stack_base < image.stack_top);
        assert_eq!(image.bytes.len(), image.stack_top - image.stack_base);
        assert!(image.bytes.windows(4).any(|window| window == b"prog"));
    }

    #[test]
    fn syscall_wrapper_decodes_success_and_error() {
        let runtime = Runtime::new(DummyBackend);
        let buffer = [0u8; 8];
        let written = runtime.write(1, &buffer).unwrap();
        assert_eq!(written, buffer.len());
        assert_eq!(
            runtime
                .syscall(SyscallFrame::new(999, [0, 0, 0, 0, 0, 0]))
                .into_result(),
            Err(ngos_user_abi::Errno::Inval)
        );
    }

    struct RecordingBackend {
        last: RefCell<Option<SyscallFrame>>,
        response: Cell<SyscallReturn>,
    }

    impl RecordingBackend {
        fn new(response: SyscallReturn) -> Self {
            Self {
                last: RefCell::new(None),
                response: Cell::new(response),
            }
        }

        fn last_frame(&self) -> SyscallFrame {
            self.last.borrow().expect("expected syscall frame")
        }
    }

    impl SyscallBackend for RecordingBackend {
        unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
            *self.last.borrow_mut() = Some(frame);
            self.response.get()
        }
    }

    #[test]
    fn write_wrapper_emits_expected_syscall_frame() {
        let backend = RecordingBackend::new(SyscallReturn::ok(5));
        let runtime = Runtime::new(backend);
        let payload = *b"hello";
        let wrote = runtime.write(7, &payload).unwrap();
        assert_eq!(wrote, payload.len());

        let frame = runtime.backend().last_frame();
        assert_eq!(frame.number, SYS_WRITE);
        assert_eq!(frame.arg0, 7);
        assert_eq!(frame.arg2, payload.len());
        assert_eq!(frame.arg3, 0);
        assert_eq!(frame.arg4, 0);
        assert_eq!(frame.arg5, 0);
    }

    #[test]
    fn readv_and_writev_wrappers_emit_expected_syscall_frames() {
        let runtime = Runtime::new(RecordingBackend::new(SyscallReturn::ok(7)));
        let mut a = [0u8; 2];
        let mut b = [0u8; 5];
        let mut reads: [&mut [u8]; 2] = [&mut a, &mut b];
        assert_eq!(runtime.readv(4, &mut reads).unwrap(), 7);
        let readv = runtime.backend().last_frame();
        assert_eq!(readv.number, SYS_READV);
        assert_eq!(readv.arg0, 4);
        assert_eq!(readv.arg2, 2);

        let left = b"ab";
        let right = b"cdefg";
        assert_eq!(
            runtime
                .writev(5, &[left.as_slice(), right.as_slice()])
                .unwrap(),
            7
        );
        let writev = runtime.backend().last_frame();
        assert_eq!(writev.number, SYS_WRITEV);
        assert_eq!(writev.arg0, 5);
        assert_eq!(writev.arg2, 2);
    }

    #[test]
    fn fcntl_and_poll_wrappers_encode_arguments_as_abi_contract() {
        let runtime = Runtime::new(RecordingBackend::new(SyscallReturn::ok(0)));
        runtime
            .fcntl(9, ngos_user_abi::FcntlCmd::SetFd { cloexec: true })
            .unwrap();
        let fcntl_frame = runtime.backend().last_frame();
        assert_eq!(fcntl_frame.number, SYS_FCNTL);
        assert_eq!(fcntl_frame.arg0, 9);
        assert_eq!(fcntl_frame.arg1, 3 | (1 << 8));

        runtime.poll(4, 0x11).unwrap();
        let poll_frame = runtime.backend().last_frame();
        assert_eq!(poll_frame.number, SYS_POLL);
        assert_eq!(poll_frame.arg0, 4);
        assert_eq!(poll_frame.arg1, 0x11);
    }

    #[test]
    fn native_model_wrappers_encode_arguments_as_abi_contract() {
        let runtime = Runtime::new(RecordingBackend::new(SyscallReturn::ok(41)));

        let domain = runtime.create_domain(None, "graphics").unwrap();
        assert_eq!(domain, 41);
        let domain_frame = runtime.backend().last_frame();
        assert_eq!(domain_frame.number, SYS_CREATE_DOMAIN);
        assert_eq!(domain_frame.arg0, 0);
        assert_eq!(domain_frame.arg2, "graphics".len());

        runtime
            .create_resource(domain, NativeResourceKind::Device, "gpu0")
            .unwrap();
        let resource_frame = runtime.backend().last_frame();
        assert_eq!(resource_frame.number, SYS_CREATE_RESOURCE);
        assert_eq!(resource_frame.arg0, domain);
        assert_eq!(resource_frame.arg1, NativeResourceKind::Device as usize);
        assert_eq!(resource_frame.arg3, "gpu0".len());

        runtime
            .create_contract(domain, 77, NativeContractKind::Display, "scanout")
            .unwrap();
        let contract_frame = runtime.backend().last_frame();
        assert_eq!(contract_frame.number, SYS_CREATE_CONTRACT);
        assert_eq!(contract_frame.arg0, domain);
        assert_eq!(contract_frame.arg1, 77);
        assert_eq!(contract_frame.arg2, NativeContractKind::Display as usize);
        assert_eq!(contract_frame.arg4, "scanout".len());

        runtime.create_bus_peer(domain, "renderer").unwrap();
        let bus_peer_frame = runtime.backend().last_frame();
        assert_eq!(bus_peer_frame.number, SYS_CREATE_BUS_PEER);
        assert_eq!(bus_peer_frame.arg0, domain);
        assert_eq!(bus_peer_frame.arg2, "renderer".len());

        runtime
            .create_bus_endpoint(domain, 77, "/ipc/render")
            .unwrap();
        let bus_endpoint_frame = runtime.backend().last_frame();
        assert_eq!(bus_endpoint_frame.number, SYS_CREATE_BUS_ENDPOINT);
        assert_eq!(bus_endpoint_frame.arg0, domain);
        assert_eq!(bus_endpoint_frame.arg1, 77);
        assert_eq!(bus_endpoint_frame.arg3, "/ipc/render".len());

        runtime.attach_bus_peer(11, 22).unwrap();
        let attach_frame = runtime.backend().last_frame();
        assert_eq!(attach_frame.number, SYS_ATTACH_BUS_PEER);
        assert_eq!(attach_frame.arg0, 11);
        assert_eq!(attach_frame.arg1, 22);
        assert_eq!(
            attach_frame.arg2,
            BlockRightsMask::READ.union(BlockRightsMask::WRITE).0 as usize
        );

        runtime.detach_bus_peer(11, 22).unwrap();
        let detach_frame = runtime.backend().last_frame();
        assert_eq!(detach_frame.number, SYS_DETACH_BUS_PEER);
        assert_eq!(detach_frame.arg0, 11);
        assert_eq!(detach_frame.arg1, 22);

        runtime
            .watch_bus_events(9, 22, 901, true, true, true, true, ngos_user_abi::POLLPRI)
            .unwrap();
        let watch_bus_frame = runtime.backend().last_frame();
        assert_eq!(watch_bus_frame.number, SYS_WATCH_BUS_EVENTS);
        assert_eq!(watch_bus_frame.arg0, 9);
        assert_eq!(watch_bus_frame.arg1, 22);

        runtime.remove_bus_events(9, 22, 901).unwrap();
        let remove_bus_frame = runtime.backend().last_frame();
        assert_eq!(remove_bus_frame.number, SYS_REMOVE_BUS_EVENTS);
        assert_eq!(remove_bus_frame.arg0, 9);
        assert_eq!(remove_bus_frame.arg1, 22);
        assert_eq!(remove_bus_frame.arg2, 901);

        runtime.set_process_affinity(7, 0b11).unwrap();
        let affinity_frame = runtime.backend().last_frame();
        assert_eq!(affinity_frame.number, SYS_SET_PROCESS_AFFINITY);
        assert_eq!(affinity_frame.arg0, 7);
        assert_eq!(affinity_frame.arg1, 0b11);
    }

    #[test]
    fn native_model_query_wrappers_encode_buffer_and_record_pointers() {
        let runtime = Runtime::new(RecordingBackend::new(SyscallReturn::ok(3)));
        let mut ids = [0u64; 4];
        assert_eq!(runtime.list_domains(&mut ids).unwrap(), 3);
        let list_domains = runtime.backend().last_frame();
        assert_eq!(list_domains.number, SYS_LIST_DOMAINS);
        assert_eq!(list_domains.arg1, ids.len());

        assert_eq!(runtime.list_resources(&mut ids).unwrap(), 3);
        let list_resources = runtime.backend().last_frame();
        assert_eq!(list_resources.number, SYS_LIST_RESOURCES);
        assert_eq!(list_resources.arg1, ids.len());

        assert_eq!(runtime.list_contracts(&mut ids).unwrap(), 3);
        let list_contracts = runtime.backend().last_frame();
        assert_eq!(list_contracts.number, SYS_LIST_CONTRACTS);
        assert_eq!(list_contracts.arg1, ids.len());

        assert_eq!(runtime.list_processes(&mut ids).unwrap(), 3);
        let list_processes = runtime.backend().last_frame();
        assert_eq!(list_processes.number, SYS_LIST_PROCESSES);
        assert_eq!(list_processes.arg1, ids.len());

        runtime.send_signal(7, 9).unwrap();
        let send_signal = runtime.backend().last_frame();
        assert_eq!(send_signal.number, SYS_SEND_SIGNAL);
        assert_eq!(send_signal.arg0, 7);
        assert_eq!(send_signal.arg1, 9);

        let mut signals = [0u8; 8];
        runtime.pending_signals(7, &mut signals).unwrap();
        let pending_signals = runtime.backend().last_frame();
        assert_eq!(pending_signals.number, SYS_PENDING_SIGNALS);
        assert_eq!(pending_signals.arg0, 7);
        assert_eq!(pending_signals.arg2, signals.len());

        runtime.blocked_pending_signals(7, &mut signals).unwrap();
        let blocked_pending_signals = runtime.backend().last_frame();
        assert_eq!(blocked_pending_signals.number, SYS_BLOCKED_PENDING_SIGNALS);
        assert_eq!(blocked_pending_signals.arg0, 7);
        assert_eq!(blocked_pending_signals.arg2, signals.len());

        assert_eq!(
            runtime.spawn_path_process("worker", "/bin/worker").unwrap(),
            3
        );
        let spawn_process = runtime.backend().last_frame();
        assert_eq!(spawn_process.number, SYS_SPAWN_PATH_PROCESS);
        assert_eq!(spawn_process.arg1, "worker".len());
        assert_eq!(spawn_process.arg3, "/bin/worker".len());

        runtime
            .set_process_args(3, &["/bin/worker", "--fullscreen", "--vsync"])
            .unwrap();
        let set_args = runtime.backend().last_frame();
        assert_eq!(set_args.number, SYS_SET_PROCESS_ARGS);
        assert_eq!(set_args.arg0, 3);
        assert_eq!(set_args.arg3, 3);
        assert_ne!(set_args.arg1, 0);
        assert_ne!(set_args.arg2, 0);

        runtime
            .set_process_env(
                3,
                &[
                    "NGOS_COMPAT_PREFIX=/compat/orbit",
                    "NGOS_GFX_BACKEND=vulkan",
                ],
            )
            .unwrap();
        let set_env = runtime.backend().last_frame();
        assert_eq!(set_env.number, SYS_SET_PROCESS_ENV);
        assert_eq!(set_env.arg0, 3);
        assert_eq!(set_env.arg3, 2);
        assert_ne!(set_env.arg1, 0);
        assert_ne!(set_env.arg2, 0);

        runtime.set_process_cwd(3, "/games/orbit").unwrap();
        let set_cwd = runtime.backend().last_frame();
        assert_eq!(set_cwd.number, SYS_SET_PROCESS_CWD);
        assert_eq!(set_cwd.arg0, 3);
        assert_eq!(set_cwd.arg2, "/games/orbit".len());

        assert_eq!(runtime.reap_process(3).unwrap(), 3);
        let reap_process = runtime.backend().last_frame();
        assert_eq!(reap_process.number, SYS_REAP_PROCESS);
        assert_eq!(reap_process.arg0, 3);

        runtime.inspect_process(3).unwrap();
        let inspect_process = runtime.backend().last_frame();
        assert_eq!(inspect_process.number, SYS_INSPECT_PROCESS);
        assert_eq!(inspect_process.arg0, 3);
        assert_ne!(inspect_process.arg1, 0);

        let mut text = [0u8; 32];
        runtime.get_process_name(3, &mut text).unwrap();
        let process_name = runtime.backend().last_frame();
        assert_eq!(process_name.number, SYS_GET_PROCESS_NAME);
        assert_eq!(process_name.arg0, 3);
        assert_eq!(process_name.arg2, text.len());

        runtime.get_process_image_path(3, &mut text).unwrap();
        let process_image = runtime.backend().last_frame();
        assert_eq!(process_image.number, SYS_GET_PROCESS_IMAGE_PATH);
        assert_eq!(process_image.arg0, 3);
        assert_eq!(process_image.arg2, text.len());

        runtime.get_process_cwd(3, &mut text).unwrap();
        let process_cwd = runtime.backend().last_frame();
        assert_eq!(process_cwd.number, SYS_GET_PROCESS_CWD);
        assert_eq!(process_cwd.arg0, 3);
        assert_eq!(process_cwd.arg2, text.len());

        runtime.inspect_domain(11).unwrap();
        let inspect_domain = runtime.backend().last_frame();
        assert_eq!(inspect_domain.number, SYS_INSPECT_DOMAIN);
        assert_eq!(inspect_domain.arg0, 11);

        runtime.inspect_resource(12).unwrap();
        let inspect_resource = runtime.backend().last_frame();
        assert_eq!(inspect_resource.number, SYS_INSPECT_RESOURCE);
        assert_eq!(inspect_resource.arg0, 12);

        runtime.inspect_contract(13).unwrap();
        let inspect_contract = runtime.backend().last_frame();
        assert_eq!(inspect_contract.number, SYS_INSPECT_CONTRACT);
        assert_eq!(inspect_contract.arg0, 13);

        assert_eq!(runtime.list_bus_peers(&mut ids).unwrap(), 3);
        let list_bus_peers = runtime.backend().last_frame();
        assert_eq!(list_bus_peers.number, SYS_LIST_BUS_PEERS);
        assert_eq!(list_bus_peers.arg1, ids.len());

        runtime.inspect_bus_peer(21).unwrap();
        let inspect_bus_peer = runtime.backend().last_frame();
        assert_eq!(inspect_bus_peer.number, SYS_INSPECT_BUS_PEER);
        assert_eq!(inspect_bus_peer.arg0, 21);

        assert_eq!(runtime.list_bus_endpoints(&mut ids).unwrap(), 3);
        let list_bus_endpoints = runtime.backend().last_frame();
        assert_eq!(list_bus_endpoints.number, SYS_LIST_BUS_ENDPOINTS);
        assert_eq!(list_bus_endpoints.arg1, ids.len());

        runtime.inspect_bus_endpoint(22).unwrap();
        let inspect_bus_endpoint = runtime.backend().last_frame();
        assert_eq!(inspect_bus_endpoint.number, SYS_INSPECT_BUS_ENDPOINT);
        assert_eq!(inspect_bus_endpoint.arg0, 22);

        let mut text = [0u8; 16];
        runtime.get_domain_name(11, &mut text).unwrap();
        let domain_name = runtime.backend().last_frame();
        assert_eq!(domain_name.number, SYS_GET_DOMAIN_NAME);
        assert_eq!(domain_name.arg0, 11);
        assert_eq!(domain_name.arg2, text.len());

        runtime.get_resource_name(12, &mut text).unwrap();
        let resource_name = runtime.backend().last_frame();
        assert_eq!(resource_name.number, SYS_GET_RESOURCE_NAME);
        assert_eq!(resource_name.arg0, 12);

        let payload = [1u8, 2, 3, 4];
        assert_eq!(runtime.publish_bus_message(11, 22, &payload).unwrap(), 3);
        let publish_bus = runtime.backend().last_frame();
        assert_eq!(publish_bus.number, SYS_PUBLISH_BUS_MESSAGE);
        assert_eq!(publish_bus.arg0, 11);
        assert_eq!(publish_bus.arg1, 22);
        assert_eq!(publish_bus.arg3, payload.len());

        let mut bus_buffer = [0u8; 16];
        assert_eq!(
            runtime
                .receive_bus_message(11, 22, &mut bus_buffer)
                .unwrap(),
            3
        );
        let receive_bus = runtime.backend().last_frame();
        assert_eq!(receive_bus.number, SYS_RECEIVE_BUS_MESSAGE);
        assert_eq!(receive_bus.arg0, 11);
        assert_eq!(receive_bus.arg1, 22);
        assert_eq!(receive_bus.arg3, bus_buffer.len());

        runtime.get_contract_label(13, &mut text).unwrap();
        let contract_label = runtime.backend().last_frame();
        assert_eq!(contract_label.number, SYS_GET_CONTRACT_LABEL);
        assert_eq!(contract_label.arg0, 13);

        let mut procfs = [0u8; 64];
        runtime.read_procfs("/proc/7/status", &mut procfs).unwrap();
        let procfs_frame = runtime.backend().last_frame();
        assert_eq!(procfs_frame.number, SYS_READ_PROCFS);
        assert_eq!(procfs_frame.arg1, "/proc/7/status".len());
        assert_eq!(procfs_frame.arg3, procfs.len());

        runtime.quarantine_vm_object(7, 33, 5).unwrap();
        let quarantine_frame = runtime.backend().last_frame();
        assert_eq!(quarantine_frame.number, SYS_QUARANTINE_VM_OBJECT);
        assert_eq!(quarantine_frame.arg0, 7);
        assert_eq!(quarantine_frame.arg1, 33);
        assert_eq!(quarantine_frame.arg2, 5);

        runtime.release_vm_object(7, 33).unwrap();
        let release_frame = runtime.backend().last_frame();
        assert_eq!(release_frame.number, SYS_RELEASE_VM_OBJECT);
        assert_eq!(release_frame.arg0, 7);
        assert_eq!(release_frame.arg1, 33);

        runtime.reclaim_memory_pressure(7, 4).unwrap();
        let reclaim_frame = runtime.backend().last_frame();
        assert_eq!(reclaim_frame.number, SYS_RECLAIM_MEMORY_PRESSURE);
        assert_eq!(reclaim_frame.arg0, 7);
        assert_eq!(reclaim_frame.arg1, 4);

        runtime.reclaim_memory_pressure_global(9).unwrap();
        let reclaim_global_frame = runtime.backend().last_frame();
        assert_eq!(
            reclaim_global_frame.number,
            SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL
        );
        assert_eq!(reclaim_global_frame.arg0, 9);

        runtime.stat_path("/proc/7/status").unwrap();
        let stat_frame = runtime.backend().last_frame();
        assert_eq!(stat_frame.number, SYS_STAT_PATH);
        assert_eq!(stat_frame.arg1, "/proc/7/status".len());

        runtime.lstat_path("/proc/7/status").unwrap();
        let lstat_frame = runtime.backend().last_frame();
        assert_eq!(lstat_frame.number, SYS_LSTAT_PATH);
        assert_eq!(lstat_frame.arg1, "/proc/7/status".len());

        runtime.statfs_path("/proc/7/status").unwrap();
        let statfs_frame = runtime.backend().last_frame();
        assert_eq!(statfs_frame.number, SYS_STATFS_PATH);
        assert_eq!(statfs_frame.arg1, "/proc/7/status".len());
        assert_ne!(statfs_frame.arg2, 0);

        runtime.open_path("/proc/7/status").unwrap();
        let open_frame = runtime.backend().last_frame();
        assert_eq!(open_frame.number, SYS_OPEN_PATH);
        assert_eq!(open_frame.arg1, "/proc/7/status".len());

        runtime.mksock_path("/run/net0.sock").unwrap();
        let mksock = runtime.backend().last_frame();
        assert_eq!(mksock.number, SYS_MKSOCK_PATH);
        assert_eq!(mksock.arg1, "/run/net0.sock".len());

        runtime.mkchan_path("/run/game.chan").unwrap();
        let mkchan = runtime.backend().last_frame();
        assert_eq!(mkchan.number, SYS_MKCHAN_PATH);
        assert_eq!(mkchan.arg1, "/run/game.chan".len());

        runtime.readlink_path("/proc/7/cwd", &mut text).unwrap();
        let readlink_frame = runtime.backend().last_frame();
        assert_eq!(readlink_frame.number, SYS_READLINK_PATH);
        assert_eq!(readlink_frame.arg1, "/proc/7/cwd".len());
        assert_eq!(readlink_frame.arg3, text.len());

        runtime
            .set_contract_state(13, NativeContractState::Suspended)
            .unwrap();
        let set_state = runtime.backend().last_frame();
        assert_eq!(set_state.number, SYS_SET_CONTRACT_STATE);
        assert_eq!(set_state.arg0, 13);
        assert_eq!(set_state.arg1, NativeContractState::Suspended as usize);

        runtime.invoke_contract(13).unwrap();
        let invoke_contract = runtime.backend().last_frame();
        assert_eq!(invoke_contract.number, SYS_INVOKE_CONTRACT);
        assert_eq!(invoke_contract.arg0, 13);

        runtime.acquire_resource(13).unwrap();
        let acquire_resource = runtime.backend().last_frame();
        assert_eq!(acquire_resource.number, SYS_ACQUIRE_RESOURCE);
        assert_eq!(acquire_resource.arg0, 13);

        runtime.release_resource(13).unwrap();
        let release_resource = runtime.backend().last_frame();
        assert_eq!(release_resource.number, SYS_RELEASE_RESOURCE);
        assert_eq!(release_resource.arg0, 13);

        runtime.transfer_resource(13, 17).unwrap();
        let transfer_resource = runtime.backend().last_frame();
        assert_eq!(transfer_resource.number, SYS_TRANSFER_RESOURCE);
        assert_eq!(transfer_resource.arg0, 13);
        assert_eq!(transfer_resource.arg1, 17);

        runtime
            .set_resource_arbitration_policy(42, NativeResourceArbitrationPolicy::Lifo)
            .unwrap();
        let set_policy = runtime.backend().last_frame();
        assert_eq!(set_policy.number, SYS_SET_RESOURCE_POLICY);
        assert_eq!(set_policy.arg0, 42);
        assert_eq!(
            set_policy.arg1,
            NativeResourceArbitrationPolicy::Lifo as usize
        );

        runtime.claim_resource(13).unwrap();
        let claim_resource = runtime.backend().last_frame();
        assert_eq!(claim_resource.number, SYS_CLAIM_RESOURCE);
        assert_eq!(claim_resource.arg0, 13);

        runtime.release_claimed_resource(13).unwrap();
        let release_claimed_resource = runtime.backend().last_frame();
        assert_eq!(
            release_claimed_resource.number,
            SYS_RELEASE_CLAIMED_RESOURCE
        );
        assert_eq!(release_claimed_resource.arg0, 13);

        runtime.list_resource_waiters(42, &mut ids).unwrap();
        let list_waiters = runtime.backend().last_frame();
        assert_eq!(list_waiters.number, SYS_LIST_RESOURCE_WAITERS);
        assert_eq!(list_waiters.arg0, 42);
        assert_eq!(list_waiters.arg2, ids.len());

        runtime.cancel_resource_claim(13).unwrap();
        let cancel_claim = runtime.backend().last_frame();
        assert_eq!(cancel_claim.number, SYS_CANCEL_RESOURCE_CLAIM);
        assert_eq!(cancel_claim.arg0, 13);

        runtime
            .set_resource_governance_mode(42, NativeResourceGovernanceMode::ExclusiveLease)
            .unwrap();
        let set_governance = runtime.backend().last_frame();
        assert_eq!(set_governance.number, SYS_SET_RESOURCE_GOVERNANCE);
        assert_eq!(set_governance.arg0, 42);
        assert_eq!(
            set_governance.arg1,
            NativeResourceGovernanceMode::ExclusiveLease as usize
        );

        runtime
            .set_resource_contract_policy(42, NativeResourceContractPolicy::Io)
            .unwrap();
        let set_contract_policy = runtime.backend().last_frame();
        assert_eq!(set_contract_policy.number, SYS_SET_RESOURCE_CONTRACT_POLICY);
        assert_eq!(set_contract_policy.arg0, 42);
        assert_eq!(
            set_contract_policy.arg1,
            NativeResourceContractPolicy::Io as usize
        );

        runtime
            .set_resource_issuer_policy(42, NativeResourceIssuerPolicy::CreatorOnly)
            .unwrap();
        let set_issuer_policy = runtime.backend().last_frame();
        assert_eq!(set_issuer_policy.number, SYS_SET_RESOURCE_ISSUER_POLICY);
        assert_eq!(set_issuer_policy.arg0, 42);
        assert_eq!(
            set_issuer_policy.arg1,
            NativeResourceIssuerPolicy::CreatorOnly as usize
        );

        runtime
            .set_resource_state(42, NativeResourceState::Suspended)
            .unwrap();
        let set_state = runtime.backend().last_frame();
        assert_eq!(set_state.number, SYS_SET_RESOURCE_STATE);
        assert_eq!(set_state.arg0, 42);
        assert_eq!(set_state.arg1, NativeResourceState::Suspended as usize);

        runtime
            .report_boot_session(
                BootSessionStatus::Success,
                BootSessionStage::Complete,
                0,
                77,
            )
            .unwrap();
        let report = runtime.backend().last_frame();
        assert_eq!(report.number, SYS_BOOT_REPORT);
        assert_eq!(report.arg0, BootSessionStatus::Success as usize);
        assert_eq!(report.arg1, BootSessionStage::Complete as usize);
        assert_eq!(report.arg2 as i32, 0);
        assert_eq!(report.arg3, 77);

        runtime
            .configure_network_interface_ipv4(
                "/dev/net0",
                [10, 1, 0, 2],
                [255, 255, 255, 0],
                [10, 1, 0, 1],
            )
            .unwrap();
        let netif_config = runtime.backend().last_frame();
        assert_eq!(netif_config.number, SYS_CONFIGURE_NETIF_IPV4);
        assert_eq!(netif_config.arg1, "/dev/net0".len());
        assert_ne!(netif_config.arg2, 0);

        runtime
            .bind_udp_socket("/run/net0.sock", "/dev/net0", 4000, [10, 1, 0, 9], 5000)
            .unwrap();
        let bind_udp = runtime.backend().last_frame();
        assert_eq!(bind_udp.number, SYS_BIND_UDP_SOCKET);
        assert_eq!(bind_udp.arg1, "/run/net0.sock".len());
        assert_eq!(bind_udp.arg3, "/dev/net0".len());
        assert_ne!(bind_udp.arg4, 0);

        runtime.inspect_network_interface("/dev/net0").unwrap();
        let inspect_netif = runtime.backend().last_frame();
        assert_eq!(inspect_netif.number, SYS_INSPECT_NETIF);
        assert_eq!(inspect_netif.arg1, "/dev/net0".len());
        assert_ne!(inspect_netif.arg2, 0);

        runtime.inspect_network_socket("/run/net0.sock").unwrap();
        let inspect_netsock = runtime.backend().last_frame();
        assert_eq!(inspect_netsock.number, SYS_INSPECT_NETSOCK);
        assert_eq!(inspect_netsock.arg1, "/run/net0.sock".len());
        assert_ne!(inspect_netsock.arg2, 0);

        runtime.inspect_device("/dev/storage0").unwrap();
        let inspect_device = runtime.backend().last_frame();
        assert_eq!(inspect_device.number, SYS_INSPECT_DEVICE);
        assert_eq!(inspect_device.arg1, "/dev/storage0".len());
        assert_ne!(inspect_device.arg2, 0);

        runtime.inspect_driver("/drv/storage0").unwrap();
        let inspect_driver = runtime.backend().last_frame();
        assert_eq!(inspect_driver.number, SYS_INSPECT_DRIVER);
        assert_eq!(inspect_driver.arg1, "/drv/storage0".len());
        assert_ne!(inspect_driver.arg2, 0);

        runtime
            .set_network_interface_link_state("/dev/net0", false)
            .unwrap();
        let set_link = runtime.backend().last_frame();
        assert_eq!(set_link.number, SYS_SET_NETIF_LINK_STATE);
        assert_eq!(set_link.arg1, "/dev/net0".len());
        assert_ne!(set_link.arg2, 0);

        runtime
            .create_event_queue(NativeEventQueueMode::Kqueue)
            .unwrap();
        let create_queue = runtime.backend().last_frame();
        assert_eq!(create_queue.number, SYS_CREATE_EVENT_QUEUE);
        assert_eq!(create_queue.arg0, NativeEventQueueMode::Kqueue as usize);

        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 4];
        runtime.wait_event_queue(77, &mut events).unwrap();
        let wait_queue = runtime.backend().last_frame();
        assert_eq!(wait_queue.number, SYS_WAIT_EVENT_QUEUE);
        assert_eq!(wait_queue.arg0, 77);
        assert_eq!(wait_queue.arg2, events.len());

        runtime
            .watch_network_events(
                77,
                "/dev/net0",
                Some("/run/net0.sock"),
                999,
                true,
                true,
                true,
                ngos_user_abi::POLLPRI,
            )
            .unwrap();
        let watch_net = runtime.backend().last_frame();
        assert_eq!(watch_net.number, SYS_WATCH_NET_EVENTS);
        assert_eq!(watch_net.arg0, 77);
        assert_eq!(watch_net.arg2, "/dev/net0".len());
        assert_eq!(watch_net.arg4, "/run/net0.sock".len());
        assert_ne!(watch_net.arg5, 0);

        runtime
            .remove_network_events(77, "/dev/net0", Some("/run/net0.sock"), 999)
            .unwrap();
        let remove_net = runtime.backend().last_frame();
        assert_eq!(remove_net.number, SYS_REMOVE_NET_EVENTS);
        assert_eq!(remove_net.arg0, 77);
        assert_eq!(remove_net.arg5, 999);
    }

    #[test]
    fn invoke_maps_errno_from_syscall_return() {
        let runtime = Runtime::new(RecordingBackend::new(SyscallReturn::err(Errno::Badf)));
        let err = runtime.write(1, b"x").unwrap_err();
        assert_eq!(err, Errno::Badf);
    }

    #[test]
    fn exit_wrapper_uses_sys_exit_number() {
        let backend = RecordingBackend::new(SyscallReturn::ok(0));
        let frame = SyscallFrame::new(SYS_EXIT, [42usize, 0, 0, 0, 0, 0]);
        let returned = unsafe { backend.syscall(frame) };
        assert_eq!(returned, SyscallReturn::ok(0));
        assert_eq!(backend.last_frame().number, SYS_EXIT);
        assert_eq!(backend.last_frame().arg0, 42usize);
    }

    #[test]
    fn boot_report_wrapper_emits_expected_syscall_frame() {
        let runtime = Runtime::new(RecordingBackend::new(SyscallReturn::ok(0)));
        runtime
            .report_boot_session(
                BootSessionStatus::Failure,
                BootSessionStage::Complete,
                17,
                99,
            )
            .unwrap();
        let frame = runtime.backend().last_frame();
        assert_eq!(frame.number, SYS_BOOT_REPORT);
        assert_eq!(frame.arg0, BootSessionStatus::Failure as usize);
        assert_eq!(frame.arg1, BootSessionStage::Complete as usize);
        assert_eq!(frame.arg2 as i32, 17);
        assert_eq!(frame.arg3, 99);
    }

    #[test]
    fn runtime_security_helpers_match_abi_contract() {
        let low = SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified);
        let high = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::System);
        let readable_object =
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel);
        let tag = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [7; 32],
        };
        let subject = SubjectSecurityContext {
            subject_id: 1,
            active_issuer_id: 2,
            rights_ceiling: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            label: high,
            session_nonce: 3,
            current_epoch: 10,
            minimum_revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        let object = ObjectSecurityContext {
            object_id: 4,
            required_rights: BlockRightsMask::READ,
            minimum_label: low,
            current_label: low,
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 4,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 8,
                measurement: tag,
            },
            integrity: tag,
            revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        let token = CapabilityToken {
            object_id: 4,
            rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            issuer_id: 2,
            subject_id: 1,
            generation: 5,
            revocation_epoch: 1,
            delegation_depth: 0,
            delegated: 0,
            nonce: 6,
            expiry_epoch: 11,
            authenticator: tag,
        };

        assert!(validate_rights(token.rights, BlockRightsMask::READ).is_ok());
        assert!(check_ifc_read(high, readable_object).is_ok());
        assert_eq!(
            join_labels(low, high).confidentiality,
            ConfidentialityLevel::Secret
        );
        assert!(check_capability(&subject, &object, &token, BlockRightsMask::READ, &tag).is_ok());

        let req = derive_request_provenance(&subject, &object, &token, tag, 99);
        let completion = derive_completion_provenance(&req, 55, tag, 77);
        assert_eq!(completion.parent_measurement, req.measurement.bytes);

        let mismatch = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [9; 32],
        };
        assert_eq!(
            verify_integrity_tag(&tag, &mismatch).unwrap_err().code,
            SecurityErrorCode::IntegrityMismatch
        );
    }

    #[test]
    fn runtime_block_security_validators_match_abi_contract() {
        let label = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        let request_label =
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel);
        let tag = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [5; 32],
        };
        let subject = SubjectSecurityContext {
            subject_id: 10,
            active_issuer_id: 11,
            rights_ceiling: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            label,
            session_nonce: 1,
            current_epoch: 100,
            minimum_revocation_epoch: 1,
            max_delegation_depth: 2,
        };
        let capability = CapabilityToken::new(
            12,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            11,
            10,
            2,
            1,
            0,
            false,
            3,
            101,
            tag,
        );
        let object = ObjectSecurityContext {
            object_id: 12,
            required_rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            minimum_label: request_label,
            current_label: request_label,
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 12,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 4,
                measurement: tag,
            },
            integrity: tag,
            revocation_epoch: 1,
            max_delegation_depth: 2,
        };
        let request = NativeBlockIoRequest::new(
            ngos_user_abi::NATIVE_BLOCK_IO_OP_READ,
            4,
            2,
            512,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            capability,
            request_label,
            derive_request_provenance(&subject, &object, &capability, tag, 9),
            tag,
        );
        assert!(validate_block_request_security(&request, &subject, &object).is_ok());
        assert_eq!(
            required_block_rights_for_op(ngos_user_abi::NATIVE_BLOCK_IO_OP_READ),
            Some(BlockRightsMask::READ.union(BlockRightsMask::SUBMIT))
        );

        let completion = NativeBlockIoCompletion::new(
            ngos_user_abi::NATIVE_BLOCK_IO_OP_READ,
            0,
            1024,
            512,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            request_label,
            derive_completion_provenance(&request.provenance, 12, tag, 10),
            tag,
        );
        assert!(validate_block_completion_security(&completion, &request).is_ok());
    }

    #[test]
    fn runtime_compose_helpers_build_security_checked_block_messages() {
        let subject_label =
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        let request_label =
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel);
        let tag = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [8; 32],
        };
        let subject = SubjectSecurityContext {
            subject_id: 30,
            active_issuer_id: 31,
            rights_ceiling: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            label: subject_label,
            session_nonce: 1,
            current_epoch: 50,
            minimum_revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        let object = ObjectSecurityContext {
            object_id: 32,
            required_rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            minimum_label: request_label,
            current_label: request_label,
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 32,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 1,
                measurement: tag,
            },
            integrity: tag,
            revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        let capability = CapabilityToken {
            object_id: 32,
            rights: BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            issuer_id: 31,
            subject_id: 30,
            generation: 2,
            revocation_epoch: 1,
            delegation_depth: 0,
            delegated: 0,
            nonce: 3,
            expiry_epoch: 51,
            authenticator: tag,
        };

        let request = compose_block_request(
            &subject,
            &object,
            capability,
            ngos_user_abi::NATIVE_BLOCK_IO_OP_READ,
            10,
            2,
            512,
            request_label,
            tag,
            70,
        )
        .unwrap();
        assert_eq!(
            request.rights,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT)
        );
        assert_eq!(
            block_request_required_rights(&request).unwrap(),
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT)
        );

        let completion =
            compose_block_completion(&request, 32, 0, 1024, request_label, tag, 71).unwrap();
        assert!(validate_block_completion_security(&completion, &request).is_ok());
        assert!(completion.is_success());
    }

    #[test]
    fn runtime_context_validators_match_abi_contract() {
        let subject = SubjectSecurityContext {
            subject_id: 40,
            active_issuer_id: 41,
            rights_ceiling: BlockRightsMask::READ,
            label: SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified),
            session_nonce: 1,
            current_epoch: 2,
            minimum_revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        assert!(validate_subject_context(&subject).is_ok());

        let invalid_object = ObjectSecurityContext {
            object_id: 42,
            required_rights: BlockRightsMask::READ,
            minimum_label: SecurityLabel::new(
                ConfidentialityLevel::Sensitive,
                IntegrityLevel::Verified,
            ),
            current_label: SecurityLabel::new(
                ConfidentialityLevel::Internal,
                IntegrityLevel::Kernel,
            ),
            lineage: ProvenanceTag {
                origin_kind: ProvenanceOriginKind::Device,
                reserved0: 0,
                origin_id: 42,
                parent_origin_id: 0,
                parent_measurement: [0; 32],
                edge_id: 1,
                measurement: IntegrityTag {
                    kind: IntegrityTagKind::Blake3,
                    reserved: 0,
                    bytes: [1; 32],
                },
            },
            integrity: IntegrityTag {
                kind: IntegrityTagKind::Blake3,
                reserved: 0,
                bytes: [1; 32],
            },
            revocation_epoch: 1,
            max_delegation_depth: 1,
        };
        assert_eq!(
            validate_object_context(&invalid_object).unwrap_err().code,
            SecurityErrorCode::InvalidSecurityState
        );
    }

    #[test]
    fn runtime_label_transition_and_errno_mapping_match_abi_contract() {
        let from = SecurityLabel::new(ConfidentialityLevel::Sensitive, IntegrityLevel::Verified);
        let to = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert!(validate_label_transition(from, to).is_ok());
        assert_eq!(
            security_error_to_errno(SecurityErrorCode::RightsDenied),
            ngos_user_abi::Errno::Access
        );
    }

    #[test]
    fn runtime_provenance_and_integrity_validation_match_abi_contract() {
        let valid_tag = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [2; 32],
        };
        let provenance = ProvenanceTag {
            origin_kind: ProvenanceOriginKind::Completion,
            reserved0: 0,
            origin_id: 50,
            parent_origin_id: 49,
            parent_measurement: [1; 32],
            edge_id: 2,
            measurement: valid_tag,
        };
        assert!(validate_provenance_tag(&provenance).is_ok());
        assert!(validate_integrity_tag(&valid_tag).is_ok());
    }

    #[test]
    fn runtime_effective_label_helpers_match_abi_contract() {
        let subject = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        let object = SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel);
        let request = derive_effective_request_label(subject, object);
        assert_eq!(request.confidentiality, ConfidentialityLevel::Secret);
        assert_eq!(request.integrity, IntegrityLevel::Verified);

        let completion = derive_effective_completion_label(request, request).unwrap();
        assert_eq!(completion, request);
    }

    #[test]
    fn runtime_security_constructors_build_expected_values() {
        let tag = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [3; 32],
        };
        let root = ProvenanceTag::root(ProvenanceOriginKind::Device, 60, 7, tag);
        let child = ProvenanceTag::child(ProvenanceOriginKind::Request, 61, &root, 8, tag);
        assert_eq!(child.parent_origin_id, root.origin_id);
        assert_eq!(child.parent_measurement, root.measurement.bytes);

        let subject = SubjectSecurityContext::new(
            70,
            71,
            BlockRightsMask::READ,
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified),
            9,
            10,
            1,
            1,
        );
        let capability = CapabilityToken::new(
            72,
            BlockRightsMask::READ,
            71,
            70,
            1,
            1,
            0,
            false,
            2,
            11,
            tag,
        );
        let object = ObjectSecurityContext::new(
            72,
            BlockRightsMask::READ,
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified),
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified),
            root,
            tag,
            1,
            1,
        );
        assert_eq!(subject.subject_id, 70);
        assert_eq!(capability.object_id, 72);
        assert_eq!(object.object_id, 72);
    }

    #[test]
    fn runtime_capability_token_validation_matches_abi_contract() {
        let tag = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [4; 32],
        };
        let token = CapabilityToken::new(
            80,
            BlockRightsMask::READ,
            81,
            82,
            1,
            1,
            0,
            false,
            2,
            20,
            tag,
        );
        assert!(validate_capability_token(&token, 19).is_ok());
        assert_eq!(
            validate_capability_token(&token, 21).unwrap_err().code,
            SecurityErrorCode::CapabilityExpired
        );
    }

    #[test]
    fn runtime_revocation_and_delegation_match_abi_contract() {
        let tag = IntegrityTag {
            kind: IntegrityTagKind::Blake3,
            reserved: 0,
            bytes: [6; 32],
        };
        let subject = SubjectSecurityContext::new(
            90,
            91,
            BlockRightsMask::READ
                .union(BlockRightsMask::SUBMIT)
                .union(BlockRightsMask::DELEGATE),
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified),
            1,
            30,
            2,
            1,
        );
        let object = ObjectSecurityContext::new(
            92,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified),
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified),
            ProvenanceTag::root(ProvenanceOriginKind::Device, 92, 1, tag),
            tag,
            2,
            1,
        );
        let parent = CapabilityToken::new(
            92,
            BlockRightsMask::READ
                .union(BlockRightsMask::SUBMIT)
                .union(BlockRightsMask::DELEGATE),
            91,
            90,
            1,
            2,
            0,
            false,
            3,
            31,
            tag,
        );
        assert!(validate_revocation(&subject, &object, &parent).is_ok());
        let delegated = delegate_capability(
            &parent,
            93,
            BlockRightsMask::READ.union(BlockRightsMask::SUBMIT),
            4,
            31,
            tag,
        )
        .unwrap();
        assert!(
            validate_delegation(
                &subject,
                &object,
                &delegated,
                BlockRightsMask::READ.union(BlockRightsMask::SUBMIT)
            )
            .is_ok()
        );
    }
}
