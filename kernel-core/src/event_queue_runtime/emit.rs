use super::*;
use crate::eventing_model::GraphicsEventKind;

pub(crate) fn tick_event_queue_timers(runtime: &mut KernelRuntime) -> Result<(), RuntimeError> {
    let mut produced = Vec::new();
    for queue in &mut runtime.event_queues {
        let binding = QueueDescriptorTarget::Event {
            owner: queue.owner,
            queue: queue.id,
            mode: queue.mode,
        };
        for timer in queue.tick_timers(runtime.current_tick) {
            produced.push((
                binding,
                KernelEvent {
                    owner: queue.owner,
                    token: timer.token,
                    events: timer.events,
                    source: EventSource::Timer(timer.id),
                },
            ));
        }
    }
    for (binding, event) in produced {
        enqueue_event(runtime, binding, event)?;
    }
    Ok(())
}

pub(crate) fn emit_process_lifecycle_events(
    runtime: &mut KernelRuntime,
    target: ProcessId,
    kind: ProcessLifecycleEventKind,
) -> Result<(), RuntimeError> {
    let mut produced = Vec::new();
    for queue in &runtime.event_queues {
        let binding = QueueDescriptorTarget::Event {
            owner: queue.owner,
            queue: queue.id,
            mode: queue.mode,
        };
        for watch in &queue.process_watchers {
            let interested = match kind {
                ProcessLifecycleEventKind::Exited => watch.interest.exited,
                ProcessLifecycleEventKind::Reaped => watch.interest.reaped,
            };
            if watch.target == target && interested {
                produced.push((
                    binding,
                    KernelEvent {
                        owner: target,
                        token: watch.token,
                        events: watch.events,
                        source: EventSource::Process { pid: target, kind },
                    },
                ));
            }
        }
    }
    for (binding, event) in produced {
        enqueue_event(runtime, binding, event)?;
    }
    Ok(())
}

pub(crate) fn emit_signal_events(
    runtime: &mut KernelRuntime,
    target: ProcessId,
    tid: Option<ThreadId>,
    signal: u8,
) -> Result<(), RuntimeError> {
    let bit = 1u64 << (signal - 1);
    let mut produced = Vec::new();
    for queue in &runtime.event_queues {
        let binding = QueueDescriptorTarget::Event {
            owner: queue.owner,
            queue: queue.id,
            mode: queue.mode,
        };
        for watch in &queue.signal_watchers {
            if watch.target != target || watch.signal_mask & bit == 0 {
                continue;
            }
            if let Some(watch_tid) = watch.thread
                && tid != Some(watch_tid)
            {
                continue;
            }
            produced.push((
                binding,
                KernelEvent {
                    owner: target,
                    token: watch.token,
                    events: watch.events,
                    source: EventSource::Signal {
                        pid: target,
                        tid,
                        signal,
                    },
                },
            ));
        }
    }
    for (binding, event) in produced {
        enqueue_event(runtime, binding, event)?;
    }
    Ok(())
}

pub(crate) fn emit_memory_wait_events(
    runtime: &mut KernelRuntime,
    namespace: u64,
    addr: u64,
    kind: MemoryWaitEventKind,
) -> Result<(), RuntimeError> {
    let mut produced = Vec::new();
    for queue in &runtime.event_queues {
        let binding = QueueDescriptorTarget::Event {
            owner: queue.owner,
            queue: queue.id,
            mode: queue.mode,
        };
        for watch in &queue.memory_watchers {
            if watch.addr != addr
                || memory_wait_runtime::resolve_memory_wait_domain(runtime, watch.domain)?
                    != namespace
            {
                continue;
            }
            let event_owner = match watch.domain {
                MemoryWaitDomain::Shared => queue.owner,
                MemoryWaitDomain::Process(pid) => pid,
            };
            produced.push((
                binding,
                KernelEvent {
                    owner: event_owner,
                    token: watch.token,
                    events: watch.events,
                    source: EventSource::MemoryWait {
                        domain: watch.domain,
                        addr,
                        kind,
                    },
                },
            ));
        }
    }
    for (binding, event) in produced {
        enqueue_event(runtime, binding, event)?;
    }
    Ok(())
}

pub(crate) fn emit_resource_events(
    runtime: &mut KernelRuntime,
    resource: ResourceId,
    contract: ContractId,
    kind: ResourceEventKind,
) -> Result<(), RuntimeError> {
    let contract_info = runtime.contract_info(contract)?;
    let resource_info = runtime.resource_info(resource)?;
    let mut produced = Vec::new();
    for queue in &runtime.event_queues {
        let binding = QueueDescriptorTarget::Event {
            owner: queue.owner,
            queue: queue.id,
            mode: queue.mode,
        };
        for watch in &queue.resource_watchers {
            if watch.resource != resource {
                continue;
            }
            let interested = match kind {
                ResourceEventKind::Claimed => watch.interest.claimed,
                ResourceEventKind::Queued => watch.interest.queued,
                ResourceEventKind::Canceled => watch.interest.canceled,
                ResourceEventKind::Released => watch.interest.released,
                ResourceEventKind::HandedOff => watch.interest.handed_off,
                ResourceEventKind::Revoked => watch.interest.revoked,
            };
            if !interested {
                continue;
            }
            let event_owner = match kind {
                ResourceEventKind::Claimed | ResourceEventKind::HandedOff => contract_info.issuer,
                ResourceEventKind::Queued | ResourceEventKind::Canceled => contract_info.issuer,
                ResourceEventKind::Released => resource_info.creator,
                ResourceEventKind::Revoked => contract_info.issuer,
            };
            produced.push((
                binding,
                KernelEvent {
                    owner: event_owner,
                    token: watch.token,
                    events: watch.events,
                    source: EventSource::Resource {
                        resource,
                        contract,
                        kind,
                    },
                },
            ));
        }
    }
    for (binding, event) in produced {
        enqueue_event(runtime, binding, event)?;
    }
    Ok(())
}

pub(crate) fn emit_network_events(
    runtime: &mut KernelRuntime,
    interface_inode: u64,
    socket_inode: Option<u64>,
    kind: NetworkEventKind,
) -> Result<(), RuntimeError> {
    let mut produced = Vec::new();
    for queue in &runtime.event_queues {
        let binding = QueueDescriptorTarget::Event {
            owner: queue.owner,
            queue: queue.id,
            mode: queue.mode,
        };
        for watch in &queue.network_watchers {
            if watch.interface_inode != interface_inode {
                continue;
            }
            if let Some(target_socket) = watch.socket_inode
                && let Some(source_socket) = socket_inode
                && source_socket != target_socket
            {
                continue;
            }
            let interested = match kind {
                NetworkEventKind::LinkChanged => watch.interest.link_changed,
                NetworkEventKind::RxReady => watch.interest.rx_ready,
                NetworkEventKind::TxDrained => watch.interest.tx_drained,
            };
            if !interested {
                continue;
            }
            produced.push((
                binding,
                KernelEvent {
                    owner: queue.owner,
                    token: watch.token,
                    events: watch.events,
                    source: EventSource::Network {
                        interface_inode,
                        socket_inode,
                        kind,
                    },
                },
            ));
        }
    }
    for (binding, event) in produced {
        enqueue_event(runtime, binding, event)?;
    }
    Ok(())
}

pub(crate) fn emit_graphics_events(
    runtime: &mut KernelRuntime,
    device_inode: u64,
    request_id: u64,
    kind: GraphicsEventKind,
) -> Result<(), RuntimeError> {
    let mut produced = Vec::new();
    for queue in &runtime.event_queues {
        let binding = QueueDescriptorTarget::Event {
            owner: queue.owner,
            queue: queue.id,
            mode: queue.mode,
        };
        for watch in &queue.graphics_watchers {
            if watch.device_inode != device_inode {
                continue;
            }
            let interested = match kind {
                GraphicsEventKind::Submitted => watch.interest.submitted,
                GraphicsEventKind::Completed => watch.interest.completed,
                GraphicsEventKind::Failed => watch.interest.failed,
                GraphicsEventKind::Drained => watch.interest.drained,
                GraphicsEventKind::Canceled => watch.interest.canceled,
                GraphicsEventKind::Faulted => watch.interest.faulted,
                GraphicsEventKind::Recovered => watch.interest.recovered,
                GraphicsEventKind::Retired => watch.interest.retired,
                GraphicsEventKind::LeaseReleased => watch.interest.lease_released,
                GraphicsEventKind::LeaseAcquired => watch.interest.lease_acquired,
            };
            if !interested {
                continue;
            }
            produced.push((
                binding,
                KernelEvent {
                    owner: queue.owner,
                    token: watch.token,
                    events: watch.events,
                    source: EventSource::Graphics {
                        device_inode,
                        request_id,
                        kind,
                    },
                },
            ));
        }
    }
    for (binding, event) in produced {
        enqueue_event(runtime, binding, event)?;
    }
    Ok(())
}

pub(crate) fn refresh_event_queue(
    runtime: &mut KernelRuntime,
    binding: QueueDescriptorTarget,
) -> Result<(), RuntimeError> {
    let QueueDescriptorTarget::Event {
        owner,
        queue,
        mode: _,
    } = binding
    else {
        return Err(RuntimeError::EventQueue(EventQueueError::InvalidQueue));
    };
    let queue_index = runtime
        .event_queues
        .iter()
        .position(|candidate| candidate.id == queue && candidate.owner == owner)
        .ok_or(EventQueueError::InvalidQueue)?;

    let watches = runtime.event_queues[queue_index].watches.clone();
    let mut remove_tokens = Vec::new();
    let mut updated_ready = Vec::new();

    for watch in watches {
        let events = runtime.poll_io(watch.owner, watch.fd)?;
        let matched = match_interest(events, watch.interest);
        let should_emit = if watch.behavior.edge_triggered {
            matched.0 != 0 && matched != watch.last_ready
        } else {
            matched.0 != 0
        };
        updated_ready.push((watch.owner, watch.fd, watch.token, matched));
        if should_emit {
            enqueue_event(
                runtime,
                binding,
                KernelEvent {
                    owner: watch.owner,
                    token: watch.token,
                    events: matched,
                    source: EventSource::Descriptor(watch.fd),
                },
            )?;
            if watch.behavior.oneshot {
                remove_tokens.push((watch.owner, watch.fd, watch.token));
            }
        }
    }

    if !remove_tokens.is_empty() {
        let queue = &mut runtime.event_queues[queue_index];
        queue.watches.retain(|watch| {
            !remove_tokens.iter().any(|(owner_id, fd, token)| {
                watch.owner == *owner_id && watch.fd == *fd && watch.token == *token
            })
        });
    }

    let queue = &mut runtime.event_queues[queue_index];
    for watch in &mut queue.watches {
        if let Some((_, _, _, matched)) = updated_ready.iter().find(|(owner_id, fd, token, _)| {
            watch.owner == *owner_id && watch.fd == *fd && watch.token == *token
        }) {
            watch.last_ready = *matched;
        }
    }
    sync_event_queue_readability(runtime, binding)
}

pub(crate) fn enqueue_event(
    runtime: &mut KernelRuntime,
    binding: QueueDescriptorTarget,
    event: KernelEvent,
) -> Result<(), RuntimeError> {
    let queue = event_queue_mut_by_binding(runtime, binding)?;
    let record = EventRecord {
        queue: binding.event_queue().ok_or(EventQueueError::InvalidQueue)?,
        mode: match binding {
            QueueDescriptorTarget::Event { mode, .. } => mode,
            QueueDescriptorTarget::Sleep { .. } => return Err(EventQueueError::InvalidQueue.into()),
        },
        owner: event.owner,
        token: event.token,
        events: event.events,
        source: event.source,
    };
    let inserted = queue.enqueue_pending(record);
    let waiters = if inserted {
        queue.drain_waiters()
    } else {
        Vec::new()
    };
    sync_event_queue_readability(runtime, binding)?;
    for waiter in waiters {
        runtime
            .scheduler
            .wake(&mut runtime.processes, waiter.owner, waiter.class)?;
    }
    Ok(())
}

pub(crate) fn sync_event_queue_readability(
    runtime: &mut KernelRuntime,
    binding: QueueDescriptorTarget,
) -> Result<(), RuntimeError> {
    let readable = !event_queue_mut_by_binding(runtime, binding)?
        .pending
        .is_empty();
    let descriptors = runtime
        .namespaces
        .iter()
        .flat_map(|(_, namespace)| namespace.descriptors.iter().flatten())
        .filter(|descriptor| descriptor.queue_binding() == Some(binding))
        .map(|descriptor| (descriptor.owner(), descriptor.fd()))
        .collect::<Vec<_>>();
    let state = if readable {
        IoState::Readable
    } else {
        IoState::Idle
    };
    for (descriptor_owner, fd) in descriptors {
        let _ = runtime.io_registry.set_state(descriptor_owner, fd, state);
    }
    Ok(())
}

pub(crate) fn enqueue_event_queue_refresh(
    runtime: &mut KernelRuntime,
    binding: QueueDescriptorTarget,
    priority: u16,
) -> Result<(), RuntimeError> {
    if matches!(binding, QueueDescriptorTarget::Sleep { .. }) {
        return Err(RuntimeError::EventQueue(EventQueueError::InvalidQueue));
    }
    runtime
        .deferred_tasks
        .enqueue(DeferredRuntimeTask::RefreshEventQueue(binding), priority)?;
    Ok(())
}

pub(crate) fn schedule_event_queue_refreshes_for_fd(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<(), RuntimeError> {
    let tasks = runtime
        .event_queues
        .iter()
        .filter(|queue| {
            queue
                .watches
                .iter()
                .any(|watch| watch.owner == owner && watch.fd == fd)
        })
        .map(|queue| {
            DeferredRuntimeTask::RefreshEventQueue(QueueDescriptorTarget::Event {
                owner: queue.owner,
                queue: queue.id,
                mode: queue.mode,
            })
        })
        .collect::<Vec<_>>();
    for task in tasks {
        runtime.deferred_tasks.enqueue(task, 24)?;
    }
    Ok(())
}

pub(crate) fn notify_descriptor_ready(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<(), RuntimeError> {
    schedule_event_queue_refreshes_for_fd(runtime, owner, fd)?;
    flush_deferred_tasks(runtime)
}

pub(crate) fn flush_deferred_tasks(runtime: &mut KernelRuntime) -> Result<(), RuntimeError> {
    while let Some((task, _, _)) = runtime.deferred_tasks.pop() {
        match task {
            DeferredRuntimeTask::RefreshEventQueue(binding) => {
                refresh_event_queue(runtime, binding)?
            }
        }
    }
    Ok(())
}
