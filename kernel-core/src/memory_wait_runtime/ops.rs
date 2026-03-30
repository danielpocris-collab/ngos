use super::*;

pub(crate) fn wait_on_memory_word(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    namespace: u64,
    addr: u64,
    expected: u32,
    timeout_ticks: Option<u64>,
) -> Result<MemoryWordWaitResult, RuntimeError> {
    let observed = runtime.compare_memory_word(pid, addr, expected)?;
    if observed != expected {
        return Ok(MemoryWordWaitResult::ValueMismatch { expected, observed });
    }
    let queue = ensure_memory_wait_queue(runtime, pid)?;
    let key = MemoryWaitKey { namespace, addr };
    remove_memory_waiter(runtime, pid);
    let blocked = sleep_queue_runtime::sleep_on_queue(
        runtime,
        pid,
        queue,
        KernelRuntime::memory_wait_channel(key),
        0,
        timeout_ticks,
    )?;
    runtime
        .memory_waiters
        .entry(key)
        .or_default()
        .push(MemoryWaiter {
            pid: blocked,
            queue,
        });
    sleep_queue_runtime::record_wait_agent_decision(
        runtime,
        WaitAgentKind::MemoryWaitAgent,
        blocked,
        queue,
        KernelRuntime::memory_wait_channel(key),
        u64::from(expected),
        timeout_ticks.unwrap_or(0),
    );
    Ok(MemoryWordWaitResult::Blocked(blocked))
}

pub(crate) fn wait_on_memory_word_in_domain(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    domain: MemoryWaitDomain,
    addr: u64,
    expected: u32,
    timeout_ticks: Option<u64>,
) -> Result<MemoryWordWaitResult, RuntimeError> {
    let namespace = resolve_memory_wait_domain(runtime, domain)?;
    wait_on_memory_word(runtime, pid, namespace, addr, expected, timeout_ticks)
}

pub(crate) fn wake_memory_word(
    runtime: &mut KernelRuntime,
    namespace: u64,
    addr: u64,
    max_wake: usize,
) -> Result<Vec<ProcessId>, RuntimeError> {
    if max_wake == 0 {
        return Ok(Vec::new());
    }
    let key = MemoryWaitKey { namespace, addr };
    prune_memory_waiters(runtime, key);
    let channel = KernelRuntime::memory_wait_channel(key);
    let mut woke = Vec::new();
    let mut retained = Vec::new();
    if let Some(waiters) = runtime.memory_waiters.remove(&key) {
        for waiter in waiters {
            if woke.len() >= max_wake {
                retained.push(waiter);
                continue;
            }
            if let Some(pid) = sleep_queue_runtime::wake_one_sleep_queue(
                runtime,
                waiter.pid,
                waiter.queue,
                channel,
            )? {
                woke.push(pid);
            }
        }
    }
    if !retained.is_empty() {
        runtime.memory_waiters.insert(key, retained);
    }
    if !woke.is_empty() {
        for pid in &woke {
            if let Some(queue) = runtime.memory_wait_queues.get(&pid.raw()).copied() {
                sleep_queue_runtime::record_wait_agent_decision(
                    runtime,
                    WaitAgentKind::MemoryWaitAgent,
                    *pid,
                    queue,
                    channel,
                    1,
                    max_wake as u64,
                );
            }
        }
        event_queue_runtime::emit_memory_wait_events(
            runtime,
            namespace,
            addr,
            MemoryWaitEventKind::Woken,
        )?;
    }
    Ok(woke)
}

pub(crate) fn wake_memory_word_in_domain(
    runtime: &mut KernelRuntime,
    domain: MemoryWaitDomain,
    addr: u64,
    max_wake: usize,
) -> Result<Vec<ProcessId>, RuntimeError> {
    let namespace = resolve_memory_wait_domain(runtime, domain)?;
    wake_memory_word(runtime, namespace, addr, max_wake)
}

pub(crate) fn requeue_memory_word(
    runtime: &mut KernelRuntime,
    from_namespace: u64,
    from_addr: u64,
    to_namespace: u64,
    to_addr: u64,
    wake_count: usize,
    requeue_count: usize,
) -> Result<MemoryWordRequeueResult, RuntimeError> {
    let from_key = MemoryWaitKey {
        namespace: from_namespace,
        addr: from_addr,
    };
    let to_key = MemoryWaitKey {
        namespace: to_namespace,
        addr: to_addr,
    };
    prune_memory_waiters(runtime, from_key);
    let from_channel = KernelRuntime::memory_wait_channel(from_key);
    let to_channel = KernelRuntime::memory_wait_channel(to_key);
    let mut woke = Vec::new();
    let mut moved = 0usize;
    let mut retained = Vec::new();
    let mut requeued = Vec::new();

    if let Some(waiters) = runtime.memory_waiters.remove(&from_key) {
        for waiter in waiters {
            if woke.len() < wake_count
                && let Some(pid) = sleep_queue_runtime::wake_one_sleep_queue(
                    runtime,
                    waiter.pid,
                    waiter.queue,
                    from_channel,
                )?
            {
                woke.push(pid);
                continue;
            }
            if moved < requeue_count {
                let count = sleep_queue_runtime::requeue_sleep_queue(
                    runtime,
                    waiter.pid,
                    waiter.queue,
                    from_channel,
                    to_channel,
                    1,
                )?;
                if count == 1 {
                    moved += 1;
                    requeued.push(waiter);
                    continue;
                }
            }
            retained.push(waiter);
        }
    }

    if !retained.is_empty() {
        runtime.memory_waiters.insert(from_key, retained);
    }
    let requeued_for_log = requeued.clone();
    if !requeued.is_empty() {
        runtime
            .memory_waiters
            .entry(to_key)
            .or_default()
            .extend(requeued);
    }
    if !woke.is_empty() {
        for pid in &woke {
            if let Some(queue) = runtime.memory_wait_queues.get(&pid.raw()).copied() {
                sleep_queue_runtime::record_wait_agent_decision(
                    runtime,
                    WaitAgentKind::MemoryWaitAgent,
                    *pid,
                    queue,
                    from_channel,
                    2,
                    moved as u64,
                );
            }
        }
        event_queue_runtime::emit_memory_wait_events(
            runtime,
            from_namespace,
            from_addr,
            MemoryWaitEventKind::Woken,
        )?;
    }
    if moved != 0 {
        for waiter in requeued_for_log {
            sleep_queue_runtime::record_wait_agent_decision(
                runtime,
                WaitAgentKind::MemoryWaitAgent,
                waiter.pid,
                waiter.queue,
                to_channel,
                3,
                moved as u64,
            );
        }
        event_queue_runtime::emit_memory_wait_events(
            runtime,
            to_namespace,
            to_addr,
            MemoryWaitEventKind::Requeued,
        )?;
    }
    Ok(MemoryWordRequeueResult { woke, moved })
}
