use super::*;

pub const PRODUCT_NAME: &str = "Next Gen OS";
pub const PRODUCT_CODENAME: &str = "ngos";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    HostRuntime,
    Kernel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelConfig {
    pub project_name: &'static str,
    pub project_codename: &'static str,
    pub architecture: Architecture,
    pub runtime_mode: RuntimeMode,
    pub support_32_bit: bool,
    pub linux_compat_enabled: bool,
}

impl KernelConfig {
    pub const fn host_runtime(architecture: Architecture) -> Self {
        Self {
            project_name: PRODUCT_NAME,
            project_codename: PRODUCT_CODENAME,
            architecture,
            runtime_mode: RuntimeMode::HostRuntime,
            support_32_bit: false,
            linux_compat_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelState {
    pub config: KernelConfig,
    pub scheduler_ready: bool,
    pub vm_ready: bool,
    pub vfs_ready: bool,
    pub handles_ready: bool,
}

impl KernelState {
    pub fn bootstrap(config: KernelConfig) -> Self {
        Self {
            config,
            scheduler_ready: true,
            vm_ready: true,
            vfs_ready: true,
            handles_ready: true,
        }
    }

    pub fn status_line(&self) -> String {
        format!(
            "{} ({}) {:?} {:?} 64-bit-only linux-compat={}",
            self.config.project_name,
            self.config.project_codename,
            self.config.architecture,
            self.config.runtime_mode,
            self.config.linux_compat_enabled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomainId(ObjectHandle);

impl DomainId {
    pub const fn from_handle(handle: ObjectHandle) -> Self {
        Self(handle)
    }

    pub const fn handle(self) -> ObjectHandle {
        self.0
    }

    pub const fn raw(self) -> u64 {
        self.0.id().raw()
    }

    pub const fn generation(self) -> u32 {
        self.0.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(ObjectHandle);

impl ResourceId {
    pub const fn from_handle(handle: ObjectHandle) -> Self {
        Self(handle)
    }

    pub const fn handle(self) -> ObjectHandle {
        self.0
    }

    pub const fn raw(self) -> u64 {
        self.0.id().raw()
    }

    pub const fn generation(self) -> u32 {
        self.0.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractId(ObjectHandle);

impl ContractId {
    pub const fn from_handle(handle: ObjectHandle) -> Self {
        Self(handle)
    }

    pub const fn handle(self) -> ObjectHandle {
        self.0
    }

    pub const fn raw(self) -> u64 {
        self.0.id().raw()
    }

    pub const fn generation(self) -> u32 {
        self.0.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceArbitrationPolicy {
    Fifo,
    Lifo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceGovernanceMode {
    Queueing,
    ExclusiveLease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    Active,
    Suspended,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceContractPolicy {
    Any,
    Execution,
    Memory,
    Io,
    Device,
    Display,
    Observe,
}

impl ResourceContractPolicy {
    pub const fn allows(self, kind: ContractKind) -> bool {
        match self {
            Self::Any => true,
            Self::Execution => matches!(kind, ContractKind::Execution),
            Self::Memory => matches!(kind, ContractKind::Memory),
            Self::Io => matches!(kind, ContractKind::Io),
            Self::Device => matches!(kind, ContractKind::Device),
            Self::Display => matches!(kind, ContractKind::Display),
            Self::Observe => matches!(kind, ContractKind::Observe),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceIssuerPolicy {
    AnyIssuer,
    CreatorOnly,
    DomainOwnerOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeModelError {
    Exhausted,
    InvalidDomain,
    StaleDomain,
    InvalidResource,
    StaleResource,
    InvalidContract,
    StaleContract,
    InvalidOwner,
    ParentMismatch,
    InvalidStateTransition {
        from: ContractState,
        to: ContractState,
    },
    ContractNotActive {
        state: ContractState,
    },
    ResourceBusy {
        holder: ContractId,
    },
    ResourceNotHeld {
        resource: ResourceId,
    },
    ResourceClaimNotQueued {
        resource: ResourceId,
    },
    ResourceNotActive {
        state: ResourceState,
    },
    ResourceContractKindMismatch {
        expected: ResourceContractPolicy,
        actual: ContractKind,
    },
    ResourceIssuerPolicyMismatch {
        policy: ResourceIssuerPolicy,
        issuer: ProcessId,
    },
    ProcessContractMissing {
        kind: ContractKind,
    },
    ResourceBindingMismatch,
}

impl NativeModelError {
    pub(crate) fn from_domain_object_error(error: ObjectError) -> Self {
        match error {
            ObjectError::Exhausted => Self::Exhausted,
            ObjectError::InvalidHandle => Self::InvalidDomain,
            ObjectError::StaleHandle => Self::StaleDomain,
        }
    }

    pub(crate) fn from_resource_object_error(error: ObjectError) -> Self {
        match error {
            ObjectError::Exhausted => Self::Exhausted,
            ObjectError::InvalidHandle => Self::InvalidResource,
            ObjectError::StaleHandle => Self::StaleResource,
        }
    }

    pub(crate) fn from_contract_object_error(error: ObjectError) -> Self {
        match error {
            ObjectError::Exhausted => Self::Exhausted,
            ObjectError::InvalidHandle => Self::InvalidContract,
            ObjectError::StaleHandle => Self::StaleContract,
        }
    }
}

pub(crate) fn scheduler_class_from_hint(hint: u16) -> SchedulerClass {
    match hint as usize {
        0 => SchedulerClass::LatencyCritical,
        1 => SchedulerClass::Interactive,
        2 => SchedulerClass::BestEffort,
        _ => SchedulerClass::Background,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePolicy {
    pub scheduler_budget: u32,
    pub process_range: Range,
    pub capability_range: Range,
    pub domain_range: Range,
    pub resource_range: Range,
    pub contract_range: Range,
}

impl RuntimePolicy {
    pub fn host_runtime_default() -> Self {
        Self {
            scheduler_budget: 2,
            process_range: Range::new(1, 1 << 16),
            capability_range: Range::new(1, 1 << 18),
            domain_range: Range::new(1, 1 << 14),
            resource_range: Range::new(1, 1 << 16),
            contract_range: Range::new(1, 1 << 16),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    Process(ProcessError),
    Capability(CapabilityError),
    NativeModel(NativeModelError),
    DeviceModel(DeviceModelError),
    Descriptor(DescriptorError),
    Scheduler(SchedulerError),
    Vfs(VfsError),
    EventQueue(EventQueueError),
    SleepQueue(SleepQueueError),
    TaskQueue(TaskQueueError),
    Buffer(BufferError),
    Hal(HalError),
}

impl From<ProcessError> for RuntimeError {
    fn from(value: ProcessError) -> Self {
        Self::Process(value)
    }
}

impl From<CapabilityError> for RuntimeError {
    fn from(value: CapabilityError) -> Self {
        Self::Capability(value)
    }
}

impl From<NativeModelError> for RuntimeError {
    fn from(value: NativeModelError) -> Self {
        Self::NativeModel(value)
    }
}

impl From<DeviceModelError> for RuntimeError {
    fn from(value: DeviceModelError) -> Self {
        Self::DeviceModel(value)
    }
}

impl From<DescriptorError> for RuntimeError {
    fn from(value: DescriptorError) -> Self {
        Self::Descriptor(value)
    }
}

impl From<SchedulerError> for RuntimeError {
    fn from(value: SchedulerError) -> Self {
        Self::Scheduler(value)
    }
}

impl From<VfsError> for RuntimeError {
    fn from(value: VfsError) -> Self {
        Self::Vfs(value)
    }
}

impl From<BufferError> for RuntimeError {
    fn from(value: BufferError) -> Self {
        Self::Buffer(value)
    }
}

impl From<HalError> for RuntimeError {
    fn from(value: HalError) -> Self {
        Self::Hal(value)
    }
}

impl From<TaskQueueError> for RuntimeError {
    fn from(value: TaskQueueError) -> Self {
        Self::TaskQueue(value)
    }
}

impl From<SleepQueueError> for RuntimeError {
    fn from(value: SleepQueueError) -> Self {
        Self::SleepQueue(value)
    }
}

impl From<EventQueueError> for RuntimeError {
    fn from(value: EventQueueError) -> Self {
        Self::EventQueue(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub process_count: usize,
    pub active_process_count: usize,
    pub blocked_process_count: usize,
    pub thread_count: usize,
    pub capability_count: usize,
    pub domain_count: usize,
    pub resource_count: usize,
    pub contract_count: usize,
    pub queued_processes: usize,
    pub queued_latency_critical: usize,
    pub queued_interactive: usize,
    pub queued_normal: usize,
    pub queued_background: usize,
    pub deferred_task_count: usize,
    pub sleeping_processes: usize,
    pub current_tick: u64,
    pub busy_ticks: u64,
    pub running: Option<ProcessId>,
    pub running_thread: Option<ThreadId>,
    pub contract_bound_processes: usize,
    pub translated_processes: usize,
    pub total_event_queue_count: usize,
    pub total_event_queue_pending: usize,
    pub total_event_queue_waiters: usize,
    pub total_socket_count: usize,
    pub saturated_socket_count: usize,
    pub total_socket_rx_depth: usize,
    pub total_socket_rx_limit: usize,
    pub max_socket_rx_depth: usize,
    pub total_network_tx_dropped: u64,
    pub total_network_rx_dropped: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAgentKind {
    ClaimValidator,
    CancelValidator,
    ReleaseValidator,
    ResourceStateTransitionAgent,
    ContractStateTransitionAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceAgentDecisionRecord {
    pub tick: u64,
    pub agent: ResourceAgentKind,
    pub resource: u64,
    pub contract: u64,
    pub detail0: u64,
    pub detail1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitAgentKind {
    SleepEnqueueAgent,
    SleepWakeAgent,
    SleepCancelAgent,
    SleepRequeueAgent,
    MemoryWaitAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitAgentDecisionRecord {
    pub tick: u64,
    pub agent: WaitAgentKind,
    pub owner: u64,
    pub queue: u64,
    pub channel: u64,
    pub detail0: u64,
    pub detail1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerAgentKind {
    EnqueueAgent,
    WakeAgent,
    BlockAgent,
    TickAgent,
    RebindAgent,
    RemoveAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerAgentDecisionRecord {
    pub tick: u64,
    pub agent: SchedulerAgentKind,
    pub pid: u64,
    pub tid: u64,
    pub class: u64,
    pub detail0: u64,
    pub detail1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoAgentKind {
    OpenPathAgent,
    DuplicateDescriptorAgent,
    CloseDescriptorAgent,
    ReadAgent,
    WriteAgent,
    FcntlAgent,
    ReadinessAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoAgentDecisionRecord {
    pub tick: u64,
    pub agent: IoAgentKind,
    pub owner: u64,
    pub fd: u64,
    pub kind: u64,
    pub detail0: u64,
    pub detail1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmAgentKind {
    MapAgent,
    BrkAgent,
    ProtectAgent,
    UnmapAgent,
    PolicyBlockAgent,
    PressureTriggerAgent,
    PressureVictimAgent,
    FaultClassifierAgent,
    ShadowReuseAgent,
    ShadowBridgeAgent,
    CowPopulateAgent,
    PageTouchAgent,
    SyncAgent,
    AdviceAgent,
    QuarantineStateAgent,
    QuarantineBlockAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmAgentDecisionRecord {
    pub tick: u64,
    pub agent: VmAgentKind,
    pub pid: u64,
    pub vm_object_id: u64,
    pub start: u64,
    pub length: u64,
    pub detail0: u64,
    pub detail1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerPolicyInfo {
    pub class: SchedulerClass,
    pub budget: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcessContractBindings {
    pub execution: Option<ContractId>,
    pub memory: Option<ContractId>,
    pub io: Option<ContractId>,
    pub observe: Option<ContractId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Generic,
    Network,
    Storage,
    Graphics,
    Audio,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverState {
    Registered,
    Active,
    Faulted,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Registered,
    Bound,
    Faulted,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRequestKind {
    Read,
    Write,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRequestState {
    Queued,
    InFlight,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceModelError {
    InvalidPath,
    InvalidDevice,
    InvalidDriver,
    AlreadyRegistered,
    NotBound,
    PacketTooLarge,
    QueueFull,
    QueueEmpty,
    RequestNotFound,
    InvalidRequestState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRequestInfo {
    pub id: u64,
    pub device_path: String,
    pub driver_path: String,
    pub issuer: ProcessId,
    pub kind: DeviceRequestKind,
    pub state: DeviceRequestState,
    pub opcode: Option<u32>,
    pub graphics_buffer_id: Option<u64>,
    pub graphics_buffer_len: Option<usize>,
    pub payload_len: usize,
    pub response_len: usize,
    pub submitted_tick: u64,
    pub started_tick: Option<u64>,
    pub completed_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBufferInfo {
    pub id: u64,
    pub owner: ProcessId,
    pub length: usize,
    pub used_len: usize,
    pub busy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuScanoutInfo {
    pub device_path: String,
    pub presented_frames: u64,
    pub last_frame_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverInfo {
    pub path: String,
    pub owner: ProcessId,
    pub state: DriverState,
    pub capability: CapabilityId,
    pub bound_devices: Vec<String>,
    pub queued_requests: usize,
    pub in_flight_requests: usize,
    pub completed_requests: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub path: String,
    pub owner: ProcessId,
    pub class: DeviceClass,
    pub state: DeviceState,
    pub capability: CapabilityId,
    pub driver: Option<String>,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub submitted_requests: u64,
    pub completed_requests: u64,
    pub total_latency_ticks: u64,
    pub max_latency_ticks: u64,
    pub total_queue_wait_ticks: u64,
    pub max_queue_wait_ticks: u64,
    pub link_up: bool,
    pub block_size: u32,
    pub capacity_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSocketInfo {
    pub path: String,
    pub owner: ProcessId,
    pub interface: String,
    pub local_ipv4: [u8; 4],
    pub remote_ipv4: [u8; 4],
    pub local_port: u16,
    pub remote_port: u16,
    pub rx_depth: usize,
    pub rx_queue_limit: usize,
    pub connected: bool,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub dropped_packets: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkInterfaceInfo {
    pub device_path: String,
    pub driver_path: String,
    pub admin_up: bool,
    pub link_up: bool,
    pub promiscuous: bool,
    pub mtu: usize,
    pub mac: [u8; 6],
    pub ipv4_addr: [u8; 4],
    pub ipv4_netmask: [u8; 4],
    pub ipv4_gateway: [u8; 4],
    pub rx_ring_depth: usize,
    pub tx_ring_depth: usize,
    pub tx_inflight_depth: usize,
    pub free_buffer_count: usize,
    pub tx_capacity: usize,
    pub rx_capacity: usize,
    pub tx_inflight_limit: usize,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub tx_completions: u64,
    pub tx_dropped: u64,
    pub rx_dropped: u64,
    pub attached_sockets: Vec<String>,
}

impl ProcessContractBindings {
    pub const fn any_bound(self) -> bool {
        self.execution.is_some()
            || self.memory.is_some()
            || self.io.is_some()
            || self.observe.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueWatchInfo {
    pub owner: ProcessId,
    pub fd: Descriptor,
    pub token: u64,
    pub interest: ReadinessInterest,
    pub behavior: EventWatchBehavior,
    pub last_ready: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueuePendingInfo {
    pub owner: ProcessId,
    pub token: u64,
    pub events: IoPollEvents,
    pub source: EventSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueTimerInfo {
    pub id: EventTimerId,
    pub token: u64,
    pub deadline_tick: u64,
    pub interval_ticks: Option<u64>,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueProcessWatchInfo {
    pub target: ProcessId,
    pub token: u64,
    pub interest: ProcessLifecycleInterest,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueSignalWatchInfo {
    pub target: ProcessId,
    pub thread: Option<ThreadId>,
    pub signal_mask: u64,
    pub token: u64,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueMemoryWatchInfo {
    pub domain: MemoryWaitDomain,
    pub addr: u64,
    pub token: u64,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueResourceWatchInfo {
    pub resource: ResourceId,
    pub token: u64,
    pub interest: ResourceEventInterest,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueNetworkWatchInfo {
    pub interface_inode: u64,
    pub socket_inode: Option<u64>,
    pub token: u64,
    pub interest: NetworkEventInterest,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueWaiterInfo {
    pub owner: ProcessId,
    pub tid: ThreadId,
    pub class: SchedulerClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueueInfo {
    pub id: EventQueueId,
    pub owner: ProcessId,
    pub mode: EventQueueMode,
    pub watch_count: usize,
    pub timer_count: usize,
    pub process_watch_count: usize,
    pub signal_watch_count: usize,
    pub memory_watch_count: usize,
    pub resource_watch_count: usize,
    pub network_watch_count: usize,
    pub pending_count: usize,
    pub waiter_count: usize,
    pub descriptor_ref_count: usize,
    pub deferred_refresh_pending: bool,
    pub watches: Vec<EventQueueWatchInfo>,
    pub timers: Vec<EventQueueTimerInfo>,
    pub process_watches: Vec<EventQueueProcessWatchInfo>,
    pub signal_watches: Vec<EventQueueSignalWatchInfo>,
    pub memory_watches: Vec<EventQueueMemoryWatchInfo>,
    pub resource_watches: Vec<EventQueueResourceWatchInfo>,
    pub network_watches: Vec<EventQueueNetworkWatchInfo>,
    pub pending: Vec<EventQueuePendingInfo>,
    pub waiters: Vec<EventQueueWaiterInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepQueueWaiterInfo {
    pub owner: ProcessId,
    pub channel: u64,
    pub priority: u16,
    pub wake_hint: u16,
    pub deadline_tick: Option<u64>,
    pub result: SleepWaitResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepQueueInfo {
    pub id: SleepQueueId,
    pub owner: ProcessId,
    pub waiter_count: usize,
    pub channels: Vec<u64>,
    pub descriptor_ref_count: usize,
    pub signal_wait_owners: Vec<u64>,
    pub memory_wait_owners: Vec<u64>,
    pub waiters: Vec<SleepQueueWaiterInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemIntrospection {
    pub snapshot: RuntimeSnapshot,
    pub processes: Vec<ProcessInfo>,
    pub address_spaces: Vec<AddressSpaceInfo>,
    pub domains: Vec<DomainInfo>,
    pub resources: Vec<ResourceInfo>,
    pub contracts: Vec<ContractInfo>,
    pub resource_agent_decisions: Vec<ResourceAgentDecisionRecord>,
    pub wait_agent_decisions: Vec<WaitAgentDecisionRecord>,
    pub scheduler_agent_decisions: Vec<SchedulerAgentDecisionRecord>,
    pub io_agent_decisions: Vec<IoAgentDecisionRecord>,
    pub vm_agent_decisions: Vec<VmAgentDecisionRecord>,
    pub event_queues: Vec<EventQueueInfo>,
    pub sleep_queues: Vec<SleepQueueInfo>,
    pub fdshare_groups: Vec<FiledescShareGroupInfo>,
}
