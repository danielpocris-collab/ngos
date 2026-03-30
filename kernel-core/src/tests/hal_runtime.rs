use super::*;
#[test]
fn procfs_vmobjects_render_segment_metadata() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(15_1020), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(15_1021), 0),
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
        .create_vfs_node("/lib/libseg.so", ObjectKind::File, lib)
        .unwrap();

    runtime
        .map_file_memory(
            owner,
            "/lib/libseg.so".to_string(),
            0x2000,
            0,
            true,
            true,
            false,
            false,
        )
        .unwrap();
    let text = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", owner.raw()))
            .unwrap(),
    )
    .unwrap();

    assert!(text.contains("segments="));
    assert!(text.contains("resident-segments="));
    assert!(text.contains("libseg.so"));
}

#[derive(Debug, Default)]
struct TestHal {
    next_id: u64,
    active: Option<HalAddressSpaceId>,
    spaces: BTreeMap<HalAddressSpaceId, Vec<PageMapping>>,
}

impl AddressSpaceManager for TestHal {
    fn create_address_space(&mut self) -> Result<HalAddressSpaceId, HalError> {
        let id = HalAddressSpaceId::new(self.next_id.max(1));
        self.next_id = id.raw().saturating_add(1);
        self.spaces.insert(id, Vec::new());
        Ok(id)
    }

    fn destroy_address_space(&mut self, id: HalAddressSpaceId) -> Result<(), HalError> {
        if self.spaces.remove(&id).is_none() {
            return Err(HalError::InvalidAddressSpace);
        }
        if self.active == Some(id) {
            self.active = None;
        }
        Ok(())
    }

    fn map(&mut self, id: HalAddressSpaceId, mapping: PageMapping) -> Result<(), HalError> {
        self.spaces
            .get_mut(&id)
            .ok_or(HalError::InvalidAddressSpace)?
            .push(mapping);
        Ok(())
    }

    fn unmap(
        &mut self,
        id: HalAddressSpaceId,
        range: platform_hal::VirtualRange,
    ) -> Result<(), HalError> {
        let mappings = self
            .spaces
            .get_mut(&id)
            .ok_or(HalError::InvalidAddressSpace)?;
        let before = mappings.len();
        mappings.retain(|mapping| !(mapping.vaddr == range.vaddr && mapping.len == range.len));
        if mappings.len() == before {
            return Err(HalError::MappingNotFound);
        }
        Ok(())
    }

    fn protect(
        &mut self,
        id: HalAddressSpaceId,
        range: platform_hal::VirtualRange,
        perms: MemoryPermissions,
    ) -> Result<(), HalError> {
        let mappings = self
            .spaces
            .get_mut(&id)
            .ok_or(HalError::InvalidAddressSpace)?;
        let mapping = mappings
            .iter_mut()
            .find(|mapping| mapping.vaddr == range.vaddr && mapping.len == range.len)
            .ok_or(HalError::MappingNotFound)?;
        mapping.perms = perms;
        Ok(())
    }

    fn activate_address_space(&mut self, id: HalAddressSpaceId) -> Result<(), HalError> {
        if !self.spaces.contains_key(&id) {
            return Err(HalError::InvalidAddressSpace);
        }
        self.active = Some(id);
        Ok(())
    }

    fn active_address_space(&self) -> Option<HalAddressSpaceId> {
        self.active
    }

    fn address_space_layout(
        &self,
        id: HalAddressSpaceId,
    ) -> Result<HalAddressSpaceLayout, HalError> {
        let mappings = self
            .spaces
            .get(&id)
            .cloned()
            .ok_or(HalError::InvalidAddressSpace)?;
        Ok(HalAddressSpaceLayout {
            id,
            active: self.active == Some(id),
            mappings,
        })
    }
}

#[test]
fn process_introspection_projects_hal_page_mappings() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(15_1030), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(15_1031), 0),
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
        .create_vfs_node("/lib/libhal.so", ObjectKind::File, lib)
        .unwrap();
    runtime
        .map_file_memory(
            owner,
            "/lib/libhal.so",
            0x2000,
            0x1000,
            true,
            false,
            true,
            true,
        )
        .unwrap();

    let introspection = runtime.inspect_process(owner).unwrap();
    let layout =
        project_hal_address_space_layout(&introspection, HalAddressSpaceId::new(7), false).unwrap();

    assert_eq!(layout.id.raw(), 7);
    assert_eq!(
        layout.mappings.len(),
        introspection.address_space.region_count
    );
    assert!(layout.mappings.iter().any(|mapping| mapping.perms.execute));
    assert!(layout.mappings.iter().any(|mapping| mapping.user));
}

#[test]
fn hal_backed_runtime_syncs_vm_operations_and_activation() {
    let mut runtime = HalBackedKernelRuntime::host_runtime_default(TestHal::default()).unwrap();
    let app = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();

    let before = runtime.activate_process_address_space(app).unwrap();
    let mapped = runtime
        .map_anonymous_memory(app, 0x2000, true, true, false, "scratch")
        .unwrap();
    let after_map = runtime.activate_process_address_space(app).unwrap();

    assert!(after_map.active);
    assert!(after_map.mappings.len() > before.mappings.len());
    assert_eq!(runtime.hal().active_address_space(), Some(after_map.id));

    runtime
        .protect_memory(app, mapped, 0x2000, true, false, false)
        .unwrap();
    let after_protect = runtime.activate_process_address_space(app).unwrap();
    let protected = after_protect
        .mappings
        .iter()
        .find(|mapping| mapping.vaddr == mapped && mapping.len == 0x2000)
        .unwrap();
    assert_eq!(protected.perms, MemoryPermissions::read_only());

    runtime.unmap_memory(app, mapped, 0x2000).unwrap();
    let after_unmap = runtime.activate_process_address_space(app).unwrap();
    assert_eq!(after_unmap.mappings.len(), before.mappings.len());

    let scheduled = runtime.tick().unwrap();
    assert_eq!(scheduled.pid, app);
    assert_eq!(runtime.hal().active_address_space(), Some(after_unmap.id));
}
