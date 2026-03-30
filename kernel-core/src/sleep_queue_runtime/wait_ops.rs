use super::*;

pub(crate) fn sleep_on_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
    channel: u64,
    priority: u16,
    timeout_ticks: Option<u64>,
) -> Result<ProcessId, RuntimeError> {
    let binding = runtime.sleep_queue_binding(owner, queue)?;
    sleep_on_queue_with_binding(runtime, owner, binding, channel, priority, timeout_ticks)
}

pub(crate) fn sleep_on_queue_thread(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    tid: ThreadId,
    queue: SleepQueueId,
    channel: u64,
    priority: u16,
    timeout_ticks: Option<u64>,
) -> Result<ProcessId, RuntimeError> {
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
    let binding = runtime.sleep_queue_binding(owner, queue)?;
    sleep_on_queue_with_binding(runtime, owner, binding, channel, priority, timeout_ticks)
}

pub(crate) fn sleep_on_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    channel: u64,
    priority: u16,
    timeout_ticks: Option<u64>,
) -> Result<ProcessId, RuntimeError> {
    let binding = runtime.sleep_queue_binding_for_fd(owner, queue_fd)?;
    sleep_on_queue_with_binding(runtime, owner, binding, channel, priority, timeout_ticks)
}

pub(crate) fn sleep_on_queue_with_binding(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    binding: QueueDescriptorTarget,
    channel: u64,
    priority: u16,
    timeout_ticks: Option<u64>,
) -> Result<ProcessId, RuntimeError> {
    let running = runtime
        .scheduler
        .running()
        .cloned()
        .ok_or(RuntimeError::Scheduler(SchedulerError::NoRunnableProcess))?;
    if running.pid != owner {
        let state = runtime.processes.get(owner)?.state();
        return Err(RuntimeError::Scheduler(
            SchedulerError::InvalidProcessState(state),
        ));
    }

    let deadline = timeout_ticks.map(|ticks| runtime.current_tick.saturating_add(ticks));
    runtime.sleep_results.remove(&owner.raw());
    let queue = match binding {
        QueueDescriptorTarget::Sleep { queue, .. } => queue,
        QueueDescriptorTarget::Event { .. } => {
            return Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound));
        }
    };
    sleep_queue_mut_by_binding(runtime, binding)?
        .waiters
        .enqueue(
            owner,
            channel,
            priority,
            running.class.index() as u16,
            deadline,
        )?;
    record_wait_agent_decision(
        runtime,
        WaitAgentKind::SleepEnqueueAgent,
        owner,
        queue,
        channel,
        u64::from(priority),
        deadline.unwrap_or(0),
    );
    runtime.scheduler.block_running(&mut runtime.processes)?;
    Ok(owner)
}

pub(crate) fn wake_one_sleep_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
    channel: u64,
) -> Result<Option<ProcessId>, RuntimeError> {
    let binding = runtime.sleep_queue_binding(owner, queue)?;
    wake_one_sleep_queue_with_binding(runtime, owner, binding, channel)
}

pub(crate) fn wake_one_sleep_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    channel: u64,
) -> Result<Option<ProcessId>, RuntimeError> {
    let binding = runtime.sleep_queue_binding_for_fd(owner, queue_fd)?;
    wake_one_sleep_queue_with_binding(runtime, owner, binding, channel)
}

pub(crate) fn wake_one_sleep_queue_with_binding(
    runtime: &mut KernelRuntime,
    _actor_owner: ProcessId,
    binding: QueueDescriptorTarget,
    channel: u64,
) -> Result<Option<ProcessId>, RuntimeError> {
    let queue = match binding {
        QueueDescriptorTarget::Sleep { queue, .. } => queue,
        QueueDescriptorTarget::Event { .. } => {
            return Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound));
        }
    };
    let waiter = sleep_queue_mut_by_binding(runtime, binding)?
        .waiters
        .wake_one(channel);
    if let Some(waiter) = waiter {
        runtime
            .sleep_results
            .insert(waiter.owner.raw(), waiter.result);
        record_wait_agent_decision(
            runtime,
            WaitAgentKind::SleepWakeAgent,
            waiter.owner,
            queue,
            channel,
            1,
            u64::from(waiter.wake_hint),
        );
        runtime.scheduler.wake(
            &mut runtime.processes,
            waiter.owner,
            scheduler_class_from_hint(waiter.wake_hint),
        )?;
        return Ok(Some(waiter.owner));
    }
    Ok(None)
}

pub(crate) fn wake_all_sleep_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
    channel: u64,
) -> Result<Vec<ProcessId>, RuntimeError> {
    let binding = runtime.sleep_queue_binding(owner, queue)?;
    wake_all_sleep_queue_with_binding(runtime, owner, binding, channel)
}

pub(crate) fn wake_all_sleep_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    channel: u64,
) -> Result<Vec<ProcessId>, RuntimeError> {
    let binding = runtime.sleep_queue_binding_for_fd(owner, queue_fd)?;
    wake_all_sleep_queue_with_binding(runtime, owner, binding, channel)
}

pub(crate) fn wake_all_sleep_queue_with_binding(
    runtime: &mut KernelRuntime,
    _actor_owner: ProcessId,
    binding: QueueDescriptorTarget,
    channel: u64,
) -> Result<Vec<ProcessId>, RuntimeError> {
    let queue = match binding {
        QueueDescriptorTarget::Sleep { queue, .. } => queue,
        QueueDescriptorTarget::Event { .. } => {
            return Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound));
        }
    };
    let woke = sleep_queue_mut_by_binding(runtime, binding)?
        .waiters
        .wake_all(channel);
    let mut pids = Vec::with_capacity(woke.len());
    for waiter in woke {
        runtime
            .sleep_results
            .insert(waiter.owner.raw(), waiter.result);
        record_wait_agent_decision(
            runtime,
            WaitAgentKind::SleepWakeAgent,
            waiter.owner,
            queue,
            channel,
            2,
            u64::from(waiter.wake_hint),
        );
        runtime.scheduler.wake(
            &mut runtime.processes,
            waiter.owner,
            scheduler_class_from_hint(waiter.wake_hint),
        )?;
        pids.push(waiter.owner);
    }
    Ok(pids)
}

pub(crate) fn cancel_sleep_queue_owner(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
    target: ProcessId,
) -> Result<Vec<ProcessId>, RuntimeError> {
    let binding = runtime.sleep_queue_binding(owner, queue)?;
    cancel_sleep_queue_owner_with_binding(runtime, owner, binding, target)
}

pub(crate) fn cancel_sleep_queue_owner_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
    target: ProcessId,
) -> Result<Vec<ProcessId>, RuntimeError> {
    let binding = runtime.sleep_queue_binding_for_fd(owner, queue_fd)?;
    cancel_sleep_queue_owner_with_binding(runtime, owner, binding, target)
}

pub(crate) fn cancel_sleep_queue_owner_with_binding(
    runtime: &mut KernelRuntime,
    _actor_owner: ProcessId,
    binding: QueueDescriptorTarget,
    target: ProcessId,
) -> Result<Vec<ProcessId>, RuntimeError> {
    let queue = match binding {
        QueueDescriptorTarget::Sleep { queue, .. } => queue,
        QueueDescriptorTarget::Event { .. } => {
            return Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound));
        }
    };
    let canceled = sleep_queue_mut_by_binding(runtime, binding)?
        .waiters
        .finish_owner(target, SleepWaitResult::Canceled);
    let mut pids = Vec::with_capacity(canceled.len());
    for waiter in canceled {
        runtime
            .sleep_results
            .insert(waiter.owner.raw(), waiter.result);
        record_wait_agent_decision(
            runtime,
            WaitAgentKind::SleepCancelAgent,
            waiter.owner,
            queue,
            waiter.channel,
            target.raw(),
            u64::from(waiter.wake_hint),
        );
        runtime.scheduler.wake(
            &mut runtime.processes,
            waiter.owner,
            scheduler_class_from_hint(waiter.wake_hint),
        )?;
        pids.push(waiter.owner);
    }
    Ok(pids)
}

pub(crate) fn requeue_sleep_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
    from_channel: u64,
    to_channel: u64,
    max_count: usize,
) -> Result<usize, RuntimeError> {
    let binding = runtime.sleep_queue_binding(owner, queue)?;
    requeue_sleep_queue_with_binding(runtime, owner, binding, from_channel, to_channel, max_count)
}

pub(crate) fn requeue_sleep_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    from_channel: u64,
    to_channel: u64,
    max_count: usize,
) -> Result<usize, RuntimeError> {
    let binding = runtime.sleep_queue_binding_for_fd(owner, fd)?;
    requeue_sleep_queue_with_binding(runtime, owner, binding, from_channel, to_channel, max_count)
}

pub(crate) fn requeue_sleep_queue_with_binding(
    runtime: &mut KernelRuntime,
    actor_owner: ProcessId,
    binding: QueueDescriptorTarget,
    from_channel: u64,
    to_channel: u64,
    max_count: usize,
) -> Result<usize, RuntimeError> {
    let queue = match binding {
        QueueDescriptorTarget::Sleep { queue, .. } => queue,
        QueueDescriptorTarget::Event { .. } => {
            return Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound));
        }
    };
    let moved = sleep_queue_mut_by_binding(runtime, binding)?
        .waiters
        .requeue(from_channel, to_channel, max_count);
    if moved != 0 {
        record_wait_agent_decision(
            runtime,
            WaitAgentKind::SleepRequeueAgent,
            actor_owner,
            queue,
            from_channel,
            to_channel,
            moved as u64,
        );
    }
    Ok(moved)
}
