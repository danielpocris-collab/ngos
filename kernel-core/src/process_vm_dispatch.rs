use super::*;

impl KernelSyscallSurface {
    pub(crate) fn dispatch_process_vm(
        &mut self,
        context: &SyscallContext,
        syscall: &Syscall,
    ) -> Result<Option<SyscallResult>, SyscallError> {
        let result = match syscall {
            Syscall::SpawnProcess(request) => {
                context.require(CapabilityRights::WRITE)?;
                let pid = self.runtime.spawn_process(
                    request.name.clone(),
                    request.parent,
                    request.class,
                )?;
                SyscallResult::ProcessSpawned(pid)
            }
            Syscall::SpawnProcessCopyFds(request) => {
                context.require(CapabilityRights::WRITE)?;
                let pid = self.runtime.spawn_process_copy_fds(
                    request.name.clone(),
                    request.parent,
                    request.class,
                    request.source,
                )?;
                SyscallResult::ProcessSpawned(pid)
            }
            Syscall::SpawnProcessShareFds(request) => {
                context.require(CapabilityRights::WRITE)?;
                let pid = self.runtime.spawn_process_share_fds(
                    request.name.clone(),
                    request.parent,
                    request.class,
                    request.source,
                )?;
                SyscallResult::ProcessSpawned(pid)
            }
            Syscall::SpawnProcessCopyVm(request) => {
                context.require(CapabilityRights::WRITE)?;
                let pid = self.runtime.spawn_process_copy_vm(
                    request.name.clone(),
                    request.parent,
                    request.class,
                    request.source,
                )?;
                SyscallResult::ProcessSpawned(pid)
            }
            Syscall::SpawnProcessFromSource(request) => {
                context.require(CapabilityRights::WRITE)?;
                let pid = self.runtime.spawn_process_from_source(
                    request.name.clone(),
                    request.parent,
                    request.class,
                    request.source,
                    request.filedesc_mode,
                    request.vm_mode,
                )?;
                SyscallResult::ProcessSpawned(pid)
            }
            Syscall::SetProcessArgs(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .set_process_args(request.pid, request.argv.clone())?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::SetProcessEnv(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .set_process_env(request.pid, request.envp.clone())?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::SetProcessCwd(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .set_process_cwd(request.pid, request.cwd.clone())?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::ExecProcess(request) => {
                context.require(CapabilityRights::EXECUTE)?;
                let descriptors = self.runtime.exec_process(
                    request.pid,
                    request.path.clone(),
                    request.argv.clone(),
                    request.envp.clone(),
                )?;
                SyscallResult::ExecTransitioned(descriptors)
            }
            Syscall::MapAnonymousMemory(request) => {
                context.require(CapabilityRights::WRITE)?;
                let start = self.runtime.map_anonymous_memory(
                    request.pid,
                    request.length,
                    request.readable,
                    request.writable,
                    request.executable,
                    request.label.clone(),
                )?;
                SyscallResult::MemoryMapped(start)
            }
            Syscall::MapFileMemory(request) => {
                context.require(CapabilityRights::WRITE)?;
                let start = self.runtime.map_file_memory(
                    request.pid,
                    request.path.clone(),
                    request.length,
                    request.file_offset,
                    request.readable,
                    request.writable,
                    request.executable,
                    request.private,
                )?;
                SyscallResult::MemoryMapped(start)
            }
            Syscall::UnmapMemory(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .unmap_memory(request.pid, request.start, request.length)?;
                SyscallResult::MemoryUnmapped
            }
            Syscall::ProtectMemory(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.protect_memory(
                    request.pid,
                    request.start,
                    request.length,
                    request.readable,
                    request.writable,
                    request.executable,
                )?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::AdviseMemory(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.advise_memory(
                    request.pid,
                    request.start,
                    request.length,
                    request.advice,
                )?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::SyncMemory(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .sync_memory(request.pid, request.start, request.length)?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::TouchMemory(request) => {
                context.require(CapabilityRights::WRITE)?;
                SyscallResult::MemoryTouched(self.runtime.touch_memory(
                    request.pid,
                    request.start,
                    request.length,
                    request.write,
                )?)
            }
            Syscall::LoadMemoryWord(request) => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::MemoryWordLoaded(
                    self.runtime.load_memory_word(request.pid, request.addr)?,
                )
            }
            Syscall::CompareMemoryWord(request) => {
                context.require(CapabilityRights::READ)?;
                let observed = self.runtime.compare_memory_word(
                    request.pid,
                    request.addr,
                    request.expected,
                )?;
                SyscallResult::MemoryWordCompared {
                    expected: request.expected,
                    observed,
                }
            }
            Syscall::StoreMemoryWord(request) => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .store_memory_word(request.pid, request.addr, request.value)?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::UpdateMemoryWord(request) => {
                context.require(CapabilityRights::WRITE)?;
                let (old, new) =
                    self.runtime
                        .update_memory_word(request.pid, request.addr, request.op)?;
                SyscallResult::MemoryWordUpdated { old, new }
            }
            Syscall::SetProcessBreak(request) => {
                context.require(CapabilityRights::WRITE)?;
                SyscallResult::ProcessBreak(
                    self.runtime
                        .set_process_break(request.pid, request.new_end)?,
                )
            }
            Syscall::GrantCapability(request) => {
                context.require(CapabilityRights::ADMIN)?;
                let id = self.runtime.grant_capability(
                    request.owner,
                    request.target,
                    request.rights,
                    request.label.clone(),
                )?;
                SyscallResult::CapabilityGranted(id)
            }
            Syscall::DuplicateCapability(request) => {
                context.require(CapabilityRights::TRANSFER)?;
                let id = self.runtime.duplicate_capability(
                    request.capability,
                    request.new_owner,
                    request.rights,
                    request.label.clone(),
                )?;
                SyscallResult::CapabilityDuplicated(id)
            }
            Syscall::OpenDescriptor {
                owner,
                capability,
                kind,
                name,
            } => {
                context.require(CapabilityRights::WRITE)?;
                SyscallResult::DescriptorOpened(self.runtime.open_descriptor(
                    *owner,
                    *capability,
                    *kind,
                    name.clone(),
                )?)
            }
            Syscall::DuplicateDescriptor { owner, fd } => {
                context.require(CapabilityRights::TRANSFER)?;
                SyscallResult::DescriptorDuplicated(self.runtime.duplicate_descriptor(*owner, *fd)?)
            }
            Syscall::DuplicateDescriptorTo { owner, fd, target } => {
                context.require(CapabilityRights::TRANSFER)?;
                SyscallResult::DescriptorDuplicatedTo(
                    self.runtime.duplicate_descriptor_to(*owner, *fd, *target)?,
                )
            }
            Syscall::CloseDescriptor { owner, fd } => {
                context.require(CapabilityRights::WRITE)?;
                SyscallResult::DescriptorClosed(self.runtime.close_descriptor(*owner, *fd)?)
            }
            Syscall::ExecTransition { owner } => {
                context.require(CapabilityRights::EXECUTE)?;
                SyscallResult::ExecTransitioned(self.runtime.exec_transition(*owner)?)
            }
            Syscall::Mount { mount_path, name } => {
                context.require(CapabilityRights::ADMIN)?;
                self.runtime.mount(mount_path.clone(), name.clone())?;
                SyscallResult::Mounted
            }
            Syscall::CreateVfsNode {
                path,
                kind,
                capability,
            } => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .create_vfs_node(path.clone(), *kind, *capability)?;
                SyscallResult::VfsNodeCreated
            }
            Syscall::CreateVfsSymlink {
                path,
                target,
                capability,
            } => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .create_vfs_symlink(path.clone(), target.clone(), *capability)?;
                SyscallResult::VfsSymlinkCreated
            }
            Syscall::UnlinkPath { path } => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.unlink_path(path)?;
                SyscallResult::VfsNodeRemoved
            }
            Syscall::RenamePath { from, to } => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.rename_path(from, to)?;
                SyscallResult::VfsNodeRenamed
            }
            Syscall::OpenPath { owner, path } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::PathOpened(self.runtime.open_path(*owner, path)?)
            }
            Syscall::InspectProcess { pid } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::ProcessIntrospection(self.runtime.inspect_process(*pid)?)
            }
            Syscall::InspectVmObjectLayouts { pid } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::VmObjectLayouts(self.runtime.inspect_vm_object_layouts(*pid)?)
            }
            Syscall::InspectDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::DescriptorInspected(self.runtime.inspect_io(*owner, *fd)?.clone())
            }
            Syscall::InspectDescriptorLayout { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::DescriptorLayoutInspected(
                    self.runtime.inspect_io_layout(*owner, *fd)?,
                )
            }
            Syscall::GetDescriptorFlags { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::DescriptorFlags(self.runtime.descriptor_flags(*owner, *fd)?)
            }
            Syscall::SetCloexec { owner, fd, cloexec } => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime.set_descriptor_cloexec(*owner, *fd, *cloexec)?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::SetNonblock {
                owner,
                fd,
                nonblock,
            } => {
                context.require(CapabilityRights::WRITE)?;
                self.runtime
                    .set_descriptor_nonblock(*owner, *fd, *nonblock)?;
                SyscallResult::DescriptorFlagsUpdated
            }
            Syscall::StatPath { path } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::FileStatus(self.runtime.stat_path(path)?)
            }
            Syscall::LstatPath { path } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::FileStatus(self.runtime.lstat_path(path)?)
            }
            Syscall::ReadLink { path } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::LinkTarget(self.runtime.readlink_path(path)?)
            }
            Syscall::StatDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::FileStatus(self.runtime.fstat_descriptor(*owner, *fd)?)
            }
            Syscall::StatFs { path } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::FileSystemStatus(self.runtime.statfs(path)?)
            }
            Syscall::FiledescEntries { owner } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::FiledescEntries(self.runtime.filedesc_entries(*owner)?)
            }
            Syscall::KinfoFileEntries { owner } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::KinfoFileEntries(self.runtime.kinfo_file_entries(*owner)?)
            }
            Syscall::CloseFrom { owner, low_fd } => {
                context.require(CapabilityRights::WRITE)?;
                SyscallResult::ExecTransitioned(self.runtime.close_from(*owner, *low_fd)?)
            }
            Syscall::CloseRange {
                owner,
                start_fd,
                end_fd,
                mode,
            } => {
                context.require(CapabilityRights::WRITE)?;
                SyscallResult::ExecTransitioned(
                    self.runtime
                        .close_range(*owner, *start_fd, *end_fd, *mode)?,
                )
            }
            Syscall::FcntlDescriptor { owner, fd, cmd } => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::FcntlResult(self.runtime.fcntl(*owner, *fd, *cmd)?)
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
