use super::*;

impl KernelRuntime {
    fn enforce_process_memory_contract_for_vm(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        operation_code: u64,
    ) -> Result<(), RuntimeError> {
        if let Err(err) = self.enforce_process_memory_contract(pid) {
            if let RuntimeError::NativeModel(NativeModelError::ContractNotActive { state }) = err {
                self.sync_vm_decision_tick();
                self.processes.record_vm_policy_block(
                    pid,
                    start,
                    length,
                    match state {
                        ContractState::Active => 0,
                        ContractState::Suspended => 1,
                        ContractState::Revoked => 2,
                    },
                    operation_code,
                );
                return Err(RuntimeError::NativeModel(
                    NativeModelError::ContractNotActive { state },
                ));
            }
            return Err(err);
        }
        Ok(())
    }

    fn sync_vm_decision_tick(&mut self) {
        self.processes.set_decision_tick(self.current_tick);
    }

    pub fn spawn_process(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
    ) -> Result<ProcessId, RuntimeError> {
        let pid = self.processes.spawn(name, parent)?;
        self.ensure_namespace(pid);
        self.scheduler.enqueue(&mut self.processes, pid, class)?;
        Ok(pid)
    }

    pub fn spawn_process_copy_fds(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
        source: ProcessId,
    ) -> Result<ProcessId, RuntimeError> {
        self.spawn_process_from_source(
            name,
            parent,
            class,
            source,
            SpawnFiledescMode::Copy,
            SpawnVmMode::Fresh,
        )
    }

    pub fn spawn_process_share_fds(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
        source: ProcessId,
    ) -> Result<ProcessId, RuntimeError> {
        self.spawn_process_from_source(
            name,
            parent,
            class,
            source,
            SpawnFiledescMode::Share,
            SpawnVmMode::Fresh,
        )
    }

    pub fn spawn_process_copy_vm(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
        source: ProcessId,
    ) -> Result<ProcessId, RuntimeError> {
        self.spawn_process_from_source(
            name,
            parent,
            class,
            source,
            SpawnFiledescMode::Empty,
            SpawnVmMode::Copy,
        )
    }

    pub fn spawn_process_from_source(
        &mut self,
        name: impl Into<String>,
        parent: Option<ProcessId>,
        class: SchedulerClass,
        source: ProcessId,
        filedesc_mode: SpawnFiledescMode,
        vm_mode: SpawnVmMode,
    ) -> Result<ProcessId, RuntimeError> {
        let pid = self.spawn_process(name, parent, class)?;
        if matches!(vm_mode, SpawnVmMode::Copy) {
            self.processes.copy_vm_state(pid, source)?;
        }
        if matches!(
            filedesc_mode,
            SpawnFiledescMode::Copy | SpawnFiledescMode::Share
        ) {
            let namespace = self.namespace(source)?.rebind_owner(pid);
            let dropped = self.replace_namespace(pid, namespace);
            for descriptor in &dropped {
                self.finalize_queue_descriptor_close(descriptor)?;
            }
            self.sync_io_from_namespace(pid)?;
            if matches!(filedesc_mode, SpawnFiledescMode::Share) {
                self.join_fdshare_group(source, pid);
            }
        }
        Ok(pid)
    }

    pub fn grant_capability(
        &mut self,
        owner: ProcessId,
        target: ObjectHandle,
        rights: CapabilityRights,
        label: impl Into<String>,
    ) -> Result<CapabilityId, RuntimeError> {
        self.capabilities
            .grant(&self.processes, owner, target, rights, label)
            .map_err(Into::into)
    }

    pub fn duplicate_capability(
        &mut self,
        id: CapabilityId,
        new_owner: ProcessId,
        rights: CapabilityRights,
        label: impl Into<String>,
    ) -> Result<CapabilityId, RuntimeError> {
        self.capabilities
            .duplicate_restricted(id, new_owner, rights, label, &self.processes)
            .map_err(Into::into)
    }

    pub fn revoke_capability(&mut self, id: CapabilityId) -> Result<Capability, RuntimeError> {
        self.capabilities.revoke(id).map_err(Into::into)
    }

    pub fn set_process_args(
        &mut self,
        pid: ProcessId,
        argv: Vec<String>,
    ) -> Result<(), RuntimeError> {
        self.processes.set_args(pid, argv)?;
        Ok(())
    }

    pub fn set_process_env(
        &mut self,
        pid: ProcessId,
        envp: Vec<String>,
    ) -> Result<(), RuntimeError> {
        self.processes.set_env(pid, envp)?;
        Ok(())
    }

    pub fn set_process_cwd(
        &mut self,
        pid: ProcessId,
        cwd: impl Into<String>,
    ) -> Result<(), RuntimeError> {
        let cwd = cwd.into();
        let status = self.stat_path(&cwd)?;
        if status.kind != ObjectKind::Directory {
            return Err(RuntimeError::Vfs(VfsError::NotDirectory));
        }
        self.processes.set_cwd(pid, status.path)?;
        Ok(())
    }

    pub fn set_process_root(
        &mut self,
        pid: ProcessId,
        root: impl Into<String>,
    ) -> Result<(), RuntimeError> {
        let root = root.into();
        let status = self.stat_path(&root)?;
        if status.kind != ObjectKind::Directory {
            return Err(RuntimeError::Vfs(VfsError::NotDirectory));
        }
        self.processes.set_root(pid, status.path.clone())?;
        if !self.processes.get(pid)?.cwd().starts_with(&status.path) {
            self.processes.set_cwd(pid, status.path)?;
        }
        Ok(())
    }

    fn ensure_standard_streams(&mut self, pid: ProcessId) -> Result<(), RuntimeError> {
        for (target, name) in [
            (Descriptor::new(0), "[stdio:stdin]"),
            (Descriptor::new(1), "[stdio:stdout]"),
            (Descriptor::new(2), "[stdio:stderr]"),
        ] {
            let capability = self.grant_capability(
                pid,
                pid.handle(),
                CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
                name,
            )?;
            let opened = self.open_descriptor(pid, capability, ObjectKind::Channel, name)?;
            let installed = if opened == target {
                opened
            } else {
                self.duplicate_descriptor_to(pid, opened, target)?
            };
            if installed != opened {
                let _ = self.close_descriptor(pid, opened)?;
            }
            self.io_registry
                .reset_payload(pid, installed)
                .map_err(map_runtime_io_error)?;
        }
        Ok(())
    }

    pub fn seed_standard_input(
        &mut self,
        pid: ProcessId,
        bytes: &[u8],
    ) -> Result<(), RuntimeError> {
        self.ensure_standard_streams(pid)?;
        self.io_registry
            .replace_payload(pid, Descriptor::new(0), bytes)
            .map_err(map_runtime_io_error)?;
        self.notify_descriptor_ready(pid, Descriptor::new(0))?;
        Ok(())
    }

    pub fn exec_process(
        &mut self,
        pid: ProcessId,
        path: impl Into<String>,
        argv: Vec<String>,
        envp: Vec<String>,
    ) -> Result<Vec<ObjectDescriptor>, RuntimeError> {
        let path = path.into();
        let status = self.stat_path(&path)?;
        if status.kind == ObjectKind::Directory {
            return Err(RuntimeError::Vfs(VfsError::NotExecutable));
        }
        let executable_image = executable_image_from_status(&status);
        let memory_map = default_memory_map(&executable_image);
        let auxv = default_auxiliary_vector(&status.path, executable_image.phnum);
        let closed = self.exec_transition(pid)?;
        self.processes.exec(
            pid,
            status.path,
            executable_image,
            memory_map,
            argv,
            envp,
            auxv,
        )?;
        self.ensure_standard_streams(pid)?;
        Ok(closed)
    }

    pub fn prepare_user_launch(&self, pid: ProcessId) -> Result<UserLaunchPlan, RuntimeError> {
        user_launch::prepare_user_launch(self, pid)
    }

    pub fn map_anonymous_memory(
        &mut self,
        pid: ProcessId,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: impl Into<String>,
    ) -> Result<u64, RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, 0, length, 0)?;
        self.sync_vm_decision_tick();
        let label_text = label.into();
        let label = compose_labeled_name(" [anon:", &label_text, "]");
        self.processes
            .map_anonymous_memory(pid, length, readable, writable, executable, label)
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn map_file_memory(
        &mut self,
        pid: ProcessId,
        path: impl Into<String>,
        length: u64,
        file_offset: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        private: bool,
    ) -> Result<u64, RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, 0, length, 1)?;
        self.sync_vm_decision_tick();
        let path = path.into();
        let status = self.stat_path(&path)?;
        if matches!(status.kind, ObjectKind::Directory | ObjectKind::Symlink) {
            return Err(RuntimeError::Vfs(VfsError::NotExecutable));
        }
        self.processes
            .map_file_memory(
                pid,
                status.path,
                length.max(status.size.max(1)),
                file_offset,
                readable,
                writable,
                executable,
                private,
            )
            .map_err(Into::into)
    }

    pub fn unmap_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
    ) -> Result<(), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, start, length, 2)?;
        self.sync_vm_decision_tick();
        self.processes.unmap_memory(pid, start, length)?;
        Ok(())
    }

    pub fn reclaim_memory_pressure(
        &mut self,
        pid: ProcessId,
        target_pages: u64,
    ) -> Result<u64, RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, 0, target_pages, 14)?;
        self.sync_vm_decision_tick();
        self.processes
            .reclaim_memory_pressure(pid, target_pages)
            .map_err(Into::into)
    }

    pub fn reclaim_memory_pressure_global(
        &mut self,
        target_pages: u64,
    ) -> Result<u64, RuntimeError> {
        self.sync_vm_decision_tick();
        self.processes
            .reclaim_memory_pressure_global(target_pages)
            .map_err(Into::into)
    }

    pub fn protect_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
    ) -> Result<(), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, start, length, 3)?;
        self.sync_vm_decision_tick();
        self.processes
            .protect_memory(pid, start, length, readable, writable, executable)?;
        Ok(())
    }

    pub fn advise_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        advice: MemoryAdvice,
    ) -> Result<(), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, start, length, 4)?;
        self.sync_vm_decision_tick();
        self.processes.advise_memory(pid, start, length, advice)?;
        Ok(())
    }

    pub fn sync_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
    ) -> Result<(), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, start, length, 5)?;
        self.sync_vm_decision_tick();
        self.processes.sync_memory(pid, start, length)?;
        Ok(())
    }

    pub fn touch_memory(
        &mut self,
        pid: ProcessId,
        start: u64,
        length: u64,
        write: bool,
    ) -> Result<MemoryTouchStats, RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, start, length, 6)?;
        self.sync_vm_decision_tick();
        self.processes
            .touch_memory(pid, start, length, write)
            .map_err(Into::into)
    }

    pub fn quarantine_vm_object(
        &mut self,
        pid: ProcessId,
        vm_object_id: u64,
        reason: u64,
    ) -> Result<(), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, 0, 0, 7)?;
        self.sync_vm_decision_tick();
        self.processes
            .quarantine_vm_object(pid, vm_object_id, reason)
            .map_err(Into::into)
    }

    pub fn release_vm_object_quarantine(
        &mut self,
        pid: ProcessId,
        vm_object_id: u64,
    ) -> Result<(), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, 0, 0, 8)?;
        self.sync_vm_decision_tick();
        self.processes
            .release_vm_object_quarantine(pid, vm_object_id)
            .map_err(Into::into)
    }

    pub fn load_memory_word(&mut self, pid: ProcessId, addr: u64) -> Result<u32, RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, addr, 4, 9)?;
        self.sync_vm_decision_tick();
        self.processes
            .load_memory_word(pid, addr)
            .map_err(Into::into)
    }

    pub fn compare_memory_word(
        &mut self,
        pid: ProcessId,
        addr: u64,
        expected: u32,
    ) -> Result<u32, RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, addr, 4, 10)?;
        self.sync_vm_decision_tick();
        let observed = self.processes.load_memory_word(pid, addr)?;
        let _ = expected;
        Ok(observed)
    }

    pub fn store_memory_word(
        &mut self,
        pid: ProcessId,
        addr: u64,
        value: u32,
    ) -> Result<(), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, addr, 4, 11)?;
        self.sync_vm_decision_tick();
        self.processes
            .store_memory_word(pid, addr, value)
            .map_err(Into::into)
    }

    pub fn update_memory_word(
        &mut self,
        pid: ProcessId,
        addr: u64,
        op: MemoryWordUpdateOp,
    ) -> Result<(u32, u32), RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, addr, 4, 12)?;
        self.sync_vm_decision_tick();
        self.processes
            .update_memory_word(pid, addr, op)
            .map_err(Into::into)
    }

    pub fn set_process_break(&mut self, pid: ProcessId, new_end: u64) -> Result<u64, RuntimeError> {
        self.enforce_process_memory_contract_for_vm(pid, new_end, 0, 13)?;
        self.sync_vm_decision_tick();
        self.processes.set_brk(pid, new_end).map_err(Into::into)
    }

    pub fn reap_process(&mut self, pid: ProcessId) -> Result<Process, RuntimeError> {
        let process = self.processes.reap(pid)?;
        self.emit_process_lifecycle_events(pid, ProcessLifecycleEventKind::Reaped)?;
        self.purge_reaped_process_runtime_state(pid);
        Ok(process)
    }

    pub(crate) fn purge_reaped_process_runtime_state(&mut self, pid: ProcessId) {
        self.namespaces.retain(|(owner, _)| *owner != pid);
        self.io_registry.remove_owner(pid);
        self.sleep_results.remove(&pid.raw());
        self.signal_wait_masks.remove(&pid.raw());
        self.signal_wait_queues.remove(&pid.raw());
        self.memory_wait_queues.remove(&pid.raw());
        self.remove_memory_waiter(pid);
        self.purge_descriptor_runtime_state(pid, |_| true);
        for queue in &mut self.event_queues {
            queue.remove_owner_waiters(pid);
        }
        for queue in &mut self.sleep_queues {
            queue.waiters.remove_owner(pid);
        }
        let owned_event_queues = self
            .event_queues
            .iter()
            .filter(|queue| queue.owner == pid)
            .map(|queue| queue.id)
            .collect::<Vec<_>>();
        for queue in owned_event_queues {
            if self.queue_descriptor_reference_count(QueueDescriptorTarget::Event {
                owner: pid,
                queue,
                mode: self
                    .event_queue_mode(pid, queue)
                    .unwrap_or(EventQueueMode::Kqueue),
            }) == 0
            {
                let _ = self.remove_event_queue_record(pid, queue);
            }
        }
        let owned_sleep_queues = self
            .sleep_queues
            .iter()
            .filter(|queue| queue.owner == pid)
            .map(|queue| queue.id)
            .collect::<Vec<_>>();
        for queue in owned_sleep_queues {
            if self.queue_descriptor_reference_count(QueueDescriptorTarget::Sleep {
                owner: pid,
                queue,
            }) == 0
            {
                let _ = self.remove_sleep_queue_record(pid, queue);
            }
        }
        for group in &mut self.fdshare_groups {
            group.members.retain(|member| *member != pid);
        }
        self.fdshare_groups.retain(|group| group.members.len() > 1);
    }

    pub(crate) fn purge_descriptor_runtime_state(
        &mut self,
        owner: ProcessId,
        mut should_remove: impl FnMut(Descriptor) -> bool,
    ) {
        self.readiness
            .retain(|registration| registration.owner != owner || !should_remove(registration.fd));
        for queue in &mut self.event_queues {
            queue
                .watches
                .retain(|watch| watch.owner != owner || !should_remove(watch.fd));
            queue.retain_pending(|event| {
                event.owner != owner
                    || !matches!(event.source, EventSource::Descriptor(fd) if should_remove(fd))
            });
        }
        self.deferred_tasks.retain(|task, _, _| match task {
            DeferredRuntimeTask::RefreshEventQueue(QueueDescriptorTarget::Event {
                owner: task_owner,
                ..
            }) => *task_owner != owner,
            DeferredRuntimeTask::RefreshEventQueue(QueueDescriptorTarget::Sleep { .. }) => true,
        });
    }

    pub(crate) fn purge_event_queue_runtime_state(
        &mut self,
        owner: ProcessId,
        queue: EventQueueId,
    ) {
        let binding = self
            .event_queue_mode(owner, queue)
            .map(|mode| QueueDescriptorTarget::Event { owner, queue, mode });
        self.deferred_tasks.retain(|task, _, _| match task {
            DeferredRuntimeTask::RefreshEventQueue(target) => Some(target) != binding.as_ref(),
        });
    }
}
