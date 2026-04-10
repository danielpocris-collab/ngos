use super::*;
use crate::eventing_model::{
    BusEventInterest, BusEventRegistration, GraphicsEventInterest, GraphicsEventRegistration,
};

pub(crate) fn create_event_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    mode: EventQueueMode,
) -> Result<EventQueueId, RuntimeError> {
    runtime.processes.get(owner)?;
    let id = EventQueueId(runtime.next_event_queue_id);
    runtime.next_event_queue_id = runtime.next_event_queue_id.saturating_add(1);
    runtime.event_queues.push(EventQueue::new(id, owner, mode));
    Ok(id)
}

pub(crate) fn create_event_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    mode: EventQueueMode,
) -> Result<Descriptor, RuntimeError> {
    let queue = create_event_queue(runtime, owner, mode)?;
    open_event_queue_descriptor(runtime, owner, queue)
}

pub(crate) fn open_event_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: EventQueueId,
) -> Result<Descriptor, RuntimeError> {
    runtime.processes.get(owner)?;
    let mode = runtime
        .event_queue_mode(owner, queue)
        .ok_or(EventQueueError::InvalidQueue)?;
    let binding = QueueDescriptorTarget::Event { owner, queue, mode };
    let fd = descriptor_runtime::open_runtime_queue_descriptor(
        runtime,
        owner,
        ObjectKind::EventQueue,
        binding,
        event_queue_descriptor_name(owner, queue),
    )?;
    sync_event_queue_readability(runtime, binding)?;
    Ok(fd)
}

pub(crate) fn register_event_queue_timer_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    token: u64,
    delay_ticks: u64,
    interval_ticks: Option<u64>,
    events: IoPollEvents,
) -> Result<EventTimerId, RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let id = EventTimerId(runtime.next_event_timer_id);
    runtime.next_event_timer_id = runtime.next_event_timer_id.saturating_add(1);
    let deadline_tick = runtime.current_tick.saturating_add(delay_ticks.max(1));
    event_queue_mut_by_binding(runtime, binding)?
        .timers
        .push(EventTimerRegistration {
            id,
            token,
            deadline_tick,
            interval_ticks,
            events,
        });
    Ok(id)
}

pub(crate) fn remove_event_queue_timer_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    timer: EventTimerId,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.timers.len();
    queue.timers.retain(|candidate| candidate.id != timer);
    if queue.timers.len() == original_len {
        return Err(EventQueueError::TimerNotFound.into());
    }
    Ok(())
}

pub(crate) fn watch_process_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    target: ProcessId,
    token: u64,
    interest: ProcessLifecycleInterest,
    events: IoPollEvents,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    runtime.processes.get(target)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    queue
        .process_watchers
        .retain(|watch| !(watch.target == target && watch.token == token));
    queue.process_watchers.push(ProcessEventRegistration {
        target,
        token,
        interest,
        events,
    });
    Ok(())
}

pub(crate) fn remove_process_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    target: ProcessId,
    token: u64,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.process_watchers.len();
    queue
        .process_watchers
        .retain(|watch| !(watch.target == target && watch.token == token));
    if queue.process_watchers.len() == original_len {
        return Err(EventQueueError::ProcessWatchNotFound.into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn watch_signal_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    target: ProcessId,
    thread: Option<ThreadId>,
    signal_mask: u64,
    token: u64,
    events: IoPollEvents,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    runtime.processes.get(target)?;
    if let Some(tid) = thread {
        let process = runtime.processes.get(target)?;
        if !process.threads().contains(&tid) {
            return Err(ProcessError::InvalidTid.into());
        }
    }
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    queue.signal_watchers.retain(|watch| {
        !(watch.target == target && watch.thread == thread && watch.token == token)
    });
    queue.signal_watchers.push(SignalEventRegistration {
        target,
        thread,
        signal_mask,
        token,
        events,
    });
    Ok(())
}

pub(crate) fn remove_signal_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    target: ProcessId,
    thread: Option<ThreadId>,
    token: u64,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.signal_watchers.len();
    queue.signal_watchers.retain(|watch| {
        !(watch.target == target && watch.thread == thread && watch.token == token)
    });
    if queue.signal_watchers.len() == original_len {
        return Err(EventQueueError::SignalWatchNotFound.into());
    }
    Ok(())
}

pub(crate) fn watch_memory_wait_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    domain: MemoryWaitDomain,
    addr: u64,
    token: u64,
    events: IoPollEvents,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let _ = memory_wait_runtime::resolve_memory_wait_domain(runtime, domain)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    queue
        .memory_watchers
        .retain(|watch| !(watch.domain == domain && watch.addr == addr && watch.token == token));
    queue.memory_watchers.push(MemoryWaitEventRegistration {
        domain,
        addr,
        token,
        events,
    });
    Ok(())
}

pub(crate) fn remove_memory_wait_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    domain: MemoryWaitDomain,
    addr: u64,
    token: u64,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.memory_watchers.len();
    queue
        .memory_watchers
        .retain(|watch| !(watch.domain == domain && watch.addr == addr && watch.token == token));
    if queue.memory_watchers.len() == original_len {
        return Err(EventQueueError::MemoryWatchNotFound.into());
    }
    Ok(())
}

pub(crate) fn watch_resource_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    resource: ResourceId,
    token: u64,
    interest: ResourceEventInterest,
    events: IoPollEvents,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    runtime.resources.get(resource)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    queue
        .resource_watchers
        .retain(|watch| !(watch.resource == resource && watch.token == token));
    queue.resource_watchers.push(ResourceEventRegistration {
        resource,
        token,
        interest,
        events,
    });
    Ok(())
}

pub(crate) fn remove_resource_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    resource: ResourceId,
    token: u64,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.resource_watchers.len();
    queue
        .resource_watchers
        .retain(|watch| !(watch.resource == resource && watch.token == token));
    if queue.resource_watchers.len() == original_len {
        return Err(EventQueueError::ResourceWatchNotFound.into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn watch_network_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    interface_inode: u64,
    socket_inode: Option<u64>,
    token: u64,
    interest: NetworkEventInterest,
    events: IoPollEvents,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    queue.network_watchers.retain(|watch| {
        !(watch.interface_inode == interface_inode
            && watch.socket_inode == socket_inode
            && watch.token == token)
    });
    queue.network_watchers.push(NetworkEventRegistration {
        interface_inode,
        socket_inode,
        token,
        interest,
        events,
    });
    Ok(())
}

pub(crate) fn remove_network_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    interface_inode: u64,
    socket_inode: Option<u64>,
    token: u64,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.network_watchers.len();
    queue.network_watchers.retain(|watch| {
        !(watch.interface_inode == interface_inode
            && watch.socket_inode == socket_inode
            && watch.token == token)
    });
    if queue.network_watchers.len() == original_len {
        return Err(EventQueueError::NetworkWatchNotFound.into());
    }
    Ok(())
}

pub(crate) fn watch_bus_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    endpoint: BusEndpointId,
    token: u64,
    interest: BusEventInterest,
    events: IoPollEvents,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    runtime.bus_endpoint_info(endpoint)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    queue
        .bus_watchers
        .retain(|watch| !(watch.endpoint == endpoint && watch.token == token));
    queue.bus_watchers.push(BusEventRegistration {
        endpoint,
        token,
        interest,
        events,
    });
    Ok(())
}

pub(crate) fn remove_bus_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    endpoint: BusEndpointId,
    token: u64,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.bus_watchers.len();
    queue
        .bus_watchers
        .retain(|watch| !(watch.endpoint == endpoint && watch.token == token));
    if queue.bus_watchers.len() == original_len {
        return Err(EventQueueError::BusWatchNotFound.into());
    }
    Ok(())
}

pub(crate) fn watch_graphics_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    device_inode: u64,
    token: u64,
    interest: GraphicsEventInterest,
    events: IoPollEvents,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    queue
        .graphics_watchers
        .retain(|watch| !(watch.device_inode == device_inode && watch.token == token));
    queue.graphics_watchers.push(GraphicsEventRegistration {
        device_inode,
        token,
        interest,
        events,
    });
    Ok(())
}

pub(crate) fn remove_graphics_events_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    device_inode: u64,
    token: u64,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let original_len = queue.graphics_watchers.len();
    queue
        .graphics_watchers
        .retain(|watch| !(watch.device_inode == device_inode && watch.token == token));
    if queue.graphics_watchers.len() == original_len {
        return Err(EventQueueError::NetworkWatchNotFound.into());
    }
    Ok(())
}

pub(crate) fn destroy_event_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
) -> Result<(), RuntimeError> {
    let _ = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let _ = descriptor_runtime::close_descriptor(runtime, owner, queue_fd)?;
    Ok(())
}

pub(crate) fn destroy_event_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: EventQueueId,
) -> Result<(), RuntimeError> {
    let fd = open_event_queue_descriptor(runtime, owner, queue)?;
    destroy_event_queue_descriptor(runtime, owner, fd)
}

pub(crate) fn watch_event(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: EventQueueId,
    fd: Descriptor,
    token: u64,
    interest: ReadinessInterest,
    behavior: EventWatchBehavior,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding(owner, queue)?;
    watch_event_with_binding(runtime, owner, binding, fd, token, interest, behavior)
}

pub(crate) fn watch_event_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    fd: Descriptor,
    token: u64,
    interest: ReadinessInterest,
    behavior: EventWatchBehavior,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    watch_event_with_binding(runtime, owner, binding, fd, token, interest, behavior)
}

pub(crate) fn watch_event_with_binding(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    binding: QueueDescriptorTarget,
    fd: Descriptor,
    token: u64,
    interest: ReadinessInterest,
    behavior: EventWatchBehavior,
) -> Result<(), RuntimeError> {
    runtime.inspect_io(owner, fd)?;
    {
        let queue_ref = event_queue_mut_by_binding(runtime, binding)?;
        queue_ref
            .watches
            .retain(|watch| !(watch.owner == owner && watch.fd == fd));
        queue_ref.retain_pending(|event| {
            !(event.owner == owner
                && matches!(event.source, EventSource::Descriptor(event_fd) if event_fd == fd))
        });
        queue_ref.watches.push(EventWatch {
            owner,
            fd,
            token,
            interest,
            behavior,
            last_ready: IoPollEvents::empty(),
        });
    }
    sync_event_queue_readability(runtime, binding)?;
    enqueue_event_queue_refresh(runtime, binding, 32)?;
    Ok(())
}

pub(crate) fn modify_watched_event(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: EventQueueId,
    fd: Descriptor,
    token: u64,
    interest: ReadinessInterest,
    behavior: EventWatchBehavior,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding(owner, queue)?;
    modify_watched_event_with_binding(runtime, owner, binding, fd, token, interest, behavior)
}

pub(crate) fn modify_watched_event_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    fd: Descriptor,
    token: u64,
    interest: ReadinessInterest,
    behavior: EventWatchBehavior,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    modify_watched_event_with_binding(runtime, owner, binding, fd, token, interest, behavior)
}

pub(crate) fn modify_watched_event_with_binding(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    binding: QueueDescriptorTarget,
    fd: Descriptor,
    token: u64,
    interest: ReadinessInterest,
    behavior: EventWatchBehavior,
) -> Result<(), RuntimeError> {
    runtime.inspect_io(owner, fd)?;
    {
        let queue_ref = event_queue_mut_by_binding(runtime, binding)?;
        let watch = queue_ref
            .watches
            .iter_mut()
            .find(|watch| watch.owner == owner && watch.fd == fd)
            .ok_or(EventQueueError::WatchNotFound)?;
        watch.token = token;
        watch.interest = interest;
        watch.behavior = behavior;
        watch.last_ready = IoPollEvents::empty();
        queue_ref.retain_pending(|event| {
            !(event.owner == owner
                && matches!(event.source, EventSource::Descriptor(event_fd) if event_fd == fd))
        });
    }
    sync_event_queue_readability(runtime, binding)?;
    enqueue_event_queue_refresh(runtime, binding, 32)?;
    Ok(())
}

pub(crate) fn remove_watched_event(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: EventQueueId,
    fd: Descriptor,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding(owner, queue)?;
    remove_watched_event_with_binding(runtime, owner, binding, fd)
}

pub(crate) fn remove_watched_event_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    fd: Descriptor,
) -> Result<(), RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    remove_watched_event_with_binding(runtime, owner, binding, fd)
}

pub(crate) fn remove_watched_event_with_binding(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    binding: QueueDescriptorTarget,
    fd: Descriptor,
) -> Result<(), RuntimeError> {
    {
        let queue_ref = event_queue_mut_by_binding(runtime, binding)?;
        let original_len = queue_ref.watches.len();
        queue_ref
            .watches
            .retain(|watch| !(watch.owner == owner && watch.fd == fd));
        queue_ref.retain_pending(|event| {
            !(event.owner == owner
                && matches!(event.source, EventSource::Descriptor(event_fd) if event_fd == fd))
        });
        if queue_ref.watches.len() == original_len {
            return Err(EventQueueError::WatchNotFound.into());
        }
    }
    sync_event_queue_readability(runtime, binding)?;
    enqueue_event_queue_refresh(runtime, binding, 32)?;
    Ok(())
}

pub(crate) fn wait_event_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: EventQueueId,
) -> Result<Vec<EventRecord>, RuntimeError> {
    let binding = runtime.event_queue_binding(owner, queue)?;
    wait_event_queue_now(runtime, binding)
}

pub(crate) fn wait_event_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    tid: ThreadId,
) -> Result<EventQueueWaitResult, RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    let descriptor = runtime
        .namespace(owner)?
        .get(queue_fd)
        .map_err(RuntimeError::from)?
        .clone();
    wait_event_queue_descriptor_thread(runtime, owner, tid, descriptor, binding)
}

pub(crate) fn wait_event_queue_now(
    runtime: &mut KernelRuntime,
    binding: QueueDescriptorTarget,
) -> Result<Vec<EventRecord>, RuntimeError> {
    enqueue_event_queue_refresh(runtime, binding, 64)?;
    flush_deferred_tasks(runtime)?;
    runtime.drain_event_queue(binding)
}

pub(crate) fn wait_event_queue_descriptor_thread(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    tid: ThreadId,
    descriptor: ObjectDescriptor,
    binding: QueueDescriptorTarget,
) -> Result<EventQueueWaitResult, RuntimeError> {
    let ready = wait_event_queue_now(runtime, binding)?;
    if !ready.is_empty() {
        return Ok(EventQueueWaitResult::Ready(ready));
    }
    if descriptor.nonblock() {
        return Ok(EventQueueWaitResult::Ready(Vec::new()));
    }
    let running = runtime
        .scheduler
        .running()
        .cloned()
        .ok_or(RuntimeError::Scheduler(SchedulerError::NoRunnableProcess))?;
    if running.pid != owner || running.tid != tid {
        let state = runtime.processes.get(owner)?.state();
        return Err(RuntimeError::Scheduler(
            SchedulerError::InvalidProcessState(state),
        ));
    }
    event_queue_mut_by_binding(runtime, binding)?.enqueue_waiter(EventQueueWaiter {
        owner,
        tid,
        class: running.class,
    });
    runtime.scheduler.block_running(&mut runtime.processes)?;
    Ok(EventQueueWaitResult::Blocked(owner))
}
