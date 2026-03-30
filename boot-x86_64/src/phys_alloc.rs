use platform_x86_64::{
    BootMemoryRegion, BootModule, EarlyFrameAllocator, FrameAllocatorError, FramebufferInfo,
    PAGE_SIZE_4K,
};

use crate::serial;

pub const BOOT_FRAME_REGION_CAPACITY: usize = 128;
pub type BootFrameAllocator = EarlyFrameAllocator<BOOT_FRAME_REGION_CAPACITY>;

#[allow(dead_code)]
const LEGACY_PHYSICAL_RESERVE_END: u64 = 0x10_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysAllocBringupError {
    FrameAllocator(FrameAllocatorError),
}

impl From<FrameAllocatorError> for PhysAllocBringupError {
    fn from(value: FrameAllocatorError) -> Self {
        Self::FrameAllocator(value)
    }
}

#[allow(dead_code)]
pub unsafe fn write_initialized(
    slot: *mut BootFrameAllocator,
    memory_regions_ptr: *const BootMemoryRegion,
    memory_regions_len: usize,
    kernel_phys_range: BootMemoryRegion,
    modules_ptr: *const BootModule<'static>,
    modules_len: usize,
    framebuffer: Option<FramebufferInfo>,
) -> Result<(), PhysAllocBringupError> {
    serial::debug_marker(b'g');
    unsafe {
        slot.write(BootFrameAllocator::new());
    }
    serial::debug_marker(b'h');
    let allocator = unsafe { &mut *slot };
    allocator.clear();
    serial::debug_marker(b'i');
    let mut memory_region_index = 0usize;
    while memory_region_index < memory_regions_len {
        let region = unsafe { *memory_regions_ptr.add(memory_region_index) };
        if region.kind == platform_x86_64::BootMemoryRegionKind::Usable {
            allocator.add_usable_region(region.start, region.len)?;
        }
        memory_region_index += 1;
    }
    serial::debug_marker(b'j');
    let initial = allocator.stats();
    if initial.usable_frames == 0 {
        return Err(PhysAllocBringupError::FrameAllocator(
            FrameAllocatorError::NoUsableMemory,
        ));
    }

    let mut reserved_frames = 0u64;
    serial::debug_marker(b'k');
    reserved_frames =
        reserved_frames.saturating_add(allocator.reserve_range(0, LEGACY_PHYSICAL_RESERVE_END)?);
    serial::debug_marker(b'l');
    reserved_frames = reserved_frames
        .saturating_add(allocator.reserve_range(kernel_phys_range.start, kernel_phys_range.len)?);
    serial::debug_marker(b'm');
    let mut module_index = 0usize;
    while module_index < modules_len {
        let module = unsafe { *modules_ptr.add(module_index) };
        reserved_frames = reserved_frames
            .saturating_add(allocator.reserve_range(module.physical_start, module.len)?);
        module_index += 1;
    }
    serial::debug_marker(b'n');
    if let Some(framebuffer) = framebuffer {
        let framebuffer_len = (framebuffer.pitch as u64).saturating_mul(framebuffer.height as u64);
        reserved_frames = reserved_frames
            .saturating_add(allocator.reserve_range(framebuffer.physical_start, framebuffer_len)?);
    }

    serial::debug_marker(b'G');
    serial::print(format_args!(
        "ngos/x86_64: phys alloc init usable_regions={} usable_frames={} reserved_frames={} free_frames={}\n",
        initial.usable_regions,
        initial.usable_frames,
        reserved_frames,
        allocator.stats().free_frames
    ));
    Ok(())
}

pub fn log_state(label: &str, allocator: &BootFrameAllocator) {
    let stats = allocator.stats();
    serial::print(format_args!(
        "ngos/x86_64: phys alloc {} usable_frames={} reserved_frames={} allocated_frames={} free_frames={}\n",
        label,
        stats.usable_frames,
        stats.reserved_frames,
        stats.allocated_frames,
        stats.free_frames
    ));
}

pub const fn frame_bytes(frame_count: usize) -> u64 {
    (frame_count as u64).saturating_mul(PAGE_SIZE_4K)
}
