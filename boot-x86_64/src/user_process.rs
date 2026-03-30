extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::mem::size_of;
use core::ptr;
use core::slice;

use ngos_user_abi::bootstrap::{BootOutcomePolicy, build_initial_stack};
use ngos_user_abi::{
    AT_ENTRY, AT_PAGESZ, Amd64UserEntryRegisters, AuxvEntry, BOOT_ARG_FLAG,
    BOOT_ENV_CMDLINE_PREFIX, BOOT_ENV_MARKER, BOOT_ENV_MODULE_LEN_PREFIX,
    BOOT_ENV_MODULE_PHYS_END_PREFIX, BOOT_ENV_MODULE_PHYS_START_PREFIX, BOOT_ENV_MODULE_PREFIX,
    BOOT_ENV_OUTCOME_POLICY_PREFIX, BOOT_ENV_PROOF_PREFIX, BOOT_ENV_PROTOCOL_PREFIX, BootstrapArgs,
    CWD_ENV_PREFIX, FRAMEBUFFER_BPP_ENV_PREFIX, FRAMEBUFFER_HEIGHT_ENV_PREFIX,
    FRAMEBUFFER_PITCH_ENV_PREFIX, FRAMEBUFFER_PRESENT_ENV_PREFIX, FRAMEBUFFER_WIDTH_ENV_PREFIX,
    IMAGE_BASE_ENV_PREFIX, IMAGE_PATH_ENV_PREFIX, KERNEL_PHYS_END_ENV_PREFIX,
    KERNEL_PHYS_START_ENV_PREFIX, MEMORY_REGION_COUNT_ENV_PREFIX, PHDR_ENV_PREFIX,
    PHENT_ENV_PREFIX, PHNUM_ENV_PREFIX, PHYSICAL_MEMORY_OFFSET_ENV_PREFIX, PROCESS_NAME_ENV_PREFIX,
    ROOT_MOUNT_NAME_ENV_PREFIX, ROOT_MOUNT_PATH_ENV_PREFIX, RSDP_ENV_PREFIX, STACK_TOP_ENV_PREFIX,
    USABLE_MEMORY_BYTES_ENV_PREFIX,
};
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
use crate::{EarlyBootState, serial};

const USER_MODULE_NAME: &str = "ngos-userland-native";
const MAX_USER_SEGMENTS: usize = 8;
const USER_STACK_RESERVE_BYTES: u64 = 16 * 1024;

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

struct BootBootstrapInputs {
    argv: Vec<String>,
    envp: Vec<String>,
    auxv: Vec<AuxvEntry>,
    boot_outcome_policy: BootOutcomePolicy,
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
    let registers = Amd64UserEntryRegisters::from_start_frame(
        parsed.entry_point as usize,
        stack_image.stack_base,
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

fn build_bootstrap_inputs(
    boot_info: &platform_x86_64::BootInfo<'_>,
    module: platform_x86_64::BootModule<'_>,
    entry_point: u64,
    image_base: u64,
    stack_top: u64,
    phdr_addr: u64,
    phent_size: u64,
    phnum: u64,
) -> BootBootstrapInputs {
    let argv = vec![String::from(USER_MODULE_NAME), String::from(BOOT_ARG_FLAG)];
    let module_phys_end = module.physical_start.saturating_add(module.len);
    let kernel_phys_end = boot_info
        .kernel_phys_range
        .start
        .saturating_add(boot_info.kernel_phys_range.len);
    let usable_bytes = boot_info
        .memory_regions
        .iter()
        .filter(|region| region.kind == BootMemoryRegionKind::Usable)
        .map(|region| region.len)
        .sum::<u64>();
    let mut envp = vec![
        String::from(BOOT_ENV_MARKER),
        format!(
            "{}{}",
            BOOT_ENV_PROTOCOL_PREFIX,
            boot_protocol_name(boot_info.protocol)
        ),
        format!("{}{}", BOOT_ENV_MODULE_PREFIX, module.name),
        format!("{}{}", BOOT_ENV_MODULE_LEN_PREFIX, module.len),
        format!(
            "{}{:#x}",
            BOOT_ENV_MODULE_PHYS_START_PREFIX, module.physical_start
        ),
        format!("{}{:#x}", BOOT_ENV_MODULE_PHYS_END_PREFIX, module_phys_end),
        format!("{}{}", PROCESS_NAME_ENV_PREFIX, USER_MODULE_NAME),
        format!("{}{}", IMAGE_PATH_ENV_PREFIX, module.name),
        format!("{}{}", CWD_ENV_PREFIX, "/"),
        format!("{}{}", ROOT_MOUNT_PATH_ENV_PREFIX, "/"),
        format!("{}{}", ROOT_MOUNT_NAME_ENV_PREFIX, "rootfs"),
        format!("{}{:#x}", IMAGE_BASE_ENV_PREFIX, image_base),
        format!("{}{:#x}", STACK_TOP_ENV_PREFIX, stack_top),
        format!("{}{:#x}", PHDR_ENV_PREFIX, phdr_addr),
        format!("{}{}", PHENT_ENV_PREFIX, phent_size),
        format!("{}{}", PHNUM_ENV_PREFIX, phnum),
        format!(
            "{}{}",
            MEMORY_REGION_COUNT_ENV_PREFIX,
            boot_info.memory_regions.len()
        ),
        format!("{}{}", USABLE_MEMORY_BYTES_ENV_PREFIX, usable_bytes),
        format!(
            "{}{:#x}",
            PHYSICAL_MEMORY_OFFSET_ENV_PREFIX, boot_info.physical_memory_offset
        ),
        format!(
            "{}{:#x}",
            KERNEL_PHYS_START_ENV_PREFIX, boot_info.kernel_phys_range.start
        ),
        format!("{}{:#x}", KERNEL_PHYS_END_ENV_PREFIX, kernel_phys_end),
    ];
    let boot_outcome_policy = boot_outcome_policy_from_boot_info(boot_info);
    envp.push(format!(
        "{}{}",
        BOOT_ENV_OUTCOME_POLICY_PREFIX,
        boot_outcome_policy_name(boot_outcome_policy)
    ));
    if let Some(command_line) = boot_info.command_line {
        envp.push(format!("{}{}", BOOT_ENV_CMDLINE_PREFIX, command_line));
        if let Some(proof) = boot_proof_from_command_line(command_line) {
            envp.push(format!("{}{}", BOOT_ENV_PROOF_PREFIX, proof));
        }
    }
    if let Some(rsdp) = boot_info.rsdp {
        envp.push(format!("{}{:#x}", RSDP_ENV_PREFIX, rsdp));
    }
    if let Some(framebuffer) = boot_info.framebuffer {
        envp.push(format!("{}1", FRAMEBUFFER_PRESENT_ENV_PREFIX));
        envp.push(format!(
            "{}{}",
            FRAMEBUFFER_WIDTH_ENV_PREFIX, framebuffer.width
        ));
        envp.push(format!(
            "{}{}",
            FRAMEBUFFER_HEIGHT_ENV_PREFIX, framebuffer.height
        ));
        envp.push(format!(
            "{}{}",
            FRAMEBUFFER_PITCH_ENV_PREFIX, framebuffer.pitch
        ));
        envp.push(format!("{}{}", FRAMEBUFFER_BPP_ENV_PREFIX, framebuffer.bpp));
    }

    BootBootstrapInputs {
        argv,
        envp,
        boot_outcome_policy,
        auxv: vec![
            AuxvEntry {
                key: AT_PAGESZ,
                value: PAGE_SIZE_4K as usize,
            },
            AuxvEntry {
                key: AT_ENTRY,
                value: entry_point as usize,
            },
        ],
    }
}

fn boot_outcome_policy_from_boot_info(
    boot_info: &platform_x86_64::BootInfo<'_>,
) -> BootOutcomePolicy {
    let command_line = match boot_info.command_line {
        Some(value) => value,
        None => return BootOutcomePolicy::RequireZeroExit,
    };
    if command_line
        .split_ascii_whitespace()
        .any(|token| token == "ngos.boot_outcome=allow-any-exit")
    {
        BootOutcomePolicy::AllowAnyExit
    } else {
        BootOutcomePolicy::RequireZeroExit
    }
}

fn boot_proof_from_command_line(command_line: &str) -> Option<&'static str> {
    for token in command_line.split_ascii_whitespace() {
        match token {
            "ngos.boot.proof=vfs" => return Some("vfs"),
            "ngos.boot.proof=wasm" => return Some("wasm"),
            _ => {}
        }
    }
    None
}

fn boot_outcome_policy_name(policy: BootOutcomePolicy) -> &'static str {
    match policy {
        BootOutcomePolicy::RequireZeroExit => "require-zero-exit",
        BootOutcomePolicy::AllowAnyExit => "allow-any-exit",
    }
}

struct ParsedUserElf<'a> {
    base_addr: u64,
    entry_point: u64,
    phdr_addr: u64,
    phent_size: u64,
    phnum: u64,
    stack_top: u64,
    segments: Vec<UserImageSegment<'a>>,
}

fn parse_user_elf<'a>(
    module_name: &'a str,
    image: &'a [u8],
) -> Result<ParsedUserElf<'a>, UserProcessError> {
    if image.len() < size_of::<Elf64Header>() {
        return Err(UserProcessError::InvalidElf);
    }

    let header = unsafe { ptr::read_unaligned(image.as_ptr().cast::<Elf64Header>()) };
    if header.e_ident[..4] != [0x7f, b'E', b'L', b'F']
        || header.e_ident[4] != 2
        || header.e_ident[5] != 1
    {
        return Err(UserProcessError::InvalidElf);
    }
    if header.e_machine != 0x3e || (header.e_type != 2 && header.e_type != 3) {
        return Err(UserProcessError::UnsupportedElf);
    }

    let ph_end = header
        .e_phoff
        .checked_add((header.e_phentsize as u64).saturating_mul(header.e_phnum as u64))
        .ok_or(UserProcessError::SegmentOverflow)?;
    if ph_end as usize > image.len()
        || header.e_phentsize as usize != size_of::<Elf64ProgramHeader>()
    {
        return Err(UserProcessError::InvalidElf);
    }

    let mut segments = Vec::new();
    for index in 0..header.e_phnum as usize {
        let ph_offset = header.e_phoff as usize + index * size_of::<Elf64ProgramHeader>();
        let ph = unsafe {
            ptr::read_unaligned(image.as_ptr().add(ph_offset).cast::<Elf64ProgramHeader>())
        };
        if ph.p_type != 1 || ph.p_memsz == 0 {
            continue;
        }
        if segments.len() >= MAX_USER_SEGMENTS {
            return Err(UserProcessError::TooManySegments {
                count: segments.len() + 1,
                capacity: MAX_USER_SEGMENTS,
            });
        }

        let file_end = ph
            .p_offset
            .checked_add(ph.p_filesz)
            .ok_or(UserProcessError::SegmentOverflow)?;
        if file_end as usize > image.len() || ph.p_filesz > ph.p_memsz {
            return Err(UserProcessError::InvalidElf);
        }

        let mapping_vaddr = align_down(ph.p_vaddr, PAGE_SIZE_4K);
        let copy_offset = (ph.p_vaddr - mapping_vaddr) as usize;
        let mapping_len = align_up(ph.p_memsz.saturating_add(copy_offset as u64), PAGE_SIZE_4K);
        let file_bytes = &image[ph.p_offset as usize..file_end as usize];
        segments.push(UserImageSegment {
            mapping: PageMapping {
                vaddr: mapping_vaddr,
                paddr: 0,
                len: mapping_len,
                perms: MemoryPermissions {
                    read: (ph.p_flags & 0x4) != 0,
                    write: (ph.p_flags & 0x2) != 0,
                    execute: (ph.p_flags & 0x1) != 0,
                },
                cache: CachePolicy::WriteBack,
                user: true,
            },
            file_bytes,
            copy_offset,
        });
    }

    segments.sort_by_key(|segment| segment.mapping.vaddr);
    if segments.is_empty() {
        return Err(UserProcessError::InvalidElf);
    }
    let base_addr = segments[0].mapping.vaddr;

    let stack_top = 0x0000_7fff_ffff_0000u64;
    serial::print(format_args!(
        "ngos/x86_64: exec userland-native module=\"{}\" entry={:#x} segments={}\n",
        module_name,
        header.e_entry,
        segments.len()
    ));

    Ok(ParsedUserElf {
        base_addr,
        entry_point: header.e_entry,
        phdr_addr: header.e_phoff,
        phent_size: header.e_phentsize as u64,
        phnum: header.e_phnum as u64,
        stack_top,
        segments,
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

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
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
                .any(|entry| entry == "NGOS_BOOT_PROTOCOL=limine")
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
    }
}
