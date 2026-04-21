use super::*;
#[test]
fn kernel_state_bootstrap_marks_handles_ready() {
    let state = KernelState::bootstrap(KernelConfig::host_runtime(Architecture::X86_64));
    assert!(state.handles_ready);
}

#[test]
fn kernel_state_bootstrap_marks_vm_ready() {
    let state = KernelState::bootstrap(KernelConfig::host_runtime(Architecture::X86_64));
    assert!(state.vm_ready);
}

#[test]
fn kernel_state_bootstrap_marks_vfs_ready() {
    let state = KernelState::bootstrap(KernelConfig::host_runtime(Architecture::X86_64));
    assert!(state.vfs_ready);
}

#[test]
fn runtime_policy_scheduler_topology_updates_cpu_count() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.apply_scheduler_cpu_topology(vec![
        SchedulerCpuTopologyEntry {
            apic_id: 17,
            package_id: 0,
            core_group: 0,
            sibling_group: 0,
            inferred: false,
        },
        SchedulerCpuTopologyEntry {
            apic_id: 29,
            package_id: 0,
            core_group: 0,
            sibling_group: 1,
            inferred: false,
        },
    ]);

    assert_eq!(policy.scheduler_logical_cpu_count, 2);
    assert_eq!(policy.scheduler_cpu_topology.len(), 2);
    assert_eq!(policy.scheduler_cpu_topology[0].apic_id, 17);
    assert!(!policy.scheduler_cpu_topology[1].inferred);
}

#[test]
fn handle_space_allocates_in_order_and_reuses_released_handles() {
    let mut handles = HandleSpace::new(10, 14);
    let a = handles.allocate().unwrap();
    let b = handles.allocate().unwrap();
    assert_eq!(a.raw(), 10);
    assert_eq!(b.raw(), 11);
    handles.release(a).unwrap();
    let c = handles.allocate().unwrap();
    assert_eq!(c.raw(), 10);
}

#[test]
fn handle_space_can_reserve_specific_handle() {
    let mut handles = HandleSpace::new(100, 104);
    handles.reserve(Handle::new(102)).unwrap();
    assert!(!handles.is_allocated(Handle::new(100)));
    assert!(handles.is_allocated(Handle::new(102)));
    assert_eq!(handles.allocate().unwrap().raw(), 100);
    assert_eq!(handles.allocate().unwrap().raw(), 101);
    assert_eq!(handles.allocate().unwrap().raw(), 103);
}

#[test]
fn handle_space_reports_invalid_release() {
    let mut handles = HandleSpace::new(1, 3);
    assert_eq!(
        handles.release(Handle::new(1)),
        Err(HandleError::AlreadyFree)
    );
    assert_eq!(
        handles.reserve(Handle::new(9)),
        Err(HandleError::OutOfRange)
    );
}

#[test]
fn handle_space_exhaustion_is_explicit() {
    let mut handles = HandleSpace::new(1, 2);
    assert_eq!(handles.allocate().unwrap().raw(), 1);
    assert_eq!(handles.allocate(), Err(HandleError::Exhausted));
}

#[test]
fn object_table_insert_lookup_and_remove_work() {
    let mut table = KernelObjectTable::new(10, 20);
    let handle = table.insert(String::from("init")).unwrap();
    assert_eq!(table.get(handle).unwrap(), "init");
    assert!(table.contains(handle));
    assert_eq!(table.remove(handle).unwrap(), "init");
    assert!(!table.contains(handle));
}

#[test]
fn object_table_detects_stale_handles_after_reuse() {
    let mut table = KernelObjectTable::new(1, 3);
    let first = table.insert(111u32).unwrap();
    assert_eq!(table.remove(first).unwrap(), 111);

    let second = table.insert(222u32).unwrap();
    assert_eq!(first.id(), second.id());
    assert_ne!(first.generation(), second.generation());
    assert_eq!(table.get(first), Err(ObjectError::StaleHandle));
    assert_eq!(table.get(second), Ok(&222));
}

#[test]
fn object_table_mutation_is_checked_by_generation() {
    let mut table = KernelObjectTable::new(5, 8);
    let handle = table.insert(String::from("proc")).unwrap();
    table.get_mut(handle).unwrap().push('0');
    assert_eq!(table.get(handle).unwrap(), "proc0");
}

#[test]
fn object_table_reports_exhaustion() {
    let mut table = KernelObjectTable::<u32>::new(1, 2);
    let _ = table.insert(7).unwrap();
    assert_eq!(table.insert(8), Err(ObjectError::Exhausted));
}

#[test]
fn process_table_create_and_spawn_assign_typed_pids() {
    let mut processes = ProcessTable::new(100, 110);
    let init = processes.create("init", None).unwrap();
    assert_eq!(init.raw(), 100);
    assert_eq!(processes.get(init).unwrap().name(), "init");
    assert_eq!(processes.get(init).unwrap().state(), ProcessState::Created);

    let shell = processes.spawn("sh", Some(init)).unwrap();
    let shell_process = processes.get(shell).unwrap();
    assert_eq!(shell_process.parent(), Some(init));
    assert_eq!(shell_process.state(), ProcessState::Ready);
}

#[test]
fn process_table_enforces_state_transitions() {
    let mut processes = ProcessTable::new(1, 8);
    let pid = processes.spawn("worker", None).unwrap();
    assert_eq!(
        processes.set_state(pid, ProcessState::Created),
        Err(ProcessError::InvalidTransition {
            from: ProcessState::Ready,
            to: ProcessState::Created,
        })
    );
    assert_eq!(
        processes.set_state(pid, ProcessState::Running),
        Ok(ProcessState::Ready)
    );
    assert_eq!(
        processes.set_state(pid, ProcessState::Blocked),
        Ok(ProcessState::Running)
    );
    assert_eq!(
        processes.set_state(pid, ProcessState::Ready),
        Ok(ProcessState::Blocked)
    );
    assert_eq!(processes.ready_queue(), vec![pid]);
}

#[test]
fn process_table_exit_and_reap_reuse_pid_with_new_generation() {
    let mut processes = ProcessTable::new(10, 12);
    let old = processes.spawn("daemon", None).unwrap();
    processes.exit(old, 17).unwrap();
    let reaped = processes.reap(old).unwrap();
    assert_eq!(reaped.exit_code(), Some(17));
    assert!(!processes.contains(old));

    let new = processes.spawn("daemon-new", None).unwrap();
    assert_eq!(old.raw(), new.raw());
    assert_ne!(old.generation(), new.generation());
    assert_eq!(processes.get(old), Err(ProcessError::StalePid));
    assert_eq!(processes.get(new).unwrap().name(), "daemon-new");
}

#[test]
fn process_table_requires_exited_state_before_reap() {
    let mut processes = ProcessTable::new(1, 4);
    let pid = processes.spawn("service", None).unwrap();
    assert_eq!(processes.reap(pid), Err(ProcessError::NotExited));
}

#[test]
fn capability_table_grants_and_queries_rights() {
    let mut processes = ProcessTable::new(1, 8);
    let owner = processes.spawn("init", None).unwrap();
    let target = ObjectHandle::new(Handle::new(900), 3);
    let mut capabilities = CapabilityTable::new(100, 110);

    let id = capabilities
        .grant(
            &processes,
            owner,
            target,
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root-fs",
        )
        .unwrap();

    let capability = capabilities.require(id, CapabilityRights::READ).unwrap();
    assert_eq!(capability.owner(), owner);
    assert_eq!(capability.target(), target);
    assert_eq!(capability.label(), "root-fs");
}

#[test]
fn capability_table_denies_missing_rights() {
    let mut processes = ProcessTable::new(1, 8);
    let owner = processes.spawn("init", None).unwrap();
    let mut capabilities = CapabilityTable::new(10, 20);
    let id = capabilities
        .grant(
            &processes,
            owner,
            ObjectHandle::new(Handle::new(42), 0),
            CapabilityRights::READ,
            "readonly",
        )
        .unwrap();

    assert_eq!(
        capabilities.require(id, CapabilityRights::WRITE),
        Err(CapabilityError::RightDenied {
            required: CapabilityRights::WRITE,
            actual: CapabilityRights::READ,
        })
    );
}

#[test]
fn capability_table_duplicates_with_restricted_rights_only() {
    let mut processes = ProcessTable::new(1, 8);
    let init = processes.spawn("init", None).unwrap();
    let child = processes.spawn("child", Some(init)).unwrap();
    let mut capabilities = CapabilityTable::new(100, 110);
    let root = capabilities
        .grant(
            &processes,
            init,
            ObjectHandle::new(Handle::new(500), 1),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "bus",
        )
        .unwrap();

    let derived = capabilities
        .duplicate_restricted(
            root,
            child,
            CapabilityRights::READ,
            "bus-readonly",
            &processes,
        )
        .unwrap();

    let derived_cap = capabilities.get(derived).unwrap();
    assert_eq!(derived_cap.owner(), child);
    assert_eq!(derived_cap.rights(), CapabilityRights::READ);
    assert_eq!(capabilities.by_owner(child), vec![derived]);
}

#[test]
fn capability_table_rejects_invalid_owners_and_reuses_generation_safely() {
    let mut processes = ProcessTable::new(1, 4);
    let init = processes.spawn("init", None).unwrap();
    let mut capabilities = CapabilityTable::new(20, 22);

    processes.exit(init, 0).unwrap();
    let dead = processes.reap(init).unwrap().pid();
    assert_eq!(
        capabilities.grant(
            &processes,
            dead,
            ObjectHandle::new(Handle::new(7), 0),
            CapabilityRights::READ,
            "dead-owner",
        ),
        Err(CapabilityError::InvalidOwner)
    );

    let live = processes.spawn("new-init", None).unwrap();
    let first = capabilities
        .grant(
            &processes,
            live,
            ObjectHandle::new(Handle::new(77), 0),
            CapabilityRights::READ,
            "one",
        )
        .unwrap();
    let _ = capabilities.revoke(first).unwrap();
    let second = capabilities
        .grant(
            &processes,
            live,
            ObjectHandle::new(Handle::new(78), 0),
            CapabilityRights::READ,
            "two",
        )
        .unwrap();
    assert_eq!(first.raw(), second.raw());
    assert_ne!(first.generation(), second.generation());
    assert_eq!(
        capabilities.get(first),
        Err(CapabilityError::StaleCapability)
    );
}

#[test]
fn scheduler_picks_higher_priority_classes_first() {
    let mut processes = ProcessTable::new(1, 16);
    let bg = processes.spawn("bg", None).unwrap();
    let ui = processes.spawn("ui", None).unwrap();
    let mut scheduler = Scheduler::new(2);

    scheduler
        .enqueue(&mut processes, bg, SchedulerClass::Background)
        .unwrap();
    scheduler
        .enqueue(&mut processes, ui, SchedulerClass::Interactive)
        .unwrap();

    let first = scheduler.tick(&mut processes).unwrap();
    assert_eq!(first.pid, ui);
    assert_eq!(first.class, SchedulerClass::Interactive);
    assert_eq!(processes.get(ui).unwrap().state(), ProcessState::Running);
}

#[test]
fn scheduler_rotates_process_after_budget_expires() {
    let mut processes = ProcessTable::new(1, 16);
    let a = processes.spawn("a", None).unwrap();
    let b = processes.spawn("b", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, a, SchedulerClass::BestEffort)
        .unwrap();
    scheduler
        .enqueue(&mut processes, b, SchedulerClass::BestEffort)
        .unwrap();

    assert_eq!(scheduler.tick(&mut processes).unwrap().pid, a);
    assert_eq!(scheduler.tick(&mut processes).unwrap().pid, b);
    assert_eq!(processes.get(a).unwrap().state(), ProcessState::Ready);
    assert_eq!(processes.get(b).unwrap().state(), ProcessState::Running);
}

#[test]
fn scheduler_prevents_background_starvation_under_interactive_pressure() {
    let mut processes = ProcessTable::new(1, 16);
    let bg = processes.spawn("bg", None).unwrap();
    let ui_a = processes.spawn("ui-a", None).unwrap();
    let ui_b = processes.spawn("ui-b", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, bg, SchedulerClass::Background)
        .unwrap();
    scheduler
        .enqueue(&mut processes, ui_a, SchedulerClass::Interactive)
        .unwrap();
    scheduler
        .enqueue(&mut processes, ui_b, SchedulerClass::Interactive)
        .unwrap();

    let mut saw_background = false;
    for _ in 0..12 {
        let scheduled = scheduler.tick(&mut processes).unwrap();
        if scheduled.pid == bg {
            saw_background = true;
            break;
        }
    }

    assert!(saw_background, "background queue should make progress");
}

#[test]
fn scheduler_uses_lag_debt_to_avoid_interactive_token_domination() {
    let mut processes = ProcessTable::new(1, 16);
    let ui_a = processes.spawn("ui-a", None).unwrap();
    let ui_b = processes.spawn("ui-b", None).unwrap();
    let be = processes.spawn("be", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, ui_a, SchedulerClass::Interactive)
        .unwrap();
    scheduler
        .enqueue(&mut processes, ui_b, SchedulerClass::Interactive)
        .unwrap();
    scheduler
        .enqueue(&mut processes, be, SchedulerClass::BestEffort)
        .unwrap();

    let first = scheduler.tick(&mut processes).unwrap();
    assert_eq!(first.class, SchedulerClass::Interactive);

    let second = scheduler.tick(&mut processes).unwrap();
    assert_eq!(second.pid, be);
    assert_eq!(second.class, SchedulerClass::BestEffort);
    assert!(
        scheduler.class_lag_debt()[SchedulerClass::BestEffort.index()]
            <= scheduler.class_lag_debt()[SchedulerClass::Interactive.index()]
    );
}

#[test]
fn scheduler_tracks_service_distribution_by_class() {
    let mut processes = ProcessTable::new(1, 16);
    let ui = processes.spawn("ui", None).unwrap();
    let bg = processes.spawn("bg", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, ui, SchedulerClass::Interactive)
        .unwrap();
    scheduler
        .enqueue(&mut processes, bg, SchedulerClass::Background)
        .unwrap();

    let mut saw_background_dispatch = false;
    let mut saw_background_runtime = false;
    for _ in 0..8 {
        let scheduled = scheduler.tick(&mut processes).unwrap();
        if scheduled.class == SchedulerClass::Background {
            saw_background_dispatch = true;
        }
        let runtime_ticks = scheduler.class_runtime_ticks();
        if runtime_ticks[SchedulerClass::Background.index()] > 0 {
            saw_background_runtime = true;
            break;
        }
    }

    let dispatches = scheduler.class_dispatch_counts();
    let runtime_ticks = scheduler.class_runtime_ticks();
    assert!(dispatches[SchedulerClass::Interactive.index()] >= 1);
    assert!(saw_background_dispatch);
    assert!(dispatches[SchedulerClass::Background.index()] >= 1);
    assert!(runtime_ticks[SchedulerClass::Interactive.index()] >= 1);
    assert!(saw_background_runtime);
    assert!(runtime_ticks[SchedulerClass::Background.index()] >= 1);
}

#[test]
fn scheduler_reschedules_quickly_when_higher_priority_work_arrives() {
    let mut processes = ProcessTable::new(1, 16);
    let bg = processes.spawn("bg", None).unwrap();
    let ui = processes.spawn("ui", None).unwrap();
    let mut scheduler = Scheduler::new(4);

    scheduler
        .enqueue(&mut processes, bg, SchedulerClass::Background)
        .unwrap();
    let first = scheduler.tick(&mut processes).unwrap();
    assert_eq!(first.pid, bg);
    assert_eq!(first.budget, 4);

    scheduler
        .enqueue(&mut processes, ui, SchedulerClass::Interactive)
        .unwrap();

    let resumed = scheduler.running().unwrap().clone();
    assert_eq!(resumed.pid, bg);
    assert_eq!(resumed.budget, 1);

    let next = scheduler.tick(&mut processes).unwrap();
    assert_eq!(next.pid, ui);
    assert_eq!(next.class, SchedulerClass::Interactive);
}

#[test]
fn scheduler_prioritizes_woken_work_within_the_same_class() {
    let mut processes = ProcessTable::new(1, 16);
    let a = processes.spawn("a", None).unwrap();
    let b = processes.spawn("b", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, a, SchedulerClass::Interactive)
        .unwrap();
    scheduler
        .enqueue(&mut processes, b, SchedulerClass::Interactive)
        .unwrap();

    assert_eq!(scheduler.tick(&mut processes).unwrap().pid, a);
    assert_eq!(scheduler.block_running(&mut processes).unwrap(), a);

    scheduler
        .wake(&mut processes, a, SchedulerClass::Interactive)
        .unwrap();

    let queued = scheduler.queued_threads_by_class();
    assert_eq!(
        queued[SchedulerClass::Interactive.index()],
        vec![
            processes.get(a).unwrap().main_thread().unwrap(),
            processes.get(b).unwrap().main_thread().unwrap()
        ]
    );

    let next = scheduler.tick(&mut processes).unwrap();
    assert_eq!(next.pid, a);
    assert_eq!(next.class, SchedulerClass::Interactive);
    assert_eq!(scheduler.queued_urgent_len_by_class(), [0, 0, 0, 0]);
}

#[test]
fn scheduler_exposes_urgent_queue_policy_state() {
    let mut processes = ProcessTable::new(1, 16);
    let a = processes.spawn("a", None).unwrap();
    let b = processes.spawn("b", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, a, SchedulerClass::Interactive)
        .unwrap();
    scheduler
        .enqueue(&mut processes, b, SchedulerClass::Interactive)
        .unwrap();
    assert_eq!(scheduler.tick(&mut processes).unwrap().pid, a);
    assert_eq!(scheduler.block_running(&mut processes).unwrap(), a);
    scheduler
        .wake(&mut processes, a, SchedulerClass::Interactive)
        .unwrap();

    assert_eq!(scheduler.queued_len_by_class(), [0, 2, 0, 0]);
    assert_eq!(scheduler.queued_urgent_len_by_class(), [0, 1, 0, 0]);
    assert_eq!(scheduler.starved_classes(), [false, false, false, false]);
    assert_eq!(scheduler.starvation_guard_ticks(), 8);
}

#[test]
fn scheduler_can_block_and_wake_processes() {
    let mut processes = ProcessTable::new(1, 16);
    let pid = processes.spawn("io", None).unwrap();
    let mut scheduler = Scheduler::new(2);

    scheduler
        .enqueue(&mut processes, pid, SchedulerClass::BestEffort)
        .unwrap();
    assert_eq!(scheduler.tick(&mut processes).unwrap().pid, pid);
    assert_eq!(scheduler.block_running(&mut processes).unwrap(), pid);
    assert_eq!(processes.get(pid).unwrap().state(), ProcessState::Blocked);

    scheduler
        .wake(&mut processes, pid, SchedulerClass::LatencyCritical)
        .unwrap();
    let resumed = scheduler.tick(&mut processes).unwrap();
    assert_eq!(resumed.pid, pid);
    assert_eq!(resumed.class, SchedulerClass::LatencyCritical);
    let decisions = scheduler.recent_decisions();
    assert!(decisions.iter().any(|entry| {
        entry.agent == SchedulerAgentKind::EnqueueAgent && entry.pid == pid.raw()
    }));
    assert!(
        decisions.iter().any(|entry| {
            entry.agent == SchedulerAgentKind::BlockAgent && entry.pid == pid.raw()
        })
    );
    assert!(
        decisions.iter().any(|entry| {
            entry.agent == SchedulerAgentKind::WakeAgent && entry.pid == pid.raw()
        })
    );
    assert!(decisions.iter().any(|entry| {
        entry.agent == SchedulerAgentKind::TickAgent && entry.pid == pid.raw() && entry.detail0 == 3
    }));
}

#[test]
fn scheduler_records_rotation_and_rebind_decisions() {
    let mut processes = ProcessTable::new(1, 16);
    let a = processes.spawn("a", None).unwrap();
    let b = processes.spawn("b", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, a, SchedulerClass::BestEffort)
        .unwrap();
    scheduler
        .enqueue(&mut processes, b, SchedulerClass::BestEffort)
        .unwrap();
    let _ = scheduler.tick(&mut processes).unwrap();
    scheduler
        .rebind_process(&processes, b, SchedulerClass::LatencyCritical, 4)
        .unwrap();
    let _ = scheduler.tick(&mut processes).unwrap();

    let decisions = scheduler.recent_decisions();
    assert!(decisions.iter().any(|entry| {
        entry.agent == SchedulerAgentKind::TickAgent && entry.pid == a.raw() && entry.detail0 == 2
    }));
    assert!(decisions.iter().any(|entry| {
        entry.agent == SchedulerAgentKind::RebindAgent
            && entry.pid == b.raw()
            && entry.class == SchedulerClass::LatencyCritical.index() as u64
            && entry.detail0 == 4
    }));
}

#[test]
fn scheduler_rebind_updates_visible_queue_membership_without_duplicates() {
    let mut processes = ProcessTable::new(1, 16);
    let pid = processes.spawn("moved", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, pid, SchedulerClass::BestEffort)
        .unwrap();
    scheduler
        .rebind_process(&processes, pid, SchedulerClass::Interactive, 3)
        .unwrap();

    let queued = scheduler.queued_threads_by_class();
    assert!(queued[SchedulerClass::BestEffort.index()].is_empty());
    assert_eq!(
        queued[SchedulerClass::Interactive.index()],
        vec![processes.get(pid).unwrap().main_thread().unwrap()]
    );
    assert_eq!(scheduler.queued_len_by_class(), [0, 1, 0, 0]);
}

#[test]
fn scheduler_rejects_duplicate_or_invalid_queueing() {
    let mut processes = ProcessTable::new(1, 16);
    let pid = processes.spawn("dup", None).unwrap();
    let mut scheduler = Scheduler::new(1);

    scheduler
        .enqueue(&mut processes, pid, SchedulerClass::BestEffort)
        .unwrap();
    assert_eq!(
        scheduler.enqueue(&mut processes, pid, SchedulerClass::Background),
        Err(SchedulerError::DuplicateProcess)
    );

    processes.exit(pid, 0).unwrap();
    let dead = processes.reap(pid).unwrap().pid();
    assert_eq!(
        scheduler.enqueue(&mut processes, dead, SchedulerClass::BestEffort),
        Err(SchedulerError::InvalidPid)
    );
}

#[test]
fn scheduler_reports_queue_capacity_exhaustion_explicitly() {
    let mut processes = ProcessTable::new(1, (Scheduler::QUEUE_CAPACITY as u64) + 4);
    let mut scheduler = Scheduler::new(1);

    for index in 0..Scheduler::QUEUE_CAPACITY {
        let pid = processes.spawn(format!("p{index}"), None).unwrap();
        scheduler
            .enqueue(&mut processes, pid, SchedulerClass::BestEffort)
            .unwrap();
    }

    let overflow = processes.spawn("overflow", None).unwrap();
    assert_eq!(
        scheduler.enqueue(&mut processes, overflow, SchedulerClass::BestEffort),
        Err(SchedulerError::QueueFull)
    );
}

#[test]
fn scheduler_balances_threads_across_logical_cpus_and_respects_affinity() {
    let mut processes = ProcessTable::new(1, 16);
    let a = processes.spawn("a", None).unwrap();
    let b = processes.spawn("b", None).unwrap();
    let c = processes.spawn("c", None).unwrap();
    let a_tid = processes.get(a).unwrap().main_thread().unwrap();
    let b_tid = processes.get(b).unwrap().main_thread().unwrap();
    let c_tid = processes.get(c).unwrap().main_thread().unwrap();
    let mut scheduler = Scheduler::new_with_cpus(1, 2);

    scheduler.set_thread_affinity(a_tid, 0b01).unwrap();
    scheduler.set_thread_affinity(b_tid, 0b10).unwrap();
    scheduler
        .enqueue(&mut processes, a, SchedulerClass::BestEffort)
        .unwrap();
    scheduler
        .enqueue(&mut processes, b, SchedulerClass::BestEffort)
        .unwrap();
    scheduler
        .enqueue(&mut processes, c, SchedulerClass::BestEffort)
        .unwrap();

    assert_eq!(scheduler.cpu_queued_loads(), &[2, 1]);
    assert_eq!(scheduler.thread_assignment(a_tid), Some((0, 0b01)));
    assert_eq!(scheduler.thread_assignment(b_tid), Some((1, 0b10)));
    assert_eq!(
        scheduler.thread_assignment(c_tid).map(|entry| entry.0),
        Some(0)
    );
}

#[test]
fn scheduler_rejects_empty_affinity_and_recovers_with_valid_cpu_mask() {
    let mut processes = ProcessTable::new(1, 16);
    let pid = processes.spawn("affinity", None).unwrap();
    let tid = processes.get(pid).unwrap().main_thread().unwrap();
    let mut scheduler = Scheduler::new_with_cpus(1, 2);

    assert_eq!(
        scheduler.set_thread_affinity(tid, 0),
        Err(SchedulerError::InvalidCpuAffinity)
    );

    scheduler.set_thread_affinity(tid, 0b10).unwrap();
    scheduler
        .enqueue(&mut processes, pid, SchedulerClass::Interactive)
        .unwrap();
    assert_eq!(scheduler.thread_assignment(tid), Some((1, 0b10)));
    assert_eq!(scheduler.cpu_queued_loads(), &[0, 1]);
}

#[test]
fn scheduler_rebalances_queued_threads_when_affinity_allows_migration() {
    let mut processes = ProcessTable::new(1, 16);
    let a = processes.spawn("a", None).unwrap();
    let b = processes.spawn("b", None).unwrap();
    let c = processes.spawn("c", None).unwrap();
    let d = processes.spawn("d", None).unwrap();
    let mut scheduler = Scheduler::new_with_cpus(1, 2);

    for pid in [a, b, c, d] {
        scheduler
            .set_thread_affinity(processes.get(pid).unwrap().main_thread().unwrap(), 0b01)
            .unwrap();
        scheduler
            .enqueue(&mut processes, pid, SchedulerClass::BestEffort)
            .unwrap();
    }
    scheduler
        .set_thread_affinity(processes.get(d).unwrap().main_thread().unwrap(), 0b11)
        .unwrap();
    let d_tid = processes.get(d).unwrap().main_thread().unwrap();

    assert_eq!(scheduler.cpu_queued_loads(), &[4, 0]);
    let running = scheduler.tick(&mut processes).unwrap();
    assert_eq!(scheduler.thread_assignment(d_tid), Some((1, 0b11)));
    assert_eq!(scheduler.cpu_queued_loads().iter().sum::<usize>(), 3);
    assert!(running.cpu == 1 || scheduler.cpu_queued_loads()[1] == 1);
    assert!(scheduler.rebalance_migrations() >= 1);
    assert!(scheduler.last_rebalance_migrations() >= 1);
}

#[test]
fn scheduler_does_not_rebalance_when_affinity_forbids_other_cpu() {
    let mut processes = ProcessTable::new(1, 16);
    let a = processes.spawn("a", None).unwrap();
    let b = processes.spawn("b", None).unwrap();
    let c = processes.spawn("c", None).unwrap();
    let d = processes.spawn("d", None).unwrap();
    let mut scheduler = Scheduler::new_with_cpus(1, 2);
    for pid in [a, b, c, d] {
        scheduler
            .set_thread_affinity(processes.get(pid).unwrap().main_thread().unwrap(), 0b01)
            .unwrap();
        scheduler
            .enqueue(&mut processes, pid, SchedulerClass::BestEffort)
            .unwrap();
    }

    assert_eq!(scheduler.cpu_queued_loads(), &[4, 0]);
    let _ = scheduler.tick(&mut processes).unwrap();
    assert_eq!(scheduler.cpu_queued_loads()[1], 0);
    assert_eq!(scheduler.last_rebalance_migrations(), 0);
}

#[test]
fn scheduler_exposes_per_cpu_class_queue_distribution() {
    let mut processes = ProcessTable::new(1, 16);
    let lc = processes.spawn("lc", None).unwrap();
    let ui = processes.spawn("ui", None).unwrap();
    let bg = processes.spawn("bg", None).unwrap();
    let mut scheduler = Scheduler::new_with_cpus(1, 2);

    let lc_tid = processes.get(lc).unwrap().main_thread().unwrap();
    let ui_tid = processes.get(ui).unwrap().main_thread().unwrap();
    let bg_tid = processes.get(bg).unwrap().main_thread().unwrap();

    scheduler.set_thread_affinity(lc_tid, 0b01).unwrap();
    scheduler.set_thread_affinity(ui_tid, 0b10).unwrap();
    scheduler.set_thread_affinity(bg_tid, 0b10).unwrap();

    scheduler
        .enqueue(&mut processes, lc, SchedulerClass::LatencyCritical)
        .unwrap();
    scheduler
        .enqueue(&mut processes, ui, SchedulerClass::Interactive)
        .unwrap();
    scheduler
        .enqueue(&mut processes, bg, SchedulerClass::Background)
        .unwrap();

    assert_eq!(
        scheduler.cpu_class_queued_loads(),
        vec![[1, 0, 0, 0], [0, 1, 0, 1]]
    );
    assert_eq!(
        scheduler
            .queued_threads_for_cpu_and_class(0, SchedulerClass::LatencyCritical)
            .into_iter()
            .map(|tid| tid.raw())
            .collect::<Vec<_>>(),
        vec![lc_tid.raw()]
    );
    assert_eq!(
        scheduler
            .queued_threads_for_cpu_and_class(1, SchedulerClass::Interactive)
            .into_iter()
            .map(|tid| tid.raw())
            .collect::<Vec<_>>(),
        vec![ui_tid.raw()]
    );
    assert_eq!(
        scheduler
            .queued_threads_for_cpu_and_class(1, SchedulerClass::Background)
            .into_iter()
            .map(|tid| tid.raw())
            .collect::<Vec<_>>(),
        vec![bg_tid.raw()]
    );
}
