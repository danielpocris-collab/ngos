use super::*;

impl KernelSyscallSurface {
    pub fn create_event_multiplexer(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        flavor: EventMultiplexerFlavor,
    ) -> Result<EventMultiplexerDescriptor, SyscallError> {
        let fd = match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::CreateEventQueueDescriptor {
                owner,
                mode: flavor.event_queue_mode(),
            },
        )? {
            SyscallResult::QueueDescriptorCreated(fd) => fd,
            other => panic!("unexpected syscall result: {other:?}"),
        };
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::FcntlDescriptor {
                owner,
                fd,
                cmd: FcntlCmd::SetFl { nonblock: true },
            },
        )? {
            SyscallResult::FcntlResult(FcntlResult::Updated(_))
            | SyscallResult::FcntlResult(FcntlResult::Flags(_)) => {}
            other => panic!("unexpected syscall result: {other:?}"),
        }
        Ok(EventMultiplexerDescriptor { fd, flavor })
    }

    pub fn destroy_event_multiplexer(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::DestroyEventQueueDescriptor { owner, fd: mux.fd },
        )? {
            SyscallResult::QueueDescriptorDestroyed(_) => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_ctl_fd(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        op: EventMultiplexerFdOp,
    ) -> Result<(), SyscallError> {
        let syscall = match op {
            EventMultiplexerFdOp::Add(watch) => Syscall::WatchEventDescriptor {
                owner,
                queue_fd: mux.fd,
                fd: watch.fd,
                token: watch.token,
                interest: watch.interest,
                behavior: watch.behavior,
            },
            EventMultiplexerFdOp::Modify(watch) => Syscall::ModifyWatchedEventDescriptor {
                owner,
                queue_fd: mux.fd,
                fd: watch.fd,
                token: watch.token,
                interest: watch.interest,
                behavior: watch.behavior,
            },
            EventMultiplexerFdOp::Remove { fd } => Syscall::RemoveWatchedEventDescriptor {
                owner,
                queue_fd: mux.fd,
                fd,
            },
        };
        match self.dispatch(SyscallContext::kernel(caller), syscall)? {
            SyscallResult::EventWatchRegistered
            | SyscallResult::EventWatchModified
            | SyscallResult::EventWatchRemoved => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_register_timer(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        watch: EventMultiplexerTimerWatch,
    ) -> Result<EventTimerId, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::RegisterEventQueueTimerDescriptor {
                owner,
                queue_fd: mux.fd,
                token: watch.token,
                delay_ticks: watch.delay_ticks,
                interval_ticks: watch.interval_ticks,
                events: watch.events,
            },
        )? {
            SyscallResult::EventQueueTimerRegistered(timer) => Ok(timer),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_remove_timer(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        timer: EventTimerId,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::RemoveEventQueueTimerDescriptor {
                owner,
                queue_fd: mux.fd,
                timer,
            },
        )? {
            SyscallResult::EventQueueTimerRemoved => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_watch_process(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        watch: EventMultiplexerProcessWatch,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::WatchProcessEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                target: watch.target,
                token: watch.token,
                interest: watch.interest,
                events: watch.events,
            },
        )? {
            SyscallResult::ProcessEventWatchRegistered => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_watch_signals(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        watch: EventMultiplexerSignalWatch,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::WatchSignalEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                target: watch.target,
                thread: watch.thread,
                signal_mask: watch.signal_mask,
                token: watch.token,
                events: watch.events,
            },
        )? {
            SyscallResult::SignalEventWatchRegistered => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_remove_signal_watch(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        target: ProcessId,
        thread: Option<ThreadId>,
        token: u64,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::RemoveSignalEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                target,
                thread,
                token,
            },
        )? {
            SyscallResult::SignalEventWatchRemoved => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_watch_memory_waits(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        watch: EventMultiplexerMemoryWatch,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::WatchMemoryWaitEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                domain: watch.domain,
                addr: watch.addr,
                token: watch.token,
                events: watch.events,
            },
        )? {
            SyscallResult::MemoryWaitEventWatchRegistered => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn event_multiplexer_watch_resource(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        resource: ResourceId,
        token: u64,
        interest: ResourceEventInterest,
        events: IoPollEvents,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::WatchResourceEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                resource,
                token,
                interest,
                events,
            },
        )? {
            SyscallResult::ResourceEventWatchRegistered => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_remove_memory_wait_watch(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        domain: MemoryWaitDomain,
        addr: u64,
        token: u64,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::RemoveMemoryWaitEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                domain,
                addr,
                token,
            },
        )? {
            SyscallResult::MemoryWaitEventWatchRemoved => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_remove_resource_watch(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        resource: ResourceId,
        token: u64,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::RemoveResourceEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                resource,
                token,
            },
        )? {
            SyscallResult::ResourceEventWatchRemoved => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn event_multiplexer_remove_process_watch(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
        target: ProcessId,
        token: u64,
    ) -> Result<(), SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::RemoveProcessEventsDescriptor {
                owner,
                queue_fd: mux.fd,
                target,
                token,
            },
        )? {
            SyscallResult::ProcessEventWatchRemoved => Ok(()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn wait_event_multiplexer(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
    ) -> Result<Vec<EventRecord>, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::WaitEventQueueDescriptor { owner, fd: mux.fd },
        )? {
            SyscallResult::EventQueueReady(events) => Ok(events),
            SyscallResult::ProcessBlocked(_) => Err(SyscallError::InvalidArgument),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn inspect_event_multiplexer(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        mux: EventMultiplexerDescriptor,
    ) -> Result<EventQueueInfo, SyscallError> {
        match self.dispatch(
            SyscallContext::kernel(caller),
            Syscall::InspectEventQueueDescriptor { owner, fd: mux.fd },
        )? {
            SyscallResult::EventQueueInspected(info) => Ok(info),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    pub fn poll_via_event_multiplexer(
        &mut self,
        caller: ProcessId,
        owner: ProcessId,
        requests: &[EventMultiplexerPollRequest],
    ) -> Result<Vec<EventRecord>, SyscallError> {
        let mux = self.create_event_multiplexer(caller, owner, EventMultiplexerFlavor::Poll)?;
        let mut result = Ok(Vec::new());
        for request in requests {
            if let Err(error) = self.event_multiplexer_ctl_fd(
                caller,
                owner,
                mux,
                EventMultiplexerFdOp::Add(EventMultiplexerFdWatch {
                    fd: request.fd,
                    token: request.token,
                    interest: request.interest,
                    behavior: request.behavior,
                }),
            ) {
                result = Err(error);
                break;
            }
        }
        if result.is_ok() {
            result = self.wait_event_multiplexer(caller, owner, mux);
        }
        let destroy_result = self.destroy_event_multiplexer(caller, owner, mux);
        match (result, destroy_result) {
            (Ok(events), Ok(())) => Ok(events),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub(crate) fn dispatch_eventing(
        &mut self,
        context: &SyscallContext,
        syscall: &Syscall,
    ) -> Result<Option<SyscallResult>, SyscallError> {
        let result = match syscall {
            Syscall::SendSignal(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.send_signal(
                    PendingSignalSender {
                        pid: context.caller,
                        tid: context.tid,
                    },
                    request.pid,
                    request.signal,
                )?;
                SyscallResult::SignalQueued
            }
            Syscall::SendQueuedSignal(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.send_signal_with_value(
                    PendingSignalSender {
                        pid: context.caller,
                        tid: context.tid,
                    },
                    request.pid,
                    request.signal,
                    Some(request.value),
                )?;
                SyscallResult::SignalQueued
            }
            Syscall::SendThreadSignal(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.send_thread_signal(
                    PendingSignalSender {
                        pid: context.caller,
                        tid: context.tid,
                    },
                    request.pid,
                    request.tid,
                    request.signal,
                )?;
                SyscallResult::SignalQueued
            }
            Syscall::SendQueuedThreadSignal(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.send_thread_signal_with_value(
                    PendingSignalSender {
                        pid: context.caller,
                        tid: context.tid,
                    },
                    request.pid,
                    request.tid,
                    request.signal,
                    Some(request.value),
                )?;
                SyscallResult::SignalQueued
            }
            Syscall::SetSignalMask(request) => {
                context.require(CapabilityRights::WRITE)?;
                let (old, new) =
                    self.runtime
                        .set_signal_mask(request.pid, request.how, request.mask)?;
                SyscallResult::SignalMaskUpdated { old, new }
            }
            Syscall::SetSignalDisposition(request) => {
                context.require(CapabilityRights::WRITE)?;
                let (old, new, old_mask, new_mask, old_restart, new_restart) =
                    self.runtime.set_signal_disposition(
                        request.pid,
                        request.signal,
                        request.disposition,
                        request.mask,
                        request.restart,
                    )?;
                SyscallResult::SignalDispositionUpdated {
                    old: SignalActionState {
                        disposition: old,
                        mask: old_mask,
                        restart: old_restart,
                    },
                    new: SignalActionState {
                        disposition: new,
                        mask: new_mask,
                        restart: new_restart,
                    },
                }
            }
            Syscall::TakePendingSignal(request) => {
                context.require(CapabilityRights::WRITE)?;
                SyscallResult::PendingSignalTaken(self.runtime.take_pending_signal(
                    request.pid,
                    request.mask,
                    request.blocked_only,
                )?)
            }
            Syscall::WaitForPendingSignal(request) => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::PendingSignalWaited(self.runtime.wait_for_pending_signal(
                    request.pid,
                    request.mask,
                    request.timeout_ticks,
                )?)
            }
            Syscall::CreateEventQueue { owner, mode }
            | Syscall::CreateEventQueueDescriptor { owner, mode } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::QueueDescriptorCreated(
                    self.runtime.create_event_queue_descriptor(*owner, *mode)?,
                )
            }
            Syscall::OpenEventQueueDescriptor { owner, queue } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::QueueDescriptorOpened(
                    self.runtime.open_event_queue_descriptor(*owner, *queue)?,
                )
            }
            Syscall::DestroyEventQueue { owner, queue } => {
                context.require(CapabilityRights::READ)?;
                let fd = self.runtime.open_event_queue_descriptor(*owner, *queue)?;
                self.runtime.destroy_event_queue_descriptor(*owner, fd)?;
                SyscallResult::QueueDescriptorDestroyed(fd)
            }
            Syscall::DestroyEventQueueDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.destroy_event_queue_descriptor(*owner, *fd)?;
                SyscallResult::QueueDescriptorDestroyed(*fd)
            }
            Syscall::CreateSleepQueue { owner } | Syscall::CreateSleepQueueDescriptor { owner } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::QueueDescriptorCreated(
                    self.runtime.create_sleep_queue_descriptor(*owner)?,
                )
            }
            Syscall::OpenSleepQueueDescriptor { owner, queue } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::QueueDescriptorOpened(
                    self.runtime.open_sleep_queue_descriptor(*owner, *queue)?,
                )
            }
            Syscall::DestroySleepQueue { owner, queue } => {
                context.require(CapabilityRights::READ)?;
                let fd = self.runtime.open_sleep_queue_descriptor(*owner, *queue)?;
                self.runtime.destroy_sleep_queue_descriptor(*owner, fd)?;
                SyscallResult::QueueDescriptorDestroyed(fd)
            }
            Syscall::DestroySleepQueueDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.destroy_sleep_queue_descriptor(*owner, *fd)?;
                SyscallResult::QueueDescriptorDestroyed(*fd)
            }
            Syscall::WatchEvent {
                owner,
                queue,
                fd,
                token,
                interest,
                behavior,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .watch_event(*owner, *queue, *fd, *token, *interest, *behavior)?;
                SyscallResult::EventWatchRegistered
            }
            Syscall::WatchEventDescriptor {
                owner,
                queue_fd,
                fd,
                token,
                interest,
                behavior,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .watch_event_descriptor(*owner, *queue_fd, *fd, *token, *interest, *behavior)?;
                SyscallResult::EventWatchRegistered
            }
            Syscall::RegisterEventQueueTimerDescriptor {
                owner,
                queue_fd,
                token,
                delay_ticks,
                interval_ticks,
                events,
            } => {
                context.require(CapabilityRights::READ)?;
                let timer = self.runtime.register_event_queue_timer_descriptor(
                    *owner,
                    *queue_fd,
                    *token,
                    *delay_ticks,
                    *interval_ticks,
                    *events,
                )?;
                SyscallResult::EventQueueTimerRegistered(timer)
            }
            Syscall::RemoveEventQueueTimerDescriptor {
                owner,
                queue_fd,
                timer,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .remove_event_queue_timer_descriptor(*owner, *queue_fd, *timer)?;
                SyscallResult::EventQueueTimerRemoved
            }
            Syscall::WatchProcessEventsDescriptor {
                owner,
                queue_fd,
                target,
                token,
                interest,
                events,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.watch_process_events_descriptor(
                    *owner, *queue_fd, *target, *token, *interest, *events,
                )?;
                SyscallResult::ProcessEventWatchRegistered
            }
            Syscall::RemoveProcessEventsDescriptor {
                owner,
                queue_fd,
                target,
                token,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .remove_process_events_descriptor(*owner, *queue_fd, *target, *token)?;
                SyscallResult::ProcessEventWatchRemoved
            }
            Syscall::WatchSignalEventsDescriptor {
                owner,
                queue_fd,
                target,
                thread,
                signal_mask,
                token,
                events,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.watch_signal_events_descriptor(
                    *owner,
                    *queue_fd,
                    *target,
                    *thread,
                    *signal_mask,
                    *token,
                    *events,
                )?;
                SyscallResult::SignalEventWatchRegistered
            }
            Syscall::RemoveSignalEventsDescriptor {
                owner,
                queue_fd,
                target,
                thread,
                token,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .remove_signal_events_descriptor(*owner, *queue_fd, *target, *thread, *token)?;
                SyscallResult::SignalEventWatchRemoved
            }
            Syscall::WatchMemoryWaitEventsDescriptor {
                owner,
                queue_fd,
                domain,
                addr,
                token,
                events,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.watch_memory_wait_events_descriptor(
                    *owner, *queue_fd, *domain, *addr, *token, *events,
                )?;
                SyscallResult::MemoryWaitEventWatchRegistered
            }
            Syscall::RemoveMemoryWaitEventsDescriptor {
                owner,
                queue_fd,
                domain,
                addr,
                token,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.remove_memory_wait_events_descriptor(
                    *owner, *queue_fd, *domain, *addr, *token,
                )?;
                SyscallResult::MemoryWaitEventWatchRemoved
            }
            Syscall::WatchResourceEventsDescriptor {
                owner,
                queue_fd,
                resource,
                token,
                interest,
                events,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.watch_resource_events_descriptor(
                    *owner, *queue_fd, *resource, *token, *interest, *events,
                )?;
                SyscallResult::ResourceEventWatchRegistered
            }
            Syscall::WatchNetworkEventsDescriptor {
                owner,
                queue_fd,
                interface_path,
                socket_path,
                token,
                interest,
                events,
            } => {
                context.require(CapabilityRights::READ)?;
                let interface_inode = self.runtime.stat_path(interface_path)?.inode;
                let socket_inode = socket_path
                    .as_deref()
                    .map(|path| self.runtime.stat_path(path).map(|status| status.inode))
                    .transpose()?;
                self.runtime.watch_network_events_descriptor(
                    *owner,
                    *queue_fd,
                    interface_inode,
                    socket_inode,
                    *token,
                    *interest,
                    *events,
                )?;
                SyscallResult::NetworkEventWatchRegistered
            }
            Syscall::RemoveResourceEventsDescriptor {
                owner,
                queue_fd,
                resource,
                token,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .remove_resource_events_descriptor(*owner, *queue_fd, *resource, *token)?;
                SyscallResult::ResourceEventWatchRemoved
            }
            Syscall::RemoveNetworkEventsDescriptor {
                owner,
                queue_fd,
                interface_path,
                socket_path,
                token,
            } => {
                context.require(CapabilityRights::READ)?;
                let interface_inode = self.runtime.stat_path(interface_path)?.inode;
                let socket_inode = socket_path
                    .as_deref()
                    .map(|path| self.runtime.stat_path(path).map(|status| status.inode))
                    .transpose()?;
                self.runtime.remove_network_events_descriptor(
                    *owner,
                    *queue_fd,
                    interface_inode,
                    socket_inode,
                    *token,
                )?;
                SyscallResult::NetworkEventWatchRemoved
            }
            Syscall::ModifyWatchedEvent {
                owner,
                queue,
                fd,
                token,
                interest,
                behavior,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .modify_watched_event(*owner, *queue, *fd, *token, *interest, *behavior)?;
                SyscallResult::EventWatchModified
            }
            Syscall::ModifyWatchedEventDescriptor {
                owner,
                queue_fd,
                fd,
                token,
                interest,
                behavior,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.modify_watched_event_descriptor(
                    *owner, *queue_fd, *fd, *token, *interest, *behavior,
                )?;
                SyscallResult::EventWatchModified
            }
            Syscall::RemoveWatchedEvent { owner, queue, fd } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.remove_watched_event(*owner, *queue, *fd)?;
                SyscallResult::EventWatchRemoved
            }
            Syscall::RemoveWatchedEventDescriptor {
                owner,
                queue_fd,
                fd,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime
                    .remove_watched_event_descriptor(*owner, *queue_fd, *fd)?;
                SyscallResult::EventWatchRemoved
            }
            Syscall::WaitEventQueue { owner, queue } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::EventQueueReady(self.runtime.wait_event_queue(*owner, *queue)?)
            }
            Syscall::WaitEventQueueDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                match self
                    .runtime
                    .wait_event_queue_descriptor(*owner, *fd, context.tid)?
                {
                    EventQueueWaitResult::Ready(events) => SyscallResult::EventQueueReady(events),
                    EventQueueWaitResult::Blocked(pid) => SyscallResult::ProcessBlocked(pid),
                }
            }
            Syscall::InspectEventQueue { owner, queue } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::EventQueueInspected(
                    self.runtime.inspect_event_queue(*owner, *queue)?,
                )
            }
            Syscall::InspectEventQueueDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::EventQueueInspected(
                    self.runtime.inspect_event_queue_descriptor(*owner, *fd)?,
                )
            }
            Syscall::SleepOnQueue {
                owner,
                queue,
                channel,
                priority,
                timeout_ticks,
            } => {
                context.require(CapabilityRights::READ)?;
                let pid = self.runtime.sleep_on_queue_thread(
                    *owner,
                    context.tid,
                    *queue,
                    *channel,
                    *priority,
                    *timeout_ticks,
                )?;
                SyscallResult::ProcessBlockedOnSleepQueue(pid)
            }
            Syscall::SleepOnQueueDescriptor {
                owner,
                fd,
                channel,
                priority,
                timeout_ticks,
            } => {
                context.require(CapabilityRights::READ)?;
                let pid = self.runtime.sleep_on_queue_descriptor(
                    *owner,
                    *fd,
                    *channel,
                    *priority,
                    *timeout_ticks,
                )?;
                SyscallResult::ProcessBlockedOnSleepQueue(pid)
            }
            Syscall::WakeOneSleepQueue {
                owner,
                queue,
                channel,
            } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueWakeResult(
                    self.runtime
                        .wake_one_sleep_queue(*owner, *queue, *channel)?
                        .into_iter()
                        .collect(),
                )
            }
            Syscall::WakeOneSleepQueueDescriptor { owner, fd, channel } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueWakeResult(
                    self.runtime
                        .wake_one_sleep_queue_descriptor(*owner, *fd, *channel)?
                        .into_iter()
                        .collect(),
                )
            }
            Syscall::WakeAllSleepQueue {
                owner,
                queue,
                channel,
            } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueWakeResult(
                    self.runtime
                        .wake_all_sleep_queue(*owner, *queue, *channel)?,
                )
            }
            Syscall::WakeAllSleepQueueDescriptor { owner, fd, channel } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueWakeResult(
                    self.runtime
                        .wake_all_sleep_queue_descriptor(*owner, *fd, *channel)?,
                )
            }
            Syscall::CancelSleepQueueOwner {
                owner,
                queue,
                target,
            } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueWakeResult(
                    self.runtime
                        .cancel_sleep_queue_owner(*owner, *queue, *target)?,
                )
            }
            Syscall::CancelSleepQueueOwnerDescriptor { owner, fd, target } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueWakeResult(
                    self.runtime
                        .cancel_sleep_queue_owner_descriptor(*owner, *fd, *target)?,
                )
            }
            Syscall::RequeueSleepQueue {
                owner,
                queue,
                from_channel,
                to_channel,
                max_count,
            } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueRequeueResult(self.runtime.requeue_sleep_queue(
                    *owner,
                    *queue,
                    *from_channel,
                    *to_channel,
                    *max_count,
                )?)
            }
            Syscall::RequeueSleepQueueDescriptor {
                owner,
                fd,
                from_channel,
                to_channel,
                max_count,
            } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueRequeueResult(
                    self.runtime.requeue_sleep_queue_descriptor(
                        *owner,
                        *fd,
                        *from_channel,
                        *to_channel,
                        *max_count,
                    )?,
                )
            }
            Syscall::InspectSleepQueue { owner, queue } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueInspected(
                    self.runtime.inspect_sleep_queue(*owner, *queue)?,
                )
            }
            Syscall::InspectSleepQueueDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepQueueInspected(
                    self.runtime.inspect_sleep_queue_descriptor(*owner, *fd)?,
                )
            }
            Syscall::InspectSleepResult { pid } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::SleepResultInspected(self.runtime.last_sleep_result(*pid))
            }
            Syscall::InspectPendingSignals { pid } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::PendingSignals(self.runtime.pending_signals(*pid)?)
            }
            Syscall::InspectThreadPendingSignals { pid, tid } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::PendingSignals(self.runtime.pending_thread_signals(*pid, *tid)?)
            }
            Syscall::InspectBlockedPendingSignals { pid } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::PendingSignals(self.runtime.blocked_pending_signals(*pid)?)
            }
            Syscall::InspectPendingSignalWait { pid } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::PendingSignalWaitInspected(
                    self.runtime.inspect_pending_signal_wait(*pid)?,
                )
            }
            Syscall::InspectSignalDisposition { pid, signal } => {
                context.require(CapabilityRights::READ)?;
                let disposition = self.runtime.signal_disposition(*pid, *signal)?;
                let mask = self.runtime.signal_action_mask(*pid, *signal)?;
                let restart = self.runtime.signal_action_restart(*pid, *signal)?;
                SyscallResult::SignalDispositionUpdated {
                    old: SignalActionState {
                        disposition,
                        mask,
                        restart,
                    },
                    new: SignalActionState {
                        disposition,
                        mask,
                        restart,
                    },
                }
            }
            Syscall::InspectSignalMask { pid } => {
                context.require(CapabilityRights::READ)?;
                let mask = self.runtime.signal_mask(*pid)?;
                SyscallResult::SignalMaskUpdated {
                    old: mask,
                    new: mask,
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
