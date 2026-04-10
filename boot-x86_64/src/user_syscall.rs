use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::fmt::Write;
#[cfg(target_os = "none")]
use core::mem::MaybeUninit;
use core::ptr;
use core::slice;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[cfg(target_os = "none")]
use ngos_user_abi::Amd64UserEntryRegisters;
use ngos_user_abi::{
    BlockRightsMask, BootSessionReport, BootSessionStage, BootSessionStatus, ConfidentialityLevel,
    Errno, IntegrityLevel, IntegrityTag, IntegrityTagKind, NativeBusEndpointRecord,
    NativeBusEventWatchConfig, NativeBusPeerRecord, NativeContractKind, NativeContractRecord,
    NativeContractState, NativeDeviceRecord, NativeDeviceRequestRecord, NativeDomainRecord,
    NativeDriverRecord, NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind,
    NativeFileStatusRecord, NativeFileSystemStatusRecord, NativeGpuDisplayRecord,
    NativeGpuScanoutRecord, NativeMountPropagationMode, NativeMountRecord,
    NativeNetworkAdminConfig, NativeNetworkEventKind, NativeNetworkEventWatchConfig,
    NativeNetworkInterfaceConfig, NativeNetworkInterfaceRecord, NativeNetworkLinkStateConfig,
    NativeNetworkSocketRecord, NativeObjectKind, NativeProcessCompatRecord,
    NativeProcessIdentityRecord, NativeProcessRecord, NativeResourceArbitrationPolicy,
    NativeResourceCancelRecord, NativeResourceClaimRecord, NativeResourceContractPolicy,
    NativeResourceEventWatchConfig, NativeResourceGovernanceMode, NativeResourceIssuerPolicy,
    NativeResourceKind, NativeResourceRecord, NativeResourceReleaseRecord, NativeResourceState,
    NativeSchedulerClass, NativeSpawnProcessConfig, NativeStorageLineageRecord,
    NativeStorageVolumeRecord, NativeUdpBindConfig, NativeUdpConnectConfig, NativeUdpRecvMeta,
    NativeUdpSendToConfig, NativeVfsEventKind, NativeVfsEventWatchConfig, ObjectSecurityContext,
    POLLIN, POLLOUT, POLLPRI, ProvenanceOriginKind, ProvenanceTag, SYS_ACQUIRE_RESOURCE,
    SYS_ADVISE_MEMORY_RANGE, SYS_ATTACH_BUS_PEER, SYS_BIND_PROCESS_CONTRACT, SYS_BIND_UDP_SOCKET,
    SYS_BLOCKED_PENDING_SIGNALS, SYS_BOOT_REPORT, SYS_CANCEL_RESOURCE_CLAIM, SYS_CHMOD_PATH,
    SYS_CHMOD_PATH_AT, SYS_CHOWN_PATH, SYS_CHOWN_PATH_AT, SYS_CLAIM_RESOURCE, SYS_CLOSE,
    SYS_COMPLETE_NET_TX, SYS_CONFIGURE_NETIF_ADMIN, SYS_CONFIGURE_NETIF_IPV4,
    SYS_CONNECT_UDP_SOCKET, SYS_CREATE_BUS_ENDPOINT, SYS_CREATE_BUS_PEER, SYS_CREATE_CONTRACT,
    SYS_CREATE_DOMAIN, SYS_CREATE_EVENT_QUEUE, SYS_CREATE_RESOURCE, SYS_DETACH_BUS_PEER, SYS_DUP,
    SYS_EXIT, SYS_FCNTL, SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME, SYS_GET_PROCESS_CWD,
    SYS_GET_PROCESS_IDENTITY, SYS_GET_PROCESS_IMAGE_PATH, SYS_GET_PROCESS_NAME,
    SYS_GET_PROCESS_ROOT, SYS_GET_PROCESS_SECURITY_LABEL, SYS_GET_RESOURCE_NAME,
    SYS_INSPECT_BUS_ENDPOINT, SYS_INSPECT_BUS_PEER, SYS_INSPECT_CONTRACT, SYS_INSPECT_DEVICE,
    SYS_INSPECT_DEVICE_REQUEST, SYS_INSPECT_DOMAIN, SYS_INSPECT_DRIVER, SYS_INSPECT_GPU_DISPLAY,
    SYS_INSPECT_GPU_SCANOUT, SYS_INSPECT_MOUNT, SYS_INSPECT_NETIF, SYS_INSPECT_NETSOCK,
    SYS_INSPECT_PATH_SECURITY_CONTEXT, SYS_INSPECT_PROCESS, SYS_INSPECT_PROCESS_COMPAT,
    SYS_INSPECT_RESOURCE, SYS_INSPECT_STORAGE_LINEAGE, SYS_INSPECT_STORAGE_VOLUME,
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
    SYS_RELEASE_CLAIMED_RESOURCE, SYS_RELEASE_RESOURCE, SYS_RELEASE_VM_OBJECT,
    SYS_REMOVE_BUS_EVENTS, SYS_REMOVE_NET_EVENTS, SYS_REMOVE_RESOURCE_EVENTS,
    SYS_REMOVE_VFS_EVENTS, SYS_REMOVE_VFS_EVENTS_AT, SYS_RENAME_PATH, SYS_RENAME_PATH_AT,
    SYS_RENICE_PROCESS, SYS_REPAIR_STORAGE_SNAPSHOT, SYS_RESUME_PROCESS, SYS_SEEK, SYS_SEND_SIGNAL,
    SYS_SENDTO_UDP_SOCKET, SYS_SET_CONTRACT_STATE, SYS_SET_FD_RIGHTS, SYS_SET_MOUNT_PROPAGATION,
    SYS_SET_NETIF_LINK_STATE, SYS_SET_PATH_SECURITY_LABEL, SYS_SET_PROCESS_AFFINITY,
    SYS_SET_PROCESS_ARGS, SYS_SET_PROCESS_BREAK, SYS_SET_PROCESS_CWD, SYS_SET_PROCESS_ENV,
    SYS_SET_PROCESS_IDENTITY, SYS_SET_PROCESS_ROOT, SYS_SET_PROCESS_SECURITY_LABEL,
    SYS_SET_RESOURCE_CONTRACT_POLICY, SYS_SET_RESOURCE_GOVERNANCE, SYS_SET_RESOURCE_ISSUER_POLICY,
    SYS_SET_RESOURCE_POLICY, SYS_SET_RESOURCE_STATE, SYS_SPAWN_CONFIGURED_PROCESS,
    SYS_SPAWN_PATH_PROCESS, SYS_SPAWN_PROCESS_COPY_VM, SYS_STAT_PATH, SYS_STAT_PATH_AT,
    SYS_STATFS_PATH, SYS_STORE_MEMORY_WORD, SYS_SYMLINK_PATH, SYS_SYMLINK_PATH_AT,
    SYS_SYNC_MEMORY_RANGE, SYS_TRANSFER_RESOURCE, SYS_TRUNCATE_PATH, SYS_TRUNCATE_PATH_AT,
    SYS_UNLINK_PATH, SYS_UNLINK_PATH_AT, SYS_UNMAP_MEMORY_RANGE, SYS_UNMOUNT_STORAGE_VOLUME,
    SYS_WAIT_EVENT_QUEUE, SYS_WATCH_BUS_EVENTS, SYS_WATCH_NET_EVENTS, SYS_WATCH_RESOURCE_EVENTS,
    SYS_WATCH_VFS_EVENTS, SYS_WATCH_VFS_EVENTS_AT, SYS_WRITE, SYS_WRITEV, SecurityLabel,
    SeekWhence, SyscallFrame, SyscallReturn, UserIoVec, check_ifc_read, check_ifc_write,
};

use crate::diagnostics::{self, DiagnosticsPath, GuardKind, WatchKind};
#[cfg(target_os = "none")]
use crate::paging::{ActivePageTables, PageInit};
#[cfg(target_os = "none")]
use crate::phys_alloc::BootFrameAllocator;
use crate::serial;
use crate::tty;
#[cfg(target_os = "none")]
use crate::user_process::prepare_spawned_same_image_launch;
use crate::user_runtime_status;
#[path = "user_syscall_network_events.rs"]
mod user_syscall_network_events;
#[path = "user_syscall_path_vfs.rs"]
mod user_syscall_path_vfs;
#[path = "user_syscall_process_vm.rs"]
mod user_syscall_process_vm;
#[path = "user_syscall_resource_graphics.rs"]
mod user_syscall_resource_graphics;
use user_syscall_network_events::dispatch_network_event_syscall;
use user_syscall_path_vfs::dispatch_path_vfs_syscall;
use user_syscall_process_vm::dispatch_process_vm_syscall;
use user_syscall_resource_graphics::dispatch_resource_graphics_syscall;

#[cfg(not(test))]
fn syscall_trace(_args: core::fmt::Arguments<'_>) {}

#[cfg(test)]
fn syscall_trace(_args: core::fmt::Arguments<'_>) {}

const MAX_DESCRIPTOR_COUNT: usize = 8;
const MAX_DOMAIN_COUNT: usize = 16;
const MAX_RESOURCE_COUNT: usize = 16;
const MAX_CONTRACT_COUNT: usize = 32;
const MAX_BUS_PEER_COUNT: usize = 16;
const MAX_BUS_ENDPOINT_COUNT: usize = 16;
const MAX_NAME_LEN: usize = 32;
const MAX_PROCESS_COUNT: usize = 16;
const MAX_EVENT_QUEUE_COUNT: usize = 8;
const MAX_EVENT_QUEUE_WATCH_COUNT: usize = 16;
const MAX_EVENT_QUEUE_PENDING: usize = 32;
const BUS_ENDPOINT_QUEUE_CAPACITY: usize = 64;
const BOOT_OWNER_ID: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
enum SyscallDisposition {
    Return = 0,
    Halt = 1,
    #[cfg(target_os = "none")]
    Switch = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DescriptorTarget {
    Stdin,
    Stdout,
    Stderr,
    EventQueue(usize),
    StorageDevice,
    StorageDriver,
    GpuDevice,
    GpuDriver,
    AudioDevice,
    AudioDriver,
    InputDevice,
    InputDriver,
    NetworkDevice,
    NetworkDriver,
    BootDirectory(u64),
    BootFile(u64),
    BootChannel(u64),
    Procfs(BootProcfsNode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootProcfsNodeKind {
    ProcRootDir,
    ProcessDir,
    SystemDir,
    FdDirListing,
    FdInfoDirListing,
    Status,
    Root,
    Cwd,
    Exe,
    Cmdline,
    Environ,
    Auxv,
    Mounts,
    Fd,
    Caps,
    FdInfo(u64),
    VfsLocks,
    VfsWatches,
    VfsStats,
    Queues,
    Maps,
    VmObjects,
    VmDecisions,
    VmEpisodes,
    SystemScheduler,
    SystemSchedulerEpisodes,
    SystemBus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootProcfsNode {
    pid: u64,
    kind: BootProcfsNodeKind,
}

struct ProcfsLineBuffer {
    bytes: [u8; 384],
    len: usize,
}

impl ProcfsLineBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; 384],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl Write for ProcfsLineBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        if self.len + bytes.len() > self.bytes.len() {
            return Err(core::fmt::Error);
        }
        self.bytes[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DescriptorStatusFlags {
    nonblock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DescriptorFlags {
    nonblock: bool,
    cloexec: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DescriptorDescription {
    target: DescriptorTarget,
    flags: DescriptorStatusFlags,
    offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DescriptorTable {
    slots: [Option<DescriptorState>; MAX_DESCRIPTOR_COUNT],
    descriptions: [Option<DescriptorDescription>; MAX_DESCRIPTOR_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DescriptorState {
    description_id: usize,
    cloexec: bool,
    rights: BlockRightsMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DescriptorSnapshot {
    description_id: usize,
    target: DescriptorTarget,
    nonblock: bool,
    cloexec: bool,
    offset: usize,
    rights: BlockRightsMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedAtTarget {
    Path,
    Handle(DescriptorTarget),
}

struct DescriptorTableCell(UnsafeCell<DescriptorTable>);
struct NativeRegistryCell(UnsafeCell<NativeRegistry>);
struct BootBusRegistryCell(UnsafeCell<BootBusRegistry>);
struct BootEventQueueRegistryCell(UnsafeCell<BootEventQueueRegistry>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodedFcntl {
    GetFl,
    GetFd,
    SetFl { nonblock: bool },
    SetFd { cloexec: bool },
    QueryLock,
    TryLockExclusive { token: u16 },
    UnlockExclusive { token: u16 },
    TryLockShared { token: u16 },
    UnlockShared { token: u16 },
    UpgradeLockExclusive { token: u16 },
    DowngradeLockShared { token: u16 },
}

#[repr(C)]
#[derive(Default)]
pub struct SyscallDispatchResult {
    pub raw_return: usize,
    pub disposition: u64,
    pub switch_rip: u64,
    pub switch_rsp: u64,
    pub switch_rflags: u64,
    pub switch_r15: u64,
    pub switch_r14: u64,
    pub switch_r13: u64,
    pub switch_r12: u64,
    pub switch_rbp: u64,
    pub switch_rbx: u64,
    pub switch_r10: u64,
    pub switch_r9: u64,
    pub switch_r8: u64,
    pub switch_rdi: u64,
    pub switch_rsi: u64,
    pub switch_rdx: u64,
    pub switch_rax: u64,
}

#[cfg(target_os = "none")]
#[repr(C)]
struct SyscallSavedContext {
    frame: SyscallFrame,
    result: SyscallDispatchResult,
    saved_r15: u64,
    saved_r14: u64,
    saved_r13: u64,
    saved_r12: u64,
    saved_rbp: u64,
    saved_rbx: u64,
    saved_r10: u64,
    saved_r9: u64,
    saved_r8: u64,
    saved_rdi: u64,
    saved_rsi: u64,
    saved_rdx: u64,
    saved_rax: u64,
}

#[cfg(target_os = "none")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SavedUserContext {
    rip: u64,
    rsp: u64,
    rflags: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rax: u64,
}

static mut PROCESS_EXIT_CODE: i32 = 0;
static mut PROCESS_EXITED: bool = false;
static ACTIVE_PROCESS_PID: AtomicU64 = AtomicU64::new(1);
#[cfg(target_os = "none")]
static BOOT_PROCESS_EXEC_RUNTIME: BootProcessExecRuntimeCell = BootProcessExecRuntimeCell::new();
#[cfg(target_os = "none")]
static BOOT_PROCESS_EXEC_ALLOCATOR_READY: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "none")]
static mut BOOT_PROCESS_EXEC_ALLOCATOR: MaybeUninit<BootFrameAllocator> = MaybeUninit::uninit();
static DESCRIPTORS: DescriptorTableCell = DescriptorTableCell::new();
static NATIVE_REGISTRY: NativeRegistryCell = NativeRegistryCell::new();
static BOOT_BUS: BootBusRegistryCell = BootBusRegistryCell::new();
static BOOT_VFS: BootVfsCell = BootVfsCell::new();
static BOOT_PROCESSES: BootProcessRegistryCell = BootProcessRegistryCell::new();
static BOOT_EVENT_QUEUES: BootEventQueueRegistryCell = BootEventQueueRegistryCell::new();
static STORAGE_MOUNT: StorageMountCell = StorageMountCell::new();
static VFS_LOCKS: VfsLockCell = VfsLockCell::new();

#[cfg(target_os = "none")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BlockingChildExecution {
    parent_pid: u64,
    child_pid: u64,
    parent_context: SavedUserContext,
}

#[cfg(target_os = "none")]
#[derive(Default)]
struct BootProcessExecRuntime {
    root_phys: u64,
    pending_reap_launch: u64,
    active: Option<BlockingChildExecution>,
}

#[cfg(target_os = "none")]
struct BootProcessExecRuntimeCell {
    locked: AtomicBool,
    state: UnsafeCell<BootProcessExecRuntime>,
}

#[cfg(target_os = "none")]
impl BootProcessExecRuntimeCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(BootProcessExecRuntime {
                root_phys: 0,
                pending_reap_launch: 0,
                active: None,
            }),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootProcessExecRuntime) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let result = f(unsafe { &mut *self.state.get() });
        self.locked.store(false, Ordering::Release);
        result
    }
}

#[cfg(target_os = "none")]
unsafe impl Sync for BootProcessExecRuntimeCell {}

const FN_CLAIM_RESOURCE: u64 = 1;
const FN_RELEASE_CLAIMED_RESOURCE: u64 = 2;
const FN_SET_RESOURCE_GOVERNANCE: u64 = 3;
const FN_SET_RESOURCE_CONTRACT_POLICY: u64 = 4;
const FN_SET_RESOURCE_STATE: u64 = 5;
const FN_CREATE_CONTRACT: u64 = 6;
const FN_SET_RESOURCE_ISSUER_POLICY: u64 = 7;
const GPU_DEVICE_PATH: &str = "/dev/gpu0";
const GPU_DRIVER_PATH: &str = "/drv/gpu0";
const AUDIO_DEVICE_PATH: &str = "/dev/audio0";
const AUDIO_DRIVER_PATH: &str = "/drv/audio0";
const INPUT_DEVICE_PATH: &str = "/dev/input0";
const INPUT_DRIVER_PATH: &str = "/drv/input0";
const NETWORK_DEVICE_PATH: &str = "/dev/net0";
const NETWORK_DRIVER_PATH: &str = "/drv/net0";

unsafe impl Sync for DescriptorTableCell {}
unsafe impl Sync for NativeRegistryCell {}
unsafe impl Sync for BootBusRegistryCell {}
unsafe impl Sync for BootVfsCell {}
unsafe impl Sync for BootProcessRegistryCell {}
unsafe impl Sync for BootEventQueueRegistryCell {}
unsafe impl Sync for StorageMountCell {}
unsafe impl Sync for VfsLockCell {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum BootResourceEventKind {
    Claimed = 0,
    Queued = 1,
    Canceled = 2,
    Released = 3,
    HandedOff = 4,
    Revoked = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResourceEventWatch {
    resource: u64,
    token: u64,
    events: u32,
    claimed: bool,
    queued: bool,
    canceled: bool,
    released: bool,
    handed_off: bool,
    revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NetworkEventWatch {
    interface_path: String,
    socket_path: Option<String>,
    token: u64,
    events: u32,
    link_changed: bool,
    rx_ready: bool,
    tx_drained: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootBusEventKind {
    Attached = 0,
    Detached = 1,
    Published = 2,
    Received = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BusEventWatch {
    endpoint: u64,
    token: u64,
    events: u32,
    attached: bool,
    detached: bool,
    published: bool,
    received: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootBusAttachmentEntry {
    peer: u64,
    rights: u64,
}

#[derive(Debug, Clone)]
struct VfsEventWatch {
    inode: u64,
    token: u64,
    events: u32,
    subtree: bool,
    anchor_path: Option<String>,
    owner_pid: u64,
    created: bool,
    opened: bool,
    closed: bool,
    written: bool,
    renamed: bool,
    unlinked: bool,
    mounted: bool,
    unmounted: bool,
    lock_acquired: bool,
    lock_refused: bool,
    permission_refused: bool,
    truncated: bool,
    linked: bool,
}

#[derive(Debug, Clone)]
struct BootEventQueueEntry {
    id: usize,
    mode: NativeEventQueueMode,
    pending: Vec<NativeEventRecord>,
    pending_peak: usize,
    resource_watches: Vec<ResourceEventWatch>,
    bus_watches: Vec<BusEventWatch>,
    network_watches: Vec<NetworkEventWatch>,
    vfs_watches: Vec<VfsEventWatch>,
}

#[derive(Debug)]
struct BootEventQueueRegistry {
    next_id: usize,
    queues: [Option<BootEventQueueEntry>; MAX_EVENT_QUEUE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootBusPeerEntry {
    id: u64,
    owner: u64,
    domain: u64,
    name: InlineName,
    attached_endpoint_count: u64,
    readable_endpoint_count: u64,
    writable_endpoint_count: u64,
    publish_count: u64,
    receive_count: u64,
    last_endpoint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootBusEndpointEntry {
    id: u64,
    domain: u64,
    resource: u64,
    path: String,
    attached_peers: Vec<BootBusAttachmentEntry>,
    queue: Vec<Vec<u8>>,
    publish_count: u64,
    receive_count: u64,
    byte_count: u64,
    peak_queue_depth: u64,
    overflow_count: u64,
    last_peer: u64,
}

#[derive(Debug, Clone)]
struct BootBusRegistry {
    peers: [Option<BootBusPeerEntry>; MAX_BUS_PEER_COUNT],
    endpoints: [Option<BootBusEndpointEntry>; MAX_BUS_ENDPOINT_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootNodeKind {
    Directory,
    File,
    Channel,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootNode {
    path: String,
    kind: BootNodeKind,
    inode: u64,
    bytes: Vec<u8>,
    link_target: Option<String>,
    owner_uid: u32,
    group_gid: u32,
    mode: u32,
    minimum_label: SecurityLabel,
    current_label: SecurityLabel,
    mount_layer: u64,
    mount_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootOrphanNode {
    inode: u64,
    kind: BootNodeKind,
    bytes: Vec<u8>,
    link_target: Option<String>,
    owner_uid: u32,
    group_gid: u32,
    mode: u32,
    minimum_label: SecurityLabel,
    current_label: SecurityLabel,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct BootVfsStats {
    lookup_hits: u64,
    lookup_misses: u64,
    lookup_evictions: u64,
    stat_hits: u64,
    stat_misses: u64,
    stat_evictions: u64,
    directory_hits: u64,
    directory_misses: u64,
    directory_evictions: u64,
    page_hits: u64,
    page_misses: u64,
    page_evictions: u64,
    object_lock_conflicts: u64,
    namespace_lock_conflicts: u64,
    subtree_lock_conflicts: u64,
    vfs_events_emitted: u64,
    vfs_events_delivered: u64,
    vfs_events_filtered: u64,
    vfs_event_queue_overflows: u64,
    vfs_events_coalesced: u64,
    vfs_pending_peak: u64,
    process_reaps: u64,
    reaped_descriptor_records: u64,
    reaped_env_records: u64,
    reaped_vm_objects: u64,
    reaped_vm_decisions: u64,
}

#[derive(Debug, Default)]
struct BootVfs {
    next_inode: u64,
    nodes: Vec<BootNode>,
    orphan_nodes: Vec<BootOrphanNode>,
    lookup_cache: Vec<BootLookupCacheEntry>,
    stat_cache: Vec<BootStatCacheEntry>,
    directory_cache: Vec<BootDirectoryCacheEntry>,
    page_cache: Vec<BootPageCacheEntry>,
    stats: BootVfsStats,
}

#[derive(Debug, Clone)]
struct BootLookupCacheEntry {
    path: String,
    follow_symlink: bool,
    node_index: usize,
}

#[derive(Debug, Clone)]
struct BootStatCacheEntry {
    path: String,
    follow_symlink: bool,
    record: NativeFileStatusRecord,
}

#[derive(Debug, Clone)]
struct BootDirectoryCacheEntry {
    path: String,
    observer_label: SecurityLabel,
    listing: String,
}

#[derive(Debug, Clone)]
struct BootPageCacheEntry {
    node_index: usize,
    page_base: usize,
    bytes: Vec<u8>,
}

struct BootVfsCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<BootVfs>>,
}

#[derive(Debug, Clone)]
struct StorageMountRecord {
    id: u64,
    device_path: String,
    mount_path: String,
    parent_mount_id: u64,
    peer_group: u64,
    master_group: u64,
    propagation_mode: u32,
    entry_count: usize,
    created_mount_root: bool,
}

#[derive(Debug, Default, Clone)]
struct StorageMountState {
    next_id: u64,
    mounts: Vec<StorageMountRecord>,
}

struct StorageMountCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<StorageMountState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VfsLockMode {
    Shared,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VfsLockRecord {
    inode: u64,
    owner_fd: usize,
    token: u16,
    mode: VfsLockMode,
}

struct VfsLockCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<Vec<VfsLockRecord>>>,
}

impl VfsLockCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Vec<VfsLockRecord>) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let state = unsafe { &mut *self.state.get() };
        if state.is_none() {
            *state = Some(Vec::new());
        }
        let result = f(state.as_mut().unwrap());
        self.locked.store(false, Ordering::Release);
        result
    }
}

impl StorageMountCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut StorageMountState) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let state = unsafe { &mut *self.state.get() };
        if state.is_none() {
            *state = Some(StorageMountState::default());
        }
        let result = f(state.as_mut().unwrap());
        self.locked.store(false, Ordering::Release);
        result
    }
}

impl BootVfsCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootVfs) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let state = unsafe { &mut *self.state.get() };
        if state.is_none() {
            *state = Some(BootVfs::new());
        }
        let result = f(state.as_mut().unwrap());
        self.locked.store(false, Ordering::Release);
        result
    }
}

impl BootEventQueueRegistryCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(BootEventQueueRegistry::new()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootEventQueueRegistry) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }

    fn with<R>(&self, f: impl FnOnce(&BootEventQueueRegistry) -> R) -> R {
        unsafe { f(&*self.0.get()) }
    }
}

impl BootVfs {
    const PAGE_CACHE_GRANULE: usize = 256;
    const LOOKUP_CACHE_LIMIT: usize = 64;
    const STAT_CACHE_LIMIT: usize = 64;
    const DIRECTORY_CACHE_LIMIT: usize = 32;
    const PAGE_CACHE_LIMIT: usize = 128;

    fn new() -> Self {
        Self {
            next_inode: 0x424f_4f54_5653_0001,
            nodes: vec![BootNode {
                path: String::from("/"),
                kind: BootNodeKind::Directory,
                inode: 0x424f_4f54_5653_0000,
                bytes: Vec::new(),
                link_target: None,
                owner_uid: 1000,
                group_gid: 1000,
                mode: 0o755,
                minimum_label: SecurityLabel::new(
                    ConfidentialityLevel::Public,
                    IntegrityLevel::Verified,
                ),
                current_label: SecurityLabel::new(
                    ConfidentialityLevel::Public,
                    IntegrityLevel::Verified,
                ),
                mount_layer: 0,
                mount_id: None,
            }],
            orphan_nodes: Vec::new(),
            lookup_cache: Vec::new(),
            stat_cache: Vec::new(),
            directory_cache: Vec::new(),
            page_cache: Vec::new(),
            stats: BootVfsStats::default(),
        }
    }

    fn invalidate_caches(&mut self) {
        self.lookup_cache.clear();
        self.stat_cache.clear();
        self.directory_cache.clear();
        self.page_cache.clear();
    }

    fn push_lookup_cache(&mut self, path: String, follow_symlink: bool, node_index: usize) {
        self.lookup_cache
            .retain(|entry| !(entry.path == path && entry.follow_symlink == follow_symlink));
        if self.lookup_cache.len() >= Self::LOOKUP_CACHE_LIMIT {
            self.lookup_cache.remove(0);
            self.stats.lookup_evictions += 1;
        }
        self.lookup_cache.push(BootLookupCacheEntry {
            path,
            follow_symlink,
            node_index,
        });
    }

    fn push_stat_cache(
        &mut self,
        path: String,
        follow_symlink: bool,
        record: NativeFileStatusRecord,
    ) {
        self.stat_cache
            .retain(|entry| !(entry.path == path && entry.follow_symlink == follow_symlink));
        if self.stat_cache.len() >= Self::STAT_CACHE_LIMIT {
            self.stat_cache.remove(0);
            self.stats.stat_evictions += 1;
        }
        self.stat_cache.push(BootStatCacheEntry {
            path,
            follow_symlink,
            record,
        });
    }

    fn push_directory_cache(
        &mut self,
        path: String,
        observer_label: SecurityLabel,
        listing: String,
    ) {
        self.directory_cache
            .retain(|entry| !(entry.path == path && entry.observer_label == observer_label));
        if self.directory_cache.len() >= Self::DIRECTORY_CACHE_LIMIT {
            self.directory_cache.remove(0);
            self.stats.directory_evictions += 1;
        }
        self.directory_cache.push(BootDirectoryCacheEntry {
            path,
            observer_label,
            listing,
        });
    }

    fn push_page_cache(&mut self, node_index: usize, page_base: usize, bytes: Vec<u8>) {
        self.page_cache
            .retain(|entry| !(entry.node_index == node_index && entry.page_base == page_base));
        if self.page_cache.len() >= Self::PAGE_CACHE_LIMIT {
            self.page_cache.remove(0);
            self.stats.page_evictions += 1;
        }
        self.page_cache.push(BootPageCacheEntry {
            node_index,
            page_base,
            bytes,
        });
    }

    fn stats_text(&self) -> String {
        format!(
            "lookup-hits={} lookup-misses={} lookup-evictions={}\nstat-hits={} stat-misses={} stat-evictions={}\ndirectory-hits={} directory-misses={} directory-evictions={}\npage-hits={} page-misses={} page-evictions={}\nobject-conflicts={} namespace-conflicts={} subtree-conflicts={}\nvfs-events-emitted={} vfs-events-delivered={} vfs-events-filtered={} vfs-event-queue-overflows={} vfs-events-coalesced={} vfs-pending-peak={}\nprocess-reaps={} reaped-descriptors={} reaped-env={} reaped-vm-objects={} reaped-vm-decisions={}\n",
            self.stats.lookup_hits,
            self.stats.lookup_misses,
            self.stats.lookup_evictions,
            self.stats.stat_hits,
            self.stats.stat_misses,
            self.stats.stat_evictions,
            self.stats.directory_hits,
            self.stats.directory_misses,
            self.stats.directory_evictions,
            self.stats.page_hits,
            self.stats.page_misses,
            self.stats.page_evictions,
            self.stats.object_lock_conflicts,
            self.stats.namespace_lock_conflicts,
            self.stats.subtree_lock_conflicts,
            self.stats.vfs_events_emitted,
            self.stats.vfs_events_delivered,
            self.stats.vfs_events_filtered,
            self.stats.vfs_event_queue_overflows,
            self.stats.vfs_events_coalesced,
            self.stats.vfs_pending_peak,
            self.stats.process_reaps,
            self.stats.reaped_descriptor_records,
            self.stats.reaped_env_records,
            self.stats.reaped_vm_objects,
            self.stats.reaped_vm_decisions,
        )
    }

    fn find_live_node_index_by_inode(&self, inode: u64) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.inode == inode)
            .max_by_key(|(_, node)| node.mount_layer)
            .map(|(index, _)| index)
    }

    fn live_link_count(&self, inode: u64) -> u64 {
        self.nodes.iter().filter(|node| node.inode == inode).count() as u64
    }

    fn live_path_for_inode(&self, inode: u64) -> Option<String> {
        self.find_live_node_index_by_inode(inode)
            .map(|index| self.nodes[index].path.clone())
    }

    fn path_contains(ancestor: &str, candidate: &str) -> bool {
        ancestor == candidate
            || ancestor == "/"
            || candidate
                .strip_prefix(ancestor)
                .is_some_and(|suffix| suffix.starts_with('/'))
    }

    fn orphan_index_by_inode(&self, inode: u64) -> Option<usize> {
        self.orphan_nodes
            .iter()
            .position(|node| node.inode == inode)
    }

    fn object_kind_by_inode(&self, inode: u64) -> Option<BootNodeKind> {
        self.find_live_node_index_by_inode(inode)
            .map(|index| self.nodes[index].kind)
            .or_else(|| {
                self.orphan_index_by_inode(inode)
                    .map(|index| self.orphan_nodes[index].kind)
            })
    }

    fn object_len_by_inode(&self, inode: u64) -> Option<usize> {
        self.find_live_node_index_by_inode(inode)
            .map(|index| self.nodes[index].bytes.len())
            .or_else(|| {
                self.orphan_index_by_inode(inode)
                    .map(|index| self.orphan_nodes[index].bytes.len())
            })
    }

    fn object_bytes_range_by_inode(
        &self,
        inode: u64,
        offset: usize,
        len: usize,
    ) -> Option<Vec<u8>> {
        let bytes = self
            .find_live_node_index_by_inode(inode)
            .map(|index| self.nodes[index].bytes.as_slice())
            .or_else(|| {
                self.orphan_index_by_inode(inode)
                    .map(|index| self.orphan_nodes[index].bytes.as_slice())
            })?;
        let mut out = vec![0u8; len];
        if offset < bytes.len() {
            let available = (bytes.len() - offset).min(len);
            out[..available].copy_from_slice(&bytes[offset..offset + available]);
        }
        Some(out)
    }

    fn clone_object_by_inode(&self, inode: u64) -> Option<BootOrphanNode> {
        self.find_live_node_index_by_inode(inode)
            .map(|index| {
                let node = &self.nodes[index];
                BootOrphanNode {
                    inode: node.inode,
                    kind: node.kind,
                    bytes: node.bytes.clone(),
                    link_target: node.link_target.clone(),
                    owner_uid: node.owner_uid,
                    group_gid: node.group_gid,
                    mode: node.mode,
                    minimum_label: node.minimum_label,
                    current_label: node.current_label,
                }
            })
            .or_else(|| {
                self.orphan_index_by_inode(inode)
                    .map(|index| self.orphan_nodes[index].clone())
            })
    }

    fn object_current_label_by_inode(&self, inode: u64) -> Option<SecurityLabel> {
        self.find_live_node_index_by_inode(inode)
            .map(|index| self.nodes[index].current_label)
            .or_else(|| {
                self.orphan_index_by_inode(inode)
                    .map(|index| self.orphan_nodes[index].current_label)
            })
    }

    fn require_observe_inode(&self, inode: u64) -> Result<(), Errno> {
        let Some(label) = self.object_current_label_by_inode(inode) else {
            return Err(Errno::NoEnt);
        };
        if check_ifc_read(Self::current_subject_label(), label).is_err() {
            Err(Errno::Access)
        } else {
            Ok(())
        }
    }

    fn subject_can_observe_entry(node: &BootNode) -> bool {
        check_ifc_read(Self::current_subject_label(), node.current_label).is_ok()
    }

    fn ensure_orphan_inode(&mut self, inode: u64) {
        if self.orphan_index_by_inode(inode).is_some() {
            return;
        }
        if let Some(orphan) = self.clone_object_by_inode(inode) {
            self.orphan_nodes.push(orphan);
        }
    }

    fn release_orphan_inode_if_unreferenced(&mut self, inode: u64) {
        if self.live_link_count(inode) != 0 {
            return;
        }
        let still_open = DESCRIPTORS.with(|descriptors| descriptors.references_inode(inode));
        if !still_open {
            self.orphan_nodes.retain(|node| node.inode != inode);
        }
    }

    fn current_subject() -> (u32, u32) {
        BOOT_PROCESSES.with_mut(|registry| {
            registry
                .find_index(1)
                .map(|index| {
                    let entry = &registry.entries[index];
                    (entry.uid, entry.gid)
                })
                .unwrap_or((1000, 1000))
        })
    }

    fn current_subject_label() -> SecurityLabel {
        BOOT_PROCESSES.with_mut(|registry| {
            registry
                .find_index(1)
                .map(|index| registry.entries[index].subject_label)
                .unwrap_or(SecurityLabel::new(
                    ConfidentialityLevel::Public,
                    IntegrityLevel::Verified,
                ))
        })
    }

    fn set_current_subject(uid: u32, gid: u32) {
        BOOT_PROCESSES.with_mut(|registry| {
            if let Some(index) = registry.find_index(1) {
                registry.entries[index].uid = uid;
                registry.entries[index].gid = gid;
            }
        });
    }

    fn set_current_subject_label(label: SecurityLabel) {
        BOOT_PROCESSES.with_mut(|registry| {
            if let Some(index) = registry.find_index(1) {
                registry.entries[index].subject_label = label;
            }
        });
    }

    fn current_umask() -> u32 {
        BOOT_PROCESSES.with_mut(|registry| {
            registry
                .find_index(1)
                .map(|index| registry.entries[index].umask)
                .unwrap_or(0o022)
        })
    }

    fn set_current_umask(umask: u32) {
        BOOT_PROCESSES.with_mut(|registry| {
            if let Some(index) = registry.find_index(1) {
                registry.entries[index].umask = umask & 0o777;
            }
        });
    }

    fn set_current_supplemental_groups(groups: &[u32]) {
        BOOT_PROCESSES.with_mut(|registry| {
            if let Some(index) = registry.find_index(1) {
                let entry = &mut registry.entries[index];
                entry.supplemental_count = groups.len().min(entry.supplemental_gids.len());
                for (index, gid) in groups.iter().take(entry.supplemental_count).enumerate() {
                    entry.supplemental_gids[index] = *gid;
                }
                for index in entry.supplemental_count..entry.supplemental_gids.len() {
                    entry.supplemental_gids[index] = 0;
                }
            }
        });
    }

    fn supplemental_group_match(group_gid: u32) -> bool {
        BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(1) else {
                return false;
            };
            registry.entries[index].supplemental_gids[..registry.entries[index].supplemental_count]
                .iter()
                .any(|candidate| *candidate == group_gid)
        })
    }

    fn permission_mask(mode: u32, owner_uid: u32, group_gid: u32, uid: u32, gid: u32) -> u32 {
        if uid == 0 {
            return 0o7;
        }
        if uid == owner_uid {
            (mode >> 6) & 0o7
        } else if gid == group_gid || Self::supplemental_group_match(group_gid) {
            (mode >> 3) & 0o7
        } else {
            mode & 0o7
        }
    }

    fn require_access_for_node(
        node: &BootNode,
        read: bool,
        write: bool,
        execute: bool,
    ) -> Result<(), Errno> {
        let (uid, gid) = Self::current_subject();
        let mask = Self::permission_mask(node.mode, node.owner_uid, node.group_gid, uid, gid);
        let needed =
            (u32::from(read) * 0o4) | (u32::from(write) * 0o2) | (u32::from(execute) * 0o1);
        if (mask & needed) != needed {
            return Err(Errno::Access);
        }
        let subject_label = Self::current_subject_label();
        if (read || execute) && check_ifc_read(subject_label, node.current_label).is_err() {
            return Err(Errno::Access);
        }
        if write && check_ifc_write(subject_label, node.current_label).is_err() {
            return Err(Errno::Access);
        }
        Ok(())
    }

    fn require_access(
        &self,
        path: &str,
        read: bool,
        write: bool,
        execute: bool,
    ) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        let index = self.find_node(&path).ok_or(Errno::NoEnt)?;
        Self::require_access_for_node(&self.nodes[index], read, write, execute)
    }

    fn require_parent_mutation_access(&self, path: &str) -> Result<(), Errno> {
        self.require_access(Self::parent_path(path), false, true, true)
    }

    fn require_sticky_mutation_access(&self, path: &str) -> Result<(), Errno> {
        let parent = Self::parent_path(path);
        let Some(parent_index) = self.find_node(parent) else {
            return Err(Errno::NoEnt);
        };
        let parent_node = &self.nodes[parent_index];
        if (parent_node.mode & 0o1000) == 0 {
            return Ok(());
        }
        let (uid, _) = Self::current_subject();
        if uid == 0 || uid == parent_node.owner_uid {
            return Ok(());
        }
        let Some(index) = self.find_node(path) else {
            return Err(Errno::NoEnt);
        };
        if uid == self.nodes[index].owner_uid {
            Ok(())
        } else {
            Err(Errno::Access)
        }
    }

    fn inherited_metadata_for_new_node(&self, path: &str, kind: BootNodeKind) -> (u32, u32, u32) {
        let parent = Self::parent_path(path);
        let (uid, gid) = Self::current_subject();
        let Some(parent_index) = self.find_node(parent) else {
            let mut mode = match kind {
                BootNodeKind::Directory => 0o755,
                BootNodeKind::File => 0o644,
                BootNodeKind::Channel => 0o660,
                BootNodeKind::Symlink => 0o777,
            };
            if kind != BootNodeKind::Symlink {
                mode &= !Self::current_umask();
            }
            return (uid, gid, mode);
        };
        let parent_node = &self.nodes[parent_index];
        let mut mode = match kind {
            BootNodeKind::Directory => 0o755,
            BootNodeKind::File => 0o644,
            BootNodeKind::Channel => 0o660,
            BootNodeKind::Symlink => 0o777,
        };
        if kind != BootNodeKind::Symlink {
            mode &= !Self::current_umask();
        }
        let inherited_gid = if (parent_node.mode & 0o2000) != 0 {
            parent_node.group_gid
        } else {
            gid
        };
        if kind == BootNodeKind::Directory && (parent_node.mode & 0o2000) != 0 {
            mode |= 0o2000;
        }
        (uid, inherited_gid, mode)
    }

    fn inherited_security_for_new_node(&self, path: &str) -> (SecurityLabel, SecurityLabel) {
        let parent = Self::parent_path(path);
        let Some(parent_index) = self.find_node(parent) else {
            let label = SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
            return (label, label);
        };
        let parent_node = &self.nodes[parent_index];
        (parent_node.minimum_label, parent_node.current_label)
    }

    fn require_traversal_access(
        &self,
        path: &str,
        include_self_directory: bool,
    ) -> Result<(), Errno> {
        let path = Self::normalize_absolute_path(path)?;
        if path == "/" {
            return if include_self_directory {
                self.require_access("/", false, false, true)
            } else {
                Ok(())
            };
        }
        let final_is_directory = self
            .find_node(&path)
            .is_some_and(|index| self.nodes[index].kind == BootNodeKind::Directory);
        let segments = path
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        let mut probe = String::from("/");
        for (index, segment) in segments.iter().enumerate() {
            if probe != "/" {
                probe.push('/');
            }
            probe.push_str(segment);
            let is_last = index + 1 == segments.len();
            if !is_last || (include_self_directory && final_is_directory) {
                self.require_access(&probe, false, false, true)?;
            }
        }
        Ok(())
    }

    fn page_bytes(&mut self, node_index: usize, page_base: usize) -> Option<Vec<u8>> {
        if let Some(entry) = self
            .page_cache
            .iter()
            .find(|entry| entry.node_index == node_index && entry.page_base == page_base)
        {
            self.stats.page_hits += 1;
            return Some(entry.bytes.clone());
        }
        self.stats.page_misses += 1;
        let node = self.nodes.get(node_index)?;
        if page_base >= node.bytes.len() {
            return Some(Vec::new());
        }
        let end = (page_base + Self::PAGE_CACHE_GRANULE).min(node.bytes.len());
        let bytes = node.bytes[page_base..end].to_vec();
        self.push_page_cache(node_index, page_base, bytes.clone());
        Some(bytes)
    }

    fn find_node(&self, path: &str) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.path == path)
            .max_by(|left, right| {
                left.1
                    .mount_layer
                    .cmp(&right.1.mount_layer)
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|(index, _)| index)
    }

    fn list_directory(&self, path: &str) -> Result<Vec<&BootNode>, Errno> {
        let path = Self::normalize_path(path)?;
        let Some(index) = self.find_node(&path) else {
            return Err(Errno::NoEnt);
        };
        if self.nodes[index].kind != BootNodeKind::Directory {
            return Err(Errno::NotDir);
        }
        let prefix = if path == "/" {
            String::from("/")
        } else {
            format!("{path}/")
        };
        let mut visible: Vec<&BootNode> = Vec::new();
        let mut names = Vec::<String>::new();
        for candidate in self.nodes.iter().filter(|candidate| {
            candidate.path.starts_with(&prefix) && !candidate.path[prefix.len()..].contains('/')
        }) {
            let name = candidate
                .path
                .rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .unwrap_or("/")
                .to_string();
            if let Some(existing_index) = names.iter().position(|existing| existing == &name) {
                let existing = visible[existing_index];
                if candidate.mount_layer >= existing.mount_layer {
                    visible[existing_index] = candidate;
                }
            } else {
                names.push(name);
                visible.push(candidate);
            }
        }
        Ok(visible)
    }

    fn list_directory_text(&mut self, path: &str) -> Result<String, Errno> {
        let path = Self::normalize_path(path)?;
        let observer_label = Self::current_subject_label();
        if let Some(entry) = self
            .directory_cache
            .iter()
            .find(|entry| entry.path == path && entry.observer_label == observer_label)
        {
            self.stats.directory_hits += 1;
            return Ok(entry.listing.clone());
        }
        self.stats.directory_misses += 1;
        let nodes = self.list_directory(&path)?;
        let mut out = String::new();
        for node in nodes {
            if !Self::subject_can_observe_entry(node) {
                continue;
            }
            let name = node
                .path
                .rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .unwrap_or("/");
            out.push_str(name);
            out.push('\t');
            out.push_str(match node.kind {
                BootNodeKind::Directory => "Directory",
                BootNodeKind::File => "File",
                BootNodeKind::Symlink => "Symlink",
                BootNodeKind::Channel => "Channel",
            });
            out.push('\n');
        }
        self.push_directory_cache(path, observer_label, out.clone());
        Ok(out)
    }

    fn normalize_path(path: &str) -> Result<String, Errno> {
        Self::normalize_absolute_path(path)
    }

    fn normalize_absolute_path(path: &str) -> Result<String, Errno> {
        if path.is_empty() || !path.starts_with('/') {
            return Err(Errno::Inval);
        }
        let mut segments = Vec::<&str>::new();
        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(Errno::Inval);
                    }
                }
                value => segments.push(value),
            }
        }
        if segments.is_empty() {
            return Ok(String::from("/"));
        }
        Ok(format!("/{}", segments.join("/")))
    }

    fn normalize_relative_target(target: &str) -> Result<String, Errno> {
        if target.is_empty() {
            return Err(Errno::Inval);
        }
        if target.starts_with('/') {
            return Self::normalize_absolute_path(target);
        }
        let mut segments = Vec::<&str>::new();
        for segment in target.split('/') {
            match segment {
                "" | "." => {}
                ".." => segments.push(".."),
                value => segments.push(value),
            }
        }
        if segments.is_empty() {
            return Ok(String::from("."));
        }
        Ok(segments.join("/"))
    }

    fn resolve_path_from_root(root: &str, cwd: &str, path: &str) -> Result<String, Errno> {
        let root = Self::normalize_absolute_path(root)?;
        let root_segments = root
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut base_segments = if path.starts_with('/') {
            root_segments.clone()
        } else {
            Self::normalize_absolute_path(cwd)?
                .trim_start_matches('/')
                .split('/')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        if base_segments.len() < root_segments.len()
            || base_segments[..root_segments.len()] != root_segments[..]
        {
            return Err(Errno::Access);
        }
        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if base_segments.len() <= root_segments.len() {
                        return Err(Errno::Access);
                    }
                    base_segments.pop();
                }
                value => base_segments.push(value.to_string()),
            }
        }
        if base_segments.is_empty() {
            Ok(String::from("/"))
        } else {
            Ok(format!("/{}", base_segments.join("/")))
        }
    }

    fn join_relative_target(base_path: &str, target: &str) -> Result<String, Errno> {
        if target.starts_with('/') {
            return Self::normalize_absolute_path(target);
        }
        let mut base_segments = Self::normalize_absolute_path(Self::parent_path(base_path))?
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        for segment in target.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if base_segments.pop().is_none() {
                        return Err(Errno::Inval);
                    }
                }
                value => base_segments.push(value.to_string()),
            }
        }
        if base_segments.is_empty() {
            Ok(String::from("/"))
        } else {
            Ok(format!("/{}", base_segments.join("/")))
        }
    }

    fn parent_path(path: &str) -> &str {
        if path == "/" {
            return "/";
        }
        match path.rfind('/') {
            Some(0) => "/",
            Some(index) => &path[..index],
            None => "/",
        }
    }

    fn ensure_parent_directory(&self, path: &str) -> Result<(), Errno> {
        let parent = Self::parent_path(path);
        let Some(index) = self.find_node(parent) else {
            return Err(Errno::NoEnt);
        };
        if self.nodes[index].kind != BootNodeKind::Directory {
            return Err(Errno::NotDir);
        }
        Ok(())
    }

    fn create(&mut self, path: &str, kind: BootNodeKind) -> Result<(), Errno> {
        self.create_with_mount(path, kind, None)
    }

    fn create_with_mount(
        &mut self,
        path: &str,
        kind: BootNodeKind,
        mount_id: Option<u64>,
    ) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        if path == "/" {
            return Err(Errno::Exist);
        }
        self.ensure_parent_directory(&path)?;
        self.require_parent_mutation_access(&path)?;
        self.namespace_lock_conflict(&path, None)?;
        let inherited_mount_id = self
            .find_node(Self::parent_path(&path))
            .and_then(|index| self.nodes.get(index).and_then(|node| node.mount_id));
        let mount_id = mount_id.or(inherited_mount_id);
        if mount_id.is_none() && self.find_node(&path).is_some() {
            return Err(Errno::Exist);
        }
        if self
            .nodes
            .iter()
            .any(|node| node.path == path && node.mount_id == mount_id)
        {
            return Err(Errno::Exist);
        }
        let inode = self.next_inode;
        self.next_inode = self.next_inode.saturating_add(1);
        let (owner_uid, group_gid, mode) = self.inherited_metadata_for_new_node(&path, kind);
        let (minimum_label, current_label) = self.inherited_security_for_new_node(&path);
        self.nodes.push(BootNode {
            path,
            kind,
            inode,
            bytes: Vec::new(),
            link_target: None,
            owner_uid,
            group_gid,
            mode,
            minimum_label,
            current_label,
            mount_layer: mount_id.unwrap_or(0),
            mount_id,
        });
        self.invalidate_caches();
        Ok(())
    }

    fn create_symlink(&mut self, path: &str, target: &str) -> Result<(), Errno> {
        self.create_symlink_with_mount(path, target, None)
    }

    fn create_symlink_with_mount(
        &mut self,
        path: &str,
        target: &str,
        mount_id: Option<u64>,
    ) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        let target = Self::normalize_relative_target(target)?;
        if path == "/" {
            return Err(Errno::Exist);
        }
        self.ensure_parent_directory(&path)?;
        self.require_parent_mutation_access(&path)?;
        self.namespace_lock_conflict(&path, None)?;
        let inherited_mount_id = self
            .find_node(Self::parent_path(&path))
            .and_then(|index| self.nodes.get(index).and_then(|node| node.mount_id));
        let mount_id = mount_id.or(inherited_mount_id);
        if mount_id.is_none() && self.find_node(&path).is_some() {
            return Err(Errno::Exist);
        }
        if self
            .nodes
            .iter()
            .any(|node| node.path == path && node.mount_id == mount_id)
        {
            return Err(Errno::Exist);
        }
        let inode = self.next_inode;
        self.next_inode = self.next_inode.saturating_add(1);
        let (owner_uid, group_gid, mode) =
            self.inherited_metadata_for_new_node(&path, BootNodeKind::Symlink);
        let (minimum_label, current_label) = self.inherited_security_for_new_node(&path);
        self.nodes.push(BootNode {
            path,
            kind: BootNodeKind::Symlink,
            inode,
            bytes: Vec::new(),
            link_target: Some(target),
            owner_uid,
            group_gid,
            mode,
            minimum_label,
            current_label,
            mount_layer: mount_id.unwrap_or(0),
            mount_id,
        });
        self.invalidate_caches();
        Ok(())
    }

    fn link_file(&mut self, source: &str, destination: &str) -> Result<(), Errno> {
        let source = Self::normalize_path(source)?;
        let destination = Self::normalize_path(destination)?;
        if source == "/" || destination == "/" || source == destination {
            return Err(Errno::Inval);
        }
        self.ensure_parent_directory(&destination)?;
        self.require_access(&source, true, false, false)?;
        self.require_parent_mutation_access(&destination)?;
        self.namespace_lock_conflict(&destination, None)?;
        self.object_lock_conflict(&source, None)?;
        if self.find_node(&destination).is_some() {
            return Err(Errno::Exist);
        }
        let source_index = self.resolve_node_index(&source, false)?;
        let destination_parent_index =
            self.resolve_node_index(Self::parent_path(&destination), true)?;
        let source_node = self.nodes.get(source_index).ok_or(Errno::NoEnt)?.clone();
        if source_node.mount_id != self.nodes[destination_parent_index].mount_id {
            return Err(Errno::Busy);
        }
        if source_node.kind != BootNodeKind::File {
            return Err(Errno::Inval);
        }
        self.nodes.push(BootNode {
            path: destination,
            kind: BootNodeKind::File,
            inode: source_node.inode,
            bytes: source_node.bytes,
            link_target: None,
            owner_uid: source_node.owner_uid,
            group_gid: source_node.group_gid,
            mode: source_node.mode,
            minimum_label: source_node.minimum_label,
            current_label: source_node.current_label,
            mount_layer: source_node.mount_layer,
            mount_id: source_node.mount_id,
        });
        self.invalidate_caches();
        Ok(())
    }

    fn resolve_node_index(&mut self, path: &str, follow_symlink: bool) -> Result<usize, Errno> {
        self.resolve_node_index_depth(path, follow_symlink, 0)
    }

    fn resolve_node_index_depth(
        &mut self,
        path: &str,
        follow_symlink: bool,
        depth: usize,
    ) -> Result<usize, Errno> {
        if depth > 8 {
            return Err(Errno::Inval);
        }
        let path = Self::normalize_path(path)?;
        if depth == 0 {
            if let Some(entry) = self
                .lookup_cache
                .iter()
                .find(|entry| entry.path == path && entry.follow_symlink == follow_symlink)
            {
                self.stats.lookup_hits += 1;
                return Ok(entry.node_index);
            }
            self.stats.lookup_misses += 1;
        }
        let Some(index) = self.find_node(&path) else {
            return Err(Errno::NoEnt);
        };
        let node = &self.nodes[index];
        if follow_symlink && node.kind == BootNodeKind::Symlink {
            let target = node.link_target.as_deref().ok_or(Errno::Inval)?;
            let resolved = Self::join_relative_target(&path, target)?;
            return self.resolve_node_index_depth(&resolved, true, depth + 1);
        }
        if depth == 0 {
            self.push_lookup_cache(path, follow_symlink, index);
        }
        Ok(index)
    }

    fn link_count_for_inode(&self, inode: u64) -> u64 {
        self.nodes.iter().filter(|node| node.inode == inode).count() as u64
    }

    fn stat(&mut self, path: &str, follow_symlink: bool) -> Option<NativeFileStatusRecord> {
        let path = Self::normalize_path(path).ok()?;
        if let Some(entry) = self
            .stat_cache
            .iter()
            .find(|entry| entry.path == path && entry.follow_symlink == follow_symlink)
        {
            self.stats.stat_hits += 1;
            return Some(entry.record);
        }
        self.stats.stat_misses += 1;
        let index = self.resolve_node_index(&path, follow_symlink).ok()?;
        let node = self.nodes.get(index)?;
        let (kind, readable, writable) = match node.kind {
            BootNodeKind::Directory => (NativeObjectKind::Directory as u32, 1, 0),
            BootNodeKind::File => (NativeObjectKind::File as u32, 1, 1),
            BootNodeKind::Channel => (NativeObjectKind::Channel as u32, 1, 1),
            BootNodeKind::Symlink => (NativeObjectKind::Symlink as u32, 1, 0),
        };
        let record = NativeFileStatusRecord {
            inode: node.inode,
            link_count: self.link_count_for_inode(node.inode),
            size: node
                .link_target
                .as_ref()
                .map(|target| target.len())
                .unwrap_or_else(|| node.bytes.len()) as u64,
            kind,
            cloexec: 0,
            nonblock: 0,
            readable,
            writable,
            executable: u32::from((node.mode & 0o111) != 0),
            owner_uid: node.owner_uid,
            group_gid: node.group_gid,
            mode: node.mode,
        };
        self.push_stat_cache(path, follow_symlink, record);
        Some(record)
    }

    fn stat_by_inode(&mut self, inode: u64) -> Option<NativeFileStatusRecord> {
        if let Some(index) = self.find_live_node_index_by_inode(inode) {
            let node = self.nodes.get(index)?;
            let (kind, readable, writable) = match node.kind {
                BootNodeKind::Directory => (NativeObjectKind::Directory as u32, 1, 0),
                BootNodeKind::File => (NativeObjectKind::File as u32, 1, 1),
                BootNodeKind::Channel => (NativeObjectKind::Channel as u32, 1, 1),
                BootNodeKind::Symlink => (NativeObjectKind::Symlink as u32, 1, 0),
            };
            return Some(NativeFileStatusRecord {
                inode: node.inode,
                link_count: self.link_count_for_inode(node.inode),
                size: node
                    .link_target
                    .as_ref()
                    .map(|target| target.len())
                    .unwrap_or_else(|| node.bytes.len()) as u64,
                kind,
                cloexec: 0,
                nonblock: 0,
                readable,
                writable,
                executable: u32::from((node.mode & 0o111) != 0),
                owner_uid: node.owner_uid,
                group_gid: node.group_gid,
                mode: node.mode,
            });
        }
        let index = self.orphan_index_by_inode(inode)?;
        let node = self.orphan_nodes.get(index)?;
        let (kind, readable, writable) = match node.kind {
            BootNodeKind::Directory => (NativeObjectKind::Directory as u32, 1, 0),
            BootNodeKind::File => (NativeObjectKind::File as u32, 1, 1),
            BootNodeKind::Channel => (NativeObjectKind::Channel as u32, 1, 1),
            BootNodeKind::Symlink => (NativeObjectKind::Symlink as u32, 1, 0),
        };
        Some(NativeFileStatusRecord {
            inode: node.inode,
            link_count: 0,
            size: node
                .link_target
                .as_ref()
                .map(|target| target.len())
                .unwrap_or_else(|| node.bytes.len()) as u64,
            kind,
            cloexec: 0,
            nonblock: 0,
            readable,
            writable,
            executable: u32::from((node.mode & 0o111) != 0),
            owner_uid: node.owner_uid,
            group_gid: node.group_gid,
            mode: node.mode,
        })
    }

    fn file_size(&mut self, path: &str) -> Result<usize, Errno> {
        let index = self.resolve_node_index(path, true)?;
        let node = &self.nodes[index];
        match node.kind {
            BootNodeKind::Directory => Err(Errno::IsDir),
            BootNodeKind::File | BootNodeKind::Channel => {
                self.object_len_by_inode(node.inode).ok_or(Errno::Badf)
            }
            BootNodeKind::Symlink => Err(Errno::Inval),
        }
    }

    fn object_lock_conflict(
        &mut self,
        path: &str,
        actor_description: Option<usize>,
    ) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        let index = self.resolve_node_index(&path, false)?;
        let node = &self.nodes[index];
        if !matches!(node.kind, BootNodeKind::File | BootNodeKind::Channel) {
            return Ok(());
        }
        let inode = node.inode;
        VFS_LOCKS.with_mut(|locks| {
            if locks.iter().any(|lock| {
                lock.inode == inode && actor_description.is_none_or(|actor| actor != lock.owner_fd)
            }) {
                self.stats.object_lock_conflicts += 1;
                Err(Errno::Busy)
            } else {
                Ok(())
            }
        })
    }

    fn namespace_lock_conflict(
        &mut self,
        path: &str,
        actor_description: Option<usize>,
    ) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        VFS_LOCKS.with_mut(|locks| {
            for lock in locks.iter() {
                if actor_description.is_some_and(|actor| actor == lock.owner_fd) {
                    continue;
                }
                if self.object_kind_by_inode(lock.inode) != Some(BootNodeKind::Directory) {
                    continue;
                }
                let Some(lock_path) = self.live_path_for_inode(lock.inode) else {
                    continue;
                };
                if Self::path_contains(&lock_path, &path) {
                    self.stats.namespace_lock_conflicts += 1;
                    return Err(Errno::Busy);
                }
            }
            Ok(())
        })
    }

    fn subtree_lock_conflict(
        &mut self,
        path: &str,
        actor_description: Option<usize>,
    ) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        VFS_LOCKS.with_mut(|locks| {
            for lock in locks.iter() {
                if actor_description.is_some_and(|actor| actor == lock.owner_fd) {
                    continue;
                }
                let Some(lock_path) = self.live_path_for_inode(lock.inode) else {
                    continue;
                };
                if Self::path_contains(&path, &lock_path) {
                    self.stats.subtree_lock_conflicts += 1;
                    return Err(Errno::Busy);
                }
            }
            Ok(())
        })
    }

    fn readlink(&mut self, path: &str) -> Result<&str, Errno> {
        let path = Self::normalize_path(path)?;
        let Some(index) = self.find_node(&path) else {
            return Err(Errno::NoEnt);
        };
        let node = &self.nodes[index];
        if node.kind != BootNodeKind::Symlink {
            return Err(Errno::Inval);
        }
        node.link_target.as_deref().ok_or(Errno::Inval)
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), Errno> {
        let from = Self::normalize_path(from)?;
        let to = Self::normalize_path(to)?;
        if from == "/" || to == "/" {
            return Err(Errno::Inval);
        }
        if to == from || to.starts_with(&(from.clone() + "/")) {
            return Err(Errno::Inval);
        }
        self.ensure_parent_directory(&to)?;
        self.require_parent_mutation_access(&from)?;
        self.require_parent_mutation_access(&to)?;
        self.namespace_lock_conflict(&from, None)?;
        self.namespace_lock_conflict(&to, None)?;
        self.require_sticky_mutation_access(&from)?;
        self.object_lock_conflict(&from, None)?;
        let Some(mut index) = self.find_node(&from) else {
            return Err(Errno::NoEnt);
        };
        let destination_parent_index = self.resolve_node_index(Self::parent_path(&to), true)?;
        if self.nodes[index].mount_id != self.nodes[destination_parent_index].mount_id {
            return Err(Errno::Busy);
        }
        let source_inode = self.nodes[index].inode;
        if let Some(target_index) = self.find_node(&to) {
            self.object_lock_conflict(&to, None)?;
            self.require_sticky_mutation_access(&to)?;
            let source_kind = self.nodes[index].kind;
            let target_kind = self.nodes[target_index].kind;
            if source_kind == BootNodeKind::Directory && target_kind != BootNodeKind::Directory {
                return Err(Errno::Inval);
            }
            if source_kind != BootNodeKind::Directory && target_kind == BootNodeKind::Directory {
                return Err(Errno::Inval);
            }
            if target_kind == BootNodeKind::Directory {
                self.subtree_lock_conflict(&to, None)?;
                let prefix = to.clone() + "/";
                if self.nodes.iter().any(|node| node.path.starts_with(&prefix)) {
                    return Err(Errno::Busy);
                }
            }
            let target_inode = self.nodes[target_index].inode;
            if self.live_link_count(target_inode) == 1 {
                self.ensure_orphan_inode(target_inode);
            }
            self.nodes.remove(target_index);
            if target_index < index {
                index -= 1;
            }
            self.release_orphan_inode_if_unreferenced(target_inode);
        }
        if self.nodes[index].kind == BootNodeKind::Directory {
            self.subtree_lock_conflict(&from, None)?;
            let from_prefix = from.clone() + "/";
            let to_prefix = to.clone() + "/";
            for node in &mut self.nodes {
                if node.path == from {
                    node.path = to.clone();
                } else if node.path.starts_with(&from_prefix) {
                    node.path = format!("{}{}", to_prefix, &node.path[from_prefix.len()..]);
                }
            }
        } else {
            self.nodes[index].path = to;
        }
        self.release_orphan_inode_if_unreferenced(source_inode);
        self.invalidate_caches();
        Ok(())
    }

    fn unlink(&mut self, path: &str) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        if path == "/" {
            return Err(Errno::Inval);
        }
        self.require_parent_mutation_access(&path)?;
        self.namespace_lock_conflict(&path, None)?;
        self.require_sticky_mutation_access(&path)?;
        self.object_lock_conflict(&path, None)?;
        let Some(index) = self.find_node(&path) else {
            return Err(Errno::NoEnt);
        };
        if self.nodes[index].kind == BootNodeKind::Directory {
            self.subtree_lock_conflict(&path, None)?;
            let prefix = path.clone() + "/";
            if self.nodes.iter().any(|node| node.path.starts_with(&prefix)) {
                return Err(Errno::Busy);
            }
        }
        let inode = self.nodes[index].inode;
        if self.live_link_count(inode) == 1 {
            self.ensure_orphan_inode(inode);
        }
        self.nodes.remove(index);
        self.release_orphan_inode_if_unreferenced(inode);
        self.invalidate_caches();
        Ok(())
    }

    fn chmod(&mut self, path: &str, mode: u32) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        let index = self.resolve_node_index(&path, false)?;
        let (uid, _) = Self::current_subject();
        if uid != 0 && uid != self.nodes[index].owner_uid {
            return Err(Errno::Perm);
        }
        self.nodes[index].mode = mode & 0o7777;
        self.invalidate_caches();
        Ok(())
    }

    fn chmod_by_inode(&mut self, inode: u64, mode: u32) -> Result<(), Errno> {
        let (uid, _) = Self::current_subject();
        if let Some(index) = self.find_live_node_index_by_inode(inode) {
            if uid != 0 && uid != self.nodes[index].owner_uid {
                return Err(Errno::Perm);
            }
            let masked = mode & 0o7777;
            for entry in self.nodes.iter_mut().filter(|entry| entry.inode == inode) {
                entry.mode = masked;
            }
            if let Some(orphan_index) = self.orphan_index_by_inode(inode) {
                self.orphan_nodes[orphan_index].mode = masked;
            }
            self.invalidate_caches();
            return Ok(());
        }
        let orphan_index = self.orphan_index_by_inode(inode).ok_or(Errno::Badf)?;
        if uid != 0 && uid != self.orphan_nodes[orphan_index].owner_uid {
            return Err(Errno::Perm);
        }
        self.orphan_nodes[orphan_index].mode = mode & 0o7777;
        Ok(())
    }

    fn chown(&mut self, path: &str, owner_uid: u32, group_gid: u32) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        let index = self.resolve_node_index(&path, false)?;
        let (uid, _) = Self::current_subject();
        if uid != 0 {
            return Err(Errno::Perm);
        }
        self.nodes[index].owner_uid = owner_uid;
        self.nodes[index].group_gid = group_gid;
        self.invalidate_caches();
        Ok(())
    }

    fn chown_by_inode(&mut self, inode: u64, owner_uid: u32, group_gid: u32) -> Result<(), Errno> {
        let (uid, _) = Self::current_subject();
        if uid != 0 {
            return Err(Errno::Perm);
        }
        if self.find_live_node_index_by_inode(inode).is_some() {
            for entry in self.nodes.iter_mut().filter(|entry| entry.inode == inode) {
                entry.owner_uid = owner_uid;
                entry.group_gid = group_gid;
            }
            if let Some(orphan_index) = self.orphan_index_by_inode(inode) {
                self.orphan_nodes[orphan_index].owner_uid = owner_uid;
                self.orphan_nodes[orphan_index].group_gid = group_gid;
            }
            self.invalidate_caches();
            return Ok(());
        }
        let orphan_index = self.orphan_index_by_inode(inode).ok_or(Errno::Badf)?;
        self.orphan_nodes[orphan_index].owner_uid = owner_uid;
        self.orphan_nodes[orphan_index].group_gid = group_gid;
        Ok(())
    }

    fn security_context(&mut self, path: &str) -> Result<ObjectSecurityContext, Errno> {
        let path = Self::normalize_path(path)?;
        let index = self.resolve_node_index(&path, false)?;
        let node = &self.nodes[index];
        Ok(ObjectSecurityContext::new(
            node.inode,
            BlockRightsMask::READ.union(BlockRightsMask::WRITE),
            node.minimum_label,
            node.current_label,
            ProvenanceTag::root(
                ProvenanceOriginKind::Subject,
                node.inode,
                node.inode,
                IntegrityTag::zeroed(IntegrityTagKind::Blake3),
            ),
            IntegrityTag::zeroed(IntegrityTagKind::Blake3),
            0,
            0,
        ))
    }

    fn set_security_label(&mut self, path: &str, label: SecurityLabel) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        let index = self.resolve_node_index(&path, false)?;
        let inode = self.nodes[index].inode;
        for entry in self.nodes.iter_mut().filter(|entry| entry.inode == inode) {
            entry.minimum_label = label;
            entry.current_label = label;
        }
        if let Some(orphan_index) = self.orphan_index_by_inode(inode) {
            self.orphan_nodes[orphan_index].minimum_label = label;
            self.orphan_nodes[orphan_index].current_label = label;
        }
        self.invalidate_caches();
        Ok(())
    }

    fn truncate_by_inode(&mut self, inode: u64, len: usize) -> Result<(), Errno> {
        let kind = self.object_kind_by_inode(inode).ok_or(Errno::Badf)?;
        match kind {
            BootNodeKind::File | BootNodeKind::Channel => {
                let had_live_node = self.find_live_node_index_by_inode(inode).is_some();
                for entry in self.nodes.iter_mut().filter(|entry| entry.inode == inode) {
                    entry.bytes.resize(len, 0);
                }
                if let Some(orphan_index) = self.orphan_index_by_inode(inode) {
                    self.orphan_nodes[orphan_index].bytes.resize(len, 0);
                }
                if had_live_node {
                    self.invalidate_caches();
                }
                Ok(())
            }
            BootNodeKind::Directory => Err(Errno::IsDir),
            BootNodeKind::Symlink => Err(Errno::Inval),
        }
    }

    fn link_inode_to_path(&mut self, inode: u64, destination: &str) -> Result<(), Errno> {
        let destination = Self::normalize_path(destination)?;
        if destination == "/" {
            return Err(Errno::Inval);
        }
        self.ensure_parent_directory(&destination)?;
        self.require_parent_mutation_access(&destination)?;
        self.namespace_lock_conflict(&destination, None)?;
        let source = self.clone_object_by_inode(inode).ok_or(Errno::Badf)?;
        if source.kind != BootNodeKind::File {
            return Err(Errno::Perm);
        }
        if self.find_node(&destination).is_some() {
            return Err(Errno::Exist);
        }
        self.nodes.push(BootNode {
            path: destination,
            kind: source.kind,
            inode: source.inode,
            bytes: source.bytes,
            link_target: source.link_target,
            owner_uid: source.owner_uid,
            group_gid: source.group_gid,
            mode: source.mode,
            minimum_label: source.minimum_label,
            current_label: source.current_label,
            mount_layer: 0,
            mount_id: None,
        });
        self.invalidate_caches();
        Ok(())
    }
}

impl BootEventQueueRegistry {
    const fn new() -> Self {
        Self {
            next_id: 1,
            queues: [None, None, None, None, None, None, None, None],
        }
    }

    fn create_queue(&mut self, mode: NativeEventQueueMode) -> Result<usize, Errno> {
        let Some(slot) = self.queues.iter().position(Option::is_none) else {
            return Err(Errno::Again);
        };
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.queues[slot] = Some(BootEventQueueEntry {
            id,
            mode,
            pending: Vec::new(),
            pending_peak: 0,
            resource_watches: Vec::new(),
            bus_watches: Vec::new(),
            network_watches: Vec::new(),
            vfs_watches: Vec::new(),
        });
        Ok(id)
    }

    fn queue(&self, id: usize) -> Result<&BootEventQueueEntry, Errno> {
        self.queues
            .iter()
            .flatten()
            .find(|entry| entry.id == id)
            .ok_or(Errno::Badf)
    }

    fn queue_mut(&mut self, id: usize) -> Result<&mut BootEventQueueEntry, Errno> {
        self.queues
            .iter_mut()
            .flatten()
            .find(|entry| entry.id == id)
            .ok_or(Errno::Badf)
    }

    fn remove_queue(&mut self, id: usize) {
        if let Some(slot) = self
            .queues
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|queue| queue.id == id))
        {
            self.queues[slot] = None;
        }
    }

    fn push_event(queue: &mut BootEventQueueEntry, event: NativeEventRecord) -> (bool, bool) {
        if let Some(existing) = queue.pending.iter_mut().find(|existing| {
            existing.token == event.token
                && existing.source_kind == event.source_kind
                && existing.source_arg0 == event.source_arg0
                && existing.source_arg1 == event.source_arg1
                && existing.source_arg2 == event.source_arg2
                && existing.detail0 == event.detail0
        }) {
            existing.events |= event.events;
            existing.detail1 = event.detail1;
            return (false, true);
        }
        let overflowed = if queue.pending.len() >= MAX_EVENT_QUEUE_PENDING {
            queue.pending.remove(0);
            true
        } else {
            false
        };
        queue.pending.push(event);
        queue.pending_peak = queue.pending_peak.max(queue.pending.len());
        (overflowed, false)
    }
}

fn boot_vfs_stat(path: &str) -> Option<NativeFileStatusRecord> {
    BOOT_VFS.with_mut(|vfs| {
        let normalized = BootVfs::normalize_path(path).ok()?;
        let include_self_directory = vfs
            .find_node(&normalized)
            .is_some_and(|index| vfs.nodes[index].kind == BootNodeKind::Directory);
        if vfs
            .require_traversal_access(&normalized, include_self_directory)
            .is_err()
            || vfs
                .require_access(&normalized, true, false, include_self_directory)
                .is_err()
        {
            return None;
        }
        vfs.stat(&normalized, true)
    })
}

fn boot_vfs_lstat(path: &str) -> Option<NativeFileStatusRecord> {
    BOOT_VFS.with_mut(|vfs| {
        let normalized = BootVfs::normalize_path(path).ok()?;
        let include_self_directory = vfs
            .find_node(&normalized)
            .is_some_and(|index| vfs.nodes[index].kind == BootNodeKind::Directory);
        if vfs
            .require_traversal_access(&normalized, include_self_directory)
            .is_err()
            || vfs
                .require_access(&normalized, true, false, include_self_directory)
                .is_err()
        {
            return None;
        }
        vfs.stat(&normalized, false)
    })
}

fn boot_vfs_create(path: &str, kind: BootNodeKind) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.create(path, kind))
}

fn boot_vfs_symlink(path: &str, target: &str) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.create_symlink(path, target))
}

fn boot_vfs_file_size(path: &str) -> Result<usize, Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.file_size(path))
}

fn boot_vfs_readlink(path: &str) -> Result<String, Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let path = BootVfs::normalize_path(path)?;
        vfs.require_traversal_access(&path, false)?;
        vfs.require_access(&path, true, false, false)?;
        vfs.readlink(&path).map(String::from)
    })
}

fn boot_vfs_rename(from: &str, to: &str) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.rename(from, to))
}

fn boot_vfs_unlink(path: &str) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.unlink(path))
}

fn boot_vfs_link(source: &str, destination: &str) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.link_file(source, destination))
}

fn boot_vfs_truncate(path: &str, len: usize) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let path = BootVfs::normalize_path(path)?;
        vfs.require_traversal_access(&path, false)?;
        vfs.require_access(&path, false, true, false)?;
        vfs.object_lock_conflict(&path, None)?;
        let index = vfs.resolve_node_index(&path, true)?;
        let node = vfs.nodes.get(index).ok_or(Errno::Badf)?.clone();
        match node.kind {
            BootNodeKind::File | BootNodeKind::Channel => {
                for entry in vfs
                    .nodes
                    .iter_mut()
                    .filter(|entry| entry.inode == node.inode)
                {
                    entry.bytes.resize(len, 0);
                }
                vfs.invalidate_caches();
                Ok(())
            }
            BootNodeKind::Directory => Err(Errno::IsDir),
            BootNodeKind::Symlink => Err(Errno::Inval),
        }
    })
}

fn boot_vfs_list(path: &str) -> Result<String, Errno> {
    if let Some(listing) = boot_procfs_directory_listing(path)? {
        return Ok(listing);
    }
    BOOT_VFS.with_mut(|vfs| {
        let path = BootVfs::normalize_path(path)?;
        vfs.require_traversal_access(&path, true)?;
        vfs.require_access(&path, true, false, true)?;
        vfs.list_directory_text(&path)
    })
}

fn boot_vfs_chmod(path: &str, mode: u32) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.chmod(path, mode))
}

fn boot_vfs_chown(path: &str, owner_uid: u32, group_gid: u32) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.chown(path, owner_uid, group_gid))
}

fn storage_mount_state() -> StorageMountState {
    STORAGE_MOUNT.with_mut(|state| state.clone())
}

fn storage_mount_by_path(path: &str) -> Option<StorageMountRecord> {
    let normalized = BootVfs::normalize_path(path).ok()?;
    STORAGE_MOUNT.with_mut(|state| {
        state
            .mounts
            .iter()
            .find(|record| record.mount_path == normalized)
            .cloned()
    })
}

fn storage_mount_has_nested_child(mount_path: &str, exclude_id: u64) -> bool {
    let nested_prefix = format!("{mount_path}/");
    STORAGE_MOUNT.with_mut(|mounts| {
        mounts
            .mounts
            .iter()
            .any(|record| record.id != exclude_id && record.mount_path.starts_with(&nested_prefix))
    })
}

fn storage_mount_parent(path: &str) -> Option<StorageMountRecord> {
    let normalized = BootVfs::normalize_path(path).ok()?;
    STORAGE_MOUNT.with_mut(|state| {
        state
            .mounts
            .iter()
            .filter(|record| {
                normalized != record.mount_path
                    && normalized.starts_with(&(record.mount_path.clone() + "/"))
            })
            .max_by_key(|record| record.mount_path.len())
            .cloned()
    })
}

fn storage_mount_relative_suffix(
    parent_mount_path: &str,
    child_mount_path: &str,
) -> Option<String> {
    if child_mount_path == parent_mount_path {
        return Some(String::new());
    }
    let prefix = format!("{parent_mount_path}/");
    child_mount_path
        .strip_prefix(&prefix)
        .map(|suffix| format!("/{suffix}"))
}

fn storage_mount_unmount_ids(record: &StorageMountRecord) -> Vec<u64> {
    STORAGE_MOUNT.with_mut(|state| {
        state
            .mounts
            .iter()
            .filter(|candidate| {
                if candidate.id == record.id {
                    return true;
                }
                match NativeMountPropagationMode::from_raw(record.propagation_mode) {
                    Some(NativeMountPropagationMode::Shared) => {
                        (candidate.peer_group != 0 && candidate.peer_group == record.peer_group)
                            || (candidate.master_group != 0
                                && candidate.master_group == record.peer_group)
                    }
                    _ => false,
                }
            })
            .map(|candidate| candidate.id)
            .collect()
    })
}

fn storage_mount_recursive_unmount_ids(record: &StorageMountRecord) -> Vec<u64> {
    let mut active_ids = storage_mount_unmount_ids(record);
    loop {
        let targets = STORAGE_MOUNT.with_mut(|state| {
            state
                .mounts
                .iter()
                .filter(|candidate| active_ids.iter().any(|id| *id == candidate.id))
                .cloned()
                .collect::<Vec<_>>()
        });
        let mut changed = false;
        STORAGE_MOUNT.with_mut(|state| {
            for target in &targets {
                let prefix = format!("{}/", target.mount_path);
                for candidate in &state.mounts {
                    if active_ids.iter().any(|id| *id == candidate.id) {
                        continue;
                    }
                    let propagated = candidate.propagation_mode
                        != NativeMountPropagationMode::Private as u32
                        || candidate.peer_group != 0
                        || candidate.master_group != 0;
                    if propagated && candidate.mount_path.starts_with(&prefix) {
                        active_ids.push(candidate.id);
                        changed = true;
                    }
                }
            }
        });
        if !changed {
            break;
        }
    }
    active_ids.sort_unstable();
    active_ids.dedup();
    active_ids
}

fn storage_mount_has_nested_child_outside(mount_path: &str, active_ids: &[u64]) -> bool {
    let nested_prefix = format!("{mount_path}/");
    STORAGE_MOUNT.with_mut(|state| {
        state.mounts.iter().any(|record| {
            !active_ids.iter().any(|id| *id == record.id)
                && record.mount_path.starts_with(&nested_prefix)
        })
    })
}

fn storage_mount_propagation_clones(
    parent: &StorageMountRecord,
    child_mount_path: &str,
) -> Vec<(String, NativeMountPropagationMode)> {
    let Some(relative_suffix) = storage_mount_relative_suffix(&parent.mount_path, child_mount_path)
    else {
        return Vec::new();
    };
    STORAGE_MOUNT.with_mut(|state| {
        let mut planned = Vec::new();
        for peer in state.mounts.iter() {
            if peer.id == parent.id {
                continue;
            }
            if peer.peer_group == parent.peer_group
                && peer.peer_group != 0
                && peer.propagation_mode == NativeMountPropagationMode::Shared as u32
            {
                planned.push((
                    format!("{}{}", peer.mount_path, relative_suffix),
                    NativeMountPropagationMode::Shared,
                ));
            } else if peer.master_group == parent.peer_group
                && parent.peer_group != 0
                && peer.propagation_mode == NativeMountPropagationMode::Slave as u32
            {
                planned.push((
                    format!("{}{}", peer.mount_path, relative_suffix),
                    NativeMountPropagationMode::Slave,
                ));
            }
        }
        planned
    })
}

fn storage_mount_descendants(root: &StorageMountRecord) -> Vec<StorageMountRecord> {
    let prefix = format!("{}/", root.mount_path);
    STORAGE_MOUNT.with_mut(|state| {
        let mut descendants = state
            .mounts
            .iter()
            .filter(|record| record.id != root.id && record.mount_path.starts_with(&prefix))
            .cloned()
            .collect::<Vec<_>>();
        descendants.sort_by_key(|record| record.mount_path.len());
        descendants
    })
}

fn storage_mount_promote_descendants_to_shared(root: &StorageMountRecord) {
    let prefix = format!("{}/", root.mount_path);
    STORAGE_MOUNT.with_mut(|state| {
        for record in state
            .mounts
            .iter_mut()
            .filter(|record| record.id != root.id && record.mount_path.starts_with(&prefix))
        {
            if record.peer_group == 0 && record.master_group == 0 {
                record.propagation_mode = NativeMountPropagationMode::Shared as u32;
                record.peer_group = record.id;
                record.master_group = 0;
            }
        }
    });
}

fn storage_mount_privatize_descendants(root: &StorageMountRecord) {
    let prefix = format!("{}/", root.mount_path);
    STORAGE_MOUNT.with_mut(|state| {
        for record in state
            .mounts
            .iter_mut()
            .filter(|record| record.id != root.id && record.mount_path.starts_with(&prefix))
        {
            record.propagation_mode = NativeMountPropagationMode::Private as u32;
            record.peer_group = 0;
            record.master_group = 0;
        }
    });
}

fn storage_mount_rebind_descendants_to_slave(
    source_root: &StorageMountRecord,
    target_root: &StorageMountRecord,
) {
    let target_prefix = format!("{}/", target_root.mount_path);
    let target_descendants = STORAGE_MOUNT.with_mut(|state| {
        state
            .mounts
            .iter()
            .filter(|record| {
                record.id != target_root.id && record.mount_path.starts_with(&target_prefix)
            })
            .cloned()
            .collect::<Vec<_>>()
    });
    for target in target_descendants {
        let Some(relative_suffix) =
            storage_mount_relative_suffix(&target_root.mount_path, &target.mount_path)
        else {
            continue;
        };
        let source_path = format!("{}{}", source_root.mount_path, relative_suffix);
        let source = storage_mount_by_path(&source_path);
        STORAGE_MOUNT.with_mut(|state| {
            let Some(record) = state
                .mounts
                .iter_mut()
                .find(|record| record.id == target.id)
            else {
                return;
            };
            record.propagation_mode = NativeMountPropagationMode::Slave as u32;
            record.peer_group = 0;
            record.master_group = source
                .as_ref()
                .map(|record| record.peer_group.max(record.master_group).max(record.id))
                .unwrap_or(0);
        });
    }
}

fn storage_mount_clone_existing_descendants(
    source_root: &StorageMountRecord,
    target_root: &StorageMountRecord,
) -> Result<(), Errno> {
    let descendants = storage_mount_descendants(source_root);
    for source in descendants {
        let Some(relative_suffix) =
            storage_mount_relative_suffix(&source_root.mount_path, &source.mount_path)
        else {
            continue;
        };
        let clone_path = format!("{}{}", target_root.mount_path, relative_suffix);
        if storage_mount_by_path(&clone_path).is_some() {
            continue;
        }
        let entries =
            BOOT_VFS.with_mut(|vfs| collect_persist_entries(vfs, source.id, &source.mount_path))?;
        let clone_mode = match NativeMountPropagationMode::from_raw(target_root.propagation_mode) {
            Some(NativeMountPropagationMode::Slave) => NativeMountPropagationMode::Slave,
            _ => NativeMountPropagationMode::Shared,
        };
        let clone_id = STORAGE_MOUNT.with_mut(|state| {
            let id = state.next_id.max(1);
            state.next_id = id.saturating_add(1);
            id
        });
        let (loaded, created_mount_root) =
            BOOT_VFS.with_mut(|vfs| apply_persist_entries(vfs, clone_id, &clone_path, &entries))?;
        let parent_mount_id = storage_mount_parent(&clone_path)
            .map(|record| record.id)
            .unwrap_or(target_root.id);
        let (peer_group, master_group) = match clone_mode {
            NativeMountPropagationMode::Shared => (source.peer_group.max(source.id), 0),
            NativeMountPropagationMode::Slave => {
                (0, source.peer_group.max(source.master_group).max(source.id))
            }
            NativeMountPropagationMode::Private => (0, 0),
        };
        STORAGE_MOUNT.with_mut(|state| {
            state.mounts.push(StorageMountRecord {
                id: clone_id,
                device_path: source.device_path.clone(),
                mount_path: clone_path,
                parent_mount_id,
                peer_group,
                master_group,
                propagation_mode: clone_mode as u32,

                entry_count: loaded,
                created_mount_root,
            });
        });
    }
    Ok(())
}

fn collect_persist_entries(
    vfs: &BootVfs,
    mount_id: u64,
    mount_path: &str,
) -> Result<Vec<crate::virtio_blk_boot::StorageSnapshotEntry>, Errno> {
    let prefix = format!("{mount_path}/");
    let mut entries = vfs
        .nodes
        .iter()
        .filter(|node| {
            node.mount_id == Some(mount_id)
                && node.path != mount_path
                && matches!(
                    node.kind,
                    BootNodeKind::Directory | BootNodeKind::File | BootNodeKind::Symlink
                )
                && node.path.starts_with(&prefix)
        })
        .map(|node| crate::virtio_blk_boot::StorageSnapshotEntry {
            name: String::from(&node.path[prefix.len()..]),
            kind: match node.kind {
                BootNodeKind::Directory => crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_DIRECTORY,
                BootNodeKind::File => crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_FILE,
                BootNodeKind::Symlink => crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_SYMLINK,
                BootNodeKind::Channel => unreachable!(),
            },
            bytes: match node.kind {
                BootNodeKind::Directory => Vec::new(),
                BootNodeKind::File => node.bytes.clone(),
                BootNodeKind::Symlink => node.link_target.clone().unwrap_or_default().into_bytes(),
                BootNodeKind::Channel => Vec::new(),
            },
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.name
            .split('/')
            .count()
            .cmp(&right.name.split('/').count())
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

fn apply_persist_entries(
    vfs: &mut BootVfs,
    mount_id: u64,
    mount_path: &str,
    entries: &[crate::virtio_blk_boot::StorageSnapshotEntry],
) -> Result<(usize, bool), Errno> {
    let mount_path = BootVfs::normalize_path(mount_path)?;
    let created_mount_root = if vfs.find_node(&mount_path).is_none() {
        vfs.create(&mount_path, BootNodeKind::Directory)?;
        true
    } else {
        false
    };
    if vfs.find_node(&mount_path).is_none() {
        vfs.create(&mount_path, BootNodeKind::Directory)?;
    }
    vfs.nodes.retain(|node| node.mount_id != Some(mount_id));
    if !vfs
        .nodes
        .iter()
        .any(|node| node.path == mount_path && node.mount_id == Some(mount_id))
    {
        vfs.create_with_mount(&mount_path, BootNodeKind::Directory, Some(mount_id))?;
    }
    let mut loaded = 0usize;
    for entry in entries {
        let path = format!("{mount_path}/{}", entry.name);
        match entry.kind {
            crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_DIRECTORY => {
                vfs.create_with_mount(&path, BootNodeKind::Directory, Some(mount_id))?;
            }
            crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_FILE => {
                vfs.create_with_mount(&path, BootNodeKind::File, Some(mount_id))?;
                let index = vfs.resolve_node_index(&path, true)?;
                vfs.nodes[index].bytes.extend_from_slice(&entry.bytes);
            }
            crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_SYMLINK => {
                let target = core::str::from_utf8(&entry.bytes).map_err(|_| Errno::Inval)?;
                vfs.create_symlink_with_mount(&path, target, Some(mount_id))?;
            }
            _ => return Err(Errno::Inval),
        }
        loaded += 1;
    }
    Ok((loaded, created_mount_root))
}

fn boot_vfs_lookup_target(path: &str) -> Result<DescriptorTarget, Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let path = BootVfs::normalize_path(path)?;
        vfs.require_traversal_access(&path, false)?;
        let index = vfs.resolve_node_index(&path, true)?;
        match vfs.nodes[index].kind {
            BootNodeKind::Directory => {
                vfs.require_access(&path, false, false, true)?;
                Ok(DescriptorTarget::BootDirectory(vfs.nodes[index].inode))
            }
            BootNodeKind::File => {
                vfs.require_access(&path, true, false, false)?;
                Ok(DescriptorTarget::BootFile(vfs.nodes[index].inode))
            }
            BootNodeKind::Channel => {
                vfs.require_access(&path, true, false, false)?;
                Ok(DescriptorTarget::BootChannel(vfs.nodes[index].inode))
            }
            BootNodeKind::Symlink => Err(Errno::Inval),
        }
    })
}

fn descriptor_target_kind_name(target: DescriptorTarget) -> &'static str {
    match target {
        DescriptorTarget::Stdin
        | DescriptorTarget::Stdout
        | DescriptorTarget::Stderr
        | DescriptorTarget::BootFile(_) => "File",
        DescriptorTarget::Procfs(node) => match node.kind {
            BootProcfsNodeKind::ProcRootDir
            | BootProcfsNodeKind::ProcessDir
            | BootProcfsNodeKind::FdDirListing
            | BootProcfsNodeKind::FdInfoDirListing => "Directory",
            _ => "File",
        },
        DescriptorTarget::BootDirectory(_) => "Directory",
        DescriptorTarget::BootChannel(_) => "Channel",
        DescriptorTarget::EventQueue(_) => "EventQueue",
        DescriptorTarget::GpuDevice
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::InputDevice
        | DescriptorTarget::NetworkDevice
        | DescriptorTarget::StorageDevice => "Device",
        DescriptorTarget::GpuDriver
        | DescriptorTarget::AudioDriver
        | DescriptorTarget::InputDriver
        | DescriptorTarget::NetworkDriver
        | DescriptorTarget::StorageDriver => "Driver",
    }
}

fn descriptor_target_path_text(target: DescriptorTarget) -> Result<String, Errno> {
    match target {
        DescriptorTarget::Stdin => Ok(String::from("stdin")),
        DescriptorTarget::Stdout => Ok(String::from("stdout")),
        DescriptorTarget::Stderr => Ok(String::from("stderr")),
        DescriptorTarget::EventQueue(id) => Ok(format!("event-queue:{id}")),
        DescriptorTarget::GpuDevice => Ok(String::from(GPU_DEVICE_PATH)),
        DescriptorTarget::GpuDriver => Ok(String::from(GPU_DRIVER_PATH)),
        DescriptorTarget::AudioDevice => Ok(String::from(AUDIO_DEVICE_PATH)),
        DescriptorTarget::AudioDriver => Ok(String::from(AUDIO_DRIVER_PATH)),
        DescriptorTarget::InputDevice => Ok(String::from(INPUT_DEVICE_PATH)),
        DescriptorTarget::InputDriver => Ok(String::from(INPUT_DRIVER_PATH)),
        DescriptorTarget::NetworkDevice => Ok(String::from(NETWORK_DEVICE_PATH)),
        DescriptorTarget::NetworkDriver => Ok(String::from(NETWORK_DRIVER_PATH)),
        DescriptorTarget::StorageDevice => Ok(String::from("/dev/storage0")),
        DescriptorTarget::StorageDriver => Ok(String::from("/drv/storage0")),
        DescriptorTarget::BootDirectory(inode)
        | DescriptorTarget::BootFile(inode)
        | DescriptorTarget::BootChannel(inode) => BOOT_VFS.with_mut(|vfs| {
            Ok(vfs
                .live_path_for_inode(inode)
                .unwrap_or_else(|| format!("inode:{inode:016x} (deleted)")))
        }),
        DescriptorTarget::Procfs(node) => Ok(match node.kind {
            BootProcfsNodeKind::ProcRootDir => String::from("/proc"),
            BootProcfsNodeKind::ProcessDir => format!("/proc/{}", node.pid),
            BootProcfsNodeKind::SystemDir => String::from("/proc/system"),
            BootProcfsNodeKind::FdDirListing => format!("/proc/{}/fd", node.pid),
            BootProcfsNodeKind::FdInfoDirListing => format!("/proc/{}/fdinfo", node.pid),
            BootProcfsNodeKind::Status => format!("/proc/{}/status", node.pid),
            BootProcfsNodeKind::Root => format!("/proc/{}/root", node.pid),
            BootProcfsNodeKind::Cwd => format!("/proc/{}/cwd", node.pid),
            BootProcfsNodeKind::Exe => format!("/proc/{}/exe", node.pid),
            BootProcfsNodeKind::Cmdline => format!("/proc/{}/cmdline", node.pid),
            BootProcfsNodeKind::Environ => format!("/proc/{}/environ", node.pid),
            BootProcfsNodeKind::Auxv => format!("/proc/{}/auxv", node.pid),
            BootProcfsNodeKind::Mounts => format!("/proc/{}/mounts", node.pid),
            BootProcfsNodeKind::Fd => format!("/proc/{}/fd", node.pid),
            BootProcfsNodeKind::Caps => format!("/proc/{}/caps", node.pid),
            BootProcfsNodeKind::FdInfo(fd) => format!("/proc/{}/fdinfo/{}", node.pid, fd),
            BootProcfsNodeKind::VfsLocks => format!("/proc/{}/vfslocks", node.pid),
            BootProcfsNodeKind::VfsWatches => format!("/proc/{}/vfswatches", node.pid),
            BootProcfsNodeKind::VfsStats => format!("/proc/{}/vfsstats", node.pid),
            BootProcfsNodeKind::Queues => format!("/proc/{}/queues", node.pid),
            BootProcfsNodeKind::Maps => format!("/proc/{}/maps", node.pid),
            BootProcfsNodeKind::VmObjects => format!("/proc/{}/vmobjects", node.pid),
            BootProcfsNodeKind::VmDecisions => format!("/proc/{}/vmdecisions", node.pid),
            BootProcfsNodeKind::VmEpisodes => format!("/proc/{}/vmepisodes", node.pid),
            BootProcfsNodeKind::SystemScheduler => String::from("/proc/system/scheduler"),
            BootProcfsNodeKind::SystemSchedulerEpisodes => {
                String::from("/proc/system/schedulerepisodes")
            }
            BootProcfsNodeKind::SystemBus => String::from("/proc/system/bus"),
        }),
    }
}

const fn descriptor_default_rights(target: DescriptorTarget) -> BlockRightsMask {
    match target {
        DescriptorTarget::Stdin => BlockRightsMask::READ,
        DescriptorTarget::Stdout | DescriptorTarget::Stderr => BlockRightsMask::WRITE,
        DescriptorTarget::EventQueue(_) => BlockRightsMask::READ
            .union(BlockRightsMask::WRITE)
            .union(BlockRightsMask::DELEGATE),
        DescriptorTarget::Procfs(_) => BlockRightsMask::READ,
        DescriptorTarget::BootDirectory(_)
        | DescriptorTarget::BootFile(_)
        | DescriptorTarget::BootChannel(_)
        | DescriptorTarget::GpuDevice
        | DescriptorTarget::GpuDriver
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::AudioDriver
        | DescriptorTarget::InputDevice
        | DescriptorTarget::InputDriver
        | DescriptorTarget::NetworkDevice
        | DescriptorTarget::NetworkDriver
        | DescriptorTarget::StorageDevice
        | DescriptorTarget::StorageDriver => BlockRightsMask::READ
            .union(BlockRightsMask::WRITE)
            .union(BlockRightsMask::DELEGATE),
    }
}

fn boot_procfs_fd_listing(pid: u64) -> Result<String, Errno> {
    require_procfs_access(BootProcfsNode {
        pid,
        kind: BootProcfsNodeKind::Fd,
    })?;
    if pid != 1 {
        return BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid) else {
                return Err(Errno::Srch);
            };
            let mut text = String::new();
            for descriptor in &registry.entries[index].descriptors {
                text.push_str(&format!(
                    "{}\t{}\t{}\tcloexec={}\tnonblock={}\n",
                    descriptor.fd,
                    descriptor.path,
                    descriptor.kind_name,
                    descriptor.cloexec,
                    descriptor.nonblock,
                ));
            }
            Ok(text)
        });
    }
    DESCRIPTORS.with(|table| {
        let mut text = String::new();
        for (fd, descriptor) in table.slots.iter().enumerate() {
            let Some(_) = descriptor else {
                continue;
            };
            let snapshot = table.descriptor(fd)?;
            let path = descriptor_target_path_text(snapshot.target)?;
            text.push_str(&format!(
                "{}\t{}\t{}\tcloexec={}\tnonblock={}\n",
                fd,
                path,
                descriptor_target_kind_name(snapshot.target),
                snapshot.cloexec,
                snapshot.nonblock,
            ));
        }
        Ok(text)
    })
}

fn boot_procfs_descriptor_record_text(fd: u64, descriptor: &BootProcessDescriptorRecord) -> String {
    format!(
        "fd:\t{fd}\npath:\t{}\nkind:\t{}\npos:\t{}\nflags:\tcloexec={} nonblock={}\nrights:\t0x{:x}\n",
        descriptor.path,
        descriptor.kind_name,
        descriptor.pos,
        descriptor.cloexec,
        descriptor.nonblock,
        descriptor.rights,
    )
}

fn boot_procfs_fdinfo(pid: u64, fd: u64) -> Result<String, Errno> {
    require_procfs_access(BootProcfsNode {
        pid,
        kind: BootProcfsNodeKind::FdInfo(fd),
    })?;
    if pid != 1 {
        return BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid) else {
                return Err(Errno::Srch);
            };
            let Some(descriptor) = registry.entries[index]
                .descriptors
                .iter()
                .find(|descriptor| descriptor.fd == fd)
            else {
                return Err(Errno::NoEnt);
            };
            Ok(boot_procfs_descriptor_record_text(fd, descriptor))
        });
    }
    DESCRIPTORS.with(|table| {
        let descriptor = match table.descriptor(fd as usize) {
            Ok(descriptor) => descriptor,
            Err(Errno::Badf) => return Err(Errno::NoEnt),
            Err(error) => return Err(error),
        };
        let path = descriptor_target_path_text(descriptor.target)?;
        Ok(format!(
            "fd:\t{fd}\npath:\t{path}\nkind:\t{}\npos:\t{}\nflags:\tcloexec={} nonblock={}\nrights:\t0x{:x}\n",
            descriptor_target_kind_name(descriptor.target),
            descriptor.offset,
            descriptor.cloexec,
            descriptor.nonblock,
            descriptor.rights.0,
        ))
    })
}

fn boot_process_capability_names(
    entry: &BootProcessEntry,
    descriptor_count: usize,
) -> Vec<&'static str> {
    let mut names = Vec::new();
    if descriptor_count != 0 {
        names.push("descriptor-table");
    }
    if !entry.vm_objects.is_empty() {
        names.push("vm-space");
    }
    if entry.contract_bindings.execution != 0 {
        names.push("execution-contract");
    }
    if entry.contract_bindings.memory != 0 {
        names.push("memory-contract");
    }
    if entry.contract_bindings.io != 0 {
        names.push("io-contract");
    }
    if entry.contract_bindings.observe != 0 {
        names.push("observe-contract");
    }
    names
}

fn boot_procfs_caps(pid: u64) -> Result<String, Errno> {
    require_procfs_access(BootProcfsNode {
        pid,
        kind: BootProcfsNodeKind::Caps,
    })?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        let descriptor_count = if entry.pid == 1 {
            DESCRIPTORS.with(|table| table.slots.iter().flatten().count())
        } else {
            entry.descriptors.len()
        };
        let mut text = boot_process_capability_names(entry, descriptor_count).join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        Ok(text)
    })
}

fn boot_procfs_auxv(pid: u64) -> Result<String, Errno> {
    require_procfs_access(BootProcfsNode {
        pid,
        kind: BootProcfsNodeKind::Auxv,
    })?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        let execfn = if entry.image_path.is_empty() {
            "/bin/ngos-userland-native"
        } else {
            entry.image_path.as_str()
        };
        let image_base = entry
            .envp
            .iter()
            .find_map(|record| record.strip_prefix("NGOS_IMAGE_BASE="))
            .unwrap_or("0x400000");
        Ok(format!(
            "AT_PAGESZ=4096\nAT_ENTRY={image_base}\nAT_EXECFN={execfn}\n"
        ))
    })
}

fn mount_propagation_name(mode: u32) -> &'static str {
    match NativeMountPropagationMode::from_raw(mode) {
        Some(NativeMountPropagationMode::Private) => "private",
        Some(NativeMountPropagationMode::Shared) => "shared",
        Some(NativeMountPropagationMode::Slave) => "slave",
        None => "unknown",
    }
}

fn boot_procfs_mounts(pid: u64) -> Result<String, Errno> {
    require_procfs_access(BootProcfsNode {
        pid,
        kind: BootProcfsNodeKind::Mounts,
    })?;
    let mut lines = vec![String::from(
        "/\tdevice=rootfs\tmode=private\tpeer_group=0\tmaster_group=0\tcreated_root=yes",
    )];
    let mut mounts = storage_mount_state().mounts;
    mounts.sort_by(|left, right| left.mount_path.cmp(&right.mount_path));
    for record in mounts {
        lines.push(format!(
            "{}\tdevice={}\tmode={}\tpeer_group={}\tmaster_group={}\tcreated_root={}",
            record.mount_path,
            record.device_path,
            mount_propagation_name(record.propagation_mode),
            record.peer_group,
            record.master_group,
            if record.created_mount_root {
                "yes"
            } else {
                "no"
            },
        ));
    }
    Ok(lines.join("\n") + "\n")
}

fn boot_procfs_queues(pid: u64) -> Result<String, Errno> {
    require_procfs_access(BootProcfsNode {
        pid,
        kind: BootProcfsNodeKind::Queues,
    })?;

    let descriptor_ref_count = |queue_id: usize| -> Result<usize, Errno> {
        if pid != 1 {
            return BOOT_PROCESSES.with_mut(|registry| {
                let Some(index) = registry.find_index(pid) else {
                    return Err(Errno::Srch);
                };
                Ok(registry.entries[index]
                    .descriptors
                    .iter()
                    .filter(|descriptor| descriptor.path == format!("event-queue:{queue_id}"))
                    .count())
            });
        }
        DESCRIPTORS.with(|table| {
            Ok(table
                .slots
                .iter()
                .flatten()
                .filter(|state| {
                    table
                        .description(state.description_id)
                        .ok()
                        .is_some_and(|description| {
                            description.target == DescriptorTarget::EventQueue(queue_id)
                        })
                })
                .count())
        })
    };

    BOOT_EVENT_QUEUES.with_mut(|registry| {
        let mut text = String::new();
        for queue in registry.queues.iter().flatten() {
            let descriptors = descriptor_ref_count(queue.id)?;
            if pid != 1 && descriptors == 0 {
                continue;
            }
            text.push_str(&format!(
                "event\t{}\t{:?}\twatches={}\ttimers=0\tprocwatches=0\tsigwatches=0\tmemwatches=0\tresourcewatches={}\tpending={}\twaiters=0\tdescriptors={}\tdeferred=0\n",
                queue.id,
                queue.mode,
                queue.network_watches.len() + queue.vfs_watches.len() + queue.resource_watches.len(),
                queue.resource_watches.len(),
                queue.pending.len(),
                descriptors,
            ));
        }
        Ok(text)
    })
}

fn boot_procfs_vfs_locks_text() -> String {
    VFS_LOCKS.with_mut(|locks| {
        let mut text = String::new();
        for lock in locks.iter() {
            let path = BOOT_VFS
                .with_mut(|vfs| vfs.live_path_for_inode(lock.inode))
                .unwrap_or_else(|| String::from("(deleted)"));
            let mode = match lock.mode {
                VfsLockMode::Shared => "shared",
                VfsLockMode::Exclusive => "exclusive",
            };
            text.push_str(&format!(
                "inode={:#x}\tpath={}\towner={}\ttoken={}\tmode={}\n",
                lock.inode, path, lock.owner_fd, lock.token, mode
            ));
        }
        text
    })
}

fn boot_procfs_vfs_watches_text() -> String {
    BOOT_EVENT_QUEUES.with_mut(|registry| {
        let mut text = String::new();
        for queue in registry.queues.iter().flatten() {
            for watch in &queue.vfs_watches {
                let path = BOOT_VFS
                    .with_mut(|vfs| vfs.live_path_for_inode(watch.inode))
                    .or_else(|| watch.anchor_path.clone())
                    .unwrap_or_else(|| String::from("(deleted)"));
                let kind = BOOT_VFS
                    .with_mut(|vfs| vfs.object_kind_by_inode(watch.inode))
                    .map(|kind| match kind {
                        BootNodeKind::Directory => "Directory",
                        BootNodeKind::File => "File",
                        BootNodeKind::Channel => "Channel",
                        BootNodeKind::Symlink => "Symlink",
                    })
                    .unwrap_or("Deleted");
                let mut flags = Vec::new();
                if watch.created {
                    flags.push("created");
                }
                if watch.opened {
                    flags.push("opened");
                }
                if watch.closed {
                    flags.push("closed");
                }
                if watch.written {
                    flags.push("written");
                }
                if watch.renamed {
                    flags.push("renamed");
                }
                if watch.unlinked {
                    flags.push("unlinked");
                }
                if watch.mounted {
                    flags.push("mounted");
                }
                if watch.unmounted {
                    flags.push("unmounted");
                }
                if watch.lock_acquired {
                    flags.push("lock-acquired");
                }
                if watch.lock_refused {
                    flags.push("lock-refused");
                }
                if watch.permission_refused {
                    flags.push("permission-refused");
                }
                if watch.truncated {
                    flags.push("truncated");
                }
                if watch.linked {
                    flags.push("linked");
                }
                let flags = if flags.is_empty() {
                    String::from("(none)")
                } else {
                    flags.join(",")
                };
                let pending = queue.pending.len();
                let peak = queue.pending_peak;
                text.push_str(&format!(
                    "queue={}\towner-pid={}\tinode={:#x}\tkind={}\tsubtree={}\ttoken={}\tpath={}\tpending={}\tpeak={}\tevents={}\n",
                    queue.id, watch.owner_pid, watch.inode, kind, watch.subtree, watch.token, path, pending, peak, flags
                ));
            }
        }
        text
    })
}

fn boot_procfs_vfs_stats_text() -> String {
    let watch_count = BOOT_EVENT_QUEUES.with_mut(|registry| {
        registry
            .queues
            .iter()
            .flatten()
            .map(|queue| queue.vfs_watches.len() as u64)
            .sum::<u64>()
    });
    let lock_count = VFS_LOCKS.with_mut(|locks| locks.len() as u64);
    let mount_count = STORAGE_MOUNT.with_mut(|state| state.mounts.len() as u64);
    format!(
        "{}live: nodes={} orphans={} locks={} watches={} mounts={}\n",
        BOOT_VFS.with_mut(|vfs| vfs.stats_text()),
        BOOT_VFS.with_mut(|vfs| vfs.nodes.len()),
        BOOT_VFS.with_mut(|vfs| vfs.orphan_nodes.len()),
        lock_count,
        watch_count,
        mount_count,
    )
}

fn boot_procfs_system_scheduler_text() -> Result<String, Errno> {
    require_system_observe_contract()?;
    let current_tick = boot_scheduler_tick();
    let class_values = [
        NativeSchedulerClass::LatencyCritical as u32,
        NativeSchedulerClass::Interactive as u32,
        NativeSchedulerClass::BestEffort as u32,
        NativeSchedulerClass::Background as u32,
    ];
    let class_names = [
        "latency-critical",
        "interactive",
        "best-effort",
        "background",
    ];
    let tokens = [8u32, 4, 2, 1];

    BOOT_PROCESSES.with_mut(|registry| {
        let cpu_topology = crate::early_boot_info()
            .map(crate::cpu_handoff::boot_scheduler_cpu_topology_handoff)
            .unwrap_or_else(|| {
                vec![kernel_core::SchedulerCpuTopologyEntry {
                    apic_id: 0,
                    package_id: 0,
                    core_group: 0,
                    sibling_group: 0,
                    inferred: true,
                }]
            });
        let cpu_count = cpu_topology.len().max(1);
        let mut queued_counts = [0usize; 4];
        let mut queued_tids = [String::new(), String::new(), String::new(), String::new()];
        let mut cpu_queued_counts = vec![[0usize; 4]; cpu_count];
        let mut cpu_queued_tids = vec![
            [String::new(), String::new(), String::new(), String::new()];
            cpu_count
        ];
        let mut class_dispatches = [0u64; 4];
        let mut class_runtime_ticks = [0u64; 4];
        let mut cpu_dispatches = vec![0u64; cpu_count];
        let mut cpu_runtime_ticks = vec![0u64; cpu_count];
        let mut cpu_running = vec![false; cpu_count];
        let mut running_pid = 0u64;
        let mut running_class = NativeSchedulerClass::LatencyCritical as u32;
        let mut running_budget = 4u32;
        let mut busy_ticks = 0u64;

        for entry in registry.entries.iter().filter(|entry| !entry.reaped) {
            let scheduler_state = registry.scheduler_state(entry.pid);
            let assigned_cpu = scheduler_state.assigned_cpu.min(cpu_count.saturating_sub(1));
            busy_ticks = busy_ticks.saturating_add(entry.cpu_runtime_ticks);
            if let Some(index) = class_values
                .iter()
                .position(|candidate| *candidate == entry.scheduler_class)
            {
                if entry.state == 1 {
                    queued_counts[index] += 1;
                    if !queued_tids[index].is_empty() {
                        queued_tids[index].push(',');
                    }
                    queued_tids[index].push_str(&entry.pid.to_string());
                    cpu_queued_counts[assigned_cpu][index] += 1;
                    if !cpu_queued_tids[assigned_cpu][index].is_empty() {
                        cpu_queued_tids[assigned_cpu][index].push(',');
                    }
                    cpu_queued_tids[assigned_cpu][index].push_str(&entry.pid.to_string());
                }
                if entry.state == 2 {
                    class_dispatches[index] = class_dispatches[index].saturating_add(1);
                    class_runtime_ticks[index] =
                        class_runtime_ticks[index].saturating_add(entry.cpu_runtime_ticks);
                    cpu_dispatches[assigned_cpu] =
                        cpu_dispatches[assigned_cpu].saturating_add(1);
                    cpu_runtime_ticks[assigned_cpu] =
                        cpu_runtime_ticks[assigned_cpu].saturating_add(entry.cpu_runtime_ticks);
                }
            }
            if entry.state == 2 && running_pid == 0 {
                running_pid = entry.pid;
                running_class = entry.scheduler_class;
                running_budget = entry.scheduler_budget;
                cpu_running[assigned_cpu] = true;
            }
        }

        let queued_total: usize = queued_counts.iter().sum();
        let fairness_dispatch_total = class_dispatches.iter().copied().sum::<u64>();
        let fairness_runtime_total = class_runtime_ticks.iter().copied().sum::<u64>();
        let fairness_runtime_max = class_runtime_ticks.iter().copied().max().unwrap_or(0);
        let fairness_runtime_min = class_runtime_ticks.iter().copied().min().unwrap_or(0);
        let fairness_runtime_imbalance =
            fairness_runtime_max.saturating_sub(fairness_runtime_min);
        let cpu_loads = cpu_queued_counts
            .iter()
            .map(|counts| counts.iter().sum::<usize>())
            .collect::<Vec<_>>();
        let cpu_load_max = cpu_loads.iter().copied().max().unwrap_or(0);
        let cpu_load_min = cpu_loads.iter().copied().min().unwrap_or(0);
        let load_imbalance = cpu_load_max.saturating_sub(cpu_load_min);
        let running_count = cpu_running.iter().copied().filter(|running| *running).count();
        let rebalance_operations = registry.scheduler_events.rebalance_operations;
        let rebalance_migrations = registry.scheduler_events.rebalance_migrations;
        let last_rebalance_migrations = registry.scheduler_events.last_rebalance_migrations;
        let cpu_summary = format!(
            "cpu-summary:\tcount={cpu_count}\trunning={running_count}\tload-imbalance={load_imbalance}\trebalance-ops={rebalance_operations}\trebalance-migrations={rebalance_migrations}\tlast-rebalance={last_rebalance_migrations}\n"
        );
        let mut cpu_lines = String::new();
        let mut cpu_queue_lines = String::new();
        for (cpu_index, cpu) in cpu_topology.iter().enumerate() {
            cpu_lines.push_str(&format!(
                "cpu\tindex={cpu_index}\tapic-id={}\tpackage={}\tcore-group={}\tsibling-group={}\tinferred-topology={}\tqueued-load={cpu_load}\tdispatches={cpu_dispatches}\truntime-ticks={cpu_runtime_ticks}\trunning={cpu_running}\n",
                cpu.apic_id,
                cpu.package_id,
                cpu.core_group,
                cpu.sibling_group,
                cpu.inferred,
                cpu_load = cpu_loads[cpu_index],
                cpu_dispatches = cpu_dispatches[cpu_index],
                cpu_runtime_ticks = cpu_runtime_ticks[cpu_index],
                cpu_running = cpu_running[cpu_index],
            ));
            for class_index in 0..class_names.len() {
                cpu_queue_lines.push_str(&format!(
                    "cpu-queue\tindex={cpu_index}\tclass={}\tcount={count}\ttids=[{tids}]\n",
                    class_names[class_index],
                    count = cpu_queued_counts[cpu_index][class_index],
                    tids = cpu_queued_tids[cpu_index][class_index].as_str(),
                ));
            }
        }
        let mut text = format!(
            "current-tick:\t{current_tick}\n\
busy-ticks:\t{busy_ticks}\n\
default-budget:\t2\n\
decision-tracing:\tenabled\n\
queued-total:\t{queued_total}\n\
queued-latency-critical:\t{}\n\
queued-interactive:\t{}\n\
queued-best-effort:\t{}\n\
queued-background:\t{}\n\
{cpu_summary}\
{cpu_lines}\
{cpu_queue_lines}\
fairness-dispatch-total:\t{fairness_dispatch_total}\n\
fairness-runtime-total:\t{fairness_runtime_total}\n\
fairness-runtime-imbalance:\t{fairness_runtime_imbalance}\n\
running:\tpid={running_pid} tid={running_pid} class={} budget={running_budget}\n",
            queued_counts[0],
            queued_counts[1],
            queued_counts[2],
            queued_counts[3],
            boot_scheduler_class_name(running_class),
        );

        for index in 0..class_names.len() {
            text.push_str(&format!(
                "queue\tclass={}\tcount={}\ttokens={}\twait-ticks=0\tlag-debt=0\tdispatches={}\truntime-ticks={}\ttids=[{}]\n",
                class_names[index],
                queued_counts[index],
                tokens[index],
                class_dispatches[index],
                class_runtime_ticks[index],
                queued_tids[index]
            ));
        }
        if registry.scheduler_events.last_affinity_pid != 0 {
            text.push_str(&format!(
                "decision\ttick={current_tick}\tagent=AffinityAgent\tmeaning=affinity cpu-mask=0x{:x} assigned-cpu={}\tselected={}\tclass={}\n",
                registry.scheduler_events.last_affinity_mask,
                registry.scheduler_events.last_affinity_cpu,
                registry.scheduler_events.last_affinity_pid,
                boot_scheduler_class_name(running_class)
            ));
        }
        if registry.scheduler_events.last_rebalance_pid != 0 {
            text.push_str(&format!(
                "decision\ttick={current_tick}\tagent=RebalanceAgent\tmeaning=rebalance queued-moved from-cpu={} to-cpu={}\tselected={}\tclass={}\n",
                registry.scheduler_events.last_rebalance_from_cpu,
                registry.scheduler_events.last_rebalance_to_cpu,
                registry.scheduler_events.last_rebalance_pid,
                boot_scheduler_class_name(running_class)
            ));
        }
        text.push_str(&format!(
            "decision\ttick={current_tick}\tagent=boot-scheduler\tselected={running_pid}\tclass={}\n",
            boot_scheduler_class_name(running_class)
        ));
        Ok(text)
    })
}

fn boot_procfs_system_schedulerepisodes_text() -> Result<String, Errno> {
    require_system_observe_contract()?;
    let current_tick = boot_scheduler_tick();
    BOOT_PROCESSES.with_mut(|registry| {
        let mut lines = Vec::<String>::new();
        let mut affinity_pid = registry.scheduler_events.last_affinity_pid;
        let mut affinity_tid = registry.scheduler_events.last_affinity_pid;
        let mut affinity_class = NativeSchedulerClass::Interactive as u32;
        let mut affinity_budget = registry.scheduler_events.last_affinity_mask.max(1) as u32;
        let mut dispatch_pid = 0u64;
        let mut dispatch_tid = 0u64;
        let mut dispatch_class = NativeSchedulerClass::LatencyCritical as u32;
        let mut dispatch_budget = 1u32;

        for entry in registry.entries.iter().filter(|entry| !entry.reaped) {
            if affinity_pid == entry.pid {
                affinity_class = entry.scheduler_class;
            }
            if dispatch_pid == 0 && entry.state == 2 {
                dispatch_pid = entry.pid;
                dispatch_tid = entry.pid;
                dispatch_class = entry.scheduler_class;
                dispatch_budget = entry.scheduler_budget.max(1);
            }
        }

        if registry.scheduler_events.last_rebalance_pid != 0 {
            let rebalance_pid = registry.scheduler_events.last_rebalance_pid;
            let rebalance_class = registry
                .entries
                .iter()
                .find(|entry| !entry.reaped && entry.pid == rebalance_pid)
                .map(|entry| entry.scheduler_class)
                .unwrap_or(NativeSchedulerClass::Interactive as u32);
            let rebalance_budget = registry
                .entries
                .iter()
                .find(|entry| !entry.reaped && entry.pid == rebalance_pid)
                .map(|entry| entry.scheduler_budget.max(1))
                .unwrap_or(1);
            lines.push(format!(
                "episode\tkind=rebalance\ttick={current_tick}\tpid={rebalance_pid}\ttid={rebalance_pid}\tclass={rebalance_class}\tbudget={rebalance_budget}\tcausal=queued-moved from-cpu={} to-cpu={}",
                registry.scheduler_events.last_rebalance_from_cpu,
                registry.scheduler_events.last_rebalance_to_cpu,
            ));
        }
        if affinity_pid == 0 {
            if let Some(entry) = registry.entries.iter().find(|entry| !entry.reaped) {
                affinity_pid = entry.pid;
                affinity_tid = entry.pid;
                affinity_class = entry.scheduler_class;
                affinity_budget = registry.scheduler_state(entry.pid).affinity_mask.max(1) as u32;
            }
        }
        if dispatch_pid == 0 {
            if let Some(entry) = registry.entries.iter().find(|entry| !entry.reaped) {
                dispatch_pid = entry.pid;
                dispatch_tid = entry.pid;
                dispatch_class = entry.scheduler_class;
                dispatch_budget = entry.scheduler_budget.max(1);
            }
        }

        lines.push(format!(
            "episode\tkind=affinity\ttick={current_tick}\tpid={affinity_pid}\ttid={affinity_tid}\tclass={affinity_class}\tbudget={affinity_budget}\tcausal=cpu-mask-updated"
        ));
        lines.push(format!(
            "episode\tkind=dispatch\ttick={current_tick}\tpid={dispatch_pid}\ttid={dispatch_tid}\tclass={dispatch_class}\tbudget={dispatch_budget}\tcausal=selected-next-runnable"
        ));
        Ok(format!("episodes:\t{}\n{}\n", lines.len(), lines.join("\n")))
    })
}

fn boot_procfs_system_bus_text() -> Result<String, Errno> {
    require_system_observe_contract()?;
    let (peer_count, endpoint_count, attached_total, publish_total, receive_total, byte_total, queue_depth_total, last_peer, last_endpoint, lines) =
        BOOT_BUS.with(|registry| {
            let peer_count = registry.peers.iter().flatten().count() as u64;
            let endpoint_count = registry.endpoints.iter().flatten().count() as u64;
            let attached_total = registry
                .endpoints
                .iter()
                .flatten()
                .map(|entry| entry.attached_peers.len() as u64)
                .sum::<u64>();
            let publish_total = registry
                .endpoints
                .iter()
                .flatten()
                .map(|entry| entry.publish_count)
                .sum::<u64>();
            let receive_total = registry
                .endpoints
                .iter()
                .flatten()
                .map(|entry| entry.receive_count)
                .sum::<u64>();
            let byte_total = registry
                .endpoints
                .iter()
                .flatten()
                .map(|entry| entry.byte_count)
                .sum::<u64>();
            let queue_depth_total = registry
                .endpoints
                .iter()
                .flatten()
                .map(|entry| entry.queue.len() as u64)
                .sum::<u64>();
            let last_peer = registry
                .endpoints
                .iter()
                .flatten()
                .rev()
                .find_map(|entry| (entry.last_peer != 0).then_some(entry.last_peer))
                .unwrap_or(0);
            let last_endpoint = registry
                .endpoints
                .iter()
                .flatten()
                .rev()
                .find_map(|entry| (entry.id != 0).then_some(entry.id))
                .unwrap_or(0);
            let mut lines = String::new();
            for peer in registry.peers.iter().flatten() {
                lines.push_str(&format!(
                    "peer\tid={}\towner={}\tdomain={}\tattachments={}\treadable-endpoints={}\twritable-endpoints={}\tpublishes={}\treceives={}\tlast-endpoint={}\n",
                    peer.id,
                    peer.owner,
                    peer.domain,
                    peer.attached_endpoint_count,
                    peer.readable_endpoint_count,
                    peer.writable_endpoint_count,
                    peer.publish_count,
                    peer.receive_count,
                    peer.last_endpoint
                ));
            }
            for endpoint in registry.endpoints.iter().flatten() {
                let readable_peer_count = endpoint
                    .attached_peers
                    .iter()
                    .filter(|attachment| {
                        bus_attachment_contains(attachment.rights, BlockRightsMask::READ)
                    })
                    .count();
                let writable_peer_count = endpoint
                    .attached_peers
                    .iter()
                    .filter(|attachment| {
                        bus_attachment_contains(attachment.rights, BlockRightsMask::WRITE)
                    })
                    .count();
                lines.push_str(&format!(
                    "endpoint\tid={}\tdomain={}\tresource={}\tkind=channel\tpath={}\tattached={}\treaders={}\twriters={}\tqueue-depth={}\tqueue-capacity={}\tqueue-peak={}\toverflows={}\tpublishes={}\treceives={}\tbytes={}\tlast-peer={}\n",
                    endpoint.id,
                    endpoint.domain,
                    endpoint.resource,
                    endpoint.path,
                    endpoint.attached_peers.len(),
                    readable_peer_count,
                    writable_peer_count,
                    endpoint.queue.len(),
                    BUS_ENDPOINT_QUEUE_CAPACITY,
                    endpoint.peak_queue_depth,
                    endpoint.overflow_count,
                    endpoint.publish_count,
                    endpoint.receive_count,
                    endpoint.byte_count,
                    endpoint.last_peer
                ));
            }
            (
                peer_count,
                endpoint_count,
                attached_total,
                publish_total,
                receive_total,
                byte_total,
                queue_depth_total,
                last_peer,
                last_endpoint,
                lines,
            )
        });
    Ok(format!(
        "bus-peers:\t{peer_count}\nbus-endpoints:\t{endpoint_count}\nattached-peers:\t{attached_total}\npublishes:\t{publish_total}\nreceives:\t{receive_total}\nbytes:\t{byte_total}\nqueue-depth:\t{queue_depth_total}\nlast-peer:\t{last_peer}\nlast-endpoint:\t{last_endpoint}\n{lines}"
    ))
}

#[cfg(target_os = "none")]
fn boot_scheduler_tick() -> u64 {
    crate::timer::boot_uptime_micros().unwrap_or(0) / 10_000
}

#[cfg(not(target_os = "none"))]
fn boot_scheduler_tick() -> u64 {
    0
}

fn boot_scheduler_cpu_count() -> usize {
    crate::early_boot_info()
        .map(crate::cpu_handoff::boot_scheduler_cpu_topology_handoff)
        .map(|entries| entries.len().max(1))
        .unwrap_or(1)
}

fn boot_scheduler_online_mask() -> u64 {
    let cpu_count = boot_scheduler_cpu_count().min(u64::BITS as usize);
    if cpu_count >= u64::BITS as usize {
        u64::MAX
    } else {
        (1u64 << cpu_count) - 1
    }
}

fn boot_scheduler_default_cpu(pid: u64) -> usize {
    let _ = pid;
    0
}

fn boot_scheduler_pick_cpu(affinity_mask: u64, previous_cpu: usize) -> usize {
    if affinity_mask == 0 {
        return 0;
    }
    if previous_cpu < u64::BITS as usize && (affinity_mask & (1u64 << previous_cpu)) != 0 {
        return previous_cpu;
    }
    affinity_mask.trailing_zeros() as usize
}

fn procfs_kind_is_sensitive(kind: BootProcfsNodeKind) -> bool {
    matches!(
        kind,
        BootProcfsNodeKind::Environ
            | BootProcfsNodeKind::Fd
            | BootProcfsNodeKind::FdDirListing
            | BootProcfsNodeKind::FdInfoDirListing
            | BootProcfsNodeKind::FdInfo(_)
            | BootProcfsNodeKind::VfsLocks
            | BootProcfsNodeKind::VfsWatches
            | BootProcfsNodeKind::VfsStats
            | BootProcfsNodeKind::Queues
            | BootProcfsNodeKind::Maps
            | BootProcfsNodeKind::VmObjects
            | BootProcfsNodeKind::VmDecisions
            | BootProcfsNodeKind::VmEpisodes
            | BootProcfsNodeKind::SystemSchedulerEpisodes
            | BootProcfsNodeKind::SystemBus
    )
}

fn procfs_kind_allows_observe_contract(kind: BootProcfsNodeKind) -> bool {
    matches!(
        kind,
        BootProcfsNodeKind::Maps
            | BootProcfsNodeKind::VmObjects
            | BootProcfsNodeKind::VmDecisions
            | BootProcfsNodeKind::VmEpisodes
            | BootProcfsNodeKind::SystemSchedulerEpisodes
    )
}

fn procfs_kind_is_global(kind: BootProcfsNodeKind) -> bool {
    matches!(
        kind,
        BootProcfsNodeKind::VfsLocks
            | BootProcfsNodeKind::VfsWatches
            | BootProcfsNodeKind::VfsStats
            | BootProcfsNodeKind::Mounts
            | BootProcfsNodeKind::SystemScheduler
            | BootProcfsNodeKind::SystemSchedulerEpisodes
            | BootProcfsNodeKind::SystemBus
    )
}

fn boot_scheduler_class_name(class: u32) -> &'static str {
    match NativeSchedulerClass::from_raw(class) {
        Some(NativeSchedulerClass::LatencyCritical) => "latency-critical",
        Some(NativeSchedulerClass::Interactive) => "interactive",
        Some(NativeSchedulerClass::BestEffort) => "best-effort",
        Some(NativeSchedulerClass::Background) => "background",
        None => "unknown",
    }
}

fn require_system_observe_contract() -> Result<(), Errno> {
    let requester_pid = active_process_pid()?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(requester_pid) else {
            return Err(Errno::Srch);
        };
        if registry.entries[index].contract_bindings.observe == 0 {
            return Err(Errno::Access);
        }
        Ok(())
    })
}

fn require_process_inspect_target(pid: usize, sensitive: bool) -> Result<(), Errno> {
    let requester_pid = active_process_pid()?;
    let requester_uid = BootVfs::current_subject().0;
    let requester_label = BootVfs::current_subject_label();
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let target = &registry.entries[index];
        if requester_uid == 0 || requester_pid == target.pid {
            return Ok(());
        }
        if sensitive {
            return Err(Errno::Access);
        }
        if requester_uid == target.uid {
            return Ok(());
        }
        if check_ifc_read(requester_label, target.subject_label).is_err() {
            return Err(Errno::Access);
        }
        Ok(())
    })
}

fn process_visible_to_requester(
    requester_pid: u64,
    requester_uid: u32,
    requester_label: SecurityLabel,
    entry: &BootProcessEntry,
    sensitive: bool,
) -> bool {
    if requester_uid == 0 || requester_pid == entry.pid {
        return true;
    }
    if sensitive {
        return false;
    }
    if requester_uid == entry.uid {
        return true;
    }
    check_ifc_read(requester_label, entry.subject_label).is_ok()
}

fn process_subject_label(pid: u64) -> Option<SecurityLabel> {
    BOOT_PROCESSES.with_mut(|registry| {
        registry
            .find_index(pid)
            .map(|index| registry.entries[index].subject_label)
    })
}

fn require_procfs_access(node: BootProcfsNode) -> Result<(), Errno> {
    let requester_pid = active_process_pid()?;
    let requester_uid = BootVfs::current_subject().0;
    let requester_label = BootVfs::current_subject_label();
    let requester_has_observe_contract = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(requester_pid) else {
            return Err(Errno::Srch);
        };
        Ok(registry.entries[index].contract_bindings.observe != 0)
    })?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(node.pid) else {
            return Err(Errno::Srch);
        };
        let target = &registry.entries[index];
        if requester_uid == 0 || requester_pid == node.pid {
            return Ok(());
        }
        if procfs_kind_is_sensitive(node.kind) {
            if !procfs_kind_allows_observe_contract(node.kind) || !requester_has_observe_contract {
                return Err(Errno::Access);
            }
        }
        if check_ifc_read(requester_label, target.subject_label).is_err() {
            return Err(Errno::Access);
        }
        Ok(())
    })
}

fn boot_procfs_node(path: &str) -> Result<Option<BootProcfsNode>, Errno> {
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    if segments.next() != Some("proc") {
        return Ok(None);
    }
    let Some(pid_segment) = segments.next() else {
        return Ok(None);
    };
    if pid_segment == "system" {
        let Some(kind_segment) = segments.next() else {
            return Ok(None);
        };
        let kind = match kind_segment {
            "scheduler" => BootProcfsNodeKind::SystemScheduler,
            "schedulerepisodes" => BootProcfsNodeKind::SystemSchedulerEpisodes,
            "bus" => BootProcfsNodeKind::SystemBus,
            _ => return Err(Errno::NoEnt),
        };
        if segments.next().is_some() {
            return Err(Errno::NoEnt);
        }
        return Ok(Some(BootProcfsNode { pid: 1, kind }));
    }
    let pid = pid_segment.parse::<u64>().map_err(|_| Errno::Inval)?;
    let Some(kind_segment) = segments.next() else {
        return Ok(None);
    };
    let kind = match kind_segment {
        "status" => BootProcfsNodeKind::Status,
        "root" => BootProcfsNodeKind::Root,
        "cwd" => BootProcfsNodeKind::Cwd,
        "exe" => BootProcfsNodeKind::Exe,
        "cmdline" => BootProcfsNodeKind::Cmdline,
        "environ" => BootProcfsNodeKind::Environ,
        "auxv" => BootProcfsNodeKind::Auxv,
        "mounts" => BootProcfsNodeKind::Mounts,
        "fd" => BootProcfsNodeKind::Fd,
        "caps" => BootProcfsNodeKind::Caps,
        "vfslocks" => BootProcfsNodeKind::VfsLocks,
        "vfswatches" => BootProcfsNodeKind::VfsWatches,
        "vfsstats" => BootProcfsNodeKind::VfsStats,
        "queues" => BootProcfsNodeKind::Queues,
        "fdinfo" => {
            let fd = segments
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or(Errno::Inval)?;
            BootProcfsNodeKind::FdInfo(fd)
        }
        "maps" => BootProcfsNodeKind::Maps,
        "vmobjects" => BootProcfsNodeKind::VmObjects,
        "vmdecisions" => BootProcfsNodeKind::VmDecisions,
        "vmepisodes" => BootProcfsNodeKind::VmEpisodes,
        _ => return Err(Errno::NoEnt),
    };
    if procfs_kind_is_global(kind) && pid != 1 {
        return Err(Errno::NoEnt);
    }
    if segments.next().is_some() {
        return Err(Errno::NoEnt);
    }
    Ok(Some(BootProcfsNode { pid, kind }))
}

fn boot_procfs_directory_node(path: &str) -> Result<Option<BootProcfsNode>, Errno> {
    let path = BootVfs::normalize_path(path)?;
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    if segments.next() != Some("proc") {
        return Ok(None);
    }
    let Some(pid_segment) = segments.next() else {
        return Ok(Some(BootProcfsNode {
            pid: 1,
            kind: BootProcfsNodeKind::ProcRootDir,
        }));
    };
    if pid_segment == "system" {
        if segments.next().is_none() {
            return Ok(Some(BootProcfsNode {
                pid: 1,
                kind: BootProcfsNodeKind::SystemDir,
            }));
        }
        return Ok(None);
    }
    let pid = pid_segment.parse::<u64>().map_err(|_| Errno::Inval)?;
    let Some(child) = segments.next() else {
        return Ok(Some(BootProcfsNode {
            pid,
            kind: BootProcfsNodeKind::ProcessDir,
        }));
    };
    let kind = match child {
        "fd" if segments.next().is_none() => BootProcfsNodeKind::FdDirListing,
        "fdinfo" if segments.next().is_none() => BootProcfsNodeKind::FdInfoDirListing,
        _ => return Ok(None),
    };
    Ok(Some(BootProcfsNode { pid, kind }))
}

fn boot_procfs_directory_listing(path: &str) -> Result<Option<String>, Errno> {
    let path = BootVfs::normalize_path(path)?;
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    if segments.next() != Some("proc") {
        return Ok(None);
    }
    let requester_pid = active_process_pid()?;
    let requester_uid = BootVfs::current_subject().0;
    let requester_label = BootVfs::current_subject_label();
    let Some(pid_segment) = segments.next() else {
        let listing = BOOT_PROCESSES.with_mut(|registry| {
            let mut out = String::new();
            out.push_str("system\tDirectory\n");
            for entry in registry.entries.iter().filter(|entry| !entry.reaped) {
                if !process_visible_to_requester(
                    requester_pid,
                    requester_uid,
                    requester_label,
                    entry,
                    false,
                ) {
                    continue;
                }
                out.push_str(&format!("{}\tDirectory\n", entry.pid));
            }
            Ok(out)
        })?;
        return Ok(Some(listing));
    };
    if pid_segment == "system" {
        require_system_observe_contract()?;
        let Some(child) = segments.next() else {
            return Ok(Some(String::from(
                "scheduler\tFile\nschedulerepisodes\tFile\nbus\tFile\n",
            )));
        };
        if (child == "scheduler" || child == "schedulerepisodes" || child == "bus")
            && segments.next().is_none()
        {
            return Ok(Some(String::new()));
        }
        return Ok(None);
    }
    let pid = pid_segment.parse::<u64>().map_err(|_| Errno::Inval)?;
    let Some(child) = segments.next() else {
        require_process_inspect_target(pid as usize, false)?;
        let mut out = String::new();
        for (name, kind) in [
            ("status", "File"),
            ("root", "File"),
            ("cwd", "File"),
            ("exe", "File"),
            ("cmdline", "File"),
            ("environ", "File"),
            ("auxv", "File"),
            ("fd", "Directory"),
            ("caps", "File"),
            ("fdinfo", "Directory"),
            ("queues", "File"),
            ("maps", "File"),
            ("vmobjects", "File"),
            ("vmdecisions", "File"),
            ("vmepisodes", "File"),
        ] {
            out.push_str(&format!("{name}\t{kind}\n"));
        }
        if pid == 1 {
            out.push_str("mounts\tFile\n");
            out.push_str("vfslocks\tFile\n");
            out.push_str("vfswatches\tFile\n");
            out.push_str("vfsstats\tFile\n");
        }
        return Ok(Some(out));
    };
    let listing = match child {
        "fd" => {
            require_procfs_access(BootProcfsNode {
                pid,
                kind: BootProcfsNodeKind::Fd,
            })?;
            if pid != 1 {
                BOOT_PROCESSES.with_mut(|registry| {
                    let Some(index) = registry.find_index(pid) else {
                        return Err(Errno::Srch);
                    };
                    let mut out = String::new();
                    for descriptor in &registry.entries[index].descriptors {
                        out.push_str(&format!("{}\tFile\n", descriptor.fd));
                    }
                    Ok(out)
                })?
            } else {
                DESCRIPTORS.with(|table| {
                    let mut out = String::new();
                    for (fd, descriptor) in table.slots.iter().enumerate() {
                        if descriptor.is_some() {
                            out.push_str(&format!("{fd}\tFile\n"));
                        }
                    }
                    Ok(out)
                })?
            }
        }
        "fdinfo" => {
            require_procfs_access(BootProcfsNode {
                pid,
                kind: BootProcfsNodeKind::Fd,
            })?;
            if pid != 1 {
                BOOT_PROCESSES.with_mut(|registry| {
                    let Some(index) = registry.find_index(pid) else {
                        return Err(Errno::Srch);
                    };
                    let mut out = String::new();
                    for descriptor in &registry.entries[index].descriptors {
                        out.push_str(&format!("{}\tFile\n", descriptor.fd));
                    }
                    Ok(out)
                })?
            } else {
                DESCRIPTORS.with(|table| {
                    let mut out = String::new();
                    for (fd, descriptor) in table.slots.iter().enumerate() {
                        if descriptor.is_some() {
                            out.push_str(&format!("{fd}\tFile\n"));
                        }
                    }
                    Ok(out)
                })?
            }
        }
        _ => return Ok(None),
    };
    if segments.next().is_some() {
        return Err(Errno::NoEnt);
    }
    Ok(Some(listing))
}

fn boot_procfs_payload(pid: u64, kind: BootProcfsNodeKind) -> Result<String, Errno> {
    match kind {
        BootProcfsNodeKind::ProcRootDir
        | BootProcfsNodeKind::ProcessDir
        | BootProcfsNodeKind::SystemDir
        | BootProcfsNodeKind::FdDirListing
        | BootProcfsNodeKind::FdInfoDirListing => {
            let path = descriptor_target_path_text(DescriptorTarget::Procfs(BootProcfsNode {
                pid,
                kind,
            }))?;
            return boot_procfs_directory_listing(&path)?.ok_or(Errno::NoEnt);
        }
        _ => {}
    }
    match kind {
        BootProcfsNodeKind::Fd => return boot_procfs_fd_listing(pid),
        BootProcfsNodeKind::Caps => return boot_procfs_caps(pid),
        BootProcfsNodeKind::Auxv => return boot_procfs_auxv(pid),
        BootProcfsNodeKind::Mounts => return boot_procfs_mounts(pid),
        BootProcfsNodeKind::FdInfo(fd) => return boot_procfs_fdinfo(pid, fd),
        BootProcfsNodeKind::Queues => return boot_procfs_queues(pid),
        BootProcfsNodeKind::SystemScheduler => return boot_procfs_system_scheduler_text(),
        BootProcfsNodeKind::SystemSchedulerEpisodes => {
            return boot_procfs_system_schedulerepisodes_text();
        }
        BootProcfsNodeKind::SystemBus => return boot_procfs_system_bus_text(),
        _ => {}
    }
    require_procfs_access(BootProcfsNode { pid, kind })?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        let text = match kind {
            BootProcfsNodeKind::ProcRootDir
            | BootProcfsNodeKind::ProcessDir
            | BootProcfsNodeKind::SystemDir
            | BootProcfsNodeKind::FdDirListing
            | BootProcfsNodeKind::FdInfoDirListing => {
                let path = descriptor_target_path_text(DescriptorTarget::Procfs(BootProcfsNode {
                    pid,
                    kind,
                }))?;
                boot_procfs_directory_listing(&path)?.ok_or(Errno::NoEnt)?
            }
            BootProcfsNodeKind::Status => format!(
                "Name:\t{}\nState:\t{}\nPid:\t{}\nUid:\t{}\nGid:\t{}\nUmask:\t{:03o}\nSupplementalGroups:\t{}\nSubjectLabel:\t{}\nRoot:\t{}\nCwd:\t{}\nVmObjects:\t{}\n",
                entry.name,
                if entry.state == 2 { "Running" } else { "Exited" },
                entry.pid,
                entry.uid,
                entry.gid,
                entry.umask & 0o777,
                if entry.supplemental_count == 0 {
                    String::from("-")
                } else {
                    entry.supplemental_gids[..entry.supplemental_count]
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                },
                security_label_text(entry.subject_label),
                entry.root,
                entry.cwd,
                entry.vm_objects.len()
            ),
            BootProcfsNodeKind::Root => entry.root.clone(),
            BootProcfsNodeKind::Cwd => entry.cwd.clone(),
            BootProcfsNodeKind::Exe => entry.image_path.clone(),
            BootProcfsNodeKind::Cmdline => entry
                .argv
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
            BootProcfsNodeKind::Environ => entry.envp.join("\n"),
            BootProcfsNodeKind::Auxv | BootProcfsNodeKind::Mounts => unreachable!(),
            BootProcfsNodeKind::Fd
            | BootProcfsNodeKind::Caps
            | BootProcfsNodeKind::FdInfo(_)
            | BootProcfsNodeKind::Queues
            | BootProcfsNodeKind::SystemScheduler
            | BootProcfsNodeKind::SystemSchedulerEpisodes
            | BootProcfsNodeKind::SystemBus => unreachable!(),
            BootProcfsNodeKind::VfsLocks => boot_procfs_vfs_locks_text(),
            BootProcfsNodeKind::VfsWatches => boot_procfs_vfs_watches_text(),
            BootProcfsNodeKind::VfsStats => boot_procfs_vfs_stats_text(),
            BootProcfsNodeKind::Maps => {
                let mut text = String::new();
                for object in &entry.vm_objects {
                    let perms = if object.quarantined {
                        "----"
                    } else {
                        match (object.readable, object.writable, object.executable) {
                            (true, true, true) => "rwxp",
                            (true, true, false) => "rw-p",
                            (true, false, true) => "r-xp",
                            (true, false, false) => "r--p",
                            (false, true, false) => "-w-p",
                            (false, true, true) => "-wxp",
                            (false, false, true) => "--xp",
                            (false, false, false) => "----",
                        }
                    };
                    text.push_str(&format!(
                        "{:016x}-{:016x} {} {:08x} {}\n",
                        object.start,
                        object.start.saturating_add(object.len),
                        perms,
                        object.file_offset,
                        object.name
                    ));
                }
                text
            }
            BootProcfsNodeKind::VmObjects => {
                let mut text = String::new();
                for object in &entry.vm_objects {
                    let owners = boot_vm_owner_count(registry, object.share_key);
                    let (segment_count, resident_segment_count) = boot_vm_segment_counts(object);
                    let shadow = object
                        .shadow_source_id
                        .map(|source| {
                            format!(
                                "\tshadow={:08x}@{:08x}/depth={}",
                                source, object.shadow_source_offset, object.shadow_depth
                            )
                        })
                        .unwrap_or_default();
                    text.push_str(&format!(
                        "{:08x}\t{}\tprivate={}\towners={}\toffset={:08x}\tcommitted={}\tresident={}\tdirty={}\taccessed={}\tsegments={}\tresident-segments={}\tfaults={}(r={},w={},cow={})\t{}\treadable={}\twritable={}\texecutable={}\tquarantined={}\treason={}{}\n",
                        object.id,
                        object.kind,
                        object.private_mapping,
                        owners,
                        object.file_offset,
                        object.committed_pages,
                        object.resident_pages,
                        object.dirty_pages,
                        object.accessed_pages,
                        segment_count,
                        resident_segment_count,
                        object.read_fault_count
                            .saturating_add(object.write_fault_count)
                            .saturating_add(object.cow_fault_count),
                        object.read_fault_count,
                        object.write_fault_count,
                        object.cow_fault_count,
                        object.name,
                        object.readable as u8,
                        object.writable as u8,
                        object.executable as u8,
                        object.quarantined as u8,
                        object.quarantine_reason,
                        shadow,
                    ));
                }
                text
            }
            BootProcfsNodeKind::VmDecisions => {
                let mut text = String::new();
                for (tick, decision) in entry.vm_decisions.iter().enumerate() {
                    push_vm_decision_line(&mut text, entry, tick, decision).map_err(|_| Errno::Io)?;
                }
                text
            }
            BootProcfsNodeKind::VmEpisodes => {
                let mut text = String::new();
                boot_procfs_write_vm_episodes(&mut text, entry).map_err(|_| Errno::Io)?;
                text
            }
        };
        Ok(text)
    })
}

fn security_label_text(label: SecurityLabel) -> String {
    format!("{:?}/{:?}", label.confidentiality, label.integrity)
}

fn boot_procfs_read(
    node: BootProcfsNode,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    if len == 0 {
        return Ok(0);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    match node.kind {
        BootProcfsNodeKind::Maps => {
            return boot_procfs_read_maps(node.pid, offset, buffer, len);
        }
        BootProcfsNodeKind::VmObjects => {
            return boot_procfs_read_vm_objects(node.pid, offset, buffer, len);
        }
        BootProcfsNodeKind::VmDecisions => {
            return boot_procfs_read_vm_decisions(node.pid, offset, buffer, len);
        }
        BootProcfsNodeKind::VmEpisodes => {
            return boot_procfs_read_vm_episodes(node.pid, offset, buffer, len);
        }
        _ => {}
    }
    let payload = boot_procfs_payload(node.pid, node.kind)?;
    let bytes = payload.as_bytes();
    let count = bytes.len().saturating_sub(*offset).min(len);
    if count == 0 {
        return Ok(0);
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes[*offset..*offset + count].as_ptr(), buffer, count);
    }
    *offset += count;
    Ok(count)
}

fn boot_procfs_len(node: BootProcfsNode) -> Result<usize, Errno> {
    Ok(boot_procfs_payload(node.pid, node.kind)?.len())
}

fn boot_procfs_read_maps(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|table| {
        let Some(index) = table.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &table.entries[index];
        let mut absolute_offset = 0usize;
        let mut written = 0usize;
        for object in &entry.vm_objects {
            let perms = if object.quarantined {
                "----"
            } else {
                match (object.readable, object.writable, object.executable) {
                    (true, true, true) => "rwxp",
                    (true, true, false) => "rw-p",
                    (true, false, true) => "r-xp",
                    (true, false, false) => "r--p",
                    (false, true, false) => "-w-p",
                    (false, true, true) => "-wxp",
                    (false, false, true) => "--xp",
                    (false, false, false) => "----",
                }
            };
            let mut line = ProcfsLineBuffer::new();
            write!(
                &mut line,
                "{:016x}-{:016x} {} {:08x} {}\n",
                object.start,
                object.start.saturating_add(object.len),
                perms,
                object.file_offset,
                object.name
            )
            .map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }
        Ok(written)
    })
}

fn boot_procfs_read_vm_decisions(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|table| {
        let Some(index) = table.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &table.entries[index];
        let mut absolute_offset = 0usize;
        let mut written = 0usize;
        for (tick, decision) in entry.vm_decisions.iter().enumerate() {
            let mut line = ProcfsLineBuffer::new();
            push_vm_decision_line(&mut line, entry, tick, decision).map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }
        Ok(written)
    })
}

fn push_vm_object_line(
    line: &mut ProcfsLineBuffer,
    registry: &BootProcessRegistry,
    object: &BootVmObject,
) -> core::fmt::Result {
    let owners = boot_vm_owner_count(registry, object.share_key);
    let (segment_count, resident_segment_count) = boot_vm_segment_counts(object);
    write!(
        line,
        "{:08x}\t{}\tprivate={}\towners={}\toffset={:08x}\tcommitted={}\tresident={}\tdirty={}\taccessed={}\tsegments={}\tresident-segments={}\tfaults={}(r={},w={},cow={})\t{}\treadable={}\twritable={}\texecutable={}\tquarantined={}\treason={}",
        object.id,
        object.kind,
        object.private_mapping,
        owners,
        object.file_offset,
        object.committed_pages,
        object.resident_pages,
        object.dirty_pages,
        object.accessed_pages,
        segment_count,
        resident_segment_count,
        object
            .read_fault_count
            .saturating_add(object.write_fault_count)
            .saturating_add(object.cow_fault_count),
        object.read_fault_count,
        object.write_fault_count,
        object.cow_fault_count,
        object.name,
        object.readable as u8,
        object.writable as u8,
        object.executable as u8,
        object.quarantined as u8,
        object.quarantine_reason,
    )?;
    if let Some(source) = object.shadow_source_id {
        write!(
            line,
            "\tshadow={:08x}@{:08x}/depth={}",
            source, object.shadow_source_offset, object.shadow_depth
        )?;
    }
    line.write_str("\n")
}

fn boot_procfs_read_vm_objects(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &registry.entries[index];
        let mut absolute_offset = 0usize;
        let mut written = 0usize;
        for object in &entry.vm_objects {
            let mut line = ProcfsLineBuffer::new();
            push_vm_object_line(&mut line, registry, object).map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }
        Ok(written)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootVmEpisodeRecord {
    start_tick: usize,
    vm_object_id: u64,
    kind: &'static str,
    fields: String,
}

fn boot_vm_episode_object_ids(entry: &BootProcessEntry) -> Vec<u64> {
    let mut ids = Vec::new();
    for object in &entry.vm_objects {
        if !ids.contains(&object.id) {
            ids.push(object.id);
        }
    }
    for decision in &entry.vm_decisions {
        if decision.vm_object_id == 0 || ids.contains(&decision.vm_object_id) {
            continue;
        }
        ids.push(decision.vm_object_id);
    }
    ids
}

fn boot_vm_episode_mapped_kind(decision: &BootVmDecision) -> &'static str {
    match decision.agent {
        "map" => "anon",
        "map-file" if (decision.detail1 & (1 << 3)) != 0 => "file-private",
        "map-file" => "file-shared",
        _ => "unknown",
    }
}

fn boot_collect_vm_episode_records(entry: &BootProcessEntry) -> Vec<BootVmEpisodeRecord> {
    let mut records = Vec::<BootVmEpisodeRecord>::new();

    let mut policy_start = None;
    let mut policy_end = 0usize;
    let mut policy_state = 0u64;
    let mut policy_operation = 0u64;
    let mut policy_blocked = false;
    let mut policy_decisions = 0usize;
    for (tick, decision) in entry.vm_decisions.iter().enumerate() {
        if decision.agent != "policy-block" {
            continue;
        }
        if policy_start.is_none() {
            policy_start = Some(tick + 1);
        }
        policy_end = tick + 1;
        policy_state = decision.detail0;
        policy_operation = decision.detail1;
        policy_blocked = true;
        policy_decisions += 1;
    }
    if let Some(start_tick) = policy_start {
        records.push(BootVmEpisodeRecord {
            start_tick,
            vm_object_id: 0,
            kind: "policy",
            fields: format!(
                "start-tick={}\tend-tick={}\tstate={}\toperation={}\tblocked={}\tdecisions={}\tlast=policy-block",
                start_tick,
                policy_end.max(start_tick),
                policy_state,
                policy_operation,
                if policy_blocked { "yes" } else { "no" },
                policy_decisions.max(1),
            ),
        });
    }

    for object_id in boot_vm_episode_object_ids(entry) {
        let resident_pages = entry
            .vm_objects
            .iter()
            .find(|object| object.id == object_id)
            .map(|object| object.resident_pages)
            .unwrap_or(0);

        let mut map_start = None;
        let mut map_end = 0usize;
        let mut map_decisions = 0usize;
        let mut mapped_kind = "unknown";
        let mut map_last = "";

        let mut heap_start = None;
        let mut heap_end = 0usize;
        let mut heap_decisions = 0usize;
        let mut heap_old_end = 0u64;
        let mut heap_new_end = 0u64;
        let mut heap_grew = false;
        let mut heap_shrank = false;
        let mut heap_last = "";

        let mut quarantine_start = None;
        let mut quarantine_end = 0usize;
        let mut quarantine_reason = 0u64;
        let mut quarantine_blocked = false;
        let mut quarantine_released = false;
        let mut quarantine_decisions = 0usize;
        let mut quarantine_last = "";

        let mut reclaim_start = None;
        let mut reclaim_end = 0usize;
        let mut reclaim_decisions = 0usize;
        let mut reclaim_evicted = false;
        let mut reclaim_restored = false;
        let mut reclaim_last = "";

        let mut fault_start = None;
        let mut fault_end = 0usize;
        let mut fault_decisions = 0usize;
        let mut faulted = false;
        let mut cow = false;
        let mut bridged = false;
        let mut touched = false;
        let mut synced = false;
        let mut advised = false;
        let mut fault_last = "";

        let mut region_start = None;
        let mut region_end = 0usize;
        let mut region_decisions = 0usize;
        let mut region_protected = false;
        let mut region_unmapped = false;
        let mut region_last = "";

        for (tick, decision) in entry.vm_decisions.iter().enumerate() {
            if decision.vm_object_id != object_id {
                continue;
            }
            match decision.agent {
                "map" | "map-file" => {
                    if map_start.is_none() {
                        map_start = Some(tick + 1);
                    }
                    map_end = tick + 1;
                    map_decisions += 1;
                    mapped_kind = boot_vm_episode_mapped_kind(decision);
                    map_last = decision.agent;
                }
                "brk" => {
                    if heap_start.is_none() {
                        heap_start = Some(tick + 1);
                        heap_old_end = decision.detail0;
                    }
                    heap_end = tick + 1;
                    heap_decisions += 1;
                    heap_old_end = heap_old_end.min(decision.detail0);
                    heap_new_end = decision.detail1;
                    heap_grew |= decision.detail1 > decision.detail0;
                    heap_shrank |= decision.detail1 < decision.detail0;
                    heap_last = decision.agent;
                }
                "quarantine-state" => {
                    if decision.detail1 == 1 && quarantine_start.is_none() {
                        quarantine_start = Some(tick + 1);
                        quarantine_reason = decision.detail0;
                    }
                    if quarantine_start.is_some() {
                        quarantine_end = tick + 1;
                        quarantine_decisions += 1;
                        quarantine_last = "quarantine-state";
                        if decision.detail1 == 0 {
                            quarantine_released = true;
                        }
                    }
                }
                "quarantine-block" => {
                    if quarantine_start.is_some() {
                        quarantine_end = tick + 1;
                        quarantine_decisions += 1;
                        quarantine_last = "quarantine-block";
                        quarantine_blocked = true;
                    }
                }
                "pressure-victim" => {
                    if reclaim_start.is_none() {
                        reclaim_start = Some(tick + 1);
                    }
                    reclaim_end = tick + 1;
                    reclaim_decisions += 1;
                    reclaim_last = "pressure-victim";
                }
                "advice" if reclaim_start.is_some() => {
                    reclaim_end = tick + 1;
                    reclaim_decisions += 1;
                    reclaim_last = "advice";
                    if decision.detail0 == 4 {
                        reclaim_evicted = true;
                    } else if decision.detail0 == 3 {
                        reclaim_restored = true;
                    }
                }
                "fault-classifier" | "page-touch" | "sync" | "advice" | "cow-populate"
                | "shadow-reuse" | "shadow-bridge" => {
                    if fault_start.is_none() {
                        fault_start = Some(tick + 1);
                    }
                    fault_end = tick + 1;
                    fault_decisions += 1;
                    fault_last = decision.agent;
                    if decision.agent == "fault-classifier" {
                        faulted = true;
                        if reclaim_start.is_some() {
                            reclaim_restored = true;
                        }
                    } else if decision.agent == "page-touch" {
                        touched = true;
                        if reclaim_start.is_some() {
                            reclaim_restored = true;
                        }
                    } else if decision.agent == "sync" {
                        synced = true;
                        if reclaim_start.is_some() {
                            reclaim_restored = true;
                        }
                    } else if decision.agent == "advice" {
                        advised = true;
                    } else if decision.agent == "shadow-bridge" {
                        bridged = true;
                    } else {
                        cow = true;
                    }
                    if reclaim_start.is_some() {
                        reclaim_end = tick + 1;
                        reclaim_decisions += 1;
                        reclaim_last = decision.agent;
                    }
                }
                "protect" | "unmap" => {
                    if region_start.is_none() {
                        region_start = Some(tick + 1);
                    }
                    region_end = tick + 1;
                    region_decisions += 1;
                    region_last = decision.agent;
                    if decision.agent == "protect" {
                        region_protected = true;
                    } else {
                        region_unmapped = true;
                    }
                }
                _ => {}
            }
        }

        if let Some(start_tick) = map_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "map",
                fields: format!(
                    "start-tick={}\tend-tick={}\tmapped={}\tdecisions={}\tlast={}",
                    start_tick,
                    map_end.max(start_tick),
                    mapped_kind,
                    map_decisions.max(1),
                    if map_last.is_empty() { "map" } else { map_last },
                ),
            });
        }

        if let Some(start_tick) = heap_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "heap",
                fields: format!(
                    "start-tick={}\tend-tick={}\tgrew={}\tshrank={}\told-end={}\tnew-end={}\tdecisions={}\tlast={}",
                    start_tick,
                    heap_end.max(start_tick),
                    if heap_grew { "yes" } else { "no" },
                    if heap_shrank { "yes" } else { "no" },
                    heap_old_end,
                    heap_new_end,
                    heap_decisions.max(1),
                    if heap_last.is_empty() { "brk" } else { heap_last },
                ),
            });
        }

        if let Some(start_tick) = quarantine_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "quarantine",
                fields: format!(
                    "start-tick={}\tend-tick={}\treason={}\tblocked={}\treleased={}\tdecisions={}\tlast={}",
                    start_tick,
                    quarantine_end.max(start_tick),
                    quarantine_reason,
                    if quarantine_blocked { "yes" } else { "no" },
                    if quarantine_released { "yes" } else { "no" },
                    quarantine_decisions.max(1),
                    if quarantine_last.is_empty() {
                        "quarantine-state"
                    } else {
                        quarantine_last
                    },
                ),
            });
        }

        if let Some(start_tick) = reclaim_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "reclaim",
                fields: format!(
                    "start-tick={}\tend-tick={}\tevicted={}\trestored={}\tdecisions={}\tlast={}",
                    start_tick,
                    reclaim_end.max(start_tick),
                    if reclaim_evicted || resident_pages == 0 {
                        "yes"
                    } else {
                        "no"
                    },
                    if reclaim_restored { "yes" } else { "no" },
                    reclaim_decisions.max(1),
                    if reclaim_last.is_empty() {
                        "pressure-victim"
                    } else {
                        reclaim_last
                    },
                ),
            });
        }

        if let Some(start_tick) = fault_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "fault",
                fields: format!(
                    "start-tick={}\tend-tick={}\tfaulted={}\tcow={}\tbridged={}\ttouched={}\tsynced={}\tadvised={}\tdecisions={}\tlast={}",
                    start_tick,
                    fault_end.max(start_tick),
                    if faulted { "yes" } else { "no" },
                    if cow { "yes" } else { "no" },
                    if bridged { "yes" } else { "no" },
                    if touched { "yes" } else { "no" },
                    if synced { "yes" } else { "no" },
                    if advised { "yes" } else { "no" },
                    fault_decisions.max(1),
                    if fault_last.is_empty() {
                        "fault-classifier"
                    } else {
                        fault_last
                    },
                ),
            });
        }

        if let Some(start_tick) = region_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "region",
                fields: format!(
                    "start-tick={}\tend-tick={}\tprotected={}\tunmapped={}\tdecisions={}\tlast={}",
                    start_tick,
                    region_end.max(start_tick),
                    if region_protected { "yes" } else { "no" },
                    if region_unmapped { "yes" } else { "no" },
                    region_decisions.max(1),
                    if region_last.is_empty() {
                        "protect"
                    } else {
                        region_last
                    },
                ),
            });
        }
    }

    records.sort_by_key(|record| (record.start_tick, record.vm_object_id));
    records
}

fn boot_procfs_write_vm_episodes(
    output: &mut impl Write,
    entry: &BootProcessEntry,
) -> core::fmt::Result {
    let records = boot_collect_vm_episode_records(entry);
    for (episode, record) in records.iter().enumerate() {
        write!(
            output,
            "episode={}\tkind={}\tvm-object={:08x}\t{}\n",
            episode + 1,
            record.kind,
            record.vm_object_id,
            record.fields,
        )?;
    }

    Ok(())
}

fn boot_procfs_read_vm_episodes(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &registry.entries[index];
        let records = boot_collect_vm_episode_records(entry);
        let mut absolute_offset = 0usize;
        let mut written = 0usize;

        for (episode, record) in records.iter().enumerate() {
            let mut line = ProcfsLineBuffer::new();
            write!(
                &mut line,
                "episode={}\tkind={}\tvm-object={:08x}\t{}\n",
                episode + 1,
                record.kind,
                record.vm_object_id,
                record.fields,
            )
            .map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }

        Ok(written)
    })
}

fn push_vm_decision_line(
    output: &mut impl Write,
    entry: &BootProcessEntry,
    tick: usize,
    decision: &BootVmDecision,
) -> core::fmt::Result {
    let object_name = entry
        .vm_objects
        .iter()
        .find(|object| object.id == decision.vm_object_id)
        .map(|object| object.name.as_str())
        .unwrap_or("-");
    write!(
        output,
        "tick={}\tagent={}\tvm-object={:08x}\tstart={:08x}\tlen={:08x}\tdetail0={}\tdetail1={}\tname={}\n",
        tick + 1,
        decision.agent,
        decision.vm_object_id,
        decision.start,
        decision.len,
        decision.detail0,
        decision.detail1,
        object_name,
    )
}

fn boot_vm_page_count_for_len(len: u64) -> usize {
    (len / 0x1000) as usize
}

fn boot_vm_recount_object_pages(object: &mut BootVmObject) {
    object.committed_pages = object.page_states.len() as u64;
    object.resident_pages = object
        .page_states
        .iter()
        .filter(|page| page.resident)
        .count() as u64;
    object.dirty_pages = object.page_states.iter().filter(|page| page.dirty).count() as u64;
    object.accessed_pages = object
        .page_states
        .iter()
        .filter(|page| page.accessed)
        .count() as u64;
}

fn boot_vm_segment_counts(object: &BootVmObject) -> (usize, usize) {
    if object.page_states.is_empty() {
        return (0, 0);
    }

    let mut segment_count = 0usize;
    let mut resident_segment_count = 0usize;
    let mut current: Option<BootVmPageState> = None;

    for page in &object.page_states {
        if current != Some(*page) {
            segment_count = segment_count.saturating_add(1);
            if page.resident {
                resident_segment_count = resident_segment_count.saturating_add(1);
            }
            current = Some(*page);
        }
    }

    (segment_count, resident_segment_count)
}

fn boot_vm_shadow_label(name: &str) -> String {
    format!("{} [cow]", name)
}

fn boot_vm_page_range(object: &BootVmObject, page_index: usize) -> (u64, u64) {
    let start = object.start + ((page_index as u64) * 0x1000);
    (start, start + 0x1000)
}

fn boot_vm_find_adjacent_shadow_neighbors(
    entry: &BootProcessEntry,
    object_index: usize,
    shadow_source_id: u64,
    shadow_depth: u32,
    shadow_source_offset: u64,
) -> (Option<usize>, Option<usize>) {
    let mut left = None;
    let mut right = None;
    if object_index > 0 {
        let candidate = &entry.vm_objects[object_index - 1];
        if candidate.shadow_source_id == Some(shadow_source_id)
            && candidate.shadow_depth == shadow_depth
            && candidate.start.saturating_add(candidate.len) == entry.vm_objects[object_index].start
            && candidate.shadow_source_offset.saturating_add(candidate.len) == shadow_source_offset
        {
            left = Some(object_index - 1);
        }
    }
    if object_index + 1 < entry.vm_objects.len() {
        let candidate = &entry.vm_objects[object_index + 1];
        if candidate.shadow_source_id == Some(shadow_source_id)
            && candidate.shadow_depth == shadow_depth
            && entry.vm_objects[object_index]
                .start
                .saturating_add(entry.vm_objects[object_index].len)
                == candidate.start
            && shadow_source_offset.saturating_add(entry.vm_objects[object_index].len)
                == candidate.shadow_source_offset
        {
            right = Some(object_index + 1);
        }
    }
    (left, right)
}

fn boot_vm_merge_object_range(left: &mut BootVmObject, right: BootVmObject) {
    left.len = left.len.saturating_add(right.len);
    left.page_states.extend(right.page_states);
    left.read_fault_count = left.read_fault_count.saturating_add(right.read_fault_count);
    left.write_fault_count = left
        .write_fault_count
        .saturating_add(right.write_fault_count);
    left.cow_fault_count = left.cow_fault_count.saturating_add(right.cow_fault_count);
    boot_vm_recount_object_pages(left);
}

fn boot_vm_touch_object_page(
    entry: &mut BootProcessEntry,
    mut object_index: usize,
    addr: u64,
    is_write: bool,
    owners: u64,
) -> Result<(), Errno> {
    let Some(object) = entry.vm_objects.get(object_index) else {
        return Err(Errno::Fault);
    };
    if addr < object.start || addr >= object.start.saturating_add(object.len) {
        return Err(Errno::Fault);
    }
    let page_index = ((addr - object.start) / 0x1000) as usize;
    let (absolute_page_start, absolute_page_end) = boot_vm_page_range(object, page_index);
    let mut replacement = None;
    if is_write && owners > 1 {
        if absolute_page_start > object.start {
            object_index = split_vm_object_at(entry, object_index, absolute_page_start)
                .unwrap_or(object_index);
        }
        if absolute_page_end
            < entry.vm_objects[object_index]
                .start
                .saturating_add(entry.vm_objects[object_index].len)
        {
            split_vm_object_at(entry, object_index, absolute_page_end);
        }
        let object = entry.vm_objects[object_index].clone();
        let shadow_source_id = object.shadow_source_id.unwrap_or(object.share_key);
        let mut shadow = object.clone();
        shadow.id = entry.next_vm_object_id;
        entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
        shadow.share_key = shadow.id;
        shadow.kind = "Anonymous";
        shadow.backing_inode = None;
        shadow.name = boot_vm_shadow_label(&object.name);
        shadow.shadow_source_id = Some(shadow_source_id);
        shadow.shadow_source_offset = object.shadow_source_offset;
        shadow.shadow_depth = object.shadow_depth.saturating_add(1);
        shadow.cow_fault_count = 0;
        replacement = Some(shadow);
    }
    let object = if let Some(shadow) = replacement {
        let (left_shadow, right_shadow) = boot_vm_find_adjacent_shadow_neighbors(
            entry,
            object_index,
            shadow.shadow_source_id.unwrap_or(0),
            shadow.shadow_depth,
            shadow.shadow_source_offset,
        );
        entry.vm_decisions.push(BootVmDecision {
            agent: "shadow-reuse",
            vm_object_id: shadow.id,
            start: absolute_page_start,
            len: 0x1000,
            detail0: shadow.shadow_source_id.unwrap_or(0),
            detail1: shadow.shadow_depth as u64,
        });
        entry.vm_objects[object_index] = shadow;
        if let Some(left_index) = left_shadow {
            let current = entry.vm_objects.remove(object_index);
            object_index = left_index;
            let left = &mut entry.vm_objects[left_index];
            boot_vm_merge_object_range(left, current);
        }
        if let Some(right_index) = right_shadow {
            let adjusted_right = if left_shadow.is_some() {
                right_index - 1
            } else {
                right_index
            };
            let right = entry.vm_objects.remove(adjusted_right);
            let left = &mut entry.vm_objects[object_index];
            boot_vm_merge_object_range(left, right);
            if left_shadow.is_some() {
                entry.vm_decisions.push(BootVmDecision {
                    agent: "shadow-bridge",
                    vm_object_id: entry.vm_objects[object_index].id,
                    start: absolute_page_start,
                    len: 0x1000,
                    detail0: 1,
                    detail1: 1,
                });
            }
        }
        let object = &entry.vm_objects[object_index];
        let local_page_index = ((absolute_page_start - object.start) / 0x1000) as u64;
        entry.vm_decisions.push(BootVmDecision {
            agent: "cow-populate",
            vm_object_id: object.id,
            start: absolute_page_start,
            len: 0x1000,
            detail0: 1,
            detail1: local_page_index,
        });
        let object = &mut entry.vm_objects[object_index];
        object.cow_fault_count = object.cow_fault_count.saturating_add(1);
        object
    } else {
        &mut entry.vm_objects[object_index]
    };
    let page_index = ((absolute_page_start - object.start) / 0x1000) as usize;
    let Some(page) = object.page_states.get_mut(page_index) else {
        return Err(Errno::Fault);
    };
    let was_resident = page.resident;
    page.resident = true;
    page.accessed = true;
    if is_write {
        page.dirty = true;
    }
    if !was_resident {
        if is_write {
            object.write_fault_count = object.write_fault_count.saturating_add(1);
        } else {
            object.read_fault_count = object.read_fault_count.saturating_add(1);
        }
        entry.vm_decisions.push(BootVmDecision {
            agent: "fault-classifier",
            vm_object_id: object.id,
            start: absolute_page_start,
            len: 0x1000,
            detail0: if is_write { 1 } else { 0 },
            detail1: page_index as u64,
        });
    }
    entry.vm_decisions.push(BootVmDecision {
        agent: "page-touch",
        vm_object_id: object.id,
        start: absolute_page_start,
        len: 0x1000,
        detail0: page_index as u64,
        detail1: if is_write { 1 } else { 0 },
    });
    boot_vm_recount_object_pages(object);
    Ok(())
}

fn boot_vm_owner_count(registry: &BootProcessRegistry, share_key: u64) -> u64 {
    registry
        .entries
        .iter()
        .filter(|entry| !entry.reaped)
        .filter(|entry| {
            entry
                .vm_objects
                .iter()
                .any(|object| object.share_key == share_key)
        })
        .count() as u64
}

fn boot_vm_clone_for_copy(entry: &mut BootProcessEntry, source: &BootVmObject) -> BootVmObject {
    let object_id = entry.next_vm_object_id;
    entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
    BootVmObject {
        id: object_id,
        start: source.start,
        len: source.len,
        name: source.name.clone(),
        kind: source.kind,
        backing_inode: source.backing_inode,
        share_key: source.share_key,
        shadow_source_id: source.shadow_source_id,
        shadow_source_offset: source.shadow_source_offset,
        shadow_depth: source.shadow_depth,
        private_mapping: source.private_mapping,
        file_offset: source.file_offset,
        bytes: source.bytes.clone(),
        readable: source.readable,
        writable: source.writable,
        executable: source.executable,
        read_fault_count: source.read_fault_count,
        write_fault_count: source.write_fault_count,
        cow_fault_count: source.cow_fault_count,
        committed_pages: source.committed_pages,
        resident_pages: source.resident_pages,
        dirty_pages: source.dirty_pages,
        accessed_pages: source.accessed_pages,
        quarantined: source.quarantined,
        quarantine_reason: source.quarantine_reason,
        page_states: source.page_states.clone(),
    }
}

fn boot_copy_vm_state(source_pid: u64, target_pid: u64) -> Result<(), Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(source_index) = registry.find_index(source_pid) else {
            return Err(Errno::Srch);
        };
        let Some(target_index) = registry.find_index(target_pid) else {
            return Err(Errno::Srch);
        };
        let source_objects = registry.entries[source_index].vm_objects.clone();
        let source_next_vm_addr = registry.entries[source_index].next_vm_addr;
        let target = &mut registry.entries[target_index];
        target.next_vm_addr = source_next_vm_addr;
        target.vm_objects = source_objects
            .iter()
            .map(|object| boot_vm_clone_for_copy(target, object))
            .collect();
        target.vm_decisions.push(BootVmDecision {
            agent: "fork",
            vm_object_id: 0,
            start: source_pid,
            len: target_pid,
            detail0: source_objects.len() as u64,
            detail1: 0,
        });
        Ok(())
    })
}

fn boot_procfs_poll(node: BootProcfsNode, offset: usize, interest: u32) -> u32 {
    let Ok(payload) = boot_procfs_payload(node.pid, node.kind) else {
        return 0;
    };
    let mut ready = POLLOUT;
    if offset < payload.len() {
        ready |= POLLIN;
    }
    ready & interest
}

fn boot_vfs_read(
    inode: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let node = vfs.clone_object_by_inode(inode).ok_or(Errno::Badf)?;
        BootVfs::require_access_for_node(
            &BootNode {
                path: String::new(),
                kind: node.kind,
                inode: node.inode,
                bytes: node.bytes.clone(),
                link_target: node.link_target.clone(),
                owner_uid: node.owner_uid,
                group_gid: node.group_gid,
                mode: node.mode,
                minimum_label: node.minimum_label,
                current_label: node.current_label,
                mount_layer: 0,
                mount_id: None,
            },
            true,
            false,
            false,
        )?;
        let node_len = node.bytes.len();
        let available = node_len.saturating_sub(*offset);
        let count = available.min(len);
        if count == 0 {
            return Ok(0);
        }
        let mut copied = 0usize;
        while copied < count {
            let absolute = *offset + copied;
            let page_base = (absolute / BootVfs::PAGE_CACHE_GRANULE) * BootVfs::PAGE_CACHE_GRANULE;
            let page = if let Some(index) = vfs.find_live_node_index_by_inode(inode) {
                vfs.page_bytes(index, page_base).ok_or(Errno::Badf)?
            } else {
                let end = (page_base + BootVfs::PAGE_CACHE_GRANULE).min(node.bytes.len());
                node.bytes[page_base.min(end)..end].to_vec()
            };
            let page_offset = absolute.saturating_sub(page_base);
            let chunk = (count - copied).min(page.len().saturating_sub(page_offset));
            unsafe {
                ptr::copy_nonoverlapping(
                    page[page_offset..page_offset + chunk].as_ptr(),
                    buffer.add(copied),
                    chunk,
                );
            }
            copied += chunk;
        }
        *offset += count;
        Ok(count)
    })
}

fn boot_vfs_write(
    inode: u64,
    offset: &mut usize,
    bytes: &[u8],
    actor_description: Option<usize>,
) -> Result<usize, Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let node = vfs.clone_object_by_inode(inode).ok_or(Errno::Badf)?;
        let access_node = BootNode {
            path: String::new(),
            kind: node.kind,
            inode: node.inode,
            bytes: node.bytes.clone(),
            link_target: node.link_target.clone(),
            owner_uid: node.owner_uid,
            group_gid: node.group_gid,
            mode: node.mode,
            minimum_label: node.minimum_label,
            current_label: node.current_label,
            mount_layer: 0,
            mount_id: None,
        };
        BootVfs::require_access_for_node(&access_node, false, true, false)?;
        if matches!(node.kind, BootNodeKind::Directory) {
            return Err(Errno::IsDir);
        }
        if let Some(path) = vfs.live_path_for_inode(inode) {
            vfs.object_lock_conflict(&path, actor_description)?;
        } else {
            VFS_LOCKS.with_mut(|locks| {
                if locks.iter().any(|lock| {
                    lock.inode == inode
                        && actor_description.is_none_or(|actor| actor != lock.owner_fd)
                }) {
                    Err(Errno::Busy)
                } else {
                    Ok(())
                }
            })?;
        }
        if matches!(node.kind, BootNodeKind::Channel) {
            for entry in vfs.nodes.iter_mut().filter(|entry| entry.inode == inode) {
                entry.bytes.extend_from_slice(bytes);
                *offset = entry.bytes.len();
            }
            if let Some(orphan_index) = vfs.orphan_index_by_inode(inode) {
                vfs.orphan_nodes[orphan_index]
                    .bytes
                    .extend_from_slice(bytes);
                *offset = vfs.orphan_nodes[orphan_index].bytes.len();
            }
            return Ok(bytes.len());
        }
        let end = offset.saturating_add(bytes.len());
        for entry in vfs.nodes.iter_mut().filter(|entry| entry.inode == inode) {
            if *offset > entry.bytes.len() {
                entry.bytes.resize(*offset, 0);
            }
            if end > entry.bytes.len() {
                entry.bytes.resize(end, 0);
            }
            entry.bytes[*offset..end].copy_from_slice(bytes);
        }
        if let Some(orphan_index) = vfs.orphan_index_by_inode(inode) {
            let entry = &mut vfs.orphan_nodes[orphan_index];
            if *offset > entry.bytes.len() {
                entry.bytes.resize(*offset, 0);
            }
            if end > entry.bytes.len() {
                entry.bytes.resize(end, 0);
            }
            entry.bytes[*offset..end].copy_from_slice(bytes);
        }
        *offset = end;
        vfs.invalidate_caches();
        Ok(bytes.len())
    })
}

fn boot_vfs_poll(inode: u64, offset: usize, interest: u32) -> u32 {
    BOOT_VFS.with_mut(|vfs| {
        let Some(node) = vfs.clone_object_by_inode(inode) else {
            return 0;
        };
        let mut ready = 0;
        if offset < node.bytes.len() {
            ready |= POLLIN;
        }
        if !matches!(node.kind, BootNodeKind::Directory) {
            ready |= POLLOUT;
        }
        ready & interest
    })
}

fn boot_stream_target(path: &str) -> Option<DescriptorTarget> {
    match path {
        GPU_DEVICE_PATH => Some(DescriptorTarget::GpuDevice),
        GPU_DRIVER_PATH => Some(DescriptorTarget::GpuDriver),
        AUDIO_DEVICE_PATH => Some(DescriptorTarget::AudioDevice),
        AUDIO_DRIVER_PATH => Some(DescriptorTarget::AudioDriver),
        INPUT_DEVICE_PATH => Some(DescriptorTarget::InputDevice),
        INPUT_DRIVER_PATH => Some(DescriptorTarget::InputDriver),
        NETWORK_DEVICE_PATH => Some(DescriptorTarget::NetworkDevice),
        NETWORK_DRIVER_PATH => Some(DescriptorTarget::NetworkDriver),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootProcessEntry {
    pid: u64,
    parent: u64,
    name: String,
    image_path: String,
    uid: u32,
    gid: u32,
    umask: u32,
    subject_label: SecurityLabel,
    supplemental_count: usize,
    supplemental_gids: [u32; 8],
    root: String,
    cwd: String,
    descriptors: Vec<BootProcessDescriptorRecord>,
    argv: Vec<String>,
    argv_count: u64,
    env_count: u64,
    envp: Vec<String>,
    execution_mode: BootProcessExecutionMode,
    state: u32,
    exit_code: i32,
    pending_signal_count: u64,
    scheduler_class: u32,
    scheduler_budget: u32,
    cpu_runtime_ticks: u64,
    contract_bindings: BootProcessContractBindings,
    next_vm_addr: u64,
    next_vm_object_id: u64,
    vm_objects: Vec<BootVmObject>,
    vm_decisions: Vec<BootVmDecision>,
    reaped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootProcessDescriptorRecord {
    fd: u64,
    path: String,
    kind_name: &'static str,
    cloexec: bool,
    nonblock: bool,
    pos: usize,
    rights: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootProcessSchedulerState {
    pid: u64,
    affinity_mask: u64,
    assigned_cpu: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BootSchedulerEventState {
    rebalance_operations: u64,
    rebalance_migrations: u64,
    last_rebalance_migrations: u64,
    last_rebalance_pid: u64,
    last_rebalance_from_cpu: usize,
    last_rebalance_to_cpu: usize,
    last_affinity_pid: u64,
    last_affinity_mask: u64,
    last_affinity_cpu: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BootProcessContractBindings {
    execution: u64,
    memory: u64,
    io: u64,
    observe: u64,
}

impl BootProcessContractBindings {
    fn bind(&mut self, kind: NativeContractKind, contract: u64) {
        match kind {
            NativeContractKind::Execution => self.execution = contract,
            NativeContractKind::Memory => self.memory = contract,
            NativeContractKind::Io => self.io = contract,
            NativeContractKind::Observe => self.observe = contract,
            NativeContractKind::Device | NativeContractKind::Display => {}
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct BootProcessReapSummary {
    descriptors: u64,
    env_records: u64,
    vm_objects: u64,
    vm_decisions: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum BootProcessExecutionMode {
    #[default]
    MetadataOnly,
    SameImageBlocking,
}

struct BootProcessContractBindAgent;

impl BootProcessContractBindAgent {
    fn execute(contract_id: usize) -> Result<NativeContractKind, Errno> {
        let pid = BOOT_OWNER_ID;
        let kind = NATIVE_REGISTRY.with(|registry| {
            let contract = registry.contract(contract_id)?;
            if contract.issuer != pid {
                return Err(Errno::Access);
            }
            Ok(contract.kind)
        })?;
        BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid) else {
                return Err(Errno::Srch);
            };
            registry.entries[index]
                .contract_bindings
                .bind(kind, contract_id as u64);
            Ok(())
        })?;
        Ok(kind)
    }
}

struct BootVmPolicyBlockAgent;

impl BootVmPolicyBlockAgent {
    fn record(pid: usize, start: usize, len: usize, state_code: u64, operation_code: u64) {
        let _ = BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid as u64) else {
                return Err(Errno::Srch);
            };
            registry.entries[index].vm_decisions.push(BootVmDecision {
                agent: "policy-block",
                vm_object_id: 0,
                start: start as u64,
                len: len as u64,
                detail0: state_code,
                detail1: operation_code,
            });
            Ok(())
        });
    }
}

struct BootVmPolicyEnforcementAgent;

impl BootVmPolicyEnforcementAgent {
    fn enforce(pid: usize, start: usize, len: usize, operation_code: u64) -> Result<(), Errno> {
        let contract_id = BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid as u64) else {
                return Err(Errno::Srch);
            };
            Ok(registry.entries[index].contract_bindings.memory)
        })?;
        if contract_id == 0 {
            return Ok(());
        }

        let state = NATIVE_REGISTRY.with(|registry| {
            let contract = registry.contract(contract_id as usize)?;
            let resource = registry.resource(contract.resource as usize)?;
            if !contract_kind_allowed(resource.contract_policy, NativeContractKind::Memory) {
                return Err(Errno::Access);
            }
            Ok(contract.state)
        })?;

        if state != NativeContractState::Active {
            let state_code = match state {
                NativeContractState::Active => 0,
                NativeContractState::Suspended => 1,
                NativeContractState::Revoked => 2,
            };
            BootVmPolicyBlockAgent::record(pid, start, len, state_code, operation_code);
            return Err(Errno::Access);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootVmObject {
    id: u64,
    start: u64,
    len: u64,
    name: String,
    kind: &'static str,
    backing_inode: Option<u64>,
    share_key: u64,
    shadow_source_id: Option<u64>,
    shadow_source_offset: u64,
    shadow_depth: u32,
    private_mapping: bool,
    file_offset: u64,
    bytes: Vec<u8>,
    readable: bool,
    writable: bool,
    executable: bool,
    read_fault_count: u64,
    write_fault_count: u64,
    cow_fault_count: u64,
    committed_pages: u64,
    resident_pages: u64,
    dirty_pages: u64,
    accessed_pages: u64,
    quarantined: bool,
    quarantine_reason: u64,
    page_states: Vec<BootVmPageState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootVmDecision {
    agent: &'static str,
    vm_object_id: u64,
    start: u64,
    len: u64,
    detail0: u64,
    detail1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BootVmPageState {
    resident: bool,
    dirty: bool,
    accessed: bool,
}

#[derive(Debug, Default)]
struct BootProcessRegistry {
    next_pid: u64,
    entries: Vec<BootProcessEntry>,
    scheduler_states: Vec<BootProcessSchedulerState>,
    scheduler_events: BootSchedulerEventState,
}

struct BootProcessRegistryCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<BootProcessRegistry>>,
}

impl BootProcessRegistryCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootProcessRegistry) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let state = unsafe { &mut *self.state.get() };
        if state.is_none() {
            *state = Some(BootProcessRegistry::new());
        }
        let result = f(state.as_mut().unwrap());
        self.locked.store(false, Ordering::Release);
        result
    }
}

impl BootProcessRegistry {
    fn new() -> Self {
        Self {
            next_pid: 2,
            scheduler_states: vec![BootProcessSchedulerState {
                pid: 1,
                affinity_mask: boot_scheduler_online_mask(),
                assigned_cpu: 0,
            }],
            scheduler_events: BootSchedulerEventState::default(),
            entries: vec![BootProcessEntry {
                pid: 1,
                parent: 0,
                name: String::from("ngos-userland-native"),
                image_path: String::from("/kernel/ngos-userland-native"),
                uid: 1000,
                gid: 1000,
                umask: 0o022,
                subject_label: SecurityLabel::new(
                    ConfidentialityLevel::Public,
                    IntegrityLevel::Verified,
                ),
                supplemental_count: 0,
                supplemental_gids: [0; 8],
                root: String::from("/"),
                cwd: String::from("/"),
                descriptors: vec![
                    BootProcessDescriptorRecord {
                        fd: 0,
                        path: String::from("stdin"),
                        kind_name: "File",
                        cloexec: false,
                        nonblock: false,
                        pos: 0,
                        rights: BlockRightsMask::READ.0 as u32,
                    },
                    BootProcessDescriptorRecord {
                        fd: 1,
                        path: String::from("stdout"),
                        kind_name: "File",
                        cloexec: false,
                        nonblock: false,
                        pos: 0,
                        rights: BlockRightsMask::WRITE.0 as u32,
                    },
                    BootProcessDescriptorRecord {
                        fd: 2,
                        path: String::from("stderr"),
                        kind_name: "File",
                        cloexec: false,
                        nonblock: false,
                        pos: 0,
                        rights: BlockRightsMask::WRITE.0 as u32,
                    },
                ],
                argv: vec![String::from("/kernel/ngos-userland-native")],
                argv_count: 1,
                env_count: 0,
                envp: Vec::new(),
                execution_mode: BootProcessExecutionMode::SameImageBlocking,
                state: 2,
                exit_code: 0,
                pending_signal_count: 0,
                scheduler_class: NativeSchedulerClass::LatencyCritical as u32,
                scheduler_budget: 4,
                cpu_runtime_ticks: 1,
                contract_bindings: BootProcessContractBindings::default(),
                next_vm_addr: 0x6000_0000,
                next_vm_object_id: 2,
                vm_objects: vec![BootVmObject {
                    id: 1,
                    start: 0x4000_0000,
                    len: 0x4000,
                    name: String::from("[heap]"),
                    kind: "Heap",
                    backing_inode: None,
                    share_key: 1,
                    shadow_source_id: None,
                    shadow_source_offset: 0,
                    shadow_depth: 0,
                    private_mapping: true,
                    file_offset: 0,
                    bytes: vec![0; 0x4000],
                    readable: true,
                    writable: true,
                    executable: false,
                    read_fault_count: 0,
                    write_fault_count: 0,
                    cow_fault_count: 0,
                    committed_pages: 4,
                    resident_pages: 4,
                    dirty_pages: 1,
                    accessed_pages: 1,
                    quarantined: false,
                    quarantine_reason: 0,
                    page_states: vec![
                        BootVmPageState {
                            resident: true,
                            dirty: true,
                            accessed: true,
                        },
                        BootVmPageState {
                            resident: true,
                            dirty: false,
                            accessed: false,
                        },
                        BootVmPageState {
                            resident: true,
                            dirty: false,
                            accessed: false,
                        },
                        BootVmPageState {
                            resident: true,
                            dirty: false,
                            accessed: false,
                        },
                    ],
                }],
                vm_decisions: Vec::new(),
                reaped: false,
            }],
        }
    }

    fn find_index(&self, pid: u64) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.pid == pid && !entry.reaped)
    }

    fn scheduler_state_index(&self, pid: u64) -> Option<usize> {
        self.scheduler_states
            .iter()
            .position(|entry| entry.pid == pid)
    }

    fn scheduler_state(&self, pid: u64) -> BootProcessSchedulerState {
        self.scheduler_state_index(pid)
            .map(|index| self.scheduler_states[index])
            .unwrap_or(BootProcessSchedulerState {
                pid,
                affinity_mask: boot_scheduler_online_mask(),
                assigned_cpu: boot_scheduler_default_cpu(pid),
            })
    }

    fn scheduler_state_mut_or_insert(&mut self, pid: u64) -> &mut BootProcessSchedulerState {
        if let Some(index) = self.scheduler_state_index(pid) {
            return &mut self.scheduler_states[index];
        }
        self.scheduler_states.push(BootProcessSchedulerState {
            pid,
            affinity_mask: boot_scheduler_online_mask(),
            assigned_cpu: boot_scheduler_default_cpu(pid),
        });
        let index = self.scheduler_states.len().saturating_sub(1);
        &mut self.scheduler_states[index]
    }

    fn rebalance_queued_processes(&mut self) {
        let cpu_count = boot_scheduler_cpu_count();
        let queued_pids = self
            .entries
            .iter()
            .filter(|entry| !entry.reaped && entry.state == 1)
            .map(|entry| entry.pid)
            .collect::<Vec<_>>();
        if cpu_count <= 1 || queued_pids.is_empty() {
            self.scheduler_events.last_rebalance_migrations = 0;
            return;
        }

        self.scheduler_events.rebalance_operations =
            self.scheduler_events.rebalance_operations.saturating_add(1);
        let mut cpu_loads = vec![0usize; cpu_count];
        for pid in &queued_pids {
            let assigned_cpu = self
                .scheduler_state(*pid)
                .assigned_cpu
                .min(cpu_count.saturating_sub(1));
            cpu_loads[assigned_cpu] = cpu_loads[assigned_cpu].saturating_add(1);
        }

        let mut migrated = 0u64;
        let mut last_rebalanced = None::<(u64, usize, usize)>;
        for pid in queued_pids {
            let current = self.scheduler_state(pid);
            let current_cpu = current.assigned_cpu.min(cpu_count.saturating_sub(1));
            let mut best_cpu = current_cpu;
            let mut best_load = cpu_loads[current_cpu];
            for candidate_cpu in 0..cpu_count {
                if (current.affinity_mask & (1u64 << candidate_cpu)) == 0 {
                    continue;
                }
                let candidate_load = cpu_loads[candidate_cpu];
                if candidate_load < best_load {
                    best_cpu = candidate_cpu;
                    best_load = candidate_load;
                }
            }
            if best_cpu == current_cpu {
                continue;
            }
            let current_load = cpu_loads[current_cpu];
            if current_load <= best_load.saturating_add(1) {
                continue;
            }
            let state = self.scheduler_state_mut_or_insert(pid);
            state.assigned_cpu = best_cpu;
            cpu_loads[current_cpu] = cpu_loads[current_cpu].saturating_sub(1);
            cpu_loads[best_cpu] = cpu_loads[best_cpu].saturating_add(1);
            migrated = migrated.saturating_add(1);
            last_rebalanced = Some((pid, current_cpu, best_cpu));
        }

        self.scheduler_events.last_rebalance_migrations = migrated;
        self.scheduler_events.rebalance_migrations = self
            .scheduler_events
            .rebalance_migrations
            .saturating_add(migrated);
        if let Some((pid, from_cpu, to_cpu)) = last_rebalanced {
            self.scheduler_events.last_rebalance_pid = pid;
            self.scheduler_events.last_rebalance_from_cpu = from_cpu;
            self.scheduler_events.last_rebalance_to_cpu = to_cpu;
        }
    }

    fn spawn(
        &mut self,
        name: String,
        image_path: String,
        cwd: String,
        descriptors: Vec<BootProcessDescriptorRecord>,
        argv: Vec<String>,
        envp: Vec<String>,
    ) -> Result<u64, Errno> {
        if self.entries.iter().filter(|entry| !entry.reaped).count() >= MAX_PROCESS_COUNT {
            return Err(Errno::Again);
        }
        let parent_identity = self
            .find_index(1)
            .map(|index| {
                let parent = &self.entries[index];
                (
                    parent.uid,
                    parent.gid,
                    parent.umask,
                    parent.subject_label,
                    parent.supplemental_count,
                    parent.supplemental_gids,
                )
            })
            .unwrap_or((
                1000,
                1000,
                0o022,
                SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified),
                0,
                [0; 8],
            ));
        let pid = self.next_pid;
        self.next_pid = self.next_pid.saturating_add(1);
        self.entries.push(BootProcessEntry {
            pid,
            parent: 1,
            name,
            image_path,
            uid: parent_identity.0,
            gid: parent_identity.1,
            umask: parent_identity.2,
            subject_label: parent_identity.3,
            supplemental_count: parent_identity.4,
            supplemental_gids: parent_identity.5,
            root: String::from("/"),
            cwd,
            descriptors,
            argv_count: argv.len() as u64,
            env_count: envp.len() as u64,
            argv,
            envp,
            execution_mode: BootProcessExecutionMode::MetadataOnly,
            state: 1,
            exit_code: 0,
            pending_signal_count: 0,
            scheduler_class: NativeSchedulerClass::Interactive as u32,
            scheduler_budget: 2,
            cpu_runtime_ticks: 0,
            contract_bindings: BootProcessContractBindings::default(),
            next_vm_addr: 0x6000_0000,
            next_vm_object_id: 2,
            vm_objects: vec![BootVmObject {
                id: 1,
                start: 0x4000_0000,
                len: 0x2000,
                name: String::from("[heap]"),
                kind: "Heap",
                backing_inode: None,
                share_key: 1,
                shadow_source_id: None,
                shadow_source_offset: 0,
                shadow_depth: 0,
                private_mapping: true,
                file_offset: 0,
                bytes: vec![0; 0x2000],
                readable: true,
                writable: true,
                executable: false,
                read_fault_count: 0,
                write_fault_count: 0,
                cow_fault_count: 0,
                committed_pages: 2,
                resident_pages: 2,
                dirty_pages: 0,
                accessed_pages: 0,
                quarantined: false,
                quarantine_reason: 0,
                page_states: vec![
                    BootVmPageState {
                        resident: true,
                        dirty: false,
                        accessed: false,
                    };
                    2
                ],
            }],
            vm_decisions: Vec::new(),
            reaped: false,
        });
        let default_cpu = boot_scheduler_default_cpu(pid);
        self.scheduler_states.push(BootProcessSchedulerState {
            pid,
            affinity_mask: boot_scheduler_online_mask(),
            assigned_cpu: default_cpu,
        });
        self.rebalance_queued_processes();
        Ok(pid)
    }

    fn reap(&mut self, pid: u64) -> Result<(i32, BootProcessReapSummary), Errno> {
        if pid == 1 {
            return Err(Errno::Perm);
        }
        let Some(index) = self.find_index(pid) else {
            return Err(Errno::Srch);
        };
        if self.entries[index].state != 4 {
            return Err(Errno::Again);
        }
        let entry = self.entries.remove(index);
        Ok((
            entry.exit_code,
            BootProcessReapSummary {
                descriptors: entry.descriptors.len() as u64,
                env_records: entry.envp.len() as u64,
                vm_objects: entry.vm_objects.len() as u64,
                vm_decisions: entry.vm_decisions.len() as u64,
            },
        ))
    }
}

#[cfg(target_os = "none")]
pub(crate) fn seed_bootstrap_process_metadata(
    image_path: &str,
    cwd: &str,
    root: &str,
    argv: &[&str],
    envp: &[String],
) {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(1) else {
            return;
        };
        let entry = &mut registry.entries[index];
        entry.image_path = image_path.to_string();
        entry.cwd = cwd.to_string();
        entry.root = root.to_string();
        entry.argv = argv.iter().map(|value| (*value).to_string()).collect();
        entry.argv_count = entry.argv.len() as u64;
        entry.env_count = envp.len() as u64;
        entry.envp = envp.to_vec();
    });
}

fn set_active_process_pid(pid: u64) {
    ACTIVE_PROCESS_PID.store(pid, Ordering::Release);
}

#[cfg(target_os = "none")]
pub(crate) fn install_boot_process_exec_runtime(
    paging: ActivePageTables,
    allocator: BootFrameAllocator,
) {
    unsafe {
        ptr::addr_of_mut!(BOOT_PROCESS_EXEC_ALLOCATOR)
            .cast::<BootFrameAllocator>()
            .write(allocator);
    }
    BOOT_PROCESS_EXEC_ALLOCATOR_READY.store(true, Ordering::Release);
    BOOT_PROCESS_EXEC_RUNTIME.with_mut(|runtime| {
        runtime.root_phys = paging.root_phys();
        runtime.pending_reap_launch = 0;
        runtime.active = None;
    });
}

#[cfg(target_os = "none")]
fn request_blocking_reap_launch(pid: u64) {
    BOOT_PROCESS_EXEC_RUNTIME.with_mut(|runtime| {
        runtime.pending_reap_launch = pid;
    });
}

#[cfg(target_os = "none")]
fn take_blocking_reap_launch() -> Option<u64> {
    BOOT_PROCESS_EXEC_RUNTIME.with_mut(|runtime| {
        let pid = runtime.pending_reap_launch;
        runtime.pending_reap_launch = 0;
        (pid != 0).then_some(pid)
    })
}

#[cfg(target_os = "none")]
fn with_boot_exec_runtime<R>(
    f: impl FnOnce(&ActivePageTables, &mut BootFrameAllocator) -> Result<R, Errno>,
) -> Result<R, Errno> {
    BOOT_PROCESS_EXEC_RUNTIME.with_mut(|runtime| {
        if runtime.root_phys == 0 {
            return Err(Errno::NotSup);
        }
        let hhdm = crate::early_boot_info()
            .map(|boot_info| boot_info.physical_memory_offset)
            .ok_or(Errno::NotSup)?;
        let active_root = crate::cpu_features::read_cr3_local() & !0xfffu64;
        let paging = ActivePageTables::from_raw(active_root, hhdm);
        if !BOOT_PROCESS_EXEC_ALLOCATOR_READY.load(Ordering::Acquire) {
            return Err(Errno::NotSup);
        }
        let allocator = unsafe {
            &mut *ptr::addr_of_mut!(BOOT_PROCESS_EXEC_ALLOCATOR).cast::<BootFrameAllocator>()
        };
        f(&paging, allocator)
    })
}

#[cfg(target_os = "none")]
fn set_active_blocking_child(execution: BlockingChildExecution) {
    BOOT_PROCESS_EXEC_RUNTIME.with_mut(|runtime| runtime.active = Some(execution));
}

#[cfg(target_os = "none")]
fn take_active_blocking_child() -> Option<BlockingChildExecution> {
    BOOT_PROCESS_EXEC_RUNTIME.with_mut(|runtime| runtime.active.take())
}

#[cfg(target_os = "none")]
fn clear_boot_exec_runtime_state() {
    BOOT_PROCESS_EXEC_RUNTIME.with_mut(|runtime| {
        runtime.root_phys = 0;
        runtime.pending_reap_launch = 0;
        runtime.active = None;
    });
    BOOT_PROCESS_EXEC_ALLOCATOR_READY.store(false, Ordering::Release);
}

fn copy_struct_from_user<T: Copy>(ptr_value: usize) -> Result<T, Errno> {
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    Ok(unsafe { ptr::read(ptr_value as *const T) })
}

fn string_from_user(ptr_value: usize, len: usize) -> Result<String, Errno> {
    let text = path_from_user(ptr_value, len)?;
    Ok(String::from(text))
}

fn string_table_from_user(
    ptr_value: usize,
    len: usize,
    count: usize,
) -> Result<Vec<String>, Errno> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    let bytes = unsafe { slice::from_raw_parts(ptr_value as *const u8, len) };
    let mut values = Vec::with_capacity(count);
    let mut start = 0usize;
    for index in 0..len {
        if bytes[index] != 0 {
            continue;
        }
        let value = core::str::from_utf8(&bytes[start..index]).map_err(|_| Errno::Inval)?;
        values.push(String::from(value));
        start = index + 1;
        if values.len() == count {
            break;
        }
    }
    if values.len() != count {
        return Err(Errno::Inval);
    }
    Ok(values)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InlineName {
    len: usize,
    bytes: [u8; MAX_NAME_LEN],
}

impl InlineName {
    const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; MAX_NAME_LEN],
        }
    }

    fn from_text(text: &str) -> Result<Self, Errno> {
        if text.is_empty() || text.len() > MAX_NAME_LEN {
            return Err(Errno::Inval);
        }
        let mut name = Self::empty();
        name.len = text.len();
        name.bytes[..text.len()].copy_from_slice(text.as_bytes());
        Ok(name)
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DomainEntry {
    id: u64,
    owner: u64,
    parent: u64,
    name: InlineName,
    resource_count: u64,
    contract_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResourceEntry {
    id: u64,
    domain: u64,
    creator: u64,
    name: InlineName,
    kind: NativeResourceKind,
    state: NativeResourceState,
    arbitration: NativeResourceArbitrationPolicy,
    governance: NativeResourceGovernanceMode,
    contract_policy: NativeResourceContractPolicy,
    issuer_policy: NativeResourceIssuerPolicy,
    holder_contract: u64,
    waiting_count: usize,
    waiters: [u64; MAX_CONTRACT_COUNT],
    acquire_count: u64,
    handoff_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContractEntry {
    id: u64,
    domain: u64,
    resource: u64,
    issuer: u64,
    kind: NativeContractKind,
    state: NativeContractState,
    label: InlineName,
    invocation_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeRegistry {
    domains: [Option<DomainEntry>; MAX_DOMAIN_COUNT],
    resources: [Option<ResourceEntry>; MAX_RESOURCE_COUNT],
    contracts: [Option<ContractEntry>; MAX_CONTRACT_COUNT],
}

impl NativeRegistry {
    const fn new() -> Self {
        Self {
            domains: [None; MAX_DOMAIN_COUNT],
            resources: [None; MAX_RESOURCE_COUNT],
            contracts: [None; MAX_CONTRACT_COUNT],
        }
    }

    fn create_domain(&mut self, parent: u64, name: InlineName) -> Result<usize, Errno> {
        if parent != 0 && self.domain(parent as usize).is_err() {
            return Err(Errno::Inval);
        }
        let slot = self
            .domains
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.domains[slot] = Some(DomainEntry {
            id,
            owner: BOOT_OWNER_ID,
            parent,
            name,
            resource_count: 0,
            contract_count: 0,
        });
        Ok(id as usize)
    }

    fn create_resource(
        &mut self,
        domain: usize,
        kind: NativeResourceKind,
        name: InlineName,
    ) -> Result<usize, Errno> {
        let domain_slot = self.domain_slot(domain)?;
        let slot = self
            .resources
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.resources[slot] = Some(ResourceEntry {
            id,
            domain: domain as u64,
            creator: BOOT_OWNER_ID,
            name,
            kind,
            state: NativeResourceState::Active,
            arbitration: NativeResourceArbitrationPolicy::Fifo,
            governance: NativeResourceGovernanceMode::Queueing,
            contract_policy: NativeResourceContractPolicy::Any,
            issuer_policy: NativeResourceIssuerPolicy::AnyIssuer,
            holder_contract: 0,
            waiting_count: 0,
            waiters: [0; MAX_CONTRACT_COUNT],
            acquire_count: 0,
            handoff_count: 0,
        });
        self.domains[domain_slot].as_mut().unwrap().resource_count += 1;
        Ok(id as usize)
    }

    fn create_contract(
        &mut self,
        domain: usize,
        resource: usize,
        kind: NativeContractKind,
        label: InlineName,
    ) -> Result<usize, Errno> {
        let domain_slot = self.domain_slot(domain)?;
        let resource_slot = self.resource_slot(resource)?;
        {
            let resource_entry = self.resources[resource_slot].as_ref().unwrap();
            if resource_entry.domain != domain as u64 {
                return Err(Errno::Inval);
            }
            if resource_entry.state != NativeResourceState::Active {
                return Err(Errno::Access);
            }
            if !contract_kind_allowed(resource_entry.contract_policy, kind) {
                return Err(Errno::Access);
            }
            let domain_owner = self.domains[domain_slot].as_ref().unwrap().owner;
            if !issuer_allowed(
                resource_entry.issuer_policy,
                resource_entry.creator,
                domain_owner,
                BOOT_OWNER_ID,
            ) {
                return Err(Errno::Access);
            }
        }
        let slot = self
            .contracts
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.contracts[slot] = Some(ContractEntry {
            id,
            domain: domain as u64,
            resource: resource as u64,
            issuer: BOOT_OWNER_ID,
            kind,
            state: NativeContractState::Active,
            label,
            invocation_count: 0,
        });
        self.domains[domain_slot].as_mut().unwrap().contract_count += 1;
        Ok(id as usize)
    }

    fn domain_slot(&self, id: usize) -> Result<usize, Errno> {
        if id == 0 || id > MAX_DOMAIN_COUNT {
            return Err(Errno::Inval);
        }
        self.domains[id - 1].ok_or(Errno::Inval).map(|_| id - 1)
    }

    fn resource_slot(&self, id: usize) -> Result<usize, Errno> {
        if id == 0 || id > MAX_RESOURCE_COUNT {
            return Err(Errno::Inval);
        }
        self.resources[id - 1].ok_or(Errno::Inval).map(|_| id - 1)
    }

    fn contract_slot(&self, id: usize) -> Result<usize, Errno> {
        if id == 0 || id > MAX_CONTRACT_COUNT {
            return Err(Errno::Inval);
        }
        self.contracts[id - 1].ok_or(Errno::Inval).map(|_| id - 1)
    }

    fn domain(&self, id: usize) -> Result<&DomainEntry, Errno> {
        Ok(self.domains[self.domain_slot(id)?].as_ref().unwrap())
    }

    fn resource(&self, id: usize) -> Result<&ResourceEntry, Errno> {
        Ok(self.resources[self.resource_slot(id)?].as_ref().unwrap())
    }

    fn contract(&self, id: usize) -> Result<&ContractEntry, Errno> {
        Ok(self.contracts[self.contract_slot(id)?].as_ref().unwrap())
    }

    fn contract_and_resource_slots(&self, contract: usize) -> Result<(usize, usize), Errno> {
        let contract_slot = self.contract_slot(contract)?;
        let resource = self.contracts[contract_slot].as_ref().unwrap().resource as usize;
        let resource_slot = self.resource_slot(resource)?;
        Ok((contract_slot, resource_slot))
    }

    fn remove_waiter_at(resource: &mut ResourceEntry, index: usize) {
        let count = resource.waiting_count;
        let mut cursor = index;
        while cursor + 1 < count {
            resource.waiters[cursor] = resource.waiters[cursor + 1];
            cursor += 1;
        }
        if count > 0 {
            resource.waiters[count - 1] = 0;
            resource.waiting_count -= 1;
        }
    }

    fn remove_waiter(resource: &mut ResourceEntry, contract: u64) -> bool {
        if let Some(index) = resource.waiters[..resource.waiting_count]
            .iter()
            .position(|id| *id == contract)
        {
            Self::remove_waiter_at(resource, index);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn next_waiter_index(&self, resource: &ResourceEntry) -> Option<usize> {
        if resource.waiting_count == 0 {
            return None;
        }
        match resource.arbitration {
            NativeResourceArbitrationPolicy::Fifo => Some(0),
            NativeResourceArbitrationPolicy::Lifo => Some(resource.waiting_count - 1),
        }
    }

    fn select_handoff_waiter(&mut self, resource_slot: usize) -> Option<u64> {
        loop {
            let (waiting_count, arbitration, state) = {
                let resource = self.resources[resource_slot].as_ref().unwrap();
                (resource.waiting_count, resource.arbitration, resource.state)
            };
            let index = if waiting_count == 0 {
                return None;
            } else {
                match arbitration {
                    NativeResourceArbitrationPolicy::Fifo => 0,
                    NativeResourceArbitrationPolicy::Lifo => waiting_count - 1,
                }
            };
            let contract_id = {
                let resource = self.resources[resource_slot].as_mut().unwrap();
                let id = resource.waiters[index];
                Self::remove_waiter_at(resource, index);
                id
            };
            let Ok(contract_slot) = self.contract_slot(contract_id as usize) else {
                continue;
            };
            let contract = self.contracts[contract_slot].as_ref().unwrap();
            if contract.state != NativeContractState::Active {
                continue;
            }
            if state != NativeResourceState::Active {
                return None;
            }
            let contract_allowed = {
                let resource = self.resources[resource_slot].as_ref().unwrap();
                contract_kind_allowed(resource.contract_policy, contract.kind)
            };
            if !contract_allowed {
                continue;
            }
            return Some(contract_id);
        }
    }
}

impl NativeRegistryCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(NativeRegistry::new()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut NativeRegistry) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }

    fn with<R>(&self, f: impl FnOnce(&NativeRegistry) -> R) -> R {
        unsafe { f(&*self.0.get()) }
    }
}

impl BootBusRegistry {
    const fn new() -> Self {
        Self {
            peers: [None; MAX_BUS_PEER_COUNT],
            endpoints: [const { None }; MAX_BUS_ENDPOINT_COUNT],
        }
    }

    fn create_peer(&mut self, owner: u64, domain: u64, name: InlineName) -> Result<u64, Errno> {
        let slot = self
            .peers
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.peers[slot] = Some(BootBusPeerEntry {
            id,
            owner,
            domain,
            name,
            attached_endpoint_count: 0,
            readable_endpoint_count: 0,
            writable_endpoint_count: 0,
            publish_count: 0,
            receive_count: 0,
            last_endpoint: 0,
        });
        Ok(id)
    }

    fn create_endpoint(&mut self, domain: u64, resource: u64, path: String) -> Result<u64, Errno> {
        let slot = self
            .endpoints
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.endpoints[slot] = Some(BootBusEndpointEntry {
            id,
            domain,
            resource,
            path,
            attached_peers: Vec::new(),
            queue: Vec::new(),
            publish_count: 0,
            receive_count: 0,
            byte_count: 0,
            peak_queue_depth: 0,
            overflow_count: 0,
            last_peer: 0,
        });
        Ok(id)
    }

    fn peer(&self, id: u64) -> Result<&BootBusPeerEntry, Errno> {
        if id == 0 || id as usize > MAX_BUS_PEER_COUNT {
            return Err(Errno::Inval);
        }
        self.peers[id as usize - 1].as_ref().ok_or(Errno::Inval)
    }

    fn peer_mut(&mut self, id: u64) -> Result<&mut BootBusPeerEntry, Errno> {
        if id == 0 || id as usize > MAX_BUS_PEER_COUNT {
            return Err(Errno::Inval);
        }
        self.peers[id as usize - 1].as_mut().ok_or(Errno::Inval)
    }

    fn endpoint(&self, id: u64) -> Result<&BootBusEndpointEntry, Errno> {
        if id == 0 || id as usize > MAX_BUS_ENDPOINT_COUNT {
            return Err(Errno::Inval);
        }
        self.endpoints[id as usize - 1].as_ref().ok_or(Errno::Inval)
    }

    fn endpoint_mut(&mut self, id: u64) -> Result<&mut BootBusEndpointEntry, Errno> {
        if id == 0 || id as usize > MAX_BUS_ENDPOINT_COUNT {
            return Err(Errno::Inval);
        }
        self.endpoints[id as usize - 1].as_mut().ok_or(Errno::Inval)
    }
}

impl BootBusRegistryCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(BootBusRegistry::new()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootBusRegistry) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }

    fn with<R>(&self, f: impl FnOnce(&BootBusRegistry) -> R) -> R {
        unsafe { f(&*self.0.get()) }
    }
}

fn contract_kind_allowed(policy: NativeResourceContractPolicy, kind: NativeContractKind) -> bool {
    match policy {
        NativeResourceContractPolicy::Any => true,
        NativeResourceContractPolicy::Execution => kind == NativeContractKind::Execution,
        NativeResourceContractPolicy::Memory => kind == NativeContractKind::Memory,
        NativeResourceContractPolicy::Io => kind == NativeContractKind::Io,
        NativeResourceContractPolicy::Device => kind == NativeContractKind::Device,
        NativeResourceContractPolicy::Display => kind == NativeContractKind::Display,
        NativeResourceContractPolicy::Observe => kind == NativeContractKind::Observe,
    }
}

fn issuer_allowed(
    policy: NativeResourceIssuerPolicy,
    creator: u64,
    domain_owner: u64,
    issuer: u64,
) -> bool {
    match policy {
        NativeResourceIssuerPolicy::AnyIssuer => true,
        NativeResourceIssuerPolicy::CreatorOnly => issuer == creator,
        NativeResourceIssuerPolicy::DomainOwnerOnly => issuer == domain_owner,
    }
}

fn read_inline_name(ptr_value: usize, len: usize) -> Result<InlineName, Errno> {
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    if len == 0 || len > MAX_NAME_LEN {
        return Err(Errno::Inval);
    }
    let source = unsafe { slice::from_raw_parts(ptr_value as *const u8, len) };
    let mut name = InlineName::empty();
    name.len = len;
    name.bytes[..len].copy_from_slice(source);
    Ok(name)
}

fn copy_ids_to_user(ids: &[u64], buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    if capacity == 0 {
        return Ok(ids.len());
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let copy_len = ids.len().min(capacity);
    unsafe {
        ptr::copy_nonoverlapping(ids.as_ptr(), buffer, copy_len);
    }
    Ok(ids.len())
}

fn copy_name_to_user(name: &InlineName, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    if capacity == 0 {
        return Ok(name.len);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let copy_len = name.len.min(capacity);
    unsafe {
        ptr::copy_nonoverlapping(name.as_bytes().as_ptr(), buffer, copy_len);
    }
    Ok(copy_len)
}

fn copy_text_to_user(text: &str, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    if capacity == 0 {
        return Ok(text.len());
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let copy_len = text.len().min(capacity);
    unsafe {
        ptr::copy_nonoverlapping(text.as_ptr(), buffer, copy_len);
    }
    Ok(copy_len)
}

fn write_record<T: Copy>(dst: *mut T, value: T) -> Result<(), Errno> {
    if dst.is_null() {
        return Err(Errno::Fault);
    }
    unsafe {
        ptr::write(dst, value);
    }
    Ok(())
}

fn resource_watch_matches(watch: &ResourceEventWatch, kind: BootResourceEventKind) -> bool {
    match kind {
        BootResourceEventKind::Claimed => watch.claimed,
        BootResourceEventKind::Queued => watch.queued,
        BootResourceEventKind::Canceled => watch.canceled,
        BootResourceEventKind::Released => watch.released,
        BootResourceEventKind::HandedOff => watch.handed_off,
        BootResourceEventKind::Revoked => watch.revoked,
    }
}

fn vfs_watch_matches(watch: &VfsEventWatch, kind: NativeVfsEventKind) -> bool {
    match kind {
        NativeVfsEventKind::Created => watch.created,
        NativeVfsEventKind::Opened => watch.opened,
        NativeVfsEventKind::Closed => watch.closed,
        NativeVfsEventKind::Written => watch.written,
        NativeVfsEventKind::Renamed => watch.renamed,
        NativeVfsEventKind::Unlinked => watch.unlinked,
        NativeVfsEventKind::Mounted => watch.mounted,
        NativeVfsEventKind::Unmounted => watch.unmounted,
        NativeVfsEventKind::LockAcquired => watch.lock_acquired,
        NativeVfsEventKind::LockRefused => watch.lock_refused,
        NativeVfsEventKind::PermissionRefused => watch.permission_refused,
        NativeVfsEventKind::Truncated => watch.truncated,
        NativeVfsEventKind::Linked => watch.linked,
    }
}

fn network_watch_matches(watch: &NetworkEventWatch, kind: NativeNetworkEventKind) -> bool {
    match kind {
        NativeNetworkEventKind::LinkChanged => watch.link_changed,
        NativeNetworkEventKind::RxReady => watch.rx_ready,
        NativeNetworkEventKind::TxDrained => watch.tx_drained,
    }
}

fn bus_watch_matches(watch: &BusEventWatch, kind: BootBusEventKind) -> bool {
    match kind {
        BootBusEventKind::Attached => watch.attached,
        BootBusEventKind::Detached => watch.detached,
        BootBusEventKind::Published => watch.published,
        BootBusEventKind::Received => watch.received,
    }
}

fn bus_attachment_rights(raw: u64) -> Result<BlockRightsMask, Errno> {
    let default_rights = BlockRightsMask::READ.union(BlockRightsMask::WRITE);
    let rights = if raw == 0 {
        default_rights
    } else {
        BlockRightsMask(raw)
    };
    let allowed = default_rights;
    if !allowed.contains(rights) || !rights.intersects(default_rights) {
        return Err(Errno::Inval);
    }
    Ok(rights)
}

fn bus_attachment_contains(rights: u64, required: BlockRightsMask) -> bool {
    BlockRightsMask(rights).contains(required)
}

fn queue_pending_mask(queue_id: usize) -> Result<u32, Errno> {
    BOOT_EVENT_QUEUES.with(|registry| {
        let queue = registry.queue(queue_id)?;
        Ok(queue
            .pending
            .iter()
            .fold(0u32, |mask, event| mask | event.events))
    })
}

fn emit_resource_event(resource: u64, contract: u64, kind: BootResourceEventKind) {
    BOOT_EVENT_QUEUES.with_mut(|registry| {
        for queue in registry.queues.iter_mut().flatten() {
            let matching = queue
                .resource_watches
                .iter()
                .filter(|watch| watch.resource == resource && resource_watch_matches(watch, kind))
                .map(|watch| NativeEventRecord {
                    token: watch.token,
                    events: watch.events,
                    source_kind: NativeEventSourceKind::Resource as u32,
                    source_arg0: resource,
                    source_arg1: contract,
                    source_arg2: 0,
                    detail0: kind as u32,
                    detail1: 0,
                })
                .collect::<Vec<_>>();
            for event in matching {
                let _ = BootEventQueueRegistry::push_event(queue, event);
            }
        }
    });
}

pub(crate) fn emit_network_event(
    interface_path: &str,
    socket_path: Option<&str>,
    kind: NativeNetworkEventKind,
) {
    let interface_id = crate::boot_network_runtime::endpoint_for_path(interface_path)
        .map(|_| crate::boot_network_runtime::endpoint_id(interface_path))
        .unwrap_or(0);
    let socket_id = socket_path.map_or(0, crate::boot_network_runtime::socket_id);
    BOOT_EVENT_QUEUES.with_mut(|registry| {
        for queue in registry.queues.iter_mut().flatten() {
            let matching = queue
                .network_watches
                .iter()
                .filter(|watch| {
                    watch.interface_path == interface_path
                        && (watch.socket_path.is_none()
                            || watch.socket_path.as_deref() == socket_path)
                        && network_watch_matches(watch, kind)
                })
                .map(|watch| NativeEventRecord {
                    token: watch.token,
                    events: watch.events,
                    source_kind: NativeEventSourceKind::Network as u32,
                    source_arg0: interface_id,
                    source_arg1: socket_id,
                    source_arg2: 0,
                    detail0: socket_path.is_some() as u32,
                    detail1: kind as u32,
                })
                .collect::<Vec<_>>();
            for event in matching {
                let _ = BootEventQueueRegistry::push_event(queue, event);
            }
        }
    });
}

fn emit_bus_event(peer: u64, endpoint: u64, kind: BootBusEventKind) {
    BOOT_EVENT_QUEUES.with_mut(|registry| {
        for queue in registry.queues.iter_mut().flatten() {
            let matching = queue
                .bus_watches
                .iter()
                .filter(|watch| watch.endpoint == endpoint && bus_watch_matches(watch, kind))
                .map(|watch| NativeEventRecord {
                    token: watch.token,
                    events: watch.events,
                    source_kind: NativeEventSourceKind::Bus as u32,
                    source_arg0: peer,
                    source_arg1: endpoint,
                    source_arg2: 0,
                    detail0: kind as u32,
                    detail1: 0,
                })
                .collect::<Vec<_>>();
            for event in matching {
                let _ = BootEventQueueRegistry::push_event(queue, event);
            }
        }
    });
}

fn emit_vfs_event(
    inode: u64,
    path: Option<&str>,
    aux_path: Option<&str>,
    kind: NativeVfsEventKind,
    detail1: u32,
) {
    let has_vfs_watches = BOOT_EVENT_QUEUES.with(|registry| {
        registry
            .queues
            .iter()
            .flatten()
            .any(|queue| !queue.vfs_watches.is_empty())
    });
    if !has_vfs_watches {
        BOOT_VFS.with_mut(|vfs| {
            vfs.stats.vfs_events_emitted += 1;
        });
        return;
    }
    let mut delivered = 0u64;
    let mut filtered = 0u64;
    let mut overflows = 0u64;
    let mut coalesced = 0u64;
    let mut pending_peak = 0u64;
    BOOT_EVENT_QUEUES.with_mut(|registry| {
        for queue in registry.queues.iter_mut().flatten() {
            let object_label = BOOT_VFS.with_mut(|vfs| vfs.object_current_label_by_inode(inode));
            let matching = queue
                .vfs_watches
                .iter()
                .filter(|watch| {
                    let Some(subject_label) = process_subject_label(watch.owner_pid) else {
                        return false;
                    };
                    if object_label
                        .is_some_and(|label| check_ifc_read(subject_label, label).is_err())
                    {
                        filtered += 1;
                        return false;
                    }
                    if !vfs_watch_matches(watch, kind) {
                        return false;
                    }
                    if watch.inode == inode {
                        return true;
                    }
                    if !watch.subtree {
                        return false;
                    }
                    let root_path = BOOT_VFS
                        .with_mut(|vfs| vfs.live_path_for_inode(watch.inode))
                        .or_else(|| watch.anchor_path.clone());
                    let Some(root_path) = root_path else {
                        return false;
                    };
                    let root_prefix = format!("{root_path}/");
                    let matches_path = |candidate: &str| {
                        candidate == root_path || candidate.starts_with(&root_prefix)
                    };
                    path.is_some_and(matches_path) || aux_path.is_some_and(matches_path)
                })
                .map(|watch| NativeEventRecord {
                    token: watch.token,
                    events: watch.events,
                    source_kind: NativeEventSourceKind::Vfs as u32,
                    source_arg0: inode,
                    source_arg1: 0,
                    source_arg2: 0,
                    detail0: kind as u32,
                    detail1,
                })
                .collect::<Vec<_>>();
            for event in matching {
                let (overflowed, merged) = BootEventQueueRegistry::push_event(queue, event);
                if overflowed {
                    overflows += 1;
                }
                if merged {
                    coalesced += 1;
                } else {
                    delivered += 1;
                }
                pending_peak = pending_peak.max(queue.pending_peak as u64);
            }
        }
    });
    BOOT_VFS.with_mut(|vfs| {
        vfs.stats.vfs_events_emitted += 1;
        vfs.stats.vfs_events_delivered += delivered;
        vfs.stats.vfs_events_filtered += filtered;
        vfs.stats.vfs_event_queue_overflows += overflows;
        vfs.stats.vfs_events_coalesced += coalesced;
        vfs.stats.vfs_pending_peak = vfs.stats.vfs_pending_peak.max(pending_peak);
    });
}

fn emit_permission_refusal_for_path(path: &str, errno: Errno) {
    if let Ok(inode) = BOOT_VFS.with_mut(|vfs| {
        let normalized = BootVfs::normalize_path(path)?;
        let index = vfs.resolve_node_index(&normalized, false)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    }) {
        emit_vfs_event(
            inode,
            Some(path),
            None,
            NativeVfsEventKind::PermissionRefused,
            errno as u32,
        );
    }
}

fn emit_permission_refusal_for_parent_path(path: &str, errno: Errno) {
    if let Ok(parent) = BootVfs::normalize_path(BootVfs::parent_path(path)) {
        emit_permission_refusal_for_path(&parent, errno);
    }
}

impl DescriptorTable {
    const fn new() -> Self {
        Self {
            slots: [
                Some(DescriptorState {
                    description_id: 0,
                    cloexec: false,
                    rights: descriptor_default_rights(DescriptorTarget::Stdin),
                }),
                Some(DescriptorState {
                    description_id: 1,
                    cloexec: false,
                    rights: descriptor_default_rights(DescriptorTarget::Stdout),
                }),
                Some(DescriptorState {
                    description_id: 2,
                    cloexec: false,
                    rights: descriptor_default_rights(DescriptorTarget::Stderr),
                }),
                None,
                None,
                None,
                None,
                None,
            ],
            descriptions: [
                Some(DescriptorDescription {
                    target: DescriptorTarget::Stdin,
                    flags: DescriptorStatusFlags { nonblock: false },
                    offset: 0,
                }),
                Some(DescriptorDescription {
                    target: DescriptorTarget::Stdout,
                    flags: DescriptorStatusFlags { nonblock: false },
                    offset: 0,
                }),
                Some(DescriptorDescription {
                    target: DescriptorTarget::Stderr,
                    flags: DescriptorStatusFlags { nonblock: false },
                    offset: 0,
                }),
                None,
                None,
                None,
                None,
                None,
            ],
        }
    }

    fn descriptor_state(&self, fd: usize) -> Result<DescriptorState, Errno> {
        self.slots
            .get(fd)
            .and_then(|entry| *entry)
            .ok_or(Errno::Badf)
    }

    fn descriptor_state_mut(&mut self, fd: usize) -> Result<&mut DescriptorState, Errno> {
        self.slots
            .get_mut(fd)
            .and_then(Option::as_mut)
            .ok_or(Errno::Badf)
    }

    fn description(&self, description_id: usize) -> Result<DescriptorDescription, Errno> {
        self.descriptions
            .get(description_id)
            .and_then(|entry| *entry)
            .ok_or(Errno::Badf)
    }

    fn description_mut(
        &mut self,
        description_id: usize,
    ) -> Result<&mut DescriptorDescription, Errno> {
        self.descriptions
            .get_mut(description_id)
            .and_then(Option::as_mut)
            .ok_or(Errno::Badf)
    }

    fn descriptor(&self, fd: usize) -> Result<DescriptorSnapshot, Errno> {
        let handle = self.descriptor_state(fd)?;
        let description = self.description(handle.description_id)?;
        Ok(DescriptorSnapshot {
            description_id: handle.description_id,
            target: description.target,
            nonblock: description.flags.nonblock,
            cloexec: handle.cloexec,
            offset: description.offset,
            rights: handle.rights,
        })
    }

    fn alloc_description(&mut self, target: DescriptorTarget) -> Result<usize, Errno> {
        let free_description = self
            .descriptions
            .iter()
            .enumerate()
            .find_map(|(index, slot)| slot.is_none().then_some(index))
            .ok_or(Errno::Again)?;
        self.descriptions[free_description] = Some(DescriptorDescription {
            target,
            flags: DescriptorStatusFlags { nonblock: false },
            offset: 0,
        });
        Ok(free_description)
    }

    fn duplicate(&mut self, fd: usize) -> Result<usize, Errno> {
        let descriptor = self.descriptor_state(fd)?;
        let free_fd = self
            .slots
            .iter()
            .enumerate()
            .skip(3)
            .find_map(|(index, slot)| slot.is_none().then_some(index))
            .ok_or(Errno::Again)?;
        self.slots[free_fd] = Some(DescriptorState {
            description_id: descriptor.description_id,
            cloexec: false,
            rights: descriptor.rights,
        });
        Ok(free_fd)
    }

    fn require_rights(&self, fd: usize, required: BlockRightsMask) -> Result<(), Errno> {
        let handle = self.descriptor_state(fd)?;
        if handle.rights.contains(required) {
            Ok(())
        } else {
            Err(Errno::Access)
        }
    }

    fn restrict_rights(&mut self, fd: usize, rights: BlockRightsMask) -> Result<usize, Errno> {
        let handle = self.descriptor_state_mut(fd)?;
        if !handle.rights.contains(rights) {
            return Err(Errno::Access);
        }
        handle.rights = rights;
        Ok(rights.0 as usize)
    }

    fn close(&mut self, fd: usize) -> Result<(), Errno> {
        let descriptor = self.descriptor_state(fd)?;
        let description_id = descriptor.description_id;
        let target = self.description(description_id)?.target;
        let slot = self.slots.get_mut(fd).ok_or(Errno::Badf)?;
        *slot = None;
        if !self
            .slots
            .iter()
            .flatten()
            .any(|candidate| candidate.description_id == description_id)
        {
            VFS_LOCKS.with_mut(|locks| locks.retain(|lock| lock.owner_fd != description_id));
            if let Some(inode) = descriptor_lock_inode(target) {
                BOOT_VFS.with_mut(|vfs| vfs.release_orphan_inode_if_unreferenced(inode));
            }
            if let Some(entry) = self.descriptions.get_mut(description_id) {
                *entry = None;
            }
        }
        if let DescriptorTarget::EventQueue(queue_id) = target {
            let still_open = self.slots.iter().flatten().any(|descriptor| {
                self.description(descriptor.description_id)
                    .map(|entry| entry.target == DescriptorTarget::EventQueue(queue_id))
                    .unwrap_or(false)
            });
            if !still_open {
                BOOT_EVENT_QUEUES.with_mut(|registry| registry.remove_queue(queue_id));
            }
        }
        Ok(())
    }

    fn references_inode(&self, inode: u64) -> bool {
        self.slots.iter().flatten().any(|descriptor| {
            self.description(descriptor.description_id)
                .map(|entry| descriptor_lock_inode(entry.target) == Some(inode))
                .unwrap_or(false)
        })
    }

    fn fcntl(&mut self, fd: usize, encoded: usize) -> Result<usize, Errno> {
        let command = decode_fcntl(encoded).ok_or(Errno::Inval)?;
        let description_id = self.descriptor_state(fd)?.description_id;
        match command {
            DecodedFcntl::GetFl => Ok(encode_flags(DescriptorFlags {
                nonblock: self.description(description_id)?.flags.nonblock,
                cloexec: false,
            })),
            DecodedFcntl::GetFd => Ok(encode_flags(DescriptorFlags {
                nonblock: false,
                cloexec: self.descriptor_state(fd)?.cloexec,
            })),
            DecodedFcntl::SetFl { nonblock } => {
                self.description_mut(description_id)?.flags.nonblock = nonblock;
                Ok(encode_flags(DescriptorFlags {
                    nonblock,
                    cloexec: false,
                }))
            }
            DecodedFcntl::SetFd { cloexec } => {
                self.descriptor_state_mut(fd)?.cloexec = cloexec;
                Ok(encode_flags(DescriptorFlags {
                    nonblock: false,
                    cloexec,
                }))
            }
            DecodedFcntl::QueryLock => {
                let Some(inode) = descriptor_lock_inode(self.description(description_id)?.target)
                else {
                    return Err(Errno::Badf);
                };
                Ok(VFS_LOCKS.with_mut(|locks| {
                    locks
                        .iter()
                        .find(|lock| lock.inode == inode)
                        .map(|lock| lock.token as usize)
                        .unwrap_or(0)
                }))
            }
            DecodedFcntl::TryLockExclusive { token } => {
                if token == 0 {
                    return Err(Errno::Inval);
                }
                let Some(inode) = descriptor_lock_inode(self.description(description_id)?.target)
                else {
                    return Err(Errno::Badf);
                };
                let result = VFS_LOCKS.with_mut(|locks| {
                    if let Some(existing) = locks.iter().find(|lock| lock.inode == inode) {
                        if existing.owner_fd == description_id
                            && existing.token == token
                            && existing.mode == VfsLockMode::Exclusive
                        {
                            return Ok(token as usize);
                        }
                        return Err(Errno::Busy);
                    }
                    locks.push(VfsLockRecord {
                        inode,
                        owner_fd: description_id,
                        token,
                        mode: VfsLockMode::Exclusive,
                    });
                    Ok(token as usize)
                });
                match result {
                    Ok(value) => {
                        let path =
                            descriptor_target_path_text(self.description(description_id)?.target)
                                .ok();
                        emit_vfs_event(
                            inode,
                            path.as_deref(),
                            None,
                            NativeVfsEventKind::LockAcquired,
                            token as u32,
                        );
                        Ok(value)
                    }
                    Err(Errno::Busy) => {
                        let path =
                            descriptor_target_path_text(self.description(description_id)?.target)
                                .ok();
                        emit_vfs_event(
                            inode,
                            path.as_deref(),
                            None,
                            NativeVfsEventKind::LockRefused,
                            Errno::Busy as u32,
                        );
                        Err(Errno::Busy)
                    }
                    Err(error) => Err(error),
                }
            }
            DecodedFcntl::UnlockExclusive { token } => {
                let Some(inode) = descriptor_lock_inode(self.description(description_id)?.target)
                else {
                    return Err(Errno::Badf);
                };
                VFS_LOCKS.with_mut(|locks| {
                    let Some(index) = locks.iter().position(|lock| {
                        lock.inode == inode && lock.mode == VfsLockMode::Exclusive
                    }) else {
                        return Err(Errno::NoEnt);
                    };
                    let lock = locks[index];
                    if lock.owner_fd != description_id || lock.token != token {
                        return Err(Errno::Perm);
                    }
                    locks.remove(index);
                    Ok(token as usize)
                })
            }
            DecodedFcntl::TryLockShared { token } => {
                if token == 0 {
                    return Err(Errno::Inval);
                }
                let Some(inode) = descriptor_lock_inode(self.description(description_id)?.target)
                else {
                    return Err(Errno::Badf);
                };
                let result = VFS_LOCKS.with_mut(|locks| {
                    if locks.iter().any(|lock| {
                        lock.inode == inode
                            && lock.mode == VfsLockMode::Exclusive
                            && !(lock.owner_fd == description_id && lock.token == token)
                    }) {
                        return Err(Errno::Busy);
                    }
                    if let Some(existing) = locks.iter().find(|lock| {
                        lock.inode == inode
                            && lock.mode == VfsLockMode::Shared
                            && lock.owner_fd == description_id
                            && lock.token == token
                    }) {
                        return Ok(existing.token as usize);
                    }
                    locks.push(VfsLockRecord {
                        inode,
                        owner_fd: description_id,
                        token,
                        mode: VfsLockMode::Shared,
                    });
                    Ok(token as usize)
                });
                match result {
                    Ok(value) => {
                        let path =
                            descriptor_target_path_text(self.description(description_id)?.target)
                                .ok();
                        emit_vfs_event(
                            inode,
                            path.as_deref(),
                            None,
                            NativeVfsEventKind::LockAcquired,
                            token as u32,
                        );
                        Ok(value)
                    }
                    Err(Errno::Busy) => {
                        let path =
                            descriptor_target_path_text(self.description(description_id)?.target)
                                .ok();
                        emit_vfs_event(
                            inode,
                            path.as_deref(),
                            None,
                            NativeVfsEventKind::LockRefused,
                            Errno::Busy as u32,
                        );
                        Err(Errno::Busy)
                    }
                    Err(error) => Err(error),
                }
            }
            DecodedFcntl::UnlockShared { token } => {
                let Some(inode) = descriptor_lock_inode(self.description(description_id)?.target)
                else {
                    return Err(Errno::Badf);
                };
                VFS_LOCKS.with_mut(|locks| {
                    let Some(index) = locks.iter().position(|lock| {
                        lock.inode == inode
                            && lock.mode == VfsLockMode::Shared
                            && lock.owner_fd == description_id
                            && lock.token == token
                    }) else {
                        return Err(Errno::NoEnt);
                    };
                    locks.remove(index);
                    Ok(token as usize)
                })
            }
            DecodedFcntl::UpgradeLockExclusive { token } => {
                if token == 0 {
                    return Err(Errno::Inval);
                }
                let Some(inode) = descriptor_lock_inode(self.description(description_id)?.target)
                else {
                    return Err(Errno::Badf);
                };
                let result = VFS_LOCKS.with_mut(|locks| {
                    let Some(index) = locks.iter().position(|lock| {
                        lock.inode == inode
                            && lock.mode == VfsLockMode::Shared
                            && lock.owner_fd == description_id
                            && lock.token == token
                    }) else {
                        return Err(Errno::NoEnt);
                    };
                    if locks.iter().any(|lock| {
                        lock.inode == inode
                            && (lock.owner_fd != description_id
                                || lock.token != token
                                || lock.mode != VfsLockMode::Shared)
                    }) {
                        return Err(Errno::Busy);
                    }
                    locks[index].mode = VfsLockMode::Exclusive;
                    Ok(token as usize)
                });
                match result {
                    Ok(value) => {
                        let path =
                            descriptor_target_path_text(self.description(description_id)?.target)
                                .ok();
                        emit_vfs_event(
                            inode,
                            path.as_deref(),
                            None,
                            NativeVfsEventKind::LockAcquired,
                            token as u32,
                        );
                        Ok(value)
                    }
                    Err(Errno::Busy) => {
                        let path =
                            descriptor_target_path_text(self.description(description_id)?.target)
                                .ok();
                        emit_vfs_event(
                            inode,
                            path.as_deref(),
                            None,
                            NativeVfsEventKind::LockRefused,
                            Errno::Busy as u32,
                        );
                        Err(Errno::Busy)
                    }
                    Err(error) => Err(error),
                }
            }
            DecodedFcntl::DowngradeLockShared { token } => {
                if token == 0 {
                    return Err(Errno::Inval);
                }
                let Some(inode) = descriptor_lock_inode(self.description(description_id)?.target)
                else {
                    return Err(Errno::Badf);
                };
                VFS_LOCKS.with_mut(|locks| {
                    let Some(index) = locks.iter().position(|lock| {
                        lock.inode == inode
                            && lock.mode == VfsLockMode::Exclusive
                            && lock.owner_fd == description_id
                            && lock.token == token
                    }) else {
                        return Err(Errno::NoEnt);
                    };
                    locks[index].mode = VfsLockMode::Shared;
                    Ok(token as usize)
                })
            }
        }
    }

    fn poll(&self, fd: usize, interest: u32) -> Result<usize, Errno> {
        let descriptor = self.descriptor(fd)?;
        let available = match descriptor.target {
            DescriptorTarget::Stdin => tty::poll_mask_for_stdin(interest) as u32,
            DescriptorTarget::Stdout | DescriptorTarget::Stderr => {
                tty::poll_mask_for_output(interest) as u32
            }
            DescriptorTarget::EventQueue(queue_id) => queue_pending_mask(queue_id)?,
            DescriptorTarget::GpuDevice => crate::boot_gpu_runtime::poll(
                crate::boot_gpu_runtime::GpuEndpointKind::Device,
                interest,
            ) as u32,
            DescriptorTarget::GpuDriver => crate::boot_gpu_runtime::poll(
                crate::boot_gpu_runtime::GpuEndpointKind::Driver,
                interest,
            ) as u32,
            DescriptorTarget::AudioDevice => crate::boot_audio_runtime::poll(
                crate::boot_audio_runtime::AudioEndpointKind::Device,
                interest,
            ) as u32,
            DescriptorTarget::AudioDriver => crate::boot_audio_runtime::poll(
                crate::boot_audio_runtime::AudioEndpointKind::Driver,
                interest,
            ) as u32,
            DescriptorTarget::InputDevice => crate::boot_input_runtime::poll(
                crate::boot_input_runtime::InputEndpointKind::Device,
                interest,
            ) as u32,
            DescriptorTarget::InputDriver => crate::boot_input_runtime::poll(
                crate::boot_input_runtime::InputEndpointKind::Driver,
                interest,
            ) as u32,
            DescriptorTarget::NetworkDevice => {
                let hardware_ready = hardware_network_device_poll(interest);
                if hardware_ready != 0 || hardware_network_online() {
                    hardware_ready as u32
                } else {
                    crate::boot_network_runtime::poll(
                        crate::boot_network_runtime::NetworkEndpointKind::Device,
                        interest,
                    ) as u32
                }
            }
            DescriptorTarget::NetworkDriver => crate::boot_network_runtime::poll(
                crate::boot_network_runtime::NetworkEndpointKind::Driver,
                interest,
            ) as u32,
            DescriptorTarget::StorageDevice => crate::virtio_blk_boot::poll(
                crate::virtio_blk_boot::StorageEndpointKind::Device,
                interest,
            ) as u32,
            DescriptorTarget::StorageDriver => crate::virtio_blk_boot::poll(
                crate::virtio_blk_boot::StorageEndpointKind::Driver,
                interest,
            ) as u32,
            DescriptorTarget::BootDirectory(_) => 0,
            DescriptorTarget::BootFile(inode) | DescriptorTarget::BootChannel(inode) => {
                boot_vfs_poll(inode, descriptor.offset, interest)
            }
            DescriptorTarget::Procfs(node) => boot_procfs_poll(node, descriptor.offset, interest),
        };
        Ok((available & interest) as usize)
    }

    fn create_event_queue(&mut self, mode: NativeEventQueueMode) -> Result<usize, Errno> {
        let queue_id = BOOT_EVENT_QUEUES.with_mut(|registry| registry.create_queue(mode))?;
        let free_fd = self
            .slots
            .iter()
            .enumerate()
            .skip(3)
            .find_map(|(index, slot)| slot.is_none().then_some(index))
            .ok_or(Errno::Again)?;
        let description_id = self.alloc_description(DescriptorTarget::EventQueue(queue_id))?;
        self.slots[free_fd] = Some(DescriptorState {
            description_id,
            cloexec: false,
            rights: descriptor_default_rights(DescriptorTarget::EventQueue(queue_id)),
        });
        Ok(free_fd)
    }

    fn event_queue_descriptor(&self, fd: usize) -> Result<(usize, DescriptorStatusFlags), Errno> {
        let descriptor = self.descriptor(fd)?;
        match descriptor.target {
            DescriptorTarget::EventQueue(queue_id) => Ok((
                queue_id,
                DescriptorStatusFlags {
                    nonblock: descriptor.nonblock,
                },
            )),
            _ => Err(Errno::Badf),
        }
    }

    fn watch_resource_events(
        &mut self,
        fd: usize,
        resource: usize,
        config: NativeResourceEventWatchConfig,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            if let Some(existing) = queue
                .resource_watches
                .iter_mut()
                .find(|watch| watch.resource == resource as u64 && watch.token == config.token)
            {
                *existing = ResourceEventWatch {
                    resource: resource as u64,
                    token: config.token,
                    events: config.poll_events,
                    claimed: config.claimed != 0,
                    queued: config.queued != 0,
                    canceled: config.canceled != 0,
                    released: config.released != 0,
                    handed_off: config.handed_off != 0,
                    revoked: config.revoked != 0,
                };
                return Ok(());
            }
            if queue.resource_watches.len() >= MAX_EVENT_QUEUE_WATCH_COUNT {
                return Err(Errno::Again);
            }
            queue.resource_watches.push(ResourceEventWatch {
                resource: resource as u64,
                token: config.token,
                events: config.poll_events,
                claimed: config.claimed != 0,
                queued: config.queued != 0,
                canceled: config.canceled != 0,
                released: config.released != 0,
                handed_off: config.handed_off != 0,
                revoked: config.revoked != 0,
            });
            Ok(())
        })
    }

    fn watch_network_events(
        &mut self,
        fd: usize,
        interface_path: &str,
        socket_path: Option<&str>,
        config: NativeNetworkEventWatchConfig,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            if let Some(existing) = queue.network_watches.iter_mut().find(|watch| {
                watch.interface_path == interface_path
                    && watch.socket_path.as_deref() == socket_path
                    && watch.token == config.token
            }) {
                *existing = NetworkEventWatch {
                    interface_path: interface_path.to_string(),
                    socket_path: socket_path.map(String::from),
                    token: config.token,
                    events: config.poll_events,
                    link_changed: config.link_changed != 0,
                    rx_ready: config.rx_ready != 0,
                    tx_drained: config.tx_drained != 0,
                };
                return Ok(());
            }
            if queue.network_watches.len() >= MAX_EVENT_QUEUE_WATCH_COUNT {
                return Err(Errno::Again);
            }
            queue.network_watches.push(NetworkEventWatch {
                interface_path: interface_path.to_string(),
                socket_path: socket_path.map(String::from),
                token: config.token,
                events: config.poll_events,
                link_changed: config.link_changed != 0,
                rx_ready: config.rx_ready != 0,
                tx_drained: config.tx_drained != 0,
            });
            Ok(())
        })
    }

    fn watch_bus_events(
        &mut self,
        fd: usize,
        endpoint: u64,
        config: NativeBusEventWatchConfig,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            if let Some(existing) = queue
                .bus_watches
                .iter_mut()
                .find(|watch| watch.endpoint == endpoint && watch.token == config.token)
            {
                *existing = BusEventWatch {
                    endpoint,
                    token: config.token,
                    events: config.poll_events,
                    attached: config.attached != 0,
                    detached: config.detached != 0,
                    published: config.published != 0,
                    received: config.received != 0,
                };
                return Ok(());
            }
            if queue.bus_watches.len() >= MAX_EVENT_QUEUE_WATCH_COUNT {
                return Err(Errno::Again);
            }
            queue.bus_watches.push(BusEventWatch {
                endpoint,
                token: config.token,
                events: config.poll_events,
                attached: config.attached != 0,
                detached: config.detached != 0,
                published: config.published != 0,
                received: config.received != 0,
            });
            Ok(())
        })
    }

    fn watch_vfs_events_by_anchor(
        &mut self,
        fd: usize,
        inode: u64,
        kind: BootNodeKind,
        anchor_path: Option<String>,
        config: NativeVfsEventWatchConfig,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        if config.subtree != 0 && kind != BootNodeKind::Directory {
            return Err(Errno::NotDir);
        }
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            if let Some(existing) = queue
                .vfs_watches
                .iter_mut()
                .find(|watch| watch.inode == inode && watch.token == config.token)
            {
                *existing = VfsEventWatch {
                    inode,
                    token: config.token,
                    events: config.poll_events,
                    subtree: config.subtree != 0,
                    anchor_path: anchor_path.clone(),
                    owner_pid: active_process_pid()?,
                    created: config.created != 0,
                    opened: config.opened != 0,
                    closed: config.closed != 0,
                    written: config.written != 0,
                    renamed: config.renamed != 0,
                    unlinked: config.unlinked != 0,
                    mounted: config.mounted != 0,
                    unmounted: config.unmounted != 0,
                    lock_acquired: config.lock_acquired != 0,
                    lock_refused: config.lock_refused != 0,
                    permission_refused: config.permission_refused != 0,
                    truncated: config.truncated != 0,
                    linked: config.linked != 0,
                };
                return Ok(());
            }
            if queue.vfs_watches.len() >= MAX_EVENT_QUEUE_WATCH_COUNT {
                return Err(Errno::Again);
            }
            queue.vfs_watches.push(VfsEventWatch {
                inode,
                token: config.token,
                events: config.poll_events,
                subtree: config.subtree != 0,
                anchor_path,
                owner_pid: active_process_pid()?,
                created: config.created != 0,
                opened: config.opened != 0,
                closed: config.closed != 0,
                written: config.written != 0,
                renamed: config.renamed != 0,
                unlinked: config.unlinked != 0,
                mounted: config.mounted != 0,
                unmounted: config.unmounted != 0,
                lock_acquired: config.lock_acquired != 0,
                lock_refused: config.lock_refused != 0,
                permission_refused: config.permission_refused != 0,
                truncated: config.truncated != 0,
                linked: config.linked != 0,
            });
            Ok(())
        })
    }

    fn watch_vfs_events(
        &mut self,
        fd: usize,
        path: &str,
        config: NativeVfsEventWatchConfig,
    ) -> Result<(), Errno> {
        let normalized = BootVfs::normalize_path(path)?;
        let (inode, kind) = BOOT_VFS.with_mut(|vfs| {
            let include_self_directory = vfs
                .find_node(&normalized)
                .is_some_and(|index| vfs.nodes[index].kind == BootNodeKind::Directory);
            vfs.require_traversal_access(&normalized, include_self_directory)?;
            vfs.require_access(&normalized, true, false, include_self_directory)?;
            let index = vfs.resolve_node_index(&normalized, true)?;
            vfs.require_observe_inode(vfs.nodes[index].inode)?;
            Ok((vfs.nodes[index].inode, vfs.nodes[index].kind))
        })?;
        self.watch_vfs_events_by_anchor(fd, inode, kind, Some(normalized), config)
    }

    fn remove_resource_events(
        &mut self,
        fd: usize,
        resource: usize,
        token: u64,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let original = queue.resource_watches.len();
            queue
                .resource_watches
                .retain(|watch| !(watch.resource == resource as u64 && watch.token == token));
            if queue.resource_watches.len() == original {
                return Err(Errno::NoEnt);
            }
            Ok(())
        })
    }

    fn remove_network_events(
        &mut self,
        fd: usize,
        interface_path: &str,
        socket_path: Option<&str>,
        token: u64,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let original = queue.network_watches.len();
            queue.network_watches.retain(|watch| {
                !(watch.interface_path == interface_path
                    && watch.socket_path.as_deref() == socket_path
                    && watch.token == token)
            });
            if queue.network_watches.len() == original {
                return Err(Errno::NoEnt);
            }
            Ok(())
        })
    }

    fn remove_bus_events(&mut self, fd: usize, endpoint: u64, token: u64) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let original = queue.bus_watches.len();
            queue
                .bus_watches
                .retain(|watch| !(watch.endpoint == endpoint && watch.token == token));
            if queue.bus_watches.len() == original {
                return Err(Errno::NoEnt);
            }
            Ok(())
        })
    }

    fn remove_vfs_events(&mut self, fd: usize, path: &str, token: u64) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        let normalized = BootVfs::normalize_path(path)?;
        let inode = BOOT_VFS.with_mut(|vfs| {
            let index = vfs.resolve_node_index(&normalized, true)?;
            Ok(vfs.nodes[index].inode)
        })?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let original = queue.vfs_watches.len();
            queue
                .vfs_watches
                .retain(|watch| !(watch.inode == inode && watch.token == token));
            if queue.vfs_watches.len() == original {
                Err(Errno::NoEnt)
            } else {
                Ok(())
            }
        })
    }

    fn remove_vfs_events_by_anchor(
        &mut self,
        fd: usize,
        inode: u64,
        token: u64,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let original = queue.vfs_watches.len();
            queue
                .vfs_watches
                .retain(|watch| !(watch.inode == inode && watch.token == token));
            if queue.vfs_watches.len() == original {
                Err(Errno::NoEnt)
            } else {
                Ok(())
            }
        })
    }

    fn wait_event_queue(
        &mut self,
        fd: usize,
        buffer: *mut NativeEventRecord,
        capacity: usize,
    ) -> Result<usize, Errno> {
        if buffer.is_null() && capacity != 0 {
            return Err(Errno::Fault);
        }
        let (queue_id, flags) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let _ = queue.mode;
            if queue.pending.is_empty() {
                return Err(Errno::Again);
            }
            if capacity == 0 {
                return Ok(0);
            }
            let count = capacity.min(queue.pending.len());
            for (index, event) in queue.pending.drain(..count).enumerate() {
                unsafe {
                    ptr::write(buffer.add(index), event);
                }
            }
            if flags.nonblock && queue.pending.is_empty() {
                return Ok(count);
            }
            Ok(count)
        })
    }

    fn open_path(&mut self, path: &str) -> Result<usize, Errno> {
        let target = match crate::virtio_blk_boot::endpoint_for_path(path) {
            Some(crate::virtio_blk_boot::StorageEndpointKind::Device) => {
                DescriptorTarget::StorageDevice
            }
            Some(crate::virtio_blk_boot::StorageEndpointKind::Driver) => {
                DescriptorTarget::StorageDriver
            }
            None => match boot_stream_target(path) {
                Some(target) => target,
                None => match boot_procfs_directory_node(path)? {
                    Some(node) => {
                        require_procfs_access(node)?;
                        DescriptorTarget::Procfs(node)
                    }
                    None => match boot_procfs_node(path)? {
                        Some(node) => {
                            require_procfs_access(node)?;
                            DescriptorTarget::Procfs(node)
                        }
                        None => boot_vfs_lookup_target(path)?,
                    },
                },
            },
        };
        let free_fd = self
            .slots
            .iter()
            .enumerate()
            .skip(3)
            .find_map(|(index, slot)| slot.is_none().then_some(index))
            .ok_or(Errno::Again)?;
        let description_id = self.alloc_description(target)?;
        self.slots[free_fd] = Some(DescriptorState {
            description_id,
            cloexec: false,
            rights: descriptor_default_rights(target),
        });
        Ok(free_fd)
    }

    fn seek(&mut self, fd: usize, offset: i64, whence: SeekWhence) -> Result<usize, Errno> {
        let snapshot = self.descriptor(fd)?;
        let length = match snapshot.target {
            DescriptorTarget::Stdin => 0usize,
            DescriptorTarget::Stdout | DescriptorTarget::Stderr => 0usize,
            DescriptorTarget::EventQueue(_) => return Err(Errno::Badf),
            DescriptorTarget::StorageDevice
            | DescriptorTarget::StorageDriver
            | DescriptorTarget::GpuDevice
            | DescriptorTarget::GpuDriver
            | DescriptorTarget::AudioDevice
            | DescriptorTarget::AudioDriver
            | DescriptorTarget::InputDevice
            | DescriptorTarget::InputDriver
            | DescriptorTarget::NetworkDevice
            | DescriptorTarget::NetworkDriver => return Err(Errno::Badf),
            DescriptorTarget::BootDirectory(_) => return Err(Errno::IsDir),
            DescriptorTarget::BootFile(inode) | DescriptorTarget::BootChannel(inode) => {
                BOOT_VFS.with_mut(|vfs| vfs.object_len_by_inode(inode).ok_or(Errno::Badf))?
            }
            DescriptorTarget::Procfs(node) => boot_procfs_len(node)?,
        };
        let base = match whence {
            SeekWhence::Set => 0i64,
            SeekWhence::Cur => snapshot.offset as i64,
            SeekWhence::End => length as i64,
        };
        let Some(new_offset) = base.checked_add(offset) else {
            return Err(Errno::Range);
        };
        if new_offset < 0 {
            return Err(Errno::Inval);
        }
        let new_offset = new_offset as usize;
        let description_id = self.descriptor_state(fd)?.description_id;
        self.description_mut(description_id)?.offset = new_offset;
        Ok(new_offset)
    }

    fn snapshot_for_spawn(&self) -> Result<Vec<BootProcessDescriptorRecord>, Errno> {
        let mut records = Vec::new();
        for (fd, slot) in self.slots.iter().enumerate() {
            let Some(handle) = slot else {
                continue;
            };
            if fd >= 3 && handle.cloexec {
                continue;
            }
            let snapshot = self.descriptor(fd)?;
            let path = descriptor_target_path_text(snapshot.target)?;
            records.push(BootProcessDescriptorRecord {
                fd: fd as u64,
                path,
                kind_name: descriptor_target_kind_name(snapshot.target),
                cloexec: snapshot.cloexec,
                nonblock: snapshot.nonblock,
                pos: snapshot.offset,
                rights: snapshot.rights.0 as u32,
            });
        }
        Ok(records)
    }
}

impl DescriptorTableCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(DescriptorTable::new()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut DescriptorTable) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }

    fn with<R>(&self, f: impl FnOnce(&DescriptorTable) -> R) -> R {
        unsafe { f(&*self.0.get()) }
    }
}

#[cfg(target_os = "none")]
fn set_syscall_switch_result(
    result: &mut SyscallDispatchResult,
    context: SavedUserContext,
    raw_return: usize,
) {
    result.raw_return = raw_return;
    result.disposition = SyscallDisposition::Switch as u64;
    result.switch_rip = context.rip;
    result.switch_rsp = context.rsp;
    result.switch_rflags = context.rflags;
    result.switch_r15 = context.r15;
    result.switch_r14 = context.r14;
    result.switch_r13 = context.r13;
    result.switch_r12 = context.r12;
    result.switch_rbp = context.rbp;
    result.switch_rbx = context.rbx;
    result.switch_r10 = context.r10;
    result.switch_r9 = context.r9;
    result.switch_r8 = context.r8;
    result.switch_rdi = context.rdi;
    result.switch_rsi = context.rsi;
    result.switch_rdx = context.rdx;
    result.switch_rax = context.rax;
}

#[cfg(target_os = "none")]
fn switch_context_from_launch(registers: Amd64UserEntryRegisters) -> SavedUserContext {
    SavedUserContext {
        rip: registers.rip as u64,
        rsp: registers.rsp as u64,
        rflags: registers.rflags as u64,
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        rbp: 0,
        rbx: 0,
        r10: 0,
        r9: registers.r9 as u64,
        r8: registers.r8 as u64,
        rdi: registers.rdi as u64,
        rsi: registers.rsi as u64,
        rdx: registers.rdx as u64,
        rax: 0,
    }
}

#[cfg(target_os = "none")]
fn capture_saved_user_context(
    context: &SyscallSavedContext,
    user_rip: u64,
    user_rsp: u64,
    user_rflags: u64,
) -> SavedUserContext {
    SavedUserContext {
        rip: user_rip,
        rsp: user_rsp,
        rflags: user_rflags,
        r15: context.saved_r15,
        r14: context.saved_r14,
        r13: context.saved_r13,
        r12: context.saved_r12,
        rbp: context.saved_rbp,
        rbx: context.saved_rbx,
        r10: context.saved_r10,
        r9: context.saved_r9,
        r8: context.saved_r8,
        rdi: context.saved_rdi,
        rsi: context.saved_rsi,
        rdx: context.saved_rdx,
        rax: 0,
    }
}

#[cfg(target_os = "none")]
fn try_begin_blocking_reap_launch(
    trap: &SyscallSavedContext,
    user_rip: u64,
    user_rsp: u64,
    user_rflags: u64,
    result: &mut SyscallDispatchResult,
) -> Result<bool, Errno> {
    let Some(child_pid) = take_blocking_reap_launch() else {
        return Ok(false);
    };
    let parent_pid = active_process_pid()?;
    let (process_name, image_path, cwd, root, argv, envp) =
        BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(child_pid) else {
                return Err(Errno::Srch);
            };
            let entry = &registry.entries[index];
            Ok((
                entry.name.clone(),
                entry.image_path.clone(),
                entry.cwd.clone(),
                entry.root.clone(),
                entry.argv.clone(),
                entry.envp.clone(),
            ))
        })?;
    let boot_info = crate::early_boot_info().ok_or(Errno::NotSup)?;
    let launch = prepare_spawned_same_image_launch(
        boot_info,
        child_pid,
        &process_name,
        &image_path,
        &cwd,
        &root,
        &argv,
        &envp,
    )
    .map_err(|_| Errno::NoMem)?;
    with_boot_exec_runtime(|paging, allocator| {
        paging
            .map_pages(
                allocator,
                launch.stack_mapping,
                Some(PageInit {
                    bytes: &launch.stack_bytes,
                    offset: 0,
                }),
            )
            .map_err(|_| Errno::NoMem)?;
        paging.flush_tlb();
        Ok(())
    })?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(child_pid) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].state = 2;
        Ok(())
    })?;
    set_active_blocking_child(BlockingChildExecution {
        parent_pid,
        child_pid,
        parent_context: capture_saved_user_context(trap, user_rip, user_rsp, user_rflags),
    });
    set_active_process_pid(child_pid);
    set_syscall_switch_result(result, switch_context_from_launch(launch.registers), 0);
    Ok(true)
}

#[cfg(target_os = "none")]
fn finish_blocking_child_exit(
    code: i32,
    result: &mut SyscallDispatchResult,
) -> Result<bool, Errno> {
    let Some(execution) = take_active_blocking_child() else {
        return Ok(false);
    };
    let (exit_code, summary) = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(execution.child_pid) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].state = 4;
        registry.entries[index].exit_code = code;
        registry.reap(execution.child_pid)
    })?;
    BOOT_VFS.with_mut(|vfs| {
        vfs.stats.process_reaps = vfs.stats.process_reaps.saturating_add(1);
        vfs.stats.reaped_descriptor_records = vfs
            .stats
            .reaped_descriptor_records
            .saturating_add(summary.descriptors);
        vfs.stats.reaped_env_records = vfs
            .stats
            .reaped_env_records
            .saturating_add(summary.env_records);
        vfs.stats.reaped_vm_objects = vfs
            .stats
            .reaped_vm_objects
            .saturating_add(summary.vm_objects);
        vfs.stats.reaped_vm_decisions = vfs
            .stats
            .reaped_vm_decisions
            .saturating_add(summary.vm_decisions);
    });
    set_active_process_pid(execution.parent_pid);
    set_syscall_switch_result(
        result,
        execution.parent_context,
        encode_syscall_result(Ok(exit_code as usize)),
    );
    Ok(true)
}

#[unsafe(no_mangle)]
extern "C" fn x86_64_syscall_dispatch(
    frame: *const SyscallFrame,
    user_rip: u64,
    user_rsp: u64,
    user_rflags: u64,
    result: *mut SyscallDispatchResult,
) {
    #[cfg(target_os = "none")]
    let trap = unsafe { &*(frame as *const SyscallSavedContext) };
    #[cfg(target_os = "none")]
    let frame = &trap.frame;
    #[cfg(not(target_os = "none"))]
    let frame = unsafe { &*frame };
    let result = unsafe { &mut *result };

    serial::debug_marker(b'T');
    syscall_trace(format_args!(
        "ngos/x86_64: syscall entry reached rip={:#x} rsp={:#x} rflags={:#x}\n",
        user_rip, user_rsp, user_rflags
    ));
    serial::debug_marker(b'U');
    syscall_trace(format_args!(
        "ngos/x86_64: syscall number decoded nr={} a0={:#x} a1={:#x} a2={:#x}\n",
        frame.number, frame.arg0, frame.arg1, frame.arg2
    ));
    user_runtime_status::record_syscall(frame.number);
    diagnostics::record_syscall_enter(
        frame.number as u64,
        frame.arg0 as u64,
        frame.arg1 as u64,
        frame.arg2 as u64,
    );

    let syscall_result = match frame.number {
        SYS_EXIT => handle_exit(frame.arg0 as i32, result),
        SYS_READ => read_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2),
        SYS_READV => readv_syscall(frame.arg0, frame.arg1 as *const UserIoVec, frame.arg2),
        SYS_WRITE => write_syscall(frame.arg0, frame.arg1 as *const u8, frame.arg2),
        SYS_WRITEV => writev_syscall(frame.arg0, frame.arg1 as *const UserIoVec, frame.arg2),
        SYS_DUP => duplicate_syscall(frame.arg0),
        SYS_SEEK => seek_syscall(frame.arg0, frame.arg1 as i64, frame.arg2 as u32),
        SYS_CLOSE => close_syscall(frame.arg0),
        SYS_FCNTL => fcntl_syscall(frame.arg0, frame.arg1),
        SYS_POLL => poll_syscall(frame.arg0, frame.arg1 as u32),
        SYS_STAT_PATH => stat_path_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeFileStatusRecord,
        ),
        SYS_STAT_PATH_AT => stat_path_at_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3 as *mut NativeFileStatusRecord,
        ),
        SYS_LSTAT_PATH => lstat_path_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeFileStatusRecord,
        ),
        SYS_LSTAT_PATH_AT => lstat_path_at_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3 as *mut NativeFileStatusRecord,
        ),
        SYS_STATFS_PATH => statfs_path_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeFileSystemStatusRecord,
        ),
        SYS_OPEN_PATH => open_path_syscall(frame.arg0, frame.arg1),
        SYS_OPEN_PATH_AT => open_path_at_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_LIST_PROCESSES => list_processes_syscall(frame.arg0 as *mut u64, frame.arg1),
        _ => dispatch_network_event_syscall(frame)
            .or_else(|| dispatch_process_vm_syscall(frame))
            .or_else(|| dispatch_path_vfs_syscall(frame))
            .or_else(|| dispatch_resource_graphics_syscall(frame))
            .unwrap_or(Err(Errno::Inval)),
    };

    #[cfg(target_os = "none")]
    let syscall_result = if frame.number == SYS_REAP_PROCESS {
        match try_begin_blocking_reap_launch(trap, user_rip, user_rsp, user_rflags, result) {
            Ok(true) => return,
            Ok(false) => syscall_result,
            Err(errno) => Err(errno),
        }
    } else {
        syscall_result
    };

    if frame.number != SYS_EXIT {
        let (ok, errno) = match &syscall_result {
            Ok(_) => (true, 0),
            Err(errno) => (false, *errno as u16),
        };
        diagnostics::record_syscall_exit(
            frame.number as u64,
            frame.arg0 as u64,
            frame.arg1 as u64,
            frame.arg2 as u64,
            ok,
            errno,
        );
        result.raw_return = encode_syscall_result(syscall_result);
        result.disposition = SyscallDisposition::Return as u64;
    }
}

fn handle_exit(code: i32, result: &mut SyscallDispatchResult) -> Result<usize, Errno> {
    #[cfg(target_os = "none")]
    if finish_blocking_child_exit(code, result)? {
        return Ok(0);
    }
    unsafe {
        PROCESS_EXIT_CODE = code;
        PROCESS_EXITED = true;
    }
    user_runtime_status::mark_exit(code);
    serial::debug_marker(b'V');
    syscall_trace(format_args!(
        "ngos/x86_64: exit syscall handled code={}\n",
        code
    ));
    serial::debug_marker(b'W');
    syscall_trace(format_args!(
        "ngos/x86_64: process exit propagated code={} exited={}\n",
        code,
        unsafe { PROCESS_EXITED }
    ));
    user_runtime_status::emit_final_report_if_terminal();
    let _ = user_runtime_status::apply_configured_boot_outcome_policy();
    result.raw_return = 0;
    result.disposition = SyscallDisposition::Halt as u64;
    Ok(0)
}

fn duplicate_syscall(fd: usize) -> Result<usize, Errno> {
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::DELEGATE))?;
    let duplicated = DESCRIPTORS.with_mut(|descriptors| descriptors.duplicate(fd))?;
    serial::print(format_args!(
        "ngos/x86_64: dup handled fd={} -> {}\n",
        fd, duplicated
    ));
    Ok(duplicated)
}

fn set_fd_rights_syscall(fd: usize, rights: BlockRightsMask) -> Result<usize, Errno> {
    DESCRIPTORS.with_mut(|descriptors| descriptors.restrict_rights(fd, rights))
}

fn close_syscall(fd: usize) -> Result<usize, Errno> {
    let target = DESCRIPTORS
        .with(|descriptors| descriptors.descriptor(fd).map(|snapshot| snapshot.target))?;
    DESCRIPTORS.with_mut(|descriptors| descriptors.close(fd))?;
    if let Some(inode) = descriptor_lock_inode(target) {
        let path = descriptor_target_path_text(target).ok();
        emit_vfs_event(
            inode,
            path.as_deref(),
            None,
            NativeVfsEventKind::Closed,
            fd as u32,
        );
    }
    Ok(0)
}

fn fcntl_syscall(fd: usize, encoded: usize) -> Result<usize, Errno> {
    let flags = DESCRIPTORS.with_mut(|descriptors| descriptors.fcntl(fd, encoded))?;
    Ok(flags)
}

fn seek_syscall(fd: usize, offset: i64, whence_raw: u32) -> Result<usize, Errno> {
    let whence = SeekWhence::from_raw(whence_raw).ok_or(Errno::Inval)?;
    let new_offset = DESCRIPTORS.with_mut(|descriptors| descriptors.seek(fd, offset, whence))?;
    Ok(new_offset)
}

fn poll_syscall(fd: usize, interest: u32) -> Result<usize, Errno> {
    let required = match (
        (interest & POLLIN) != 0,
        (interest & POLLOUT) != 0 || (interest & POLLPRI) != 0,
    ) {
        (true, true) => BlockRightsMask::READ.union(BlockRightsMask::WRITE),
        (true, false) => BlockRightsMask::READ,
        (false, true) => BlockRightsMask::WRITE,
        (false, false) => BlockRightsMask::NONE,
    };
    if required != BlockRightsMask::NONE {
        DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, required))?;
    }
    serial::print(format_args!(
        "ngos/x86_64: poll enter fd={} interest={:#x}\n",
        fd, interest
    ));
    let ready = DESCRIPTORS.with(|descriptors| descriptors.poll(fd, interest))?;
    serial::print(format_args!(
        "ngos/x86_64: poll handled fd={} interest={:#x} ready={:#x}\n",
        fd, interest, ready
    ));
    Ok(ready)
}

fn create_event_queue_syscall(mode_raw: u32) -> Result<usize, Errno> {
    let mode = NativeEventQueueMode::from_raw(mode_raw).ok_or(Errno::Inval)?;
    let fd = DESCRIPTORS.with_mut(|descriptors| descriptors.create_event_queue(mode))?;
    serial::print(format_args!(
        "ngos/x86_64: create_event_queue handled mode={} fd={}\n",
        mode_raw, fd
    ));
    Ok(fd)
}

fn wait_event_queue_syscall(
    fd: usize,
    buffer: *mut NativeEventRecord,
    capacity: usize,
) -> Result<usize, Errno> {
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::READ))?;
    let count =
        DESCRIPTORS.with_mut(|descriptors| descriptors.wait_event_queue(fd, buffer, capacity))?;
    serial::print(format_args!(
        "ngos/x86_64: wait_event_queue handled fd={} count={}\n",
        fd, count
    ));
    Ok(count)
}

fn watch_resource_events_syscall(
    fd: usize,
    resource: usize,
    config_ptr: *const NativeResourceEventWatchConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let config = unsafe { ptr::read(config_ptr) };
    DESCRIPTORS.with_mut(|descriptors| descriptors.watch_resource_events(fd, resource, config))?;
    serial::print(format_args!(
        "ngos/x86_64: watch_resource_events handled fd={} resource={} token={}\n",
        fd, resource, config.token
    ));
    Ok(0)
}

fn remove_resource_events_syscall(fd: usize, resource: usize, token: u64) -> Result<usize, Errno> {
    DESCRIPTORS.with_mut(|descriptors| descriptors.remove_resource_events(fd, resource, token))?;
    serial::print(format_args!(
        "ngos/x86_64: remove_resource_events handled fd={} resource={} token={}\n",
        fd, resource, token
    ));
    Ok(0)
}

fn watch_network_events_syscall(
    fd: usize,
    interface_ptr: usize,
    interface_len: usize,
    socket_ptr: usize,
    socket_len: usize,
    config_ptr: *const NativeNetworkEventWatchConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let interface_path = path_from_user(interface_ptr, interface_len)?;
    let socket_path = if socket_ptr == 0 || socket_len == 0 {
        None
    } else {
        Some(path_from_user(socket_ptr, socket_len)?)
    };
    let config = unsafe { ptr::read(config_ptr) };
    DESCRIPTORS.with_mut(|descriptors| {
        descriptors.watch_network_events(fd, interface_path, socket_path, config)
    })?;
    Ok(0)
}

fn remove_network_events_syscall(
    fd: usize,
    interface_ptr: usize,
    interface_len: usize,
    socket_ptr: usize,
    socket_len: usize,
    token: u64,
) -> Result<usize, Errno> {
    let interface_path = path_from_user(interface_ptr, interface_len)?;
    let socket_path = if socket_ptr == 0 || socket_len == 0 {
        None
    } else {
        Some(path_from_user(socket_ptr, socket_len)?)
    };
    DESCRIPTORS.with_mut(|descriptors| {
        descriptors.remove_network_events(fd, interface_path, socket_path, token)
    })?;
    Ok(0)
}

fn watch_bus_events_syscall(
    fd: usize,
    endpoint: usize,
    config_ptr: *const NativeBusEventWatchConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let config = unsafe { ptr::read(config_ptr) };
    DESCRIPTORS
        .with_mut(|descriptors| descriptors.watch_bus_events(fd, endpoint as u64, config))?;
    Ok(0)
}

fn remove_bus_events_syscall(fd: usize, endpoint: usize, token: u64) -> Result<usize, Errno> {
    DESCRIPTORS
        .with_mut(|descriptors| descriptors.remove_bus_events(fd, endpoint as u64, token))?;
    Ok(0)
}

fn watch_vfs_events_syscall(
    fd: usize,
    path_ptr: usize,
    path_len: usize,
    config_ptr: *const NativeVfsEventWatchConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::WRITE))?;
    let path = path_from_user(path_ptr, path_len)?;
    let config = unsafe { ptr::read(config_ptr) };
    DESCRIPTORS.with_mut(|descriptors| descriptors.watch_vfs_events(fd, path, config))?;
    Ok(0)
}

fn watch_vfs_events_at_syscall(
    fd: usize,
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    config_ptr: *const NativeVfsEventWatchConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::WRITE))?;
    let config = unsafe { ptr::read(config_ptr) };
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    match resolved {
        ResolvedAtTarget::Handle(DescriptorTarget::BootDirectory(inode)) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::READ))?;
            BOOT_VFS.with_mut(|vfs| vfs.require_observe_inode(inode))?;
            let anchor_path = BOOT_VFS.with_mut(|vfs| vfs.live_path_for_inode(inode));
            DESCRIPTORS.with_mut(|descriptors| {
                descriptors.watch_vfs_events_by_anchor(
                    fd,
                    inode,
                    BootNodeKind::Directory,
                    anchor_path,
                    config,
                )
            })?;
        }
        ResolvedAtTarget::Handle(DescriptorTarget::BootFile(inode)) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::READ))?;
            BOOT_VFS.with_mut(|vfs| vfs.require_observe_inode(inode))?;
            DESCRIPTORS.with_mut(|descriptors| {
                descriptors.watch_vfs_events_by_anchor(fd, inode, BootNodeKind::File, None, config)
            })?;
        }
        ResolvedAtTarget::Handle(DescriptorTarget::BootChannel(inode)) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::READ))?;
            BOOT_VFS.with_mut(|vfs| vfs.require_observe_inode(inode))?;
            DESCRIPTORS.with_mut(|descriptors| {
                descriptors.watch_vfs_events_by_anchor(
                    fd,
                    inode,
                    BootNodeKind::Channel,
                    None,
                    config,
                )
            })?;
        }
        ResolvedAtTarget::Handle(_) => return Err(Errno::Inval),
        ResolvedAtTarget::Path => {
            DESCRIPTORS.with_mut(|descriptors| descriptors.watch_vfs_events(fd, &path, config))?;
        }
    }
    Ok(0)
}

fn remove_vfs_events_syscall(
    fd: usize,
    path_ptr: usize,
    path_len: usize,
    token: u64,
) -> Result<usize, Errno> {
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::WRITE))?;
    let path = path_from_user(path_ptr, path_len)?;
    DESCRIPTORS.with_mut(|descriptors| descriptors.remove_vfs_events(fd, path, token))?;
    Ok(0)
}

fn remove_vfs_events_at_syscall(
    fd: usize,
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    token: u64,
) -> Result<usize, Errno> {
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::WRITE))?;
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    match resolved {
        ResolvedAtTarget::Handle(DescriptorTarget::BootDirectory(inode))
        | ResolvedAtTarget::Handle(DescriptorTarget::BootFile(inode))
        | ResolvedAtTarget::Handle(DescriptorTarget::BootChannel(inode)) => {
            DESCRIPTORS.with_mut(|descriptors| {
                descriptors.remove_vfs_events_by_anchor(fd, inode, token)
            })?;
        }
        ResolvedAtTarget::Handle(_) => return Err(Errno::Inval),
        ResolvedAtTarget::Path => {
            DESCRIPTORS.with_mut(|descriptors| descriptors.remove_vfs_events(fd, &path, token))?;
        }
    }
    Ok(0)
}

fn configure_network_interface_ipv4_syscall(
    path_ptr: usize,
    path_len: usize,
    config_ptr: *const NativeNetworkInterfaceConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let path = path_from_user(path_ptr, path_len)?;
    let config = unsafe { ptr::read(config_ptr) };
    crate::boot_network_runtime::configure_interface_ipv4(
        path,
        config.addr,
        config.netmask,
        config.gateway,
    )?;
    if hardware_network_online() {
        hardware_network_configure_interface_ipv4(
            path,
            config.addr,
            config.netmask,
            config.gateway,
        )?;
    }
    Ok(0)
}

fn bind_udp_socket_syscall(
    socket_ptr: usize,
    socket_len: usize,
    device_ptr: usize,
    device_len: usize,
    config_ptr: *const NativeUdpBindConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let socket_path = path_from_user(socket_ptr, socket_len)?;
    let device_path = path_from_user(device_ptr, device_len)?;
    let config = unsafe { ptr::read(config_ptr) };
    crate::boot_network_runtime::bind_udp_socket(
        socket_path,
        device_path,
        config.local_port,
        config.remote_ipv4,
        config.remote_port,
    )?;
    Ok(0)
}

fn inspect_network_interface_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeNetworkInterfaceRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let mut record = if hardware_network_online() {
        if path == NETWORK_DEVICE_PATH {
            hardware_network_interface_record(path)
        } else {
            crate::boot_network_runtime::interface_record(path)
        }
    } else {
        crate::boot_network_runtime::interface_record(path)
    };
    if let Some(record_ref) = record.as_mut() {
        record_ref.attached_socket_count = crate::boot_network_runtime::attached_socket_count(path);
    }
    let Some(record) = record else {
        return Err(Errno::NoEnt);
    };
    write_record(out, record)?;
    Ok(0)
}

fn inspect_network_socket_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeNetworkSocketRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let Some(record) = crate::boot_network_runtime::socket_record(path) else {
        return Err(Errno::NoEnt);
    };
    write_record(out, record)?;
    Ok(0)
}

fn set_network_link_state_syscall(
    path_ptr: usize,
    path_len: usize,
    config_ptr: *const NativeNetworkLinkStateConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let path = path_from_user(path_ptr, path_len)?;
    let config = unsafe { ptr::read(config_ptr) };
    crate::boot_network_runtime::set_link_state(path, config.link_up != 0)?;
    if hardware_network_online() && path == NETWORK_DEVICE_PATH {
        hardware_network_set_link_state(path, config.link_up != 0)?;
    }
    emit_network_event(path, None, NativeNetworkEventKind::LinkChanged);
    Ok(0)
}

fn configure_network_interface_admin_syscall(
    path_ptr: usize,
    path_len: usize,
    config_ptr: *const NativeNetworkAdminConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let path = path_from_user(path_ptr, path_len)?;
    let config = unsafe { ptr::read(config_ptr) };
    crate::boot_network_runtime::configure_interface_admin(
        path,
        config.mtu,
        config.tx_capacity,
        config.rx_capacity,
        config.tx_inflight_limit,
        config.admin_up != 0,
        config.promiscuous != 0,
    )?;
    if hardware_network_online() && path == NETWORK_DEVICE_PATH {
        hardware_network_configure_interface_admin(
            path,
            config.mtu,
            config.tx_capacity,
            config.rx_capacity,
            config.tx_inflight_limit,
            config.admin_up != 0,
            config.promiscuous != 0,
        )?;
    }
    Ok(0)
}

fn connect_udp_socket_syscall(
    path_ptr: usize,
    path_len: usize,
    config_ptr: *const NativeUdpConnectConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let path = path_from_user(path_ptr, path_len)?;
    let config = unsafe { ptr::read(config_ptr) };
    crate::boot_network_runtime::connect_udp_socket(path, config.remote_ipv4, config.remote_port)?;
    Ok(0)
}

fn sendto_udp_socket_syscall(
    path_ptr: usize,
    path_len: usize,
    config_ptr: *const NativeUdpSendToConfig,
    payload_ptr: *const u8,
    payload_len: usize,
) -> Result<usize, Errno> {
    if config_ptr.is_null() || (payload_len != 0 && payload_ptr.is_null()) {
        return Err(Errno::Fault);
    }
    let path = path_from_user(path_ptr, path_len)?;
    let config = unsafe { ptr::read(config_ptr) };
    let payload = unsafe { slice::from_raw_parts(payload_ptr, payload_len) };
    if hardware_network_online()
        && crate::boot_network_runtime::socket_device_path(path).as_deref()
            == Some(NETWORK_DEVICE_PATH)
    {
        return hardware_network_send_udp_to(path, config.remote_ipv4, config.remote_port, payload);
    }
    let (count, _request_id) = crate::boot_network_runtime::send_udp_to(
        path,
        config.remote_ipv4,
        config.remote_port,
        payload,
    )?;
    Ok(count)
}

fn recvfrom_udp_socket_syscall(
    path_ptr: usize,
    path_len: usize,
    buffer: *mut u8,
    len: usize,
    meta_out: *mut NativeUdpRecvMeta,
) -> Result<usize, Errno> {
    if buffer.is_null() || meta_out.is_null() {
        return Err(Errno::Fault);
    }
    let path = path_from_user(path_ptr, path_len)?;
    let bytes = unsafe { slice::from_raw_parts_mut(buffer, len) };
    let (count, meta) = crate::boot_network_runtime::recv_udp_from(path, bytes)?;
    write_record(meta_out, meta)?;
    Ok(count)
}

fn complete_network_tx_syscall(
    driver_ptr: usize,
    driver_len: usize,
    completions: usize,
) -> Result<usize, Errno> {
    let driver_path = path_from_user(driver_ptr, driver_len)?;
    let count = if hardware_network_online() && driver_path == NETWORK_DRIVER_PATH {
        hardware_network_complete_tx(driver_path, completions)?
    } else {
        crate::boot_network_runtime::complete_tx(driver_path, completions)?
    };
    if count != 0 {
        let interface_path = driver_path.replacen("/drv/", "/dev/", 1);
        emit_network_event(&interface_path, None, NativeNetworkEventKind::TxDrained);
    }
    Ok(count)
}

fn read_syscall(fd: usize, buffer: *mut u8, len: usize) -> Result<usize, Errno> {
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::READ))?;
    let descriptor = DESCRIPTORS.with(|descriptors| descriptors.descriptor(fd))?;
    syscall_trace(format_args!(
        "ngos/x86_64: read enter fd={} target={:?} buffer={:#x} len={}\n",
        fd, descriptor.target, buffer as usize, len
    ));
    let path = match descriptor.target {
        DescriptorTarget::StorageDevice => DiagnosticsPath::Completion,
        DescriptorTarget::StorageDriver => DiagnosticsPath::Completion,
        DescriptorTarget::EventQueue(_) => DiagnosticsPath::Syscall,
        DescriptorTarget::GpuDevice
        | DescriptorTarget::GpuDriver
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::AudioDriver
        | DescriptorTarget::InputDevice
        | DescriptorTarget::InputDriver
        | DescriptorTarget::NetworkDevice
        | DescriptorTarget::NetworkDriver => DiagnosticsPath::Syscall,
        _ => DiagnosticsPath::Syscall,
    };
    diagnostics::set_active_window(
        SYS_READ as u64,
        fd as u64,
        0,
        0x5354_4f52_4147_4530,
        0,
        path,
        diagnostics::replay_ids().request_id,
        diagnostics::replay_ids().completion_id,
    );
    let read = match descriptor.target {
        DescriptorTarget::Stdin => tty::read_stdin(buffer, len, descriptor.nonblock)?,
        DescriptorTarget::StorageDevice => crate::virtio_blk_boot::read(
            crate::virtio_blk_boot::StorageEndpointKind::Device,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::StorageDriver => crate::virtio_blk_boot::read(
            crate::virtio_blk_boot::StorageEndpointKind::Driver,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::GpuDevice => crate::boot_gpu_runtime::read(
            crate::boot_gpu_runtime::GpuEndpointKind::Device,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::GpuDriver => crate::boot_gpu_runtime::read(
            crate::boot_gpu_runtime::GpuEndpointKind::Driver,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::AudioDevice => crate::boot_audio_runtime::read(
            crate::boot_audio_runtime::AudioEndpointKind::Device,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::AudioDriver => crate::boot_audio_runtime::read(
            crate::boot_audio_runtime::AudioEndpointKind::Driver,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::InputDevice => crate::boot_input_runtime::read(
            crate::boot_input_runtime::InputEndpointKind::Device,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::InputDriver => crate::boot_input_runtime::read(
            crate::boot_input_runtime::InputEndpointKind::Driver,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::NetworkDevice => {
            if hardware_network_online() {
                hardware_network_device_read(buffer, len, descriptor.nonblock)?
            } else {
                crate::boot_network_runtime::read(
                    crate::boot_network_runtime::NetworkEndpointKind::Device,
                    buffer,
                    len,
                    descriptor.nonblock,
                )?
            }
        }
        DescriptorTarget::NetworkDriver => crate::boot_network_runtime::read(
            crate::boot_network_runtime::NetworkEndpointKind::Driver,
            buffer,
            len,
            descriptor.nonblock,
        )?,
        DescriptorTarget::EventQueue(_) => return Err(Errno::Badf),
        DescriptorTarget::BootDirectory(_) => return Err(Errno::IsDir),
        DescriptorTarget::BootFile(inode) | DescriptorTarget::BootChannel(inode) => DESCRIPTORS
            .with_mut(|descriptors| {
                let description_id = descriptors.descriptor_state(fd)?.description_id;
                let description = descriptors.description_mut(description_id)?;
                boot_vfs_read(inode, &mut description.offset, buffer, len)
            })?,
        DescriptorTarget::Procfs(node) => DESCRIPTORS.with_mut(|descriptors| {
            let description_id = descriptors.descriptor_state(fd)?.description_id;
            let description = descriptors.description_mut(description_id)?;
            boot_procfs_read(node, &mut description.offset, buffer, len)
        })?,
        DescriptorTarget::Stdout | DescriptorTarget::Stderr => return Err(Errno::Badf),
    };

    syscall_trace(format_args!(
        "ngos/x86_64: read handled fd={} len={} read={}\n",
        fd, len, read
    ));
    if read != 0 {
        diagnostics::watch_touch(WatchKind::Read, buffer as u64, read as u64);
    }
    diagnostics::clear_active_window();
    Ok(read)
}

fn write_syscall(fd: usize, buffer: *const u8, len: usize) -> Result<usize, Errno> {
    DESCRIPTORS.with(|descriptors| descriptors.require_rights(fd, BlockRightsMask::WRITE))?;
    let descriptor = DESCRIPTORS.with(|descriptors| descriptors.descriptor(fd))?;
    let path = match descriptor.target {
        DescriptorTarget::StorageDevice => DiagnosticsPath::Block,
        DescriptorTarget::StorageDriver => DiagnosticsPath::Block,
        DescriptorTarget::EventQueue(_) => DiagnosticsPath::Syscall,
        DescriptorTarget::GpuDevice
        | DescriptorTarget::GpuDriver
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::AudioDriver
        | DescriptorTarget::InputDevice
        | DescriptorTarget::InputDriver
        | DescriptorTarget::NetworkDevice
        | DescriptorTarget::NetworkDriver => DiagnosticsPath::Syscall,
        _ => DiagnosticsPath::Syscall,
    };
    diagnostics::set_active_window(
        SYS_WRITE as u64,
        fd as u64,
        0,
        0x5354_4f52_4147_4530,
        0,
        path,
        0,
        0,
    );
    syscall_trace(format_args!(
        "ngos/x86_64: write enter fd={} target={:?} buffer={:#x} len={}\n",
        fd, descriptor.target, buffer as usize, len
    ));
    if len == 0 {
        syscall_trace(format_args!(
            "ngos/x86_64: write short-circuit fd={} len=0\n",
            fd
        ));
        return Ok(0);
    }
    if buffer.is_null() {
        syscall_trace(format_args!(
            "ngos/x86_64: write fault fd={} null-buffer\n",
            fd
        ));
        return Err(Errno::Fault);
    }

    let bytes = unsafe { slice::from_raw_parts(buffer, len) };
    let _ = diagnostics::guard_register(
        GuardKind::RequestBuffer,
        path,
        buffer as u64,
        len as u64,
        32,
        0,
        0,
    );
    let _ = diagnostics::watch_register(WatchKind::Touch, path, buffer as u64, len as u64, 0, 0);
    let _ = diagnostics::guard_check(buffer as u64, len as u64);
    diagnostics::watch_touch(WatchKind::Read, buffer as u64, len as u64);
    syscall_trace(format_args!(
        "ngos/x86_64: write bytes ready fd={} len={} first8={:?}\n",
        fd,
        len,
        &bytes[..bytes.len().min(8)]
    ));
    match descriptor.target {
        DescriptorTarget::Stdout => {
            let _ = tty::write_stdout(bytes);
        }
        DescriptorTarget::Stderr => {
            let _ = tty::write_stderr(bytes);
        }
        DescriptorTarget::StorageDevice => {
            syscall_trace(format_args!(
                "ngos/x86_64: write dispatch storage-device fd={} len={}\n",
                fd, len
            ));
            let result = crate::virtio_blk_boot::write(
                crate::virtio_blk_boot::StorageEndpointKind::Device,
                bytes,
            );
            syscall_trace(format_args!(
                "ngos/x86_64: write return storage-device fd={} result={:?}\n",
                fd, result
            ));
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::StorageDriver => {
            syscall_trace(format_args!(
                "ngos/x86_64: write dispatch storage-driver fd={} len={}\n",
                fd, len
            ));
            let result = crate::virtio_blk_boot::write(
                crate::virtio_blk_boot::StorageEndpointKind::Driver,
                bytes,
            );
            syscall_trace(format_args!(
                "ngos/x86_64: write return storage-driver fd={} result={:?}\n",
                fd, result
            ));
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::GpuDevice => {
            let result = crate::boot_gpu_runtime::write(
                crate::boot_gpu_runtime::GpuEndpointKind::Device,
                bytes,
            );
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::GpuDriver => {
            let result = crate::boot_gpu_runtime::write(
                crate::boot_gpu_runtime::GpuEndpointKind::Driver,
                bytes,
            );
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::AudioDevice => {
            let result = crate::boot_audio_runtime::write(
                crate::boot_audio_runtime::AudioEndpointKind::Device,
                bytes,
            );
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::AudioDriver => {
            let result = crate::boot_audio_runtime::write(
                crate::boot_audio_runtime::AudioEndpointKind::Driver,
                bytes,
            );
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::EventQueue(_) => return Err(Errno::Badf),
        DescriptorTarget::InputDevice => {
            let result = crate::boot_input_runtime::write(
                crate::boot_input_runtime::InputEndpointKind::Device,
                bytes,
            );
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::InputDriver => {
            let result = crate::boot_input_runtime::write(
                crate::boot_input_runtime::InputEndpointKind::Driver,
                bytes,
            );
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::NetworkDevice => {
            let result = if hardware_network_online() {
                hardware_network_device_write(bytes)
            } else {
                crate::boot_network_runtime::write(
                    crate::boot_network_runtime::NetworkEndpointKind::Device,
                    bytes,
                )
            };
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::NetworkDriver => {
            let result = crate::boot_network_runtime::write(
                crate::boot_network_runtime::NetworkEndpointKind::Driver,
                bytes,
            );
            if result.is_ok() {
                emit_network_event(
                    NETWORK_DEVICE_PATH,
                    Some("/run/net0.sock"),
                    NativeNetworkEventKind::RxReady,
                );
            }
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::BootDirectory(_) => return Err(Errno::IsDir),
        DescriptorTarget::BootFile(inode) | DescriptorTarget::BootChannel(inode) => {
            let result = DESCRIPTORS.with_mut(|descriptors| {
                let description_id = descriptors.descriptor_state(fd)?.description_id;
                let description = descriptors.description_mut(description_id)?;
                boot_vfs_write(inode, &mut description.offset, bytes, Some(description_id))
            });
            if result.is_ok() {
                let path = descriptor_target_path_text(descriptor.target).ok();
                emit_vfs_event(
                    inode,
                    path.as_deref(),
                    None,
                    NativeVfsEventKind::Written,
                    len as u32,
                );
            }
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::Procfs(_) => return Err(Errno::Badf),
        DescriptorTarget::Stdin => return Err(Errno::Badf),
    }
    if bytes
        .windows(b"desktop.boot stage=presented".len())
        .any(|window| window == b"desktop.boot stage=presented")
    {
        serial::disable_framebuffer_mirror();
        serial::print(format_args!(
            "ngos/x86_64: framebuffer serial mirror disabled reason=desktop-presented\n"
        ));
    }
    user_runtime_status::record_write(fd, bytes.len());
    syscall_trace(format_args!(
        "ngos/x86_64: write complete fd={} len={}\n",
        fd,
        bytes.len()
    ));
    diagnostics::clear_active_window();
    Ok(bytes.len())
}

fn readv_syscall(fd: usize, iovecs: *const UserIoVec, count: usize) -> Result<usize, Errno> {
    if count == 0 {
        return Ok(0);
    }
    if iovecs.is_null() {
        return Err(Errno::Fault);
    }
    let iovecs = unsafe { slice::from_raw_parts(iovecs, count) };
    let mut total = 0usize;
    for iov in iovecs {
        let read = read_syscall(fd, iov.base as *mut u8, iov.len)?;
        total += read;
        if read < iov.len {
            break;
        }
    }
    Ok(total)
}

fn writev_syscall(fd: usize, iovecs: *const UserIoVec, count: usize) -> Result<usize, Errno> {
    if count == 0 {
        return Ok(0);
    }
    if iovecs.is_null() {
        return Err(Errno::Fault);
    }
    let iovecs = unsafe { slice::from_raw_parts(iovecs, count) };
    let mut total = 0usize;
    for iov in iovecs {
        let written = write_syscall(fd, iov.base as *const u8, iov.len)?;
        total += written;
        if written < iov.len {
            break;
        }
    }
    Ok(total)
}

fn path_from_user<'a>(ptr_value: usize, len: usize) -> Result<&'a str, Errno> {
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    let bytes = unsafe { slice::from_raw_parts(ptr_value as *const u8, len) };
    core::str::from_utf8(bytes).map_err(|_| Errno::Inval)
}

fn active_process_cwd() -> Result<String, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(1) else {
            return Err(Errno::Srch);
        };
        Ok(registry.entries[index].cwd.clone())
    })
}

fn active_process_root() -> Result<String, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(1) else {
            return Err(Errno::Srch);
        };
        Ok(registry.entries[index].root.clone())
    })
}

fn resolve_active_process_path(text: &str) -> Result<String, Errno> {
    let cwd = active_process_cwd()?;
    let root = active_process_root()?;
    BootVfs::resolve_path_from_root(&root, &cwd, text)
}

fn resolved_path_from_user(ptr_value: usize, len: usize) -> Result<String, Errno> {
    let text = path_from_user(ptr_value, len)?;
    resolve_active_process_path(text)
}

fn resolve_path_from_descriptor(dirfd: usize, text: &str) -> Result<String, Errno> {
    if text.starts_with('/') {
        return resolve_active_process_path(text);
    }
    let root = active_process_root()?;
    let anchor = DESCRIPTORS.with(|table| {
        let descriptor = table.descriptor(dirfd)?;
        match descriptor.target {
            DescriptorTarget::BootDirectory(inode) => {
                BOOT_VFS.with_mut(|vfs| vfs.live_path_for_inode(inode).ok_or(Errno::NoEnt))
            }
            DescriptorTarget::Procfs(node)
                if matches!(
                    node.kind,
                    BootProcfsNodeKind::ProcRootDir
                        | BootProcfsNodeKind::ProcessDir
                        | BootProcfsNodeKind::FdDirListing
                        | BootProcfsNodeKind::FdInfoDirListing
                ) =>
            {
                descriptor_target_path_text(DescriptorTarget::Procfs(node))
            }
            _ => Err(Errno::NotDir),
        }
    })?;
    BootVfs::resolve_path_from_root(&root, &anchor, text)
}

fn resolved_path_at_from_user(dirfd: usize, ptr_value: usize, len: usize) -> Result<String, Errno> {
    let text = path_from_user(ptr_value, len)?;
    resolve_path_from_descriptor(dirfd, &text)
}

fn resolved_at_target_from_user(
    dirfd: usize,
    ptr_value: usize,
    len: usize,
) -> Result<(ResolvedAtTarget, String), Errno> {
    let text = path_from_user(ptr_value, len)?;
    if text.is_empty() {
        let target = DESCRIPTORS.with(|descriptors| Ok(descriptors.descriptor(dirfd)?.target))?;
        return Ok((ResolvedAtTarget::Handle(target), String::new()));
    }
    let path = resolve_path_from_descriptor(dirfd, &text)?;
    Ok((ResolvedAtTarget::Path, path))
}

fn stat_descriptor_target(target: DescriptorTarget) -> Result<NativeFileStatusRecord, Errno> {
    match target {
        DescriptorTarget::BootDirectory(inode)
        | DescriptorTarget::BootFile(inode)
        | DescriptorTarget::BootChannel(inode) => BOOT_VFS
            .with_mut(|vfs| vfs.stat_by_inode(inode))
            .ok_or(Errno::Badf),
        DescriptorTarget::Procfs(node) => {
            let path = descriptor_target_path_text(DescriptorTarget::Procfs(node))?;
            let payload = boot_procfs_payload(node.pid, node.kind)?;
            Ok(NativeFileStatusRecord {
                inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                    acc.wrapping_mul(131).wrapping_add(*byte as u64)
                }),
                link_count: 1,
                size: payload.len() as u64,
                kind: match node.kind {
                    BootProcfsNodeKind::ProcRootDir
                    | BootProcfsNodeKind::ProcessDir
                    | BootProcfsNodeKind::FdDirListing
                    | BootProcfsNodeKind::FdInfoDirListing => NativeObjectKind::Directory as u32,
                    _ => NativeObjectKind::File as u32,
                },
                cloexec: 0,
                nonblock: 0,
                readable: 1,
                writable: 0,
                executable: u32::from(matches!(
                    node.kind,
                    BootProcfsNodeKind::ProcRootDir
                        | BootProcfsNodeKind::ProcessDir
                        | BootProcfsNodeKind::FdDirListing
                        | BootProcfsNodeKind::FdInfoDirListing
                )),
                owner_uid: 0,
                group_gid: 0,
                mode: if matches!(
                    node.kind,
                    BootProcfsNodeKind::ProcRootDir
                        | BootProcfsNodeKind::ProcessDir
                        | BootProcfsNodeKind::FdDirListing
                        | BootProcfsNodeKind::FdInfoDirListing
                ) {
                    0o555
                } else {
                    0o444
                },
            })
        }
        _ => {
            let path = descriptor_target_path_text(target)?;
            if matches!(boot_stream_target(&path), Some(_)) {
                Ok(NativeFileStatusRecord {
                    inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                        acc.wrapping_mul(131).wrapping_add(*byte as u64)
                    }),
                    link_count: 1,
                    size: 0,
                    kind: NativeObjectKind::Device as u32,
                    cloexec: 0,
                    nonblock: 0,
                    readable: 0,
                    writable: 1,
                    executable: 0,
                    owner_uid: 0,
                    group_gid: 0,
                    mode: 0o222,
                })
            } else {
                let inode = crate::virtio_blk_boot::inode_for_path(&path).ok_or(Errno::NoEnt)?;
                let (size, kind, readable, writable) = if path
                    == crate::virtio_blk_boot::STORAGE_DEVICE_PATH
                {
                    let info = crate::virtio_blk_boot::device_record(&path).ok_or(Errno::Nxio)?;
                    (info.capacity_bytes, NativeObjectKind::Device as u32, 1, 1)
                } else {
                    (0, NativeObjectKind::Driver as u32, 1, 0)
                };
                Ok(NativeFileStatusRecord {
                    inode,
                    link_count: 1,
                    size,
                    kind,
                    cloexec: 0,
                    nonblock: 0,
                    readable,
                    writable,
                    executable: 0,
                    owner_uid: 0,
                    group_gid: 0,
                    mode: if writable != 0 { 0o666 } else { 0o444 },
                })
            }
        }
    }
}

fn stat_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileStatusRecord,
) -> Result<usize, Errno> {
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    if let ResolvedAtTarget::Handle(target) = resolved {
        DESCRIPTORS.with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::READ))?;
        write_record(out, stat_descriptor_target(target)?)?;
        return Ok(0);
    }
    if let Some(record) = boot_vfs_stat(&path) {
        write_record(out, record)?;
        return Ok(0);
    }
    if let Some(node) =
        boot_procfs_node(&path)?.or_else(|| boot_procfs_directory_node(&path).ok().flatten())
    {
        let payload = boot_procfs_payload(node.pid, node.kind)?;
        write_record(
            out,
            NativeFileStatusRecord {
                inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                    acc.wrapping_mul(131).wrapping_add(*byte as u64)
                }),
                link_count: 1,
                size: payload.len() as u64,
                kind: match node.kind {
                    BootProcfsNodeKind::ProcRootDir
                    | BootProcfsNodeKind::ProcessDir
                    | BootProcfsNodeKind::FdDirListing
                    | BootProcfsNodeKind::FdInfoDirListing => NativeObjectKind::Directory as u32,
                    _ => NativeObjectKind::File as u32,
                },
                cloexec: 0,
                nonblock: 0,
                readable: 1,
                writable: 0,
                executable: u32::from(matches!(
                    node.kind,
                    BootProcfsNodeKind::ProcRootDir
                        | BootProcfsNodeKind::ProcessDir
                        | BootProcfsNodeKind::FdDirListing
                        | BootProcfsNodeKind::FdInfoDirListing
                )),
                owner_uid: 0,
                group_gid: 0,
                mode: if matches!(
                    node.kind,
                    BootProcfsNodeKind::ProcRootDir
                        | BootProcfsNodeKind::ProcessDir
                        | BootProcfsNodeKind::FdDirListing
                        | BootProcfsNodeKind::FdInfoDirListing
                ) {
                    0o555
                } else {
                    0o444
                },
            },
        )?;
        return Ok(0);
    }
    if matches!(boot_stream_target(&path), Some(_)) {
        write_record(
            out,
            NativeFileStatusRecord {
                inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                    acc.wrapping_mul(131).wrapping_add(*byte as u64)
                }),
                link_count: 1,
                size: 0,
                kind: NativeObjectKind::Device as u32,
                cloexec: 0,
                nonblock: 0,
                readable: 0,
                writable: 1,
                executable: 0,
                owner_uid: 0,
                group_gid: 0,
                mode: 0o222,
            },
        )?;
        return Ok(0);
    }
    let inode = crate::virtio_blk_boot::inode_for_path(&path).ok_or(Errno::NoEnt)?;
    let (size, kind, readable, writable) = if path == crate::virtio_blk_boot::STORAGE_DEVICE_PATH {
        let info = crate::virtio_blk_boot::device_record(&path).ok_or(Errno::Nxio)?;
        (info.capacity_bytes, NativeObjectKind::Device as u32, 1, 1)
    } else {
        (0, NativeObjectKind::Driver as u32, 1, 0)
    };
    write_record(
        out,
        NativeFileStatusRecord {
            inode,
            link_count: 1,
            size,
            kind,
            cloexec: 0,
            nonblock: 0,
            readable,
            writable,
            executable: 0,
            owner_uid: 0,
            group_gid: 0,
            mode: if writable != 0 { 0o666 } else { 0o444 },
        },
    )?;
    Ok(0)
}

fn stat_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileStatusRecord,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Some(record) = boot_vfs_stat(&path) {
        write_record(out, record)?;
        return Ok(0);
    }
    if let Some(node) =
        boot_procfs_node(&path)?.or_else(|| boot_procfs_directory_node(&path).ok().flatten())
    {
        let payload = boot_procfs_payload(node.pid, node.kind)?;
        write_record(
            out,
            NativeFileStatusRecord {
                inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                    acc.wrapping_mul(131).wrapping_add(*byte as u64)
                }),
                link_count: 1,
                size: payload.len() as u64,
                kind: match node.kind {
                    BootProcfsNodeKind::ProcRootDir
                    | BootProcfsNodeKind::ProcessDir
                    | BootProcfsNodeKind::FdDirListing
                    | BootProcfsNodeKind::FdInfoDirListing => NativeObjectKind::Directory as u32,
                    _ => NativeObjectKind::File as u32,
                },
                cloexec: 0,
                nonblock: 0,
                readable: 1,
                writable: 0,
                executable: u32::from(matches!(
                    node.kind,
                    BootProcfsNodeKind::ProcRootDir
                        | BootProcfsNodeKind::ProcessDir
                        | BootProcfsNodeKind::FdDirListing
                        | BootProcfsNodeKind::FdInfoDirListing
                )),
                owner_uid: 0,
                group_gid: 0,
                mode: if matches!(
                    node.kind,
                    BootProcfsNodeKind::ProcRootDir
                        | BootProcfsNodeKind::ProcessDir
                        | BootProcfsNodeKind::FdDirListing
                        | BootProcfsNodeKind::FdInfoDirListing
                ) {
                    0o555
                } else {
                    0o444
                },
            },
        )?;
        return Ok(0);
    }
    if matches!(boot_stream_target(&path), Some(_)) {
        write_record(
            out,
            NativeFileStatusRecord {
                inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                    acc.wrapping_mul(131).wrapping_add(*byte as u64)
                }),
                link_count: 1,
                size: 0,
                kind: NativeObjectKind::Device as u32,
                cloexec: 0,
                nonblock: 0,
                readable: 0,
                writable: 1,
                executable: 0,
                owner_uid: 0,
                group_gid: 0,
                mode: 0o222,
            },
        )?;
        return Ok(0);
    }
    let inode = crate::virtio_blk_boot::inode_for_path(&path).ok_or(Errno::NoEnt)?;
    let (size, kind, readable, writable) = if path == crate::virtio_blk_boot::STORAGE_DEVICE_PATH {
        let info = crate::virtio_blk_boot::device_record(&path).ok_or(Errno::Nxio)?;
        (info.capacity_bytes, NativeObjectKind::Device as u32, 1, 1)
    } else {
        (0, NativeObjectKind::Driver as u32, 1, 0)
    };
    write_record(
        out,
        NativeFileStatusRecord {
            inode,
            link_count: 1,
            size,
            kind,
            cloexec: 0,
            nonblock: 0,
            readable,
            writable,
            executable: 0,
            owner_uid: 0,
            group_gid: 0,
            mode: if writable != 0 { 0o666 } else { 0o444 },
        },
    )?;
    Ok(0)
}

fn lstat_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileStatusRecord,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Some(record) = boot_vfs_lstat(&path) {
        write_record(out, record)?;
        return Ok(0);
    }
    stat_path_syscall(path_ptr, path_len, out)
}

fn lstat_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileStatusRecord,
) -> Result<usize, Errno> {
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    if let ResolvedAtTarget::Handle(target) = resolved {
        DESCRIPTORS.with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::READ))?;
        write_record(out, stat_descriptor_target(target)?)?;
        return Ok(0);
    }
    if let Some(record) = boot_vfs_lstat(&path) {
        write_record(out, record)?;
        return Ok(0);
    }
    stat_path_at_syscall(dirfd, path_ptr, path_len, out)
}

fn statfs_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileSystemStatusRecord,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if boot_vfs_stat(&path).is_some() {
        let storage_mount = storage_mount_state();
        write_record(
            out,
            NativeFileSystemStatusRecord {
                mount_count: 1 + storage_mount.mounts.len() as u64,
                node_count: BOOT_VFS.with_mut(|vfs| vfs.nodes.len()) as u64,
                read_only: 0,
                reserved: 0,
            },
        )?;
        return Ok(0);
    }
    if crate::virtio_blk_boot::inode_for_path(&path).is_none() {
        return Err(Errno::NoEnt);
    }
    write_record(
        out,
        NativeFileSystemStatusRecord {
            mount_count: 1,
            node_count: 2,
            read_only: 0,
            reserved: 0,
        },
    )?;
    Ok(0)
}

fn mkdir_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Err(error) = boot_vfs_create(&path, BootNodeKind::Directory) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Created, 0);
    Ok(0)
}

fn mkdir_path_at_syscall(dirfd: usize, path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_at_from_user(dirfd, path_ptr, path_len)?;
    if let Err(error) = boot_vfs_create(&path, BootNodeKind::Directory) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Created, 0);
    Ok(0)
}

fn mkfile_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Err(error) = boot_vfs_create(&path, BootNodeKind::File) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Created, 0);
    Ok(0)
}

fn mkfile_path_at_syscall(dirfd: usize, path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_at_from_user(dirfd, path_ptr, path_len)?;
    if let Err(error) = boot_vfs_create(&path, BootNodeKind::File) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Created, 0);
    Ok(0)
}

fn mkchan_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Err(error) = boot_vfs_create(&path, BootNodeKind::Channel) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    Ok(0)
}

fn mksock_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Err(error) = boot_vfs_create(&path, BootNodeKind::Channel) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    Ok(0)
}

fn mount_storage_volume_syscall(
    device_ptr: usize,
    device_len: usize,
    mount_ptr: usize,
    mount_len: usize,
) -> Result<usize, Errno> {
    let device_path = path_from_user(device_ptr, device_len)?;
    let mount_path = resolved_path_from_user(mount_ptr, mount_len)?;
    let mount_path = BootVfs::normalize_path(&mount_path)?;
    let entries = crate::virtio_blk_boot::read_storage_snapshot(device_path)?;
    let parent_mount = storage_mount_parent(&mount_path);
    let parent_mount_id = parent_mount.as_ref().map(|record| record.id).unwrap_or(0);
    let default_mode = match parent_mount
        .as_ref()
        .and_then(|record| NativeMountPropagationMode::from_raw(record.propagation_mode))
    {
        Some(NativeMountPropagationMode::Shared) => NativeMountPropagationMode::Shared,
        Some(NativeMountPropagationMode::Slave) => NativeMountPropagationMode::Slave,
        _ => NativeMountPropagationMode::Private,
    };
    let mount_ids = STORAGE_MOUNT.with_mut(|state| {
        if state
            .mounts
            .iter()
            .any(|record| record.mount_path == mount_path)
        {
            return None;
        }
        let primary_id = state.next_id.max(1);
        state.next_id = primary_id.saturating_add(1);
        let mut planned = vec![(primary_id, mount_path.clone(), default_mode)];
        if matches!(default_mode, NativeMountPropagationMode::Shared) {
            if let Some(parent) = &parent_mount {
                if let Some(relative_suffix) =
                    storage_mount_relative_suffix(&parent.mount_path, &mount_path)
                {
                    for peer in state.mounts.iter() {
                        let clone_mode = if peer.id != parent.id
                            && peer.peer_group == parent.peer_group
                            && peer.peer_group != 0
                            && peer.propagation_mode == NativeMountPropagationMode::Shared as u32
                        {
                            Some(NativeMountPropagationMode::Shared)
                        } else if peer.id != parent.id
                            && peer.master_group == parent.peer_group
                            && parent.peer_group != 0
                            && peer.propagation_mode == NativeMountPropagationMode::Slave as u32
                        {
                            Some(NativeMountPropagationMode::Slave)
                        } else {
                            None
                        };
                        if let Some(clone_mode) = clone_mode {
                            let clone_id = state.next_id.max(1);
                            state.next_id = clone_id.saturating_add(1);
                            planned.push((
                                clone_id,
                                format!("{}{}", peer.mount_path, relative_suffix),
                                clone_mode,
                            ));
                        }
                    }
                }
            }
        }
        if planned
            .iter()
            .any(|(_, path, _)| state.mounts.iter().any(|record| record.mount_path == *path))
        {
            return None;
        }
        Some(planned)
    });
    let Some(planned_mounts) = mount_ids else {
        return Err(Errno::Exist);
    };
    let primary_id = planned_mounts[0].0;
    let primary_peer_group = if matches!(default_mode, NativeMountPropagationMode::Shared) {
        primary_id
    } else {
        0
    };
    let primary_master_group = if matches!(default_mode, NativeMountPropagationMode::Slave) {
        parent_mount
            .as_ref()
            .map(|record| record.master_group.max(record.peer_group))
            .unwrap_or(0)
    } else {
        0
    };
    let mut applied = Vec::<(u64, String, usize, bool, NativeMountPropagationMode)>::new();
    for (mount_id, target_path, mode) in planned_mounts.iter() {
        let (loaded, created_mount_root) = match BOOT_VFS
            .with_mut(|vfs| apply_persist_entries(vfs, *mount_id, target_path, &entries))
        {
            Ok(result) => result,
            Err(error) => {
                BOOT_VFS.with_mut(|vfs| {
                    for (applied_id, _, _, _, _) in &applied {
                        vfs.nodes.retain(|node| node.mount_id != Some(*applied_id));
                    }
                    vfs.invalidate_caches();
                });
                return Err(error);
            }
        };
        applied.push((
            *mount_id,
            target_path.clone(),
            loaded,
            created_mount_root,
            *mode,
        ));
    }
    STORAGE_MOUNT.with_mut(|state| {
        for (mount_id, target_path, loaded, created_mount_root, mode) in &applied {
            let (peer_group, master_group) = match mode {
                NativeMountPropagationMode::Shared => (primary_peer_group, 0),
                NativeMountPropagationMode::Slave => {
                    (0, primary_peer_group.max(primary_master_group))
                }
                NativeMountPropagationMode::Private => (0, 0),
            };
            let nested_prefix = format!("{target_path}/");
            let parent_id = state
                .mounts
                .iter()
                .filter(|record| {
                    target_path != &record.mount_path
                        && target_path.starts_with(&(record.mount_path.clone() + "/"))
                        && !record.mount_path.starts_with(&nested_prefix)
                })
                .max_by_key(|record| record.mount_path.len())
                .map(|record| record.id)
                .unwrap_or(parent_mount_id);
            state.mounts.push(StorageMountRecord {
                id: *mount_id,
                device_path: device_path.to_string(),
                mount_path: target_path.clone(),
                parent_mount_id: parent_id,
                peer_group,
                master_group,
                propagation_mode: *mode as u32,

                entry_count: *loaded,
                created_mount_root: *created_mount_root,
            });
        }
    });
    for (_, target_path, _, _, _) in &applied {
        let inode = BOOT_VFS.with_mut(|vfs| {
            let index = vfs.resolve_node_index(target_path, true)?;
            Ok::<u64, Errno>(vfs.nodes[index].inode)
        })?;
        emit_vfs_event(
            inode,
            Some(target_path),
            None,
            NativeVfsEventKind::Mounted,
            0,
        );
    }
    Ok(applied
        .first()
        .map(|(_, _, loaded, _, _)| *loaded)
        .unwrap_or(0))
}

fn unmount_storage_volume_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let mount_path = resolved_path_from_user(path_ptr, path_len)?;
    let mount_path = BootVfs::normalize_path(&mount_path)?;
    let Some(state) = storage_mount_by_path(&mount_path) else {
        return Err(Errno::NoEnt);
    };
    let unmount_events = BOOT_VFS.with_mut(|vfs| {
        let mut events = Vec::new();
        if let Some(index) = vfs.find_node(&mount_path) {
            events.push((vfs.nodes[index].inode, mount_path.clone()));
        }
        Ok::<Vec<(u64, String)>, Errno>(events)
    })?;
    let active_ids = storage_mount_unmount_ids(&state);
    let targets = STORAGE_MOUNT.with_mut(|registry| {
        registry
            .mounts
            .iter()
            .filter(|record| active_ids.iter().any(|id| *id == record.id))
            .cloned()
            .collect::<Vec<_>>()
    });
    for target in &targets {
        if storage_mount_has_nested_child_outside(&target.mount_path, &active_ids) {
            return Err(Errno::Busy);
        }
    }
    let mut generation = 0usize;
    for target in &targets {
        let entries =
            BOOT_VFS.with_mut(|vfs| collect_persist_entries(vfs, target.id, &target.mount_path))?;
        generation = crate::virtio_blk_boot::write_storage_snapshot(
            &target.device_path,
            "boot-vfs-unmount",
            &entries,
        )?;
    }
    BOOT_VFS.with_mut(|vfs| {
        for target in &targets {
            vfs.nodes.retain(|node| node.mount_id != Some(target.id));
            if target.created_mount_root {
                let prefix = format!("{}/", target.mount_path);
                if vfs
                    .nodes
                    .iter()
                    .all(|node| node.path != target.mount_path && !node.path.starts_with(&prefix))
                {
                    if let Some(index) = vfs.find_node(&target.mount_path) {
                        if vfs.nodes[index].mount_id.is_none()
                            && vfs.nodes[index].kind == BootNodeKind::Directory
                        {
                            vfs.nodes.remove(index);
                        }
                    }
                }
            }
        }
        vfs.invalidate_caches();
    });
    STORAGE_MOUNT.with_mut(|registry| {
        registry
            .mounts
            .retain(|record| !active_ids.iter().any(|id| *id == record.id));
    });
    for (inode, path) in unmount_events {
        emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Unmounted, 0);
    }
    Ok(generation)
}

fn inspect_mount_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeMountRecord,
) -> Result<usize, Errno> {
    let mount_path = resolved_path_from_user(path_ptr, path_len)?;
    let mount_path = BootVfs::normalize_path(&mount_path)?;
    let Some(record) = storage_mount_by_path(&mount_path) else {
        return Err(Errno::NoEnt);
    };
    write_record(
        out,
        NativeMountRecord {
            id: record.id,
            parent_mount_id: record.parent_mount_id,
            peer_group: record.peer_group,
            master_group: record.master_group,
            layer: record.id,
            entry_count: record.entry_count as u64,
            propagation_mode: record.propagation_mode,
            created_mount_root: u32::from(record.created_mount_root),
        },
    )?;
    Ok(0)
}

fn set_mount_propagation_syscall(
    path_ptr: usize,
    path_len: usize,
    mode_raw: u32,
) -> Result<usize, Errno> {
    let mount_path = resolved_path_from_user(path_ptr, path_len)?;
    let mount_path = BootVfs::normalize_path(&mount_path)?;
    let mode = NativeMountPropagationMode::from_raw(mode_raw).ok_or(Errno::Inval)?;
    let (updated_record, shared_source) = STORAGE_MOUNT.with_mut(|state| {
        let Some(index) = state
            .mounts
            .iter()
            .position(|record| record.mount_path == mount_path)
        else {
            return Err(Errno::NoEnt);
        };
        let record_id = state.mounts[index].id;
        let device_path = state.mounts[index].device_path.clone();
        let parent_mount_id = state.mounts[index].parent_mount_id;
        let shared_group_candidate = state
            .mounts
            .iter()
            .find(|candidate| {
                candidate.id != record_id
                    && candidate.device_path == device_path
                    && candidate.parent_mount_id == parent_mount_id
                    && candidate.propagation_mode == NativeMountPropagationMode::Shared as u32
                    && candidate.peer_group != 0
            })
            .map(|candidate| candidate.peer_group);
        let shared_source = if matches!(mode, NativeMountPropagationMode::Slave) {
            state
                .mounts
                .iter()
                .find(|candidate| {
                    candidate.id != record_id
                        && candidate.device_path == device_path
                        && candidate.parent_mount_id == parent_mount_id
                        && candidate.propagation_mode == NativeMountPropagationMode::Shared as u32
                        && candidate.peer_group == shared_group_candidate.unwrap_or(0)
                })
                .cloned()
        } else {
            None
        };
        let record = &mut state.mounts[index];
        match mode {
            NativeMountPropagationMode::Private => {
                record.propagation_mode = NativeMountPropagationMode::Private as u32;
                record.peer_group = 0;
                record.master_group = 0;
            }
            NativeMountPropagationMode::Shared => {
                let group = shared_group_candidate.unwrap_or(record.id);
                record.propagation_mode = NativeMountPropagationMode::Shared as u32;
                record.peer_group = group;
                record.master_group = 0;
            }
            NativeMountPropagationMode::Slave => {
                let Some(group) = shared_group_candidate else {
                    return Err(Errno::NoEnt);
                };
                record.propagation_mode = NativeMountPropagationMode::Slave as u32;
                record.peer_group = 0;
                record.master_group = group;
            }
        }
        Ok::<(StorageMountRecord, Option<StorageMountRecord>), Errno>((
            record.clone(),
            shared_source,
        ))
    })?;
    match mode {
        NativeMountPropagationMode::Shared => {
            storage_mount_promote_descendants_to_shared(&updated_record);
            let targets = STORAGE_MOUNT.with_mut(|state| {
                state
                    .mounts
                    .iter()
                    .filter(|candidate| {
                        candidate.id != updated_record.id
                            && ((candidate.peer_group != 0
                                && candidate.peer_group == updated_record.peer_group
                                && candidate.propagation_mode
                                    == NativeMountPropagationMode::Shared as u32)
                                || (candidate.master_group != 0
                                    && candidate.master_group == updated_record.peer_group
                                    && candidate.propagation_mode
                                        == NativeMountPropagationMode::Slave as u32))
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            });
            for target in targets {
                storage_mount_clone_existing_descendants(&updated_record, &target)?;
            }
        }
        NativeMountPropagationMode::Slave => {
            storage_mount_privatize_descendants(&updated_record);
            if let Some(source_root) = shared_source {
                storage_mount_clone_existing_descendants(&source_root, &updated_record)?;
                storage_mount_rebind_descendants_to_slave(&source_root, &updated_record);
            }
        }
        NativeMountPropagationMode::Private => {
            storage_mount_privatize_descendants(&updated_record);
        }
    }
    Ok(0)
}

fn symlink_path_syscall(
    path_ptr: usize,
    path_len: usize,
    target_ptr: usize,
    target_len: usize,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let target = path_from_user(target_ptr, target_len)?;
    if let Err(error) = boot_vfs_symlink(&path, target) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, false)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Created, 0);
    Ok(0)
}

fn symlink_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    target_ptr: usize,
    target_len: usize,
) -> Result<usize, Errno> {
    let path = resolved_path_at_from_user(dirfd, path_ptr, path_len)?;
    let target = path_from_user(target_ptr, target_len)?;
    if let Err(error) = boot_vfs_symlink(&path, target) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, false)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Created, 0);
    Ok(0)
}

fn chmod_path_syscall(path_ptr: usize, path_len: usize, mode: u32) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Err(error) = boot_vfs_chmod(&path, mode) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_path(&path, error);
        }
        return Err(error);
    }
    Ok(0)
}

fn chmod_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    mode: u32,
) -> Result<usize, Errno> {
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    match resolved {
        ResolvedAtTarget::Handle(DescriptorTarget::BootDirectory(inode))
        | ResolvedAtTarget::Handle(DescriptorTarget::BootFile(inode))
        | ResolvedAtTarget::Handle(DescriptorTarget::BootChannel(inode)) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::WRITE))?;
            if let Err(error) = BOOT_VFS.with_mut(|vfs| vfs.chmod_by_inode(inode, mode)) {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&path, error);
                }
                return Err(error);
            }
        }
        ResolvedAtTarget::Handle(_) => return Err(Errno::Perm),
        ResolvedAtTarget::Path => {
            if let Err(error) = boot_vfs_chmod(&path, mode) {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&path, error);
                }
                return Err(error);
            }
        }
    }
    Ok(0)
}

fn chown_path_syscall(
    path_ptr: usize,
    path_len: usize,
    owner_uid: u32,
    group_gid: u32,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    if let Err(error) = boot_vfs_chown(&path, owner_uid, group_gid) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_path(&path, error);
        }
        return Err(error);
    }
    Ok(0)
}

fn chown_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    owner_uid: u32,
    group_gid: u32,
) -> Result<usize, Errno> {
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    match resolved {
        ResolvedAtTarget::Handle(DescriptorTarget::BootDirectory(inode))
        | ResolvedAtTarget::Handle(DescriptorTarget::BootFile(inode))
        | ResolvedAtTarget::Handle(DescriptorTarget::BootChannel(inode)) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::WRITE))?;
            if let Err(error) =
                BOOT_VFS.with_mut(|vfs| vfs.chown_by_inode(inode, owner_uid, group_gid))
            {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&path, error);
                }
                return Err(error);
            }
        }
        ResolvedAtTarget::Handle(_) => return Err(Errno::Perm),
        ResolvedAtTarget::Path => {
            if let Err(error) = boot_vfs_chown(&path, owner_uid, group_gid) {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&path, error);
                }
                return Err(error);
            }
        }
    }
    Ok(0)
}

fn rename_path_syscall(
    from_ptr: usize,
    from_len: usize,
    to_ptr: usize,
    to_len: usize,
) -> Result<usize, Errno> {
    let from = resolved_path_from_user(from_ptr, from_len)?;
    let to = resolved_path_from_user(to_ptr, to_len)?;
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&from, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    if let Err(error) = boot_vfs_rename(&from, &to) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&from, error);
            emit_permission_refusal_for_parent_path(&to, error);
        }
        return Err(error);
    }
    emit_vfs_event(
        inode,
        Some(&from),
        Some(&to),
        NativeVfsEventKind::Renamed,
        0,
    );
    Ok(0)
}

fn rename_path_at_syscall(
    from_dirfd: usize,
    from_ptr: usize,
    from_len: usize,
    to_dirfd: usize,
    to_ptr: usize,
    to_len: usize,
) -> Result<usize, Errno> {
    let from = resolved_path_at_from_user(from_dirfd, from_ptr, from_len)?;
    let to = resolved_path_at_from_user(to_dirfd, to_ptr, to_len)?;
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&from, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    if let Err(error) = boot_vfs_rename(&from, &to) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&from, error);
            emit_permission_refusal_for_parent_path(&to, error);
        }
        return Err(error);
    }
    emit_vfs_event(
        inode,
        Some(&from),
        Some(&to),
        NativeVfsEventKind::Renamed,
        0,
    );
    Ok(0)
}

fn unlink_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, false)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    if let Err(error) = boot_vfs_unlink(&path) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let _ = crate::boot_network_runtime::remove_socket(&path);
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Unlinked, 0);
    Ok(0)
}

fn unlink_path_at_syscall(dirfd: usize, path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_at_from_user(dirfd, path_ptr, path_len)?;
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, false)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    if let Err(error) = boot_vfs_unlink(&path) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_parent_path(&path, error);
        }
        return Err(error);
    }
    let _ = crate::boot_network_runtime::remove_socket(&path);
    emit_vfs_event(inode, Some(&path), None, NativeVfsEventKind::Unlinked, 0);
    Ok(0)
}

fn truncate_path_syscall(path_ptr: usize, path_len: usize, len: usize) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let inode = BOOT_VFS.with_mut(|vfs| {
        let normalized = BootVfs::normalize_path(&path)?;
        let index = vfs.resolve_node_index(&normalized, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    if let Err(error) = boot_vfs_truncate(&path, len) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_path(&path, error);
        }
        return Err(error);
    }
    BOOT_PROCESSES.with_mut(|registry| {
        for entry in &mut registry.entries {
            for object in entry
                .vm_objects
                .iter_mut()
                .filter(|object| object.backing_inode == Some(inode))
            {
                let object_len = object.len as usize;
                object.bytes.resize(object_len, 0);
                let backing = BOOT_VFS.with_mut(|vfs| {
                    vfs.object_bytes_range_by_inode(inode, object.file_offset as usize, object_len)
                        .ok_or(Errno::Badf)
                })?;
                object.bytes = backing;
            }
        }
        Ok::<(), Errno>(())
    })?;
    emit_vfs_event(
        inode,
        Some(&path),
        None,
        NativeVfsEventKind::Truncated,
        len as u32,
    );
    Ok(0)
}

fn truncate_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    len: usize,
) -> Result<usize, Errno> {
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    let inode = match resolved {
        ResolvedAtTarget::Handle(DescriptorTarget::BootFile(inode))
        | ResolvedAtTarget::Handle(DescriptorTarget::BootChannel(inode)) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::WRITE))?;
            if let Err(error) = BOOT_VFS.with_mut(|vfs| vfs.truncate_by_inode(inode, len)) {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&path, error);
                }
                return Err(error);
            }
            inode
        }
        ResolvedAtTarget::Handle(DescriptorTarget::BootDirectory(_)) => return Err(Errno::IsDir),
        ResolvedAtTarget::Handle(_) => return Err(Errno::Inval),
        ResolvedAtTarget::Path => {
            let inode = BOOT_VFS.with_mut(|vfs| {
                let normalized = BootVfs::normalize_path(&path)?;
                let index = vfs.resolve_node_index(&normalized, true)?;
                Ok::<u64, Errno>(vfs.nodes[index].inode)
            })?;
            if let Err(error) = boot_vfs_truncate(&path, len) {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&path, error);
                }
                return Err(error);
            }
            inode
        }
    };
    BOOT_PROCESSES.with_mut(|registry| {
        for entry in &mut registry.entries {
            for object in entry
                .vm_objects
                .iter_mut()
                .filter(|object| object.backing_inode == Some(inode))
            {
                let object_len = object.len as usize;
                object.bytes.resize(object_len, 0);
                let backing = BOOT_VFS.with_mut(|vfs| {
                    vfs.object_bytes_range_by_inode(inode, object.file_offset as usize, object_len)
                        .ok_or(Errno::Badf)
                })?;
                object.bytes = backing;
            }
        }
        Ok::<(), Errno>(())
    })?;
    emit_vfs_event(
        inode,
        (!path.is_empty()).then_some(path.as_str()),
        None,
        NativeVfsEventKind::Truncated,
        len as u32,
    );
    Ok(0)
}

fn link_path_syscall(
    source_ptr: usize,
    source_len: usize,
    destination_ptr: usize,
    destination_len: usize,
) -> Result<usize, Errno> {
    let source = resolved_path_from_user(source_ptr, source_len)?;
    let destination = resolved_path_from_user(destination_ptr, destination_len)?;
    let inode = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&source, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    })?;
    if let Err(error) = boot_vfs_link(&source, &destination) {
        if matches!(error, Errno::Access | Errno::Perm) {
            emit_permission_refusal_for_path(&source, error);
            emit_permission_refusal_for_parent_path(&destination, error);
        }
        return Err(error);
    }
    emit_vfs_event(
        inode,
        Some(&source),
        Some(&destination),
        NativeVfsEventKind::Linked,
        0,
    );
    Ok(0)
}

fn link_path_at_syscall(
    source_dirfd: usize,
    source_ptr: usize,
    source_len: usize,
    destination_dirfd: usize,
    destination_ptr: usize,
    destination_len: usize,
) -> Result<usize, Errno> {
    let (source_kind, source) = resolved_at_target_from_user(source_dirfd, source_ptr, source_len)?;
    let destination =
        resolved_path_at_from_user(destination_dirfd, destination_ptr, destination_len)?;
    let inode = match source_kind {
        ResolvedAtTarget::Handle(DescriptorTarget::BootFile(inode)) => {
            DESCRIPTORS.with(|descriptors| {
                descriptors.require_rights(
                    source_dirfd,
                    BlockRightsMask::WRITE.union(BlockRightsMask::DELEGATE),
                )
            })?;
            if let Err(error) = BOOT_VFS.with_mut(|vfs| vfs.link_inode_to_path(inode, &destination))
            {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&source, error);
                    emit_permission_refusal_for_parent_path(&destination, error);
                }
                return Err(error);
            }
            inode
        }
        ResolvedAtTarget::Handle(_) => return Err(Errno::Perm),
        ResolvedAtTarget::Path => {
            let inode = BOOT_VFS.with_mut(|vfs| {
                let index = vfs.resolve_node_index(&source, true)?;
                Ok::<u64, Errno>(vfs.nodes[index].inode)
            })?;
            if let Err(error) = boot_vfs_link(&source, &destination) {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&source, error);
                    emit_permission_refusal_for_parent_path(&destination, error);
                }
                return Err(error);
            }
            inode
        }
    };
    emit_vfs_event(
        inode,
        (!source.is_empty()).then_some(source.as_str()),
        Some(&destination),
        NativeVfsEventKind::Linked,
        0,
    );
    Ok(0)
}

fn readlink_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let target = match boot_vfs_readlink(&path) {
        Ok(target) => target,
        Err(error) => {
            if matches!(error, Errno::Access | Errno::Perm) {
                emit_permission_refusal_for_path(&path, error);
            }
            return Err(error);
        }
    };
    copy_text_to_user(&target, out, capacity)
}

fn readlink_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    out: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    let path = resolved_path_at_from_user(dirfd, path_ptr, path_len)?;
    let target = match boot_vfs_readlink(&path) {
        Ok(target) => target,
        Err(error) => {
            if matches!(error, Errno::Access | Errno::Perm) {
                emit_permission_refusal_for_path(&path, error);
            }
            return Err(error);
        }
    };
    copy_text_to_user(&target, out, capacity)
}

fn open_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let fd = match DESCRIPTORS.with_mut(|descriptors| descriptors.open_path(&path)) {
        Ok(fd) => fd,
        Err(Errno::Access) => {
            if let Ok(inode) = BOOT_VFS.with_mut(|vfs| {
                let index = vfs.resolve_node_index(&path, false)?;
                Ok::<u64, Errno>(vfs.nodes[index].inode)
            }) {
                emit_vfs_event(
                    inode,
                    Some(&path),
                    None,
                    NativeVfsEventKind::PermissionRefused,
                    Errno::Access as u32,
                );
            }
            return Err(Errno::Access);
        }
        Err(error) => return Err(error),
    };
    if let Ok(inode) = BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, true)?;
        Ok::<u64, Errno>(vfs.nodes[index].inode)
    }) {
        emit_vfs_event(
            inode,
            Some(&path),
            None,
            NativeVfsEventKind::Opened,
            fd as u32,
        );
    }
    Ok(fd)
}

fn open_path_at_syscall(dirfd: usize, path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    let fd = match resolved {
        ResolvedAtTarget::Handle(_) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::DELEGATE))?;
            DESCRIPTORS.with_mut(|descriptors| descriptors.duplicate(dirfd))?
        }
        ResolvedAtTarget::Path => {
            match DESCRIPTORS.with_mut(|descriptors| descriptors.open_path(&path)) {
                Ok(fd) => fd,
                Err(Errno::Access) => {
                    if let Ok(inode) = BOOT_VFS.with_mut(|vfs| {
                        let index = vfs.resolve_node_index(&path, false)?;
                        Ok::<u64, Errno>(vfs.nodes[index].inode)
                    }) {
                        emit_vfs_event(
                            inode,
                            Some(&path),
                            None,
                            NativeVfsEventKind::PermissionRefused,
                            Errno::Access as u32,
                        );
                    }
                    return Err(Errno::Access);
                }
                Err(error) => return Err(error),
            }
        }
    };
    if let Ok(target) = DESCRIPTORS
        .with(|descriptors| Ok::<DescriptorTarget, Errno>(descriptors.descriptor(fd)?.target))
    {
        let inode = match target {
            DescriptorTarget::BootDirectory(inode)
            | DescriptorTarget::BootFile(inode)
            | DescriptorTarget::BootChannel(inode) => Some(inode),
            _ => None,
        };
        if let Some(inode) = inode {
            emit_vfs_event(
                inode,
                (!path.is_empty()).then_some(path.as_str()),
                None,
                NativeVfsEventKind::Opened,
                fd as u32,
            );
        }
    }
    Ok(fd)
}

fn list_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let listing = match boot_vfs_list(&path) {
        Ok(listing) => listing,
        Err(error) => {
            if matches!(error, Errno::Access | Errno::Perm) {
                emit_permission_refusal_for_path(&path, error);
            }
            return Err(error);
        }
    };
    copy_text_to_user(&listing, out, capacity)
}

fn list_path_at_syscall(
    dirfd: usize,
    path_ptr: usize,
    path_len: usize,
    out: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    let (resolved, path) = resolved_at_target_from_user(dirfd, path_ptr, path_len)?;
    let listing = match resolved {
        ResolvedAtTarget::Handle(DescriptorTarget::BootDirectory(inode)) => {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::READ))?;
            let live_path = BOOT_VFS
                .with_mut(|vfs| vfs.live_path_for_inode(inode))
                .ok_or(Errno::Badf)?;
            match boot_vfs_list(&live_path) {
                Ok(listing) => listing,
                Err(error) => {
                    if matches!(error, Errno::Access | Errno::Perm) {
                        emit_permission_refusal_for_path(&live_path, error);
                    }
                    return Err(error);
                }
            }
        }
        ResolvedAtTarget::Handle(DescriptorTarget::Procfs(node))
            if matches!(
                node.kind,
                BootProcfsNodeKind::ProcRootDir
                    | BootProcfsNodeKind::ProcessDir
                    | BootProcfsNodeKind::FdDirListing
                    | BootProcfsNodeKind::FdInfoDirListing
            ) =>
        {
            DESCRIPTORS
                .with(|descriptors| descriptors.require_rights(dirfd, BlockRightsMask::READ))?;
            let live_path = descriptor_target_path_text(DescriptorTarget::Procfs(node))?;
            boot_procfs_directory_listing(&live_path)?.ok_or(Errno::NoEnt)?
        }
        ResolvedAtTarget::Handle(_) => return Err(Errno::NotDir),
        ResolvedAtTarget::Path => match boot_vfs_list(&path) {
            Ok(listing) => listing,
            Err(error) => {
                if matches!(error, Errno::Access | Errno::Perm) {
                    emit_permission_refusal_for_path(&path, error);
                }
                return Err(error);
            }
        },
    };
    copy_text_to_user(&listing, out, capacity)
}

fn list_processes_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    let requester_pid = active_process_pid()?;
    let requester_uid = BootVfs::current_subject().0;
    let requester_label = BootVfs::current_subject_label();
    BOOT_PROCESSES.with_mut(|registry| {
        let ids = registry
            .entries
            .iter()
            .filter(|entry| !entry.reaped)
            .filter(|entry| {
                process_visible_to_requester(
                    requester_pid,
                    requester_uid,
                    requester_label,
                    entry,
                    false,
                )
            })
            .map(|entry| entry.pid)
            .collect::<Vec<_>>();
        copy_ids_to_user(&ids, buffer, capacity)
    })
}

fn inspect_process_syscall(pid: usize, out: *mut NativeProcessRecord) -> Result<usize, Errno> {
    require_process_inspect_target(pid, false)?;
    let record = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        Ok(NativeProcessRecord {
            pid: entry.pid,
            parent: entry.parent,
            address_space: entry.pid,
            main_thread: entry.pid,
            state: entry.state,
            exit_code: entry.exit_code,
            descriptor_count: if entry.pid == 1 {
                DESCRIPTORS.with(|table| table.slots.iter().flatten().count() as u64)
            } else {
                entry.descriptors.len() as u64
            },
            capability_count: boot_process_capability_names(
                entry,
                if entry.pid == 1 {
                    DESCRIPTORS.with(|table| table.slots.iter().flatten().count())
                } else {
                    entry.descriptors.len()
                },
            )
            .len() as u64,
            environment_count: entry.env_count,
            memory_region_count: entry.vm_objects.len() as u64,
            thread_count: 1,
            pending_signal_count: entry.pending_signal_count,
            session_reported: 0,
            session_status: 0,
            session_stage: 0,
            scheduler_class: entry.scheduler_class,
            scheduler_budget: entry.scheduler_budget,
            cpu_runtime_ticks: entry.cpu_runtime_ticks,
            execution_contract: entry.contract_bindings.execution,
            memory_contract: entry.contract_bindings.memory,
            io_contract: entry.contract_bindings.io,
            observe_contract: entry.contract_bindings.observe,
            reserved: 0,
        })
    })?;
    write_record(out, record)?;
    Ok(0)
}

fn boot_process_compat_env_value<'a>(envp: &'a [String], key: &str) -> Option<&'a str> {
    envp.iter().rev().find_map(|entry| {
        entry
            .split_once('=')
            .and_then(|(candidate_key, value)| (candidate_key == key).then_some(value))
    })
}

fn fill_text_field<const N: usize>(dst: &mut [u8; N], value: &str) {
    let bytes = value.as_bytes();
    let count = bytes.len().min(N);
    dst[..count].copy_from_slice(&bytes[..count]);
}

fn inspect_process_compat_syscall(
    pid: usize,
    out: *mut NativeProcessCompatRecord,
) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: inspect_process_compat enter pid={} out={:p}\n",
        pid, out
    ));
    require_process_inspect_target(pid, false)?;
    let record = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        let target =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_TARGET").unwrap_or("native");
        let route_class = boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_ROUTE_CLASS")
            .unwrap_or("native-process-abi");
        let handle_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_HANDLE_PROFILE")
                .unwrap_or("native-handles");
        let path_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_PATH_PROFILE")
                .unwrap_or("native-paths");
        let scheduler_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_SCHEDULER_PROFILE")
                .unwrap_or("native-scheduler");
        let sync_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_SYNC_PROFILE")
                .unwrap_or("native-sync");
        let timer_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_TIMER_PROFILE")
                .unwrap_or("native-timer");
        let module_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_MODULE_PROFILE")
                .unwrap_or("native-module");
        let event_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_EVENT_PROFILE")
                .unwrap_or("native-event");
        let requires_kernel_abi_shims =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ABI_REQUIRES_SHIMS")
                .map(|value| value == "1")
                .unwrap_or(false);
        let prefix =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_PREFIX").unwrap_or("/");
        let loader_route_class =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ROUTE_CLASS")
                .unwrap_or("native-direct");
        let loader_launch_mode =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_LAUNCH_MODE")
                .unwrap_or("native-direct");
        let loader_entry_profile =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_ENTRY_PROFILE")
                .unwrap_or("native-entry");
        let loader_requires_compat_shims =
            boot_process_compat_env_value(&entry.envp, "NGOS_COMPAT_REQUIRES_SHIMS")
                .map(|value| value == "1")
                .unwrap_or(false);
        Ok(NativeProcessCompatRecord {
            pid: entry.pid,
            target: {
                let mut field = [0; 16];
                fill_text_field(&mut field, target);
                field
            },
            route_class: {
                let mut field = [0; 32];
                fill_text_field(&mut field, route_class);
                field
            },
            handle_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, handle_profile);
                field
            },
            path_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, path_profile);
                field
            },
            scheduler_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, scheduler_profile);
                field
            },
            sync_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, sync_profile);
                field
            },
            timer_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, timer_profile);
                field
            },
            module_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, module_profile);
                field
            },
            event_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, event_profile);
                field
            },
            requires_kernel_abi_shims: u32::from(requires_kernel_abi_shims),
            prefix: {
                let mut field = [0; 64];
                fill_text_field(&mut field, prefix);
                field
            },
            executable_path: {
                let mut field = [0; 64];
                fill_text_field(&mut field, &entry.image_path);
                field
            },
            working_dir: {
                let mut field = [0; 64];
                fill_text_field(&mut field, &entry.cwd);
                field
            },
            loader_route_class: {
                let mut field = [0; 32];
                fill_text_field(&mut field, loader_route_class);
                field
            },
            loader_launch_mode: {
                let mut field = [0; 32];
                fill_text_field(&mut field, loader_launch_mode);
                field
            },
            loader_entry_profile: {
                let mut field = [0; 32];
                fill_text_field(&mut field, loader_entry_profile);
                field
            },
            loader_requires_compat_shims: u32::from(loader_requires_compat_shims),
        })
    })?;
    write_record(out, record)?;
    serial::print(format_args!(
        "ngos/x86_64: inspect_process_compat handled pid={}\n",
        pid
    ));
    Ok(0)
}

fn get_process_name_syscall(pid: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    require_process_inspect_target(pid, false)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        copy_name_to_user(
            &InlineName::from_text(&registry.entries[index].name)?,
            buffer,
            capacity,
        )
    })
}

fn get_process_image_path_syscall(
    pid: usize,
    buffer: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    require_process_inspect_target(pid, false)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        copy_text_to_user(&registry.entries[index].image_path, buffer, capacity)
    })
}

fn get_process_cwd_syscall(pid: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    require_process_inspect_target(pid, false)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        copy_text_to_user(&registry.entries[index].cwd, buffer, capacity)
    })
}

fn get_process_root_syscall(pid: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    require_process_inspect_target(pid, false)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        copy_text_to_user(&registry.entries[index].root, buffer, capacity)
    })
}

fn copy_signal_list_to_user(
    signal: u8,
    count: usize,
    buffer: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    if capacity == 0 {
        return Ok(count);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let copy_len = count.min(capacity);
    unsafe {
        for index in 0..copy_len {
            ptr::write(buffer.add(index), signal);
        }
    }
    Ok(count)
}

fn pending_signals_syscall(pid: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    require_process_inspect_target(pid, false)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let count = registry.entries[index].pending_signal_count as usize;
        copy_signal_list_to_user(9, count, buffer, capacity)
    })
}

fn blocked_pending_signals_syscall(
    pid: usize,
    buffer: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    require_process_inspect_target(pid, false)?;
    if capacity == 0 {
        Ok(0)
    } else if buffer.is_null() {
        Err(Errno::Fault)
    } else {
        Ok(0)
    }
}

fn pause_process_syscall(pid: usize) -> Result<usize, Errno> {
    if pid == 1 {
        return Err(Errno::Perm);
    }
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if entry.state == 4 {
            return Err(Errno::Srch);
        }
        entry.state = 3;
        Ok(0)
    })
}

fn resume_process_syscall(pid: usize) -> Result<usize, Errno> {
    if pid == 1 {
        return Err(Errno::Perm);
    }
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if entry.state == 4 {
            return Err(Errno::Srch);
        }
        entry.state = 1;
        registry.rebalance_queued_processes();
        Ok(0)
    })
}

fn renice_process_syscall(pid: usize, class_raw: usize, budget: usize) -> Result<usize, Errno> {
    if pid == 1 || budget == 0 {
        return Err(Errno::Inval);
    }
    let Some(class) = NativeSchedulerClass::from_raw(class_raw as u32) else {
        return Err(Errno::Inval);
    };
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if entry.state == 4 {
            return Err(Errno::Srch);
        }
        entry.scheduler_class = class as u32;
        entry.scheduler_budget = budget as u32;
        Ok(0)
    })
}

fn set_process_affinity_syscall(pid: usize, affinity_mask: usize) -> Result<usize, Errno> {
    if pid == 1 {
        return Err(Errno::Perm);
    }
    let sanitized = (affinity_mask as u64) & boot_scheduler_online_mask();
    if sanitized == 0 {
        return Err(Errno::Inval);
    }
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry_pid = registry.entries[index].pid;
        let entry_state = registry.entries[index].state;
        if entry_state == 4 {
            return Err(Errno::Srch);
        }
        let previous_cpu = registry.scheduler_state(entry_pid).assigned_cpu;
        let next_cpu = boot_scheduler_pick_cpu(sanitized, previous_cpu);
        let state = registry.scheduler_state_mut_or_insert(entry_pid);
        state.affinity_mask = sanitized;
        state.assigned_cpu = next_cpu;
        registry.scheduler_events.rebalance_operations = registry
            .scheduler_events
            .rebalance_operations
            .saturating_add(1);
        let migrated = u64::from(previous_cpu != next_cpu);
        if migrated != 0 {
            registry.scheduler_events.last_rebalance_migrations = migrated;
            registry.scheduler_events.rebalance_migrations = registry
                .scheduler_events
                .rebalance_migrations
                .saturating_add(migrated);
            registry.scheduler_events.last_rebalance_pid = entry_pid;
            registry.scheduler_events.last_rebalance_from_cpu = previous_cpu;
            registry.scheduler_events.last_rebalance_to_cpu = next_cpu;
        }
        registry.scheduler_events.last_affinity_pid = entry_pid;
        registry.scheduler_events.last_affinity_mask = sanitized;
        registry.scheduler_events.last_affinity_cpu = next_cpu;
        Ok(0)
    })
}

fn inspect_process_identity_syscall(
    pid: usize,
    out: *mut NativeProcessIdentityRecord,
) -> Result<usize, Errno> {
    require_process_inspect_target(pid, true)?;
    let record = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        Ok(NativeProcessIdentityRecord {
            uid: entry.uid,
            gid: entry.gid,
            umask: entry.umask & 0o777,
            supplemental_count: entry.supplemental_count as u32,
            supplemental_gids: entry.supplemental_gids,
        })
    })?;
    write_record(out, record)?;
    Ok(0)
}

fn inspect_process_security_label_syscall(
    pid: usize,
    out: *mut SecurityLabel,
) -> Result<usize, Errno> {
    require_process_inspect_target(pid, true)?;
    let label = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        Ok::<SecurityLabel, Errno>(registry.entries[index].subject_label)
    })?;
    write_record(out, label)?;
    Ok(0)
}

fn active_process_root_and_cwd() -> Result<(String, String), Errno> {
    let pid = active_process_pid()?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        Ok((
            registry.entries[index].root.clone(),
            registry.entries[index].cwd.clone(),
        ))
    })
}

fn active_process_pid() -> Result<u64, Errno> {
    let pid = ACTIVE_PROCESS_PID.load(Ordering::Acquire);
    BOOT_PROCESSES.with_mut(|registry| {
        if registry.find_index(pid).is_some() {
            Ok(pid)
        } else {
            Err(Errno::Srch)
        }
    })
}

fn classify_boot_process_execution_mode(path: &str, argv: &[String]) -> BootProcessExecutionMode {
    if boot_same_image_path(path)
        && argv
            .first()
            .map(|value| boot_same_image_path(value))
            .unwrap_or(false)
    {
        BootProcessExecutionMode::SameImageBlocking
    } else {
        BootProcessExecutionMode::MetadataOnly
    }
}

fn boot_same_image_path(path: &str) -> bool {
    let normalized = path.rsplit('/').next().unwrap_or(path);
    normalized == "ngos-userland-native" || normalized == "userland-native"
}

fn require_process_control_target(pid: usize) -> Result<(), Errno> {
    let (uid, _) = BootVfs::current_subject();
    if uid == 0 || active_process_pid()? == pid as u64 {
        Ok(())
    } else {
        Err(Errno::Perm)
    }
}

fn label_does_not_raise_subject_privilege(current: SecurityLabel, next: SecurityLabel) -> bool {
    (next.confidentiality as u8) <= (current.confidentiality as u8)
        && (next.integrity as u8) <= (current.integrity as u8)
}

fn label_tightens_object_policy(current: SecurityLabel, next: SecurityLabel) -> bool {
    (next.confidentiality as u8) >= (current.confidentiality as u8)
        && (next.integrity as u8) >= (current.integrity as u8)
}

fn group_is_current_or_supplemental(gid: u32, current_gid: u32, supplemental: &[u32]) -> bool {
    gid == current_gid || supplemental.iter().any(|candidate| *candidate == gid)
}

fn supplemental_subset(candidate: &[u32], current: &[u32]) -> bool {
    candidate
        .iter()
        .all(|gid| current.iter().any(|current_gid| current_gid == gid))
}

fn send_signal_syscall(pid: usize, signal: u8) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        entry.pending_signal_count = entry.pending_signal_count.saturating_add(1);
        if signal != 0 {
            entry.state = 4;
            entry.exit_code = 128 + signal as i32;
            entry.pending_signal_count = 0;
        }
        Ok(0)
    })
}

fn spawn_path_process_syscall(
    name_ptr: usize,
    name_len: usize,
    path_ptr: usize,
    path_len: usize,
) -> Result<usize, Errno> {
    let name = string_from_user(name_ptr, name_len)?;
    let path = resolve_active_process_path(&string_from_user(path_ptr, path_len)?)?;
    let trace_worker_spawn = path == "/bin/worker";
    if trace_worker_spawn {
        serial::write_bytes(
            format!("ngos/x86_64: spawn-path enter name={name} path={path}\n").as_bytes(),
        );
    }
    let (root, cwd) = active_process_root_and_cwd()?;
    let descriptors = DESCRIPTORS.with(|table| table.snapshot_for_spawn())?;
    let pid = BOOT_PROCESSES.with_mut(|registry| {
        let pid = registry.spawn(
            name,
            path.clone(),
            cwd,
            descriptors,
            vec![path.clone()],
            Vec::new(),
        )?;
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].root = root;
        Ok(pid)
    })?;
    if trace_worker_spawn {
        serial::write_bytes(
            format!("ngos/x86_64: spawn-path handled pid={pid} path=/bin/worker\n").as_bytes(),
        );
    }
    Ok(pid as usize)
}

fn spawn_process_copy_vm_syscall(
    name_ptr: usize,
    name_len: usize,
    path_ptr: usize,
    path_len: usize,
    source_pid: usize,
) -> Result<usize, Errno> {
    let name = string_from_user(name_ptr, name_len)?;
    let path = resolve_active_process_path(&string_from_user(path_ptr, path_len)?)?;
    let (root, cwd) = active_process_root_and_cwd()?;
    let descriptors = DESCRIPTORS.with(|table| table.snapshot_for_spawn())?;
    let pid = BOOT_PROCESSES.with_mut(|registry| {
        let pid = registry.spawn(
            name,
            path.clone(),
            cwd,
            descriptors,
            vec![path.clone()],
            Vec::new(),
        )?;
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].root = root;
        Ok(pid)
    })?;
    boot_copy_vm_state(source_pid as u64, pid)?;
    Ok(pid as usize)
}

fn spawn_configured_process_syscall(config_ptr: usize) -> Result<usize, Errno> {
    let config = copy_struct_from_user::<NativeSpawnProcessConfig>(config_ptr)?;
    let name = string_from_user(config.name_ptr, config.name_len)?;
    let path = resolve_active_process_path(&string_from_user(config.path_ptr, config.path_len)?)?;
    let cwd = resolve_active_process_path(&string_from_user(config.cwd_ptr, config.cwd_len)?)?;
    let (root, _) = active_process_root_and_cwd()?;
    let argv = string_table_from_user(config.argv_ptr, config.argv_len, config.argv_count)?;
    let envp = string_table_from_user(config.envp_ptr, config.envp_len, config.envp_count)?;
    let execution_mode = classify_boot_process_execution_mode(&path, &argv);
    let descriptors = DESCRIPTORS.with(|table| table.snapshot_for_spawn())?;
    let pid = BOOT_PROCESSES.with_mut(|registry| {
        let pid = registry.spawn(name, path, cwd, descriptors, argv, envp)?;
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].root = root;
        registry.entries[index].execution_mode = execution_mode;
        Ok(pid)
    })?;
    Ok(pid as usize)
}

fn set_process_args_syscall(
    pid: usize,
    argv_ptr: usize,
    argv_len: usize,
    argv_count: usize,
) -> Result<usize, Errno> {
    require_process_control_target(pid)?;
    let argv = string_table_from_user(argv_ptr, argv_len, argv_count)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let argv_count = argv.len() as u64;
        registry.entries[index].argv = argv;
        registry.entries[index].argv_count = argv_count;
        Ok(0)
    })
}

fn set_process_env_syscall(
    pid: usize,
    env_ptr: usize,
    env_len: usize,
    env_count: usize,
) -> Result<usize, Errno> {
    require_process_control_target(pid)?;
    let envp = string_table_from_user(env_ptr, env_len, env_count)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].env_count = envp.len() as u64;
        registry.entries[index].envp = envp;
        Ok(0)
    })
}

fn set_process_cwd_syscall(pid: usize, cwd_ptr: usize, cwd_len: usize) -> Result<usize, Errno> {
    require_process_control_target(pid)?;
    let requested = string_from_user(cwd_ptr, cwd_len)?;
    let (root, current_cwd) = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        Ok((
            registry.entries[index].root.clone(),
            registry.entries[index].cwd.clone(),
        ))
    })?;
    let cwd = BootVfs::resolve_path_from_root(&root, &current_cwd, &requested)?;
    BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&cwd, true)?;
        if vfs.nodes[index].kind != BootNodeKind::Directory {
            return Err(Errno::NotDir);
        }
        vfs.require_traversal_access(&cwd, true)
    })?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].cwd = cwd;
        Ok(0)
    })
}

fn set_process_root_syscall(pid: usize, root_ptr: usize, root_len: usize) -> Result<usize, Errno> {
    require_process_control_target(pid)?;
    let requested = string_from_user(root_ptr, root_len)?;
    let current_root = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        Ok(registry.entries[index].root.clone())
    })?;
    let root = BootVfs::resolve_path_from_root(&current_root, &current_root, &requested)?;
    let cwd_after = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        Ok::<String, Errno>(
            BootVfs::resolve_path_from_root(&root, &root, &registry.entries[index].cwd)
                .unwrap_or_else(|_| root.clone()),
        )
    })?;
    BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&root, true)?;
        if vfs.nodes[index].kind != BootNodeKind::Directory {
            return Err(Errno::NotDir);
        }
        vfs.require_traversal_access(&root, true)
    })?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].root = root;
        registry.entries[index].cwd = cwd_after;
        Ok(0)
    })
}

fn set_process_identity_syscall(
    pid: usize,
    identity_ptr: *const NativeProcessIdentityRecord,
) -> Result<usize, Errno> {
    if identity_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let identity = unsafe { ptr::read(identity_ptr) };
    if identity.supplemental_count as usize > identity.supplemental_gids.len() {
        return Err(Errno::Inval);
    }
    require_process_control_target(pid)?;
    let (current_uid, current_gid) = BootVfs::current_subject();
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if current_uid != 0 {
            if identity.uid != current_uid {
                return Err(Errno::Perm);
            }
            let current_groups = &entry.supplemental_gids[..entry.supplemental_count];
            let requested_groups =
                &identity.supplemental_gids[..identity.supplemental_count as usize];
            if !group_is_current_or_supplemental(identity.gid, current_gid, current_groups) {
                return Err(Errno::Perm);
            }
            if !supplemental_subset(requested_groups, current_groups) {
                return Err(Errno::Perm);
            }
        }
        entry.uid = identity.uid;
        entry.gid = identity.gid;
        entry.umask = identity.umask & 0o777;
        entry.supplemental_count = identity.supplemental_count as usize;
        entry.supplemental_gids = identity.supplemental_gids;
        for index in entry.supplemental_count..entry.supplemental_gids.len() {
            entry.supplemental_gids[index] = 0;
        }
        Ok(0)
    })
}

fn set_process_security_label_syscall(
    pid: usize,
    label_ptr: *const SecurityLabel,
) -> Result<usize, Errno> {
    if label_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let label = unsafe { ptr::read(label_ptr) };
    require_process_control_target(pid)?;
    let (uid, _) = BootVfs::current_subject();
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        if uid != 0
            && !label_does_not_raise_subject_privilege(registry.entries[index].subject_label, label)
        {
            return Err(Errno::Perm);
        }
        registry.entries[index].subject_label = label;
        Ok(0)
    })
}

fn inspect_path_security_context_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut ObjectSecurityContext,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let record = BOOT_VFS.with_mut(|vfs| vfs.security_context(&path))?;
    write_record(out, record)?;
    Ok(0)
}

fn set_path_security_label_syscall(
    path_ptr: usize,
    path_len: usize,
    label_ptr: *const SecurityLabel,
) -> Result<usize, Errno> {
    if label_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let label = unsafe { ptr::read(label_ptr) };
    BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, false)?;
        let node = vfs.nodes.get(index).ok_or(Errno::NoEnt)?;
        let (uid, _) = BootVfs::current_subject();
        if uid != 0 {
            if uid != node.owner_uid {
                return Err(Errno::Perm);
            }
            if !label_tightens_object_policy(node.current_label, label) {
                return Err(Errno::Perm);
            }
        }
        vfs.set_security_label(&path, label)
    })?;
    Ok(0)
}

fn reap_process_syscall(pid: usize) -> Result<usize, Errno> {
    #[cfg(target_os = "none")]
    {
        let launch_pid = BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid as u64) else {
                return Err(Errno::Srch);
            };
            let entry = &registry.entries[index];
            if entry.state == 4 {
                return Ok(None);
            }
            if entry.execution_mode == BootProcessExecutionMode::SameImageBlocking
                && entry.state == 1
            {
                return Ok(Some(entry.pid));
            }
            Err(Errno::Again)
        })?;
        if let Some(launch_pid) = launch_pid {
            request_blocking_reap_launch(launch_pid);
            return Ok(0);
        }
    }
    let (exit_code, summary) = BOOT_PROCESSES.with_mut(|registry| registry.reap(pid as u64))?;
    BOOT_VFS.with_mut(|vfs| {
        vfs.stats.process_reaps = vfs.stats.process_reaps.saturating_add(1);
        vfs.stats.reaped_descriptor_records = vfs
            .stats
            .reaped_descriptor_records
            .saturating_add(summary.descriptors);
        vfs.stats.reaped_env_records = vfs
            .stats
            .reaped_env_records
            .saturating_add(summary.env_records);
        vfs.stats.reaped_vm_objects = vfs
            .stats
            .reaped_vm_objects
            .saturating_add(summary.vm_objects);
        vfs.stats.reaped_vm_decisions = vfs
            .stats
            .reaped_vm_decisions
            .saturating_add(summary.vm_decisions);
    });
    Ok(exit_code as usize)
}

fn read_procfs_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    let node = boot_procfs_node(&path)?.ok_or(Errno::NoEnt)?;
    let payload = boot_procfs_payload(node.pid, node.kind)?;
    copy_text_to_user(&payload, out, capacity)
}

fn load_memory_word_syscall(pid: usize, addr: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, addr, 4, 9)?;
    let result = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let share_key = {
            let entry = &registry.entries[index];
            let Some(object_index) = entry.vm_objects.iter().position(|object| {
                !object.quarantined
                    && object.readable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            }) else {
                let entry = &mut registry.entries[index];
                if let Some(object) = entry.vm_objects.iter().find(|object| {
                    object.quarantined
                        && (addr as u64) >= object.start
                        && (addr as u64) < object.start.saturating_add(object.len)
                }) {
                    entry.vm_decisions.push(BootVmDecision {
                        agent: "quarantine-block",
                        vm_object_id: object.id,
                        start: addr as u64,
                        len: 8,
                        detail0: object.quarantine_reason,
                        detail1: 0,
                    });
                    return Err(Errno::Fault);
                }
                return Err(Errno::Fault);
            };
            entry.vm_objects[object_index].share_key
        };
        let owners = boot_vm_owner_count(registry, share_key);
        let entry = &mut registry.entries[index];
        let object_index = entry
            .vm_objects
            .iter()
            .position(|object| {
                !object.quarantined
                    && object.readable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            })
            .ok_or(Errno::Fault)?;
        boot_vm_touch_object_page(entry, object_index, addr as u64, false, owners)?;
        let object = &entry.vm_objects[object_index];
        let offset = (addr as u64).saturating_sub(object.start) as usize;
        let mut word = [0u8; 4];
        let available = object.bytes.len().saturating_sub(offset).min(4);
        if available != 0 {
            word[..available].copy_from_slice(&object.bytes[offset..offset + available]);
        }
        Ok(u32::from_le_bytes(word) as usize)
    });
    result
}

fn store_memory_word_syscall(pid: usize, addr: usize, value: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, addr, 4, 11)?;
    let result = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let share_key = {
            let entry = &registry.entries[index];
            let Some(object_index) = entry.vm_objects.iter().position(|object| {
                !object.quarantined
                    && object.writable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            }) else {
                let entry = &mut registry.entries[index];
                if let Some(object) = entry.vm_objects.iter().find(|object| {
                    object.quarantined
                        && (addr as u64) >= object.start
                        && (addr as u64) < object.start.saturating_add(object.len)
                }) {
                    entry.vm_decisions.push(BootVmDecision {
                        agent: "quarantine-block",
                        vm_object_id: object.id,
                        start: addr as u64,
                        len: 8,
                        detail0: object.quarantine_reason,
                        detail1: 1,
                    });
                    return Err(Errno::Fault);
                }
                return Err(Errno::Fault);
            };
            entry.vm_objects[object_index].share_key
        };
        let owners = boot_vm_owner_count(registry, share_key);
        let entry = &mut registry.entries[index];
        let object_index = entry
            .vm_objects
            .iter()
            .position(|object| {
                !object.quarantined
                    && object.writable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            })
            .ok_or(Errno::Fault)?;
        boot_vm_touch_object_page(entry, object_index, addr as u64, true, owners)?;
        let object_index = entry
            .vm_objects
            .iter()
            .position(|object| {
                !object.quarantined
                    && object.writable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            })
            .ok_or(Errno::Fault)?;
        let object = &mut entry.vm_objects[object_index];
        let offset = (addr as u64).saturating_sub(object.start) as usize;
        let bytes = (value as u32).to_le_bytes();
        let available = object.bytes.len().saturating_sub(offset).min(4);
        if available != 0 {
            object.bytes[offset..offset + available].copy_from_slice(&bytes[..available]);
        }
        Ok(0)
    });
    result
}

fn quarantine_vm_object_syscall(
    pid: usize,
    vm_object_id: usize,
    reason: usize,
) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 7)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let object = entry
            .vm_objects
            .iter_mut()
            .find(|object| object.id == vm_object_id as u64)
            .ok_or(Errno::NoEnt)?;
        object.quarantined = true;
        object.quarantine_reason = reason as u64;
        entry.vm_decisions.push(BootVmDecision {
            agent: "quarantine-state",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: reason as u64,
            detail1: 1,
        });
        Ok(0)
    })
}

fn release_vm_object_syscall(pid: usize, vm_object_id: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 8)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let object = entry
            .vm_objects
            .iter_mut()
            .find(|object| object.id == vm_object_id as u64)
            .ok_or(Errno::NoEnt)?;
        let reason = object.quarantine_reason;
        object.quarantined = false;
        object.quarantine_reason = 0;
        entry.vm_decisions.push(BootVmDecision {
            agent: "quarantine-state",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: reason,
            detail1: 0,
        });
        Ok(0)
    })
}

fn map_anonymous_memory_syscall(
    pid: usize,
    length: usize,
    label_ptr: usize,
    label_len: usize,
) -> Result<usize, Errno> {
    let label = string_from_user(label_ptr, label_len)?;
    BootVmPolicyEnforcementAgent::enforce(pid, 0, length, 0)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = entry.next_vm_addr;
        let len = (length as u64).max(0x1000).next_multiple_of(0x1000);
        let object_id = entry.next_vm_object_id;
        entry.next_vm_addr = entry
            .next_vm_addr
            .saturating_add(len)
            .saturating_add(0x1000);
        entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
        entry.vm_objects.push(BootVmObject {
            id: object_id,
            start,
            len,
            name: format!("[anon:{}]", label),
            kind: "Anonymous",
            backing_inode: None,
            share_key: object_id,
            shadow_source_id: None,
            shadow_source_offset: 0,
            shadow_depth: 0,
            private_mapping: true,
            file_offset: 0,
            bytes: vec![0; len as usize],
            readable: true,
            writable: true,
            executable: false,
            read_fault_count: 0,
            write_fault_count: 0,
            cow_fault_count: 0,
            committed_pages: len / 0x1000,
            resident_pages: 0,
            dirty_pages: 0,
            accessed_pages: 0,
            quarantined: false,
            quarantine_reason: 0,
            page_states: vec![BootVmPageState::default(); boot_vm_page_count_for_len(len)],
        });
        entry.vm_decisions.push(BootVmDecision {
            agent: "map",
            vm_object_id: object_id,
            start,
            len,
            detail0: 1,
            detail1: entry.vm_objects.len() as u64,
        });
        Ok(start as usize)
    })
}

fn map_file_backed_memory_boot(
    pid: usize,
    path_ptr: usize,
    path_len: usize,
    length: usize,
    offset: usize,
    readable: usize,
    writable: usize,
    executable: usize,
    private_mapping: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let len = (length as u64).max(0x1000).next_multiple_of(0x1000);
    let offset = offset as u64;
    let (backing_inode, file_size, mapped_bytes) = BOOT_VFS.with_mut(|vfs| {
        let normalized = BootVfs::normalize_path(path)?;
        let index = vfs.resolve_node_index(&normalized, true)?;
        let node = &vfs.nodes[index];
        if !matches!(node.kind, BootNodeKind::File | BootNodeKind::Channel) {
            return Err(Errno::Inval);
        }
        let bytes = vfs
            .object_bytes_range_by_inode(node.inode, offset as usize, len as usize)
            .ok_or(Errno::Badf)?;
        Ok((node.inode, vfs.file_size(&normalized)? as u64, bytes))
    })?;
    if offset >= file_size {
        return Err(Errno::Inval);
    }
    BootVmPolicyEnforcementAgent::enforce(pid, 0, length, 1)?;

    let result = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = entry.next_vm_addr;
        let object_id = entry.next_vm_object_id;
        entry.next_vm_addr = entry
            .next_vm_addr
            .saturating_add(len)
            .saturating_add(0x1000);
        entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
        entry.vm_objects.push(BootVmObject {
            id: object_id,
            start,
            len,
            name: String::from(path),
            kind: "File",
            backing_inode: Some(backing_inode),
            share_key: object_id,
            shadow_source_id: None,
            shadow_source_offset: 0,
            shadow_depth: 0,
            private_mapping: private_mapping != 0,
            file_offset: offset,
            bytes: mapped_bytes,
            readable: readable != 0,
            writable: writable != 0,
            executable: executable != 0,
            read_fault_count: 0,
            write_fault_count: 0,
            cow_fault_count: 0,
            committed_pages: len / 0x1000,
            resident_pages: 0,
            dirty_pages: 0,
            accessed_pages: 0,
            quarantined: false,
            quarantine_reason: 0,
            page_states: vec![BootVmPageState::default(); boot_vm_page_count_for_len(len)],
        });
        entry.vm_decisions.push(BootVmDecision {
            agent: "map-file",
            vm_object_id: object_id,
            start,
            len,
            detail0: offset,
            detail1: ((readable != 0) as u64)
                | (((writable != 0) as u64) << 1)
                | (((executable != 0) as u64) << 2)
                | (((private_mapping != 0) as u64) << 3),
        });
        Ok(start as usize)
    });
    result
}

fn split_vm_object_at(
    entry: &mut BootProcessEntry,
    object_index: usize,
    split_at: u64,
) -> Option<usize> {
    if object_index >= entry.vm_objects.len() {
        return None;
    }
    let object = entry.vm_objects[object_index].clone();
    if split_at <= object.start || split_at >= object.start.saturating_add(object.len) {
        return None;
    }

    let left_len = split_at.saturating_sub(object.start);
    let right_len = object.len.saturating_sub(left_len);
    let left_pages = left_len / 0x1000;
    let right_pages = right_len / 0x1000;
    let left_page_count = left_pages as usize;

    entry.vm_objects[object_index].len = left_len;
    entry.vm_objects[object_index].page_states = object.page_states[..left_page_count].to_vec();
    entry.vm_objects[object_index].bytes = object.bytes[..left_len as usize].to_vec();
    boot_vm_recount_object_pages(&mut entry.vm_objects[object_index]);

    let right = BootVmObject {
        id: entry.next_vm_object_id,
        start: split_at,
        len: right_len,
        name: object.name,
        kind: object.kind,
        backing_inode: object.backing_inode,
        share_key: object.share_key,
        shadow_source_id: object.shadow_source_id,
        shadow_source_offset: object.shadow_source_offset.saturating_add(left_len),
        shadow_depth: object.shadow_depth,
        private_mapping: object.private_mapping,
        file_offset: if object.kind == "File" {
            object.file_offset.saturating_add(left_len)
        } else {
            object.file_offset
        },
        bytes: object.bytes[left_len as usize..].to_vec(),
        readable: object.readable,
        writable: object.writable,
        executable: object.executable,
        read_fault_count: object.read_fault_count,
        write_fault_count: object.write_fault_count,
        cow_fault_count: object.cow_fault_count,
        committed_pages: right_pages,
        resident_pages: 0,
        dirty_pages: 0,
        accessed_pages: 0,
        quarantined: object.quarantined,
        quarantine_reason: object.quarantine_reason,
        page_states: object.page_states[left_page_count..].to_vec(),
    };
    let left_committed = entry.vm_objects[object_index].committed_pages.max(1);
    let right_committed = right.committed_pages.max(1);
    let total_committed = left_committed.saturating_add(right_committed).max(1);
    entry.vm_objects[object_index].read_fault_count =
        object.read_fault_count.saturating_mul(left_committed) / total_committed;
    entry.vm_objects[object_index].write_fault_count =
        object.write_fault_count.saturating_mul(left_committed) / total_committed;
    entry.vm_objects[object_index].cow_fault_count =
        object.cow_fault_count.saturating_mul(left_committed) / total_committed;
    let mut right = right;
    right.read_fault_count = object
        .read_fault_count
        .saturating_sub(entry.vm_objects[object_index].read_fault_count);
    right.write_fault_count = object
        .write_fault_count
        .saturating_sub(entry.vm_objects[object_index].write_fault_count);
    right.cow_fault_count = object
        .cow_fault_count
        .saturating_sub(entry.vm_objects[object_index].cow_fault_count);
    boot_vm_recount_object_pages(&mut right);
    entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
    entry.vm_objects.insert(object_index + 1, right);
    Some(object_index + 1)
}

fn vm_object_index_for_range(entry: &BootProcessEntry, start: u64, len: u64) -> Option<usize> {
    if len == 0 {
        return None;
    }
    entry.vm_objects.iter().position(|object| {
        let object_end = object.start.saturating_add(object.len);
        let range_end = start.saturating_add(len);
        start < object_end && range_end > object.start
    })
}

fn protect_memory_range_syscall(
    pid: usize,
    start: usize,
    len: usize,
    readable: usize,
    writable: usize,
    executable: usize,
) -> Result<usize, Errno> {
    if len == 0 {
        return Err(Errno::Inval);
    }
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 3)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = start as u64;
        let len = (len as u64).next_multiple_of(0x1000);
        let end = start.saturating_add(len);
        let Some(mut object_index) = vm_object_index_for_range(entry, start, len) else {
            return Err(Errno::Fault);
        };

        if start > entry.vm_objects[object_index].start {
            object_index = split_vm_object_at(entry, object_index, start).unwrap_or(object_index);
        }

        let mut cursor = object_index;
        let mut touched = 0usize;
        while cursor < entry.vm_objects.len() {
            let object_start = entry.vm_objects[cursor].start;
            if object_start >= end {
                break;
            }
            let object_end = object_start.saturating_add(entry.vm_objects[cursor].len);
            if end < object_end {
                split_vm_object_at(entry, cursor, end);
            }
            let object = &mut entry.vm_objects[cursor];
            object.readable = readable != 0;
            object.writable = writable != 0;
            object.executable = executable != 0;
            let object_id = object.id;
            let object_start = object.start;
            let object_len = object.len;
            entry.vm_decisions.push(BootVmDecision {
                agent: "protect",
                vm_object_id: object_id,
                start: object_start,
                len: object_len,
                detail0: ((readable != 0) as u64)
                    | (((writable != 0) as u64) << 1)
                    | (((executable != 0) as u64) << 2),
                detail1: 0,
            });
            touched += 1;
            cursor += 1;
        }

        if touched == 0 {
            Err(Errno::Fault)
        } else {
            Ok(0)
        }
    })
}

fn unmap_memory_range_syscall(pid: usize, start: usize, len: usize) -> Result<usize, Errno> {
    if len == 0 {
        return Err(Errno::Inval);
    }
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 2)?;
    let result = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = start as u64;
        let len = (len as u64).next_multiple_of(0x1000);
        let end = start.saturating_add(len);
        let Some(mut object_index) = vm_object_index_for_range(entry, start, len) else {
            return Err(Errno::Fault);
        };

        if start > entry.vm_objects[object_index].start {
            object_index = split_vm_object_at(entry, object_index, start).unwrap_or(object_index);
        }

        let mut removed = 0usize;
        while object_index < entry.vm_objects.len() {
            let object_start = entry.vm_objects[object_index].start;
            if object_start >= end {
                break;
            }
            let object_end = object_start.saturating_add(entry.vm_objects[object_index].len);
            if end < object_end {
                split_vm_object_at(entry, object_index, end);
            }
            let object = entry.vm_objects.remove(object_index);
            entry.vm_decisions.push(BootVmDecision {
                agent: "unmap",
                vm_object_id: object.id,
                start: object.start,
                len: object.len,
                detail0: object.committed_pages,
                detail1: 0,
            });
            removed += 1;
        }

        if removed == 0 {
            Err(Errno::Fault)
        } else {
            Ok(0)
        }
    });
    result
}

fn set_process_break_vm_syscall(pid: usize, new_end: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 13)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let object = entry
            .vm_objects
            .iter_mut()
            .find(|object| object.kind == "Heap")
            .ok_or(Errno::NoEnt)?;
        let old_end = object.start.saturating_add(object.len);
        let requested = new_end as u64;
        if requested <= object.start {
            return Err(Errno::Inval);
        }
        object.len = requested
            .saturating_sub(object.start)
            .next_multiple_of(0x1000);
        object.page_states.resize(
            boot_vm_page_count_for_len(object.len),
            BootVmPageState::default(),
        );
        boot_vm_recount_object_pages(object);
        entry.vm_decisions.push(BootVmDecision {
            agent: "brk",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: old_end,
            detail1: object.start.saturating_add(object.len),
        });
        Ok((object.start.saturating_add(object.len)) as usize)
    })
}

fn sync_memory_range_syscall(pid: usize, start: usize, len: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 5)?;
    let sync_request = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if let Some(object) = entry.vm_objects.iter_mut().find(|object| {
            (start as u64) >= object.start
                && (start as u64) < object.start.saturating_add(object.len)
        }) {
            let sync_start = (start as u64).saturating_sub(object.start) as usize;
            let sync_len = len.min(object.bytes.len().saturating_sub(sync_start));
            let sync_bytes = if sync_len == 0 {
                Vec::new()
            } else {
                object.bytes[sync_start..sync_start + sync_len].to_vec()
            };
            if let Some(inode) = object.backing_inode {
                let file_offset = object.file_offset as usize + sync_start;
                let file_end = file_offset.saturating_add(sync_len);
                Ok((
                    Some((inode, file_offset, file_end, sync_bytes)),
                    object.id,
                    object.committed_pages,
                ))
            } else {
                Ok((None, object.id, object.committed_pages))
            }
        } else {
            Err(Errno::Fault)
        }
    });
    let result = match sync_request {
        Ok((writeback, object_id, committed_pages)) => {
            if let Some((inode, file_offset, _file_end, sync_bytes)) = writeback
                && !sync_bytes.is_empty()
            {
                let backing = BOOT_VFS.with_mut(|vfs| vfs.clone_object_by_inode(inode));
                if let Some(backing) = backing {
                    let access_node = BootNode {
                        path: String::new(),
                        kind: backing.kind,
                        inode: backing.inode,
                        bytes: backing.bytes,
                        link_target: backing.link_target,
                        owner_uid: backing.owner_uid,
                        group_gid: backing.group_gid,
                        mode: backing.mode,
                        minimum_label: backing.minimum_label,
                        current_label: backing.current_label,
                        mount_layer: 0,
                        mount_id: None,
                    };
                    BootVfs::require_access_for_node(&access_node, false, true, false)?;
                }
                BOOT_VFS.with_mut(|vfs| {
                    for entry in vfs.nodes.iter_mut().filter(|entry| entry.inode == inode) {
                        if file_offset >= entry.bytes.len() {
                            continue;
                        }
                        let copy_len = sync_bytes.len().min(entry.bytes.len() - file_offset);
                        entry.bytes[file_offset..file_offset + copy_len]
                            .copy_from_slice(&sync_bytes[..copy_len]);
                    }
                    if let Some(orphan_index) = vfs.orphan_index_by_inode(inode) {
                        let orphan = &mut vfs.orphan_nodes[orphan_index];
                        if file_offset < orphan.bytes.len() {
                            let copy_len = sync_bytes.len().min(orphan.bytes.len() - file_offset);
                            orphan.bytes[file_offset..file_offset + copy_len]
                                .copy_from_slice(&sync_bytes[..copy_len]);
                        }
                    }
                    vfs.invalidate_caches();
                });
            }
            BOOT_PROCESSES.with_mut(|registry| {
                let Some(index) = registry.find_index(pid as u64) else {
                    return Err(Errno::Srch);
                };
                let entry = &mut registry.entries[index];
                let object = entry
                    .vm_objects
                    .iter_mut()
                    .find(|object| object.id == object_id)
                    .ok_or(Errno::Fault)?;
                for page in &mut object.page_states {
                    page.dirty = false;
                }
                boot_vm_recount_object_pages(object);
                entry.vm_decisions.push(BootVmDecision {
                    agent: "sync",
                    vm_object_id: object.id,
                    start: start as u64,
                    len: len as u64,
                    detail0: committed_pages,
                    detail1: 1,
                });
                Ok(0)
            })
        }
        Err(errno) => Err(errno),
    };
    result
}

fn advise_memory_range_syscall(
    pid: usize,
    start: usize,
    len: usize,
    advice: usize,
) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 4)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if let Some(object) = entry.vm_objects.iter_mut().find(|object| {
            (start as u64) >= object.start
                && (start as u64) < object.start.saturating_add(object.len)
        }) {
            if advice == 4 {
                for page in &mut object.page_states {
                    page.resident = false;
                    page.dirty = false;
                    page.accessed = false;
                }
            } else if advice == 3 {
                for page in &mut object.page_states {
                    page.resident = true;
                    page.accessed = true;
                }
            }
            boot_vm_recount_object_pages(object);
            entry.vm_decisions.push(BootVmDecision {
                agent: "advice",
                vm_object_id: object.id,
                start: start as u64,
                len: len as u64,
                detail0: advice as u64,
                detail1: object.resident_pages,
            });
            Ok(0)
        } else {
            Err(Errno::Fault)
        }
    })
}

fn reclaim_memory_pressure_syscall(pid: usize, target_pages: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 14)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        entry.vm_decisions.push(BootVmDecision {
            agent: "pressure-trigger",
            vm_object_id: 0,
            start: 0,
            len: target_pages as u64,
            detail0: target_pages as u64,
            detail1: entry.vm_objects.len() as u64,
        });
        let mut reclaimed = 0u64;
        for object in &mut entry.vm_objects {
            if reclaimed >= target_pages as u64 || object.quarantined {
                break;
            }
            if object.dirty_pages > 0 {
                for page in &mut object.page_states {
                    page.dirty = false;
                }
                boot_vm_recount_object_pages(object);
                entry.vm_decisions.push(BootVmDecision {
                    agent: "sync",
                    vm_object_id: object.id,
                    start: object.start,
                    len: object.len,
                    detail0: object.committed_pages,
                    detail1: 1,
                });
            }
            let victim = object
                .resident_pages
                .min((target_pages as u64).saturating_sub(reclaimed));
            let mut evicted = 0u64;
            let mut cursor = object.page_states.len();
            while cursor > 0 && evicted < victim {
                cursor -= 1;
                if object.page_states[cursor].resident {
                    object.page_states[cursor].resident = false;
                    object.page_states[cursor].dirty = false;
                    object.page_states[cursor].accessed = false;
                    evicted += 1;
                }
            }
            boot_vm_recount_object_pages(object);
            entry.vm_decisions.push(BootVmDecision {
                agent: "pressure-victim",
                vm_object_id: object.id,
                start: object.start,
                len: object.len,
                detail0: evicted,
                detail1: object.committed_pages,
            });
            reclaimed = reclaimed.saturating_add(evicted);
        }
        Ok(reclaimed as usize)
    })
}

fn reclaim_memory_pressure_global_syscall(target_pages: usize) -> Result<usize, Errno> {
    if let Some(pid) = BOOT_PROCESSES.with_mut(|registry| {
        registry
            .entries
            .iter()
            .find(|entry| !entry.reaped)
            .map(|entry| entry.pid as usize)
    }) {
        BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 14)?;
    }
    BOOT_PROCESSES.with_mut(|registry| {
        let mut best_pid = None;
        let mut best_index = None;
        let mut best_pages = 0u64;
        for entry in &registry.entries {
            if entry.reaped {
                continue;
            }
            if let Some((index, object)) = entry
                .vm_objects
                .iter()
                .enumerate()
                .filter(|(_, object)| !object.quarantined)
                .max_by_key(|(_, object)| object.resident_pages)
            {
                if object.resident_pages > best_pages {
                    best_pages = object.resident_pages;
                    best_pid = Some(entry.pid);
                    best_index = Some(index);
                }
            }
        }
        let Some(pid) = best_pid else {
            return Ok(0);
        };
        let entry = registry
            .entries
            .iter_mut()
            .find(|entry| entry.pid == pid && !entry.reaped)
            .ok_or(Errno::Srch)?;
        entry.vm_decisions.push(BootVmDecision {
            agent: "pressure-trigger",
            vm_object_id: 0,
            start: 0,
            len: target_pages as u64,
            detail0: target_pages as u64,
            detail1: 1,
        });
        let object = &mut entry.vm_objects[best_index.unwrap()];
        if object.dirty_pages > 0 {
            for page in &mut object.page_states {
                page.dirty = false;
            }
            boot_vm_recount_object_pages(object);
            entry.vm_decisions.push(BootVmDecision {
                agent: "sync",
                vm_object_id: object.id,
                start: object.start,
                len: object.len,
                detail0: object.committed_pages,
                detail1: 1,
            });
        }
        let reclaimed = object.resident_pages.min(target_pages as u64);
        let mut evicted = 0u64;
        let mut cursor = object.page_states.len();
        while cursor > 0 && evicted < reclaimed {
            cursor -= 1;
            if object.page_states[cursor].resident {
                object.page_states[cursor].resident = false;
                object.page_states[cursor].dirty = false;
                object.page_states[cursor].accessed = false;
                evicted += 1;
            }
        }
        boot_vm_recount_object_pages(object);
        entry.vm_decisions.push(BootVmDecision {
            agent: "pressure-victim",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: evicted,
            detail1: object.committed_pages,
        });
        Ok(evicted as usize)
    })
}

fn inspect_device_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeDeviceRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let record = crate::virtio_blk_boot::device_record(path)
        .or_else(|| crate::boot_gpu_runtime::device_record(path))
        .or_else(|| crate::boot_audio_runtime::device_record(path))
        .or_else(|| crate::boot_input_runtime::device_record(path))
        .or_else(|| hardware_network_device_record(path))
        .or_else(|| crate::boot_network_runtime::device_record(path))
        .ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    serial::print(format_args!(
        "ngos/x86_64: inspect_device handled path={} class={} block_size={} capacity={}\n",
        path, record.class, record.block_size, record.capacity_bytes
    ));
    Ok(0)
}

fn inspect_storage_volume_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeStorageVolumeRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let record = crate::virtio_blk_boot::inspect_volume(path).ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    Ok(0)
}

fn inspect_storage_lineage_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeStorageLineageRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let record = crate::virtio_blk_boot::inspect_lineage(path).ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    Ok(0)
}

fn prepare_storage_commit_syscall(
    path_ptr: usize,
    path_len: usize,
    tag_ptr: usize,
    tag_len: usize,
    payload_ptr: *const u8,
    payload_len: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    if payload_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let tag_bytes = unsafe { slice::from_raw_parts(tag_ptr as *const u8, tag_len) };
    let tag = core::str::from_utf8(tag_bytes).map_err(|_| Errno::Inval)?;
    let payload = unsafe { slice::from_raw_parts(payload_ptr, payload_len) };
    serial::print(format_args!(
        "ngos/x86_64: prepare_storage_commit enter path={} tag={} payload_len={}\n",
        path, tag, payload_len
    ));
    crate::virtio_blk_boot::prepare_storage_commit(path, tag, payload)
}

fn recover_storage_volume_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    crate::virtio_blk_boot::recover_storage_volume(path)
}

fn repair_storage_snapshot_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    crate::virtio_blk_boot::repair_storage_snapshot(path)
}

fn inspect_driver_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeDriverRecord,
) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: inspect_driver enter path_ptr={:#x} path_len={} out={:#x}\n",
        path_ptr, path_len, out as usize
    ));
    let path = path_from_user(path_ptr, path_len)?;
    serial::print(format_args!(
        "ngos/x86_64: inspect_driver path decoded={}\n",
        path
    ));
    let record = crate::virtio_blk_boot::driver_record(path)
        .or_else(|| crate::boot_gpu_runtime::driver_record(path))
        .or_else(|| crate::boot_audio_runtime::driver_record(path))
        .or_else(|| crate::boot_input_runtime::driver_record(path))
        .or_else(|| hardware_network_driver_record(path))
        .or_else(|| crate::boot_network_runtime::driver_record(path))
        .ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    serial::print(format_args!(
        "ngos/x86_64: inspect_driver handled path={} bound={} queued={} completed={}\n",
        path, record.bound_device_count, record.queued_requests, record.completed_requests
    ));
    Ok(0)
}

#[cfg(target_os = "none")]
fn hardware_network_device_record(path: &str) -> Option<NativeDeviceRecord> {
    crate::virtio_net_boot::device_record(path)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_device_record(_path: &str) -> Option<NativeDeviceRecord> {
    None
}

#[cfg(target_os = "none")]
fn hardware_network_driver_record(path: &str) -> Option<NativeDriverRecord> {
    crate::virtio_net_boot::driver_record(path)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_driver_record(_path: &str) -> Option<NativeDriverRecord> {
    None
}

#[cfg(target_os = "none")]
fn hardware_network_interface_record(path: &str) -> Option<NativeNetworkInterfaceRecord> {
    crate::virtio_net_boot::interface_record(path)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_interface_record(_path: &str) -> Option<NativeNetworkInterfaceRecord> {
    None
}

#[cfg(target_os = "none")]
fn hardware_network_configure_interface_ipv4(
    path: &str,
    addr: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
) -> Result<(), Errno> {
    crate::virtio_net_boot::configure_interface_ipv4(path, addr, netmask, gateway)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_configure_interface_ipv4(
    _path: &str,
    _addr: [u8; 4],
    _netmask: [u8; 4],
    _gateway: [u8; 4],
) -> Result<(), Errno> {
    Err(Errno::Nxio)
}

#[cfg(target_os = "none")]
fn hardware_network_configure_interface_admin(
    path: &str,
    mtu: u64,
    tx_capacity: u64,
    rx_capacity: u64,
    tx_inflight_limit: u64,
    admin_up: bool,
    promiscuous: bool,
) -> Result<(), Errno> {
    crate::virtio_net_boot::configure_interface_admin(
        path,
        mtu,
        tx_capacity,
        rx_capacity,
        tx_inflight_limit,
        admin_up,
        promiscuous,
    )
}

#[cfg(not(target_os = "none"))]
fn hardware_network_configure_interface_admin(
    _path: &str,
    _mtu: u64,
    _tx_capacity: u64,
    _rx_capacity: u64,
    _tx_inflight_limit: u64,
    _admin_up: bool,
    _promiscuous: bool,
) -> Result<(), Errno> {
    Err(Errno::Nxio)
}

#[cfg(target_os = "none")]
fn hardware_network_set_link_state(path: &str, link_up: bool) -> Result<(), Errno> {
    crate::virtio_net_boot::set_link_state(path, link_up)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_set_link_state(_path: &str, _link_up: bool) -> Result<(), Errno> {
    Err(Errno::Nxio)
}

#[cfg(target_os = "none")]
fn hardware_network_online() -> bool {
    crate::virtio_net_boot::is_online()
}

#[cfg(not(target_os = "none"))]
fn hardware_network_online() -> bool {
    false
}

#[cfg(target_os = "none")]
fn hardware_network_device_poll(interest: u32) -> usize {
    crate::virtio_net_boot::poll_device(interest)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_device_poll(_interest: u32) -> usize {
    0
}

#[cfg(target_os = "none")]
fn hardware_network_device_read(
    buffer: *mut u8,
    len: usize,
    nonblock: bool,
) -> Result<usize, Errno> {
    crate::virtio_net_boot::read_device(buffer, len, nonblock)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_device_read(
    _buffer: *mut u8,
    _len: usize,
    _nonblock: bool,
) -> Result<usize, Errno> {
    Err(Errno::Nxio)
}

#[cfg(target_os = "none")]
fn hardware_network_send_udp_to(
    socket_path: &str,
    remote_ipv4: [u8; 4],
    remote_port: u16,
    payload: &[u8],
) -> Result<usize, Errno> {
    let binding = crate::boot_network_runtime::udp_socket_local_binding(socket_path)?;
    let count = crate::virtio_net_boot::send_udp_packet(
        binding.local_ipv4,
        binding.local_port,
        remote_ipv4,
        remote_port,
        payload,
    )?;
    crate::boot_network_runtime::record_udp_socket_tx(socket_path)?;
    Ok(count)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_send_udp_to(
    _socket_path: &str,
    _remote_ipv4: [u8; 4],
    _remote_port: u16,
    _payload: &[u8],
) -> Result<usize, Errno> {
    Err(Errno::Nxio)
}

#[cfg(target_os = "none")]
fn hardware_network_complete_tx(driver_path: &str, completions: usize) -> Result<usize, Errno> {
    crate::virtio_net_boot::complete_udp_tx(driver_path, completions)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_complete_tx(_driver_path: &str, _completions: usize) -> Result<usize, Errno> {
    Err(Errno::Nxio)
}

#[cfg(target_os = "none")]
fn hardware_network_device_write(bytes: &[u8]) -> Result<usize, Errno> {
    crate::virtio_net_boot::write_device(bytes)
}

#[cfg(not(target_os = "none"))]
fn hardware_network_device_write(_bytes: &[u8]) -> Result<usize, Errno> {
    Err(Errno::Nxio)
}

fn inspect_device_request_syscall(
    request_id: usize,
    out: *mut NativeDeviceRequestRecord,
) -> Result<usize, Errno> {
    let record = crate::boot_gpu_runtime::device_request_record(request_id as u64)
        .or_else(|| crate::boot_audio_runtime::device_request_record(request_id as u64))
        .or_else(|| crate::boot_input_runtime::device_request_record(request_id as u64))
        .or_else(|| crate::boot_network_runtime::device_request_record(request_id as u64))
        .ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    Ok(0)
}

fn inspect_gpu_display_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeGpuDisplayRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let record = crate::boot_gpu_runtime::gpu_display_record(path).ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    Ok(0)
}

fn inspect_gpu_scanout_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeGpuScanoutRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let record = crate::boot_gpu_runtime::gpu_scanout_record(path).ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    Ok(0)
}

fn present_gpu_frame_syscall(
    path_ptr: usize,
    path_len: usize,
    frame_ptr: usize,
    frame_len: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    if frame_ptr == 0 {
        return Err(Errno::Fault);
    }
    let bytes = unsafe { slice::from_raw_parts(frame_ptr as *const u8, frame_len) };
    let request_id = crate::boot_gpu_runtime::present_frame(path, bytes)?;
    Ok(request_id as usize)
}

fn read_gpu_scanout_frame_syscall(
    path_ptr: usize,
    path_len: usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    crate::boot_gpu_runtime::read_scanout_frame(path, buffer, len)
}

fn boot_report_syscall(status: u32, stage: u32, code: i32, detail: u64) -> Result<usize, Errno> {
    if BootSessionStatus::from_raw(status).is_none() {
        return Err(Errno::Inval);
    }
    if BootSessionStage::from_raw(stage).is_none() {
        return Err(Errno::Inval);
    }
    user_runtime_status::record_boot_report(BootSessionReport {
        status,
        stage,
        code,
        reserved: 0,
        detail,
    })
    .map_err(|_| Errno::Inval)?;
    serial::print(format_args!(
        "ngos/x86_64: boot report handled status={} stage={} code={} detail={:#x}\n",
        status, stage, code, detail
    ));
    Ok(0)
}

fn create_domain_syscall(parent: usize, name_ptr: usize, name_len: usize) -> Result<usize, Errno> {
    let name = read_inline_name(name_ptr, name_len)?;
    let id = NATIVE_REGISTRY.with_mut(|registry| registry.create_domain(parent as u64, name))?;
    serial::print(format_args!(
        "ngos/x86_64: create_domain handled parent={} id={} name={}\n",
        parent,
        id,
        core::str::from_utf8(name.as_bytes()).unwrap_or("<bin>")
    ));
    Ok(id)
}

fn create_resource_syscall(
    domain: usize,
    kind_raw: u32,
    name_ptr: usize,
    name_len: usize,
) -> Result<usize, Errno> {
    let kind = NativeResourceKind::from_raw(kind_raw).ok_or(Errno::Inval)?;
    let name = read_inline_name(name_ptr, name_len)?;
    let id = NATIVE_REGISTRY.with_mut(|registry| registry.create_resource(domain, kind, name))?;
    serial::print(format_args!(
        "ngos/x86_64: create_resource handled domain={} id={} kind={} name={}\n",
        domain,
        id,
        kind_raw,
        core::str::from_utf8(name.as_bytes()).unwrap_or("<bin>")
    ));
    Ok(id)
}

fn create_contract_syscall(
    domain: usize,
    resource: usize,
    kind_raw: u32,
    label_ptr: usize,
    label_len: usize,
) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_CREATE_CONTRACT,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    let kind = NativeContractKind::from_raw(kind_raw).ok_or(Errno::Inval)?;
    diagnostics::record_function_checkpoint(
        FN_CREATE_CONTRACT,
        1,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    serial::print(format_args!(
        "ngos/x86_64: create_contract enter domain={} resource={} kind={} label_ptr={:#x} label_len={}\n",
        domain, resource, kind_raw, label_ptr, label_len
    ));
    let label = read_inline_name(label_ptr, label_len)?;
    diagnostics::record_function_checkpoint(
        FN_CREATE_CONTRACT,
        2,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    serial::print(format_args!(
        "ngos/x86_64: create_contract label-read domain={} resource={} kind={} label={}\n",
        domain,
        resource,
        kind_raw,
        core::str::from_utf8(label.as_bytes()).unwrap_or("<bin>")
    ));
    serial::print(format_args!(
        "ngos/x86_64: create_contract registry-enter domain={} resource={} kind={}\n",
        domain, resource, kind_raw
    ));
    let id = NATIVE_REGISTRY
        .with_mut(|registry| registry.create_contract(domain, resource, kind, label))?;
    diagnostics::record_function_checkpoint(
        FN_CREATE_CONTRACT,
        4,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    diagnostics::record_function_exit(
        FN_CREATE_CONTRACT,
        6,
        kind_raw as u64,
        domain as u64,
        resource as u64,
        true,
        0,
    );
    serial::print(format_args!(
        "ngos/x86_64: create_contract handled domain={} resource={} id={} kind={} label={}\n",
        domain,
        resource,
        id,
        kind_raw,
        core::str::from_utf8(label.as_bytes()).unwrap_or("<bin>")
    ));
    Ok(id)
}

fn list_domains_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let mut ids = [0u64; MAX_DOMAIN_COUNT];
        let mut count = 0usize;
        for entry in registry.domains.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_domain_syscall(id: usize, out: *mut NativeDomainRecord) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let entry = registry.domain(id)?;
        write_record(
            out,
            NativeDomainRecord {
                id: entry.id,
                owner: entry.owner,
                parent: entry.parent,
                resource_count: entry.resource_count,
                contract_count: entry.contract_count,
            },
        )?;
        Ok(0)
    })
}

fn list_resources_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let mut ids = [0u64; MAX_RESOURCE_COUNT];
        let mut count = 0usize;
        for entry in registry.resources.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_resource_syscall(id: usize, out: *mut NativeResourceRecord) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let entry = registry.resource(id)?;
        write_record(
            out,
            NativeResourceRecord {
                id: entry.id,
                domain: entry.domain,
                creator: entry.creator,
                holder_contract: entry.holder_contract,
                kind: entry.kind as u32,
                state: entry.state as u32,
                arbitration: entry.arbitration as u32,
                governance: entry.governance as u32,
                contract_policy: entry.contract_policy as u32,
                issuer_policy: entry.issuer_policy as u32,
                waiting_count: entry.waiting_count as u64,
                acquire_count: entry.acquire_count,
                handoff_count: entry.handoff_count,
            },
        )?;
        Ok(0)
    })
}

fn create_bus_peer_syscall(
    domain: usize,
    name_ptr: usize,
    name_len: usize,
) -> Result<usize, Errno> {
    let name = read_inline_name(name_ptr, name_len)?;
    let owner = active_process_pid()?;
    NATIVE_REGISTRY.with(|registry| {
        registry.domain(domain)?;
        Ok(())
    })?;
    let id = BOOT_BUS.with_mut(|bus| bus.create_peer(owner, domain as u64, name))?;
    Ok(id as usize)
}

fn create_bus_endpoint_syscall(
    domain: usize,
    resource: usize,
    path_ptr: usize,
    path_len: usize,
) -> Result<usize, Errno> {
    let path = resolved_path_from_user(path_ptr, path_len)?;
    NATIVE_REGISTRY.with(|registry| {
        let domain_entry = registry.domain(domain)?;
        let resource_entry = registry.resource(resource)?;
        if resource_entry.domain != domain_entry.id {
            return Err(Errno::Inval);
        }
        if resource_entry.kind != NativeResourceKind::Channel {
            return Err(Errno::Inval);
        }
        Ok(())
    })?;
    BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(&path, true)?;
        if vfs.nodes[index].kind != BootNodeKind::Channel {
            return Err(Errno::Inval);
        }
        Ok(())
    })?;
    let id = BOOT_BUS.with_mut(|bus| bus.create_endpoint(domain as u64, resource as u64, path))?;
    Ok(id as usize)
}

fn list_bus_peers_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    BOOT_BUS.with(|bus| {
        let mut ids = [0u64; MAX_BUS_PEER_COUNT];
        let mut count = 0usize;
        for entry in bus.peers.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_bus_peer_syscall(id: usize, out: *mut NativeBusPeerRecord) -> Result<usize, Errno> {
    BOOT_BUS.with(|bus| {
        let entry = bus.peer(id as u64)?;
        write_record(
            out,
            NativeBusPeerRecord {
                id: entry.id,
                owner: entry.owner,
                domain: entry.domain,
                attached_endpoint_count: entry.attached_endpoint_count,
                readable_endpoint_count: entry.readable_endpoint_count,
                writable_endpoint_count: entry.writable_endpoint_count,
                publish_count: entry.publish_count,
                receive_count: entry.receive_count,
                last_endpoint: entry.last_endpoint,
            },
        )?;
        Ok(0)
    })
}

fn list_bus_endpoints_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    BOOT_BUS.with(|bus| {
        let mut ids = [0u64; MAX_BUS_ENDPOINT_COUNT];
        let mut count = 0usize;
        for entry in bus.endpoints.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_bus_endpoint_syscall(
    id: usize,
    out: *mut NativeBusEndpointRecord,
) -> Result<usize, Errno> {
    BOOT_BUS.with(|bus| {
        let entry = bus.endpoint(id as u64)?;
        let readable_peer_count = entry
            .attached_peers
            .iter()
            .filter(|attachment| bus_attachment_contains(attachment.rights, BlockRightsMask::READ))
            .count() as u64;
        let writable_peer_count = entry
            .attached_peers
            .iter()
            .filter(|attachment| bus_attachment_contains(attachment.rights, BlockRightsMask::WRITE))
            .count() as u64;
        write_record(
            out,
            NativeBusEndpointRecord {
                id: entry.id,
                domain: entry.domain,
                resource: entry.resource,
                kind: NativeObjectKind::Channel as u32,
                reserved: 0,
                attached_peer_count: entry.attached_peers.len() as u64,
                readable_peer_count,
                writable_peer_count,
                publish_count: entry.publish_count,
                receive_count: entry.receive_count,
                byte_count: entry.byte_count,
                queue_depth: entry.queue.len() as u64,
                queue_capacity: BUS_ENDPOINT_QUEUE_CAPACITY as u64,
                peak_queue_depth: entry.peak_queue_depth,
                overflow_count: entry.overflow_count,
                last_peer: entry.last_peer,
            },
        )?;
        Ok(0)
    })
}

fn attach_bus_peer_syscall(peer: usize, endpoint: usize, rights_raw: u64) -> Result<usize, Errno> {
    let rights = bus_attachment_rights(rights_raw)?;
    BOOT_BUS.with_mut(|bus| {
        let peer_domain = bus.peer(peer as u64)?.domain;
        let endpoint_domain = bus.endpoint(endpoint as u64)?.domain;
        if peer_domain != endpoint_domain {
            return Err(Errno::Inval);
        }
        let previous = {
            let endpoint_entry = bus.endpoint(endpoint as u64)?;
            endpoint_entry
                .attached_peers
                .iter()
                .find(|attachment| attachment.peer == peer as u64)
                .copied()
        };
        {
            let endpoint_entry = bus.endpoint_mut(endpoint as u64)?;
            if let Some(attachment) = endpoint_entry
                .attached_peers
                .iter_mut()
                .find(|attachment| attachment.peer == peer as u64)
            {
                attachment.rights = rights.0;
            } else {
                endpoint_entry.attached_peers.push(BootBusAttachmentEntry {
                    peer: peer as u64,
                    rights: rights.0,
                });
            }
        }
        let peer_entry = bus.peer_mut(peer as u64)?;
        if previous.is_none() {
            peer_entry.attached_endpoint_count =
                peer_entry.attached_endpoint_count.saturating_add(1);
        }
        if previous
            .map(|attachment| bus_attachment_contains(attachment.rights, BlockRightsMask::READ))
            .unwrap_or(false)
            && !rights.contains(BlockRightsMask::READ)
        {
            peer_entry.readable_endpoint_count =
                peer_entry.readable_endpoint_count.saturating_sub(1);
        } else if !previous
            .map(|attachment| bus_attachment_contains(attachment.rights, BlockRightsMask::READ))
            .unwrap_or(false)
            && rights.contains(BlockRightsMask::READ)
        {
            peer_entry.readable_endpoint_count =
                peer_entry.readable_endpoint_count.saturating_add(1);
        }
        if previous
            .map(|attachment| bus_attachment_contains(attachment.rights, BlockRightsMask::WRITE))
            .unwrap_or(false)
            && !rights.contains(BlockRightsMask::WRITE)
        {
            peer_entry.writable_endpoint_count =
                peer_entry.writable_endpoint_count.saturating_sub(1);
        } else if !previous
            .map(|attachment| bus_attachment_contains(attachment.rights, BlockRightsMask::WRITE))
            .unwrap_or(false)
            && rights.contains(BlockRightsMask::WRITE)
        {
            peer_entry.writable_endpoint_count =
                peer_entry.writable_endpoint_count.saturating_add(1);
        }
        peer_entry.last_endpoint = endpoint as u64;
        Ok(())
    })?;
    emit_bus_event(peer as u64, endpoint as u64, BootBusEventKind::Attached);
    Ok(0)
}

fn detach_bus_peer_syscall(peer: usize, endpoint: usize) -> Result<usize, Errno> {
    BOOT_BUS.with_mut(|bus| {
        let endpoint_entry = bus.endpoint_mut(endpoint as u64)?;
        let Some(index) = endpoint_entry
            .attached_peers
            .iter()
            .position(|candidate| candidate.peer == peer as u64)
        else {
            return Err(Errno::Inval);
        };
        let attachment = endpoint_entry.attached_peers.remove(index);
        let peer_entry = bus.peer_mut(peer as u64)?;
        peer_entry.attached_endpoint_count = peer_entry.attached_endpoint_count.saturating_sub(1);
        if bus_attachment_contains(attachment.rights, BlockRightsMask::READ) {
            peer_entry.readable_endpoint_count =
                peer_entry.readable_endpoint_count.saturating_sub(1);
        }
        if bus_attachment_contains(attachment.rights, BlockRightsMask::WRITE) {
            peer_entry.writable_endpoint_count =
                peer_entry.writable_endpoint_count.saturating_sub(1);
        }
        peer_entry.last_endpoint = endpoint as u64;
        Ok(())
    })?;
    emit_bus_event(peer as u64, endpoint as u64, BootBusEventKind::Detached);
    Ok(0)
}

fn publish_bus_message_syscall(
    peer: usize,
    endpoint: usize,
    payload_ptr: *const u8,
    payload_len: usize,
) -> Result<usize, Errno> {
    if payload_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let payload = unsafe { slice::from_raw_parts(payload_ptr, payload_len) };
    BOOT_BUS.with_mut(|bus| {
        let endpoint_entry = bus.endpoint_mut(endpoint as u64)?;
        let Some(attachment) = endpoint_entry
            .attached_peers
            .iter()
            .find(|attachment| attachment.peer == peer as u64)
            .copied()
        else {
            return Err(Errno::Inval);
        };
        if !bus_attachment_contains(attachment.rights, BlockRightsMask::WRITE) {
            return Err(Errno::Access);
        }
        if endpoint_entry.queue.len() >= BUS_ENDPOINT_QUEUE_CAPACITY {
            endpoint_entry.overflow_count = endpoint_entry.overflow_count.saturating_add(1);
            return Err(Errno::Again);
        }
        endpoint_entry.queue.push(payload.to_vec());
        endpoint_entry.publish_count = endpoint_entry.publish_count.saturating_add(1);
        endpoint_entry.byte_count = endpoint_entry.byte_count.saturating_add(payload_len as u64);
        endpoint_entry.peak_queue_depth = endpoint_entry
            .peak_queue_depth
            .max(endpoint_entry.queue.len() as u64);
        endpoint_entry.last_peer = peer as u64;
        let peer_entry = bus.peer_mut(peer as u64)?;
        peer_entry.publish_count = peer_entry.publish_count.saturating_add(1);
        peer_entry.last_endpoint = endpoint as u64;
        Ok(payload_len)
    })?;
    emit_bus_event(peer as u64, endpoint as u64, BootBusEventKind::Published);
    Ok(payload_len)
}

fn receive_bus_message_syscall(
    peer: usize,
    endpoint: usize,
    buffer: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let received = BOOT_BUS.with_mut(|bus| {
        let endpoint_entry = bus.endpoint_mut(endpoint as u64)?;
        let Some(attachment) = endpoint_entry
            .attached_peers
            .iter()
            .find(|attachment| attachment.peer == peer as u64)
            .copied()
        else {
            return Err(Errno::Inval);
        };
        if !bus_attachment_contains(attachment.rights, BlockRightsMask::READ) {
            return Err(Errno::Access);
        }
        if endpoint_entry.queue.is_empty() {
            return Err(Errno::Again);
        }
        let payload = endpoint_entry.queue.remove(0);
        let bytes = payload.len().min(capacity);
        unsafe {
            ptr::copy_nonoverlapping(payload.as_ptr(), buffer, bytes);
        }
        endpoint_entry.receive_count = endpoint_entry.receive_count.saturating_add(1);
        endpoint_entry.last_peer = peer as u64;
        let peer_entry = bus.peer_mut(peer as u64)?;
        peer_entry.receive_count = peer_entry.receive_count.saturating_add(1);
        peer_entry.last_endpoint = endpoint as u64;
        Ok(bytes)
    })?;
    emit_bus_event(peer as u64, endpoint as u64, BootBusEventKind::Received);
    Ok(received)
}

fn list_contracts_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let mut ids = [0u64; MAX_CONTRACT_COUNT];
        let mut count = 0usize;
        for entry in registry.contracts.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_contract_syscall(id: usize, out: *mut NativeContractRecord) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: inspect_contract enter id={} out={:p}\n",
        id, out
    ));
    NATIVE_REGISTRY.with(|registry| {
        let entry = registry.contract(id)?;
        write_record(
            out,
            NativeContractRecord {
                id: entry.id,
                domain: entry.domain,
                resource: entry.resource,
                issuer: entry.issuer,
                kind: entry.kind as u32,
                state: entry.state as u32,
            },
        )?;
        serial::print(format_args!(
            "ngos/x86_64: inspect_contract handled id={} domain={} resource={} state={}\n",
            entry.id, entry.domain, entry.resource, entry.state as u32
        ));
        Ok(0)
    })
}

fn get_domain_name_syscall(id: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| copy_name_to_user(&registry.domain(id)?.name, buffer, capacity))
}

fn get_resource_name_syscall(id: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY
        .with(|registry| copy_name_to_user(&registry.resource(id)?.name, buffer, capacity))
}

fn get_contract_label_syscall(id: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY
        .with(|registry| copy_name_to_user(&registry.contract(id)?.label, buffer, capacity))
}

fn bind_process_contract_syscall(contract: usize) -> Result<usize, Errno> {
    BootProcessContractBindAgent::execute(contract)?;
    Ok(0)
}

fn set_contract_state_syscall(id: usize, state_raw: u32) -> Result<usize, Errno> {
    let state = NativeContractState::from_raw(state_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let contract_slot = registry.contract_slot(id)?;
        let resource_id = registry.contracts[contract_slot].as_ref().unwrap().resource as usize;
        let resource_slot = registry.resource_slot(resource_id)?;
        let contract_id = registry.contracts[contract_slot].as_ref().unwrap().id;
        registry.contracts[contract_slot].as_mut().unwrap().state = state;
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        let removed_waiter = NativeRegistry::remove_waiter(resource, contract_id);
        if state == NativeContractState::Revoked && resource.holder_contract == contract_id {
            resource.holder_contract = 0;
        }
        Ok((resource.id, contract_id, removed_waiter))
    });
    if let Ok((resource_id, contract_id, removed_waiter)) = result {
        if removed_waiter || state == NativeContractState::Revoked {
            emit_resource_event(resource_id, contract_id, BootResourceEventKind::Revoked);
        }
        serial::print(format_args!(
            "ngos/x86_64: set_contract_state handled id={} state={}\n",
            id, state_raw
        ));
        return Ok(0);
    }
    result.map(|_| 0)
}

fn invoke_contract_syscall(id: usize) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: invoke_contract enter id={}\n",
        id
    ));
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let contract_slot = registry.contract_slot(id)?;
        let resource_slot = registry
            .resource_slot(registry.contracts[contract_slot].as_ref().unwrap().resource as usize)?;
        let contract = registry.contracts[contract_slot].as_mut().unwrap();
        let resource = registry.resources[resource_slot].as_ref().unwrap();
        if contract.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
        {
            return Err(Errno::Access);
        }
        contract.invocation_count += 1;
        Ok(contract.invocation_count as usize)
    });
    match result {
        Ok(count) => {
            serial::print(format_args!(
                "ngos/x86_64: invoke_contract handled id={} count={}\n",
                id, count
            ));
            Ok(count)
        }
        Err(err) => {
            serial::print(format_args!(
                "ngos/x86_64: invoke_contract rejected id={} err={:?}\n",
                id, err
            ));
            Err(err)
        }
    }
}

fn set_resource_policy_syscall(resource: usize, policy_raw: u32) -> Result<usize, Errno> {
    let policy = NativeResourceArbitrationPolicy::from_raw(policy_raw).ok_or(Errno::Inval)?;
    NATIVE_REGISTRY.with_mut(|registry| {
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .arbitration = policy;
        Ok(0)
    })
}

fn claim_resource_syscall(
    contract: usize,
    out: *mut NativeResourceClaimRecord,
) -> Result<usize, Errno> {
    diagnostics::record_function_enter(FN_CLAIM_RESOURCE, 0, contract as u64, out as u64);
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            1,
            0,
            contract as u64,
            out as u64,
        );
        let (contract_slot, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let contract_entry = registry.contracts[contract_slot].as_ref().unwrap();
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            2,
            0,
            contract as u64,
            resource.id,
        );
        if contract_entry.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
            || !contract_kind_allowed(resource.contract_policy, contract_entry.kind)
        {
            return Err(Errno::Access);
        }
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            3,
            0,
            contract as u64,
            resource.id,
        );
        if resource.holder_contract == 0 {
            diagnostics::record_function_checkpoint(
                FN_CLAIM_RESOURCE,
                4,
                1,
                contract as u64,
                resource.id,
            );
            resource.holder_contract = contract as u64;
            resource.acquire_count += 1;
            diagnostics::record_function_checkpoint(
                FN_CLAIM_RESOURCE,
                5,
                1,
                contract as u64,
                resource.id,
            );
            write_record(
                out,
                NativeResourceClaimRecord {
                    resource: resource.id,
                    holder_contract: 0,
                    acquire_count: resource.acquire_count,
                    position: 0,
                    queued: 0,
                    reserved: 0,
                },
            )?;
            diagnostics::record_function_checkpoint(
                FN_CLAIM_RESOURCE,
                6,
                1,
                contract as u64,
                resource.id,
            );
            return Ok((
                0usize,
                Some((resource.id, contract as u64, BootResourceEventKind::Claimed)),
            ));
        }
        if resource.holder_contract == contract as u64 {
            return Err(Errno::Access);
        }
        if resource.governance == NativeResourceGovernanceMode::ExclusiveLease {
            return Err(Errno::Busy);
        }
        if resource.waiting_count >= MAX_CONTRACT_COUNT {
            return Err(Errno::Again);
        }
        if resource.waiters[..resource.waiting_count]
            .iter()
            .any(|id| *id == contract as u64)
        {
            return Err(Errno::Access);
        }
        resource.waiters[resource.waiting_count] = contract as u64;
        resource.waiting_count += 1;
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            4,
            2,
            contract as u64,
            resource.id,
        );
        write_record(
            out,
            NativeResourceClaimRecord {
                resource: resource.id,
                holder_contract: resource.holder_contract,
                acquire_count: resource.acquire_count,
                position: resource.waiting_count as u64,
                queued: 1,
                reserved: 0,
            },
        )?;
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            5,
            2,
            contract as u64,
            resource.id,
        );
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            6,
            2,
            contract as u64,
            resource.id,
        );
        Ok((
            0usize,
            Some((resource.id, contract as u64, BootResourceEventKind::Queued)),
        ))
    });
    if let Ok((value, event)) = result {
        if let Some((resource, contract, kind)) = event {
            emit_resource_event(resource, contract, kind);
        }
        diagnostics::record_function_exit(FN_CLAIM_RESOURCE, 6, 0, contract as u64, 0, true, 0);
        serial::print(format_args!(
            "ngos/x86_64: claim_resource handled contract={}\n",
            contract
        ));
        return Ok(value);
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_CLAIM_RESOURCE,
            6,
            0,
            contract as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: claim_resource rejected contract={} err={:?}\n",
            contract, err
        ));
    }
    result.map(|(value, _)| value)
}

fn acquire_resource_syscall(contract: usize) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (contract_slot, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let contract_entry = registry.contracts[contract_slot].as_ref().unwrap();
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if contract_entry.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
            || !contract_kind_allowed(resource.contract_policy, contract_entry.kind)
        {
            return Err(Errno::Access);
        }
        if resource.holder_contract == 0 {
            resource.holder_contract = contract as u64;
            resource.acquire_count += 1;
            return Ok(resource.id as usize);
        }
        if resource.holder_contract == contract as u64 {
            return Ok(resource.id as usize);
        }
        Err(Errno::Busy)
    });
    if let Ok(resource) = result {
        serial::print(format_args!(
            "ngos/x86_64: acquire_resource handled contract={} resource={}\n",
            contract, resource
        ));
    } else if let Err(err) = result {
        serial::print(format_args!(
            "ngos/x86_64: acquire_resource rejected contract={} err={:?}\n",
            contract, err
        ));
    }
    result
}

fn list_resource_waiters_syscall(
    resource: usize,
    buffer: *mut u64,
    capacity: usize,
) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with(|registry| {
        let entry = registry.resource(resource)?;
        copy_ids_to_user(&entry.waiters[..entry.waiting_count], buffer, capacity)
    });
    if let Ok(count) = result {
        serial::print(format_args!(
            "ngos/x86_64: list_resource_waiters handled resource={} count={}\n",
            resource, count
        ));
    }
    result
}

fn cancel_resource_claim_syscall(
    contract: usize,
    out: *mut NativeResourceCancelRecord,
) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (_, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if !NativeRegistry::remove_waiter(resource, contract as u64) {
            return Err(Errno::Access);
        }
        write_record(
            out,
            NativeResourceCancelRecord {
                resource: resource.id,
                waiting_count: resource.waiting_count as u64,
            },
        )?;
        Ok((resource.id, contract as u64))
    });
    if let Ok((resource_id, contract_id)) = result {
        emit_resource_event(resource_id, contract_id, BootResourceEventKind::Canceled);
        serial::print(format_args!(
            "ngos/x86_64: cancel_resource_claim handled contract={}\n",
            contract
        ));
        return Ok(0);
    }
    result.map(|_| 0)
}

fn release_claimed_resource_syscall(
    contract: usize,
    out: *mut NativeResourceReleaseRecord,
) -> Result<usize, Errno> {
    diagnostics::record_function_enter(FN_RELEASE_CLAIMED_RESOURCE, 0, contract as u64, out as u64);
    serial::print(format_args!(
        "ngos/x86_64: release_claimed_resource enter contract={} out={:p}\n",
        contract, out
    ));
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_RELEASE_CLAIMED_RESOURCE,
            1,
            0,
            contract as u64,
            out as u64,
        );
        let (_, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let next_waiter = {
            let resource = registry.resources[resource_slot].as_ref().unwrap();
            diagnostics::record_function_checkpoint(
                FN_RELEASE_CLAIMED_RESOURCE,
                2,
                0,
                contract as u64,
                resource.id,
            );
            if resource.holder_contract != contract as u64 {
                return Err(Errno::Access);
            }
            if resource.governance == NativeResourceGovernanceMode::Queueing {
                registry.select_handoff_waiter(resource_slot)
            } else {
                None
            }
        };
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        diagnostics::record_function_checkpoint(
            FN_RELEASE_CLAIMED_RESOURCE,
            3,
            next_waiter.unwrap_or(0),
            contract as u64,
            resource.id,
        );
        match next_waiter {
            Some(next_contract) => {
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    4,
                    next_contract,
                    contract as u64,
                    resource.id,
                );
                resource.holder_contract = next_contract;
                resource.acquire_count += 1;
                resource.handoff_count += 1;
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    5,
                    next_contract,
                    contract as u64,
                    resource.id,
                );
                write_record(
                    out,
                    NativeResourceReleaseRecord {
                        resource: resource.id,
                        handoff_contract: next_contract,
                        acquire_count: resource.acquire_count,
                        handoff_count: resource.handoff_count,
                        handed_off: 1,
                        reserved: 0,
                    },
                )?;
                Ok((
                    0usize,
                    resource.id,
                    Some((contract as u64, BootResourceEventKind::Released)),
                    Some((next_contract, BootResourceEventKind::HandedOff)),
                ))
            }
            None => {
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    4,
                    0,
                    contract as u64,
                    resource.id,
                );
                resource.holder_contract = 0;
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    5,
                    0,
                    contract as u64,
                    resource.id,
                );
                write_record(
                    out,
                    NativeResourceReleaseRecord {
                        resource: resource.id,
                        handoff_contract: 0,
                        acquire_count: resource.acquire_count,
                        handoff_count: resource.handoff_count,
                        handed_off: 0,
                        reserved: 0,
                    },
                )?;
                Ok((
                    0usize,
                    resource.id,
                    Some((contract as u64, BootResourceEventKind::Released)),
                    None,
                ))
            }
        }
    });
    if let Ok((value, resource_id, released, handed_off)) = result {
        if let Some((event_contract, kind)) = released {
            emit_resource_event(resource_id, event_contract, kind);
        }
        if let Some((event_contract, kind)) = handed_off {
            emit_resource_event(resource_id, event_contract, kind);
        }
        diagnostics::record_function_exit(
            FN_RELEASE_CLAIMED_RESOURCE,
            6,
            0,
            contract as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: release_claimed_resource handled contract={}\n",
            contract
        ));
        return Ok(value);
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_RELEASE_CLAIMED_RESOURCE,
            6,
            0,
            contract as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: release_claimed_resource rejected contract={} err={:?}\n",
            contract, err
        ));
    }
    result.map(|(value, ..)| value)
}

fn transfer_resource_syscall(source: usize, target: usize) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (source_slot, resource_slot) = registry.contract_and_resource_slots(source)?;
        let target_slot = registry.contract_slot(target)?;
        let source_contract = registry.contracts[source_slot].as_ref().unwrap();
        let target_contract = registry.contracts[target_slot].as_ref().unwrap();
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if resource.holder_contract != source as u64
            || source_contract.resource != target_contract.resource
            || target_contract.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
            || !contract_kind_allowed(resource.contract_policy, target_contract.kind)
        {
            return Err(Errno::Access);
        }
        NativeRegistry::remove_waiter(resource, target as u64);
        resource.holder_contract = target as u64;
        resource.acquire_count += 1;
        resource.handoff_count += 1;
        Ok((resource.id as usize, source as u64, target as u64))
    });
    if let Ok((resource, _source, target)) = result {
        emit_resource_event(resource as u64, target, BootResourceEventKind::HandedOff);
        serial::print(format_args!(
            "ngos/x86_64: transfer_resource handled source={} target={} resource={}\n",
            source, target, resource
        ));
        return Ok(resource);
    }
    result.map(|(resource, ..)| resource)
}

fn release_resource_syscall(contract: usize) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (_, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if resource.holder_contract != contract as u64 {
            return Err(Errno::Access);
        }
        resource.holder_contract = 0;
        Ok((resource.id as usize, contract as u64))
    });
    if let Ok((resource, contract_id)) = result {
        emit_resource_event(
            resource as u64,
            contract_id,
            BootResourceEventKind::Released,
        );
        serial::print(format_args!(
            "ngos/x86_64: release_resource handled contract={} resource={}\n",
            contract, resource
        ));
        return Ok(resource);
    }
    result.map(|(resource, _)| resource)
}

fn set_resource_governance_syscall(resource: usize, mode_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_SET_RESOURCE_GOVERNANCE,
        mode_raw as u64,
        resource as u64,
        0,
    );
    let mode = NativeResourceGovernanceMode::from_raw(mode_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_GOVERNANCE,
            1,
            mode_raw as u64,
            resource as u64,
            0,
        );
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .governance = mode;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_GOVERNANCE,
            4,
            mode_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_GOVERNANCE,
            6,
            mode_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_GOVERNANCE,
            6,
            mode_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_governance handled resource={} mode={}\n",
            resource, mode_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_GOVERNANCE,
            6,
            mode_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_governance rejected resource={} mode={} err={:?}\n",
            resource, mode_raw, err
        ));
    }
    result
}

fn set_resource_contract_policy_syscall(resource: usize, policy_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_SET_RESOURCE_CONTRACT_POLICY,
        policy_raw as u64,
        resource as u64,
        0,
    );
    let policy = NativeResourceContractPolicy::from_raw(policy_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            1,
            policy_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            3,
            policy_raw as u64,
            resource as u64,
            0,
        );
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .contract_policy = policy;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            4,
            policy_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_contract_policy handled resource={} policy={}\n",
            resource, policy_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_contract_policy rejected resource={} policy={} err={:?}\n",
            resource, policy_raw, err
        ));
    }
    result
}

fn set_resource_issuer_policy_syscall(resource: usize, policy_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_SET_RESOURCE_ISSUER_POLICY,
        policy_raw as u64,
        resource as u64,
        0,
    );
    let policy = NativeResourceIssuerPolicy::from_raw(policy_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_ISSUER_POLICY,
            1,
            policy_raw as u64,
            resource as u64,
            0,
        );
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .issuer_policy = policy;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_ISSUER_POLICY,
            4,
            policy_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_ISSUER_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_ISSUER_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_issuer_policy handled resource={} policy={}\n",
            resource, policy_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_ISSUER_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_issuer_policy rejected resource={} policy={} err={:?}\n",
            resource, policy_raw, err
        ));
    }
    result
}

fn set_resource_state_syscall(resource: usize, state_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(FN_SET_RESOURCE_STATE, state_raw as u64, resource as u64, 0);
    let state = NativeResourceState::from_raw(state_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            1,
            state_raw as u64,
            resource as u64,
            0,
        );
        let resource_slot = registry.resource_slot(resource)?;
        let resource_entry = registry.resources[resource_slot].as_mut().unwrap();
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            2,
            state_raw as u64,
            resource as u64,
            0,
        );
        resource_entry.state = state;
        resource_entry.holder_contract = 0;
        resource_entry.waiters = [0; MAX_CONTRACT_COUNT];
        resource_entry.waiting_count = 0;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            4,
            state_raw as u64,
            resource as u64,
            0,
        );
        if state == NativeResourceState::Retired {
            for contract in registry.contracts.iter_mut().flatten() {
                if contract.resource == resource as u64 {
                    contract.state = NativeContractState::Revoked;
                }
            }
        }
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            6,
            state_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_STATE,
            6,
            state_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_state handled resource={} state={}\n",
            resource, state_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_STATE,
            6,
            state_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_state rejected resource={} state={} err={:?}\n",
            resource, state_raw, err
        ));
    }
    result
}

fn encode_syscall_result(result: Result<usize, Errno>) -> usize {
    match result {
        Ok(value) => SyscallReturn::ok(value).raw() as usize,
        Err(errno) => SyscallReturn::err(errno).raw() as usize,
    }
}

fn decode_fcntl(encoded: usize) -> Option<DecodedFcntl> {
    let command = match encoded & 0xff {
        0 => DecodedFcntl::GetFl,
        1 => DecodedFcntl::GetFd,
        2 => DecodedFcntl::SetFl {
            nonblock: ((encoded >> 8) & 0x1) != 0,
        },
        3 => DecodedFcntl::SetFd {
            cloexec: ((encoded >> 8) & 0x1) != 0,
        },
        4 => DecodedFcntl::QueryLock,
        5 => DecodedFcntl::TryLockExclusive {
            token: ((encoded >> 8) & 0xffff) as u16,
        },
        6 => DecodedFcntl::UnlockExclusive {
            token: ((encoded >> 8) & 0xffff) as u16,
        },
        7 => DecodedFcntl::TryLockShared {
            token: ((encoded >> 8) & 0xffff) as u16,
        },
        8 => DecodedFcntl::UnlockShared {
            token: ((encoded >> 8) & 0xffff) as u16,
        },
        9 => DecodedFcntl::UpgradeLockExclusive {
            token: ((encoded >> 8) & 0xffff) as u16,
        },
        10 => DecodedFcntl::DowngradeLockShared {
            token: ((encoded >> 8) & 0xffff) as u16,
        },
        _ => return None,
    };
    Some(command)
}

fn encode_flags(flags: DescriptorFlags) -> usize {
    ((flags.cloexec as usize) << 1) | (flags.nonblock as usize)
}

fn descriptor_lock_inode(target: DescriptorTarget) -> Option<u64> {
    match target {
        DescriptorTarget::BootDirectory(inode)
        | DescriptorTarget::BootFile(inode)
        | DescriptorTarget::BootChannel(inode) => Some(inode),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_runtime_status;
    use std::sync::{Mutex, OnceLock};

    fn user_syscall_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TestGuards {
        _cpu: std::sync::MutexGuard<'static, ()>,
        _state: std::sync::MutexGuard<'static, ()>,
        _io: std::sync::MutexGuard<'static, ()>,
    }

    fn lock_user_syscall_test_state() -> TestGuards {
        TestGuards {
            _cpu: crate::cpu_runtime_status::lock_shared_test_state(),
            _state: user_syscall_test_lock()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            _io: crate::serial::lock_test_io(),
        }
    }

    fn reset_user_syscall_test_state() {
        crate::keyboard::reset_state();
        crate::serial::clear_input();
        crate::serial::clear_output();
        crate::boot_gpu_runtime::reset();
        crate::boot_audio_runtime::reset();
        crate::boot_input_runtime::reset();
        crate::boot_network_runtime::reset();
        unsafe {
            PROCESS_EXIT_CODE = 0;
            PROCESS_EXITED = false;
        }
        NATIVE_REGISTRY.with_mut(|registry| *registry = NativeRegistry::new());
        BOOT_BUS.with_mut(|registry| *registry = BootBusRegistry::new());
        BOOT_PROCESSES.with_mut(|registry| *registry = BootProcessRegistry::new());
        BOOT_VFS.with_mut(|vfs| *vfs = BootVfs::new());
        STORAGE_MOUNT.with_mut(|state| *state = StorageMountState::default());
        BOOT_EVENT_QUEUES.with_mut(|queues| *queues = BootEventQueueRegistry::new());
        DESCRIPTORS.with_mut(|descriptors| *descriptors = DescriptorTable::new());
        VFS_LOCKS.with_mut(|locks| locks.clear());
        set_active_process_pid(1);
        BootVfs::set_current_subject(1000, 1000);
        BootVfs::set_current_subject_label(SecurityLabel::new(
            ConfidentialityLevel::Public,
            IntegrityLevel::Verified,
        ));
        BootVfs::set_current_umask(0o022);
        BootVfs::set_current_supplemental_groups(&[]);
    }

    fn read_procfs_text(path: &str) -> String {
        let mut buffer = vec![0u8; 8192];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        String::from_utf8(buffer[..count].to_vec()).unwrap()
    }

    fn bind_observe_contract(name: &'static [u8]) {
        let domain = create_domain_syscall(0, name.as_ptr() as usize, name.len()).unwrap();
        let resource = create_resource_syscall(
            domain,
            NativeResourceKind::Namespace as u32,
            name.as_ptr() as usize,
            name.len(),
        )
        .unwrap();
        assert_eq!(
            set_resource_contract_policy_syscall(
                resource,
                NativeResourceContractPolicy::Observe as u32,
            ),
            Ok(0)
        );
        let contract = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Observe as u32,
            name.as_ptr() as usize,
            name.len(),
        )
        .unwrap();
        assert_eq!(bind_process_contract_syscall(contract), Ok(0));
    }

    fn list_path_text(path: &str) -> String {
        let mut buffer = vec![0u8; 8192];
        let count = list_path_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        String::from_utf8(buffer[..count].to_vec()).unwrap()
    }

    fn parse_procfs_counter(text: &str, key: &str) -> u64 {
        text.split_whitespace()
            .find_map(|part| {
                let value = part.strip_prefix(key)?.trim_end_matches('\n');
                value.parse::<u64>().ok()
            })
            .unwrap_or(0)
    }

    #[test]
    fn boot_procfs_system_scheduler_exposes_scheduler_smoke_baseline_markers() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert!(
            read_procfs_syscall(
                b"/proc/system/scheduler".as_ptr() as usize,
                "/proc/system/scheduler".len(),
                core::ptr::null_mut(),
                0,
            )
            .is_err()
        );

        let domain =
            create_domain_syscall(0, b"boot-scheduler-observe".as_ptr() as usize, 22).unwrap();
        let resource = create_resource_syscall(
            domain,
            NativeResourceKind::Namespace as u32,
            b"boot-scheduler-observe".as_ptr() as usize,
            22,
        )
        .unwrap();
        assert_eq!(
            set_resource_contract_policy_syscall(
                resource,
                NativeResourceContractPolicy::Observe as u32
            ),
            Ok(0)
        );
        let contract = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Observe as u32,
            b"boot-scheduler-observe".as_ptr() as usize,
            22,
        )
        .unwrap();
        assert_eq!(bind_process_contract_syscall(contract), Ok(0));

        let system_listing = list_path_text("/proc/system");
        assert!(
            system_listing.contains("schedulerepisodes\tFile"),
            "missing schedulerepisodes listing in:\n{system_listing}"
        );

        let scheduler = read_procfs_text("/proc/system/scheduler");
        for marker in [
            "current-tick:\t",
            "decision-tracing:\t",
            "running:\t",
            "cpu-summary:\t",
            "rebalance-ops=",
            "rebalance-migrations=",
            "last-rebalance=",
            "cpu\tindex=0\tapic-id=",
            "inferred-topology=",
            "queued-load=",
            "tokens=",
            "wait-ticks=",
            "lag-debt=",
            "dispatches=",
            "runtime-ticks=",
            "fairness-dispatch-total:\t",
            "fairness-runtime-total:\t",
            "fairness-runtime-imbalance:\t",
            "decision\ttick=",
        ] {
            assert!(
                scheduler.contains(marker),
                "missing marker {marker} in:\n{scheduler}"
            );
        }

        let episodes = read_procfs_text("/proc/system/schedulerepisodes");
        for marker in [
            "episodes:\t",
            "episode\tkind=affinity",
            "causal=cpu-mask-updated",
            "episode\tkind=dispatch",
            "causal=selected-next-runnable",
        ] {
            assert!(
                episodes.contains(marker),
                "missing marker {marker} in:\n{episodes}"
            );
        }
    }

    #[test]
    fn boot_scheduler_affinity_updates_procfs_and_rebalance_state() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-scheduler-affinity");

        let child = spawn_path_process_syscall(
            b"scheduler-worker".as_ptr() as usize,
            "scheduler-worker".len(),
            b"/bin/worker".as_ptr() as usize,
            "/bin/worker".len(),
        )
        .unwrap();
        let target_cpu = if boot_scheduler_cpu_count() >= 2 {
            1usize
        } else {
            0usize
        };
        let affinity_mask = 1usize << target_cpu;

        assert_eq!(set_process_affinity_syscall(child, 0), Err(Errno::Inval));
        assert_eq!(set_process_affinity_syscall(child, affinity_mask), Ok(0));

        let scheduler = read_procfs_text("/proc/system/scheduler");
        assert!(
            scheduler.contains("agent=AffinityAgent"),
            "missing affinity agent in:\n{scheduler}"
        );
        assert!(
            scheduler.contains(&format!(
                "meaning=affinity cpu-mask=0x{:x} assigned-cpu={target_cpu}",
                affinity_mask
            )),
            "missing affinity meaning in:\n{scheduler}"
        );
        let expected_migrations = u64::from(target_cpu != boot_scheduler_default_cpu(child as u64));
        assert!(
            scheduler.contains(&format!("rebalance-migrations={expected_migrations}")),
            "missing migration evidence in:\n{scheduler}"
        );
        assert!(
            scheduler.contains(&format!("last-rebalance={expected_migrations}")),
            "missing last rebalance evidence in:\n{scheduler}"
        );
        assert!(
            scheduler.contains(&format!(
                "cpu-queue\tindex={target_cpu}\tclass=interactive\tcount=1\ttids=[{child}]"
            )),
            "missing migrated queue placement in:\n{scheduler}"
        );

        let episodes = read_procfs_text("/proc/system/schedulerepisodes");
        assert!(
            episodes.contains(&format!(
                "episode\tkind=affinity\ttick=0\tpid={child}\ttid={child}\tclass={}\tbudget={affinity_mask}\tcausal=cpu-mask-updated",
                NativeSchedulerClass::Interactive as u32
            )),
            "missing affinity episode in:\n{episodes}"
        );
    }

    #[test]
    fn boot_scheduler_rebalances_spawn_pressure_without_explicit_affinity() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-scheduler-balance");
        if boot_scheduler_cpu_count() < 2 {
            return;
        }

        let first = spawn_path_process_syscall(
            b"scheduler-a".as_ptr() as usize,
            "scheduler-a".len(),
            b"/bin/worker".as_ptr() as usize,
            "/bin/worker".len(),
        )
        .unwrap();
        let second = spawn_path_process_syscall(
            b"scheduler-b".as_ptr() as usize,
            "scheduler-b".len(),
            b"/bin/worker".as_ptr() as usize,
            "/bin/worker".len(),
        )
        .unwrap();

        let scheduler = read_procfs_text("/proc/system/scheduler");
        assert!(
            scheduler.contains("agent=RebalanceAgent"),
            "missing rebalance agent in:\n{scheduler}"
        );
        assert!(
            scheduler.contains("rebalance-migrations=1"),
            "missing rebalance migration evidence in:\n{scheduler}"
        );
        assert!(
            scheduler.contains("last-rebalance=1"),
            "missing last rebalance evidence in:\n{scheduler}"
        );
        assert!(
            scheduler.contains(&format!(
                "cpu-queue\tindex=1\tclass=interactive\tcount=1\ttids=[{second}]"
            )) || scheduler.contains(&format!(
                "cpu-queue\tindex=1\tclass=interactive\tcount=1\ttids=[{first}]"
            )),
            "missing queued migration placement in:\n{scheduler}"
        );

        let episodes = read_procfs_text("/proc/system/schedulerepisodes");
        assert!(
            episodes.contains("episode\tkind=rebalance"),
            "missing rebalance episode in:\n{episodes}"
        );
        assert!(
            episodes.contains("causal=queued-moved"),
            "missing rebalance causal marker in:\n{episodes}"
        );
    }

    #[test]
    fn duplicate_close_and_poll_keep_stdio_subset_observable() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        let dup_fd = table.duplicate(1).unwrap();
        assert!(dup_fd >= 3);
        assert_eq!(table.poll(dup_fd, POLLOUT).unwrap(), POLLOUT as usize);
        table.close(dup_fd).unwrap();
        assert_eq!(table.poll(dup_fd, POLLOUT), Err(Errno::Badf));
    }

    #[test]
    fn duplicate_shares_file_description_state_but_keeps_cloexec_per_handle() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        boot_vfs_create("/shared.txt", BootNodeKind::File).unwrap();
        let fd = table.open_path("/shared.txt").unwrap();
        let dup_fd = table.duplicate(fd).unwrap();

        let description_id = table.descriptor_state(fd).unwrap().description_id;
        table.description_mut(description_id).unwrap().offset = 5;
        assert_eq!(table.descriptor(dup_fd).unwrap().offset, 5);

        assert_eq!(table.fcntl(fd, 2 | (1 << 8)).unwrap(), 1);
        assert_eq!(table.fcntl(dup_fd, 0).unwrap(), 1);

        assert_eq!(table.fcntl(fd, 3 | (1 << 8)).unwrap(), 2);
        assert_eq!(table.fcntl(dup_fd, 1).unwrap(), 0);

        assert_eq!(table.fcntl(fd, 5 | (0x44 << 8)).unwrap(), 0x44);
        assert_eq!(table.fcntl(dup_fd, 6 | (0x44 << 8)).unwrap(), 0x44);
        assert_eq!(table.fcntl(fd, 4).unwrap(), 0);

        table.close(fd).unwrap();
        let duplicate_snapshot = table.descriptor(dup_fd).unwrap();
        assert_eq!(duplicate_snapshot.offset, 5);
        assert!(duplicate_snapshot.nonblock);
        assert!(!duplicate_snapshot.cloexec);
    }

    #[test]
    fn boot_vfs_seek_supports_set_cur_end_and_refuses_negative_offsets() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        boot_vfs_create("/seek.txt", BootNodeKind::File).unwrap();
        let fd = DESCRIPTORS
            .with_mut(|descriptors| descriptors.open_path("/seek.txt"))
            .unwrap();
        let data = b"abcdef";
        assert_eq!(write_syscall(fd, data.as_ptr(), data.len()), Ok(data.len()));
        assert_eq!(seek_syscall(fd, 0, SeekWhence::Set as u32), Ok(0));
        assert_eq!(seek_syscall(fd, 2, SeekWhence::Set as u32), Ok(2));
        assert_eq!(seek_syscall(fd, 1, SeekWhence::Cur as u32), Ok(3));
        assert_eq!(seek_syscall(fd, -1, SeekWhence::End as u32), Ok(5));

        let mut byte = [0u8; 1];
        assert_eq!(read_syscall(fd, byte.as_mut_ptr(), byte.len()), Ok(1));
        assert_eq!(byte[0], b'f');
        assert_eq!(
            seek_syscall(fd, -7, SeekWhence::End as u32),
            Err(Errno::Inval)
        );
    }

    #[test]
    fn boot_vfs_truncate_shrinks_and_extends_with_zero_fill() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        boot_vfs_create("/truncate.txt", BootNodeKind::File).unwrap();
        let fd = DESCRIPTORS
            .with_mut(|descriptors| descriptors.open_path("/truncate.txt"))
            .unwrap();
        assert_eq!(write_syscall(fd, b"abcdef".as_ptr(), 6), Ok(6));
        assert_eq!(
            truncate_path_syscall("/truncate.txt".as_ptr() as usize, "/truncate.txt".len(), 3),
            Ok(0)
        );
        assert_eq!(boot_vfs_file_size("/truncate.txt"), Ok(3));
        assert_eq!(seek_syscall(fd, 0, SeekWhence::Set as u32), Ok(0));
        let mut small = [0u8; 8];
        assert_eq!(read_syscall(fd, small.as_mut_ptr(), small.len()), Ok(3));
        assert_eq!(&small[..3], b"abc");

        assert_eq!(
            truncate_path_syscall("/truncate.txt".as_ptr() as usize, "/truncate.txt".len(), 8),
            Ok(0)
        );
        assert_eq!(boot_vfs_file_size("/truncate.txt"), Ok(8));
        assert_eq!(seek_syscall(fd, 0, SeekWhence::Set as u32), Ok(0));
        let mut extended = [0u8; 8];
        assert_eq!(
            read_syscall(fd, extended.as_mut_ptr(), extended.len()),
            Ok(8)
        );
        assert_eq!(&extended[..3], b"abc");
        assert_eq!(&extended[3..], &[0, 0, 0, 0, 0]);
    }

    #[test]
    fn boot_vfs_hardlink_shares_inode_and_mutations() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        boot_vfs_create("/origin.txt", BootNodeKind::File).unwrap();
        let origin_fd = DESCRIPTORS
            .with_mut(|descriptors| descriptors.open_path("/origin.txt"))
            .unwrap();
        assert_eq!(write_syscall(origin_fd, b"alpha".as_ptr(), 5), Ok(5));
        assert_eq!(
            link_path_syscall(
                "/origin.txt".as_ptr() as usize,
                "/origin.txt".len(),
                "/alias.txt".as_ptr() as usize,
                "/alias.txt".len()
            ),
            Ok(0)
        );
        let origin = boot_vfs_stat("/origin.txt").unwrap();
        let alias = boot_vfs_stat("/alias.txt").unwrap();
        assert_eq!(origin.inode, alias.inode);
        assert_eq!(origin.link_count, 2);
        assert_eq!(alias.link_count, 2);

        let alias_fd = DESCRIPTORS
            .with_mut(|descriptors| descriptors.open_path("/alias.txt"))
            .unwrap();
        assert_eq!(seek_syscall(alias_fd, 5, SeekWhence::Set as u32), Ok(5));
        assert_eq!(write_syscall(alias_fd, b"-beta".as_ptr(), 5), Ok(5));

        assert_eq!(seek_syscall(origin_fd, 0, SeekWhence::Set as u32), Ok(0));
        let mut data = [0u8; 16];
        assert_eq!(
            read_syscall(origin_fd, data.as_mut_ptr(), data.len()),
            Ok(10)
        );
        assert_eq!(&data[..10], b"alpha-beta");

        assert_eq!(
            truncate_path_syscall("/alias.txt".as_ptr() as usize, "/alias.txt".len(), 4),
            Ok(0)
        );
        assert_eq!(seek_syscall(origin_fd, 0, SeekWhence::Set as u32), Ok(0));
        let mut small = [0u8; 8];
        assert_eq!(
            read_syscall(origin_fd, small.as_mut_ptr(), small.len()),
            Ok(4)
        );
        assert_eq!(&small[..4], b"alph");

        assert_eq!(
            unlink_path_syscall("/alias.txt".as_ptr() as usize, "/alias.txt".len()),
            Ok(0)
        );
        assert!(boot_vfs_stat("/alias.txt").is_none());
        let origin_after_unlink = boot_vfs_stat("/origin.txt").unwrap();
        assert_eq!(origin_after_unlink.link_count, 1);
    }

    #[test]
    fn boot_vfs_unlink_keeps_open_descriptor_alive_as_deleted_object() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        boot_vfs_create("/live.txt", BootNodeKind::File).unwrap();
        let fd = DESCRIPTORS
            .with_mut(|descriptors| descriptors.open_path("/live.txt"))
            .unwrap();
        assert_eq!(write_syscall(fd, b"alive".as_ptr(), 5), Ok(5));
        assert_eq!(
            unlink_path_syscall("/live.txt".as_ptr() as usize, "/live.txt".len()),
            Ok(0)
        );
        assert!(boot_vfs_stat("/live.txt").is_none());
        assert_eq!(
            open_path_syscall("/live.txt".as_ptr() as usize, "/live.txt".len()),
            Err(Errno::NoEnt)
        );
        assert_eq!(seek_syscall(fd, 0, SeekWhence::Set as u32), Ok(0));
        let mut bytes = [0u8; 8];
        assert_eq!(read_syscall(fd, bytes.as_mut_ptr(), bytes.len()), Ok(5));
        assert_eq!(&bytes[..5], b"alive");
        let fdinfo = boot_procfs_fdinfo(1, fd as u64).unwrap();
        assert!(fdinfo.contains("(deleted)"));
        assert_eq!(close_syscall(fd), Ok(0));
    }

    #[test]
    fn unlinking_network_socket_path_removes_runtime_socket_registration() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        assert_eq!(
            mksock_path_syscall("/run/net0.sock".as_ptr() as usize, "/run/net0.sock".len()),
            Err(Errno::NoEnt)
        );
        boot_vfs_create("/run", BootNodeKind::Directory).unwrap();
        assert_eq!(
            mksock_path_syscall("/run/net0.sock".as_ptr() as usize, "/run/net0.sock".len()),
            Ok(0)
        );
        assert_eq!(
            configure_network_interface_ipv4_syscall(
                NETWORK_DEVICE_PATH.as_ptr() as usize,
                NETWORK_DEVICE_PATH.len(),
                &NativeNetworkInterfaceConfig {
                    addr: [10, 1, 0, 2],
                    netmask: [255, 255, 255, 0],
                    gateway: [10, 1, 0, 1],
                } as *const _
            ),
            Ok(0)
        );
        assert_eq!(
            bind_udp_socket_syscall(
                "/run/net0.sock".as_ptr() as usize,
                "/run/net0.sock".len(),
                NETWORK_DEVICE_PATH.as_ptr() as usize,
                NETWORK_DEVICE_PATH.len(),
                &NativeUdpBindConfig {
                    remote_ipv4: [0, 0, 0, 0],
                    local_port: 4000,
                    remote_port: 0,
                } as *const _
            ),
            Ok(0)
        );

        let mut interface: NativeNetworkInterfaceRecord = unsafe { core::mem::zeroed() };
        assert_eq!(
            inspect_network_interface_syscall(
                NETWORK_DEVICE_PATH.as_ptr() as usize,
                NETWORK_DEVICE_PATH.len(),
                &mut interface as *mut _
            ),
            Ok(0)
        );
        assert_eq!(interface.attached_socket_count, 1);

        let mut socket: NativeNetworkSocketRecord = unsafe { core::mem::zeroed() };
        assert_eq!(
            inspect_network_socket_syscall(
                "/run/net0.sock".as_ptr() as usize,
                "/run/net0.sock".len(),
                &mut socket as *mut _
            ),
            Ok(0)
        );
        assert_eq!(socket.local_port, 4000);

        assert_eq!(
            unlink_path_syscall("/run/net0.sock".as_ptr() as usize, "/run/net0.sock".len()),
            Ok(0)
        );
        assert_eq!(
            inspect_network_socket_syscall(
                "/run/net0.sock".as_ptr() as usize,
                "/run/net0.sock".len(),
                &mut socket as *mut _
            ),
            Err(Errno::NoEnt)
        );
        assert_eq!(
            inspect_network_interface_syscall(
                NETWORK_DEVICE_PATH.as_ptr() as usize,
                NETWORK_DEVICE_PATH.len(),
                &mut interface as *mut _
            ),
            Ok(0)
        );
        assert_eq!(interface.attached_socket_count, 0);
    }

    #[test]
    fn boot_vfs_rename_replace_preserves_old_target_fd_and_rebinds_path() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        boot_vfs_create("/replace-src.txt", BootNodeKind::File).unwrap();
        boot_vfs_create("/replace-dst.txt", BootNodeKind::File).unwrap();
        let src_fd = DESCRIPTORS
            .with_mut(|descriptors| descriptors.open_path("/replace-src.txt"))
            .unwrap();
        let dst_fd = DESCRIPTORS
            .with_mut(|descriptors| descriptors.open_path("/replace-dst.txt"))
            .unwrap();
        assert_eq!(write_syscall(src_fd, b"source".as_ptr(), 6), Ok(6));
        assert_eq!(write_syscall(dst_fd, b"target".as_ptr(), 6), Ok(6));
        assert_eq!(
            rename_path_syscall(
                "/replace-src.txt".as_ptr() as usize,
                "/replace-src.txt".len(),
                "/replace-dst.txt".as_ptr() as usize,
                "/replace-dst.txt".len(),
            ),
            Ok(0)
        );
        assert!(boot_vfs_stat("/replace-src.txt").is_none());
        let rebound = boot_vfs_stat("/replace-dst.txt").unwrap();
        assert_eq!(rebound.link_count, 1);

        assert_eq!(seek_syscall(dst_fd, 0, SeekWhence::Set as u32), Ok(0));
        let mut old = [0u8; 8];
        assert_eq!(read_syscall(dst_fd, old.as_mut_ptr(), old.len()), Ok(6));
        assert_eq!(&old[..6], b"target");
        assert!(
            boot_procfs_fdinfo(1, dst_fd as u64)
                .unwrap()
                .contains("(deleted)")
        );

        let rebound_fd = open_path_syscall(
            "/replace-dst.txt".as_ptr() as usize,
            "/replace-dst.txt".len(),
        )
        .unwrap();
        assert_eq!(seek_syscall(rebound_fd, 0, SeekWhence::Set as u32), Ok(0));
        let mut new = [0u8; 8];
        assert_eq!(read_syscall(rebound_fd, new.as_mut_ptr(), new.len()), Ok(6));
        assert_eq!(&new[..6], b"source");
    }

    #[test]
    fn boot_vfs_mount_layering_prefers_topmost_node_and_restores_lower_layer() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut vfs = BootVfs::new();
        let lower_entries = vec![
            crate::virtio_blk_boot::StorageSnapshotEntry {
                name: String::from("config"),
                kind: crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_DIRECTORY,
                bytes: Vec::new(),
            },
            crate::virtio_blk_boot::StorageSnapshotEntry {
                name: String::from("config/base.txt"),
                kind: crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_FILE,
                bytes: b"lower".to_vec(),
            },
        ];
        let upper_entries = vec![crate::virtio_blk_boot::StorageSnapshotEntry {
            name: String::from("base.txt"),
            kind: crate::virtio_blk_boot::STORAGE_SNAPSHOT_ENTRY_FILE,
            bytes: b"upper".to_vec(),
        }];

        let (loaded_lower, created_root) =
            apply_persist_entries(&mut vfs, 1, "/persist", &lower_entries).unwrap();
        assert_eq!(loaded_lower, 2);
        assert!(created_root);
        let (loaded_upper, created_upper_root) =
            apply_persist_entries(&mut vfs, 2, "/persist/config", &upper_entries).unwrap();
        assert_eq!(loaded_upper, 1);
        assert!(!created_upper_root);

        let top_index = vfs
            .resolve_node_index("/persist/config/base.txt", true)
            .unwrap();
        assert_eq!(vfs.nodes[top_index].bytes, b"upper");

        let parent_entries = collect_persist_entries(&vfs, 1, "/persist").unwrap();
        assert_eq!(parent_entries.len(), 2);
        assert!(
            parent_entries
                .iter()
                .any(|entry| entry.name == "config/base.txt")
        );
        assert_eq!(
            parent_entries
                .iter()
                .find(|entry| entry.name == "config/base.txt")
                .unwrap()
                .bytes,
            b"lower"
        );

        vfs.nodes.retain(|node| node.mount_id != Some(2));
        vfs.invalidate_caches();
        let restored_index = vfs
            .resolve_node_index("/persist/config/base.txt", true)
            .unwrap();
        assert_eq!(vfs.nodes[restored_index].bytes, b"lower");
        let listing = vfs.list_directory_text("/persist/config").unwrap();
        assert!(listing.contains("base.txt\tFile"));
    }

    #[test]
    fn boot_vfs_nested_mount_registry_blocks_parent_unmount_until_child_is_removed() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        STORAGE_MOUNT.with_mut(|registry| {
            registry.next_id = 3;
            registry.mounts.push(StorageMountRecord {
                id: 1,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist"),
                parent_mount_id: 0,
                peer_group: 0,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Private as u32,

                entry_count: 0,
                created_mount_root: true,
            });
            registry.mounts.push(StorageMountRecord {
                id: 2,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist/addon"),
                parent_mount_id: 1,
                peer_group: 0,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Private as u32,

                entry_count: 0,
                created_mount_root: false,
            });
        });

        assert!(storage_mount_has_nested_child("/persist", 1));
        assert!(!storage_mount_has_nested_child("/persist/addon", 2));

        let result = unmount_storage_volume_syscall("/persist".as_ptr() as usize, "/persist".len());
        assert_eq!(result, Err(Errno::Busy));

        STORAGE_MOUNT.with_mut(|registry| registry.mounts.retain(|record| record.id != 2));
        assert!(!storage_mount_has_nested_child("/persist", 1));
    }

    #[test]
    fn boot_vfs_shared_mount_propagation_plans_child_mount_clones_for_peer_mounts() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        STORAGE_MOUNT.with_mut(|registry| {
            registry.next_id = 5;
            registry.mounts.push(StorageMountRecord {
                id: 1,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist"),
                parent_mount_id: 0,
                peer_group: 1,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: true,
            });
            registry.mounts.push(StorageMountRecord {
                id: 2,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/mirror"),
                parent_mount_id: 0,
                peer_group: 1,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: true,
            });
            registry.mounts.push(StorageMountRecord {
                id: 3,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/slave"),
                parent_mount_id: 0,
                peer_group: 0,
                master_group: 1,
                propagation_mode: NativeMountPropagationMode::Slave as u32,

                entry_count: 1,
                created_mount_root: true,
            });
        });

        let mut persist_mount = NativeMountRecord {
            id: 0,
            parent_mount_id: 0,
            peer_group: 0,
            master_group: 0,
            layer: 0,
            entry_count: 0,
            propagation_mode: 0,
            created_mount_root: 0,
        };
        assert_eq!(
            inspect_mount_syscall(
                "/persist".as_ptr() as usize,
                "/persist".len(),
                &mut persist_mount as *mut _
            ),
            Ok(0)
        );
        assert_eq!(
            persist_mount.propagation_mode,
            NativeMountPropagationMode::Shared as u32
        );
        let clones = storage_mount_propagation_clones(
            &storage_mount_by_path("/persist").unwrap(),
            "/persist/addon",
        );
        assert_eq!(clones.len(), 2);
        assert!(clones.iter().any(|(path, mode)| {
            path == "/mirror/addon" && *mode == NativeMountPropagationMode::Shared
        }));
        assert!(clones.iter().any(|(path, mode)| {
            path == "/slave/addon" && *mode == NativeMountPropagationMode::Slave
        }));
    }

    #[test]
    fn boot_vfs_set_shared_mount_propagation_clones_existing_descendants_to_peer_tree() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        BOOT_VFS.with_mut(|vfs| {
            vfs.create_with_mount("/persist", BootNodeKind::Directory, Some(1))
                .unwrap();
            vfs.create_with_mount("/mirror", BootNodeKind::Directory, Some(2))
                .unwrap();
            vfs.create_with_mount("/persist/addon", BootNodeKind::Directory, Some(3))
                .unwrap();
            vfs.create_with_mount("/persist/addon/file.txt", BootNodeKind::File, Some(3))
                .unwrap();
            let index = vfs
                .resolve_node_index("/persist/addon/file.txt", true)
                .unwrap();
            vfs.nodes[index].bytes = b"shared-tree".to_vec();
            vfs.invalidate_caches();
        });
        STORAGE_MOUNT.with_mut(|registry| {
            registry.next_id = 4;
            registry.mounts.push(StorageMountRecord {
                id: 1,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist"),
                parent_mount_id: 0,
                peer_group: 0,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Private as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 2,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/mirror"),
                parent_mount_id: 0,
                peer_group: 2,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 3,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist/addon"),
                parent_mount_id: 1,
                peer_group: 0,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Private as u32,

                entry_count: 1,
                created_mount_root: false,
            });
        });

        assert_eq!(
            set_mount_propagation_syscall(
                "/persist".as_ptr() as usize,
                "/persist".len(),
                NativeMountPropagationMode::Shared as u32,
            ),
            Ok(0)
        );

        let source_child = storage_mount_by_path("/persist/addon").unwrap();
        let clone_child = storage_mount_by_path("/mirror/addon").unwrap();
        assert_eq!(
            NativeMountPropagationMode::from_raw(source_child.propagation_mode),
            Some(NativeMountPropagationMode::Shared)
        );
        assert_eq!(source_child.peer_group, 3);
        assert_eq!(
            NativeMountPropagationMode::from_raw(clone_child.propagation_mode),
            Some(NativeMountPropagationMode::Shared)
        );
        assert_eq!(clone_child.peer_group, source_child.peer_group);
        BOOT_VFS.with_mut(|vfs| {
            let index = vfs
                .resolve_node_index("/mirror/addon/file.txt", true)
                .unwrap();
            assert_eq!(vfs.nodes[index].bytes, b"shared-tree");
        });
    }

    #[test]
    fn boot_vfs_set_slave_mount_propagation_clones_existing_descendants_from_shared_peer() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        BOOT_VFS.with_mut(|vfs| {
            vfs.create_with_mount("/persist", BootNodeKind::Directory, Some(1))
                .unwrap();
            vfs.create_with_mount("/mirror", BootNodeKind::Directory, Some(2))
                .unwrap();
            vfs.create_with_mount("/persist/addon", BootNodeKind::Directory, Some(3))
                .unwrap();
            vfs.create_with_mount("/persist/addon/file.txt", BootNodeKind::File, Some(3))
                .unwrap();
            let index = vfs
                .resolve_node_index("/persist/addon/file.txt", true)
                .unwrap();
            vfs.nodes[index].bytes = b"slave-tree".to_vec();
            vfs.invalidate_caches();
        });
        STORAGE_MOUNT.with_mut(|registry| {
            registry.next_id = 4;
            registry.mounts.push(StorageMountRecord {
                id: 1,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist"),
                parent_mount_id: 0,
                peer_group: 1,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 2,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/mirror"),
                parent_mount_id: 0,
                peer_group: 0,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Private as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 3,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist/addon"),
                parent_mount_id: 1,
                peer_group: 3,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
        });

        assert_eq!(
            set_mount_propagation_syscall(
                "/mirror".as_ptr() as usize,
                "/mirror".len(),
                NativeMountPropagationMode::Slave as u32,
            ),
            Ok(0)
        );

        let mirror = storage_mount_by_path("/mirror").unwrap();
        let clone_child = storage_mount_by_path("/mirror/addon").unwrap();
        assert_eq!(
            NativeMountPropagationMode::from_raw(mirror.propagation_mode),
            Some(NativeMountPropagationMode::Slave)
        );
        assert_eq!(mirror.master_group, 1);
        assert_eq!(
            NativeMountPropagationMode::from_raw(clone_child.propagation_mode),
            Some(NativeMountPropagationMode::Slave)
        );
        assert_eq!(clone_child.master_group, 3);
        BOOT_VFS.with_mut(|vfs| {
            let index = vfs
                .resolve_node_index("/mirror/addon/file.txt", true)
                .unwrap();
            assert_eq!(vfs.nodes[index].bytes, b"slave-tree");
        });
    }

    #[test]
    fn boot_vfs_set_private_mount_propagation_privatizes_existing_descendant_subtree() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        STORAGE_MOUNT.with_mut(|registry| {
            registry.next_id = 4;
            registry.mounts.push(StorageMountRecord {
                id: 1,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist"),
                parent_mount_id: 0,
                peer_group: 1,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 2,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist/addon"),
                parent_mount_id: 1,
                peer_group: 2,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 3,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist/addon/nested"),
                parent_mount_id: 2,
                peer_group: 3,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
        });

        assert_eq!(
            set_mount_propagation_syscall(
                "/persist".as_ptr() as usize,
                "/persist".len(),
                NativeMountPropagationMode::Private as u32,
            ),
            Ok(0)
        );

        let root = storage_mount_by_path("/persist").unwrap();
        let child = storage_mount_by_path("/persist/addon").unwrap();
        let nested = storage_mount_by_path("/persist/addon/nested").unwrap();
        assert_eq!(
            NativeMountPropagationMode::from_raw(root.propagation_mode),
            Some(NativeMountPropagationMode::Private)
        );
        assert_eq!(root.peer_group, 0);
        assert_eq!(root.master_group, 0);
        assert_eq!(
            NativeMountPropagationMode::from_raw(child.propagation_mode),
            Some(NativeMountPropagationMode::Private)
        );
        assert_eq!(child.peer_group, 0);
        assert_eq!(child.master_group, 0);
        assert_eq!(
            NativeMountPropagationMode::from_raw(nested.propagation_mode),
            Some(NativeMountPropagationMode::Private)
        );
        assert_eq!(nested.peer_group, 0);
        assert_eq!(nested.master_group, 0);
    }

    #[test]
    fn boot_vfs_set_slave_mount_propagation_rebinds_existing_descendants_to_source_groups() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        STORAGE_MOUNT.with_mut(|registry| {
            registry.next_id = 5;
            registry.mounts.push(StorageMountRecord {
                id: 1,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist"),
                parent_mount_id: 0,
                peer_group: 1,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 2,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist/addon"),
                parent_mount_id: 1,
                peer_group: 22,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 3,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/mirror"),
                parent_mount_id: 0,
                peer_group: 0,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Private as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 4,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/mirror/addon"),
                parent_mount_id: 3,
                peer_group: 44,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
        });

        assert_eq!(
            set_mount_propagation_syscall(
                "/mirror".as_ptr() as usize,
                "/mirror".len(),
                NativeMountPropagationMode::Slave as u32,
            ),
            Ok(0)
        );

        let root = storage_mount_by_path("/mirror").unwrap();
        let child = storage_mount_by_path("/mirror/addon").unwrap();
        assert_eq!(
            NativeMountPropagationMode::from_raw(root.propagation_mode),
            Some(NativeMountPropagationMode::Slave)
        );
        assert_eq!(root.master_group, 1);
        assert_eq!(
            NativeMountPropagationMode::from_raw(child.propagation_mode),
            Some(NativeMountPropagationMode::Slave)
        );
        assert_eq!(child.peer_group, 0);
        assert_eq!(child.master_group, 22);
    }

    #[test]
    fn boot_vfs_recursive_unmount_collects_propagated_descendant_subtrees() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        BOOT_VFS.with_mut(|vfs| {
            vfs.create_with_mount("/persist", BootNodeKind::Directory, Some(1))
                .unwrap();
            vfs.create_with_mount("/persist/addon", BootNodeKind::Directory, Some(2))
                .unwrap();
            vfs.create_with_mount("/mirror", BootNodeKind::Directory, Some(3))
                .unwrap();
            vfs.create_with_mount("/mirror/addon", BootNodeKind::Directory, Some(4))
                .unwrap();
            vfs.create_with_mount("/slave", BootNodeKind::Directory, Some(5))
                .unwrap();
            vfs.create_with_mount("/slave/addon", BootNodeKind::Directory, Some(6))
                .unwrap();
            vfs.invalidate_caches();
        });
        STORAGE_MOUNT.with_mut(|registry| {
            registry.next_id = 7;
            registry.mounts.push(StorageMountRecord {
                id: 1,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist"),
                parent_mount_id: 0,
                peer_group: 1,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 2,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/persist/addon"),
                parent_mount_id: 1,
                peer_group: 2,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 3,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/mirror"),
                parent_mount_id: 0,
                peer_group: 1,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 4,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/mirror/addon"),
                parent_mount_id: 3,
                peer_group: 2,
                master_group: 0,
                propagation_mode: NativeMountPropagationMode::Shared as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 5,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/slave"),
                parent_mount_id: 0,
                peer_group: 0,
                master_group: 1,
                propagation_mode: NativeMountPropagationMode::Slave as u32,

                entry_count: 1,
                created_mount_root: false,
            });
            registry.mounts.push(StorageMountRecord {
                id: 6,
                device_path: String::from("/dev/storage0"),
                mount_path: String::from("/slave/addon"),
                parent_mount_id: 5,
                peer_group: 0,
                master_group: 2,
                propagation_mode: NativeMountPropagationMode::Slave as u32,

                entry_count: 1,
                created_mount_root: false,
            });
        });

        let root = storage_mount_by_path("/persist").unwrap();
        let active_ids = storage_mount_recursive_unmount_ids(&root);
        assert_eq!(active_ids, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn boot_vfs_cache_pressure_evicts_old_entries_and_repopulates_cleanly() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut vfs = BootVfs::new();
        vfs.create("/cache".as_ref(), BootNodeKind::Directory)
            .unwrap();
        for index in 0..96usize {
            let path = format!("/cache/file-{index:03}.txt");
            vfs.create(&path, BootNodeKind::File).unwrap();
            let node_index = vfs.resolve_node_index(&path, true).unwrap();
            vfs.nodes[node_index].bytes = vec![index as u8; 600];
            let _ = vfs.resolve_node_index(&path, true).unwrap();
            let _ = vfs.stat(&path, true).unwrap();
            let _ = vfs.list_directory_text("/cache").unwrap();
            let _ = vfs.page_bytes(node_index, 0).unwrap();
            let _ = vfs
                .page_bytes(node_index, BootVfs::PAGE_CACHE_GRANULE)
                .unwrap();
        }

        assert!(vfs.lookup_cache.len() <= BootVfs::LOOKUP_CACHE_LIMIT);
        assert!(vfs.stat_cache.len() <= BootVfs::STAT_CACHE_LIMIT);
        assert!(vfs.directory_cache.len() <= BootVfs::DIRECTORY_CACHE_LIMIT);
        assert!(vfs.page_cache.len() <= BootVfs::PAGE_CACHE_LIMIT);

        let first = "/cache/file-000.txt";
        let first_index = vfs.resolve_node_index(first, true).unwrap();
        let stat = vfs.stat(first, true).unwrap();
        assert_eq!(stat.size, 600);
        let listing = vfs.list_directory_text("/cache").unwrap();
        assert!(listing.contains("file-000.txt\tFile"));
        let page = vfs.page_bytes(first_index, 0).unwrap();
        assert_eq!(page.len(), BootVfs::PAGE_CACHE_GRANULE);
        assert_eq!(page[0], 0);
    }

    #[test]
    fn fcntl_tracks_nonblock_and_cloexec_bits_separately() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        assert_eq!(table.fcntl(1, 0).unwrap(), 0);
        assert_eq!(table.fcntl(1, 2 | (1 << 8)).unwrap(), 1);
        assert_eq!(table.fcntl(1, 0).unwrap(), 1);
        assert_eq!(table.fcntl(1, 3 | (1 << 8)).unwrap(), 2);
        assert_eq!(table.fcntl(1, 1).unwrap(), 2);
    }

    #[test]
    fn resource_event_queues_observe_queue_handoff_revoke_and_recovery() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let domain = create_domain_syscall(0, b"eventing".as_ptr() as usize, 8).unwrap();
        let resource = create_resource_syscall(
            domain,
            NativeResourceKind::Device as u32,
            b"queue0".as_ptr() as usize,
            6,
        )
        .unwrap();
        let primary = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Display as u32,
            b"primary".as_ptr() as usize,
            7,
        )
        .unwrap();
        let mirror = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Display as u32,
            b"mirror".as_ptr() as usize,
            6,
        )
        .unwrap();

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        assert_eq!(fcntl_syscall(queue_fd, 2 | (1 << 8)), Ok(1));

        let watch = NativeResourceEventWatchConfig {
            token: 515,
            poll_events: POLLPRI,
            claimed: 0,
            queued: 1,
            canceled: 0,
            released: 0,
            handed_off: 1,
            revoked: 1,
        };
        assert_eq!(
            watch_resource_events_syscall(queue_fd, resource, &watch),
            Ok(0)
        );

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
        assert_eq!(
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()),
            Err(Errno::Again)
        );

        let mut claim = NativeResourceClaimRecord {
            resource: 0,
            holder_contract: 0,
            acquire_count: 0,
            position: 0,
            queued: 0,
            reserved: 0,
        };
        assert_eq!(claim_resource_syscall(primary, &mut claim), Ok(0));
        assert_eq!(claim_resource_syscall(mirror, &mut claim), Ok(0));

        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert_eq!(count, 1);
        assert_eq!(events[0].token, 515);
        assert_eq!(events[0].events, POLLPRI);
        assert_eq!(
            events[0].source_kind,
            NativeEventSourceKind::Resource as u32
        );
        assert_eq!(events[0].source_arg0, resource as u64);
        assert_eq!(events[0].source_arg1, mirror as u64);
        assert_eq!(events[0].detail0, BootResourceEventKind::Queued as u32);

        let mut release = NativeResourceReleaseRecord {
            resource: 0,
            handoff_contract: 0,
            acquire_count: 0,
            handoff_count: 0,
            handed_off: 0,
            reserved: 0,
        };
        assert_eq!(
            release_claimed_resource_syscall(primary, &mut release),
            Ok(0)
        );
        assert_eq!(release.handoff_contract, mirror as u64);

        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert_eq!(count, 1);
        assert_eq!(events[0].source_arg0, resource as u64);
        assert_eq!(events[0].source_arg1, mirror as u64);
        assert_eq!(events[0].detail0, BootResourceEventKind::HandedOff as u32);

        assert_eq!(
            remove_resource_events_syscall(queue_fd, resource, 515),
            Ok(0)
        );
        assert_eq!(
            remove_resource_events_syscall(queue_fd, resource, 515),
            Err(Errno::NoEnt)
        );
        assert_eq!(
            set_contract_state_syscall(mirror, NativeContractState::Revoked as u32),
            Ok(0)
        );
        assert_eq!(
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()),
            Err(Errno::Again)
        );

        let mut info = NativeResourceRecord {
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
        assert_eq!(inspect_resource_syscall(resource, &mut info), Ok(0));
        assert_eq!(info.holder_contract, 0);
        assert_eq!(info.waiting_count, 0);
        assert_eq!(info.acquire_count, 2);
        assert_eq!(info.handoff_count, 1);

        assert_eq!(close_syscall(queue_fd), Ok(0));
        assert_eq!(poll_syscall(queue_fd, POLLPRI), Err(Errno::Badf));
    }

    #[test]
    fn duplicate_exhaustion_maps_to_eagain() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        for _ in 0..5 {
            table.duplicate(1).unwrap();
        }
        assert_eq!(table.duplicate(1), Err(Errno::Again));
    }

    #[test]
    fn invalid_fcntl_command_maps_to_einval() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        assert_eq!(table.fcntl(1, 0xff), Err(Errno::Inval));
    }

    #[test]
    fn stdin_is_not_writable() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let descriptor = DescriptorTable::new().descriptor(0).unwrap();
        assert_eq!(descriptor.target, DescriptorTarget::Stdin);
    }

    #[test]
    fn stdin_poll_reports_readable_when_serial_input_is_pending() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        crate::serial::inject_input(b"help\n");
        let table = DescriptorTable::new();
        assert_eq!(table.poll(0, POLLIN).unwrap(), POLLIN as usize);
        crate::serial::clear_input();
    }

    #[test]
    fn read_syscall_consumes_serial_input_from_stdin() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        crate::serial::inject_input(b"echo boot\r\n");
        let mut buffer = [0u8; 32];
        let read = read_syscall(0, buffer.as_mut_ptr(), buffer.len()).unwrap();
        assert_eq!(&buffer[..read], b"echo boot\n");
        assert_eq!(DescriptorTable::new().poll(0, POLLIN).unwrap(), 0);
        crate::serial::clear_input();
    }

    #[test]
    fn readv_and_writev_syscalls_span_multiple_iovecs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        crate::serial::inject_input(b"hello\n");
        let mut a = [0u8; 2];
        let mut b = [0u8; 8];
        let read_iovecs = [
            UserIoVec {
                base: a.as_mut_ptr() as usize,
                len: a.len(),
            },
            UserIoVec {
                base: b.as_mut_ptr() as usize,
                len: b.len(),
            },
        ];
        let read = readv_syscall(0, read_iovecs.as_ptr(), read_iovecs.len()).unwrap();
        assert_eq!(read, 6);
        assert_eq!(&a, b"he");
        assert_eq!(&b[..4], b"llo\n");

        let left = b"ng";
        let right = b"os\n";
        let write_iovecs = [
            UserIoVec {
                base: left.as_ptr() as usize,
                len: left.len(),
            },
            UserIoVec {
                base: right.as_ptr() as usize,
                len: right.len(),
            },
        ];
        assert_eq!(
            writev_syscall(1, write_iovecs.as_ptr(), write_iovecs.len()).unwrap(),
            5
        );
        crate::serial::clear_input();
    }

    #[test]
    fn nonblocking_stdin_returns_eagain_without_input() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        assert_eq!(table.fcntl(0, 2 | (1 << 8)).unwrap(), 1);
        DESCRIPTORS.with_mut(|descriptors| *descriptors = table);
        let mut buffer = [0u8; 8];
        assert_eq!(
            read_syscall(0, buffer.as_mut_ptr(), buffer.len()),
            Err(Errno::Again)
        );
        crate::serial::clear_input();
    }

    #[test]
    fn runtime_status_tracks_syscall_count_and_exit() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let frame = SyscallFrame::new(SYS_DUP, [1, 0, 0, 0, 0, 0]);
        let mut result = SyscallDispatchResult {
            raw_return: 0,
            disposition: 0,
            ..Default::default()
        };
        x86_64_syscall_dispatch(&frame, 0x401000, 0x7fff_0000, 0x202, &mut result);
        let status = user_runtime_status::snapshot();
        assert_eq!(status.syscall_count, 1);
        assert_eq!(status.last_syscall, SYS_DUP);
        assert!(!status.exited);

        let exit = SyscallFrame::new(SYS_EXIT, [7, 0, 0, 0, 0, 0]);
        x86_64_syscall_dispatch(&exit, 0x401100, 0x7fff_0000, 0x202, &mut result);
        let status = user_runtime_status::snapshot();
        assert!(status.exited);
        assert_eq!(status.exit_code, 7);
    }

    #[test]
    fn write_syscall_records_stdout_runtime_telemetry() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let payload = b"boot-runtime";
        let result = write_syscall(1, payload.as_ptr(), payload.len());
        assert_eq!(result, Ok(payload.len()));
        assert_eq!(crate::serial::take_output(), payload);
        assert!(crate::serial::take_error_output().is_empty());

        let status = user_runtime_status::snapshot();
        assert_eq!(status.stdout_write_count, 1);
        assert_eq!(status.stderr_write_count, 0);
        assert_eq!(status.bytes_written, payload.len() as u64);
        assert_eq!(status.last_write_fd, 1);
        assert_eq!(status.last_write_len, payload.len() as u64);
    }

    #[test]
    fn write_syscall_records_stderr_runtime_telemetry() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let payload = b"fault-path";
        let result = write_syscall(2, payload.as_ptr(), payload.len());
        assert_eq!(result, Ok(payload.len()));
        assert_eq!(crate::serial::take_output(), payload);
        assert_eq!(crate::serial::take_error_output(), payload);

        let status = user_runtime_status::snapshot();
        assert_eq!(status.stdout_write_count, 0);
        assert_eq!(status.stderr_write_count, 1);
        assert_eq!(status.bytes_written, payload.len() as u64);
        assert_eq!(status.last_write_fd, 2);
        assert_eq!(status.last_write_len, payload.len() as u64);
    }

    #[test]
    fn boot_gpu_device_and_driver_records_are_exposed_through_generic_inspect() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let payload =
            b"frame=boot-gfx-001\nsource-api=directx12\ntranslation=compat-to-vulkan\npresent=0,0,1280,720";
        let device_fd =
            open_path_syscall(GPU_DEVICE_PATH.as_ptr() as usize, GPU_DEVICE_PATH.len()).unwrap();
        let driver_fd =
            open_path_syscall(GPU_DRIVER_PATH.as_ptr() as usize, GPU_DRIVER_PATH.len()).unwrap();
        assert_eq!(
            write_syscall(device_fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mut device = NativeDeviceRecord {
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
        let mut driver = NativeDriverRecord {
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
        assert_eq!(
            inspect_device_syscall(
                GPU_DEVICE_PATH.as_ptr() as usize,
                GPU_DEVICE_PATH.len(),
                &mut device as *mut _
            ),
            Ok(0)
        );
        assert_eq!(
            inspect_driver_syscall(
                GPU_DRIVER_PATH.as_ptr() as usize,
                GPU_DRIVER_PATH.len(),
                &mut driver as *mut _
            ),
            Ok(0)
        );
        assert_eq!(device.class, 3);
        assert_eq!(device.submitted_requests, 1);
        assert_eq!(device.queue_depth, 1);
        assert_eq!(driver.bound_device_count, 1);
        assert_eq!(driver.queued_requests, 1);
        assert_eq!(poll_syscall(device_fd, POLLOUT), Ok(POLLOUT as usize));
        assert_eq!(poll_syscall(driver_fd, POLLIN), Ok(POLLIN as usize));
    }

    #[test]
    fn boot_gpu_scanout_and_display_syscalls_report_presented_metadata() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let payload =
            b"frame=boot-gfx-002\nsource-api=opengl\ntranslation=compat-to-vulkan\npresent=0,0,800,600";
        let device_fd =
            open_path_syscall(GPU_DEVICE_PATH.as_ptr() as usize, GPU_DEVICE_PATH.len()).unwrap();
        let driver_fd =
            open_path_syscall(GPU_DRIVER_PATH.as_ptr() as usize, GPU_DRIVER_PATH.len()).unwrap();
        assert_eq!(
            write_syscall(device_fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mut request = [0u8; 256];
        let request_len = read_syscall(driver_fd, request.as_mut_ptr(), request.len()).unwrap();
        assert_eq!(&request[..request_len], payload);
        assert_eq!(
            write_syscall(driver_fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mut display = NativeGpuDisplayRecord {
            present: 0,
            active_pipes: 0,
            planned_frames: 0,
            last_present_offset: 0,
            last_present_len: 0,
            hardware_programming_confirmed: 0,
        };
        let mut scanout = NativeGpuScanoutRecord {
            presented_frames: 0,
            last_frame_len: 0,
            last_frame_tag: [0; 64],
            last_source_api_name: [0; 24],
            last_translation_label: [0; 32],
        };
        assert_eq!(
            inspect_gpu_display_syscall(
                GPU_DEVICE_PATH.as_ptr() as usize,
                GPU_DEVICE_PATH.len(),
                &mut display as *mut _
            ),
            Ok(0)
        );
        assert_eq!(
            inspect_gpu_scanout_syscall(
                GPU_DEVICE_PATH.as_ptr() as usize,
                GPU_DEVICE_PATH.len(),
                &mut scanout as *mut _
            ),
            Ok(0)
        );
        assert_eq!(display.present, 1);
        assert_eq!(display.active_pipes, 1);
        assert_eq!(display.planned_frames, 1);
        assert_eq!(scanout.presented_frames, 1);
        let tag_end = scanout
            .last_frame_tag
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(scanout.last_frame_tag.len());
        assert_eq!(
            core::str::from_utf8(&scanout.last_frame_tag[..tag_end]).unwrap(),
            "boot-gfx-002"
        );
        let api_end = scanout
            .last_source_api_name
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(scanout.last_source_api_name.len());
        assert_eq!(
            core::str::from_utf8(&scanout.last_source_api_name[..api_end]).unwrap(),
            "opengl"
        );
        let mut frame = [0u8; 256];
        let frame_len = read_gpu_scanout_frame_syscall(
            GPU_DEVICE_PATH.as_ptr() as usize,
            GPU_DEVICE_PATH.len(),
            frame.as_mut_ptr(),
            frame.len(),
        )
        .unwrap();
        assert_eq!(&frame[..frame_len], payload);

        let mut request_record = NativeDeviceRequestRecord {
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
        assert_eq!(
            inspect_device_request_syscall(1, &mut request_record as *mut _),
            Ok(0)
        );
        assert_eq!(request_record.state, 2);
        assert_eq!(request_record.opcode, 0x4750_0001);
        let request_tag_end = request_record
            .frame_tag
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(request_record.frame_tag.len());
        assert_eq!(
            core::str::from_utf8(&request_record.frame_tag[..request_tag_end]).unwrap(),
            "boot-gfx-002"
        );
    }

    #[test]
    fn boot_gpu_failed_request_retains_terminal_metadata_and_device_error_payload() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let payload =
            b"frame=boot-gfx-fail-003\nsource-api=directx12\ntranslation=compat-to-vulkan\npresent=0,0,1280,720";
        let driver_reply = b"failed-request:1\nerror:boot-present";
        let device_fd =
            open_path_syscall(GPU_DEVICE_PATH.as_ptr() as usize, GPU_DEVICE_PATH.len()).unwrap();
        let driver_fd =
            open_path_syscall(GPU_DRIVER_PATH.as_ptr() as usize, GPU_DRIVER_PATH.len()).unwrap();
        assert_eq!(
            write_syscall(device_fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mut request = [0u8; 256];
        let request_len = read_syscall(driver_fd, request.as_mut_ptr(), request.len()).unwrap();
        assert_eq!(&request[..request_len], payload);
        assert_eq!(
            write_syscall(driver_fd, driver_reply.as_ptr(), driver_reply.len()),
            Ok(driver_reply.len())
        );

        let mut device = NativeDeviceRecord {
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
        let mut driver = NativeDriverRecord {
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
        assert_eq!(
            inspect_device_syscall(
                GPU_DEVICE_PATH.as_ptr() as usize,
                GPU_DEVICE_PATH.len(),
                &mut device as *mut _
            ),
            Ok(0)
        );
        assert_eq!(
            inspect_driver_syscall(
                GPU_DRIVER_PATH.as_ptr() as usize,
                GPU_DRIVER_PATH.len(),
                &mut driver as *mut _
            ),
            Ok(0)
        );
        assert_eq!(device.last_terminal_request_id, 1);
        assert_eq!(device.last_terminal_state, 3);
        assert_eq!(driver.last_terminal_state, 3);
        assert_eq!(device.completed_requests, 0);

        let mut request_record = NativeDeviceRequestRecord {
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
        assert_eq!(
            inspect_device_request_syscall(1, &mut request_record as *mut _),
            Ok(0)
        );
        assert_eq!(request_record.state, 3);
        assert_eq!(
            request_record.response_len,
            b"error:boot-present".len() as u64
        );

        let mut completion = [0u8; 128];
        let completion_len =
            read_syscall(device_fd, completion.as_mut_ptr(), completion.len()).unwrap();
        assert_eq!(&completion[..completion_len], b"error:boot-present");

        let mut scanout = NativeGpuScanoutRecord {
            presented_frames: 0,
            last_frame_len: 0,
            last_frame_tag: [0; 64],
            last_source_api_name: [0; 24],
            last_translation_label: [0; 32],
        };
        assert_eq!(
            inspect_gpu_scanout_syscall(
                GPU_DEVICE_PATH.as_ptr() as usize,
                GPU_DEVICE_PATH.len(),
                &mut scanout as *mut _
            ),
            Ok(0)
        );
        assert_eq!(scanout.presented_frames, 0);
    }

    #[test]
    fn boot_gpu_canceled_request_retains_terminal_metadata_without_completion_or_scanout() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let payload =
            b"frame=boot-gfx-cancel-004\nsource-api=opengl\ntranslation=compat-to-vulkan\npresent=0,0,800,600";
        let driver_reply = b"cancel-request:1\nabort:boot-present";
        let device_fd =
            open_path_syscall(GPU_DEVICE_PATH.as_ptr() as usize, GPU_DEVICE_PATH.len()).unwrap();
        let driver_fd =
            open_path_syscall(GPU_DRIVER_PATH.as_ptr() as usize, GPU_DRIVER_PATH.len()).unwrap();
        assert_eq!(
            write_syscall(device_fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mut request = [0u8; 256];
        let request_len = read_syscall(driver_fd, request.as_mut_ptr(), request.len()).unwrap();
        assert_eq!(&request[..request_len], payload);
        assert_eq!(
            write_syscall(driver_fd, driver_reply.as_ptr(), driver_reply.len()),
            Ok(driver_reply.len())
        );

        let mut device = NativeDeviceRecord {
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
        let mut driver = NativeDriverRecord {
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
        assert_eq!(
            inspect_device_syscall(
                GPU_DEVICE_PATH.as_ptr() as usize,
                GPU_DEVICE_PATH.len(),
                &mut device as *mut _
            ),
            Ok(0)
        );
        assert_eq!(
            inspect_driver_syscall(
                GPU_DRIVER_PATH.as_ptr() as usize,
                GPU_DRIVER_PATH.len(),
                &mut driver as *mut _
            ),
            Ok(0)
        );
        assert_eq!(device.last_terminal_request_id, 1);
        assert_eq!(device.last_terminal_state, 4);
        assert_eq!(driver.last_terminal_state, 4);
        assert_eq!(device.completed_requests, 0);

        let mut request_record = NativeDeviceRequestRecord {
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
        assert_eq!(
            inspect_device_request_syscall(1, &mut request_record as *mut _),
            Ok(0)
        );
        assert_eq!(request_record.state, 4);
        assert_eq!(request_record.response_len, 0);

        let mut completion = [0u8; 128];
        assert_eq!(
            read_syscall(device_fd, completion.as_mut_ptr(), completion.len()),
            Ok(0)
        );

        let mut scanout = NativeGpuScanoutRecord {
            presented_frames: 0,
            last_frame_len: 0,
            last_frame_tag: [0; 64],
            last_source_api_name: [0; 24],
            last_translation_label: [0; 32],
        };
        assert_eq!(
            inspect_gpu_scanout_syscall(
                GPU_DEVICE_PATH.as_ptr() as usize,
                GPU_DEVICE_PATH.len(),
                &mut scanout as *mut _
            ),
            Ok(0)
        );
        assert_eq!(scanout.presented_frames, 0);
    }

    #[test]
    fn boot_report_syscall_records_structured_boot_session_report() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let result = boot_report_syscall(
            BootSessionStatus::Success as u32,
            BootSessionStage::Bootstrap as u32,
            0,
            0x1000,
        );
        assert_eq!(result, Ok(0));
        let result = boot_report_syscall(
            BootSessionStatus::Success as u32,
            BootSessionStage::NativeRuntime as u32,
            0,
            0x2000,
        );
        assert_eq!(result, Ok(0));
        let result = boot_report_syscall(
            BootSessionStatus::Success as u32,
            BootSessionStage::Complete as u32,
            0,
            0xfeed,
        );
        assert_eq!(result, Ok(0));

        let status = user_runtime_status::snapshot();
        assert!(status.boot_reported);
        assert_eq!(status.boot_report_status, BootSessionStatus::Success as u32);
        assert_eq!(status.boot_report_stage, BootSessionStage::Complete as u32);
        assert_eq!(status.boot_report_code, 0);
        assert_eq!(status.boot_report_detail, 0xfeed);
    }

    #[test]
    fn boot_report_syscall_rejects_stage_regression() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        assert_eq!(
            boot_report_syscall(
                BootSessionStatus::Success as u32,
                BootSessionStage::NativeRuntime as u32,
                0,
                1,
            ),
            Ok(0)
        );
        assert_eq!(
            boot_report_syscall(
                BootSessionStatus::Success as u32,
                BootSessionStage::Bootstrap as u32,
                0,
                2,
            ),
            Err(Errno::Inval)
        );
    }

    #[test]
    fn boot_vm_procfs_reports_vmobjects_and_decisions_after_global_reclaim() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        assert_eq!(reclaim_memory_pressure_global_syscall(3), Ok(3));

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("[heap]"));
        assert!(text.contains("resident=1"));
        assert!(text.contains("dirty=0"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=pressure-trigger"));
        assert!(text.contains("agent=sync"));
        assert!(text.contains("agent=pressure-victim"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=reclaim"));
        assert!(text.contains("evicted=no"));
    }

    #[test]
    fn boot_vm_map_and_quarantine_are_observable_through_procfs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"boot-scratch";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(store_memory_word_syscall(1, mapped, 7), Ok(0));
        assert_eq!(quarantine_vm_object_syscall(1, 2, 44), Ok(0));
        assert_eq!(store_memory_word_syscall(1, mapped, 9), Err(Errno::Fault));
        assert_eq!(release_vm_object_syscall(1, 2), Ok(0));
        assert_eq!(store_memory_word_syscall(1, mapped, 9), Ok(0));

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("[anon:boot-scratch]"));
        assert!(text.contains("quarantined=0\treason=0"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=map"));
        assert!(text.contains("agent=quarantine-state"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=map"), "{text}");
        assert!(text.contains("mapped=anon"), "{text}");
        assert!(text.contains("kind=quarantine"));
        assert!(text.contains("reason=44"));
        assert!(text.contains("blocked=yes"));
        assert!(text.contains("released=yes"));
    }

    #[test]
    fn boot_vm_protect_and_unmap_are_observable_and_enforced() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"prot-range";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(
            protect_memory_range_syscall(1, mapped + 0x1000, 0x1000, 1, 0, 0),
            Ok(0)
        );
        assert_eq!(
            store_memory_word_syscall(1, mapped + 0x1000, 7),
            Err(Errno::Fault)
        );
        assert_eq!(load_memory_word_syscall(1, mapped + 0x1000), Ok(0));

        let path = b"/proc/1/maps";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("rw-p 00000000 [anon:prot-range]"));
        assert!(text.contains("r--p 00000000 [anon:prot-range]"), "{text}");

        let path = b"/proc/1/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("readable=1\twritable=0\texecutable=0"));

        assert_eq!(
            unmap_memory_range_syscall(1, mapped + 0x1000, 0x1000),
            Ok(0)
        );
        assert_eq!(
            load_memory_word_syscall(1, mapped + 0x1000),
            Err(Errno::Fault)
        );

        let path = b"/proc/1/maps";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[anon:prot-range]").count(), 2);

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=protect"));
        assert!(text.contains("agent=unmap"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=region"), "{text}");
        assert!(text.contains("protected=yes"), "{text}");
        assert!(text.contains("unmapped=yes"), "{text}");
    }

    #[test]
    fn boot_vm_protect_and_unmap_refuse_invalid_ranges() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(
            protect_memory_range_syscall(1, 0xdead_0000, 0x1000, 1, 0, 0),
            Err(Errno::Fault)
        );
        assert_eq!(
            unmap_memory_range_syscall(1, 0xdead_0000, 0x1000),
            Err(Errno::Fault)
        );
        assert_eq!(
            protect_memory_range_syscall(1, 0x4000_0000, 0, 1, 0, 0),
            Err(Errno::Inval)
        );
        assert_eq!(
            unmap_memory_range_syscall(1, 0x4000_0000, 0),
            Err(Errno::Inval)
        );
    }

    #[test]
    fn boot_vm_file_backed_mapping_is_observable_and_recoverable() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libboot.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x5au8; 0x3000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mapped = map_file_backed_memory_boot(
            1,
            lib_path.as_ptr() as usize,
            lib_path.len(),
            0x2000,
            0x1000,
            1,
            0,
            1,
            1,
        )
        .unwrap();

        assert_eq!(
            load_memory_word_syscall(1, mapped),
            Ok(u32::from_le_bytes([0x5a, 0x5a, 0x5a, 0x5a]) as usize)
        );
        assert_eq!(store_memory_word_syscall(1, mapped, 7), Err(Errno::Fault));
        assert_eq!(
            protect_memory_range_syscall(1, mapped, 0x2000, 1, 1, 0),
            Ok(0)
        );
        assert_eq!(store_memory_word_syscall(1, mapped, 9), Ok(0));
        assert_eq!(sync_memory_range_syscall(1, mapped, 0x2000), Ok(0));

        let mut buffer = [0u8; 2048];
        let path = b"/proc/1/maps";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("rw-p 00001000 /lib/libboot.so"));

        let path = b"/proc/1/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("File\tprivate=true\towners=1\toffset=00001000"));
        assert!(text.contains("/lib/libboot.so"));
        assert!(text.contains("dirty=0"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=map-file"));
        assert!(text.contains("agent=protect"));
        assert!(text.contains("agent=sync"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=map"), "{text}");
        assert!(text.contains("mapped=file-private"), "{text}");
    }

    #[test]
    fn boot_vm_file_backed_mapping_refuses_missing_directory_and_invalid_offset() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libboot.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x11u8; 0x2000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let missing = b"/lib/missing.so";
        assert_eq!(
            map_file_backed_memory_boot(
                1,
                missing.as_ptr() as usize,
                missing.len(),
                0x1000,
                0,
                1,
                0,
                0,
                1
            ),
            Err(Errno::NoEnt)
        );
        assert_eq!(
            map_file_backed_memory_boot(
                1,
                lib_dir.as_ptr() as usize,
                lib_dir.len(),
                0x1000,
                0,
                1,
                0,
                0,
                1
            ),
            Err(Errno::Inval)
        );
        assert_eq!(
            map_file_backed_memory_boot(
                1,
                lib_path.as_ptr() as usize,
                lib_path.len(),
                0x1000,
                0x3000,
                1,
                0,
                0,
                1
            ),
            Err(Errno::Inval)
        );
    }

    #[test]
    fn boot_vm_file_backed_mapping_tracks_sync_truncate_and_unlink_lifecycle() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let file = b"/mapped.bin";
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(write_syscall(fd, b"abcdwxyz".as_ptr(), 8), Ok(8));
        assert_eq!(close_syscall(fd), Ok(0));

        let mapped = map_file_backed_memory_boot(
            1,
            file.as_ptr() as usize,
            file.len(),
            0x1000,
            0,
            1,
            1,
            0,
            1,
        )
        .unwrap();

        assert_eq!(
            load_memory_word_syscall(1, mapped),
            Ok(u32::from_le_bytes(*b"abcd") as usize)
        );
        assert_eq!(
            store_memory_word_syscall(1, mapped + 4, u32::from_le_bytes(*b"4321") as usize),
            Ok(0)
        );
        assert_eq!(sync_memory_range_syscall(1, mapped, 0x1000), Ok(0));

        let file_fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let mut synced = [0u8; 8];
        assert_eq!(
            read_syscall(file_fd, synced.as_mut_ptr(), synced.len()),
            Ok(8)
        );
        assert_eq!(&synced, b"abcd4321");
        assert_eq!(close_syscall(file_fd), Ok(0));

        assert_eq!(
            truncate_path_syscall(file.as_ptr() as usize, file.len(), 2),
            Ok(0)
        );
        assert_eq!(load_memory_word_syscall(1, mapped + 4), Ok(0));

        assert_eq!(
            unlink_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        assert_eq!(
            open_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::NoEnt)
        );
        assert_eq!(
            load_memory_word_syscall(1, mapped),
            Ok(u32::from_le_bytes([b'a', b'b', 0, 0]) as usize)
        );
        assert_eq!(sync_memory_range_syscall(1, mapped, 0x1000), Ok(0));
        assert_eq!(unmap_memory_range_syscall(1, mapped, 0x1000), Ok(0));
    }

    #[test]
    fn boot_vm_faults_and_page_touch_are_observable_through_procfs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libfault.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x22u8; 0x3000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mapped = map_file_backed_memory_boot(
            1,
            lib_path.as_ptr() as usize,
            lib_path.len(),
            0x3000,
            0,
            1,
            0,
            1,
            1,
        )
        .unwrap();
        assert_eq!(
            protect_memory_range_syscall(1, mapped, 0x3000, 1, 1, 0),
            Ok(0)
        );
        assert_eq!(store_memory_word_syscall(1, mapped, 7), Ok(0));
        assert_eq!(
            load_memory_word_syscall(1, mapped + 0x1000),
            Ok(u32::from_le_bytes([0x22; 4]) as usize)
        );
        assert_eq!(
            load_memory_word_syscall(1, mapped + 0x1000),
            Ok(u32::from_le_bytes([0x22; 4]) as usize)
        );

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("resident=2\tdirty=1\taccessed=2"), "{text}");
        assert!(text.contains("segments=3\tresident-segments=2"), "{text}");
        assert!(text.contains("faults=2(r=1,w=1,cow=0)\t/lib/libfault.so"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=fault-classifier"));
        assert!(text.contains("agent=page-touch"));
        assert_eq!(text.matches("agent=fault-classifier").count(), 2);

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=fault"));
        assert!(text.contains("faulted=yes"));
        assert!(text.contains("touched=yes"));
    }

    #[test]
    fn boot_vm_copy_state_creates_cow_shadow_and_observability() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-vm-cow-observe");

        let label = b"cow-boot";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 9), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("owners=1"));
        assert!(text.contains("[anon:cow-boot] [cow]"));
        assert!(text.contains("shadow="));
        assert!(text.contains("depth=1"));
        assert!(text.contains("faults=2(r=0,w=1,cow=1)"));

        let path = b"/proc/2/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=shadow-reuse"));
        assert!(text.contains("agent=cow-populate"));

        let path = b"/proc/2/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=fault"));
        assert!(text.contains("cow=yes"));
    }

    #[test]
    fn boot_vm_copy_state_tracks_nested_shadow_depth() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-vm-depth-observe");

        let label = b"cow-depth";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();

        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 9), Ok(0));

        let grandchild_name = b"grandchild";
        let grandchild_path = b"/bin/grandchild";
        let grandchild = spawn_path_process_syscall(
            grandchild_name.as_ptr() as usize,
            grandchild_name.len(),
            grandchild_path.as_ptr() as usize,
            grandchild_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(child as u64, grandchild as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(grandchild, mapped, 11), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/3/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("depth=2"));
        assert!(text.contains("[anon:cow-depth] [cow] [cow]"));
        assert!(text.contains("faults=2(r=0,w=1,cow=1)"));

        let path = b"/proc/3/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=shadow-reuse"));
        assert!(text.contains("detail1=2"));
    }

    #[test]
    fn boot_vm_reuses_shadow_for_adjacent_partial_cow_faults() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-vm-reuse-observe");

        let label = b"shadow-reuse";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 1), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 2), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[cow]").count(), 1);
        assert!(text.contains("@00000000/depth=1"));
        assert!(text.contains("committed=2\tresident=2\tdirty=2\taccessed=2"));

        let path = b"/proc/2/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.matches("agent=shadow-reuse").count() >= 2);
    }

    #[test]
    fn boot_vm_reuses_shadow_for_reverse_adjacent_partial_cow_faults() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-vm-reuse-rev-observe");

        let label = b"shadow-reuse-reverse";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 1), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped, 2), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[cow]").count(), 1);
        assert!(text.contains("@00000000/depth=1"));
        assert!(text.contains("committed=2\tresident=2\tdirty=2\taccessed=2"));
    }

    #[test]
    fn boot_vm_bridges_shadow_objects_for_middle_cow_fault() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-vm-bridge-observe");

        let label = b"shadow-bridge";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 1), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x2000, 2), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 3), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[cow]").count(), 1);
        assert!(text.contains("committed=3\tresident=3\tdirty=3\taccessed=3"));

        let path = b"/proc/2/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=shadow-bridge"));

        let path = b"/proc/2/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=fault"));
        assert!(text.contains("cow=yes"));
        assert!(text.contains("bridged=yes"));
    }

    #[test]
    fn boot_vm_tracks_nonzero_shadow_offsets_across_generations() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        bind_observe_contract(b"boot-vm-offset-observe");

        let label = b"shadow-offset";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();

        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 1), Ok(0));

        let grandchild_name = b"grandchild";
        let grandchild_path = b"/bin/grandchild";
        let grandchild = spawn_path_process_syscall(
            grandchild_name.as_ptr() as usize,
            grandchild_name.len(),
            grandchild_path.as_ptr() as usize,
            grandchild_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(child as u64, grandchild as u64), Ok(()));
        assert_eq!(
            store_memory_word_syscall(grandchild, mapped + 0x1000, 2),
            Ok(0)
        );

        let mut buffer = [0u8; 4096];
        let path = b"/proc/3/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("@00001000/depth=2"));
        assert!(text.matches("[cow]").count() >= 1);
    }

    #[test]
    fn boot_vmobjects_report_real_segment_counts_for_sparse_page_state() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"segment-shape";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(store_memory_word_syscall(1, mapped, 1), Ok(0));
        assert_eq!(load_memory_word_syscall(1, mapped + 0x1000), Ok(0));

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 4096];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("[anon:segment-shape]"));
        assert!(text.contains("segments=3\tresident-segments=2"), "{text}");
    }

    #[test]
    fn boot_vm_heap_growth_and_shrink_are_observable_through_procfs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(
            set_process_break_vm_syscall(1, 0x4000_7000),
            Ok(0x4000_7000)
        );
        assert_eq!(
            set_process_break_vm_syscall(1, 0x4000_3000),
            Ok(0x4000_3000)
        );

        let path = b"/proc/1/vmepisodes";
        let mut buffer = [0u8; 4096];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=heap"), "{text}");
        assert!(text.contains("grew=yes"), "{text}");
        assert!(text.contains("shrank=yes"), "{text}");
        assert!(text.contains("old-end=1073758208"), "{text}");
        assert!(text.contains("new-end=1073754112"), "{text}");
        assert!(text.contains("decisions=2"), "{text}");
        assert!(text.contains("last=brk"), "{text}");
    }

    #[test]
    fn boot_procfs_open_read_and_stat_work_through_descriptor_path() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"procfs-open";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(
            protect_memory_range_syscall(1, mapped + 0x1000, 0x1000, 1, 0, 0),
            Ok(0)
        );

        let path = b"/proc/1/maps";
        let fd = open_path_syscall(path.as_ptr() as usize, path.len()).unwrap();
        let mut buffer = [0u8; 2048];
        let read = read_syscall(fd, buffer.as_mut_ptr(), buffer.len()).unwrap();
        let text = core::str::from_utf8(&buffer[..read]).unwrap();
        assert!(text.contains("[anon:procfs-open]"));
        assert!(text.contains("rw-p 00000000 [anon:procfs-open]"));
        assert_eq!(
            read_syscall(fd, buffer.as_mut_ptr(), buffer.len()).unwrap(),
            0
        );

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
        assert_eq!(
            stat_path_syscall(path.as_ptr() as usize, path.len(), &mut record as *mut _),
            Ok(0)
        );
        assert_eq!(record.kind, NativeObjectKind::File as u32);
        assert_eq!(record.readable, 1);
        assert_eq!(record.writable, 0);
        assert!(record.size >= read as u64);
    }

    #[test]
    fn boot_procfs_open_path_refuses_unknown_nodes() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let path = b"/proc/1/unknown";
        assert_eq!(
            open_path_syscall(path.as_ptr() as usize, path.len()),
            Err(Errno::NoEnt)
        );
    }

    #[test]
    fn boot_procfs_fd_and_process_path_views_work_on_real_boot_path() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let mut buffer = [0u8; 2048];

        let cwd_path = b"/proc/1/cwd";
        let cwd_read = read_procfs_syscall(
            cwd_path.as_ptr() as usize,
            cwd_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let cwd_text = core::str::from_utf8(&buffer[..cwd_read]).unwrap();
        assert!(!cwd_text.is_empty(), "{cwd_text}");

        let exe_path = b"/proc/1/exe";
        let exe_read = read_procfs_syscall(
            exe_path.as_ptr() as usize,
            exe_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let exe_text = core::str::from_utf8(&buffer[..exe_read]).unwrap();
        assert!(!exe_text.is_empty(), "{exe_text}");

        let fd_path = b"/proc/1/fd";
        let fd_read = read_procfs_syscall(
            fd_path.as_ptr() as usize,
            fd_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let fd_text = core::str::from_utf8(&buffer[..fd_read]).unwrap();
        assert!(fd_text.contains("0\t"));
        assert!(fd_text.contains("1\t"));
        assert!(fd_text.contains("2\t"));

        assert_eq!(
            read_procfs_syscall(
                b"/proc/1/fdinfo/9999".as_ptr() as usize,
                b"/proc/1/fdinfo/9999".len(),
                buffer.as_mut_ptr(),
                buffer.len(),
            ),
            Err(Errno::NoEnt)
        );
    }

    #[test]
    fn boot_procfs_spawned_process_environ_tracks_compat_env_markers() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let name = b"compat-environ";
        let path = b"/bin/worker";
        let cwd = b"/abi/nova";
        let argv = b"/bin/worker\0--compat\0";
        let envp = b"NGOS_COMPAT_TARGET=game\0NGOS_COMPAT_ROUTE_CLASS=compat-game-runtime\0NGOS_COMPAT_LAUNCH_MODE=compat-shim\0NGOS_COMPAT_ENTRY_PROFILE=dx-to-vulkan-entry\0NGOS_COMPAT_PREFIX=/compat/abi-game\0";
        let config = NativeSpawnProcessConfig {
            name_ptr: name.as_ptr() as usize,
            name_len: name.len(),
            path_ptr: path.as_ptr() as usize,
            path_len: path.len(),
            cwd_ptr: cwd.as_ptr() as usize,
            cwd_len: cwd.len(),
            argv_ptr: argv.as_ptr() as usize,
            argv_len: argv.len(),
            argv_count: 2,
            envp_ptr: envp.as_ptr() as usize,
            envp_len: envp.len(),
            envp_count: 5,
        };
        let pid = spawn_configured_process_syscall(&config as *const _ as usize).unwrap();

        let environ_path = format!("/proc/{pid}/environ");
        let mut buffer = [0u8; 1024];
        BootVfs::set_current_subject(0, 0);
        let environ_read = read_procfs_syscall(
            environ_path.as_ptr() as usize,
            environ_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let environ_text = core::str::from_utf8(&buffer[..environ_read]).unwrap();
        assert!(
            environ_text.contains("NGOS_COMPAT_TARGET=game"),
            "{environ_text}"
        );
        assert!(
            environ_text.contains("NGOS_COMPAT_ROUTE_CLASS=compat-game-runtime"),
            "{environ_text}"
        );
        assert!(
            environ_text.contains("NGOS_COMPAT_LAUNCH_MODE=compat-shim"),
            "{environ_text}"
        );
        assert!(
            environ_text.contains("NGOS_COMPAT_ENTRY_PROFILE=dx-to-vulkan-entry"),
            "{environ_text}"
        );
        assert!(
            environ_text.contains("NGOS_COMPAT_PREFIX=/compat/abi-game"),
            "{environ_text}"
        );
        BootVfs::set_current_subject(1000, 1000);
    }

    #[test]
    fn boot_procfs_spawned_process_cmdline_preserves_full_argv() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let name = b"argv-worker";
        let path = b"/bin/ngos-userland-native";
        let cwd = b"/";
        let argv = b"/bin/ngos-userland-native\0--compat-proc-probe\0--flag=fast\0";
        let config = NativeSpawnProcessConfig {
            name_ptr: name.as_ptr() as usize,
            name_len: name.len(),
            path_ptr: path.as_ptr() as usize,
            path_len: path.len(),
            cwd_ptr: cwd.as_ptr() as usize,
            cwd_len: cwd.len(),
            argv_ptr: argv.as_ptr() as usize,
            argv_len: argv.len(),
            argv_count: 3,
            envp_ptr: b"".as_ptr() as usize,
            envp_len: 0,
            envp_count: 0,
        };
        let pid = spawn_configured_process_syscall(&config as *const _ as usize).unwrap();

        let cmdline = read_procfs_text(&format!("/proc/{pid}/cmdline"));
        assert_eq!(
            cmdline,
            "/bin/ngos-userland-native\n--compat-proc-probe\n--flag=fast"
        );
    }

    #[test]
    fn boot_vfs_symlink_rename_unlink_and_readlink_work_on_real_boot_path() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/vfs";
        let bin = b"/vfs/bin";
        let app = b"/vfs/bin/app";
        let app2 = b"/vfs/bin/app2";
        let link = b"/vfs/link";
        let invalid_subtree = b"/vfs/bin/subdir";

        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(mkfile_path_syscall(app.as_ptr() as usize, app.len()), Ok(0));
        assert_eq!(
            symlink_path_syscall(
                link.as_ptr() as usize,
                link.len(),
                app.as_ptr() as usize,
                app.len(),
            ),
            Ok(0)
        );

        let mut link_stat = NativeFileStatusRecord {
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
        assert_eq!(
            lstat_path_syscall(link.as_ptr() as usize, link.len(), &mut link_stat as *mut _),
            Ok(0)
        );
        assert_eq!(link_stat.kind, NativeObjectKind::Symlink as u32);

        let mut target = [0u8; 64];
        let copied = readlink_path_syscall(
            link.as_ptr() as usize,
            link.len(),
            target.as_mut_ptr(),
            target.len(),
        )
        .unwrap();
        assert_eq!(&target[..copied], app);

        let fd = open_path_syscall(link.as_ptr() as usize, link.len()).unwrap();
        assert!(fd >= 3);

        assert_eq!(
            rename_path_syscall(
                app.as_ptr() as usize,
                app.len(),
                app2.as_ptr() as usize,
                app2.len(),
            ),
            Ok(0)
        );
        assert_eq!(
            rename_path_syscall(
                bin.as_ptr() as usize,
                bin.len(),
                invalid_subtree.as_ptr() as usize,
                invalid_subtree.len(),
            ),
            Err(Errno::Inval)
        );
        assert_eq!(
            unlink_path_syscall(link.as_ptr() as usize, link.len()),
            Ok(0)
        );
        assert_eq!(
            readlink_path_syscall(
                link.as_ptr() as usize,
                link.len(),
                target.as_mut_ptr(),
                target.len(),
            ),
            Err(Errno::NoEnt)
        );
    }

    #[test]
    fn boot_vfs_normalizes_dot_dot_and_redundant_slashes() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(
            BootVfs::normalize_path("/vfs//bin/./app/../tool///").unwrap(),
            "/vfs/bin/tool"
        );
        assert_eq!(BootVfs::normalize_path("/./").unwrap(), "/");
        assert_eq!(BootVfs::normalize_path("/a/b/../../c").unwrap(), "/c");
        assert_eq!(BootVfs::normalize_path("/../../escape"), Err(Errno::Inval));
    }

    #[test]
    fn boot_vfs_relative_symlink_resolves_against_link_parent_and_preserves_readlink_target() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/vfs";
        let bin = b"/vfs/bin";
        let assets = b"/vfs/assets";
        let sprite = b"/vfs/assets/sprite";
        let link = b"/vfs/bin/sprite-link";
        let relative_target = b"../assets/sprite";

        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(
            mkdir_path_syscall(assets.as_ptr() as usize, assets.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(sprite.as_ptr() as usize, sprite.len()),
            Ok(0)
        );
        assert_eq!(
            symlink_path_syscall(
                link.as_ptr() as usize,
                link.len(),
                relative_target.as_ptr() as usize,
                relative_target.len(),
            ),
            Ok(0)
        );

        let fd = open_path_syscall(link.as_ptr() as usize, link.len()).unwrap();
        assert!(fd >= 3);

        let mut target = [0u8; 64];
        let copied = readlink_path_syscall(
            link.as_ptr() as usize,
            link.len(),
            target.as_mut_ptr(),
            target.len(),
        )
        .unwrap();
        assert_eq!(&target[..copied], relative_target);
    }

    #[test]
    fn boot_vfs_permissions_block_and_recover_on_real_boot_path() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/secure";
        let file = b"/secure/data.txt";
        let renamed = b"/secure/data-renamed.txt";

        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(
            write_syscall(fd, b"secret".as_ptr(), b"secret".len()),
            Ok(6)
        );
        assert_eq!(close_syscall(fd), Ok(0));

        assert_eq!(
            chmod_path_syscall(root.as_ptr() as usize, root.len(), 0),
            Ok(0)
        );

        let mut list_buffer = [0u8; 128];
        assert_eq!(
            list_path_syscall(
                root.as_ptr() as usize,
                root.len(),
                list_buffer.as_mut_ptr(),
                list_buffer.len(),
            ),
            Err(Errno::Access)
        );
        assert_eq!(
            open_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Access)
        );
        assert_eq!(
            rename_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                renamed.as_ptr() as usize,
                renamed.len(),
            ),
            Err(Errno::Access)
        );
        assert_eq!(
            unlink_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Access)
        );

        assert_eq!(
            chmod_path_syscall(root.as_ptr() as usize, root.len(), 0o755),
            Ok(0)
        );
        assert_eq!(
            chmod_path_syscall(file.as_ptr() as usize, file.len(), 0),
            Ok(0)
        );
        assert_eq!(
            open_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Access)
        );
        assert_eq!(
            chmod_path_syscall(file.as_ptr() as usize, file.len(), 0o644),
            Ok(0)
        );

        let recovered_fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(close_syscall(recovered_fd), Ok(0));
        assert_eq!(
            rename_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                renamed.as_ptr() as usize,
                renamed.len(),
            ),
            Ok(0)
        );
        assert_eq!(
            rename_path_syscall(
                renamed.as_ptr() as usize,
                renamed.len(),
                file.as_ptr() as usize,
                file.len(),
            ),
            Ok(0)
        );
        assert_eq!(
            chown_path_syscall(file.as_ptr() as usize, file.len(), 0, 0),
            Err(Errno::Perm)
        );
    }

    #[test]
    fn boot_vfs_sticky_directory_blocks_rename_and_unlink_for_non_owner() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/sticky";
        let file = b"/sticky/other.txt";
        let renamed = b"/sticky/renamed.txt";
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        BOOT_VFS.with_mut(|vfs| {
            let root_index = vfs.find_node("/sticky").unwrap();
            vfs.nodes[root_index].mode = 0o1777;
            let file_index = vfs.find_node("/sticky/other.txt").unwrap();
            vfs.nodes[file_index].owner_uid = 2000;
            vfs.invalidate_caches();
        });
        BootVfs::set_current_subject(1000, 1000);

        assert_eq!(
            unlink_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Access)
        );
        assert_eq!(
            rename_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                renamed.as_ptr() as usize,
                renamed.len(),
            ),
            Err(Errno::Access)
        );

        BootVfs::set_current_subject(0, 0);
        BOOT_VFS.with_mut(|vfs| {
            let file_index = vfs.find_node("/sticky/other.txt").unwrap();
            vfs.nodes[file_index].owner_uid = 1000;
            vfs.invalidate_caches();
        });
        BootVfs::set_current_subject(1000, 1000);

        assert_eq!(
            rename_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                renamed.as_ptr() as usize,
                renamed.len(),
            ),
            Ok(0)
        );
        assert_eq!(
            unlink_path_syscall(renamed.as_ptr() as usize, renamed.len()),
            Ok(0)
        );
    }

    #[test]
    fn boot_vfs_sgid_directory_inherits_group_and_directory_bit() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/shared";
        let file = b"/shared/file.txt";
        let dir = b"/shared/subdir";
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        BOOT_VFS.with_mut(|vfs| {
            let index = vfs.find_node("/shared").unwrap();
            vfs.nodes[index].group_gid = 4242;
            vfs.nodes[index].mode = 0o2777;
            vfs.invalidate_caches();
        });
        BootVfs::set_current_subject(1000, 1000);

        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(dir.as_ptr() as usize, dir.len()), Ok(0));

        let file_status = boot_vfs_stat("/shared/file.txt").unwrap();
        let dir_status = boot_vfs_stat("/shared/subdir").unwrap();
        assert_eq!(file_status.group_gid, 4242);
        assert_eq!(dir_status.group_gid, 4242);
        assert_eq!(dir_status.mode & 0o2000, 0o2000);
    }

    #[test]
    fn boot_vfs_supplemental_groups_grant_group_class_access() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/group-access";
        let file = b"/group-access/secret.txt";
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        BOOT_VFS.with_mut(|vfs| {
            let index = vfs.find_node("/group-access/secret.txt").unwrap();
            vfs.nodes[index].owner_uid = 2000;
            vfs.nodes[index].group_gid = 4242;
            vfs.nodes[index].mode = 0o640;
            vfs.invalidate_caches();
        });
        BootVfs::set_current_subject(1000, 1000);
        BootVfs::set_current_supplemental_groups(&[4242]);

        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let mut buffer = [0u8; 8];
        assert_eq!(read_syscall(fd, buffer.as_mut_ptr(), buffer.len()), Ok(0));

        BootVfs::set_current_supplemental_groups(&[]);
        assert_eq!(
            open_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Access)
        );
    }

    #[test]
    fn boot_vfs_umask_shapes_created_file_and_directory_modes() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/umask-root";
        let dir = b"/umask-root/dir";
        let file = b"/umask-root/file.txt";
        BootVfs::set_current_subject(1000, 1000);
        BootVfs::set_current_umask(0o027);

        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(dir.as_ptr() as usize, dir.len()), Ok(0));
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let dir_status = boot_vfs_stat("/umask-root/dir").unwrap();
        let file_status = boot_vfs_stat("/umask-root/file.txt").unwrap();
        assert_eq!(dir_status.mode & 0o777, 0o750);
        assert_eq!(file_status.mode & 0o777, 0o640);
    }

    #[test]
    fn boot_vfs_fcntl_locking_refuses_contended_descriptor_and_recovers() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/locks";
        let file = b"/locks/data.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let primary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let secondary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();

        assert_eq!(fcntl_syscall(primary, 5 | ((0x44usize) << 8)), Ok(0x44));
        assert_eq!(fcntl_syscall(primary, 4), Ok(0x44));
        assert_eq!(
            fcntl_syscall(secondary, 5 | ((0x55usize) << 8)),
            Err(Errno::Busy)
        );
        assert_eq!(
            fcntl_syscall(secondary, 6 | ((0x55usize) << 8)),
            Err(Errno::Perm)
        );
        assert_eq!(fcntl_syscall(primary, 6 | ((0x44usize) << 8)), Ok(0x44));
        assert_eq!(fcntl_syscall(primary, 4), Ok(0));
    }

    #[test]
    fn boot_vfs_shared_locking_allows_read_sharing_and_blocks_exclusive_until_release() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/shared-locks";
        let file = b"/shared-locks/data.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let primary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let secondary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let exclusive = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();

        assert_eq!(fcntl_syscall(primary, 7 | ((0x66usize) << 8)), Ok(0x66));
        assert_eq!(fcntl_syscall(secondary, 7 | ((0x77usize) << 8)), Ok(0x77));
        assert_eq!(
            fcntl_syscall(exclusive, 5 | ((0x88usize) << 8)),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(primary, 8 | ((0x66usize) << 8)), Ok(0x66));
        assert_eq!(
            fcntl_syscall(exclusive, 5 | ((0x88usize) << 8)),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(secondary, 8 | ((0x77usize) << 8)), Ok(0x77));
        assert_eq!(fcntl_syscall(exclusive, 5 | ((0x88usize) << 8)), Ok(0x88));
        assert_eq!(fcntl_syscall(exclusive, 6 | ((0x88usize) << 8)), Ok(0x88));
    }

    #[test]
    fn boot_vfs_lock_upgrade_and_downgrade_coordinate_shared_and_exclusive_phases() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/upgrade-locks";
        let file = b"/upgrade-locks/data.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let primary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let secondary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let contender = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();

        assert_eq!(fcntl_syscall(primary, 7 | ((0x91usize) << 8)), Ok(0x91));
        assert_eq!(fcntl_syscall(secondary, 7 | ((0x92usize) << 8)), Ok(0x92));
        assert_eq!(
            fcntl_syscall(primary, 9 | ((0x91usize) << 8)),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(secondary, 8 | ((0x92usize) << 8)), Ok(0x92));
        assert_eq!(fcntl_syscall(primary, 9 | ((0x91usize) << 8)), Ok(0x91));
        assert_eq!(
            fcntl_syscall(contender, 7 | ((0x93usize) << 8)),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(primary, 10 | ((0x91usize) << 8)), Ok(0x91));
        assert_eq!(fcntl_syscall(contender, 7 | ((0x93usize) << 8)), Ok(0x93));
    }

    #[test]
    fn boot_vfs_locks_block_path_mutations_and_foreign_writes_until_release() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/mutation-locks";
        let file = b"/mutation-locks/data.txt";
        let alias = b"/mutation-locks/data-link.txt";
        let renamed = b"/mutation-locks/data-renamed.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let primary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let secondary = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();

        assert_eq!(fcntl_syscall(primary, 5 | ((0x91usize) << 8)), Ok(0x91));
        assert_eq!(write_syscall(primary, b"owner".as_ptr(), 5), Ok(5));
        assert_eq!(
            write_syscall(secondary, b"peer".as_ptr(), 4),
            Err(Errno::Busy)
        );
        assert_eq!(
            truncate_path_syscall(file.as_ptr() as usize, file.len(), 2),
            Err(Errno::Busy)
        );
        assert_eq!(
            link_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                alias.as_ptr() as usize,
                alias.len()
            ),
            Err(Errno::Busy)
        );
        assert_eq!(
            rename_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                renamed.as_ptr() as usize,
                renamed.len()
            ),
            Err(Errno::Busy)
        );
        assert_eq!(
            unlink_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Busy)
        );

        assert_eq!(fcntl_syscall(primary, 6 | ((0x91usize) << 8)), Ok(0x91));
        assert_eq!(write_syscall(secondary, b"peer".as_ptr(), 4), Ok(4));
        assert_eq!(
            link_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                alias.as_ptr() as usize,
                alias.len()
            ),
            Ok(0)
        );
        assert_eq!(
            truncate_path_syscall(file.as_ptr() as usize, file.len(), 3),
            Ok(0)
        );
        assert_eq!(
            rename_path_syscall(
                alias.as_ptr() as usize,
                alias.len(),
                renamed.as_ptr() as usize,
                renamed.len()
            ),
            Ok(0)
        );
        assert_eq!(
            unlink_path_syscall(renamed.as_ptr() as usize, renamed.len()),
            Ok(0)
        );
    }

    #[test]
    fn boot_vfs_shared_locks_block_path_mutations_until_all_readers_release() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/shared-mutation-locks";
        let file = b"/shared-mutation-locks/data.txt";
        let alias = b"/shared-mutation-locks/data-link.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let first = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let second = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();

        assert_eq!(fcntl_syscall(first, 7 | ((0xa1usize) << 8)), Ok(0xa1));
        assert_eq!(fcntl_syscall(second, 7 | ((0xa2usize) << 8)), Ok(0xa2));
        assert_eq!(
            truncate_path_syscall(file.as_ptr() as usize, file.len(), 1),
            Err(Errno::Busy)
        );
        assert_eq!(
            link_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                alias.as_ptr() as usize,
                alias.len()
            ),
            Err(Errno::Busy)
        );

        assert_eq!(fcntl_syscall(first, 8 | ((0xa1usize) << 8)), Ok(0xa1));
        assert_eq!(
            unlink_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(second, 8 | ((0xa2usize) << 8)), Ok(0xa2));
        assert_eq!(
            link_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                alias.as_ptr() as usize,
                alias.len()
            ),
            Ok(0)
        );
    }

    #[test]
    fn boot_vfs_directory_descriptors_lock_namespace_mutations_until_release() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/dir-locks";
        let file = b"/dir-locks/data.txt";
        let created = b"/dir-locks/new.txt";
        let renamed = b"/dir-locks/data-renamed.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let directory_fd = open_path_syscall(root.as_ptr() as usize, root.len()).unwrap();
        let info = boot_procfs_fdinfo(1, directory_fd as u64).unwrap();
        assert!(info.contains("kind:\tDirectory"), "{info}");

        assert_eq!(
            fcntl_syscall(directory_fd, 5 | ((0xa1usize) << 8)),
            Ok(0xa1)
        );
        assert_eq!(
            mkfile_path_syscall(created.as_ptr() as usize, created.len()),
            Err(Errno::Busy)
        );
        assert_eq!(
            rename_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                renamed.as_ptr() as usize,
                renamed.len()
            ),
            Err(Errno::Busy)
        );
        assert_eq!(
            fcntl_syscall(directory_fd, 6 | ((0xa1usize) << 8)),
            Ok(0xa1)
        );

        assert_eq!(
            mkfile_path_syscall(created.as_ptr() as usize, created.len()),
            Ok(0)
        );
        assert_eq!(
            rename_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                renamed.as_ptr() as usize,
                renamed.len()
            ),
            Ok(0)
        );
    }

    #[test]
    fn boot_vfs_locked_descendant_blocks_directory_rename_until_release() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/tree-locks";
        let subdir = b"/tree-locks/sub";
        let file = b"/tree-locks/sub/data.txt";
        let moved = b"/tree-locks-moved";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkdir_path_syscall(subdir.as_ptr() as usize, subdir.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let file_fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(fcntl_syscall(file_fd, 5 | ((0xb2usize) << 8)), Ok(0xb2));
        assert_eq!(
            rename_path_syscall(
                root.as_ptr() as usize,
                root.len(),
                moved.as_ptr() as usize,
                moved.len()
            ),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(file_fd, 6 | ((0xb2usize) << 8)), Ok(0xb2));
        assert_eq!(
            rename_path_syscall(
                root.as_ptr() as usize,
                root.len(),
                moved.as_ptr() as usize,
                moved.len()
            ),
            Ok(0)
        );
        assert_eq!(
            open_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::NoEnt)
        );
        let rebound = open_path_syscall(
            b"/tree-locks-moved/sub/data.txt".as_ptr() as usize,
            b"/tree-locks-moved/sub/data.txt".len(),
        );
        assert!(rebound.is_ok(), "{rebound:?}");
    }

    #[test]
    fn boot_vfs_relative_paths_follow_process_cwd_across_core_path_syscalls() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let workspace = b"/workspace";
        let bin_dir = b"/workspace/bin";
        let rel_file = b"bin/app.txt";
        let rel_open = b"./bin/app.txt";
        let rel_renamed = b"bin/tool.txt";
        let rel_link = b"./bin/tool-link.txt";
        let rel_list = b"bin";
        let rel_unlink = b"bin/tool-link.txt";
        assert_eq!(
            mkdir_path_syscall(workspace.as_ptr() as usize, workspace.len()),
            Ok(0)
        );
        assert_eq!(
            mkdir_path_syscall(bin_dir.as_ptr() as usize, bin_dir.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_cwd_syscall(1, workspace.as_ptr() as usize, workspace.len()),
            Ok(0)
        );

        assert_eq!(
            mkfile_path_syscall(rel_file.as_ptr() as usize, rel_file.len()),
            Ok(0)
        );
        let fd = open_path_syscall(rel_open.as_ptr() as usize, rel_open.len()).unwrap();
        assert_eq!(write_syscall(fd, b"hello".as_ptr(), 5), Ok(5));
        assert_eq!(
            rename_path_syscall(
                rel_file.as_ptr() as usize,
                rel_file.len(),
                rel_renamed.as_ptr() as usize,
                rel_renamed.len()
            ),
            Ok(0)
        );
        assert_eq!(
            link_path_syscall(
                rel_renamed.as_ptr() as usize,
                rel_renamed.len(),
                rel_link.as_ptr() as usize,
                rel_link.len()
            ),
            Ok(0)
        );
        let mut listing = [0u8; 128];
        let count = list_path_syscall(
            rel_list.as_ptr() as usize,
            rel_list.len(),
            listing.as_mut_ptr(),
            listing.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&listing[..count]).unwrap();
        assert!(text.contains("tool.txt\tFile"), "{text}");
        assert!(text.contains("tool-link.txt\tFile"), "{text}");
        assert_eq!(
            unlink_path_syscall(rel_unlink.as_ptr() as usize, rel_unlink.len()),
            Ok(0)
        );
        assert_eq!(
            truncate_path_syscall(rel_renamed.as_ptr() as usize, rel_renamed.len(), 2),
            Ok(0)
        );
        let stat = boot_vfs_stat("/workspace/bin/tool.txt").unwrap();
        assert_eq!(stat.size, 2);
    }

    #[test]
    fn boot_vfs_set_process_cwd_accepts_relative_and_procfs_relative_reads_follow_it() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let games = b"/games";
        let orbit = b"/games/orbit";
        let rel_orbit = b"./orbit";
        let proc_rel = b"../../proc/1/cwd";
        assert_eq!(
            mkdir_path_syscall(games.as_ptr() as usize, games.len()),
            Ok(0)
        );
        assert_eq!(
            mkdir_path_syscall(orbit.as_ptr() as usize, orbit.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_cwd_syscall(1, games.as_ptr() as usize, games.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_cwd_syscall(1, rel_orbit.as_ptr() as usize, rel_orbit.len()),
            Ok(0)
        );

        let mut cwd = [0u8; 64];
        let copied = get_process_cwd_syscall(1, cwd.as_mut_ptr(), cwd.len()).unwrap();
        let cwd_text = core::str::from_utf8(&cwd[..copied]).unwrap();
        assert_eq!(cwd_text, "/games/orbit");

        let mut proc_buf = [0u8; 64];
        let proc_read = read_procfs_syscall(
            proc_rel.as_ptr() as usize,
            proc_rel.len(),
            proc_buf.as_mut_ptr(),
            proc_buf.len(),
        )
        .unwrap();
        let proc_text = core::str::from_utf8(&proc_buf[..proc_read]).unwrap();
        assert_eq!(proc_text, "/games/orbit");
    }

    #[test]
    fn boot_vfs_process_root_rebases_absolute_paths_and_blocks_escape() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let jail = b"/jail";
        let bin = b"/jail/bin";
        let tool = b"/jail/bin/tool.txt";
        let outside = b"/outside.txt";
        assert_eq!(
            mkdir_path_syscall(jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(
            mkfile_path_syscall(tool.as_ptr() as usize, tool.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(outside.as_ptr() as usize, outside.len()),
            Ok(0)
        );

        assert_eq!(
            set_process_root_syscall(1, jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_cwd_syscall(1, b"/bin".as_ptr() as usize, 4),
            Ok(0)
        );

        let mut root_buf = [0u8; 64];
        let root_read = get_process_root_syscall(1, root_buf.as_mut_ptr(), root_buf.len()).unwrap();
        let root_text = core::str::from_utf8(&root_buf[..root_read]).unwrap();
        assert_eq!(root_text, "/jail");

        let jailed_fd = open_path_syscall(b"/bin/tool.txt".as_ptr() as usize, 13).unwrap();
        assert!(jailed_fd >= 3);
        assert_eq!(
            open_path_syscall(b"/outside.txt".as_ptr() as usize, 12),
            Err(Errno::NoEnt)
        );
        assert_eq!(
            open_path_syscall(b"../../outside.txt".as_ptr() as usize, 17),
            Err(Errno::Access)
        );
    }

    #[test]
    fn boot_vfs_set_process_root_rehomes_cwd_when_old_cwd_falls_outside_new_root() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let jail = b"/sandbox";
        let inner = b"/sandbox/work";
        assert_eq!(
            mkdir_path_syscall(jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(
            mkdir_path_syscall(inner.as_ptr() as usize, inner.len()),
            Ok(0)
        );
        assert_eq!(set_process_cwd_syscall(1, b"/".as_ptr() as usize, 1), Ok(0));
        assert_eq!(
            set_process_root_syscall(1, jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );

        let mut cwd_buf = [0u8; 64];
        let cwd_read = get_process_cwd_syscall(1, cwd_buf.as_mut_ptr(), cwd_buf.len()).unwrap();
        let cwd_text = core::str::from_utf8(&cwd_buf[..cwd_read]).unwrap();
        assert_eq!(cwd_text, "/sandbox");
    }

    #[test]
    fn boot_vfs_spawned_processes_inherit_root_and_resolved_cwd() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let jail = b"/apps";
        let bin = b"/apps/bin";
        let image = b"/apps/bin/game.bin";
        assert_eq!(
            mkdir_path_syscall(jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_root_syscall(1, jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_cwd_syscall(1, b"/bin".as_ptr() as usize, 4),
            Ok(0)
        );

        let child = spawn_path_process_syscall(
            b"game".as_ptr() as usize,
            4,
            b"/bin/game.bin".as_ptr() as usize,
            13,
        )
        .unwrap();

        let mut root_buf = [0u8; 64];
        let root_read =
            get_process_root_syscall(child, root_buf.as_mut_ptr(), root_buf.len()).unwrap();
        let root_text = core::str::from_utf8(&root_buf[..root_read]).unwrap();
        assert_eq!(root_text, "/apps");

        let mut cwd_buf = [0u8; 64];
        let cwd_read = get_process_cwd_syscall(child, cwd_buf.as_mut_ptr(), cwd_buf.len()).unwrap();
        let cwd_text = core::str::from_utf8(&cwd_buf[..cwd_read]).unwrap();
        assert_eq!(cwd_text, "/apps/bin");

        let mut exe_buf = [0u8; 64];
        let exe_read =
            get_process_image_path_syscall(child, exe_buf.as_mut_ptr(), exe_buf.len()).unwrap();
        let exe_text = core::str::from_utf8(&exe_buf[..exe_read]).unwrap();
        assert_eq!(exe_text, "/apps/bin/game.bin");
    }

    #[test]
    fn boot_vfs_active_process_context_follows_selected_pid() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let jail = b"/ctx";
        let bin = b"/ctx/bin";
        let image = b"/ctx/bin/task.bin";
        assert_eq!(
            mkdir_path_syscall(jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );

        let child = spawn_path_process_syscall(
            b"task".as_ptr() as usize,
            4,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        set_active_process_pid(child as u64);
        assert_eq!(active_process_pid(), Ok(child as u64));
        assert_eq!(
            set_process_cwd_syscall(child, bin.as_ptr() as usize, bin.len()),
            Ok(0)
        );
        assert_eq!(
            active_process_root_and_cwd(),
            Ok((String::from("/"), String::from("/ctx/bin")))
        );

        set_active_process_pid(1);
        assert_eq!(active_process_pid(), Ok(1));
    }

    #[test]
    fn boot_vfs_non_root_cannot_reconfigure_other_process_namespace_identity_or_label() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let jail = b"/ctl";
        let bin = b"/ctl/bin";
        let image = b"/ctl/bin/task.bin";
        assert_eq!(
            mkdir_path_syscall(jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );

        let child = spawn_path_process_syscall(
            b"task".as_ptr() as usize,
            4,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        BootVfs::set_current_subject(1000, 1000);

        let identity = NativeProcessIdentityRecord {
            uid: 1000,
            gid: 1000,
            umask: 0o077,
            supplemental_count: 0,
            supplemental_gids: [0; 8],
        };
        let label = SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified);
        assert_eq!(
            set_process_root_syscall(child, jail.as_ptr() as usize, jail.len()),
            Err(Errno::Perm)
        );
        assert_eq!(
            set_process_cwd_syscall(child, bin.as_ptr() as usize, bin.len()),
            Err(Errno::Perm)
        );
        assert_eq!(
            set_process_identity_syscall(child, &identity as *const _),
            Err(Errno::Perm)
        );
        assert_eq!(
            set_process_security_label_syscall(child, &label as *const _),
            Err(Errno::Perm)
        );
    }

    #[test]
    fn boot_vfs_spawned_processes_inherit_vfs_identity_and_proc_status_exposes_it() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/identity";
        let image = b"/identity/task.bin";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );

        BootVfs::set_current_subject(4242, 4343);
        BootVfs::set_current_umask(0o077);
        BootVfs::set_current_supplemental_groups(&[77, 88, 99]);

        let child = spawn_path_process_syscall(
            b"task".as_ptr() as usize,
            4,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        BOOT_PROCESSES.with_mut(|registry| {
            let index = registry.find_index(child as u64).unwrap();
            let entry = &registry.entries[index];
            assert_eq!(entry.uid, 4242);
            assert_eq!(entry.gid, 4343);
            assert_eq!(entry.umask, 0o077);
            assert_eq!(
                &entry.supplemental_gids[..entry.supplemental_count],
                &[77, 88, 99]
            );
        });

        let status_path = format!("/proc/{child}/status");
        let mut buffer = [0u8; 256];
        let read = read_procfs_syscall(
            status_path.as_ptr() as usize,
            status_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..read]).unwrap();
        assert!(text.contains("Uid:\t4242"), "{text}");
        assert!(text.contains("Gid:\t4343"), "{text}");
        assert!(text.contains("Umask:\t077"), "{text}");
        assert!(text.contains("SupplementalGroups:\t77,88,99"), "{text}");
    }

    #[test]
    fn boot_vfs_process_identity_syscalls_roundtrip_and_shape_spawn_inheritance() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/identity-abi";
        let image = b"/identity-abi/task.bin";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );

        let configured = NativeProcessIdentityRecord {
            uid: 7001,
            gid: 7002,
            umask: 0o137,
            supplemental_count: 2,
            supplemental_gids: [17, 23, 0, 0, 0, 0, 0, 0],
        };
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_process_identity_syscall(1, &configured as *const _),
            Ok(0)
        );

        let mut observed = NativeProcessIdentityRecord::default();
        assert_eq!(
            inspect_process_identity_syscall(1, &mut observed as *mut _),
            Ok(0)
        );
        assert_eq!(observed.uid, 7001);
        assert_eq!(observed.gid, 7002);
        assert_eq!(observed.umask, 0o137);
        assert_eq!(observed.supplemental_count, 2);
        assert_eq!(&observed.supplemental_gids[..2], &[17, 23]);

        let child = spawn_path_process_syscall(
            b"task".as_ptr() as usize,
            4,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();
        let mut child_identity = NativeProcessIdentityRecord::default();
        assert_eq!(
            inspect_process_identity_syscall(child, &mut child_identity as *mut _),
            Err(Errno::Access)
        );
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            inspect_process_identity_syscall(child, &mut child_identity as *mut _),
            Ok(0)
        );
        assert_eq!(child_identity.uid, 7001);
        assert_eq!(child_identity.gid, 7002);
        assert_eq!(child_identity.umask, 0o137);
        assert_eq!(child_identity.supplemental_count, 2);
        assert_eq!(&child_identity.supplemental_gids[..2], &[17, 23]);
    }

    #[test]
    fn boot_vfs_non_root_identity_changes_can_only_drop_within_current_group_set() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let current = NativeProcessIdentityRecord {
            uid: 1000,
            gid: 1000,
            umask: 0o022,
            supplemental_count: 2,
            supplemental_gids: [7, 8, 0, 0, 0, 0, 0, 0],
        };
        BootVfs::set_current_subject(0, 0);
        assert_eq!(set_process_identity_syscall(1, &current as *const _), Ok(0));
        BootVfs::set_current_subject(1000, 1000);
        BootVfs::set_current_supplemental_groups(&[7, 8]);

        let narrowed = NativeProcessIdentityRecord {
            uid: 1000,
            gid: 7,
            umask: 0o077,
            supplemental_count: 1,
            supplemental_gids: [8, 0, 0, 0, 0, 0, 0, 0],
        };
        assert_eq!(
            set_process_identity_syscall(1, &narrowed as *const _),
            Ok(0)
        );

        let mut observed = NativeProcessIdentityRecord::default();
        assert_eq!(
            inspect_process_identity_syscall(1, &mut observed as *mut _),
            Ok(0)
        );
        assert_eq!(observed.uid, 1000);
        assert_eq!(observed.gid, 7);
        assert_eq!(observed.umask, 0o077);
        assert_eq!(observed.supplemental_count, 1);
        assert_eq!(&observed.supplemental_gids[..1], &[8]);

        let raised_uid = NativeProcessIdentityRecord {
            uid: 2000,
            ..narrowed
        };
        assert_eq!(
            set_process_identity_syscall(1, &raised_uid as *const _),
            Err(Errno::Perm)
        );

        let foreign_gid = NativeProcessIdentityRecord {
            gid: 9000,
            ..narrowed
        };
        assert_eq!(
            set_process_identity_syscall(1, &foreign_gid as *const _),
            Err(Errno::Perm)
        );

        let expanded_groups = NativeProcessIdentityRecord {
            supplemental_count: 2,
            supplemental_gids: [8, 77, 0, 0, 0, 0, 0, 0],
            ..narrowed
        };
        assert_eq!(
            set_process_identity_syscall(1, &expanded_groups as *const _),
            Err(Errno::Perm)
        );
    }

    #[test]
    fn boot_vfs_process_security_label_syscalls_roundtrip_and_shape_spawn_inheritance() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/label-identity";
        let image = b"/label-identity/task.bin";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );

        BootVfs::set_current_subject(0, 0);
        let configured = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Kernel);
        assert_eq!(
            set_process_security_label_syscall(1, &configured as *const _),
            Ok(0)
        );

        let mut observed =
            SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
        assert_eq!(
            inspect_process_security_label_syscall(1, &mut observed as *mut _),
            Ok(0)
        );
        assert_eq!(observed, configured);

        let child = spawn_path_process_syscall(
            b"task".as_ptr() as usize,
            4,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();
        let mut child_label =
            SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
        assert_eq!(
            inspect_process_security_label_syscall(child, &mut child_label as *mut _),
            Ok(0)
        );
        assert_eq!(child_label, configured);

        let status_path = format!("/proc/{child}/status");
        let mut buffer = [0u8; 256];
        let read = read_procfs_syscall(
            status_path.as_ptr() as usize,
            status_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..read]).unwrap();
        assert!(text.contains("SubjectLabel:\tSecret/Kernel"), "{text}");
    }

    #[test]
    fn boot_vfs_non_root_process_label_can_only_reduce_self_and_not_raise() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        BootVfs::set_current_subject(0, 0);
        let seeded = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::System);
        assert_eq!(
            set_process_security_label_syscall(1, &seeded as *const _),
            Ok(0)
        );

        BootVfs::set_current_subject(1000, 1000);
        let reduced = SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &reduced as *const _),
            Ok(0)
        );

        let raised_conf =
            SecurityLabel::new(ConfidentialityLevel::Kernel, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &raised_conf as *const _),
            Err(Errno::Perm)
        );

        let raised_integrity =
            SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Kernel);
        assert_eq!(
            set_process_security_label_syscall(1, &raised_integrity as *const _),
            Err(Errno::Perm)
        );
    }

    #[test]
    fn boot_vfs_non_root_owner_can_only_tighten_object_label_while_root_can_relax() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/label-owner";
        let file = b"/label-owner/data.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        BootVfs::set_current_subject(0, 0);
        let seeded = SecurityLabel::new(ConfidentialityLevel::Internal, IntegrityLevel::Verified);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &seeded),
            Ok(0)
        );
        assert_eq!(
            chown_path_syscall(file.as_ptr() as usize, file.len(), 1000, 1000),
            Ok(0)
        );

        BootVfs::set_current_subject(1000, 1000);
        let tightened = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::System);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &tightened),
            Ok(0)
        );

        let relaxed = SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &relaxed),
            Err(Errno::Perm)
        );

        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &relaxed),
            Ok(0)
        );
    }

    #[test]
    fn boot_vfs_path_security_context_blocks_read_until_subject_label_recovers() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/secure";
        let file = b"/secure/secret.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        let seed_fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(write_syscall(seed_fd, b"secret".as_ptr(), 6), Ok(6));
        assert_eq!(close_syscall(seed_fd), Ok(0));

        BootVfs::set_current_subject(0, 0);
        let object_label =
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &object_label),
            Ok(0)
        );

        let mut context = ObjectSecurityContext::new(
            0,
            BlockRightsMask::NONE,
            SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified),
            SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified),
            ProvenanceTag::root(
                ProvenanceOriginKind::Subject,
                0,
                0,
                IntegrityTag::zeroed(IntegrityTagKind::Blake3),
            ),
            IntegrityTag::zeroed(IntegrityTagKind::Blake3),
            0,
            0,
        );
        assert_eq!(
            inspect_path_security_context_syscall(
                file.as_ptr() as usize,
                file.len(),
                &mut context as *mut _
            ),
            Ok(0)
        );
        assert_eq!(context.minimum_label, object_label);
        assert_eq!(context.current_label, object_label);

        let low_label = SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &low_label as *const _),
            Ok(0)
        );
        assert_eq!(
            open_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Access)
        );

        assert_eq!(
            set_process_security_label_syscall(1, &object_label as *const _),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let mut buffer = [0u8; 8];
        assert_eq!(read_syscall(fd, buffer.as_mut_ptr(), buffer.len()), Ok(6));
        assert_eq!(&buffer[..6], b"secret");
        assert_eq!(close_syscall(fd), Ok(0));
    }

    #[test]
    fn boot_vfs_path_security_label_blocks_write_until_subject_label_recovers() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/ifc";
        let file = b"/ifc/kernel.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        BootVfs::set_current_subject(0, 0);
        let object_label = SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Kernel);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &object_label),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();

        let low_integrity =
            SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &low_integrity as *const _),
            Ok(0)
        );
        assert_eq!(write_syscall(fd, b"x".as_ptr(), 1), Err(Errno::Access));

        assert_eq!(
            set_process_security_label_syscall(1, &object_label as *const _),
            Ok(0)
        );
        assert_eq!(write_syscall(fd, b"ok".as_ptr(), 2), Ok(2));
        assert_eq!(close_syscall(fd), Ok(0));
    }

    #[test]
    fn boot_vfs_fd_rights_restrict_file_handle_mutation_and_delegation_but_keep_read() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let file = b"/rights.txt";
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(write_syscall(fd, b"hello".as_ptr(), 5), Ok(5));
        assert_eq!(seek_syscall(fd, 0, SeekWhence::Set as u32), Ok(0));

        assert_eq!(
            set_fd_rights_syscall(fd, BlockRightsMask::READ),
            Ok(BlockRightsMask::READ.0 as usize)
        );

        let mut status = NativeFileStatusRecord {
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
        assert_eq!(
            stat_path_at_syscall(fd, "".as_ptr() as usize, 0, &mut status),
            Ok(0)
        );
        let mut bytes = [0u8; 8];
        assert_eq!(read_syscall(fd, bytes.as_mut_ptr(), bytes.len()), Ok(5));
        assert_eq!(&bytes[..5], b"hello");
        assert_eq!(write_syscall(fd, b"!".as_ptr(), 1), Err(Errno::Access));
        assert_eq!(
            truncate_path_at_syscall(fd, "".as_ptr() as usize, 0, 1),
            Err(Errno::Access)
        );
        assert_eq!(
            open_path_at_syscall(fd, "".as_ptr() as usize, 0),
            Err(Errno::Access)
        );
    }

    #[test]
    fn boot_vfs_fd_rights_survive_spawn_snapshot_and_procfs_reports_them() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/rights-spawn";
        let image = b"/rights-spawn/task.bin";
        let file = b"/rights-spawn/data.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        let narrowed = BlockRightsMask::READ.union(BlockRightsMask::DELEGATE);
        assert_eq!(set_fd_rights_syscall(fd, narrowed), Ok(narrowed.0 as usize));

        let child = spawn_path_process_syscall(
            b"task".as_ptr() as usize,
            4,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        let fdinfo_path = format!("/proc/{child}/fdinfo/{fd}");
        let mut buffer = [0u8; 256];
        BootVfs::set_current_subject(0, 0);
        let read = read_procfs_syscall(
            fdinfo_path.as_ptr() as usize,
            fdinfo_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..read]).unwrap();
        assert!(text.contains("rights:\t0x81"), "{text}");
        BootVfs::set_current_subject(1000, 1000);
    }

    #[test]
    fn boot_vfs_fd_rights_block_dup_and_read_poll_without_delegate_or_read() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let file = b"/rights-poll.txt";
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(write_syscall(fd, b"abc".as_ptr(), 3), Ok(3));
        assert_eq!(
            set_fd_rights_syscall(fd, BlockRightsMask::WRITE),
            Ok(BlockRightsMask::WRITE.0 as usize)
        );
        assert_eq!(duplicate_syscall(fd), Err(Errno::Access));
        assert_eq!(poll_syscall(fd, POLLIN), Err(Errno::Access));
        assert_eq!(poll_syscall(fd, POLLOUT), Ok(POLLOUT as usize));
    }

    #[test]
    fn boot_vfs_fd_rights_block_vfs_watch_registration_without_queue_write_or_anchor_read() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/watch-rights";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let dir_fd = open_path_syscall(root.as_ptr() as usize, root.len()).unwrap();
        let watch = NativeVfsEventWatchConfig {
            token: 77,
            poll_events: POLLPRI,
            subtree: 1,
            created: 1,
            opened: 1,
            closed: 1,
            written: 1,
            renamed: 1,
            unlinked: 1,
            mounted: 1,
            unmounted: 1,
            lock_acquired: 1,
            lock_refused: 1,
            permission_refused: 1,
            truncated: 1,
            linked: 1,
        };

        assert_eq!(
            set_fd_rights_syscall(queue_fd, BlockRightsMask::READ),
            Ok(BlockRightsMask::READ.0 as usize)
        );
        assert_eq!(
            watch_vfs_events_at_syscall(queue_fd, dir_fd, "".as_ptr() as usize, 0, &watch),
            Err(Errno::Access)
        );

        let queue_fd2 = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        assert_eq!(
            set_fd_rights_syscall(dir_fd, BlockRightsMask::DELEGATE),
            Ok(BlockRightsMask::DELEGATE.0 as usize)
        );
        assert_eq!(
            watch_vfs_events_at_syscall(queue_fd2, dir_fd, "".as_ptr() as usize, 0, &watch),
            Err(Errno::Access)
        );
    }

    #[test]
    fn boot_vfs_event_queue_reports_object_lifecycle_and_lock_refusals() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let root = b"/watch";
        let file = b"/watch/a.txt";
        let link = b"/watch/b.txt";
        let renamed = b"/watch/c.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );

        let watch = NativeVfsEventWatchConfig {
            token: 9001,
            poll_events: POLLPRI,
            subtree: 1,
            created: 1,
            opened: 1,
            closed: 1,
            written: 1,
            renamed: 1,
            unlinked: 1,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 1,
            lock_refused: 1,
            permission_refused: 1,
            truncated: 1,
            linked: 1,
        };
        assert_eq!(
            watch_vfs_events_syscall(queue_fd, root.as_ptr() as usize, root.len(), &watch),
            Ok(0)
        );

        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(write_syscall(fd, b"hello".as_ptr(), 5), Ok(5));
        assert_eq!(fcntl_syscall(fd, 5 | ((0x33usize) << 8)), Ok(0x33));
        let peer = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(
            fcntl_syscall(peer, 5 | ((0x44usize) << 8)),
            Err(Errno::Busy)
        );
        assert_eq!(
            truncate_path_syscall(file.as_ptr() as usize, file.len(), 3),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(fd, 6 | ((0x33usize) << 8)), Ok(0x33));
        assert_eq!(
            truncate_path_syscall(file.as_ptr() as usize, file.len(), 3),
            Ok(0)
        );
        assert_eq!(
            link_path_syscall(
                file.as_ptr() as usize,
                file.len(),
                link.as_ptr() as usize,
                link.len(),
            ),
            Ok(0)
        );
        assert_eq!(
            rename_path_syscall(
                link.as_ptr() as usize,
                link.len(),
                renamed.as_ptr() as usize,
                renamed.len(),
            ),
            Ok(0)
        );
        assert_eq!(
            unlink_path_syscall(renamed.as_ptr() as usize, renamed.len()),
            Ok(0)
        );
        assert_eq!(close_syscall(peer), Ok(0));
        assert_eq!(close_syscall(fd), Ok(0));

        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 16];
        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert!(count >= 9, "count={count}");
        let kinds = events[..count]
            .iter()
            .map(|event| event.detail0)
            .collect::<Vec<_>>();
        assert!(
            kinds.contains(&(NativeVfsEventKind::Created as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Opened as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Written as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::LockAcquired as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::LockRefused as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Truncated as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Linked as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Renamed as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Unlinked as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Closed as u32)),
            "{kinds:?}"
        );
        assert!(
            events[..count]
                .iter()
                .all(|event| event.source_kind == NativeEventSourceKind::Vfs as u32)
        );
        assert!(events[..count].iter().all(|event| event.token == 9001));
    }

    #[test]
    fn boot_vfs_event_queue_reports_permission_refusal_and_remove_stops_delivery() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let root = b"/guarded";
        let file = b"/guarded/secret.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let watch = NativeVfsEventWatchConfig {
            token: 444,
            poll_events: POLLPRI,
            subtree: 1,
            created: 0,
            opened: 0,
            closed: 0,
            written: 0,
            renamed: 0,
            unlinked: 0,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 0,
            lock_refused: 0,
            permission_refused: 1,
            truncated: 0,
            linked: 0,
        };
        assert_eq!(
            watch_vfs_events_syscall(queue_fd, root.as_ptr() as usize, root.len(), &watch),
            Ok(0)
        );

        assert_eq!(
            chmod_path_syscall(root.as_ptr() as usize, root.len(), 0),
            Ok(0)
        );
        let mut listing = [0u8; 64];
        assert_eq!(
            list_path_syscall(
                root.as_ptr() as usize,
                root.len(),
                listing.as_mut_ptr(),
                listing.len()
            ),
            Err(Errno::Access)
        );
        assert_eq!(
            open_path_syscall(file.as_ptr() as usize, file.len()),
            Err(Errno::Access)
        );

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
        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert_eq!(count, 2, "{events:?}");
        assert!(
            events[..count]
                .iter()
                .all(|event| event.detail0 == NativeVfsEventKind::PermissionRefused as u32)
        );
        assert!(
            events[..count]
                .iter()
                .all(|event| event.detail1 == Errno::Access as u32)
        );
        let inodes = events[..count]
            .iter()
            .map(|event| event.source_arg0)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(inodes.len(), 2, "{events:?}");

        assert_eq!(
            remove_vfs_events_syscall(queue_fd, root.as_ptr() as usize, root.len(), 444),
            Ok(0)
        );
        assert_eq!(
            chmod_path_syscall(root.as_ptr() as usize, root.len(), 0o755),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(close_syscall(fd), Ok(0));
        assert_eq!(
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()),
            Err(Errno::Again)
        );
    }

    #[test]
    fn boot_vfs_watch_at_tracks_dir_handle_subtree_across_rename_and_remove() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        assert_eq!(
            mkdir_path_syscall("/capwatch".as_ptr() as usize, "/capwatch".len()),
            Ok(0)
        );
        let dir_fd = open_path_syscall("/capwatch".as_ptr() as usize, "/capwatch".len()).unwrap();

        let watch = NativeVfsEventWatchConfig {
            token: 7331,
            poll_events: POLLPRI,
            subtree: 1,
            created: 1,
            opened: 1,
            closed: 1,
            written: 1,
            renamed: 1,
            unlinked: 1,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 0,
            lock_refused: 0,
            permission_refused: 0,
            truncated: 0,
            linked: 0,
        };
        assert_eq!(
            watch_vfs_events_at_syscall(queue_fd, dir_fd, "".as_ptr() as usize, 0, &watch),
            Ok(0)
        );

        assert_eq!(
            rename_path_syscall(
                "/capwatch".as_ptr() as usize,
                "/capwatch".len(),
                "/capwatch-renamed".as_ptr() as usize,
                "/capwatch-renamed".len(),
            ),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_at_syscall(dir_fd, "after.txt".as_ptr() as usize, "after.txt".len()),
            Ok(0)
        );
        let file_fd =
            open_path_at_syscall(dir_fd, "after.txt".as_ptr() as usize, "after.txt".len()).unwrap();
        assert_eq!(write_syscall(file_fd, b"ok".as_ptr(), 2), Ok(2));
        assert_eq!(close_syscall(file_fd), Ok(0));

        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 8];
        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        let kinds = events[..count]
            .iter()
            .map(|event| event.detail0)
            .collect::<Vec<_>>();
        assert!(
            kinds.contains(&(NativeVfsEventKind::Renamed as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Created as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Opened as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Written as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Closed as u32)),
            "{kinds:?}"
        );
        assert!(events[..count].iter().all(|event| event.token == 7331));

        assert_eq!(
            remove_vfs_events_at_syscall(queue_fd, dir_fd, "".as_ptr() as usize, 0, 7331),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_at_syscall(dir_fd, "silent.txt".as_ptr() as usize, "silent.txt".len()),
            Ok(0)
        );
        assert_eq!(
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()),
            Err(Errno::Again)
        );

        assert_eq!(close_syscall(dir_fd), Ok(0));
        assert_eq!(close_syscall(queue_fd), Ok(0));
    }

    #[test]
    fn boot_vfs_spawn_configured_process_resolves_relative_image_and_cwd_inside_root() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let jail = b"/suite";
        let bin = b"/suite/bin";
        let image = b"/suite/bin/tool.bin";
        assert_eq!(
            mkdir_path_syscall(jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_root_syscall(1, jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );
        assert_eq!(
            set_process_cwd_syscall(1, b"/bin".as_ptr() as usize, 4),
            Ok(0)
        );

        let config = NativeSpawnProcessConfig {
            name_ptr: b"tool".as_ptr() as usize,
            name_len: 4,
            path_ptr: b"./tool.bin".as_ptr() as usize,
            path_len: 10,
            cwd_ptr: b".".as_ptr() as usize,
            cwd_len: 1,
            argv_ptr: 0,
            argv_len: 0,
            argv_count: 0,
            envp_ptr: 0,
            envp_len: 0,
            envp_count: 0,
        };
        let child = spawn_configured_process_syscall(&config as *const _ as usize).unwrap();

        let mut root_buf = [0u8; 64];
        let root_read =
            get_process_root_syscall(child, root_buf.as_mut_ptr(), root_buf.len()).unwrap();
        let root_text = core::str::from_utf8(&root_buf[..root_read]).unwrap();
        assert_eq!(root_text, "/suite");

        let mut cwd_buf = [0u8; 64];
        let cwd_read = get_process_cwd_syscall(child, cwd_buf.as_mut_ptr(), cwd_buf.len()).unwrap();
        let cwd_text = core::str::from_utf8(&cwd_buf[..cwd_read]).unwrap();
        assert_eq!(cwd_text, "/suite/bin");

        let mut exe_buf = [0u8; 64];
        let exe_read =
            get_process_image_path_syscall(child, exe_buf.as_mut_ptr(), exe_buf.len()).unwrap();
        let exe_text = core::str::from_utf8(&exe_buf[..exe_read]).unwrap();
        assert_eq!(exe_text, "/suite/bin/tool.bin");
    }

    #[test]
    fn boot_vm_memory_contract_policy_blocks_vm_operations_and_exposes_policy_episodes() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let domain = create_domain_syscall(0, b"vm".as_ptr() as usize, 2).unwrap();
        let resource = create_resource_syscall(
            domain,
            NativeResourceKind::Memory as u32,
            b"vm-budget".as_ptr() as usize,
            9,
        )
        .unwrap();
        assert_eq!(
            set_resource_contract_policy_syscall(
                resource,
                NativeResourceContractPolicy::Memory as u32
            ),
            Ok(0)
        );
        let contract = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Memory as u32,
            b"vm".as_ptr() as usize,
            2,
        )
        .unwrap();
        assert_eq!(bind_process_contract_syscall(contract), Ok(0));

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libpolicy.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x33u8; 0x2000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let label = b"policy-scratch";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(store_memory_word_syscall(1, mapped, 41), Ok(0));

        let vm_object_id = BOOT_PROCESSES
            .with_mut(|registry| {
                let entry = registry
                    .entries
                    .iter()
                    .find(|entry| entry.pid == 1)
                    .unwrap();
                entry
                    .vm_objects
                    .iter()
                    .find(|object| object.start == mapped as u64)
                    .map(|object| object.id as usize)
                    .ok_or(Errno::NoEnt)
            })
            .unwrap();

        assert_eq!(
            set_contract_state_syscall(contract, NativeContractState::Suspended as u32),
            Ok(0)
        );

        for result in [
            map_anonymous_memory_syscall(1, 0x1000, b"blocked-map".as_ptr() as usize, 11),
            map_file_backed_memory_boot(
                1,
                lib_path.as_ptr() as usize,
                lib_path.len(),
                0x1000,
                0,
                1,
                0,
                1,
                1,
            ),
            unmap_memory_range_syscall(1, mapped, 0x1000),
            protect_memory_range_syscall(1, mapped, 0x1000, 1, 0, 0),
            advise_memory_range_syscall(1, mapped, 0x1000, 4),
            sync_memory_range_syscall(1, mapped, 0x1000),
            quarantine_vm_object_syscall(1, vm_object_id, 7),
            release_vm_object_syscall(1, vm_object_id),
            load_memory_word_syscall(1, mapped),
            store_memory_word_syscall(1, mapped, 99),
            set_process_break_vm_syscall(1, mapped + 0x4000),
            reclaim_memory_pressure_syscall(1, 1),
            reclaim_memory_pressure_global_syscall(1),
        ] {
            assert_eq!(result, Err(Errno::Access));
        }

        let path = b"/proc/1/vmdecisions";
        let mut buffer = [0u8; 4096];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        for operation in [
            "\tdetail1=0",
            "\tdetail1=1",
            "\tdetail1=2",
            "\tdetail1=3",
            "\tdetail1=4",
            "\tdetail1=5",
            "\tdetail1=7",
            "\tdetail1=8",
            "\tdetail1=9",
            "\tdetail1=11",
            "\tdetail1=13",
            "\tdetail1=14",
        ] {
            assert!(text.contains("agent=policy-block"), "{text}");
            assert!(text.contains(operation), "{text}");
        }

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=policy"), "{text}");
        assert!(text.contains("state=1"), "{text}");
        assert!(text.contains("operation=14"), "{text}");
        assert!(text.contains("blocked=yes"), "{text}");

        assert_eq!(
            set_contract_state_syscall(contract, NativeContractState::Active as u32),
            Ok(0)
        );
        assert_eq!(load_memory_word_syscall(1, mapped), Ok(41));
        assert_eq!(store_memory_word_syscall(1, mapped, 41), Ok(0));
    }

    #[test]
    fn boot_vfs_dirfd_relative_flow_tracks_directory_handle_after_rename() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/cap", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/cap/base", BootNodeKind::Directory).unwrap();

        let dir_fd = DESCRIPTORS
            .with_mut(|table| table.open_path("/cap/base"))
            .unwrap();
        boot_vfs_rename("/cap/base", "/cap/live").unwrap();

        let note = "note.txt";
        assert_eq!(
            mkfile_path_at_syscall(dir_fd, note.as_ptr() as usize, note.len()),
            Ok(0)
        );

        let note_fd = open_path_at_syscall(dir_fd, note.as_ptr() as usize, note.len()).unwrap();
        let payload = b"cap-note";
        assert_eq!(
            write_syscall(note_fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );
        assert_eq!(close_syscall(note_fd), Ok(0));

        let link_name = "note-link";
        let target = "note.txt";
        assert_eq!(
            symlink_path_at_syscall(
                dir_fd,
                link_name.as_ptr() as usize,
                link_name.len(),
                target.as_ptr() as usize,
                target.len(),
            ),
            Ok(0)
        );

        let mut listing = [0u8; 128];
        let listing_len = list_path_at_syscall(
            dir_fd,
            ".".as_ptr() as usize,
            1,
            listing.as_mut_ptr(),
            listing.len(),
        )
        .unwrap();
        let listing_text = core::str::from_utf8(&listing[..listing_len]).unwrap();
        assert!(
            listing_text.contains("note-link\tSymlink"),
            "{listing_text}"
        );
        assert!(listing_text.contains("note.txt\tFile"), "{listing_text}");

        let mut link_target = [0u8; 64];
        let link_len = readlink_path_at_syscall(
            dir_fd,
            link_name.as_ptr() as usize,
            link_name.len(),
            link_target.as_mut_ptr(),
            link_target.len(),
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(&link_target[..link_len]).unwrap(),
            "note.txt"
        );

        let mut status = NativeFileStatusRecord {
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
        assert_eq!(
            stat_path_at_syscall(
                dir_fd,
                note.as_ptr() as usize,
                note.len(),
                &mut status as *mut _,
            ),
            Ok(0)
        );
        assert_ne!(status.inode, 0);
        assert_eq!(status.size, payload.len() as u64);
        assert!(boot_vfs_stat("/cap/live/note.txt").is_some());
        assert_eq!(close_syscall(dir_fd), Ok(0));
    }

    #[test]
    fn boot_vfs_dirfd_relative_ops_respect_process_root_and_refuse_non_directory_anchor() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/jail", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/jail/safe", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/jail/safe/file.txt", BootNodeKind::File).unwrap();

        let jail = "/jail";
        assert_eq!(
            set_process_root_syscall(1, jail.as_ptr() as usize, jail.len()),
            Ok(0)
        );

        let dir_fd = open_path_syscall("/safe".as_ptr() as usize, "/safe".len()).unwrap();
        let escaped = "../../escape.txt";
        assert_eq!(
            mkfile_path_at_syscall(dir_fd, escaped.as_ptr() as usize, escaped.len()),
            Err(Errno::Access)
        );
        assert!(boot_vfs_stat("/jail/escape.txt").is_none());
        assert!(boot_vfs_stat("/escape.txt").is_none());

        let file_name = "file.txt";
        let file_fd =
            open_path_at_syscall(dir_fd, file_name.as_ptr() as usize, file_name.len()).unwrap();
        assert_eq!(
            mkfile_path_at_syscall(file_fd, "child".as_ptr() as usize, 5),
            Err(Errno::NotDir)
        );
        assert_eq!(close_syscall(file_fd), Ok(0));
        assert_eq!(close_syscall(dir_fd), Ok(0));
    }

    #[test]
    fn boot_vfs_dirfd_relative_mutation_surface_covers_links_metadata_and_unlink_lifecycle() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/scope", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/scope/a", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/scope/b", BootNodeKind::Directory).unwrap();

        let dir_a = DESCRIPTORS
            .with_mut(|table| table.open_path("/scope/a"))
            .unwrap();
        let dir_b = DESCRIPTORS
            .with_mut(|table| table.open_path("/scope/b"))
            .unwrap();

        assert_eq!(
            mkfile_path_at_syscall(dir_a, "data.txt".as_ptr() as usize, 8),
            Ok(0)
        );
        assert_eq!(
            link_path_at_syscall(
                dir_a,
                "data.txt".as_ptr() as usize,
                8,
                dir_b,
                "linked.txt".as_ptr() as usize,
                10,
            ),
            Ok(0)
        );
        assert_eq!(
            rename_path_at_syscall(
                dir_b,
                "linked.txt".as_ptr() as usize,
                10,
                dir_b,
                "renamed.txt".as_ptr() as usize,
                11,
            ),
            Ok(0)
        );
        assert_eq!(
            truncate_path_at_syscall(dir_b, "renamed.txt".as_ptr() as usize, 11, 6),
            Ok(0)
        );
        assert_eq!(
            chmod_path_at_syscall(dir_b, "renamed.txt".as_ptr() as usize, 11, 0o640),
            Ok(0)
        );
        let root_identity = NativeProcessIdentityRecord {
            uid: 0,
            gid: 0,
            umask: 0,
            supplemental_count: 0,
            supplemental_gids: [0; 8],
        };
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_process_identity_syscall(1, &root_identity as *const _),
            Ok(0)
        );
        assert_eq!(
            chown_path_at_syscall(dir_b, "renamed.txt".as_ptr() as usize, 11, 55, 77),
            Ok(0)
        );
        assert_eq!(
            symlink_path_at_syscall(
                dir_b,
                "alias".as_ptr() as usize,
                5,
                "../a/data.txt".as_ptr() as usize,
                13,
            ),
            Ok(0)
        );

        let mut status = NativeFileStatusRecord {
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
        assert_eq!(
            stat_path_at_syscall(
                dir_b,
                "renamed.txt".as_ptr() as usize,
                11,
                &mut status as *mut _,
            ),
            Ok(0)
        );
        assert_eq!(status.size, 6);
        assert_eq!(status.owner_uid, 55);
        assert_eq!(status.group_gid, 77);
        assert_eq!(status.mode & 0o777, 0o640);
        assert_eq!(status.link_count, 2);

        let mut alias_status = NativeFileStatusRecord {
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
        assert_eq!(
            lstat_path_at_syscall(
                dir_b,
                "alias".as_ptr() as usize,
                5,
                &mut alias_status as *mut _
            ),
            Ok(0)
        );
        assert_eq!(alias_status.kind, NativeObjectKind::Symlink as u32);

        let mut alias_target = [0u8; 64];
        let alias_len = readlink_path_at_syscall(
            dir_b,
            "alias".as_ptr() as usize,
            5,
            alias_target.as_mut_ptr(),
            alias_target.len(),
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(&alias_target[..alias_len]).unwrap(),
            "../a/data.txt"
        );

        assert_eq!(
            unlink_path_at_syscall(dir_b, "renamed.txt".as_ptr() as usize, 11),
            Ok(0)
        );
        let source_status = boot_vfs_stat("/scope/a/data.txt").unwrap();
        assert_eq!(source_status.link_count, 1);
        assert!(boot_vfs_stat("/scope/b/renamed.txt").is_none());
        assert_eq!(close_syscall(dir_a), Ok(0));
        assert_eq!(close_syscall(dir_b), Ok(0));
    }

    #[test]
    fn boot_vfs_spawn_inherits_non_cloexec_descriptors_and_procfs_exposes_child_fd_table() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/inherit", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/inherit/keep.txt", BootNodeKind::File).unwrap();
        boot_vfs_create("/inherit/drop.txt", BootNodeKind::File).unwrap();

        let keep_fd = open_path_syscall(
            "/inherit/keep.txt".as_ptr() as usize,
            "/inherit/keep.txt".len(),
        )
        .unwrap();
        let drop_fd = open_path_syscall(
            "/inherit/drop.txt".as_ptr() as usize,
            "/inherit/drop.txt".len(),
        )
        .unwrap();

        assert_eq!(fcntl_syscall(drop_fd, 3 | (1usize << 8)), Ok(0b10));
        assert_eq!(fcntl_syscall(keep_fd, 2 | (1usize << 8)), Ok(0b1));
        assert_eq!(seek_syscall(keep_fd, 5, SeekWhence::Set as u32), Ok(5));

        let child = spawn_path_process_syscall(
            b"child-fd".as_ptr() as usize,
            8,
            b"/bin/app".as_ptr() as usize,
            8,
        )
        .unwrap();

        BootVfs::set_current_subject(0, 0);
        let listing = boot_procfs_fd_listing(child as u64).unwrap();
        assert!(listing.contains("0\tstdin\tFile"), "{listing}");
        assert!(listing.contains("1\tstdout\tFile"), "{listing}");
        assert!(listing.contains("2\tstderr\tFile"), "{listing}");
        assert!(listing.contains("/inherit/keep.txt"), "{listing}");
        assert!(!listing.contains("/inherit/drop.txt"), "{listing}");
        assert!(listing.contains("nonblock=true"), "{listing}");

        let fdinfo = boot_procfs_fdinfo(child as u64, keep_fd as u64).unwrap();
        assert!(fdinfo.contains("path:\t/inherit/keep.txt"), "{fdinfo}");
        assert!(fdinfo.contains("pos:\t5"), "{fdinfo}");
        assert!(fdinfo.contains("cloexec=false nonblock=true"), "{fdinfo}");

        BootVfs::set_current_subject(0, 0);
        let mut process = NativeProcessRecord {
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
        assert_eq!(
            inspect_process_syscall(child, &mut process as *mut _),
            Ok(0)
        );
        assert_eq!(process.descriptor_count, 4);
        assert_eq!(
            boot_procfs_fdinfo(child as u64, drop_fd as u64),
            Err(Errno::NoEnt)
        );
        BootVfs::set_current_subject(1000, 1000);
    }

    #[test]
    fn boot_vfs_procfs_sensitive_cross_process_views_require_root_or_observe_contract_and_global_nodes_are_pid1_only()
     {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/procfs-guard";
        let image = b"/procfs-guard/worker.bin";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(image.as_ptr() as usize, image.len()),
            Ok(0)
        );
        let argv = b"/procfs-guard/worker.bin\0";
        let envp = b"SECRET_ENV=yes\0";
        let config = NativeSpawnProcessConfig {
            name_ptr: b"procfs-guard".as_ptr() as usize,
            name_len: 12,
            path_ptr: image.as_ptr() as usize,
            path_len: image.len(),
            cwd_ptr: root.as_ptr() as usize,
            cwd_len: root.len(),
            argv_ptr: argv.as_ptr() as usize,
            argv_len: argv.len(),
            argv_count: 1,
            envp_ptr: envp.as_ptr() as usize,
            envp_len: envp.len(),
            envp_count: 1,
        };
        let child = spawn_configured_process_syscall(&config as *const _ as usize).unwrap();

        let mut buffer = [0u8; 512];
        let environ_path = format!("/proc/{child}/environ");
        assert_eq!(
            read_procfs_syscall(
                environ_path.as_ptr() as usize,
                environ_path.len(),
                buffer.as_mut_ptr(),
                buffer.len(),
            ),
            Err(Errno::Access)
        );

        let fd_path = format!("/proc/{child}/fd");
        assert_eq!(
            read_procfs_syscall(
                fd_path.as_ptr() as usize,
                fd_path.len(),
                buffer.as_mut_ptr(),
                buffer.len(),
            ),
            Err(Errno::Access)
        );

        assert_eq!(
            read_procfs_syscall(
                b"/proc/2/vfsstats".as_ptr() as usize,
                b"/proc/2/vfsstats".len(),
                buffer.as_mut_ptr(),
                buffer.len(),
            ),
            Err(Errno::NoEnt)
        );

        let maps_path = format!("/proc/{child}/maps");
        assert_eq!(
            read_procfs_syscall(
                maps_path.as_ptr() as usize,
                maps_path.len(),
                buffer.as_mut_ptr(),
                buffer.len(),
            ),
            Err(Errno::Access)
        );

        let observe_domain =
            create_domain_syscall(0, b"boot-vm-observe".as_ptr() as usize, 15).unwrap();
        let observe_resource = create_resource_syscall(
            observe_domain,
            NativeResourceKind::Namespace as u32,
            b"boot-vm-observe".as_ptr() as usize,
            15,
        )
        .unwrap();
        assert_eq!(
            set_resource_contract_policy_syscall(
                observe_resource,
                NativeResourceContractPolicy::Observe as u32
            ),
            Ok(0)
        );
        let observe_contract = create_contract_syscall(
            observe_domain,
            observe_resource,
            NativeContractKind::Observe as u32,
            b"boot-vm-observe".as_ptr() as usize,
            15,
        )
        .unwrap();
        assert_eq!(bind_process_contract_syscall(observe_contract), Ok(0));

        let maps_count = read_procfs_syscall(
            maps_path.as_ptr() as usize,
            maps_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let maps_text = core::str::from_utf8(&buffer[..maps_count]).unwrap();
        assert!(maps_text.contains("[heap]"), "{maps_text}");

        BootVfs::set_current_subject(0, 0);
        let count = read_procfs_syscall(
            environ_path.as_ptr() as usize,
            environ_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("SECRET_ENV=yes"), "{text}");
        let count = read_procfs_syscall(
            b"/proc/1/vfsstats".as_ptr() as usize,
            b"/proc/1/vfsstats".len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        assert!(count > 0);
    }

    #[test]
    fn boot_vfs_procfs_cross_process_status_respects_subject_label_visibility() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let image = b"/bin/secret-task";
        let child = spawn_path_process_syscall(
            b"secret-task".as_ptr() as usize,
            11,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();
        let secret = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_process_security_label_syscall(child, &secret as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let status_path = format!("/proc/{child}/status");
        let mut buffer = [0u8; 512];
        assert_eq!(
            read_procfs_syscall(
                status_path.as_ptr() as usize,
                status_path.len(),
                buffer.as_mut_ptr(),
                buffer.len(),
            ),
            Err(Errno::Access)
        );

        let raised = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_process_security_label_syscall(1, &raised as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let count = read_procfs_syscall(
            status_path.as_ptr() as usize,
            status_path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("SubjectLabel:\tSecret/Verified"), "{text}");
    }

    #[test]
    fn boot_vfs_procfs_directory_listing_hides_invisible_processes_and_sensitive_subdirs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let image = b"/bin/proc-list-guard";
        let child = spawn_path_process_syscall(
            b"proc-list-guard".as_ptr() as usize,
            15,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        BootVfs::set_current_subject(0, 0);
        let foreign_identity = NativeProcessIdentityRecord {
            uid: 2001,
            gid: 2001,
            umask: 0o022,
            supplemental_count: 0,
            supplemental_gids: [0; 8],
        };
        assert_eq!(
            set_process_identity_syscall(child, &foreign_identity as *const _),
            Ok(0)
        );
        let secret = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(child, &secret as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let proc_listing = list_path_text("/proc");
        assert!(proc_listing.contains("1\tDirectory"), "{proc_listing}");
        assert!(
            !proc_listing.contains(&format!("{child}\tDirectory")),
            "{proc_listing}"
        );

        let fd_path = format!("/proc/{child}/fd");
        let mut buffer = [0u8; 64];
        assert_eq!(
            list_path_syscall(
                fd_path.as_ptr() as usize,
                fd_path.len(),
                buffer.as_mut_ptr(),
                buffer.len(),
            ),
            Err(Errno::Access)
        );

        BootVfs::set_current_subject(0, 0);
        let raised = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &raised as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let proc_listing = list_path_text("/proc");
        assert!(
            proc_listing.contains(&format!("{child}\tDirectory")),
            "{proc_listing}"
        );

        BootVfs::set_current_subject(0, 0);
        let fd_listing = list_path_text(&fd_path);
        assert!(fd_listing.contains("0\tFile"), "{fd_listing}");
        assert!(fd_listing.contains("1\tFile"), "{fd_listing}");
        assert!(fd_listing.contains("2\tFile"), "{fd_listing}");
        let process_listing = list_path_text(&format!("/proc/{child}"));
        assert!(
            process_listing.contains("fd\tDirectory"),
            "{process_listing}"
        );
        assert!(
            process_listing.contains("fdinfo\tDirectory"),
            "{process_listing}"
        );
        BootVfs::set_current_subject(1000, 1000);
    }

    #[test]
    fn boot_vfs_direct_process_inspection_matches_procfs_isolation_policy() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let image = b"/bin/inspectee";
        let child = spawn_path_process_syscall(
            b"inspectee".as_ptr() as usize,
            9,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        let mut process = NativeProcessRecord {
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
        assert_eq!(
            inspect_process_syscall(child, &mut process as *mut _),
            Ok(0)
        );

        let mut path = [0u8; 64];
        assert!(get_process_image_path_syscall(child, path.as_mut_ptr(), path.len()).is_ok());

        let mut identity = NativeProcessIdentityRecord::default();
        assert_eq!(
            inspect_process_identity_syscall(child, &mut identity as *mut _),
            Err(Errno::Access)
        );

        let mut label = SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
        assert_eq!(
            inspect_process_security_label_syscall(child, &mut label as *mut _),
            Err(Errno::Access)
        );

        let mut compat = NativeProcessCompatRecord {
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
        assert_eq!(
            inspect_process_compat_syscall(child, &mut compat as *mut _),
            Ok(0)
        );
        assert_eq!(compat.pid, child as u64);

        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            inspect_process_identity_syscall(child, &mut identity as *mut _),
            Ok(0)
        );
        assert_eq!(
            inspect_process_security_label_syscall(child, &mut label as *mut _),
            Ok(0)
        );
        assert_eq!(
            inspect_process_compat_syscall(child, &mut compat as *mut _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);
    }

    #[test]
    fn boot_vfs_event_watch_respects_object_label_visibility_until_subject_recovers() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let file = b"/secret-watch.txt";
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );

        let secret_label =
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &secret_label),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let watch = NativeVfsEventWatchConfig {
            token: 77,
            poll_events: POLLIN,
            subtree: 0,
            created: 0,
            opened: 1,
            closed: 0,
            written: 1,
            renamed: 0,
            unlinked: 0,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 0,
            lock_refused: 0,
            permission_refused: 0,
            truncated: 0,
            linked: 0,
        };
        assert_eq!(
            watch_vfs_events_syscall(
                queue_fd,
                file.as_ptr() as usize,
                file.len(),
                &watch as *const _
            ),
            Err(Errno::Access)
        );

        BootVfs::set_current_subject(0, 0);
        let raised = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_process_security_label_syscall(1, &raised as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);
        assert_eq!(
            watch_vfs_events_syscall(
                queue_fd,
                file.as_ptr() as usize,
                file.len(),
                &watch as *const _
            ),
            Ok(0)
        );

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
        assert_eq!(
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()),
            Err(Errno::Again)
        );

        BootVfs::set_current_subject(0, 0);
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();
        assert_eq!(write_syscall(fd, b"y".as_ptr(), 1), Ok(1));
        assert_eq!(close_syscall(fd), Ok(0));
        BootVfs::set_current_subject(1000, 1000);

        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert!(count >= 2, "{count}");
        let kinds = events[..count]
            .iter()
            .map(|event| event.detail0)
            .collect::<Vec<_>>();
        assert!(
            kinds.contains(&(NativeVfsEventKind::Opened as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Written as u32)),
            "{kinds:?}"
        );
    }

    #[test]
    fn boot_vfs_procfs_exposes_watch_registry_and_filtered_event_counters() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/watch-scope";
        let file = b"/watch-scope/secret.txt";
        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(file.as_ptr() as usize, file.len()),
            Ok(0)
        );
        let fd = open_path_syscall(file.as_ptr() as usize, file.len()).unwrap();

        let secret_label =
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_path_security_label_syscall(file.as_ptr() as usize, file.len(), &secret_label),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let watch = NativeVfsEventWatchConfig {
            token: 91,
            poll_events: POLLIN,
            subtree: 1,
            created: 0,
            opened: 1,
            closed: 0,
            written: 1,
            renamed: 0,
            unlinked: 0,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 0,
            lock_refused: 0,
            permission_refused: 0,
            truncated: 0,
            linked: 0,
        };
        assert_eq!(
            watch_vfs_events_syscall(queue_fd, root.as_ptr() as usize, root.len(), &watch),
            Ok(0)
        );

        BootVfs::set_current_subject(0, 0);
        let admin_listing = read_procfs_text("/proc/1/vfswatches");
        assert!(admin_listing.contains("queue="), "{admin_listing}");
        assert!(admin_listing.contains("owner-pid=1"), "{admin_listing}");
        assert!(
            admin_listing.contains("path=/watch-scope"),
            "{admin_listing}"
        );
        assert!(admin_listing.contains("subtree=true"), "{admin_listing}");
        assert!(admin_listing.contains("pending=0"), "{admin_listing}");
        BootVfs::set_current_subject(1000, 1000);

        let mut restricted = [0u8; 64];
        assert_eq!(
            read_procfs_syscall(
                b"/proc/2/vfswatches".as_ptr() as usize,
                b"/proc/2/vfswatches".len(),
                restricted.as_mut_ptr(),
                restricted.len(),
            ),
            Err(Errno::NoEnt)
        );

        assert_eq!(write_syscall(fd, b"z".as_ptr(), 1), Ok(1));
        assert_eq!(close_syscall(fd), Ok(0));

        let stats = read_procfs_text("/proc/1/vfsstats");
        assert!(
            parse_procfs_counter(&stats, "vfs-events-filtered=") >= 1,
            "{stats}"
        );
    }

    #[test]
    fn boot_vfs_event_queue_coalesces_repeated_object_events_and_reports_pending_peak() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(mkdir_path_syscall(b"/coalesce".as_ptr() as usize, 9), Ok(0));
        assert_eq!(
            mkfile_path_syscall(b"/coalesce/file.txt".as_ptr() as usize, 18),
            Ok(0)
        );

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let watch = NativeVfsEventWatchConfig {
            token: 123,
            poll_events: POLLIN,
            subtree: 0,
            created: 0,
            opened: 1,
            closed: 1,
            written: 1,
            renamed: 0,
            unlinked: 0,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 0,
            lock_refused: 0,
            permission_refused: 0,
            truncated: 0,
            linked: 0,
        };
        assert_eq!(
            watch_vfs_events_syscall(
                queue_fd,
                b"/coalesce/file.txt".as_ptr() as usize,
                18,
                &watch
            ),
            Ok(0)
        );

        for _ in 0..48 {
            let fd = open_path_syscall(b"/coalesce/file.txt".as_ptr() as usize, 18).unwrap();
            assert_eq!(write_syscall(fd, b"x".as_ptr(), 1), Ok(1));
            assert_eq!(close_syscall(fd), Ok(0));
        }

        let listing = read_procfs_text("/proc/1/vfswatches");
        assert!(listing.contains("path=/coalesce/file.txt"), "{listing}");
        assert!(listing.contains("pending=3"), "{listing}");
        assert!(listing.contains("peak=3"), "{listing}");

        let stats = read_procfs_text("/proc/1/vfsstats");
        assert!(
            parse_procfs_counter(&stats, "vfs-events-coalesced=") >= 100,
            "{stats}"
        );
        assert_eq!(
            parse_procfs_counter(&stats, "vfs-event-queue-overflows="),
            0,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "vfs-pending-peak=") >= 3,
            "{stats}"
        );

        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 8];
        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert_eq!(count, 3, "{count}");
        let kinds = events[..count]
            .iter()
            .map(|event| event.detail0)
            .collect::<Vec<_>>();
        assert!(
            kinds.contains(&(NativeVfsEventKind::Opened as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Written as u32)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&(NativeVfsEventKind::Closed as u32)),
            "{kinds:?}"
        );
    }

    #[test]
    fn boot_vfs_directory_listing_filters_secret_children_and_cache_tracks_subject_label() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(
            mkdir_path_syscall(b"/list-guard".as_ptr() as usize, 11),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(b"/list-guard/public.txt".as_ptr() as usize, 22),
            Ok(0)
        );
        assert_eq!(
            mkfile_path_syscall(b"/list-guard/secret.txt".as_ptr() as usize, 22),
            Ok(0)
        );

        let secret_label =
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_path_security_label_syscall(
                b"/list-guard/secret.txt".as_ptr() as usize,
                22,
                &secret_label
            ),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let public_listing = list_path_text("/list-guard");
        assert!(
            public_listing.contains("public.txt\tFile"),
            "{public_listing}"
        );
        assert!(
            !public_listing.contains("secret.txt\tFile"),
            "{public_listing}"
        );

        BootVfs::set_current_subject(0, 0);
        let raised = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &raised as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let secret_listing = list_path_text("/list-guard");
        assert!(
            secret_listing.contains("public.txt\tFile"),
            "{secret_listing}"
        );
        assert!(
            secret_listing.contains("secret.txt\tFile"),
            "{secret_listing}"
        );

        BootVfs::set_current_subject(0, 0);
        let lowered = SecurityLabel::new(ConfidentialityLevel::Public, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &lowered as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let recovered_listing = list_path_text("/list-guard");
        assert!(
            recovered_listing.contains("public.txt\tFile"),
            "{recovered_listing}"
        );
        assert!(
            !recovered_listing.contains("secret.txt\tFile"),
            "{recovered_listing}"
        );
    }

    #[test]
    fn boot_vfs_procfs_directories_are_first_class_for_stat_open_and_list_at() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let image = b"/bin/proc-dir-test";
        let child = spawn_path_process_syscall(
            b"proc-dir-test".as_ptr() as usize,
            13,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        let mut proc_status = NativeFileStatusRecord {
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
        assert_eq!(
            stat_path_syscall(b"/proc".as_ptr() as usize, 5, &mut proc_status as *mut _),
            Ok(0)
        );
        assert_eq!(proc_status.kind, NativeObjectKind::Directory as u32);
        assert_eq!(proc_status.mode, 0o555);

        let proc_fd = open_path_syscall(b"/proc".as_ptr() as usize, 5).unwrap();
        let mut proc_listing = [0u8; 512];
        let proc_count = list_path_at_syscall(
            proc_fd,
            "".as_ptr() as usize,
            0,
            proc_listing.as_mut_ptr(),
            proc_listing.len(),
        )
        .unwrap();
        let proc_listing = core::str::from_utf8(&proc_listing[..proc_count]).unwrap();
        assert!(proc_listing.contains("1\tDirectory"), "{proc_listing}");
        assert!(
            proc_listing.contains(&format!("{child}\tDirectory")),
            "{proc_listing}"
        );

        let child_name = format!("{child}");
        let mut child_status = NativeFileStatusRecord {
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
        assert_eq!(
            stat_path_at_syscall(
                proc_fd,
                child_name.as_ptr() as usize,
                child_name.len(),
                &mut child_status as *mut _
            ),
            Ok(0)
        );
        assert_eq!(child_status.kind, NativeObjectKind::Directory as u32);

        let child_path = format!("/proc/{child}");
        let child_fd = open_path_syscall(child_path.as_ptr() as usize, child_path.len()).unwrap();
        let mut child_listing = [0u8; 512];
        let child_count = list_path_at_syscall(
            child_fd,
            "".as_ptr() as usize,
            0,
            child_listing.as_mut_ptr(),
            child_listing.len(),
        )
        .unwrap();
        let child_listing = core::str::from_utf8(&child_listing[..child_count]).unwrap();
        assert!(child_listing.contains("fd\tDirectory"), "{child_listing}");
        assert!(child_listing.contains("status\tFile"), "{child_listing}");

        BootVfs::set_current_subject(0, 0);
        let fd_dir_fd = open_path_at_syscall(child_fd, "fd".as_ptr() as usize, 2).unwrap();
        let mut fd_listing = [0u8; 256];
        let fd_count = list_path_at_syscall(
            fd_dir_fd,
            "".as_ptr() as usize,
            0,
            fd_listing.as_mut_ptr(),
            fd_listing.len(),
        )
        .unwrap();
        let fd_listing = core::str::from_utf8(&fd_listing[..fd_count]).unwrap();
        assert!(fd_listing.contains("0\tFile"), "{fd_listing}");
        assert!(fd_listing.contains("1\tFile"), "{fd_listing}");
        assert!(fd_listing.contains("2\tFile"), "{fd_listing}");

        assert_eq!(close_syscall(fd_dir_fd), Ok(0));
        BootVfs::set_current_subject(1000, 1000);
        assert_eq!(close_syscall(child_fd), Ok(0));
        assert_eq!(close_syscall(proc_fd), Ok(0));
    }

    #[test]
    fn boot_vfs_list_processes_hides_cross_process_secret_entries_until_label_recovers() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let image = b"/bin/hidden-task";
        let child = spawn_path_process_syscall(
            b"hidden-task".as_ptr() as usize,
            11,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        BootVfs::set_current_subject(0, 0);
        let foreign_identity = NativeProcessIdentityRecord {
            uid: 2001,
            gid: 2001,
            umask: 0o022,
            supplemental_count: 0,
            supplemental_gids: [0; 8],
        };
        assert_eq!(
            set_process_identity_syscall(child, &foreign_identity as *const _),
            Ok(0)
        );
        let secret = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(child, &secret as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let mut ids = [0u64; 16];
        let count = list_processes_syscall(ids.as_mut_ptr(), ids.len()).unwrap();
        let visible = &ids[..count.min(ids.len())];
        assert!(visible.contains(&1), "{visible:?}");
        assert!(!visible.contains(&(child as u64)), "{visible:?}");

        BootVfs::set_current_subject(0, 0);
        let raised = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &raised as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let count = list_processes_syscall(ids.as_mut_ptr(), ids.len()).unwrap();
        let visible = &ids[..count.min(ids.len())];
        assert!(visible.contains(&(child as u64)), "{visible:?}");
    }

    #[test]
    fn boot_vfs_list_processes_keeps_same_owner_visible_even_when_label_is_higher() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let image = b"/bin/owned-task";
        let child = spawn_path_process_syscall(
            b"owned-task".as_ptr() as usize,
            10,
            image.as_ptr() as usize,
            image.len(),
        )
        .unwrap();

        BootVfs::set_current_subject(0, 0);
        let owner_identity = NativeProcessIdentityRecord {
            uid: 1000,
            gid: 1000,
            umask: 0o022,
            supplemental_count: 0,
            supplemental_gids: [0; 8],
        };
        assert_eq!(
            set_process_identity_syscall(child, &owner_identity as *const _),
            Ok(0)
        );
        let secret = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(child, &secret as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let mut ids = [0u64; 16];
        let count = list_processes_syscall(ids.as_mut_ptr(), ids.len()).unwrap();
        let visible = &ids[..count.min(ids.len())];
        assert!(visible.contains(&(child as u64)), "{visible:?}");
    }

    #[test]
    fn boot_vfs_reap_process_reclaims_heavy_state_and_updates_vfs_stats() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/reap", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/reap/keep.txt", BootNodeKind::File).unwrap();

        let keep_fd =
            open_path_syscall("/reap/keep.txt".as_ptr() as usize, "/reap/keep.txt".len()).unwrap();

        let argv = b"/reap/worker.bin\0--flag\0";
        let envp = b"ALPHA=1\0BETA=2\0";
        let config = NativeSpawnProcessConfig {
            name_ptr: b"reap-worker".as_ptr() as usize,
            name_len: 11,
            path_ptr: b"/reap/worker.bin".as_ptr() as usize,
            path_len: 15,
            cwd_ptr: b"/reap".as_ptr() as usize,
            cwd_len: 5,
            argv_ptr: argv.as_ptr() as usize,
            argv_len: argv.len(),
            argv_count: 2,
            envp_ptr: envp.as_ptr() as usize,
            envp_len: envp.len(),
            envp_count: 2,
        };
        let child = spawn_configured_process_syscall(&config as *const _ as usize).unwrap();

        BootVfs::set_current_subject(0, 0);
        let mut process = NativeProcessRecord {
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
        assert_eq!(
            inspect_process_syscall(child, &mut process as *mut _),
            Ok(0)
        );
        assert!(process.descriptor_count >= 4, "{process:?}");
        assert_eq!(process.environment_count, 2);
        assert_eq!(process.memory_region_count, 1);
        assert!(read_procfs_text("/proc/1/vfsstats").contains("process-reaps=0"));

        assert_eq!(send_signal_syscall(child, 9), Ok(0));
        assert_eq!(reap_process_syscall(child), Ok(137));
        let status_path = format!("/proc/{child}/status");
        let mut status_buffer = [0u8; 64];
        assert_eq!(
            read_procfs_syscall(
                status_path.as_ptr() as usize,
                status_path.len(),
                status_buffer.as_mut_ptr(),
                status_buffer.len()
            ),
            Err(Errno::Srch)
        );
        assert_eq!(
            inspect_process_syscall(child, &mut process as *mut _),
            Err(Errno::Srch)
        );
        let listing = list_path_text("/proc");
        assert!(
            !listing.contains(&format!("{child}\tDirectory")),
            "{listing}"
        );

        let stats = read_procfs_text("/proc/1/vfsstats");
        assert_eq!(parse_procfs_counter(&stats, "process-reaps="), 1);
        assert!(
            parse_procfs_counter(&stats, "reaped-descriptors=") >= 4,
            "{stats}"
        );
        assert_eq!(parse_procfs_counter(&stats, "reaped-env="), 2, "{stats}");
        assert_eq!(
            parse_procfs_counter(&stats, "reaped-vm-objects="),
            1,
            "{stats}"
        );

        BootVfs::set_current_subject(1000, 1000);
        assert_eq!(close_syscall(keep_fd), Ok(0));
    }

    #[test]
    fn boot_vfs_reap_refuses_running_process_and_recovers_process_slots_under_pressure() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let mut children = Vec::new();
        for _ in 0..(MAX_PROCESS_COUNT - 1) {
            let child = spawn_path_process_syscall(
                b"slot-child".as_ptr() as usize,
                10,
                b"/bin/app".as_ptr() as usize,
                8,
            )
            .unwrap();
            children.push(child);
        }

        assert_eq!(
            spawn_path_process_syscall(
                b"overflow".as_ptr() as usize,
                8,
                b"/bin/app".as_ptr() as usize,
                8
            ),
            Err(Errno::Again)
        );

        let victim = children[0];
        assert_eq!(reap_process_syscall(victim), Err(Errno::Again));
        assert_eq!(send_signal_syscall(victim, 15), Ok(0));
        assert_eq!(reap_process_syscall(victim), Ok(143));

        let recovered = spawn_path_process_syscall(
            b"recovered".as_ptr() as usize,
            9,
            b"/bin/app".as_ptr() as usize,
            8,
        )
        .unwrap();
        assert!(recovered >= 2);

        let stats = read_procfs_text("/proc/1/vfsstats");
        assert_eq!(parse_procfs_counter(&stats, "process-reaps="), 1);
    }

    #[test]
    fn signal_syscalls_report_pending_and_blocked_lists_for_shell_observability() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let child = spawn_path_process_syscall(
            b"signal-child".as_ptr() as usize,
            12,
            b"/bin/app".as_ptr() as usize,
            8,
        )
        .unwrap();

        let mut pending = [0u8; 8];
        assert_eq!(
            pending_signals_syscall(child as usize, pending.as_mut_ptr(), pending.len()),
            Ok(0)
        );

        assert_eq!(send_signal_syscall(child, 9), Ok(0));
        assert_eq!(
            pending_signals_syscall(child as usize, pending.as_mut_ptr(), pending.len()),
            Ok(0)
        );

        let mut blocked = [0u8; 8];
        assert_eq!(
            blocked_pending_signals_syscall(child as usize, blocked.as_mut_ptr(), blocked.len()),
            Ok(0)
        );
    }

    #[test]
    fn boot_vfs_empty_at_path_uses_handle_capability_for_deleted_file_lifecycle() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/cap-handle", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/cap-handle/live.txt", BootNodeKind::File).unwrap();

        let dir_fd =
            open_path_syscall("/cap-handle".as_ptr() as usize, "/cap-handle".len()).unwrap();
        let file_fd =
            open_path_at_syscall(dir_fd, "live.txt".as_ptr() as usize, "live.txt".len()).unwrap();
        assert_eq!(write_syscall(file_fd, b"abcdef".as_ptr(), 6), Ok(6));

        assert_eq!(
            unlink_path_at_syscall(dir_fd, "live.txt".as_ptr() as usize, "live.txt".len()),
            Ok(0)
        );
        assert!(boot_vfs_stat("/cap-handle/live.txt").is_none());

        let mut status = NativeFileStatusRecord {
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
        assert_eq!(
            stat_path_at_syscall(file_fd, "".as_ptr() as usize, 0, &mut status as *mut _),
            Ok(0)
        );
        assert_eq!(status.size, 6);
        assert_eq!(status.link_count, 0);

        assert_eq!(
            truncate_path_at_syscall(file_fd, "".as_ptr() as usize, 0, 4),
            Ok(0)
        );
        let root_identity = NativeProcessIdentityRecord {
            uid: 0,
            gid: 0,
            umask: 0,
            supplemental_count: 0,
            supplemental_gids: [0; 8],
        };
        BootVfs::set_current_subject(0, 0);
        assert_eq!(
            set_process_identity_syscall(1, &root_identity as *const _),
            Ok(0)
        );
        assert_eq!(
            chmod_path_at_syscall(file_fd, "".as_ptr() as usize, 0, 0o640),
            Ok(0)
        );
        assert_eq!(
            chown_path_at_syscall(file_fd, "".as_ptr() as usize, 0, 77, 88),
            Ok(0)
        );
        assert_eq!(
            link_path_at_syscall(
                file_fd,
                "".as_ptr() as usize,
                0,
                dir_fd,
                "restored.txt".as_ptr() as usize,
                "restored.txt".len(),
            ),
            Ok(0)
        );

        let restored = boot_vfs_stat("/cap-handle/restored.txt").unwrap();
        assert_eq!(restored.size, 4);
        assert_eq!(restored.mode & 0o777, 0o640);
        assert_eq!(restored.owner_uid, 77);
        assert_eq!(restored.group_gid, 88);
        assert_eq!(restored.link_count, 1);

        let dup_fd = open_path_at_syscall(file_fd, "".as_ptr() as usize, 0).unwrap();
        assert_eq!(seek_syscall(file_fd, 1, SeekWhence::Set as u32), Ok(1));
        assert_eq!(seek_syscall(dup_fd, 0, SeekWhence::Cur as u32), Ok(1));
        assert_eq!(close_syscall(dup_fd), Ok(0));
        assert_eq!(close_syscall(file_fd), Ok(0));
        assert_eq!(close_syscall(dir_fd), Ok(0));
    }

    #[test]
    fn boot_vfs_empty_at_path_lists_directory_handle_after_rename() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/cap-dir", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/cap-dir/child.txt", BootNodeKind::File).unwrap();

        let dir_fd = open_path_syscall("/cap-dir".as_ptr() as usize, "/cap-dir".len()).unwrap();
        assert_eq!(
            rename_path_syscall(
                "/cap-dir".as_ptr() as usize,
                "/cap-dir".len(),
                "/cap-dir-renamed".as_ptr() as usize,
                "/cap-dir-renamed".len(),
            ),
            Ok(0)
        );

        let mut listing = [0u8; 128];
        let listed = list_path_at_syscall(
            dir_fd,
            "".as_ptr() as usize,
            0,
            listing.as_mut_ptr(),
            listing.len(),
        )
        .unwrap();
        let listing = core::str::from_utf8(&listing[..listed]).unwrap();
        assert!(listing.contains("child.txt\tFile"), "{listing}");

        let mut status = NativeFileStatusRecord {
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
        assert_eq!(
            stat_path_at_syscall(dir_fd, "".as_ptr() as usize, 0, &mut status as *mut _),
            Ok(0)
        );
        assert_eq!(status.kind, NativeObjectKind::Directory as u32);
        assert_eq!(close_syscall(dir_fd), Ok(0));
    }

    #[test]
    fn boot_vfs_procfs_exposes_live_locks_and_contention_counters() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/obs", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/obs/sub", BootNodeKind::Directory).unwrap();
        boot_vfs_create("/obs/data.txt", BootNodeKind::File).unwrap();
        boot_vfs_create("/obs/sub/tree.txt", BootNodeKind::File).unwrap();

        let dir_fd = open_path_syscall("/obs".as_ptr() as usize, "/obs".len()).unwrap();
        let file_fd =
            open_path_syscall("/obs/data.txt".as_ptr() as usize, "/obs/data.txt".len()).unwrap();
        let child_fd = open_path_syscall(
            "/obs/sub/tree.txt".as_ptr() as usize,
            "/obs/sub/tree.txt".len(),
        )
        .unwrap();

        assert_eq!(fcntl_syscall(file_fd, 5 | (0x22usize << 8)), Ok(0x22));
        assert_eq!(
            unlink_path_syscall("/obs/data.txt".as_ptr() as usize, "/obs/data.txt".len()),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(dir_fd, 5 | (0x11usize << 8)), Ok(0x11));
        assert_eq!(
            mkfile_path_syscall("/obs/new.txt".as_ptr() as usize, "/obs/new.txt".len()),
            Err(Errno::Busy)
        );
        assert_eq!(fcntl_syscall(dir_fd, 6 | (0x11usize << 8)), Ok(0x11));
        assert_eq!(fcntl_syscall(child_fd, 5 | (0x33usize << 8)), Ok(0x33));
        assert_eq!(
            rename_path_syscall(
                "/obs/sub".as_ptr() as usize,
                "/obs/sub".len(),
                "/obs-sub".as_ptr() as usize,
                "/obs-sub".len(),
            ),
            Err(Errno::Busy)
        );

        let locks = read_procfs_text("/proc/1/vfslocks");
        assert!(locks.contains("path=/obs"), "{locks}");
        assert!(locks.contains("path=/obs/data.txt"), "{locks}");
        assert!(locks.contains("path=/obs/sub/tree.txt"), "{locks}");
        assert!(locks.contains("mode=exclusive"), "{locks}");

        let stats = read_procfs_text("/proc/1/vfsstats");
        assert!(
            parse_procfs_counter(&stats, "object-conflicts=") >= 1,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "namespace-conflicts=") >= 1,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "subtree-conflicts=") >= 1,
            "{stats}"
        );
        assert!(parse_procfs_counter(&stats, "locks=") >= 2, "{stats}");
    }

    #[test]
    fn boot_vfs_combined_stress_updates_cache_event_and_lock_metrics_together() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        boot_vfs_create("/stress", BootNodeKind::Directory).unwrap();
        for index in 0..160 {
            let path = format!("/stress/file-{index:03}.txt");
            boot_vfs_create(&path, BootNodeKind::File).unwrap();
        }

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let watch = NativeVfsEventWatchConfig {
            token: 0x44,
            poll_events: POLLIN,
            subtree: 1,
            created: 1,
            opened: 1,
            closed: 1,
            written: 1,
            renamed: 1,
            unlinked: 1,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 1,
            lock_refused: 1,
            permission_refused: 0,
            truncated: 1,
            linked: 1,
        };
        assert_eq!(
            watch_vfs_events_syscall(
                queue_fd,
                "/stress".as_ptr() as usize,
                "/stress".len(),
                &watch
            ),
            Ok(0)
        );

        let dir_fd = open_path_syscall("/stress".as_ptr() as usize, "/stress".len()).unwrap();
        let hot_fd = open_path_syscall(
            "/stress/file-000.txt".as_ptr() as usize,
            "/stress/file-000.txt".len(),
        )
        .unwrap();
        let foreign_fd = open_path_syscall(
            "/stress/file-000.txt".as_ptr() as usize,
            "/stress/file-000.txt".len(),
        )
        .unwrap();
        assert_eq!(fcntl_syscall(hot_fd, 5 | (0x91usize << 8)), Ok(0x91));
        assert_eq!(
            write_syscall(foreign_fd, b"x".as_ptr(), 1),
            Err(Errno::Busy)
        );

        for index in 1..160 {
            let path = format!("/stress/file-{index:03}.txt");
            let _ = boot_vfs_stat(&path);
            let _ = boot_vfs_lstat(&path);
            let fd = open_path_syscall(path.as_ptr() as usize, path.len()).unwrap();
            assert_eq!(write_syscall(fd, b"payload".as_ptr(), 7), Ok(7));
            let mut bytes = [0u8; 7];
            assert_eq!(seek_syscall(fd, 0, SeekWhence::Set as u32), Ok(0));
            assert_eq!(read_syscall(fd, bytes.as_mut_ptr(), bytes.len()), Ok(7));
            assert_eq!(close_syscall(fd), Ok(0));
        }

        for index in 1..160 {
            let path = format!("/stress/file-{index:03}.txt");
            let _ = boot_vfs_stat(&path);
            let _ = boot_vfs_lstat(&path);
            let fd = open_path_syscall(path.as_ptr() as usize, path.len()).unwrap();
            let mut bytes = [0u8; 7];
            assert_eq!(read_syscall(fd, bytes.as_mut_ptr(), bytes.len()), Ok(7));
            assert_eq!(close_syscall(fd), Ok(0));
        }

        for _ in 0..48 {
            let mut listing = [0u8; 1024];
            let _ = list_path_syscall(
                "/stress".as_ptr() as usize,
                "/stress".len(),
                listing.as_mut_ptr(),
                listing.len(),
            );
        }

        assert_eq!(
            rename_path_at_syscall(
                dir_fd,
                "file-001.txt".as_ptr() as usize,
                "file-001.txt".len(),
                dir_fd,
                "file-001-renamed.txt".as_ptr() as usize,
                "file-001-renamed.txt".len(),
            ),
            Ok(0)
        );
        assert_eq!(
            truncate_path_at_syscall(
                dir_fd,
                "file-002.txt".as_ptr() as usize,
                "file-002.txt".len(),
                3,
            ),
            Ok(0)
        );
        assert_eq!(
            unlink_path_at_syscall(
                dir_fd,
                "file-003.txt".as_ptr() as usize,
                "file-003.txt".len()
            ),
            Ok(0)
        );

        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 64];
        let delivered =
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert!(delivered > 0);

        let stats = read_procfs_text("/proc/1/vfsstats");
        assert!(
            parse_procfs_counter(&stats, "lookup-misses=") > 0,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "lookup-evictions=") > 0,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "page-evictions=") > 0,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "stat-evictions=") > 0,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "vfs-events-delivered=") >= delivered as u64,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "vfs-event-queue-overflows=") > 0,
            "{stats}"
        );
        assert!(
            parse_procfs_counter(&stats, "object-conflicts=") >= 1,
            "{stats}"
        );
        assert!(parse_procfs_counter(&stats, "watches=") >= 1, "{stats}");
    }

    #[test]
    fn boot_vfs_handle_watch_registration_requires_current_object_visibility() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(
            mkfile_path_syscall(b"/handle-secret.txt".as_ptr() as usize, 18),
            Ok(0)
        );

        BootVfs::set_current_subject(0, 0);
        let file_fd = open_path_syscall(b"/handle-secret.txt".as_ptr() as usize, 18).unwrap();
        let secret_label =
            SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_path_security_label_syscall(
                b"/handle-secret.txt".as_ptr() as usize,
                18,
                &secret_label
            ),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        let watch = NativeVfsEventWatchConfig {
            token: 808,
            poll_events: POLLIN,
            subtree: 0,
            created: 0,
            opened: 1,
            closed: 0,
            written: 1,
            renamed: 0,
            unlinked: 0,
            mounted: 0,
            unmounted: 0,
            lock_acquired: 0,
            lock_refused: 0,
            permission_refused: 0,
            truncated: 0,
            linked: 0,
        };
        assert_eq!(
            watch_vfs_events_at_syscall(queue_fd, file_fd, "".as_ptr() as usize, 0, &watch),
            Err(Errno::Access)
        );

        BootVfs::set_current_subject(0, 0);
        let raised = SecurityLabel::new(ConfidentialityLevel::Secret, IntegrityLevel::Verified);
        assert_eq!(
            set_process_security_label_syscall(1, &raised as *const _),
            Ok(0)
        );
        BootVfs::set_current_subject(1000, 1000);

        assert_eq!(
            watch_vfs_events_at_syscall(queue_fd, file_fd, "".as_ptr() as usize, 0, &watch),
            Ok(0)
        );
        BootVfs::set_current_subject(0, 0);
        assert_eq!(write_syscall(file_fd, b"q".as_ptr(), 1), Ok(1));
        BootVfs::set_current_subject(1000, 1000);

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
        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert!(count >= 1, "{count}");
        assert!(
            events[..count]
                .iter()
                .any(|event| event.detail0 == NativeVfsEventKind::Written as u32),
            "{:?}",
            &events[..count]
        );

        BootVfs::set_current_subject(0, 0);
        assert_eq!(close_syscall(file_fd), Ok(0));
        BootVfs::set_current_subject(1000, 1000);
    }
}
