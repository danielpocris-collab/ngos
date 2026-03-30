use super::*;
#[test]
fn runtime_supports_event_queues() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(22_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(22_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/render.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = runtime.open_path(owner, "/run/render.sock").unwrap();

    let queue = runtime
        .create_event_queue(owner, EventQueueMode::Epoll)
        .unwrap();
    runtime
        .watch_event(
            owner,
            queue,
            fd,
            77,
            ReadinessInterest {
                readable: true,
                writable: true,
                priority: false,
            },
            EventWatchBehavior::ONESHOT,
        )
        .unwrap();

    let first = runtime.wait_event_queue(owner, queue).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].queue, queue);
    assert_eq!(first[0].token, 77);
    assert_eq!(first[0].mode, EventQueueMode::Epoll);
    assert!(first[0].events.contains(IoPollEvents::WRITABLE));

    let second = runtime.wait_event_queue(owner, queue).unwrap();
    assert!(second.is_empty());
}

#[test]
fn syscall_surface_can_register_timer_watchers_on_event_queue_descriptors() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("timer-app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let queue_fd = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateEventQueueDescriptor {
                owner: app,
                mode: EventQueueMode::Kqueue,
            },
        )
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let timer = match surface
        .dispatch(
            context.clone(),
            Syscall::RegisterEventQueueTimerDescriptor {
                owner: app,
                queue_fd,
                token: 123,
                delay_ticks: 1,
                interval_ticks: None,
                events: IoPollEvents::PRIORITY,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueTimerRegistered(timer) => timer,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueInspected(info) => {
            assert_eq!(info.timer_count, 1);
            assert_eq!(info.timers[0].id, timer);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::RemoveEventQueueTimerDescriptor {
                owner: app,
                queue_fd,
                timer,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueTimerRemoved => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueInspected(info) => {
            assert_eq!(info.timer_count, 0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    let repeating = match surface
        .dispatch(
            context.clone(),
            Syscall::RegisterEventQueueTimerDescriptor {
                owner: app,
                queue_fd,
                token: 124,
                delay_ticks: 1,
                interval_ticks: Some(2),
                events: IoPollEvents::PRIORITY,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueTimerRegistered(timer) => timer,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let _ = surface.runtime.tick().unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::WaitEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueReady(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].source, EventSource::Timer(repeating));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_register_process_watchers_on_event_queue_descriptors() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process(
            "proc-watch-app",
            Some(bootstrap),
            SchedulerClass::Interactive,
        )
        .unwrap();
    let target = surface
        .runtime
        .spawn_process("proc-target", Some(bootstrap), SchedulerClass::BestEffort)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let queue_fd = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateEventQueueDescriptor {
                owner: app,
                mode: EventQueueMode::Epoll,
            },
        )
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::WatchProcessEventsDescriptor {
                owner: app,
                queue_fd,
                target,
                token: 321,
                interest: ProcessLifecycleInterest {
                    exited: true,
                    reaped: true,
                },
                events: IoPollEvents::PRIORITY,
            },
        )
        .unwrap()
    {
        SyscallResult::ProcessEventWatchRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueInspected(info) => {
            assert_eq!(info.process_watch_count, 1);
            assert_eq!(info.process_watches[0].target, target);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface.runtime.exit(target, 0).unwrap();
    match surface
        .dispatch(
            context.clone(),
            Syscall::WaitEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueReady(events) => {
            assert_eq!(
                events[0].source,
                EventSource::Process {
                    pid: target,
                    kind: ProcessLifecycleEventKind::Exited,
                }
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::RemoveProcessEventsDescriptor {
                owner: app,
                queue_fd,
                target,
                token: 321,
            },
        )
        .unwrap()
    {
        SyscallResult::ProcessEventWatchRemoved => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_register_signal_watchers_on_event_queue_descriptors() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process(
            "signal-watch-app",
            Some(bootstrap),
            SchedulerClass::Interactive,
        )
        .unwrap();
    surface
        .runtime
        .set_signal_disposition(app, 12, Some(SignalDisposition::Catch), 0, false)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let queue_fd = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateEventQueueDescriptor {
                owner: app,
                mode: EventQueueMode::Kqueue,
            },
        )
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::WatchSignalEventsDescriptor {
                owner: app,
                queue_fd,
                target: app,
                thread: None,
                signal_mask: 1u64 << (12 - 1),
                token: 777,
                events: IoPollEvents::PRIORITY,
            },
        )
        .unwrap()
    {
        SyscallResult::SignalEventWatchRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueInspected(info) => {
            assert_eq!(info.signal_watch_count, 1);
            assert_eq!(info.signal_watches[0].target, app);
            assert_eq!(info.signal_watches[0].signal_mask, 1u64 << (12 - 1));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface
        .runtime
        .send_signal(
            PendingSignalSender {
                pid: bootstrap,
                tid: context.tid,
            },
            app,
            12,
        )
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::WaitEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueReady(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token, 777);
            assert_eq!(
                events[0].source,
                EventSource::Signal {
                    pid: app,
                    tid: None,
                    signal: 12,
                }
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_register_memory_wait_watchers_on_event_queue_descriptors() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process(
            "mem-watch-app",
            Some(bootstrap),
            SchedulerClass::Interactive,
        )
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let mapped = surface
        .runtime
        .map_anonymous_memory(app, 0x1000, true, true, false, "futex-watch")
        .unwrap();
    surface.runtime.tick().unwrap();
    surface.runtime.block_running().unwrap();
    surface.runtime.tick().unwrap();

    let queue_fd = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateEventQueueDescriptor {
                owner: app,
                mode: EventQueueMode::Epoll,
            },
        )
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::WatchMemoryWaitEventsDescriptor {
                owner: app,
                queue_fd,
                domain: MemoryWaitDomain::Process(app),
                addr: mapped,
                token: 991,
                events: IoPollEvents::READABLE,
            },
        )
        .unwrap()
    {
        SyscallResult::MemoryWaitEventWatchRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueInspected(info) => {
            assert_eq!(info.memory_watch_count, 1);
            assert_eq!(
                info.memory_watches[0].domain,
                MemoryWaitDomain::Process(app)
            );
            assert_eq!(info.memory_watches[0].addr, mapped);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    assert_eq!(
            surface
                .runtime
                .wait_on_memory_word_in_domain(
                    app,
                    MemoryWaitDomain::Process(app),
                    mapped,
                    0,
                    Some(10),
                )
                .unwrap(),
            MemoryWordWaitResult::Blocked(app)
        );
    let woke = surface
        .runtime
        .wake_memory_word_in_domain(MemoryWaitDomain::Process(app), mapped, 1)
        .unwrap();
    assert_eq!(woke, vec![app]);

    match surface
        .dispatch(
            context,
            Syscall::WaitEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueReady(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token, 991);
            assert_eq!(
                events[0].source,
                EventSource::MemoryWait {
                    domain: MemoryWaitDomain::Process(app),
                    addr: mapped,
                    kind: MemoryWaitEventKind::Woken,
                }
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn runtime_event_queue_descriptor_becomes_readable_when_event_is_enqueued() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue-readable", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime
        .processes
        .get(owner)
        .unwrap()
        .main_thread()
        .expect("main thread");
    let queue_fd = runtime
        .create_event_queue_descriptor(owner, EventQueueMode::Kqueue)
        .unwrap();
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd).unwrap();

    assert!(
        !runtime
            .poll_io(owner, queue_fd)
            .unwrap()
            .contains(IoPollEvents::READABLE)
    );

    event_queue_runtime::enqueue_event(
        &mut runtime,
        binding,
        KernelEvent {
            owner,
            token: 11,
            events: IoPollEvents::READABLE,
            source: EventSource::Descriptor(Descriptor::new(99)),
        },
    )
    .unwrap();

    assert!(
        runtime
            .poll_io(owner, queue_fd)
            .unwrap()
            .contains(IoPollEvents::READABLE)
    );

    match runtime
        .wait_event_queue_descriptor(owner, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token, 11);
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    assert!(
        !runtime
            .poll_io(owner, queue_fd)
            .unwrap()
            .contains(IoPollEvents::READABLE)
    );
}

#[test]
fn runtime_event_queue_waiters_wake_when_event_arrives() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue-waiter", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime
        .processes
        .get(owner)
        .unwrap()
        .main_thread()
        .expect("main thread");
    let queue_fd = runtime
        .create_event_queue_descriptor(owner, EventQueueMode::Epoll)
        .unwrap();
    let binding = runtime.event_queue_binding_for_fd(owner, queue_fd).unwrap();

    let running = runtime.tick().unwrap();
    assert_eq!(running.pid, owner);

    match runtime
        .wait_event_queue_descriptor(owner, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Blocked(pid) => assert_eq!(pid, owner),
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    assert_eq!(
        runtime.processes.get(owner).unwrap().state(),
        ProcessState::Blocked
    );
    assert_eq!(
        runtime
            .inspect_event_queue_descriptor(owner, queue_fd)
            .unwrap()
            .waiter_count,
        1
    );

    event_queue_runtime::enqueue_event(
        &mut runtime,
        binding,
        KernelEvent {
            owner,
            token: 22,
            events: IoPollEvents::WRITABLE,
            source: EventSource::Descriptor(Descriptor::new(7)),
        },
    )
    .unwrap();

    assert_eq!(
        runtime.processes.get(owner).unwrap().state(),
        ProcessState::Ready
    );
    assert!(
        runtime
            .poll_io(owner, queue_fd)
            .unwrap()
            .contains(IoPollEvents::READABLE)
    );

    let running = runtime.tick().unwrap();
    assert_eq!(running.pid, owner);

    match runtime
        .wait_event_queue_descriptor(owner, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token, 22);
            assert!(events[0].events.contains(IoPollEvents::WRITABLE));
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    assert_eq!(
        runtime
            .inspect_event_queue_descriptor(owner, queue_fd)
            .unwrap()
            .waiter_count,
        0
    );
}

#[test]
fn runtime_event_queue_timer_producer_enqueues_events() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue-timer", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime
        .processes
        .get(owner)
        .unwrap()
        .main_thread()
        .expect("main thread");
    let queue_fd = runtime
        .create_event_queue_descriptor(owner, EventQueueMode::Kqueue)
        .unwrap();
    let timer = runtime
        .register_event_queue_timer_descriptor(owner, queue_fd, 33, 1, None, IoPollEvents::PRIORITY)
        .unwrap();

    assert!(
        !runtime
            .poll_io(owner, queue_fd)
            .unwrap()
            .contains(IoPollEvents::READABLE)
    );

    let _ = runtime.tick().unwrap();

    assert!(
        runtime
            .poll_io(owner, queue_fd)
            .unwrap()
            .contains(IoPollEvents::READABLE)
    );

    match runtime
        .wait_event_queue_descriptor(owner, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token, 33);
            assert_eq!(events[0].source, EventSource::Timer(timer));
            assert!(events[0].events.contains(IoPollEvents::PRIORITY));
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }
}

#[test]
fn runtime_event_queue_process_lifecycle_producer_enqueues_events() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let watcher = runtime
        .spawn_process("queue-proc-watch", None, SchedulerClass::Interactive)
        .unwrap();
    let watched = runtime
        .spawn_process("queue-proc-target", None, SchedulerClass::BestEffort)
        .unwrap();
    let tid = runtime
        .processes
        .get(watcher)
        .unwrap()
        .main_thread()
        .expect("main thread");
    let queue_fd = runtime
        .create_event_queue_descriptor(watcher, EventQueueMode::Epoll)
        .unwrap();
    runtime
        .watch_process_events_descriptor(
            watcher,
            queue_fd,
            watched,
            44,
            ProcessLifecycleInterest {
                exited: true,
                reaped: true,
            },
            IoPollEvents::PRIORITY,
        )
        .unwrap();

    runtime.exit(watched, 0).unwrap();
    match runtime
        .wait_event_queue_descriptor(watcher, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(
                events[0].source,
                EventSource::Process {
                    pid: watched,
                    kind: ProcessLifecycleEventKind::Exited,
                }
            );
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    let _ = runtime.reap_process(watched).unwrap();
    match runtime
        .wait_event_queue_descriptor(watcher, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(
                events[0].source,
                EventSource::Process {
                    pid: watched,
                    kind: ProcessLifecycleEventKind::Reaped,
                }
            );
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }
}

#[test]
fn runtime_resource_claim_events_wake_event_queue_waiters_and_report_handoff() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let holder = runtime
        .spawn_process("resource-holder", None, SchedulerClass::BestEffort)
        .unwrap();
    let watcher = runtime
        .spawn_process("resource-watcher", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime
        .processes
        .get(watcher)
        .unwrap()
        .main_thread()
        .expect("main thread");
    let domain = runtime.create_domain(holder, None, "display").unwrap();
    let resource = runtime
        .create_resource(holder, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(holder, domain, resource, ContractKind::Display, "primary")
        .unwrap();
    let mirror = runtime
        .create_contract(watcher, domain, resource, ContractKind::Display, "mirror")
        .unwrap();
    let queue_fd = runtime
        .create_event_queue_descriptor(watcher, EventQueueMode::Epoll)
        .unwrap();
    runtime
        .watch_resource_events_descriptor(
            watcher,
            queue_fd,
            resource,
            515,
            ResourceEventInterest {
                claimed: false,
                queued: true,
                canceled: false,
                released: false,
                handed_off: true,
                revoked: true,
            },
            IoPollEvents::PRIORITY,
        )
        .unwrap();

    assert_eq!(
        runtime.claim_resource_via_contract(primary).unwrap(),
        ResourceClaimResult::Acquired {
            resource,
            acquire_count: 1,
        }
    );

    let running = runtime.tick().unwrap();
    assert_eq!(running.pid, watcher);
    match runtime
        .wait_event_queue_descriptor(watcher, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Blocked(pid) => assert_eq!(pid, watcher),
        other => panic!("unexpected event queue wait result: {other:?}"),
    }
    assert_eq!(
        runtime.processes.get(watcher).unwrap().state(),
        ProcessState::Blocked
    );

    assert_eq!(
        runtime.claim_resource_via_contract(mirror).unwrap(),
        ResourceClaimResult::Queued {
            resource,
            holder: primary,
            position: 1,
        }
    );
    assert_eq!(
        runtime.processes.get(watcher).unwrap().state(),
        ProcessState::Ready
    );
    let running = runtime.tick().unwrap();
    assert_eq!(running.pid, watcher);
    match runtime
        .wait_event_queue_descriptor(watcher, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token, 515);
            assert_eq!(
                events[0].source,
                EventSource::Resource {
                    resource,
                    contract: mirror,
                    kind: ResourceEventKind::Queued,
                }
            );
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    match runtime
        .release_claimed_resource_via_contract(primary)
        .unwrap()
    {
        ResourceReleaseResult::HandedOff {
            resource: released,
            contract,
            acquire_count,
            handoff_count,
        } => {
            assert_eq!(released, resource);
            assert_eq!(contract, mirror);
            assert_eq!(acquire_count, 2);
            assert_eq!(handoff_count, 1);
        }
        other => panic!("unexpected release result: {other:?}"),
    }
    match runtime
        .wait_event_queue_descriptor(watcher, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert!(events.iter().any(|event| {
                event.source
                    == EventSource::Resource {
                        resource,
                        contract: mirror,
                        kind: ResourceEventKind::HandedOff,
                    }
            }));
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    let info = runtime
        .inspect_event_queue_descriptor(watcher, queue_fd)
        .unwrap();
    assert_eq!(info.resource_watch_count, 1);
    assert_eq!(info.resource_watches[0].resource, resource);
}

#[test]
fn syscall_surface_can_register_resource_watchers_on_event_queue_descriptors() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let watcher = surface
        .runtime
        .spawn_process(
            "resource-watch-app",
            Some(bootstrap),
            SchedulerClass::Interactive,
        )
        .unwrap();
    let holder = surface
        .runtime
        .spawn_process(
            "resource-holder",
            Some(bootstrap),
            SchedulerClass::BestEffort,
        )
        .unwrap();
    let domain = surface
        .runtime
        .create_domain(holder, None, "display")
        .unwrap();
    let resource = surface
        .runtime
        .create_resource(holder, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = surface
        .runtime
        .create_contract(holder, domain, resource, ContractKind::Display, "primary")
        .unwrap();
    let mirror = surface
        .runtime
        .create_contract(watcher, domain, resource, ContractKind::Display, "mirror")
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let queue_fd = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateEventQueueDescriptor {
                owner: watcher,
                mode: EventQueueMode::Kqueue,
            },
        )
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::WatchResourceEventsDescriptor {
                owner: watcher,
                queue_fd,
                resource,
                token: 909,
                interest: ResourceEventInterest {
                    claimed: true,
                    queued: true,
                    canceled: true,
                    released: true,
                    handed_off: true,
                    revoked: true,
                },
                events: IoPollEvents::PRIORITY,
            },
        )
        .unwrap()
    {
        SyscallResult::ResourceEventWatchRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectEventQueueDescriptor {
                owner: watcher,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueInspected(info) => {
            assert_eq!(info.resource_watch_count, 1);
            assert_eq!(info.resource_watches[0].resource, resource);
            assert_eq!(info.resource_watches[0].token, 909);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface
        .runtime
        .claim_resource_via_contract(primary)
        .unwrap();
    surface.runtime.claim_resource_via_contract(mirror).unwrap();

    let tid = surface
        .runtime
        .processes
        .get(watcher)
        .unwrap()
        .main_thread()
        .unwrap();
    match surface
        .runtime
        .wait_event_queue_descriptor(watcher, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert!(events.iter().any(|event| {
                event.source
                    == EventSource::Resource {
                        resource,
                        contract: primary,
                        kind: ResourceEventKind::Claimed,
                    }
            }));
            assert!(events.iter().any(|event| {
                event.source
                    == EventSource::Resource {
                        resource,
                        contract: mirror,
                        kind: ResourceEventKind::Queued,
                    }
            }));
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }
}

#[test]
fn runtime_network_watchers_wake_on_rx_txdrain_and_link_changes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-watch", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime.processes.get(owner).unwrap().threads()[0];
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(42_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let nic = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(42_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "nic",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(42_002), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "driver",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(42_003), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net0", ObjectKind::Device, nic)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .create_vfs_node("/run/net0.sock", ObjectKind::Socket, socket)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/net0", "/drv/net0")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net0",
            [10, 1, 0, 2],
            [255, 255, 255, 0],
            [10, 1, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net0.sock",
            owner,
            "/dev/net0",
            4000,
            [10, 1, 0, 9],
            5000,
        )
        .unwrap();

    let queue_fd = runtime
        .create_event_queue_descriptor(owner, EventQueueMode::Kqueue)
        .unwrap();
    let interface_inode = runtime.stat_path("/dev/net0").unwrap().inode;
    let socket_inode = runtime.stat_path("/run/net0.sock").unwrap().inode;
    runtime
        .watch_network_events_descriptor(
            owner,
            queue_fd,
            interface_inode,
            None,
            800,
            NetworkEventInterest {
                link_changed: true,
                rx_ready: true,
                tx_drained: true,
            },
            IoPollEvents::PRIORITY,
        )
        .unwrap();
    runtime
        .watch_network_events_descriptor(
            owner,
            queue_fd,
            interface_inode,
            Some(socket_inode),
            801,
            NetworkEventInterest {
                link_changed: false,
                rx_ready: true,
                tx_drained: false,
            },
            IoPollEvents::READABLE,
        )
        .unwrap();

    let driver_fd = runtime.open_path(owner, "/drv/net0").unwrap();
    let socket_fd = runtime.open_path(owner, "/run/net0.sock").unwrap();

    runtime.write_io(owner, socket_fd, b"frame:tx").unwrap();
    let _ = runtime.read_io(owner, driver_fd, 256).unwrap();
    match runtime
        .wait_event_queue_descriptor(owner, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert!(events.iter().any(|event| {
                event.token == 800
                    && event.source
                        == EventSource::Network {
                            interface_inode,
                            socket_inode: None,
                            kind: NetworkEventKind::TxDrained,
                        }
            }));
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    let injected = {
        let mut bytes = vec![0xff; 6];
        bytes.extend_from_slice(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        bytes.extend_from_slice(&0x0800u16.to_be_bytes());
        let payload = b"frame:rx";
        let total_len = 20 + 8 + payload.len();
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&[10, 1, 0, 9]);
        ip[16..20].copy_from_slice(&[10, 1, 0, 2]);
        bytes.extend_from_slice(&ip);
        bytes.extend_from_slice(&5000u16.to_be_bytes());
        bytes.extend_from_slice(&4000u16.to_be_bytes());
        bytes.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    };
    runtime.write_io(owner, driver_fd, &injected).unwrap();
    match runtime
        .wait_event_queue_descriptor(owner, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert!(events.iter().any(|event| {
                event.token == 800
                    && event.source
                        == EventSource::Network {
                            interface_inode,
                            socket_inode: None,
                            kind: NetworkEventKind::RxReady,
                        }
            }));
            assert!(events.iter().any(|event| {
                event.token == 801
                    && event.source
                        == EventSource::Network {
                            interface_inode,
                            socket_inode: Some(socket_inode),
                            kind: NetworkEventKind::RxReady,
                        }
            }));
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }

    runtime
        .set_network_interface_link_state("/dev/net0", false)
        .unwrap();
    match runtime
        .wait_event_queue_descriptor(owner, queue_fd, tid)
        .unwrap()
    {
        EventQueueWaitResult::Ready(events) => {
            assert!(events.iter().any(|event| {
                event.token == 800
                    && event.source
                        == EventSource::Network {
                            interface_inode,
                            socket_inode: None,
                            kind: NetworkEventKind::LinkChanged,
                        }
            }));
        }
        other => panic!("unexpected event queue wait result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_register_network_watchers_on_event_queue_descriptors() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let watcher = surface
        .runtime
        .spawn_process(
            "network-watch-app",
            Some(bootstrap),
            SchedulerClass::Interactive,
        )
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            watcher,
            ObjectHandle::new(Handle::new(43_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let nic = surface
        .runtime
        .grant_capability(
            watcher,
            ObjectHandle::new(Handle::new(43_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "nic",
        )
        .unwrap();
    let driver = surface
        .runtime
        .grant_capability(
            watcher,
            ObjectHandle::new(Handle::new(43_002), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "driver",
        )
        .unwrap();
    let socket = surface
        .runtime
        .grant_capability(
            watcher,
            ObjectHandle::new(Handle::new(43_003), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "socket",
        )
        .unwrap();

    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/dev/net1", ObjectKind::Device, nic)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/drv/net1", ObjectKind::Driver, driver)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/run/net1.sock", ObjectKind::Socket, socket)
        .unwrap();
    surface
        .runtime
        .bind_device_to_driver("/dev/net1", "/drv/net1")
        .unwrap();
    surface
        .runtime
        .bind_udp_socket(
            "/run/net1.sock",
            watcher,
            "/dev/net1",
            4100,
            [10, 2, 0, 9],
            5100,
        )
        .unwrap();

    let context = SyscallContext::kernel(bootstrap);
    let queue_fd = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateEventQueueDescriptor {
                owner: watcher,
                mode: EventQueueMode::Epoll,
            },
        )
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::WatchNetworkEventsDescriptor {
                owner: watcher,
                queue_fd,
                interface_path: "/dev/net1".to_string(),
                socket_path: Some("/run/net1.sock".to_string()),
                token: 919,
                interest: NetworkEventInterest {
                    link_changed: true,
                    rx_ready: true,
                    tx_drained: true,
                },
                events: IoPollEvents::PRIORITY,
            },
        )
        .unwrap()
    {
        SyscallResult::NetworkEventWatchRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectEventQueueDescriptor {
                owner: watcher,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueInspected(info) => {
            assert_eq!(info.network_watch_count, 1);
            assert_eq!(info.network_watches[0].token, 919);
            assert!(info.network_watches[0].socket_inode.is_some());
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::RemoveNetworkEventsDescriptor {
                owner: watcher,
                queue_fd,
                interface_path: "/dev/net1".to_string(),
                socket_path: Some("/run/net1.sock".to_string()),
                token: 919,
            },
        )
        .unwrap()
    {
        SyscallResult::NetworkEventWatchRemoved => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn runtime_uses_deferred_taskqueue_for_event_queue_refresh() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue-task", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(22_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(22_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/deferred.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = runtime.open_path(owner, "/run/deferred.sock").unwrap();
    let queue = runtime
        .create_event_queue(owner, EventQueueMode::Kqueue)
        .unwrap();

    runtime
        .watch_event(
            owner,
            queue,
            fd,
            99,
            ReadinessInterest {
                readable: false,
                writable: true,
                priority: false,
            },
            EventWatchBehavior::LEVEL,
        )
        .unwrap();
    assert_eq!(runtime.snapshot().deferred_task_count, 1);

    let events = runtime.wait_event_queue(owner, queue).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].token, 99);
    assert_eq!(runtime.snapshot().deferred_task_count, 0);
}

#[test]
fn reap_process_discards_deferred_event_queue_refresh_tasks() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue-reap", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(47_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(47_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/reap.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = runtime.open_path(owner, "/run/reap.sock").unwrap();
    let queue = runtime
        .create_event_queue(owner, EventQueueMode::Kqueue)
        .unwrap();

    runtime
        .watch_event(
            owner,
            queue,
            fd,
            17,
            ReadinessInterest {
                readable: true,
                writable: true,
                priority: false,
            },
            EventWatchBehavior::LEVEL,
        )
        .unwrap();
    assert_eq!(runtime.snapshot().deferred_task_count, 1);

    runtime.exit(owner, 0).unwrap();
    let _ = runtime.reap_process(owner).unwrap();

    assert_eq!(runtime.snapshot().deferred_task_count, 0);
    assert!(
        !runtime
            .inspect_system()
            .event_queues
            .iter()
            .any(|entry| entry.id == queue)
    );
}

#[test]
fn runtime_supports_sleep_queues_and_timeout_wakeups() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("sleeper", None, SchedulerClass::Interactive)
        .unwrap();
    runtime.tick().unwrap();

    let queue = runtime.create_sleep_queue(owner).unwrap();
    runtime
        .sleep_on_queue(owner, queue, 0x55, 10, Some(1))
        .unwrap();
    assert_eq!(
        runtime.processes().get(owner).unwrap().state(),
        ProcessState::Blocked
    );
    assert_eq!(runtime.snapshot().sleeping_processes, 1);

    let scheduled = runtime.tick().unwrap();
    assert_eq!(scheduled.pid, owner);
    assert_eq!(runtime.snapshot().sleeping_processes, 0);
    assert_eq!(
        runtime.last_sleep_result(owner),
        Some(SleepWaitResult::TimedOut)
    );

    runtime.sleep_on_queue(owner, queue, 0x66, 5, None).unwrap();
    let woke = runtime.wake_one_sleep_queue(owner, queue, 0x66).unwrap();
    assert_eq!(woke, Some(owner));
    assert_eq!(
        runtime.processes().get(owner).unwrap().state(),
        ProcessState::Ready
    );
    assert_eq!(
        runtime.last_sleep_result(owner),
        Some(SleepWaitResult::Woken)
    );

    let decisions = runtime.recent_wait_agent_decisions();
    assert!(decisions.iter().any(|entry| {
        entry.agent == WaitAgentKind::SleepEnqueueAgent
            && entry.owner == owner.raw()
            && entry.queue == queue.0
            && entry.channel == 0x55
    }));
    assert!(decisions.iter().any(|entry| {
        entry.agent == WaitAgentKind::SleepWakeAgent
            && entry.owner == owner.raw()
            && entry.queue == queue.0
            && entry.channel == 0x66
            && entry.detail0 == 1
    }));
}

#[test]
fn inspect_system_drops_reaped_process_runtime_artifacts() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::BestEffort)
        .unwrap();
    let app = runtime
        .spawn_process_share_fds("app", Some(init), SchedulerClass::Interactive, init)
        .unwrap();
    let mut scheduled = runtime.tick().unwrap();
    while scheduled.pid != app {
        scheduled = runtime.tick().unwrap();
    }

    let queue = runtime
        .create_event_queue(app, EventQueueMode::Epoll)
        .unwrap();
    let sleep_queue = runtime.create_sleep_queue(app).unwrap();
    runtime
        .sleep_on_queue(app, sleep_queue, 0x99, 5, None)
        .unwrap();
    assert_eq!(
        runtime.processes().get(app).unwrap().state(),
        ProcessState::Blocked
    );
    runtime
        .cancel_sleep_queue_owner(app, sleep_queue, app)
        .unwrap();
    runtime.exit(app, 0).unwrap();
    let _ = runtime.reap_process(app).unwrap();

    let system = runtime.inspect_system();
    assert!(!system.processes.iter().any(|process| process.pid == app));
    assert!(!system.address_spaces.iter().any(|space| space.owner == app));
    assert!(!system.event_queues.iter().any(|entry| entry.id == queue));
    assert!(
        !system
            .sleep_queues
            .iter()
            .any(|entry| entry.id == sleep_queue)
    );
    assert!(
        !system
            .fdshare_groups
            .iter()
            .any(|group| group.members.contains(&app))
    );
    assert_eq!(system.snapshot.process_count, 1);
    assert_eq!(system.snapshot.sleeping_processes, 0);
}

#[test]
fn runtime_can_update_memory_words_with_kernel_ops() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let app = runtime
        .spawn_process("wordops", None, SchedulerClass::Interactive)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(app, 0x1000, true, true, false, "wordops")
        .unwrap();

    assert_eq!(
        runtime
            .update_memory_word(app, mapped, MemoryWordUpdateOp::Add(5))
            .unwrap(),
        (0, 5)
    );
    assert_eq!(
        runtime
            .update_memory_word(app, mapped, MemoryWordUpdateOp::Or(0b1000))
            .unwrap(),
        (5, 13)
    );
    assert_eq!(
        runtime
            .update_memory_word(app, mapped, MemoryWordUpdateOp::AndNot(0b0001))
            .unwrap(),
        (13, 12)
    );
    assert_eq!(
        runtime
            .update_memory_word(app, mapped, MemoryWordUpdateOp::Xor(0b0110))
            .unwrap(),
        (12, 10)
    );
    assert_eq!(runtime.compare_memory_word(app, mapped, 10).unwrap(), 10);
    assert_eq!(runtime.load_memory_word(app, mapped).unwrap(), 10);
}

#[test]
fn runtime_can_requeue_sleep_waiters_between_channels() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("sleeper", None, SchedulerClass::Interactive)
        .unwrap();
    runtime.tick().unwrap();

    let queue = runtime.create_sleep_queue(owner).unwrap();
    runtime.sleep_on_queue(owner, queue, 0x11, 5, None).unwrap();
    assert_eq!(
        runtime
            .requeue_sleep_queue(owner, queue, 0x11, 0x22, 1)
            .unwrap(),
        1
    );
    assert_eq!(
        runtime.wake_one_sleep_queue(owner, queue, 0x11).unwrap(),
        None
    );
    assert_eq!(
        runtime.wake_one_sleep_queue(owner, queue, 0x22).unwrap(),
        Some(owner)
    );

    let decisions = runtime.recent_wait_agent_decisions();
    assert!(decisions.iter().any(|entry| {
        entry.agent == WaitAgentKind::SleepRequeueAgent
            && entry.owner == owner.raw()
            && entry.queue == queue.0
            && entry.channel == 0x11
            && entry.detail0 == 0x22
            && entry.detail1 == 1
    }));
}

#[test]
fn runtime_supports_memory_word_wait_wake_and_requeue() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("waiter", None, SchedulerClass::Interactive)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(owner, 0x1000, true, true, false, "futex")
        .unwrap();
    runtime.store_memory_word(owner, mapped, 7).unwrap();
    runtime.tick().unwrap();

    assert_eq!(
        runtime
            .wait_on_memory_word(owner, 0, mapped, 9, None)
            .unwrap(),
        MemoryWordWaitResult::ValueMismatch {
            expected: 9,
            observed: 7,
        }
    );
    assert_eq!(
        runtime
            .wait_on_memory_word(owner, 0, mapped, 7, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(owner)
    );
    assert_eq!(runtime.wake_memory_word(0, mapped, 1).unwrap(), vec![owner]);
    assert_eq!(
        runtime.last_sleep_result(owner),
        Some(SleepWaitResult::Woken)
    );

    runtime.tick().unwrap();
    assert_eq!(
        runtime
            .wait_on_memory_word(owner, 0, mapped, 7, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(owner)
    );
    assert_eq!(
        runtime
            .requeue_memory_word(0, mapped, 0, mapped + 8, 0, 1)
            .unwrap(),
        MemoryWordRequeueResult {
            woke: Vec::new(),
            moved: 1,
        }
    );
    assert!(runtime.wake_memory_word(0, mapped, 1).unwrap().is_empty());
    assert_eq!(
        runtime.wake_memory_word(0, mapped + 8, 1).unwrap(),
        vec![owner]
    );

    let decisions = runtime.recent_wait_agent_decisions();
    assert!(decisions.iter().any(|entry| {
        entry.agent == WaitAgentKind::MemoryWaitAgent
            && entry.owner == owner.raw()
            && entry.detail0 == 7
    }));
    assert!(decisions.iter().any(|entry| {
        entry.agent == WaitAgentKind::MemoryWaitAgent
            && entry.owner == owner.raw()
            && entry.detail0 == 3
            && entry.detail1 == 1
    }));
}

#[test]
fn inspect_system_exports_wait_agent_decisions() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("sleeper", None, SchedulerClass::Interactive)
        .unwrap();
    runtime.tick().unwrap();

    let queue = runtime.create_sleep_queue(owner).unwrap();
    runtime.sleep_on_queue(owner, queue, 0x44, 3, None).unwrap();
    let _ = runtime.wake_one_sleep_queue(owner, queue, 0x44).unwrap();

    let system = runtime.inspect_system();
    assert!(!system.wait_agent_decisions.is_empty());
    assert!(system.wait_agent_decisions.iter().any(|entry| {
        entry.agent == WaitAgentKind::SleepEnqueueAgent
            && entry.owner == owner.raw()
            && entry.queue == queue.0
    }));
    assert!(system.wait_agent_decisions.iter().any(|entry| {
        entry.agent == WaitAgentKind::SleepWakeAgent
            && entry.owner == owner.raw()
            && entry.queue == queue.0
    }));
}

#[test]
fn runtime_can_wait_on_any_memory_word_and_track_resume_index() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("waitv", None, SchedulerClass::Interactive)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(owner, 0x1000, true, true, false, "waitv")
        .unwrap();
    runtime.store_memory_word(owner, mapped, 1).unwrap();
    runtime.store_memory_word(owner, mapped + 8, 5).unwrap();
    runtime.tick().unwrap();

    assert_eq!(
        runtime
            .wait_on_any_memory_word(
                owner,
                &[
                    MemoryWordWaitEntry {
                        namespace: 0,
                        addr: mapped,
                        expected: 9,
                    },
                    MemoryWordWaitEntry {
                        namespace: 0,
                        addr: mapped + 8,
                        expected: 5,
                    },
                ],
                None,
            )
            .unwrap(),
        MemoryWordWaitAnyResult::Blocked {
            pid: owner,
            index: 1,
        }
    );
    assert_eq!(runtime.memory_wait_resume_index(owner), Some(1));
    assert!(runtime.wake_memory_word(0, mapped, 1).unwrap().is_empty());
    assert_eq!(
        runtime.wake_memory_word(0, mapped + 8, 1).unwrap(),
        vec![owner]
    );
}

#[test]
fn runtime_can_apply_memory_word_wake_op() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let from_waiter = runtime
        .spawn_process("wakeop-from", None, SchedulerClass::Interactive)
        .unwrap();
    let to_waiter = runtime
        .spawn_process("wakeop-to", None, SchedulerClass::Interactive)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(from_waiter, 0x1000, true, true, false, "wakeop")
        .unwrap();
    runtime.store_memory_word(from_waiter, mapped, 2).unwrap();
    runtime
        .store_memory_word(from_waiter, mapped + 8, 1)
        .unwrap();
    runtime
        .processes
        .copy_vm_state(to_waiter, from_waiter)
        .unwrap();
    runtime.tick().unwrap();

    assert_eq!(
        runtime
            .wait_on_memory_word(from_waiter, 0, mapped, 2, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(from_waiter)
    );
    runtime.tick().unwrap();
    assert_eq!(
        runtime
            .wait_on_memory_word(to_waiter, 0, mapped + 8, 1, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(to_waiter)
    );

    let result = runtime
        .wake_memory_word_op(
            from_waiter,
            0,
            mapped,
            0,
            mapped + 8,
            1,
            1,
            MemoryWordUpdateOp::Add(1),
            MemoryWordCompareOp::Eq,
            1,
        )
        .unwrap();
    assert_eq!(result.old_value, 1);
    assert_eq!(result.new_value, 2);
    assert!(result.comparison_matched);
    assert_eq!(result.woke_from, vec![from_waiter]);
    assert_eq!(result.woke_to, vec![to_waiter]);
}

#[test]
fn runtime_can_cmp_requeue_memory_words() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let first = runtime
        .spawn_process("cmpreq-first", None, SchedulerClass::Interactive)
        .unwrap();
    let second = runtime
        .spawn_process("cmpreq-second", None, SchedulerClass::Interactive)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(first, 0x1000, true, true, false, "cmpreq")
        .unwrap();
    runtime.store_memory_word(first, mapped, 6).unwrap();
    runtime.processes.copy_vm_state(second, first).unwrap();
    runtime.tick().unwrap();
    assert_eq!(
        runtime
            .wait_on_memory_word(first, 0, mapped, 6, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(first)
    );
    runtime.tick().unwrap();
    assert_eq!(
        runtime
            .wait_on_memory_word(second, 0, mapped, 6, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(second)
    );

    assert_eq!(
        runtime
            .cmp_requeue_memory_word(first, 0, mapped, 0, mapped + 8, 9, 1, 1)
            .unwrap(),
        MemoryWordCmpRequeueResult::ValueMismatch {
            expected: 9,
            observed: 6,
        }
    );
    assert_eq!(
        runtime
            .cmp_requeue_memory_word(first, 0, mapped, 0, mapped + 8, 6, 1, 1)
            .unwrap(),
        MemoryWordCmpRequeueResult::Requeued(MemoryWordRequeueResult {
            woke: vec![first],
            moved: 1,
        })
    );
    assert!(runtime.wake_memory_word(0, mapped, 1).unwrap().is_empty());
    assert_eq!(
        runtime.wake_memory_word(0, mapped + 8, 1).unwrap(),
        vec![second]
    );
}

#[test]
fn runtime_signal_delivery_marks_pending_and_cancels_memory_waits() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("signaled", None, SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_signal_disposition(target, 9, Some(SignalDisposition::Catch), 0, false)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(target, 0x1000, true, true, false, "signal")
        .unwrap();
    runtime.store_memory_word(target, mapped, 4).unwrap();
    runtime.tick().unwrap();
    assert_eq!(
        runtime
            .wait_on_memory_word(target, 0, mapped, 4, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(target)
    );

    runtime
        .send_signal(
            PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
            target,
            9,
        )
        .unwrap();
    assert_eq!(runtime.pending_signals(target).unwrap(), vec![9]);
    assert_eq!(
        runtime.last_sleep_result(target),
        Some(SleepWaitResult::Canceled)
    );
}

#[test]
fn runtime_blocked_signal_stays_pending_without_canceling_wait() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("masked", None, SchedulerClass::Interactive)
        .unwrap();
    let mapped = runtime
        .map_anonymous_memory(target, 0x1000, true, true, false, "mask")
        .unwrap();
    runtime.store_memory_word(target, mapped, 3).unwrap();
    runtime
        .set_signal_mask(target, SignalMaskHow::Block, 1u64 << (9 - 1))
        .unwrap();
    runtime.tick().unwrap();
    assert_eq!(
        runtime
            .wait_on_memory_word(target, 0, mapped, 3, None)
            .unwrap(),
        MemoryWordWaitResult::Blocked(target)
    );

    runtime
        .send_signal(
            PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
            target,
            9,
        )
        .unwrap();
    assert_eq!(runtime.pending_signals(target).unwrap(), vec![9]);
    assert_eq!(runtime.blocked_pending_signals(target).unwrap(), vec![9]);
    assert_eq!(runtime.last_sleep_result(target), None);
    assert_eq!(
        runtime.processes.get(target).unwrap().state(),
        ProcessState::Blocked
    );
}

#[test]
fn runtime_can_take_blocked_pending_signal_from_mask() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("sigwait-target", None, SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_signal_mask(
            target,
            SignalMaskHow::Block,
            (1u64 << (9 - 1)) | (1u64 << (3 - 1)),
        )
        .unwrap();
    runtime
        .send_signal(
            PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
            target,
            9,
        )
        .unwrap();
    runtime
        .send_signal(
            PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
            target,
            3,
        )
        .unwrap();

    assert_eq!(
        runtime
            .take_pending_signal(target, 1u64 << (9 - 1), true)
            .unwrap(),
        Some(9)
    );
    assert_eq!(runtime.pending_signals(target).unwrap(), vec![3]);
    assert_eq!(
        runtime
            .take_pending_signal(target, (1u64 << (9 - 1)) | (1u64 << (3 - 1)), true)
            .unwrap(),
        Some(3)
    );
    assert!(runtime.pending_signals(target).unwrap().is_empty());
    assert!(runtime.blocked_pending_signals(target).unwrap().is_empty());
}

#[test]
fn runtime_wait_for_pending_signal_blocks_and_wakes_on_masked_delivery() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("sigwait-blocked", None, SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_signal_mask(target, SignalMaskHow::Block, 1u64 << (9 - 1))
        .unwrap();
    runtime.tick().unwrap();

    assert_eq!(
        runtime
            .wait_for_pending_signal(target, 1u64 << (9 - 1), Some(5))
            .unwrap(),
        PendingSignalWaitResult::Blocked(target)
    );
    runtime
        .send_signal(
            PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
            target,
            9,
        )
        .unwrap();
    assert_eq!(
        runtime.inspect_pending_signal_wait(target).unwrap(),
        Some(PendingSignalWaitResume::Delivered(PendingSignalDelivery {
            signal: 9,
            code: PendingSignalCode::Kill,
            value: None,
            source: PendingSignalSource::Process,
            sender: PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
        }))
    );
    assert!(runtime.pending_signals(target).unwrap().is_empty());
}

#[test]
fn runtime_wait_for_pending_signal_can_timeout_immediately() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("sigwait-timeout", None, SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_signal_mask(target, SignalMaskHow::Block, 1u64 << (9 - 1))
        .unwrap();
    assert_eq!(
        runtime
            .wait_for_pending_signal(target, 1u64 << (9 - 1), Some(0))
            .unwrap(),
        PendingSignalWaitResult::TimedOut
    );
    assert_eq!(
        runtime.inspect_pending_signal_wait(target).unwrap(),
        Some(PendingSignalWaitResume::TimedOut)
    );
}

#[test]
fn syscall_surface_supports_sleep_queue_operations() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let app_context = SyscallContext::kernel(app);

    let queue_fd = match surface
        .dispatch(context.clone(), Syscall::CreateSleepQueue { owner: app })
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    surface.runtime.tick().unwrap();
    surface.runtime.block_running().unwrap();
    surface.runtime.tick().unwrap();
    match surface
        .dispatch(
            app_context,
            Syscall::SleepOnQueueDescriptor {
                owner: app,
                fd: queue_fd,
                channel: 0xaa,
                priority: 7,
                timeout_ticks: None,
            },
        )
        .unwrap()
    {
        SyscallResult::ProcessBlockedOnSleepQueue(pid) => assert_eq!(pid, app),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::WakeOneSleepQueueDescriptor {
                owner: app,
                fd: queue_fd,
                channel: 0xaa,
            },
        )
        .unwrap()
    {
        SyscallResult::SleepQueueWakeResult(pids) => assert_eq!(pids, vec![app]),
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_requeue_sleep_waiters() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let app_context = SyscallContext::kernel(app);

    let queue_fd = match surface
        .dispatch(context.clone(), Syscall::CreateSleepQueue { owner: app })
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    surface.runtime.tick().unwrap();
    surface.runtime.block_running().unwrap();
    surface.runtime.tick().unwrap();
    match surface
        .dispatch(
            app_context,
            Syscall::SleepOnQueueDescriptor {
                owner: app,
                fd: queue_fd,
                channel: 0x33,
                priority: 7,
                timeout_ticks: None,
            },
        )
        .unwrap()
    {
        SyscallResult::ProcessBlockedOnSleepQueue(pid) => assert_eq!(pid, app),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::RequeueSleepQueueDescriptor {
                owner: app,
                fd: queue_fd,
                from_channel: 0x33,
                to_channel: 0x44,
                max_count: 1,
            },
        )
        .unwrap()
    {
        SyscallResult::SleepQueueRequeueResult(count) => assert_eq!(count, 1),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::WakeOneSleepQueueDescriptor {
                owner: app,
                fd: queue_fd,
                channel: 0x33,
            },
        )
        .unwrap()
    {
        SyscallResult::SleepQueueWakeResult(pids) => assert!(pids.is_empty()),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::WakeOneSleepQueueDescriptor {
                owner: app,
                fd: queue_fd,
                channel: 0x44,
            },
        )
        .unwrap()
    {
        SyscallResult::SleepQueueWakeResult(pids) => assert_eq!(pids, vec![app]),
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_inspect_sleep_results() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let app_context = SyscallContext::kernel(app);

    let queue_fd = match surface
        .dispatch(context.clone(), Syscall::CreateSleepQueue { owner: app })
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    surface.runtime.tick().unwrap();
    surface.runtime.block_running().unwrap();
    surface.runtime.tick().unwrap();
    match surface
        .dispatch(
            app_context,
            Syscall::SleepOnQueueDescriptor {
                owner: app,
                fd: queue_fd,
                channel: 0x91,
                priority: 7,
                timeout_ticks: Some(1),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcessBlockedOnSleepQueue(pid) => assert_eq!(pid, app),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface.runtime.tick().unwrap();
    match surface
        .dispatch(context, Syscall::InspectSleepResult { pid: app })
        .unwrap()
    {
        SyscallResult::SleepResultInspected(result) => {
            assert_eq!(result, Some(SleepWaitResult::TimedOut))
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_update_memory_words() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let mapped = surface
        .runtime
        .map_anonymous_memory(app, 0x1000, true, true, false, "wordops")
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    match surface
        .dispatch(
            context.clone(),
            Syscall::UpdateMemoryWord(UpdateMemoryWord {
                pid: app,
                addr: mapped,
                op: MemoryWordUpdateOp::Set(9),
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryWordUpdated { old, new } => assert_eq!((old, new), (0, 9)),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::UpdateMemoryWord(UpdateMemoryWord {
                pid: app,
                addr: mapped,
                op: MemoryWordUpdateOp::Add(3),
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryWordUpdated { old, new } => assert_eq!((old, new), (9, 12)),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::LoadMemoryWord(LoadMemoryWord {
                pid: app,
                addr: mapped,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryWordLoaded(value) => assert_eq!(value, 12),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::CompareMemoryWord(CompareMemoryWord {
                pid: app,
                addr: mapped,
                expected: 12,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryWordCompared { expected, observed } => {
            assert_eq!((expected, observed), (12, 12))
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn runtime_can_modify_and_remove_event_watches() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue-mod", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(24_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(24_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/mod.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = runtime.open_path(owner, "/run/mod.sock").unwrap();
    let queue = runtime
        .create_event_queue(owner, EventQueueMode::Kqueue)
        .unwrap();

    runtime
        .watch_event(
            owner,
            queue,
            fd,
            1,
            ReadinessInterest {
                readable: false,
                writable: true,
                priority: false,
            },
            EventWatchBehavior::LEVEL,
        )
        .unwrap();
    runtime
        .modify_watched_event(
            owner,
            queue,
            fd,
            2,
            ReadinessInterest {
                readable: true,
                writable: false,
                priority: false,
            },
            EventWatchBehavior::EDGE,
        )
        .unwrap();

    let ready = runtime.wait_event_queue(owner, queue).unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].token, 2);
    assert!(ready[0].events.contains(IoPollEvents::READABLE));
    assert!(!ready[0].events.contains(IoPollEvents::WRITABLE));

    runtime.remove_watched_event(owner, queue, fd).unwrap();
    let empty = runtime.wait_event_queue(owner, queue).unwrap();
    assert!(empty.is_empty());
}

#[test]
fn runtime_edge_triggered_watch_emits_only_on_state_change() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("edge", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(24_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(24_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/edge.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = runtime.open_path(owner, "/run/edge.sock").unwrap();
    let queue = runtime
        .create_event_queue(owner, EventQueueMode::Epoll)
        .unwrap();

    runtime
        .watch_event(
            owner,
            queue,
            fd,
            300,
            ReadinessInterest {
                readable: true,
                writable: false,
                priority: false,
            },
            EventWatchBehavior::EDGE,
        )
        .unwrap();

    let first = runtime.wait_event_queue(owner, queue).unwrap();
    assert_eq!(first.len(), 1);
    let second = runtime.wait_event_queue(owner, queue).unwrap();
    assert!(second.is_empty());

    let _ = runtime.read_io(owner, fd, 4096).unwrap();
    let drained = runtime.wait_event_queue(owner, queue).unwrap();
    assert!(drained.is_empty());

    let _ = runtime.write_io(owner, fd, b":wake").unwrap();
    let third = runtime.wait_event_queue(owner, queue).unwrap();
    assert_eq!(third.len(), 1);
    assert_eq!(third[0].token, 300);
    assert!(third[0].events.contains(IoPollEvents::READABLE));
}

#[test]
fn runtime_exposes_filedesc_entries_and_closefrom() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("fds", None, SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(24_200), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "asset",
        )
        .unwrap();

    let fd0 = runtime
        .open_descriptor(owner, cap, ObjectKind::File, "/data/a")
        .unwrap();
    let fd1 = runtime
        .open_descriptor(owner, cap, ObjectKind::Socket, "/run/b")
        .unwrap();
    let fd2 = runtime.duplicate_descriptor(owner, fd1).unwrap();
    runtime.set_descriptor_cloexec(owner, fd2, true).unwrap();

    let entries = runtime.filedesc_entries(owner).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].fd, fd0);
    assert_eq!(entries[1].fd, fd1);
    assert_eq!(entries[2].fd, fd2);
    assert!(entries[2].flags.cloexec);
    assert_eq!(entries[0].kind_code, 1);
    assert!(entries[0].readable);
    assert!(entries[0].writable);
    assert!(entries[1].size > 0);

    let closed = runtime.close_from(owner, Descriptor::new(1)).unwrap();
    assert_eq!(closed.len(), 2);
    let remaining = runtime.filedesc_entries(owner).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].fd, fd0);
}

#[test]
fn runtime_supports_close_range_cloexec_mode() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("close-range", None, SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(24_300), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "asset",
        )
        .unwrap();

    let _fd0 = runtime
        .open_descriptor(owner, cap, ObjectKind::File, "/data/a")
        .unwrap();
    let fd1 = runtime
        .open_descriptor(owner, cap, ObjectKind::Socket, "/run/b")
        .unwrap();
    let fd2 = runtime.duplicate_descriptor(owner, fd1).unwrap();

    let marked = runtime
        .close_range(
            owner,
            Descriptor::new(1),
            Some(fd2),
            CloseRangeMode::Cloexec,
        )
        .unwrap();
    assert_eq!(marked.len(), 2);

    let entries = runtime.filedesc_entries(owner).unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries[1].flags.cloexec);
    assert!(entries[2].flags.cloexec);
}

#[test]
fn runtime_supports_fdcopy_and_fdshare_semantics() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let parent = runtime
        .spawn_process("parent", None, SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            parent,
            ObjectHandle::new(Handle::new(24_400), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "shared",
        )
        .unwrap();
    let fd = runtime
        .open_descriptor(parent, cap, ObjectKind::Socket, "/run/share.sock")
        .unwrap();

    let copy = runtime
        .spawn_process_copy_fds("copy", Some(parent), SchedulerClass::Interactive, parent)
        .unwrap();
    let share = runtime
        .spawn_process_share_fds("share", Some(parent), SchedulerClass::Interactive, parent)
        .unwrap();

    assert_eq!(runtime.filedesc_entries(copy).unwrap().len(), 1);
    assert_eq!(runtime.filedesc_entries(share).unwrap().len(), 1);

    runtime.close_descriptor(parent, fd).unwrap();

    assert!(runtime.filedesc_entries(parent).unwrap().is_empty());
    assert_eq!(runtime.filedesc_entries(copy).unwrap().len(), 1);
    assert!(runtime.filedesc_entries(share).unwrap().is_empty());
}

#[test]
fn runtime_can_spawn_processes_with_combined_vm_and_filedesc_modes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let parent = runtime
        .spawn_process("parent", None, SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            parent,
            ObjectHandle::new(Handle::new(24_450), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "shared",
        )
        .unwrap();
    let fd = runtime
        .open_descriptor(parent, cap, ObjectKind::Socket, "/run/combined.sock")
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x2000, true, true, false, "combined-vm")
        .unwrap();

    let copy = runtime
        .spawn_process_from_source(
            "fork-copy",
            Some(parent),
            SchedulerClass::Interactive,
            parent,
            SpawnFiledescMode::Copy,
            SpawnVmMode::Copy,
        )
        .unwrap();
    let share = runtime
        .spawn_process_from_source(
            "fork-share",
            Some(parent),
            SchedulerClass::Interactive,
            parent,
            SpawnFiledescMode::Share,
            SpawnVmMode::Copy,
        )
        .unwrap();

    let copy_info = runtime.process_info(copy).unwrap();
    let share_info = runtime.process_info(share).unwrap();
    assert!(copy_info.copy_on_write_region_count >= 1);
    assert!(share_info.copy_on_write_region_count >= 1);
    assert_eq!(runtime.filedesc_entries(copy).unwrap().len(), 1);
    assert_eq!(runtime.filedesc_entries(share).unwrap().len(), 1);

    runtime.close_descriptor(parent, fd).unwrap();

    assert!(runtime.filedesc_entries(parent).unwrap().is_empty());
    assert_eq!(runtime.filedesc_entries(copy).unwrap().len(), 1);
    assert!(runtime.filedesc_entries(share).unwrap().is_empty());

    let touch = runtime.touch_memory(copy, scratch, 0x1000, true).unwrap();
    assert_eq!(touch.cow_faulted_pages, 1);
    let share_touch = runtime.touch_memory(share, scratch, 0x1000, true).unwrap();
    assert_eq!(share_touch.cow_faulted_pages, 1);
}

#[test]
fn runtime_exports_kinfo_file_entries() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let parent = runtime
        .spawn_process("kinfo", None, SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            parent,
            ObjectHandle::new(Handle::new(24_500), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "shared",
        )
        .unwrap();
    let fd = runtime
        .open_descriptor(parent, cap, ObjectKind::Socket, "/run/kinfo.sock")
        .unwrap();
    let shared = runtime
        .spawn_process_share_fds(
            "kinfo-share",
            Some(parent),
            SchedulerClass::Interactive,
            parent,
        )
        .unwrap();
    let _ = runtime.write_io(parent, fd, b":hello").unwrap();

    let entries = runtime.kinfo_file_entries(parent).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].fd, fd);
    assert_eq!(entries[0].kind_code, 2);
    assert_eq!(entries[0].ref_count, 2);
    assert_eq!(entries[0].socket_domain, Some(1));
    assert_eq!(entries[0].socket_type, Some(1));
    assert!(entries[0].size > 0);

    let shared_entries = runtime.kinfo_file_entries(shared).unwrap();
    assert_eq!(shared_entries.len(), 1);
    assert_eq!(shared_entries[0].ref_count, 2);
}

#[test]
fn syscall_surface_exposes_event_queues() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(23_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(23_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/run/render.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = surface.runtime.open_path(app, "/run/render.sock").unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let queue_fd = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateEventQueue {
                owner: app,
                mode: EventQueueMode::Kqueue,
            },
        )
        .unwrap()
    {
        SyscallResult::QueueDescriptorCreated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::WatchEventDescriptor {
                owner: app,
                queue_fd,
                fd,
                token: 99,
                interest: ReadinessInterest {
                    readable: true,
                    writable: true,
                    priority: false,
                },
                behavior: EventWatchBehavior::LEVEL,
            },
        )
        .unwrap()
    {
        SyscallResult::EventWatchRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::WaitEventQueueDescriptor {
                owner: app,
                fd: queue_fd,
            },
        )
        .unwrap()
    {
        SyscallResult::EventQueueReady(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token, 99);
            assert_eq!(events[0].mode, EventQueueMode::Kqueue);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}
