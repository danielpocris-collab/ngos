use super::*;

pub(crate) fn inspect_event_queue(
    runtime: &KernelRuntime,
    owner: ProcessId,
    queue: EventQueueId,
) -> Result<EventQueueInfo, RuntimeError> {
    let binding = runtime.event_queue_binding(owner, queue)?;
    event_queue_info_by_binding(runtime, binding)
}

pub(crate) fn inspect_event_queue_descriptor(
    runtime: &KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
) -> Result<EventQueueInfo, RuntimeError> {
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd)?;
    event_queue_info_by_binding(runtime, binding)
}

pub(crate) fn inspect_sleep_queue(
    runtime: &KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
) -> Result<SleepQueueInfo, RuntimeError> {
    let binding = runtime.sleep_queue_binding(owner, queue)?;
    sleep_queue_info_by_binding(runtime, binding)
}

pub(crate) fn inspect_sleep_queue_descriptor(
    runtime: &KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
) -> Result<SleepQueueInfo, RuntimeError> {
    let binding = runtime.sleep_queue_binding_for_fd(owner, queue_fd)?;
    sleep_queue_info_by_binding(runtime, binding)
}

pub(crate) fn event_queue_info_by_binding(
    runtime: &KernelRuntime,
    binding: QueueDescriptorTarget,
) -> Result<EventQueueInfo, RuntimeError> {
    match binding {
        QueueDescriptorTarget::Event { owner, queue, .. } => {
            let queue = runtime
                .event_queues
                .iter()
                .find(|candidate| candidate.id == queue && candidate.owner == owner)
                .ok_or(EventQueueError::InvalidQueue)?;
            Ok(event_queue_info(runtime, queue))
        }
        QueueDescriptorTarget::Sleep { .. } => {
            Err(RuntimeError::EventQueue(EventQueueError::InvalidQueue))
        }
    }
}

pub(crate) fn sleep_queue_info_by_binding(
    runtime: &KernelRuntime,
    binding: QueueDescriptorTarget,
) -> Result<SleepQueueInfo, RuntimeError> {
    match binding {
        QueueDescriptorTarget::Sleep { owner, queue } => {
            let queue = runtime
                .sleep_queues
                .iter()
                .find(|candidate| candidate.id == queue && candidate.owner == owner)
                .ok_or(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound))?;
            Ok(sleep_queue_info(runtime, queue))
        }
        QueueDescriptorTarget::Event { .. } => {
            Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound))
        }
    }
}

pub(crate) fn event_queue_info(runtime: &KernelRuntime, queue: &EventQueue) -> EventQueueInfo {
    let binding = QueueDescriptorTarget::Event {
        owner: queue.owner,
        queue: queue.id,
        mode: queue.mode,
    };
    EventQueueInfo {
        id: queue.id,
        owner: queue.owner,
        mode: queue.mode,
        watch_count: queue.watches.len(),
        timer_count: queue.timers.len(),
        process_watch_count: queue.process_watchers.len(),
        signal_watch_count: queue.signal_watchers.len(),
        memory_watch_count: queue.memory_watchers.len(),
        resource_watch_count: queue.resource_watchers.len(),
        network_watch_count: queue.network_watchers.len(),
        bus_watch_count: queue.bus_watchers.len(),
        pending_count: queue.pending.len(),
        waiter_count: queue.waiters.len(),
        descriptor_ref_count: descriptor_runtime::queue_descriptor_reference_count(runtime, binding),
        deferred_refresh_pending: runtime.deferred_tasks.snapshot().iter().any(|(task, _, _)| {
            matches!(task, DeferredRuntimeTask::RefreshEventQueue(target) if *target == binding)
        }),
        watches: queue
            .watches
            .iter()
            .map(|watch| EventQueueWatchInfo {
                owner: watch.owner,
                fd: watch.fd,
                token: watch.token,
                interest: watch.interest,
                behavior: watch.behavior,
                last_ready: watch.last_ready,
            })
            .collect(),
        timers: queue
            .timers
            .iter()
            .map(|timer| EventQueueTimerInfo {
                id: timer.id,
                token: timer.token,
                deadline_tick: timer.deadline_tick,
                interval_ticks: timer.interval_ticks,
                events: timer.events,
            })
            .collect(),
        process_watches: queue
            .process_watchers
            .iter()
            .map(|watch| EventQueueProcessWatchInfo {
                target: watch.target,
                token: watch.token,
                interest: watch.interest,
                events: watch.events,
            })
            .collect(),
        signal_watches: queue
            .signal_watchers
            .iter()
            .map(|watch| EventQueueSignalWatchInfo {
                target: watch.target,
                thread: watch.thread,
                signal_mask: watch.signal_mask,
                token: watch.token,
                events: watch.events,
            })
            .collect(),
        memory_watches: queue
            .memory_watchers
            .iter()
            .map(|watch| EventQueueMemoryWatchInfo {
                domain: watch.domain,
                addr: watch.addr,
                token: watch.token,
                events: watch.events,
            })
            .collect(),
        resource_watches: queue
            .resource_watchers
            .iter()
            .map(|watch| EventQueueResourceWatchInfo {
                resource: watch.resource,
                token: watch.token,
                interest: watch.interest,
                events: watch.events,
            })
            .collect(),
        network_watches: queue
            .network_watchers
            .iter()
            .map(|watch| EventQueueNetworkWatchInfo {
                interface_inode: watch.interface_inode,
                socket_inode: watch.socket_inode,
                token: watch.token,
                interest: watch.interest,
                events: watch.events,
            })
            .collect(),
        bus_watches: queue
            .bus_watchers
            .iter()
            .map(|watch| EventQueueBusWatchInfo {
                endpoint: watch.endpoint,
                token: watch.token,
                interest: watch.interest,
                events: watch.events,
            })
            .collect(),
        pending: queue
            .pending_snapshot()
            .into_iter()
            .map(|event| EventQueuePendingInfo {
                owner: event.owner,
                token: event.token,
                events: event.events,
                source: event.source,
            })
            .collect(),
        waiters: queue
            .waiters
            .iter()
            .map(|waiter| EventQueueWaiterInfo {
                owner: waiter.owner,
                tid: waiter.tid,
                class: waiter.class,
            })
            .collect(),
    }
}

pub(crate) fn sleep_queue_info(
    runtime: &KernelRuntime,
    queue: &RuntimeSleepQueue,
) -> SleepQueueInfo {
    let binding = QueueDescriptorTarget::Sleep {
        owner: queue.owner,
        queue: queue.id,
    };
    SleepQueueInfo {
        id: queue.id,
        owner: queue.owner,
        waiter_count: queue.waiters.len(),
        channels: queue
            .waiters
            .waiters()
            .iter()
            .map(|waiter| waiter.channel)
            .collect(),
        descriptor_ref_count: descriptor_runtime::queue_descriptor_reference_count(
            runtime, binding,
        ),
        signal_wait_owners: runtime
            .signal_wait_queues
            .iter()
            .filter_map(|(pid, queued)| (*queued == queue.id).then_some(*pid))
            .collect(),
        memory_wait_owners: runtime
            .memory_wait_queues
            .iter()
            .filter_map(|(pid, queued)| (*queued == queue.id).then_some(*pid))
            .collect(),
        waiters: queue
            .waiters
            .waiters()
            .iter()
            .map(|waiter| SleepQueueWaiterInfo {
                owner: waiter.owner,
                channel: waiter.channel,
                priority: waiter.priority,
                wake_hint: waiter.wake_hint,
                deadline_tick: waiter.deadline_tick,
                result: waiter.result,
            })
            .collect(),
    }
}

pub(crate) fn render_procfs_system_queues(runtime: &KernelRuntime) -> Result<String, RuntimeError> {
    let capacity = runtime.event_queues.len().saturating_mul(112)
        + runtime.sleep_queues.len().saturating_mul(112);
    let mut out = KernelBuffer::with_capacity(capacity.max(128));

    for queue in &runtime.event_queues {
        let info = event_queue_info(runtime, queue);
        writeln!(
            out,
            "event\towner={}\tid={}\tmode={:?}\twatches={}\ttimers={}\tprocwatches={}\tsigwatches={}\tmemwatches={}\tresourcewatches={}\tnetwatches={}\tpending={}\twaiters={}\tdescriptors={}\tdeferred={}",
            info.owner.raw(),
            info.id.raw(),
            info.mode,
            info.watch_count,
            info.timer_count,
            info.process_watch_count,
            info.signal_watch_count,
            info.memory_watch_count,
            info.resource_watch_count,
            info.network_watch_count,
            info.pending_count,
            info.waiter_count,
            info.descriptor_ref_count,
            info.deferred_refresh_pending,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }

    for queue in &runtime.sleep_queues {
        let info = sleep_queue_info(runtime, queue);
        writeln!(
            out,
            "sleep\towner={}\tid={}\twaiters={}\tdescriptors={}\tsignal-owners={}\tmemory-owners={}",
            info.owner.raw(),
            info.id.raw(),
            info.waiter_count,
            info.descriptor_ref_count,
            info.signal_wait_owners.len(),
            info.memory_wait_owners.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }

    out.finish()?;
    Ok(out
        .as_str()
        .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
        .to_owned())
}

pub(crate) fn render_procfs_queues(
    runtime: &KernelRuntime,
    pid: ProcessId,
) -> Result<String, RuntimeError> {
    runtime.processes.get(pid)?;
    let event_queues = runtime
        .event_queues
        .iter()
        .filter(|queue| queue.owner == pid)
        .map(|queue| event_queue_info(runtime, queue))
        .collect::<Vec<_>>();
    let sleep_queues = runtime
        .sleep_queues
        .iter()
        .filter(|queue| queue.owner == pid)
        .map(|queue| sleep_queue_info(runtime, queue))
        .collect::<Vec<_>>();
    let capacity = event_queues.len().saturating_mul(96) + sleep_queues.len().saturating_mul(96);
    let mut out = KernelBuffer::with_capacity(capacity.max(96));

    for queue in event_queues {
        writeln!(
            out,
            "event\t{}\t{:?}\twatches={}\ttimers={}\tprocwatches={}\tsigwatches={}\tmemwatches={}\tresourcewatches={}\tpending={}\twaiters={}\tdescriptors={}\tdeferred={}",
            queue.id.raw(),
            queue.mode,
            queue.watch_count,
            queue.timer_count,
            queue.process_watch_count,
            queue.signal_watch_count,
            queue.memory_watch_count,
            queue.resource_watch_count,
            queue.pending_count,
            queue.waiter_count,
            queue.descriptor_ref_count,
            queue.deferred_refresh_pending,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for queue in sleep_queues {
        writeln!(
            out,
            "sleep\t{}\twaiters={}\tdescriptors={}\tsignal-owners={}\tmemory-owners={}",
            queue.id.raw(),
            queue.waiter_count,
            queue.descriptor_ref_count,
            queue.signal_wait_owners.len(),
            queue.memory_wait_owners.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }

    out.finish()?;
    Ok(out
        .as_str()
        .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
        .to_owned())
}

pub(crate) fn render_procfs_event_queue(
    runtime: &KernelRuntime,
    pid: ProcessId,
    queue: EventQueueId,
) -> Result<String, RuntimeError> {
    let info = inspect_event_queue(runtime, pid, queue)?;
    let capacity = 192
        + info.watches.len().saturating_mul(128)
        + info.pending.len().saturating_mul(112)
        + info.signal_watches.len().saturating_mul(96)
        + info.memory_watches.len().saturating_mul(96)
        + info.resource_watches.len().saturating_mul(96);
    let mut out = KernelBuffer::with_capacity(capacity);
    write!(
        out,
        "id:\t{}\nowner:\t{}\nmode:\t{:?}\nwatches:\t{}\ntimers:\t{}\nprocess-watches:\t{}\nsignal-watches:\t{}\nmemory-watches:\t{}\nresource-watches:\t{}\npending:\t{}\nwaiters:\t{}\ndescriptors:\t{}\ndeferred-refresh:\t{}\n",
        info.id.raw(),
        info.owner.raw(),
        info.mode,
        info.watch_count,
        info.timer_count,
        info.process_watch_count,
        info.signal_watch_count,
        info.memory_watch_count,
        info.resource_watch_count,
        info.pending_count,
        info.waiter_count,
        info.descriptor_ref_count,
        info.deferred_refresh_pending,
    )
    .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    for watch in &info.watches {
        writeln!(
            out,
            "watch\towner={}\tfd={}\ttoken={}\tinterest=r{}w{}p{}\tbehavior=edge:{} oneshot:{}\tlast=0x{:x}",
            watch.owner.raw(),
            watch.fd.raw(),
            watch.token,
            u8::from(watch.interest.readable),
            u8::from(watch.interest.writable),
            u8::from(watch.interest.priority),
            watch.behavior.edge_triggered,
            watch.behavior.oneshot,
            watch.last_ready.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for timer in &info.timers {
        writeln!(
            out,
            "timer\tid={}\ttoken={}\tdeadline={}\tinterval={:?}\tevents=0x{:x}",
            timer.id.raw(),
            timer.token,
            timer.deadline_tick,
            timer.interval_ticks,
            timer.events.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for watch in &info.process_watches {
        writeln!(
            out,
            "procwatch\ttarget={}\ttoken={}\tinterest=exit:{} reap:{}\tevents=0x{:x}",
            watch.target.raw(),
            watch.token,
            watch.interest.exited,
            watch.interest.reaped,
            watch.events.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for watch in &info.signal_watches {
        writeln!(
            out,
            "sigwatch\ttarget={}\tthread={:?}\tmask=0x{:x}\ttoken={}\tevents=0x{:x}",
            watch.target.raw(),
            watch.thread.map(ThreadId::raw),
            watch.signal_mask,
            watch.token,
            watch.events.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for watch in &info.memory_watches {
        writeln!(
            out,
            "memwatch\tdomain={:?}\taddr=0x{:x}\ttoken={}\tevents=0x{:x}",
            watch.domain, watch.addr, watch.token, watch.events.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for watch in &info.resource_watches {
        writeln!(
            out,
            "resourcewatch\tresource={}\ttoken={}\tinterest=claim:{} queue:{} cancel:{} release:{} handoff:{} revoke:{}\tevents=0x{:x}",
            watch.resource.raw(),
            watch.token,
            watch.interest.claimed,
            watch.interest.queued,
            watch.interest.canceled,
            watch.interest.released,
            watch.interest.handed_off,
            watch.interest.revoked,
            watch.events.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for watch in &info.network_watches {
        writeln!(
            out,
            "networkwatch\tiface_inode={}\tsocket_inode={:?}\ttoken={}\tinterest=link:{} rx:{} tx_drain:{}\tevents=0x{:x}",
            watch.interface_inode,
            watch.socket_inode,
            watch.token,
            watch.interest.link_changed,
            watch.interest.rx_ready,
            watch.interest.tx_drained,
            watch.events.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for watch in &info.bus_watches {
        writeln!(
            out,
            "buswatch\tendpoint={}\ttoken={}\tinterest=attach:{} detach:{} publish:{} receive:{}\tevents=0x{:x}",
            watch.endpoint.raw(),
            watch.token,
            watch.interest.attached,
            watch.interest.detached,
            watch.interest.published,
            watch.interest.received,
            watch.events.0,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for event in &info.pending {
        match event.source {
            EventSource::Descriptor(fd) => writeln!(
                out,
                "pending\towner={}\tsource=fd:{}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                fd.raw(),
                event.token,
                event.events.0,
            ),
            EventSource::Timer(timer) => writeln!(
                out,
                "pending\towner={}\tsource=timer:{}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                timer.raw(),
                event.token,
                event.events.0,
            ),
            EventSource::Process { pid, kind } => writeln!(
                out,
                "pending\towner={}\tsource=process:{}:{:?}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                pid.raw(),
                kind,
                event.token,
                event.events.0,
            ),
            EventSource::Signal { pid, tid, signal } => writeln!(
                out,
                "pending\towner={}\tsource=signal:{}:{:?}:{}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                pid.raw(),
                tid.map(ThreadId::raw),
                signal,
                event.token,
                event.events.0,
            ),
            EventSource::MemoryWait { domain, addr, kind } => writeln!(
                out,
                "pending\towner={}\tsource=mem:{:?}:0x{:x}:{:?}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                domain,
                addr,
                kind,
                event.token,
                event.events.0,
            ),
            EventSource::Resource {
                resource,
                contract,
                kind,
            } => writeln!(
                out,
                "pending\towner={}\tsource=resource:{}:{}:{:?}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                resource.raw(),
                contract.raw(),
                kind,
                event.token,
                event.events.0,
            ),
            EventSource::Network {
                interface_inode,
                socket_inode,
                kind,
            } => writeln!(
                out,
                "pending\towner={}\tsource=network:{}:{:?}:{:?}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                interface_inode,
                socket_inode,
                kind,
                event.token,
                event.events.0,
            ),
            EventSource::Graphics {
                device_inode,
                request_id,
                kind,
            } => writeln!(
                out,
                "pending\towner={}\tsource=graphics:{}:{}:{:?}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                device_inode,
                request_id,
                kind,
                event.token,
                event.events.0,
            ),
            EventSource::Bus {
                peer,
                endpoint,
                kind,
            } => writeln!(
                out,
                "pending\towner={}\tsource=bus:{}:{}:{:?}\ttoken={}\tevents=0x{:x}",
                event.owner.raw(),
                peer.raw(),
                endpoint.raw(),
                kind,
                event.token,
                event.events.0,
            ),
        }
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    for waiter in &info.waiters {
        writeln!(
            out,
            "waiter\towner={}\ttid={}\tclass={:?}",
            waiter.owner.raw(),
            waiter.tid.raw(),
            waiter.class,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    out.finish()?;
    Ok(out
        .as_str()
        .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
        .to_owned())
}

pub(crate) fn render_procfs_sleep_queue(
    runtime: &KernelRuntime,
    pid: ProcessId,
    queue: SleepQueueId,
) -> Result<String, RuntimeError> {
    let info = inspect_sleep_queue(runtime, pid, queue)?;
    let capacity = 128 + info.waiters.len().saturating_mul(128);
    let mut out = KernelBuffer::with_capacity(capacity);
    write!(
        out,
        "id:\t{}\nowner:\t{}\nwaiters:\t{}\ndescriptors:\t{}\nsignal-owners:\t{:?}\nmemory-owners:\t{:?}\n",
        info.id.raw(),
        info.owner.raw(),
        info.waiter_count,
        info.descriptor_ref_count,
        info.signal_wait_owners,
        info.memory_wait_owners,
    )
    .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    for waiter in &info.waiters {
        writeln!(
            out,
            "waiter\towner={}\tchannel=0x{:x}\tprio={}\thint={}\tdeadline={}\tresult={:?}",
            waiter.owner.raw(),
            waiter.channel,
            waiter.priority,
            waiter.wake_hint,
            waiter
                .deadline_tick
                .map(|deadline| deadline.to_string())
                .unwrap_or_else(|| String::from("-")),
            waiter.result,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
    }
    out.finish()?;
    Ok(out
        .as_str()
        .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
        .to_owned())
}
