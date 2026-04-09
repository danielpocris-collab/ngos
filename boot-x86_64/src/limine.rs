use core::ptr;

use limine::{
    BaseRevision,
    request::{
        BootloaderInfoRequest, ExecutableAddressRequest, ExecutableCmdlineRequest,
        FramebufferRequest, HhdmRequest, MemoryMapRequest, ModuleRequest, RequestsEndMarker,
        RequestsStartMarker, RsdpRequest, StackSizeRequest,
    },
};
use platform_x86_64::{
    BootInfo, BootInfoValidationError, LoaderDefinedHandoffError,
    LimineBootBuffers, LimineBootSnapshot,
};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};
use crate::serial;

pub const LIMINE_STACK_SIZE: u64 = 512 * 1024;
pub const LIMINE_BASE_REVISION: u64 = 4;
const MAX_MEMORY_REGIONS: usize = 256;
const MAX_MODULES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimineBootError {
    MissingBaseRevision,
    UnsupportedBaseRevision { loaded: Option<u64> },
    MissingResponse(&'static str),
    TooManyMemoryRegions { count: usize, capacity: usize },
    TooManyModules { count: usize, capacity: usize },
    InvalidCommandLineUtf8,
    InvalidModulePathUtf8 { index: usize },
    InvalidBootInfo(BootInfoValidationError),
}

impl LimineBootError {
    pub fn summary_family(self) -> &'static str {
        "limine"
    }

    pub fn summary_detail(self) -> &'static str {
        match self {
            Self::MissingBaseRevision => "missing-base-revision",
            Self::UnsupportedBaseRevision { .. } => "unsupported-base-revision",
            Self::MissingResponse("memory map") => "missing-memory-map",
            Self::MissingResponse("higher-half direct map") => "missing-hhdm",
            Self::MissingResponse("executable address") => "missing-executable-address",
            Self::MissingResponse(_) => "missing-response",
            Self::TooManyMemoryRegions { .. } => "too-many-memory-regions",
            Self::TooManyModules { .. } => "too-many-modules",
            Self::InvalidCommandLineUtf8 => "invalid-command-line-utf8",
            Self::InvalidModulePathUtf8 { .. } => "invalid-module-path-utf8",
            Self::InvalidBootInfo(BootInfoValidationError::UnalignedPhysicalMemoryOffset) => {
                "invalid-hhdm-offset"
            }
            Self::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBeKernelImage) => {
                "invalid-kernel-range-kind"
            }
            Self::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBePageAligned) => {
                "invalid-kernel-range-alignment"
            }
            Self::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBeNonEmpty) => {
                "empty-kernel-range"
            }
            Self::InvalidBootInfo(BootInfoValidationError::MemoryRegionMustBePageAligned) => {
                "invalid-memory-region-alignment"
            }
            Self::InvalidBootInfo(BootInfoValidationError::MemoryRegionMustBeNonEmpty) => {
                "empty-memory-region"
            }
            Self::InvalidBootInfo(BootInfoValidationError::MemoryRegionsOverlap) => {
                "overlapping-memory-regions"
            }
        }
    }

    pub fn locator_status_code(self) -> u64 {
        match self {
            Self::MissingBaseRevision => 0x01,
            Self::UnsupportedBaseRevision { .. } => 0x02,
            Self::MissingResponse("memory map") => 0x10,
            Self::MissingResponse("higher-half direct map") => 0x11,
            Self::MissingResponse("executable address") => 0x12,
            Self::MissingResponse(_) => 0x1f,
            Self::TooManyMemoryRegions { .. } => 0x20,
            Self::TooManyModules { .. } => 0x21,
            Self::InvalidCommandLineUtf8 => 0x30,
            Self::InvalidModulePathUtf8 { .. } => 0x31,
            Self::InvalidBootInfo(BootInfoValidationError::UnalignedPhysicalMemoryOffset) => 0x40,
            Self::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBeKernelImage) => 0x41,
            Self::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBePageAligned) => 0x42,
            Self::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBeNonEmpty) => 0x43,
            Self::InvalidBootInfo(BootInfoValidationError::MemoryRegionMustBePageAligned) => 0x44,
            Self::InvalidBootInfo(BootInfoValidationError::MemoryRegionMustBeNonEmpty) => 0x45,
            Self::InvalidBootInfo(BootInfoValidationError::MemoryRegionsOverlap) => 0x46,
        }
    }
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

static mut LIMINE_BOOT_BUFFERS: LimineBootBuffers = LimineBootBuffers::new();

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
        return Err(report_limine_failure(LimineBootError::MissingBaseRevision));
    }
    if !LIMINE_BASE_REVISION_TAG.is_supported() {
        return Err(report_limine_failure(
            LimineBootError::UnsupportedBaseRevision {
                loaded: LIMINE_BASE_REVISION_TAG.loaded_revision(),
            },
        ));
    }
    serial::debug_marker(b'F');

    let memory_map_response = LIMINE_MEMORY_MAP_REQUEST
        .get_response()
        .ok_or_else(|| report_limine_failure(LimineBootError::MissingResponse("memory map")))?;
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
    let hhdm_response = LIMINE_HHDM_REQUEST.get_response().ok_or_else(|| {
        report_limine_failure(LimineBootError::MissingResponse("higher-half direct map"))
    })?;
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
        .ok_or_else(|| {
            report_limine_failure(LimineBootError::MissingResponse("executable address"))
        })?;
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
    serial::debug_marker(b'U');

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
    let snapshot = LimineBootSnapshot {
        command_line: LIMINE_EXECUTABLE_CMDLINE_REQUEST
            .get_response()
            .map(|response| response.cmdline()),
        rsdp,
        memory_map,
        modules,
        framebuffer,
        physical_memory_offset,
        kernel_physical_base: executable_address_response.physical_base(),
    };
    let mut handoff = unsafe { LIMINE_BOOT_BUFFERS.build_loader_defined_handoff(snapshot, kernel_image_len) }
        .map_err(map_limine_handoff_error)
        .map_err(report_limine_failure)?;
    serial::debug_marker(b'V');
    if let Some(mode) = crate::boot_handoff_proof::apply(&mut handoff) {
        serial::print(format_args!(
            "ngos/x86_64: post-handoff corruption applied mode={}\n",
            mode
        ));
    }
    serial::debug_marker(b'W');
    if handoff.memory_regions.len() > MAX_MEMORY_REGIONS {
        return Err(report_limine_failure(LimineBootError::TooManyMemoryRegions {
            count: handoff.memory_regions.len(),
            capacity: MAX_MEMORY_REGIONS,
        }));
    }
    if handoff.modules.len() > MAX_MODULES {
        return Err(report_limine_failure(LimineBootError::TooManyModules {
            count: handoff.modules.len(),
            capacity: MAX_MODULES,
        }));
    }
    serial::debug_marker(b'K');
    serial::debug_marker(b'O');
    serial::debug_marker(b'P');
    let boot_info = handoff
        .as_boot_info()
        .map_err(map_loader_defined_handoff_error)
        .map_err(report_limine_failure)?;
    unsafe {
        serial::debug_marker(b'0');
        ptr::addr_of_mut!((*out).protocol).write(boot_info.protocol);
        serial::debug_marker(b'1');
        ptr::addr_of_mut!((*out).command_line).write(boot_info.command_line);
        serial::debug_marker(b'2');
        ptr::addr_of_mut!((*out).rsdp).write(boot_info.rsdp);
        serial::debug_marker(b'3');
        ptr::addr_of_mut!((*out).memory_regions).write(boot_info.memory_regions);
        serial::debug_marker(b'4');
        ptr::addr_of_mut!((*out).modules).write(boot_info.modules);
        serial::debug_marker(b'5');
        ptr::addr_of_mut!((*out).framebuffer).write(boot_info.framebuffer);
        serial::debug_marker(b'6');
        ptr::addr_of_mut!((*out).physical_memory_offset).write(boot_info.physical_memory_offset);
        serial::debug_marker(b'7');
        ptr::addr_of_mut!((*out).kernel_phys_range).write(boot_info.kernel_phys_range);
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

fn report_limine_failure(error: LimineBootError) -> LimineBootError {
    boot_locator::event(
        BootLocatorStage::Limine,
        BootLocatorKind::Fault,
        BootLocatorSeverity::Error,
        0x2ff,
        BootPayloadLabel::Status,
        error.locator_status_code(),
        BootPayloadLabel::Value,
        0,
    );
    serial::print(format_args!(
        "ngos/x86_64: limine handoff refusal detail={} status={:#x}\n",
        error.summary_detail(),
        error.locator_status_code()
    ));
    error
}

fn map_limine_handoff_error(error: platform_x86_64::LimineHandoffError) -> LimineBootError {
    match error {
        platform_x86_64::LimineHandoffError::TooManyMemoryRegions { count, capacity } => {
            LimineBootError::TooManyMemoryRegions { count, capacity }
        }
        platform_x86_64::LimineHandoffError::TooManyModules { count, capacity } => {
            LimineBootError::TooManyModules { count, capacity }
        }
        platform_x86_64::LimineHandoffError::InvalidCommandLineUtf8 => {
            LimineBootError::InvalidCommandLineUtf8
        }
        platform_x86_64::LimineHandoffError::InvalidModulePathUtf8 { index } => {
            LimineBootError::InvalidModulePathUtf8 { index }
        }
    }
}

fn map_loader_defined_handoff_error(error: LoaderDefinedHandoffError) -> LimineBootError {
    match error {
        LoaderDefinedHandoffError::InvalidBootInfo(error) => LimineBootError::InvalidBootInfo(error),
        LoaderDefinedHandoffError::InvalidMagic => unreachable!("Limine handoff must preserve loader-defined magic"),
    }
}

#[cfg(test)]
mod tests {
    use super::{LimineBootError, map_loader_defined_handoff_error, map_limine_handoff_error, report_limine_failure};
    use platform_x86_64::{
        BootInfoValidationError, LoaderDefinedHandoffError, LimineHandoffError,
    };

    #[test]
    fn limine_handoff_mapping_preserves_contract_refusal_families() {
        assert_eq!(
            map_limine_handoff_error(LimineHandoffError::TooManyMemoryRegions {
                count: 257,
                capacity: 256,
            }),
            LimineBootError::TooManyMemoryRegions {
                count: 257,
                capacity: 256,
            }
        );
        assert_eq!(
            map_limine_handoff_error(LimineHandoffError::InvalidCommandLineUtf8),
            LimineBootError::InvalidCommandLineUtf8
        );
        assert_eq!(
            map_loader_defined_handoff_error(LoaderDefinedHandoffError::InvalidBootInfo(
                BootInfoValidationError::MemoryRegionsOverlap
            )),
            LimineBootError::InvalidBootInfo(BootInfoValidationError::MemoryRegionsOverlap)
        );
    }

    #[test]
    fn limine_boot_error_summary_tokens_are_stable_for_boot_contract_refusals() {
        assert_eq!(
            LimineBootError::MissingResponse("memory map").summary_detail(),
            "missing-memory-map"
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(BootInfoValidationError::MemoryRegionsOverlap)
                .summary_detail(),
            "overlapping-memory-regions"
        );
        assert_eq!(
            LimineBootError::MissingBaseRevision.summary_family(),
            "limine"
        );
    }

    #[test]
    fn limine_boot_error_locator_codes_are_stable_for_refusal_paths() {
        assert_eq!(
            LimineBootError::MissingResponse("memory map").locator_status_code(),
            0x10
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(BootInfoValidationError::MemoryRegionsOverlap)
                .locator_status_code(),
            0x46
        );
        assert_eq!(
            report_limine_failure(LimineBootError::TooManyModules {
                count: 99,
                capacity: 32
            })
            .locator_status_code(),
            0x21
        );
    }

    #[test]
    fn limine_boot_error_summary_tokens_cover_remaining_refusal_families() {
        assert_eq!(
            LimineBootError::MissingBaseRevision.summary_detail(),
            "missing-base-revision"
        );
        assert_eq!(
            LimineBootError::UnsupportedBaseRevision { loaded: Some(3) }.summary_detail(),
            "unsupported-base-revision"
        );
        assert_eq!(
            LimineBootError::MissingResponse("higher-half direct map").summary_detail(),
            "missing-hhdm"
        );
        assert_eq!(
            LimineBootError::MissingResponse("executable address").summary_detail(),
            "missing-executable-address"
        );
        assert_eq!(
            LimineBootError::MissingResponse("framebuffer").summary_detail(),
            "missing-response"
        );
        assert_eq!(
            LimineBootError::TooManyMemoryRegions {
                count: 257,
                capacity: 256,
            }
            .summary_detail(),
            "too-many-memory-regions"
        );
        assert_eq!(
            LimineBootError::InvalidModulePathUtf8 { index: 0 }.summary_detail(),
            "invalid-module-path-utf8"
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(
                BootInfoValidationError::KernelRangeMustBeKernelImage
            )
            .summary_detail(),
            "invalid-kernel-range-kind"
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(
                BootInfoValidationError::KernelRangeMustBePageAligned
            )
            .summary_detail(),
            "invalid-kernel-range-alignment"
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBeNonEmpty)
                .summary_detail(),
            "empty-kernel-range"
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(
                BootInfoValidationError::MemoryRegionMustBePageAligned
            )
            .summary_detail(),
            "invalid-memory-region-alignment"
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(BootInfoValidationError::MemoryRegionMustBeNonEmpty)
                .summary_detail(),
            "empty-memory-region"
        );
    }

    #[test]
    fn limine_boot_error_locator_codes_cover_remaining_refusal_families() {
        assert_eq!(LimineBootError::MissingBaseRevision.locator_status_code(), 0x01);
        assert_eq!(
            LimineBootError::UnsupportedBaseRevision { loaded: Some(3) }.locator_status_code(),
            0x02
        );
        assert_eq!(
            LimineBootError::MissingResponse("higher-half direct map").locator_status_code(),
            0x11
        );
        assert_eq!(
            LimineBootError::MissingResponse("executable address").locator_status_code(),
            0x12
        );
        assert_eq!(
            LimineBootError::MissingResponse("framebuffer").locator_status_code(),
            0x1f
        );
        assert_eq!(
            LimineBootError::TooManyMemoryRegions {
                count: 257,
                capacity: 256,
            }
            .locator_status_code(),
            0x20
        );
        assert_eq!(
            LimineBootError::InvalidModulePathUtf8 { index: 0 }.locator_status_code(),
            0x31
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(
                BootInfoValidationError::KernelRangeMustBeKernelImage
            )
            .locator_status_code(),
            0x41
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(
                BootInfoValidationError::KernelRangeMustBePageAligned
            )
            .locator_status_code(),
            0x42
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(BootInfoValidationError::KernelRangeMustBeNonEmpty)
                .locator_status_code(),
            0x43
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(
                BootInfoValidationError::MemoryRegionMustBePageAligned
            )
            .locator_status_code(),
            0x44
        );
        assert_eq!(
            LimineBootError::InvalidBootInfo(BootInfoValidationError::MemoryRegionMustBeNonEmpty)
                .locator_status_code(),
            0x45
        );
    }

    #[test]
    fn report_limine_failure_preserves_remaining_refusal_details_and_status_codes() {
        let cases = [
            (
                LimineBootError::MissingBaseRevision,
                "missing-base-revision",
                0x01,
            ),
            (
                LimineBootError::UnsupportedBaseRevision { loaded: Some(3) },
                "unsupported-base-revision",
                0x02,
            ),
            (
                LimineBootError::MissingResponse("higher-half direct map"),
                "missing-hhdm",
                0x11,
            ),
            (
                LimineBootError::MissingResponse("executable address"),
                "missing-executable-address",
                0x12,
            ),
            (
                LimineBootError::TooManyMemoryRegions {
                    count: 257,
                    capacity: 256,
                },
                "too-many-memory-regions",
                0x20,
            ),
            (
                LimineBootError::InvalidModulePathUtf8 { index: 0 },
                "invalid-module-path-utf8",
                0x31,
            ),
            (
                LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::KernelRangeMustBeKernelImage,
                ),
                "invalid-kernel-range-kind",
                0x41,
            ),
            (
                LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::KernelRangeMustBePageAligned,
                ),
                "invalid-kernel-range-alignment",
                0x42,
            ),
            (
                LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::KernelRangeMustBeNonEmpty,
                ),
                "empty-kernel-range",
                0x43,
            ),
            (
                LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::MemoryRegionMustBePageAligned,
                ),
                "invalid-memory-region-alignment",
                0x44,
            ),
            (
                LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::MemoryRegionMustBeNonEmpty,
                ),
                "empty-memory-region",
                0x45,
            ),
        ];

        for (error, detail, status) in cases {
            let reported = report_limine_failure(error);
            assert_eq!(reported.summary_detail(), detail);
            assert_eq!(reported.locator_status_code(), status);
        }
    }
}
