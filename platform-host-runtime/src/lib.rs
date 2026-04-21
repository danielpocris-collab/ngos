//! Canonical subsystem role:
//! - subsystem: host platform mediation
//! - owner layer: auxiliary execution layer
//! - semantic owner: `platform-host-runtime`
//! - truth path role: host-only platform mechanism surface for auxiliary
//!   execution, not real product truth
//!
//! Canonical contract families handled here:
//! - host platform contracts
//! - host address-space mediation contracts
//! - host platform descriptor contracts
//!
//! This crate may provide host-side platform mechanics for validation, but it
//! must not be treated as the final platform truth surface for subsystem
//! closure.

use std::collections::BTreeMap;

use kernel_core::{
    KernelConfig, KernelState, ProcessIntrospection, project_hal_address_space_layout,
};
use platform_hal::{
    AddressSpaceId, AddressSpaceLayout, AddressSpaceManager, Architecture, HalError, PageMapping,
    Platform, PlatformDescriptor, VirtualRange,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostRuntimeAddressSpace {
    mappings: Vec<PageMapping>,
}

impl HostRuntimeAddressSpace {
    fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }
}

pub struct HostRuntimePlatform {
    descriptor: PlatformDescriptor,
    address_spaces: BTreeMap<AddressSpaceId, HostRuntimeAddressSpace>,
    next_address_space_id: u64,
    active_address_space: Option<AddressSpaceId>,
}

impl HostRuntimePlatform {
    pub fn detect() -> Self {
        let architecture = if cfg!(target_arch = "aarch64") {
            Architecture::AArch64
        } else {
            Architecture::X86_64
        };

        Self {
            descriptor: PlatformDescriptor {
                name: std::env::consts::OS,
                architecture,
                host_runtime_mode: true,
            },
            address_spaces: BTreeMap::new(),
            next_address_space_id: 1,
            active_address_space: None,
        }
    }

    pub fn bootstrap_kernel(&self) -> KernelState {
        KernelState::bootstrap(KernelConfig::host_runtime(self.architecture()))
    }

    fn validate_mapping(mapping: PageMapping) -> Result<(), HalError> {
        if mapping.len == 0 || mapping.vaddr == 0 || mapping.paddr == 0 {
            return Err(HalError::InvalidMapping);
        }
        if mapping.vaddr.checked_add(mapping.len).is_none()
            || mapping.paddr.checked_add(mapping.len).is_none()
        {
            return Err(HalError::InvalidMapping);
        }
        if !mapping.len.is_multiple_of(0x1000)
            || !mapping.vaddr.is_multiple_of(0x1000)
            || !mapping.paddr.is_multiple_of(0x1000)
        {
            return Err(HalError::InvalidMapping);
        }
        Ok(())
    }

    fn ranges_overlap(lhs: VirtualRange, rhs: VirtualRange) -> bool {
        lhs.vaddr < rhs.vaddr.saturating_add(rhs.len)
            && rhs.vaddr < lhs.vaddr.saturating_add(lhs.len)
    }

    fn mapping_range(mapping: PageMapping) -> VirtualRange {
        VirtualRange {
            vaddr: mapping.vaddr,
            len: mapping.len,
        }
    }

    fn space_mut(&mut self, id: AddressSpaceId) -> Result<&mut HostRuntimeAddressSpace, HalError> {
        self.address_spaces
            .get_mut(&id)
            .ok_or(HalError::InvalidAddressSpace)
    }

    fn space(&self, id: AddressSpaceId) -> Result<&HostRuntimeAddressSpace, HalError> {
        self.address_spaces
            .get(&id)
            .ok_or(HalError::InvalidAddressSpace)
    }

    pub fn sync_process_address_space(
        &mut self,
        introspection: &ProcessIntrospection,
    ) -> Result<AddressSpaceId, HalError> {
        let Some(id) = introspection.process.address_space else {
            return Err(HalError::InvalidAddressSpace);
        };
        let host_runtime_id = AddressSpaceId::new(id.raw());
        self.next_address_space_id = self
            .next_address_space_id
            .max(host_runtime_id.raw().saturating_add(1));
        self.address_spaces
            .entry(host_runtime_id)
            .or_insert_with(HostRuntimeAddressSpace::new)
            .mappings
            .clear();

        let layout = project_hal_address_space_layout(introspection, host_runtime_id, false)?;
        for mapping in layout.mappings {
            self.map(host_runtime_id, mapping)?;
        }

        Ok(host_runtime_id)
    }
}

impl Platform for HostRuntimePlatform {
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

impl AddressSpaceManager for HostRuntimePlatform {
    fn create_address_space(&mut self) -> Result<AddressSpaceId, HalError> {
        let id = AddressSpaceId::new(self.next_address_space_id);
        self.next_address_space_id = self
            .next_address_space_id
            .checked_add(1)
            .ok_or(HalError::Exhausted)?;
        self.address_spaces
            .insert(id, HostRuntimeAddressSpace::new());
        Ok(id)
    }

    fn destroy_address_space(&mut self, id: AddressSpaceId) -> Result<(), HalError> {
        if self.address_spaces.remove(&id).is_none() {
            return Err(HalError::InvalidAddressSpace);
        }
        if self.active_address_space == Some(id) {
            self.active_address_space = None;
        }
        Ok(())
    }

    fn map(&mut self, id: AddressSpaceId, mapping: PageMapping) -> Result<(), HalError> {
        Self::validate_mapping(mapping)?;
        let space = self.space_mut(id)?;
        let range = Self::mapping_range(mapping);
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
        perms: platform_hal::MemoryPermissions,
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
mod tests {
    use super::*;
    use kernel_core::{
        CapabilityRights, Handle, KernelRuntime, ObjectHandle, ObjectKind, SchedulerClass,
    };
    use platform_hal::{AddressSpaceManager, CachePolicy, MemoryPermissions};

    #[test]
    fn host_runtime_platform_tracks_address_space_layouts() {
        let mut platform = HostRuntimePlatform::detect();
        let kernel = platform.bootstrap_kernel();
        assert!(kernel.scheduler_ready);

        let first = platform.create_address_space().unwrap();
        let second = platform.create_address_space().unwrap();

        platform
            .map(
                first,
                PageMapping {
                    vaddr: 0x1000,
                    paddr: 0x2000,
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
    fn host_runtime_platform_rejects_overlapping_or_missing_mappings() {
        let mut platform = HostRuntimePlatform::detect();
        let id = platform.create_address_space().unwrap();

        let mapping = PageMapping {
            vaddr: 0x4000,
            paddr: 0x8000,
            len: 0x1000,
            perms: MemoryPermissions::read_only(),
            cache: CachePolicy::WriteBack,
            user: false,
        };

        platform.map(id, mapping).unwrap();
        assert_eq!(platform.map(id, mapping), Err(HalError::OverlappingMapping));
        assert_eq!(
            platform.unmap(
                id,
                VirtualRange {
                    vaddr: 0x5000,
                    len: 0x1000,
                }
            ),
            Err(HalError::MappingNotFound)
        );

        platform
            .protect(
                id,
                VirtualRange {
                    vaddr: 0x4000,
                    len: 0x1000,
                },
                MemoryPermissions::read_execute(),
            )
            .unwrap();
        let layout = platform.address_space_layout(id).unwrap();
        assert_eq!(layout.mappings[0].perms, MemoryPermissions::read_execute());
    }

    #[test]
    fn host_runtime_platform_can_sync_kernel_process_address_spaces() {
        let mut runtime = KernelRuntime::host_runtime_default();
        let owner = runtime
            .spawn_process("app", None, SchedulerClass::Interactive)
            .unwrap();
        let root = runtime
            .grant_capability(
                owner,
                ObjectHandle::new(Handle::new(41_000), 0),
                CapabilityRights::READ | CapabilityRights::WRITE,
                "root",
            )
            .unwrap();
        let lib = runtime
            .grant_capability(
                owner,
                ObjectHandle::new(Handle::new(41_001), 0),
                CapabilityRights::READ | CapabilityRights::WRITE,
                "lib",
            )
            .unwrap();
        runtime
            .create_vfs_node("/", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/lib", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/lib/libsync.so", ObjectKind::File, lib)
            .unwrap();
        runtime
            .map_file_memory(
                owner,
                "/lib/libsync.so",
                0x2000,
                0x1000,
                true,
                false,
                true,
                true,
            )
            .unwrap();

        let introspection = runtime.inspect_process(owner).unwrap();
        let mut platform = HostRuntimePlatform::detect();
        let synced = platform.sync_process_address_space(&introspection).unwrap();
        platform.activate_address_space(synced).unwrap();

        let layout = platform.address_space_layout(synced).unwrap();
        assert!(layout.active);
        assert_eq!(layout.id.raw(), introspection.address_space.id.raw());
        assert_eq!(
            layout.mappings.len(),
            introspection.address_space.region_count
        );
        assert!(layout.mappings.iter().any(|mapping| mapping.perms.execute));
        assert!(
            layout
                .mappings
                .iter()
                .any(|mapping| mapping.vaddr >= 0x1000_0000)
        );
    }
}
