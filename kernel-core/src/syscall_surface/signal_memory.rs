use super::*;

impl KernelSyscallSurface {
    pub fn wait_on_memory_word(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        namespace: u64,
        addr: u64,
        expected: u32,
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitResult, SyscallError> {
        let _ = caller;
        self.runtime
            .wait_on_memory_word(pid, namespace, addr, expected, timeout_ticks)
            .map_err(Into::into)
    }

    pub fn wait_on_any_memory_word(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        entries: &[MemoryWordWaitEntry],
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitAnyResult, SyscallError> {
        let _ = caller;
        self.runtime
            .wait_on_any_memory_word(pid, entries, timeout_ticks)
            .map_err(Into::into)
    }

    pub fn wait_on_any_memory_word_in_domain(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        entries: &[MemoryWordWaitDomainEntry],
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitAnyResult, SyscallError> {
        let _ = caller;
        self.runtime
            .wait_on_any_memory_word_in_domain(pid, entries, timeout_ticks)
            .map_err(Into::into)
    }

    pub fn wait_on_memory_word_in_domain(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        domain: MemoryWaitDomain,
        addr: u64,
        expected: u32,
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitResult, SyscallError> {
        let _ = caller;
        self.runtime
            .wait_on_memory_word_in_domain(pid, domain, addr, expected, timeout_ticks)
            .map_err(Into::into)
    }

    pub fn send_signal(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        signal: u8,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::SendSignal(SendSignal { pid, signal }),
        )? {
            SyscallResult::SignalQueued => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn send_queued_signal(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        signal: u8,
        value: u64,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::SendQueuedSignal(SendQueuedSignal { pid, signal, value }),
        )? {
            SyscallResult::SignalQueued => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn send_thread_signal(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        tid: ThreadId,
        signal: u8,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::SendThreadSignal(SendThreadSignal { pid, tid, signal }),
        )? {
            SyscallResult::SignalQueued => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn send_queued_thread_signal(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        tid: ThreadId,
        signal: u8,
        value: u64,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::SendQueuedThreadSignal(SendQueuedThreadSignal {
                pid,
                tid,
                signal,
                value,
            }),
        )? {
            SyscallResult::SignalQueued => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn pending_signals(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
    ) -> Result<Vec<u8>, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::InspectPendingSignals { pid },
        )? {
            SyscallResult::PendingSignals(signals) => Ok(signals),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn pending_thread_signals(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        tid: ThreadId,
    ) -> Result<Vec<u8>, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::InspectThreadPendingSignals { pid, tid },
        )? {
            SyscallResult::PendingSignals(signals) => Ok(signals),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn blocked_pending_signals(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
    ) -> Result<Vec<u8>, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::InspectBlockedPendingSignals { pid },
        )? {
            SyscallResult::PendingSignals(signals) => Ok(signals),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn wait_for_pending_signal(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        mask: u64,
        timeout_ticks: Option<u64>,
    ) -> Result<PendingSignalWaitResult, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::WaitForPendingSignal(WaitForPendingSignal {
                pid,
                mask,
                timeout_ticks,
            }),
        )? {
            SyscallResult::PendingSignalWaited(result) => Ok(result),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn inspect_pending_signal_wait(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
    ) -> Result<Option<PendingSignalWaitResume>, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::InspectPendingSignalWait { pid },
        )? {
            SyscallResult::PendingSignalWaitInspected(result) => Ok(result),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn signal_mask(&mut self, caller: ProcessId, pid: ProcessId) -> Result<u64, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::InspectSignalMask { pid },
        )? {
            SyscallResult::SignalMaskUpdated { new, .. } => Ok(new),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn signal_disposition(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        signal: u8,
    ) -> Result<Option<SignalDisposition>, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::InspectSignalDisposition { pid, signal },
        )? {
            SyscallResult::SignalDispositionUpdated { new, .. } => Ok(new.disposition),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn set_signal_mask(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        how: SignalMaskHow,
        mask: u64,
    ) -> Result<(u64, u64), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::SetSignalMask(SetSignalMask { pid, how, mask }),
        )? {
            SyscallResult::SignalMaskUpdated { old, new } => Ok((old, new)),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn set_signal_disposition(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        signal: u8,
        disposition: Option<SignalDisposition>,
        mask: u64,
        restart: bool,
    ) -> Result<(SignalActionState, SignalActionState), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::SetSignalDisposition(SetSignalDisposition {
                pid,
                signal,
                disposition,
                mask,
                restart,
            }),
        )? {
            SyscallResult::SignalDispositionUpdated { old, new } => Ok((old, new)),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn take_pending_signal(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        mask: u64,
        blocked_only: bool,
    ) -> Result<Option<u8>, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::TakePendingSignal(TakePendingSignal {
                pid,
                mask,
                blocked_only,
            }),
        )? {
            SyscallResult::PendingSignalTaken(signal) => Ok(signal),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wake_memory_word_op(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        from_namespace: u64,
        from_addr: u64,
        to_namespace: u64,
        to_addr: u64,
        wake_from_count: usize,
        wake_to_count: usize,
        op: MemoryWordUpdateOp,
        cmp: MemoryWordCompareOp,
        cmp_arg: u32,
    ) -> Result<MemoryWordWakeOpResult, SyscallError> {
        let _ = caller;
        self.runtime
            .wake_memory_word_op(
                pid,
                from_namespace,
                from_addr,
                to_namespace,
                to_addr,
                wake_from_count,
                wake_to_count,
                op,
                cmp,
                cmp_arg,
            )
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wake_memory_word_op_in_domain(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        from_domain: MemoryWaitDomain,
        from_addr: u64,
        to_domain: MemoryWaitDomain,
        to_addr: u64,
        wake_from_count: usize,
        wake_to_count: usize,
        op: MemoryWordUpdateOp,
        cmp: MemoryWordCompareOp,
        cmp_arg: u32,
    ) -> Result<MemoryWordWakeOpResult, SyscallError> {
        let _ = caller;
        self.runtime
            .wake_memory_word_op_in_domain(
                pid,
                from_domain,
                from_addr,
                to_domain,
                to_addr,
                wake_from_count,
                wake_to_count,
                op,
                cmp,
                cmp_arg,
            )
            .map_err(Into::into)
    }

    pub fn wake_memory_word(
        &mut self,
        caller: ProcessId,
        namespace: u64,
        addr: u64,
        max_wake: usize,
    ) -> Result<Vec<ProcessId>, SyscallError> {
        let _ = caller;
        self.runtime
            .wake_memory_word(namespace, addr, max_wake)
            .map_err(Into::into)
    }

    pub fn wake_memory_word_in_domain(
        &mut self,
        caller: ProcessId,
        domain: MemoryWaitDomain,
        addr: u64,
        max_wake: usize,
    ) -> Result<Vec<ProcessId>, SyscallError> {
        let _ = caller;
        self.runtime
            .wake_memory_word_in_domain(domain, addr, max_wake)
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn requeue_memory_word(
        &mut self,
        caller: ProcessId,
        from_namespace: u64,
        from_addr: u64,
        to_namespace: u64,
        to_addr: u64,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordRequeueResult, SyscallError> {
        let _ = caller;
        self.runtime
            .requeue_memory_word(
                from_namespace,
                from_addr,
                to_namespace,
                to_addr,
                wake_count,
                requeue_count,
            )
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn requeue_memory_word_in_domain(
        &mut self,
        caller: ProcessId,
        from_domain: MemoryWaitDomain,
        from_addr: u64,
        to_domain: MemoryWaitDomain,
        to_addr: u64,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordRequeueResult, SyscallError> {
        let _ = caller;
        self.runtime
            .requeue_memory_word_in_domain(
                from_domain,
                from_addr,
                to_domain,
                to_addr,
                wake_count,
                requeue_count,
            )
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn cmp_requeue_memory_word(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        from_namespace: u64,
        from_addr: u64,
        to_namespace: u64,
        to_addr: u64,
        expected: u32,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordCmpRequeueResult, SyscallError> {
        let _ = caller;
        self.runtime
            .cmp_requeue_memory_word(
                pid,
                from_namespace,
                from_addr,
                to_namespace,
                to_addr,
                expected,
                wake_count,
                requeue_count,
            )
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn cmp_requeue_memory_word_in_domain(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        from_domain: MemoryWaitDomain,
        from_addr: u64,
        to_domain: MemoryWaitDomain,
        to_addr: u64,
        expected: u32,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordCmpRequeueResult, SyscallError> {
        let _ = caller;
        self.runtime
            .cmp_requeue_memory_word_in_domain(
                pid,
                from_domain,
                from_addr,
                to_domain,
                to_addr,
                expected,
                wake_count,
                requeue_count,
            )
            .map_err(Into::into)
    }

    pub fn set_memory_wait_resume_index(
        &mut self,
        caller: ProcessId,
        pid: ProcessId,
        index: usize,
    ) -> Result<(), SyscallError> {
        let _ = caller;
        self.runtime
            .set_memory_wait_resume_index(pid, index)
            .map_err(Into::into)
    }

    pub fn memory_wait_resume_index(
        &self,
        caller: ProcessId,
        pid: ProcessId,
    ) -> Result<Option<usize>, SyscallError> {
        let _ = caller;
        self.runtime
            .processes
            .get(pid)
            .map_err(RuntimeError::from)?;
        Ok(self.runtime.memory_wait_resume_index(pid))
    }
}
