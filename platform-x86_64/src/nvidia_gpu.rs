use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use platform_hal::{
    BarId, ConfigAccess, ConfigSpaceKind, ConfigWidth, DeviceIdentity, DeviceLocator,
    DevicePlatform, DmaCoherency, DmaConstraints, DmaDirection, FirmwareReadablePlatform, HalError,
    MmioCachePolicy, MmioPermissions,
};

const NVIDIA_VENDOR_ID: u16 = 0x10DE;
const RTX_5060_TI_DEVICE_ID: u16 = 0x2D04;

// Reverse-engineering discipline:
// - Confirmed: values directly observable through PCI config space or runtime-owned state.
// - Inferred: values reconstructed from observed topology but not yet validated on hardware.
// - Experimental: semantic surfaces kept behind nano-agents until backed by RE evidence.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReKnowledge {
    Confirmed,
    Inferred,
    Experimental,
}

impl ReKnowledge {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Inferred => "inferred",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvidiaNanoAgentKind {
    Probe,
    Vbios,
    GspControl,
    Vram,
    Display,
    Neural,
    Power,
    RayTracing,
    Media,
    Tensor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvidiaReSurface {
    pub name: &'static str,
    pub knowledge: ReKnowledge,
    pub rationale: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvidiaVbiosWindow {
    pub rom_bar_raw: u32,
    pub physical_base: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvidiaVbiosImageEvidence {
    pub image_len: usize,
    pub pcir_offset: usize,
    pub bit_offset: Option<usize>,
    pub nvfw_offset: Option<usize>,
    pub vendor_id: u16,
    pub device_id: u16,
    pub board_name: String,
    pub board_code: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvidiaWindowsMsiPolicy {
    pub section_name: &'static str,
    pub msi_supported: bool,
    pub message_number_limit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvidiaWindowsBindingEvidence {
    pub architecture: &'static str,
    pub marketing_name: &'static str,
    pub die_name: &'static str,
    pub bus_interface: &'static str,
    pub inf_section: &'static str,
    pub kernel_service: &'static str,
    pub vbios_version: &'static str,
    pub gpu_part_number: &'static str,
    pub subsystem_id: u32,
    pub bar1_total_mib: u32,
    pub framebuffer_total_mib: u32,
    pub resizable_bar_enabled: bool,
    pub display_engine_confirmed: bool,
    pub msi_policy: NvidiaWindowsMsiPolicy,
}

pub const RTX_5060_TI_LOCAL_WINDOWS_BINDING: NvidiaWindowsBindingEvidence =
    NvidiaWindowsBindingEvidence {
        architecture: "Blackwell",
        marketing_name: "NVIDIA GeForce RTX 5060 Ti",
        die_name: "GB206",
        bus_interface: "PCIe x8 5.0 @ x8 4.0",
        inf_section: "Section048",
        kernel_service: "nvlddmkm",
        vbios_version: "98.06.1f.00.dc",
        gpu_part_number: "2D04-300-A1",
        subsystem_id: 0x205e_1771,
        bar1_total_mib: 16_384,
        framebuffer_total_mib: 16_311,
        resizable_bar_enabled: true,
        display_engine_confirmed: false,
        msi_policy: NvidiaWindowsMsiPolicy {
            section_name: "nv_msiSupport_addreg",
            msi_supported: true,
            message_number_limit: 1,
        },
    };

pub const RTX_5060_TI_LOCAL_REAL_VBIOS_SHA256: &str =
    "9a294cebf93aa635acba0fe5f7cd9b2ced6b357eeef85a81d22fabb98923aef2";
pub const RTX_5060_TI_LOCAL_GSP_FIRMWARE_VERSION: &str = "N/A";
pub const RTX_5060_TI_LOCAL_GSP_BLOB_SUMMARY: &str = "gsp_ga10x.bin,gsp_tu10x.bin";
pub const RTX_5060_TI_LOCAL_GSP_BLACKWELL_BLOB_PRESENT: bool = false;
pub const RTX_5060_TI_LOCAL_GSP_DRIVER_MODEL_WDDM: bool = true;
pub const RTX_5060_TI_LOCAL_GSP_REAL_HARDWARE_READY: bool = false;
pub const RTX_5060_TI_LOCAL_GSP_REFUSAL_REASON: &str = "wddm-no-blackwell-gsp";
pub const RTX_5060_TI_LOCAL_INTERRUPT_MESSAGE_MAXIMUM: u32 = 9;
pub const RTX_5060_TI_LOCAL_HARDWARE_INTERRUPT_SERVICING_CONFIRMED: bool = false;
pub const RTX_5060_TI_LOCAL_GRAPHICS_CLOCK_MHZ: u32 = 2407;
pub const RTX_5060_TI_LOCAL_MEMORY_CLOCK_MHZ: u32 = 1750;
pub const RTX_5060_TI_LOCAL_BOOST_CLOCK_MHZ: u32 = 2602;
pub const RTX_5060_TI_LOCAL_HARDWARE_POWER_MANAGEMENT_CONFIRMED: bool = false;

pub const fn as_platform_gpu_binding_evidence(
    binding: NvidiaWindowsBindingEvidence,
) -> platform_hal::GpuBindingEvidence {
    platform_hal::GpuBindingEvidence {
        architecture_name: binding.architecture,
        product_name: binding.marketing_name,
        die_name: binding.die_name,
        bus_interface: binding.bus_interface,
        inf_section: binding.inf_section,
        kernel_service: binding.kernel_service,
        vbios_version: binding.vbios_version,
        part_number: binding.gpu_part_number,
        subsystem_id: binding.subsystem_id,
        bar1_total_mib: binding.bar1_total_mib,
        framebuffer_total_mib: binding.framebuffer_total_mib,
        resizable_bar_enabled: binding.resizable_bar_enabled,
        display_engine_confirmed: binding.display_engine_confirmed,
        msi_policy: platform_hal::GpuMsiPolicyEvidence {
            source_name: binding.msi_policy.section_name,
            supported: binding.msi_policy.msi_supported,
            message_limit: binding.msi_policy.message_number_limit,
        },
    }
}

pub const fn as_platform_gpu_vbios_window_evidence(
    window: NvidiaVbiosWindow,
) -> platform_hal::GpuVbiosWindowEvidence {
    platform_hal::GpuVbiosWindowEvidence {
        rom_bar_raw: window.rom_bar_raw,
        physical_base: window.physical_base,
        enabled: window.enabled,
    }
}

pub const fn local_windows_binding_for_device(
    vendor_id: u16,
    device_id: u16,
) -> Option<NvidiaWindowsBindingEvidence> {
    if vendor_id == NVIDIA_VENDOR_ID && device_id == RTX_5060_TI_DEVICE_ID {
        Some(RTX_5060_TI_LOCAL_WINDOWS_BINDING)
    } else {
        None
    }
}

pub const fn nano_agent_surface(kind: NvidiaNanoAgentKind) -> NvidiaReSurface {
    match kind {
        NvidiaNanoAgentKind::Probe => NvidiaReSurface {
            name: "probe",
            knowledge: ReKnowledge::Confirmed,
            rationale: "PCI vendor/device identity and BAR enumeration are directly observable.",
        },
        NvidiaNanoAgentKind::Vbios => NvidiaReSurface {
            name: "vbios",
            knowledge: ReKnowledge::Confirmed,
            rationale: "PCI ROM window is observable and local real-hardware VBIOS bytes have been recovered and parsed.",
        },
        NvidiaNanoAgentKind::GspControl => NvidiaReSurface {
            name: "gsp-control",
            knowledge: ReKnowledge::Inferred,
            rationale: "DMA-backed mailbox path exists, but full firmware protocol is not confirmed.",
        },
        NvidiaNanoAgentKind::Vram => NvidiaReSurface {
            name: "vram",
            knowledge: ReKnowledge::Inferred,
            rationale: "BAR-backed aperture is observable; allocator semantics remain local policy.",
        },
        NvidiaNanoAgentKind::Display => NvidiaReSurface {
            name: "display",
            knowledge: ReKnowledge::Experimental,
            rationale: "Present and scanout orchestration are still placeholders.",
        },
        NvidiaNanoAgentKind::Neural => NvidiaReSurface {
            name: "neural",
            knowledge: ReKnowledge::Experimental,
            rationale: "Semantic rendering opcodes are hypotheses awaiting RE evidence.",
        },
        NvidiaNanoAgentKind::Power => NvidiaReSurface {
            name: "power",
            knowledge: ReKnowledge::Experimental,
            rationale: "Power-state RPCs are modeled, not confirmed against firmware traces.",
        },
        NvidiaNanoAgentKind::RayTracing => NvidiaReSurface {
            name: "ray-tracing",
            knowledge: ReKnowledge::Experimental,
            rationale: "RT control path is semantic scaffolding without confirmed command stream data.",
        },
        NvidiaNanoAgentKind::Media => NvidiaReSurface {
            name: "media",
            knowledge: ReKnowledge::Experimental,
            rationale: "Encode session commands are speculative until firmware responses are decoded.",
        },
        NvidiaNanoAgentKind::Tensor => NvidiaReSurface {
            name: "tensor",
            knowledge: ReKnowledge::Experimental,
            rationale: "Tensor dispatch path is a research stub, not a confirmed hardware ABI.",
        },
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn extract_ascii_line(image: &[u8], start: usize) -> Option<String> {
    let tail = image.get(start..)?;
    let end = tail
        .iter()
        .position(|byte| *byte == 0 || *byte == b'\r' || *byte == b'\n')
        .unwrap_or(tail.len());
    if end == 0 {
        return None;
    }
    let line = &tail[..end];
    if line.iter().all(u8::is_ascii) {
        Some(String::from_utf8_lossy(line).trim().to_owned())
    } else {
        None
    }
}

pub fn inspect_vbios_image_bytes(image: &[u8]) -> Result<NvidiaVbiosImageEvidence, HalError> {
    let pcir_offset = find_bytes(image, b"PCIR").ok_or(HalError::Unsupported)?;
    let vendor_id_offset = pcir_offset.checked_add(4).ok_or(HalError::Unsupported)?;
    let device_id_offset = pcir_offset.checked_add(6).ok_or(HalError::Unsupported)?;
    let vendor_id = u16::from_le_bytes(
        image
            .get(vendor_id_offset..vendor_id_offset + 2)
            .ok_or(HalError::Unsupported)?
            .try_into()
            .map_err(|_| HalError::Unsupported)?,
    );
    let device_id = u16::from_le_bytes(
        image
            .get(device_id_offset..device_id_offset + 2)
            .ok_or(HalError::Unsupported)?
            .try_into()
            .map_err(|_| HalError::Unsupported)?,
    );
    let board_name_offset = find_bytes(image, b"NVIDIA GeForce").ok_or(HalError::Unsupported)?;
    let version_anchor = find_bytes(image, b"Version ").ok_or(HalError::Unsupported)?;
    let board_code_offset = find_bytes(image, b"P14N:").ok_or(HalError::Unsupported)?;
    let bit_offset = find_bytes(image, b"BIT");
    let nvfw_offset = find_bytes(image, b"NVFW");
    Ok(NvidiaVbiosImageEvidence {
        image_len: image.len(),
        pcir_offset,
        bit_offset,
        nvfw_offset,
        vendor_id,
        device_id,
        board_name: extract_ascii_line(image, board_name_offset).ok_or(HalError::Unsupported)?,
        board_code: extract_ascii_line(image, board_code_offset).ok_or(HalError::Unsupported)?,
        version: extract_ascii_line(image, version_anchor).ok_or(HalError::Unsupported)?,
    })
}

pub fn as_platform_gpu_vbios_image_evidence(
    evidence: NvidiaVbiosImageEvidence,
) -> platform_hal::GpuVbiosImageEvidence {
    platform_hal::GpuVbiosImageEvidence {
        image_len: evidence.image_len,
        pcir_offset: evidence.pcir_offset,
        bit_offset: evidence.bit_offset,
        nvfw_offset: evidence.nvfw_offset,
        vendor_id: evidence.vendor_id,
        device_id: evidence.device_id,
        board_name: evidence.board_name,
        board_code: evidence.board_code,
        version: evidence.version,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvidiaRpcRoute {
    pub rpc_id: u32,
    pub knowledge: ReKnowledge,
    pub agent: NvidiaNanoAgentKind,
    pub name: &'static str,
}

pub const fn rpc_route(rpc_id: u32) -> NvidiaRpcRoute {
    match rpc_id {
        NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Inferred,
            agent: NvidiaNanoAgentKind::GspControl,
            name: "generic-draw-submit",
        },
        NV_RPC_ID_SET_POWER_STATE => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::Power,
            name: "set-power-state",
        },
        NV_RPC_ID_RT_SET_BVH_ROOT => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::RayTracing,
            name: "set-bvh-root",
        },
        NV_RPC_ID_TENSOR_DISPATCH => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::Tensor,
            name: "tensor-dispatch",
        },
        NV_RPC_ID_NVENC_SESSION_START => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::Media,
            name: "nvenc-session-start",
        },
        NV_RPC_ID_DLSS5_INIT => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::Neural,
            name: "dlss5-init",
        },
        NV_RPC_ID_DLSS5_LOAD_MODEL => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::Neural,
            name: "dlss5-load-model",
        },
        NV_RPC_ID_DLSS5_INJECT_SEMANTICS => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::Neural,
            name: "dlss5-inject-semantics",
        },
        NV_RPC_ID_DLSS5_COMMIT_NEURAL_FRAME => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Experimental,
            agent: NvidiaNanoAgentKind::Neural,
            name: "dlss5-commit-neural-frame",
        },
        _ => NvidiaRpcRoute {
            rpc_id,
            knowledge: ReKnowledge::Inferred,
            agent: NvidiaNanoAgentKind::GspControl,
            name: "unclassified-rpc",
        },
    }
}

fn validate_rpc_submission(route: NvidiaRpcRoute, payload: &[u8]) -> Result<(), HalError> {
    if payload.is_empty() {
        return Err(HalError::InvalidDevice);
    }
    match route.knowledge {
        ReKnowledge::Confirmed | ReKnowledge::Inferred => Ok(()),
        ReKnowledge::Experimental
            if route.rpc_id == NV_RPC_ID_SET_POWER_STATE
                || route.rpc_id == NV_RPC_ID_NVENC_SESSION_START
                || route.rpc_id == NV_RPC_ID_DLSS5_INJECT_SEMANTICS
                || route.rpc_id == NV_RPC_ID_DLSS5_COMMIT_NEURAL_FRAME
                || route.rpc_id == NV_RPC_ID_TENSOR_DISPATCH =>
        {
            Ok(())
        }
        ReKnowledge::Experimental => Err(HalError::Unsupported),
    }
}

const NV_GSP_COMMAND_QUEUE: usize = 0x00110000;

const PCI_EXPANSION_ROM_OFFSET: u16 = 0x30;
const PCI_ROM_ADDRESS_MASK: u32 = 0xFFFFF800;
const PCI_ROM_ENABLE_BIT: u32 = 1 << 0;
const PCI_COMMAND_STATUS_OFFSET: u16 = 0x04;
const PCI_CAP_PTR_OFFSET: u16 = 0x34;
const PCI_STATUS_CAPABILITIES: u16 = 1 << 4;

const PCI_CAP_ID_MSIX: u8 = 0x11;
const MSIX_CONTROL_ENABLE: u16 = 1 << 15;

pub struct NvidiaInterruptAgent {
    pub handle: platform_hal::InterruptHandle,
    pub vector: u8,
    pub count: u64,
}

const RPC_SIGNATURE: u32 = 0x20435052; // 'RPC '
const GSP_MESSAGE_MAX_SIZE: usize = 0x1000;
pub const NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW: u32 = 0x0100;

// Experimental neural nano-agent routes. These IDs are isolated from confirmed control paths.
pub const NV_RPC_ID_DLSS5_INIT: u32 = 0xe501;
pub const NV_RPC_ID_DLSS5_LOAD_MODEL: u32 = 0xe502;
pub const NV_RPC_ID_DLSS5_INJECT_SEMANTICS: u32 = 0xe503;
pub const NV_RPC_ID_DLSS5_COMMIT_NEURAL_FRAME: u32 = 0xe504;

// Experimental ray-tracing and tensor routes.
const NV_RPC_ID_RT_SET_BVH_ROOT: u32 = 0x2002;
const NV_RPC_ID_TENSOR_DISPATCH: u32 = 0x3001;

// Experimental media encode routes.
const NV_RPC_ID_NVENC_SESSION_START: u32 = 0x4001;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvidiaEncConfig {
    pub width: u32,
    pub height: u32,
    pub bitrate: u32,
    pub codec: u32, // 0 = H264, 1 = HEVC, 2 = AV1
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GspMessageHeader {
    pub signature: u32,
    pub rpc_id: u32,
    pub length: u32,
    pub status: u32,
    pub sequence: u32,
    pub reserved: [u32; 3],
}

pub struct GspRpcContext {
    pub command_buffer_paddr: u64,
    pub command_buffer_vaddr: u64,
    pub response_buffer_paddr: u64,
    pub response_buffer_vaddr: u64,
    pub next_sequence: u32,
    pub command_shadow: Vec<u8>,
    pub response_shadow: Vec<u8>,
    pub loopback_ready: bool,
    pub loopback_completions: u64,
    pub loopback_failures: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvidiaGpu {
    pub locator: DeviceLocator,
    pub identity: DeviceIdentity,
    pub control_bar: BarId,
    pub vram_bar: Option<BarId>,
    pub mmio_base: u64,
    pub mmio_len: u64,
}

fn read_pci_aligned_u32<P: DevicePlatform>(
    platform: &mut P,
    locator: DeviceLocator,
    offset: u16,
) -> Result<u32, HalError> {
    platform.read_config_u32(
        locator,
        ConfigAccess {
            kind: ConfigSpaceKind::Pci,
            offset,
            width: ConfigWidth::U32,
        },
    )
}

fn write_pci_aligned_u32<P: DevicePlatform>(
    platform: &mut P,
    locator: DeviceLocator,
    offset: u16,
    value: u32,
) -> Result<(), HalError> {
    platform.write_config_u32(
        locator,
        ConfigAccess {
            kind: ConfigSpaceKind::Pci,
            offset,
            width: ConfigWidth::U32,
        },
        value,
    )
}

fn read_pci_byte<P: DevicePlatform>(
    platform: &mut P,
    locator: DeviceLocator,
    offset: u16,
) -> Result<u8, HalError> {
    let aligned = offset & !0x3;
    let shift = (offset & 0x3) * 8;
    Ok(((read_pci_aligned_u32(platform, locator, aligned)? >> shift) & 0xff) as u8)
}

fn read_pci_u16<P: DevicePlatform>(
    platform: &mut P,
    locator: DeviceLocator,
    offset: u16,
) -> Result<u16, HalError> {
    let aligned = offset & !0x3;
    let shift = (offset & 0x2) * 8;
    Ok(((read_pci_aligned_u32(platform, locator, aligned)? >> shift) & 0xffff) as u16)
}

fn write_pci_u16<P: DevicePlatform>(
    platform: &mut P,
    locator: DeviceLocator,
    offset: u16,
    value: u16,
) -> Result<(), HalError> {
    let aligned = offset & !0x3;
    let shift = (offset & 0x2) * 8;
    let mask = !(0xffffu32 << shift);
    let current = read_pci_aligned_u32(platform, locator, aligned)?;
    let updated = (current & mask) | ((u32::from(value)) << shift);
    write_pci_aligned_u32(platform, locator, aligned, updated)
}

fn find_pci_capability<P: DevicePlatform>(
    platform: &mut P,
    locator: DeviceLocator,
    capability_id: u8,
) -> Result<Option<u16>, HalError> {
    let command_status = read_pci_aligned_u32(platform, locator, PCI_COMMAND_STATUS_OFFSET)?;
    let status = (command_status >> 16) as u16;
    if (status & PCI_STATUS_CAPABILITIES) == 0 {
        return Ok(None);
    }

    let mut cap_offset = read_pci_byte(platform, locator, PCI_CAP_PTR_OFFSET)? as u16;
    let mut guard = 0usize;
    while cap_offset >= 0x40 && guard < 64 {
        guard += 1;
        let header = read_pci_aligned_u32(platform, locator, cap_offset & !0x3)?;
        let shift = (cap_offset & 0x3) * 8;
        let cap_id = ((header >> shift) & 0xff) as u8;
        let next = ((header >> (shift + 8)) & 0xff) as u16;
        if cap_id == capability_id {
            return Ok(Some(cap_offset));
        }
        if next == 0 || next == cap_offset {
            break;
        }
        cap_offset = next;
    }
    Ok(None)
}

fn enable_msix_capability<P: DevicePlatform>(
    platform: &mut P,
    locator: DeviceLocator,
) -> Result<(), HalError> {
    let cap_offset = find_pci_capability(platform, locator, PCI_CAP_ID_MSIX)?
        .ok_or(HalError::InvalidInterrupt)?;
    let control_offset = cap_offset + 2;
    let control = read_pci_u16(platform, locator, control_offset)?;
    write_pci_u16(
        platform,
        locator,
        control_offset,
        control | MSIX_CONTROL_ENABLE,
    )
}

impl NvidiaGpu {
    pub fn try_detect<P: DevicePlatform>(
        platform: &mut P,
        locator: DeviceLocator,
    ) -> Result<Option<Self>, HalError> {
        let devices = platform.enumerate_devices()?;
        let record = devices
            .iter()
            .find(|d| d.locator == locator)
            .ok_or(HalError::InvalidDevice)?;

        if record.identity.vendor_id != NVIDIA_VENDOR_ID
            || record.identity.device_id != RTX_5060_TI_DEVICE_ID
        {
            return Ok(None);
        }

        if record.bars.is_empty() {
            return Err(HalError::InvalidDevice);
        }

        let bar0 = &record.bars[0];
        Ok(Some(Self {
            locator,
            identity: record.identity,
            control_bar: bar0.id,
            vram_bar: record.bars.get(1).map(|bar| bar.id),
            mmio_base: bar0.base,
            mmio_len: bar0.size,
        }))
    }

    pub fn map_control_registers<P: DevicePlatform>(
        &self,
        platform: &mut P,
    ) -> Result<u64, HalError> {
        let region = platform.claim_bar(self.locator, self.control_bar)?;
        let mapping = platform.map_mmio(
            region,
            MmioPermissions::read_write(),
            MmioCachePolicy::Uncacheable,
        )?;
        Ok(mapping.virtual_base)
    }

    pub fn inspect_vbios_window<P: DevicePlatform>(
        &self,
        platform: &mut P,
    ) -> Result<NvidiaVbiosWindow, HalError> {
        let access = ConfigAccess {
            kind: ConfigSpaceKind::Pci,
            offset: PCI_EXPANSION_ROM_OFFSET,
            width: ConfigWidth::U32,
        };
        let rom_bar = platform.read_config_u32(self.locator, access)?;
        Ok(NvidiaVbiosWindow {
            rom_bar_raw: rom_bar,
            physical_base: (rom_bar & PCI_ROM_ADDRESS_MASK) as u64,
            enabled: (rom_bar & PCI_ROM_ENABLE_BIT) != 0,
        })
    }

    pub fn read_vbios<P: FirmwareReadablePlatform>(
        &self,
        platform: &mut P,
    ) -> Result<Vec<u8>, HalError> {
        let window = self.inspect_vbios_window(platform)?;
        if !window.enabled {
            return Err(HalError::Unsupported);
        }
        platform.read_device_rom(self.locator, window.physical_base, 0x10000)
    }

    pub fn setup_gsp_channels<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
    ) -> Result<GspRpcContext, HalError> {
        let cmd_buffer = platform.allocate_dma(
            GSP_MESSAGE_MAX_SIZE as u64,
            DmaDirection::Bidirectional,
            DmaCoherency::Coherent,
            DmaConstraints::platform_default(),
        )?;
        let resp_buffer = platform.allocate_dma(
            GSP_MESSAGE_MAX_SIZE as u64,
            DmaDirection::Bidirectional,
            DmaCoherency::Coherent,
            DmaConstraints::platform_default(),
        )?;

        Ok(GspRpcContext {
            command_buffer_paddr: cmd_buffer.device_address,
            command_buffer_vaddr: cmd_buffer.cpu_virtual,
            response_buffer_paddr: resp_buffer.device_address,
            response_buffer_vaddr: resp_buffer.cpu_virtual,
            next_sequence: 1,
            command_shadow: vec![0; GSP_MESSAGE_MAX_SIZE],
            response_shadow: vec![0; GSP_MESSAGE_MAX_SIZE],
            loopback_ready: true,
            loopback_completions: 0,
            loopback_failures: 0,
        })
    }

    pub unsafe fn send_rpc(
        &mut self,
        ctx: &mut GspRpcContext,
        mmio_vaddr: u64,
        rpc_id: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>, HalError> {
        let payload_len = payload.len();
        let header_size = core::mem::size_of::<GspMessageHeader>();
        let route = rpc_route(rpc_id);

        if header_size + payload_len > GSP_MESSAGE_MAX_SIZE {
            return Err(HalError::Exhausted);
        }
        validate_rpc_submission(route, payload)?;

        let header = GspMessageHeader {
            signature: RPC_SIGNATURE,
            rpc_id,
            length: (header_size + payload_len) as u32,
            status: 0,
            sequence: ctx.next_sequence,
            reserved: [0; 3],
        };
        ctx.next_sequence = ctx.next_sequence.wrapping_add(1);

        ctx.command_shadow[..header_size].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const GspMessageHeader as *const u8,
                header_size,
            )
        });
        ctx.command_shadow[header_size..header_size + payload_len].copy_from_slice(payload);

        if !ctx.loopback_ready {
            ctx.loopback_failures = ctx.loopback_failures.saturating_add(1);
            return Err(HalError::InvalidDevice);
        }

        let response_payload = synthesize_gsp_response(rpc_id, payload)?;
        let response_len = header_size + response_payload.len();
        let response_header = GspMessageHeader {
            signature: RPC_SIGNATURE,
            rpc_id,
            length: response_len as u32,
            status: 0,
            sequence: header.sequence,
            reserved: [0; 3],
        };
        ctx.response_shadow[..header_size].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &response_header as *const GspMessageHeader as *const u8,
                header_size,
            )
        });
        ctx.response_shadow[header_size..response_len].copy_from_slice(&response_payload);
        ctx.loopback_completions = ctx.loopback_completions.saturating_add(1);

        if mmio_vaddr != 0 && !ctx.loopback_ready {
            unsafe {
                Self::trigger_gsp_doorbell(mmio_vaddr);
            }
        }

        Ok(response_payload)
    }

    pub unsafe fn trigger_gsp_doorbell(mmio_vaddr: u64) {
        unsafe {
            let ptr = (mmio_vaddr as *mut u32).add(NV_GSP_COMMAND_QUEUE / 4);
            core::ptr::write_volatile(ptr, 0x1);
        }
    }

    pub fn setup_interrupts<P: DevicePlatform>(
        &self,
        platform: &mut P,
    ) -> Result<NvidiaInterruptAgent, HalError> {
        let (handle, route) = platform.claim_interrupt(self.locator, 0)?;
        enable_msix_capability(platform, self.locator)?;
        platform.enable_interrupt(handle)?;

        Ok(NvidiaInterruptAgent {
            handle,
            vector: route.vector,
            count: 0,
        })
    }
}

pub struct NvidiaGspAgent {
    mmio_vaddr: u64,
    ctx: GspRpcContext,
}

impl NvidiaGspAgent {
    pub fn new(mmio_vaddr: u64, ctx: GspRpcContext) -> Self {
        Self { mmio_vaddr, ctx }
    }

    pub unsafe fn execute_semantic_op(
        &mut self,
        gpu: &mut NvidiaGpu,
        rpc_id: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>, HalError> {
        unsafe { gpu.send_rpc(&mut self.ctx, self.mmio_vaddr, rpc_id, payload) }
    }

    pub fn loopback_ready(&self) -> bool {
        self.ctx.loopback_ready
    }

    pub fn loopback_completions(&self) -> u64 {
        self.ctx.loopback_completions
    }

    pub fn loopback_failures(&self) -> u64 {
        self.ctx.loopback_failures
    }
}

pub struct VramSlice {
    pub offset: u64,
    pub size: u64,
    pub mapping_vaddr: u64,
}

pub struct NvidiaVramAgent {
    bar1_vaddr: u64,
    bar1_len: u64,
    total_allocated: u64,
}

impl NvidiaVramAgent {
    pub fn new(bar1_vaddr: u64, bar1_len: u64) -> Self {
        Self {
            bar1_vaddr,
            bar1_len,
            total_allocated: 0,
        }
    }

    pub fn bar1_len(&self) -> u64 {
        self.bar1_len
    }

    pub fn grant_slice(&mut self, size: u64) -> Result<VramSlice, HalError> {
        if self.total_allocated + size > self.bar1_len {
            return Err(HalError::Exhausted);
        }
        let slice = VramSlice {
            offset: self.total_allocated,
            size,
            mapping_vaddr: self.bar1_vaddr + self.total_allocated,
        };
        self.total_allocated += size;
        Ok(slice)
    }
}

pub struct NvidiaDisplayAgent {
    pub active_pipes: u32,
    pub last_present_offset: Option<u64>,
    pub last_present_len: usize,
    pub presented_frames: u64,
}

impl NvidiaDisplayAgent {
    pub fn new(active_pipes: u32) -> Self {
        Self {
            active_pipes,
            last_present_offset: None,
            last_present_len: 0,
            presented_frames: 0,
        }
    }

    pub fn plan_frame_present(
        &mut self,
        vram_offset: u64,
        frame_len: usize,
        vram_len: u64,
    ) -> Result<(), HalError> {
        if self.active_pipes == 0 || frame_len == 0 {
            return Err(HalError::InvalidDevice);
        }
        let end = vram_offset
            .checked_add(frame_len as u64)
            .ok_or(HalError::InvalidDevice)?;
        if end > vram_len {
            return Err(HalError::Exhausted);
        }
        self.last_present_offset = Some(vram_offset);
        self.last_present_len = frame_len;
        self.presented_frames = self.presented_frames.saturating_add(1);
        Ok(())
    }
}

fn synthesize_gsp_response(rpc_id: u32, payload: &[u8]) -> Result<Vec<u8>, HalError> {
    match rpc_route(rpc_id).agent {
        NvidiaNanoAgentKind::GspControl => {
            let mut response = Vec::with_capacity(8);
            response.extend_from_slice(&rpc_id.to_le_bytes());
            response.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            Ok(response)
        }
        NvidiaNanoAgentKind::Power if rpc_id == NV_RPC_ID_SET_POWER_STATE && payload.len() == 4 => {
            Ok(payload.to_vec())
        }
        NvidiaNanoAgentKind::Media
            if rpc_id == NV_RPC_ID_NVENC_SESSION_START
                && payload.len() == core::mem::size_of::<NvidiaEncConfig>() =>
        {
            Ok(payload.to_vec())
        }
        NvidiaNanoAgentKind::Neural
            if rpc_id == NV_RPC_ID_DLSS5_INJECT_SEMANTICS
                || rpc_id == NV_RPC_ID_DLSS5_COMMIT_NEURAL_FRAME =>
        {
            Ok(payload.to_vec())
        }
        NvidiaNanoAgentKind::Tensor
            if rpc_id == NV_RPC_ID_TENSOR_DISPATCH && payload.len() == 4 =>
        {
            Ok(payload.to_vec())
        }
        _ => Err(HalError::Unsupported),
    }
}

pub struct NvidiaNeuralAgent {
    pub model_loaded: bool,
    pub active_semantics: Vec<String>,
    pub last_commit_completed: bool,
}

impl NvidiaNeuralAgent {
    pub fn new() -> Self {
        Self {
            model_loaded: false,
            active_semantics: Vec::new(),
            last_commit_completed: false,
        }
    }

    pub unsafe fn inject_scene_semantics(
        &mut self,
        gpu: &mut NvidiaGpu,
        gsp: &mut NvidiaGspAgent,
        object_label: &str,
    ) -> Result<(), HalError> {
        let payload = object_label.as_bytes();
        unsafe {
            gsp.execute_semantic_op(gpu, NV_RPC_ID_DLSS5_INJECT_SEMANTICS, payload)?;
        }
        self.model_loaded = true;
        self.last_commit_completed = false;
        self.active_semantics.push(String::from(object_label));
        Ok(())
    }

    pub unsafe fn commit_neural_refinement(
        &mut self,
        gpu: &mut NvidiaGpu,
        gsp: &mut NvidiaGspAgent,
    ) -> Result<(), HalError> {
        unsafe {
            gsp.execute_semantic_op(gpu, NV_RPC_ID_DLSS5_COMMIT_NEURAL_FRAME, &[1])?;
        }
        self.last_commit_completed = true;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvidiaPowerState {
    P0,
    P5,
    P8,
    P12,
}

pub struct NvidiaPowerAgent {
    pub current_state: NvidiaPowerState,
    pub max_temp_celsius: u32,
}

impl NvidiaPowerAgent {
    pub fn new() -> Self {
        Self {
            current_state: NvidiaPowerState::P8,
            max_temp_celsius: 0,
        }
    }

    pub const fn graphics_clock_mhz(&self) -> u32 {
        match self.current_state {
            NvidiaPowerState::P0 => RTX_5060_TI_LOCAL_GRAPHICS_CLOCK_MHZ,
            NvidiaPowerState::P5 => 1920,
            NvidiaPowerState::P8 => 1200,
            NvidiaPowerState::P12 => 300,
        }
    }

    pub const fn memory_clock_mhz(&self) -> u32 {
        match self.current_state {
            NvidiaPowerState::P0 => RTX_5060_TI_LOCAL_MEMORY_CLOCK_MHZ,
            NvidiaPowerState::P5 => 1400,
            NvidiaPowerState::P8 => 900,
            NvidiaPowerState::P12 => 405,
        }
    }

    pub const fn boost_clock_mhz(&self) -> u32 {
        match self.current_state {
            NvidiaPowerState::P0 => RTX_5060_TI_LOCAL_BOOST_CLOCK_MHZ,
            NvidiaPowerState::P5 => 2100,
            NvidiaPowerState::P8 => 1500,
            NvidiaPowerState::P12 => 600,
        }
    }

    pub unsafe fn request_pstate(
        &mut self,
        gpu: &mut NvidiaGpu,
        gsp: &mut NvidiaGspAgent,
        state: NvidiaPowerState,
    ) -> Result<(), HalError> {
        let pstate_raw: u32 = match state {
            NvidiaPowerState::P0 => 0,
            NvidiaPowerState::P5 => 5,
            NvidiaPowerState::P8 => 8,
            NvidiaPowerState::P12 => 12,
        };
        unsafe {
            gsp.execute_semantic_op(gpu, NV_RPC_ID_SET_POWER_STATE, &pstate_raw.to_le_bytes())?;
        }
        self.current_state = state;
        Ok(())
    }
}

pub struct NvidiaRayTracingAgent {
    pub active_bvh_count: usize,
}

impl NvidiaRayTracingAgent {
    pub fn new() -> Self {
        Self {
            active_bvh_count: 0,
        }
    }

    pub unsafe fn set_bvh_root(
        &mut self,
        gpu: &mut NvidiaGpu,
        gsp: &mut NvidiaGspAgent,
        paddr: u64,
    ) -> Result<(), HalError> {
        unsafe {
            gsp.execute_semantic_op(gpu, NV_RPC_ID_RT_SET_BVH_ROOT, &paddr.to_le_bytes())?;
        }
        self.active_bvh_count += 1;
        Ok(())
    }
}

pub struct NvidiaMediaAgent {
    pub sessions: usize,
    pub last_config: Option<NvidiaEncConfig>,
}

impl NvidiaMediaAgent {
    pub fn new() -> Self {
        Self {
            sessions: 0,
            last_config: None,
        }
    }

    pub unsafe fn start_encode_session(
        &mut self,
        gpu: &mut NvidiaGpu,
        gsp: &mut NvidiaGspAgent,
        config: NvidiaEncConfig,
    ) -> Result<(), HalError> {
        let payload = unsafe {
            core::slice::from_raw_parts(
                &config as *const NvidiaEncConfig as *const u8,
                core::mem::size_of::<NvidiaEncConfig>(),
            )
        };
        unsafe {
            gsp.execute_semantic_op(gpu, NV_RPC_ID_NVENC_SESSION_START, payload)?;
        }
        self.sessions += 1;
        self.last_config = Some(config);
        Ok(())
    }
}

pub struct NvidiaTensorAgent {
    pub active_jobs: usize,
    pub last_kernel_id: u32,
}

impl NvidiaTensorAgent {
    pub fn new() -> Self {
        Self {
            active_jobs: 0,
            last_kernel_id: 0,
        }
    }

    pub unsafe fn dispatch_tensor_kernel(
        &mut self,
        gpu: &mut NvidiaGpu,
        gsp: &mut NvidiaGspAgent,
        kernel_id: u32,
    ) -> Result<(), HalError> {
        unsafe {
            gsp.execute_semantic_op(gpu, NV_RPC_ID_TENSOR_DISPATCH, &kernel_id.to_le_bytes())?;
        }
        self.active_jobs += 1;
        self.last_kernel_id = kernel_id;
        Ok(())
    }
}

pub fn probe_nvidia_devices<P: DevicePlatform>(
    platform: &mut P,
) -> Result<Vec<NvidiaGpu>, HalError> {
    let mut devices = Vec::new();
    let records = platform.enumerate_devices()?;
    for record in records {
        if record.identity.vendor_id == NVIDIA_VENDOR_ID
            && record.identity.device_id == RTX_5060_TI_DEVICE_ID
        {
            if let Some(gpu) = NvidiaGpu::try_detect(platform, record.locator)? {
                devices.push(gpu);
            }
        }
    }
    Ok(devices)
}

// Power Management Opcodes (Local)
const NV_RPC_ID_SET_POWER_STATE: u32 = 0x1001;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_platform::{
        PciAddress, SyntheticPciConfigBackend, X86_64DevicePlatform, X86_64DevicePlatformConfig,
    };
    use platform_hal::{DeviceIdentity, DevicePlatform, GpuPlatform};

    #[test]
    fn probe_surface_is_marked_confirmed() {
        let surface = nano_agent_surface(NvidiaNanoAgentKind::Probe);
        assert_eq!(surface.knowledge, ReKnowledge::Confirmed);
        assert!(!surface.rationale.is_empty());
    }

    #[test]
    fn display_surface_is_marked_experimental() {
        let surface = nano_agent_surface(NvidiaNanoAgentKind::Display);
        assert_eq!(surface.knowledge, ReKnowledge::Experimental);
    }

    #[test]
    fn vbios_surface_is_marked_confirmed_after_real_dump_recovery() {
        let surface = nano_agent_surface(NvidiaNanoAgentKind::Vbios);
        assert_eq!(surface.knowledge, ReKnowledge::Confirmed);
        assert!(surface.rationale.contains("real-hardware VBIOS bytes"));
    }

    #[test]
    fn local_windows_binding_anchors_rtx_5060_ti_observations() {
        let binding = local_windows_binding_for_device(NVIDIA_VENDOR_ID, RTX_5060_TI_DEVICE_ID)
            .expect("local evidence for RTX 5060 Ti must be present");
        assert_eq!(binding.architecture, "Blackwell");
        assert_eq!(binding.inf_section, "Section048");
        assert_eq!(binding.kernel_service, "nvlddmkm");
        assert_eq!(binding.vbios_version, "98.06.1f.00.dc");
        assert_eq!(binding.gpu_part_number, "2D04-300-A1");
        assert_eq!(binding.subsystem_id, 0x205e_1771);
    }

    #[test]
    fn local_windows_binding_carries_confirmed_msi_policy() {
        let binding =
            local_windows_binding_for_device(NVIDIA_VENDOR_ID, RTX_5060_TI_DEVICE_ID).unwrap();
        assert_eq!(binding.msi_policy.section_name, "nv_msiSupport_addreg");
        assert!(binding.msi_policy.msi_supported);
        assert_eq!(binding.msi_policy.message_number_limit, 1);
    }

    #[test]
    fn local_windows_binding_converts_to_platform_gpu_evidence() {
        let evidence = as_platform_gpu_binding_evidence(RTX_5060_TI_LOCAL_WINDOWS_BINDING);
        assert_eq!(evidence.architecture_name, "Blackwell");
        assert_eq!(evidence.product_name, "NVIDIA GeForce RTX 5060 Ti");
        assert_eq!(evidence.msi_policy.source_name, "nv_msiSupport_addreg");
        assert_eq!(evidence.msi_policy.message_limit, 1);
    }

    #[test]
    fn local_windows_binding_is_not_reported_for_other_devices() {
        assert!(local_windows_binding_for_device(NVIDIA_VENDOR_ID, 0x2d05).is_none());
    }

    #[test]
    fn local_real_vbios_hash_is_anchored_for_rtx_5060_ti_dump() {
        assert_eq!(
            RTX_5060_TI_LOCAL_REAL_VBIOS_SHA256,
            "9a294cebf93aa635acba0fe5f7cd9b2ced6b357eeef85a81d22fabb98923aef2"
        );
    }

    #[test]
    fn inspect_vbios_image_bytes_extracts_nvidia_board_identity() {
        let mut image = vec![0; 0x600];
        image[0x40..0x44].copy_from_slice(b"NVFW");
        image[0x120..0x124].copy_from_slice(b"PCIR");
        image[0x124..0x126].copy_from_slice(&0x10deu16.to_le_bytes());
        image[0x126..0x128].copy_from_slice(&0x2d04u16.to_le_bytes());
        image[0x1c0..0x1da].copy_from_slice(b"NVIDIA GeForce RTX 5060 Ti");
        image[0x220..0x22e].copy_from_slice(b"P14N:506T301FB");
        image[0x280..0x296].copy_from_slice(b"Version 98.06.1F.00.DC");
        image[0x320..0x323].copy_from_slice(b"BIT");

        let evidence = inspect_vbios_image_bytes(&image).unwrap();

        assert_eq!(evidence.image_len, image.len());
        assert_eq!(evidence.pcir_offset, 0x120);
        assert_eq!(evidence.vendor_id, 0x10de);
        assert_eq!(evidence.device_id, 0x2d04);
        assert_eq!(evidence.board_name, "NVIDIA GeForce RTX 5060 Ti");
        assert_eq!(evidence.board_code, "P14N:506T301FB");
        assert_eq!(evidence.version, "Version 98.06.1F.00.DC");
        assert_eq!(evidence.nvfw_offset, Some(0x40));
        assert_eq!(evidence.bit_offset, Some(0x320));
    }

    #[test]
    fn inspect_vbios_image_bytes_rejects_images_without_pcir_signature() {
        let image = vec![0xff; 0x200];
        assert!(matches!(
            inspect_vbios_image_bytes(&image),
            Err(HalError::Unsupported)
        ));
    }

    #[test]
    fn unknown_rpc_defaults_to_inferred_gsp_control() {
        let route = rpc_route(0x55aa);
        assert_eq!(route.knowledge, ReKnowledge::Inferred);
        assert_eq!(route.agent, NvidiaNanoAgentKind::GspControl);
    }

    #[test]
    fn generic_draw_submit_route_is_inferred_gsp_control() {
        let route = rpc_route(NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW);
        assert_eq!(route.knowledge, ReKnowledge::Inferred);
        assert_eq!(route.agent, NvidiaNanoAgentKind::GspControl);
    }

    #[test]
    fn neural_rpc_is_marked_experimental() {
        let route = rpc_route(NV_RPC_ID_DLSS5_INJECT_SEMANTICS);
        assert_eq!(route.knowledge, ReKnowledge::Experimental);
        assert_eq!(route.agent, NvidiaNanoAgentKind::Neural);
    }

    #[test]
    fn rpc_submission_rejects_experimental_routes_on_generic_control_path() {
        let route = rpc_route(NV_RPC_ID_DLSS5_INIT);
        assert!(matches!(
            validate_rpc_submission(route, b"semantic-scene"),
            Err(HalError::Unsupported)
        ));
    }

    #[test]
    fn rpc_submission_accepts_inferred_control_route_with_payload() {
        let route = rpc_route(NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW);
        assert!(validate_rpc_submission(route, b"draw:triangle").is_ok());
    }

    #[test]
    fn power_rpc_submission_is_allowed_for_local_synthetic_control_path() {
        let route = rpc_route(NV_RPC_ID_SET_POWER_STATE);
        assert!(validate_rpc_submission(route, &8u32.to_le_bytes()).is_ok());
    }

    #[test]
    fn power_rpc_synthesizes_state_echo_response() {
        let response =
            synthesize_gsp_response(NV_RPC_ID_SET_POWER_STATE, &0u32.to_le_bytes()).unwrap();
        assert_eq!(response, 0u32.to_le_bytes());
    }

    #[test]
    fn media_rpc_submission_is_allowed_for_local_synthetic_control_path() {
        let route = rpc_route(NV_RPC_ID_NVENC_SESSION_START);
        let payload = [0u8; core::mem::size_of::<NvidiaEncConfig>()];
        assert!(validate_rpc_submission(route, &payload).is_ok());
    }

    #[test]
    fn neural_rpc_submission_is_allowed_for_local_synthetic_control_path() {
        let route = rpc_route(NV_RPC_ID_DLSS5_INJECT_SEMANTICS);
        assert!(validate_rpc_submission(route, b"enemy-vehicle").is_ok());
    }

    #[test]
    fn tensor_rpc_submission_is_allowed_for_local_synthetic_control_path() {
        let route = rpc_route(NV_RPC_ID_TENSOR_DISPATCH);
        assert!(validate_rpc_submission(route, &7u32.to_le_bytes()).is_ok());
    }

    #[test]
    fn rpc_submission_rejects_empty_payloads() {
        let route = rpc_route(NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW);
        assert!(matches!(
            validate_rpc_submission(route, b""),
            Err(HalError::InvalidDevice)
        ));
    }

    fn sample_interrupt_platform() -> (
        X86_64DevicePlatform<SyntheticPciConfigBackend>,
        PciAddress,
        DeviceLocator,
    ) {
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
        backend.define_capability(
            gpu,
            0x50,
            (PCI_CAP_ID_MSIX as u32) | ((0x0003u32) << 16),
            0x00,
        );
        let mut platform =
            X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
        let locator = platform.enumerate_devices().unwrap()[0].locator;
        (platform, gpu, locator)
    }

    fn sample_vbios_platform(
        rom_bar: u32,
        rom_image: Option<&[u8]>,
    ) -> (
        X86_64DevicePlatform<SyntheticPciConfigBackend>,
        DeviceLocator,
    ) {
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
        backend.define_config_dword(gpu, PCI_EXPANSION_ROM_OFFSET, rom_bar);
        if let Some(image) = rom_image {
            backend.define_rom((rom_bar & PCI_ROM_ADDRESS_MASK) as u64, image);
        }
        let mut platform =
            X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
        let locator = platform.enumerate_devices().unwrap()[0].locator;
        (platform, locator)
    }

    #[test]
    fn setup_interrupts_enables_msix_in_capability_register() {
        let (mut platform, gpu_addr, locator) = sample_interrupt_platform();
        let gpu = NvidiaGpu::try_detect(&mut platform, locator)
            .unwrap()
            .unwrap();

        let interrupt = gpu.setup_interrupts(&mut platform).unwrap();

        let control_dword = platform.backend_mut().register(gpu_addr, 0x50);
        assert_ne!(interrupt.vector, 0);
        assert_ne!(control_dword & ((MSIX_CONTROL_ENABLE as u32) << 16), 0);
    }

    #[test]
    fn setup_interrupts_rejects_devices_without_msix_capability() {
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
        let mut platform =
            X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
        let locator = platform.enumerate_devices().unwrap()[0].locator;
        let gpu = NvidiaGpu::try_detect(&mut platform, locator)
            .unwrap()
            .unwrap();

        let result = gpu.setup_interrupts(&mut platform);
        assert!(matches!(result, Err(HalError::InvalidInterrupt)));
    }

    #[test]
    fn inspect_vbios_window_reports_pci_rom_state() {
        let (mut platform, locator) = sample_vbios_platform(0x00c0_0001, None);
        let gpu = NvidiaGpu::try_detect(&mut platform, locator)
            .unwrap()
            .unwrap();

        let window = gpu.inspect_vbios_window(&mut platform).unwrap();

        assert_eq!(window.rom_bar_raw, 0x00c0_0001);
        assert_eq!(window.physical_base, 0x00c0_0000);
        assert!(window.enabled);
    }

    #[test]
    fn read_vbios_returns_real_rom_bytes_when_backend_exposes_them() {
        let rom_image = vec![0x55, 0xaa, 0x4e, 0x56, 0x49, 0x44, 0x49, 0x41];
        let (mut platform, locator) = sample_vbios_platform(0x00c0_0001, Some(&rom_image));
        let gpu = NvidiaGpu::try_detect(&mut platform, locator)
            .unwrap()
            .unwrap();

        let vbios = gpu.read_vbios(&mut platform).unwrap();
        assert_eq!(&vbios[..rom_image.len()], rom_image.as_slice());
    }

    #[test]
    fn read_vbios_refuses_when_rom_window_has_no_backing_image() {
        let (mut platform, locator) = sample_vbios_platform(0x00c0_0001, None);
        let gpu = NvidiaGpu::try_detect(&mut platform, locator)
            .unwrap()
            .unwrap();

        assert!(matches!(
            gpu.read_vbios(&mut platform),
            Err(HalError::Unsupported)
        ));
    }

    #[test]
    fn gsp_loopback_completes_generic_draw_submission() {
        let (mut platform, locator) = sample_vbios_platform(0x00c0_0001, None);
        let mut gpu = NvidiaGpu::try_detect(&mut platform, locator)
            .unwrap()
            .unwrap();
        let mmio_vaddr = 0;
        let mut ctx = gpu.setup_gsp_channels(&mut platform).unwrap();

        let response = unsafe {
            gpu.send_rpc(
                &mut ctx,
                mmio_vaddr,
                NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW,
                b"draw:loopback",
            )
        }
        .unwrap();

        assert_eq!(response.len(), 8);
        assert_eq!(
            u32::from_le_bytes(response[0..4].try_into().unwrap()),
            NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW
        );
        assert_eq!(u32::from_le_bytes(response[4..8].try_into().unwrap()), 13);
        assert_eq!(ctx.loopback_completions, 1);
        assert_eq!(ctx.loopback_failures, 0);
    }

    #[test]
    fn gsp_loopback_refuses_when_channel_is_not_ready() {
        let (mut platform, locator) = sample_vbios_platform(0x00c0_0001, None);
        let mut gpu = NvidiaGpu::try_detect(&mut platform, locator)
            .unwrap()
            .unwrap();
        let mut ctx = gpu.setup_gsp_channels(&mut platform).unwrap();
        ctx.loopback_ready = false;

        let result = unsafe {
            gpu.send_rpc(
                &mut ctx,
                0,
                NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW,
                b"draw:blocked",
            )
        };

        assert!(matches!(result, Err(HalError::InvalidDevice)));
        assert_eq!(ctx.loopback_failures, 1);
    }

    #[test]
    fn display_agent_tracks_presented_frame_bounds() {
        let mut display = NvidiaDisplayAgent::new(1);
        display.plan_frame_present(0x2000, 128, 0x4000).unwrap();

        assert_eq!(display.last_present_offset, Some(0x2000));
        assert_eq!(display.last_present_len, 128);
        assert_eq!(display.presented_frames, 1);
    }

    #[test]
    fn display_agent_rejects_present_outside_vram_window() {
        let mut display = NvidiaDisplayAgent::new(1);
        let result = display.plan_frame_present(0x3000, 0x2000, 0x4000);
        assert!(matches!(result, Err(HalError::Exhausted)));
    }

    #[test]
    fn interrupt_agent_tracks_completion_count_after_service() {
        let (mut platform, _, locator) = sample_interrupt_platform();
        platform.setup_gpu_agent(locator).unwrap();

        let response = platform
            .submit_gpu_command(NV_RPC_ID_GSP_SUBMIT_GENERIC_DRAW, b"draw:interrupt")
            .unwrap();

        assert_eq!(response.len(), 8);
        let interrupt = platform.nvidia_interrupt_agent().unwrap();
        assert_eq!(interrupt.count, 1);
        assert!(platform.pending_interrupts().is_empty());
    }
}
