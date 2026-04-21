//! Canonical subsystem role:
//! - subsystem: descriptor I/O syscall dispatch
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical syscall routing for descriptor I/O families
//!
//! Canonical contract families handled here:
//! - descriptor read/write syscall contracts
//! - vectored I/O syscall contracts
//! - descriptor permission gate contracts
//!
//! This module may route canonical descriptor I/O syscalls into runtime truth,
//! but it must not invent a second descriptor or I/O model.

use super::*;

impl KernelSyscallSurface {
    pub(crate) fn dispatch_descriptor_io(
        &mut self,
        context: &SyscallContext,
        syscall: &Syscall,
    ) -> Result<Option<SyscallResult>, SyscallError> {
        let result = match syscall {
            Syscall::ReadDescriptor { owner, fd, len } => {
                context.require(CapabilityRights::READ)?;
                let bytes = self.runtime.read_io(*owner, *fd, *len)?;
                SyscallResult::DescriptorRead(bytes)
            }
            Syscall::ReadDescriptorVectored {
                owner,
                fd,
                segments,
            } => {
                context.require(CapabilityRights::READ)?;
                let bytes = self.runtime.read_io_vectored(*owner, *fd, segments)?;
                SyscallResult::DescriptorReadVectored(bytes)
            }
            Syscall::ReadDescriptorVectoredWithLayout {
                owner,
                fd,
                segments,
            } => {
                context.require(CapabilityRights::READ)?;
                let (bytes, layout) = self
                    .runtime
                    .read_io_vectored_with_layout(*owner, *fd, segments)?;
                SyscallResult::DescriptorReadVectoredWithLayout {
                    segments: bytes,
                    layout,
                }
            }
            Syscall::WriteDescriptor { owner, fd, bytes } => {
                context.require(CapabilityRights::WRITE)?;
                let written = self.runtime.write_io(*owner, *fd, bytes)?;
                SyscallResult::DescriptorWritten(written)
            }
            Syscall::WriteDescriptorVectored {
                owner,
                fd,
                segments,
            } => {
                context.require(CapabilityRights::WRITE)?;
                let written = self.runtime.write_io_vectored(*owner, *fd, segments)?;
                SyscallResult::DescriptorWritten(written)
            }
            Syscall::PollDescriptor { owner, fd } => {
                context.require(CapabilityRights::READ)?;
                let events = self.runtime.poll_io(*owner, *fd)?;
                SyscallResult::DescriptorPolled(events)
            }
            Syscall::ControlDescriptor { owner, fd, opcode } => {
                context.require(CapabilityRights::ADMIN)?;
                let response = self.runtime.control_io(*owner, *fd, *opcode)?;
                SyscallResult::DescriptorControlled(response)
            }
            Syscall::RegisterReadiness {
                owner,
                fd,
                interest,
            } => {
                context.require(CapabilityRights::READ)?;
                self.runtime.register_readiness(*owner, *fd, *interest)?;
                SyscallResult::ReadinessRegistered
            }
            Syscall::CollectReadiness => {
                context.require(CapabilityRights::READ)?;
                SyscallResult::ReadinessEvents(self.runtime.collect_ready()?)
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
