//! Canonical subsystem role:
//! - subsystem: x86_64 physical-frame mediation
//! - owner layer: platform mediation
//! - semantic owner: `platform-x86_64`
//! - truth path role: platform-specific physical frame discovery and allocation
//!   mechanics for the real x86 path
//!
//! Canonical contract families handled here:
//! - physical frame run contracts
//! - boot memory region mediation contracts
//! - frame allocation mechanism contracts
//!
//! This module may mediate physical memory mechanics, but it must not redefine
//! higher-level VM ownership from `kernel-core`.

use crate::{BootInfo, BootMemoryRegion, BootMemoryRegionKind, PAGE_SIZE_4K, align_down, align_up};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalFrameRun {
    pub start: u64,
    pub frame_count: u64,
}

impl PhysicalFrameRun {
    pub const fn empty() -> Self {
        Self {
            start: 0,
            frame_count: 0,
        }
    }

    pub const fn len_bytes(self) -> u64 {
        self.frame_count.saturating_mul(PAGE_SIZE_4K)
    }

    pub const fn end(self) -> u64 {
        self.start.saturating_add(self.len_bytes())
    }

    pub const fn is_empty(self) -> bool {
        self.frame_count == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EarlyFrameAllocatorStats {
    pub usable_regions: usize,
    pub usable_frames: u64,
    pub reserved_frames: u64,
    pub allocated_frames: u64,
    pub free_frames: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameAllocatorError {
    RegionCapacityExceeded { capacity: usize },
    NoUsableMemory,
    ZeroFrameAllocation,
    OutOfMemory { requested_frames: usize },
    FreeUnsupported,
}

pub struct EarlyFrameAllocator<const N: usize> {
    regions: [PhysicalFrameRun; N],
    region_count: usize,
    usable_frames: u64,
    reserved_frames: u64,
    allocated_frames: u64,
}

impl<const N: usize> EarlyFrameAllocator<N> {
    pub const fn new() -> Self {
        Self {
            regions: [PhysicalFrameRun::empty(); N],
            region_count: 0,
            usable_frames: 0,
            reserved_frames: 0,
            allocated_frames: 0,
        }
    }

    pub fn clear(&mut self) {
        let mut index = 0usize;
        while index < N {
            self.regions[index] = PhysicalFrameRun::empty();
            index += 1;
        }
        self.region_count = 0;
        self.usable_frames = 0;
        self.reserved_frames = 0;
        self.allocated_frames = 0;
    }

    pub fn ingest_usable_regions(
        &mut self,
        boot_info: &BootInfo<'_>,
    ) -> Result<EarlyFrameAllocatorStats, FrameAllocatorError> {
        self.ingest_usable_memory_regions(boot_info.memory_regions)
    }

    pub fn ingest_usable_memory_regions(
        &mut self,
        memory_regions: &[BootMemoryRegion],
    ) -> Result<EarlyFrameAllocatorStats, FrameAllocatorError> {
        self.clear();
        let mut index = 0usize;
        while index < memory_regions.len() {
            let region = memory_regions[index];
            if region.kind != BootMemoryRegionKind::Usable {
                index += 1;
                continue;
            }
            self.add_usable_region(region.start, region.len)?;
            index += 1;
        }

        if self.region_count == 0 {
            Err(FrameAllocatorError::NoUsableMemory)
        } else {
            Ok(self.stats())
        }
    }

    pub fn reserve_range(&mut self, start: u64, len: u64) -> Result<u64, FrameAllocatorError> {
        if len == 0 {
            return Ok(0);
        }

        let reserve_start = align_down(start, PAGE_SIZE_4K);
        let reserve_end = align_up(start.saturating_add(len), PAGE_SIZE_4K);
        if reserve_end <= reserve_start {
            return Ok(0);
        }

        let mut reserved_here = 0u64;
        let mut index = 0usize;
        while index < self.region_count {
            let region = self.regions[index];
            let overlap_start = reserve_start.max(region.start);
            let overlap_end = reserve_end.min(region.end());
            if overlap_end <= overlap_start {
                index += 1;
                continue;
            }

            let removed_frames = (overlap_end - overlap_start) / PAGE_SIZE_4K;
            reserved_here = reserved_here.saturating_add(removed_frames);

            let left_frames = (overlap_start - region.start) / PAGE_SIZE_4K;
            let right_frames = (region.end() - overlap_end) / PAGE_SIZE_4K;

            if left_frames == 0 && right_frames == 0 {
                self.remove_region(index);
                continue;
            }

            if left_frames == 0 {
                self.regions[index] = PhysicalFrameRun {
                    start: overlap_end,
                    frame_count: right_frames,
                };
                index += 1;
                continue;
            }

            if right_frames == 0 {
                self.regions[index].frame_count = left_frames;
                index += 1;
                continue;
            }

            if self.region_count >= N {
                return Err(FrameAllocatorError::RegionCapacityExceeded { capacity: N });
            }

            self.regions[index].frame_count = left_frames;
            self.insert_region_at(
                index + 1,
                PhysicalFrameRun {
                    start: overlap_end,
                    frame_count: right_frames,
                },
            )?;
            index += 2;
        }

        self.reserved_frames = self.reserved_frames.saturating_add(reserved_here);
        Ok(reserved_here)
    }

    pub fn allocate_frame(&mut self) -> Result<PhysicalFrameRun, FrameAllocatorError> {
        self.allocate_frames(1)
    }

    pub fn allocate_frames(
        &mut self,
        frame_count: usize,
    ) -> Result<PhysicalFrameRun, FrameAllocatorError> {
        if frame_count == 0 {
            return Err(FrameAllocatorError::ZeroFrameAllocation);
        }

        let requested = frame_count as u64;
        for index in 0..self.region_count {
            let region = self.regions[index];
            if region.frame_count < requested {
                continue;
            }

            let allocation = PhysicalFrameRun {
                start: region.start,
                frame_count: requested,
            };
            self.regions[index].start = self.regions[index]
                .start
                .saturating_add(requested.saturating_mul(PAGE_SIZE_4K));
            self.regions[index].frame_count -= requested;
            self.allocated_frames = self.allocated_frames.saturating_add(requested);
            if self.regions[index].frame_count == 0 {
                self.remove_region(index);
            }
            return Ok(allocation);
        }

        Err(FrameAllocatorError::OutOfMemory {
            requested_frames: frame_count,
        })
    }

    pub fn allocate_frames_under(
        &mut self,
        frame_count: usize,
        max_end: u64,
    ) -> Result<PhysicalFrameRun, FrameAllocatorError> {
        if frame_count == 0 {
            return Err(FrameAllocatorError::ZeroFrameAllocation);
        }
        if max_end == 0 {
            return Err(FrameAllocatorError::OutOfMemory {
                requested_frames: frame_count,
            });
        }

        let requested = frame_count as u64;
        for index in 0..self.region_count {
            let region = self.regions[index];
            if region.start >= max_end {
                continue;
            }
            let capped_end = region.end().min(max_end);
            if capped_end <= region.start {
                continue;
            }
            let capped_frames = (capped_end - region.start) / PAGE_SIZE_4K;
            if capped_frames < requested {
                continue;
            }

            let allocation = PhysicalFrameRun {
                start: region.start,
                frame_count: requested,
            };
            self.regions[index].start = self.regions[index]
                .start
                .saturating_add(requested.saturating_mul(PAGE_SIZE_4K));
            self.regions[index].frame_count -= requested;
            self.allocated_frames = self.allocated_frames.saturating_add(requested);
            if self.regions[index].frame_count == 0 {
                self.remove_region(index);
            }
            return Ok(allocation);
        }

        Err(FrameAllocatorError::OutOfMemory {
            requested_frames: frame_count,
        })
    }

    pub fn free_frames(
        &mut self,
        _allocation: PhysicalFrameRun,
    ) -> Result<(), FrameAllocatorError> {
        Err(FrameAllocatorError::FreeUnsupported)
    }

    pub fn stats(&self) -> EarlyFrameAllocatorStats {
        let mut free_frames = 0u64;
        for index in 0..self.region_count {
            free_frames = free_frames.saturating_add(self.regions[index].frame_count);
        }
        EarlyFrameAllocatorStats {
            usable_regions: self.region_count,
            usable_frames: self.usable_frames,
            reserved_frames: self.reserved_frames,
            allocated_frames: self.allocated_frames,
            free_frames,
        }
    }

    pub fn add_usable_region(&mut self, start: u64, len: u64) -> Result<(), FrameAllocatorError> {
        let aligned_start = align_up(start, PAGE_SIZE_4K);
        let aligned_end = align_down(start.saturating_add(len), PAGE_SIZE_4K);
        if aligned_end <= aligned_start {
            return Ok(());
        }
        let run = PhysicalFrameRun {
            start: aligned_start,
            frame_count: (aligned_end - aligned_start) / PAGE_SIZE_4K,
        };
        self.append_region(run)?;
        self.usable_frames = self.usable_frames.saturating_add(run.frame_count);
        Ok(())
    }

    fn append_region(&mut self, run: PhysicalFrameRun) -> Result<(), FrameAllocatorError> {
        if run.is_empty() {
            return Ok(());
        }
        if self.region_count != 0 {
            let last_index = self.region_count - 1;
            let last = self.regions[last_index];
            if last.end() >= run.start {
                let merged_end = last.end().max(run.end());
                self.regions[last_index] = PhysicalFrameRun {
                    start: last.start,
                    frame_count: (merged_end - last.start) / PAGE_SIZE_4K,
                };
                return Ok(());
            }
        }
        if self.region_count >= N {
            return Err(FrameAllocatorError::RegionCapacityExceeded { capacity: N });
        }
        self.regions[self.region_count] = run;
        self.region_count += 1;
        Ok(())
    }

    fn insert_region_at(
        &mut self,
        index: usize,
        run: PhysicalFrameRun,
    ) -> Result<(), FrameAllocatorError> {
        if self.region_count >= N {
            return Err(FrameAllocatorError::RegionCapacityExceeded { capacity: N });
        }
        let mut cursor = self.region_count;
        while cursor > index {
            self.regions[cursor] = self.regions[cursor - 1];
            cursor -= 1;
        }
        self.regions[index] = run;
        self.region_count += 1;
        Ok(())
    }

    fn remove_region(&mut self, index: usize) {
        let mut cursor = index;
        while cursor + 1 < self.region_count {
            self.regions[cursor] = self.regions[cursor + 1];
            cursor += 1;
        }
        if self.region_count != 0 {
            self.region_count -= 1;
            self.regions[self.region_count] = PhysicalFrameRun::empty();
        }
    }
}

impl<const N: usize> Default for EarlyFrameAllocator<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BootMemoryRegion, BootProtocol};

    #[test]
    fn allocator_filters_usable_regions_and_aligns_them() {
        let regions = [
            BootMemoryRegion {
                start: 0x1003,
                len: 0x1fffd,
                kind: BootMemoryRegionKind::Usable,
            },
            BootMemoryRegion {
                start: 0x40000,
                len: 0x20000,
                kind: BootMemoryRegionKind::Reserved,
            },
            BootMemoryRegion {
                start: 0x80000,
                len: 0x20000,
                kind: BootMemoryRegionKind::Usable,
            },
        ];
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: None,
            memory_regions: &regions,
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: BootMemoryRegion {
                start: 0x20_0000,
                len: 0x10_000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };
        let mut allocator = EarlyFrameAllocator::<8>::new();

        let stats = allocator.ingest_usable_regions(&boot_info).unwrap();

        assert_eq!(stats.usable_regions, 2);
        assert_eq!(stats.usable_frames, 63);
        assert_eq!(stats.free_frames, 63);
    }

    #[test]
    fn reserve_range_splits_existing_run() {
        let mut allocator = EarlyFrameAllocator::<8>::new();
        allocator.regions[0] = PhysicalFrameRun {
            start: 0x1000,
            frame_count: 16,
        };
        allocator.region_count = 1;
        allocator.usable_frames = 16;

        let reserved = allocator.reserve_range(0x5000, 0x4000).unwrap();
        let stats = allocator.stats();

        assert_eq!(reserved, 4);
        assert_eq!(stats.reserved_frames, 4);
        assert_eq!(stats.free_frames, 12);
        assert_eq!(allocator.region_count, 2);
        assert_eq!(allocator.regions[0].start, 0x1000);
        assert_eq!(allocator.regions[0].frame_count, 4);
        assert_eq!(allocator.regions[1].start, 0x9000);
        assert_eq!(allocator.regions[1].frame_count, 8);
    }

    #[test]
    fn allocate_frames_consumes_first_fitting_region() {
        let mut allocator = EarlyFrameAllocator::<8>::new();
        allocator.regions[0] = PhysicalFrameRun {
            start: 0x1000,
            frame_count: 2,
        };
        allocator.regions[1] = PhysicalFrameRun {
            start: 0x4000,
            frame_count: 8,
        };
        allocator.region_count = 2;
        allocator.usable_frames = 10;

        let allocation = allocator.allocate_frames(4).unwrap();
        let stats = allocator.stats();

        assert_eq!(allocation.start, 0x4000);
        assert_eq!(allocation.frame_count, 4);
        assert_eq!(stats.allocated_frames, 4);
        assert_eq!(stats.free_frames, 6);
        assert_eq!(allocator.regions[1].start, 0x8000);
        assert_eq!(allocator.regions[1].frame_count, 4);
    }

    #[test]
    fn allocate_frames_under_respects_max_end() {
        let mut allocator = EarlyFrameAllocator::<8>::new();
        allocator.regions[0] = PhysicalFrameRun {
            start: 0x1000,
            frame_count: 8,
        };
        allocator.regions[1] = PhysicalFrameRun {
            start: 0x20_000,
            frame_count: 8,
        };
        allocator.region_count = 2;
        allocator.usable_frames = 16;

        let allocation = allocator.allocate_frames_under(4, 0x9000).unwrap();

        assert_eq!(allocation.start, 0x1000);
        assert_eq!(allocation.frame_count, 4);
        assert_eq!(allocator.regions[0].start, 0x5000);
        assert_eq!(allocator.regions[0].frame_count, 4);
    }

    #[test]
    fn allocate_frames_under_rejects_regions_beyond_limit() {
        let mut allocator = EarlyFrameAllocator::<8>::new();
        allocator.regions[0] = PhysicalFrameRun {
            start: 0x20_000,
            frame_count: 8,
        };
        allocator.region_count = 1;
        allocator.usable_frames = 8;

        assert_eq!(
            allocator.allocate_frames_under(1, 0x10_000),
            Err(FrameAllocatorError::OutOfMemory {
                requested_frames: 1
            })
        );
    }
}
