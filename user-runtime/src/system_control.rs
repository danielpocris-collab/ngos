use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

/// Canonical subsystem role:
/// - subsystem: user runtime semantic extraction and control helpers
/// - owner layer: Layer 2
/// - semantic owner: `user-runtime`
/// - truth path role: consumes kernel truth from `user-abi` and prepares it
///   for userland control surfaces
///
/// Canonical contract families consumed here:
/// - system snapshot contracts
/// - process/network/device inspection contracts
/// - verified-core contracts
/// - scheduler fairness contracts
///
/// This module may classify and explain system truth.
/// It must not redefine the underlying kernel truth it consumes.
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
    Errno, NativeBusEndpointRecord, NativeBusPeerRecord, NativeContractRecord,
    NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind, NativeNetworkInterfaceRecord,
    NativeNetworkSocketRecord, NativeProcessRecord, NativeResourceRecord, NativeSchedulerClass,
    NativeSystemSnapshotRecord, PollEvents, SyscallBackend,
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
pub struct BusPeerEntity {
    pub id: usize,
    pub record: NativeBusPeerRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusEndpointEntity {
    pub id: usize,
    pub record: NativeBusEndpointRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemFact {
    Process(ProcessEntity),
    Device(DeviceEntity),
    Socket(SocketEntity),
    BusPeer(BusPeerEntity),
    BusEndpoint(BusEndpointEntity),
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
    Bus {
        endpoint: usize,
        token: CapabilityToken,
        attached: bool,
        detached: bool,
        published: bool,
        received: bool,
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
    SetAffinity {
        cpu_mask: u64,
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
            ProcessAction::SetAffinity { cpu_mask } => {
                self.runtime.set_process_affinity(handle.pid, cpu_mask)
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
        self.runtime
            .fcntl(queue_fd, ngos_user_abi::FcntlCmd::SetFl { nonblock: true })?;
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
            EventFilter::Bus {
                endpoint,
                token,
                attached,
                detached,
                published,
                received,
                poll_events,
            } => self.runtime.watch_bus_events(
                queue_fd,
                *endpoint,
                token.value,
                *attached,
                *detached,
                *published,
                *received,
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
        let facts = self.collect_facts()?;
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
        let bus_endpoints = collect_bus_endpoint_entities(&facts);
        let bus_endpoint_count = bus_endpoints.len() as u64;
        let bus_queue_depth_total = bus_endpoints
            .iter()
            .map(|endpoint| endpoint.record.queue_depth)
            .sum::<u64>();
        let bus_queue_capacity_total = bus_endpoints
            .iter()
            .map(|endpoint| endpoint.record.queue_capacity)
            .sum::<u64>();
        let bus_overflow_total = bus_endpoints
            .iter()
            .map(|endpoint| endpoint.record.overflow_count)
            .sum::<u64>();
        let saturated_bus_endpoint_count = bus_endpoints
            .iter()
            .filter(|endpoint| {
                endpoint.record.queue_capacity != 0
                    && endpoint.record.queue_depth >= endpoint.record.queue_capacity
            })
            .count() as u64;
        let bus_pressure_pct = bus_queue_depth_total
            .saturating_mul(100)
            .checked_div(bus_queue_capacity_total.max(1))
            .unwrap_or(0) as u32;
        Ok(SystemPressureMetrics {
            verified_core_ok: snapshot.verified_core_ok(),
            verified_core_violation_count: snapshot.verified_core_violation_count(),
            run_queue_total: snapshot.queued_processes,
            run_queue_latency_critical: snapshot.queued_latency_critical,
            run_queue_interactive: snapshot.queued_interactive,
            run_queue_normal: snapshot.queued_normal,
            run_queue_background: snapshot.queued_background,
            run_queue_urgent_latency_critical: snapshot.queued_urgent_latency_critical,
            run_queue_urgent_interactive: snapshot.queued_urgent_interactive,
            run_queue_urgent_normal: snapshot.queued_urgent_normal,
            run_queue_urgent_background: snapshot.queued_urgent_background,
            scheduler_lag_debt_total: snapshot.scheduler_lag_debt_total(),
            scheduler_dispatch_total: snapshot.scheduler_dispatch_total(),
            scheduler_runtime_ticks_total: snapshot.scheduler_runtime_ticks_total(),
            scheduler_runtime_imbalance: snapshot.scheduler_runtime_imbalance(),
            scheduler_cpu_count: snapshot.scheduler_cpu_count,
            scheduler_running_cpu: snapshot
                .scheduler_has_running_cpu()
                .then_some(snapshot.scheduler_running_cpu),
            scheduler_cpu_load_imbalance: snapshot.scheduler_cpu_load_imbalance,
            scheduler_starved: snapshot.starved_any(),
            bus_endpoint_count,
            saturated_bus_endpoint_count,
            bus_queue_depth_total,
            bus_queue_capacity_total,
            bus_pressure_pct,
            bus_overflow_total,
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
        if !metrics.verified_core_ok {
            return PressureState::MixedPressure;
        }
        let high_scheduler_pressure = (metrics.run_queue_total >= 3
            && metrics.cpu_utilization_pct >= 75)
            || metrics.scheduler_starved
            || metrics.scheduler_lag_debt_total >= 6
            || (metrics.scheduler_runtime_ticks_total >= 4
                && metrics.scheduler_runtime_imbalance >= 3)
            || (metrics.scheduler_cpu_count >= 2 && metrics.scheduler_cpu_load_imbalance >= 2)
            || metrics.run_queue_urgent_total() >= 2;
        let network_backpressure = metrics.snapshot.saturated_socket_count > 0
            || metrics.socket_pressure_pct >= 80
            || metrics.rx_drop_delta > 0
            || metrics.tx_drop_delta > 0
            || metrics.event_queue_pressure_pct >= 75
            || metrics.saturated_bus_endpoint_count > 0
            || metrics.bus_pressure_pct >= 80
            || metrics.bus_overflow_total > 0;
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
        let channel = if metrics.verified_core_ok {
            pressure_channel_name(pressure).to_string()
        } else {
            String::from("kernel::verified-core")
        };
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
        if metrics.run_queue_urgent_total() >= 2 {
            anomaly = anomaly.saturating_add(15);
        }
        if metrics.scheduler_starved {
            anomaly = anomaly.saturating_add(20);
        }
        if metrics.scheduler_lag_debt_total >= 6 {
            anomaly = anomaly.saturating_add(15);
        }
        if metrics.scheduler_runtime_ticks_total >= 4 && metrics.scheduler_runtime_imbalance >= 3 {
            anomaly = anomaly.saturating_add(15);
        }
        if metrics.scheduler_cpu_count >= 2 && metrics.scheduler_cpu_load_imbalance >= 2 {
            anomaly = anomaly.saturating_add(15);
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
        if metrics.bus_pressure_pct >= 80 {
            anomaly = anomaly.saturating_add(15);
        }
        if metrics.bus_overflow_total > 0 {
            anomaly = anomaly.saturating_add(20);
        }
        if metrics.rx_drop_delta > 0 || metrics.tx_drop_delta > 0 {
            anomaly = anomaly.saturating_add(20);
        }
        if !metrics.verified_core_ok {
            anomaly = anomaly
                .saturating_add(35)
                .saturating_add((metrics.verified_core_violation_count as u32).saturating_mul(5));
        }
        anomaly.min(100)
    }

    fn thermal_proxy(&self, metrics: &SystemPressureMetrics) -> i16 {
        let thermal = 35u32
            .saturating_add(metrics.cpu_utilization_pct / 2)
            .saturating_add(metrics.event_queue_pressure_pct / 4)
            .saturating_add(metrics.bus_pressure_pct / 8);
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

        let mut bus_peers = vec![0u64; 64];
        let bus_peer_count = self.runtime.list_bus_peers(&mut bus_peers)?;
        bus_peers.truncate(bus_peer_count);
        for peer in bus_peers {
            facts.push(SystemFact::BusPeer(BusPeerEntity {
                id: peer as usize,
                record: self.runtime.inspect_bus_peer(peer as usize)?,
            }));
        }

        let mut bus_endpoints = vec![0u64; 64];
        let bus_endpoint_count = self.runtime.list_bus_endpoints(&mut bus_endpoints)?;
        bus_endpoints.truncate(bus_endpoint_count);
        for endpoint in bus_endpoints {
            facts.push(SystemFact::BusEndpoint(BusEndpointEntity {
                id: endpoint as usize,
                record: self.runtime.inspect_bus_endpoint(endpoint as usize)?,
            }));
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
                SystemFact::BusPeer(peer) => entities.push(SemanticEntity {
                    kind: SemanticEntityKind::BusPeer,
                    subject: format!("bus-peer:{}", peer.id),
                    semantic: semantic_for_channel("ipc::bus-peer"),
                    policy: SemanticPolicyView {
                        cpu_mask: u64::MAX,
                        policy_fingerprint: bus_peer_policy_fingerprint(&peer),
                    },
                }),
                SystemFact::BusEndpoint(endpoint) => entities.push(SemanticEntity {
                    kind: SemanticEntityKind::BusEndpoint,
                    subject: format!("bus-endpoint:{}", endpoint.id),
                    semantic: semantic_for_channel("ipc::bus-endpoint"),
                    policy: SemanticPolicyView {
                        cpu_mask: u64::MAX,
                        policy_fingerprint: bus_endpoint_policy_fingerprint(&endpoint),
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
        if let Some(topology) = self.observe_scheduler_topology_from_procfs()? {
            return Ok(topology);
        }
        Ok(self.observe_topology_from_snapshot(previous, &snapshot))
    }

    fn observe_topology_from_snapshot(
        &self,
        previous: Option<&NativeSystemSnapshotRecord>,
        snapshot: &NativeSystemSnapshotRecord,
    ) -> SemanticTopologySnapshot {
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
        let online_cpus = snapshot.scheduler_cpu_count.max(1) as usize;
        let running_cpu = if snapshot.scheduler_has_running_cpu() {
            Some(snapshot.scheduler_running_cpu as usize)
        } else {
            None
        };
        let entries = (0..online_cpus)
            .map(|cpu_index| SemanticCpuTopologyEntry {
                cpu_index,
                apic_id: cpu_index as u32,
                launched: true,
                online: true,
                load: CpuLoadStats {
                    run_events: if Some(cpu_index) == running_cpu {
                        run_events.max(1)
                    } else {
                        0
                    },
                    idle_events,
                },
            })
            .collect();
        SemanticTopologySnapshot {
            online_cpus,
            entries,
        }
    }

    fn observe_scheduler_topology_from_procfs(
        &self,
    ) -> Result<Option<SemanticTopologySnapshot>, Errno> {
        let text = match self.read_procfs_text("/proc/system/scheduler") {
            Ok(text) => text,
            Err(Errno::NoEnt | Errno::Access | Errno::Perm | Errno::Inval) => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut online_cpus = None;
        let mut entries = Vec::new();
        for line in text.lines() {
            if let Some(count) = parse_scheduler_cpu_summary_count(line) {
                online_cpus = Some(count);
                continue;
            }
            if let Some(entry) = parse_scheduler_cpu_topology_entry(line) {
                entries.push(entry);
            }
        }
        if entries.is_empty() {
            return Ok(None);
        }
        entries.sort_by_key(|entry| entry.cpu_index);
        Ok(Some(SemanticTopologySnapshot {
            online_cpus: online_cpus.unwrap_or(entries.len().max(1)),
            entries,
        }))
    }

    fn read_procfs_text(&self, path: &str) -> Result<String, Errno> {
        let mut capacity = 256usize;
        loop {
            let mut buffer = vec![0u8; capacity];
            let count = self.runtime.read_procfs(path, &mut buffer)?;
            if count < buffer.len() {
                buffer.truncate(count);
                return String::from_utf8(buffer).map_err(|_| Errno::Inval);
            }
            capacity = capacity.saturating_mul(2);
            if capacity > 64 * 1024 {
                return Err(Errno::TooBig);
            }
        }
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
        let bus_endpoints = collect_bus_endpoint_entities(&facts);
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

        if matches!(
            semantic_state.pressure,
            PressureState::NetworkBackpressure | PressureState::MixedPressure
        ) {
            for endpoint in bus_endpoints {
                if endpoint.record.queue_depth == 0 && endpoint.record.overflow_count == 0 {
                    continue;
                }
                if endpoint.record.last_peer == 0 {
                    continue;
                }
                actions.push(SemanticActionRecord {
                    reason: String::from("bus-backpressure"),
                    detail: format!(
                        "drain endpoint={} peer={} depth={} capacity={} overflows={}",
                        endpoint.id,
                        endpoint.record.last_peer,
                        endpoint.record.queue_depth,
                        endpoint.record.queue_capacity,
                        endpoint.record.overflow_count
                    ),
                });
            }
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
        Some(NativeEventSourceKind::Bus) => "bus",
        Some(NativeEventSourceKind::Vfs) => "vfs",
        None => "unknown",
    }
}

fn parse_scheduler_cpu_summary_count(line: &str) -> Option<usize> {
    if !line.starts_with("cpu-summary:\t") {
        return None;
    }
    line.split('\t')
        .find_map(|field| field.strip_prefix("count="))
        .and_then(|value| value.parse::<usize>().ok())
}

fn parse_scheduler_cpu_topology_entry(line: &str) -> Option<SemanticCpuTopologyEntry> {
    if !line.starts_with("cpu\t") {
        return None;
    }
    let mut cpu_index = None;
    let mut apic_id = None;
    let mut queued_load = 0u64;
    let mut runtime_ticks = 0u64;
    let mut running = false;
    for field in line.split('\t').skip(1) {
        if let Some(value) = field.strip_prefix("index=") {
            cpu_index = value.parse::<usize>().ok();
        } else if let Some(value) = field.strip_prefix("apic-id=") {
            apic_id = value.parse::<u32>().ok();
        } else if let Some(value) = field.strip_prefix("queued-load=") {
            queued_load = value.parse::<u64>().ok()?;
        } else if let Some(value) = field.strip_prefix("runtime-ticks=") {
            runtime_ticks = value.parse::<u64>().ok()?;
        } else if let Some(value) = field.strip_prefix("running=") {
            running = value == "true";
        }
    }
    let cpu_index = cpu_index?;
    Some(SemanticCpuTopologyEntry {
        cpu_index,
        apic_id: apic_id.unwrap_or(cpu_index as u32),
        launched: true,
        online: true,
        load: CpuLoadStats {
            run_events: queued_load
                .saturating_add(runtime_ticks)
                .saturating_add(u64::from(running)),
            idle_events: 1,
        },
    })
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

fn bus_peer_policy_fingerprint(peer: &BusPeerEntity) -> u64 {
    peer.record.attached_endpoint_count
        | (peer.record.publish_count << 16)
        | (peer.record.receive_count << 32)
        | (peer.record.last_endpoint << 48)
}

fn bus_endpoint_policy_fingerprint(endpoint: &BusEndpointEntity) -> u64 {
    endpoint.record.attached_peer_count
        | (endpoint.record.queue_depth << 16)
        | (endpoint.record.queue_capacity << 24)
        | (endpoint.record.peak_queue_depth << 32)
        | (endpoint.record.overflow_count << 40)
        | (endpoint.record.last_peer << 48)
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

fn collect_bus_endpoint_entities(facts: &[SystemFact]) -> Vec<BusEndpointEntity> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::BusEndpoint(endpoint) => Some(endpoint.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;
    use ngos_user_abi::{
        SYS_INSPECT_SYSTEM_SNAPSHOT, SYS_LIST_BUS_ENDPOINTS, SYS_LIST_BUS_PEERS,
        SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS, SYS_LIST_PATH, SYS_LIST_PROCESSES,
        SYS_LIST_RESOURCES, SYS_READ_PROCFS, SyscallFrame, SyscallReturn,
    };

    struct SnapshotBackend {
        snapshot: NativeSystemSnapshotRecord,
        procfs_scheduler: Option<Vec<u8>>,
        last: RefCell<Option<SyscallFrame>>,
    }

    impl SnapshotBackend {
        fn new(snapshot: NativeSystemSnapshotRecord) -> Self {
            Self {
                snapshot,
                procfs_scheduler: None,
                last: RefCell::new(None),
            }
        }

        fn with_scheduler_procfs(snapshot: NativeSystemSnapshotRecord, text: &str) -> Self {
            Self {
                snapshot,
                procfs_scheduler: Some(text.as_bytes().to_vec()),
                last: RefCell::new(None),
            }
        }
    }

    impl SyscallBackend for SnapshotBackend {
        unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
            *self.last.borrow_mut() = Some(frame);
            if frame.number == SYS_INSPECT_SYSTEM_SNAPSHOT {
                let ptr = frame.arg0 as *mut NativeSystemSnapshotRecord;
                unsafe {
                    ptr.write(self.snapshot);
                }
                SyscallReturn::ok(0)
            } else if frame.number == SYS_READ_PROCFS {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                if path != "/proc/system/scheduler" {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let Some(payload) = self.procfs_scheduler.as_ref() else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let dst =
                    unsafe { core::slice::from_raw_parts_mut(frame.arg2 as *mut u8, frame.arg3) };
                let copy_len = core::cmp::min(dst.len(), payload.len());
                dst[..copy_len].copy_from_slice(&payload[..copy_len]);
                SyscallReturn::ok(copy_len)
            } else if matches!(
                frame.number,
                SYS_LIST_PROCESSES
                    | SYS_LIST_PATH
                    | SYS_LIST_BUS_PEERS
                    | SYS_LIST_BUS_ENDPOINTS
                    | SYS_LIST_DOMAINS
                    | SYS_LIST_RESOURCES
                    | SYS_LIST_CONTRACTS
            ) {
                SyscallReturn::ok(0)
            } else {
                SyscallReturn::err(Errno::Inval)
            }
        }
    }

    fn base_snapshot() -> NativeSystemSnapshotRecord {
        NativeSystemSnapshotRecord {
            current_tick: 100,
            busy_ticks: 60,
            process_count: 3,
            active_process_count: 3,
            blocked_process_count: 0,
            queued_processes: 2,
            queued_latency_critical: 0,
            queued_interactive: 1,
            queued_normal: 1,
            queued_background: 0,
            queued_urgent_latency_critical: 0,
            queued_urgent_interactive: 0,
            queued_urgent_normal: 0,
            queued_urgent_background: 0,
            lag_debt_latency_critical: 0,
            lag_debt_interactive: 0,
            lag_debt_normal: 0,
            lag_debt_background: 0,
            dispatch_count_latency_critical: 0,
            dispatch_count_interactive: 0,
            dispatch_count_normal: 0,
            dispatch_count_background: 0,
            runtime_ticks_latency_critical: 0,
            runtime_ticks_interactive: 0,
            runtime_ticks_normal: 0,
            runtime_ticks_background: 0,
            scheduler_cpu_count: 1,
            scheduler_running_cpu: u64::MAX,
            scheduler_cpu_load_imbalance: 0,
            starved_latency_critical: 0,
            starved_interactive: 0,
            starved_normal: 0,
            starved_background: 0,
            deferred_task_count: 0,
            sleeping_processes: 0,
            total_event_queue_count: 1,
            total_event_queue_pending: 1,
            total_event_queue_waiters: 0,
            total_socket_count: 1,
            saturated_socket_count: 0,
            total_socket_rx_depth: 1,
            total_socket_rx_limit: 16,
            max_socket_rx_depth: 1,
            total_network_tx_dropped: 0,
            total_network_rx_dropped: 0,
            running_pid: 1,
            reserved0: NativeSystemSnapshotRecord::VERIFIED_CORE_OK_TRUE,
            reserved1: 0,
        }
    }

    #[test]
    fn observe_semantic_state_keeps_pressure_channel_when_verified_core_is_clean() {
        let runtime = Runtime::new(SnapshotBackend::new(base_snapshot()));
        let controller = SystemController::new(&runtime);
        let mut adaptive = AdaptiveState::new();
        let state = controller
            .observe_semantic_state(None, &mut adaptive)
            .unwrap();

        assert_eq!(state.channel, "proc::steady");
        assert_eq!(state.pressure, PressureState::Stable);
        assert!(state.metrics.verified_core_ok);
        assert_eq!(state.metrics.verified_core_violation_count, 0);
    }

    #[test]
    fn observe_semantic_state_escalates_to_kernel_channel_when_verified_core_is_broken() {
        let mut snapshot = base_snapshot();
        snapshot.reserved0 = NativeSystemSnapshotRecord::VERIFIED_CORE_OK_FALSE;
        snapshot.reserved1 = 4;
        let runtime = Runtime::new(SnapshotBackend::new(snapshot));
        let controller = SystemController::new(&runtime);
        let mut adaptive = AdaptiveState::new();
        let state = controller
            .observe_semantic_state(None, &mut adaptive)
            .unwrap();

        assert_eq!(state.channel, "kernel::verified-core");
        assert_eq!(state.pressure, PressureState::MixedPressure);
        assert!(!state.metrics.verified_core_ok);
        assert_eq!(state.metrics.verified_core_violation_count, 4);
        assert!(state.observation.anomaly_score >= 35);
        assert_eq!(state.semantic.class, SemanticClass::Process);
        assert!(
            state
                .semantic
                .capabilities
                .contains(&SemanticCapability::Protect)
        );
    }

    #[test]
    fn observe_semantic_state_escalates_scheduler_pressure_when_starved() {
        let mut snapshot = base_snapshot();
        snapshot.starved_background = NativeSystemSnapshotRecord::SCHEDULER_POLICY_TRUE;
        let runtime = Runtime::new(SnapshotBackend::new(snapshot));
        let controller = SystemController::new(&runtime);
        let mut adaptive = AdaptiveState::new();
        let state = controller
            .observe_semantic_state(None, &mut adaptive)
            .unwrap();

        assert_eq!(state.channel, "proc::scheduler");
        assert_eq!(state.pressure, PressureState::HighSchedulerPressure);
        assert!(state.metrics.scheduler_starved);
        assert!(state.observation.anomaly_score >= 20);
    }

    #[test]
    fn observe_semantic_state_escalates_scheduler_pressure_when_lag_debt_accumulates() {
        let mut snapshot = base_snapshot();
        snapshot.lag_debt_interactive = 4;
        snapshot.lag_debt_background = 3;
        let runtime = Runtime::new(SnapshotBackend::new(snapshot));
        let controller = SystemController::new(&runtime);
        let mut adaptive = AdaptiveState::new();
        let state = controller
            .observe_semantic_state(None, &mut adaptive)
            .unwrap();

        assert_eq!(state.channel, "proc::scheduler");
        assert_eq!(state.pressure, PressureState::HighSchedulerPressure);
        assert_eq!(state.metrics.scheduler_lag_debt_total, 7);
        assert!(state.observation.anomaly_score >= 15);
    }

    #[test]
    fn observe_semantic_state_escalates_scheduler_pressure_when_runtime_service_is_imbalanced() {
        let mut snapshot = base_snapshot();
        snapshot.runtime_ticks_interactive = 5;
        snapshot.runtime_ticks_background = 1;
        snapshot.dispatch_count_interactive = 5;
        snapshot.dispatch_count_background = 1;
        let runtime = Runtime::new(SnapshotBackend::new(snapshot));
        let controller = SystemController::new(&runtime);
        let mut adaptive = AdaptiveState::new();
        let state = controller
            .observe_semantic_state(None, &mut adaptive)
            .unwrap();

        assert_eq!(state.channel, "proc::scheduler");
        assert_eq!(state.pressure, PressureState::HighSchedulerPressure);
        assert_eq!(state.metrics.scheduler_runtime_imbalance, 4);
        assert!(state.observation.anomaly_score >= 15);
    }

    #[test]
    fn observe_semantic_state_escalates_scheduler_pressure_when_cpu_load_is_imbalanced() {
        let mut snapshot = base_snapshot();
        snapshot.scheduler_cpu_count = 2;
        snapshot.scheduler_running_cpu = 0;
        snapshot.scheduler_cpu_load_imbalance = 3;
        let runtime = Runtime::new(SnapshotBackend::new(snapshot));
        let controller = SystemController::new(&runtime);
        let mut adaptive = AdaptiveState::new();
        let state = controller
            .observe_semantic_state(None, &mut adaptive)
            .unwrap();

        assert_eq!(state.channel, "proc::scheduler");
        assert_eq!(state.pressure, PressureState::HighSchedulerPressure);
        assert_eq!(state.metrics.scheduler_cpu_count, 2);
        assert_eq!(state.metrics.scheduler_running_cpu, Some(0));
        assert_eq!(state.metrics.scheduler_cpu_load_imbalance, 3);
        assert!(state.observation.anomaly_score >= 15);
    }

    #[test]
    fn observe_topology_reads_real_scheduler_cpu_entries_from_procfs() {
        let mut snapshot = base_snapshot();
        snapshot.scheduler_cpu_count = 2;
        let runtime = Runtime::new(SnapshotBackend::with_scheduler_procfs(
            snapshot,
            "cpu-summary:\tcount=2\trunning=1\tload-imbalance=2\trebalance-ops=4\trebalance-migrations=1\tlast-rebalance=1\n\
cpu\tindex=0\tapic-id=17\tpackage=0\tcore-group=0\tsibling-group=0\tinferred-topology=true\tqueued-load=3\tdispatches=5\truntime-ticks=8\trunning=false\n\
cpu\tindex=1\tapic-id=18\tpackage=0\tcore-group=0\tsibling-group=1\tinferred-topology=true\tqueued-load=1\tdispatches=9\truntime-ticks=2\trunning=true\n",
        ));
        let controller = SystemController::new(&runtime);
        let topology = controller.observe_topology(None).unwrap();

        assert_eq!(topology.online_cpus, 2);
        assert_eq!(topology.entries.len(), 2);
        assert_eq!(topology.entries[0].cpu_index, 0);
        assert_eq!(topology.entries[0].apic_id, 17);
        assert_eq!(topology.entries[0].load.run_events, 11);
        assert_eq!(topology.entries[1].cpu_index, 1);
        assert_eq!(topology.entries[1].apic_id, 18);
        assert_eq!(topology.entries[1].load.run_events, 4);
    }

    #[test]
    fn observe_topology_falls_back_to_snapshot_when_procfs_is_unavailable() {
        let mut snapshot = base_snapshot();
        snapshot.scheduler_cpu_count = 2;
        snapshot.scheduler_running_cpu = 1;
        snapshot.current_tick = 120;
        snapshot.busy_ticks = 75;
        let previous = base_snapshot();
        let runtime = Runtime::new(SnapshotBackend::new(snapshot));
        let controller = SystemController::new(&runtime);
        let topology = controller.observe_topology(Some(&previous)).unwrap();

        assert_eq!(topology.online_cpus, 2);
        assert_eq!(topology.entries.len(), 2);
        assert_eq!(topology.entries[0].load.run_events, 0);
        assert_eq!(topology.entries[1].load.run_events, 15);
    }

    #[test]
    fn event_source_name_reports_bus_for_bus_event_records() {
        let record = NativeEventRecord {
            token: 1,
            events: 0,
            source_kind: NativeEventSourceKind::Bus as u32,
            source_arg0: 10,
            source_arg1: 20,
            source_arg2: 0,
            detail0: 2,
            detail1: 0,
        };
        assert_eq!(event_source_name(&record), "bus");
    }
}
