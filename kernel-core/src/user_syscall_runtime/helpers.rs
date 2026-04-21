use super::*;
use ngos_user_abi::{NativeObjectKind, UserIoVec};

pub(super) fn frame_fd(raw: usize) -> Result<Descriptor, RuntimeError> {
    let fd = u32::try_from(raw)
        .map_err(|_| RuntimeError::Descriptor(DescriptorError::InvalidDescriptor))?;
    Ok(Descriptor::new(fd))
}

pub(super) fn frame_pid(raw: usize) -> Result<ProcessId, RuntimeError> {
    let pid = u64::try_from(raw).map_err(|_| RuntimeError::Process(ProcessError::InvalidPid))?;
    Ok(ProcessId::from_handle(ObjectHandle::new(
        Handle::new(pid),
        0,
    )))
}

pub(super) fn decode_fcntl(encoded: usize) -> Option<FcntlCmd> {
    let flag = ((encoded >> 8) & 0x1) != 0;
    let cmd = match encoded & 0xff {
        0 => FcntlCmd::GetFl,
        1 => FcntlCmd::GetFd,
        2 => FcntlCmd::SetFl { nonblock: flag },
        3 => FcntlCmd::SetFd { cloexec: flag },
        _ => return None,
    };
    Some(cmd)
}

pub(super) fn decode_native_resource_kind(raw: usize) -> Option<ResourceKind> {
    match NativeResourceKind::from_raw(raw as u32)? {
        NativeResourceKind::Memory => Some(ResourceKind::Memory),
        NativeResourceKind::Storage => Some(ResourceKind::Storage),
        NativeResourceKind::Channel => Some(ResourceKind::Channel),
        NativeResourceKind::Device => Some(ResourceKind::Device),
        NativeResourceKind::Namespace => Some(ResourceKind::Namespace),
        NativeResourceKind::Surface => Some(ResourceKind::Surface),
    }
}

pub(super) fn decode_native_contract_kind(raw: usize) -> Option<ContractKind> {
    match NativeContractKind::from_raw(raw as u32)? {
        NativeContractKind::Execution => Some(ContractKind::Execution),
        NativeContractKind::Memory => Some(ContractKind::Memory),
        NativeContractKind::Io => Some(ContractKind::Io),
        NativeContractKind::Device => Some(ContractKind::Device),
        NativeContractKind::Display => Some(ContractKind::Display),
        NativeContractKind::Observe => Some(ContractKind::Observe),
    }
}

pub(super) fn encode_native_resource_kind(kind: ResourceKind) -> NativeResourceKind {
    match kind {
        ResourceKind::Memory => NativeResourceKind::Memory,
        ResourceKind::Storage => NativeResourceKind::Storage,
        ResourceKind::Channel => NativeResourceKind::Channel,
        ResourceKind::Device => NativeResourceKind::Device,
        ResourceKind::Namespace => NativeResourceKind::Namespace,
        ResourceKind::Surface => NativeResourceKind::Surface,
    }
}

pub(super) fn encode_native_contract_kind(kind: ContractKind) -> NativeContractKind {
    match kind {
        ContractKind::Execution => NativeContractKind::Execution,
        ContractKind::Memory => NativeContractKind::Memory,
        ContractKind::Io => NativeContractKind::Io,
        ContractKind::Device => NativeContractKind::Device,
        ContractKind::Display => NativeContractKind::Display,
        ContractKind::Observe => NativeContractKind::Observe,
    }
}

pub(super) fn encode_native_object_kind(kind: ObjectKind) -> NativeObjectKind {
    match kind {
        ObjectKind::File => NativeObjectKind::File,
        ObjectKind::Directory => NativeObjectKind::Directory,
        ObjectKind::Symlink => NativeObjectKind::Symlink,
        ObjectKind::Socket => NativeObjectKind::Socket,
        ObjectKind::Device => NativeObjectKind::Device,
        ObjectKind::Driver => NativeObjectKind::Driver,
        ObjectKind::Process => NativeObjectKind::Process,
        ObjectKind::Memory => NativeObjectKind::Memory,
        ObjectKind::Channel => NativeObjectKind::Channel,
        ObjectKind::EventQueue => NativeObjectKind::EventQueue,
        ObjectKind::SleepQueue => NativeObjectKind::SleepQueue,
    }
}

pub(super) fn encode_native_contract_state(state: ContractState) -> NativeContractState {
    match state {
        ContractState::Active => NativeContractState::Active,
        ContractState::Suspended => NativeContractState::Suspended,
        ContractState::Revoked => NativeContractState::Revoked,
    }
}

pub(super) fn decode_native_contract_state(raw: usize) -> Option<ContractState> {
    match raw as u32 {
        0 => Some(ContractState::Active),
        1 => Some(ContractState::Suspended),
        2 => Some(ContractState::Revoked),
        _ => None,
    }
}

pub(super) fn decode_native_resource_arbitration_policy(
    raw: usize,
) -> Option<ResourceArbitrationPolicy> {
    match NativeResourceArbitrationPolicy::from_raw(raw as u32)? {
        NativeResourceArbitrationPolicy::Fifo => Some(ResourceArbitrationPolicy::Fifo),
        NativeResourceArbitrationPolicy::Lifo => Some(ResourceArbitrationPolicy::Lifo),
    }
}

pub(super) fn encode_native_resource_arbitration_policy(
    policy: ResourceArbitrationPolicy,
) -> NativeResourceArbitrationPolicy {
    match policy {
        ResourceArbitrationPolicy::Fifo => NativeResourceArbitrationPolicy::Fifo,
        ResourceArbitrationPolicy::Lifo => NativeResourceArbitrationPolicy::Lifo,
    }
}

pub(super) fn decode_native_resource_governance_mode(raw: usize) -> Option<ResourceGovernanceMode> {
    match NativeResourceGovernanceMode::from_raw(raw as u32)? {
        NativeResourceGovernanceMode::Queueing => Some(ResourceGovernanceMode::Queueing),
        NativeResourceGovernanceMode::ExclusiveLease => {
            Some(ResourceGovernanceMode::ExclusiveLease)
        }
    }
}

pub(super) fn encode_native_resource_governance_mode(
    mode: ResourceGovernanceMode,
) -> NativeResourceGovernanceMode {
    match mode {
        ResourceGovernanceMode::Queueing => NativeResourceGovernanceMode::Queueing,
        ResourceGovernanceMode::ExclusiveLease => NativeResourceGovernanceMode::ExclusiveLease,
    }
}

pub(super) fn decode_native_resource_state(raw: usize) -> Option<ResourceState> {
    match NativeResourceState::from_raw(raw as u32)? {
        NativeResourceState::Active => Some(ResourceState::Active),
        NativeResourceState::Suspended => Some(ResourceState::Suspended),
        NativeResourceState::Retired => Some(ResourceState::Retired),
    }
}

pub(super) fn encode_native_resource_state(state: ResourceState) -> NativeResourceState {
    match state {
        ResourceState::Active => NativeResourceState::Active,
        ResourceState::Suspended => NativeResourceState::Suspended,
        ResourceState::Retired => NativeResourceState::Retired,
    }
}

pub(super) fn decode_native_resource_contract_policy(raw: usize) -> Option<ResourceContractPolicy> {
    match NativeResourceContractPolicy::from_raw(raw as u32)? {
        NativeResourceContractPolicy::Any => Some(ResourceContractPolicy::Any),
        NativeResourceContractPolicy::Execution => Some(ResourceContractPolicy::Execution),
        NativeResourceContractPolicy::Memory => Some(ResourceContractPolicy::Memory),
        NativeResourceContractPolicy::Io => Some(ResourceContractPolicy::Io),
        NativeResourceContractPolicy::Device => Some(ResourceContractPolicy::Device),
        NativeResourceContractPolicy::Display => Some(ResourceContractPolicy::Display),
        NativeResourceContractPolicy::Observe => Some(ResourceContractPolicy::Observe),
    }
}

pub(super) fn encode_native_resource_contract_policy(
    policy: ResourceContractPolicy,
) -> NativeResourceContractPolicy {
    match policy {
        ResourceContractPolicy::Any => NativeResourceContractPolicy::Any,
        ResourceContractPolicy::Execution => NativeResourceContractPolicy::Execution,
        ResourceContractPolicy::Memory => NativeResourceContractPolicy::Memory,
        ResourceContractPolicy::Io => NativeResourceContractPolicy::Io,
        ResourceContractPolicy::Device => NativeResourceContractPolicy::Device,
        ResourceContractPolicy::Display => NativeResourceContractPolicy::Display,
        ResourceContractPolicy::Observe => NativeResourceContractPolicy::Observe,
    }
}

pub(super) fn decode_native_resource_issuer_policy(raw: usize) -> Option<ResourceIssuerPolicy> {
    match NativeResourceIssuerPolicy::from_raw(raw as u32)? {
        NativeResourceIssuerPolicy::AnyIssuer => Some(ResourceIssuerPolicy::AnyIssuer),
        NativeResourceIssuerPolicy::CreatorOnly => Some(ResourceIssuerPolicy::CreatorOnly),
        NativeResourceIssuerPolicy::DomainOwnerOnly => Some(ResourceIssuerPolicy::DomainOwnerOnly),
    }
}

pub(super) fn encode_native_resource_issuer_policy(
    policy: ResourceIssuerPolicy,
) -> NativeResourceIssuerPolicy {
    match policy {
        ResourceIssuerPolicy::AnyIssuer => NativeResourceIssuerPolicy::AnyIssuer,
        ResourceIssuerPolicy::CreatorOnly => NativeResourceIssuerPolicy::CreatorOnly,
        ResourceIssuerPolicy::DomainOwnerOnly => NativeResourceIssuerPolicy::DomainOwnerOnly,
    }
}

pub(super) const fn encode_native_process_state(state: ProcessState) -> u32 {
    match state {
        ProcessState::Created => 0,
        ProcessState::Ready => 1,
        ProcessState::Running => 2,
        ProcessState::Blocked => 3,
        ProcessState::Exited => 4,
    }
}

pub(super) fn frame_string(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
    len: usize,
) -> Result<String, SyscallReturn> {
    let bytes = match runtime.copy_from_user(caller, ptr, len) {
        Ok(bytes) => bytes,
        Err(error) => return Err(SyscallReturn::err(error.errno())),
    };
    String::from_utf8(bytes).map_err(|_| SyscallReturn::err(Errno::Inval))
}

pub(super) fn frame_string_table(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
    len: usize,
    count: usize,
) -> Result<Vec<String>, SyscallReturn> {
    if count == 0 {
        if len == 0 {
            return Ok(Vec::new());
        }
        return Err(SyscallReturn::err(Errno::Inval));
    }
    let bytes = match runtime.copy_from_user(caller, ptr, len) {
        Ok(bytes) => bytes,
        Err(error) => return Err(SyscallReturn::err(error.errno())),
    };
    if bytes.last().copied() != Some(0) {
        return Err(SyscallReturn::err(Errno::Inval));
    }
    let values = bytes
        .split(|byte| *byte == 0)
        .take(count)
        .map(|segment| {
            String::from_utf8(segment.to_vec()).map_err(|_| SyscallReturn::err(Errno::Inval))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != count {
        return Err(SyscallReturn::err(Errno::Inval));
    }
    let expected = values
        .iter()
        .fold(0usize, |acc, value| acc + value.len() + 1);
    if expected != len {
        return Err(SyscallReturn::err(Errno::Inval));
    }
    Ok(values)
}

pub(super) fn copy_struct_to_user<T: Copy>(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
    value: &T,
) -> Result<(), SyscallReturn> {
    let bytes = unsafe {
        core::slice::from_raw_parts((value as *const T).cast::<u8>(), core::mem::size_of::<T>())
    };
    runtime
        .copy_to_user(caller, ptr, bytes)
        .map_err(|error| SyscallReturn::err(error.errno()))
}

pub(super) fn copy_struct_from_user<T: Copy>(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
) -> Result<T, SyscallReturn> {
    let bytes = match runtime.copy_from_user(caller, ptr, core::mem::size_of::<T>()) {
        Ok(bytes) => bytes,
        Err(error) => return Err(SyscallReturn::err(error.errno())),
    };
    let value = unsafe { (bytes.as_ptr() as *const T).read_unaligned() };
    Ok(value)
}

pub(super) fn copy_u64_slice_to_user(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
    capacity: usize,
    values: &[u64],
) -> Result<(), SyscallReturn> {
    let count = capacity.min(values.len());
    let bytes = unsafe {
        core::slice::from_raw_parts(
            values.as_ptr().cast::<u8>(),
            count * core::mem::size_of::<u64>(),
        )
    };
    runtime
        .copy_to_user(caller, ptr, bytes)
        .map_err(|error| SyscallReturn::err(error.errno()))
}

pub(super) fn copy_signal_slice_to_user(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
    capacity: usize,
    values: &[u8],
) -> Result<(), SyscallReturn> {
    let count = capacity.min(values.len());
    runtime
        .copy_to_user(caller, ptr, &values[..count])
        .map_err(|error| SyscallReturn::err(error.errno()))
}

pub(super) fn copy_string_to_user(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
    capacity: usize,
    text: &str,
) -> Result<usize, RuntimeError> {
    let bytes = text.as_bytes();
    let count = capacity.min(bytes.len());
    runtime
        .copy_to_user(caller, ptr, &bytes[..count])
        .map_err(|_| RuntimeError::Process(ProcessError::InvalidMemoryLayout))?;
    Ok(count)
}

pub(super) fn frame_iovecs(
    runtime: &mut KernelRuntime,
    caller: ProcessId,
    ptr: usize,
    count: usize,
) -> Result<Vec<UserIoVec>, SyscallReturn> {
    let byte_len = count
        .checked_mul(core::mem::size_of::<UserIoVec>())
        .ok_or(SyscallReturn::err(Errno::Inval))?;
    let bytes = match runtime.copy_from_user(caller, ptr, byte_len) {
        Ok(bytes) => bytes,
        Err(error) => return Err(SyscallReturn::err(error.errno())),
    };
    let mut iovecs = Vec::with_capacity(count);
    for chunk in bytes.chunks_exact(core::mem::size_of::<UserIoVec>()) {
        let mut base_bytes = [0u8; core::mem::size_of::<usize>()];
        let mut len_bytes = [0u8; core::mem::size_of::<usize>()];
        base_bytes.copy_from_slice(&chunk[..core::mem::size_of::<usize>()]);
        len_bytes.copy_from_slice(&chunk[core::mem::size_of::<usize>()..]);
        iovecs.push(UserIoVec {
            base: usize::from_ne_bytes(base_bytes),
            len: usize::from_ne_bytes(len_bytes),
        });
    }
    Ok(iovecs)
}

pub(super) fn map_runtime_error_to_errno(error: RuntimeError) -> Errno {
    match error {
        RuntimeError::Descriptor(
            DescriptorError::InvalidOwner
            | DescriptorError::InvalidDescriptor
            | DescriptorError::DescriptorExhausted,
        ) => Errno::Badf,
        RuntimeError::Descriptor(DescriptorError::RightDenied { .. }) => Errno::Access,
        RuntimeError::NativeModel(NativeModelError::ResourceBusy { .. }) => Errno::Busy,
        RuntimeError::NativeModel(NativeModelError::ResourceClaimNotQueued { .. }) => Errno::Inval,
        RuntimeError::NativeModel(NativeModelError::ResourceContractKindMismatch { .. }) => {
            Errno::Access
        }
        RuntimeError::NativeModel(NativeModelError::ResourceIssuerPolicyMismatch { .. }) => {
            Errno::Access
        }
        RuntimeError::NativeModel(NativeModelError::ResourceNotActive { .. }) => Errno::Access,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing { .. }) => Errno::Access,
        RuntimeError::NativeModel(NativeModelError::BusAccessDenied { .. }) => Errno::Access,
        RuntimeError::NativeModel(NativeModelError::BusQueueFull { .. }) => Errno::Again,
        RuntimeError::Process(
            ProcessError::InvalidPid
            | ProcessError::StalePid
            | ProcessError::InvalidTid
            | ProcessError::StaleTid,
        ) => Errno::Srch,
        RuntimeError::Process(ProcessError::InvalidMemoryLayout) => Errno::Inval,
        RuntimeError::Process(ProcessError::MemoryQuarantined { .. }) => Errno::Busy,
        RuntimeError::Process(ProcessError::Exhausted) => Errno::Again,
        RuntimeError::Process(
            ProcessError::InvalidSignal
            | ProcessError::CpuExtendedStateUnavailable
            | ProcessError::InvalidSessionReport
            | ProcessError::InvalidTransition { .. },
        ) => Errno::Inval,
        RuntimeError::Process(ProcessError::NotExited) => Errno::Child,
        RuntimeError::Capability(_) => Errno::Perm,
        RuntimeError::DeviceModel(DeviceModelError::QueueFull | DeviceModelError::QueueEmpty) => {
            Errno::Again
        }
        RuntimeError::DeviceModel(DeviceModelError::PacketTooLarge) => Errno::TooBig,
        RuntimeError::DeviceModel(DeviceModelError::AlreadyRegistered) => Errno::Exist,
        RuntimeError::DeviceModel(_) => Errno::Inval,
        RuntimeError::NativeModel(NativeModelError::ContractNotActive { .. }) => Errno::Access,
        RuntimeError::NativeModel(_) => Errno::Inval,
        RuntimeError::Scheduler(SchedulerError::InvalidCpuAffinity) => Errno::Inval,
        RuntimeError::Scheduler(_) => Errno::Again,
        RuntimeError::Vfs(VfsError::AlreadyExists) => Errno::Exist,
        RuntimeError::Vfs(VfsError::NotDirectory) => Errno::NotDir,
        RuntimeError::Vfs(VfsError::DirectoryNotEmpty | VfsError::CrossMountRename) => Errno::Busy,
        RuntimeError::Vfs(_) => Errno::NoEnt,
        RuntimeError::EventQueue(_) | RuntimeError::SleepQueue(_) | RuntimeError::TaskQueue(_) => {
            Errno::Again
        }
        RuntimeError::Buffer(_) => Errno::Io,
        RuntimeError::Hal(_) => Errno::Io,
    }
}
