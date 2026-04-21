use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use platform_x86_64::{EarlyFrameAllocator, FrameAllocatorError, PAGE_SIZE_4K, PhysicalFrameRun};

#[derive(Clone, Copy)]
pub struct EarlyHeapAllocator;

static HEAP_START: AtomicUsize = AtomicUsize::new(0);
static HEAP_NEXT: AtomicUsize = AtomicUsize::new(0);
static HEAP_END: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapInitError {
    AllocateFrames(FrameAllocatorError),
    VirtualOverflow { physical_start: u64 },
    AlreadyInitialized,
}

unsafe impl GlobalAlloc for EarlyHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let start = HEAP_START.load(Ordering::Acquire);
        let end = HEAP_END.load(Ordering::Acquire);
        if start == 0 || end <= start {
            return ptr::null_mut();
        }

        let align = layout.align().max(1);
        let size = layout.size().max(1);
        let mut current = HEAP_NEXT.load(Ordering::Acquire);
        loop {
            let aligned = align_up_usize(current.max(start), align);
            let Some(next) = aligned.checked_add(size) else {
                return ptr::null_mut();
            };
            if next > end {
                return ptr::null_mut();
            }
            match HEAP_NEXT.compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return aligned as *mut u8,
                Err(observed) => current = observed,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

pub static GLOBAL_ALLOCATOR: EarlyHeapAllocator = EarlyHeapAllocator;

pub fn init_from_allocator<const N: usize>(
    allocator: &mut EarlyFrameAllocator<N>,
    physical_memory_offset: u64,
    frame_count: usize,
) -> Result<PhysicalFrameRun, HeapInitError> {
    if HEAP_START.load(Ordering::Acquire) != 0 {
        return Err(HeapInitError::AlreadyInitialized);
    }

    let frames = allocator
        .allocate_frames(frame_count)
        .map_err(HeapInitError::AllocateFrames)?;
    let start =
        physical_memory_offset
            .checked_add(frames.start)
            .ok_or(HeapInitError::VirtualOverflow {
                physical_start: frames.start,
            })? as usize;
    let len = frame_count.saturating_mul(PAGE_SIZE_4K as usize);
    unsafe {
        ptr::write_bytes(start as *mut u8, 0, len);
    }
    let end = start.saturating_add(len);

    HEAP_START.store(start, Ordering::Release);
    HEAP_NEXT.store(start, Ordering::Release);
    HEAP_END.store(end, Ordering::Release);
    Ok(frames)
}

pub fn allocated_bytes() -> usize {
    let start = HEAP_START.load(Ordering::Acquire);
    let next = HEAP_NEXT.load(Ordering::Acquire);
    next.saturating_sub(start)
}

pub fn capacity_bytes() -> usize {
    let start = HEAP_START.load(Ordering::Acquire);
    let end = HEAP_END.load(Ordering::Acquire);
    end.saturating_sub(start)
}

const fn align_up_usize(value: usize, align: usize) -> usize {
    if align == 0 {
        value
    } else {
        let rem = value % align;
        if rem == 0 {
            value
        } else {
            value + (align - rem)
        }
    }
}
