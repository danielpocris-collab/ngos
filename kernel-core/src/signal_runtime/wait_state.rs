use super::*;

pub(crate) fn pending_signals(
    runtime: &KernelRuntime,
    pid: ProcessId,
) -> Result<Vec<u8>, RuntimeError> {
    Ok(runtime.processes.get(pid)?.pending_signals())
}

pub(crate) fn pending_thread_signals(
    runtime: &KernelRuntime,
    pid: ProcessId,
    tid: ThreadId,
) -> Result<Vec<u8>, RuntimeError> {
    Ok(runtime.processes.get(pid)?.pending_thread_signals(tid)?)
}

pub(crate) fn take_waitable_signal(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    mask: u64,
    blocked_only: bool,
) -> Result<Option<PendingSignalDelivery>, RuntimeError> {
    let main_thread = runtime.processes.get(pid)?.main_thread();
    let process = runtime
        .processes
        .objects
        .get_mut(pid.handle())
        .map_err(ProcessError::from_object_error)?;
    if let Some(tid) = main_thread
        && let Some((signal, sender, value)) =
            process.take_thread_pending_signal_in_mask(tid, mask, blocked_only)?
    {
        return Ok(Some(PendingSignalDelivery {
            signal,
            code: PendingSignalCode::Tgkill,
            value,
            source: PendingSignalSource::Thread(tid),
            sender,
        }));
    }
    Ok(process
        .take_pending_signal_in_mask(mask, blocked_only)
        .map(|(signal, sender, value)| PendingSignalDelivery {
            signal,
            code: PendingSignalCode::Kill,
            value,
            source: PendingSignalSource::Process,
            sender,
        }))
}

pub(crate) fn signal_mask(runtime: &KernelRuntime, pid: ProcessId) -> Result<u64, RuntimeError> {
    Ok(runtime.processes.get(pid)?.signal_mask_raw())
}

pub(crate) fn blocked_pending_signals(
    runtime: &KernelRuntime,
    pid: ProcessId,
) -> Result<Vec<u8>, RuntimeError> {
    Ok(runtime.processes.get(pid)?.pending_blocked_signals())
}

pub(crate) fn wait_for_pending_signal(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    mask: u64,
    timeout_ticks: Option<u64>,
) -> Result<PendingSignalWaitResult, RuntimeError> {
    runtime.signal_wait_masks.remove(&pid.raw());
    if let Some(signal) = take_waitable_signal(runtime, pid, mask, true)? {
        let action_mask = runtime
            .processes
            .get(pid)?
            .signal_action_mask(signal.signal)?;
        if action_mask != 0 {
            let process = runtime
                .processes
                .objects
                .get_mut(pid.handle())
                .map_err(ProcessError::from_object_error)?;
            process.set_signal_mask_raw(process.signal_mask_raw() | action_mask);
        }
        return Ok(PendingSignalWaitResult::Delivered(signal));
    }
    if timeout_ticks == Some(0) {
        runtime
            .sleep_results
            .insert(pid.raw(), SleepWaitResult::TimedOut);
        return Ok(PendingSignalWaitResult::TimedOut);
    }
    let queue = signal_wait_queue(runtime, pid)?;
    runtime.signal_wait_masks.insert(pid.raw(), mask);
    sleep_queue_runtime::sleep_on_queue(
        runtime,
        pid,
        queue,
        SIGNAL_WAIT_CHANNEL,
        0,
        timeout_ticks,
    )?;
    Ok(PendingSignalWaitResult::Blocked(pid))
}

pub(crate) fn inspect_pending_signal_wait(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
) -> Result<Option<PendingSignalWaitResume>, RuntimeError> {
    match runtime.last_sleep_result(pid) {
        Some(SleepWaitResult::Woken) => {
            let mask = runtime.signal_wait_masks.remove(&pid.raw()).unwrap_or(0);
            Ok(take_waitable_signal(runtime, pid, mask, true)?
                .map(PendingSignalWaitResume::Delivered))
        }
        Some(SleepWaitResult::TimedOut) => {
            runtime.signal_wait_masks.remove(&pid.raw());
            Ok(Some(PendingSignalWaitResume::TimedOut))
        }
        Some(SleepWaitResult::Canceled) => {
            runtime.signal_wait_masks.remove(&pid.raw());
            Ok(Some(PendingSignalWaitResume::Canceled))
        }
        Some(SleepWaitResult::Restarted) => {
            runtime.signal_wait_masks.remove(&pid.raw());
            Ok(Some(PendingSignalWaitResume::Restarted))
        }
        _ => Ok(None),
    }
}

pub(crate) fn signal_disposition(
    runtime: &KernelRuntime,
    pid: ProcessId,
    signal: u8,
) -> Result<Option<SignalDisposition>, RuntimeError> {
    Ok(runtime.processes.get(pid)?.signal_disposition(signal)?)
}

pub(crate) fn signal_action_mask(
    runtime: &KernelRuntime,
    pid: ProcessId,
    signal: u8,
) -> Result<u64, RuntimeError> {
    Ok(runtime.processes.get(pid)?.signal_action_mask(signal)?)
}

pub(crate) fn signal_action_restart(
    runtime: &KernelRuntime,
    pid: ProcessId,
    signal: u8,
) -> Result<bool, RuntimeError> {
    Ok(runtime.processes.get(pid)?.signal_action_restart(signal)?)
}

#[allow(clippy::type_complexity)]
pub(crate) fn set_signal_disposition(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    signal: u8,
    disposition: Option<SignalDisposition>,
    mask: u64,
    restart: bool,
) -> Result<
    (
        Option<SignalDisposition>,
        Option<SignalDisposition>,
        u64,
        u64,
        bool,
        bool,
    ),
    RuntimeError,
> {
    let old = runtime.processes.get(pid)?.signal_disposition(signal)?;
    let old_mask = runtime.processes.get(pid)?.signal_action_mask(signal)?;
    let old_restart = runtime.processes.get(pid)?.signal_action_restart(signal)?;
    {
        let process = runtime
            .processes
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.set_signal_disposition(signal, disposition, mask, restart)?;
    }
    let new = runtime.processes.get(pid)?.signal_disposition(signal)?;
    let new_mask = runtime.processes.get(pid)?.signal_action_mask(signal)?;
    let new_restart = runtime.processes.get(pid)?.signal_action_restart(signal)?;
    Ok((old, new, old_mask, new_mask, old_restart, new_restart))
}

pub(crate) fn set_signal_mask(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    how: SignalMaskHow,
    mask: u64,
) -> Result<(u64, u64), RuntimeError> {
    let process = runtime
        .processes
        .objects
        .get_mut(pid.handle())
        .map_err(ProcessError::from_object_error)?;
    let old = process.signal_mask_raw();
    let new = match how {
        SignalMaskHow::Set => mask,
        SignalMaskHow::Block => old | mask,
        SignalMaskHow::Unblock => old & !mask,
    };
    process.set_signal_mask_raw(new);
    Ok((old, new))
}

pub(crate) fn take_pending_signal(
    runtime: &mut KernelRuntime,
    pid: ProcessId,
    mask: u64,
    blocked_only: bool,
) -> Result<Option<u8>, RuntimeError> {
    Ok(runtime
        .processes
        .objects
        .get_mut(pid.handle())
        .map_err(ProcessError::from_object_error)?
        .take_pending_signal_in_mask(mask, blocked_only)
        .map(|(signal, _sender, _value)| signal))
}
