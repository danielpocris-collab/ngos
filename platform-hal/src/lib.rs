#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    AArch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddressSpaceId(u64);

impl AddressSpaceId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl MemoryPermissions {
    pub const fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            execute: false,
        }
    }

    pub const fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
        }
    }

    pub const fn read_execute() -> Self {
        Self {
            read: true,
            write: false,
            execute: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePolicy {
    WriteBack,
    Uncacheable,
    WriteCombining,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMapping {
    pub vaddr: u64,
    pub paddr: u64,
    pub len: u64,
    pub perms: MemoryPermissions,
    pub cache: CachePolicy,
    pub user: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualRange {
    pub vaddr: u64,
    pub len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressSpaceLayout {
    pub id: AddressSpaceId,
    pub active: bool,
    pub mappings: Vec<PageMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalError {
    Exhausted,
    InvalidAddressSpace,
    InvalidMapping,
    OverlappingMapping,
    MappingNotFound,
    InvalidBus,
    InvalidDevice,
    InvalidConfigAccess,
    InvalidBar,
    BarAlreadyClaimed,
    BarNotClaimed,
    InvalidMmioMapping,
    InvalidInterrupt,
    InterruptAlreadyClaimed,
    InterruptNotClaimed,
    InvalidDmaBuffer,
    DmaBusy,
    Unsupported,
}

pub trait Platform {
    fn name(&self) -> &'static str;
    fn architecture(&self) -> Architecture;
    fn supports_host_runtime_mode(&self) -> bool;
}

pub trait AddressSpaceManager {
    fn create_address_space(&mut self) -> Result<AddressSpaceId, HalError>;
    fn destroy_address_space(&mut self, id: AddressSpaceId) -> Result<(), HalError>;
    fn map(&mut self, id: AddressSpaceId, mapping: PageMapping) -> Result<(), HalError>;
    fn unmap(&mut self, id: AddressSpaceId, range: VirtualRange) -> Result<(), HalError>;
    fn protect(
        &mut self,
        id: AddressSpaceId,
        range: VirtualRange,
        perms: MemoryPermissions,
    ) -> Result<(), HalError>;
    fn activate_address_space(&mut self, id: AddressSpaceId) -> Result<(), HalError>;
    fn active_address_space(&self) -> Option<AddressSpaceId>;
    fn address_space_layout(&self, id: AddressSpaceId) -> Result<AddressSpaceLayout, HalError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformDescriptor {
    pub name: &'static str,
    pub architecture: Architecture,
    pub host_runtime_mode: bool,
}

impl PlatformDescriptor {
    pub fn describe(&self) -> String {
        format!(
            "{} {:?} host-runtime={}",
            self.name, self.architecture, self.host_runtime_mode
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusKind {
    Pci,
    PlatformMmio,
    Virtual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Network,
    Storage,
    Display,
    Bridge,
    Input,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceLocator(u64);

impl DeviceLocator {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciAddress {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusAddress {
    Pci(PciAddress),
    PlatformMmio { base: u64, span: u64 },
    Virtual(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub vendor_id: u16,
    pub device_id: u16,
    pub subsystem_vendor_id: u16,
    pub subsystem_device_id: u16,
    pub revision_id: u8,
    pub base_class: u8,
    pub sub_class: u8,
    pub programming_interface: u8,
}

impl DeviceIdentity {
    pub const fn class(self) -> DeviceClass {
        match (self.base_class, self.sub_class) {
            (0x02, _) => DeviceClass::Network,
            (0x01, _) => DeviceClass::Storage,
            (0x03, _) => DeviceClass::Display,
            (0x06, _) => DeviceClass::Bridge,
            (0x09, _) | (0x0c, 0x03) => DeviceClass::Input,
            _ => DeviceClass::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSpaceKind {
    Pci,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigWidth {
    U8,
    U16,
    U32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigAccess {
    pub kind: ConfigSpaceKind,
    pub offset: u16,
    pub width: ConfigWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarKind {
    Memory32,
    Memory64,
    IoPort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarFlags {
    pub prefetchable: bool,
    pub cacheable: bool,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BarId(u64);

impl BarId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarInfo {
    pub id: BarId,
    pub kind: BarKind,
    pub base: u64,
    pub size: u64,
    pub flags: BarFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MmioRegionId(u64);

impl MmioRegionId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmioPermissions {
    pub read: bool,
    pub write: bool,
}

impl MmioPermissions {
    pub const fn read_write() -> Self {
        Self {
            read: true,
            write: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioCachePolicy {
    Uncacheable,
    WriteCombining,
    WriteBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MmioMappingId(u64);

impl MmioMappingId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmioMapping {
    pub id: MmioMappingId,
    pub region: MmioRegionId,
    pub physical_base: u64,
    pub virtual_base: u64,
    pub len: u64,
    pub perms: MmioPermissions,
    pub cache_policy: MmioCachePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptKind {
    LegacyLine,
    Msi,
    Msix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptTrigger {
    Edge,
    Level,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptPolarity {
    High,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptCapability {
    pub kind: InterruptKind,
    pub vectors: u16,
    pub line: Option<u8>,
    pub trigger: InterruptTrigger,
    pub polarity: InterruptPolarity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterruptHandle(u64);

impl InterruptHandle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptRoute {
    pub kind: InterruptKind,
    pub vector: u8,
    pub line: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    ToDevice,
    FromDevice,
    Bidirectional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaCoherency {
    Coherent,
    NonCoherent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaConstraints {
    pub alignment: u64,
    pub max_address_bits: u8,
    pub segment_boundary: u64,
    pub contiguous: bool,
}

impl DmaConstraints {
    pub const fn platform_default() -> Self {
        Self {
            alignment: 64,
            max_address_bits: 64,
            segment_boundary: u64::MAX,
            contiguous: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DmaBufferId(u64);

impl DmaBufferId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaOwnership {
    Cpu,
    Device,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaBufferInfo {
    pub id: DmaBufferId,
    pub cpu_virtual: u64,
    pub device_address: u64,
    pub len: u64,
    pub direction: DmaDirection,
    pub coherency: DmaCoherency,
    pub ownership: DmaOwnership,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRecord {
    pub locator: DeviceLocator,
    pub bus_kind: BusKind,
    pub address: BusAddress,
    pub identity: DeviceIdentity,
    pub bars: Vec<BarInfo>,
    pub interrupts: Vec<InterruptCapability>,
}

pub trait DevicePlatform: Platform {
    fn enumerate_devices(&mut self) -> Result<Vec<DeviceRecord>, HalError>;
    fn read_config_u32(
        &mut self,
        device: DeviceLocator,
        access: ConfigAccess,
    ) -> Result<u32, HalError>;
    fn write_config_u32(
        &mut self,
        device: DeviceLocator,
        access: ConfigAccess,
        value: u32,
    ) -> Result<(), HalError>;

    fn claim_bar(&mut self, device: DeviceLocator, bar: BarId) -> Result<MmioRegionId, HalError>;
    fn release_bar(&mut self, region: MmioRegionId) -> Result<(), HalError>;
    fn map_mmio(
        &mut self,
        region: MmioRegionId,
        perms: MmioPermissions,
        cache_policy: MmioCachePolicy,
    ) -> Result<MmioMapping, HalError>;
    fn unmap_mmio(&mut self, mapping: MmioMappingId) -> Result<(), HalError>;

    fn claim_interrupt(
        &mut self,
        device: DeviceLocator,
        capability_index: usize,
    ) -> Result<(InterruptHandle, InterruptRoute), HalError>;
    fn enable_interrupt(&mut self, handle: InterruptHandle) -> Result<(), HalError>;
    fn disable_interrupt(&mut self, handle: InterruptHandle) -> Result<(), HalError>;
    fn acknowledge_interrupt(&mut self, handle: InterruptHandle) -> Result<(), HalError>;

    fn allocate_dma(
        &mut self,
        len: u64,
        direction: DmaDirection,
        coherency: DmaCoherency,
        constraints: DmaConstraints,
    ) -> Result<DmaBufferInfo, HalError>;
    fn prepare_dma_for_device(&mut self, buffer: DmaBufferId) -> Result<(), HalError>;
    fn complete_dma_from_device(&mut self, buffer: DmaBufferId) -> Result<(), HalError>;
    fn release_dma(&mut self, buffer: DmaBufferId) -> Result<(), HalError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMemoryKind {
    Vram,
    SystemShared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMsiPolicyEvidence {
    pub source_name: &'static str,
    pub supported: bool,
    pub message_limit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBindingEvidence {
    pub architecture_name: &'static str,
    pub product_name: &'static str,
    pub die_name: &'static str,
    pub bus_interface: &'static str,
    pub inf_section: &'static str,
    pub kernel_service: &'static str,
    pub vbios_version: &'static str,
    pub part_number: &'static str,
    pub subsystem_id: u32,
    pub bar1_total_mib: u32,
    pub framebuffer_total_mib: u32,
    pub resizable_bar_enabled: bool,
    pub display_engine_confirmed: bool,
    pub msi_policy: GpuMsiPolicyEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuVbiosWindowEvidence {
    pub rom_bar_raw: u32,
    pub physical_base: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuVbiosImageEvidence {
    pub image_len: usize,
    pub pcir_offset: usize,
    pub bit_offset: Option<usize>,
    pub nvfw_offset: Option<usize>,
    pub vendor_id: u16,
    pub device_id: u16,
    pub board_name: alloc::string::String,
    pub board_code: alloc::string::String,
    pub version: alloc::string::String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuGspEvidence {
    pub present: bool,
    pub loopback_ready: bool,
    pub firmware_known: bool,
    pub blackwell_blob_present: bool,
    pub hardware_ready: bool,
    pub driver_model_wddm: bool,
    pub loopback_completions: u64,
    pub loopback_failures: u64,
    pub firmware_version: &'static str,
    pub blob_summary: &'static str,
    pub refusal_reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuInterruptEvidence {
    pub present: bool,
    pub vector: u8,
    pub delivered_count: u64,
    pub msi_supported: bool,
    pub message_limit: u32,
    pub windows_interrupt_message_maximum: u32,
    pub hardware_servicing_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuDisplayEvidence {
    pub present: bool,
    pub active_pipes: u32,
    pub planned_frames: u64,
    pub last_present_offset: u64,
    pub last_present_len: u64,
    pub hardware_programming_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPowerEvidence {
    pub present: bool,
    pub pstate: u32,
    pub graphics_clock_mhz: u32,
    pub memory_clock_mhz: u32,
    pub boost_clock_mhz: u32,
    pub hardware_power_management_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMediaEvidence {
    pub present: bool,
    pub sessions: u32,
    pub codec: u32,
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
    pub hardware_media_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuNeuralEvidence {
    pub present: bool,
    pub model_loaded: bool,
    pub active_semantics: u32,
    pub last_commit_completed: bool,
    pub hardware_neural_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuTensorEvidence {
    pub present: bool,
    pub active_jobs: u32,
    pub last_kernel_id: u32,
    pub hardware_tensor_confirmed: bool,
}

pub trait GpuPlatform: DevicePlatform {
    fn setup_gpu_agent(&mut self, device: DeviceLocator) -> Result<(), HalError>;
    fn submit_gpu_command(&mut self, rpc_id: u32, payload: &[u8]) -> Result<Vec<u8>, HalError>;
    fn allocate_gpu_memory(&mut self, kind: GpuMemoryKind, size: u64) -> Result<u64, HalError>;
    fn set_primary_gpu_power_state(&mut self, _pstate: u32) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }
    fn start_primary_gpu_media_session(
        &mut self,
        _width: u32,
        _height: u32,
        _bitrate_kbps: u32,
        _codec: u32,
    ) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }
    fn inject_primary_gpu_neural_semantic(
        &mut self,
        _semantic_label: &str,
    ) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }
    fn commit_primary_gpu_neural_frame(&mut self) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }
    fn dispatch_primary_gpu_tensor_kernel(&mut self, _kernel_id: u32) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }
    fn gpu_binding_evidence(
        &mut self,
        device: DeviceLocator,
    ) -> Result<Option<GpuBindingEvidence>, HalError> {
        let _ = device;
        Ok(None)
    }
    fn primary_gpu_binding_evidence(&mut self) -> Result<Option<GpuBindingEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_vbios_window(&mut self) -> Result<Option<GpuVbiosWindowEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_vbios_bytes(&mut self, _max_len: usize) -> Result<Vec<u8>, HalError> {
        Err(HalError::Unsupported)
    }
    fn primary_gpu_vbios_image_evidence(
        &mut self,
    ) -> Result<Option<GpuVbiosImageEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_gsp_evidence(&mut self) -> Result<Option<GpuGspEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_interrupt_evidence(&mut self) -> Result<Option<GpuInterruptEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_display_evidence(&mut self) -> Result<Option<GpuDisplayEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_power_evidence(&mut self) -> Result<Option<GpuPowerEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_media_evidence(&mut self) -> Result<Option<GpuMediaEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_neural_evidence(&mut self) -> Result<Option<GpuNeuralEvidence>, HalError> {
        Ok(None)
    }
    fn primary_gpu_tensor_evidence(&mut self) -> Result<Option<GpuTensorEvidence>, HalError> {
        Ok(None)
    }
}

pub trait FirmwareReadablePlatform: DevicePlatform {
    fn read_device_rom(
        &mut self,
        device: DeviceLocator,
        physical_base: u64,
        len: usize,
    ) -> Result<Vec<u8>, HalError>;
}
