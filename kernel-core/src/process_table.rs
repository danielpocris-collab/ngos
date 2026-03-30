use super::*;
use ngos_user_abi::BootSessionReport;

const VM_AGENT_DECISION_LIMIT: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VmTouchPlan {
    Direct {
        replacement_vm_object_id: Option<u64>,
        bridge_shadow_pair: Option<(u64, u64)>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VmTouchCommit {
    vm_object_id: u64,
    pages_touched: u64,
    faulted_pages: u64,
    cow_faulted_pages: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VmPressureCandidate {
    record_pid: ProcessId,
    vm_object_id: u64,
    backing_offset: u64,
    byte_len: u64,
    resident_pages: u64,
    score: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessTable {
    pub(crate) objects: KernelObjectTable<Process>,
    pub(crate) threads: KernelObjectTable<Thread>,
    pub(crate) address_spaces: KernelObjectTable<AddressSpace>,
    pub(crate) vm: VmManager,
    vm_agent_decisions: Vec<VmAgentDecisionRecord>,
    priority_vm_objects: Vec<u64>,
    decision_tracing_enabled: bool,
    decision_tick: u64,
    decision_clock: u64,
}

impl ProcessTable {
    pub fn new(start: u64, end_exclusive: u64) -> Self {
        Self {
            objects: KernelObjectTable::new(start, end_exclusive),
            threads: KernelObjectTable::new(start, end_exclusive),
            address_spaces: KernelObjectTable::new(start, end_exclusive),
            vm: VmManager::new(),
            vm_agent_decisions: Vec::with_capacity(VM_AGENT_DECISION_LIMIT),
            priority_vm_objects: Vec::with_capacity(32),
            decision_tracing_enabled: true,
            decision_tick: 0,
            decision_clock: 0,
        }
    }

    pub fn recent_vm_agent_decisions(&self) -> &[VmAgentDecisionRecord] {
        &self.vm_agent_decisions
    }

    pub fn set_decision_tracing_enabled(&mut self, enabled: bool) {
        self.decision_tracing_enabled = enabled;
    }

    pub fn set_decision_tick(&mut self, tick: u64) {
        self.decision_tick = tick;
    }

    fn next_vm_decision_tick(&mut self) -> u64 {
        if self.decision_tick > self.decision_clock {
            self.decision_clock = self.decision_tick;
        } else {
            self.decision_clock = self.decision_clock.saturating_add(1);
        }
        self.decision_clock
    }

    fn remember_priority_vm_object(&mut self, vm_object_id: u64) {
        if self.priority_vm_objects.contains(&vm_object_id) {
            return;
        }
        if self.priority_vm_objects.len() == 32 {
            self.priority_vm_objects.remove(0);
        }
        self.priority_vm_objects.push(vm_object_id);
    }

    fn derive_priority_vm_object(&mut self, source_vm_object_id: u64, derived_vm_object_id: u64) {
        if self.priority_vm_objects.contains(&source_vm_object_id) {
            self.remember_priority_vm_object(derived_vm_object_id);
        }
    }

    fn is_priority_vm_decision(&self, entry: &VmAgentDecisionRecord) -> bool {
        is_priority_vm_agent(entry.agent) || self.priority_vm_objects.contains(&entry.vm_object_id)
    }

    fn record_vm_agent_decision(
        &mut self,
        agent: VmAgentKind,
        pid: ProcessId,
        vm_object_id: u64,
        start: u64,
        length: u64,
        detail0: u64,
        detail1: u64,
    ) {
        if !self.decision_tracing_enabled {
            return;
        }
        if self
            .vm
            .objects
            .get(&vm_object_id)
            .map(|object| object.name.contains("host-runtime-scratch"))
            .unwrap_or(false)
        {
            return;
        }
        if self.vm_agent_decisions.len() == VM_AGENT_DECISION_LIMIT {
            let remove_index = self
                .vm_agent_decisions
                .iter()
                .position(|entry| !self.is_priority_vm_decision(entry))
                .unwrap_or(0);
            self.vm_agent_decisions.remove(remove_index);
        }
        let tick = self.next_vm_decision_tick();
        self.vm_agent_decisions.push(VmAgentDecisionRecord {
            tick,
            agent,
            pid: pid.raw(),
            vm_object_id,
            start,
            length,
            detail0,
            detail1,
        });
    }

    fn require_vm_object_access(
        &self,
        pid: ProcessId,
        vm_object_id: u64,
    ) -> Result<(), ProcessError> {
        self.get(pid)?;
        let object = self
            .vm
            .objects
            .get(&vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        if !object.owners.contains(&pid) {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        Ok(())
    }

    fn resolve_vm_object_id_for_range(
        &self,
        pid: ProcessId,
        start: u64,
        length: u64,
    ) -> Result<u64, ProcessError> {
        Ok(self
            .get_process_address_space(pid)?
            .resolve_range(start, length)?
            .vm_object_id)
    }

    fn ensure_vm_object_not_quarantined(
        &mut self,
        pid: ProcessId,
        vm_object_id: u64,
        start: u64,
        length: u64,
    ) -> Result<(), ProcessError> {
        let object = self
            .vm
            .objects
            .get(&vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        if object.quarantined {
            self.record_vm_agent_decision(
                VmAgentKind::QuarantineBlockAgent,
                pid,
                vm_object_id,
                start,
                length,
                object.quarantine_reason,
                0,
            );
            return Err(ProcessError::MemoryQuarantined { vm_object_id });
        }
        Ok(())
    }

    pub fn kernel_default() -> Self {
        Self::new(1, 1 << 16)
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn create(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
    ) -> Result<ProcessId, ProcessError> {
        if let Some(parent_pid) = parent {
            self.get(parent_pid)?;
        }

        let handle = self
            .objects
            .insert(Process::new_unbound(name, parent))
            .map_err(ProcessError::from_object_error)?;
        let pid = ProcessId::from_handle(handle);
        self.objects
            .get_mut(handle)
            .map_err(ProcessError::from_object_error)?
            .attach_pid(pid);
        let image = self
            .objects
            .get(handle)
            .map_err(ProcessError::from_object_error)?
            .executable_image()
            .clone();
        let mut address_space_value = AddressSpace::new_unbound(pid, &image);
        self.assign_vm_objects(pid, &mut address_space_value.memory_map);
        let space_handle = self
            .address_spaces
            .insert(address_space_value)
            .map_err(ProcessError::from_object_error)?;
        let address_space = AddressSpaceId::from_handle(space_handle);
        self.address_spaces
            .get_mut(space_handle)
            .map_err(ProcessError::from_object_error)?
            .attach_id(address_space);
        self.objects
            .get_mut(handle)
            .map_err(ProcessError::from_object_error)?
            .attach_address_space(address_space);
        self.create_main_thread(pid)?;
        Ok(pid)
    }

    pub fn spawn(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
    ) -> Result<ProcessId, ProcessError> {
        let pid = self.create(name, parent)?;
        self.set_state(pid, ProcessState::Ready)?;
        Ok(pid)
    }

    pub fn get(&self, pid: ProcessId) -> Result<&Process, ProcessError> {
        self.objects
            .get(pid.handle())
            .map_err(ProcessError::from_object_error)
    }

    pub fn contract_bindings(
        &self,
        pid: ProcessId,
    ) -> Result<ProcessContractBindings, ProcessError> {
        Ok(self.get(pid)?.contract_bindings())
    }

    pub fn bind_contract(
        &mut self,
        pid: ProcessId,
        kind: ContractKind,
        contract: ContractId,
    ) -> Result<(), ProcessError> {
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.bind_contract(kind, contract);
        Ok(())
    }

    pub fn get_thread(&self, tid: ThreadId) -> Result<&Thread, ProcessError> {
        self.threads
            .get(tid.handle())
            .map_err(ProcessError::from_thread_object_error)
    }

    pub fn get_address_space(&self, id: AddressSpaceId) -> Result<&AddressSpace, ProcessError> {
        self.address_spaces
            .get(id.handle())
            .map_err(ProcessError::from_object_error)
    }

    pub fn get_process_address_space(&self, pid: ProcessId) -> Result<&AddressSpace, ProcessError> {
        let id = self
            .get(pid)?
            .address_space()
            .ok_or(ProcessError::InvalidPid)?;
        self.get_address_space(id)
    }

    pub fn get_process_address_space_mut(
        &mut self,
        pid: ProcessId,
    ) -> Result<&mut AddressSpace, ProcessError> {
        let id = self
            .get(pid)?
            .address_space()
            .ok_or(ProcessError::InvalidPid)?;
        self.address_spaces
            .get_mut(id.handle())
            .map_err(ProcessError::from_object_error)
    }

    pub fn set_state(
        &mut self,
        pid: ProcessId,
        next: ProcessState,
    ) -> Result<ProcessState, ProcessError> {
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        let previous = process.state;
        if previous == next {
            return Ok(previous);
        }
        if !previous.can_transition_to(next) {
            return Err(ProcessError::InvalidTransition {
                from: previous,
                to: next,
            });
        }
        process.state = next;
        if next != ProcessState::Exited {
            process.exit_code = None;
        }
        self.sync_main_thread(pid)?;
        Ok(previous)
    }

    pub fn exit(&mut self, pid: ProcessId, code: i32) -> Result<(), ProcessError> {
        let previous = self
            .objects
            .get(pid.handle())
            .map_err(ProcessError::from_object_error)?
            .state();
        if previous != ProcessState::Exited && !previous.can_transition_to(ProcessState::Exited) {
            return Err(ProcessError::InvalidTransition {
                from: previous,
                to: ProcessState::Exited,
            });
        }

        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.state = ProcessState::Exited;
        process.exit_code = Some(code);
        self.sync_main_thread(pid)?;
        Ok(())
    }

    pub fn reap(&mut self, pid: ProcessId) -> Result<Process, ProcessError> {
        if self.get(pid)?.state() != ProcessState::Exited {
            return Err(ProcessError::NotExited);
        }
        self.unregister_vm_owners(pid);
        if let Some(id) = self.get(pid)?.address_space() {
            let _ = self.address_spaces.remove(id.handle());
        }
        if let Some(tid) = self.get(pid)?.main_thread() {
            let _ = self.threads.remove(tid.handle());
        }
        self.objects
            .remove(pid.handle())
            .map_err(ProcessError::from_object_error)
    }

    pub fn contains(&self, pid: ProcessId) -> bool {
        self.objects.contains(pid.handle())
    }

    pub fn set_args(&mut self, pid: ProcessId, argv: Vec<String>) -> Result<(), ProcessError> {
        if argv.is_empty() {
            return Ok(());
        }
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.set_argv(argv);
        self.sync_main_thread(pid)?;
        Ok(())
    }

    pub fn set_env(&mut self, pid: ProcessId, envp: Vec<String>) -> Result<(), ProcessError> {
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.set_envp(envp);
        Ok(())
    }

    pub fn record_session_report(
        &mut self,
        pid: ProcessId,
        report: BootSessionReport,
    ) -> Result<(), ProcessError> {
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.record_session_report(report)
    }

    pub fn set_cwd(&mut self, pid: ProcessId, cwd: String) -> Result<(), ProcessError> {
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.set_cwd(cwd);
        Ok(())
    }

    pub fn account_runtime_tick(&mut self, pid: ProcessId) -> Result<(), ProcessError> {
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.account_runtime_tick();
        Ok(())
    }

    pub fn copy_vm_state(&mut self, pid: ProcessId, source: ProcessId) -> Result<(), ProcessError> {
        let source_process = self
            .objects
            .get(source.handle())
            .map_err(ProcessError::from_object_error)?
            .clone();
        let mut shared_regions = self.get_process_address_space(source)?.memory_map.clone();
        for region in &mut shared_regions {
            region.share_count = region.share_count.saturating_add(1);
            if region.private {
                region.copy_on_write = true;
            }
        }
        self.get_process_address_space_mut(source)?.memory_map = shared_regions.clone();
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.name = source_process.name;
        process.image_path = source_process.image_path;
        process.executable_image = source_process.executable_image;
        process.cwd = source_process.cwd;
        process.argv = source_process.argv;
        process.envp = source_process.envp;
        process.auxv = source_process.auxv;
        process.exit_code = None;
        self.get_process_address_space_mut(pid)?.memory_map = shared_regions;
        self.sync_main_thread(pid)?;
        self.reconcile_vm_owners(source)?;
        self.reconcile_vm_owners(pid)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn exec(
        &mut self,
        pid: ProcessId,
        image_path: String,
        executable_image: ExecutableImage,
        mut memory_map: Vec<ProcessMemoryRegion>,
        argv: Vec<String>,
        envp: Vec<String>,
        auxv: Vec<AuxiliaryVectorEntry>,
    ) -> Result<(), ProcessError> {
        self.unregister_vm_owners(pid);
        self.assign_vm_objects(pid, &mut memory_map);
        self.get_process_address_space_mut(pid)?.memory_map = memory_map;
        let process = self
            .objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?;
        process.set_exec_image(image_path, executable_image, argv, envp, auxv);
        self.sync_main_thread(pid)?;
        Ok(())
    }

    pub fn map_anonymous_memory(
        &mut self,
        pid: ProcessId,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: String,
    ) -> Result<u64, ProcessError> {
        let vm_object_id = self.allocate_vm_object(
            inferred_vm_object_kind(&label),
            normalize_vm_object_name(&label),
            true,
            None,
            0,
            0,
            0,
            align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?,
            writable,
            pid,
        );
        let start = self
            .get_process_address_space_mut(pid)?
            .map_anonymous_memory(vm_object_id, length, readable, writable, executable, label)?;
        let region_count = self.get_process_address_space(pid)?.memory_map().len() as u64;
        self.remember_priority_vm_object(vm_object_id);
        self.record_vm_agent_decision(
            VmAgentKind::MapAgent,
            pid,
            vm_object_id,
            start,
            align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?,
            0,
            region_count,
        );
        Ok(start)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn map_file_memory(
        &mut self,
        pid: ProcessId,
        path: String,
        length: u64,
        file_offset: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        private: bool,
    ) -> Result<u64, ProcessError> {
        let vm_object_id = self.get_or_create_file_vm_object(
            pid,
            path.clone(),
            file_offset,
            align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?,
            private,
            writable,
        );
        let start = self.get_process_address_space_mut(pid)?.map_file_memory(
            vm_object_id,
            path,
            length,
            file_offset,
            readable,
            writable,
            executable,
            private,
        )?;
        let region_count = self.get_process_address_space(pid)?.memory_map().len() as u64;
        self.remember_priority_vm_object(vm_object_id);
        self.record_vm_agent_decision(
            VmAgentKind::MapAgent,
            pid,
            vm_object_id,
            start,
            align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?,
            if private { 2 } else { 1 },
            region_count,
        );
        Ok(start)
    }

    pub fn unmap_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
    ) -> Result<(), ProcessError> {
        let chunks = self
            .get_process_address_space(pid)?
            .range_chunks(start, length)?;
        self.get_process_address_space_mut(pid)?
            .unmap_memory(start, length)?;
        let region_count = self.get_process_address_space(pid)?.memory_map().len() as u64;
        for chunk in chunks {
            self.remember_priority_vm_object(chunk.vm_object_id);
            self.record_vm_agent_decision(
                VmAgentKind::UnmapAgent,
                pid,
                chunk.vm_object_id,
                chunk.start,
                chunk.end.saturating_sub(chunk.start),
                chunk.file_offset,
                region_count,
            );
        }
        self.reconcile_vm_owners(pid)?;
        Ok(())
    }

    pub fn set_brk(&mut self, pid: ProcessId, new_end: u64) -> Result<u64, ProcessError> {
        let old_heap = heap_region(self.get_process_address_space(pid)?)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let end = self.get_process_address_space_mut(pid)?.set_brk(new_end)?;
        let new_heap = heap_region(self.get_process_address_space(pid)?)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        self.remember_priority_vm_object(new_heap.vm_object_id);
        self.record_vm_agent_decision(
            VmAgentKind::BrkAgent,
            pid,
            new_heap.vm_object_id,
            new_heap.start,
            new_heap.end.saturating_sub(new_heap.start),
            old_heap.end,
            end,
        );
        self.reconcile_vm_owners(pid)?;
        Ok(end)
    }

    pub fn protect_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
    ) -> Result<(), ProcessError> {
        let chunks = self
            .get_process_address_space(pid)?
            .range_chunks(start, length)?;
        self.get_process_address_space_mut(pid)?
            .protect_memory(start, length, readable, writable, executable)?;
        let protection_bits =
            u64::from(readable) | (u64::from(writable) << 1) | (u64::from(executable) << 2);
        let region_count = self.get_process_address_space(pid)?.memory_map().len() as u64;
        for chunk in chunks {
            self.remember_priority_vm_object(chunk.vm_object_id);
            self.record_vm_agent_decision(
                VmAgentKind::ProtectAgent,
                pid,
                chunk.vm_object_id,
                chunk.start,
                chunk.end.saturating_sub(chunk.start),
                protection_bits,
                region_count,
            );
        }
        self.reconcile_vm_owners(pid)?;
        Ok(())
    }

    pub fn advise_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        advice: MemoryAdvice,
    ) -> Result<(), ProcessError> {
        let advised = {
            self.get_process_address_space_mut(pid)?
                .advise_memory(start, length, advice)?
        };
        for (vm_object_id, backing_offset, chunk_len) in advised {
            if let Some(object) = self.vm.objects.get_mut(&vm_object_id) {
                object.advise_pages(backing_offset, chunk_len, advice);
                let resident_pages = object.resident_pages;
                let detail0 = match advice {
                    MemoryAdvice::Normal => 0,
                    MemoryAdvice::Sequential => 1,
                    MemoryAdvice::Random => 2,
                    MemoryAdvice::WillNeed => 3,
                    MemoryAdvice::DontNeed => 4,
                };
                let _ = object;
                self.remember_priority_vm_object(vm_object_id);
                self.record_vm_agent_decision(
                    VmAgentKind::AdviceAgent,
                    pid,
                    vm_object_id,
                    backing_offset,
                    chunk_len,
                    detail0,
                    resident_pages,
                );
            }
        }
        self.reconcile_vm_owners(pid)?;
        Ok(())
    }

    pub fn sync_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
    ) -> Result<(), ProcessError> {
        let synced = {
            self.get_process_address_space_mut(pid)?
                .sync_memory(start, length)?
        };
        for (vm_object_id, backing_offset, chunk_len) in synced {
            let mut persist_file_backing = false;
            if let Some(object) = self.vm.objects.get_mut(&vm_object_id) {
                object.sync_pages(backing_offset, chunk_len);
                persist_file_backing = object.kind == VmObjectKind::File;
                let synced_pages = object.synced_pages;
                let sync_count = object.sync_count;
                let _ = object;
                self.record_vm_agent_decision(
                    VmAgentKind::SyncAgent,
                    pid,
                    vm_object_id,
                    backing_offset,
                    chunk_len,
                    synced_pages,
                    sync_count,
                );
            }
            if persist_file_backing {
                self.persist_file_backing_from_object(vm_object_id);
            }
        }
        self.reconcile_vm_owners(pid)?;
        Ok(())
    }

    fn plan_vm_touch(
        &mut self,
        pid: ProcessId,
        target_region: &ProcessMemoryRegion,
        cursor: u64,
        chunk_len: u64,
        write: bool,
    ) -> Result<VmTouchPlan, ProcessError> {
        if write && target_region.copy_on_write {
            self.record_vm_agent_decision(
                VmAgentKind::FaultClassifierAgent,
                pid,
                target_region.vm_object_id,
                cursor,
                chunk_len,
                1,
                target_region.file_offset,
            );
            let source = self
                .vm
                .objects
                .get(&target_region.vm_object_id)
                .ok_or(ProcessError::InvalidMemoryLayout)?
                .clone();
            let shadow_offset =
                target_region.file_offset + cursor.saturating_sub(target_region.start);
            let shadow_depth = source.shadow_depth.saturating_add(1);
            let (left_shadow_id, right_shadow_id) = self.find_adjacent_shadow_neighbors(
                pid,
                target_region,
                cursor,
                chunk_len,
                source.id,
                shadow_depth,
            )?;
            if let Some(existing_object_id) = left_shadow_id.or(right_shadow_id) {
                self.derive_priority_vm_object(source.id, existing_object_id);
                self.vm
                    .objects
                    .get_mut(&existing_object_id)
                    .expect("reused shadow object must exist")
                    .extend_range(shadow_offset, chunk_len);
                self.record_vm_agent_decision(
                    VmAgentKind::ShadowReuseAgent,
                    pid,
                    existing_object_id,
                    shadow_offset,
                    chunk_len,
                    left_shadow_id.unwrap_or(0),
                    right_shadow_id.unwrap_or(0),
                );
                return Ok(VmTouchPlan::Direct {
                    replacement_vm_object_id: Some(existing_object_id),
                    bridge_shadow_pair: left_shadow_id
                        .zip(right_shadow_id)
                        .filter(|(left, right)| left != right),
                });
            }

            let new_shadow_id = self.allocate_vm_object(
                VmObjectKind::Anonymous,
                compose_labeled_name("", &source.name, " [cow]"),
                true,
                Some(source.id),
                shadow_offset,
                shadow_depth,
                shadow_offset,
                chunk_len,
                true,
                pid,
            );
            self.derive_priority_vm_object(source.id, new_shadow_id);
            self.record_vm_agent_decision(
                VmAgentKind::ShadowReuseAgent,
                pid,
                new_shadow_id,
                shadow_offset,
                chunk_len,
                source.id,
                u64::from(shadow_depth),
            );
            Ok(VmTouchPlan::Direct {
                replacement_vm_object_id: Some(new_shadow_id),
                bridge_shadow_pair: None,
            })
        } else {
            self.record_vm_agent_decision(
                VmAgentKind::FaultClassifierAgent,
                pid,
                target_region.vm_object_id,
                cursor,
                chunk_len,
                0,
                target_region.file_offset,
            );
            Ok(VmTouchPlan::Direct {
                replacement_vm_object_id: None,
                bridge_shadow_pair: None,
            })
        }
    }

    fn commit_vm_touch_plan(
        &mut self,
        pid: ProcessId,
        target_region: &ProcessMemoryRegion,
        cursor: u64,
        chunk_len: u64,
        write: bool,
        plan: VmTouchPlan,
    ) -> Result<VmTouchCommit, ProcessError> {
        let VmTouchPlan::Direct {
            replacement_vm_object_id,
            bridge_shadow_pair,
        } = plan;
        let (vm_object_id, pages_touched, cow_faulted_pages) = self
            .get_process_address_space_mut(pid)?
            .touch_memory(cursor, chunk_len, write, replacement_vm_object_id)?;
        let backing_offset = target_region.file_offset + cursor.saturating_sub(target_region.start);
        let object = self
            .vm
            .objects
            .get_mut(&vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let faulted_pages = if cow_faulted_pages > 0 {
            object.populate_pages(backing_offset, chunk_len, true);
            object.mark_cow_fault(chunk_len);
            self.record_vm_agent_decision(
                VmAgentKind::CowPopulateAgent,
                pid,
                vm_object_id,
                backing_offset,
                chunk_len,
                cow_faulted_pages,
                pages_touched,
            );
            cow_faulted_pages
        } else {
            let faulted_pages = object.touch_pages(backing_offset, chunk_len, write);
            self.record_vm_agent_decision(
                VmAgentKind::PageTouchAgent,
                pid,
                vm_object_id,
                backing_offset,
                chunk_len,
                faulted_pages,
                u64::from(write),
            );
            faulted_pages
        };
        if let Some((left_shadow_id, right_shadow_id)) = bridge_shadow_pair {
            self.merge_shadow_objects_for_process(pid, left_shadow_id, right_shadow_id)?;
            self.record_vm_agent_decision(
                VmAgentKind::ShadowBridgeAgent,
                pid,
                left_shadow_id,
                cursor,
                chunk_len,
                left_shadow_id,
                right_shadow_id,
            );
        }
        Ok(VmTouchCommit {
            vm_object_id,
            pages_touched,
            faulted_pages,
            cow_faulted_pages,
        })
    }

    pub fn touch_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        write: bool,
    ) -> Result<MemoryTouchStats, ProcessError> {
        let chunks = self
            .get_process_address_space(pid)?
            .range_chunks(start, length)?;
        let mut first_vm_object_id = None;
        let mut total_pages_touched: u64 = 0;
        let mut total_faulted_pages: u64 = 0;
        let mut total_cow_faulted_pages: u64 = 0;

        for target_region in chunks {
            let cursor = target_region.start;
            let chunk_len = target_region.end.saturating_sub(target_region.start);
            self.ensure_vm_object_not_quarantined(
                pid,
                target_region.vm_object_id,
                cursor,
                chunk_len,
            )?;
            let plan = self.plan_vm_touch(pid, &target_region, cursor, chunk_len, write)?;
            let commit =
                self.commit_vm_touch_plan(pid, &target_region, cursor, chunk_len, write, plan)?;
            first_vm_object_id.get_or_insert(commit.vm_object_id);
            total_pages_touched = total_pages_touched.saturating_add(commit.pages_touched);
            total_faulted_pages = total_faulted_pages.saturating_add(commit.faulted_pages);
            total_cow_faulted_pages =
                total_cow_faulted_pages.saturating_add(commit.cow_faulted_pages);
        }
        self.reconcile_vm_owners(pid)?;
        Ok(MemoryTouchStats {
            vm_object_id: first_vm_object_id.ok_or(ProcessError::InvalidMemoryLayout)?,
            pages_touched: total_pages_touched,
            faulted_pages: total_faulted_pages,
            cow_faulted_pages: total_cow_faulted_pages,
        })
    }

    pub fn load_memory_word(&mut self, pid: ProcessId, addr: u64) -> Result<u32, ProcessError> {
        let page_base = addr & !0xfffu64;
        let page_end = page_base.saturating_add(0x1000);
        let end = addr
            .checked_add(4)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        if end > page_end {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.touch_memory(pid, page_base, 0x1000, false)?;
        let region = self
            .get_process_address_space(pid)?
            .resolve_range(addr, 4)?;
        let backing_offset = region.file_offset + (addr - region.start);
        let object = self
            .vm
            .objects
            .get(&region.vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        Ok(object.words.get(&backing_offset).copied().unwrap_or(0))
    }

    pub fn store_memory_word(
        &mut self,
        pid: ProcessId,
        addr: u64,
        value: u32,
    ) -> Result<(), ProcessError> {
        let page_base = addr & !0xfffu64;
        let page_end = page_base.saturating_add(0x1000);
        let end = addr
            .checked_add(4)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        if end > page_end {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.touch_memory(pid, page_base, 0x1000, true)?;
        let region = self
            .get_process_address_space(pid)?
            .resolve_range(addr, 4)?;
        let backing_offset = region.file_offset + (addr - region.start);
        let object = self
            .vm
            .objects
            .get_mut(&region.vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        object.words.insert(backing_offset, value);
        Ok(())
    }

    pub fn update_memory_word(
        &mut self,
        pid: ProcessId,
        addr: u64,
        op: MemoryWordUpdateOp,
    ) -> Result<(u32, u32), ProcessError> {
        let page_base = addr & !0xfffu64;
        let page_end = page_base.saturating_add(0x1000);
        let end = addr
            .checked_add(4)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        if end > page_end {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let old = self.load_memory_word(pid, addr)?;
        self.touch_memory(pid, page_base, 0x1000, true)?;
        let region = self
            .get_process_address_space(pid)?
            .resolve_range(addr, 4)?;
        let backing_offset = region.file_offset + (addr - region.start);
        let object = self
            .vm
            .objects
            .get_mut(&region.vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let new = match op {
            MemoryWordUpdateOp::Set(value) => value,
            MemoryWordUpdateOp::Add(value) => old.wrapping_add(value),
            MemoryWordUpdateOp::Or(value) => old | value,
            MemoryWordUpdateOp::AndNot(value) => old & !value,
            MemoryWordUpdateOp::Xor(value) => old ^ value,
        };
        object.words.insert(backing_offset, new);
        Ok((old, new))
    }

    fn find_adjacent_shadow_neighbors(
        &self,
        pid: ProcessId,
        target_region: &ProcessMemoryRegion,
        start: u64,
        aligned_len: u64,
        shadow_source_id: u64,
        shadow_depth: u32,
    ) -> Result<(Option<u64>, Option<u64>), ProcessError> {
        let end = start
            .checked_add(aligned_len)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let shadow_offset = target_region.file_offset + start.saturating_sub(target_region.start);
        let mut left_shadow_id = None;
        let mut right_shadow_id = None;
        for region in self.get_process_address_space(pid)?.memory_map() {
            let adjacent_side = if region.end == start {
                Some(false)
            } else if region.start == end {
                Some(true)
            } else {
                None
            };
            let Some(is_right) = adjacent_side else {
                continue;
            };
            let Some(object) = self.vm.objects.get(&region.vm_object_id) else {
                continue;
            };
            if !object.can_absorb_shadow_range(
                shadow_source_id,
                shadow_depth,
                shadow_offset,
                aligned_len,
            ) {
                continue;
            }
            if is_right {
                right_shadow_id = Some(region.vm_object_id);
            } else {
                left_shadow_id = Some(region.vm_object_id);
            }
        }

        Ok((left_shadow_id, right_shadow_id))
    }

    fn merge_shadow_objects_for_process(
        &mut self,
        pid: ProcessId,
        left_shadow_id: u64,
        right_shadow_id: u64,
    ) -> Result<(), ProcessError> {
        if left_shadow_id == right_shadow_id {
            return Ok(());
        }

        let right_shadow = self
            .vm
            .objects
            .remove(&right_shadow_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let left_shadow = self
            .vm
            .objects
            .get_mut(&left_shadow_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        left_shadow.append_shadow_object(right_shadow);

        let space = self.get_process_address_space_mut(pid)?;
        for region in &mut space.memory_map {
            if region.vm_object_id == right_shadow_id {
                region.vm_object_id = left_shadow_id;
            }
        }
        space.coalesce_memory_map();
        Ok(())
    }

    pub fn vm_objects_for_process(&self, pid: ProcessId) -> Result<Vec<VmObject>, ProcessError> {
        self.get(pid)?;
        let mut objects = self
            .vm
            .objects
            .values()
            .filter(|object| object.owners.contains(&pid))
            .cloned()
            .collect::<Vec<_>>();
        objects.sort_by_key(|object| object.id);
        Ok(objects)
    }

    pub fn vm_object_id_for_address(
        &self,
        pid: ProcessId,
        addr: u64,
        length: u64,
    ) -> Result<u64, ProcessError> {
        self.resolve_vm_object_id_for_range(pid, addr, length)
    }

    pub fn quarantine_vm_object(
        &mut self,
        pid: ProcessId,
        vm_object_id: u64,
        reason: u64,
    ) -> Result<(), ProcessError> {
        self.require_vm_object_access(pid, vm_object_id)?;
        self.remember_priority_vm_object(vm_object_id);
        let object = self
            .vm
            .objects
            .get_mut(&vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        object.quarantined = true;
        object.quarantine_reason = reason;
        self.record_vm_agent_decision(
            VmAgentKind::QuarantineStateAgent,
            pid,
            vm_object_id,
            0,
            0,
            reason,
            1,
        );
        Ok(())
    }

    pub fn record_vm_policy_block(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        state_code: u64,
        operation_code: u64,
    ) {
        self.record_vm_agent_decision(
            VmAgentKind::PolicyBlockAgent,
            pid,
            0,
            start,
            length,
            state_code,
            operation_code,
        );
    }

    fn collect_vm_pressure_candidates(&self, scope: Option<ProcessId>) -> Vec<VmPressureCandidate> {
        let mut candidates = self
            .vm
            .objects
            .values()
            .filter_map(|object| {
                if object.kind != VmObjectKind::File
                    || object.quarantined
                    || object.resident_pages == 0
                    || object.owners.is_empty()
                {
                    return None;
                }
                let record_pid = if let Some(pid) = scope {
                    if !object.owners.contains(&pid) {
                        return None;
                    }
                    pid
                } else {
                    object
                        .owners
                        .iter()
                        .copied()
                        .min_by_key(|pid| pid.raw())
                        .unwrap_or(ProcessId::from_handle(ObjectHandle::new(Handle::new(0), 0)))
                };
                let score = object
                    .resident_pages
                    .saturating_mul(1024)
                    .saturating_add((object.owners.len() as u64).saturating_mul(64))
                    .saturating_add(if object.private { 17 } else { 33 })
                    .saturating_add(if object.dirty_pages > 0 { 9 } else { 3 });
                Some(VmPressureCandidate {
                    record_pid,
                    vm_object_id: object.id,
                    backing_offset: object.backing_offset,
                    byte_len: object.committed_pages.saturating_mul(object.page_size),
                    resident_pages: object.resident_pages,
                    score,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| right.resident_pages.cmp(&left.resident_pages))
                .then_with(|| left.vm_object_id.cmp(&right.vm_object_id))
        });
        candidates
    }

    fn reclaim_vm_pressure_candidates(
        &mut self,
        scope: Option<ProcessId>,
        target_pages: u64,
    ) -> Result<u64, ProcessError> {
        let actor_pid =
            scope.unwrap_or(ProcessId::from_handle(ObjectHandle::new(Handle::new(0), 0)));
        let candidates = self.collect_vm_pressure_candidates(scope);
        self.record_vm_agent_decision(
            VmAgentKind::PressureTriggerAgent,
            actor_pid,
            0,
            0,
            target_pages,
            target_pages,
            candidates.len() as u64,
        );
        let mut reclaimed_pages = 0u64;
        for candidate in candidates {
            if reclaimed_pages >= target_pages {
                break;
            }
            self.remember_priority_vm_object(candidate.vm_object_id);
            let (dirty_pages_before, synced_pages_before, sync_count_before) = {
                let object = self
                    .vm
                    .objects
                    .get(&candidate.vm_object_id)
                    .ok_or(ProcessError::InvalidMemoryLayout)?;
                (object.dirty_pages, object.synced_pages, object.sync_count)
            };
            if dirty_pages_before > 0 {
                {
                    let object = self
                        .vm
                        .objects
                        .get_mut(&candidate.vm_object_id)
                        .ok_or(ProcessError::InvalidMemoryLayout)?;
                    object.sync_pages(candidate.backing_offset, candidate.byte_len);
                }
                self.persist_file_backing_from_object(candidate.vm_object_id);
                let (synced_pages_after, sync_count_after) = {
                    let object = self
                        .vm
                        .objects
                        .get(&candidate.vm_object_id)
                        .ok_or(ProcessError::InvalidMemoryLayout)?;
                    (object.synced_pages, object.sync_count)
                };
                self.record_vm_agent_decision(
                    VmAgentKind::SyncAgent,
                    candidate.record_pid,
                    candidate.vm_object_id,
                    candidate.backing_offset,
                    candidate.byte_len,
                    synced_pages_after.saturating_sub(synced_pages_before),
                    sync_count_after.saturating_sub(sync_count_before),
                );
            }
            let resident_after = {
                let object = self
                    .vm
                    .objects
                    .get_mut(&candidate.vm_object_id)
                    .ok_or(ProcessError::InvalidMemoryLayout)?;
                object.advise_pages(
                    candidate.backing_offset,
                    candidate.byte_len,
                    MemoryAdvice::DontNeed,
                );
                object.resident_pages
            };
            self.record_vm_agent_decision(
                VmAgentKind::AdviceAgent,
                candidate.record_pid,
                candidate.vm_object_id,
                candidate.backing_offset,
                candidate.byte_len,
                4,
                resident_after,
            );
            let victim_reclaimed = candidate.resident_pages.saturating_sub(resident_after);
            reclaimed_pages = reclaimed_pages.saturating_add(victim_reclaimed);
            self.record_vm_agent_decision(
                VmAgentKind::PressureVictimAgent,
                candidate.record_pid,
                candidate.vm_object_id,
                candidate.backing_offset,
                candidate.byte_len,
                victim_reclaimed,
                candidate.score,
            );
        }
        Ok(reclaimed_pages)
    }

    pub fn reclaim_memory_pressure(
        &mut self,
        pid: ProcessId,
        target_pages: u64,
    ) -> Result<u64, ProcessError> {
        self.get(pid)?;
        self.reclaim_vm_pressure_candidates(Some(pid), target_pages)
    }

    pub fn reclaim_memory_pressure_global(
        &mut self,
        target_pages: u64,
    ) -> Result<u64, ProcessError> {
        self.reclaim_vm_pressure_candidates(None, target_pages)
    }

    pub fn release_vm_object_quarantine(
        &mut self,
        pid: ProcessId,
        vm_object_id: u64,
    ) -> Result<(), ProcessError> {
        self.require_vm_object_access(pid, vm_object_id)?;
        self.remember_priority_vm_object(vm_object_id);
        let object = self
            .vm
            .objects
            .get_mut(&vm_object_id)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let reason = object.quarantine_reason;
        object.quarantined = false;
        object.quarantine_reason = 0;
        self.record_vm_agent_decision(
            VmAgentKind::QuarantineStateAgent,
            pid,
            vm_object_id,
            0,
            0,
            reason,
            0,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn allocate_vm_object(
        &mut self,
        kind: VmObjectKind,
        name: String,
        private: bool,
        shadow_source_id: Option<u64>,
        shadow_source_offset: u64,
        shadow_depth: u32,
        backing_offset: u64,
        byte_len: u64,
        dirty: bool,
        owner: ProcessId,
    ) -> u64 {
        let id = self.vm.next_object_id;
        self.vm.next_object_id = self.vm.next_object_id.saturating_add(1);
        let committed_pages = byte_len / 0x1000;
        let resident_pages = initial_resident_pages(kind, committed_pages);
        let dirty_pages = initial_dirty_pages(kind, resident_pages, dirty);
        let mut pages = PctrieMap::new();
        for page_index in 0..committed_pages {
            pages.insert(
                page_index,
                VmPageState {
                    resident: page_index < resident_pages,
                    dirty: page_index < dirty_pages,
                    accessed: false,
                },
            );
        }
        self.vm.objects.insert(
            id,
            VmObject {
                id,
                kind,
                name,
                private,
                quarantined: false,
                quarantine_reason: 0,
                shadow_source_id,
                shadow_source_offset,
                shadow_depth,
                backing_offset,
                page_size: 0x1000,
                committed_pages,
                resident_pages,
                dirty_pages,
                accessed_pages: 0,
                fault_count: 0,
                read_fault_count: 0,
                write_fault_count: 0,
                cow_fault_count: 0,
                sync_count: 0,
                synced_pages: 0,
                words: BTreeMap::new(),
                pages,
                owners: vec![owner],
            },
        );
        id
    }

    fn assign_vm_objects(&mut self, pid: ProcessId, memory_map: &mut [ProcessMemoryRegion]) {
        for region in memory_map {
            let object_id = self.allocate_vm_object(
                inferred_vm_object_kind(&region.label),
                normalize_vm_object_name(&region.label),
                region.private,
                None,
                0,
                0,
                region.file_offset,
                region.end.saturating_sub(region.start),
                region.dirty,
                pid,
            );
            region.vm_object_id = object_id;
            region.share_count = 1;
            region.copy_on_write = false;
        }
    }

    fn get_or_create_file_vm_object(
        &mut self,
        owner: ProcessId,
        path: String,
        file_offset: u64,
        byte_len: u64,
        private: bool,
        dirty: bool,
    ) -> u64 {
        if !private
            && let Some((id, object)) = self.vm.objects.iter_mut().find(|(_, object)| {
                object.kind == VmObjectKind::File
                    && !object.private
                    && object.name == path
                    && object.backing_offset == file_offset
                    && object.committed_pages.saturating_mul(object.page_size) == byte_len
            })
        {
            if !object.owners.contains(&owner) {
                object.owners.push(owner);
            }
            return *id;
        }
        let object_id = self.allocate_vm_object(
            VmObjectKind::File,
            path.clone(),
            private,
            None,
            0,
            0,
            file_offset,
            byte_len,
            dirty,
            owner,
        );
        self.restore_file_backing_into_object(object_id, &path, file_offset, byte_len);
        object_id
    }

    fn file_backing_key(
        path: &str,
        file_offset: u64,
        byte_len: u64,
    ) -> crate::vm_model::FileVmBackingKey {
        crate::vm_model::FileVmBackingKey {
            path: path.to_string(),
            backing_offset: file_offset,
            byte_len,
        }
    }

    fn restore_file_backing_into_object(
        &mut self,
        object_id: u64,
        path: &str,
        file_offset: u64,
        byte_len: u64,
    ) {
        let key = Self::file_backing_key(path, file_offset, byte_len);
        let Some(backing) = self.vm.file_backings.get(&key).cloned() else {
            return;
        };
        if let Some(object) = self.vm.objects.get_mut(&object_id) {
            object.words = backing.words;
        }
    }

    fn persist_file_backing_from_object(&mut self, object_id: u64) {
        let Some(object) = self.vm.objects.get(&object_id) else {
            return;
        };
        if object.kind != VmObjectKind::File {
            return;
        }
        let key = Self::file_backing_key(
            &object.name,
            object.backing_offset,
            object.committed_pages.saturating_mul(object.page_size),
        );
        self.vm.file_backings.insert(
            key,
            crate::vm_model::FileVmBackingState {
                words: object.words.clone(),
            },
        );
    }

    fn unregister_vm_owners(&mut self, pid: ProcessId) {
        for object in self.vm.objects.values_mut() {
            object.owners.retain(|owner| *owner != pid);
        }
        self.vm
            .objects
            .retain(|_, object| !object.owners.is_empty());
        self.refresh_vm_region_metadata();
    }

    fn reconcile_vm_owners(&mut self, pid: ProcessId) -> Result<(), ProcessError> {
        let process_object_ids = self
            .get_process_address_space(pid)?
            .memory_map()
            .iter()
            .map(|region| region.vm_object_id)
            .collect::<Vec<_>>();

        for object in self.vm.objects.values_mut() {
            object.owners.retain(|owner| *owner != pid);
        }
        for object_id in process_object_ids {
            if let Some(object) = self.vm.objects.get_mut(&object_id)
                && !object.owners.contains(&pid)
            {
                object.owners.push(pid);
            }
        }
        self.vm
            .objects
            .retain(|_, object| !object.owners.is_empty());
        self.refresh_vm_region_metadata();
        Ok(())
    }

    fn refresh_vm_region_metadata(&mut self) {
        let ref_counts = self
            .vm
            .objects
            .iter()
            .map(|(id, object)| (*id, object.owners.len() as u32))
            .collect::<BTreeMap<_, _>>();
        let mut committed_by_owner = BTreeMap::<(u64, u64), u64>::new();
        for (handle, process) in self.objects.iter() {
            let pid = ProcessId::from_handle(handle);
            let Some(space_id) = process.address_space() else {
                continue;
            };
            let Ok(space) = self.address_spaces.get(space_id.handle()) else {
                continue;
            };
            for region in space.memory_map() {
                let pages = region.end.saturating_sub(region.start) / 0x1000;
                let entry = committed_by_owner
                    .entry((region.vm_object_id, pid.raw()))
                    .or_insert(0);
                *entry = entry.saturating_add(pages);
            }
        }
        for (id, object) in &mut self.vm.objects {
            let mut committed_pages = 0;
            for ((object_id, _), pages) in &committed_by_owner {
                if object_id == id {
                    committed_pages = committed_pages.max(*pages);
                }
            }
            object.committed_pages = committed_pages;
            if matches!(
                object.kind,
                VmObjectKind::Image | VmObjectKind::Heap | VmObjectKind::Stack
            ) {
                object.resident_pages = committed_pages;
            } else {
                object.resident_pages = object.resident_pages.min(committed_pages);
            }
            object.accessed_pages = object.accessed_pages.min(object.resident_pages);
            object.dirty_pages = object.dirty_pages.min(object.resident_pages);
        }
        let handles = self
            .objects
            .iter()
            .map(|(handle, _)| handle)
            .collect::<Vec<_>>();
        for handle in handles {
            if let Ok(process) = self.objects.get(handle) {
                let Some(space_id) = process.address_space() else {
                    continue;
                };
                if let Ok(space) = self.address_spaces.get_mut(space_id.handle()) {
                    for region in &mut space.memory_map {
                        let refs = ref_counts.get(&region.vm_object_id).copied().unwrap_or(0);
                        region.share_count = refs;
                        region.copy_on_write = region.private && refs > 1;
                    }
                }
            }
        }
    }

    pub fn ready_queue(&self) -> Vec<ProcessId> {
        self.objects
            .iter()
            .filter_map(|(handle, process)| {
                (process.state() == ProcessState::Ready).then_some(ProcessId::from_handle(handle))
            })
            .collect()
    }

    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }

    pub fn threads_for_process(&self, pid: ProcessId) -> Result<Vec<ThreadId>, ProcessError> {
        Ok(self.get(pid)?.threads().to_vec())
    }

    fn create_main_thread(&mut self, pid: ProcessId) -> Result<ThreadId, ProcessError> {
        let process = self.get(pid)?.clone();
        let handle = self
            .threads
            .insert(Thread::new_main_unbound(pid, process.name()))
            .map_err(ProcessError::from_thread_object_error)?;
        let tid = ThreadId::from_handle(handle);
        let thread = self
            .threads
            .get_mut(handle)
            .map_err(ProcessError::from_thread_object_error)?;
        thread.attach_tid(tid);
        thread.sync_from_process(&process);
        self.objects
            .get_mut(pid.handle())
            .map_err(ProcessError::from_object_error)?
            .attach_main_thread(tid);
        Ok(tid)
    }

    fn sync_main_thread(&mut self, pid: ProcessId) -> Result<(), ProcessError> {
        let process = self.get(pid)?.clone();
        let Some(tid) = process.main_thread() else {
            return Ok(());
        };
        self.threads
            .get_mut(tid.handle())
            .map_err(ProcessError::from_thread_object_error)?
            .sync_from_process(&process);
        Ok(())
    }
}

fn is_priority_vm_agent(agent: VmAgentKind) -> bool {
    matches!(
        agent,
        VmAgentKind::MapAgent
            | VmAgentKind::BrkAgent
            | VmAgentKind::ProtectAgent
            | VmAgentKind::UnmapAgent
            | VmAgentKind::PolicyBlockAgent
            | VmAgentKind::PressureTriggerAgent
            | VmAgentKind::PressureVictimAgent
            | VmAgentKind::QuarantineStateAgent
            | VmAgentKind::QuarantineBlockAgent
    )
}

fn heap_region(space: &AddressSpace) -> Option<ProcessMemoryRegion> {
    space
        .memory_map()
        .iter()
        .find(|region| region.label == " [heap]")
        .cloned()
}
