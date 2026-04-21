use platform_x86_64::{FrameAllocatorError, PAGE_SIZE_4K, align_up};

use crate::phys_alloc::BootFrameAllocator;

pub const RUNTIME_KERNEL_STACK_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKernelStack {
    pub base: u64,
    pub top: u64,
    pub bytes: u64,
}

pub fn allocate(
    frame_allocator: &mut BootFrameAllocator,
    physical_memory_offset: u64,
) -> Result<RuntimeKernelStack, FrameAllocatorError> {
    let bytes = align_up(RUNTIME_KERNEL_STACK_BYTES, PAGE_SIZE_4K);
    let frames = frame_allocator.allocate_frames((bytes / PAGE_SIZE_4K) as usize)?;
    let base = physical_memory_offset.saturating_add(frames.start);
    Ok(RuntimeKernelStack {
        base,
        top: base.saturating_add(bytes),
        bytes,
    })
}
