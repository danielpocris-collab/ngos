use super::*;

#[path = "syscall_surface/signal_memory.rs"]
mod signal_memory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnProcess {
    pub name: String,
    pub parent: Option<ProcessId>,
    pub class: SchedulerClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnProcessWithFiledesc {
    pub name: String,
    pub parent: Option<ProcessId>,
    pub class: SchedulerClass,
    pub source: ProcessId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnProcessWithVm {
    pub name: String,
    pub parent: Option<ProcessId>,
    pub class: SchedulerClass,
    pub source: ProcessId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnFiledescMode {
    Empty,
    Copy,
    Share,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnVmMode {
    Fresh,
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnProcessFromSource {
    pub name: String,
    pub parent: Option<ProcessId>,
    pub class: SchedulerClass,
    pub source: ProcessId,
    pub filedesc_mode: SpawnFiledescMode,
    pub vm_mode: SpawnVmMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetProcessArgs {
    pub pid: ProcessId,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetProcessEnv {
    pub pid: ProcessId,
    pub envp: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetProcessCwd {
    pub pid: ProcessId,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecProcess {
    pub pid: ProcessId,
    pub path: String,
    pub argv: Vec<String>,
    pub envp: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAnonymousMemory {
    pub pid: ProcessId,
    pub length: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmapMemory {
    pub pid: ProcessId,
    pub start: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetProcessBreak {
    pub pid: ProcessId,
    pub new_end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapFileMemory {
    pub pid: ProcessId,
    pub path: String,
    pub length: u64,
    pub file_offset: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub private: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectMemory {
    pub pid: ProcessId,
    pub start: u64,
    pub length: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdviseMemory {
    pub pid: ProcessId,
    pub start: u64,
    pub length: u64,
    pub advice: MemoryAdvice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMemory {
    pub pid: ProcessId,
    pub start: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TouchMemory {
    pub pid: ProcessId,
    pub start: u64,
    pub length: u64,
    pub write: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadMemoryWord {
    pub pid: ProcessId,
    pub addr: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareMemoryWord {
    pub pid: ProcessId,
    pub addr: u64,
    pub expected: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreMemoryWord {
    pub pid: ProcessId,
    pub addr: u64,
    pub value: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryWordUpdateOp {
    Set(u32),
    Add(u32),
    Or(u32),
    AndNot(u32),
    Xor(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryWordCompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWordWakeOpResult {
    pub old_value: u32,
    pub new_value: u32,
    pub comparison_matched: bool,
    pub woke_from: Vec<ProcessId>,
    pub woke_to: Vec<ProcessId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMemoryWord {
    pub pid: ProcessId,
    pub addr: u64,
    pub op: MemoryWordUpdateOp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendSignal {
    pub pid: ProcessId,
    pub signal: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendQueuedSignal {
    pub pid: ProcessId,
    pub signal: u8,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendThreadSignal {
    pub pid: ProcessId,
    pub tid: ThreadId,
    pub signal: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendQueuedThreadSignal {
    pub pid: ProcessId,
    pub tid: ThreadId,
    pub signal: u8,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalMaskHow {
    Set,
    Block,
    Unblock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSignalMask {
    pub pid: ProcessId,
    pub how: SignalMaskHow,
    pub mask: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSignalDisposition {
    pub pid: ProcessId,
    pub signal: u8,
    pub disposition: Option<SignalDisposition>,
    pub mask: u64,
    pub restart: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalActionState {
    pub disposition: Option<SignalDisposition>,
    pub mask: u64,
    pub restart: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakePendingSignal {
    pub pid: ProcessId,
    pub mask: u64,
    pub blocked_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitForPendingSignal {
    pub pid: ProcessId,
    pub mask: u64,
    pub timeout_ticks: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantCapability {
    pub owner: ProcessId,
    pub target: ObjectHandle,
    pub rights: CapabilityRights,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateCapability {
    pub capability: CapabilityId,
    pub new_owner: ProcessId,
    pub rights: CapabilityRights,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDomain {
    pub owner: ProcessId,
    pub parent: Option<DomainId>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateResource {
    pub creator: ProcessId,
    pub domain: DomainId,
    pub kind: ResourceKind,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateContract {
    pub issuer: ProcessId,
    pub domain: DomainId,
    pub resource: ResourceId,
    pub kind: ContractKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetContractState {
    pub id: ContractId,
    pub state: ContractState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvokeContract {
    pub id: ContractId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquireResourceViaContract {
    pub contract: ContractId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseResourceViaContract {
    pub contract: ContractId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferResourceViaContract {
    pub source: ContractId,
    pub target: ContractId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetResourceArbitrationPolicy {
    pub resource: ResourceId,
    pub policy: ResourceArbitrationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetResourceGovernanceMode {
    pub resource: ResourceId,
    pub mode: ResourceGovernanceMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetResourceContractPolicy {
    pub resource: ResourceId,
    pub policy: ResourceContractPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetResourceIssuerPolicy {
    pub resource: ResourceId,
    pub policy: ResourceIssuerPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetResourceState {
    pub resource: ResourceId,
    pub state: ResourceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimResourceViaContract {
    pub contract: ContractId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelResourceClaimViaContract {
    pub contract: ContractId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseClaimedResourceViaContract {
    pub contract: ContractId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    Runtime(RuntimeError),
    AccessDenied,
    InvalidArgument,
}

impl From<RuntimeError> for SyscallError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Syscall {
    SpawnProcess(SpawnProcess),
    SpawnProcessCopyFds(SpawnProcessWithFiledesc),
    SpawnProcessShareFds(SpawnProcessWithFiledesc),
    SpawnProcessCopyVm(SpawnProcessWithVm),
    SpawnProcessFromSource(SpawnProcessFromSource),
    SetProcessArgs(SetProcessArgs),
    SetProcessEnv(SetProcessEnv),
    SetProcessCwd(SetProcessCwd),
    ExecProcess(ExecProcess),
    MapAnonymousMemory(MapAnonymousMemory),
    MapFileMemory(MapFileMemory),
    UnmapMemory(UnmapMemory),
    ProtectMemory(ProtectMemory),
    AdviseMemory(AdviseMemory),
    SyncMemory(SyncMemory),
    TouchMemory(TouchMemory),
    LoadMemoryWord(LoadMemoryWord),
    CompareMemoryWord(CompareMemoryWord),
    StoreMemoryWord(StoreMemoryWord),
    UpdateMemoryWord(UpdateMemoryWord),
    SendSignal(SendSignal),
    SendQueuedSignal(SendQueuedSignal),
    SendThreadSignal(SendThreadSignal),
    SendQueuedThreadSignal(SendQueuedThreadSignal),
    SetSignalMask(SetSignalMask),
    SetSignalDisposition(SetSignalDisposition),
    TakePendingSignal(TakePendingSignal),
    WaitForPendingSignal(WaitForPendingSignal),
    SetProcessBreak(SetProcessBreak),
    GrantCapability(GrantCapability),
    DuplicateCapability(DuplicateCapability),
    CreateDomain(CreateDomain),
    CreateResource(CreateResource),
    CreateContract(CreateContract),
    OpenDescriptor {
        owner: ProcessId,
        capability: CapabilityId,
        kind: ObjectKind,
        name: String,
    },
    DuplicateDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    DuplicateDescriptorTo {
        owner: ProcessId,
        fd: Descriptor,
        target: Descriptor,
    },
    CloseDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    ExecTransition {
        owner: ProcessId,
    },
    Mount {
        mount_path: String,
        name: String,
    },
    CreateVfsNode {
        path: String,
        kind: ObjectKind,
        capability: CapabilityId,
    },
    CreateVfsSymlink {
        path: String,
        target: String,
        capability: CapabilityId,
    },
    UnlinkPath {
        path: String,
    },
    RenamePath {
        from: String,
        to: String,
    },
    OpenPath {
        owner: ProcessId,
        path: String,
    },
    InspectVmObjectLayouts {
        pid: ProcessId,
    },
    InspectDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    InspectDescriptorLayout {
        owner: ProcessId,
        fd: Descriptor,
    },
    GetDescriptorFlags {
        owner: ProcessId,
        fd: Descriptor,
    },
    SetCloexec {
        owner: ProcessId,
        fd: Descriptor,
        cloexec: bool,
    },
    SetNonblock {
        owner: ProcessId,
        fd: Descriptor,
        nonblock: bool,
    },
    StatPath {
        path: String,
    },
    LstatPath {
        path: String,
    },
    ReadLink {
        path: String,
    },
    StatDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    StatFs {
        path: String,
    },
    FiledescEntries {
        owner: ProcessId,
    },
    KinfoFileEntries {
        owner: ProcessId,
    },
    CloseFrom {
        owner: ProcessId,
        low_fd: Descriptor,
    },
    CloseRange {
        owner: ProcessId,
        start_fd: Descriptor,
        end_fd: Option<Descriptor>,
        mode: CloseRangeMode,
    },
    FcntlDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        cmd: FcntlCmd,
    },
    CreateEventQueue {
        owner: ProcessId,
        mode: EventQueueMode,
    },
    CreateEventQueueDescriptor {
        owner: ProcessId,
        mode: EventQueueMode,
    },
    OpenEventQueueDescriptor {
        owner: ProcessId,
        queue: EventQueueId,
    },
    DestroyEventQueue {
        owner: ProcessId,
        queue: EventQueueId,
    },
    DestroyEventQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    CreateSleepQueue {
        owner: ProcessId,
    },
    CreateSleepQueueDescriptor {
        owner: ProcessId,
    },
    OpenSleepQueueDescriptor {
        owner: ProcessId,
        queue: SleepQueueId,
    },
    DestroySleepQueue {
        owner: ProcessId,
        queue: SleepQueueId,
    },
    DestroySleepQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    WatchEvent {
        owner: ProcessId,
        queue: EventQueueId,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    },
    WatchEventDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    },
    RegisterEventQueueTimerDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        token: u64,
        delay_ticks: u64,
        interval_ticks: Option<u64>,
        events: IoPollEvents,
    },
    RemoveEventQueueTimerDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        timer: EventTimerId,
    },
    WatchProcessEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        token: u64,
        interest: ProcessLifecycleInterest,
        events: IoPollEvents,
    },
    RemoveProcessEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        token: u64,
    },
    WatchSignalEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        thread: Option<ThreadId>,
        signal_mask: u64,
        token: u64,
        events: IoPollEvents,
    },
    RemoveSignalEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        thread: Option<ThreadId>,
        token: u64,
    },
    WatchMemoryWaitEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        domain: MemoryWaitDomain,
        addr: u64,
        token: u64,
        events: IoPollEvents,
    },
    RemoveMemoryWaitEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        domain: MemoryWaitDomain,
        addr: u64,
        token: u64,
    },
    WatchResourceEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        resource: ResourceId,
        token: u64,
        interest: ResourceEventInterest,
        events: IoPollEvents,
    },
    WatchNetworkEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        interface_path: String,
        socket_path: Option<String>,
        token: u64,
        interest: NetworkEventInterest,
        events: IoPollEvents,
    },
    RemoveResourceEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        resource: ResourceId,
        token: u64,
    },
    RemoveNetworkEventsDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        interface_path: String,
        socket_path: Option<String>,
        token: u64,
    },
    ModifyWatchedEvent {
        owner: ProcessId,
        queue: EventQueueId,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    },
    ModifyWatchedEventDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    },
    RemoveWatchedEvent {
        owner: ProcessId,
        queue: EventQueueId,
        fd: Descriptor,
    },
    RemoveWatchedEventDescriptor {
        owner: ProcessId,
        queue_fd: Descriptor,
        fd: Descriptor,
    },
    WaitEventQueue {
        owner: ProcessId,
        queue: EventQueueId,
    },
    WaitEventQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    SleepOnQueue {
        owner: ProcessId,
        queue: SleepQueueId,
        channel: u64,
        priority: u16,
        timeout_ticks: Option<u64>,
    },
    SleepOnQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        channel: u64,
        priority: u16,
        timeout_ticks: Option<u64>,
    },
    WakeOneSleepQueue {
        owner: ProcessId,
        queue: SleepQueueId,
        channel: u64,
    },
    WakeOneSleepQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        channel: u64,
    },
    WakeAllSleepQueue {
        owner: ProcessId,
        queue: SleepQueueId,
        channel: u64,
    },
    WakeAllSleepQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        channel: u64,
    },
    CancelSleepQueueOwner {
        owner: ProcessId,
        queue: SleepQueueId,
        target: ProcessId,
    },
    CancelSleepQueueOwnerDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        target: ProcessId,
    },
    RequeueSleepQueue {
        owner: ProcessId,
        queue: SleepQueueId,
        from_channel: u64,
        to_channel: u64,
        max_count: usize,
    },
    RequeueSleepQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        from_channel: u64,
        to_channel: u64,
        max_count: usize,
    },
    InspectEventQueue {
        owner: ProcessId,
        queue: EventQueueId,
    },
    InspectEventQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    InspectSleepQueue {
        owner: ProcessId,
        queue: SleepQueueId,
    },
    InspectSleepQueueDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    InspectSleepResult {
        pid: ProcessId,
    },
    InspectPendingSignals {
        pid: ProcessId,
    },
    InspectThreadPendingSignals {
        pid: ProcessId,
        tid: ThreadId,
    },
    InspectBlockedPendingSignals {
        pid: ProcessId,
    },
    InspectPendingSignalWait {
        pid: ProcessId,
    },
    InspectSignalDisposition {
        pid: ProcessId,
        signal: u8,
    },
    InspectSignalMask {
        pid: ProcessId,
    },
    ReadDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        len: usize,
    },
    ReadDescriptorVectored {
        owner: ProcessId,
        fd: Descriptor,
        segments: Vec<usize>,
    },
    ReadDescriptorVectoredWithLayout {
        owner: ProcessId,
        fd: Descriptor,
        segments: Vec<usize>,
    },
    WriteDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        bytes: Vec<u8>,
    },
    WriteDescriptorVectored {
        owner: ProcessId,
        fd: Descriptor,
        segments: Vec<Vec<u8>>,
    },
    PollDescriptor {
        owner: ProcessId,
        fd: Descriptor,
    },
    ControlDescriptor {
        owner: ProcessId,
        fd: Descriptor,
        opcode: u32,
    },
    RegisterReadiness {
        owner: ProcessId,
        fd: Descriptor,
        interest: ReadinessInterest,
    },
    CollectReadiness,
    Tick,
    BlockRunning,
    WakeProcess {
        pid: ProcessId,
        class: SchedulerClass,
    },
    ExitRunning {
        code: i32,
    },
    ReapProcess {
        pid: ProcessId,
    },
    InspectProcess {
        pid: ProcessId,
    },
    InspectDomain {
        id: DomainId,
    },
    ListDomains,
    InspectResource {
        id: ResourceId,
    },
    ListResources,
    InspectContract {
        id: ContractId,
    },
    ListContracts,
    SetContractState(SetContractState),
    InvokeContract(InvokeContract),
    AcquireResourceViaContract(AcquireResourceViaContract),
    ReleaseResourceViaContract(ReleaseResourceViaContract),
    TransferResourceViaContract(TransferResourceViaContract),
    SetResourceArbitrationPolicy(SetResourceArbitrationPolicy),
    SetResourceGovernanceMode(SetResourceGovernanceMode),
    SetResourceContractPolicy(SetResourceContractPolicy),
    SetResourceIssuerPolicy(SetResourceIssuerPolicy),
    SetResourceState(SetResourceState),
    ClaimResourceViaContract(ClaimResourceViaContract),
    CancelResourceClaimViaContract(CancelResourceClaimViaContract),
    ReleaseClaimedResourceViaContract(ReleaseClaimedResourceViaContract),
    ListProcesses,
    ReadProcFs {
        path: String,
    },
    InspectSystem,
    Snapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyscallResult {
    ProcessSpawned(ProcessId),
    CapabilityGranted(CapabilityId),
    CapabilityDuplicated(CapabilityId),
    DomainCreated(DomainId),
    ResourceCreated(ResourceId),
    ContractCreated(ContractId),
    DescriptorOpened(Descriptor),
    DescriptorDuplicated(Descriptor),
    DescriptorDuplicatedTo(Descriptor),
    DescriptorClosed(ObjectDescriptor),
    ExecTransitioned(Vec<ObjectDescriptor>),
    Mounted,
    VfsNodeCreated,
    VfsSymlinkCreated,
    VfsNodeRemoved,
    VfsNodeRenamed,
    PathOpened(Descriptor),
    LinkTarget(String),
    DescriptorInspected(IoObject),
    DescriptorLayoutInspected(IoPayloadLayoutInfo),
    DescriptorFlags(DescriptorFlags),
    FileStatus(FileStatus),
    FileSystemStatus(FileSystemStatus),
    FiledescEntries(Vec<FiledescEntry>),
    KinfoFileEntries(Vec<KinfoFileEntry>),
    FcntlResult(FcntlResult),
    QueueDescriptorCreated(Descriptor),
    QueueDescriptorOpened(Descriptor),
    QueueDescriptorDestroyed(Descriptor),
    EventWatchRegistered,
    EventQueueTimerRegistered(EventTimerId),
    EventQueueTimerRemoved,
    ProcessEventWatchRegistered,
    ProcessEventWatchRemoved,
    SignalEventWatchRegistered,
    SignalEventWatchRemoved,
    MemoryWaitEventWatchRegistered,
    MemoryWaitEventWatchRemoved,
    ResourceEventWatchRegistered,
    ResourceEventWatchRemoved,
    NetworkEventWatchRegistered,
    NetworkEventWatchRemoved,
    EventWatchModified,
    EventWatchRemoved,
    EventQueueInspected(EventQueueInfo),
    EventQueueReady(Vec<EventRecord>),
    ProcessBlockedOnSleepQueue(ProcessId),
    SleepQueueInspected(SleepQueueInfo),
    SleepQueueWakeResult(Vec<ProcessId>),
    SleepQueueRequeueResult(usize),
    SleepResultInspected(Option<SleepWaitResult>),
    DescriptorRead(Vec<u8>),
    DescriptorReadVectored(Vec<Vec<u8>>),
    DescriptorReadVectoredWithLayout {
        segments: Vec<Vec<u8>>,
        layout: IoPayloadLayoutInfo,
    },
    DescriptorWritten(usize),
    DescriptorPolled(IoPollEvents),
    DescriptorControlled(u32),
    DescriptorFlagsUpdated,
    ReadinessRegistered,
    ReadinessEvents(Vec<ReadinessRegistration>),
    Scheduled(ScheduledProcess),
    ProcessBlocked(ProcessId),
    ProcessWoken(ProcessId),
    ProcessExited(ProcessId),
    ProcessReaped(Process),
    ProcessInfo(ProcessInfo),
    DomainInfo(DomainInfo),
    DomainList(Vec<DomainInfo>),
    ResourceInfo(ResourceInfo),
    ResourceList(Vec<ResourceInfo>),
    ContractInfo(ContractInfo),
    ContractList(Vec<ContractInfo>),
    ContractStateChanged {
        id: ContractId,
        state: ContractState,
    },
    ContractInvoked {
        id: ContractId,
        invocation_count: u64,
    },
    ResourceAcquired {
        resource: ResourceId,
        contract: ContractId,
        acquire_count: u64,
    },
    ResourceReleased {
        resource: ResourceId,
        contract: ContractId,
    },
    ResourceTransferred {
        resource: ResourceId,
        from: ContractId,
        to: ContractId,
        acquire_count: u64,
    },
    ResourceArbitrationPolicyChanged {
        resource: ResourceId,
        policy: ResourceArbitrationPolicy,
    },
    ResourceGovernanceModeChanged {
        resource: ResourceId,
        mode: ResourceGovernanceMode,
    },
    ResourceContractPolicyChanged {
        resource: ResourceId,
        policy: ResourceContractPolicy,
    },
    ResourceIssuerPolicyChanged {
        resource: ResourceId,
        policy: ResourceIssuerPolicy,
    },
    ResourceStateChanged {
        resource: ResourceId,
        state: ResourceState,
    },
    ResourceClaimed {
        resource: ResourceId,
        contract: ContractId,
        acquire_count: u64,
    },
    ResourceClaimQueued {
        resource: ResourceId,
        contract: ContractId,
        holder: ContractId,
        position: usize,
    },
    ResourceClaimCanceled {
        resource: ResourceId,
        contract: ContractId,
        waiting_count: usize,
    },
    ResourceClaimReleased {
        resource: ResourceId,
        contract: ContractId,
    },
    ResourceClaimHandedOff {
        resource: ResourceId,
        from: ContractId,
        to: ContractId,
        acquire_count: u64,
        handoff_count: u64,
    },
    ProcessList(Vec<ProcessInfo>),
    ProcessIntrospection(ProcessIntrospection),
    VmObjectLayouts(Vec<VmObjectLayoutInfo>),
    SystemIntrospection(SystemIntrospection),
    MemoryMapped(u64),
    MemoryUnmapped,
    MemoryTouched(MemoryTouchStats),
    MemoryWordLoaded(u32),
    MemoryWordCompared {
        expected: u32,
        observed: u32,
    },
    MemoryWordUpdated {
        old: u32,
        new: u32,
    },
    SignalQueued,
    PendingSignals(Vec<u8>),
    PendingSignalTaken(Option<u8>),
    PendingSignalWaited(PendingSignalWaitResult),
    PendingSignalWaitInspected(Option<PendingSignalWaitResume>),
    SignalMaskUpdated {
        old: u64,
        new: u64,
    },
    SignalDispositionUpdated {
        old: SignalActionState,
        new: SignalActionState,
    },
    ProcessBreak(u64),
    ProcFsBytes(Vec<u8>),
    Snapshot(RuntimeSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyscallContext {
    pub caller: ProcessId,
    pub tid: ThreadId,
    pub authority: CapabilityRights,
}

impl SyscallContext {
    pub fn kernel(caller: ProcessId) -> Self {
        Self {
            caller,
            tid: ThreadId::from_process_id(caller),
            authority: CapabilityRights::all(),
        }
    }

    pub(crate) fn require(&self, required: CapabilityRights) -> Result<(), SyscallError> {
        if self.authority.contains(required) {
            Ok(())
        } else {
            Err(SyscallError::AccessDenied)
        }
    }
}

#[derive(Debug)]
pub struct HalBackedKernelRuntime<H: AddressSpaceManager> {
    runtime: KernelRuntime,
    hal: H,
    hal_address_spaces: BTreeMap<u64, HalAddressSpaceId>,
}

impl<H: AddressSpaceManager> HalBackedKernelRuntime<H> {
    pub fn new(runtime: KernelRuntime, hal: H) -> Result<Self, RuntimeError> {
        let mut runtime = Self {
            runtime,
            hal,
            hal_address_spaces: BTreeMap::new(),
        };
        for process in runtime.runtime.process_list() {
            runtime.sync_process_address_space(process.pid)?;
        }
        Ok(runtime)
    }

    pub fn host_runtime_default(hal: H) -> Result<Self, RuntimeError> {
        Self::new(KernelRuntime::host_runtime_default(), hal)
    }

    pub fn runtime(&self) -> &KernelRuntime {
        &self.runtime
    }

    pub fn hal(&self) -> &H {
        &self.hal
    }

    pub fn hal_mut(&mut self) -> &mut H {
        &mut self.hal
    }

    pub fn spawn_process(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
    ) -> Result<ProcessId, RuntimeError> {
        let pid = self.runtime.spawn_process(name, parent, class)?;
        self.sync_process_address_space(pid)?;
        Ok(pid)
    }

    pub fn spawn_process_copy_vm(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
        source: ProcessId,
    ) -> Result<ProcessId, RuntimeError> {
        let pid = self
            .runtime
            .spawn_process_copy_vm(name, parent, class, source)?;
        self.sync_process_address_space(pid)?;
        Ok(pid)
    }

    pub fn spawn_process_from_source(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
        source: ProcessId,
        filedesc_mode: SpawnFiledescMode,
        vm_mode: SpawnVmMode,
    ) -> Result<ProcessId, RuntimeError> {
        let pid = self.runtime.spawn_process_from_source(
            name,
            parent,
            class,
            source,
            filedesc_mode,
            vm_mode,
        )?;
        self.sync_process_address_space(pid)?;
        Ok(pid)
    }

    pub fn map_anonymous_memory(
        &mut self,
        pid: ProcessId,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: impl Into<String>,
    ) -> Result<u64, RuntimeError> {
        let start = self
            .runtime
            .map_anonymous_memory(pid, length, readable, writable, executable, label)?;
        self.sync_process_address_space(pid)?;
        Ok(start)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn map_file_memory(
        &mut self,
        pid: ProcessId,
        path: impl Into<String>,
        length: u64,
        file_offset: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        private: bool,
    ) -> Result<u64, RuntimeError> {
        let start = self.runtime.map_file_memory(
            pid,
            path,
            length,
            file_offset,
            readable,
            writable,
            executable,
            private,
        )?;
        self.sync_process_address_space(pid)?;
        Ok(start)
    }

    pub fn unmap_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
    ) -> Result<(), RuntimeError> {
        self.runtime.unmap_memory(pid, start, length)?;
        self.sync_process_address_space(pid)?;
        Ok(())
    }

    pub fn protect_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
    ) -> Result<(), RuntimeError> {
        self.runtime
            .protect_memory(pid, start, length, readable, writable, executable)?;
        self.sync_process_address_space(pid)?;
        Ok(())
    }

    pub fn activate_process_address_space(
        &mut self,
        pid: ProcessId,
    ) -> Result<HalAddressSpaceLayout, RuntimeError> {
        let hal_id = self.ensure_hal_address_space(pid)?;
        self.hal.activate_address_space(hal_id)?;
        self.hal.address_space_layout(hal_id).map_err(Into::into)
    }

    pub fn sync_process_address_space(
        &mut self,
        pid: ProcessId,
    ) -> Result<HalAddressSpaceLayout, RuntimeError> {
        let kernel_address_space = self.runtime.processes.get_process_address_space(pid)?.id();
        let existing = self.hal_address_spaces.remove(&kernel_address_space.raw());
        let was_active = existing
            .map(|id| self.hal.active_address_space() == Some(id))
            .unwrap_or(false);
        if let Some(existing) = existing {
            self.hal.destroy_address_space(existing)?;
        }
        let hal_id = self.hal.create_address_space()?;
        let introspection = self.runtime.inspect_process(pid)?;
        let layout = project_hal_address_space_layout(&introspection, hal_id, was_active)?;
        for mapping in layout.mappings.iter().copied() {
            self.hal.map(hal_id, mapping)?;
        }
        if was_active {
            self.hal.activate_address_space(hal_id)?;
        }
        self.hal_address_spaces
            .insert(kernel_address_space.raw(), hal_id);
        self.hal.address_space_layout(hal_id).map_err(Into::into)
    }

    pub fn tick(&mut self) -> Result<ScheduledProcess, RuntimeError> {
        let scheduled = self.runtime.tick()?;
        self.activate_process_address_space(scheduled.pid)?;
        Ok(scheduled)
    }

    fn ensure_hal_address_space(
        &mut self,
        pid: ProcessId,
    ) -> Result<HalAddressSpaceId, RuntimeError> {
        let kernel_address_space = self.runtime.processes.get_process_address_space(pid)?.id();
        if let Some(id) = self
            .hal_address_spaces
            .get(&kernel_address_space.raw())
            .copied()
        {
            return Ok(id);
        }
        Ok(self.sync_process_address_space(pid)?.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSyscallSurface {
    pub(crate) runtime: KernelRuntime,
}

impl KernelSyscallSurface {
    pub fn new(runtime: KernelRuntime) -> Self {
        Self { runtime }
    }

    pub fn host_runtime_default() -> Self {
        Self::new(KernelRuntime::host_runtime_default())
    }

    pub fn runtime(&self) -> &KernelRuntime {
        &self.runtime
    }

    pub fn dispatch_user_syscall_frame(
        &mut self,
        caller: ProcessId,
        frame: ngos_user_abi::SyscallFrame,
    ) -> ngos_user_abi::SyscallReturn {
        self.runtime.dispatch_user_syscall_frame(caller, frame)
    }

    pub fn dispatch(
        &mut self,
        context: SyscallContext,
        syscall: Syscall,
    ) -> Result<SyscallResult, SyscallError> {
        if let Some(result) = self.dispatch_eventing(&context, &syscall)? {
            return Ok(result);
        }
        if let Some(result) = self.dispatch_process_vm(&context, &syscall)? {
            return Ok(result);
        }
        if let Some(result) = self.dispatch_descriptor_io(&context, &syscall)? {
            return Ok(result);
        }
        match syscall {
            Syscall::Tick => {
                context.require(CapabilityRights::EXECUTE)?;
                Ok(SyscallResult::Scheduled(self.runtime.tick()?))
            }
            Syscall::BlockRunning => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::ProcessBlocked(
                    self.runtime
                        .block_running_thread(context.caller, context.tid)?,
                ))
            }
            Syscall::WakeProcess { pid, class } => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.wake_process(pid, class)?;
                Ok(SyscallResult::ProcessWoken(pid))
            }
            Syscall::ExitRunning { code } => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::ProcessExited(
                    self.runtime
                        .exit_running_thread(context.caller, context.tid, code)?,
                ))
            }
            Syscall::ReapProcess { pid } => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::ProcessReaped(
                    self.runtime.reap_process(pid)?,
                ))
            }
            Syscall::CreateDomain(args) => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::DomainCreated(self.runtime.create_domain(
                    args.owner,
                    args.parent,
                    args.name,
                )?))
            }
            Syscall::CreateResource(args) => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::ResourceCreated(
                    self.runtime.create_resource(
                        args.creator,
                        args.domain,
                        args.kind,
                        args.name,
                    )?,
                ))
            }
            Syscall::CreateContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::ContractCreated(
                    self.runtime.create_contract(
                        args.issuer,
                        args.domain,
                        args.resource,
                        args.kind,
                        args.label,
                    )?,
                ))
            }
            Syscall::InspectDomain { id } => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::DomainInfo(self.runtime.domain_info(id)?))
            }
            Syscall::ListDomains => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::DomainList(self.runtime.domain_list()))
            }
            Syscall::InspectResource { id } => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::ResourceInfo(self.runtime.resource_info(id)?))
            }
            Syscall::ListResources => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::ResourceList(self.runtime.resource_list()))
            }
            Syscall::InspectContract { id } => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::ContractInfo(self.runtime.contract_info(id)?))
            }
            Syscall::ListContracts => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::ContractList(self.runtime.contract_list()))
            }
            Syscall::SetContractState(args) => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::ContractStateChanged {
                    id: args.id,
                    state: self
                        .runtime
                        .transition_contract_state(args.id, args.state)?,
                })
            }
            Syscall::InvokeContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                Ok(SyscallResult::ContractInvoked {
                    id: args.id,
                    invocation_count: self.runtime.invoke_contract(args.id)?,
                })
            }
            Syscall::AcquireResourceViaContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                let (resource, acquire_count) =
                    self.runtime.acquire_resource_via_contract(args.contract)?;
                Ok(SyscallResult::ResourceAcquired {
                    resource,
                    contract: args.contract,
                    acquire_count,
                })
            }
            Syscall::ReleaseResourceViaContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                let resource = self.runtime.release_resource_via_contract(args.contract)?;
                Ok(SyscallResult::ResourceReleased {
                    resource,
                    contract: args.contract,
                })
            }
            Syscall::TransferResourceViaContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                let (resource, acquire_count) = self
                    .runtime
                    .transfer_resource_via_contract(args.source, args.target)?;
                Ok(SyscallResult::ResourceTransferred {
                    resource,
                    from: args.source,
                    to: args.target,
                    acquire_count,
                })
            }
            Syscall::SetResourceArbitrationPolicy(args) => {
                context.require(CapabilityRights::WRITE)?;
                let policy = self
                    .runtime
                    .set_resource_arbitration_policy(args.resource, args.policy)?;
                Ok(SyscallResult::ResourceArbitrationPolicyChanged {
                    resource: args.resource,
                    policy,
                })
            }
            Syscall::SetResourceGovernanceMode(args) => {
                context.require(CapabilityRights::WRITE)?;
                let mode = self
                    .runtime
                    .set_resource_governance_mode(args.resource, args.mode)?;
                Ok(SyscallResult::ResourceGovernanceModeChanged {
                    resource: args.resource,
                    mode,
                })
            }
            Syscall::SetResourceContractPolicy(args) => {
                context.require(CapabilityRights::WRITE)?;
                let policy = self
                    .runtime
                    .set_resource_contract_policy(args.resource, args.policy)?;
                Ok(SyscallResult::ResourceContractPolicyChanged {
                    resource: args.resource,
                    policy,
                })
            }
            Syscall::SetResourceIssuerPolicy(args) => {
                context.require(CapabilityRights::WRITE)?;
                let policy = self
                    .runtime
                    .set_resource_issuer_policy(args.resource, args.policy)?;
                Ok(SyscallResult::ResourceIssuerPolicyChanged {
                    resource: args.resource,
                    policy,
                })
            }
            Syscall::SetResourceState(args) => {
                context.require(CapabilityRights::WRITE)?;
                let state = self
                    .runtime
                    .transition_resource_state(args.resource, args.state)?;
                Ok(SyscallResult::ResourceStateChanged {
                    resource: args.resource,
                    state,
                })
            }
            Syscall::ClaimResourceViaContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                match self.runtime.claim_resource_via_contract(args.contract)? {
                    ResourceClaimResult::Acquired {
                        resource,
                        acquire_count,
                    } => Ok(SyscallResult::ResourceClaimed {
                        resource,
                        contract: args.contract,
                        acquire_count,
                    }),
                    ResourceClaimResult::Queued {
                        resource,
                        holder,
                        position,
                    } => Ok(SyscallResult::ResourceClaimQueued {
                        resource,
                        contract: args.contract,
                        holder,
                        position,
                    }),
                }
            }
            Syscall::CancelResourceClaimViaContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                let (resource, waiting_count) = self
                    .runtime
                    .cancel_resource_claim_via_contract(args.contract)?;
                Ok(SyscallResult::ResourceClaimCanceled {
                    resource,
                    contract: args.contract,
                    waiting_count,
                })
            }
            Syscall::ReleaseClaimedResourceViaContract(args) => {
                context.require(CapabilityRights::WRITE)?;
                match self
                    .runtime
                    .release_claimed_resource_via_contract(args.contract)?
                {
                    ResourceReleaseResult::Released { resource } => {
                        Ok(SyscallResult::ResourceClaimReleased {
                            resource,
                            contract: args.contract,
                        })
                    }
                    ResourceReleaseResult::HandedOff {
                        resource,
                        contract,
                        acquire_count,
                        handoff_count,
                    } => Ok(SyscallResult::ResourceClaimHandedOff {
                        resource,
                        from: args.contract,
                        to: contract,
                        acquire_count,
                        handoff_count,
                    }),
                }
            }
            Syscall::ListProcesses => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::ProcessList(self.runtime.process_list()))
            }
            Syscall::ReadProcFs { path } => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::ProcFsBytes(
                    self.runtime.read_procfs_path(&path)?,
                ))
            }
            Syscall::InspectSystem => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::SystemIntrospection(
                    self.runtime.inspect_system(),
                ))
            }
            Syscall::Snapshot => {
                context.require(CapabilityRights::READ)?;
                Ok(SyscallResult::Snapshot(self.runtime.snapshot()))
            }
            other => {
                unreachable!(
                    "eventing syscall should be dispatched in syscall_eventing.rs: {other:?}"
                )
            }
        }
    }
}

pub(crate) fn memory_advice_code(advice: MemoryAdvice) -> &'static str {
    match advice {
        MemoryAdvice::Normal => "normal",
        MemoryAdvice::Sequential => "seq",
        MemoryAdvice::Random => "rand",
        MemoryAdvice::WillNeed => "willneed",
        MemoryAdvice::DontNeed => "dontneed",
    }
}

pub(crate) fn map_runtime_io_error(error: IoError) -> RuntimeError {
    match error {
        IoError::NotFound | IoError::Closed => {
            RuntimeError::Descriptor(DescriptorError::InvalidDescriptor)
        }
        IoError::InvalidOwner => RuntimeError::Descriptor(DescriptorError::InvalidOwner),
        IoError::OperationNotSupported | IoError::AccessDenied => {
            RuntimeError::Descriptor(DescriptorError::InvalidDescriptor)
        }
    }
}

pub(crate) fn proc_state_code(state: ProcessState) -> char {
    match state {
        ProcessState::Created => 'N',
        ProcessState::Ready => 'R',
        ProcessState::Running => 'S',
        ProcessState::Blocked => 'D',
        ProcessState::Exited => 'Z',
    }
}
