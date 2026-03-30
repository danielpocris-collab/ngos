use super::*;

pub(crate) fn resolve_memory_wait_domain(
    runtime: &KernelRuntime,
    domain: MemoryWaitDomain,
) -> Result<u64, RuntimeError> {
    Ok(match domain {
        MemoryWaitDomain::Shared => 0,
        MemoryWaitDomain::Process(pid) => {
            runtime.processes.get(pid)?;
            pid.raw()
        }
    })
}

pub(crate) fn ensure_memory_wait_queue(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
) -> Result<SleepQueueId, RuntimeError> {
    if let Some(queue) = runtime.memory_wait_queues.get(&pid.raw()).copied() {
        return Ok(queue);
    }
    let queue = sleep_queue_runtime::create_sleep_queue(runtime, pid)?;
    runtime.memory_wait_queues.insert(pid.raw(), queue);
    Ok(queue)
}

pub(crate) fn remove_memory_waiter(runtime: &mut KernelRuntime, pid: ProcessId) {
    runtime.memory_wait_resume_indices.remove(&pid.raw());
    runtime.memory_waiters.retain(|_, waiters| {
        waiters.retain(|waiter| waiter.pid != pid);
        !waiters.is_empty()
    });
}

pub(crate) fn prune_memory_waiters(runtime: &mut KernelRuntime, key: MemoryWaitKey) {
    let mut retained = Vec::new();
    if let Some(waiters) = runtime.memory_waiters.remove(&key) {
        for waiter in waiters {
            let state = runtime
                .processes
                .get(waiter.pid)
                .map(|process| process.state())
                .ok();
            if state == Some(ProcessState::Blocked) {
                retained.push(waiter);
            }
        }
    }
    if !retained.is_empty() {
        runtime.memory_waiters.insert(key, retained);
    }
}
