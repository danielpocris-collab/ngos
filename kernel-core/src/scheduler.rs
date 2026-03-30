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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidPid,
    DuplicateProcess,
    QueueFull,
    NotQueued,
    NoRunnableProcess,
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
            ProcessError::InvalidTransition { from, .. } => Self::InvalidProcessState(from),
            ProcessError::NotExited => Self::InvalidProcessState(ProcessState::Ready),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheduler {
    queues: [BufRing<ThreadId>; 4],
    running: Option<ScheduledProcess>,
    default_budget: u32,
    budget_overrides: BTreeMap<u64, u32>,
    decisions: Vec<SchedulerAgentDecisionRecord>,
    tick_counter: u64,
    decision_tracing_enabled: bool,
}

impl Scheduler {
    pub const QUEUE_CAPACITY: usize = 1 << 16;

    pub fn new(default_budget: u32) -> Self {
        assert!(default_budget > 0, "scheduler budget must be non-zero");
        Self {
            queues: [
                BufRing::with_capacity(Self::QUEUE_CAPACITY),
                BufRing::with_capacity(Self::QUEUE_CAPACITY),
                BufRing::with_capacity(Self::QUEUE_CAPACITY),
                BufRing::with_capacity(Self::QUEUE_CAPACITY),
            ],
            running: None,
            default_budget,
            budget_overrides: BTreeMap::new(),
            decisions: Vec::with_capacity(64),
            tick_counter: 0,
            decision_tracing_enabled: true,
        }
    }

    pub fn running(&self) -> Option<&ScheduledProcess> {
        self.running.as_ref()
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

        self.queues[class.index()]
            .push(tid)
            .map_err(|_| SchedulerError::QueueFull)?;
        self.budget_overrides.insert(tid.raw(), budget.max(1));
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
        let result = self.enqueue(processes, pid, class);
        if let Ok(process) = processes.get(pid)
            && let Some(tid) = process.main_thread()
        {
            self.record_decision(SchedulerAgentKind::WakeAgent, pid, tid, class, 0, 0);
        }
        result
    }

    pub fn wake_with_budget(
        &mut self,
        processes: &mut ProcessTable,
        pid: ProcessId,
        class: SchedulerClass,
        budget: u32,
    ) -> Result<(), SchedulerError> {
        self.enqueue_with_budget(processes, pid, class, budget)
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
        if let Some(mut running) = self.running.take() {
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
            self.queues[running.class.index()]
                .push(running.tid)
                .map_err(|_| SchedulerError::QueueFull)?;
            self.budget_overrides
                .insert(running.tid.raw(), running.budget.max(1));
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
        self.running = Some(next.clone());
        Ok(next)
    }

    pub fn queued_len(&self) -> usize {
        self.queues.iter().map(BufRing::len).sum()
    }

    pub fn queued_len_by_class(&self) -> [usize; 4] {
        [
            self.queues[SchedulerClass::LatencyCritical.index()].len(),
            self.queues[SchedulerClass::Interactive.index()].len(),
            self.queues[SchedulerClass::BestEffort.index()].len(),
            self.queues[SchedulerClass::Background.index()].len(),
        ]
    }

    pub fn contains(&self, tid: ThreadId) -> bool {
        self.running
            .as_ref()
            .is_some_and(|running| running.tid == tid)
            || self.queues.iter().any(|queue| {
                scheduler_queue_snapshot(queue)
                    .into_iter()
                    .any(|queued| queued == tid)
            })
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
        self.budget_overrides.remove(&tid.raw());
        for queue in &mut self.queues {
            queue.retain(|queued| *queued != tid);
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
        self.budget_overrides.insert(tid.raw(), budget);
        if let Some(running) = self.running.as_mut()
            && running.pid == pid
        {
            running.class = class;
            running.budget = budget;
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
        for queue in &mut self.queues {
            queue.retain(|queued| *queued != tid);
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
        self.queues[class.index()]
            .push(tid)
            .map_err(|_| SchedulerError::QueueFull)?;
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

    fn dequeue_next(&mut self, processes: &ProcessTable) -> Option<ScheduledProcess> {
        for class in SchedulerClass::ALL {
            let queue = &mut self.queues[class.index()];
            if queue.is_empty() {
                continue;
            }
            let tid = queue.pop()?;
            let pid = processes.get_thread(tid).ok()?.owner();
            let budget = self
                .budget_overrides
                .get(&tid.raw())
                .copied()
                .unwrap_or(self.default_budget);
            return Some(ScheduledProcess {
                tid,
                pid,
                class,
                budget,
            });
        }
        None
    }
}
