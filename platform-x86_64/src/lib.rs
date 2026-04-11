#![no_std]

//! Canonical subsystem role:
//! - subsystem: x86_64 platform mediation
//! - owner layer: Layer 0
//! - semantic owner: `platform-x86_64`
//! - truth path role: provides hardware-facing mechanisms to the real system path
//!
//! Canonical contract families produced here:
//! - paging/address-space mechanism contracts
//! - interrupt/APIC mechanism contracts
//! - device transport and discovery contracts
//!
//! This crate owns platform mechanics, not the semantic truth of kernel object
//! models. `kernel-core` remains the owner of higher-level subsystem meaning.

extern crate alloc;

use alloc::vec::Vec;

pub mod ac97_audio;
pub mod acpi;
pub mod device_platform;
pub mod limine;
pub mod nvidia_gpu;
pub mod paging;
pub mod phys_alloc;
pub mod traps;
pub mod user_mode;
pub mod virtio_blk;
pub mod virtio_gpu;
pub mod virtio_net;

pub use ac97_audio::{
    Ac97AudioController, Ac97AudioDriver, Ac97Error, Ac97FormatInfo,
};

pub use acpi::{
    AcpiProbeInfo, AcpiRootInfo, ApicTopologyInfo, InterruptSourceOverride, IoApicEntry,
    ProcessorTopologyEntry, acpi_probe_info, acpi_probe_signatures, acpi_root_info, apic_topology,
};
#[cfg(target_arch = "x86_64")]
pub use device_platform::PciLegacyPortBackend;
pub use device_platform::{
    DmaWindow, PciConfigBackend, SyntheticPciConfigBackend, X86_64DevicePlatform,
    X86_64DevicePlatformConfig, X86_64InterruptEvent,
};
pub use limine::{LimineBootBuffers, LimineBootSnapshot, LimineHandoffError};
pub use paging::{
    BootstrapPageTableBuilder, BootstrapPageTables, EarlyPagingPlan, PageTable,
    PageTableBuildStats, PagingBuildOptions, PagingError, highest_bootstrap_physical_address,
    plan_early_paging,
};
pub use phys_alloc::{
    EarlyFrameAllocator, EarlyFrameAllocatorStats, FrameAllocatorError, PhysicalFrameRun,
};
use platform_hal::{
    AddressSpaceId, AddressSpaceLayout, AddressSpaceManager, Architecture, CachePolicy, HalError,
    MemoryPermissions, PageMapping, Platform, PlatformDescriptor, VirtualRange,
};
pub use traps::{
    EXCEPTION_VECTOR_COUNT, ExceptionFrame, ExceptionVector, IDT_ENTRY_COUNT, IdtEntry,
    IdtGateKind, IdtGateOptions, IdtPointer, InterruptDescriptorTable,
};
pub use virtio_blk::{
    VirtioBlkDriver, VirtioBlkError, VirtioBlkInterruptSummary, VirtioBlkPciMatch,
};
pub use virtio_gpu::{
    VirtioGpuCompletedRequest, VirtioGpuDriver, VirtioGpuError, VirtioGpuInterruptSummary,
    VirtioGpuPciMatch, VirtioGpuRequestKind,
};
pub use virtio_net::{
    VirtioNetDriver, VirtioNetError, VirtioNetFrame, VirtioNetInterruptSummary, VirtioNetPciMatch,
};

pub const PAGE_SIZE_4K: u64 = 4 * 1024;
pub const PAGE_SIZE_2M: u64 = 2 * 1024 * 1024;
pub const PAGE_SIZE_1G: u64 = 1024 * 1024 * 1024;
pub const LOADER_DEFINED_HANDOFF_MAGIC: u64 = 0x4e47_4f53_5848_3634;

pub const DEFAULT_KERNEL_BASE: u64 = 0xffff_ffff_8000_0000;
pub const DEFAULT_DIRECT_MAP_BASE: u64 = 0xffff_8000_0000_0000;
pub const DEFAULT_BOOT_STACK_BASE: u64 = 0xffff_ffff_8040_0000;
pub const DEFAULT_BOOT_STACK_SIZE: u64 = 512 * 1024;
pub const DEFAULT_DIRECT_MAP_SIZE: u64 = 512 * PAGE_SIZE_1G;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootProtocol {
    Limine,
    Multiboot2,
    Uefi,
    LoaderDefined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMemoryRegionKind {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    Mmio,
    BadMemory,
    BootloaderReclaimable,
    KernelImage,
    Framebuffer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootMemoryRegion {
    pub start: u64,
    pub len: u64,
    pub kind: BootMemoryRegionKind,
}

impl BootMemoryRegion {
    pub const fn end(self) -> u64 {
        self.start.saturating_add(self.len)
    }

    pub const fn is_page_aligned(self) -> bool {
        self.start.is_multiple_of(PAGE_SIZE_4K) && self.len.is_multiple_of(PAGE_SIZE_4K)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootModule<'a> {
    pub name: &'a str,
    pub physical_start: u64,
    pub len: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferInfo {
    pub physical_start: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u16,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootInfo<'a> {
    pub protocol: BootProtocol,
    pub command_line: Option<&'a str>,
    pub rsdp: Option<u64>,
    pub memory_regions: &'a [BootMemoryRegion],
    pub modules: &'a [BootModule<'a>],
    pub framebuffer: Option<FramebufferInfo>,
    pub physical_memory_offset: u64,
    pub kernel_phys_range: BootMemoryRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootInfoValidationError {
    UnalignedPhysicalMemoryOffset,
    KernelRangeMustBeKernelImage,
    KernelRangeMustBePageAligned,
    KernelRangeMustBeNonEmpty,
    MemoryRegionMustBePageAligned,
    MemoryRegionMustBeNonEmpty,
    MemoryRegionsOverlap,
}

impl<'a> BootInfo<'a> {
    pub fn validate(&self) -> Result<(), BootInfoValidationError> {
        if !self.physical_memory_offset.is_multiple_of(PAGE_SIZE_4K) {
            return Err(BootInfoValidationError::UnalignedPhysicalMemoryOffset);
        }
        if self.kernel_phys_range.kind != BootMemoryRegionKind::KernelImage {
            return Err(BootInfoValidationError::KernelRangeMustBeKernelImage);
        }
        if self.kernel_phys_range.len == 0 {
            return Err(BootInfoValidationError::KernelRangeMustBeNonEmpty);
        }
        if !self.kernel_phys_range.is_page_aligned() {
            return Err(BootInfoValidationError::KernelRangeMustBePageAligned);
        }

        let mut previous_end = 0u64;
        let mut first = true;
        for region in self.memory_regions {
            if region.len == 0 {
                return Err(BootInfoValidationError::MemoryRegionMustBeNonEmpty);
            }
            if !region.is_page_aligned() {
                return Err(BootInfoValidationError::MemoryRegionMustBePageAligned);
            }
            if !first && region.start < previous_end {
                return Err(BootInfoValidationError::MemoryRegionsOverlap);
            }
            previous_end = region.end();
            first = false;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderDefinedHandoffError {
    InvalidMagic,
    InvalidBootInfo(BootInfoValidationError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct LoaderDefinedBootHandoff<'a> {
    pub magic: u64,
    pub protocol: BootProtocol,
    pub command_line: Option<&'a str>,
    pub rsdp: Option<u64>,
    pub memory_regions: &'a [BootMemoryRegion],
    pub modules: &'a [BootModule<'a>],
    pub framebuffer: Option<FramebufferInfo>,
    pub physical_memory_offset: u64,
    pub kernel_phys_range: BootMemoryRegion,
}

impl<'a> LoaderDefinedBootHandoff<'a> {
    pub const fn new(
        command_line: Option<&'a str>,
        rsdp: Option<u64>,
        memory_regions: &'a [BootMemoryRegion],
        modules: &'a [BootModule<'a>],
        framebuffer: Option<FramebufferInfo>,
        physical_memory_offset: u64,
        kernel_phys_range: BootMemoryRegion,
    ) -> Self {
        Self {
            magic: LOADER_DEFINED_HANDOFF_MAGIC,
            protocol: BootProtocol::LoaderDefined,
            command_line,
            rsdp,
            memory_regions,
            modules,
            framebuffer,
            physical_memory_offset,
            kernel_phys_range,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn from_protocol(
        protocol: BootProtocol,
        command_line: Option<&'a str>,
        rsdp: Option<u64>,
        memory_regions: &'a [BootMemoryRegion],
        modules: &'a [BootModule<'a>],
        framebuffer: Option<FramebufferInfo>,
        physical_memory_offset: u64,
        kernel_phys_range: BootMemoryRegion,
    ) -> Self {
        Self {
            magic: LOADER_DEFINED_HANDOFF_MAGIC,
            protocol,
            command_line,
            rsdp,
            memory_regions,
            modules,
            framebuffer,
            physical_memory_offset,
            kernel_phys_range,
        }
    }

    pub fn validate(&self) -> Result<(), LoaderDefinedHandoffError> {
        if self.magic != LOADER_DEFINED_HANDOFF_MAGIC {
            Err(LoaderDefinedHandoffError::InvalidMagic)
        } else {
            self.as_boot_info().map(|_| ())
        }
    }

    pub fn as_boot_info(&self) -> Result<BootInfo<'a>, LoaderDefinedHandoffError> {
        if self.magic != LOADER_DEFINED_HANDOFF_MAGIC {
            Err(LoaderDefinedHandoffError::InvalidMagic)
        } else {
            let boot_info = BootInfo {
                protocol: self.protocol,
                command_line: self.command_line,
                rsdp: self.rsdp,
                memory_regions: self.memory_regions,
                modules: self.modules,
                framebuffer: self.framebuffer,
                physical_memory_offset: self.physical_memory_offset,
                kernel_phys_range: self.kernel_phys_range,
            };
            boot_info
                .validate()
                .map_err(LoaderDefinedHandoffError::InvalidBootInfo)?;
            Ok(boot_info)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86_64KernelLayout {
    pub kernel_base: u64,
    pub direct_map_base: u64,
    pub direct_map_size: u64,
    pub boot_stack_base: u64,
    pub boot_stack_size: u64,
}

impl X86_64KernelLayout {
    pub const fn new(
        kernel_base: u64,
        direct_map_base: u64,
        direct_map_size: u64,
        boot_stack_base: u64,
        boot_stack_size: u64,
    ) -> Self {
        Self {
            kernel_base,
            direct_map_base,
            direct_map_size,
            boot_stack_base,
            boot_stack_size,
        }
    }

    pub const fn higher_half_default() -> Self {
        Self::new(
            DEFAULT_KERNEL_BASE,
            DEFAULT_DIRECT_MAP_BASE,
            DEFAULT_DIRECT_MAP_SIZE,
            DEFAULT_BOOT_STACK_BASE,
            DEFAULT_BOOT_STACK_SIZE,
        )
    }

    pub const fn kernel_window(self, image_len: u64) -> VirtualRange {
        VirtualRange {
            vaddr: self.kernel_base,
            len: align_up(image_len, PAGE_SIZE_4K),
        }
    }

    pub const fn direct_map_window(self) -> VirtualRange {
        VirtualRange {
            vaddr: self.direct_map_base,
            len: self.direct_map_size,
        }
    }

    pub const fn boot_stack_window(self) -> VirtualRange {
        VirtualRange {
            vaddr: self.boot_stack_base,
            len: self.boot_stack_size,
        }
    }

    pub const fn boot_stack_top(self) -> u64 {
        self.boot_stack_base.saturating_add(self.boot_stack_size)
    }

    pub const fn is_canonical(self, address: u64) -> bool {
        let sign = (address >> 47) & 1;
        if sign == 0 {
            address <= 0x0000_7fff_ffff_ffff
        } else {
            address >= 0xffff_8000_0000_0000
        }
    }
}

impl Default for X86_64KernelLayout {
    fn default() -> Self {
        Self::higher_half_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapMappingPlan {
    pub identity: PageMapping,
    pub kernel_image: PageMapping,
    pub direct_map: PageMapping,
    pub boot_stack: PageMapping,
}

impl BootstrapMappingPlan {
    pub const fn minimum_page_table_span(self) -> u64 {
        self.identity
            .len
            .saturating_add(self.kernel_image.len)
            .saturating_add(self.direct_map.len)
            .saturating_add(self.boot_stack.len)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86_64BootRequirements {
    pub minimum_loader_alignment: u64,
    pub page_table_granularity: u64,
    pub stack_alignment: u64,
    pub nx_enabled: bool,
    pub write_protect_enabled: bool,
}

impl X86_64BootRequirements {
    pub const fn baseline() -> Self {
        Self {
            minimum_loader_alignment: PAGE_SIZE_4K,
            page_table_granularity: PAGE_SIZE_4K,
            stack_alignment: 16,
            nx_enabled: true,
            write_protect_enabled: true,
        }
    }
}

pub const fn align_up(value: u64, align: u64) -> u64 {
    if align == 0 {
        value
    } else {
        let rem = value % align;
        if rem == 0 {
            value
        } else {
            value.saturating_add(align - rem)
        }
    }
}

pub const fn align_down(value: u64, align: u64) -> u64 {
    if align == 0 {
        value
    } else {
        value - (value % align)
    }
}

pub const fn const_max_u64(lhs: u64, rhs: u64) -> u64 {
    if lhs >= rhs { lhs } else { rhs }
}

pub const fn plan_bootstrap_mappings(
    layout: X86_64KernelLayout,
    kernel_phys_base: u64,
    kernel_image_len: u64,
    identity_window_len: u64,
    direct_map_phys_len: u64,
) -> BootstrapMappingPlan {
    let kernel_len = align_up(kernel_image_len, PAGE_SIZE_4K);
    let identity_len = align_up(
        const_max_u64(identity_window_len, PAGE_SIZE_2M),
        PAGE_SIZE_2M,
    );
    let direct_map_len = align_up(
        const_max_u64(direct_map_phys_len, PAGE_SIZE_2M),
        PAGE_SIZE_2M,
    );
    let boot_stack_len = align_up(layout.boot_stack_size, PAGE_SIZE_4K);
    let kernel_phys_aligned = align_down(kernel_phys_base, PAGE_SIZE_4K);

    BootstrapMappingPlan {
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
            paddr: kernel_phys_aligned,
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
        boot_stack: PageMapping {
            vaddr: layout.boot_stack_base,
            paddr: kernel_phys_aligned.saturating_add(kernel_len),
            len: boot_stack_len,
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: false,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X86_64Platform {
    descriptor: PlatformDescriptor,
    layout: X86_64KernelLayout,
    requirements: X86_64BootRequirements,
    next_address_space_id: u64,
    active_address_space: Option<AddressSpaceId>,
    address_spaces: Vec<X86_64AddressSpace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct X86_64AddressSpace {
    id: AddressSpaceId,
    mappings: Vec<PageMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterializedAddressSpace {
    pub id: AddressSpaceId,
    pub root_phys: u64,
    pub stats: PageTableBuildStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceMaterializationError {
    InvalidAddressSpace(AddressSpaceId),
    Paging(PagingError),
}

struct AddressSpacePageTableMaterializeAgent;

impl AddressSpacePageTableMaterializeAgent {
    fn execute(
        space: &X86_64AddressSpace,
        id: AddressSpaceId,
        tables: &mut [PageTable],
        phys_base: u64,
    ) -> Result<MaterializedAddressSpace, AddressSpaceMaterializationError> {
        let mut builder = BootstrapPageTableBuilder::new(tables, phys_base)
            .map_err(AddressSpaceMaterializationError::Paging)?;
        let mut stats = PageTableBuildStats::default();
        for mapping in &space.mappings {
            builder
                .map_region(*mapping, PagingBuildOptions::default(), &mut stats)
                .map_err(AddressSpaceMaterializationError::Paging)?;
        }
        stats.table_pages_used = builder.table_pages_used();
        Ok(MaterializedAddressSpace {
            id,
            root_phys: builder.root_phys(),
            stats,
        })
    }
}

impl X86_64Platform {
    pub fn new(layout: X86_64KernelLayout) -> Self {
        Self {
            descriptor: PlatformDescriptor {
                name: "ngos-x86_64",
                architecture: Architecture::X86_64,
                host_runtime_mode: false,
            },
            layout,
            requirements: X86_64BootRequirements::baseline(),
            next_address_space_id: 1,
            active_address_space: None,
            address_spaces: Vec::new(),
        }
    }

    pub const fn boot_layout(&self) -> X86_64KernelLayout {
        self.layout
    }

    pub const fn boot_requirements(&self) -> X86_64BootRequirements {
        self.requirements
    }

    pub const fn bootstrap_mappings(
        &self,
        kernel_phys_base: u64,
        kernel_image_len: u64,
        identity_window_len: u64,
        direct_map_phys_len: u64,
    ) -> BootstrapMappingPlan {
        plan_bootstrap_mappings(
            self.layout,
            kernel_phys_base,
            kernel_image_len,
            identity_window_len,
            direct_map_phys_len,
        )
    }

    pub fn materialize_address_space(
        &self,
        id: AddressSpaceId,
        tables: &mut [PageTable],
        phys_base: u64,
    ) -> Result<MaterializedAddressSpace, AddressSpaceMaterializationError> {
        let space = self
            .space(id)
            .map_err(|_| AddressSpaceMaterializationError::InvalidAddressSpace(id))?;
        AddressSpacePageTableMaterializeAgent::execute(space, id, tables, phys_base)
    }
}

impl Default for X86_64Platform {
    fn default() -> Self {
        Self::new(X86_64KernelLayout::higher_half_default())
    }
}

impl Platform for X86_64Platform {
    fn name(&self) -> &'static str {
        self.descriptor.name
    }

    fn architecture(&self) -> Architecture {
        self.descriptor.architecture
    }

    fn supports_host_runtime_mode(&self) -> bool {
        self.descriptor.host_runtime_mode
    }
}

impl X86_64Platform {
    fn validate_mapping(mapping: PageMapping) -> Result<(), HalError> {
        if mapping.len == 0
            || !mapping.vaddr.is_multiple_of(PAGE_SIZE_4K)
            || !mapping.paddr.is_multiple_of(PAGE_SIZE_4K)
            || !mapping.len.is_multiple_of(PAGE_SIZE_4K)
        {
            return Err(HalError::InvalidMapping);
        }
        Ok(())
    }

    fn mapping_range(mapping: PageMapping) -> VirtualRange {
        VirtualRange {
            vaddr: mapping.vaddr,
            len: mapping.len,
        }
    }

    fn ranges_overlap(left: VirtualRange, right: VirtualRange) -> bool {
        let left_end = left.vaddr.saturating_add(left.len);
        let right_end = right.vaddr.saturating_add(right.len);
        left.vaddr < right_end && right.vaddr < left_end
    }

    fn space(&self, id: AddressSpaceId) -> Result<&X86_64AddressSpace, HalError> {
        self.address_spaces
            .iter()
            .find(|space| space.id == id)
            .ok_or(HalError::InvalidAddressSpace)
    }

    fn space_mut(&mut self, id: AddressSpaceId) -> Result<&mut X86_64AddressSpace, HalError> {
        self.address_spaces
            .iter_mut()
            .find(|space| space.id == id)
            .ok_or(HalError::InvalidAddressSpace)
    }
}

impl AddressSpaceManager for X86_64Platform {
    fn create_address_space(&mut self) -> Result<AddressSpaceId, HalError> {
        let id = AddressSpaceId::new(self.next_address_space_id);
        self.next_address_space_id = self
            .next_address_space_id
            .checked_add(1)
            .ok_or(HalError::Exhausted)?;
        self.address_spaces.push(X86_64AddressSpace {
            id,
            mappings: Vec::new(),
        });
        Ok(id)
    }

    fn destroy_address_space(&mut self, id: AddressSpaceId) -> Result<(), HalError> {
        let before = self.address_spaces.len();
        self.address_spaces.retain(|space| space.id != id);
        if self.address_spaces.len() == before {
            return Err(HalError::InvalidAddressSpace);
        }
        if self.active_address_space == Some(id) {
            self.active_address_space = None;
        }
        Ok(())
    }

    fn map(&mut self, id: AddressSpaceId, mapping: PageMapping) -> Result<(), HalError> {
        Self::validate_mapping(mapping)?;
        let range = Self::mapping_range(mapping);
        let space = self.space_mut(id)?;
        if space
            .mappings
            .iter()
            .any(|existing| Self::ranges_overlap(Self::mapping_range(*existing), range))
        {
            return Err(HalError::OverlappingMapping);
        }
        space.mappings.push(mapping);
        space.mappings.sort_by_key(|entry| entry.vaddr);
        Ok(())
    }

    fn unmap(&mut self, id: AddressSpaceId, range: VirtualRange) -> Result<(), HalError> {
        if range.len == 0 {
            return Err(HalError::InvalidMapping);
        }
        let space = self.space_mut(id)?;
        let before = space.mappings.len();
        space
            .mappings
            .retain(|mapping| Self::mapping_range(*mapping) != range);
        if space.mappings.len() == before {
            return Err(HalError::MappingNotFound);
        }
        Ok(())
    }

    fn protect(
        &mut self,
        id: AddressSpaceId,
        range: VirtualRange,
        perms: MemoryPermissions,
    ) -> Result<(), HalError> {
        if range.len == 0 {
            return Err(HalError::InvalidMapping);
        }
        let space = self.space_mut(id)?;
        let Some(mapping) = space
            .mappings
            .iter_mut()
            .find(|mapping| Self::mapping_range(**mapping) == range)
        else {
            return Err(HalError::MappingNotFound);
        };
        mapping.perms = perms;
        Ok(())
    }

    fn activate_address_space(&mut self, id: AddressSpaceId) -> Result<(), HalError> {
        self.space(id)?;
        self.active_address_space = Some(id);
        Ok(())
    }

    fn active_address_space(&self) -> Option<AddressSpaceId> {
        self.active_address_space
    }

    fn address_space_layout(&self, id: AddressSpaceId) -> Result<AddressSpaceLayout, HalError> {
        let space = self.space(id)?;
        Ok(AddressSpaceLayout {
            id,
            active: self.active_address_space == Some(id),
            mappings: space.mappings.clone(),
        })
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use platform_hal::AddressSpaceManager;

    #[test]
    fn higher_half_layout_uses_canonical_addresses() {
        let layout = X86_64KernelLayout::higher_half_default();
        assert!(layout.is_canonical(layout.kernel_base));
        assert!(layout.is_canonical(layout.direct_map_base));
        assert!(layout.is_canonical(layout.boot_stack_base));
        assert!(!layout.is_canonical(0x0000_8000_0000_0000));
    }

    #[test]
    fn bootstrap_plan_aligns_kernel_and_boot_windows() {
        let layout = X86_64KernelLayout::higher_half_default();
        let plan = plan_bootstrap_mappings(layout, 0x20_1234, 0x34567, 0x1000, 0x3000);
        assert_eq!(plan.identity.len, PAGE_SIZE_2M);
        assert_eq!(plan.kernel_image.paddr, 0x20_1000);
        assert_eq!(plan.kernel_image.len % PAGE_SIZE_4K, 0);
        assert_eq!(plan.boot_stack.len % PAGE_SIZE_4K, 0);
        assert_eq!(plan.direct_map.len, PAGE_SIZE_2M);
    }

    #[test]
    fn platform_descriptor_reports_non_hosted_x86_64() {
        let platform = X86_64Platform::default();
        assert_eq!(platform.architecture(), Architecture::X86_64);
        assert!(!platform.supports_host_runtime_mode());
        assert_eq!(platform.name(), "ngos-x86_64");
    }

    #[test]
    fn platform_tracks_address_space_layouts() {
        let mut platform = X86_64Platform::default();
        let first = platform.create_address_space().unwrap();
        let second = platform.create_address_space().unwrap();

        platform
            .map(
                first,
                PageMapping {
                    vaddr: 0x4000,
                    paddr: 0x8000,
                    len: 0x2000,
                    perms: MemoryPermissions::read_write(),
                    cache: CachePolicy::WriteBack,
                    user: true,
                },
            )
            .unwrap();
        platform.activate_address_space(first).unwrap();

        let first_layout = platform.address_space_layout(first).unwrap();
        let second_layout = platform.address_space_layout(second).unwrap();

        assert!(first_layout.active);
        assert_eq!(first_layout.mappings.len(), 1);
        assert!(!second_layout.active);
        assert!(second_layout.mappings.is_empty());
        assert_eq!(platform.active_address_space(), Some(first));
    }

    #[test]
    fn platform_rejects_overlapping_or_missing_mappings() {
        let mut platform = X86_64Platform::default();
        let id = platform.create_address_space().unwrap();

        let mapping = PageMapping {
            vaddr: 0x8000,
            paddr: 0x20_000,
            len: 0x1000,
            perms: MemoryPermissions::read_only(),
            cache: CachePolicy::WriteBack,
            user: true,
        };

        platform.map(id, mapping).unwrap();
        assert_eq!(platform.map(id, mapping), Err(HalError::OverlappingMapping));
        assert_eq!(
            platform.unmap(
                id,
                VirtualRange {
                    vaddr: 0x9000,
                    len: 0x1000,
                }
            ),
            Err(HalError::MappingNotFound)
        );
        assert_eq!(
            platform.protect(
                id,
                VirtualRange {
                    vaddr: 0x9000,
                    len: 0x1000,
                },
                MemoryPermissions::read_write(),
            ),
            Err(HalError::MappingNotFound)
        );
    }

    #[test]
    fn platform_can_protect_unmap_and_destroy_address_spaces() {
        let mut platform = X86_64Platform::default();
        let id = platform.create_address_space().unwrap();
        let range = VirtualRange {
            vaddr: 0x20_000,
            len: 0x2000,
        };

        platform
            .map(
                id,
                PageMapping {
                    vaddr: range.vaddr,
                    paddr: 0x40_000,
                    len: range.len,
                    perms: MemoryPermissions::read_only(),
                    cache: CachePolicy::WriteBack,
                    user: false,
                },
            )
            .unwrap();
        platform
            .protect(id, range, MemoryPermissions::read_write())
            .unwrap();
        let layout = platform.address_space_layout(id).unwrap();
        assert_eq!(layout.mappings[0].perms, MemoryPermissions::read_write());

        platform.unmap(id, range).unwrap();
        assert!(
            platform
                .address_space_layout(id)
                .unwrap()
                .mappings
                .is_empty()
        );
        platform.activate_address_space(id).unwrap();
        platform.destroy_address_space(id).unwrap();
        assert_eq!(platform.active_address_space(), None);
        assert_eq!(
            platform.address_space_layout(id),
            Err(HalError::InvalidAddressSpace)
        );
    }

    #[test]
    fn platform_materializes_address_space_layout_into_page_tables() {
        let mut platform = X86_64Platform::default();
        let id = platform.create_address_space().unwrap();
        platform
            .map(
                id,
                PageMapping {
                    vaddr: 0x400000,
                    paddr: 0x20_0000,
                    len: 0x3000,
                    perms: MemoryPermissions::read_execute(),
                    cache: CachePolicy::WriteBack,
                    user: true,
                },
            )
            .unwrap();
        platform
            .map(
                id,
                PageMapping {
                    vaddr: 0x7fff_ffff_f000,
                    paddr: 0x40_0000,
                    len: 0x2000,
                    perms: MemoryPermissions::read_write(),
                    cache: CachePolicy::WriteBack,
                    user: true,
                },
            )
            .unwrap();

        let mut tables = [PageTable::zeroed(); 16];
        let materialized = platform
            .materialize_address_space(id, &mut tables, 0x80_0000)
            .unwrap();

        assert_eq!(materialized.id, id);
        assert_eq!(materialized.root_phys, 0x80_0000);
        assert_eq!(materialized.stats.mapping_regions, 2);
        assert!(materialized.stats.table_pages_used >= 4);
        assert!(materialized.stats.leaf_4k_pages >= 5);
    }

    #[test]
    fn platform_materialization_rejects_unknown_address_space() {
        let platform = X86_64Platform::default();
        let mut tables = [PageTable::zeroed(); 4];
        assert_eq!(
            platform.materialize_address_space(AddressSpaceId::new(99), &mut tables, 0x80_0000),
            Err(AddressSpaceMaterializationError::InvalidAddressSpace(
                AddressSpaceId::new(99)
            ))
        );
    }

    #[test]
    fn loader_defined_handoff_converts_to_boot_info() {
        let memory = [BootMemoryRegion {
            start: 0,
            len: PAGE_SIZE_2M,
            kind: BootMemoryRegionKind::Usable,
        }];
        let modules = [BootModule {
            name: "initrd",
            physical_start: 0x40_0000,
            len: 0x20_000,
        }];
        let handoff = LoaderDefinedBootHandoff::new(
            Some("console=ttyS0"),
            Some(0xdead_beef),
            &memory,
            &modules,
            None,
            DEFAULT_DIRECT_MAP_BASE,
            BootMemoryRegion {
                start: 0x20_0000,
                len: 0x18_000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        );

        let boot_info = handoff.as_boot_info().unwrap();
        assert_eq!(boot_info.protocol, BootProtocol::LoaderDefined);
        assert_eq!(boot_info.command_line, Some("console=ttyS0"));
        assert_eq!(boot_info.memory_regions.len(), 1);
        assert_eq!(boot_info.modules[0].name, "initrd");
    }

    #[test]
    fn handoff_preserves_protocol_when_requested() {
        let memory = [BootMemoryRegion {
            start: 0,
            len: PAGE_SIZE_4K,
            kind: BootMemoryRegionKind::Usable,
        }];
        let handoff = LoaderDefinedBootHandoff::from_protocol(
            BootProtocol::Limine,
            None,
            None,
            &memory,
            &[],
            None,
            DEFAULT_DIRECT_MAP_BASE,
            BootMemoryRegion {
                start: 0x10_0000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::KernelImage,
            },
        );

        let boot_info = handoff.as_boot_info().unwrap();
        assert_eq!(boot_info.protocol, BootProtocol::Limine);
    }

    #[test]
    fn boot_info_validation_rejects_unaligned_or_empty_kernel_ranges() {
        let memory = [BootMemoryRegion {
            start: 0,
            len: PAGE_SIZE_2M,
            kind: BootMemoryRegionKind::Usable,
        }];
        let boot_info = BootInfo {
            protocol: BootProtocol::LoaderDefined,
            command_line: None,
            rsdp: None,
            memory_regions: &memory,
            modules: &[],
            framebuffer: None,
            physical_memory_offset: DEFAULT_DIRECT_MAP_BASE,
            kernel_phys_range: BootMemoryRegion {
                start: 0x10_0001,
                len: 0,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };

        assert_eq!(
            boot_info.validate(),
            Err(BootInfoValidationError::KernelRangeMustBeNonEmpty)
        );

        let misaligned = BootInfo {
            kernel_phys_range: BootMemoryRegion {
                start: 0x10_0001,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::KernelImage,
            },
            ..boot_info
        };
        assert_eq!(
            misaligned.validate(),
            Err(BootInfoValidationError::KernelRangeMustBePageAligned)
        );
    }

    #[test]
    fn handoff_rejects_overlapping_or_misaligned_memory_regions() {
        let overlapping = [
            BootMemoryRegion {
                start: 0,
                len: PAGE_SIZE_2M,
                kind: BootMemoryRegionKind::Usable,
            },
            BootMemoryRegion {
                start: PAGE_SIZE_4K,
                len: PAGE_SIZE_2M,
                kind: BootMemoryRegionKind::Reserved,
            },
        ];
        let handoff = LoaderDefinedBootHandoff::new(
            None,
            None,
            &overlapping,
            &[],
            None,
            DEFAULT_DIRECT_MAP_BASE,
            BootMemoryRegion {
                start: 0x20_0000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::KernelImage,
            },
        );
        assert_eq!(
            handoff.as_boot_info(),
            Err(LoaderDefinedHandoffError::InvalidBootInfo(
                BootInfoValidationError::MemoryRegionsOverlap
            ))
        );

        let misaligned = [BootMemoryRegion {
            start: 3,
            len: PAGE_SIZE_4K,
            kind: BootMemoryRegionKind::Usable,
        }];
        let handoff = LoaderDefinedBootHandoff::new(
            None,
            None,
            &misaligned,
            &[],
            None,
            DEFAULT_DIRECT_MAP_BASE,
            BootMemoryRegion {
                start: 0x20_0000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::KernelImage,
            },
        );
        assert_eq!(
            handoff.validate(),
            Err(LoaderDefinedHandoffError::InvalidBootInfo(
                BootInfoValidationError::MemoryRegionMustBePageAligned
            ))
        );
    }

    #[test]
    fn handoff_rejects_non_kernel_image_or_unaligned_physical_offset() {
        let memory = [BootMemoryRegion {
            start: 0,
            len: PAGE_SIZE_2M,
            kind: BootMemoryRegionKind::Usable,
        }];
        let handoff = LoaderDefinedBootHandoff::new(
            None,
            None,
            &memory,
            &[],
            None,
            DEFAULT_DIRECT_MAP_BASE + 1,
            BootMemoryRegion {
                start: 0x20_0000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::Reserved,
            },
        );
        assert_eq!(
            handoff.as_boot_info(),
            Err(LoaderDefinedHandoffError::InvalidBootInfo(
                BootInfoValidationError::UnalignedPhysicalMemoryOffset
            ))
        );

        let handoff = LoaderDefinedBootHandoff::new(
            None,
            None,
            &memory,
            &[],
            None,
            DEFAULT_DIRECT_MAP_BASE,
            BootMemoryRegion {
                start: 0x20_0000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::Reserved,
            },
        );
        assert_eq!(
            handoff.as_boot_info(),
            Err(LoaderDefinedHandoffError::InvalidBootInfo(
                BootInfoValidationError::KernelRangeMustBeKernelImage
            ))
        );
    }
}
