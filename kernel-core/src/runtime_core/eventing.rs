use super::*;
use crate::eventing_model::GraphicsEventInterest;

impl KernelRuntime {
    pub fn create_sleep_queue(&mut self, owner: ProcessId) -> Result<SleepQueueId, RuntimeError> {
        sleep_queue_runtime::create_sleep_queue(self, owner)
    }

    pub fn create_sleep_queue_descriptor(
        &mut self,
        owner: ProcessId,
    ) -> Result<Descriptor, RuntimeError> {
        sleep_queue_runtime::create_sleep_queue_descriptor(self, owner)
    }

    pub fn open_sleep_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
    ) -> Result<Descriptor, RuntimeError> {
        sleep_queue_runtime::open_sleep_queue_descriptor(self, owner, queue)
    }

    pub fn destroy_sleep_queue(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
    ) -> Result<(), RuntimeError> {
        sleep_queue_runtime::destroy_sleep_queue(self, owner, queue)
    }

    pub fn destroy_sleep_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
    ) -> Result<(), RuntimeError> {
        sleep_queue_runtime::destroy_sleep_queue_descriptor(self, owner, queue_fd)
    }

    pub fn sleep_on_queue(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
        channel: u64,
        priority: u16,
        timeout_ticks: Option<u64>,
    ) -> Result<ProcessId, RuntimeError> {
        sleep_queue_runtime::sleep_on_queue(self, owner, queue, channel, priority, timeout_ticks)
    }

    pub fn sleep_on_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        channel: u64,
        priority: u16,
        timeout_ticks: Option<u64>,
    ) -> Result<ProcessId, RuntimeError> {
        sleep_queue_runtime::sleep_on_queue_descriptor(
            self,
            owner,
            queue_fd,
            channel,
            priority,
            timeout_ticks,
        )
    }

    pub fn sleep_on_queue_thread(
        &mut self,
        owner: ProcessId,
        tid: ThreadId,
        queue: SleepQueueId,
        channel: u64,
        priority: u16,
        timeout_ticks: Option<u64>,
    ) -> Result<ProcessId, RuntimeError> {
        sleep_queue_runtime::sleep_on_queue_thread(
            self,
            owner,
            tid,
            queue,
            channel,
            priority,
            timeout_ticks,
        )
    }

    pub fn wake_one_sleep_queue(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
        channel: u64,
    ) -> Result<Option<ProcessId>, RuntimeError> {
        sleep_queue_runtime::wake_one_sleep_queue(self, owner, queue, channel)
    }

    pub fn wake_one_sleep_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        channel: u64,
    ) -> Result<Option<ProcessId>, RuntimeError> {
        sleep_queue_runtime::wake_one_sleep_queue_descriptor(self, owner, queue_fd, channel)
    }

    pub fn wake_all_sleep_queue(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
        channel: u64,
    ) -> Result<Vec<ProcessId>, RuntimeError> {
        sleep_queue_runtime::wake_all_sleep_queue(self, owner, queue, channel)
    }

    pub fn wake_all_sleep_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        channel: u64,
    ) -> Result<Vec<ProcessId>, RuntimeError> {
        sleep_queue_runtime::wake_all_sleep_queue_descriptor(self, owner, queue_fd, channel)
    }

    pub fn cancel_sleep_queue_owner(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
        target: ProcessId,
    ) -> Result<Vec<ProcessId>, RuntimeError> {
        sleep_queue_runtime::cancel_sleep_queue_owner(self, owner, queue, target)
    }

    pub fn cancel_sleep_queue_owner_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
    ) -> Result<Vec<ProcessId>, RuntimeError> {
        sleep_queue_runtime::cancel_sleep_queue_owner_descriptor(self, owner, queue_fd, target)
    }

    pub fn requeue_sleep_queue(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
        from_channel: u64,
        to_channel: u64,
        max_count: usize,
    ) -> Result<usize, RuntimeError> {
        sleep_queue_runtime::requeue_sleep_queue(
            self,
            owner,
            queue,
            from_channel,
            to_channel,
            max_count,
        )
    }

    pub fn requeue_sleep_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        from_channel: u64,
        to_channel: u64,
        max_count: usize,
    ) -> Result<usize, RuntimeError> {
        sleep_queue_runtime::requeue_sleep_queue_descriptor(
            self,
            owner,
            queue_fd,
            from_channel,
            to_channel,
            max_count,
        )
    }

    pub(crate) fn memory_wait_channel(key: MemoryWaitKey) -> u64 {
        0x4d45_4d57_0000_0000u64 ^ key.namespace.rotate_left(17) ^ key.addr
    }

    pub(crate) fn resolve_memory_wait_domain(
        &self,
        domain: MemoryWaitDomain,
    ) -> Result<u64, RuntimeError> {
        memory_wait_runtime::resolve_memory_wait_domain(self, domain)
    }

    pub(crate) fn remove_memory_waiter(&mut self, pid: ProcessId) {
        memory_wait_runtime::remove_memory_waiter(self, pid)
    }

    pub fn wait_on_memory_word(
        &mut self,
        pid: ProcessId,
        namespace: u64,
        addr: u64,
        expected: u32,
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitResult, RuntimeError> {
        memory_wait_runtime::wait_on_memory_word(
            self,
            pid,
            namespace,
            addr,
            expected,
            timeout_ticks,
        )
    }

    pub fn wait_on_memory_word_in_domain(
        &mut self,
        pid: ProcessId,
        domain: MemoryWaitDomain,
        addr: u64,
        expected: u32,
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitResult, RuntimeError> {
        memory_wait_runtime::wait_on_memory_word_in_domain(
            self,
            pid,
            domain,
            addr,
            expected,
            timeout_ticks,
        )
    }

    pub fn set_memory_wait_resume_index(
        &mut self,
        pid: ProcessId,
        index: usize,
    ) -> Result<(), RuntimeError> {
        self.processes.get(pid)?;
        self.memory_wait_resume_indices.insert(pid.raw(), index);
        Ok(())
    }

    pub fn memory_wait_resume_index(&self, pid: ProcessId) -> Option<usize> {
        self.memory_wait_resume_indices.get(&pid.raw()).copied()
    }

    pub fn wait_on_any_memory_word(
        &mut self,
        pid: ProcessId,
        entries: &[MemoryWordWaitEntry],
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitAnyResult, RuntimeError> {
        let mut selected = None;
        for (index, entry) in entries.iter().copied().enumerate() {
            let observed = self.compare_memory_word(pid, entry.addr, entry.expected)?;
            if observed == entry.expected {
                selected = Some((index, entry));
                break;
            }
        }
        let Some((index, entry)) = selected else {
            return Ok(MemoryWordWaitAnyResult::ValueMismatch);
        };
        match self.wait_on_memory_word(
            pid,
            entry.namespace,
            entry.addr,
            entry.expected,
            timeout_ticks,
        )? {
            MemoryWordWaitResult::Blocked(pid) => {
                self.set_memory_wait_resume_index(pid, index)?;
                Ok(MemoryWordWaitAnyResult::Blocked { pid, index })
            }
            MemoryWordWaitResult::ValueMismatch { .. } => {
                Ok(MemoryWordWaitAnyResult::ValueMismatch)
            }
        }
    }

    pub fn wait_on_any_memory_word_in_domain(
        &mut self,
        pid: ProcessId,
        entries: &[MemoryWordWaitDomainEntry],
        timeout_ticks: Option<u64>,
    ) -> Result<MemoryWordWaitAnyResult, RuntimeError> {
        let entries = entries
            .iter()
            .copied()
            .map(|entry| {
                Ok(MemoryWordWaitEntry {
                    namespace: self.resolve_memory_wait_domain(entry.domain)?,
                    addr: entry.addr,
                    expected: entry.expected,
                })
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?;
        self.wait_on_any_memory_word(pid, &entries, timeout_ticks)
    }

    pub fn wake_memory_word_in_domain(
        &mut self,
        domain: MemoryWaitDomain,
        addr: u64,
        max_wake: usize,
    ) -> Result<Vec<ProcessId>, RuntimeError> {
        memory_wait_runtime::wake_memory_word_in_domain(self, domain, addr, max_wake)
    }

    pub(crate) fn memory_word_compare_matches(
        left: u32,
        cmp: MemoryWordCompareOp,
        right: u32,
    ) -> bool {
        match cmp {
            MemoryWordCompareOp::Eq => left == right,
            MemoryWordCompareOp::Ne => left != right,
            MemoryWordCompareOp::Lt => left < right,
            MemoryWordCompareOp::Le => left <= right,
            MemoryWordCompareOp::Gt => left > right,
            MemoryWordCompareOp::Ge => left >= right,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wake_memory_word_op(
        &mut self,
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
    ) -> Result<MemoryWordWakeOpResult, RuntimeError> {
        let (old_value, new_value) = self.update_memory_word(pid, to_addr, op)?;
        let comparison_matched = Self::memory_word_compare_matches(old_value, cmp, cmp_arg);
        let woke_from = memory_wait_runtime::wake_memory_word(
            self,
            from_namespace,
            from_addr,
            wake_from_count,
        )?;
        let woke_to = if comparison_matched {
            memory_wait_runtime::wake_memory_word(self, to_namespace, to_addr, wake_to_count)?
        } else {
            Vec::new()
        };
        Ok(MemoryWordWakeOpResult {
            old_value,
            new_value,
            comparison_matched,
            woke_from,
            woke_to,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wake_memory_word_op_in_domain(
        &mut self,
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
    ) -> Result<MemoryWordWakeOpResult, RuntimeError> {
        let from_namespace = memory_wait_runtime::resolve_memory_wait_domain(self, from_domain)?;
        let to_namespace = memory_wait_runtime::resolve_memory_wait_domain(self, to_domain)?;
        self.wake_memory_word_op(
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
    }

    pub fn wake_memory_word(
        &mut self,
        namespace: u64,
        addr: u64,
        max_wake: usize,
    ) -> Result<Vec<ProcessId>, RuntimeError> {
        memory_wait_runtime::wake_memory_word(self, namespace, addr, max_wake)
    }

    pub fn requeue_memory_word(
        &mut self,
        from_namespace: u64,
        from_addr: u64,
        to_namespace: u64,
        to_addr: u64,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordRequeueResult, RuntimeError> {
        memory_wait_runtime::requeue_memory_word(
            self,
            from_namespace,
            from_addr,
            to_namespace,
            to_addr,
            wake_count,
            requeue_count,
        )
    }

    pub fn requeue_memory_word_in_domain(
        &mut self,
        from_domain: MemoryWaitDomain,
        from_addr: u64,
        to_domain: MemoryWaitDomain,
        to_addr: u64,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordRequeueResult, RuntimeError> {
        let from_namespace = memory_wait_runtime::resolve_memory_wait_domain(self, from_domain)?;
        let to_namespace = memory_wait_runtime::resolve_memory_wait_domain(self, to_domain)?;
        self.requeue_memory_word(
            from_namespace,
            from_addr,
            to_namespace,
            to_addr,
            wake_count,
            requeue_count,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn cmp_requeue_memory_word(
        &mut self,
        pid: ProcessId,
        from_namespace: u64,
        from_addr: u64,
        to_namespace: u64,
        to_addr: u64,
        expected: u32,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordCmpRequeueResult, RuntimeError> {
        let observed = self.compare_memory_word(pid, from_addr, expected)?;
        if observed != expected {
            return Ok(MemoryWordCmpRequeueResult::ValueMismatch { expected, observed });
        }
        Ok(MemoryWordCmpRequeueResult::Requeued(
            memory_wait_runtime::requeue_memory_word(
                self,
                from_namespace,
                from_addr,
                to_namespace,
                to_addr,
                wake_count,
                requeue_count,
            )?,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn cmp_requeue_memory_word_in_domain(
        &mut self,
        pid: ProcessId,
        from_domain: MemoryWaitDomain,
        from_addr: u64,
        to_domain: MemoryWaitDomain,
        to_addr: u64,
        expected: u32,
        wake_count: usize,
        requeue_count: usize,
    ) -> Result<MemoryWordCmpRequeueResult, RuntimeError> {
        let from_namespace = memory_wait_runtime::resolve_memory_wait_domain(self, from_domain)?;
        let to_namespace = memory_wait_runtime::resolve_memory_wait_domain(self, to_domain)?;
        self.cmp_requeue_memory_word(
            pid,
            from_namespace,
            from_addr,
            to_namespace,
            to_addr,
            expected,
            wake_count,
            requeue_count,
        )
    }

    pub fn register_readiness(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        interest: ReadinessInterest,
    ) -> Result<(), RuntimeError> {
        descriptor_io_runtime::register_readiness(self, owner, fd, interest)
    }

    pub fn collect_ready(&self) -> Result<Vec<ReadinessRegistration>, RuntimeError> {
        descriptor_io_runtime::collect_ready(self)
    }

    pub fn create_event_queue(
        &mut self,
        owner: ProcessId,
        mode: EventQueueMode,
    ) -> Result<EventQueueId, RuntimeError> {
        event_queue_runtime::create_event_queue(self, owner, mode)
    }

    pub fn create_event_queue_descriptor(
        &mut self,
        owner: ProcessId,
        mode: EventQueueMode,
    ) -> Result<Descriptor, RuntimeError> {
        event_queue_runtime::create_event_queue_descriptor(self, owner, mode)
    }

    pub fn open_event_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
    ) -> Result<Descriptor, RuntimeError> {
        event_queue_runtime::open_event_queue_descriptor(self, owner, queue)
    }

    pub fn register_event_queue_timer_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        token: u64,
        delay_ticks: u64,
        interval_ticks: Option<u64>,
        events: IoPollEvents,
    ) -> Result<EventTimerId, RuntimeError> {
        event_queue_runtime::register_event_queue_timer_descriptor(
            self,
            owner,
            queue_fd,
            token,
            delay_ticks,
            interval_ticks,
            events,
        )
    }

    pub fn remove_event_queue_timer_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        timer: EventTimerId,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_event_queue_timer_descriptor(self, owner, queue_fd, timer)
    }

    pub fn watch_process_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        token: u64,
        interest: ProcessLifecycleInterest,
        events: IoPollEvents,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_process_events_descriptor(
            self, owner, queue_fd, target, token, interest, events,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn watch_signal_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        thread: Option<ThreadId>,
        signal_mask: u64,
        token: u64,
        events: IoPollEvents,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_signal_events_descriptor(
            self,
            owner,
            queue_fd,
            target,
            thread,
            signal_mask,
            token,
            events,
        )
    }

    pub fn remove_signal_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        thread: Option<ThreadId>,
        token: u64,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_signal_events_descriptor(
            self, owner, queue_fd, target, thread, token,
        )
    }

    pub fn watch_memory_wait_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        domain: MemoryWaitDomain,
        addr: u64,
        token: u64,
        events: IoPollEvents,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_memory_wait_events_descriptor(
            self, owner, queue_fd, domain, addr, token, events,
        )
    }

    pub fn watch_resource_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        resource: ResourceId,
        token: u64,
        interest: ResourceEventInterest,
        events: IoPollEvents,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_resource_events_descriptor(
            self, owner, queue_fd, resource, token, interest, events,
        )
    }

    pub fn remove_memory_wait_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        domain: MemoryWaitDomain,
        addr: u64,
        token: u64,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_memory_wait_events_descriptor(
            self, owner, queue_fd, domain, addr, token,
        )
    }

    pub fn remove_resource_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        resource: ResourceId,
        token: u64,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_resource_events_descriptor(
            self, owner, queue_fd, resource, token,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn watch_network_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        interface_inode: u64,
        socket_inode: Option<u64>,
        token: u64,
        interest: NetworkEventInterest,
        events: IoPollEvents,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_network_events_descriptor(
            self,
            owner,
            queue_fd,
            interface_inode,
            socket_inode,
            token,
            interest,
            events,
        )
    }

    pub fn remove_network_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        interface_inode: u64,
        socket_inode: Option<u64>,
        token: u64,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_network_events_descriptor(
            self,
            owner,
            queue_fd,
            interface_inode,
            socket_inode,
            token,
        )
    }

    pub fn watch_graphics_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        device_inode: u64,
        token: u64,
        interest: GraphicsEventInterest,
        events: IoPollEvents,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_graphics_events_descriptor(
            self,
            owner,
            queue_fd,
            device_inode,
            token,
            interest,
            events,
        )
    }

    pub fn remove_graphics_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        device_inode: u64,
        token: u64,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_graphics_events_descriptor(
            self,
            owner,
            queue_fd,
            device_inode,
            token,
        )
    }

    pub fn remove_process_events_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        target: ProcessId,
        token: u64,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_process_events_descriptor(self, owner, queue_fd, target, token)
    }

    pub fn destroy_event_queue(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::destroy_event_queue(self, owner, queue)
    }

    pub fn destroy_event_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::destroy_event_queue_descriptor(self, owner, queue_fd)
    }

    pub fn watch_event(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_event(self, owner, queue, fd, token, interest, behavior)
    }

    pub fn watch_event_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::watch_event_descriptor(
            self, owner, queue_fd, fd, token, interest, behavior,
        )
    }

    pub fn modify_watched_event(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::modify_watched_event(self, owner, queue, fd, token, interest, behavior)
    }

    pub fn modify_watched_event_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        fd: Descriptor,
        token: u64,
        interest: ReadinessInterest,
        behavior: EventWatchBehavior,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::modify_watched_event_descriptor(
            self, owner, queue_fd, fd, token, interest, behavior,
        )
    }

    pub fn remove_watched_event(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
        fd: Descriptor,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_watched_event(self, owner, queue, fd)
    }

    pub fn remove_watched_event_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        fd: Descriptor,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::remove_watched_event_descriptor(self, owner, queue_fd, fd)
    }

    pub fn wait_event_queue(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
    ) -> Result<Vec<EventRecord>, RuntimeError> {
        event_queue_runtime::wait_event_queue(self, owner, queue)
    }

    pub fn wait_event_queue_descriptor(
        &mut self,
        owner: ProcessId,
        queue_fd: Descriptor,
        tid: ThreadId,
    ) -> Result<EventQueueWaitResult, RuntimeError> {
        event_queue_runtime::wait_event_queue_descriptor(self, owner, queue_fd, tid)
    }

    pub(crate) fn tick_event_queue_timers(&mut self) -> Result<(), RuntimeError> {
        event_queue_runtime::tick_event_queue_timers(self)
    }

    pub(crate) fn emit_process_lifecycle_events(
        &mut self,
        target: ProcessId,
        kind: ProcessLifecycleEventKind,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::emit_process_lifecycle_events(self, target, kind)
    }

    pub(crate) fn drain_event_queue(
        &mut self,
        binding: QueueDescriptorTarget,
    ) -> Result<Vec<EventRecord>, RuntimeError> {
        let drained = self.event_queue_mut_by_binding(binding)?.drain_pending();
        self.sync_event_queue_readability(binding)?;
        Ok(drained)
    }

    pub(crate) fn sync_event_queue_readability(
        &mut self,
        binding: QueueDescriptorTarget,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::sync_event_queue_readability(self, binding)
    }

    pub(crate) fn notify_descriptor_ready(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::notify_descriptor_ready(self, owner, fd)
    }

    pub(crate) fn tick_sleep_queues(&mut self) -> Result<(), RuntimeError> {
        let mut woke = Vec::new();
        for queue in &mut self.sleep_queues {
            for waiter in queue.waiters.tick(self.current_tick) {
                self.sleep_results.insert(waiter.owner.raw(), waiter.result);
                if self.decision_tracing_enabled {
                    self.wait_agent_decisions.push(WaitAgentDecisionRecord {
                        tick: self.current_tick,
                        agent: WaitAgentKind::SleepWakeAgent,
                        owner: waiter.owner.raw(),
                        queue: queue.id.0,
                        channel: waiter.channel,
                        detail0: 3,
                        detail1: u64::from(waiter.wake_hint),
                    });
                    if self.wait_agent_decisions.len() > WAIT_AGENT_DECISION_LIMIT {
                        self.wait_agent_decisions.remove(0);
                    }
                }
                woke.push((waiter.owner, scheduler_class_from_hint(waiter.wake_hint)));
            }
        }
        for (owner, class) in woke {
            self.scheduler.wake(&mut self.processes, owner, class)?;
        }
        Ok(())
    }

    pub(crate) fn event_queue_mut_by_binding(
        &mut self,
        binding: QueueDescriptorTarget,
    ) -> Result<&mut EventQueue, RuntimeError> {
        event_queue_runtime::event_queue_mut_by_binding(self, binding)
    }
}
