//! Canonical subsystem role:
//! - subsystem: first-user launch assembly
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-stage materialization of the first native user
//!   process for handoff into the real system path
//!
//! Canonical contract families handled here:
//! - first-user image materialization contracts
//! - bootstrap stack assembly contracts
//! - user launch plan construction contracts
//!
//! This module may assemble boot-stage user launch state, but it must not
//! redefine long-term process or VM truth that belongs to `kernel-core`.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::mem::size_of;
use core::ptr;
use core::slice;

use ngos_user_abi::bootstrap::{BootOutcomePolicy, build_initial_stack};
use ngos_user_abi::{
    AT_ENTRY, AT_PAGESZ, Amd64UserEntryRegisters, AuxvEntry, BOOT_ARG_FLAG,
    BOOT_ENV_CMDLINE_PREFIX, BOOT_ENV_CPU_BOOT_SEED_PREFIX, BOOT_ENV_CPU_HW_PROVIDER_PREFIX,
    BOOT_ENV_CPU_SAVE_AREA_PREFIX, BOOT_ENV_CPU_XCR0_PREFIX, BOOT_ENV_CPU_XSAVE_PREFIX,
    BOOT_ENV_MARKER, BOOT_ENV_MODULE_LEN_PREFIX, BOOT_ENV_MODULE_PHYS_END_PREFIX,
    BOOT_ENV_MODULE_PHYS_START_PREFIX, BOOT_ENV_MODULE_PREFIX, BOOT_ENV_OUTCOME_POLICY_PREFIX,
    BOOT_ENV_PROOF_PREFIX, BOOT_ENV_PROTOCOL_PREFIX, BootstrapArgs, CWD_ENV_PREFIX,
    FRAMEBUFFER_BPP_ENV_PREFIX, FRAMEBUFFER_HEIGHT_ENV_PREFIX, FRAMEBUFFER_PITCH_ENV_PREFIX,
    FRAMEBUFFER_PRESENT_ENV_PREFIX, FRAMEBUFFER_WIDTH_ENV_PREFIX, IMAGE_BASE_ENV_PREFIX,
    IMAGE_PATH_ENV_PREFIX, KERNEL_PHYS_END_ENV_PREFIX, KERNEL_PHYS_START_ENV_PREFIX,
    MEMORY_REGION_COUNT_ENV_PREFIX, PHDR_ENV_PREFIX, PHENT_ENV_PREFIX, PHNUM_ENV_PREFIX,
    PHYSICAL_MEMORY_OFFSET_ENV_PREFIX, PROCESS_NAME_ENV_PREFIX, ROOT_MOUNT_NAME_ENV_PREFIX,
    ROOT_MOUNT_PATH_ENV_PREFIX, RSDP_ENV_PREFIX, SESSION_ENV_MARKER,
    SESSION_ENV_OUTCOME_POLICY_PREFIX, SESSION_ENV_PROTOCOL_PREFIX, STACK_TOP_ENV_PREFIX,
    USABLE_MEMORY_BYTES_ENV_PREFIX,
};

/// Prefix pentru variabila de mediu care semnalează că compat layer-ul este prezent.
pub const COMPAT_LAYER_ENV_KEY: &str = "NGOS_COMPAT_LAYER=unified";
/// Versiunea compat layer-ului expusă în bootstrap.
pub const COMPAT_VERSION_ENV_KEY: &str = "NGOS_COMPAT_VERSION=1";
use platform_hal::{AddressSpaceManager, CachePolicy, MemoryPermissions, PageMapping};
use platform_x86_64::user_mode::{UserAddressSpaceMapper, UserModeError, UserModeLaunchPlan};
use platform_x86_64::{
    BootMemoryRegionKind, BootProtocol, BootstrapPageTableBuilder, EarlyFrameAllocator,
    MaterializedAddressSpace, PAGE_SIZE_2M, PAGE_SIZE_4K, PageTable, PageTableBuildStats,
    PagingBuildOptions, X86_64Platform, align_down, align_up,
};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};
use crate::paging::{ActivePageTables, PagingBringupError};
use crate::user_syscall::seed_bootstrap_process_metadata;
use crate::{EarlyBootState, serial};
#[path = "user_process_bootstrap.rs"]
mod user_process_bootstrap;
use user_process_bootstrap::build_bootstrap_inputs;
#[path = "user_process_elf.rs"]
mod user_process_elf;
use user_process_elf::parse_user_elf;

const USER_MODULE_NAME: &str = "ngos-userland-native";
const MAX_USER_SEGMENTS: usize = 8;
pub(crate) const USER_STACK_RESERVE_BYTES: u64 = 128 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserProcessError {
    ModuleNotFound,
    ModuleRangeOverflow,
    InvalidElf,
    UnsupportedElf,
    SegmentOverflow,
    TooManySegments { count: usize, capacity: usize },
    StackBuild,
    EnterState,
}

#[derive(Clone)]
pub struct PreparedUserProcess<'a> {
    pub module_name: &'a str,
    pub entry_point: u64,
    pub boot_outcome_policy: BootOutcomePolicy,
    pub plan: UserModeLaunchPlan,
    segments: Vec<UserImageSegment<'a>>,
}

#[derive(Clone)]
struct UserImageSegment<'a> {
    mapping: PageMapping,
    file_bytes: &'a [u8],
    copy_offset: usize,
}

pub struct BootstrapUserMapper<'a, 'b, const N: usize> {
    paging: &'a ActivePageTables,
    allocator: &'a mut EarlyFrameAllocator<N>,
    segments: &'b [UserImageSegment<'b>],
    platform: X86_64Platform,
    address_space_id: platform_hal::AddressSpaceId,
}

impl<'a, 'b, const N: usize> BootstrapUserMapper<'a, 'b, N> {
    fn new(
        paging: &'a ActivePageTables,
        allocator: &'a mut EarlyFrameAllocator<N>,
        segments: &'b [UserImageSegment<'b>],
    ) -> Result<Self, UserModeError> {
        let mut platform = X86_64Platform::default();
        let address_space_id = platform
            .create_address_space()
            .map_err(|_| UserModeError::MappingFailure)?;
        Ok(Self {
            paging,
            allocator,
            segments,
            platform,
            address_space_id,
        })
    }
}

impl<const N: usize> UserAddressSpaceMapper for BootstrapUserMapper<'_, '_, N> {
    fn map_user_pages(
        &mut self,
        mapping: PageMapping,
        initial_bytes: Option<&[u8]>,
    ) -> Result<(), UserModeError> {
        let (init, init_offset) = if let Some(bytes) = initial_bytes {
            (bytes, 0usize)
        } else {
            let segment = self
                .segments
                .iter()
                .find(|segment| {
                    segment.mapping.vaddr == mapping.vaddr && segment.mapping.len == mapping.len
                })
                .ok_or(UserModeError::MappingFailure)?;
            (segment.file_bytes, segment.copy_offset)
        };

        let frame_count = page_count_for_len(mapping.len)?;
        let frames = self
            .allocator
            .allocate_frames(frame_count)
            .map_err(|_| UserModeError::MappingFailure)?;
        let bytes_ptr = physical_bytes_ptr(self.paging, frames.start)?;
        unsafe {
            ptr::write_bytes(bytes_ptr, 0, mapping.len as usize);
        }
        let end = init_offset.saturating_add(init.len());
        if end > mapping.len as usize {
            return Err(UserModeError::MappingFailure);
        }
        unsafe {
            ptr::copy_nonoverlapping(init.as_ptr(), bytes_ptr.add(init_offset), init.len());
        }

        let physical_mapping = PageMapping {
            paddr: frames.start,
            ..mapping
        };
        self.platform
            .map(self.address_space_id, physical_mapping)
            .map_err(|_| UserModeError::MappingFailure)?;
        serial::print(format_args!(
            "ngos/x86_64: user map vaddr={:#x} len={:#x} phys={:#x} frames={}\n",
            mapping.vaddr, mapping.len, frames.start, frames.frame_count
        ));
        Ok(())
    }

    fn activate_user_address_space(&mut self) -> Result<(), UserModeError> {
        self.platform
            .activate_address_space(self.address_space_id)
            .map_err(|_| UserModeError::MappingFailure)?;
        let materialized = materialize_boot_user_address_space(
            self.paging,
            self.allocator,
            &self.platform,
            self.address_space_id,
        )?;
        self.paging
            .activate_root(materialized.root_phys)
            .map_err(map_paging_bringup_error)?;
        serial::print(format_args!(
            "ngos/x86_64: user address space active asid={} root={:#x} tables={} maps={}\n",
            materialized.id.raw(),
            materialized.root_phys,
            materialized.stats.table_pages_used,
            materialized.stats.mapping_regions
        ));
        Ok(())
    }
}

pub fn prepare_user_launch(
    state: &EarlyBootState<'static>,
) -> Result<PreparedUserProcess<'static>, UserProcessError> {
    boot_locator::event(
        BootLocatorStage::User,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x540,
        BootPayloadLabel::Count,
        state.boot_info.modules.len() as u64,
        BootPayloadLabel::None,
        0,
    );
    let module = state
        .boot_info
        .modules
        .iter()
        .copied()
        .find(|module| module.name.ends_with(USER_MODULE_NAME))
        .ok_or(UserProcessError::ModuleNotFound)?;

    let module_end = module
        .physical_start
        .checked_add(module.len)
        .ok_or(UserProcessError::ModuleRangeOverflow)?;
    let virt_start = state
        .boot_info
        .physical_memory_offset
        .checked_add(module.physical_start)
        .ok_or(UserProcessError::ModuleRangeOverflow)?;
    let virt_end = state
        .boot_info
        .physical_memory_offset
        .checked_add(module_end)
        .ok_or(UserProcessError::ModuleRangeOverflow)?;
    let module_len = virt_end
        .checked_sub(virt_start)
        .ok_or(UserProcessError::ModuleRangeOverflow)? as usize;

    let module_bytes = unsafe { core::slice::from_raw_parts(virt_start as *const u8, module_len) };
    boot_locator::event(
        BootLocatorStage::User,
        BootLocatorKind::Memory,
        BootLocatorSeverity::Info,
        0x550,
        BootPayloadLabel::Address,
        virt_start,
        BootPayloadLabel::Length,
        module_len as u64,
    );
    let parsed = parse_user_elf(module.name, module_bytes)?;

    let bootstrap_inputs = build_bootstrap_inputs(
        &state.boot_info,
        module,
        parsed.entry_point,
        parsed.base_addr,
        parsed.stack_top,
        parsed.phdr_addr,
        parsed.phent_size,
        parsed.phnum,
    );
    let argv = bootstrap_inputs
        .argv
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let envp = bootstrap_inputs
        .envp
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    seed_bootstrap_process_metadata(module.name, "/", "/", &argv, &bootstrap_inputs.envp);
    let bootstrap = BootstrapArgs::new(&argv, &envp, &bootstrap_inputs.auxv);
    let stack_image = build_initial_stack(parsed.stack_top as usize, &bootstrap)
        .map_err(|_| UserProcessError::StackBuild)?;
    let stack_mapping_base = align_down(
        (stack_image.stack_base as u64).saturating_sub(USER_STACK_RESERVE_BYTES),
        PAGE_SIZE_4K,
    );
    let stack_prefix = (stack_image.stack_base as u64).saturating_sub(stack_mapping_base) as usize;
    let mut stack_bytes = vec![0u8; stack_prefix];
    stack_bytes.extend_from_slice(&stack_image.bytes);
    let entry_rsp = stack_image.stack_base.saturating_sub(size_of::<usize>());
    let registers = Amd64UserEntryRegisters::from_start_frame(
        parsed.entry_point as usize,
        entry_rsp,
        stack_image.start_frame,
    );
    let argv_words = unsafe {
        slice::from_raw_parts(
            stack_image
                .bytes
                .as_ptr()
                .add(size_of::<usize>())
                .cast::<usize>(),
            bootstrap.argc,
        )
    };
    serial::print(format_args!(
        "ngos/x86_64: user start-frame argc={} argv={:#x} envp={:#x} auxv={:#x} argv0={:#x} argv1={:#x}\n",
        stack_image.start_frame.argc,
        stack_image.start_frame.argv as usize,
        stack_image.start_frame.envp as usize,
        stack_image.start_frame.auxv as usize,
        argv_words.first().copied().unwrap_or(0),
        argv_words.get(1).copied().unwrap_or(0),
    ));
    for (index, value) in stack_image.envp_addrs.iter().copied().take(12).enumerate() {
        serial::print(format_args!(
            "ngos/x86_64: user envp[{}]={:#x}\n",
            index, value
        ));
    }
    boot_locator::event(
        BootLocatorStage::User,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x560,
        BootPayloadLabel::Rip,
        parsed.entry_point,
        BootPayloadLabel::Address,
        stack_image.stack_top as u64,
    );
    if registers.rip == 0 || registers.rsp == 0 {
        return Err(UserProcessError::EnterState);
    }

    let stack_mapping = PageMapping {
        vaddr: stack_mapping_base,
        paddr: 0,
        len: align_up(stack_bytes.len() as u64, PAGE_SIZE_4K),
        perms: MemoryPermissions::read_write(),
        cache: CachePolicy::WriteBack,
        user: true,
    };

    Ok(PreparedUserProcess {
        module_name: module.name,
        entry_point: parsed.entry_point,
        boot_outcome_policy: bootstrap_inputs.boot_outcome_policy,
        plan: UserModeLaunchPlan {
            registers,
            image_mappings: parsed
                .segments
                .iter()
                .map(|segment| segment.mapping)
                .collect(),
            stack_mapping,
            stack_bytes,
        },
        segments: parsed.segments,
    })
}

pub(crate) fn prepare_spawned_same_image_launch(
    boot_info: &platform_x86_64::BootInfo<'static>,
    pid: u64,
    process_name: &str,
    image_path: &str,
    cwd: &str,
    root: &str,
    argv: &[String],
    envp: &[String],
) -> Result<UserModeLaunchPlan, UserProcessError> {
    let module = boot_info
        .modules
        .iter()
        .copied()
        .find(|module| module.name.ends_with(USER_MODULE_NAME))
        .ok_or(UserProcessError::ModuleNotFound)?;
    let module_end = module
        .physical_start
        .checked_add(module.len)
        .ok_or(UserProcessError::ModuleRangeOverflow)?;
    let virt_start = boot_info
        .physical_memory_offset
        .checked_add(module.physical_start)
        .ok_or(UserProcessError::ModuleRangeOverflow)?;
    let virt_end = boot_info
        .physical_memory_offset
        .checked_add(module_end)
        .ok_or(UserProcessError::ModuleRangeOverflow)?;
    let module_len = virt_end
        .checked_sub(virt_start)
        .ok_or(UserProcessError::ModuleRangeOverflow)? as usize;
    let module_bytes = unsafe { core::slice::from_raw_parts(virt_start as *const u8, module_len) };
    let parsed = parse_user_elf(module.name, module_bytes)?;

    let stack_slot_index = pid.saturating_sub(1);
    let stack_top = parsed
        .stack_top
        .checked_sub(USER_STACK_RESERVE_BYTES.saturating_mul(stack_slot_index))
        .ok_or(UserProcessError::StackBuild)?;
    let mut bootstrap_inputs = build_bootstrap_inputs(
        boot_info,
        module,
        parsed.entry_point,
        parsed.base_addr,
        stack_top,
        parsed.phdr_addr,
        parsed.phent_size,
        parsed.phnum,
    );
    bootstrap_inputs.argv = if argv.is_empty() {
        vec![image_path.to_string()]
    } else {
        argv.to_vec()
    };
    bootstrap_inputs.envp = merge_spawned_bootstrap_env(
        bootstrap_inputs.envp,
        process_name,
        image_path,
        cwd,
        root,
        envp,
    );

    let argv_refs = bootstrap_inputs
        .argv
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let envp_refs = bootstrap_inputs
        .envp
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let bootstrap = BootstrapArgs::new(&argv_refs, &envp_refs, &bootstrap_inputs.auxv);
    let stack_image = build_initial_stack(stack_top as usize, &bootstrap)
        .map_err(|_| UserProcessError::StackBuild)?;
    let stack_mapping_base = align_down(
        (stack_image.stack_base as u64).saturating_sub(USER_STACK_RESERVE_BYTES),
        PAGE_SIZE_4K,
    );
    let stack_prefix = (stack_image.stack_base as u64).saturating_sub(stack_mapping_base) as usize;
    let mut stack_bytes = vec![0u8; stack_prefix];
    stack_bytes.extend_from_slice(&stack_image.bytes);
    let entry_rsp = stack_image.stack_base.saturating_sub(size_of::<usize>());
    let registers = Amd64UserEntryRegisters::from_start_frame(
        parsed.entry_point as usize,
        entry_rsp,
        stack_image.start_frame,
    );
    if registers.rip == 0 || registers.rsp == 0 {
        return Err(UserProcessError::EnterState);
    }

    Ok(UserModeLaunchPlan {
        registers,
        image_mappings: Vec::new(),
        stack_mapping: PageMapping {
            vaddr: stack_mapping_base,
            paddr: 0,
            len: align_up(stack_bytes.len() as u64, PAGE_SIZE_4K),
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: true,
        },
        stack_bytes,
    })
}

pub fn mapper_for<'a, 'b, const N: usize>(
    paging: &'a ActivePageTables,
    allocator: &'a mut EarlyFrameAllocator<N>,
    prepared: &'b PreparedUserProcess<'b>,
) -> Result<BootstrapUserMapper<'a, 'b, N>, UserModeError> {
    BootstrapUserMapper::new(paging, allocator, &prepared.segments)
}

fn page_count_for_len(len: u64) -> Result<usize, UserModeError> {
    if len == 0 || !len.is_multiple_of(PAGE_SIZE_4K) {
        return Err(UserModeError::MappingFailure);
    }
    usize::try_from(len / PAGE_SIZE_4K).map_err(|_| UserModeError::MappingFailure)
}

fn physical_bytes_ptr(
    paging: &ActivePageTables,
    physical_start: u64,
) -> Result<*mut u8, UserModeError> {
    paging
        .physical_memory_offset()
        .checked_add(physical_start)
        .map(|virt| virt as *mut u8)
        .ok_or(UserModeError::MappingFailure)
}

fn page_table_slice<'a, const N: usize>(
    paging: &'a ActivePageTables,
    allocator: &'a mut EarlyFrameAllocator<N>,
    table_count: usize,
) -> Result<(u64, &'a mut [PageTable]), UserModeError> {
    let frames = allocator
        .allocate_frames(table_count)
        .map_err(|_| UserModeError::MappingFailure)?;
    let tables_ptr = physical_bytes_ptr(paging, frames.start)? as *mut PageTable;
    unsafe {
        ptr::write_bytes(
            tables_ptr.cast::<u8>(),
            0,
            table_count.saturating_mul(size_of::<PageTable>()),
        );
        Ok((
            tables_ptr as u64,
            slice::from_raw_parts_mut(tables_ptr, table_count),
        ))
    }
}

fn seed_kernel_root_entries(
    paging: &ActivePageTables,
    target: &mut PageTable,
) -> Result<(), UserModeError> {
    let current_root_ptr = physical_bytes_ptr(paging, paging.root_phys())? as *const PageTable;
    let current_root = unsafe { &*current_root_ptr };
    target.entries[256..].copy_from_slice(&current_root.entries[256..]);
    Ok(())
}

fn estimate_page_table_count(mappings: &[PageMapping]) -> Result<usize, UserModeError> {
    let mut total = 1usize;
    for mapping in mappings {
        let pt_count = mapping.len.div_ceil(PAGE_SIZE_2M);
        let pt_count = usize::try_from(pt_count).map_err(|_| UserModeError::MappingFailure)?;
        total = total
            .checked_add(3)
            .and_then(|value| value.checked_add(pt_count))
            .ok_or(UserModeError::MappingFailure)?;
    }
    Ok(total.max(4))
}

fn materialize_boot_user_address_space<const N: usize>(
    paging: &ActivePageTables,
    allocator: &mut EarlyFrameAllocator<N>,
    platform: &X86_64Platform,
    address_space_id: platform_hal::AddressSpaceId,
) -> Result<MaterializedAddressSpace, UserModeError> {
    let layout = platform
        .address_space_layout(address_space_id)
        .map_err(|_| UserModeError::MappingFailure)?;
    let table_count = estimate_page_table_count(&layout.mappings)?;
    let (tables_addr, tables) = page_table_slice(paging, allocator, table_count)?;
    seed_kernel_root_entries(paging, &mut tables[0])?;

    let phys_base = tables_addr
        .checked_sub(paging.physical_memory_offset())
        .ok_or(UserModeError::MappingFailure)?;
    let mut builder = MaybeUninit::<BootstrapPageTableBuilder<'_>>::uninit();
    unsafe {
        BootstrapPageTableBuilder::write_prezeroed(builder.as_mut_ptr(), tables, phys_base)
            .map_err(map_materialization_error)?;
    }
    let mut builder = unsafe { builder.assume_init() };
    let mut stats = PageTableBuildStats::default();
    for mapping in &layout.mappings {
        builder
            .map_region(*mapping, PagingBuildOptions::default(), &mut stats)
            .map_err(map_materialization_error)?;
    }
    stats.table_pages_used = builder.table_pages_used();
    Ok(MaterializedAddressSpace {
        id: address_space_id,
        root_phys: builder.root_phys(),
        stats,
    })
}

fn map_materialization_error(error: platform_x86_64::PagingError) -> UserModeError {
    let _ = error;
    UserModeError::MappingFailure
}

fn map_paging_bringup_error(error: PagingBringupError) -> UserModeError {
    let _ = error;
    UserModeError::MappingFailure
}

fn boot_protocol_name(protocol: BootProtocol) -> &'static str {
    match protocol {
        BootProtocol::Limine => "limine",
        BootProtocol::Multiboot2 => "multiboot2",
        BootProtocol::Uefi => "uefi",
        BootProtocol::LoaderDefined => "loader-defined",
    }
}

fn merge_spawned_bootstrap_env(
    mut base: Vec<String>,
    process_name: &str,
    image_path: &str,
    cwd: &str,
    root: &str,
    extra_envp: &[String],
) -> Vec<String> {
    upsert_bootstrap_env_value(&mut base, PROCESS_NAME_ENV_PREFIX, process_name);
    upsert_bootstrap_env_value(&mut base, IMAGE_PATH_ENV_PREFIX, image_path);
    upsert_bootstrap_env_value(&mut base, CWD_ENV_PREFIX, cwd);
    upsert_bootstrap_env_value(&mut base, ROOT_MOUNT_PATH_ENV_PREFIX, root);
    for entry in extra_envp {
        upsert_bootstrap_env_assignment(&mut base, entry);
    }
    base
}

fn upsert_bootstrap_env_value(envp: &mut Vec<String>, prefix: &str, value: &str) {
    let entry = format!("{prefix}{value}");
    if let Some(index) = envp
        .iter()
        .position(|candidate| candidate.starts_with(prefix))
    {
        envp[index] = entry;
    } else {
        envp.push(entry);
    }
}

fn upsert_bootstrap_env_assignment(envp: &mut Vec<String>, assignment: &str) {
    let Some(eq_index) = assignment.find('=') else {
        envp.push(assignment.to_string());
        return;
    };
    let prefix = &assignment[..=eq_index];
    if let Some(index) = envp
        .iter()
        .position(|candidate| candidate.starts_with(prefix))
    {
        envp[index] = assignment.to_string();
    } else {
        envp.push(assignment.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_x86_64::{
        BootInfo, BootMemoryRegion, BootMemoryRegionKind, BootModule, FramebufferInfo,
    };

    #[test]
    fn boot_protocol_names_are_stable() {
        assert_eq!(boot_protocol_name(BootProtocol::Limine), "limine");
        assert_eq!(boot_protocol_name(BootProtocol::Multiboot2), "multiboot2");
        assert_eq!(boot_protocol_name(BootProtocol::Uefi), "uefi");
        assert_eq!(
            boot_protocol_name(BootProtocol::LoaderDefined),
            "loader-defined"
        );
    }

    #[test]
    fn bootstrap_inputs_include_protocol_module_and_entry_metadata() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::boot_locator::reset();
        let memory_regions = [
            BootMemoryRegion {
                start: 0x100000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::KernelImage,
            },
            BootMemoryRegion {
                start: 0x200000,
                len: 0x800000,
                kind: BootMemoryRegionKind::Usable,
            },
        ];
        let modules = [BootModule {
            name: USER_MODULE_NAME,
            physical_start: 0x200000,
            len: 0x3000,
        }];
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0"),
            rsdp: Some(0xdead_beef),
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: Some(FramebufferInfo {
                physical_start: 0xb8000,
                width: 1920,
                height: 1080,
                pitch: 7680,
                bpp: 32,
                red_mask_size: 8,
                red_mask_shift: 16,
                green_mask_size: 8,
                green_mask_shift: 8,
                blue_mask_size: 8,
                blue_mask_shift: 0,
            }),
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 4096, true, true, true, true, true, true, true, 0xe7,
        );
        crate::cpu_runtime_status::record_probe(true, true, true, 4096, 0, 0x1234_5678);
        crate::cpu_runtime_status::record_hardware_provider_install(true, false, 0);

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert_eq!(
            bootstrap.argv,
            vec![USER_MODULE_NAME.into(), BOOT_ARG_FLAG.into()]
        );
        assert!(bootstrap.envp.iter().any(|entry| entry == BOOT_ENV_MARKER));
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == SESSION_ENV_MARKER)
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROTOCOL=limine")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_SESSION_PROTOCOL=kernel-launch")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_MODULE=ngos-userland-native")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_MODULE_LEN=12288")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_MODULE_PHYS_START=0x200000")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_MODULE_PHYS_END=0x203000")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_PROCESS_NAME=ngos-userland-native")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_IMAGE_PATH=ngos-userland-native")
        );
        assert!(bootstrap.envp.iter().any(|entry| entry == "NGOS_CWD=/"));
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_ROOT_MOUNT_PATH=/")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_ROOT_MOUNT_NAME=rootfs")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_CPU_XSAVE=1")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_CPU_SAVE_AREA=4096")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_CPU_XCR0=0xe7")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_CPU_BOOT_SEED=0x12345678")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_CPU_HW_PROVIDER=1")
        );
        let locator = crate::boot_locator::snapshot();
        assert_eq!(locator.stage, crate::boot_locator::BootLocatorStage::User);
        assert_eq!(locator.checkpoint, 0x565);
        assert_eq!(locator.payload0, 4096);
        assert_eq!(locator.payload1, 3);
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_IMAGE_BASE=0x400000")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_STACK_TOP=0x7fffffff0000")
        );
        assert!(bootstrap.envp.iter().any(|entry| entry == "NGOS_PHDR=0x40"));
        assert!(bootstrap.envp.iter().any(|entry| entry == "NGOS_PHENT=56"));
        assert!(bootstrap.envp.iter().any(|entry| entry == "NGOS_PHNUM=2"));
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_MEMORY_REGION_COUNT=2")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_USABLE_MEMORY_BYTES=8388608")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_PHYSICAL_MEMORY_OFFSET=0x0")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_KERNEL_PHYS_START=0x100000")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_KERNEL_PHYS_END=0x101000")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_RSDP=0xdeadbeef")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_FRAMEBUFFER_PRESENT=1")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_FRAMEBUFFER_WIDTH=1920")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_FRAMEBUFFER_HEIGHT=1080")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_FRAMEBUFFER_PITCH=7680")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_FRAMEBUFFER_BPP=32")
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_CMDLINE=console=ttyS0")
        );
        assert_eq!(
            bootstrap.auxv,
            vec![
                AuxvEntry {
                    key: AT_PAGESZ,
                    value: PAGE_SIZE_4K as usize,
                },
                AuxvEntry {
                    key: AT_ENTRY,
                    value: 0x401000,
                },
            ]
        );
        assert_eq!(
            bootstrap.boot_outcome_policy,
            BootOutcomePolicy::RequireZeroExit
        );
    }

    #[test]
    fn bootstrap_inputs_can_select_allow_any_exit_from_cmdline() {
        let memory_regions = [BootMemoryRegion {
            start: 0x200000,
            len: 0x800000,
            kind: BootMemoryRegionKind::Usable,
        }];
        let modules = [BootModule {
            name: USER_MODULE_NAME,
            physical_start: 0x200000,
            len: 0x3000,
        }];
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot_outcome=allow-any-exit"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert_eq!(
            bootstrap.boot_outcome_policy,
            BootOutcomePolicy::AllowAnyExit
        );
        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_OUTCOME_POLICY=allow-any-exit")
        );
    }

    #[test]
    fn bootstrap_inputs_propagate_supported_boot_proof_from_cmdline() {
        let memory_regions = [BootMemoryRegion {
            start: 0x200000,
            len: 0x800000,
            kind: BootMemoryRegionKind::Usable,
        }];
        let modules = [BootModule {
            name: USER_MODULE_NAME,
            physical_start: 0x200000,
            len: 0x3000,
        }];
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=wasm"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROOF=wasm")
        );

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=vm"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROOF=vm")
        );

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=compat-gfx"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROOF=compat-gfx")
        );

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=compat-loader"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROOF=compat-loader")
        );

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=compat-abi"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROOF=compat-abi")
        );

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=compat-foreign"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROOF=compat-foreign")
        );

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=bus"),
            rsdp: None,
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: memory_regions[0],
        };

        let bootstrap = build_bootstrap_inputs(
            &boot_info,
            modules[0],
            0x401000,
            0x400000,
            0x0000_7fff_ffff_0000,
            0x40,
            56,
            2,
        );

        assert!(
            bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_BOOT_PROOF=bus")
        );
    }

    #[test]
    fn spawned_same_image_launch_uses_pid_stack_slot_and_overrides_process_env() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        let memory_regions = [
            BootMemoryRegion {
                start: 0x100000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::KernelImage,
            },
            BootMemoryRegion {
                start: 0x200000,
                len: 0x800000,
                kind: BootMemoryRegionKind::Usable,
            },
        ];
        let mut module_bytes = vec![0u8; 0x200];
        module_bytes[..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        module_bytes[4] = 2;
        module_bytes[5] = 1;
        module_bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
        module_bytes[18..20].copy_from_slice(&0x3eu16.to_le_bytes());
        module_bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        module_bytes[24..32].copy_from_slice(&0x401000u64.to_le_bytes());
        module_bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        module_bytes[52..54].copy_from_slice(
            &(size_of::<super::user_process_elf::Elf64Header>() as u16).to_le_bytes(),
        );
        module_bytes[54..56].copy_from_slice(
            &(size_of::<super::user_process_elf::Elf64ProgramHeader>() as u16).to_le_bytes(),
        );
        module_bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
        let phoff = 64usize;
        module_bytes[phoff..phoff + 4].copy_from_slice(&1u32.to_le_bytes());
        module_bytes[phoff + 4..phoff + 8].copy_from_slice(&0x5u32.to_le_bytes());
        module_bytes[phoff + 8..phoff + 16].copy_from_slice(&0x100u64.to_le_bytes());
        module_bytes[phoff + 16..phoff + 24].copy_from_slice(&0x401000u64.to_le_bytes());
        module_bytes[phoff + 32..phoff + 40].copy_from_slice(&16u64.to_le_bytes());
        module_bytes[phoff + 40..phoff + 48].copy_from_slice(&16u64.to_le_bytes());
        module_bytes[0x100..0x110].copy_from_slice(&[0x90; 16]);
        let module_len = module_bytes.len() as u64;
        let module_bytes = alloc::boxed::Box::leak(module_bytes.into_boxed_slice());
        let modules = [BootModule {
            name: USER_MODULE_NAME,
            physical_start: 0x200000,
            len: module_len,
        }];
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: Some("console=ttyS0 ngos.boot.proof=compat-abi"),
            rsdp: Some(0xdead_beef),
            memory_regions: &memory_regions,
            modules: &modules,
            framebuffer: None,
            physical_memory_offset: module_bytes.as_ptr() as u64 - modules[0].physical_start,
            kernel_phys_range: memory_regions[0],
        };
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 4096, true, true, true, true, true, true, true, 0xe7,
        );
        crate::cpu_runtime_status::record_probe(true, true, true, 4096, 0, 0x1234_5678);
        crate::cpu_runtime_status::record_hardware_provider_install(true, false, 0);

        let argv = vec![
            String::from("/bin/ngos-userland-native"),
            String::from("--compat-proc-probe"),
        ];
        let envp = vec![
            String::from("NGOS_PROCESS_NAME=proc-exec-child"),
            String::from("NGOS_IMAGE_PATH=/bin/ngos-userland-native"),
            String::from("NGOS_CWD=/workers"),
            String::from("NGOS_ROOT_MOUNT_PATH=/sandbox"),
            String::from("NGOS_COMPAT_EXPECT_CWD=/workers"),
        ];
        let plan = prepare_spawned_same_image_launch(
            &boot_info,
            2,
            "proc-exec-child",
            "/bin/ngos-userland-native",
            "/workers",
            "/sandbox",
            &argv,
            &envp,
        )
        .unwrap();

        assert_eq!(plan.image_mappings, Vec::new());
        assert_eq!(plan.registers.rip as u64, 0x401000);
        assert_eq!(plan.stack_mapping.vaddr, 0x0000_7fff_fffb_0000);
        let stack_text = core::str::from_utf8(&plan.stack_bytes).unwrap();
        assert!(stack_text.contains("/bin/ngos-userland-native"));
        assert!(stack_text.contains("--compat-proc-probe"));
        assert!(stack_text.contains("NGOS_PROCESS_NAME=proc-exec-child"));
        assert!(stack_text.contains("NGOS_IMAGE_PATH=/bin/ngos-userland-native"));
        assert!(stack_text.contains("NGOS_CWD=/workers"));
        assert!(stack_text.contains("NGOS_ROOT_MOUNT_PATH=/sandbox"));
        assert!(stack_text.contains("NGOS_BOOT_PROOF=compat-abi"));
    }
}
