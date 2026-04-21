//! Canonical subsystem role:
//! - subsystem: x86_64 user-mode platform mediation
//! - owner layer: platform mediation
//! - semantic owner: `platform-x86_64`
//! - truth path role: platform-specific user-mode launch and address-space
//!   mediation for the real x86 path
//!
//! Canonical contract families handled here:
//! - user-mode launch contracts
//! - user address-space mapping contracts
//! - platform user-mode validation contracts
//!
//! This module may mediate x86_64-specific user-mode mechanics, but it must
//! not redefine kernel process, VM, or runtime truth.

extern crate alloc;

use alloc::vec::Vec;
use ngos_user_abi::Amd64UserEntryRegisters;
use platform_hal::{AddressSpaceId, AddressSpaceManager, PageMapping};

use crate::{MaterializedAddressSpace, PageTable, X86_64Platform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserModeError {
    InvalidStackImage,
    InvalidEntryState,
    MappingFailure,
}

pub trait UserAddressSpaceMapper {
    fn map_user_pages(
        &mut self,
        mapping: PageMapping,
        initial_bytes: Option<&[u8]>,
    ) -> Result<(), UserModeError>;
    fn activate_user_address_space(&mut self) -> Result<(), UserModeError>;
}

struct UserLaunchPlanValidationAgent;

impl UserLaunchPlanValidationAgent {
    fn validate(plan: &UserModeLaunchPlan) -> Result<(), UserModeError> {
        if plan.registers.rip == 0 || plan.registers.rsp == 0 {
            return Err(UserModeError::InvalidEntryState);
        }
        if plan.stack_bytes.is_empty() {
            return Err(UserModeError::InvalidStackImage);
        }
        if !plan.stack_mapping.user {
            return Err(UserModeError::InvalidStackImage);
        }
        let stack_len = plan.stack_bytes.len() as u64;
        if plan.stack_mapping.len < stack_len {
            return Err(UserModeError::InvalidStackImage);
        }
        Ok(())
    }
}

struct UserMappingInstallAgent;

impl UserMappingInstallAgent {
    fn install<M: AddressSpaceManager>(
        manager: &mut M,
        address_space_id: AddressSpaceId,
        mapping: PageMapping,
    ) -> Result<(), UserModeError> {
        manager
            .map(address_space_id, mapping)
            .map_err(|_| UserModeError::MappingFailure)
    }
}

struct UserInitializationCaptureAgent;

impl UserInitializationCaptureAgent {
    fn capture(
        initialized_segments: &mut Vec<(PageMapping, Vec<u8>)>,
        mapping: PageMapping,
        initial_bytes: Option<&[u8]>,
    ) {
        if let Some(bytes) = initial_bytes {
            initialized_segments.push((mapping, bytes.to_vec()));
        }
    }
}

struct UserAddressSpaceActivationAgent;

impl UserAddressSpaceActivationAgent {
    fn activate<M: AddressSpaceManager>(
        manager: &mut M,
        address_space_id: AddressSpaceId,
    ) -> Result<(), UserModeError> {
        manager
            .activate_address_space(address_space_id)
            .map_err(|_| UserModeError::MappingFailure)
    }
}

struct UserPageTableMaterializationAgent;

impl UserPageTableMaterializationAgent {
    fn materialize(
        platform: &X86_64Platform,
        address_space_id: AddressSpaceId,
        page_tables: &mut [PageTable],
        phys_base: u64,
    ) -> Result<MaterializedAddressSpace, UserModeError> {
        platform
            .materialize_address_space(address_space_id, page_tables, phys_base)
            .map_err(|_| UserModeError::MappingFailure)
    }
}

#[derive(Debug)]
pub struct ManagedUserAddressSpaceMapper<'a, M: AddressSpaceManager> {
    manager: &'a mut M,
    address_space_id: AddressSpaceId,
    initialized_segments: Vec<(PageMapping, Vec<u8>)>,
}

impl<'a, M: AddressSpaceManager> ManagedUserAddressSpaceMapper<'a, M> {
    pub fn new(manager: &'a mut M, address_space_id: AddressSpaceId) -> Self {
        Self {
            manager,
            address_space_id,
            initialized_segments: Vec::new(),
        }
    }

    pub fn address_space_id(&self) -> AddressSpaceId {
        self.address_space_id
    }

    pub fn initialized_segments(&self) -> &[(PageMapping, Vec<u8>)] {
        &self.initialized_segments
    }
}

pub struct MaterializingUserAddressSpaceMapper<'a> {
    platform: &'a mut X86_64Platform,
    address_space_id: AddressSpaceId,
    page_tables: &'a mut [PageTable],
    phys_base: u64,
    initialized_segments: Vec<(PageMapping, Vec<u8>)>,
    materialized: Option<MaterializedAddressSpace>,
}

impl<'a> MaterializingUserAddressSpaceMapper<'a> {
    pub fn new(
        platform: &'a mut X86_64Platform,
        address_space_id: AddressSpaceId,
        page_tables: &'a mut [PageTable],
        phys_base: u64,
    ) -> Self {
        Self {
            platform,
            address_space_id,
            page_tables,
            phys_base,
            initialized_segments: Vec::new(),
            materialized: None,
        }
    }

    pub fn address_space_id(&self) -> AddressSpaceId {
        self.address_space_id
    }

    pub fn initialized_segments(&self) -> &[(PageMapping, Vec<u8>)] {
        &self.initialized_segments
    }

    pub fn materialized_address_space(&self) -> Option<MaterializedAddressSpace> {
        self.materialized
    }
}

impl<M: AddressSpaceManager> UserAddressSpaceMapper for ManagedUserAddressSpaceMapper<'_, M> {
    fn map_user_pages(
        &mut self,
        mapping: PageMapping,
        initial_bytes: Option<&[u8]>,
    ) -> Result<(), UserModeError> {
        UserMappingInstallAgent::install(self.manager, self.address_space_id, mapping)?;
        UserInitializationCaptureAgent::capture(
            &mut self.initialized_segments,
            mapping,
            initial_bytes,
        );
        Ok(())
    }

    fn activate_user_address_space(&mut self) -> Result<(), UserModeError> {
        UserAddressSpaceActivationAgent::activate(self.manager, self.address_space_id)
    }
}

impl UserAddressSpaceMapper for MaterializingUserAddressSpaceMapper<'_> {
    fn map_user_pages(
        &mut self,
        mapping: PageMapping,
        initial_bytes: Option<&[u8]>,
    ) -> Result<(), UserModeError> {
        UserMappingInstallAgent::install(self.platform, self.address_space_id, mapping)?;
        UserInitializationCaptureAgent::capture(
            &mut self.initialized_segments,
            mapping,
            initial_bytes,
        );
        Ok(())
    }

    fn activate_user_address_space(&mut self) -> Result<(), UserModeError> {
        UserAddressSpaceActivationAgent::activate(self.platform, self.address_space_id)?;
        let materialized = UserPageTableMaterializationAgent::materialize(
            self.platform,
            self.address_space_id,
            self.page_tables,
            self.phys_base,
        )?;
        self.materialized = Some(materialized);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserModeLaunchPlan {
    pub registers: Amd64UserEntryRegisters,
    pub image_mappings: Vec<PageMapping>,
    pub stack_mapping: PageMapping,
    pub stack_bytes: Vec<u8>,
}

impl UserModeLaunchPlan {
    pub fn validate(&self) -> Result<(), UserModeError> {
        UserLaunchPlanValidationAgent::validate(self)
    }
}

pub fn install_user_mode_address_space<M: UserAddressSpaceMapper>(
    mapper: &mut M,
    plan: &UserModeLaunchPlan,
) -> Result<(), UserModeError> {
    plan.validate()?;
    for mapping in &plan.image_mappings {
        mapper.map_user_pages(*mapping, None)?;
    }
    mapper.map_user_pages(plan.stack_mapping, Some(&plan.stack_bytes))?;
    mapper.activate_user_address_space()?;
    Ok(())
}

#[cfg(target_os = "none")]
/// # Safety
///
/// The caller must ensure the provided register set describes a valid user-mode
/// entry context and that the active address space has matching executable and
/// stack mappings installed.
pub unsafe fn enter_user_mode(registers: &Amd64UserEntryRegisters) -> ! {
    use core::arch::asm;

    unsafe {
        asm!(
            "push {ss}",
            "push {rsp}",
            "push {rflags}",
            "push {cs}",
            "push {rip}",
            "xor eax, eax",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov rdi, {rdi}",
            "mov rsi, {rsi}",
            "mov rdx, {rdx}",
            "mov rcx, {rcx}",
            "mov r8,  {r8}",
            "mov r9,  {r9}",
            "iretq",
            ss = in(reg) (registers.ss as u64),
            cs = in(reg) (registers.cs as u64),
            rsp = in(reg) (registers.rsp as u64),
            rflags = in(reg) (registers.rflags as u64),
            rip = in(reg) (registers.rip as u64),
            rdi = in(reg) (registers.rdi as u64),
            rsi = in(reg) (registers.rsi as u64),
            rdx = in(reg) (registers.rdx as u64),
            rcx = in(reg) (registers.rcx as u64),
            r8 = in(reg) (registers.r8 as u64),
            r9 = in(reg) (registers.r9 as u64),
            options(noreturn),
        );
    }
}

#[cfg(not(target_os = "none"))]
/// # Safety
///
/// This function never returns and always panics on hosted targets.
pub unsafe fn enter_user_mode(_registers: &Amd64UserEntryRegisters) -> ! {
    panic!("x86_64 user-mode transition is only available for target_os=none");
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::X86_64Platform;
    use ngos_user_abi::{AMD64_USER_CODE_SELECTOR, AMD64_USER_STACK_SELECTOR};
    use platform_hal::{CachePolicy, MemoryPermissions};

    #[derive(Default)]
    struct RecordingMapper {
        mapped: Vec<(PageMapping, Option<Vec<u8>>)>,
        activated: bool,
    }

    impl UserAddressSpaceMapper for RecordingMapper {
        fn map_user_pages(
            &mut self,
            mapping: PageMapping,
            initial_bytes: Option<&[u8]>,
        ) -> Result<(), UserModeError> {
            self.mapped
                .push((mapping, initial_bytes.map(|bytes| bytes.to_vec())));
            Ok(())
        }

        fn activate_user_address_space(&mut self) -> Result<(), UserModeError> {
            self.activated = true;
            Ok(())
        }
    }

    #[test]
    fn install_user_mode_address_space_maps_images_then_stack() {
        let image = PageMapping {
            vaddr: 0x400000,
            paddr: 0,
            len: 0x3000,
            perms: MemoryPermissions::read_execute(),
            cache: CachePolicy::WriteBack,
            user: true,
        };
        let stack = PageMapping {
            vaddr: 0x7fff_fffe_f000,
            paddr: 0,
            len: 0x1000,
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: true,
        };
        let plan = UserModeLaunchPlan {
            registers: Amd64UserEntryRegisters {
                rip: 0x401000,
                rsp: 0x7fff_fffe_ff00,
                rflags: 0x202,
                cs: AMD64_USER_CODE_SELECTOR,
                ss: AMD64_USER_STACK_SELECTOR,
                rdi: 1,
                rsi: 2,
                rdx: 3,
                rcx: 4,
                r8: 5,
                r9: 6,
            },
            image_mappings: Vec::from([image]),
            stack_mapping: stack,
            stack_bytes: Vec::from([1, 2, 3, 4]),
        };

        let mut mapper = RecordingMapper::default();
        install_user_mode_address_space(&mut mapper, &plan).unwrap();
        assert_eq!(mapper.mapped.len(), 2);
        assert_eq!(mapper.mapped[0].0, image);
        assert!(mapper.mapped[0].1.is_none());
        assert_eq!(mapper.mapped[1].0, stack);
        assert_eq!(
            mapper.mapped[1].1.as_ref().unwrap(),
            &Vec::from([1, 2, 3, 4])
        );
        assert!(mapper.activated);
    }

    #[test]
    fn managed_mapper_installs_plan_into_platform_address_space() {
        let image = PageMapping {
            vaddr: 0x400000,
            paddr: 0x20_0000,
            len: 0x2000,
            perms: MemoryPermissions::read_execute(),
            cache: CachePolicy::WriteBack,
            user: true,
        };
        let stack = PageMapping {
            vaddr: 0x7fff_fffe_f000,
            paddr: 0x21_0000,
            len: 0x1000,
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: true,
        };
        let plan = UserModeLaunchPlan {
            registers: Amd64UserEntryRegisters {
                rip: 0x401000,
                rsp: 0x7fff_fffe_ff00,
                rflags: 0x202,
                cs: AMD64_USER_CODE_SELECTOR,
                ss: AMD64_USER_STACK_SELECTOR,
                rdi: 7,
                rsi: 8,
                rdx: 9,
                rcx: 10,
                r8: 11,
                r9: 12,
            },
            image_mappings: Vec::from([image]),
            stack_mapping: stack,
            stack_bytes: Vec::from([9, 8, 7, 6]),
        };

        let mut platform = X86_64Platform::default();
        let address_space_id = platform.create_address_space().unwrap();
        let mut mapper = ManagedUserAddressSpaceMapper::new(&mut platform, address_space_id);

        install_user_mode_address_space(&mut mapper, &plan).unwrap();

        assert_eq!(mapper.address_space_id(), address_space_id);
        assert_eq!(mapper.initialized_segments().len(), 1);
        assert_eq!(mapper.initialized_segments()[0].0, stack);
        assert_eq!(mapper.initialized_segments()[0].1, Vec::from([9, 8, 7, 6]));
        drop(mapper);

        let layout = platform.address_space_layout(address_space_id).unwrap();
        assert!(layout.active);
        assert_eq!(layout.mappings, Vec::from([image, stack]));
        assert_eq!(platform.active_address_space(), Some(address_space_id));

        let mut tables = [crate::PageTable::zeroed(); 8];
        let materialized = platform
            .materialize_address_space(address_space_id, &mut tables, 0x80_0000)
            .unwrap();
        assert_eq!(materialized.id, address_space_id);
        assert_eq!(materialized.stats.mapping_regions, 2);
        assert!(materialized.stats.table_pages_used >= 4);
    }

    #[test]
    fn materializing_mapper_installs_plan_and_materializes_page_tables() {
        let image = PageMapping {
            vaddr: 0x400000,
            paddr: 0x20_0000,
            len: 0x2000,
            perms: MemoryPermissions::read_execute(),
            cache: CachePolicy::WriteBack,
            user: true,
        };
        let stack = PageMapping {
            vaddr: 0x7fff_fffe_f000,
            paddr: 0x21_0000,
            len: 0x1000,
            perms: MemoryPermissions::read_write(),
            cache: CachePolicy::WriteBack,
            user: true,
        };
        let plan = UserModeLaunchPlan {
            registers: Amd64UserEntryRegisters {
                rip: 0x401000,
                rsp: 0x7fff_fffe_ff00,
                rflags: 0x202,
                cs: AMD64_USER_CODE_SELECTOR,
                ss: AMD64_USER_STACK_SELECTOR,
                rdi: 13,
                rsi: 14,
                rdx: 15,
                rcx: 16,
                r8: 17,
                r9: 18,
            },
            image_mappings: Vec::from([image]),
            stack_mapping: stack,
            stack_bytes: Vec::from([1, 3, 3, 7]),
        };

        let mut platform = X86_64Platform::default();
        let address_space_id = platform.create_address_space().unwrap();
        let mut tables = [crate::PageTable::zeroed(); 16];
        let mut mapper = MaterializingUserAddressSpaceMapper::new(
            &mut platform,
            address_space_id,
            &mut tables,
            0x90_0000,
        );

        install_user_mode_address_space(&mut mapper, &plan).unwrap();

        assert_eq!(mapper.address_space_id(), address_space_id);
        assert_eq!(mapper.initialized_segments().len(), 1);
        assert_eq!(mapper.initialized_segments()[0].0, stack);
        assert_eq!(mapper.initialized_segments()[0].1, Vec::from([1, 3, 3, 7]));
        let materialized = mapper.materialized_address_space().unwrap();
        assert_eq!(materialized.id, address_space_id);
        assert_eq!(materialized.root_phys, 0x90_0000);
        assert_eq!(materialized.stats.mapping_regions, 2);
        assert!(materialized.stats.table_pages_used >= 4);
        drop(mapper);

        let layout = platform.address_space_layout(address_space_id).unwrap();
        assert!(layout.active);
        assert_eq!(layout.mappings, Vec::from([image, stack]));
    }
}
