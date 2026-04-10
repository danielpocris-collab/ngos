//! Canonical subsystem role:
//! - subsystem: scheduler
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical scheduler state machine and service policy
//!
//! Canonical contract families defined here:
//! - scheduling class contracts
//! - queue membership and dispatch contracts
//! - starvation / lag / fairness contracts
//! - scheduler observability source contracts
//!
//! This module may define scheduler truth for `ngos`. Other layers may inspect,
//! transport, or react to scheduler state, but they must not invent an
//! alternative scheduler semantic model.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchedulerClass {
    LatencyCritical,
    Interactive,
    BestEffort,
    Background,
}

impl SchedulerClass {
    pub(crate) const ALL: [Self; 4] = [
        Self::LatencyCritical,
        Self::Interactive,
        Self::BestEffort,
        Self::Background,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::LatencyCritical => 0,
            Self::Interactive => 1,
            Self::BestEffort => 2,
            Self::Background => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledProcess {
    pub pid: ProcessId,
    pub tid: ThreadId,
    pub class: SchedulerClass,
    pub budget: u32,
    pub cpu: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueueMembership {
    class: SchedulerClass,
    urgent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SchedulerThreadState {
    budget: u32,
    queued: Option<QueueMembership>,
    affinity_mask: u64,
    assigned_cpu: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidPid,
    DuplicateProcess,
    QueueFull,
    NotQueued,
    NoRunnableProcess,
    InvalidCpuAffinity,
    InvalidProcessState(ProcessState),
}

impl SchedulerError {
    pub(crate) fn from_process_error(error: ProcessError) -> Self {
        match error {
            ProcessError::InvalidPid | ProcessError::StalePid => Self::InvalidPid,
            ProcessError::InvalidTid | ProcessError::StaleTid => Self::InvalidPid,
            ProcessError::Exhausted => Self::InvalidPid,
            ProcessError::InvalidMemoryLayout => Self::InvalidPid,
            ProcessError::MemoryQuarantined { .. } => Self::InvalidPid,
            ProcessError::InvalidSignal => Self::InvalidPid,
            ProcessError::InvalidSessionReport => Self::InvalidPid,
            ProcessError::CpuExtendedStateUnavailable => Self::InvalidPid,
            ProcessError::InvalidTransition { from, .. } => Self::InvalidProcessState(from),
            ProcessError::NotExited => Self::InvalidProcessState(ProcessState::Ready),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheduler {
    queues: Vec<[BufRing<ThreadId>; 4]>,
    urgent_queues: Vec<[BufRing<ThreadId>; 4]>,
    queued_counts: [usize; 4],
    running: Option<ScheduledProcess>,
    default_budget: u32,
    class_dispatch_tokens: [u8; 4],
    class_wait_ticks: [u32; 4],
    class_lag_debt: [i32; 4],
    class_dispatch_counts: [u64; 4],
    class_runtime_ticks: [u64; 4],
    logical_cpu_count: usize,
    cpu_topology: Vec<SchedulerCpuTopologyEntry>,
    cpu_queued_loads: Vec<usize>,
    cpu_dispatch_counts: Vec<u64>,
    cpu_runtime_ticks: Vec<u64>,
    next_dispatch_cpu: usize,
    rebalance_operations: u64,
    rebalance_migrations: u64,
    last_rebalance_migrations: usize,
    thread_states: Vec<Option<SchedulerThreadState>>,
    decisions: Vec<SchedulerAgentDecisionRecord>,
    tick_counter: u64,
    decision_tracing_enabled: bool,
}

impl Scheduler {
    pub const QUEUE_CAPACITY: usize = 1 << 16;
    const STARVATION_GUARD_TICKS: u32 = 8;

    pub fn new(default_budget: u32) -> Self {
        Self::new_with_cpus(default_budget, 1)
    }

    pub fn new_with_cpus(default_budget: u32, logical_cpu_count: usize) -> Self {
        Self::new_with_topology(default_budget, logical_cpu_count, Vec::new())
    }

    pub fn new_with_topology(
        default_budget: u32,
        logical_cpu_count: usize,
        cpu_topology: Vec<SchedulerCpuTopologyEntry>,
    ) -> Self {
        assert!(default_budget > 0, "scheduler budget must be non-zero");
        assert!(
            logical_cpu_count > 0,
            "scheduler cpu count must be non-zero"
        );
        let cpu_topology = Self::normalize_cpu_topology(logical_cpu_count, cpu_topology);
        Self {
            queues: (0..logical_cpu_count)
                .map(|_| Self::new_queue_set())
                .collect(),
            urgent_queues: (0..logical_cpu_count)
                .map(|_| Self::new_queue_set())
                .collect(),
            queued_counts: [0; 4],
            running: None,
            default_budget,
            class_dispatch_tokens: Self::class_token_refill(),
            class_wait_ticks: [0; 4],
            class_lag_debt: [0; 4],
            class_dispatch_counts: [0; 4],
            class_runtime_ticks: [0; 4],
            logical_cpu_count,
            cpu_topology,
            cpu_queued_loads: vec![0; logical_cpu_count],
            cpu_dispatch_counts: vec![0; logical_cpu_count],
            cpu_runtime_ticks: vec![0; logical_cpu_count],
            next_dispatch_cpu: 0,
            rebalance_operations: 0,
            rebalance_migrations: 0,
            last_rebalance_migrations: 0,
            thread_states: Vec::new(),
            decisions: Vec::with_capacity(64),
            tick_counter: 0,
            decision_tracing_enabled: true,
        }
    }

    pub fn running(&self) -> Option<&ScheduledProcess> {
        self.running.as_ref()
    }

    pub fn default_budget(&self) -> u32 {
        self.default_budget
    }

    pub fn class_dispatch_tokens(&self) -> [u8; 4] {
        self.class_dispatch_tokens
    }

    pub fn class_wait_ticks(&self) -> [u32; 4] {
        self.class_wait_ticks
    }

    pub fn class_lag_debt(&self) -> [i32; 4] {
        self.class_lag_debt
    }

    pub fn class_dispatch_counts(&self) -> [u64; 4] {
        self.class_dispatch_counts
    }

    pub fn class_runtime_ticks(&self) -> [u64; 4] {
        self.class_runtime_ticks
    }

    pub fn logical_cpu_count(&self) -> usize {
        self.logical_cpu_count
    }

    pub fn cpu_queued_loads(&self) -> &[usize] {
        &self.cpu_queued_loads
    }

    pub fn cpu_dispatch_counts(&self) -> &[u64] {
        &self.cpu_dispatch_counts
    }

    pub fn cpu_runtime_ticks(&self) -> &[u64] {
        &self.cpu_runtime_ticks
    }

    pub fn cpu_apic_id(&self, cpu: usize) -> u32 {
        self.cpu_topology
            .get(cpu)
            .map(|entry| entry.apic_id)
            .unwrap_or(cpu as u32)
    }

    pub fn cpu_package_id(&self, cpu: usize) -> usize {
        self.cpu_topology
            .get(cpu)
            .map(|entry| entry.package_id)
            .unwrap_or(0)
    }

    pub fn cpu_core_group(&self, cpu: usize) -> usize {
        self.cpu_topology
            .get(cpu)
            .map(|entry| entry.core_group)
            .unwrap_or(cpu / 2)
    }

    pub fn cpu_sibling_group(&self, cpu: usize) -> usize {
        self.cpu_topology
            .get(cpu)
            .map(|entry| entry.sibling_group)
            .unwrap_or(cpu % 2)
    }

    pub fn cpu_topology_inferred(&self, cpu: usize) -> bool {
        self.cpu_topology
            .get(cpu)
            .map(|entry| entry.inferred)
            .unwrap_or(true)
    }

    pub fn cpu_topology_distance(&self, from_cpu: usize, to_cpu: usize) -> usize {
        if from_cpu == to_cpu {
            return 0;
        }
        let Some(from) = self.cpu_topology.get(from_cpu) else {
            return 4;
        };
        let Some(to) = self.cpu_topology.get(to_cpu) else {
            return 4;
        };
        if from.package_id == to.package_id && from.core_group == to.core_group {
            return 1;
        }
        if from.package_id == to.package_id {
            return 2;
        }
        3
    }

    pub fn queued_threads_for_cpu_and_class(
        &self,
        cpu: usize,
        class: SchedulerClass,
    ) -> Vec<ThreadId> {
        if cpu >= self.logical_cpu_count {
            return Vec::new();
        }
        let urgent = scheduler_queue_snapshot(&self.urgent_queues[cpu][class.index()])
            .into_iter()
            .filter(|tid| {
                self.thread_state(*tid)
                    .and_then(|entry| entry.queued)
                    .is_some_and(|entry| entry.class == class && entry.urgent)
                    && self
                        .thread_state(*tid)
                        .is_some_and(|state| state.assigned_cpu == cpu)
            });
        let normal = scheduler_queue_snapshot(&self.queues[cpu][class.index()])
            .into_iter()
            .filter(|tid| {
                self.thread_state(*tid)
                    .and_then(|entry| entry.queued)
                    .is_some_and(|entry| entry.class == class && !entry.urgent)
                    && self
                        .thread_state(*tid)
                        .is_some_and(|state| state.assigned_cpu == cpu)
            });
        urgent.chain(normal).collect()
    }

    pub fn cpu_class_queued_loads(&self) -> Vec<[usize; 4]> {
        let mut loads = vec![[0usize; 4]; self.logical_cpu_count];
        for class in SchedulerClass::ALL {
            let class_index = class.index();
            for tid in self.queued_threads_for_class(class) {
                if let Some(state) = self.thread_state(tid)
                    && state.assigned_cpu < self.logical_cpu_count
                {
                    loads[state.assigned_cpu][class_index] =
                        loads[state.assigned_cpu][class_index].saturating_add(1);
                }
            }
        }
        loads
    }

    pub(crate) fn queued_thread_assignments(&self) -> Vec<(u64, usize, u64)> {
        self.thread_states
            .iter()
            .enumerate()
            .filter_map(|(index, state)| {
                let state = (*state)?;
                state
                    .queued
                    .map(|_| (index as u64, state.assigned_cpu, state.affinity_mask))
            })
            .collect()
    }

    pub fn thread_assignment(&self, tid: ThreadId) -> Option<(usize, u64)> {
        self.thread_state(tid)
            .map(|state| (state.assigned_cpu, state.affinity_mask))
    }

    pub fn cpu_load_imbalance(&self) -> usize {
        let min = self.cpu_queued_loads.iter().copied().min().unwrap_or(0);
        let max = self.cpu_queued_loads.iter().copied().max().unwrap_or(0);
        max.saturating_sub(min)
    }

    pub fn rebalance_operations(&self) -> u64 {
        self.rebalance_operations
    }

    pub fn rebalance_migrations(&self) -> u64 {
        self.rebalance_migrations
    }

    pub fn last_rebalance_migrations(&self) -> usize {
        self.last_rebalance_migrations
    }

    const fn class_token_refill() -> [u8; 4] {
        [8, 4, 2, 1]
    }

    fn normalize_cpu_topology(
        logical_cpu_count: usize,
        cpu_topology: Vec<SchedulerCpuTopologyEntry>,
    ) -> Vec<SchedulerCpuTopologyEntry> {
        if cpu_topology.len() == logical_cpu_count {
            return cpu_topology;
        }
        (0..logical_cpu_count)
            .map(|cpu| SchedulerCpuTopologyEntry {
                apic_id: cpu as u32,
                package_id: 0,
                core_group: cpu / 2,
                sibling_group: cpu % 2,
                inferred: true,
            })
            .collect()
    }

    fn new_queue_set() -> [BufRing<ThreadId>; 4] {
        [
            BufRing::with_capacity(Self::QUEUE_CAPACITY),
            BufRing::with_capacity(Self::QUEUE_CAPACITY),
            BufRing::with_capacity(Self::QUEUE_CAPACITY),
            BufRing::with_capacity(Self::QUEUE_CAPACITY),
        ]
    }

    pub fn decision_tracing_enabled(&self) -> bool {
        self.decision_tracing_enabled
    }

    pub fn recent_decisions(&self) -> &[SchedulerAgentDecisionRecord] {
        &self.decisions
    }

    pub fn set_decision_tracing_enabled(&mut self, enabled: bool) {
        self.decision_tracing_enabled = enabled;
    }

    fn record_decision(
        &mut self,
        agent: SchedulerAgentKind,
        pid: ProcessId,
        tid: ThreadId,
        class: SchedulerClass,
        detail0: u64,
        detail1: u64,
    ) {
        if !self.decision_tracing_enabled {
            return;
        }
        if self.decisions.len() == 64 {
            self.decisions.remove(0);
        }
        self.decisions.push(SchedulerAgentDecisionRecord {
            tick: self.tick_counter,
            agent,
            pid: pid.raw(),
            tid: tid.raw(),
            class: class.index() as u64,
            detail0,
            detail1,
        });
    }

    pub fn enqueue(
        &mut self,
        processes: &mut ProcessTable,
        pid: ProcessId,
        class: SchedulerClass,
    ) -> Result<(), SchedulerError> {
        self.enqueue_with_budget(processes, pid, class, self.default_budget)
    }

    pub fn enqueue_with_budget(
        &mut self,
        processes: &mut ProcessTable,
        pid: ProcessId,
        class: SchedulerClass,
        budget: u32,
    ) -> Result<(), SchedulerError> {
        let tid = processes
            .get(pid)
            .map_err(SchedulerError::from_process_error)?
            .main_thread()
            .ok_or(SchedulerError::InvalidPid)?;
        let state = processes
            .get(pid)
            .map_err(SchedulerError::from_process_error)?
            .state();

        if self.contains(tid) {
            return Err(SchedulerError::DuplicateProcess);
        }

        match state {
            ProcessState::Created | ProcessState::Blocked => {
                processes
                    .set_state(pid, ProcessState::Ready)
                    .map_err(SchedulerError::from_process_error)?;
            }
            ProcessState::Ready => {}
            other => return Err(SchedulerError::InvalidProcessState(other)),
        }

        self.push_ready_thread(tid, class, budget.max(1), false)?;
        self.request_reschedule_for(class);
        self.record_decision(
            SchedulerAgentKind::EnqueueAgent,
            pid,
            tid,
            class,
            budget.max(1) as u64,
            state as u64,
        );
        Ok(())
    }

    pub fn wake(
        &mut self,
        processes: &mut ProcessTable,
        pid: ProcessId,
        class: SchedulerClass,
    ) -> Result<(), SchedulerError> {
        self.wake_with_budget(processes, pid, class, self.default_budget)
    }

    pub fn wake_with_budget(
        &mut self,
        processes: &mut ProcessTable,
        pid: ProcessId,
        class: SchedulerClass,
        budget: u32,
    ) -> Result<(), SchedulerError> {
        let tid = processes
            .get(pid)
            .map_err(SchedulerError::from_process_error)?
            .main_thread()
            .ok_or(SchedulerError::InvalidPid)?;
        let state = processes
            .get(pid)
            .map_err(SchedulerError::from_process_error)?
            .state();
        if self.contains(tid) {
            return Err(SchedulerError::DuplicateProcess);
        }
        match state {
            ProcessState::Created | ProcessState::Blocked => {
                processes
                    .set_state(pid, ProcessState::Ready)
                    .map_err(SchedulerError::from_process_error)?;
            }
            ProcessState::Ready => {}
            other => return Err(SchedulerError::InvalidProcessState(other)),
        }
        self.push_ready_thread(tid, class, budget.max(1), true)?;
        self.request_reschedule_for(class);
        self.record_decision(
            SchedulerAgentKind::EnqueueAgent,
            pid,
            tid,
            class,
            budget.max(1) as u64,
            state as u64,
        );
        self.record_decision(SchedulerAgentKind::WakeAgent, pid, tid, class, 1, 0);
        Ok(())
    }

    pub fn block_running(
        &mut self,
        processes: &mut ProcessTable,
    ) -> Result<ProcessId, SchedulerError> {
        let running = self
            .running
            .take()
            .ok_or(SchedulerError::NoRunnableProcess)?;
        processes
            .set_state(running.pid, ProcessState::Blocked)
            .map_err(SchedulerError::from_process_error)?;
        self.record_decision(
            SchedulerAgentKind::BlockAgent,
            running.pid,
            running.tid,
            running.class,
            running.budget as u64,
            0,
        );
        Ok(running.pid)
    }

    pub fn exit_running(
        &mut self,
        processes: &mut ProcessTable,
        code: i32,
    ) -> Result<ProcessId, SchedulerError> {
        let running = self
            .running
            .take()
            .ok_or(SchedulerError::NoRunnableProcess)?;
        processes
            .exit(running.pid, code)
            .map_err(SchedulerError::from_process_error)?;
        Ok(running.pid)
    }

    pub fn tick(
        &mut self,
        processes: &mut ProcessTable,
    ) -> Result<ScheduledProcess, SchedulerError> {
        self.tick_counter = self.tick_counter.saturating_add(1);
        self.rebalance_queued_threads();
        if let Some(mut running) = self.running.take() {
            self.class_runtime_ticks[running.class.index()] =
                self.class_runtime_ticks[running.class.index()].saturating_add(1);
            self.cpu_runtime_ticks[running.cpu] =
                self.cpu_runtime_ticks[running.cpu].saturating_add(1);
            processes
                .account_runtime_tick(running.pid)
                .map_err(SchedulerError::from_process_error)?;
            match processes
                .get(running.pid)
                .map_err(SchedulerError::from_process_error)?
                .state()
            {
                ProcessState::Running => {}
                ProcessState::Exited => {
                    return self
                        .dequeue_next(processes)
                        .ok_or(SchedulerError::NoRunnableProcess)
                        .and_then(|next| {
                            processes
                                .set_state(next.pid, ProcessState::Running)
                                .map_err(SchedulerError::from_process_error)?;
                            self.running = Some(next.clone());
                            Ok(next)
                        });
                }
                other => return Err(SchedulerError::InvalidProcessState(other)),
            }
            if running.budget > 1 {
                running.budget -= 1;
                self.record_decision(
                    SchedulerAgentKind::TickAgent,
                    running.pid,
                    running.tid,
                    running.class,
                    1,
                    running.budget as u64,
                );
                self.running = Some(running.clone());
                return Ok(running);
            }

            processes
                .set_state(running.pid, ProcessState::Ready)
                .map_err(SchedulerError::from_process_error)?;
            self.push_ready_thread(running.tid, running.class, running.budget.max(1), false)?;
            self.record_decision(
                SchedulerAgentKind::TickAgent,
                running.pid,
                running.tid,
                running.class,
                2,
                running.budget.max(1) as u64,
            );
        }

        let next = self
            .dequeue_next(processes)
            .ok_or(SchedulerError::NoRunnableProcess)?;
        processes
            .set_state(next.pid, ProcessState::Running)
            .map_err(SchedulerError::from_process_error)?;
        self.record_decision(
            SchedulerAgentKind::TickAgent,
            next.pid,
            next.tid,
            next.class,
            3,
            next.budget as u64,
        );
        self.class_dispatch_counts[next.class.index()] =
            self.class_dispatch_counts[next.class.index()].saturating_add(1);
        self.cpu_dispatch_counts[next.cpu] = self.cpu_dispatch_counts[next.cpu].saturating_add(1);
        self.running = Some(next.clone());
        Ok(next)
    }

    pub fn queued_len(&self) -> usize {
        self.queued_counts.iter().sum()
    }

    pub fn queued_len_by_class(&self) -> [usize; 4] {
        self.queued_counts
    }

    pub fn queued_urgent_len_by_class(&self) -> [usize; 4] {
        [
            self.queued_urgent_threads_for_class(SchedulerClass::LatencyCritical)
                .len(),
            self.queued_urgent_threads_for_class(SchedulerClass::Interactive)
                .len(),
            self.queued_urgent_threads_for_class(SchedulerClass::BestEffort)
                .len(),
            self.queued_urgent_threads_for_class(SchedulerClass::Background)
                .len(),
        ]
    }

    pub fn queued_threads_by_class(&self) -> [Vec<ThreadId>; 4] {
        [
            self.queued_threads_for_class(SchedulerClass::LatencyCritical),
            self.queued_threads_for_class(SchedulerClass::Interactive),
            self.queued_threads_for_class(SchedulerClass::BestEffort),
            self.queued_threads_for_class(SchedulerClass::Background),
        ]
    }

    pub fn starved_classes(&self) -> [bool; 4] {
        let mut starved = [false; 4];
        for class in SchedulerClass::ALL {
            let index = class.index();
            starved[index] = self.queued_counts[index] > 0
                && self.class_wait_ticks[index] >= Self::STARVATION_GUARD_TICKS;
        }
        starved
    }

    pub const fn starvation_guard_ticks(&self) -> u32 {
        Self::STARVATION_GUARD_TICKS
    }

    pub fn contains(&self, tid: ThreadId) -> bool {
        self.running
            .as_ref()
            .is_some_and(|running| running.tid == tid)
            || self
                .thread_state(tid)
                .is_some_and(|state| state.queued.is_some())
    }

    pub fn remove(
        &mut self,
        processes: &ProcessTable,
        pid: ProcessId,
    ) -> Result<(), SchedulerError> {
        let tid = processes
            .get(pid)
            .map_err(SchedulerError::from_process_error)?
            .main_thread()
            .ok_or(SchedulerError::InvalidPid)?;
        if self
            .running
            .as_ref()
            .is_some_and(|running| running.pid == pid)
        {
            self.running = None;
        }
        if let Some(state) = self.take_thread_state(tid)
            && let Some(membership) = state.queued
        {
            self.queued_counts[membership.class.index()] =
                self.queued_counts[membership.class.index()].saturating_sub(1);
            self.cpu_queued_loads[state.assigned_cpu] =
                self.cpu_queued_loads[state.assigned_cpu].saturating_sub(1);
        }
        self.record_decision(
            SchedulerAgentKind::RemoveAgent,
            pid,
            tid,
            SchedulerClass::BestEffort,
            0,
            0,
        );
        Ok(())
    }

    pub fn rebind_process(
        &mut self,
        processes: &ProcessTable,
        pid: ProcessId,
        class: SchedulerClass,
        budget: u32,
    ) -> Result<(), SchedulerError> {
        let process = processes
            .get(pid)
            .map_err(SchedulerError::from_process_error)?;
        let tid = process.main_thread().ok_or(SchedulerError::InvalidPid)?;
        let budget = budget.max(1);
        let assigned_cpu = {
            let state = self.thread_state_mut(tid, budget);
            state.budget = budget;
            state.assigned_cpu
        };
        if let Some(running) = self.running.as_mut()
            && running.pid == pid
        {
            running.class = class;
            running.budget = budget;
            running.cpu = assigned_cpu;
            self.record_decision(
                SchedulerAgentKind::RebindAgent,
                pid,
                tid,
                class,
                budget as u64,
                1,
            );
            return Ok(());
        }
        if let Some(previous) = self.thread_state_mut(tid, budget).queued.take() {
            self.queued_counts[previous.class.index()] =
                self.queued_counts[previous.class.index()].saturating_sub(1);
            let assigned_cpu = self
                .thread_state(tid)
                .map(|state| state.assigned_cpu)
                .unwrap_or(0);
            self.cpu_queued_loads[assigned_cpu] =
                self.cpu_queued_loads[assigned_cpu].saturating_sub(1);
        }
        if process.state() != ProcessState::Ready {
            self.record_decision(
                SchedulerAgentKind::RebindAgent,
                pid,
                tid,
                class,
                budget as u64,
                0,
            );
            return Ok(());
        }
        self.push_ready_thread(tid, class, budget, false)?;
        self.record_decision(
            SchedulerAgentKind::RebindAgent,
            pid,
            tid,
            class,
            budget as u64,
            2,
        );
        Ok(())
    }

    pub fn set_thread_affinity(
        &mut self,
        tid: ThreadId,
        affinity_mask: u64,
    ) -> Result<(), SchedulerError> {
        let sanitized = self.sanitize_affinity_mask(affinity_mask)?;
        let budget = self
            .thread_state(tid)
            .map(|state| state.budget)
            .unwrap_or(self.default_budget);
        let previous_cpu = self
            .thread_state(tid)
            .map(|state| state.assigned_cpu)
            .unwrap_or(0);
        let previous_membership = self.thread_state(tid).and_then(|state| state.queued);
        let was_queued = previous_membership.is_some();
        let next_cpu = if (sanitized & (1u64 << previous_cpu)) != 0 {
            previous_cpu
        } else {
            self.pick_cpu_for_mask_near(sanitized, previous_cpu)
        };
        {
            let state = self.thread_state_mut(tid, budget);
            state.affinity_mask = sanitized;
            state.assigned_cpu = next_cpu;
        }
        if was_queued && previous_cpu != next_cpu {
            self.cpu_queued_loads[previous_cpu] =
                self.cpu_queued_loads[previous_cpu].saturating_sub(1);
            if let Some(membership) = previous_membership
                && self.push_existing_thread_to_cpu(tid, membership, next_cpu)
            {
                self.cpu_queued_loads[next_cpu] = self.cpu_queued_loads[next_cpu].saturating_add(1);
            }
        }
        if let Some(running) = self.running.as_mut()
            && running.tid == tid
        {
            running.cpu = next_cpu;
        }
        self.record_decision(
            SchedulerAgentKind::AffinityAgent,
            ProcessId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            tid,
            previous_membership
                .map(|membership| membership.class)
                .or_else(|| {
                    self.running
                        .as_ref()
                        .filter(|running| running.tid == tid)
                        .map(|running| running.class)
                })
                .unwrap_or(SchedulerClass::BestEffort),
            sanitized,
            next_cpu as u64,
        );
        Ok(())
    }

    fn dequeue_next(&mut self, processes: &ProcessTable) -> Option<ScheduledProcess> {
        loop {
            let class = self.pick_next_class()?;
            let tid = self.pop_valid_thread_for_class(class)?;
            self.queued_counts[class.index()] = self.queued_counts[class.index()].saturating_sub(1);
            if self.queued_counts[class.index()] == 0 {
                self.class_lag_debt[class.index()] = 0;
            }
            if let Some(assigned_cpu) = self.thread_state(tid).map(|state| state.assigned_cpu) {
                self.cpu_queued_loads[assigned_cpu] =
                    self.cpu_queued_loads[assigned_cpu].saturating_sub(1);
            }
            if let Some(state) = self.thread_state_mut_existing(tid) {
                state.queued = None;
            }
            let cpu = self
                .thread_state(tid)
                .map(|state| state.assigned_cpu)
                .unwrap_or(0);
            let pid = processes.get_thread(tid).ok()?.owner();
            let budget = self
                .thread_state(tid)
                .map(|state| state.budget)
                .unwrap_or(self.default_budget);
            return Some(ScheduledProcess {
                tid,
                pid,
                class,
                budget,
                cpu,
            });
        }
    }

    fn pick_next_class(&mut self) -> Option<SchedulerClass> {
        let mut any_non_empty = false;
        for class in SchedulerClass::ALL {
            let index = class.index();
            if self.queued_counts[index] == 0 {
                self.class_wait_ticks[index] = 0;
                self.class_lag_debt[index] = 0;
                continue;
            }
            any_non_empty = true;
            self.class_wait_ticks[index] = self.class_wait_ticks[index].saturating_add(1);
            self.class_lag_debt[index] =
                self.class_lag_debt[index].saturating_add(Self::class_lag_credit(class));
        }
        if !any_non_empty {
            return None;
        }

        if let Some(class) = self.pick_starved_class() {
            self.commit_class_pick(class);
            return Some(class);
        }

        if let Some(class) = self.pick_lag_eligible_class() {
            self.commit_class_pick(class);
            return Some(class);
        }

        if let Some(class) = self.pick_token_eligible_class() {
            self.commit_class_pick(class);
            return Some(class);
        }

        self.class_dispatch_tokens = Self::class_token_refill();
        let class = self.pick_token_eligible_class().or_else(|| {
            SchedulerClass::ALL
                .into_iter()
                .find(|class| self.queued_counts[class.index()] > 0)
        })?;
        self.commit_class_pick(class);
        Some(class)
    }

    fn pick_starved_class(&self) -> Option<SchedulerClass> {
        let mut selected = None;
        let mut selected_age = 0;
        for class in SchedulerClass::ALL {
            let index = class.index();
            if self.queued_counts[index] == 0 {
                continue;
            }
            let age = self.class_wait_ticks[index];
            if age < Self::STARVATION_GUARD_TICKS {
                continue;
            }
            if age > selected_age {
                selected = Some(class);
                selected_age = age;
            }
        }
        selected
    }

    fn pick_token_eligible_class(&self) -> Option<SchedulerClass> {
        SchedulerClass::ALL.into_iter().find(|class| {
            let index = class.index();
            self.queued_counts[index] > 0 && self.class_dispatch_tokens[index] > 0
        })
    }

    fn pick_lag_eligible_class(&self) -> Option<SchedulerClass> {
        let mut selected = None;
        let mut selected_debt = i32::MIN;
        for class in SchedulerClass::ALL {
            let index = class.index();
            if self.queued_counts[index] == 0 || self.class_dispatch_tokens[index] == 0 {
                continue;
            }
            let debt = self.class_lag_debt[index];
            if debt > selected_debt {
                selected = Some(class);
                selected_debt = debt;
            }
        }
        selected
    }

    fn commit_class_pick(&mut self, class: SchedulerClass) {
        let index = class.index();
        self.class_wait_ticks[index] = 0;
        self.class_dispatch_tokens[index] = self.class_dispatch_tokens[index].saturating_sub(1);
        self.class_lag_debt[index] =
            self.class_lag_debt[index].saturating_sub(Self::class_dispatch_cost(class));
    }

    const fn class_lag_credit(class: SchedulerClass) -> i32 {
        match class {
            SchedulerClass::LatencyCritical => 4,
            SchedulerClass::Interactive => 3,
            SchedulerClass::BestEffort => 2,
            SchedulerClass::Background => 1,
        }
    }

    const fn class_dispatch_cost(class: SchedulerClass) -> i32 {
        match class {
            SchedulerClass::LatencyCritical => 6,
            SchedulerClass::Interactive => 5,
            SchedulerClass::BestEffort => 4,
            SchedulerClass::Background => 3,
        }
    }

    fn request_reschedule_for(&mut self, class: SchedulerClass) {
        if let Some(running) = self.running.as_mut()
            && class.index() < running.class.index()
            && running.budget > 1
        {
            running.budget = 1;
        }
    }

    fn push_ready_thread(
        &mut self,
        tid: ThreadId,
        class: SchedulerClass,
        budget: u32,
        urgent: bool,
    ) -> Result<(), SchedulerError> {
        let affinity_mask = self
            .thread_state(tid)
            .map(|state| state.affinity_mask)
            .unwrap_or_else(|| self.default_affinity_mask());
        let preferred_cpu = self.next_dispatch_cpu % self.logical_cpu_count;
        let assigned_cpu = self.pick_cpu_for_mask_near(affinity_mask, preferred_cpu);
        if !self.push_existing_thread_to_cpu(tid, QueueMembership { class, urgent }, assigned_cpu) {
            return Err(SchedulerError::QueueFull);
        }
        let state = self.thread_state_mut(tid, budget);
        state.budget = budget;
        state.assigned_cpu = assigned_cpu;
        state.queued = Some(QueueMembership { class, urgent });
        self.queued_counts[class.index()] += 1;
        self.cpu_queued_loads[assigned_cpu] = self.cpu_queued_loads[assigned_cpu].saturating_add(1);
        self.next_dispatch_cpu = (assigned_cpu + 1) % self.logical_cpu_count;
        Ok(())
    }

    fn push_existing_thread_to_cpu(
        &mut self,
        tid: ThreadId,
        membership: QueueMembership,
        cpu: usize,
    ) -> bool {
        let index = membership.class.index();
        let queue = if membership.urgent {
            &mut self.urgent_queues[cpu][index]
        } else {
            &mut self.queues[cpu][index]
        };
        if queue.push(tid).is_ok() {
            return true;
        }
        self.compact_queue(cpu, membership.class, membership.urgent);
        let queue = if membership.urgent {
            &mut self.urgent_queues[cpu][index]
        } else {
            &mut self.queues[cpu][index]
        };
        queue.push(tid).is_ok()
    }

    fn compact_queue(&mut self, cpu: usize, class: SchedulerClass, urgent: bool) {
        let index = class.index();
        let thread_states = &self.thread_states;
        let queue = if urgent {
            &mut self.urgent_queues[cpu][index]
        } else {
            &mut self.queues[cpu][index]
        };
        queue.retain(|queued| {
            thread_states
                .get(queued.raw() as usize)
                .and_then(|state| *state)
                .and_then(|state| state.queued)
                .is_some_and(|state| state.class == class && state.urgent == urgent)
                && thread_states
                    .get(queued.raw() as usize)
                    .and_then(|state| *state)
                    .is_some_and(|state| state.assigned_cpu == cpu)
        });
    }

    fn pop_valid_thread_for_class(&mut self, class: SchedulerClass) -> Option<ThreadId> {
        let index = class.index();
        for urgent in [true, false] {
            for offset in 0..self.logical_cpu_count {
                let cpu = (self.next_dispatch_cpu + offset) % self.logical_cpu_count;
                loop {
                    let next = {
                        let queue = if urgent {
                            &mut self.urgent_queues[cpu][index]
                        } else {
                            &mut self.queues[cpu][index]
                        };
                        queue.pop()
                    };
                    let Some(tid) = next else {
                        break;
                    };
                    if self
                        .thread_state(tid)
                        .and_then(|membership| membership.queued)
                        .is_some_and(|membership| {
                            membership.class == class && membership.urgent == urgent
                        })
                        && self
                            .thread_state(tid)
                            .is_some_and(|state| state.assigned_cpu == cpu)
                    {
                        self.next_dispatch_cpu = (cpu + 1) % self.logical_cpu_count;
                        return Some(tid);
                    }
                }
            }
        }
        None
    }

    fn rebalance_queued_threads(&mut self) {
        if self.logical_cpu_count <= 1 || self.queued_len() == 0 {
            self.last_rebalance_migrations = 0;
            return;
        }
        self.rebalance_operations = self.rebalance_operations.saturating_add(1);
        let mut migrated = 0usize;
        for class in SchedulerClass::ALL {
            for tid in self.queued_threads_for_class(class) {
                let Some(state) = self.thread_state(tid) else {
                    continue;
                };
                let current_cpu = state.assigned_cpu;
                let best_cpu = self.pick_cpu_for_mask_near(state.affinity_mask, current_cpu);
                if best_cpu == current_cpu {
                    continue;
                }
                let current_load = self.cpu_queued_loads[current_cpu];
                let best_load = self.cpu_queued_loads[best_cpu];
                if current_load <= best_load.saturating_add(1) {
                    continue;
                }
                let Some(membership) = state.queued else {
                    continue;
                };
                if !self.push_existing_thread_to_cpu(tid, membership, best_cpu) {
                    continue;
                }
                self.cpu_queued_loads[current_cpu] =
                    self.cpu_queued_loads[current_cpu].saturating_sub(1);
                self.cpu_queued_loads[best_cpu] = self.cpu_queued_loads[best_cpu].saturating_add(1);
                if let Some(entry) = self.thread_state_mut_existing(tid) {
                    entry.assigned_cpu = best_cpu;
                }
                migrated = migrated.saturating_add(1);
            }
        }
        self.last_rebalance_migrations = migrated;
        self.rebalance_migrations = self.rebalance_migrations.saturating_add(migrated as u64);
    }

    fn queued_threads_for_class(&self, class: SchedulerClass) -> Vec<ThreadId> {
        (0..self.logical_cpu_count)
            .flat_map(|cpu| self.queued_threads_for_cpu_and_class(cpu, class))
            .collect()
    }

    fn queued_urgent_threads_for_class(&self, class: SchedulerClass) -> Vec<ThreadId> {
        (0..self.logical_cpu_count)
            .flat_map(|cpu| {
                scheduler_queue_snapshot(&self.urgent_queues[cpu][class.index()])
                    .into_iter()
                    .filter(move |tid| {
                        self.thread_state(*tid)
                            .and_then(|entry| entry.queued)
                            .is_some_and(|entry| entry.class == class && entry.urgent)
                            && self
                                .thread_state(*tid)
                                .is_some_and(|state| state.assigned_cpu == cpu)
                    })
            })
            .collect()
    }

    fn thread_state_mut(&mut self, tid: ThreadId, budget: u32) -> &mut SchedulerThreadState {
        self.ensure_thread_state_capacity(tid);
        let default_affinity_mask = self.default_affinity_mask();
        self.thread_states[tid.raw() as usize].get_or_insert(SchedulerThreadState {
            budget,
            queued: None,
            affinity_mask: default_affinity_mask,
            assigned_cpu: 0,
        })
    }

    fn thread_state_mut_existing(&mut self, tid: ThreadId) -> Option<&mut SchedulerThreadState> {
        self.thread_states
            .get_mut(tid.raw() as usize)
            .and_then(Option::as_mut)
    }

    fn thread_state(&self, tid: ThreadId) -> Option<SchedulerThreadState> {
        self.thread_states
            .get(tid.raw() as usize)
            .and_then(|state| *state)
    }

    fn take_thread_state(&mut self, tid: ThreadId) -> Option<SchedulerThreadState> {
        self.thread_states
            .get_mut(tid.raw() as usize)
            .and_then(Option::take)
    }

    fn ensure_thread_state_capacity(&mut self, tid: ThreadId) {
        let index = tid.raw() as usize;
        if self.thread_states.len() <= index {
            self.thread_states.resize(index + 1, None);
        }
    }

    fn default_affinity_mask(&self) -> u64 {
        if self.logical_cpu_count >= u64::BITS as usize {
            u64::MAX
        } else {
            (1u64 << self.logical_cpu_count) - 1
        }
    }

    fn sanitize_affinity_mask(&self, mask: u64) -> Result<u64, SchedulerError> {
        let visible = mask & self.default_affinity_mask();
        if visible == 0 {
            return Err(SchedulerError::InvalidCpuAffinity);
        }
        Ok(visible)
    }

    fn pick_cpu_for_mask_near(&self, affinity_mask: u64, preferred_cpu: usize) -> usize {
        let mut best_cpu = 0usize;
        let mut best_load = usize::MAX;
        let mut best_distance = usize::MAX;
        for cpu in 0..self.logical_cpu_count {
            if (affinity_mask & (1u64 << cpu)) == 0 {
                continue;
            }
            let load = self.cpu_queued_loads[cpu]
                + usize::from(
                    self.running
                        .as_ref()
                        .is_some_and(|running| running.cpu == cpu),
                );
            let distance = self.cpu_topology_distance(preferred_cpu, cpu);
            if load < best_load || (load == best_load && distance < best_distance) {
                best_load = load;
                best_distance = distance;
                best_cpu = cpu;
            }
        }
        best_cpu
    }

    #[cfg(test)]
    pub(crate) fn inject_wait_ticks_for_test(&mut self, class: SchedulerClass, ticks: u32) {
        self.class_wait_ticks[class.index()] = ticks;
    }

    #[cfg(test)]
    pub(crate) fn inject_lag_debt_for_test(&mut self, class: SchedulerClass, debt: i32) {
        self.class_lag_debt[class.index()] = debt;
    }

    #[cfg(test)]
    pub(crate) fn class_runtime_ticks_mut_for_test(&mut self) -> &mut [u64; 4] {
        &mut self.class_runtime_ticks
    }

    #[cfg(test)]
    pub(crate) fn inject_thread_assignment_for_test(
        &mut self,
        tid: ThreadId,
        assigned_cpu: usize,
        affinity_mask: u64,
    ) {
        let budget = self
            .thread_state(tid)
            .map(|state| state.budget)
            .unwrap_or(self.default_budget);
        let state = self.thread_state_mut(tid, budget);
        state.assigned_cpu = assigned_cpu;
        state.affinity_mask = affinity_mask;
    }
}
