use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
#[cfg(target_arch = "x86_64")]
use core::arch::asm;

use platform_hal::{
    Architecture, BarFlags, BarId, BarInfo, BarKind, BusAddress, BusKind, ConfigAccess,
    ConfigSpaceKind, ConfigWidth, DeviceIdentity, DeviceLocator, DevicePlatform, DeviceRecord,
    DmaBufferId, DmaBufferInfo, DmaCoherency, DmaConstraints, DmaDirection, DmaOwnership,
    GpuMemoryKind, GpuVendor, HalError, InterruptCapability, InterruptHandle, InterruptKind,
    InterruptPolarity, InterruptRoute, InterruptTrigger, MmioCachePolicy, MmioMapping,
    MmioMappingId, MmioPermissions, MmioRegionId, Platform, PlatformDescriptor,
};

use crate::{
    DEFAULT_DIRECT_MAP_BASE, DEFAULT_DIRECT_MAP_SIZE, PAGE_SIZE_4K, X86_64BootRequirements,
    X86_64KernelLayout,
};

use crate::nvidia_gpu::{
    NvidiaDisplayAgent, NvidiaGpu, NvidiaGspAgent, NvidiaInterruptAgent, NvidiaMediaAgent,
    NvidiaNeuralAgent, NvidiaPowerAgent, NvidiaTensorAgent, NvidiaVramAgent,
    RTX_5060_TI_LOCAL_GSP_BLACKWELL_BLOB_PRESENT, RTX_5060_TI_LOCAL_GSP_BLOB_SUMMARY,
    RTX_5060_TI_LOCAL_GSP_DRIVER_MODEL_WDDM, RTX_5060_TI_LOCAL_GSP_FIRMWARE_VERSION,
    RTX_5060_TI_LOCAL_GSP_REAL_HARDWARE_READY, RTX_5060_TI_LOCAL_GSP_REFUSAL_REASON,
    RTX_5060_TI_LOCAL_HARDWARE_INTERRUPT_SERVICING_CONFIRMED,
    RTX_5060_TI_LOCAL_HARDWARE_POWER_MANAGEMENT_CONFIRMED,
    RTX_5060_TI_LOCAL_INTERRUPT_MESSAGE_MAXIMUM, as_platform_gpu_binding_evidence,
    as_platform_gpu_vbios_image_evidence, as_platform_gpu_vbios_window_evidence,
    inspect_vbios_image_bytes, local_windows_binding_for_device,
};
use crate::virtio_gpu::{VirtioGpuDriver, VirtioGpuError};

const PCI_VENDOR_DEVICE_OFFSET: u16 = 0x00;
const PCI_COMMAND_STATUS_OFFSET: u16 = 0x04;
const PCI_CLASS_REVISION_OFFSET: u16 = 0x08;
const PCI_HEADER_TYPE_OFFSET: u16 = 0x0c;
const PCI_BAR0_OFFSET: u16 = 0x10;
const GPU_PRESENT_OPCODE: u32 = 0x4750_0001;
const PCI_CAP_PTR_OFFSET: u16 = 0x34;
const PCI_INTERRUPT_LINE_OFFSET: u16 = 0x3c;

const PCI_COMMAND_IO_SPACE: u16 = 1 << 0;
const PCI_COMMAND_MEMORY_SPACE: u16 = 1 << 1;
const PCI_STATUS_CAPABILITIES: u16 = 1 << 4;

const PCI_CAP_ID_MSI: u8 = 0x05;
const PCI_CAP_ID_MSIX: u8 = 0x11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaWindow {
    pub physical_start: u64,
    pub len: u64,
}

impl DmaWindow {
    pub const fn end(self) -> u64 {
        self.physical_start.saturating_add(self.len)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86_64DevicePlatformConfig {
    pub direct_map_base: u64,
    pub direct_map_size: u64,
    pub dma_window: DmaWindow,
    pub interrupt_vector_base: u8,
    pub interrupt_vector_count: u8,
}

impl Default for X86_64DevicePlatformConfig {
    fn default() -> Self {
        Self {
            direct_map_base: DEFAULT_DIRECT_MAP_BASE,
            direct_map_size: DEFAULT_DIRECT_MAP_SIZE,
            dma_window: DmaWindow {
                physical_start: 0x0200_0000,
                len: 0x0200_0000,
            },
            interrupt_vector_base: 64,
            interrupt_vector_count: 64,
        }
    }
}

pub trait PciConfigBackend {
    fn read_u32(&mut self, address: PciAddress, offset: u16) -> Result<u32, HalError>;
    fn write_u32(&mut self, address: PciAddress, offset: u16, value: u32) -> Result<(), HalError>;
    fn read_physical_bytes(
        &mut self,
        _physical_base: u64,
        _len: usize,
    ) -> Result<Vec<u8>, HalError> {
        Err(HalError::Unsupported)
    }
}

#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, Default)]
pub struct PciLegacyPortBackend;

#[cfg(target_arch = "x86_64")]
impl PciLegacyPortBackend {
    pub const fn new() -> Self {
        Self
    }

    fn address_value(address: PciAddress, offset: u16) -> Result<u32, HalError> {
        if address.segment != 0 || offset >= 256 || !offset.is_multiple_of(4) {
            return Err(HalError::InvalidConfigAccess);
        }
        Ok(0x8000_0000
            | ((address.bus as u32) << 16)
            | ((address.device as u32) << 11)
            | ((address.function as u32) << 8)
            | (u32::from(offset) & 0xfc))
    }

    unsafe fn outl(port: u16, value: u32) {
        unsafe {
            asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
        }
    }

    unsafe fn inl(port: u16) -> u32 {
        let value: u32;
        unsafe {
            asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
        }
        value
    }
}

#[cfg(target_arch = "x86_64")]
impl PciConfigBackend for PciLegacyPortBackend {
    fn read_u32(&mut self, address: PciAddress, offset: u16) -> Result<u32, HalError> {
        let config_address = Self::address_value(address, offset)?;
        unsafe {
            Self::outl(0xcf8, config_address);
            Ok(Self::inl(0xcfc))
        }
    }

    fn write_u32(&mut self, address: PciAddress, offset: u16, value: u32) -> Result<(), HalError> {
        let config_address = Self::address_value(address, offset)?;
        unsafe {
            Self::outl(0xcf8, config_address);
            Self::outl(0xcfc, value);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciAddress {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    pub const fn encoded(self) -> u64 {
        ((self.segment as u64) << 24)
            | ((self.bus as u64) << 16)
            | ((self.device as u64) << 8)
            | self.function as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DiscoveredBar {
    info: BarInfo,
    register_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiscoveredDevice {
    locator: DeviceLocator,
    pci: PciAddress,
    identity: DeviceIdentity,
    bars: Vec<DiscoveredBar>,
    interrupts: Vec<InterruptCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BarClaim {
    device: DeviceLocator,
    bar: BarId,
    physical_base: u64,
    len: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InterruptClaim {
    device: DeviceLocator,
    capability_index: usize,
    route: InterruptRoute,
    enabled: bool,
    delivered_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DmaAllocation {
    info: DmaBufferInfo,
    physical_base: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalRun {
    start: u64,
    len: u64,
}

impl PhysicalRun {
    const fn end(self) -> u64 {
        self.start.saturating_add(self.len)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86_64InterruptEvent {
    pub handle: InterruptHandle,
    pub route: InterruptRoute,
    pub delivered_count: u64,
}

pub struct X86_64DevicePlatform<B: PciConfigBackend> {
    descriptor: PlatformDescriptor,
    layout: X86_64KernelLayout,
    requirements: X86_64BootRequirements,
    config: X86_64DevicePlatformConfig,
    backend: B,
    devices: Vec<DiscoveredDevice>,
    bars_by_region: BTreeMap<MmioRegionId, BarClaim>,
    mappings: BTreeMap<MmioMappingId, MmioMapping>,
    interrupts: BTreeMap<InterruptHandle, InterruptClaim>,
    pending_interrupts: Vec<InterruptHandle>,
    dma_allocations: BTreeMap<DmaBufferId, DmaAllocation>,
    free_dma_runs: Vec<PhysicalRun>,
    next_bar_id: u64,
    next_region_id: u64,
    next_mapping_id: u64,
    next_interrupt_handle: u64,
    next_dma_id: u64,
    next_vector_offset: u8,

    // NVIDIA Nano-Agents
    nvidia_gpu: Option<NvidiaGpu>,
    nvidia_gsp: Option<NvidiaGspAgent>,
    nvidia_vram: Option<NvidiaVramAgent>,
    nvidia_display: Option<NvidiaDisplayAgent>,
    nvidia_interrupt: Option<NvidiaInterruptAgent>,
    nvidia_power: Option<NvidiaPowerAgent>,
    nvidia_media: Option<NvidiaMediaAgent>,
    nvidia_neural: Option<NvidiaNeuralAgent>,
    nvidia_tensor: Option<NvidiaTensorAgent>,
    virtio_gpu: Option<VirtioGpuDriver>,
}

impl<B: PciConfigBackend> X86_64DevicePlatform<B> {
    pub fn new(backend: B, config: X86_64DevicePlatformConfig) -> Self {
        Self {
            descriptor: PlatformDescriptor {
                name: "ngos-x86_64-device-platform",
                architecture: Architecture::X86_64,
                host_runtime_mode: false,
            },
            layout: X86_64KernelLayout::higher_half_default(),
            requirements: X86_64BootRequirements::baseline(),
            config,
            backend,
            devices: Vec::new(),
            bars_by_region: BTreeMap::new(),
            mappings: BTreeMap::new(),
            interrupts: BTreeMap::new(),
            pending_interrupts: Vec::new(),
            dma_allocations: BTreeMap::new(),
            free_dma_runs: vec![PhysicalRun {
                start: config.dma_window.physical_start,
                len: config.dma_window.len,
            }],
            next_bar_id: 1,
            next_region_id: 1,
            next_mapping_id: 1,
            next_interrupt_handle: 1,
            next_dma_id: 1,
            next_vector_offset: 0,
            nvidia_gpu: None,
            nvidia_gsp: None,
            nvidia_vram: None,
            nvidia_display: None,
            nvidia_interrupt: None,
            nvidia_power: None,
            nvidia_media: None,
            nvidia_neural: None,
            nvidia_tensor: None,
            virtio_gpu: None,
        }
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn layout(&self) -> X86_64KernelLayout {
        self.layout
    }

    pub fn requirements(&self) -> X86_64BootRequirements {
        self.requirements
    }

    pub fn dispatch_interrupt_vector(
        &mut self,
        vector: u8,
    ) -> Result<Option<X86_64InterruptEvent>, HalError> {
        let handle = self.interrupts.iter().find_map(|(handle, claim)| {
            (claim.enabled && claim.route.vector == vector).then_some(*handle)
        });
        let Some(handle) = handle else {
            return Ok(None);
        };
        let claim = self
            .interrupts
            .get_mut(&handle)
            .ok_or(HalError::InvalidInterrupt)?;
        claim.delivered_count = claim.delivered_count.saturating_add(1);
        self.pending_interrupts.push(handle);
        Ok(Some(X86_64InterruptEvent {
            handle,
            route: claim.route,
            delivered_count: claim.delivered_count,
        }))
    }

    pub fn pending_interrupts(&self) -> &[InterruptHandle] {
        &self.pending_interrupts
    }

    pub fn mapping_for_region(&self, region: MmioRegionId) -> Option<MmioMapping> {
        self.mappings
            .values()
            .find(|mapping| mapping.region == region)
            .copied()
    }

    pub fn nvidia_interrupt_agent(&self) -> Option<&NvidiaInterruptAgent> {
        self.nvidia_interrupt.as_ref()
    }

    pub fn nvidia_display_agent(&self) -> Option<&NvidiaDisplayAgent> {
        self.nvidia_display.as_ref()
    }

    pub fn service_nvidia_interrupt(&mut self) -> Result<Option<X86_64InterruptEvent>, HalError> {
        let vector = match self.nvidia_interrupt.as_ref() {
            Some(interrupt) => interrupt.vector,
            None => return Ok(None),
        };
        let event = self.dispatch_interrupt_vector(vector)?;
        if let Some(delivered) = event {
            if let Some(interrupt) = self.nvidia_interrupt.as_mut() {
                interrupt.count = delivered.delivered_count;
            }
            self.acknowledge_interrupt(delivered.handle)?;
        }
        Ok(event)
    }

    fn scan_pci(&mut self) -> Result<Vec<DiscoveredDevice>, HalError> {
        let mut devices = Vec::new();
        for bus in 0u16..=255 {
            for device in 0u8..32 {
                let address = PciAddress {
                    segment: 0,
                    bus: bus as u8,
                    device,
                    function: 0,
                };
                if self.vendor_id(address)? == 0xffff {
                    continue;
                }
                let function_count = if self.is_multifunction(address)? {
                    8
                } else {
                    1
                };
                for function in 0u8..function_count {
                    let function_address = PciAddress {
                        function,
                        ..address
                    };
                    if self.vendor_id(function_address)? == 0xffff {
                        continue;
                    }
                    devices.push(self.read_device(function_address)?);
                }
            }
        }
        devices.sort_by_key(|lhs| lhs.locator.raw());
        Ok(devices)
    }

    fn read_device(&mut self, pci: PciAddress) -> Result<DiscoveredDevice, HalError> {
        let vendor_device = self.read_pci_dword(pci, PCI_VENDOR_DEVICE_OFFSET)?;
        let class_revision = self.read_pci_dword(pci, PCI_CLASS_REVISION_OFFSET)?;
        let subsystem = self.read_pci_dword(pci, 0x2c)?;
        let locator = DeviceLocator::new(
            (u64::from(pci.segment) << 24)
                | (u64::from(pci.bus) << 16)
                | (u64::from(pci.device) << 8)
                | u64::from(pci.function),
        );

        Ok(DiscoveredDevice {
            locator,
            pci,
            identity: DeviceIdentity {
                vendor_id: vendor_device as u16,
                device_id: (vendor_device >> 16) as u16,
                subsystem_vendor_id: subsystem as u16,
                subsystem_device_id: (subsystem >> 16) as u16,
                revision_id: class_revision as u8,
                base_class: (class_revision >> 24) as u8,
                sub_class: (class_revision >> 16) as u8,
                programming_interface: (class_revision >> 8) as u8,
            },
            bars: self.read_bars(pci)?,
            interrupts: self.read_interrupt_capabilities(pci)?,
        })
    }

    fn read_bars(&mut self, pci: PciAddress) -> Result<Vec<DiscoveredBar>, HalError> {
        let mut bars = Vec::new();
        let mut register_index = 0u8;
        while register_index < 6 {
            let offset = PCI_BAR0_OFFSET + u16::from(register_index) * 4;
            let original = self.read_pci_dword(pci, offset)?;
            if original == 0 {
                register_index += 1;
                continue;
            }
            self.write_pci_dword(pci, offset, u32::MAX)?;
            let size_probe = self.read_pci_dword(pci, offset)?;
            self.write_pci_dword(pci, offset, original)?;

            if (original & 0x1) == 0x1 {
                let size_mask = (size_probe & 0xffff_fffc) as u64;
                let size = (!(size_mask)).wrapping_add(1) & 0xffff_ffff;
                if size == 0 {
                    register_index += 1;
                    continue;
                }
                bars.push(DiscoveredBar {
                    info: BarInfo {
                        id: BarId::new(self.alloc_bar_id()),
                        kind: BarKind::IoPort,
                        base: (original & 0xffff_fffc) as u64,
                        size,
                        flags: BarFlags {
                            prefetchable: false,
                            cacheable: false,
                            read_only: false,
                        },
                    },
                    register_index,
                });
                register_index += 1;
                continue;
            }

            let memory_type = (original >> 1) & 0x3;
            let prefetchable = (original & (1 << 3)) != 0;
            if memory_type == 0x2 {
                if register_index + 1 >= 6 {
                    return Err(HalError::InvalidBar);
                }
                let next_offset = offset + 4;
                let original_high = self.read_pci_dword(pci, next_offset)?;
                self.write_pci_dword(pci, next_offset, u32::MAX)?;
                let size_probe_high = self.read_pci_dword(pci, next_offset)?;
                self.write_pci_dword(pci, next_offset, original_high)?;
                let base = ((original_high as u64) << 32) | (original as u64 & 0xffff_fff0);
                let size_mask =
                    ((size_probe_high as u64) << 32) | (size_probe as u64 & 0xffff_fff0);
                let size = (!size_mask).wrapping_add(1);
                if size != 0 {
                    bars.push(DiscoveredBar {
                        info: BarInfo {
                            id: BarId::new(self.alloc_bar_id()),
                            kind: BarKind::Memory64,
                            base,
                            size,
                            flags: BarFlags {
                                prefetchable,
                                cacheable: false,
                                read_only: false,
                            },
                        },
                        register_index,
                    });
                }
                register_index += 2;
                continue;
            }

            let size_mask = (size_probe & 0xffff_fff0) as u64;
            let size = (!(size_mask)).wrapping_add(1) & 0xffff_ffff;
            if size != 0 {
                bars.push(DiscoveredBar {
                    info: BarInfo {
                        id: BarId::new(self.alloc_bar_id()),
                        kind: if memory_type == 0x0 {
                            BarKind::Memory32
                        } else {
                            return Err(HalError::InvalidBar);
                        },
                        base: (original & 0xffff_fff0) as u64,
                        size,
                        flags: BarFlags {
                            prefetchable,
                            cacheable: false,
                            read_only: false,
                        },
                    },
                    register_index,
                });
            }
            register_index += 1;
        }
        Ok(bars)
    }

    fn read_interrupt_capabilities(
        &mut self,
        pci: PciAddress,
    ) -> Result<Vec<InterruptCapability>, HalError> {
        let mut capabilities = Vec::new();
        let interrupt_line = self.read_pci_byte(pci, PCI_INTERRUPT_LINE_OFFSET)?;
        let interrupt_pin = self.read_pci_byte(pci, PCI_INTERRUPT_LINE_OFFSET + 1)?;
        if interrupt_pin != 0 {
            capabilities.push(InterruptCapability {
                kind: InterruptKind::LegacyLine,
                vectors: 1,
                line: (interrupt_line != 0xff).then_some(interrupt_line),
                trigger: InterruptTrigger::Level,
                polarity: InterruptPolarity::Low,
            });
        }

        let command_status = self.read_pci_dword(pci, PCI_COMMAND_STATUS_OFFSET)?;
        let status = (command_status >> 16) as u16;
        if (status & PCI_STATUS_CAPABILITIES) == 0 {
            return Ok(capabilities);
        }

        let mut cap_offset = self.read_pci_byte(pci, PCI_CAP_PTR_OFFSET)? as u16;
        let mut guard = 0usize;
        while cap_offset >= 0x40 && guard < 64 {
            guard += 1;
            let header = self.read_pci_dword(pci, cap_offset)?;
            let cap_id = header as u8;
            let next = ((header >> 8) & 0xff) as u16;
            match cap_id {
                PCI_CAP_ID_MSI => {
                    let control = (header >> 16) as u16;
                    let vectors = 1u16 << ((control >> 1) & 0x7);
                    capabilities.push(InterruptCapability {
                        kind: InterruptKind::Msi,
                        vectors,
                        line: None,
                        trigger: InterruptTrigger::Edge,
                        polarity: InterruptPolarity::High,
                    });
                }
                PCI_CAP_ID_MSIX => {
                    let control = (header >> 16) as u16;
                    capabilities.push(InterruptCapability {
                        kind: InterruptKind::Msix,
                        vectors: (control & 0x07ff).saturating_add(1),
                        line: None,
                        trigger: InterruptTrigger::Edge,
                        polarity: InterruptPolarity::High,
                    });
                }
                _ => {}
            }
            if next == 0 || next == cap_offset {
                break;
            }
            cap_offset = next;
        }

        Ok(capabilities)
    }

    fn vendor_id(&mut self, pci: PciAddress) -> Result<u16, HalError> {
        Ok(self.read_pci_dword(pci, PCI_VENDOR_DEVICE_OFFSET)? as u16)
    }

    fn is_multifunction(&mut self, pci: PciAddress) -> Result<bool, HalError> {
        Ok((self.read_pci_byte(pci, PCI_HEADER_TYPE_OFFSET + 2)? & 0x80) != 0)
    }

    fn read_pci_dword(&mut self, pci: PciAddress, offset: u16) -> Result<u32, HalError> {
        if !offset.is_multiple_of(4) {
            return Err(HalError::InvalidConfigAccess);
        }
        self.backend.read_u32(pci, offset)
    }

    fn write_pci_dword(
        &mut self,
        pci: PciAddress,
        offset: u16,
        value: u32,
    ) -> Result<(), HalError> {
        if !offset.is_multiple_of(4) {
            return Err(HalError::InvalidConfigAccess);
        }
        self.backend.write_u32(pci, offset, value)
    }

    fn read_pci_byte(&mut self, pci: PciAddress, offset: u16) -> Result<u8, HalError> {
        let aligned = offset & !0x3;
        let shift = (offset & 0x3) * 8;
        Ok(((self.read_pci_dword(pci, aligned)? >> shift) & 0xff) as u8)
    }

    fn validate_config_access(access: ConfigAccess) -> Result<(), HalError> {
        if access.kind != ConfigSpaceKind::Pci {
            return Err(HalError::InvalidConfigAccess);
        }
        let align = match access.width {
            ConfigWidth::U8 => 1,
            ConfigWidth::U16 => 2,
            ConfigWidth::U32 => 4,
        };
        if !access.offset.is_multiple_of(align) {
            return Err(HalError::InvalidConfigAccess);
        }
        Ok(())
    }

    fn device(&self, locator: DeviceLocator) -> Result<&DiscoveredDevice, HalError> {
        self.devices
            .iter()
            .find(|device| device.locator == locator)
            .ok_or(HalError::InvalidDevice)
    }

    fn find_bar(&self, device: DeviceLocator, bar: BarId) -> Result<&DiscoveredBar, HalError> {
        let device = self.device(device)?;
        device
            .bars
            .iter()
            .find(|candidate| candidate.info.id == bar)
            .ok_or(HalError::InvalidBar)
    }

    fn alloc_bar_id(&mut self) -> u64 {
        let value = self.next_bar_id;
        self.next_bar_id = self.next_bar_id.saturating_add(1);
        value
    }

    fn alloc_vector(&mut self) -> Result<u8, HalError> {
        if self.next_vector_offset >= self.config.interrupt_vector_count {
            return Err(HalError::Exhausted);
        }
        let vector = self
            .config
            .interrupt_vector_base
            .saturating_add(self.next_vector_offset);
        self.next_vector_offset = self.next_vector_offset.saturating_add(1);
        Ok(vector)
    }

    fn direct_map_address(&self, physical_base: u64, len: u64) -> Result<u64, HalError> {
        let end = physical_base
            .checked_add(len)
            .ok_or(HalError::InvalidMmioMapping)?;
        if end > self.config.direct_map_size {
            return Err(HalError::InvalidMmioMapping);
        }
        self.config
            .direct_map_base
            .checked_add(physical_base)
            .ok_or(HalError::InvalidMmioMapping)
    }

    fn allocate_physical_run(
        &mut self,
        len: u64,
        constraints: DmaConstraints,
    ) -> Result<PhysicalRun, HalError> {
        if len == 0 {
            return Err(HalError::InvalidDmaBuffer);
        }
        let alignment = constraints.alignment.max(PAGE_SIZE_4K);
        let requested = align_up(len, PAGE_SIZE_4K);
        let max_address = if constraints.max_address_bits >= 64 {
            u64::MAX
        } else {
            (1u64 << constraints.max_address_bits) - 1
        };
        for index in 0..self.free_dma_runs.len() {
            let run = self.free_dma_runs[index];
            let aligned_start = align_up(run.start, alignment);
            let end = aligned_start.saturating_add(requested);
            if end > run.end() {
                continue;
            }
            if end.saturating_sub(1) > max_address {
                continue;
            }
            if constraints.segment_boundary != u64::MAX
                && crosses_boundary(aligned_start, requested, constraints.segment_boundary)
            {
                continue;
            }
            let mut replacement = Vec::new();
            if aligned_start > run.start {
                replacement.push(PhysicalRun {
                    start: run.start,
                    len: aligned_start - run.start,
                });
            }
            if end < run.end() {
                replacement.push(PhysicalRun {
                    start: end,
                    len: run.end() - end,
                });
            }
            self.free_dma_runs.remove(index);
            for (insert_offset, segment) in replacement.into_iter().enumerate() {
                self.free_dma_runs.insert(index + insert_offset, segment);
            }
            return Ok(PhysicalRun {
                start: aligned_start,
                len: requested,
            });
        }
        Err(HalError::Exhausted)
    }

    fn release_physical_run(&mut self, released: PhysicalRun) {
        self.free_dma_runs.push(released);
        self.free_dma_runs.sort_by_key(|lhs| lhs.start);
        let mut merged: Vec<PhysicalRun> = Vec::with_capacity(self.free_dma_runs.len());
        for run in self.free_dma_runs.iter().copied() {
            match merged.last_mut() {
                Some(previous) if previous.end() >= run.start => {
                    previous.len = previous.end().max(run.end()) - previous.start;
                }
                _ => merged.push(run),
            }
        }
        self.free_dma_runs = merged;
    }
}

#[cfg(target_arch = "x86_64")]
impl X86_64DevicePlatform<PciLegacyPortBackend> {
    pub fn with_legacy_pci_ports(config: X86_64DevicePlatformConfig) -> Self {
        Self::new(PciLegacyPortBackend::new(), config)
    }
}

impl<B: PciConfigBackend> Platform for X86_64DevicePlatform<B> {
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

impl<B: PciConfigBackend> DevicePlatform for X86_64DevicePlatform<B> {
    fn enumerate_devices(&mut self) -> Result<Vec<DeviceRecord>, HalError> {
        self.devices = self.scan_pci()?;
        Ok(self
            .devices
            .iter()
            .map(|device| DeviceRecord {
                locator: device.locator,
                bus_kind: BusKind::Pci,
                address: BusAddress::Pci(platform_hal::PciAddress {
                    segment: device.pci.segment,
                    bus: device.pci.bus,
                    device: device.pci.device,
                    function: device.pci.function,
                }),
                identity: device.identity,
                bars: device.bars.iter().map(|bar| bar.info).collect(),
                interrupts: device.interrupts.clone(),
            })
            .collect())
    }

    fn read_config_u32(
        &mut self,
        device: DeviceLocator,
        access: ConfigAccess,
    ) -> Result<u32, HalError> {
        Self::validate_config_access(access)?;
        let pci = self.device(device)?.pci;
        let aligned = access.offset & !0x3;
        let shift = (access.offset & 0x3) * 8;
        let raw = self.read_pci_dword(pci, aligned)?;
        Ok(match access.width {
            ConfigWidth::U8 => (raw >> shift) & 0xff,
            ConfigWidth::U16 => (raw >> shift) & 0xffff,
            ConfigWidth::U32 => raw,
        })
    }

    fn write_config_u32(
        &mut self,
        device: DeviceLocator,
        access: ConfigAccess,
        value: u32,
    ) -> Result<(), HalError> {
        Self::validate_config_access(access)?;
        let pci = self.device(device)?.pci;
        let aligned = access.offset & !0x3;
        if access.width == ConfigWidth::U32 {
            return self.write_pci_dword(pci, aligned, value);
        }
        let shift = (access.offset & 0x3) * 8;
        let mut raw = self.read_pci_dword(pci, aligned)?;
        match access.width {
            ConfigWidth::U8 => {
                raw &= !(0xff << shift);
                raw |= (value & 0xff) << shift;
            }
            ConfigWidth::U16 => {
                raw &= !(0xffff << shift);
                raw |= (value & 0xffff) << shift;
            }
            ConfigWidth::U32 => {}
        }
        self.write_pci_dword(pci, aligned, raw)
    }

    fn claim_bar(&mut self, device: DeviceLocator, bar: BarId) -> Result<MmioRegionId, HalError> {
        if self
            .bars_by_region
            .values()
            .any(|claim| claim.device == device && claim.bar == bar)
        {
            return Err(HalError::BarAlreadyClaimed);
        }
        let bar = *self.find_bar(device, bar)?;
        if bar.info.kind == BarKind::IoPort {
            return Err(HalError::InvalidBar);
        }
        let region = MmioRegionId::new(self.next_region_id);
        self.next_region_id = self.next_region_id.saturating_add(1);
        self.bars_by_region.insert(
            region,
            BarClaim {
                device,
                bar: bar.info.id,
                physical_base: bar.info.base,
                len: bar.info.size,
            },
        );
        Ok(region)
    }

    fn release_bar(&mut self, region: MmioRegionId) -> Result<(), HalError> {
        if self
            .mappings
            .values()
            .any(|mapping| mapping.region == region)
        {
            return Err(HalError::DmaBusy);
        }
        self.bars_by_region
            .remove(&region)
            .map(|_| ())
            .ok_or(HalError::BarNotClaimed)
    }

    fn map_mmio(
        &mut self,
        region: MmioRegionId,
        perms: MmioPermissions,
        cache_policy: MmioCachePolicy,
    ) -> Result<MmioMapping, HalError> {
        let claim = self
            .bars_by_region
            .get(&region)
            .copied()
            .ok_or(HalError::BarNotClaimed)?;
        if self
            .mappings
            .values()
            .any(|mapping| mapping.region == region)
        {
            return Err(HalError::BarAlreadyClaimed);
        }
        let virtual_base = self.direct_map_address(claim.physical_base, claim.len)?;
        let mapping = MmioMapping {
            id: MmioMappingId::new(self.next_mapping_id),
            region,
            physical_base: claim.physical_base,
            virtual_base,
            len: claim.len,
            perms,
            cache_policy,
        };
        self.next_mapping_id = self.next_mapping_id.saturating_add(1);
        self.mappings.insert(mapping.id, mapping);
        Ok(mapping)
    }

    fn unmap_mmio(&mut self, mapping: MmioMappingId) -> Result<(), HalError> {
        self.mappings
            .remove(&mapping)
            .map(|_| ())
            .ok_or(HalError::InvalidMmioMapping)
    }

    fn claim_interrupt(
        &mut self,
        device: DeviceLocator,
        capability_index: usize,
    ) -> Result<(InterruptHandle, InterruptRoute), HalError> {
        let device_locator = device;
        let capability = *self
            .device(device)?
            .interrupts
            .get(capability_index)
            .ok_or(HalError::InvalidInterrupt)?;
        if self.interrupts.values().any(|claim| {
            claim.device == device_locator && claim.capability_index == capability_index
        }) {
            return Err(HalError::InterruptAlreadyClaimed);
        }
        let handle = InterruptHandle::new(self.next_interrupt_handle);
        self.next_interrupt_handle = self.next_interrupt_handle.saturating_add(1);
        let vector = match capability.kind {
            InterruptKind::LegacyLine => {
                let line = capability.line.ok_or(HalError::InvalidInterrupt)?;
                let line_offset = self.config.interrupt_vector_base.saturating_add(line);
                let max_vector = self
                    .config
                    .interrupt_vector_base
                    .saturating_add(self.config.interrupt_vector_count);
                if line_offset >= max_vector {
                    return Err(HalError::InvalidInterrupt);
                }
                line_offset
            }
            InterruptKind::Msi | InterruptKind::Msix => self.alloc_vector()?,
        };
        let route = InterruptRoute {
            kind: capability.kind,
            vector,
            line: capability.line,
        };
        self.interrupts.insert(
            handle,
            InterruptClaim {
                device: device_locator,
                capability_index,
                route,
                enabled: false,
                delivered_count: 0,
            },
        );
        Ok((handle, route))
    }

    fn enable_interrupt(&mut self, handle: InterruptHandle) -> Result<(), HalError> {
        let claim = self
            .interrupts
            .get_mut(&handle)
            .ok_or(HalError::InvalidInterrupt)?;
        claim.enabled = true;
        Ok(())
    }

    fn disable_interrupt(&mut self, handle: InterruptHandle) -> Result<(), HalError> {
        let claim = self
            .interrupts
            .get_mut(&handle)
            .ok_or(HalError::InvalidInterrupt)?;
        claim.enabled = false;
        Ok(())
    }

    fn acknowledge_interrupt(&mut self, handle: InterruptHandle) -> Result<(), HalError> {
        if !self.interrupts.contains_key(&handle) {
            return Err(HalError::InvalidInterrupt);
        }
        if let Some(index) = self
            .pending_interrupts
            .iter()
            .position(|candidate| *candidate == handle)
        {
            self.pending_interrupts.remove(index);
        }
        Ok(())
    }

    fn allocate_dma(
        &mut self,
        len: u64,
        direction: DmaDirection,
        coherency: DmaCoherency,
        constraints: DmaConstraints,
    ) -> Result<DmaBufferInfo, HalError> {
        let run = self.allocate_physical_run(len, constraints)?;
        let id = DmaBufferId::new(self.next_dma_id);
        self.next_dma_id = self.next_dma_id.saturating_add(1);
        let info = DmaBufferInfo {
            id,
            cpu_virtual: self.direct_map_address(run.start, run.len)?,
            device_address: run.start,
            len: run.len,
            direction,
            coherency,
            ownership: DmaOwnership::Cpu,
        };
        self.dma_allocations.insert(
            id,
            DmaAllocation {
                info,
                physical_base: run.start,
            },
        );
        Ok(info)
    }

    fn prepare_dma_for_device(&mut self, buffer: DmaBufferId) -> Result<(), HalError> {
        let allocation = self
            .dma_allocations
            .get_mut(&buffer)
            .ok_or(HalError::InvalidDmaBuffer)?;
        if allocation.info.ownership != DmaOwnership::Cpu {
            return Err(HalError::DmaBusy);
        }
        allocation.info.ownership = DmaOwnership::Device;
        Ok(())
    }

    fn complete_dma_from_device(&mut self, buffer: DmaBufferId) -> Result<(), HalError> {
        let allocation = self
            .dma_allocations
            .get_mut(&buffer)
            .ok_or(HalError::InvalidDmaBuffer)?;
        if allocation.info.ownership != DmaOwnership::Device {
            return Err(HalError::DmaBusy);
        }
        allocation.info.ownership = DmaOwnership::Cpu;
        Ok(())
    }

    fn release_dma(&mut self, buffer: DmaBufferId) -> Result<(), HalError> {
        let allocation = self
            .dma_allocations
            .remove(&buffer)
            .ok_or(HalError::InvalidDmaBuffer)?;
        if allocation.info.ownership != DmaOwnership::Cpu {
            return Err(HalError::DmaBusy);
        }
        self.release_physical_run(PhysicalRun {
            start: allocation.physical_base,
            len: allocation.info.len,
        });
        Ok(())
    }
}

impl<B: PciConfigBackend> platform_hal::FirmwareReadablePlatform for X86_64DevicePlatform<B> {
    fn read_device_rom(
        &mut self,
        _device: DeviceLocator,
        physical_base: u64,
        len: usize,
    ) -> Result<Vec<u8>, HalError> {
        self.backend.read_physical_bytes(physical_base, len)
    }
}

impl<B: PciConfigBackend> platform_hal::GpuPlatform for X86_64DevicePlatform<B> {
    fn get_gpu_vendor(&self) -> GpuVendor {
        if self.nvidia_gpu.is_some() {
            GpuVendor::Nvidia
        } else if self.virtio_gpu.is_some() {
            GpuVendor::Virtio
        } else {
            GpuVendor::Unknown
        }
    }

    fn get_gpu_name(&self) -> String {
        match self.get_gpu_vendor() {
            GpuVendor::Nvidia => String::from("NVIDIA GeForce RTX 5060 Ti (Blackwell)"),
            GpuVendor::Virtio => String::from("VirtIO GPU"),
            GpuVendor::Amd => String::from("AMD Radeon"),
            GpuVendor::Intel => String::from("Intel Graphics"),
            GpuVendor::Unknown => String::from("Unknown GPU Device"),
        }
    }

    fn setup_gpu_agent(&mut self, locator: DeviceLocator) -> Result<(), HalError> {
        let records = self.enumerate_devices()?;
        let record = records
            .iter()
            .find(|r| r.locator == locator)
            .cloned()
            .ok_or(HalError::InvalidDevice)?;
        if let Some(mut gpu) = NvidiaGpu::try_detect(self, locator)? {
            let mmio_vaddr = gpu.map_control_registers(self)?;
            let gsp_ctx = gpu.setup_gsp_channels(self)?;
            let interrupt = gpu.setup_interrupts(self)?;

            // Map BAR1 for VRAM
            let bar1_region = gpu
                .vram_bar
                .ok_or(HalError::InvalidBar)
                .and_then(|bar| self.claim_bar(locator, bar))?;
            let bar1_mapping = self.map_mmio(
                bar1_region,
                MmioPermissions::read_write(),
                MmioCachePolicy::WriteCombining,
            )?;

            let vram_size = record.bars.get(1).map(|b| b.size).unwrap_or(0x1000_0000);

            self.nvidia_gpu = Some(gpu);
            self.nvidia_gsp = Some(NvidiaGspAgent::new(mmio_vaddr, gsp_ctx));
            self.nvidia_vram = Some(NvidiaVramAgent::new(bar1_mapping.virtual_base, vram_size));
            self.nvidia_display = Some(NvidiaDisplayAgent::new(1));
            self.nvidia_interrupt = Some(interrupt);
            self.nvidia_power = Some(NvidiaPowerAgent::new());
            self.nvidia_media = Some(NvidiaMediaAgent::new());
            self.nvidia_neural = Some(NvidiaNeuralAgent::new());
            self.nvidia_tensor = Some(NvidiaTensorAgent::new());
            self.virtio_gpu = None;
            return Ok(());
        }

        let driver = VirtioGpuDriver::initialize(self, &record).map_err(map_virtio_gpu_error)?;
        self.nvidia_gpu = None;
        self.nvidia_gsp = None;
        self.nvidia_vram = None;
        self.nvidia_display = None;
        self.nvidia_interrupt = None;
        self.nvidia_power = None;
        self.nvidia_media = None;
        self.nvidia_neural = None;
        self.nvidia_tensor = None;
        self.virtio_gpu = Some(driver);
        Ok(())
    }

    fn submit_gpu_command(&mut self, rpc_id: u32, payload: &[u8]) -> Result<Vec<u8>, HalError> {
        if self.virtio_gpu.is_some() {
            let mut driver = self.virtio_gpu.take().ok_or(HalError::InvalidDevice)?;
            let _slot = driver
                .submit_command(self, payload)
                .map_err(map_virtio_gpu_error)?;
            self.virtio_gpu = Some(driver);
            return Ok(rpc_id.to_le_bytes().to_vec());
        }
        let gsp = self.nvidia_gsp.as_mut().ok_or(HalError::InvalidDevice)?;
        let gpu = self.nvidia_gpu.as_mut().ok_or(HalError::InvalidDevice)?;
        let response = unsafe { gsp.execute_semantic_op(gpu, rpc_id, payload) }?;
        if rpc_id == 0x0100 || rpc_id == GPU_PRESENT_OPCODE {
            let vram_len = self
                .nvidia_vram
                .as_ref()
                .ok_or(HalError::InvalidDevice)?
                .bar1_len();
            self.nvidia_display
                .as_mut()
                .ok_or(HalError::InvalidDevice)?
                .plan_frame_present(0, payload.len(), vram_len)?;
            let _ = self.service_nvidia_interrupt()?;
        }
        Ok(response)
    }

    fn allocate_gpu_memory(&mut self, kind: GpuMemoryKind, size: u64) -> Result<u64, HalError> {
        if self.virtio_gpu.is_some() {
            let dma = self.allocate_dma(
                size,
                DmaDirection::Bidirectional,
                DmaCoherency::Coherent,
                DmaConstraints::platform_default(),
            )?;
            return Ok(dma.cpu_virtual);
        }
        match kind {
            GpuMemoryKind::Vram => {
                let vram = self.nvidia_vram.as_mut().ok_or(HalError::InvalidDevice)?;
                let slice = vram.grant_slice(size)?;
                Ok(slice.mapping_vaddr)
            }
            GpuMemoryKind::SystemShared => {
                let dma = self.allocate_dma(
                    size,
                    DmaDirection::Bidirectional,
                    DmaCoherency::Coherent,
                    DmaConstraints::platform_default(),
                )?;
                Ok(dma.cpu_virtual)
            }
        }
    }

    fn gpu_binding_evidence(
        &mut self,
        device: DeviceLocator,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        let devices = self.enumerate_devices()?;
        let record = devices
            .iter()
            .find(|candidate| candidate.locator == device)
            .ok_or(HalError::InvalidDevice)?;
        Ok(
            local_windows_binding_for_device(record.identity.vendor_id, record.identity.device_id)
                .map(as_platform_gpu_binding_evidence),
        )
    }

    fn primary_gpu_binding_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        if let Some(gpu) = self.nvidia_gpu.as_ref() {
            return self.gpu_binding_evidence(gpu.locator);
        }
        let devices = self.enumerate_devices()?;
        let record = devices
            .iter()
            .find(|candidate| candidate.identity.class() == platform_hal::DeviceClass::Display);
        Ok(record
            .and_then(|candidate| {
                local_windows_binding_for_device(
                    candidate.identity.vendor_id,
                    candidate.identity.device_id,
                )
            })
            .map(as_platform_gpu_binding_evidence))
    }

    fn primary_gpu_vbios_window(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosWindowEvidence>, HalError> {
        let Some(gpu) = self.nvidia_gpu else {
            return Ok(None);
        };
        let window = gpu.inspect_vbios_window(self)?;
        Ok(Some(as_platform_gpu_vbios_window_evidence(window)))
    }

    fn primary_gpu_vbios_bytes(&mut self, max_len: usize) -> Result<Vec<u8>, HalError> {
        let gpu = self.nvidia_gpu.ok_or(HalError::InvalidDevice)?;
        let mut bytes = gpu.read_vbios(self)?;
        if bytes.len() > max_len {
            bytes.truncate(max_len);
        }
        Ok(bytes)
    }

    fn primary_gpu_vbios_image_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosImageEvidence>, HalError> {
        let gpu = self.nvidia_gpu.ok_or(HalError::InvalidDevice)?;
        let bytes = gpu.read_vbios(self)?;
        let evidence = inspect_vbios_image_bytes(&bytes)?;
        Ok(Some(as_platform_gpu_vbios_image_evidence(evidence)))
    }

    fn primary_gpu_gsp_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuGspEvidence>, HalError> {
        let Some(gsp) = self.nvidia_gsp.as_ref() else {
            return Ok(None);
        };
        Ok(Some(platform_hal::GpuGspEvidence {
            present: true,
            loopback_ready: gsp.loopback_ready(),
            firmware_known: false,
            blackwell_blob_present: RTX_5060_TI_LOCAL_GSP_BLACKWELL_BLOB_PRESENT,
            hardware_ready: RTX_5060_TI_LOCAL_GSP_REAL_HARDWARE_READY,
            driver_model_wddm: RTX_5060_TI_LOCAL_GSP_DRIVER_MODEL_WDDM,
            loopback_completions: gsp.loopback_completions(),
            loopback_failures: gsp.loopback_failures(),
            firmware_version: RTX_5060_TI_LOCAL_GSP_FIRMWARE_VERSION,
            blob_summary: RTX_5060_TI_LOCAL_GSP_BLOB_SUMMARY,
            refusal_reason: RTX_5060_TI_LOCAL_GSP_REFUSAL_REASON,
        }))
    }

    fn primary_gpu_interrupt_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuInterruptEvidence>, HalError> {
        let Some(interrupt) = self.nvidia_interrupt.as_ref() else {
            return Ok(None);
        };
        Ok(Some(platform_hal::GpuInterruptEvidence {
            present: true,
            vector: interrupt.vector,
            delivered_count: interrupt.count,
            msi_supported: true,
            message_limit: 1,
            windows_interrupt_message_maximum: RTX_5060_TI_LOCAL_INTERRUPT_MESSAGE_MAXIMUM,
            hardware_servicing_confirmed: RTX_5060_TI_LOCAL_HARDWARE_INTERRUPT_SERVICING_CONFIRMED,
        }))
    }

    fn primary_gpu_display_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuDisplayEvidence>, HalError> {
        let Some(display) = self.nvidia_display.as_ref() else {
            return Ok(None);
        };
        Ok(Some(platform_hal::GpuDisplayEvidence {
            present: true,
            active_pipes: display.active_pipes,
            planned_frames: display.presented_frames,
            last_present_offset: display.last_present_offset.unwrap_or(0),
            last_present_len: display.last_present_len as u64,
            hardware_programming_confirmed: false,
        }))
    }

    fn primary_gpu_power_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuPowerEvidence>, HalError> {
        let Some(power) = self.nvidia_power.as_ref() else {
            return Ok(None);
        };
        let pstate = match power.current_state {
            crate::nvidia_gpu::NvidiaPowerState::P0 => 0,
            crate::nvidia_gpu::NvidiaPowerState::P5 => 5,
            crate::nvidia_gpu::NvidiaPowerState::P8 => 8,
            crate::nvidia_gpu::NvidiaPowerState::P12 => 12,
        };
        Ok(Some(platform_hal::GpuPowerEvidence {
            present: true,
            pstate,
            graphics_clock_mhz: power.graphics_clock_mhz(),
            memory_clock_mhz: power.memory_clock_mhz(),
            boost_clock_mhz: power.boost_clock_mhz(),
            hardware_power_management_confirmed:
                RTX_5060_TI_LOCAL_HARDWARE_POWER_MANAGEMENT_CONFIRMED,
        }))
    }

    fn set_primary_gpu_power_state(&mut self, pstate: u32) -> Result<(), HalError> {
        let power = self.nvidia_power.as_mut().ok_or(HalError::InvalidDevice)?;
        let gpu = self.nvidia_gpu.as_mut().ok_or(HalError::InvalidDevice)?;
        let gsp = self.nvidia_gsp.as_mut().ok_or(HalError::InvalidDevice)?;
        let state = match pstate {
            0 => crate::nvidia_gpu::NvidiaPowerState::P0,
            5 => crate::nvidia_gpu::NvidiaPowerState::P5,
            8 => crate::nvidia_gpu::NvidiaPowerState::P8,
            12 => crate::nvidia_gpu::NvidiaPowerState::P12,
            _ => return Err(HalError::Unsupported),
        };
        unsafe { power.request_pstate(gpu, gsp, state) }
    }

    fn start_primary_gpu_media_session(
        &mut self,
        width: u32,
        height: u32,
        bitrate_kbps: u32,
        codec: u32,
    ) -> Result<(), HalError> {
        let media = self.nvidia_media.as_mut().ok_or(HalError::InvalidDevice)?;
        let gpu = self.nvidia_gpu.as_mut().ok_or(HalError::InvalidDevice)?;
        let gsp = self.nvidia_gsp.as_mut().ok_or(HalError::InvalidDevice)?;
        let config = crate::nvidia_gpu::NvidiaEncConfig {
            width,
            height,
            bitrate: bitrate_kbps,
            codec,
        };
        unsafe { media.start_encode_session(gpu, gsp, config) }
    }

    fn primary_gpu_media_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuMediaEvidence>, HalError> {
        let Some(media) = self.nvidia_media.as_ref() else {
            return Ok(None);
        };
        let last = media
            .last_config
            .unwrap_or(crate::nvidia_gpu::NvidiaEncConfig {
                width: 0,
                height: 0,
                bitrate: 0,
                codec: 0,
            });
        Ok(Some(platform_hal::GpuMediaEvidence {
            present: true,
            sessions: media.sessions as u32,
            codec: last.codec,
            width: last.width,
            height: last.height,
            bitrate_kbps: last.bitrate,
            hardware_media_confirmed: false,
        }))
    }

    fn inject_primary_gpu_neural_semantic(&mut self, semantic_label: &str) -> Result<(), HalError> {
        let neural = self.nvidia_neural.as_mut().ok_or(HalError::InvalidDevice)?;
        let gpu = self.nvidia_gpu.as_mut().ok_or(HalError::InvalidDevice)?;
        let gsp = self.nvidia_gsp.as_mut().ok_or(HalError::InvalidDevice)?;
        unsafe { neural.inject_scene_semantics(gpu, gsp, semantic_label) }
    }

    fn commit_primary_gpu_neural_frame(&mut self) -> Result<(), HalError> {
        let neural = self.nvidia_neural.as_mut().ok_or(HalError::InvalidDevice)?;
        let gpu = self.nvidia_gpu.as_mut().ok_or(HalError::InvalidDevice)?;
        let gsp = self.nvidia_gsp.as_mut().ok_or(HalError::InvalidDevice)?;
        unsafe { neural.commit_neural_refinement(gpu, gsp) }
    }

    fn primary_gpu_neural_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuNeuralEvidence>, HalError> {
        let Some(neural) = self.nvidia_neural.as_ref() else {
            return Ok(None);
        };
        Ok(Some(platform_hal::GpuNeuralEvidence {
            present: true,
            model_loaded: neural.model_loaded,
            active_semantics: neural.active_semantics.len() as u32,
            last_commit_completed: neural.last_commit_completed,
            hardware_neural_confirmed: false,
        }))
    }

    fn dispatch_primary_gpu_tensor_kernel(&mut self, kernel_id: u32) -> Result<(), HalError> {
        let tensor = self.nvidia_tensor.as_mut().ok_or(HalError::InvalidDevice)?;
        let gpu = self.nvidia_gpu.as_mut().ok_or(HalError::InvalidDevice)?;
        let gsp = self.nvidia_gsp.as_mut().ok_or(HalError::InvalidDevice)?;
        unsafe { tensor.dispatch_tensor_kernel(gpu, gsp, kernel_id) }
    }

    fn primary_gpu_tensor_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuTensorEvidence>, HalError> {
        let Some(tensor) = self.nvidia_tensor.as_ref() else {
            return Ok(None);
        };
        Ok(Some(platform_hal::GpuTensorEvidence {
            present: true,
            active_jobs: tensor.active_jobs as u32,
            last_kernel_id: tensor.last_kernel_id,
            hardware_tensor_confirmed: false,
        }))
    }
}

fn map_virtio_gpu_error(error: VirtioGpuError) -> HalError {
    match error {
        VirtioGpuError::Hal(error) => error,
        VirtioGpuError::QueueBackpressure | VirtioGpuError::QueueTooSmall => HalError::Exhausted,
        VirtioGpuError::DeviceNotVirtioGpu => HalError::InvalidDevice,
        VirtioGpuError::MissingBar
        | VirtioGpuError::MissingCapability
        | VirtioGpuError::FeaturesRejected
        | VirtioGpuError::InvalidQueueState => HalError::InvalidDevice,
        VirtioGpuError::PayloadTooLarge => HalError::Unsupported,
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    if align == 0 {
        value
    } else {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + (align - remainder)
        }
    }
}

fn crosses_boundary(start: u64, len: u64, boundary: u64) -> bool {
    if boundary == 0 || boundary == u64::MAX {
        return false;
    }
    let end = start.saturating_add(len.saturating_sub(1));
    (start / boundary) != (end / boundary)
}

#[derive(Debug, Clone, Default)]
pub struct SyntheticPciConfigBackend {
    registers: BTreeMap<(u16, u8, u8, u8, u16), u32>,
    bar_sizes: BTreeMap<(u16, u8, u8, u8, u16), u32>,
    rom_images: BTreeMap<u64, Vec<u8>>,
}

impl SyntheticPciConfigBackend {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn define_device(
        &mut self,
        address: PciAddress,
        identity: DeviceIdentity,
        subsystem_vendor_id: u16,
        subsystem_device_id: u16,
        multifunction: bool,
        interrupt_line: u8,
        interrupt_pin: u8,
    ) {
        self.set_register(
            address,
            PCI_VENDOR_DEVICE_OFFSET,
            u32::from(identity.vendor_id) | ((u32::from(identity.device_id)) << 16),
        );
        self.set_register(
            address,
            PCI_COMMAND_STATUS_OFFSET,
            u32::from(PCI_COMMAND_IO_SPACE | PCI_COMMAND_MEMORY_SPACE),
        );
        self.set_register(
            address,
            PCI_CLASS_REVISION_OFFSET,
            u32::from(identity.revision_id)
                | ((u32::from(identity.programming_interface)) << 8)
                | ((u32::from(identity.sub_class)) << 16)
                | ((u32::from(identity.base_class)) << 24),
        );
        self.set_register(
            address,
            PCI_HEADER_TYPE_OFFSET,
            if multifunction { 0x0080_0000 } else { 0 },
        );
        self.set_register(
            address,
            0x2c,
            u32::from(subsystem_vendor_id) | ((u32::from(subsystem_device_id)) << 16),
        );
        self.set_register(
            address,
            PCI_INTERRUPT_LINE_OFFSET,
            u32::from(interrupt_line) | ((u32::from(interrupt_pin)) << 8),
        );
    }

    pub fn define_bar(&mut self, address: PciAddress, index: u8, original: u32, size_mask: u32) {
        self.set_register(address, PCI_BAR0_OFFSET + u16::from(index) * 4, original);
        self.bar_sizes.insert(
            (
                address.segment,
                address.bus,
                address.device,
                address.function,
                PCI_BAR0_OFFSET + u16::from(index) * 4,
            ),
            size_mask,
        );
    }

    pub fn define_config_dword(&mut self, address: PciAddress, offset: u16, value: u32) {
        self.set_register(address, offset, value);
    }

    pub fn define_capability(&mut self, address: PciAddress, offset: u16, dword: u32, next: u8) {
        self.set_register(
            address,
            offset,
            (dword & 0xffff_0000) | u32::from((next as u16) << 8) | u32::from(dword as u8),
        );
        let mut command_status = self.register(address, PCI_COMMAND_STATUS_OFFSET);
        command_status |= (PCI_STATUS_CAPABILITIES as u32) << 16;
        self.set_register(address, PCI_COMMAND_STATUS_OFFSET, command_status);
        if (self.register(address, PCI_CAP_PTR_OFFSET) & 0xff) == 0 {
            self.set_byte(address, PCI_CAP_PTR_OFFSET, offset as u8);
        }
    }

    pub fn define_rom(&mut self, physical_base: u64, bytes: &[u8]) {
        self.rom_images.insert(physical_base, bytes.to_vec());
    }

    pub fn register(&self, address: PciAddress, offset: u16) -> u32 {
        self.registers
            .get(&(
                address.segment,
                address.bus,
                address.device,
                address.function,
                offset,
            ))
            .copied()
            .unwrap_or({
                if offset == PCI_VENDOR_DEVICE_OFFSET {
                    u32::MAX
                } else {
                    0
                }
            })
    }

    fn set_register(&mut self, address: PciAddress, offset: u16, value: u32) {
        self.registers.insert(
            (
                address.segment,
                address.bus,
                address.device,
                address.function,
                offset,
            ),
            value,
        );
    }

    fn set_byte(&mut self, address: PciAddress, offset: u16, value: u8) {
        let aligned = offset & !0x3;
        let shift = (offset & 0x3) * 8;
        let mut dword = self.register(address, aligned);
        dword &= !(0xff << shift);
        dword |= u32::from(value) << shift;
        self.set_register(address, aligned, dword);
    }
}

impl PciConfigBackend for SyntheticPciConfigBackend {
    fn read_u32(&mut self, address: PciAddress, offset: u16) -> Result<u32, HalError> {
        Ok(self.register(address, offset))
    }

    fn write_u32(&mut self, address: PciAddress, offset: u16, value: u32) -> Result<(), HalError> {
        if value == u32::MAX
            && let Some(size) = self.bar_sizes.get(&(
                address.segment,
                address.bus,
                address.device,
                address.function,
                offset,
            ))
        {
            self.set_register(address, offset, *size);
            return Ok(());
        }
        self.set_register(address, offset, value);
        Ok(())
    }

    fn read_physical_bytes(&mut self, physical_base: u64, len: usize) -> Result<Vec<u8>, HalError> {
        let image = self
            .rom_images
            .get(&physical_base)
            .ok_or(HalError::Unsupported)?;
        let copy_len = core::cmp::min(len, image.len());
        Ok(image[..copy_len].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_hal::GpuPlatform;

    fn sample_identity(base_class: u8, sub_class: u8, interface: u8) -> DeviceIdentity {
        DeviceIdentity {
            vendor_id: 0x8086,
            device_id: 0x100e,
            subsystem_vendor_id: 0,
            subsystem_device_id: 0,
            revision_id: 1,
            base_class,
            sub_class,
            programming_interface: interface,
        }
    }

    fn sample_platform() -> X86_64DevicePlatform<SyntheticPciConfigBackend> {
        let mut backend = SyntheticPciConfigBackend::new();
        let net = PciAddress {
            segment: 0,
            bus: 0,
            device: 1,
            function: 0,
        };
        backend.define_device(
            net,
            sample_identity(0x02, 0x00, 0x00),
            0x1af4,
            0x1000,
            false,
            11,
            1,
        );
        backend.define_bar(net, 0, 0xfebc_0000, 0xffff_f000);
        backend.define_bar(net, 1, 0x0000_c001, 0xffff_ff01);
        backend.define_capability(
            net,
            0x50,
            (PCI_CAP_ID_MSI as u32) | ((0x0002u32) << 16),
            0x60,
        );
        backend.define_capability(
            net,
            0x60,
            (PCI_CAP_ID_MSIX as u32) | ((0x0003u32) << 16),
            0x00,
        );

        let storage = PciAddress {
            segment: 0,
            bus: 0,
            device: 2,
            function: 0,
        };
        backend.define_device(
            storage,
            sample_identity(0x01, 0x06, 0x01),
            0x1af4,
            0x1001,
            false,
            10,
            1,
        );
        backend.define_bar(storage, 0, 0xfebd_0000, 0xffff_e000);

        X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default())
    }

    fn sample_nvidia_platform() -> X86_64DevicePlatform<SyntheticPciConfigBackend> {
        let mut backend = SyntheticPciConfigBackend::new();
        let gpu = PciAddress {
            segment: 0,
            bus: 0,
            device: 5,
            function: 0,
        };
        backend.define_device(
            gpu,
            DeviceIdentity {
                vendor_id: 0x10de,
                device_id: 0x2d04,
                subsystem_vendor_id: 0x10de,
                subsystem_device_id: 0x0001,
                revision_id: 1,
                base_class: 0x03,
                sub_class: 0x00,
                programming_interface: 0x00,
            },
            0x10de,
            0x0001,
            false,
            9,
            1,
        );
        backend.define_bar(gpu, 0, 0xfec0_0000, 0xffff_f000);
        backend.define_bar(gpu, 1, 0xd000_0000, 0xf000_0000);
        backend.define_capability(gpu, 0x50, 0x0003_0011, 0x00);

        X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default())
    }

    #[test]
    fn pci_enumeration_reports_multiple_devices_bars_and_interrupts() {
        let mut platform = sample_platform();
        let devices = platform.enumerate_devices().unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(
            devices[0].identity.class(),
            platform_hal::DeviceClass::Network
        );
        assert_eq!(devices[0].bars.len(), 2);
        assert!(
            devices[0]
                .interrupts
                .iter()
                .any(|cap| cap.kind == InterruptKind::LegacyLine)
        );
        assert!(
            devices[0]
                .interrupts
                .iter()
                .any(|cap| cap.kind == InterruptKind::Msi)
        );
        assert!(
            devices[0]
                .interrupts
                .iter()
                .any(|cap| cap.kind == InterruptKind::Msix)
        );
    }

    #[test]
    fn mmio_bar_claim_mapping_and_release_are_lifecycle_checked() {
        let mut platform = sample_platform();
        let devices = platform.enumerate_devices().unwrap();
        let bar = devices[0]
            .bars
            .iter()
            .find(|bar| matches!(bar.kind, BarKind::Memory32 | BarKind::Memory64))
            .unwrap();
        let region = platform.claim_bar(devices[0].locator, bar.id).unwrap();
        let mapping = platform
            .map_mmio(
                region,
                MmioPermissions::read_write(),
                MmioCachePolicy::Uncacheable,
            )
            .unwrap();
        assert_eq!(mapping.physical_base, bar.base);
        assert_eq!(mapping.virtual_base, DEFAULT_DIRECT_MAP_BASE + bar.base);
        assert_eq!(platform.release_bar(region), Err(HalError::DmaBusy));
        platform.unmap_mmio(mapping.id).unwrap();
        platform.release_bar(region).unwrap();
    }

    #[test]
    fn interrupt_claim_enable_dispatch_acknowledge_round_trip() {
        let mut platform = sample_platform();
        let devices = platform.enumerate_devices().unwrap();
        let (handle, route) = platform.claim_interrupt(devices[0].locator, 0).unwrap();
        platform.enable_interrupt(handle).unwrap();
        let event = platform
            .dispatch_interrupt_vector(route.vector)
            .unwrap()
            .unwrap();
        assert_eq!(event.handle, handle);
        assert_eq!(platform.pending_interrupts(), &[handle]);
        platform.acknowledge_interrupt(handle).unwrap();
        assert!(platform.pending_interrupts().is_empty());
    }

    #[test]
    fn dma_allocator_enforces_ownership_and_recycles_buffers() {
        let mut platform = sample_platform();
        let buffer = platform
            .allocate_dma(
                0x3000,
                DmaDirection::Bidirectional,
                DmaCoherency::Coherent,
                DmaConstraints::platform_default(),
            )
            .unwrap();
        assert_eq!(buffer.ownership, DmaOwnership::Cpu);
        platform.prepare_dma_for_device(buffer.id).unwrap();
        assert_eq!(
            platform.prepare_dma_for_device(buffer.id),
            Err(HalError::DmaBusy)
        );
        platform.complete_dma_from_device(buffer.id).unwrap();
        platform.release_dma(buffer.id).unwrap();
        let recycled = platform
            .allocate_dma(
                0x3000,
                DmaDirection::ToDevice,
                DmaCoherency::Coherent,
                DmaConstraints::platform_default(),
            )
            .unwrap();
        assert_eq!(buffer.device_address, recycled.device_address);
    }

    #[test]
    fn dma_constraints_limit_addressing_and_alignment() {
        let mut platform = sample_platform();
        let constrained = platform
            .allocate_dma(
                0x1000,
                DmaDirection::FromDevice,
                DmaCoherency::NonCoherent,
                DmaConstraints {
                    alignment: 0x2000,
                    max_address_bits: 29,
                    segment_boundary: u64::MAX,
                    contiguous: true,
                },
            )
            .unwrap();
        assert!(constrained.device_address.is_multiple_of(0x2000));
        assert!(constrained.device_address < (1u64 << 29));
    }

    #[test]
    fn nvidia_gpu_agent_setup_enables_vram_allocations() {
        let mut platform = sample_nvidia_platform();
        let device = platform
            .enumerate_devices()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        platform.setup_gpu_agent(device.locator).unwrap();

        let vram_ptr = platform
            .allocate_gpu_memory(GpuMemoryKind::Vram, 0x4000)
            .unwrap();
        let shared_ptr = platform
            .allocate_gpu_memory(GpuMemoryKind::SystemShared, 0x4000)
            .unwrap();

        assert_eq!(vram_ptr, DEFAULT_DIRECT_MAP_BASE + 0xd000_0000);
        assert!(shared_ptr >= DEFAULT_DIRECT_MAP_BASE);
        assert!(shared_ptr != vram_ptr);
    }

    #[test]
    fn gpu_binding_evidence_reports_confirmed_local_windows_binding_for_rtx_5060_ti() {
        let mut platform = sample_nvidia_platform();
        let device = platform
            .enumerate_devices()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let evidence = platform
            .gpu_binding_evidence(device.locator)
            .unwrap()
            .unwrap();

        assert_eq!(evidence.architecture_name, "Blackwell");
        assert_eq!(evidence.product_name, "NVIDIA GeForce RTX 5060 Ti");
        assert_eq!(evidence.inf_section, "Section048");
        assert_eq!(evidence.kernel_service, "nvlddmkm");
        assert_eq!(evidence.vbios_version, "98.06.1f.00.dc");
        assert_eq!(evidence.part_number, "2D04-300-A1");
        assert_eq!(evidence.subsystem_id, 0x205e_1771);
        assert_eq!(evidence.msi_policy.source_name, "nv_msiSupport_addreg");
        assert!(evidence.msi_policy.supported);
        assert_eq!(evidence.msi_policy.message_limit, 1);
    }

    #[test]
    fn primary_gpu_display_evidence_tracks_planned_frames_after_submit() {
        let mut platform = sample_nvidia_platform();
        let device = platform
            .enumerate_devices()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        platform.setup_gpu_agent(device.locator).unwrap();

        let before = platform.primary_gpu_display_evidence().unwrap().unwrap();
        assert!(before.present);
        assert_eq!(before.active_pipes, 1);
        assert_eq!(before.planned_frames, 0);

        platform
            .submit_gpu_command(0x0100, b"draw:hardware")
            .unwrap();

        let after = platform.primary_gpu_display_evidence().unwrap().unwrap();
        assert_eq!(after.planned_frames, 1);
        assert_eq!(after.last_present_offset, 0);
        assert_eq!(after.last_present_len, 13);
        assert!(!after.hardware_programming_confirmed);
    }

    #[test]
    fn gpu_binding_evidence_returns_none_for_non_nvidia_devices() {
        let mut platform = sample_platform();
        let device = platform
            .enumerate_devices()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        assert!(
            platform
                .gpu_binding_evidence(device.locator)
                .unwrap()
                .is_none()
        );
    }
}
