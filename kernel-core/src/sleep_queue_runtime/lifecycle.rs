use super::*;

pub(crate) fn create_sleep_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
) -> Result<SleepQueueId, RuntimeError> {
    runtime.processes.get(owner)?;
    let id = SleepQueueId(runtime.next_sleep_queue_id);
    runtime.next_sleep_queue_id = runtime.next_sleep_queue_id.saturating_add(1);
    runtime.sleep_queues.push(RuntimeSleepQueue {
        id,
        owner,
        waiters: SleepQueue::with_limit(256),
    });
    Ok(id)
}

pub(crate) fn create_sleep_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
) -> Result<Descriptor, RuntimeError> {
    let queue = create_sleep_queue(runtime, owner)?;
    open_sleep_queue_descriptor(runtime, owner, queue)
}

pub(crate) fn open_sleep_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
) -> Result<Descriptor, RuntimeError> {
    runtime.processes.get(owner)?;
    runtime.sleep_queue_exists(owner, queue)?;
    descriptor_runtime::open_runtime_queue_descriptor(
        runtime,
        owner,
        ObjectKind::SleepQueue,
        QueueDescriptorTarget::Sleep { owner, queue },
        sleep_queue_descriptor_name(owner, queue),
    )
}

pub(crate) fn destroy_sleep_queue(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue: SleepQueueId,
) -> Result<(), RuntimeError> {
    let fd = open_sleep_queue_descriptor(runtime, owner, queue)?;
    destroy_sleep_queue_descriptor(runtime, owner, fd)
}

pub(crate) fn destroy_sleep_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    queue_fd: Descriptor,
) -> Result<(), RuntimeError> {
    let _binding = runtime.sleep_queue_binding_for_fd(owner, queue_fd)?;
    let _ = descriptor_runtime::close_descriptor(runtime, owner, queue_fd)?;
    Ok(())
}
