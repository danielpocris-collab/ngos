use super::*;

pub(crate) fn signal_wait_queue(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
) -> Result<SleepQueueId, RuntimeError> {
    if let Some(queue) = runtime.signal_wait_queues.get(&pid.raw()).copied() {
        return Ok(queue);
    }
    let queue = sleep_queue_runtime::create_sleep_queue(runtime, pid)?;
    runtime.signal_wait_queues.insert(pid.raw(), queue);
    Ok(queue)
}

pub(crate) fn send_signal(
    runtime: &mut KernelRuntime,
    sender: PendingSignalSender,
    pid: ProcessId,
    signal: u8,
) -> Result<(), RuntimeError> {
    send_signal_with_value(runtime, sender, pid, signal, None)
}

pub(crate) fn send_signal_with_value(
    runtime: &mut KernelRuntime,
    sender: PendingSignalSender,
    pid: ProcessId,
    signal: u8,
    value: Option<u64>,
) -> Result<(), RuntimeError> {
    let (blocked, disposition, restart) = {
        let process = runtime.processes.get(pid)?;
        (
            process.signal_blocked(signal)?,
            process.signal_disposition(signal)?,
            process.signal_action_restart(signal)?,
        )
    };
    if !blocked {
        match disposition.unwrap_or(default_signal_disposition(signal)?) {
            SignalDisposition::Catch => {}
            SignalDisposition::Ignore => return Ok(()),
            SignalDisposition::Terminate => {
                terminate_process(runtime, pid, signal)?;
                return Ok(());
            }
        }
    }
    runtime
        .processes
        .objects
        .get_mut(pid.handle())
        .map_err(ProcessError::from_object_error)?
        .queue_signal_with_value(signal, sender, value)?;
    event_queue_runtime::emit_signal_events(runtime, pid, None, signal)?;
    if blocked {
        let bit = 1u64 << (signal - 1);
        if runtime
            .signal_wait_masks
            .get(&pid.raw())
            .is_some_and(|mask| mask & bit != 0)
        {
            let queue = signal_wait_queue(runtime, pid)?;
            let _ = sleep_queue_runtime::wake_one_sleep_queue(
                runtime,
                pid,
                queue,
                SIGNAL_WAIT_CHANNEL,
            )?;
        }
        return Ok(());
    }
    for queue in &mut runtime.sleep_queues {
        let canceled = queue.waiters.finish_owner(
            pid,
            if restart {
                SleepWaitResult::Restarted
            } else {
                SleepWaitResult::Canceled
            },
        );
        for waiter in canceled {
            runtime
                .sleep_results
                .insert(waiter.owner.raw(), waiter.result);
            runtime.scheduler.wake(
                &mut runtime.processes,
                waiter.owner,
                scheduler_class_from_hint(waiter.wake_hint),
            )?;
        }
    }
    memory_wait_runtime::remove_memory_waiter(runtime, pid);
    Ok(())
}

pub(crate) fn send_thread_signal(
    runtime: &mut KernelRuntime,
    sender: PendingSignalSender,
    pid: ProcessId,
    tid: ThreadId,
    signal: u8,
) -> Result<(), RuntimeError> {
    send_thread_signal_with_value(runtime, sender, pid, tid, signal, None)
}

pub(crate) fn send_thread_signal_with_value(
    runtime: &mut KernelRuntime,
    sender: PendingSignalSender,
    pid: ProcessId,
    tid: ThreadId,
    signal: u8,
    value: Option<u64>,
) -> Result<(), RuntimeError> {
    let (blocked, disposition, restart) = {
        let process = runtime.processes.get(pid)?;
        (
            process.signal_blocked(signal)?,
            process.signal_disposition(signal)?,
            process.signal_action_restart(signal)?,
        )
    };
    if !blocked {
        match disposition.unwrap_or(default_signal_disposition(signal)?) {
            SignalDisposition::Catch => {}
            SignalDisposition::Ignore => return Ok(()),
            SignalDisposition::Terminate => {
                terminate_process(runtime, pid, signal)?;
                return Ok(());
            }
        }
    }
    runtime
        .processes
        .objects
        .get_mut(pid.handle())
        .map_err(ProcessError::from_object_error)?
        .queue_thread_signal_with_value(tid, signal, sender, value)?;
    event_queue_runtime::emit_signal_events(runtime, pid, Some(tid), signal)?;
    if blocked {
        return Ok(());
    }
    for queue in &mut runtime.sleep_queues {
        let canceled = queue.waiters.finish_owner(
            pid,
            if restart {
                SleepWaitResult::Restarted
            } else {
                SleepWaitResult::Canceled
            },
        );
        for waiter in canceled {
            runtime
                .sleep_results
                .insert(waiter.owner.raw(), waiter.result);
            runtime.scheduler.wake(
                &mut runtime.processes,
                waiter.owner,
                scheduler_class_from_hint(waiter.wake_hint),
            )?;
        }
    }
    memory_wait_runtime::remove_memory_waiter(runtime, pid);
    Ok(())
}

pub(crate) fn terminate_process(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    signal: u8,
) -> Result<(), RuntimeError> {
    let exit_code = 128 + i32::from(signal);
    runtime.signal_wait_masks.remove(&pid.raw());
    runtime.memory_wait_queues.remove(&pid.raw());
    runtime.exit(pid, exit_code)?;
    Ok(())
}
