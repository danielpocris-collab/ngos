use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EventQueueWaiter {
    pub(crate) owner: ProcessId,
    pub(crate) tid: ThreadId,
    pub(crate) class: SchedulerClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventTimerId(pub(crate) u64);

impl EventTimerId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessLifecycleEventKind {
    Exited,
    Reaped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessLifecycleInterest {
    pub exited: bool,
    pub reaped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryWaitEventKind {
    Woken,
    Requeued,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceEventKind {
    Claimed,
    Queued,
    Canceled,
    Released,
    HandedOff,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceEventInterest {
    pub claimed: bool,
    pub queued: bool,
    pub canceled: bool,
    pub released: bool,
    pub handed_off: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEventKind {
    LinkChanged,
    RxReady,
    TxDrained,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkEventInterest {
    pub link_changed: bool,
    pub rx_ready: bool,
    pub tx_drained: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsEventKind {
    Submitted,
    Completed,
    Failed,
    Drained,
    Canceled,
    Faulted,
    Recovered,
    Retired,
    LeaseReleased,
    LeaseAcquired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphicsEventInterest {
    pub submitted: bool,
    pub completed: bool,
    pub failed: bool,
    pub drained: bool,
    pub canceled: bool,
    pub faulted: bool,
    pub recovered: bool,
    pub retired: bool,
    pub lease_released: bool,
    pub lease_acquired: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryWaitKey {
    pub namespace: u64,
    pub addr: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryWaitDomain {
    Shared,
    Process(ProcessId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    Descriptor(Descriptor),
    Timer(EventTimerId),
    Process {
        pid: ProcessId,
        kind: ProcessLifecycleEventKind,
    },
    Signal {
        pid: ProcessId,
        tid: Option<ThreadId>,
        signal: u8,
    },
    MemoryWait {
        domain: MemoryWaitDomain,
        addr: u64,
        kind: MemoryWaitEventKind,
    },
    Resource {
        resource: ResourceId,
        contract: ContractId,
        kind: ResourceEventKind,
    },
    Network {
        interface_inode: u64,
        socket_inode: Option<u64>,
        kind: NetworkEventKind,
    },
    Graphics {
        device_inode: u64,
        request_id: u64,
        kind: GraphicsEventKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EventTimerRegistration {
    pub(crate) id: EventTimerId,
    pub(crate) token: u64,
    pub(crate) deadline_tick: u64,
    pub(crate) interval_ticks: Option<u64>,
    pub(crate) events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessEventRegistration {
    pub(crate) target: ProcessId,
    pub(crate) token: u64,
    pub(crate) interest: ProcessLifecycleInterest,
    pub(crate) events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SignalEventRegistration {
    pub(crate) target: ProcessId,
    pub(crate) thread: Option<ThreadId>,
    pub(crate) signal_mask: u64,
    pub(crate) token: u64,
    pub(crate) events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryWaitEventRegistration {
    pub(crate) domain: MemoryWaitDomain,
    pub(crate) addr: u64,
    pub(crate) token: u64,
    pub(crate) events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResourceEventRegistration {
    pub(crate) resource: ResourceId,
    pub(crate) token: u64,
    pub(crate) interest: ResourceEventInterest,
    pub(crate) events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NetworkEventRegistration {
    pub(crate) interface_inode: u64,
    pub(crate) socket_inode: Option<u64>,
    pub(crate) token: u64,
    pub(crate) interest: NetworkEventInterest,
    pub(crate) events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphicsEventRegistration {
    pub(crate) device_inode: u64,
    pub(crate) token: u64,
    pub(crate) interest: GraphicsEventInterest,
    pub(crate) events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KernelEvent {
    pub(crate) owner: ProcessId,
    pub(crate) token: u64,
    pub(crate) events: IoPollEvents,
    pub(crate) source: EventSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventQueueWaitResult {
    Ready(Vec<EventRecord>),
    Blocked(ProcessId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeSleepQueue {
    pub(crate) id: SleepQueueId,
    pub(crate) owner: ProcessId,
    pub(crate) waiters: SleepQueue<ProcessId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryWaiter {
    pub(crate) pid: ProcessId,
    pub(crate) queue: SleepQueueId,
}

pub(crate) const SIGNAL_WAIT_CHANNEL: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryWordWaitResult {
    Blocked(ProcessId),
    ValueMismatch { expected: u32, observed: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingSignalWaitResult {
    Delivered(PendingSignalDelivery),
    Blocked(ProcessId),
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingSignalWaitResume {
    Delivered(PendingSignalDelivery),
    TimedOut,
    Canceled,
    Restarted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingSignalSource {
    Process,
    Thread(ThreadId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingSignalCode {
    Kill,
    Tgkill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingSignalSender {
    pub pid: ProcessId,
    pub tid: ThreadId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingSignalDelivery {
    pub signal: u8,
    pub code: PendingSignalCode,
    pub value: Option<u64>,
    pub source: PendingSignalSource,
    pub sender: PendingSignalSender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDisposition {
    Catch,
    Ignore,
    Terminate,
}

pub(crate) fn default_signal_disposition(signal: u8) -> Result<SignalDisposition, ProcessError> {
    if signal == 0 || signal > 64 {
        return Err(ProcessError::InvalidSignal);
    }
    Ok(match signal {
        17 | 23 | 28 => SignalDisposition::Ignore,
        _ => SignalDisposition::Terminate,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryWordWaitEntry {
    pub namespace: u64,
    pub addr: u64,
    pub expected: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryWordWaitDomainEntry {
    pub domain: MemoryWaitDomain,
    pub addr: u64,
    pub expected: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryWordWaitAnyResult {
    Blocked { pid: ProcessId, index: usize },
    ValueMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWordRequeueResult {
    pub woke: Vec<ProcessId>,
    pub moved: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryWordCmpRequeueResult {
    ValueMismatch { expected: u32, observed: u32 },
    Requeued(MemoryWordRequeueResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventQueueId(pub(crate) u64);

impl EventQueueId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SleepQueueId(pub(crate) u64);

impl SleepQueueId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventQueueMode {
    Kqueue,
    Epoll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventMultiplexerFlavor {
    Kqueue,
    Epoll,
    Poll,
}

impl EventMultiplexerFlavor {
    pub(crate) const fn event_queue_mode(self) -> EventQueueMode {
        match self {
            Self::Kqueue => EventQueueMode::Kqueue,
            Self::Epoll | Self::Poll => EventQueueMode::Epoll,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMultiplexerDescriptor {
    pub fd: Descriptor,
    pub flavor: EventMultiplexerFlavor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMultiplexerFdWatch {
    pub fd: Descriptor,
    pub token: u64,
    pub interest: ReadinessInterest,
    pub behavior: EventWatchBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventMultiplexerFdOp {
    Add(EventMultiplexerFdWatch),
    Modify(EventMultiplexerFdWatch),
    Remove { fd: Descriptor },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMultiplexerTimerWatch {
    pub token: u64,
    pub delay_ticks: u64,
    pub interval_ticks: Option<u64>,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMultiplexerProcessWatch {
    pub target: ProcessId,
    pub token: u64,
    pub interest: ProcessLifecycleInterest,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMultiplexerSignalWatch {
    pub target: ProcessId,
    pub thread: Option<ThreadId>,
    pub signal_mask: u64,
    pub token: u64,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMultiplexerMemoryWatch {
    pub domain: MemoryWaitDomain,
    pub addr: u64,
    pub token: u64,
    pub events: IoPollEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMultiplexerPollRequest {
    pub fd: Descriptor,
    pub token: u64,
    pub interest: ReadinessInterest,
    pub behavior: EventWatchBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventWatchBehavior {
    pub edge_triggered: bool,
    pub oneshot: bool,
}

impl EventWatchBehavior {
    pub const LEVEL: Self = Self {
        edge_triggered: false,
        oneshot: false,
    };

    pub const EDGE: Self = Self {
        edge_triggered: true,
        oneshot: false,
    };

    pub const ONESHOT: Self = Self {
        edge_triggered: false,
        oneshot: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessInterest {
    pub readable: bool,
    pub writable: bool,
    pub priority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessRegistration {
    pub owner: ProcessId,
    pub fd: Descriptor,
    pub interest: ReadinessInterest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventWatch {
    pub owner: ProcessId,
    pub fd: Descriptor,
    pub token: u64,
    pub interest: ReadinessInterest,
    pub behavior: EventWatchBehavior,
    pub last_ready: IoPollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueue {
    pub id: EventQueueId,
    pub owner: ProcessId,
    pub mode: EventQueueMode,
    pub watches: Vec<EventWatch>,
    pub(crate) timers: Vec<EventTimerRegistration>,
    pub(crate) process_watchers: Vec<ProcessEventRegistration>,
    pub(crate) signal_watchers: Vec<SignalEventRegistration>,
    pub(crate) memory_watchers: Vec<MemoryWaitEventRegistration>,
    pub(crate) resource_watchers: Vec<ResourceEventRegistration>,
    pub(crate) network_watchers: Vec<NetworkEventRegistration>,
    pub(crate) graphics_watchers: Vec<GraphicsEventRegistration>,
    pub pending: BufRing<EventRecord>,
    pub(crate) waiters: Vec<EventQueueWaiter>,
}

impl EventQueue {
    const PENDING_CAPACITY: usize = 64;

    pub(crate) fn new(id: EventQueueId, owner: ProcessId, mode: EventQueueMode) -> Self {
        Self {
            id,
            owner,
            mode,
            watches: Vec::new(),
            timers: Vec::new(),
            process_watchers: Vec::new(),
            signal_watchers: Vec::new(),
            memory_watchers: Vec::new(),
            resource_watchers: Vec::new(),
            network_watchers: Vec::new(),
            graphics_watchers: Vec::new(),
            pending: BufRing::with_capacity(Self::PENDING_CAPACITY),
            waiters: Vec::new(),
        }
    }

    pub(crate) fn pending_snapshot(&self) -> Vec<EventRecord> {
        let mut pending = self.pending.clone();
        pending.pop_batch(pending.len())
    }

    pub(crate) fn drain_pending(&mut self) -> Vec<EventRecord> {
        self.pending.pop_batch(self.pending.len())
    }

    pub(crate) fn retain_pending<F>(&mut self, mut keep: F)
    where
        F: FnMut(&EventRecord) -> bool,
    {
        let drained = self.drain_pending();
        self.pending
            .push_batch(drained.into_iter().filter(|event| keep(event)));
    }

    pub(crate) fn enqueue_pending(&mut self, record: EventRecord) -> bool {
        let exists = self.pending_snapshot().iter().any(|pending| {
            pending.owner == record.owner
                && pending.token == record.token
                && pending.events == record.events
                && pending.source == record.source
        });
        if exists {
            return false;
        }
        if self.pending.push(record).is_err() {
            let _ = self.pending.pop();
            self.pending
                .push(record)
                .expect("event queue ring must accept a record after dropping the oldest one");
        }
        true
    }

    pub(crate) fn enqueue_waiter(&mut self, waiter: EventQueueWaiter) {
        if self
            .waiters
            .iter()
            .any(|candidate| candidate.owner == waiter.owner && candidate.tid == waiter.tid)
        {
            return;
        }
        self.waiters.push(waiter);
    }

    pub(crate) fn drain_waiters(&mut self) -> Vec<EventQueueWaiter> {
        core::mem::take(&mut self.waiters)
    }

    pub(crate) fn remove_owner_waiters(&mut self, owner: ProcessId) {
        self.waiters.retain(|waiter| waiter.owner != owner);
    }

    pub(crate) fn tick_timers(&mut self, now_tick: u64) -> Vec<EventTimerRegistration> {
        let mut fired = Vec::new();
        let mut retained = Vec::with_capacity(self.timers.len());
        for mut timer in self.timers.drain(..) {
            if timer.deadline_tick <= now_tick {
                fired.push(timer);
                if let Some(interval) = timer.interval_ticks {
                    timer.deadline_tick = now_tick.saturating_add(interval.max(1));
                    retained.push(timer);
                }
            } else {
                retained.push(timer);
            }
        }
        self.timers = retained;
        fired
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventRecord {
    pub queue: EventQueueId,
    pub mode: EventQueueMode,
    pub owner: ProcessId,
    pub token: u64,
    pub events: IoPollEvents,
    pub source: EventSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventQueueError {
    InvalidQueue,
    WatchNotFound,
    TimerNotFound,
    ProcessWatchNotFound,
    SignalWatchNotFound,
    MemoryWatchNotFound,
    ResourceWatchNotFound,
    NetworkWatchNotFound,
}

pub(crate) fn event_queue_descriptor_name(owner: ProcessId, queue: EventQueueId) -> String {
    format!("queue:event:{}:{}", owner.raw(), queue.raw())
}

pub(crate) fn sleep_queue_descriptor_name(owner: ProcessId, queue: SleepQueueId) -> String {
    format!("queue:sleep:{}:{}", owner.raw(), queue.raw())
}
