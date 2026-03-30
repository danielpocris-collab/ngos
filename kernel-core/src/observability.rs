use super::*;
use crate::device_model::{NetworkInterface, NetworkSocket};

impl KernelRuntime {
    pub fn snapshot(&self) -> RuntimeSnapshot {
        let queued_by_class = self.scheduler.queued_len_by_class();
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
            queued_interactive: queued_by_class[1],
            queued_normal: queued_by_class[2],
            queued_background: queued_by_class[3],
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
            writeln!(
                out,
                "{}\towner={}\tiface={}\tlocal={}.{}.{}.{}:{}\tremote={}.{}.{}.{}:{}\trx-depth={}\trx-packets={}\ttx-packets={}",
                info.path,
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
}
