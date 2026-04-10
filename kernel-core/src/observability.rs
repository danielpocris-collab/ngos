use super::*;
use crate::device_model::{NetworkInterface, NetworkSocket};

// Canonical subsystem role:
// - subsystem: observability / procfs inspection
// - owner layer: Layer 1
// - semantic owner: `kernel-core`
// - truth path role: exposes canonical runtime state to user/runtime/proofs
//
// Canonical contract families exposed here:
// - procfs contracts
// - process inspection contracts
// - scheduler fairness contracts
// - verified-core contracts
//
// This module is a truth surface. Higher layers may read and explain it, but
// they must not replace it with shadow observability models.

fn render_scheduler_decision_meaning(decision: SchedulerAgentDecisionRecord) -> String {
    match decision.agent {
        SchedulerAgentKind::EnqueueAgent => {
            format!(
                "enqueue budget={} prior-state={}",
                decision.detail0, decision.detail1
            )
        }
        SchedulerAgentKind::WakeAgent => String::from("wake urgent-requeue=true"),
        SchedulerAgentKind::BlockAgent => {
            format!("block previous-budget={}", decision.detail0)
        }
        SchedulerAgentKind::TickAgent => match decision.detail0 {
            1 => format!(
                "tick continue-running remaining-budget={}",
                decision.detail1
            ),
            2 => format!(
                "tick rotate-to-ready replenished-budget={}",
                decision.detail1
            ),
            3 => format!("tick dispatch-selected budget={}", decision.detail1),
            other => format!("tick code={other} detail1={}", decision.detail1),
        },
        SchedulerAgentKind::AffinityAgent => {
            format!(
                "affinity cpu-mask=0x{:x} assigned-cpu={}",
                decision.detail0, decision.detail1
            )
        }
        SchedulerAgentKind::RebindAgent => match decision.detail1 {
            0 => format!("rebind deferred-not-ready budget={}", decision.detail0),
            1 => format!("rebind running-updated budget={}", decision.detail0),
            2 => format!("rebind queued-moved budget={}", decision.detail0),
            other => format!("rebind code={other} budget={}", decision.detail0),
        },
        SchedulerAgentKind::RemoveAgent => String::from("remove detached-from-scheduler"),
    }
}

fn env_value<'a>(envp: &'a [String], key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    envp.iter().find_map(|entry| entry.strip_prefix(&prefix))
}

fn process_abi_profile(process: &Process) -> ProcessAbiProfile {
    let target = env_value(process.envp(), "NGOS_COMPAT_TARGET").unwrap_or("native");
    let route_class =
        env_value(process.envp(), "NGOS_COMPAT_ABI_ROUTE_CLASS").unwrap_or("native-process-abi");
    let handle_profile =
        env_value(process.envp(), "NGOS_COMPAT_ABI_HANDLE_PROFILE").unwrap_or("native-handles");
    let path_profile =
        env_value(process.envp(), "NGOS_COMPAT_ABI_PATH_PROFILE").unwrap_or("native-paths");
    let scheduler_profile = env_value(process.envp(), "NGOS_COMPAT_ABI_SCHEDULER_PROFILE")
        .unwrap_or("native-scheduler");
    let sync_profile =
        env_value(process.envp(), "NGOS_COMPAT_ABI_SYNC_PROFILE").unwrap_or("native-sync");
    let timer_profile =
        env_value(process.envp(), "NGOS_COMPAT_ABI_TIMER_PROFILE").unwrap_or("native-timer");
    let module_profile =
        env_value(process.envp(), "NGOS_COMPAT_ABI_MODULE_PROFILE").unwrap_or("native-module");
    let event_profile =
        env_value(process.envp(), "NGOS_COMPAT_ABI_EVENT_PROFILE").unwrap_or("native-event");
    let requires_kernel_abi_shims = env_value(process.envp(), "NGOS_COMPAT_ABI_REQUIRES_SHIMS")
        .map(|value| value == "1")
        .unwrap_or(false);
    let prefix = env_value(process.envp(), "NGOS_COMPAT_PREFIX").unwrap_or("/");
    let loader_route_class =
        env_value(process.envp(), "NGOS_COMPAT_ROUTE_CLASS").unwrap_or("native-direct");
    let loader_launch_mode =
        env_value(process.envp(), "NGOS_COMPAT_LAUNCH_MODE").unwrap_or("native-direct");
    let loader_entry_profile =
        env_value(process.envp(), "NGOS_COMPAT_ENTRY_PROFILE").unwrap_or("native-entry");
    let loader_requires_compat_shims = env_value(process.envp(), "NGOS_COMPAT_REQUIRES_SHIMS")
        .map(|value| value == "1")
        .unwrap_or(false);
    ProcessAbiProfile {
        target: target.to_string(),
        route_class: route_class.to_string(),
        handle_profile: handle_profile.to_string(),
        path_profile: path_profile.to_string(),
        scheduler_profile: scheduler_profile.to_string(),
        sync_profile: sync_profile.to_string(),
        timer_profile: timer_profile.to_string(),
        module_profile: module_profile.to_string(),
        event_profile: event_profile.to_string(),
        requires_kernel_abi_shims,
        prefix: prefix.to_string(),
        executable_path: process.image_path().to_string(),
        working_dir: process.cwd().to_string(),
        loader_route_class: loader_route_class.to_string(),
        loader_launch_mode: loader_launch_mode.to_string(),
        loader_entry_profile: loader_entry_profile.to_string(),
        loader_requires_compat_shims,
    }
}

impl KernelRuntime {
    pub fn snapshot(&self) -> RuntimeSnapshot {
        let verified_core = self.verify_core();
        let queued_by_class = self.scheduler.queued_len_by_class();
        let queued_urgent_by_class = self.scheduler.queued_urgent_len_by_class();
        let lag_debt_by_class = self.scheduler.class_lag_debt();
        let dispatch_counts = self.scheduler.class_dispatch_counts();
        let runtime_ticks_by_class = self.scheduler.class_runtime_ticks();
        let scheduler_dispatch_total = dispatch_counts.iter().copied().sum::<u64>();
        let scheduler_runtime_ticks_total = runtime_ticks_by_class.iter().copied().sum::<u64>();
        let cpu_queued_loads = self.scheduler.cpu_queued_loads();
        let cpu_load_min = cpu_queued_loads.iter().copied().min().unwrap_or(0);
        let cpu_load_max = cpu_queued_loads.iter().copied().max().unwrap_or(0);
        let mut runtime_min = u64::MAX;
        let mut runtime_max = 0u64;
        let mut runtime_seen = false;
        for value in runtime_ticks_by_class.iter().copied() {
            if value == 0 {
                continue;
            }
            runtime_seen = true;
            runtime_min = runtime_min.min(value);
            runtime_max = runtime_max.max(value);
        }
        let scheduler_runtime_imbalance = if runtime_seen {
            runtime_max.saturating_sub(runtime_min)
        } else {
            0
        };
        let starved_classes = self.scheduler.starved_classes();
        let active_process_count = self
            .processes
            .objects
            .iter()
            .filter(|(_, process)| process.state() != ProcessState::Exited)
            .count();
        let blocked_process_count = self
            .processes
            .objects
            .iter()
            .filter(|(_, process)| process.state() == ProcessState::Blocked)
            .count();
        let total_event_queue_pending = self
            .event_queues
            .iter()
            .map(|queue| queue.pending.len())
            .sum();
        let total_event_queue_waiters = self
            .event_queues
            .iter()
            .map(|queue| queue.waiters.len())
            .sum();
        let total_socket_rx_depth = self
            .network_sockets
            .iter()
            .map(|socket| socket.rx_queue.len())
            .sum();
        let total_socket_rx_limit = self
            .network_sockets
            .iter()
            .map(|socket| socket.rx_queue_limit)
            .sum();
        let saturated_socket_count = self
            .network_sockets
            .iter()
            .filter(|socket| socket.rx_queue.len() >= socket.rx_queue_limit)
            .count();
        let max_socket_rx_depth = self
            .network_sockets
            .iter()
            .map(|socket| socket.rx_queue.len())
            .max()
            .unwrap_or(0);
        let total_network_tx_dropped = self
            .network_ifaces
            .iter()
            .map(|iface| iface.tx_dropped)
            .sum();
        let total_network_rx_dropped = self
            .network_ifaces
            .iter()
            .map(|iface| iface.rx_dropped)
            .sum();
        RuntimeSnapshot {
            process_count: self.processes.len(),
            active_process_count,
            blocked_process_count,
            thread_count: self.processes.thread_count(),
            capability_count: self.capabilities.objects.len(),
            domain_count: self.domains.objects.len(),
            resource_count: self.resources.objects.len(),
            contract_count: self.contracts.objects.len(),
            queued_processes: self.scheduler.queued_len(),
            queued_latency_critical: queued_by_class[0],
            queued_urgent_latency_critical: queued_urgent_by_class[0],
            queued_interactive: queued_by_class[1],
            queued_urgent_interactive: queued_urgent_by_class[1],
            queued_normal: queued_by_class[2],
            queued_urgent_normal: queued_urgent_by_class[2],
            queued_background: queued_by_class[3],
            queued_urgent_background: queued_urgent_by_class[3],
            lag_debt_latency_critical: lag_debt_by_class[0],
            lag_debt_interactive: lag_debt_by_class[1],
            lag_debt_normal: lag_debt_by_class[2],
            lag_debt_background: lag_debt_by_class[3],
            dispatch_count_latency_critical: dispatch_counts[0],
            dispatch_count_interactive: dispatch_counts[1],
            dispatch_count_normal: dispatch_counts[2],
            dispatch_count_background: dispatch_counts[3],
            runtime_ticks_latency_critical: runtime_ticks_by_class[0],
            runtime_ticks_interactive: runtime_ticks_by_class[1],
            runtime_ticks_normal: runtime_ticks_by_class[2],
            runtime_ticks_background: runtime_ticks_by_class[3],
            scheduler_dispatch_total,
            scheduler_runtime_ticks_total,
            scheduler_runtime_imbalance,
            scheduler_cpu_count: self.scheduler.logical_cpu_count(),
            scheduler_running_cpu: self.scheduler.running().map(|process| process.cpu),
            scheduler_cpu_load_imbalance: cpu_load_max.saturating_sub(cpu_load_min),
            starved_latency_critical: starved_classes[0],
            starved_interactive: starved_classes[1],
            starved_normal: starved_classes[2],
            starved_background: starved_classes[3],
            deferred_task_count: self.deferred_tasks.total_pending(),
            sleeping_processes: self
                .sleep_queues
                .iter()
                .map(|queue| queue.waiters.len())
                .sum(),
            current_tick: self.current_tick,
            busy_ticks: self.busy_ticks,
            running: self.scheduler.running().map(|process| process.pid),
            running_thread: self.scheduler.running().map(|process| process.tid),
            contract_bound_processes: self
                .processes
                .objects
                .iter()
                .filter(|(_, process)| process.contract_bindings().any_bound())
                .count(),
            translated_processes: 0,
            total_event_queue_count: self.event_queues.len(),
            total_event_queue_pending,
            total_event_queue_waiters,
            total_socket_count: self.network_sockets.len(),
            saturated_socket_count,
            total_socket_rx_depth,
            total_socket_rx_limit,
            max_socket_rx_depth,
            total_network_tx_dropped,
            total_network_rx_dropped,
            verified_core_ok: verified_core.is_verified(),
            verified_core_violation_count: verified_core.violations.len(),
            capability_model_verified: verified_core.capability_model_verified,
            vfs_invariants_verified: verified_core.vfs_invariants_verified,
            scheduler_state_machine_verified: verified_core.scheduler_state_machine_verified,
            cpu_extended_state_lifecycle_verified: verified_core
                .cpu_extended_state_lifecycle_verified,
        }
    }

    pub fn process_info(&self, pid: ProcessId) -> Result<ProcessInfo, RuntimeError> {
        let process = self.processes.get(pid)?;
        let address_space = self.processes.get_process_address_space(pid)?;
        let threads = self.processes.threads_for_process(pid)?;
        let descriptor_count = self
            .namespaces
            .iter()
            .find(|(owner, _)| *owner == pid)
            .map(|(_, namespace)| namespace.by_owner(pid).len())
            .unwrap_or(0);
        let capability_count = self
            .capabilities
            .objects
            .iter()
            .filter(|(_, capability)| capability.owner() == pid)
            .count();
        let shared_memory_region_count = address_space
            .memory_map()
            .iter()
            .filter(|region| region.share_count > 1)
            .count();
        let copy_on_write_region_count = address_space
            .memory_map()
            .iter()
            .filter(|region| region.copy_on_write)
            .count();
        let vm_object_count = self.processes.vm_objects_for_process(pid)?.len();
        let scheduler_policy = self.scheduler_policy_for_process(pid)?;

        Ok(ProcessInfo {
            pid: process.pid(),
            parent: process.parent(),
            address_space: process.address_space(),
            main_thread: process.main_thread(),
            name: process.name().to_string(),
            image_path: process.image_path().to_string(),
            executable_image: process.executable_image().clone(),
            root: process.root().to_string(),
            cwd: process.cwd().to_string(),
            state: process.state(),
            exit_code: process.exit_code(),
            pending_signals: process.pending_signals(),
            descriptor_count,
            capability_count,
            environment_count: process.envp().len(),
            auxiliary_vector_count: process.auxv().len(),
            memory_region_count: address_space.memory_map().len(),
            thread_count: threads.len(),
            vm_object_count,
            shared_memory_region_count,
            copy_on_write_region_count,
            session_reported: process.session_reported(),
            session_report_count: process.session_report_count(),
            session_status: process.session_status(),
            session_stage: process.session_stage(),
            session_code: process.session_code(),
            session_detail: process.session_detail(),
            abi_profile: process_abi_profile(process),
            contract_bindings: process.contract_bindings(),
            scheduler_override: process.scheduler_override(),
            scheduler_policy,
            cpu_runtime_ticks: process.cpu_runtime_ticks(),
        })
    }

    pub fn process_list(&self) -> Vec<ProcessInfo> {
        let mut processes = self
            .processes
            .objects
            .iter()
            .map(|(handle, _)| ProcessId::from_handle(handle))
            .filter_map(|pid| self.process_info(pid).ok())
            .collect::<Vec<_>>();
        processes.sort_by_key(|process| process.pid.raw());
        processes
    }

    pub fn address_space_info(&self, pid: ProcessId) -> Result<AddressSpaceInfo, RuntimeError> {
        let space = self.processes.get_process_address_space(pid)?;
        let vm_object_count = self.processes.vm_objects_for_process(pid)?.len();
        let regions = space
            .memory_map()
            .iter()
            .map(|region| AddressSpaceRegionInfo {
                start: region.start,
                end: region.end,
                vm_object_id: region.vm_object_id,
                share_count: region.share_count,
                copy_on_write: region.copy_on_write,
                readable: region.readable,
                writable: region.writable,
                executable: region.executable,
                private: region.private,
                file_offset: region.file_offset,
                label: region.label.clone(),
            })
            .collect::<Vec<_>>();
        let shared_region_count = space
            .memory_map()
            .iter()
            .filter(|region| region.share_count > 1)
            .count();
        let copy_on_write_region_count = space
            .memory_map()
            .iter()
            .filter(|region| region.copy_on_write)
            .count();
        let mapped_bytes = space
            .memory_map()
            .iter()
            .map(|region| region.end.saturating_sub(region.start))
            .sum();

        Ok(AddressSpaceInfo {
            id: space.id(),
            owner: space.owner(),
            region_count: space.memory_map().len(),
            vm_object_count,
            shared_region_count,
            copy_on_write_region_count,
            mapped_bytes,
            regions,
        })
    }

    pub fn address_space_list(&self) -> Vec<AddressSpaceInfo> {
        let mut spaces = self
            .processes
            .objects
            .iter()
            .map(|(handle, _)| ProcessId::from_handle(handle))
            .filter_map(|pid| self.address_space_info(pid).ok())
            .collect::<Vec<_>>();
        spaces.sort_by_key(|space| space.id.raw());
        spaces
    }

    pub fn inspect_process(&self, pid: ProcessId) -> Result<ProcessIntrospection, RuntimeError> {
        Ok(ProcessIntrospection {
            process: self.process_info(pid)?,
            address_space: self.address_space_info(pid)?,
            threads: self.thread_infos(pid)?,
            filedesc_entries: self.filedesc_entries(pid)?,
            kinfo_file_entries: self.kinfo_file_entries(pid)?,
            vm_object_layouts: self.inspect_vm_object_layouts(pid)?,
        })
    }

    pub fn inspect_system(&self) -> SystemIntrospection {
        SystemIntrospection {
            snapshot: self.snapshot(),
            processes: self.process_list(),
            address_spaces: self.address_space_list(),
            domains: self.domain_list(),
            resources: self.resource_list(),
            contracts: self.contract_list(),
            resource_agent_decisions: self.recent_resource_agent_decisions().to_vec(),
            wait_agent_decisions: self.recent_wait_agent_decisions().to_vec(),
            scheduler_agent_decisions: self.scheduler.recent_decisions().to_vec(),
            io_agent_decisions: self.recent_io_agent_decisions().to_vec(),
            vm_agent_decisions: self.recent_vm_agent_decisions().to_vec(),
            syscall_dispatches: Vec::new(),
            event_queues: self
                .event_queues
                .iter()
                .map(|queue| self.event_queue_info(queue))
                .collect(),
            sleep_queues: self
                .sleep_queues
                .iter()
                .map(|queue| self.sleep_queue_info(queue))
                .collect(),
            fdshare_groups: self
                .fdshare_groups
                .iter()
                .map(|group| FiledescShareGroupInfo {
                    id: group.id,
                    members: group.members.clone(),
                })
                .collect(),
        }
    }

    pub fn inspect_event_queue(
        &self,
        owner: ProcessId,
        queue: EventQueueId,
    ) -> Result<EventQueueInfo, RuntimeError> {
        queue_introspection::inspect_event_queue(self, owner, queue)
    }

    pub fn inspect_event_queue_descriptor(
        &self,
        owner: ProcessId,
        queue_fd: Descriptor,
    ) -> Result<EventQueueInfo, RuntimeError> {
        queue_introspection::inspect_event_queue_descriptor(self, owner, queue_fd)
    }

    pub fn inspect_sleep_queue(
        &self,
        owner: ProcessId,
        queue: SleepQueueId,
    ) -> Result<SleepQueueInfo, RuntimeError> {
        queue_introspection::inspect_sleep_queue(self, owner, queue)
    }

    pub fn inspect_sleep_queue_descriptor(
        &self,
        owner: ProcessId,
        queue_fd: Descriptor,
    ) -> Result<SleepQueueInfo, RuntimeError> {
        queue_introspection::inspect_sleep_queue_descriptor(self, owner, queue_fd)
    }

    fn event_queue_info(&self, queue: &EventQueue) -> EventQueueInfo {
        queue_introspection::event_queue_info(self, queue)
    }

    fn sleep_queue_info(&self, queue: &RuntimeSleepQueue) -> SleepQueueInfo {
        queue_introspection::sleep_queue_info(self, queue)
    }

    pub fn read_procfs_path(&self, path: &str) -> Result<Vec<u8>, RuntimeError> {
        let path = normalize_path(path).ok_or(RuntimeError::Vfs(VfsError::InvalidPath))?;
        let segments = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();

        if segments.len() < 3 || segments[0] != "proc" {
            return Err(RuntimeError::Vfs(VfsError::InvalidPath));
        }

        if segments[1] == "system" {
            let content = match segments[2] {
                "queues" if segments.len() == 3 => self.render_procfs_system_queues()?,
                "scheduler" if segments.len() == 3 => self.render_procfs_system_scheduler()?,
                "schedulerepisodes" if segments.len() == 3 => {
                    self.render_procfs_system_schedulerepisodes()?
                }
                "signals" if segments.len() == 3 => self.render_procfs_system_signals()?,
                "waits" if segments.len() == 3 => self.render_procfs_system_waits()?,
                "fdshare" if segments.len() == 3 => self.render_procfs_system_fdshare()?,
                "resources" if segments.len() == 3 => self.render_procfs_system_resources()?,
                "bus" if segments.len() == 3 => self.render_procfs_system_bus()?,
                "io" if segments.len() == 3 => self.render_procfs_system_io()?,
                "cpu" if segments.len() == 3 => self.render_procfs_system_cpu()?,
                "verified-core" if segments.len() == 3 => {
                    self.render_procfs_system_verified_core()?
                }
                "network" if segments.len() == 4 && segments[3] == "interfaces" => {
                    self.render_procfs_network_interfaces()?
                }
                "network" if segments.len() == 4 && segments[3] == "sockets" => {
                    self.render_procfs_network_sockets()?
                }
                "queues" if segments.len() == 6 && segments[3] == "event" => {
                    let owner_raw = segments[4]
                        .parse::<u64>()
                        .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
                    let queue_raw = segments[5]
                        .parse::<u64>()
                        .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
                    let owner = self.find_process_id_by_raw(owner_raw)?;
                    self.render_procfs_event_queue(owner, EventQueueId(queue_raw))?
                }
                "queues" if segments.len() == 6 && segments[3] == "sleep" => {
                    let owner_raw = segments[4]
                        .parse::<u64>()
                        .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
                    let queue_raw = segments[5]
                        .parse::<u64>()
                        .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
                    let owner = self.find_process_id_by_raw(owner_raw)?;
                    self.render_procfs_sleep_queue(owner, SleepQueueId(queue_raw))?
                }
                _ => return Err(RuntimeError::Vfs(VfsError::NotFound)),
            };

            return Ok(content.into_bytes());
        }

        let pid_raw = segments[1]
            .parse::<u64>()
            .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
        let pid = self.find_process_id_by_raw(pid_raw)?;

        let content = match segments[2] {
            "status" if segments.len() == 3 => self.render_procfs_status(pid)?,
            "stat" if segments.len() == 3 => self.render_procfs_stat(pid)?,
            "cmdline" if segments.len() == 3 => return self.render_procfs_cmdline(pid),
            "cwd" if segments.len() == 3 => return self.render_procfs_cwd(pid),
            "environ" if segments.len() == 3 => return self.render_procfs_environ(pid),
            "exe" if segments.len() == 3 => return self.render_procfs_exe(pid),
            "auxv" if segments.len() == 3 => return self.render_procfs_auxv(pid),
            "maps" if segments.len() == 3 => return self.render_procfs_maps(pid),
            "vmobjects" if segments.len() == 3 => return self.render_procfs_vmobjects(pid),
            "vmdecisions" if segments.len() == 3 => return self.render_procfs_vmdecisions(pid),
            "vmepisodes" if segments.len() == 3 => return self.render_procfs_vmepisodes(pid),
            "signals" if segments.len() == 3 => self.render_procfs_signals(pid)?,
            "waits" if segments.len() == 3 => self.render_procfs_waits(pid)?,
            "fdshare" if segments.len() == 3 => self.render_procfs_fdshare(pid)?,
            "resources" if segments.len() == 3 => self.render_procfs_resources(pid)?,
            "io" if segments.len() == 3 => self.render_procfs_io(pid)?,
            "cpu" if segments.len() == 3 => self.render_procfs_cpu(pid)?,
            "queues" if segments.len() == 3 => self.render_procfs_queues(pid)?,
            "queues" if segments.len() == 5 && segments[3] == "event" => {
                let queue = segments[4]
                    .parse::<u64>()
                    .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
                self.render_procfs_event_queue(pid, EventQueueId(queue))?
            }
            "queues" if segments.len() == 5 && segments[3] == "sleep" => {
                let queue = segments[4]
                    .parse::<u64>()
                    .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
                self.render_procfs_sleep_queue(pid, SleepQueueId(queue))?
            }
            "fd" if segments.len() == 3 => self.render_procfs_fd(pid)?,
            "fdinfo" if segments.len() == 4 => {
                let fd = segments[3]
                    .parse::<u32>()
                    .map_err(|_| RuntimeError::Vfs(VfsError::InvalidPath))?;
                self.render_procfs_fdinfo(pid, Descriptor::new(fd))?
            }
            "caps" if segments.len() == 3 => self.render_procfs_caps(pid)?,
            _ => return Err(RuntimeError::Vfs(VfsError::NotFound)),
        };

        Ok(content.into_bytes())
    }

    pub fn read_procfs_path_for(
        &self,
        caller: ProcessId,
        path: &str,
    ) -> Result<Vec<u8>, RuntimeError> {
        let path = normalize_path(path).ok_or(RuntimeError::Vfs(VfsError::InvalidPath))?;
        let segments = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        if segments.len() >= 2 && segments[0] == "proc" {
            if segments[1] == "system" {
                self.enforce_process_observe_contract(caller)?;
            } else if let Ok(pid_raw) = segments[1].parse::<u64>() {
                let target = self.find_process_id_by_raw(pid_raw)?;
                if target != caller {
                    self.enforce_process_observe_contract(caller)?;
                }
            }
        }
        self.read_procfs_path(&path)
    }

    pub(crate) fn find_process_id_by_raw(&self, raw: u64) -> Result<ProcessId, RuntimeError> {
        self.processes
            .objects
            .iter()
            .map(|(handle, _)| ProcessId::from_handle(handle))
            .find(|pid| pid.raw() == raw)
            .ok_or(RuntimeError::Vfs(VfsError::NotFound))
    }

    pub(crate) fn find_domain_id_by_raw(&self, raw: u64) -> Result<DomainId, RuntimeError> {
        self.domains
            .objects
            .iter()
            .map(|(handle, _)| DomainId::from_handle(handle))
            .find(|id| id.raw() == raw)
            .ok_or(RuntimeError::Vfs(VfsError::NotFound))
    }

    pub(crate) fn find_resource_id_by_raw(&self, raw: u64) -> Result<ResourceId, RuntimeError> {
        self.resources
            .objects
            .iter()
            .map(|(handle, _)| ResourceId::from_handle(handle))
            .find(|id| id.raw() == raw)
            .ok_or(RuntimeError::Vfs(VfsError::NotFound))
    }

    pub(crate) fn find_contract_id_by_raw(&self, raw: u64) -> Result<ContractId, RuntimeError> {
        self.contracts
            .objects
            .iter()
            .map(|(handle, _)| ContractId::from_handle(handle))
            .find(|id| id.raw() == raw)
            .ok_or(RuntimeError::Vfs(VfsError::NotFound))
    }

    pub(crate) fn find_bus_endpoint_id_by_raw(
        &self,
        raw: u64,
    ) -> Result<BusEndpointId, RuntimeError> {
        self.bus_endpoints
            .objects
            .iter()
            .map(|(handle, _)| BusEndpointId::from_handle(handle))
            .find(|id| id.raw() == raw)
            .ok_or(RuntimeError::Vfs(VfsError::NotFound))
    }

    fn render_procfs_status(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.process_info(pid)?;
        let mut out = KernelBuffer::with_capacity(256);
        write!(
            out,
            "Name:\t{}\nImage:\t{}\nEntry:\t0x{:x}\nBase:\t0x{:x}\nStackTop:\t0x{:x}\nCwd:\t{}\nPid:\t{}\nPPid:\t{}\nState:\t{:?}\nExitCode:\t{}\nThreads:\t{}\nFDs:\t{}\nCaps:\t{}\nEnvs:\t{}\nAuxv:\t{}\nMaps:\t{}\nVmObjects:\t{}\nSessionReported:\t{}\nSessionReports:\t{}\nSessionStatus:\t{}\nSessionStage:\t{}\nSessionCode:\t{}\nSessionDetail:\t{}\nSchedulerClass:\t{:?}\nSchedulerBudget:\t{}\nExecutionContract:\t{}\nMemoryContract:\t{}\nIoContract:\t{}\nObserveContract:\t{}\n",
            process.name,
            process.image_path,
            process.executable_image.entry_point,
            process.executable_image.base_addr,
            process.executable_image.stack_top,
            process.cwd,
            process.pid.raw(),
            process.parent.map(|parent| parent.raw()).unwrap_or(0),
            process.state,
            process
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| String::from("-")),
            process.thread_count,
            process.descriptor_count,
            process.capability_count,
            process.environment_count,
            process.auxiliary_vector_count,
            process.memory_region_count,
            process.vm_object_count,
            process.session_reported,
            process.session_report_count,
            process.session_status,
            process.session_stage,
            process.session_code,
            process.session_detail,
            process.scheduler_policy.class,
            process.scheduler_policy.budget,
            process.contract_bindings.execution.map(|id| id.raw()).unwrap_or(0),
            process.contract_bindings.memory.map(|id| id.raw()).unwrap_or(0),
            process.contract_bindings.io.map(|id| id.raw()).unwrap_or(0),
            process.contract_bindings.observe.map(|id| id.raw()).unwrap_or(0),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_stat(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.process_info(pid)?;
        let mut out = KernelBuffer::with_capacity(64);
        writeln!(
            out,
            "{} ({}) {} {} {} {}",
            process.pid.raw(),
            process.name,
            proc_state_code(process.state),
            process.parent.map(|parent| parent.raw()).unwrap_or(0),
            process.descriptor_count,
            process.capability_count,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_cmdline(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let process = self.processes.get(pid)?;
        let mut bytes = Vec::new();
        for arg in process.argv() {
            bytes.extend_from_slice(arg.as_bytes());
            bytes.push(0);
        }
        Ok(bytes)
    }

    fn render_procfs_cwd(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let process = self.processes.get(pid)?;
        Ok(process.cwd().as_bytes().to_vec())
    }

    fn render_procfs_environ(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let process = self.processes.get(pid)?;
        let mut bytes = Vec::new();
        for entry in process.envp() {
            bytes.extend_from_slice(entry.as_bytes());
            bytes.push(0);
        }
        Ok(bytes)
    }

    fn render_procfs_exe(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let process = self.processes.get(pid)?;
        Ok(process.image_path().as_bytes().to_vec())
    }

    fn render_procfs_auxv(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let process = self.processes.get(pid)?;
        let mut out = KernelBuffer::with_capacity(process.auxv().len().saturating_mul(24).max(32));
        for entry in process.auxv() {
            writeln!(out, "{}\t0x{:x}", entry.key, entry.value)
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out.as_bytes().to_vec())
    }

    fn render_procfs_maps(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let space = self.processes.get_process_address_space(pid)?;
        let mut out =
            KernelBuffer::with_capacity(space.memory_map().len().saturating_mul(96).max(96));
        for region in space.memory_map() {
            let perms = [
                if region.readable { 'r' } else { '-' },
                if region.writable { 'w' } else { '-' },
                if region.executable { 'x' } else { '-' },
                if region.private { 'p' } else { 's' },
            ];
            let vm_flags = if region.copy_on_write {
                format!(
                    " obj={:08x} refs={} cow",
                    region.vm_object_id, region.share_count
                )
            } else {
                format!(
                    " obj={:08x} refs={}",
                    region.vm_object_id, region.share_count
                )
            };
            writeln!(
                out,
                "{:016x}-{:016x} {} {:08x} {}{}{}",
                region.start,
                region.end,
                perms.iter().collect::<String>(),
                region.file_offset,
                memory_advice_code(region.advice),
                region.label,
                vm_flags
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out.as_bytes().to_vec())
    }

    fn render_procfs_vmobjects(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let objects = self.processes.vm_objects_for_process(pid)?;
        let mut out = KernelBuffer::with_capacity(objects.len().saturating_mul(160).max(160));
        for object in objects {
            let layout = object.layout_info();
            let shadow = object
                .shadow_source_id
                .map(|source| {
                    format!(
                        "\tshadow={:08x}@{:08x}/depth={}",
                        source, object.shadow_source_offset, object.shadow_depth
                    )
                })
                .unwrap_or_default();
            writeln!(
                out,
                "{:08x}\t{:?}\tprivate={}\towners={}\toffset={:08x}\tcommitted={}\tresident={}\tdirty={}\taccessed={}\tsegments={}\tresident-segments={}\tfaults={}(r={},w={},cow={})\t{}\tquarantined={}\treason={}{}",
                object.id,
                object.kind,
                object.private,
                object.owners.len(),
                object.backing_offset,
                object.committed_pages,
                object.resident_pages,
                object.dirty_pages,
                object.accessed_pages,
                layout.segment_count,
                layout.resident_segment_count,
                object.fault_count,
                object.read_fault_count,
                object.write_fault_count,
                object.cow_fault_count,
                object.name,
                object.quarantined as u8,
                object.quarantine_reason,
                shadow,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out.as_bytes().to_vec())
    }

    fn render_procfs_vmdecisions(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        let decisions = self
            .recent_vm_agent_decisions()
            .iter()
            .copied()
            .filter(|entry| entry.pid == pid.raw())
            .collect::<Vec<_>>();
        let mut out = KernelBuffer::with_capacity(decisions.len().saturating_mul(128).max(64));
        for entry in decisions {
            writeln!(
                out,
                "tick={}\tagent={}\tvm-object={:08x}\tstart={:08x}\tlen={:08x}\tdetail0={}\tdetail1={}",
                entry.tick,
                match entry.agent {
                    VmAgentKind::MapAgent => "map",
                    VmAgentKind::BrkAgent => "brk",
                    VmAgentKind::ProtectAgent => "protect",
                    VmAgentKind::UnmapAgent => "unmap",
                    VmAgentKind::PolicyBlockAgent => "policy-block",
                    VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                    VmAgentKind::PressureVictimAgent => "pressure-victim",
                    VmAgentKind::FaultClassifierAgent => "fault-classifier",
                    VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                    VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                    VmAgentKind::CowPopulateAgent => "cow-populate",
                    VmAgentKind::PageTouchAgent => "page-touch",
                    VmAgentKind::SyncAgent => "sync",
                    VmAgentKind::AdviceAgent => "advice",
                    VmAgentKind::QuarantineStateAgent => "quarantine-state",
                    VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                },
                entry.vm_object_id,
                entry.start,
                entry.length,
                entry.detail0,
                entry.detail1,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out.as_bytes().to_vec())
    }

    fn render_procfs_vmepisodes(&self, pid: ProcessId) -> Result<Vec<u8>, RuntimeError> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct VmEpisodeEntry {
            kind: &'static str,
            vm_object_id: u64,
            start_tick: u64,
            end_tick: u64,
            quarantine_reason: u64,
            blocked: bool,
            released: bool,
            mapped_kind: u64,
            old_end: u64,
            new_end: u64,
            grew: bool,
            shrank: bool,
            evicted: bool,
            restored: bool,
            decision_count: u32,
            last_agent: VmAgentKind,
            faulted: bool,
            cow: bool,
            bridged: bool,
            touched: bool,
            synced: bool,
            advised: bool,
            protected: bool,
            unmapped: bool,
        }

        fn find_open_episode(episodes: &[VmEpisodeEntry], vm_object_id: u64) -> Option<usize> {
            let mut index = 0usize;
            while index < episodes.len() {
                if episodes[index].vm_object_id == vm_object_id {
                    return Some(index);
                }
                index += 1;
            }
            None
        }

        let decisions = self
            .recent_vm_agent_decisions()
            .iter()
            .copied()
            .filter(|entry| entry.pid == pid.raw())
            .collect::<Vec<_>>();
        fn mark_fault_episode_flag(entry: &mut VmEpisodeEntry, agent: VmAgentKind) {
            match agent {
                VmAgentKind::MapAgent
                | VmAgentKind::BrkAgent
                | VmAgentKind::ProtectAgent
                | VmAgentKind::UnmapAgent
                | VmAgentKind::PolicyBlockAgent
                | VmAgentKind::PressureTriggerAgent
                | VmAgentKind::PressureVictimAgent => {}
                VmAgentKind::FaultClassifierAgent => entry.faulted = true,
                VmAgentKind::ShadowReuseAgent | VmAgentKind::CowPopulateAgent => entry.cow = true,
                VmAgentKind::ShadowBridgeAgent => entry.bridged = true,
                VmAgentKind::PageTouchAgent => entry.touched = true,
                VmAgentKind::SyncAgent => entry.synced = true,
                VmAgentKind::AdviceAgent => entry.advised = true,
                VmAgentKind::QuarantineStateAgent | VmAgentKind::QuarantineBlockAgent => {}
            }
        }

        let open_map = Vec::<VmEpisodeEntry>::new();
        let mut open_heap = Vec::<VmEpisodeEntry>::new();
        let mut open_reclaim = Vec::<VmEpisodeEntry>::new();
        let mut open_quarantine = Vec::<VmEpisodeEntry>::new();
        let mut open_fault = Vec::<VmEpisodeEntry>::new();
        let mut open_region = Vec::<VmEpisodeEntry>::new();
        let mut finished = Vec::<VmEpisodeEntry>::new();
        let mut index = 0usize;
        while index < decisions.len() {
            let entry = decisions[index];
            match entry.agent {
                VmAgentKind::MapAgent => {
                    finished.push(VmEpisodeEntry {
                        kind: "map",
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: 0,
                        blocked: false,
                        released: false,
                        mapped_kind: entry.detail0,
                        old_end: 0,
                        new_end: 0,
                        grew: false,
                        shrank: false,
                        evicted: false,
                        restored: false,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: false,
                        protected: false,
                        unmapped: false,
                    });
                }
                VmAgentKind::BrkAgent => {
                    let grew = entry.detail1 > entry.detail0;
                    let shrank = entry.detail1 < entry.detail0;
                    if let Some(slot) = find_open_episode(&open_heap, entry.vm_object_id) {
                        let episode = &mut open_heap[slot];
                        episode.end_tick = entry.tick;
                        episode.old_end = episode.old_end.min(entry.detail0);
                        episode.new_end = entry.detail1;
                        episode.grew |= grew;
                        episode.shrank |= shrank;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                    } else {
                        open_heap.push(VmEpisodeEntry {
                            kind: "heap",
                            vm_object_id: entry.vm_object_id,
                            start_tick: entry.tick,
                            end_tick: entry.tick,
                            quarantine_reason: 0,
                            blocked: false,
                            released: false,
                            mapped_kind: 0,
                            old_end: entry.detail0,
                            new_end: entry.detail1,
                            grew,
                            shrank,
                            evicted: false,
                            restored: false,
                            decision_count: 1,
                            last_agent: entry.agent,
                            faulted: false,
                            cow: false,
                            bridged: false,
                            touched: false,
                            synced: false,
                            advised: false,
                            protected: false,
                            unmapped: false,
                        });
                    }
                }
                VmAgentKind::QuarantineStateAgent if entry.detail1 == 1 => {
                    if let Some(slot) = find_open_episode(&open_fault, entry.vm_object_id) {
                        finished.push(open_fault.remove(slot));
                    }
                    if let Some(slot) = find_open_episode(&open_region, entry.vm_object_id) {
                        finished.push(open_region.remove(slot));
                    }
                    if let Some(slot) = find_open_episode(&open_quarantine, entry.vm_object_id) {
                        let episode = &mut open_quarantine[slot];
                        episode.end_tick = entry.tick;
                        episode.quarantine_reason = entry.detail0;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                    } else {
                        open_quarantine.push(VmEpisodeEntry {
                            kind: "quarantine",
                            vm_object_id: entry.vm_object_id,
                            start_tick: entry.tick,
                            end_tick: entry.tick,
                            quarantine_reason: entry.detail0,
                            blocked: false,
                            released: false,
                            mapped_kind: 0,
                            old_end: 0,
                            new_end: 0,
                            grew: false,
                            shrank: false,
                            evicted: false,
                            restored: false,
                            decision_count: 1,
                            last_agent: entry.agent,
                            faulted: false,
                            cow: false,
                            bridged: false,
                            touched: false,
                            synced: false,
                            advised: false,
                            protected: false,
                            unmapped: false,
                        });
                    }
                }
                VmAgentKind::QuarantineBlockAgent => {
                    if let Some(slot) = find_open_episode(&open_quarantine, entry.vm_object_id) {
                        let episode = &mut open_quarantine[slot];
                        episode.end_tick = entry.tick;
                        episode.blocked = true;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                    }
                }
                VmAgentKind::QuarantineStateAgent if entry.detail1 == 0 => {
                    if let Some(slot) = find_open_episode(&open_quarantine, entry.vm_object_id) {
                        let mut episode = open_quarantine.remove(slot);
                        episode.end_tick = entry.tick;
                        episode.released = true;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                        finished.push(episode);
                    }
                }
                VmAgentKind::AdviceAgent if entry.detail0 == 4 || entry.detail0 == 3 => {
                    if let Some(slot) = find_open_episode(&open_reclaim, entry.vm_object_id) {
                        let episode = &mut open_reclaim[slot];
                        episode.end_tick = entry.tick;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                        if entry.detail0 == 4 {
                            episode.evicted = true;
                        } else {
                            episode.restored = true;
                        }
                    } else {
                        open_reclaim.push(VmEpisodeEntry {
                            kind: "reclaim",
                            vm_object_id: entry.vm_object_id,
                            start_tick: entry.tick,
                            end_tick: entry.tick,
                            quarantine_reason: 0,
                            blocked: false,
                            released: false,
                            mapped_kind: 0,
                            old_end: 0,
                            new_end: 0,
                            grew: false,
                            shrank: false,
                            evicted: entry.detail0 == 4,
                            restored: entry.detail0 == 3,
                            decision_count: 1,
                            last_agent: entry.agent,
                            faulted: false,
                            cow: false,
                            bridged: false,
                            touched: false,
                            synced: false,
                            advised: true,
                            protected: false,
                            unmapped: false,
                        });
                    }
                    if entry.detail0 == 3
                        && let Some(slot) = find_open_episode(&open_reclaim, entry.vm_object_id)
                    {
                        finished.push(open_reclaim.remove(slot));
                    }
                }
                VmAgentKind::ProtectAgent | VmAgentKind::UnmapAgent => {
                    if let Some(slot) = find_open_episode(&open_fault, entry.vm_object_id) {
                        finished.push(open_fault.remove(slot));
                    }
                    if let Some(slot) = find_open_episode(&open_region, entry.vm_object_id) {
                        let episode = &mut open_region[slot];
                        episode.end_tick = entry.tick;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                        if matches!(entry.agent, VmAgentKind::ProtectAgent) {
                            episode.protected = true;
                        } else {
                            episode.unmapped = true;
                        }
                    } else {
                        open_region.push(VmEpisodeEntry {
                            kind: "region",
                            vm_object_id: entry.vm_object_id,
                            start_tick: entry.tick,
                            end_tick: entry.tick,
                            quarantine_reason: 0,
                            blocked: false,
                            released: false,
                            mapped_kind: 0,
                            old_end: 0,
                            new_end: 0,
                            grew: false,
                            shrank: false,
                            evicted: false,
                            restored: false,
                            decision_count: 1,
                            last_agent: entry.agent,
                            faulted: false,
                            cow: false,
                            bridged: false,
                            touched: false,
                            synced: false,
                            advised: false,
                            protected: matches!(entry.agent, VmAgentKind::ProtectAgent),
                            unmapped: matches!(entry.agent, VmAgentKind::UnmapAgent),
                        });
                    }
                    if matches!(entry.agent, VmAgentKind::UnmapAgent)
                        && let Some(slot) = find_open_episode(&open_region, entry.vm_object_id)
                    {
                        finished.push(open_region.remove(slot));
                    }
                }
                VmAgentKind::PolicyBlockAgent => {
                    finished.push(VmEpisodeEntry {
                        kind: "policy",
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: 0,
                        blocked: true,
                        released: false,
                        mapped_kind: 0,
                        old_end: entry.detail0,
                        new_end: entry.detail1,
                        grew: false,
                        shrank: false,
                        evicted: false,
                        restored: false,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: false,
                        protected: false,
                        unmapped: false,
                    });
                }
                VmAgentKind::PressureTriggerAgent | VmAgentKind::PressureVictimAgent => {}
                _ => {
                    if let Some(slot) = find_open_episode(&open_reclaim, entry.vm_object_id) {
                        let episode = &mut open_reclaim[slot];
                        episode.end_tick = entry.tick;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                        if matches!(
                            entry.agent,
                            VmAgentKind::FaultClassifierAgent
                                | VmAgentKind::PageTouchAgent
                                | VmAgentKind::SyncAgent
                        ) {
                            episode.restored = true;
                        }
                        if matches!(
                            entry.agent,
                            VmAgentKind::PageTouchAgent | VmAgentKind::SyncAgent
                        ) && let Some(slot) =
                            find_open_episode(&open_reclaim, entry.vm_object_id)
                        {
                            finished.push(open_reclaim.remove(slot));
                        }
                    }
                    if let Some(slot) = find_open_episode(&open_fault, entry.vm_object_id) {
                        let episode = &mut open_fault[slot];
                        episode.end_tick = entry.tick;
                        episode.last_agent = entry.agent;
                        episode.decision_count = episode.decision_count.saturating_add(1);
                        mark_fault_episode_flag(episode, entry.agent);
                    } else {
                        let mut episode = VmEpisodeEntry {
                            kind: "fault",
                            vm_object_id: entry.vm_object_id,
                            start_tick: entry.tick,
                            end_tick: entry.tick,
                            quarantine_reason: 0,
                            blocked: false,
                            released: false,
                            mapped_kind: 0,
                            old_end: 0,
                            new_end: 0,
                            grew: false,
                            shrank: false,
                            evicted: false,
                            restored: false,
                            decision_count: 1,
                            last_agent: entry.agent,
                            faulted: false,
                            cow: false,
                            bridged: false,
                            touched: false,
                            synced: false,
                            advised: false,
                            protected: false,
                            unmapped: false,
                        };
                        mark_fault_episode_flag(&mut episode, entry.agent);
                        open_fault.push(episode);
                    }
                }
            }
            index += 1;
        }
        finished.extend(open_map);
        finished.extend(open_heap);
        finished.extend(open_reclaim);
        finished.extend(open_quarantine);
        finished.extend(open_fault);
        finished.extend(open_region);
        finished.sort_by_key(|entry| (entry.start_tick, entry.vm_object_id));

        let mut out = KernelBuffer::with_capacity(finished.len().saturating_mul(128).max(64));
        let mut episode_index = 0usize;
        while episode_index < finished.len() {
            let episode = finished[episode_index];
            if episode.kind == "map" {
                writeln!(
                    out,
                    "episode={}\tkind={}\tvm-object={:08x}\tstart-tick={}\tend-tick={}\tmapped={}\tdecisions={}\tlast={}",
                    episode_index + 1,
                    episode.kind,
                    episode.vm_object_id,
                    episode.start_tick,
                    episode.end_tick,
                    match episode.mapped_kind {
                        0 => "anon",
                        1 => "file-shared",
                        2 => "file-private",
                        _ => "unknown",
                    },
                    episode.decision_count,
                    match episode.last_agent {
                        VmAgentKind::MapAgent => "map",
                        VmAgentKind::BrkAgent => "brk",
                        VmAgentKind::ProtectAgent => "protect",
                        VmAgentKind::UnmapAgent => "unmap",
                        VmAgentKind::PolicyBlockAgent => "policy-block",
                        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                        VmAgentKind::PressureVictimAgent => "pressure-victim",
                        VmAgentKind::FaultClassifierAgent => "fault-classifier",
                        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                        VmAgentKind::CowPopulateAgent => "cow-populate",
                        VmAgentKind::PageTouchAgent => "page-touch",
                        VmAgentKind::SyncAgent => "sync",
                        VmAgentKind::AdviceAgent => "advice",
                        VmAgentKind::QuarantineStateAgent => "quarantine-state",
                        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                    },
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            } else if episode.kind == "heap" {
                writeln!(
                    out,
                    "episode={}\tkind={}\tvm-object={:08x}\tstart-tick={}\tend-tick={}\tgrew={}\tshrank={}\told-end={}\tnew-end={}\tdecisions={}\tlast={}",
                    episode_index + 1,
                    episode.kind,
                    episode.vm_object_id,
                    episode.start_tick,
                    episode.end_tick,
                    if episode.grew { "yes" } else { "no" },
                    if episode.shrank { "yes" } else { "no" },
                    episode.old_end,
                    episode.new_end,
                    episode.decision_count,
                    match episode.last_agent {
                        VmAgentKind::MapAgent => "map",
                        VmAgentKind::BrkAgent => "brk",
                        VmAgentKind::ProtectAgent => "protect",
                        VmAgentKind::UnmapAgent => "unmap",
                        VmAgentKind::PolicyBlockAgent => "policy-block",
                        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                        VmAgentKind::PressureVictimAgent => "pressure-victim",
                        VmAgentKind::FaultClassifierAgent => "fault-classifier",
                        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                        VmAgentKind::CowPopulateAgent => "cow-populate",
                        VmAgentKind::PageTouchAgent => "page-touch",
                        VmAgentKind::SyncAgent => "sync",
                        VmAgentKind::AdviceAgent => "advice",
                        VmAgentKind::QuarantineStateAgent => "quarantine-state",
                        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                    },
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            } else if episode.kind == "reclaim" {
                writeln!(
                    out,
                    "episode={}\tkind={}\tvm-object={:08x}\tstart-tick={}\tend-tick={}\tevicted={}\trestored={}\tdecisions={}\tlast={}",
                    episode_index + 1,
                    episode.kind,
                    episode.vm_object_id,
                    episode.start_tick,
                    episode.end_tick,
                    if episode.evicted { "yes" } else { "no" },
                    if episode.restored { "yes" } else { "no" },
                    episode.decision_count,
                    match episode.last_agent {
                        VmAgentKind::MapAgent => "map",
                        VmAgentKind::BrkAgent => "brk",
                        VmAgentKind::ProtectAgent => "protect",
                        VmAgentKind::UnmapAgent => "unmap",
                        VmAgentKind::PolicyBlockAgent => "policy-block",
                        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                        VmAgentKind::PressureVictimAgent => "pressure-victim",
                        VmAgentKind::FaultClassifierAgent => "fault-classifier",
                        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                        VmAgentKind::CowPopulateAgent => "cow-populate",
                        VmAgentKind::PageTouchAgent => "page-touch",
                        VmAgentKind::SyncAgent => "sync",
                        VmAgentKind::AdviceAgent => "advice",
                        VmAgentKind::QuarantineStateAgent => "quarantine-state",
                        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                    },
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            } else if episode.kind == "quarantine" {
                writeln!(
                    out,
                    "episode={}\tkind={}\tvm-object={:08x}\tstart-tick={}\tend-tick={}\treason={}\tblocked={}\treleased={}\tdecisions={}\tlast={}",
                    episode_index + 1,
                    episode.kind,
                    episode.vm_object_id,
                    episode.start_tick,
                    episode.end_tick,
                    episode.quarantine_reason,
                    if episode.blocked { "yes" } else { "no" },
                    if episode.released { "yes" } else { "no" },
                    episode.decision_count,
                    match episode.last_agent {
                        VmAgentKind::MapAgent => "map",
                        VmAgentKind::BrkAgent => "brk",
                        VmAgentKind::ProtectAgent => "protect",
                        VmAgentKind::UnmapAgent => "unmap",
                        VmAgentKind::PolicyBlockAgent => "policy-block",
                        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                        VmAgentKind::PressureVictimAgent => "pressure-victim",
                        VmAgentKind::FaultClassifierAgent => "fault-classifier",
                        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                        VmAgentKind::CowPopulateAgent => "cow-populate",
                        VmAgentKind::PageTouchAgent => "page-touch",
                        VmAgentKind::SyncAgent => "sync",
                        VmAgentKind::AdviceAgent => "advice",
                        VmAgentKind::QuarantineStateAgent => "quarantine-state",
                        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                    },
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            } else if episode.kind == "policy" {
                writeln!(
                    out,
                    "episode={}\tkind={}\tvm-object={:08x}\tstart-tick={}\tend-tick={}\tstate={}\toperation={}\tblocked=yes\tdecisions={}\tlast={}",
                    episode_index + 1,
                    episode.kind,
                    episode.vm_object_id,
                    episode.start_tick,
                    episode.end_tick,
                    episode.old_end,
                    episode.new_end,
                    episode.decision_count,
                    match episode.last_agent {
                        VmAgentKind::MapAgent => "map",
                        VmAgentKind::BrkAgent => "brk",
                        VmAgentKind::ProtectAgent => "protect",
                        VmAgentKind::UnmapAgent => "unmap",
                        VmAgentKind::PolicyBlockAgent => "policy-block",
                        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                        VmAgentKind::PressureVictimAgent => "pressure-victim",
                        VmAgentKind::FaultClassifierAgent => "fault-classifier",
                        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                        VmAgentKind::CowPopulateAgent => "cow-populate",
                        VmAgentKind::PageTouchAgent => "page-touch",
                        VmAgentKind::SyncAgent => "sync",
                        VmAgentKind::AdviceAgent => "advice",
                        VmAgentKind::QuarantineStateAgent => "quarantine-state",
                        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                    },
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            } else if episode.kind == "region" {
                writeln!(
                    out,
                    "episode={}\tkind={}\tvm-object={:08x}\tstart-tick={}\tend-tick={}\tprotected={}\tunmapped={}\tdecisions={}\tlast={}",
                    episode_index + 1,
                    episode.kind,
                    episode.vm_object_id,
                    episode.start_tick,
                    episode.end_tick,
                    if episode.protected { "yes" } else { "no" },
                    if episode.unmapped { "yes" } else { "no" },
                    episode.decision_count,
                    match episode.last_agent {
                        VmAgentKind::MapAgent => "map",
                        VmAgentKind::BrkAgent => "brk",
                        VmAgentKind::ProtectAgent => "protect",
                        VmAgentKind::UnmapAgent => "unmap",
                        VmAgentKind::PolicyBlockAgent => "policy-block",
                        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                        VmAgentKind::PressureVictimAgent => "pressure-victim",
                        VmAgentKind::FaultClassifierAgent => "fault-classifier",
                        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                        VmAgentKind::CowPopulateAgent => "cow-populate",
                        VmAgentKind::PageTouchAgent => "page-touch",
                        VmAgentKind::SyncAgent => "sync",
                        VmAgentKind::AdviceAgent => "advice",
                        VmAgentKind::QuarantineStateAgent => "quarantine-state",
                        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                    },
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            } else {
                writeln!(
                    out,
                    "episode={}\tkind={}\tvm-object={:08x}\tstart-tick={}\tend-tick={}\tfaulted={}\tcow={}\tbridged={}\ttouched={}\tsynced={}\tadvised={}\tdecisions={}\tlast={}",
                    episode_index + 1,
                    episode.kind,
                    episode.vm_object_id,
                    episode.start_tick,
                    episode.end_tick,
                    if episode.faulted { "yes" } else { "no" },
                    if episode.cow { "yes" } else { "no" },
                    if episode.bridged { "yes" } else { "no" },
                    if episode.touched { "yes" } else { "no" },
                    if episode.synced { "yes" } else { "no" },
                    if episode.advised { "yes" } else { "no" },
                    episode.decision_count,
                    match episode.last_agent {
                        VmAgentKind::MapAgent => "map",
                        VmAgentKind::BrkAgent => "brk",
                        VmAgentKind::ProtectAgent => "protect",
                        VmAgentKind::UnmapAgent => "unmap",
                        VmAgentKind::PolicyBlockAgent => "policy-block",
                        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
                        VmAgentKind::PressureVictimAgent => "pressure-victim",
                        VmAgentKind::FaultClassifierAgent => "fault-classifier",
                        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
                        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
                        VmAgentKind::CowPopulateAgent => "cow-populate",
                        VmAgentKind::PageTouchAgent => "page-touch",
                        VmAgentKind::SyncAgent => "sync",
                        VmAgentKind::AdviceAgent => "advice",
                        VmAgentKind::QuarantineStateAgent => "quarantine-state",
                        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
                    },
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            }
            episode_index += 1;
        }
        out.finish()?;
        Ok(out.as_bytes().to_vec())
    }

    fn render_procfs_system_queues(&self) -> Result<String, RuntimeError> {
        queue_introspection::render_procfs_system_queues(self)
    }

    fn render_procfs_system_scheduler(&self) -> Result<String, RuntimeError> {
        let queued_by_class = self.scheduler.queued_threads_by_class();
        let urgent_by_class = self.scheduler.queued_urgent_len_by_class();
        let class_tokens = self.scheduler.class_dispatch_tokens();
        let class_wait_ticks = self.scheduler.class_wait_ticks();
        let class_lag_debt = self.scheduler.class_lag_debt();
        let class_dispatch_counts = self.scheduler.class_dispatch_counts();
        let class_runtime_ticks = self.scheduler.class_runtime_ticks();
        let starved_classes = self.scheduler.starved_classes();
        let running = self.scheduler.running().cloned();
        let decisions = self.scheduler.recent_decisions();
        let mut out = KernelBuffer::with_capacity(
            256 + decisions.len().saturating_mul(96)
                + queued_by_class.iter().map(Vec::len).sum::<usize>() * 24,
        );
        let snapshot = self.snapshot();
        write!(
            out,
            "current-tick:\t{}\nbusy-ticks:\t{}\ndefault-budget:\t{}\ndecision-tracing:\t{}\nqueued-total:\t{}\nqueued-latency-critical:\t{}\nqueued-interactive:\t{}\nqueued-best-effort:\t{}\nqueued-background:\t{}\nfairness-dispatch-total:\t{}\nfairness-runtime-total:\t{}\nfairness-runtime-imbalance:\t{}\nrunning:\t{}\n",
            snapshot.current_tick,
            snapshot.busy_ticks,
            self.scheduler.default_budget(),
            self.scheduler.decision_tracing_enabled(),
            snapshot.queued_processes,
            snapshot.queued_latency_critical,
            snapshot.queued_interactive,
            snapshot.queued_normal,
            snapshot.queued_background,
            snapshot.scheduler_dispatch_total,
            snapshot.scheduler_runtime_ticks_total,
            snapshot.scheduler_runtime_imbalance,
            running
                .as_ref()
                .map(|process| format!(
                    "pid={} tid={} class={:?} budget={} cpu={}",
                    process.pid.raw(),
                    process.tid.raw(),
                    process.class,
                    process.budget,
                    process.cpu
                ))
                .unwrap_or_else(|| String::from("-")),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        writeln!(
            out,
            "cpu-summary:\tcount={}\trunning={}\tload-imbalance={}\trebalance-ops={}\trebalance-migrations={}\tlast-rebalance={}",
            snapshot.scheduler_cpu_count,
            snapshot
                .scheduler_running_cpu
                .map(|cpu| cpu.to_string())
                .unwrap_or_else(|| String::from("-")),
            snapshot.scheduler_cpu_load_imbalance,
            self.scheduler.rebalance_operations(),
            self.scheduler.rebalance_migrations(),
            self.scheduler.last_rebalance_migrations(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for cpu in 0..self.scheduler.logical_cpu_count() {
            writeln!(
                out,
                "cpu\tindex={cpu}\tapic-id={}\tpackage={}\tcore-group={}\tsibling-group={}\tinferred-topology={}\tqueued-load={}\tdispatches={}\truntime-ticks={}\trunning={}",
                self.scheduler.cpu_apic_id(cpu),
                self.scheduler.cpu_package_id(cpu),
                self.scheduler.cpu_core_group(cpu),
                self.scheduler.cpu_sibling_group(cpu),
                self.scheduler.cpu_topology_inferred(cpu),
                self.scheduler.cpu_queued_loads()[cpu],
                self.scheduler.cpu_dispatch_counts()[cpu],
                self.scheduler.cpu_runtime_ticks()[cpu],
                self.scheduler
                    .running()
                    .is_some_and(|process| process.cpu == cpu),
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        let cpu_class_loads = self.scheduler.cpu_class_queued_loads();
        for cpu in 0..self.scheduler.logical_cpu_count() {
            for class in SchedulerClass::ALL {
                let class_name = match class {
                    SchedulerClass::LatencyCritical => "latency-critical",
                    SchedulerClass::Interactive => "interactive",
                    SchedulerClass::BestEffort => "best-effort",
                    SchedulerClass::Background => "background",
                };
                let tids = self
                    .scheduler
                    .queued_threads_for_cpu_and_class(cpu, class)
                    .into_iter()
                    .map(|tid| tid.raw().to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(
                    out,
                    "cpu-queue\tindex={cpu}\tclass={class_name}\tcount={}\ttids=[{}]",
                    cpu_class_loads[cpu][class.index()],
                    tids
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            }
        }

        for (index, class) in SchedulerClass::ALL.iter().enumerate() {
            let class_name = match *class {
                SchedulerClass::LatencyCritical => "latency-critical",
                SchedulerClass::Interactive => "interactive",
                SchedulerClass::BestEffort => "best-effort",
                SchedulerClass::Background => "background",
            };
            let tids = queued_by_class[index]
                .iter()
                .map(|tid| tid.raw().to_string())
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "queue\tclass={class_name}\tcount={}\ttokens={}\twait-ticks={}\tlag-debt={}\tdispatches={}\truntime-ticks={}\ttids=[{}]",
                queued_by_class[index].len(),
                class_tokens[index],
                class_wait_ticks[index],
                class_lag_debt[index],
                class_dispatch_counts[index],
                class_runtime_ticks[index],
                tids
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            writeln!(
                out,
                "policy\tclass={class_name}\turgent={}\tstarved={}\tstarvation-guard={}",
                urgent_by_class[index],
                starved_classes[index],
                self.scheduler.starvation_guard_ticks(),
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        writeln!(out, "decisions:\t{}", decisions.len())
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for decision in decisions {
            writeln!(
                out,
                "decision\ttick={}\tagent={:?}\tpid={}\ttid={}\tclass={}\tdetail0={}\tdetail1={}\tmeaning={}",
                decision.tick,
                decision.agent,
                decision.pid,
                decision.tid,
                decision.class,
                decision.detail0,
                decision.detail1,
                render_scheduler_decision_meaning(*decision),
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_schedulerepisodes(&self) -> Result<String, RuntimeError> {
        #[derive(Clone, Copy)]
        struct SchedulerEpisode {
            kind: &'static str,
            tick: u64,
            pid: u64,
            tid: u64,
            class: u64,
            budget: u64,
            causal: &'static str,
        }

        let decisions = self.scheduler.recent_decisions();
        let mut episodes = Vec::<SchedulerEpisode>::new();
        for decision in decisions {
            let maybe_episode = match decision.agent {
                SchedulerAgentKind::WakeAgent => Some(SchedulerEpisode {
                    kind: "wake",
                    tick: decision.tick,
                    pid: decision.pid,
                    tid: decision.tid,
                    class: decision.class,
                    budget: 0,
                    causal: "urgent-requeue",
                }),
                SchedulerAgentKind::BlockAgent => Some(SchedulerEpisode {
                    kind: "block",
                    tick: decision.tick,
                    pid: decision.pid,
                    tid: decision.tid,
                    class: decision.class,
                    budget: decision.detail0,
                    causal: "running-blocked",
                }),
                SchedulerAgentKind::TickAgent if decision.detail0 == 2 => Some(SchedulerEpisode {
                    kind: "rotation",
                    tick: decision.tick,
                    pid: decision.pid,
                    tid: decision.tid,
                    class: decision.class,
                    budget: decision.detail1,
                    causal: "budget-expired",
                }),
                SchedulerAgentKind::TickAgent if decision.detail0 == 3 => Some(SchedulerEpisode {
                    kind: "dispatch",
                    tick: decision.tick,
                    pid: decision.pid,
                    tid: decision.tid,
                    class: decision.class,
                    budget: decision.detail1,
                    causal: "selected-next-runnable",
                }),
                SchedulerAgentKind::RebindAgent => Some(SchedulerEpisode {
                    kind: "rebind",
                    tick: decision.tick,
                    pid: decision.pid,
                    tid: decision.tid,
                    class: decision.class,
                    budget: decision.detail0,
                    causal: match decision.detail1 {
                        0 => "deferred-not-ready",
                        1 => "running-updated",
                        2 => "queued-moved",
                        _ => "rebind-other",
                    },
                }),
                SchedulerAgentKind::AffinityAgent => Some(SchedulerEpisode {
                    kind: "affinity",
                    tick: decision.tick,
                    pid: decision.pid,
                    tid: decision.tid,
                    class: decision.class,
                    budget: decision.detail0,
                    causal: "cpu-mask-updated",
                }),
                SchedulerAgentKind::RemoveAgent => Some(SchedulerEpisode {
                    kind: "remove",
                    tick: decision.tick,
                    pid: decision.pid,
                    tid: decision.tid,
                    class: decision.class,
                    budget: 0,
                    causal: "detached-from-scheduler",
                }),
                _ => None,
            };
            if let Some(episode) = maybe_episode {
                episodes.push(episode);
            }
        }

        let mut out = KernelBuffer::with_capacity(episodes.len().saturating_mul(128).max(96));
        writeln!(out, "episodes:\t{}", episodes.len())
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for episode in episodes {
            writeln!(
                out,
                "episode\tkind={}\ttick={}\tpid={}\ttid={}\tclass={}\tbudget={}\tcausal={}",
                episode.kind,
                episode.tick,
                episode.pid,
                episode.tid,
                episode.class,
                episode.budget,
                episode.causal,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_signals(&self) -> Result<String, RuntimeError> {
        let mut out =
            KernelBuffer::with_capacity(self.processes.len().saturating_mul(192).max(192));
        let mut processes = self.process_list();
        processes.sort_by_key(|process| process.pid.raw());
        for process in processes {
            let wait_mask = self
                .signal_wait_masks
                .get(&process.pid.raw())
                .copied()
                .unwrap_or(0);
            let process_record = self.processes.get(process.pid)?;
            let blocked = process_record.blocked_signals();
            let blocked_pending = self.blocked_pending_signals(process.pid)?;
            let thread_count = self.processes.threads_for_process(process.pid)?.len();
            writeln!(
                out,
                "pid={}\tname={}\tstate={:?}\tthreads={}\tmask=0x{:x}\tpending={:?}\tblocked={:?}\tblocked-pending={:?}\twait-mask=0x{:x}",
                process.pid.raw(),
                process.name,
                process.state,
                thread_count,
                process_record.signal_mask_raw(),
                process.pending_signals,
                blocked,
                blocked_pending,
                wait_mask,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_waits(&self) -> Result<String, RuntimeError> {
        let mut out = KernelBuffer::with_capacity(
            self.sleep_queues
                .len()
                .saturating_mul(192)
                .saturating_add(self.recent_wait_agent_decisions().len().saturating_mul(128))
                .max(256),
        );
        let snapshot = self.snapshot();
        let decisions = self.recent_wait_agent_decisions();
        let mut sleep_results = self
            .sleep_results
            .iter()
            .map(|(pid, result)| (*pid, *result))
            .collect::<Vec<_>>();
        sleep_results.sort_by_key(|(pid, _)| *pid);
        writeln!(
            out,
            "current-tick:\t{}\nbusy-ticks:\t{}\nsleeping-processes:\t{}\nsleep-queues:\t{}\nwait-decisions:\t{}\nsleep-results:\t{}",
            snapshot.current_tick,
            snapshot.busy_ticks,
            snapshot.sleeping_processes,
            self.sleep_queues.len(),
            decisions.len(),
            sleep_results.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        for queue in &self.sleep_queues {
            let info = self.sleep_queue_info(queue);
            let channels = info
                .channels
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let signal_owners = info
                .signal_wait_owners
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let memory_owners = info
                .memory_wait_owners
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "queue\towner={}\tid={}\twaiters={}\tchannels=[{}]\tdescriptors={}\tsignal-owners=[{}]\tmemory-owners=[{}]",
                info.owner.raw(),
                info.id.raw(),
                info.waiter_count,
                channels,
                info.descriptor_ref_count,
                signal_owners,
                memory_owners,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        for (pid, result) in sleep_results {
            writeln!(out, "result\tpid={}\tlast={:?}", pid, result)
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        for decision in decisions {
            writeln!(
                out,
                "decision\ttick={}\tagent={:?}\towner={}\tqueue={}\tchannel={}\tdetail0={}\tdetail1={}",
                decision.tick,
                decision.agent,
                decision.owner,
                decision.queue,
                decision.channel,
                decision.detail0,
                decision.detail1,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_fdshare(&self) -> Result<String, RuntimeError> {
        let mut groups = self.fdshare_groups.clone();
        groups.sort_by_key(|group| group.id);
        let mut out = KernelBuffer::with_capacity(groups.len().saturating_mul(96).max(128));
        writeln!(out, "fdshare-groups:\t{}", groups.len())
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for group in groups {
            let members = group
                .members
                .iter()
                .map(|member| member.raw().to_string())
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "group\tid={}\tmembers=[{}]\tcount={}",
                group.id,
                members,
                group.members.len()
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_resources(&self) -> Result<String, RuntimeError> {
        let mut resources = self.resource_list();
        resources.sort_by_key(|resource| resource.id.raw());
        let mut out = KernelBuffer::with_capacity(resources.len().saturating_mul(160).max(192));
        let decisions = self.recent_resource_agent_decisions();
        let active = resources
            .iter()
            .filter(|resource| resource.state == ResourceState::Active)
            .count();
        let queued = resources
            .iter()
            .filter(|resource| !resource.waiters.is_empty())
            .count();
        writeln!(
            out,
            "resources:\t{}\nactive:\t{}\nqueued:\t{}\nresource-decisions:\t{}",
            resources.len(),
            active,
            queued,
            decisions.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        for resource in resources {
            let waiters = resource
                .waiters
                .iter()
                .map(|contract| contract.raw().to_string())
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "resource\tid={}\tdomain={}\tcreator={}\tkind={:?}\tstate={:?}\tholder={}\twaiting={}\tacquires={}\thandoffs={}\tpolicy={:?}\tgovernance={:?}\twaiters=[{}]",
                resource.id.raw(),
                resource.domain.raw(),
                resource.creator.raw(),
                resource.kind,
                resource.state,
                resource.holder.map(|holder| holder.raw().to_string()).unwrap_or_else(|| String::from("-")),
                resource.waiting_count,
                resource.acquire_count,
                resource.handoff_count,
                resource.contract_policy,
                resource.governance,
                waiters,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        for decision in decisions {
            writeln!(
                out,
                "decision\ttick={}\tagent={:?}\tresource={}\tcontract={}\tdetail0={}\tdetail1={}",
                decision.tick,
                decision.agent,
                decision.resource,
                decision.contract,
                decision.detail0,
                decision.detail1,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_bus(&self) -> Result<String, RuntimeError> {
        let mut peers = self.bus_peers();
        peers.sort_by_key(|peer| peer.id.raw());
        let mut endpoints = self.bus_endpoints();
        endpoints.sort_by_key(|endpoint| endpoint.id.raw());
        let attached = endpoints
            .iter()
            .filter(|endpoint| !endpoint.attached_peers.is_empty())
            .count();
        let mut out = KernelBuffer::with_capacity(
            (peers.len() + endpoints.len()).saturating_mul(160).max(192),
        );
        writeln!(
            out,
            "bus-peers:\t{}\nbus-endpoints:\t{}\nattached-endpoints:\t{}",
            peers.len(),
            endpoints.len(),
            attached,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        for peer in peers {
            let endpoints = peer
                .attached_endpoints
                .iter()
                .map(|endpoint| endpoint.raw().to_string())
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "peer\tid={}\towner={}\tdomain={}\tname={}\tpublishes={}\treceives={}\tlast-endpoint={}\tendpoints=[{}]",
                peer.id.raw(),
                peer.owner.raw(),
                peer.domain.raw(),
                peer.name,
                peer.publish_count,
                peer.receive_count,
                peer.last_endpoint.map(|endpoint| endpoint.raw().to_string()).unwrap_or_else(|| String::from("-")),
                endpoints,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        for endpoint in endpoints {
            let peers = endpoint
                .attached_peers
                .iter()
                .map(|peer| peer.raw().to_string())
                .collect::<Vec<_>>()
                .join(",");
            let resource_info = self.resource_info(endpoint.resource)?;
            let delegated_capabilities = self
                .capabilities
                .objects
                .iter()
                .filter(|(_, capability)| capability.target() == endpoint.id.handle())
                .count();
            writeln!(
                out,
                "endpoint\tid={}\tdomain={}\tresource={}\tkind={}\tpath={}\tcontract-policy={}\tissuer-policy={:?}\tdelegated-caps={}\tqueue-depth={}\tqueue-capacity={}\tqueue-peak={}\toverflows={}\tbytes={}\tpublishes={}\treceives={}\tlast-peer={}\tpeers=[{}]",
                endpoint.id.raw(),
                endpoint.domain.raw(),
                endpoint.resource.raw(),
                endpoint.kind.label(),
                endpoint.path,
                match resource_info.contract_policy {
                    ResourceContractPolicy::Any => "any",
                    ResourceContractPolicy::Execution => "execution",
                    ResourceContractPolicy::Memory => "memory",
                    ResourceContractPolicy::Io => "io",
                    ResourceContractPolicy::Device => "device",
                    ResourceContractPolicy::Display => "display",
                    ResourceContractPolicy::Observe => "observe",
                },
                resource_info.issuer_policy,
                delegated_capabilities,
                endpoint.queue_depth,
                endpoint.queue_capacity,
                endpoint.peak_queue_depth,
                endpoint.overflow_count,
                endpoint.byte_count,
                endpoint.publish_count,
                endpoint.receive_count,
                endpoint.last_peer.map(|peer| peer.raw().to_string()).unwrap_or_else(|| String::from("-")),
                peers,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_io(&self) -> Result<String, RuntimeError> {
        let mut out = KernelBuffer::with_capacity(
            self.processes.len().saturating_mul(128)
                + self.recent_io_agent_decisions().len().saturating_mul(96)
                + 256,
        );
        let decisions = self.recent_io_agent_decisions();
        let reads = decisions
            .iter()
            .filter(|entry| matches!(entry.agent, IoAgentKind::ReadAgent))
            .count();
        let writes = decisions
            .iter()
            .filter(|entry| matches!(entry.agent, IoAgentKind::WriteAgent))
            .count();
        let fcntl = decisions
            .iter()
            .filter(|entry| matches!(entry.agent, IoAgentKind::FcntlAgent))
            .count();
        let readiness = decisions
            .iter()
            .filter(|entry| matches!(entry.agent, IoAgentKind::ReadinessAgent))
            .count();
        let mut processes = self.process_list();
        processes.sort_by_key(|process| process.pid.raw());
        let fd_total = processes
            .iter()
            .map(|process| {
                self.filedesc_entries(process.pid)
                    .map(|entries| entries.len())
                    .unwrap_or(0)
            })
            .sum::<usize>();
        writeln!(
            out,
            "io-decisions:\t{}\nreads:\t{}\nwrites:\t{}\tfcntl:\t{}\treadiness:\t{}\nfd-total:\t{}\n",
            decisions.len(),
            reads,
            writes,
            fcntl,
            readiness,
            fd_total,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        for process in processes {
            let entries = self.filedesc_entries(process.pid)?;
            let fds = self
                .filedesc_entries(process.pid)?
                .into_iter()
                .map(|entry| format!("{}:{:?}", entry.fd.raw(), entry.kind))
                .collect::<Vec<_>>()
                .join(",");
            let readiness_count = self
                .readiness
                .iter()
                .filter(|registration| registration.owner == process.pid)
                .count();
            let last = decisions
                .iter()
                .rev()
                .find(|entry| entry.owner == process.pid.raw())
                .map(|entry| format!("{:?}", entry.agent))
                .unwrap_or_else(|| String::from("-"));
            writeln!(
                out,
                "pid={}\tname={}\tfd-count={}\treadiness={}\tlast={}\tfds=[{}]",
                process.pid.raw(),
                process.name,
                entries.len(),
                readiness_count,
                last,
                fds,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        for decision in decisions {
            writeln!(
                out,
                "decision\ttick={}\tagent={:?}\towner={}\tfd={}\tkind={}\tdetail0={}\tdetail1={}",
                decision.tick,
                decision.agent,
                decision.owner,
                decision.fd,
                decision.kind,
                decision.detail0,
                decision.detail1,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_cpu(&self) -> Result<String, RuntimeError> {
        let snapshot = self.snapshot();
        let mut processes = self.process_list();
        processes.sort_by_key(|process| process.pid.raw());
        let mut out = KernelBuffer::with_capacity(
            256 + processes.len().saturating_mul(192) + snapshot.thread_count.saturating_mul(160),
        );
        writeln!(
            out,
            "current-tick:\t{}\nbusy-ticks:\t{}\nprocesses:\t{}\nthreads:\t{}\nrunning:\t{}\nactive-slot:\t{}\nhardware-saves:\t{}\nhardware-restores:\t{}\nhardware-fallbacks:\t{}\nhardware-last-saved-tid:\t{}\nhardware-last-restored-tid:\t{}\nhardware-last-error:\t{}\n",
            snapshot.current_tick,
            snapshot.busy_ticks,
            snapshot.process_count,
            snapshot.thread_count,
            snapshot
                .running_thread
                .map(|tid| tid.raw().to_string())
                .unwrap_or_else(|| String::from("-")),
            self.active_cpu_extended_state()
                .map(|slot| format!(
                    "pid={} tid={} bytes={} marker={:#x}",
                    slot.owner_pid.raw(),
                    slot.owner_tid.raw(),
                    slot.image.bytes.len(),
                    slot.image.profile.last_save_marker,
                ))
                .unwrap_or_else(|| String::from("-")),
            self.cpu_extended_state_hardware_telemetry().save_count,
            self.cpu_extended_state_hardware_telemetry().restore_count,
            self.cpu_extended_state_hardware_telemetry().fallback_count,
            self.cpu_extended_state_hardware_telemetry()
                .last_saved_tid
                .map(|tid| tid.raw().to_string())
                .unwrap_or_else(|| String::from("-")),
            self.cpu_extended_state_hardware_telemetry()
                .last_restored_tid
                .map(|tid| tid.raw().to_string())
                .unwrap_or_else(|| String::from("-")),
            self.cpu_extended_state_hardware_telemetry()
                .last_error
                .map(|error| format!("{error:?}"))
                .unwrap_or_else(|| String::from("-")),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        for process in processes {
            let threads = self.thread_infos(process.pid)?;
            writeln!(
                out,
                "process\tpid={}\tname={}\tthreads={}",
                process.pid.raw(),
                process.name,
                threads.len(),
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            for thread in threads {
                writeln!(
                    out,
                    "thread\tpid={}\ttid={}\txsave-managed={}\tsave-area={}\txcr0={:#x}\tboot-probed={}\tboot-seed={:#x}\tactive={}\tsaves={}\trestores={}\tbuff-bytes={}\tbuff-align={}\tgeneration={}\tmarker={:#x}",
                    process.pid.raw(),
                    thread.tid.raw(),
                    thread.cpu_extended_state.xsave_managed,
                    thread.cpu_extended_state.save_area_bytes,
                    thread.cpu_extended_state.xcr0_mask,
                    thread.cpu_extended_state.boot_probed,
                    thread.cpu_extended_state.boot_seed_marker,
                    thread.cpu_extended_state.active_in_cpu,
                    thread.cpu_extended_state.save_count,
                    thread.cpu_extended_state.restore_count,
                    thread.cpu_extended_state.save_area_buffer_bytes,
                    thread.cpu_extended_state.save_area_alignment_bytes,
                    thread.cpu_extended_state.save_area_generation,
                    thread.cpu_extended_state.last_save_marker,
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            }
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_system_verified_core(&self) -> Result<String, RuntimeError> {
        let report = self.verify_core();
        let mut out =
            KernelBuffer::with_capacity(256 + report.violations.len().saturating_mul(192));
        writeln!(
            out,
            "verified:\t{}\ncapability-model:\t{}\nvfs-invariants:\t{}\nscheduler-state-machine:\t{}\ncpu-extended-state-lifecycle:\t{}\nbus-integrity:\t{}\nviolations:\t{}",
            report.is_verified(),
            report.capability_model_verified,
            report.vfs_invariants_verified,
            report.scheduler_state_machine_verified,
            report.cpu_extended_state_lifecycle_verified,
            report.bus_integrity_verified,
            report.violations.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for violation in report.violations {
            writeln!(
                out,
                "violation\tfamily={}\tcode={}\tdetail={}",
                violation.family.label(),
                violation.code,
                violation.detail,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_network_interfaces(&self) -> Result<String, RuntimeError> {
        let mut infos = self
            .network_ifaces
            .iter()
            .map(NetworkInterface::info)
            .collect::<Vec<_>>();
        infos.sort_by(|left, right| left.device_path.cmp(&right.device_path));
        let mut out = KernelBuffer::with_capacity(infos.len().saturating_mul(160).max(96));
        for info in infos {
            writeln!(
                out,
                "{}\tdriver={}\tlink={}\tmtu={}\tmac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\taddr={}.{}.{}.{}\tnetmask={}.{}.{}.{}\tgateway={}.{}.{}.{}\trx-depth={}\ttx-depth={}\trx-packets={}\ttx-packets={}\tsockets={}",
                info.device_path,
                info.driver_path,
                if info.link_up { "up" } else { "down" },
                info.mtu,
                info.mac[0],
                info.mac[1],
                info.mac[2],
                info.mac[3],
                info.mac[4],
                info.mac[5],
                info.ipv4_addr[0],
                info.ipv4_addr[1],
                info.ipv4_addr[2],
                info.ipv4_addr[3],
                info.ipv4_netmask[0],
                info.ipv4_netmask[1],
                info.ipv4_netmask[2],
                info.ipv4_netmask[3],
                info.ipv4_gateway[0],
                info.ipv4_gateway[1],
                info.ipv4_gateway[2],
                info.ipv4_gateway[3],
                info.rx_ring_depth,
                info.tx_ring_depth,
                info.rx_packets,
                info.tx_packets,
                info.attached_sockets.len(),
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_network_sockets(&self) -> Result<String, RuntimeError> {
        let mut infos = self
            .network_sockets
            .iter()
            .map(NetworkSocket::info)
            .collect::<Vec<_>>();
        infos.sort_by(|left, right| left.path.cmp(&right.path));
        let mut out = KernelBuffer::with_capacity(infos.len().saturating_mul(128).max(96));
        for info in infos {
            let type_str = match info.socket_type {
                crate::device_model::SocketType::Udp => "udp",
                crate::device_model::SocketType::Tcp => "tcp",
            };
            let state_str = info.tcp_state.map(|s| match s {
                crate::device_model::TcpState::Closed => "CLOSED",
                crate::device_model::TcpState::Listen => "LISTEN",
                crate::device_model::TcpState::SynSent => "SYN_SENT",
                crate::device_model::TcpState::SynReceived => "SYN_RECV",
                crate::device_model::TcpState::Established => "ESTABLISHED",
                crate::device_model::TcpState::FinWait1 => "FIN_WAIT1",
                crate::device_model::TcpState::FinWait2 => "FIN_WAIT2",
                crate::device_model::TcpState::CloseWait => "CLOSE_WAIT",
                crate::device_model::TcpState::Closing => "CLOSING",
                crate::device_model::TcpState::LastAck => "LAST_ACK",
                crate::device_model::TcpState::TimeWait => "TIME_WAIT",
            }).unwrap_or("");
            writeln!(
                out,
                "{}\ttype={}\tstate={}\towner={}\tiface={}\tlocal={}.{}.{}.{}:{}\tremote={}.{}.{}.{}:{}\trx-depth={}\trx-packets={}\ttx-packets={}",
                info.path,
                type_str,
                state_str,
                info.owner.raw(),
                info.interface,
                info.local_ipv4[0],
                info.local_ipv4[1],
                info.local_ipv4[2],
                info.local_ipv4[3],
                info.local_port,
                info.remote_ipv4[0],
                info.remote_ipv4[1],
                info.remote_ipv4[2],
                info.remote_ipv4[3],
                info.remote_port,
                info.rx_depth,
                info.rx_packets,
                info.tx_packets,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_queues(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        queue_introspection::render_procfs_queues(self, pid)
    }

    fn render_procfs_event_queue(
        &self,
        pid: ProcessId,
        queue: EventQueueId,
    ) -> Result<String, RuntimeError> {
        queue_introspection::render_procfs_event_queue(self, pid, queue)
    }

    fn render_procfs_sleep_queue(
        &self,
        pid: ProcessId,
        queue: SleepQueueId,
    ) -> Result<String, RuntimeError> {
        queue_introspection::render_procfs_sleep_queue(self, pid, queue)
    }

    fn render_procfs_fd(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let entries = self.filedesc_entries(pid)?;
        let mut out = KernelBuffer::with_capacity(entries.len().saturating_mul(48).max(64));
        for entry in entries {
            writeln!(
                out,
                "{}\t{:?}\t{}\tcloexec={}\tnonblock={}",
                entry.fd.raw(),
                entry.kind,
                entry.path,
                entry.flags.cloexec,
                entry.flags.nonblock,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_fdinfo(&self, pid: ProcessId, fd: Descriptor) -> Result<String, RuntimeError> {
        let namespace = self.namespace(pid)?;
        let descriptor = namespace.get(fd).map_err(RuntimeError::from)?;
        let io = self.inspect_io(pid, fd)?;
        let mut out = KernelBuffer::with_capacity(256);
        write!(
            out,
            "fd:\t{}\npath:\t{}\nkind:\t{:?}\npos:\t{}\nflags:\tcloexec={} nonblock={}\nrights:\t0x{:x}\n",
            fd.raw(),
            descriptor.name(),
            descriptor.kind(),
            io.cursor(),
            descriptor.cloexec(),
            descriptor.nonblock(),
            io.capabilities().bits(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        match descriptor.queue_binding() {
            Some(QueueDescriptorTarget::Event { owner, queue, .. }) => {
                let info = self.inspect_event_queue(owner, queue)?;
                write!(
                    out,
                    "queue-owner:\t{}\nqueue-id:\t{}\nqueue-mode:\t{:?}\nqueue-watches:\t{}\nqueue-pending:\t{}\nqueue-deferred-refresh:\t{}\n",
                    info.owner.raw(),
                    info.id.raw(),
                    info.mode,
                    info.watch_count,
                    info.pending_count,
                    info.deferred_refresh_pending,
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            }
            Some(QueueDescriptorTarget::Sleep { owner, queue }) => {
                let info = self.inspect_sleep_queue(owner, queue)?;
                write!(
                    out,
                    "queue-owner:\t{}\nqueue-id:\t{}\nqueue-waiters:\t{}\nqueue-signal-owners:\t{}\nqueue-memory-owners:\t{}\n",
                    info.owner.raw(),
                    info.id.raw(),
                    info.waiter_count,
                    info.signal_wait_owners.len(),
                    info.memory_wait_owners.len(),
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            }
            None => {}
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_caps(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        self.processes.get(pid)?;
        let mut out = KernelBuffer::with_capacity(128);
        for (_, capability) in self.capabilities.objects.iter() {
            if capability.owner() == pid {
                writeln!(
                    out,
                    "{}\t{}\t0x{:x}\t{}",
                    capability.id().raw(),
                    capability.target().id().raw(),
                    capability.rights().bits(),
                    capability.label(),
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            }
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_signals(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.processes.get(pid)?;
        let wait_mask = self.signal_wait_masks.get(&pid.raw()).copied().unwrap_or(0);
        let mut out = KernelBuffer::with_capacity(512);
        write!(
            out,
            "pid:\t{}\nname:\t{}\nstate:\t{:?}\nmask:\t0x{:x}\nblocked:\t{:?}\npending:\t{:?}\nblocked-pending:\t{:?}\nwait-mask:\t0x{:x}\n",
            process.pid.raw(),
            process.name(),
            process.state(),
            process.signal_mask_raw(),
            process.blocked_signals(),
            process.pending_signals(),
            process.pending_blocked_signals(),
            wait_mask,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        writeln!(out, "dispositions:")
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for signal in 1..=64 {
            let Some(disposition) = process.signal_disposition(signal)? else {
                continue;
            };
            let action_mask = process.signal_action_mask(signal)?;
            let restart = process.signal_action_restart(signal)?;
            writeln!(
                out,
                "signal={}\tdisposition={:?}\taction-mask=0x{:x}\trestart={}",
                signal, disposition, action_mask, restart,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        writeln!(out, "threads:").map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for thread in self.processes.threads_for_process(pid)? {
            let pending = process.pending_thread_signals(thread)?;
            writeln!(out, "tid={}\tpending={:?}", thread.raw(), pending)
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_waits(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.processes.get(pid)?;
        let mut sleep_queues = self
            .sleep_queues
            .iter()
            .filter(|queue| queue.owner == pid)
            .map(|queue| self.sleep_queue_info(queue))
            .collect::<Vec<_>>();
        sleep_queues.sort_by_key(|queue| queue.id.raw());
        let decisions = self
            .recent_wait_agent_decisions()
            .iter()
            .copied()
            .filter(|decision| decision.owner == pid.raw())
            .collect::<Vec<_>>();
        let mut out = KernelBuffer::with_capacity(
            192 + sleep_queues.len().saturating_mul(160) + decisions.len().saturating_mul(96),
        );
        write!(
            out,
            "pid:\t{}\nname:\t{}\nstate:\t{:?}\nlast-sleep-result:\t{:?}\nsleep-queue-count:\t{}\nwait-decision-count:\t{}\n",
            process.pid.raw(),
            process.name(),
            process.state(),
            self.last_sleep_result(pid),
            sleep_queues.len(),
            decisions.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        for queue in sleep_queues {
            let channels = queue
                .channels
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            write!(
                out,
                "sleep\tid={}\twaiters={}\tchannels=[{}]\tdescriptors={}\tsignal-owners={}\tmemory-owners={}\n",
                queue.id.raw(),
                queue.waiter_count,
                channels,
                queue.descriptor_ref_count,
                queue.signal_wait_owners.len(),
                queue.memory_wait_owners.len(),
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            for waiter in queue.waiters {
                writeln!(
                    out,
                    "waiter\towner={}\tchannel={}\tpriority={}\twake-hint={}\tdeadline={:?}\tresult={:?}",
                    waiter.owner.raw(),
                    waiter.channel,
                    waiter.priority,
                    waiter.wake_hint,
                    waiter.deadline_tick,
                    waiter.result,
                )
                .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
            }
        }

        for decision in decisions {
            writeln!(
                out,
                "decision\ttick={}\tagent={:?}\tqueue={}\tchannel={}\tdetail0={}\tdetail1={}",
                decision.tick,
                decision.agent,
                decision.queue,
                decision.channel,
                decision.detail0,
                decision.detail1,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_fdshare(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.processes.get(pid)?;
        let group = self
            .fdshare_groups
            .iter()
            .find(|group| group.members.contains(&pid));
        let mut out = KernelBuffer::with_capacity(128);
        write!(
            out,
            "pid:\t{}\nname:\t{}\ngroup:\t{}\nmembers:\t{}\nshared:\t{}\nref-count:\t{}\n",
            process.pid.raw(),
            process.name(),
            group
                .map(|group| group.id.to_string())
                .unwrap_or_else(|| String::from("-")),
            group
                .map(|group| group
                    .members
                    .iter()
                    .map(|member| member.raw().to_string())
                    .collect::<Vec<_>>()
                    .join(","))
                .unwrap_or_else(|| String::from("-")),
            group.map(|group| group.members.len() > 1).unwrap_or(false),
            group.map(|group| group.members.len()).unwrap_or(1),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_io(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.processes.get(pid)?;
        let entries = self.filedesc_entries(pid)?;
        let readiness_count = self
            .readiness
            .iter()
            .filter(|registration| registration.owner == pid)
            .count();
        let mut out = KernelBuffer::with_capacity(entries.len().saturating_mul(80).max(192));
        let decisions = self
            .recent_io_agent_decisions()
            .iter()
            .copied()
            .filter(|entry| entry.owner == pid.raw())
            .collect::<Vec<_>>();
        let last = decisions
            .last()
            .map(|entry| format!("{:?}", entry.agent))
            .unwrap_or_else(|| String::from("-"));
        writeln!(
            out,
            "pid:\t{}\nname:\t{}\nfd-count:\t{}\nreadiness:\t{}\nio-decisions:\t{}\nlast:\t{}\n",
            process.pid.raw(),
            process.name(),
            entries.len(),
            readiness_count,
            decisions.len(),
            last,
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for entry in entries {
            let io = self.inspect_io(pid, entry.fd)?;
            writeln!(
                out,
                "fd\t{}\tkind={:?}\tpath={}\tpos={}\tstate={:?}\tflags=cloexec:{} nonblock:{}\trights=0x{:x}\tlen={}",
                entry.fd.raw(),
                entry.kind,
                entry.path,
                io.cursor(),
                io.state(),
                entry.flags.cloexec,
                entry.flags.nonblock,
                io.capabilities().bits(),
                io.payload().len(),
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        for decision in decisions {
            writeln!(
                out,
                "decision\ttick={}\tagent={:?}\tfd={}\tkind={}\tdetail0={}\tdetail1={}",
                decision.tick,
                decision.agent,
                decision.fd,
                decision.kind,
                decision.detail0,
                decision.detail1,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_resources(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.processes.get(pid)?;
        let mut resources = self
            .resource_list()
            .into_iter()
            .filter(|resource| resource.creator == pid)
            .collect::<Vec<_>>();
        resources.sort_by_key(|resource| resource.id.raw());
        let mut out = KernelBuffer::with_capacity(resources.len().saturating_mul(160).max(128));
        writeln!(
            out,
            "pid:\t{}\nname:\t{}\nresources:\t{}\n",
            process.pid.raw(),
            process.name(),
            resources.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        for resource in resources {
            let waiters = resource
                .waiters
                .iter()
                .map(|contract| contract.raw().to_string())
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "resource\tid={}\tkind={:?}\tstate={:?}\tholder={}\twaiting={}\tacquires={}\thandoffs={}\twaiters=[{}]",
                resource.id.raw(),
                resource.kind,
                resource.state,
                resource.holder.map(|holder| holder.raw().to_string()).unwrap_or_else(|| String::from("-")),
                resource.waiting_count,
                resource.acquire_count,
                resource.handoff_count,
                waiters,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }
        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }

    fn render_procfs_cpu(&self, pid: ProcessId) -> Result<String, RuntimeError> {
        let process = self.processes.get(pid)?;
        let threads = self.thread_infos(pid)?;
        let mut out = KernelBuffer::with_capacity(256 + threads.len().saturating_mul(160));
        writeln!(
            out,
            "pid:\t{}\nname:\t{}\nthreads:\t{}\n",
            process.pid.raw(),
            process.name(),
            threads.len(),
        )
        .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;

        for thread in threads {
            writeln!(
                out,
                "thread\ttid={}\towned={}\txsave-managed={}\tsave-area={}\txcr0={:#x}\tboot-probed={}\tboot-seed={:#x}\tactive={}\tsaves={}\trestores={}\tbuff-bytes={}\tbuff-align={}\tgeneration={}\tmarker={:#x}",
                thread.tid.raw(),
                thread.cpu_extended_state.owned,
                thread.cpu_extended_state.xsave_managed,
                thread.cpu_extended_state.save_area_bytes,
                thread.cpu_extended_state.xcr0_mask,
                thread.cpu_extended_state.boot_probed,
                thread.cpu_extended_state.boot_seed_marker,
                thread.cpu_extended_state.active_in_cpu,
                thread.cpu_extended_state.save_count,
                thread.cpu_extended_state.restore_count,
                thread.cpu_extended_state.save_area_buffer_bytes,
                thread.cpu_extended_state.save_area_alignment_bytes,
                thread.cpu_extended_state.save_area_generation,
                thread.cpu_extended_state.last_save_marker,
            )
            .map_err(|_| RuntimeError::Buffer(BufferError::LimitExceeded))?;
        }

        out.finish()?;
        Ok(out
            .as_str()
            .map_err(|_| RuntimeError::Buffer(BufferError::DrainRejected))?
            .to_owned())
    }
}
