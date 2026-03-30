use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::Runtime;
pub use ngos_semantic_runtime::{
    AdaptiveState, AdaptiveStateSnapshot, CognitiveTier, ComputeMode, CpuLoadStats, CpuMask,
    EventSemantic, PressureState, SemanticActionRecord, SemanticCapability, SemanticClass,
    SemanticContext, SemanticContextEntry, SemanticCpuTopologyEntry, SemanticDecisionPlan,
    SemanticDiagnostics, SemanticEntity, SemanticEntityKind, SemanticFeedbackEntry,
    SemanticFeedbackStore, SemanticObservation, SemanticPolicyView, SemanticSystemState,
    SemanticTopologySnapshot, SemanticVerdict, SystemPressureMetrics, cpu_mask_for, load_percent,
    pressure_channel_name, select_cpu, semantic_capabilities_csv, semantic_class_name,
    semantic_entity_kind_name, semantic_for_channel, semantic_verdict_name,
};
use ngos_user_abi::{
    Errno, NativeContractRecord, NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind,
    NativeNetworkInterfaceRecord, NativeNetworkSocketRecord, NativeProcessRecord,
    NativeResourceRecord, NativeSchedulerClass, NativeSystemSnapshotRecord, PollEvents,
    SyscallBackend,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessHandle {
    pub pid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadHandle {
    pub tid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketHandle {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceHandle {
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityToken {
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceContract {
    pub id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEntity {
    pub handle: ProcessHandle,
    pub thread: Option<ThreadHandle>,
    pub name: String,
    pub image_path: String,
    pub cwd: String,
    pub record: NativeProcessRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntity {
    pub handle: DeviceHandle,
    pub record: Option<NativeNetworkInterfaceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketEntity {
    pub handle: SocketHandle,
    pub record: NativeNetworkSocketRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemFact {
    Process(ProcessEntity),
    Device(DeviceEntity),
    Socket(SocketEntity),
    Resource {
        id: usize,
        record: NativeResourceRecord,
    },
    Contract {
        id: usize,
        record: NativeContractRecord,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventFilter {
    Process {
        pid: u64,
        token: CapabilityToken,
        exited: bool,
        reaped: bool,
        poll_events: PollEvents,
    },
    Resource {
        resource: usize,
        token: CapabilityToken,
        claimed: bool,
        queued: bool,
        canceled: bool,
        released: bool,
        handed_off: bool,
        revoked: bool,
        poll_events: PollEvents,
    },
    Network {
        interface_path: String,
        socket_path: Option<String>,
        token: CapabilityToken,
        link_changed: bool,
        rx_ready: bool,
        tx_drained: bool,
        poll_events: PollEvents,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUpdate {
    Activate,
    Suspend,
    Retire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessAction {
    Pause,
    Resume,
    Kill {
        signal: u8,
    },
    Renice {
        class: NativeSchedulerClass,
        budget: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStream {
    pub queue_fd: usize,
    pub filter: EventFilter,
}

pub struct SystemController<'a, B> {
    runtime: &'a Runtime<B>,
}

type ProcessTextReader<B> = fn(&Runtime<B>, u64, &mut [u8]) -> Result<usize, Errno>;

impl<'a, B> SystemController<'a, B> {
    pub const fn new(runtime: &'a Runtime<B>) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &'a Runtime<B> {
        self.runtime
    }
}

impl<'a, B: SyscallBackend> SystemController<'a, B> {
    pub fn spawn_process(&self, name: &str, path: &str) -> Result<ProcessHandle, Errno> {
        self.runtime
            .spawn_path_process(name, path)
            .map(|pid| ProcessHandle { pid })
    }

    pub fn act_on_process(
        &self,
        handle: ProcessHandle,
        action: ProcessAction,
    ) -> Result<(), Errno> {
        match action {
            ProcessAction::Pause => self.runtime.pause_process(handle.pid),
            ProcessAction::Resume => self.runtime.resume_process(handle.pid),
            ProcessAction::Kill { signal } => self.runtime.send_signal(handle.pid, signal),
            ProcessAction::Renice { class, budget } => {
                self.runtime.renice_process(handle.pid, class, budget)
            }
        }
    }

    pub fn configure_interface_ipv4(
        &self,
        handle: &DeviceHandle,
        addr: [u8; 4],
        netmask: [u8; 4],
        gateway: [u8; 4],
    ) -> Result<(), Errno> {
        self.runtime
            .configure_network_interface_ipv4(&handle.path, addr, netmask, gateway)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure_interface_admin(
        &self,
        handle: &DeviceHandle,
        mtu: usize,
        tx_capacity: usize,
        rx_capacity: usize,
        tx_inflight_limit: usize,
        admin_up: bool,
        promiscuous: bool,
    ) -> Result<(), Errno> {
        self.runtime.configure_network_interface_admin(
            &handle.path,
            mtu,
            tx_capacity,
            rx_capacity,
            tx_inflight_limit,
            admin_up,
            promiscuous,
        )
    }

    pub fn observe_socket(&self, handle: &SocketHandle) -> Result<SocketEntity, Errno> {
        let record = self.runtime.inspect_network_socket(&handle.path)?;
        Ok(SocketEntity {
            handle: handle.clone(),
            record,
        })
    }

    pub fn enumerate_devices(&self) -> Result<Vec<DeviceHandle>, Errno> {
        let entries = self.list_path_entries("/dev")?;
        Ok(entries
            .into_iter()
            .map(|entry| DeviceHandle {
                path: format!("/dev/{entry}"),
            })
            .collect())
    }

    pub fn device_stats(&self, handle: &DeviceHandle) -> Result<DeviceEntity, Errno> {
        let record = self.runtime.inspect_network_interface(&handle.path).ok();
        Ok(DeviceEntity {
            handle: handle.clone(),
            record,
        })
    }

    pub fn query_resources(&self) -> Result<Vec<NativeResourceRecord>, Errno> {
        let mut ids = vec![0u64; 64];
        let count = self.runtime.list_resources(&mut ids)?;
        ids.truncate(count);
        ids.into_iter()
            .map(|id| self.runtime.inspect_resource(id as usize))
            .collect()
    }

    pub fn update_resource(
        &self,
        contract: ResourceContract,
        action: ResourceUpdate,
    ) -> Result<(), Errno> {
        let state = match action {
            ResourceUpdate::Activate => ngos_user_abi::NativeResourceState::Active,
            ResourceUpdate::Suspend => ngos_user_abi::NativeResourceState::Suspended,
            ResourceUpdate::Retire => ngos_user_abi::NativeResourceState::Retired,
        };
        let record = self.runtime.inspect_contract(contract.id)?;
        self.runtime
            .set_resource_state(record.resource as usize, state)
    }

    pub fn subscribe(&self, filter: EventFilter) -> Result<EventStream, Errno> {
        let queue_fd = self
            .runtime
            .create_event_queue(NativeEventQueueMode::Kqueue)?;
        match &filter {
            EventFilter::Process {
                pid,
                token,
                exited,
                reaped,
                poll_events,
            } => self.runtime.watch_process_events(
                queue_fd,
                *pid,
                token.value,
                *exited,
                *reaped,
                *poll_events,
            )?,
            EventFilter::Resource {
                resource,
                token,
                claimed,
                queued,
                canceled,
                released,
                handed_off,
                revoked,
                poll_events,
            } => self.runtime.watch_resource_events(
                queue_fd,
                *resource,
                token.value,
                *claimed,
                *queued,
                *canceled,
                *released,
                *handed_off,
                *revoked,
                *poll_events,
            )?,
            EventFilter::Network {
                interface_path,
                socket_path,
                token,
                link_changed,
                rx_ready,
                tx_drained,
                poll_events,
            } => self.runtime.watch_network_events(
                queue_fd,
                interface_path,
                socket_path.as_deref(),
                token.value,
                *link_changed,
                *rx_ready,
                *tx_drained,
                *poll_events,
            )?,
        }
        Ok(EventStream { queue_fd, filter })
    }

    pub fn observe_pressure(
        &self,
        previous: Option<&NativeSystemSnapshotRecord>,
    ) -> Result<SystemPressureMetrics, Errno> {
        let snapshot = self.runtime.inspect_system_snapshot()?;
        let cpu_utilization_pct = match previous {
            Some(previous) if snapshot.current_tick > previous.current_tick => {
                let tick_delta = snapshot.current_tick - previous.current_tick;
                let busy_delta = snapshot.busy_ticks.saturating_sub(previous.busy_ticks);
                ((busy_delta.saturating_mul(100)) / tick_delta.max(1)) as u32
            }
            _ => u32::from(snapshot.running_pid != 0) * 100,
        };
        let socket_pressure_pct = snapshot
            .total_socket_rx_depth
            .saturating_mul(100)
            .checked_div(snapshot.total_socket_rx_limit)
            .unwrap_or(0) as u32;
        let event_capacity = snapshot.total_event_queue_count.saturating_mul(64);
        let event_queue_pressure_pct = snapshot
            .total_event_queue_pending
            .saturating_mul(100)
            .checked_div(event_capacity)
            .unwrap_or(0) as u32;
        Ok(SystemPressureMetrics {
            run_queue_total: snapshot.queued_processes,
            run_queue_latency_critical: snapshot.queued_latency_critical,
            run_queue_interactive: snapshot.queued_interactive,
            run_queue_normal: snapshot.queued_normal,
            run_queue_background: snapshot.queued_background,
            snapshot,
            cpu_utilization_pct,
            socket_pressure_pct,
            event_queue_pressure_pct,
            tx_drop_delta: previous
                .filter(|previous| snapshot.current_tick > previous.current_tick)
                .map(|previous| {
                    snapshot
                        .total_network_tx_dropped
                        .saturating_sub(previous.total_network_tx_dropped)
                })
                .unwrap_or(snapshot.total_network_tx_dropped),
            rx_drop_delta: previous
                .filter(|previous| snapshot.current_tick > previous.current_tick)
                .map(|previous| {
                    snapshot
                        .total_network_rx_dropped
                        .saturating_sub(previous.total_network_rx_dropped)
                })
                .unwrap_or(snapshot.total_network_rx_dropped),
        })
    }

    pub fn classify_pressure(&self, metrics: &SystemPressureMetrics) -> PressureState {
        let high_scheduler_pressure =
            metrics.run_queue_total >= 3 && metrics.cpu_utilization_pct >= 75;
        let network_backpressure = metrics.snapshot.saturated_socket_count > 0
            || metrics.socket_pressure_pct >= 80
            || metrics.rx_drop_delta > 0
            || metrics.tx_drop_delta > 0
            || metrics.event_queue_pressure_pct >= 75;
        match (high_scheduler_pressure, network_backpressure) {
            (true, true) => PressureState::MixedPressure,
            (true, false) => PressureState::HighSchedulerPressure,
            (false, true) => PressureState::NetworkBackpressure,
            (false, false) => PressureState::Stable,
        }
    }

    pub fn observe_semantic_state(
        &self,
        previous: Option<&NativeSystemSnapshotRecord>,
        adaptive_state: &mut AdaptiveState,
    ) -> Result<SemanticSystemState, Errno> {
        let metrics = self.observe_pressure(previous)?;
        let pressure = self.classify_pressure(&metrics);
        let observation = SemanticObservation {
            cpu_load: metrics.cpu_utilization_pct.min(100) as u16,
            mem_pressure: metrics.socket_pressure_pct.min(100) as u16,
            anomaly_score: self.anomaly_score(&metrics).min(100) as u16,
            thermal_c: self.thermal_proxy(&metrics),
        };
        adaptive_state.record(&observation);
        let channel = pressure_channel_name(pressure).to_string();
        Ok(SemanticSystemState {
            semantic: semantic_for_channel(&channel),
            channel,
            pressure,
            observation,
            adaptive: adaptive_state.snapshot(),
            metrics,
        })
    }

    fn anomaly_score(&self, metrics: &SystemPressureMetrics) -> u32 {
        let mut anomaly = 0u32;
        if metrics.run_queue_total >= 3 {
            anomaly = anomaly.saturating_add(25);
        }
        if metrics.cpu_utilization_pct >= 75 {
            anomaly = anomaly.saturating_add(25);
        }
        if metrics.socket_pressure_pct >= 80 {
            anomaly = anomaly.saturating_add(20);
        }
        if metrics.event_queue_pressure_pct >= 75 {
            anomaly = anomaly.saturating_add(15);
        }
        if metrics.rx_drop_delta > 0 || metrics.tx_drop_delta > 0 {
            anomaly = anomaly.saturating_add(20);
        }
        anomaly.min(100)
    }

    fn thermal_proxy(&self, metrics: &SystemPressureMetrics) -> i16 {
        let thermal = 35u32
            .saturating_add(metrics.cpu_utilization_pct / 2)
            .saturating_add(metrics.event_queue_pressure_pct / 4);
        thermal.min(110) as i16
    }

    pub fn poll_events(
        &self,
        stream: &EventStream,
        buffer: &mut [NativeEventRecord],
    ) -> Result<usize, Errno> {
        self.runtime.wait_event_queue(stream.queue_fd, buffer)
    }

    pub fn collect_facts(&self) -> Result<Vec<SystemFact>, Errno> {
        let mut facts = Vec::new();

        let mut pids = vec![0u64; 64];
        let process_count = self.runtime.list_processes(&mut pids)?;
        pids.truncate(process_count);
        for pid in pids {
            let record = self.runtime.inspect_process(pid)?;
            let name = self.read_process_text(pid, Runtime::get_process_name)?;
            let image_path = self.read_process_text(pid, Runtime::get_process_image_path)?;
            let cwd = self.read_process_text(pid, Runtime::get_process_cwd)?;
            facts.push(SystemFact::Process(ProcessEntity {
                handle: ProcessHandle { pid },
                thread: (record.main_thread != 0).then_some(ThreadHandle {
                    tid: record.main_thread,
                }),
                name,
                image_path,
                cwd,
                record,
            }));
        }

        for handle in self.enumerate_devices()? {
            facts.push(SystemFact::Device(self.device_stats(&handle)?));
        }

        for resource in self.query_resources()? {
            facts.push(SystemFact::Resource {
                id: resource.id as usize,
                record: resource,
            });
        }

        let mut contracts = vec![0u64; 64];
        let count = self.runtime.list_contracts(&mut contracts)?;
        contracts.truncate(count);
        for contract in contracts {
            facts.push(SystemFact::Contract {
                id: contract as usize,
                record: self.runtime.inspect_contract(contract as usize)?,
            });
        }

        Ok(facts)
    }

    pub fn collect_semantic_entities(&self) -> Result<Vec<SemanticEntity>, Errno> {
        let facts = self.collect_facts()?;
        let mut entities = Vec::new();
        for fact in facts {
            match fact {
                SystemFact::Process(process) => entities.push(SemanticEntity {
                    kind: SemanticEntityKind::Process,
                    subject: format!("process:{}", process.handle.pid),
                    semantic: semantic_for_channel("proc::entity"),
                    policy: SemanticPolicyView {
                        cpu_mask: u64::MAX,
                        policy_fingerprint: process_policy_fingerprint(&process),
                    },
                }),
                SystemFact::Device(device) => entities.push(SemanticEntity {
                    kind: SemanticEntityKind::Device,
                    subject: format!("device:{}", device.handle.path),
                    semantic: semantic_for_channel("dev::entity"),
                    policy: SemanticPolicyView {
                        cpu_mask: u64::MAX,
                        policy_fingerprint: device_policy_fingerprint(&device),
                    },
                }),
                SystemFact::Socket(socket) => entities.push(SemanticEntity {
                    kind: SemanticEntityKind::Socket,
                    subject: format!("socket:{}", socket.handle.path),
                    semantic: semantic_for_channel("dev::network-socket"),
                    policy: SemanticPolicyView {
                        cpu_mask: u64::MAX,
                        policy_fingerprint: socket_policy_fingerprint(&socket),
                    },
                }),
                SystemFact::Resource { id, record } => entities.push(SemanticEntity {
                    kind: SemanticEntityKind::Resource,
                    subject: format!("resource:{id}"),
                    semantic: semantic_for_channel("proc::resource"),
                    policy: SemanticPolicyView {
                        cpu_mask: u64::MAX,
                        policy_fingerprint: resource_policy_fingerprint(&record),
                    },
                }),
                SystemFact::Contract { id, record } => entities.push(SemanticEntity {
                    kind: SemanticEntityKind::Contract,
                    subject: format!("contract:{id}"),
                    semantic: semantic_for_channel("proc::contract"),
                    policy: SemanticPolicyView {
                        cpu_mask: u64::MAX,
                        policy_fingerprint: contract_policy_fingerprint(&record),
                    },
                }),
            }
        }
        Ok(entities)
    }

    pub fn observe_topology(
        &self,
        previous: Option<&NativeSystemSnapshotRecord>,
    ) -> Result<SemanticTopologySnapshot, Errno> {
        let snapshot = self.runtime.inspect_system_snapshot()?;
        let run_events = match previous {
            Some(previous) if snapshot.current_tick > previous.current_tick => {
                snapshot.busy_ticks.saturating_sub(previous.busy_ticks)
            }
            _ => snapshot.busy_ticks,
        };
        let total_events = match previous {
            Some(previous) if snapshot.current_tick > previous.current_tick => {
                snapshot.current_tick.saturating_sub(previous.current_tick)
            }
            _ => snapshot.current_tick.max(1),
        };
        let idle_events = total_events.saturating_sub(run_events.min(total_events));
        Ok(SemanticTopologySnapshot {
            online_cpus: 1,
            entries: vec![SemanticCpuTopologyEntry {
                cpu_index: 0,
                apic_id: 0,
                launched: true,
                online: true,
                load: CpuLoadStats {
                    run_events,
                    idle_events,
                },
            }],
        })
    }

    pub fn plan_pressure_response(
        &self,
        previous: Option<&NativeSystemSnapshotRecord>,
        adaptive_state: &mut AdaptiveState,
    ) -> Result<SemanticDecisionPlan, Errno> {
        let semantic_state = self.observe_semantic_state(previous, adaptive_state)?;
        let facts = self.collect_facts()?;
        let processes = collect_process_entities(&facts);
        let devices = collect_device_entities(&facts);
        let mut actions = Vec::new();

        if matches!(
            semantic_state.pressure,
            PressureState::HighSchedulerPressure | PressureState::MixedPressure
        ) {
            for process in candidate_processes(&processes).into_iter().take(2) {
                if process.record.scheduler_class == NativeSchedulerClass::Background as u32
                    && process.record.scheduler_budget <= 1
                {
                    continue;
                }
                actions.push(SemanticActionRecord {
                    reason: String::from("scheduler-pressure"),
                    detail: format!(
                        "renice pid={} name={} from={} to=background/1 cpu_ticks={}",
                        process.handle.pid,
                        process.name,
                        scheduler_class_label(process.record.scheduler_class),
                        process.record.cpu_runtime_ticks
                    ),
                });
            }
        }

        if matches!(
            semantic_state.pressure,
            PressureState::NetworkBackpressure | PressureState::MixedPressure
        ) {
            for (handle, record) in devices {
                let socket_pressure = if semantic_state.metrics.snapshot.total_socket_rx_limit == 0
                {
                    0
                } else {
                    semantic_state.metrics.socket_pressure_pct
                };
                if record.rx_dropped == 0
                    && record.tx_dropped == 0
                    && socket_pressure < 80
                    && record.tx_inflight_depth < record.tx_inflight_limit
                {
                    continue;
                }
                let new_tx_capacity = (record.tx_capacity as usize)
                    .saturating_add((record.tx_capacity as usize / 2).max(1));
                let new_rx_capacity = (record.rx_capacity as usize)
                    .saturating_add((record.rx_capacity as usize / 2).max(1));
                let new_tx_inflight_limit = (record.tx_inflight_limit as usize)
                    .saturating_add((record.tx_inflight_limit as usize / 2).max(1))
                    .min(new_tx_capacity.max(1));
                actions.push(SemanticActionRecord {
                    reason: String::from("network-backpressure"),
                    detail: format!(
                        "reconfigure iface={} tx-capacity={}->{} rx-capacity={}->{} inflight={}->{} dropped={}/{}",
                        handle.path,
                        record.tx_capacity,
                        new_tx_capacity,
                        record.rx_capacity,
                        new_rx_capacity,
                        record.tx_inflight_limit,
                        new_tx_inflight_limit,
                        record.tx_dropped,
                        record.rx_dropped
                    ),
                });
            }
        }

        if actions.is_empty()
            && matches!(
                semantic_state.pressure,
                PressureState::MixedPressure | PressureState::HighSchedulerPressure
            )
            && let Some(process) = candidate_processes(&processes).into_iter().next()
        {
            actions.push(SemanticActionRecord {
                reason: String::from("fallback-throttle"),
                detail: format!(
                    "pause pid={} name={} cpu_ticks={}",
                    process.handle.pid, process.name, process.record.cpu_runtime_ticks
                ),
            });
        }

        Ok(SemanticDecisionPlan {
            trigger: semantic_state.pressure,
            semantic: semantic_state.semantic,
            observation: semantic_state.observation,
            adaptive: semantic_state.adaptive,
            before: semantic_state.metrics,
            actions,
        })
    }

    pub fn semantic_diagnostics(
        &self,
        adaptive_state: &AdaptiveState,
        context: &SemanticContext,
    ) -> SemanticDiagnostics {
        let snapshot = adaptive_state.snapshot();
        SemanticDiagnostics {
            stress: snapshot.stress,
            focus: snapshot.focus,
            tier: snapshot.tier,
            compute_mode: snapshot.compute_mode,
            budget_points: snapshot.budget_points,
            event_count: context.event_count(),
            context_tail: context.tail(snapshot.compute_mode),
        }
    }

    fn read_process_text(&self, pid: u64, reader: ProcessTextReader<B>) -> Result<String, Errno> {
        let mut buffer = [0u8; 256];
        let count = reader(self.runtime, pid, &mut buffer)?;
        Ok(core::str::from_utf8(&buffer[..count])
            .map_err(|_| Errno::Inval)?
            .to_string())
    }

    fn list_path_entries(&self, path: &str) -> Result<Vec<String>, Errno> {
        let mut buffer = vec![0u8; 512];
        loop {
            let count = self.runtime.list_path(path, &mut buffer)?;
            if count < buffer.len() {
                let text = core::str::from_utf8(&buffer[..count]).map_err(|_| Errno::Inval)?;
                if text.trim().is_empty() {
                    return Ok(Vec::new());
                }
                return Ok(text
                    .lines()
                    .filter_map(|line| line.split('\t').next())
                    .filter(|entry| !entry.is_empty())
                    .map(|entry| entry.to_string())
                    .collect());
            }
            buffer.resize(buffer.len() * 2, 0);
        }
    }
}

pub fn event_source_name(record: &NativeEventRecord) -> &'static str {
    match NativeEventSourceKind::from_raw(record.source_kind) {
        Some(NativeEventSourceKind::Descriptor) => "descriptor",
        Some(NativeEventSourceKind::Timer) => "timer",
        Some(NativeEventSourceKind::Process) => "process",
        Some(NativeEventSourceKind::Signal) => "signal",
        Some(NativeEventSourceKind::MemoryWait) => "memory-wait",
        Some(NativeEventSourceKind::Resource) => "resource",
        Some(NativeEventSourceKind::Network) => "network",
        Some(NativeEventSourceKind::Graphics) => "graphics",
        None => "unknown",
    }
}

fn process_policy_fingerprint(process: &ProcessEntity) -> u64 {
    (process.record.scheduler_class as u64)
        | ((process.record.scheduler_budget as u64) << 8)
        | ((process.record.state as u64) << 40)
}

fn device_policy_fingerprint(device: &DeviceEntity) -> u64 {
    let Some(record) = device.record else {
        return 0;
    };
    (record.admin_up as u64)
        | ((record.promiscuous as u64) << 8)
        | (record.mtu << 16)
            ^ (record.tx_capacity << 24)
            ^ (record.rx_capacity << 32)
            ^ (record.tx_inflight_limit << 40)
}

fn socket_policy_fingerprint(socket: &SocketEntity) -> u64 {
    (socket.record.connected as u64)
        | (socket.record.rx_queue_limit << 8)
        | ((socket.record.local_port as u64) << 32)
        | ((socket.record.remote_port as u64) << 48)
}

fn resource_policy_fingerprint(record: &NativeResourceRecord) -> u64 {
    (record.state as u64)
        | ((record.arbitration as u64) << 8)
        | ((record.governance as u64) << 16)
        | ((record.contract_policy as u64) << 24)
        | ((record.issuer_policy as u64) << 32)
}

fn contract_policy_fingerprint(record: &NativeContractRecord) -> u64 {
    (record.kind as u64)
        | ((record.state as u64) << 8)
        | (record.issuer << 16)
        | (record.resource << 32)
}

fn collect_process_entities(facts: &[SystemFact]) -> Vec<ProcessEntity> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Process(process) => Some(process.clone()),
            _ => None,
        })
        .collect()
}

fn collect_device_entities(
    facts: &[SystemFact],
) -> Vec<(DeviceHandle, NativeNetworkInterfaceRecord)> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Device(device) => {
                device.record.map(|record| (device.handle.clone(), record))
            }
            _ => None,
        })
        .collect()
}

fn protected_process(process: &ProcessEntity) -> bool {
    matches!(
        NativeSchedulerClass::from_raw(process.record.scheduler_class),
        Some(NativeSchedulerClass::LatencyCritical | NativeSchedulerClass::Interactive)
    ) || process.handle.pid == 1
}

fn candidate_processes(processes: &[ProcessEntity]) -> Vec<ProcessEntity> {
    let mut candidates = processes
        .iter()
        .filter(|process| !protected_process(process) && matches!(process.record.state, 1 | 2))
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .record
            .cpu_runtime_ticks
            .cmp(&left.record.cpu_runtime_ticks)
            .then(
                right
                    .record
                    .scheduler_budget
                    .cmp(&left.record.scheduler_budget),
            )
            .then(left.handle.pid.cmp(&right.handle.pid))
    });
    candidates
}

fn scheduler_class_label(raw: u32) -> &'static str {
    match NativeSchedulerClass::from_raw(raw) {
        Some(NativeSchedulerClass::LatencyCritical) => "latency-critical",
        Some(NativeSchedulerClass::Interactive) => "interactive",
        Some(NativeSchedulerClass::BestEffort) => "best-effort",
        Some(NativeSchedulerClass::Background) => "background",
        None => "unknown",
    }
}
