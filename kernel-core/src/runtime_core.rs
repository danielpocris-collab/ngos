//! Canonical subsystem role:
//! - subsystem: kernel runtime core
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical execution engine that binds kernel models,
//!   scheduler, devices, and eventing into runtime behavior
//!
//! Canonical contract families handled here:
//! - runtime orchestration contracts
//! - hardware provider integration contracts
//! - process/vm/eventing execution contracts
//! - runtime snapshot source contracts
//!
//! This module may execute and mutate canonical kernel runtime truth. Higher
//! layers may observe or drive it through contracts, but they must not shadow
//! its ownership.

use super::*;
use crate::bus_model::{BusEndpointTable, BusPeerTable};
use crate::descriptor_model::initial_payload_for_kind;
use crate::device_model::{DeviceRegistry, NetworkInterface, NetworkSocket};

#[path = "runtime_core/eventing.rs"]
mod eventing;
#[path = "runtime_core/native_model.rs"]
mod native_model;
#[path = "runtime_core/process_vm.rs"]
mod process_vm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeferredRuntimeTask {
    RefreshEventQueue(QueueDescriptorTarget),
}

pub(crate) struct HardwareSlot(Option<Box<dyn HardwareProvider>>);

impl core::fmt::Debug for HardwareSlot {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("HardwareSlot(..)")
    }
}

impl Clone for HardwareSlot {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl PartialEq for HardwareSlot {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for HardwareSlot {}

impl HardwareSlot {
    fn empty() -> Self {
        Self(None)
    }

    pub(crate) fn as_mut(&mut self) -> Option<&mut (dyn HardwareProvider + '_)> {
        match self.0 {
            Some(ref mut provider) => Some(provider.as_mut()),
            None => None,
        }
    }

    fn replace(
        &mut self,
        provider: Box<dyn HardwareProvider>,
    ) -> Option<Box<dyn HardwareProvider>> {
        self.0.replace(provider)
    }

    fn take(&mut self) -> Option<Box<dyn HardwareProvider>> {
        self.0.take()
    }
}

const RESOURCE_AGENT_DECISION_LIMIT: usize = 64;
const WAIT_AGENT_DECISION_LIMIT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntime {
    pub(crate) processes: ProcessTable,
    pub(crate) capabilities: CapabilityTable,
    pub(crate) domains: DomainTable,
    pub(crate) bus_peers: BusPeerTable,
    pub(crate) bus_endpoints: BusEndpointTable,
    pub(crate) resources: ResourceTable,
    pub(crate) contracts: ContractTable,
    pub(crate) scheduler: Scheduler,
    pub(crate) namespaces: Vec<(ProcessId, DescriptorNamespace)>,
    pub(crate) fdshare_groups: Vec<FiledescShareGroup>,
    pub(crate) next_fdshare_group_id: u64,
    pub(crate) vfs: VfsNamespace,
    pub(crate) io_registry: IoRegistry,
    pub(crate) device_registry: DeviceRegistry,
    pub(crate) network_ifaces: Vec<NetworkInterface>,
    pub(crate) network_sockets: Vec<NetworkSocket>,
    pub(crate) runtime_channels: Vec<RuntimeChannel>,
    pub(crate) readiness: Vec<ReadinessRegistration>,
    pub(crate) event_queues: Vec<EventQueue>,
    pub(crate) sleep_queues: Vec<RuntimeSleepQueue>,
    pub(crate) sleep_results: BTreeMap<u64, SleepWaitResult>,
    pub(crate) signal_wait_queues: BTreeMap<u64, SleepQueueId>,
    pub(crate) signal_wait_masks: BTreeMap<u64, u64>,
    pub(crate) memory_wait_queues: BTreeMap<u64, SleepQueueId>,
    pub(crate) memory_waiters: BTreeMap<MemoryWaitKey, Vec<MemoryWaiter>>,
    pub(crate) memory_wait_resume_indices: BTreeMap<u64, usize>,
    pub(crate) deferred_tasks: TaskQueue<DeferredRuntimeTask>,
    pub(crate) resource_agent_decisions: Vec<ResourceAgentDecisionRecord>,
    pub(crate) wait_agent_decisions: Vec<WaitAgentDecisionRecord>,
    pub(crate) io_agent_decisions: Vec<IoAgentDecisionRecord>,
    pub(crate) decision_tracing_enabled: bool,
    pub(crate) current_tick: u64,
    pub(crate) busy_ticks: u64,
    pub(crate) next_event_queue_id: u64,
    pub(crate) next_event_timer_id: u64,
    pub(crate) next_sleep_queue_id: u64,
    pub(crate) active_cpu_extended_state: Option<ActiveCpuExtendedStateSlot>,
    pub(crate) cpu_extended_state_hardware: CpuExtendedStateHardwareTelemetry,

    // Hardware Abstraction
    pub(crate) hardware: HardwareSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeChannel {
    pub(crate) path: String,
    pub(crate) messages: Vec<Vec<u8>>,
}

impl KernelRuntime {
    pub fn new(policy: RuntimePolicy) -> Self {
        let mut processes = ProcessTable::new(policy.process_range.start, policy.process_range.end);
        let default_cpu_profile = policy
            .cpu_extended_state_handoff
            .map(|handoff| handoff.default_thread_profile())
            .unwrap_or(policy.default_thread_cpu_extended_state);
        processes.set_default_thread_cpu_extended_state(default_cpu_profile);
        Self {
            processes,
            capabilities: CapabilityTable::new(
                policy.capability_range.start,
                policy.capability_range.end,
            ),
            domains: DomainTable::new(policy.domain_range.start, policy.domain_range.end),
            bus_peers: BusPeerTable::new(1, 1 << 16),
            bus_endpoints: BusEndpointTable::new(1, 1 << 16),
            resources: ResourceTable::new(policy.resource_range.start, policy.resource_range.end),
            contracts: ContractTable::new(policy.contract_range.start, policy.contract_range.end),
            scheduler: Scheduler::new_with_topology(
                policy.scheduler_budget,
                policy.scheduler_logical_cpu_count,
                policy.scheduler_cpu_topology.clone(),
            ),
            namespaces: Vec::new(),
            fdshare_groups: Vec::new(),
            next_fdshare_group_id: 1,
            vfs: VfsNamespace::new(),
            io_registry: IoRegistry::new(),
            device_registry: DeviceRegistry::new(),
            network_ifaces: Vec::new(),
            network_sockets: Vec::new(),
            runtime_channels: Vec::new(),
            readiness: Vec::new(),
            event_queues: Vec::new(),
            sleep_queues: Vec::new(),
            sleep_results: BTreeMap::new(),
            signal_wait_queues: BTreeMap::new(),
            signal_wait_masks: BTreeMap::new(),
            memory_wait_queues: BTreeMap::new(),
            memory_waiters: BTreeMap::new(),
            memory_wait_resume_indices: BTreeMap::new(),
            deferred_tasks: TaskQueue::with_limit(1024),
            resource_agent_decisions: Vec::with_capacity(RESOURCE_AGENT_DECISION_LIMIT),
            wait_agent_decisions: Vec::with_capacity(WAIT_AGENT_DECISION_LIMIT),
            io_agent_decisions: Vec::with_capacity(64),
            decision_tracing_enabled: true,
            current_tick: 0,
            busy_ticks: 0,
            next_event_queue_id: 1,
            next_event_timer_id: 1,
            next_sleep_queue_id: 1,
            active_cpu_extended_state: None,
            cpu_extended_state_hardware: CpuExtendedStateHardwareTelemetry::default(),
            hardware: HardwareSlot::empty(),
        }
    }

    pub fn host_runtime_default() -> Self {
        Self::new(RuntimePolicy::host_runtime_default())
    }

    #[allow(dead_code)]
    pub fn set_default_thread_cpu_extended_state(
        &mut self,
        profile: ThreadCpuExtendedStateProfile,
    ) {
        self.processes
            .set_default_thread_cpu_extended_state(profile);
    }

    pub fn apply_cpu_extended_state_handoff(
        &mut self,
        handoff: CpuExtendedStateHandoff,
    ) -> ThreadCpuExtendedStateProfile {
        let profile = handoff.default_thread_profile();
        self.processes
            .set_default_thread_cpu_extended_state(profile);
        profile
    }

    pub fn default_thread_cpu_extended_state(&self) -> ThreadCpuExtendedStateProfile {
        self.processes.default_thread_cpu_extended_state()
    }

    pub fn apply_cpu_handoff_to_process_threads(
        &mut self,
        pid: ProcessId,
        handoff: CpuExtendedStateHandoff,
    ) -> Result<usize, RuntimeError> {
        self.processes
            .apply_thread_cpu_extended_state_to_process(pid, handoff.default_thread_profile())
            .map_err(Into::into)
    }

    pub fn restore_process_threads_to_default_cpu_handoff(
        &mut self,
        pid: ProcessId,
    ) -> Result<usize, RuntimeError> {
        self.processes
            .restore_default_thread_cpu_extended_state_to_process(pid)
            .map_err(Into::into)
    }

    pub fn install_hardware_provider(
        &mut self,
        provider: Box<dyn HardwareProvider>,
    ) -> Option<Box<dyn HardwareProvider>> {
        self.hardware.replace(provider)
    }

    pub fn remove_hardware_provider(&mut self) -> Option<Box<dyn HardwareProvider>> {
        self.hardware.take()
    }

    pub fn processes(&self) -> &ProcessTable {
        &self.processes
    }

    pub fn capabilities(&self) -> &CapabilityTable {
        &self.capabilities
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn vfs(&self) -> &VfsNamespace {
        &self.vfs
    }

    pub fn io_registry(&self) -> &IoRegistry {
        &self.io_registry
    }

    pub fn recent_resource_agent_decisions(&self) -> &[ResourceAgentDecisionRecord] {
        &self.resource_agent_decisions
    }

    pub fn recent_wait_agent_decisions(&self) -> &[WaitAgentDecisionRecord] {
        &self.wait_agent_decisions
    }

    pub fn recent_io_agent_decisions(&self) -> &[IoAgentDecisionRecord] {
        &self.io_agent_decisions
    }

    pub fn recent_vm_agent_decisions(&self) -> &[VmAgentDecisionRecord] {
        self.processes.recent_vm_agent_decisions()
    }

    pub fn active_cpu_extended_state(&self) -> Option<&ActiveCpuExtendedStateSlot> {
        self.active_cpu_extended_state.as_ref()
    }

    pub fn cpu_extended_state_hardware_telemetry(&self) -> CpuExtendedStateHardwareTelemetry {
        self.cpu_extended_state_hardware
    }

    pub fn set_decision_tracing_enabled(&mut self, enabled: bool) {
        self.decision_tracing_enabled = enabled;
        self.scheduler.set_decision_tracing_enabled(enabled);
        self.processes.set_decision_tracing_enabled(enabled);
    }

    pub fn tick(&mut self) -> Result<ScheduledProcess, RuntimeError> {
        self.current_tick = self.current_tick.saturating_add(1);
        self.tick_sleep_queues()?;
        self.tick_event_queue_timers()?;
        let previous_running = self.scheduler.running().cloned();
        let scheduled = self
            .scheduler
            .tick(&mut self.processes)
            .map_err(RuntimeError::from)?;
        let should_flush_active_slot = self
            .active_cpu_extended_state
            .as_ref()
            .map(|slot| slot.owner_tid != scheduled.tid)
            .unwrap_or(false);
        if should_flush_active_slot && let Some(slot) = self.active_cpu_extended_state.take() {
            let mut slot = slot;
            let flushed_tid = slot.owner_tid;
            if let Some(provider) = self.hardware.as_mut() {
                match provider.save_cpu_extended_state(
                    slot.owner_pid,
                    slot.owner_tid,
                    &mut slot.image,
                ) {
                    Ok(()) => {
                        self.cpu_extended_state_hardware.save_count = self
                            .cpu_extended_state_hardware
                            .save_count
                            .saturating_add(1);
                        self.cpu_extended_state_hardware.last_saved_tid = Some(slot.owner_tid);
                        self.cpu_extended_state_hardware.last_error = None;
                    }
                    Err(error) => {
                        self.cpu_extended_state_hardware.fallback_count = self
                            .cpu_extended_state_hardware
                            .fallback_count
                            .saturating_add(1);
                        self.cpu_extended_state_hardware.last_error = Some(error);
                    }
                }
            }
            self.processes
                .import_thread_cpu_extended_state_image(slot.owner_tid, slot.image)?;
            self.processes
                .mark_thread_cpu_extended_state_saved(flushed_tid, self.current_tick)?;
        }
        if let Some(previous) = previous_running {
            if previous.tid != scheduled.tid {
                self.processes
                    .mark_thread_cpu_extended_state_saved(previous.tid, self.current_tick)?;
            }
        }
        self.active_cpu_extended_state = self
            .processes
            .export_thread_cpu_extended_state_image(scheduled.tid)
            .ok()
            .map(|image| ActiveCpuExtendedStateSlot {
                owner_pid: scheduled.pid,
                owner_tid: scheduled.tid,
                image,
            });
        let restore_slot = self
            .active_cpu_extended_state
            .as_ref()
            .map(|slot| (slot.owner_pid, slot.owner_tid, slot.image.clone()));
        if let Some((owner_pid, owner_tid, image)) = restore_slot
            && let Some(provider) = self.hardware.as_mut()
        {
            match provider.restore_cpu_extended_state(owner_pid, owner_tid, &image) {
                Ok(()) => {
                    self.cpu_extended_state_hardware.restore_count = self
                        .cpu_extended_state_hardware
                        .restore_count
                        .saturating_add(1);
                    self.cpu_extended_state_hardware.last_restored_tid = Some(owner_tid);
                    self.cpu_extended_state_hardware.last_error = None;
                }
                Err(error) => {
                    self.cpu_extended_state_hardware.fallback_count = self
                        .cpu_extended_state_hardware
                        .fallback_count
                        .saturating_add(1);
                    self.cpu_extended_state_hardware.last_error = Some(error);
                }
            }
        }
        self.processes
            .mark_thread_cpu_extended_state_restored(scheduled.tid, self.current_tick)?;
        self.busy_ticks = self.busy_ticks.saturating_add(1);
        Ok(scheduled)
    }

    pub fn block_running(&mut self) -> Result<ProcessId, RuntimeError> {
        self.scheduler
            .block_running(&mut self.processes)
            .map_err(Into::into)
    }

    pub fn block_running_thread(
        &mut self,
        caller: ProcessId,
        tid: ThreadId,
    ) -> Result<ProcessId, RuntimeError> {
        let running = self
            .scheduler
            .running()
            .cloned()
            .ok_or(RuntimeError::Scheduler(SchedulerError::NoRunnableProcess))?;
        if running.pid != caller || running.tid != tid {
            let state = self.processes.get(caller)?.state();
            return Err(RuntimeError::Scheduler(
                SchedulerError::InvalidProcessState(state),
            ));
        }
        self.block_running()
    }

    pub fn exit(&mut self, pid: ProcessId, code: i32) -> Result<(), RuntimeError> {
        self.processes.exit(pid, code)?;
        self.scheduler.remove(&self.processes, pid)?;
        self.emit_process_lifecycle_events(pid, ProcessLifecycleEventKind::Exited)?;
        Ok(())
    }

    pub fn wake_process(
        &mut self,
        pid: ProcessId,
        class: SchedulerClass,
    ) -> Result<(), RuntimeError> {
        self.scheduler
            .wake(&mut self.processes, pid, class)
            .map_err(Into::into)
    }

    pub fn pause_process(&mut self, pid: ProcessId) -> Result<(), RuntimeError> {
        let state = self.processes.get(pid)?.state();
        if matches!(state, ProcessState::Blocked) {
            return Ok(());
        }
        if matches!(state, ProcessState::Exited) {
            return Err(RuntimeError::Scheduler(
                SchedulerError::InvalidProcessState(ProcessState::Exited),
            ));
        }
        self.scheduler.remove(&self.processes, pid)?;
        self.processes.set_state(pid, ProcessState::Blocked)?;
        Ok(())
    }

    pub fn resume_process(&mut self, pid: ProcessId) -> Result<(), RuntimeError> {
        let state = self.processes.get(pid)?.state();
        if matches!(state, ProcessState::Ready | ProcessState::Running) {
            return Ok(());
        }
        if matches!(state, ProcessState::Exited) {
            return Err(RuntimeError::Scheduler(
                SchedulerError::InvalidProcessState(ProcessState::Exited),
            ));
        }
        self.processes.set_state(pid, ProcessState::Ready)?;
        let policy = self.scheduler_policy_for_process(pid)?;
        self.scheduler.enqueue_with_budget(
            &mut self.processes,
            pid,
            policy.class,
            policy.budget,
        )?;
        Ok(())
    }

    pub fn renice_process(
        &mut self,
        pid: ProcessId,
        class: SchedulerClass,
        budget: u32,
    ) -> Result<(), RuntimeError> {
        self.processes
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?
            .set_scheduler_override(Some(SchedulerPolicyInfo {
                class,
                budget: budget.max(1),
            }));
        self.scheduler
            .rebind_process(&self.processes, pid, class, budget.max(1))?;
        Ok(())
    }

    pub fn set_process_affinity(
        &mut self,
        pid: ProcessId,
        affinity_mask: u64,
    ) -> Result<(), RuntimeError> {
        let tid = self
            .processes
            .get(pid)?
            .main_thread()
            .ok_or(RuntimeError::Scheduler(SchedulerError::InvalidPid))?;
        self.scheduler.set_thread_affinity(tid, affinity_mask)?;
        Ok(())
    }

    pub fn cpu_online(&self, cpu: usize) -> bool {
        self.scheduler.cpu_online(cpu)
    }

    pub fn cpu_online_count(&self) -> usize {
        self.scheduler.cpu_online_count()
    }

    pub fn logical_cpu_count(&self) -> usize {
        self.scheduler.logical_cpu_count()
    }

    pub fn set_cpu_online(&mut self, cpu: usize, online: bool) -> Result<(), RuntimeError> {
        self.scheduler.set_cpu_online(cpu, online)?;
        Ok(())
    }

    pub fn last_sleep_result(&self, pid: ProcessId) -> Option<SleepWaitResult> {
        self.sleep_results.get(&pid.raw()).copied()
    }

    pub fn send_signal(
        &mut self,
        sender: PendingSignalSender,
        pid: ProcessId,
        signal: u8,
    ) -> Result<(), RuntimeError> {
        signal_runtime::send_signal(self, sender, pid, signal)
    }

    pub fn send_signal_with_value(
        &mut self,
        sender: PendingSignalSender,
        pid: ProcessId,
        signal: u8,
        value: Option<u64>,
    ) -> Result<(), RuntimeError> {
        signal_runtime::send_signal_with_value(self, sender, pid, signal, value)
    }

    pub fn send_thread_signal(
        &mut self,
        sender: PendingSignalSender,
        pid: ProcessId,
        tid: ThreadId,
        signal: u8,
    ) -> Result<(), RuntimeError> {
        signal_runtime::send_thread_signal(self, sender, pid, tid, signal)
    }

    pub fn send_thread_signal_with_value(
        &mut self,
        sender: PendingSignalSender,
        pid: ProcessId,
        tid: ThreadId,
        signal: u8,
        value: Option<u64>,
    ) -> Result<(), RuntimeError> {
        signal_runtime::send_thread_signal_with_value(self, sender, pid, tid, signal, value)
    }

    pub fn pending_signals(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        signal_runtime::pending_signals(self, pid)
    }

    pub fn pending_thread_signals(
        &self,
        pid: ProcessId,
        tid: ThreadId,
    ) -> Result<Vec<u8>, RuntimeError> {
        signal_runtime::pending_thread_signals(self, pid, tid)
    }

    pub fn signal_mask(&self, pid: ProcessId) -> Result<u64, RuntimeError> {
        signal_runtime::signal_mask(self, pid)
    }

    pub fn blocked_pending_signals(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        signal_runtime::blocked_pending_signals(self, pid)
    }

    pub fn wait_for_pending_signal(
        &mut self,
        pid: ProcessId,
        mask: u64,
        timeout_ticks: Option<u64>,
    ) -> Result<PendingSignalWaitResult, RuntimeError> {
        signal_runtime::wait_for_pending_signal(self, pid, mask, timeout_ticks)
    }

    pub fn inspect_pending_signal_wait(
        &mut self,
        pid: ProcessId,
    ) -> Result<Option<PendingSignalWaitResume>, RuntimeError> {
        signal_runtime::inspect_pending_signal_wait(self, pid)
    }

    pub fn signal_disposition(
        &self,
        pid: ProcessId,
        signal: u8,
    ) -> Result<Option<SignalDisposition>, RuntimeError> {
        signal_runtime::signal_disposition(self, pid, signal)
    }

    pub fn signal_action_mask(&self, pid: ProcessId, signal: u8) -> Result<u64, RuntimeError> {
        signal_runtime::signal_action_mask(self, pid, signal)
    }

    pub fn signal_action_restart(&self, pid: ProcessId, signal: u8) -> Result<bool, RuntimeError> {
        signal_runtime::signal_action_restart(self, pid, signal)
    }

    #[allow(clippy::type_complexity)]
    pub fn set_signal_disposition(
        &mut self,
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
        signal_runtime::set_signal_disposition(self, pid, signal, disposition, mask, restart)
    }

    pub fn set_signal_mask(
        &mut self,
        pid: ProcessId,
        how: SignalMaskHow,
        mask: u64,
    ) -> Result<(u64, u64), RuntimeError> {
        signal_runtime::set_signal_mask(self, pid, how, mask)
    }

    pub fn take_pending_signal(
        &mut self,
        pid: ProcessId,
        mask: u64,
        blocked_only: bool,
    ) -> Result<Option<u8>, RuntimeError> {
        signal_runtime::take_pending_signal(self, pid, mask, blocked_only)
    }

    pub fn exit_running(&mut self, code: i32) -> Result<ProcessId, RuntimeError> {
        self.scheduler
            .exit_running(&mut self.processes, code)
            .map_err(Into::into)
    }

    pub fn exit_running_thread(
        &mut self,
        caller: ProcessId,
        tid: ThreadId,
        code: i32,
    ) -> Result<ProcessId, RuntimeError> {
        let running = self
            .scheduler
            .running()
            .cloned()
            .ok_or(RuntimeError::Scheduler(SchedulerError::NoRunnableProcess))?;
        if running.pid != caller || running.tid != tid {
            let state = self.processes.get(caller)?.state();
            return Err(RuntimeError::Scheduler(
                SchedulerError::InvalidProcessState(state),
            ));
        }
        self.exit_running(code)
    }

    pub(crate) fn finalize_queue_descriptor_close(
        &mut self,
        descriptor: &ObjectDescriptor,
    ) -> Result<(), RuntimeError> {
        descriptor_runtime::finalize_queue_descriptor_close(self, descriptor)
    }

    pub(crate) fn queue_descriptor_reference_count(&self, binding: QueueDescriptorTarget) -> usize {
        descriptor_runtime::queue_descriptor_reference_count(self, binding)
    }

    pub(crate) fn event_queue_binding(
        &self,
        owner: ProcessId,
        queue: EventQueueId,
    ) -> Result<QueueDescriptorTarget, RuntimeError> {
        self.processes.get(owner)?;
        let mode = self
            .event_queues
            .iter()
            .find(|candidate| candidate.owner == owner && candidate.id == queue)
            .map(|queue| queue.mode)
            .ok_or(EventQueueError::InvalidQueue)?;
        Ok(QueueDescriptorTarget::Event { owner, queue, mode })
    }

    pub(crate) fn sleep_queue_binding(
        &self,
        owner: ProcessId,
        queue: SleepQueueId,
    ) -> Result<QueueDescriptorTarget, RuntimeError> {
        self.processes.get(owner)?;
        self.sleep_queues
            .iter()
            .any(|candidate| candidate.owner == owner && candidate.id == queue)
            .then_some(QueueDescriptorTarget::Sleep { owner, queue })
            .ok_or(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound))
    }

    pub(crate) fn event_queue_binding_for_fd(
        &self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<QueueDescriptorTarget, RuntimeError> {
        let descriptor = self.namespace(owner)?.get(fd).map_err(RuntimeError::from)?;
        match descriptor.queue_binding() {
            Some(binding @ QueueDescriptorTarget::Event { .. }) => Ok(binding),
            _ => Err(RuntimeError::EventQueue(EventQueueError::InvalidQueue)),
        }
    }

    pub(crate) fn sleep_queue_binding_for_fd(
        &self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<QueueDescriptorTarget, RuntimeError> {
        let descriptor = self.namespace(owner)?.get(fd).map_err(RuntimeError::from)?;
        match descriptor.queue_binding() {
            Some(binding @ QueueDescriptorTarget::Sleep { .. }) => Ok(binding),
            _ => Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound)),
        }
    }

    pub(crate) fn event_queue_mode(
        &self,
        owner: ProcessId,
        queue: EventQueueId,
    ) -> Option<EventQueueMode> {
        self.event_queues
            .iter()
            .find(|candidate| candidate.owner == owner && candidate.id == queue)
            .map(|queue| queue.mode)
    }

    pub(crate) fn sleep_queue_exists(
        &self,
        owner: ProcessId,
        queue: SleepQueueId,
    ) -> Result<(), RuntimeError> {
        self.sleep_queues
            .iter()
            .any(|candidate| candidate.owner == owner && candidate.id == queue)
            .then_some(())
            .ok_or(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound))
    }

    pub(crate) fn remove_event_queue_record(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
    ) -> Result<(), RuntimeError> {
        let index = self
            .event_queues
            .iter()
            .position(|candidate| candidate.id == queue && candidate.owner == owner)
            .ok_or(EventQueueError::InvalidQueue)?;
        let mut removed = self.event_queues.remove(index);
        for waiter in removed.drain_waiters() {
            let _ = self
                .scheduler
                .wake(&mut self.processes, waiter.owner, waiter.class);
        }
        self.purge_event_queue_runtime_state(owner, queue);
        Ok(())
    }

    pub(crate) fn remove_sleep_queue_record(
        &mut self,
        owner: ProcessId,
        queue: SleepQueueId,
    ) -> Result<Vec<ProcessId>, RuntimeError> {
        let index = self
            .sleep_queues
            .iter()
            .position(|candidate| candidate.id == queue && candidate.owner == owner)
            .ok_or(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound))?;
        let mut removed = self.sleep_queues.remove(index);
        self.signal_wait_queues
            .retain(|_, candidate| *candidate != queue);
        self.memory_wait_queues
            .retain(|_, candidate| *candidate != queue);
        self.memory_waiters.retain(|_, waiters| {
            waiters.retain(|waiter| waiter.queue != queue);
            !waiters.is_empty()
        });

        let mut woke = Vec::new();
        let mut blocked_owners = Vec::new();
        for waiter in removed.waiters.waiters() {
            if !blocked_owners.contains(&waiter.owner) {
                blocked_owners.push(waiter.owner);
            }
        }
        for blocked_owner in blocked_owners {
            for waiter in removed.waiters.cancel_owner(blocked_owner) {
                self.sleep_results.insert(waiter.owner.raw(), waiter.result);
                self.scheduler.wake(
                    &mut self.processes,
                    waiter.owner,
                    scheduler_class_from_hint(waiter.wake_hint),
                )?;
                woke.push(waiter.owner);
            }
        }
        Ok(woke)
    }

    pub fn thread_infos(&self, pid: ProcessId) -> Result<Vec<ThreadInfo>, RuntimeError> {
        let mut threads = self
            .processes
            .threads_for_process(pid)?
            .into_iter()
            .map(|tid| {
                let thread = self.processes.get_thread(tid)?;
                Ok(ThreadInfo {
                    tid: thread.tid(),
                    owner: thread.owner(),
                    name: thread.name().to_string(),
                    state: thread.state(),
                    is_main: thread.is_main(),
                    exit_code: thread.exit_code(),
                    cpu_extended_state: thread.cpu_extended_state(),
                })
            })
            .collect::<Result<Vec<_>, ProcessError>>()?;
        threads.sort_by_key(|thread| thread.tid.raw());
        Ok(threads)
    }

    pub fn restore_thread_cpu_extended_state_boot_seed(
        &mut self,
        pid: ProcessId,
        tid: ThreadId,
    ) -> Result<(), RuntimeError> {
        let thread = self.processes.get_thread(tid)?;
        if thread.owner() != pid {
            return Err(RuntimeError::Process(ProcessError::InvalidTid));
        }
        self.processes
            .restore_thread_cpu_extended_state_boot_seed(tid)?;
        Ok(())
    }

    pub fn export_thread_cpu_extended_state_image(
        &self,
        pid: ProcessId,
        tid: ThreadId,
    ) -> Result<ThreadCpuExtendedStateImage, RuntimeError> {
        let thread = self.processes.get_thread(tid)?;
        if thread.owner() != pid {
            return Err(RuntimeError::Process(ProcessError::InvalidTid));
        }
        self.processes
            .export_thread_cpu_extended_state_image(tid)
            .map_err(Into::into)
    }

    pub fn import_thread_cpu_extended_state_image(
        &mut self,
        pid: ProcessId,
        tid: ThreadId,
        image: ThreadCpuExtendedStateImage,
    ) -> Result<(), RuntimeError> {
        let thread = self.processes.get_thread(tid)?;
        if thread.owner() != pid {
            return Err(RuntimeError::Process(ProcessError::InvalidTid));
        }
        self.processes
            .import_thread_cpu_extended_state_image(tid, image)
            .map_err(Into::into)
    }

    pub fn clone_thread_cpu_extended_state_image(
        &mut self,
        source_pid: ProcessId,
        source_tid: ThreadId,
        target_pid: ProcessId,
        target_tid: ThreadId,
    ) -> Result<ThreadCpuExtendedStateImage, RuntimeError> {
        let image = self.export_thread_cpu_extended_state_image(source_pid, source_tid)?;
        self.import_thread_cpu_extended_state_image(target_pid, target_tid, image.clone())?;
        Ok(image)
    }

    pub fn release_thread_cpu_extended_state_image(
        &mut self,
        pid: ProcessId,
        tid: ThreadId,
    ) -> Result<(), RuntimeError> {
        let thread = self.processes.get_thread(tid)?;
        if thread.owner() != pid {
            return Err(RuntimeError::Process(ProcessError::InvalidTid));
        }
        self.processes
            .release_thread_cpu_extended_state_image(tid)
            .map_err(Into::into)
    }

    pub fn open_descriptor(
        &mut self,
        owner: ProcessId,
        capability: CapabilityId,
        kind: ObjectKind,
        name: impl Into<String>,
    ) -> Result<Descriptor, RuntimeError> {
        descriptor_runtime::open_descriptor(self, owner, capability, kind, name)
    }

    pub fn duplicate_descriptor(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<Descriptor, RuntimeError> {
        descriptor_runtime::duplicate_descriptor(self, owner, fd)
    }

    pub fn duplicate_descriptor_to(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        target: Descriptor,
    ) -> Result<Descriptor, RuntimeError> {
        descriptor_runtime::duplicate_descriptor_to(self, owner, fd, target)
    }

    pub fn close_descriptor(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<ObjectDescriptor, RuntimeError> {
        descriptor_runtime::close_descriptor(self, owner, fd)
    }

    pub fn stat_path(&self, path: &str) -> Result<FileStatus, RuntimeError> {
        let node = self
            .vfs
            .resolve_metadata_node(path)
            .map_err(RuntimeError::from)?;
        Ok(FileStatus {
            inode: node.inode(),
            kind: node.kind(),
            size: node
                .link_target()
                .map(|target| target.len() as u64)
                .unwrap_or_else(|| {
                    if node.kind() == ObjectKind::File {
                        node.content().len() as u64
                    } else {
                        initial_payload_for_kind(node.kind(), node.path()).len() as u64
                    }
                }),
            path: node.path().to_string(),
            cloexec: false,
            nonblock: false,
            readable: io_capabilities_for_kind(node.kind()).contains(IoCapabilities::READ),
            writable: io_capabilities_for_kind(node.kind()).contains(IoCapabilities::WRITE),
        })
    }

    pub fn lstat_path(&self, path: &str) -> Result<FileStatus, RuntimeError> {
        let node = self.vfs.node(path).map_err(RuntimeError::from)?;
        Ok(FileStatus {
            inode: node.inode(),
            kind: node.kind(),
            size: node
                .link_target()
                .map(|target| target.len() as u64)
                .unwrap_or_else(|| {
                    if node.kind() == ObjectKind::File {
                        node.content().len() as u64
                    } else {
                        initial_payload_for_kind(node.kind(), node.path()).len() as u64
                    }
                }),
            path: node.path().to_string(),
            cloexec: false,
            nonblock: false,
            readable: io_capabilities_for_kind(node.kind()).contains(IoCapabilities::READ),
            writable: io_capabilities_for_kind(node.kind()).contains(IoCapabilities::WRITE),
        })
    }

    pub fn readlink_path(&self, path: &str) -> Result<String, RuntimeError> {
        self.vfs
            .readlink(path)
            .map(str::to_string)
            .map_err(RuntimeError::from)
    }

    pub fn fstat_descriptor(
        &self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<FileStatus, RuntimeError> {
        let descriptor = self.namespace(owner)?.get(fd).map_err(RuntimeError::from)?;
        let io = self.inspect_io(owner, fd)?;
        let inode = self
            .vfs
            .node(descriptor.name())
            .map(|node| node.inode())
            .unwrap_or(0);
        Ok(FileStatus {
            inode,
            kind: descriptor.kind(),
            size: io.payload().len() as u64,
            path: descriptor.name().to_string(),
            cloexec: descriptor.cloexec(),
            nonblock: descriptor.nonblock(),
            readable: io.capabilities().contains(IoCapabilities::READ),
            writable: io.capabilities().contains(IoCapabilities::WRITE),
        })
    }

    pub fn statfs(&self, path: &str) -> Result<FileSystemStatus, RuntimeError> {
        let mount = self.vfs.statfs(path).map_err(RuntimeError::from)?;
        Ok(FileSystemStatus {
            mount_count: self.vfs.mounts().len(),
            node_count: self.vfs.nodes().len(),
            path: mount.mount_path().to_string(),
            mount_name: mount.name().to_string(),
            read_only: false,
        })
    }

    pub fn list_path(&self, path: &str) -> Result<Vec<u8>, RuntimeError> {
        let nodes = self.vfs.list_directory(path).map_err(RuntimeError::from)?;
        let mut out = Vec::new();
        for node in nodes {
            let name = node
                .path()
                .rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .unwrap_or("/");
            out.extend_from_slice(name.as_bytes());
            out.push(b'\t');
            out.extend_from_slice(format!("{:?}", node.kind()).as_bytes());
            out.push(b'\n');
        }
        Ok(out)
    }

    pub fn filedesc_entries(&self, owner: ProcessId) -> Result<Vec<FiledescEntry>, RuntimeError> {
        let mut entries = Vec::new();
        let namespace = self.namespace(owner)?;
        for fd in namespace.by_owner(owner) {
            let descriptor = namespace.get(fd).map_err(RuntimeError::from)?;
            let io = self.inspect_io(owner, fd)?;
            let inode = self
                .vfs
                .node(descriptor.name())
                .map(|node| node.inode())
                .unwrap_or(0);
            entries.push(FiledescEntry {
                fd,
                kind: descriptor.kind(),
                kind_code: filedesc_kind_code(descriptor.kind()),
                path: descriptor.name().to_string(),
                inode,
                size: io.payload().len() as u64,
                readable: io.capabilities().contains(IoCapabilities::READ),
                writable: io.capabilities().contains(IoCapabilities::WRITE),
                capability_bits: io.capabilities().bits(),
                flags: descriptor.flags(),
            });
        }
        Ok(entries)
    }

    pub fn kinfo_file_entries(
        &self,
        owner: ProcessId,
    ) -> Result<Vec<KinfoFileEntry>, RuntimeError> {
        let namespace = self.namespace(owner)?;
        let mut entries = Vec::new();
        for fd in namespace.by_owner(owner) {
            let descriptor = namespace.get(fd).map_err(RuntimeError::from)?;
            let io = self.inspect_io(owner, fd)?;
            let inode = self
                .vfs
                .node(descriptor.name())
                .map(|node| node.inode())
                .unwrap_or(0);
            let rights = self
                .capabilities
                .get(descriptor.capability())
                .map(|capability| capability.rights())
                .unwrap_or(CapabilityRights::empty());
            entries.push(KinfoFileEntry {
                struct_size: 96,
                kind_code: filedesc_kind_code(descriptor.kind()),
                fd,
                ref_count: self.filedesc_ref_count(owner),
                flags: descriptor.flags(),
                offset: io.cursor() as u64,
                status: kinfo_status(io.state()),
                rights,
                path: descriptor.name().to_string(),
                inode,
                size: io.payload().len() as u64,
                socket_domain: socket_domain_for_kind(descriptor.kind()),
                socket_type: socket_type_for_kind(descriptor.kind()),
                socket_protocol: socket_protocol_for_kind(descriptor.kind()),
            });
        }
        Ok(entries)
    }

    pub fn exec_transition(
        &mut self,
        owner: ProcessId,
    ) -> Result<Vec<ObjectDescriptor>, RuntimeError> {
        descriptor_runtime::exec_transition(self, owner)
    }

    pub fn mount(
        &mut self,
        mount_path: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<(), RuntimeError> {
        self.vfs.mount(mount_path, name).map_err(Into::into)
    }

    pub fn create_vfs_node(
        &mut self,
        path: impl Into<String>,
        kind: ObjectKind,
        capability: CapabilityId,
    ) -> Result<(), RuntimeError> {
        let path = path.into();
        self.vfs
            .create_node(path.clone(), kind, capability)
            .map_err(RuntimeError::from)?;
        self.ensure_endpoint_registered_for_node(&path, kind, capability)?;
        Ok(())
    }

    pub fn create_owned_vfs_node(
        &mut self,
        owner: ProcessId,
        path: impl Into<String>,
        kind: ObjectKind,
    ) -> Result<(), RuntimeError> {
        let path = path.into();
        let capability = self.grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            path.clone(),
        )?;
        self.create_vfs_node(path, kind, capability)
    }

    pub fn create_vfs_symlink(
        &mut self,
        path: impl Into<String>,
        target: impl Into<String>,
        capability: CapabilityId,
    ) -> Result<(), RuntimeError> {
        self.vfs
            .create_symlink(path, target, capability)
            .map_err(RuntimeError::from)
    }

    pub fn create_owned_vfs_symlink(
        &mut self,
        owner: ProcessId,
        path: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<(), RuntimeError> {
        let path = path.into();
        let capability = self.grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            path.clone(),
        )?;
        self.create_vfs_symlink(path, target, capability)
    }

    pub fn unlink_path(&mut self, path: &str) -> Result<(), RuntimeError> {
        self.vfs.remove_node(path).map_err(RuntimeError::from)?;
        self.retire_endpoint_for_path(path);
        Ok(())
    }

    pub fn rename_path(&mut self, from: &str, to: &str) -> Result<(), RuntimeError> {
        self.vfs.rename_node(from, to).map_err(RuntimeError::from)?;
        self.rename_endpoint_path(from, to);
        Ok(())
    }

    pub fn open_path(&mut self, owner: ProcessId, path: &str) -> Result<Descriptor, RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_runtime::open_path(self, owner, path)
    }

    pub fn inspect_io(&self, owner: ProcessId, fd: Descriptor) -> Result<&IoObject, RuntimeError> {
        descriptor_io_runtime::inspect_io(self, owner, fd)
    }

    pub fn inspect_io_layout(
        &self,
        owner: ProcessId,
        fd: Descriptor,
    ) -> Result<IoPayloadLayoutInfo, RuntimeError> {
        descriptor_io_runtime::inspect_io_layout(self, owner, fd)
    }

    pub fn inspect_vm_object_layouts(
        &self,
        pid: ProcessId,
    ) -> Result<Vec<VmObjectLayoutInfo>, RuntimeError> {
        let objects = self.processes.vm_objects_for_process(pid)?;
        Ok(objects
            .into_iter()
            .map(|object| object.layout_info())
            .collect())
    }

    pub fn resolve_vm_object_id(
        &self,
        pid: ProcessId,
        addr: u64,
        length: u64,
    ) -> Result<u64, RuntimeError> {
        self.processes
            .vm_object_id_for_address(pid, addr, length)
            .map_err(Into::into)
    }

    pub fn read_io(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        len: usize,
    ) -> Result<Vec<u8>, RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_io_runtime::read_io(self, owner, fd, len)
    }

    pub fn read_io_vectored(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        segments: &[usize],
    ) -> Result<Vec<Vec<u8>>, RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_io_runtime::read_io_vectored(self, owner, fd, segments)
    }

    pub fn read_io_vectored_with_layout(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        segments: &[usize],
    ) -> Result<(Vec<Vec<u8>>, IoPayloadLayoutInfo), RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_io_runtime::read_io_vectored_with_layout(self, owner, fd, segments)
    }

    pub fn write_io(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        bytes: &[u8],
    ) -> Result<usize, RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_io_runtime::write_io(self, owner, fd, bytes)
    }

    pub fn write_io_vectored(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        segments: &[Vec<u8>],
    ) -> Result<usize, RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_io_runtime::write_io_vectored(self, owner, fd, segments)
    }

    pub fn poll_io(&self, owner: ProcessId, fd: Descriptor) -> Result<IoPollEvents, RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_io_runtime::poll_io(self, owner, fd)
    }

    pub fn control_io(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        opcode: u32,
    ) -> Result<u32, RuntimeError> {
        self.enforce_process_io_contract(owner)?;
        descriptor_io_runtime::control_io(self, owner, fd, opcode)
    }

    pub fn fcntl(
        &mut self,
        owner: ProcessId,
        fd: Descriptor,
        cmd: FcntlCmd,
    ) -> Result<FcntlResult, RuntimeError> {
        descriptor_io_runtime::fcntl(self, owner, fd, cmd)
    }
}
