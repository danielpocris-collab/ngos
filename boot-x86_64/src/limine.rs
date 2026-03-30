use core::ptr;

use limine::{
    BaseRevision,
    memory_map::EntryType,
    request::{
        BootloaderInfoRequest, ExecutableAddressRequest, ExecutableCmdlineRequest,
        FramebufferRequest, HhdmRequest, MemoryMapRequest, ModuleRequest, RequestsEndMarker,
        RequestsStartMarker, RsdpRequest, StackSizeRequest,
    },
};
use platform_x86_64::{BootInfo, BootMemoryRegion, BootMemoryRegionKind, BootModule, BootProtocol};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};
use crate::serial;

pub const LIMINE_STACK_SIZE: u64 = 256 * 1024;
pub const LIMINE_BASE_REVISION: u64 = 4;
const MAX_MEMORY_REGIONS: usize = 256;
const MAX_MODULES: usize = 32;

const EMPTY_MEMORY_REGION: BootMemoryRegion = BootMemoryRegion {
    start: 0,
    len: 0,
    kind: BootMemoryRegionKind::Reserved,
};

const EMPTY_MODULE: BootModule<'static> = BootModule {
    name: "",
    physical_start: 0,
    len: 0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimineBootError {
    MissingBaseRevision,
    UnsupportedBaseRevision { loaded: Option<u64> },
    MissingResponse(&'static str),
    TooManyMemoryRegions { count: usize, capacity: usize },
    TooManyModules { count: usize, capacity: usize },
    InvalidCommandLineUtf8,
    InvalidModulePathUtf8 { index: usize },
}

#[used]
#[unsafe(link_section = ".limine_requests_start")]
static LIMINE_REQUESTS_START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_BASE_REVISION_TAG: BaseRevision = BaseRevision::with_revision(LIMINE_BASE_REVISION);

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_STACK_SIZE_REQUEST: StackSizeRequest =
    StackSizeRequest::new().with_size(LIMINE_STACK_SIZE);

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_BOOTLOADER_INFO_REQUEST: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_MODULE_REQUEST: ModuleRequest = ModuleRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest =
    ExecutableAddressRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static LIMINE_EXECUTABLE_CMDLINE_REQUEST: ExecutableCmdlineRequest =
    ExecutableCmdlineRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests_end")]
static LIMINE_REQUESTS_END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

static mut LIMINE_MEMORY_REGIONS: [BootMemoryRegion; MAX_MEMORY_REGIONS] =
    [EMPTY_MEMORY_REGION; MAX_MEMORY_REGIONS];
static mut LIMINE_MODULES: [BootModule<'static>; MAX_MODULES] = [EMPTY_MODULE; MAX_MODULES];

pub fn bootloader_identity() -> Option<(&'static str, &'static str)> {
    let response = LIMINE_BOOTLOADER_INFO_REQUEST.get_response()?;
    Some((response.name(), response.version()))
}

pub fn write_boot_info(
    out: *mut BootInfo<'static>,
    kernel_image_len: u64,
) -> Result<(), LimineBootError> {
    boot_locator::event(
        BootLocatorStage::Limine,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x200,
        BootPayloadLabel::Length,
        kernel_image_len,
        BootPayloadLabel::None,
        0,
    );
    serial::debug_marker(b'E');
    if !LIMINE_BASE_REVISION_TAG.is_valid() {
        return Err(LimineBootError::MissingBaseRevision);
    }
    if !LIMINE_BASE_REVISION_TAG.is_supported() {
        return Err(LimineBootError::UnsupportedBaseRevision {
            loaded: LIMINE_BASE_REVISION_TAG.loaded_revision(),
        });
    }
    serial::debug_marker(b'F');

    let memory_map_response = LIMINE_MEMORY_MAP_REQUEST
        .get_response()
        .ok_or(LimineBootError::MissingResponse("memory map"))?;
    boot_locator::event(
        BootLocatorStage::Limine,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x210,
        BootPayloadLabel::Count,
        memory_map_response.entries().len() as u64,
        BootPayloadLabel::None,
        0,
    );
    serial::debug_marker(b'G');
    let hhdm_response = LIMINE_HHDM_REQUEST
        .get_response()
        .ok_or(LimineBootError::MissingResponse("higher-half direct map"))?;
    serial::debug_marker(b'H');
    let physical_memory_offset = hhdm_response.offset();
    boot_locator::event(
        BootLocatorStage::Limine,
        BootLocatorKind::Memory,
        BootLocatorSeverity::Info,
        0x220,
        BootPayloadLabel::Address,
        physical_memory_offset,
        BootPayloadLabel::None,
        0,
    );
    let executable_address_response = LIMINE_EXECUTABLE_ADDRESS_REQUEST
        .get_response()
        .ok_or(LimineBootError::MissingResponse("executable address"))?;
    serial::debug_marker(b'I');

    serial::debug_marker(b'J');

    let memory_map = memory_map_response.entries();
    let modules = LIMINE_MODULE_REQUEST
        .get_response()
        .map(|response| response.modules())
        .unwrap_or(&[]);
    boot_locator::event(
        BootLocatorStage::Limine,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x230,
        BootPayloadLabel::Count,
        modules.len() as u64,
        BootPayloadLabel::Count,
        memory_map.len() as u64,
    );
    if memory_map.len() > MAX_MEMORY_REGIONS {
        return Err(LimineBootError::TooManyMemoryRegions {
            count: memory_map.len(),
            capacity: MAX_MEMORY_REGIONS,
        });
    }
    if modules.len() > MAX_MODULES {
        return Err(LimineBootError::TooManyModules {
            count: modules.len(),
            capacity: MAX_MODULES,
        });
    }
    serial::debug_marker(b'U');

    let memory_regions = unsafe {
        core::slice::from_raw_parts_mut(
            ptr::addr_of_mut!(LIMINE_MEMORY_REGIONS).cast::<BootMemoryRegion>(),
            MAX_MEMORY_REGIONS,
        )
    };
    for (index, entry) in memory_map.iter().copied().enumerate() {
        memory_regions[index] = BootMemoryRegion {
            start: entry.base,
            len: entry.length,
            kind: map_region_kind(entry.entry_type),
        };
    }
    serial::debug_marker(b'V');

    let module_storage = unsafe {
        core::slice::from_raw_parts_mut(
            ptr::addr_of_mut!(LIMINE_MODULES).cast::<BootModule<'static>>(),
            MAX_MODULES,
        )
    };
    for (index, module) in modules.iter().copied().enumerate() {
        let name = module
            .path()
            .to_str()
            .map_err(|_| LimineBootError::InvalidModulePathUtf8 { index })?;
        module_storage[index] = BootModule {
            name,
            physical_start: (module.addr() as u64).saturating_sub(physical_memory_offset),
            len: module.size(),
        };
    }
    serial::debug_marker(b'W');

    let command_line = LIMINE_EXECUTABLE_CMDLINE_REQUEST
        .get_response()
        .map(|response| response.cmdline())
        .filter(|cmdline| !cmdline.to_bytes().is_empty())
        .map(|cmdline| {
            cmdline
                .to_str()
                .map_err(|_| LimineBootError::InvalidCommandLineUtf8)
        })
        .transpose()?;
    serial::debug_marker(b'K');

    let framebuffer_response = LIMINE_FRAMEBUFFER_REQUEST.get_response();
    let framebuffer = framebuffer_response.and_then(|response| response.framebuffers().next());
    serial::debug_marker(b'L');

    serial::debug_marker(b'M');
    let rsdp = LIMINE_RSDP_REQUEST
        .get_response()
        .map(|response| response.address() as u64);
    boot_locator::event(
        BootLocatorStage::Limine,
        BootLocatorKind::Memory,
        BootLocatorSeverity::Info,
        0x240,
        BootPayloadLabel::Address,
        rsdp.unwrap_or(0),
        BootPayloadLabel::Status,
        rsdp.is_some() as u64,
    );
    serial::debug_marker(b'N');
    let memory_regions = &memory_regions[..memory_map.len()];
    serial::debug_marker(b'O');
    let modules = &module_storage[..modules.len()];
    serial::debug_marker(b'P');
    unsafe {
        serial::debug_marker(b'0');
        ptr::addr_of_mut!((*out).protocol).write(BootProtocol::Limine);
        serial::debug_marker(b'1');
        ptr::addr_of_mut!((*out).command_line).write(command_line);
        serial::debug_marker(b'2');
        ptr::addr_of_mut!((*out).rsdp).write(rsdp);
        serial::debug_marker(b'3');
        ptr::addr_of_mut!((*out).memory_regions).write(memory_regions);
        serial::debug_marker(b'4');
        ptr::addr_of_mut!((*out).modules).write(modules);
        serial::debug_marker(b'5');
        let framebuffer_out = ptr::addr_of_mut!((*out).framebuffer);
        match framebuffer {
            Some(fb) => framebuffer_out.write(Some(platform_x86_64::FramebufferInfo {
                physical_start: (fb.addr() as u64).saturating_sub(physical_memory_offset),
                width: fb.width().min(u32::MAX as u64) as u32,
                height: fb.height().min(u32::MAX as u64) as u32,
                pitch: fb.pitch().min(u32::MAX as u64) as u32,
                bpp: fb.bpp(),
                red_mask_size: fb.red_mask_size(),
                red_mask_shift: fb.red_mask_shift(),
                green_mask_size: fb.green_mask_size(),
                green_mask_shift: fb.green_mask_shift(),
                blue_mask_size: fb.blue_mask_size(),
                blue_mask_shift: fb.blue_mask_shift(),
            })),
            None => framebuffer_out.write(None),
        };
        serial::debug_marker(b'6');
        ptr::addr_of_mut!((*out).physical_memory_offset).write(physical_memory_offset);
        serial::debug_marker(b'7');
        ptr::addr_of_mut!((*out).kernel_phys_range).write(BootMemoryRegion {
            start: executable_address_response.physical_base(),
            len: kernel_image_len,
            kind: BootMemoryRegionKind::KernelImage,
        });
    }
    boot_locator::event(
        BootLocatorStage::Limine,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x250,
        BootPayloadLabel::Address,
        executable_address_response.physical_base(),
        BootPayloadLabel::Length,
        kernel_image_len,
    );
    serial::debug_marker(b'Q');
    serial::debug_marker(b'R');
    Ok(())
}

fn map_region_kind(entry_type: EntryType) -> BootMemoryRegionKind {
    if entry_type == EntryType::USABLE {
        BootMemoryRegionKind::Usable
    } else if entry_type == EntryType::RESERVED {
        BootMemoryRegionKind::Reserved
    } else if entry_type == EntryType::ACPI_RECLAIMABLE {
        BootMemoryRegionKind::AcpiReclaimable
    } else if entry_type == EntryType::ACPI_NVS {
        BootMemoryRegionKind::AcpiNvs
    } else if entry_type == EntryType::BAD_MEMORY {
        BootMemoryRegionKind::BadMemory
    } else if entry_type == EntryType::BOOTLOADER_RECLAIMABLE {
        BootMemoryRegionKind::BootloaderReclaimable
    } else if entry_type == EntryType::EXECUTABLE_AND_MODULES {
        BootMemoryRegionKind::KernelImage
    } else if entry_type == EntryType::FRAMEBUFFER {
        BootMemoryRegionKind::Framebuffer
    } else {
        BootMemoryRegionKind::Reserved
    }
}
