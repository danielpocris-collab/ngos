use core::marker::PhantomData;

use platform_hal::{CachePolicy, MemoryPermissions, PageMapping};

use crate::{
    BootInfo, BootMemoryRegionKind, PAGE_SIZE_1G, PAGE_SIZE_2M, PAGE_SIZE_4K, X86_64KernelLayout,
    align_down, align_up, const_max_u64,
};

pub const PAGE_TABLE_ENTRIES: usize = 512;

const ENTRY_PRESENT: u64 = 1 << 0;
const ENTRY_WRITABLE: u64 = 1 << 1;
const ENTRY_USER: u64 = 1 << 2;
const ENTRY_PAGE_SIZE: u64 = 1 << 7;
const ENTRY_NO_EXECUTE: u64 = 1u64 << 63;
const ENTRY_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarlyPagingPlan {
    pub identity: PageMapping,
    pub kernel_image: PageMapping,
    pub direct_map: PageMapping,
}

impl EarlyPagingPlan {
    pub const fn minimum_coverage_bytes(self) -> u64 {
        self.identity
            .len
            .saturating_add(self.kernel_image.len)
            .saturating_add(self.direct_map.len)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PagingBuildOptions {
    pub allow_1gib_pages: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageTableBuildStats {
    pub table_pages_used: usize,
    pub mapping_regions: usize,
    pub leaf_4k_pages: usize,
    pub leaf_2m_pages: usize,
    pub leaf_1g_pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapPageTables {
    pub root_phys: u64,
    pub plan: EarlyPagingPlan,
    pub stats: PageTableBuildStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingError {
    ArenaExhausted { capacity: usize },
    MisalignedArenaBase { phys_base: u64 },
    MisalignedMapping { vaddr: u64, paddr: u64, len: u64 },
    AddressOverflow,
    EntryConflict { vaddr: u64 },
    UnsupportedForeignTable { phys: u64 },
}

#[repr(C, align(4096))]
#[derive(Clone, Copy)]
pub struct PageTable {
    pub entries: [u64; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    pub const fn zeroed() -> Self {
        Self {
            entries: [0; PAGE_TABLE_ENTRIES],
        }
    }

    pub fn clear(&mut self) {
        self.entries = [0; PAGE_TABLE_ENTRIES];
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageSize {
    Size4K,
    Size2M,
    Size1G,
}

impl PageSize {
    const fn bytes(self) -> u64 {
        match self {
            Self::Size4K => PAGE_SIZE_4K,
            Self::Size2M => PAGE_SIZE_2M,
            Self::Size1G => PAGE_SIZE_1G,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TableHandle {
    index: usize,
    phys: u64,
}

pub struct BootstrapPageTableBuilder<'a> {
    arena: PageTableArena<'a>,
    root: TableHandle,
}

impl<'a> BootstrapPageTableBuilder<'a> {
    pub fn new(tables: &'a mut [PageTable], phys_base: u64) -> Result<Self, PagingError> {
        let mut arena = PageTableArena::new(tables, phys_base)?;
        let root = arena.allocate_table()?;
        Ok(Self { arena, root })
    }

    /// # Safety
    ///
    /// `slot` must be valid for writes of `Self`, properly aligned, and must
    /// not alias any live `BootstrapPageTableBuilder`. `tables` must outlive
    /// the written builder for lifetime `'a`.
    pub unsafe fn write_new(
        slot: *mut Self,
        tables: &'a mut [PageTable],
        phys_base: u64,
    ) -> Result<(), PagingError> {
        let mut arena = PageTableArena::new(tables, phys_base)?;
        let root = arena.allocate_table()?;
        unsafe {
            slot.write(Self { arena, root });
        }
        Ok(())
    }

    /// # Safety
    ///
    /// `slot` must be valid for writes of `Self`, properly aligned, and must
    /// not alias any live `BootstrapPageTableBuilder`. `tables[0]` is treated
    /// as an already-zeroed root table and `tables` must outlive the written
    /// builder for lifetime `'a`.
    pub unsafe fn write_prezeroed(
        slot: *mut Self,
        tables: &'a mut [PageTable],
        phys_base: u64,
    ) -> Result<(), PagingError> {
        if !phys_base.is_multiple_of(PAGE_SIZE_4K) {
            return Err(PagingError::MisalignedArenaBase { phys_base });
        }
        if tables.is_empty() {
            return Err(PagingError::ArenaExhausted { capacity: 0 });
        }
        let arena = PageTableArena {
            tables: tables.as_mut_ptr(),
            capacity: tables.len(),
            phys_base,
            used_tables: 1,
            _marker: PhantomData,
        };
        let root = TableHandle {
            index: 0,
            phys: phys_base,
        };
        unsafe {
            slot.write(Self { arena, root });
        }
        Ok(())
    }

    pub fn build(
        mut self,
        plan: EarlyPagingPlan,
        options: PagingBuildOptions,
    ) -> Result<BootstrapPageTables, PagingError> {
        let mut stats = PageTableBuildStats::default();
        self.map_region(plan.identity, options, &mut stats)?;
        self.map_region(plan.kernel_image, options, &mut stats)?;
        self.map_region(plan.direct_map, options, &mut stats)?;
        stats.table_pages_used = self.arena.used();
        Ok(BootstrapPageTables {
            root_phys: self.root.phys,
            plan,
            stats,
        })
    }

    pub fn root_phys(&self) -> u64 {
        self.root.phys
    }

    pub fn table_pages_used(&self) -> usize {
        self.arena.used()
    }

    pub fn map_region(
        &mut self,
        mapping: PageMapping,
        options: PagingBuildOptions,
        stats: &mut PageTableBuildStats,
    ) -> Result<(), PagingError> {
        if mapping.len == 0
            || !mapping.vaddr.is_multiple_of(PAGE_SIZE_4K)
            || !mapping.paddr.is_multiple_of(PAGE_SIZE_4K)
            || !mapping.len.is_multiple_of(PAGE_SIZE_4K)
        {
            return Err(PagingError::MisalignedMapping {
                vaddr: mapping.vaddr,
                paddr: mapping.paddr,
                len: mapping.len,
            });
        }

        stats.mapping_regions += 1;

        let mut vaddr = mapping.vaddr;
        let mut paddr = mapping.paddr;
        let mut remaining = mapping.len;
        while remaining != 0 {
            let page_size = select_page_size(vaddr, paddr, remaining, options.allow_1gib_pages);
            self.map_page(vaddr, paddr, page_size, mapping.perms, mapping.user)?;
            match page_size {
                PageSize::Size4K => stats.leaf_4k_pages += 1,
                PageSize::Size2M => stats.leaf_2m_pages += 1,
                PageSize::Size1G => stats.leaf_1g_pages += 1,
            }
            let step = page_size.bytes();
            vaddr = vaddr
                .checked_add(step)
                .ok_or(PagingError::AddressOverflow)?;
            paddr = paddr
                .checked_add(step)
                .ok_or(PagingError::AddressOverflow)?;
            remaining -= step;
        }

        Ok(())
    }

    fn map_page(
        &mut self,
        vaddr: u64,
        paddr: u64,
        page_size: PageSize,
        perms: MemoryPermissions,
        user: bool,
    ) -> Result<(), PagingError> {
        let pml4 = self.root;
        let pdpt = self.ensure_child(pml4, pml4_index(vaddr), user)?;

        if page_size == PageSize::Size1G {
            return self.install_leaf(
                pdpt,
                pdpt_index(vaddr),
                paddr,
                page_size,
                perms,
                user,
                vaddr,
            );
        }

        let pd = self.ensure_child(pdpt, pdpt_index(vaddr), user)?;
        if page_size == PageSize::Size2M {
            return self.install_leaf(pd, pd_index(vaddr), paddr, page_size, perms, user, vaddr);
        }

        let pt = self.ensure_child(pd, pd_index(vaddr), user)?;
        self.install_leaf(pt, pt_index(vaddr), paddr, page_size, perms, user, vaddr)
    }

    fn ensure_child(
        &mut self,
        parent: TableHandle,
        entry_index: usize,
        user: bool,
    ) -> Result<TableHandle, PagingError> {
        let entry_ptr = self.arena.entry_ptr(parent, entry_index);
        let entry_value = unsafe { *entry_ptr };
        if (entry_value & ENTRY_PRESENT) != 0 {
            if (entry_value & ENTRY_PAGE_SIZE) != 0 {
                return Err(PagingError::EntryConflict {
                    vaddr: index_to_virtual(parent, entry_index),
                });
            }
            let child_phys = entry_value & ENTRY_ADDRESS_MASK;
            let child_index = self
                .arena
                .phys_to_index(child_phys)
                .ok_or(PagingError::UnsupportedForeignTable { phys: child_phys })?;
            if user {
                unsafe {
                    *entry_ptr |= ENTRY_USER;
                }
            }
            return Ok(TableHandle {
                index: child_index,
                phys: child_phys,
            });
        }

        let child = self.arena.allocate_table()?;
        unsafe {
            *entry_ptr =
                child.phys | ENTRY_PRESENT | ENTRY_WRITABLE | if user { ENTRY_USER } else { 0 };
        }
        Ok(child)
    }

    fn install_leaf(
        &mut self,
        table: TableHandle,
        entry_index: usize,
        paddr: u64,
        page_size: PageSize,
        perms: MemoryPermissions,
        user: bool,
        vaddr: u64,
    ) -> Result<(), PagingError> {
        let entry_ptr = self.arena.entry_ptr(table, entry_index);
        if unsafe { *entry_ptr } != 0 {
            return Err(PagingError::EntryConflict { vaddr });
        }

        let mut entry = (paddr & ENTRY_ADDRESS_MASK) | ENTRY_PRESENT | leaf_flags(perms, user);
        if page_size != PageSize::Size4K {
            entry |= ENTRY_PAGE_SIZE;
        }
        unsafe {
            *entry_ptr = entry;
        }
        Ok(())
    }
}

struct PageTableArena<'a> {
    tables: *mut PageTable,
    capacity: usize,
    phys_base: u64,
    used_tables: usize,
    _marker: PhantomData<&'a mut [PageTable]>,
}

impl<'a> PageTableArena<'a> {
    fn new(tables: &'a mut [PageTable], phys_base: u64) -> Result<Self, PagingError> {
        if !phys_base.is_multiple_of(PAGE_SIZE_4K) {
            return Err(PagingError::MisalignedArenaBase { phys_base });
        }
        for table in tables.iter_mut() {
            table.clear();
        }
        Ok(Self {
            tables: tables.as_mut_ptr(),
            capacity: tables.len(),
            phys_base,
            used_tables: 0,
            _marker: PhantomData,
        })
    }

    fn used(&self) -> usize {
        self.used_tables
    }

    fn allocate_table(&mut self) -> Result<TableHandle, PagingError> {
        if self.used_tables >= self.capacity {
            return Err(PagingError::ArenaExhausted {
                capacity: self.capacity,
            });
        }
        let index = self.used_tables;
        self.used_tables += 1;
        let table_ptr = self.table_ptr(index);
        unsafe {
            (*table_ptr).clear();
        }
        Ok(TableHandle {
            index,
            phys: self.phys_base + (index as u64) * PAGE_SIZE_4K,
        })
    }

    fn phys_to_index(&self, phys: u64) -> Option<usize> {
        if phys < self.phys_base {
            return None;
        }
        let delta = phys - self.phys_base;
        if !delta.is_multiple_of(PAGE_SIZE_4K) {
            return None;
        }
        let index = (delta / PAGE_SIZE_4K) as usize;
        if index < self.used_tables {
            Some(index)
        } else {
            None
        }
    }

    fn entry_ptr(&self, table: TableHandle, entry_index: usize) -> *mut u64 {
        unsafe {
            (*self.table_ptr(table.index))
                .entries
                .as_mut_ptr()
                .add(entry_index)
        }
    }

    fn table_ptr(&self, index: usize) -> *mut PageTable {
        unsafe { self.tables.add(index) }
    }
}

pub fn highest_bootstrap_physical_address(boot_info: &BootInfo<'_>) -> u64 {
    let mut highest = boot_info.kernel_phys_range.end();
    for region in boot_info.memory_regions {
        if should_direct_map_region(region.kind) {
            highest = const_max_u64(highest, region.end());
        }
    }
    for module in boot_info.modules {
        highest = const_max_u64(highest, module.physical_start.saturating_add(module.len));
    }
    if let Some(framebuffer) = boot_info.framebuffer {
        highest = const_max_u64(
            highest,
            framebuffer.physical_start.saturating_add(
                (framebuffer.pitch as u64).saturating_mul(framebuffer.height as u64),
            ),
        );
    }
    highest
}

pub fn plan_early_paging(
    layout: X86_64KernelLayout,
    boot_info: &BootInfo<'_>,
    kernel_image_len: u64,
    identity_window_len: u64,
) -> EarlyPagingPlan {
    let kernel_phys_base = align_down(boot_info.kernel_phys_range.start, PAGE_SIZE_4K);
    let kernel_phys_delta = boot_info
        .kernel_phys_range
        .start
        .saturating_sub(kernel_phys_base);
    let kernel_len = align_up(
        const_max_u64(
            kernel_image_len.saturating_add(kernel_phys_delta),
            boot_info
                .kernel_phys_range
                .len
                .saturating_add(kernel_phys_delta),
        ),
        PAGE_SIZE_4K,
    );
    let identity_len = align_up(
        const_max_u64(identity_window_len, PAGE_SIZE_2M),
        PAGE_SIZE_2M,
    );
    let direct_map_len = align_up(
        const_max_u64(highest_bootstrap_physical_address(boot_info), PAGE_SIZE_2M),
        PAGE_SIZE_2M,
    );

    EarlyPagingPlan {
        identity: PageMapping {
            vaddr: 0,
            paddr: 0,
            len: identity_len,
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: false,
        },
        kernel_image: PageMapping {
            vaddr: layout.kernel_base,
            paddr: kernel_phys_base,
            len: kernel_len,
            perms: MemoryPermissions::read_execute(),
            cache: CachePolicy::WriteBack,
            user: false,
        },
        direct_map: PageMapping {
            vaddr: layout.direct_map_base,
            paddr: 0,
            len: direct_map_len,
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: false,
        },
    }
}

const fn leaf_flags(perms: MemoryPermissions, user: bool) -> u64 {
    let mut flags = 0u64;
    if perms.write {
        flags |= ENTRY_WRITABLE;
    }
    if user {
        flags |= ENTRY_USER;
    }
    if !perms.execute {
        flags |= ENTRY_NO_EXECUTE;
    }
    flags
}

const fn should_direct_map_region(kind: BootMemoryRegionKind) -> bool {
    matches!(
        kind,
        BootMemoryRegionKind::Usable
            | BootMemoryRegionKind::AcpiReclaimable
            | BootMemoryRegionKind::AcpiNvs
            | BootMemoryRegionKind::BootloaderReclaimable
            | BootMemoryRegionKind::KernelImage
            | BootMemoryRegionKind::Framebuffer
    )
}

const fn select_page_size(
    vaddr: u64,
    paddr: u64,
    remaining: u64,
    allow_1gib_pages: bool,
) -> PageSize {
    if allow_1gib_pages
        && vaddr.is_multiple_of(PAGE_SIZE_1G)
        && paddr.is_multiple_of(PAGE_SIZE_1G)
        && remaining >= PAGE_SIZE_1G
    {
        PageSize::Size1G
    } else if vaddr.is_multiple_of(PAGE_SIZE_2M)
        && paddr.is_multiple_of(PAGE_SIZE_2M)
        && remaining >= PAGE_SIZE_2M
    {
        PageSize::Size2M
    } else {
        PageSize::Size4K
    }
}

const fn pml4_index(vaddr: u64) -> usize {
    ((vaddr >> 39) & 0x1ff) as usize
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

const fn index_to_virtual(parent: TableHandle, entry_index: usize) -> u64 {
    ((parent.index as u64) << 12) | ((entry_index as u64) << 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BootMemoryRegion, BootMemoryRegionKind, BootProtocol};

    #[test]
    fn highest_bootstrap_physical_address_skips_high_reserved_holes() {
        let regions = [
            BootMemoryRegion {
                start: 0,
                len: 0x20_0000,
                kind: BootMemoryRegionKind::Usable,
            },
            BootMemoryRegion {
                start: 0x1_0000_0000,
                len: 0x20_0000,
                kind: BootMemoryRegionKind::Reserved,
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
                start: 0x10_0000,
                len: 0x80_000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };

        assert_eq!(highest_bootstrap_physical_address(&boot_info), 0x20_0000);
    }

    #[test]
    fn builder_maps_identity_kernel_and_direct_map() {
        let plan = EarlyPagingPlan {
            identity: PageMapping {
                vaddr: 0,
                paddr: 0,
                len: PAGE_SIZE_2M,
                perms: MemoryPermissions::read_write(),
                cache: CachePolicy::WriteBack,
                user: false,
            },
            kernel_image: PageMapping {
                vaddr: 0xffff_ffff_8000_0000,
                paddr: 0x20_0000,
                len: 0x30_000,
                perms: MemoryPermissions::read_execute(),
                cache: CachePolicy::WriteBack,
                user: false,
            },
            direct_map: PageMapping {
                vaddr: 0xffff_8000_0000_0000,
                paddr: 0,
                len: 0x40_0000,
                perms: MemoryPermissions::read_write(),
                cache: CachePolicy::WriteBack,
                user: false,
            },
        };
        let mut tables = [PageTable::zeroed(); 16];
        let built = BootstrapPageTableBuilder::new(&mut tables, 0x40_0000)
            .unwrap()
            .build(plan, PagingBuildOptions::default())
            .unwrap();

        assert_eq!(built.root_phys, 0x40_0000);
        assert!(built.stats.table_pages_used >= 5);
        assert_eq!(built.stats.mapping_regions, 3);
        assert!(built.stats.leaf_2m_pages >= 3);
        assert!(built.stats.leaf_4k_pages >= 1);
    }

    #[test]
    fn builder_propagates_user_bit_through_user_mapping_hierarchy() {
        fn table_index_for(entry: u64, phys_base: u64) -> usize {
            ((entry & ENTRY_ADDRESS_MASK).saturating_sub(phys_base) / PAGE_SIZE_4K) as usize
        }

        let mapping = PageMapping {
            vaddr: 0x7fff_ffff_f000,
            paddr: 0x20_0000,
            len: 0x1000,
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: true,
        };
        let phys_base = 0x40_0000;
        let mut tables = [PageTable::zeroed(); 8];
        let mut stats = PageTableBuildStats::default();
        let mut builder = BootstrapPageTableBuilder::new(&mut tables, phys_base).unwrap();
        builder
            .map_region(mapping, PagingBuildOptions::default(), &mut stats)
            .unwrap();

        let root = tables[0].entries[pml4_index(mapping.vaddr)];
        assert_ne!(root & ENTRY_PRESENT, 0);
        assert_ne!(root & ENTRY_USER, 0);

        let pdpt = &tables[table_index_for(root, phys_base)];
        let pdpt_entry = pdpt.entries[pdpt_index(mapping.vaddr)];
        assert_ne!(pdpt_entry & ENTRY_PRESENT, 0);
        assert_ne!(pdpt_entry & ENTRY_USER, 0);

        let pd = &tables[table_index_for(pdpt_entry, phys_base)];
        let pd_entry = pd.entries[pd_index(mapping.vaddr)];
        assert_ne!(pd_entry & ENTRY_PRESENT, 0);
        assert_ne!(pd_entry & ENTRY_USER, 0);

        let pt = &tables[table_index_for(pd_entry, phys_base)];
        let leaf = pt.entries[pt_index(mapping.vaddr)];
        assert_ne!(leaf & ENTRY_PRESENT, 0);
        assert_ne!(leaf & ENTRY_USER, 0);
    }
}
