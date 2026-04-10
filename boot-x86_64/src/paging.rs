use core::arch::asm;
use core::mem::size_of;
use core::ptr;
use core::slice;

use platform_hal::{MemoryPermissions, PageMapping};
use platform_x86_64::{
    EarlyFrameAllocator, FrameAllocatorError, PAGE_SIZE_1G, PAGE_SIZE_2M, PAGE_SIZE_4K, PageTable,
    PhysicalFrameRun, X86_64KernelLayout, align_down, align_up, highest_bootstrap_physical_address,
};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};
use crate::{EarlyBootState, phys_alloc, serial};

const EFER_MSR: u32 = 0xc000_0080;
const ENTRY_PRESENT: u64 = 1 << 0;
const ENTRY_WRITABLE: u64 = 1 << 1;
const ENTRY_USER: u64 = 1 << 2;
const ENTRY_LARGE_PAGE: u64 = 1 << 7;
const ENTRY_NO_EXECUTE: u64 = 1u64 << 63;
const ENTRY_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingBringupError {
    AllocatePageTableFrames(FrameAllocatorError),
    ArenaVirtualOverflow { physical_start: u64 },
    BootStackOutsideKernelWindow { stack_base: u64, kernel_base: u64 },
    BootStackPhysicalOverflow { physical_base: u64, offset: u64 },
    KernelMappingTooLarge { len: u64, capacity: u64 },
    DirectMapTooLarge { len: u64, capacity: u64 },
    MapBootStack(PageMapError),
    Cr3ReloadMismatch { expected: u64, observed: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMapError {
    InvalidMapping { vaddr: u64, len: u64 },
    AllocateFrames(FrameAllocatorError),
    ArenaVirtualOverflow { physical_start: u64 },
    EntryConflict { vaddr: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct PageInit<'a> {
    pub bytes: &'a [u8],
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct InstalledMapping {
    pub frames: PhysicalFrameRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePageTables {
    root_phys: u64,
    physical_memory_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectMapLayoutPlan {
    pml4_index: usize,
    pdpt_index: usize,
    pd_count: usize,
    pdpt_count: usize,
    capacity: u64,
}

pub fn bring_up<const N: usize>(
    state: &EarlyBootState<'static>,
    allocator: &mut EarlyFrameAllocator<N>,
) -> Result<ActivePageTables, PagingBringupError> {
    boot_locator::event(
        BootLocatorStage::Paging,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x300,
        BootPayloadLabel::Address,
        state.boot_info.kernel_phys_range.start,
        BootPayloadLabel::Length,
        state.kernel_image_len,
    );
    serial::debug_marker(b'R');
    let layout = X86_64KernelLayout::new(
        state.layout.kernel_base,
        state.layout.direct_map_base,
        state.layout.direct_map_size,
        state.layout.boot_stack_base,
        state.layout.boot_stack_size,
    );
    let kernel_phys_base = align_down(state.boot_info.kernel_phys_range.start, PAGE_SIZE_4K);
    let kernel_phys_delta = state
        .boot_info
        .kernel_phys_range
        .start
        .saturating_sub(kernel_phys_base);
    let kernel_len = align_up(
        state
            .kernel_image_len
            .saturating_add(kernel_phys_delta)
            .max(
                state
                    .boot_info
                    .kernel_phys_range
                    .len
                    .saturating_add(kernel_phys_delta),
            ),
        PAGE_SIZE_4K,
    );
    let identity_len = PAGE_SIZE_2M;
    let direct_map_len = align_up(
        highest_bootstrap_physical_address(&state.boot_info).max(PAGE_SIZE_2M),
        PAGE_SIZE_2M,
    );
    boot_locator::event(
        BootLocatorStage::Paging,
        BootLocatorKind::Memory,
        BootLocatorSeverity::Info,
        0x310,
        BootPayloadLabel::Length,
        direct_map_len,
        BootPayloadLabel::Length,
        kernel_len,
    );

    let kernel_pd_index = pd_index(layout.kernel_base);
    let kernel_pt_count = ceil_div(kernel_len, PAGE_SIZE_2M) as usize;
    let kernel_pt_capacity = ((512 - kernel_pd_index) as u64) * PAGE_SIZE_2M;
    if kernel_len > kernel_pt_capacity || kernel_pd_index + kernel_pt_count > 512 {
        return Err(PagingBringupError::KernelMappingTooLarge {
            len: kernel_len,
            capacity: kernel_pt_capacity,
        });
    }
    let direct_map_plan = direct_map_layout_plan(layout, direct_map_len);
    if direct_map_len > direct_map_plan.capacity
        || direct_map_plan.pml4_index + direct_map_plan.pdpt_count > kernel_pml4_limit(layout)
    {
        return Err(PagingBringupError::DirectMapTooLarge {
            len: direct_map_len,
            capacity: direct_map_plan.capacity,
        });
    }

    let cr0 = read_cr0();
    let cr3_before = read_cr3();
    let cr4 = read_cr4();
    let efer = read_efer();
    let kernel_pml4_index = pml4_index(layout.kernel_base);
    let kernel_pdpt_index = pdpt_index(layout.kernel_base);
    let table_count = 5 + kernel_pt_count + direct_map_plan.pdpt_count + direct_map_plan.pd_count;
    let arena = allocator
        .allocate_frames(table_count)
        .map_err(PagingBringupError::AllocatePageTableFrames)?;
    boot_locator::event(
        BootLocatorStage::Paging,
        BootLocatorKind::Memory,
        BootLocatorSeverity::Info,
        0x320,
        BootPayloadLabel::Address,
        arena.start,
        BootPayloadLabel::Count,
        table_count as u64,
    );

    let arena_virt = state
        .boot_info
        .physical_memory_offset
        .checked_add(arena.start)
        .ok_or(PagingBringupError::ArenaVirtualOverflow {
            physical_start: arena.start,
        })?;
    let tables_ptr = arena_virt as *mut PageTable;
    unsafe {
        ptr::write_bytes(
            tables_ptr.cast::<u8>(),
            0,
            table_count.saturating_mul(size_of::<PageTable>()),
        );
    }
    let tables = unsafe { slice::from_raw_parts_mut(tables_ptr, table_count) };

    let pml4_index_slot = 0usize;
    let low_pdpt_index_slot = 1usize;
    let low_pd_index_slot = 2usize;
    let kernel_pdpt_index_slot = 3usize;
    let kernel_pd_index_slot = 4usize;
    let kernel_pts_start_slot = 5usize;
    let direct_map_pdpts_start_slot = kernel_pts_start_slot + kernel_pt_count;
    let direct_map_pds_start_slot = direct_map_pdpts_start_slot + direct_map_plan.pdpt_count;

    let root_phys = table_phys(arena.start, pml4_index_slot);
    let low_pdpt_phys = table_phys(arena.start, low_pdpt_index_slot);
    let low_pd_phys = table_phys(arena.start, low_pd_index_slot);
    let kernel_pdpt_phys = table_phys(arena.start, kernel_pdpt_index_slot);
    let kernel_pd_phys = table_phys(arena.start, kernel_pd_index_slot);

    tables[pml4_index_slot].entries[pml4_index(0)] = table_entry(low_pdpt_phys);
    tables[low_pdpt_index_slot].entries[pdpt_index(0)] = table_entry(low_pd_phys);
    tables[low_pd_index_slot].entries[pd_index(0)] = large_page_entry(0, true, false);
    tables[pml4_index_slot].entries[kernel_pml4_index] = table_entry(kernel_pdpt_phys);
    tables[kernel_pdpt_index_slot].entries[kernel_pdpt_index] = table_entry(kernel_pd_phys);

    let kernel_page_count = (kernel_len / PAGE_SIZE_4K) as usize;
    let mut mapped_kernel_pages = 0usize;
    for pt_slot in 0..kernel_pt_count {
        let table_slot = kernel_pts_start_slot + pt_slot;
        let pt_phys = table_phys(arena.start, table_slot);
        tables[kernel_pd_index_slot].entries[kernel_pd_index + pt_slot] = table_entry(pt_phys);
        for entry_index in 0..512usize {
            if mapped_kernel_pages >= kernel_page_count {
                break;
            }
            let phys = kernel_phys_base + (mapped_kernel_pages as u64) * PAGE_SIZE_4K;
            tables[table_slot].entries[entry_index] = page_entry(phys, true, true);
            mapped_kernel_pages += 1;
        }
    }

    for pdpt_slot in 0..direct_map_plan.pdpt_count {
        let table_slot = direct_map_pdpts_start_slot + pdpt_slot;
        let pdpt_phys = table_phys(arena.start, table_slot);
        tables[pml4_index_slot].entries[direct_map_plan.pml4_index + pdpt_slot] =
            table_entry(pdpt_phys);
    }
    let mut mapped_direct_bytes = 0u64;
    for pd_slot in 0..direct_map_plan.pd_count {
        let table_slot = direct_map_pds_start_slot + pd_slot;
        let pd_phys = table_phys(arena.start, table_slot);
        let pdpt_table_slot =
            direct_map_pdpts_start_slot + ((direct_map_plan.pdpt_index + pd_slot) / 512);
        let pdpt_entry_index = (direct_map_plan.pdpt_index + pd_slot) % 512;
        tables[pdpt_table_slot].entries[pdpt_entry_index] = table_entry(pd_phys);
        for entry_index in 0..512usize {
            if mapped_direct_bytes >= direct_map_len {
                break;
            }
            tables[table_slot].entries[entry_index] =
                large_page_entry(mapped_direct_bytes, true, false);
            mapped_direct_bytes += PAGE_SIZE_2M;
        }
    }

    unsafe {
        write_cr3(root_phys);
    }
    boot_locator::event(
        BootLocatorStage::Paging,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x330,
        BootPayloadLabel::Address,
        root_phys,
        BootPayloadLabel::Address,
        cr3_before,
    );
    serial::debug_marker(b'S');
    let cr3_after = read_cr3();
    if (cr3_after & !0xfff) != (root_phys & !0xfff) {
        return Err(PagingBringupError::Cr3ReloadMismatch {
            expected: root_phys,
            observed: cr3_after,
        });
    }
    serial::debug_marker(b'T');

    serial::print(format_args!(
        "ngos/x86_64: boot stack mapped virt={:#x} phys={:#x} len={:#x}\n",
        align_down(state.layout.boot_stack_base, PAGE_SIZE_4K),
        kernel_phys_base
            .saturating_add(align_down(state.layout.boot_stack_base, PAGE_SIZE_4K))
            .saturating_sub(state.layout.kernel_base),
        align_up(
            state
                .layout
                .boot_stack_base
                .saturating_add(state.layout.boot_stack_size),
            PAGE_SIZE_4K,
        )
        .saturating_sub(align_down(state.layout.boot_stack_base, PAGE_SIZE_4K))
    ));
    serial::print(format_args!(
        "ngos/x86_64: paging arena phys={:#x} frames={} bytes={:#x} virt={:#x}\n",
        arena.start,
        arena.frame_count,
        phys_alloc::frame_bytes(table_count),
        arena_virt
    ));
    serial::print(format_args!(
        "ngos/x86_64: paging cr3={:#x}->{:#x} root={:#x} tables={}\n",
        cr3_before, cr3_after, root_phys, table_count
    ));
    serial::print(format_args!(
        "ngos/x86_64: paging leaves 4k={} 2m={} 1g={}\n",
        kernel_page_count,
        1 + ((direct_map_len / PAGE_SIZE_2M) as usize),
        0
    ));
    serial::print(format_args!(
        "ngos/x86_64: paging windows identity={:#x} kernel={:#x}..{:#x} direct_map={:#x}..{:#x}\n",
        identity_len,
        layout.kernel_base,
        layout.kernel_base.saturating_add(kernel_len),
        layout.direct_map_base,
        layout.direct_map_base.saturating_add(direct_map_len)
    ));
    serial::print(format_args!(
        "ngos/x86_64: control regs cr0={:#x} cr4={:#x} efer={:#x}\n",
        cr0, cr4, efer
    ));
    serial::print(format_args!(
        "ngos/x86_64: post-paging handoff regions={} cmdline_present={}\n",
        state.boot_info.memory_regions.len(),
        state.boot_info.command_line.is_some()
    ));
    boot_locator::event(
        BootLocatorStage::Paging,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x340,
        BootPayloadLabel::Count,
        state.boot_info.memory_regions.len() as u64,
        BootPayloadLabel::Status,
        state.boot_info.command_line.is_some() as u64,
    );

    Ok(ActivePageTables {
        root_phys,
        physical_memory_offset: layout.direct_map_base,
    })
}

const fn table_entry(phys: u64) -> u64 {
    (phys & ENTRY_ADDRESS_MASK) | ENTRY_PRESENT | ENTRY_WRITABLE
}

const fn page_entry(phys: u64, writable: bool, executable: bool) -> u64 {
    let mut entry = (phys & ENTRY_ADDRESS_MASK) | ENTRY_PRESENT;
    if writable {
        entry |= ENTRY_WRITABLE;
    }
    if !executable {
        entry |= ENTRY_NO_EXECUTE;
    }
    entry
}

const fn large_page_entry(phys: u64, writable: bool, executable: bool) -> u64 {
    page_entry(phys, writable, executable) | ENTRY_LARGE_PAGE
}

const fn pml4_index(vaddr: u64) -> usize {
    ((vaddr >> 39) & 0x1ff) as usize
}

const fn table_phys(arena_start: u64, table_index: usize) -> u64 {
    arena_start.saturating_add((table_index as u64) * PAGE_SIZE_4K)
}

const fn pdpt_index(vaddr: u64) -> usize {
    ((vaddr >> 30) & 0x1ff) as usize
}

const fn pd_index(vaddr: u64) -> usize {
    ((vaddr >> 21) & 0x1ff) as usize
}

const fn pt_index(vaddr: u64) -> usize {
    ((vaddr >> 12) & 0x1ff) as usize
}

const fn kernel_pml4_limit(layout: X86_64KernelLayout) -> usize {
    pml4_index(layout.kernel_base)
}

const fn direct_map_capacity(layout: X86_64KernelLayout) -> u64 {
    let start = pml4_index(layout.direct_map_base);
    let end = kernel_pml4_limit(layout);
    if end <= start {
        0
    } else {
        ((end - start) as u64) * PAGE_SIZE_1G * 512
    }
}

const fn direct_map_layout_plan(
    layout: X86_64KernelLayout,
    direct_map_len: u64,
) -> DirectMapLayoutPlan {
    let pml4_index = pml4_index(layout.direct_map_base);
    let pdpt_index = pdpt_index(layout.direct_map_base);
    let pd_count = ceil_div(direct_map_len, PAGE_SIZE_1G) as usize;
    let pdpt_count = ceil_div((pdpt_index as u64) + (pd_count as u64), 512) as usize;
    DirectMapLayoutPlan {
        pml4_index,
        pdpt_index,
        pd_count,
        pdpt_count,
        capacity: direct_map_capacity(layout),
    }
}

const fn ceil_div(value: u64, divisor: u64) -> u64 {
    if value == 0 {
        0
    } else {
        ((value - 1) / divisor) + 1
    }
}

#[allow(dead_code)]
fn boot_stack_mapping(
    state: &EarlyBootState<'static>,
    kernel_phys_base: u64,
) -> Result<PageMapping, PagingBringupError> {
    let stack_base = align_down(state.layout.boot_stack_base, PAGE_SIZE_4K);
    let stack_top = align_up(
        state
            .layout
            .boot_stack_base
            .saturating_add(state.layout.boot_stack_size),
        PAGE_SIZE_4K,
    );
    let stack_len = stack_top.saturating_sub(stack_base);
    let stack_offset = stack_base.checked_sub(state.layout.kernel_base).ok_or(
        PagingBringupError::BootStackOutsideKernelWindow {
            stack_base,
            kernel_base: state.layout.kernel_base,
        },
    )?;
    let stack_phys_base = kernel_phys_base.checked_add(stack_offset).ok_or(
        PagingBringupError::BootStackPhysicalOverflow {
            physical_base: kernel_phys_base,
            offset: stack_offset,
        },
    )?;
    Ok(PageMapping {
        vaddr: stack_base,
        paddr: stack_phys_base,
        len: stack_len,
        perms: MemoryPermissions::read_write(),
        cache: platform_hal::CachePolicy::WriteBack,
        user: false,
    })
}

fn read_cr0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn read_cr4() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn read_efer() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") EFER_MSR,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

unsafe fn write_cr3(value: u64) {
    unsafe {
        asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
    }
}

impl ActivePageTables {
    pub const fn from_raw(root_phys: u64, physical_memory_offset: u64) -> Self {
        Self {
            root_phys,
            physical_memory_offset,
        }
    }

    pub const fn root_phys(&self) -> u64 {
        self.root_phys
    }

    #[allow(dead_code)]
    pub const fn physical_memory_offset(&self) -> u64 {
        self.physical_memory_offset
    }

    #[allow(dead_code)]
    pub fn map_pages<const N: usize>(
        self,
        allocator: &mut EarlyFrameAllocator<N>,
        mapping: PageMapping,
        init: Option<PageInit<'_>>,
    ) -> Result<InstalledMapping, PageMapError> {
        if mapping.len == 0 || mapping.vaddr % PAGE_SIZE_4K != 0 || mapping.len % PAGE_SIZE_4K != 0
        {
            return Err(PageMapError::InvalidMapping {
                vaddr: mapping.vaddr,
                len: mapping.len,
            });
        }

        let root_phys = self.root_phys;
        let physical_memory_offset = self.physical_memory_offset;

        let frame_count = ceil_div(mapping.len, PAGE_SIZE_4K) as usize;
        let frames = allocator
            .allocate_frames(frame_count)
            .map_err(PageMapError::AllocateFrames)?;
        let bytes_virt = physical_memory_offset.checked_add(frames.start).ok_or(
            PageMapError::ArenaVirtualOverflow {
                physical_start: frames.start,
            },
        )?;
        let bytes_ptr = bytes_virt as *mut u8;
        unsafe {
            ptr::write_bytes(bytes_ptr, 0, mapping.len as usize);
        }
        if let Some(init) = init {
            let end = init.offset.saturating_add(init.bytes.len());
            if end <= mapping.len as usize {
                unsafe {
                    ptr::copy_nonoverlapping(
                        init.bytes.as_ptr(),
                        bytes_ptr.add(init.offset),
                        init.bytes.len(),
                    );
                }
            }
        }

        for page_index in 0..frame_count {
            let vaddr = mapping.vaddr + (page_index as u64) * PAGE_SIZE_4K;
            let paddr = frames.start + (page_index as u64) * PAGE_SIZE_4K;
            Self::map_4k_page_raw(
                root_phys,
                physical_memory_offset,
                allocator,
                vaddr,
                paddr,
                mapping.perms,
                mapping.user,
            )?;
        }

        Ok(InstalledMapping { frames })
    }

    pub fn map_existing_physical<const N: usize>(
        self,
        allocator: &mut EarlyFrameAllocator<N>,
        mapping: PageMapping,
    ) -> Result<(), PageMapError> {
        if mapping.len == 0 || mapping.vaddr % PAGE_SIZE_4K != 0 || mapping.len % PAGE_SIZE_4K != 0
        {
            return Err(PageMapError::InvalidMapping {
                vaddr: mapping.vaddr,
                len: mapping.len,
            });
        }
        if mapping.paddr % PAGE_SIZE_4K != 0 {
            return Err(PageMapError::InvalidMapping {
                vaddr: mapping.vaddr,
                len: mapping.len,
            });
        }

        let root_phys = self.root_phys;
        let physical_memory_offset = self.physical_memory_offset;

        let page_count = ceil_div(mapping.len, PAGE_SIZE_4K) as usize;
        for page_index in 0..page_count {
            let vaddr = mapping.vaddr + (page_index as u64) * PAGE_SIZE_4K;
            let paddr = mapping.paddr + (page_index as u64) * PAGE_SIZE_4K;
            Self::map_4k_page_raw(
                root_phys,
                physical_memory_offset,
                allocator,
                vaddr,
                paddr,
                mapping.perms,
                mapping.user,
            )?;
        }
        Ok(())
    }

    pub fn flush_tlb(self) {
        let method = crate::cpu_tlb::invalidate_address_space(self.root_phys);
        crate::cpu_runtime_status::record_tlb_flush(method as u32);
    }

    pub fn activate_root(self, root_phys: u64) -> Result<(), PagingBringupError> {
        let method = crate::cpu_tlb::invalidate_address_space(root_phys);
        crate::cpu_runtime_status::record_tlb_flush(method as u32);
        let observed = read_cr3();
        if (observed & !0xfff) != (root_phys & !0xfff) {
            return Err(PagingBringupError::Cr3ReloadMismatch {
                expected: root_phys,
                observed,
            });
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn map_existing_pages<const N: usize>(
        self,
        allocator: &mut EarlyFrameAllocator<N>,
        mapping: PageMapping,
    ) -> Result<(), PageMapError> {
        if mapping.len == 0 || mapping.vaddr % PAGE_SIZE_4K != 0 || mapping.len % PAGE_SIZE_4K != 0
        {
            return Err(PageMapError::InvalidMapping {
                vaddr: mapping.vaddr,
                len: mapping.len,
            });
        }

        let root_phys = self.root_phys;
        let physical_memory_offset = self.physical_memory_offset;

        let page_count = ceil_div(mapping.len, PAGE_SIZE_4K) as usize;
        for page_index in 0..page_count {
            let vaddr = mapping.vaddr + (page_index as u64) * PAGE_SIZE_4K;
            let paddr = mapping.paddr + (page_index as u64) * PAGE_SIZE_4K;
            Self::map_4k_page_raw(
                root_phys,
                physical_memory_offset,
                allocator,
                vaddr,
                paddr,
                mapping.perms,
                mapping.user,
            )?;
        }
        Ok(())
    }

    fn map_4k_page_raw<const N: usize>(
        root_phys: u64,
        physical_memory_offset: u64,
        allocator: &mut EarlyFrameAllocator<N>,
        vaddr: u64,
        paddr: u64,
        perms: MemoryPermissions,
        user: bool,
    ) -> Result<(), PageMapError> {
        let pml4 = root_phys;
        let pdpt = Self::ensure_child_table_raw(
            root_phys,
            physical_memory_offset,
            allocator,
            pml4,
            pml4_index(vaddr),
            user,
        )?;
        let pd = Self::ensure_child_table_raw(
            root_phys,
            physical_memory_offset,
            allocator,
            pdpt,
            pdpt_index(vaddr),
            user,
        )?;
        let pt = Self::ensure_child_table_raw(
            root_phys,
            physical_memory_offset,
            allocator,
            pd,
            pd_index(vaddr),
            user,
        )?;
        let pt_table = Self::table_mut_raw(root_phys, physical_memory_offset, pt);
        let entry = &mut pt_table.entries[pt_index(vaddr)];
        if (*entry & ENTRY_PRESENT) != 0 {
            return Err(PageMapError::EntryConflict { vaddr });
        }
        *entry = leaf_entry(paddr, perms, user);
        Ok(())
    }

    fn ensure_child_table_raw<const N: usize>(
        root_phys: u64,
        physical_memory_offset: u64,
        allocator: &mut EarlyFrameAllocator<N>,
        parent_phys: u64,
        entry_index: usize,
        user: bool,
    ) -> Result<u64, PageMapError> {
        let parent = Self::table_mut_raw(root_phys, physical_memory_offset, parent_phys);
        let entry = &mut parent.entries[entry_index];
        if (*entry & ENTRY_PRESENT) != 0 {
            if (*entry & ENTRY_LARGE_PAGE) != 0 {
                return Err(PageMapError::EntryConflict {
                    vaddr: index_virtual(entry_index),
                });
            }
            if user {
                *entry |= ENTRY_USER;
            }
            return Ok(*entry & ENTRY_ADDRESS_MASK);
        }

        let child = allocator
            .allocate_frames(1)
            .map_err(PageMapError::AllocateFrames)?;
        if child.start % PAGE_SIZE_4K != 0 {
            panic!(
                "misaligned child table frame start={:#x} frames={} root={:#x} hhdm={:#x}",
                child.start, child.frame_count, root_phys, physical_memory_offset
            );
        }
        let child_virt = physical_memory_offset.checked_add(child.start).ok_or(
            PageMapError::ArenaVirtualOverflow {
                physical_start: child.start,
            },
        )?;
        unsafe {
            ptr::write_bytes(child_virt as *mut u8, 0, PAGE_SIZE_4K as usize);
        }
        *entry = table_entry(child.start) | if user { ENTRY_USER } else { 0 };
        Ok(child.start)
    }

    fn table_mut_raw(
        root_phys: u64,
        physical_memory_offset: u64,
        phys: u64,
    ) -> &'static mut PageTable {
        if phys % PAGE_SIZE_4K != 0 {
            panic!(
                "misaligned page table physical address phys={:#x} hhdm={:#x} root={:#x}",
                phys, physical_memory_offset, root_phys
            );
        }
        if physical_memory_offset % PAGE_SIZE_4K != 0 {
            panic!(
                "misaligned page table hhdm={:#x} phys={:#x} root={:#x}",
                physical_memory_offset, phys, root_phys
            );
        }
        let virt = physical_memory_offset + phys;
        unsafe { &mut *(virt as *mut PageTable) }
    }
}

const fn leaf_entry(phys: u64, perms: MemoryPermissions, user: bool) -> u64 {
    let mut entry = (phys & ENTRY_ADDRESS_MASK) | ENTRY_PRESENT;
    if perms.write {
        entry |= ENTRY_WRITABLE;
    }
    if user {
        entry |= ENTRY_USER;
    }
    if !perms.execute {
        entry |= ENTRY_NO_EXECUTE;
    }
    entry
}

const fn index_virtual(entry_index: usize) -> u64 {
    (entry_index as u64) << 12
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_stack_mapping_tracks_linked_kernel_offset() {
        let boot_info = platform_x86_64::BootInfo {
            protocol: platform_x86_64::BootProtocol::LoaderDefined,
            command_line: None,
            rsdp: None,
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0xffff_8000_0000_0000,
            kernel_phys_range: platform_x86_64::BootMemoryRegion {
                start: 0x20_0000,
                len: 0x40_0000,
                kind: platform_x86_64::BootMemoryRegionKind::KernelImage,
            },
        };
        let state = EarlyBootState {
            boot_info,
            layout: X86_64KernelLayout::new(
                0xffff_ffff_8000_0000,
                0xffff_8000_0000_0000,
                512 * PAGE_SIZE_1G,
                0xffff_ffff_8008_0650,
                0x40_000,
            ),
            boot_requirements: platform_x86_64::X86_64BootRequirements::baseline(),
            bootstrap_span_bytes: 0,
            kernel_image_len: 0x40_0000,
        };

        let mapping = boot_stack_mapping(&state, 0x20_0000).expect("boot stack should map");

        assert_eq!(mapping.vaddr, 0xffff_ffff_8008_0000);
        assert_eq!(mapping.paddr, 0x28_0000);
        assert_eq!(mapping.len, 0x41_000);
        assert!(mapping.perms.write);
        assert!(!mapping.perms.execute);
    }

    #[test]
    fn direct_map_layout_spans_multiple_pml4_slots_when_needed() {
        let layout = X86_64KernelLayout::higher_half_default();
        let plan = direct_map_layout_plan(layout, 10 * 1024 * PAGE_SIZE_1G);

        assert_eq!(plan.pml4_index, 256);
        assert_eq!(plan.pdpt_index, 0);
        assert_eq!(plan.pd_count, 10 * 1024);
        assert_eq!(plan.pdpt_count, 20);
        assert_eq!(plan.capacity, 255 * 512 * PAGE_SIZE_1G);
        assert!(plan.pml4_index + plan.pdpt_count <= kernel_pml4_limit(layout));
    }

    #[test]
    fn direct_map_layout_preserves_single_slot_bootstrap_case() {
        let layout = X86_64KernelLayout::higher_half_default();
        let plan = direct_map_layout_plan(layout, 64 * PAGE_SIZE_1G);

        assert_eq!(plan.pd_count, 64);
        assert_eq!(plan.pdpt_count, 1);
    }
}
