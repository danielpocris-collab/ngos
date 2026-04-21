use super::*;
#[test]
fn runtime_supports_memory_mapping_unmapping_and_brk_growth() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(app, 0x2000, true, true, false, "scratch")
        .unwrap();
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("rw-p 00000000 normal [anon:scratch]"));

    runtime.unmap_memory(app, mapped, 0x2000).unwrap();
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(!maps.contains("[anon:scratch]"));

    let info = runtime.process_info(app).unwrap();
    let new_brk = runtime
        .set_process_break(app, info.executable_image.base_addr + 0x9000)
        .unwrap();
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(new_brk >= info.executable_image.base_addr + 0x9000);
    assert!(maps.contains("rw-p 00000000 normal [heap]"));
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("Heap\tprivate=true\towners=1"));
    assert!(vmobjects.contains("[heap]"));
}

#[test]
fn runtime_supports_file_mappings_and_protection_changes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_002), 0),
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
        .create_vfs_node("/lib/libui.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libui.so",
            0x2000,
            0x1000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("r-xp 00001000 normal /lib/libui.so"));

    runtime
        .protect_memory(app, mapped, 0x2000, true, true, false)
        .unwrap();
    let touch = runtime.touch_memory(app, mapped, 0x2000, true).unwrap();
    assert_eq!(touch.pages_touched, 2);
    assert_eq!(touch.faulted_pages, 2);
    runtime
        .advise_memory(app, mapped, 0x2000, MemoryAdvice::Sequential)
        .unwrap();
    runtime.sync_memory(app, mapped, 0x2000).unwrap();
    let layout = runtime
        .inspect_vm_object_layouts(app)
        .unwrap()
        .into_iter()
        .find(|layout| {
            layout.kind == VmObjectKind::File && layout.private && layout.backing_offset == 0x1000
        })
        .expect("file-backed vm layout must be present after sync");
    assert_eq!(layout.owner_count, 1);
    assert_eq!(layout.committed_pages, 2);
    assert_eq!(layout.resident_pages, 2);
    assert_eq!(layout.dirty_pages, 0);
    assert_eq!(layout.accessed_pages, 2);
    assert_eq!(layout.fault_count, 2);
    assert_eq!(layout.read_fault_count, 0);
    assert_eq!(layout.write_fault_count, 2);
    assert_eq!(layout.cow_fault_count, 0);
    assert_eq!(layout.sync_count, 1);
    assert_eq!(layout.synced_pages, 2);
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("rw-p 00001000 seq /lib/libui.so"));
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("File\tprivate=true\towners=1\toffset=00001000\tcommitted=2\tresident=2\tdirty=0\taccessed=2"));
    assert!(vmobjects.contains("segments="));
    assert!(vmobjects.contains("resident-segments="));
    assert!(vmobjects.contains("faults=2(r=0,w=2,cow=0)\t/lib/libui.so"));
}

#[test]
fn runtime_vm_object_layouts_preserve_nonzero_file_offsets() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_021), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_022), 0),
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
        .create_vfs_node("/lib/liboffset.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/liboffset.so".to_string(),
            0x2000,
            0x3000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x2000, true, true, false)
        .unwrap();
    let touch = runtime.touch_memory(app, mapped, 0x1000, true).unwrap();
    assert_eq!(touch.faulted_pages, 1);

    let layouts = runtime.inspect_vm_object_layouts(app).unwrap();
    let layout = layouts
        .into_iter()
        .find(|layout| {
            layout
                .segments
                .first()
                .is_some_and(|segment| segment.byte_offset == 0x3000)
                && layout
                    .segments
                    .iter()
                    .map(|segment| segment.byte_len)
                    .sum::<u64>()
                    == 0x2000
        })
        .expect("file-backed layout with non-zero offset must be present");

    assert_eq!(layout.kind, VmObjectKind::File);
    assert!(layout.private);
    assert_eq!(layout.owner_count, 1);
    assert_eq!(layout.backing_offset, 0x3000);
    assert_eq!(layout.committed_pages, 2);
    assert_eq!(layout.resident_pages, 1);
    assert_eq!(layout.dirty_pages, 1);
    assert_eq!(layout.accessed_pages, 1);
    assert_eq!(layout.fault_count, 1);
    assert_eq!(layout.read_fault_count, 0);
    assert_eq!(layout.write_fault_count, 1);
    assert_eq!(layout.cow_fault_count, 0);
    assert_eq!(layout.sync_count, 0);
    assert_eq!(layout.synced_pages, 0);
    assert_eq!(layout.segment_count, 2);
    assert_eq!(layout.segments[0].start_page, 0);
    assert_eq!(layout.segments[0].byte_offset, 0x3000);
    assert_eq!(layout.segments[0].byte_len, 0x1000);
    assert!(layout.segments[0].resident);
    assert_eq!(layout.segments[1].start_page, 1);
    assert_eq!(layout.segments[1].byte_offset, 0x4000);
    assert_eq!(layout.segments[1].byte_len, 0x1000);
    assert!(!layout.segments[1].resident);
}

#[test]
fn runtime_shares_file_vm_objects_across_processes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let worker = runtime
        .spawn_process("worker", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(9_050), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(9_051), 0),
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
        .create_vfs_node("/lib/libcache.so", ObjectKind::File, lib)
        .unwrap();

    let app_map = runtime
        .map_file_memory(
            app,
            "/lib/libcache.so",
            0x2000,
            0x1000,
            true,
            true,
            false,
            false,
        )
        .unwrap();
    let worker_map = runtime
        .map_file_memory(
            worker,
            "/lib/libcache.so",
            0x2000,
            0x1000,
            true,
            true,
            false,
            false,
        )
        .unwrap();
    let touch = runtime.touch_memory(app, app_map, 0x1000, true).unwrap();
    assert_eq!(touch.faulted_pages, 1);
    assert_eq!(touch.cow_faulted_pages, 0);

    let app_vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    let worker_vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", worker.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(app_vmobjects.contains("File\tprivate=false\towners=2\toffset=00001000\tcommitted=2\tresident=1\tdirty=1\taccessed=1"));
    assert!(app_vmobjects.contains("segments="));
    assert!(app_vmobjects.contains("resident-segments="));
    assert!(app_vmobjects.contains("faults=1(r=0,w=1,cow=0)\t/lib/libcache.so"));
    assert!(worker_vmobjects.contains("File\tprivate=false\towners=2\toffset=00001000\tcommitted=2\tresident=1\tdirty=1\taccessed=1"));
    assert!(worker_vmobjects.contains("segments="));
    assert!(worker_vmobjects.contains("resident-segments="));
    assert!(worker_vmobjects.contains("faults=1(r=0,w=1,cow=0)\t/lib/libcache.so"));
    runtime.sync_memory(worker, worker_map, 0x1000).unwrap();
    let worker_vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", worker.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(worker_vmobjects.contains("dirty=0"));
}

#[test]
fn runtime_splits_regions_for_partial_vm_operations() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_101), 0),
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
        .create_vfs_node("/lib/libsplit.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libsplit.so",
            0x3000,
            0x4000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped + 0x1000, 0x1000, true, true, false)
        .unwrap();
    runtime
        .advise_memory(app, mapped + 0x2000, 0x1000, MemoryAdvice::DontNeed)
        .unwrap();

    let anon = runtime
        .map_anonymous_memory(app, 0x3000, true, true, false, "scratch")
        .unwrap();
    runtime.unmap_memory(app, anon + 0x1000, 0x1000).unwrap();

    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("r-xp 00004000 normal /lib/libsplit.so"));
    assert!(maps.contains("rw-p 00005000 normal /lib/libsplit.so"));
    assert!(maps.contains("r-xp 00006000 dontneed /lib/libsplit.so"));
    assert!(maps.contains("rw-p 00000000 normal [anon:scratch]"));
    assert_eq!(maps.matches("[anon:scratch]").count(), 2);
}

#[test]
fn runtime_coalesces_regions_after_restoring_vm_semantics() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_104), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_105), 0),
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
        .create_vfs_node("/lib/libmerge.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libmerge.so".to_string(),
            0x3000,
            0x5000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped + 0x1000, 0x1000, true, true, false)
        .unwrap();
    runtime
        .protect_memory(app, mapped + 0x1000, 0x1000, true, false, true)
        .unwrap();
    runtime
        .advise_memory(app, mapped + 0x1000, 0x1000, MemoryAdvice::Sequential)
        .unwrap();
    runtime
        .advise_memory(app, mapped + 0x1000, 0x1000, MemoryAdvice::Normal)
        .unwrap();
    runtime.sync_memory(app, mapped + 0x1000, 0x1000).unwrap();

    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(maps.matches("/lib/libmerge.so").count(), 1);
    assert!(maps.contains("r-xp 00005000 normal /lib/libmerge.so"));
}

#[test]
fn runtime_vm_range_operations_span_split_regions() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_108), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_109), 0),
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
        .create_vfs_node("/lib/librange.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/librange.so".to_string(),
            0x3000,
            0x6000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped + 0x1000, 0x1000, true, true, false)
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x3000, true, true, false)
        .unwrap();

    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(maps.matches("/lib/librange.so").count(), 1);
    assert!(maps.contains("rw-p 00006000 normal /lib/librange.so"));

    runtime
        .advise_memory(app, mapped + 0x1000, 0x1000, MemoryAdvice::DontNeed)
        .unwrap();
    runtime
        .advise_memory(app, mapped, 0x3000, MemoryAdvice::Normal)
        .unwrap();

    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(maps.matches("/lib/librange.so").count(), 1);
    assert!(maps.contains("rw-p 00006000 normal /lib/librange.so"));

    runtime
        .touch_memory(app, mapped + 0x1000, 0x1000, true)
        .unwrap();
    runtime.sync_memory(app, mapped, 0x3000).unwrap();

    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(maps.matches("/lib/librange.so").count(), 1);
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("dirty=0"));
}

#[test]
fn runtime_touch_memory_spans_split_regions_and_aggregates_stats() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_110), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_111), 0),
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
        .create_vfs_node("/lib/libtouch.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libtouch.so".to_string(),
            0x3000,
            0x7000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped + 0x1000, 0x1000, true, true, false)
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x3000, true, true, false)
        .unwrap();

    let touch = runtime.touch_memory(app, mapped, 0x3000, true).unwrap();
    assert_eq!(touch.pages_touched, 3);
    assert_eq!(touch.faulted_pages, 3);
    assert_eq!(touch.cow_faulted_pages, 0);

    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(maps.matches("/lib/libtouch.so").count(), 1);
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("committed=3\tresident=3\tdirty=3\taccessed=3"));
    assert!(vmobjects.contains("faults=3(r=0,w=3,cow=0)\t/lib/libtouch.so"));
}

#[test]
fn runtime_read_faults_mark_vm_pages_accessed_without_dirtying_them() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_106), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_107), 0),
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
        .create_vfs_node("/lib/libread.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libread.so".to_string(),
            0x2000,
            0x2000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    let touch = runtime.touch_memory(app, mapped, 0x1000, false).unwrap();
    assert_eq!(touch.faulted_pages, 1);
    let second = runtime.touch_memory(app, mapped, 0x1000, false).unwrap();
    assert_eq!(second.faulted_pages, 0);

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("resident=1\tdirty=0\taccessed=1"));
    assert!(vmobjects.contains("faults=1(r=1,w=0,cow=0)\t/lib/libread.so"));
}

#[test]
fn runtime_read_fault_on_cow_region_does_not_allocate_shadow_until_write() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = runtime
        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x2000, true, true, false, "cow-read-then-write")
        .unwrap();
    let child = runtime
        .spawn_process_copy_vm("child", Some(init), SchedulerClass::Interactive, parent)
        .unwrap();

    let read = runtime.touch_memory(child, scratch, 0x1000, false).unwrap();
    assert_eq!(read.cow_faulted_pages, 0);
    let layouts = runtime.inspect_vm_object_layouts(child).unwrap();
    assert!(
        layouts
            .iter()
            .all(|layout| layout.shadow_source_id.is_none())
    );

    let write = runtime.touch_memory(child, scratch, 0x1000, true).unwrap();
    assert_eq!(write.cow_faulted_pages, 1);
    let layouts = runtime.inspect_vm_object_layouts(child).unwrap();
    assert!(
        layouts
            .iter()
            .any(|layout| layout.shadow_source_id.is_some())
    );
}

#[test]
fn runtime_touch_memory_spans_split_cow_regions_and_aggregates_stats() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = runtime
        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x3000, true, true, false, "cow-touch-range")
        .unwrap();
    runtime
        .protect_memory(parent, scratch + 0x1000, 0x1000, true, false, false)
        .unwrap();
    runtime
        .protect_memory(parent, scratch, 0x3000, true, true, false)
        .unwrap();
    let child = runtime
        .spawn_process_copy_vm("child", Some(init), SchedulerClass::Interactive, parent)
        .unwrap();

    let touch = runtime.touch_memory(child, scratch, 0x3000, true).unwrap();
    assert_eq!(touch.pages_touched, 3);
    assert_eq!(touch.faulted_pages, 3);
    assert_eq!(touch.cow_faulted_pages, 3);

    let layouts = runtime.inspect_vm_object_layouts(child).unwrap();
    let shadow_layouts = layouts
        .into_iter()
        .filter(|layout| layout.shadow_source_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(shadow_layouts.len(), 1);
    assert_eq!(shadow_layouts[0].shadow_source_offset, 0);
    assert_eq!(
        shadow_layouts[0]
            .segments
            .iter()
            .map(|segment| segment.byte_len)
            .sum::<u64>(),
        0x3000
    );

    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(maps.matches("[anon:cow-touch-range]").count(), 1);
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(vmobjects.matches("[cow]").count(), 1);
    assert!(vmobjects.contains("committed=3\tresident=3\tdirty=3\taccessed=3"));
    assert!(vmobjects.contains("faults=3(r=0,w=0,cow=3)"));
}

#[test]
fn runtime_mixed_faults_across_split_regions_preserve_read_write_counters() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_112), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_113), 0),
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
        .create_vfs_node("/lib/libmixed.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libmixed.so".to_string(),
            0x3000,
            0x8000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped + 0x1000, 0x1000, true, true, false)
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x3000, true, true, false)
        .unwrap();

    let write = runtime.touch_memory(app, mapped, 0x1000, true).unwrap();
    assert_eq!(write.pages_touched, 1);
    assert_eq!(write.faulted_pages, 1);

    let read = runtime.touch_memory(app, mapped, 0x3000, false).unwrap();
    assert_eq!(read.pages_touched, 3);
    assert_eq!(read.faulted_pages, 2);
    assert_eq!(read.cow_faulted_pages, 0);

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("committed=3\tresident=3\tdirty=1\taccessed=3"));
    assert!(vmobjects.contains("faults=3(r=2,w=1,cow=0)\t/lib/libmixed.so"));
}

#[test]
fn runtime_madvise_dontneed_evicts_pages_and_willneed_prefaults_them() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_114), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_115), 0),
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
        .create_vfs_node("/lib/libadvise.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libadvise.so".to_string(),
            0x3000,
            0x9000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x3000, true, true, false)
        .unwrap();
    runtime.touch_memory(app, mapped, 0x3000, true).unwrap();

    runtime
        .advise_memory(app, mapped + 0x1000, 0x1000, MemoryAdvice::DontNeed)
        .unwrap();
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("committed=3\tresident=2\tdirty=2\taccessed=2"));

    let reread = runtime
        .touch_memory(app, mapped + 0x1000, 0x1000, false)
        .unwrap();
    assert_eq!(reread.faulted_pages, 1);

    runtime
        .advise_memory(app, mapped, 0x3000, MemoryAdvice::DontNeed)
        .unwrap();
    runtime
        .advise_memory(app, mapped, 0x3000, MemoryAdvice::WillNeed)
        .unwrap();
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("committed=3\tresident=3\tdirty=0\taccessed=3"));

    let read = runtime.touch_memory(app, mapped, 0x3000, false).unwrap();
    assert_eq!(read.faulted_pages, 0);
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("faults=4(r=1,w=3,cow=0)\t/lib/libadvise.so"));
}

#[test]
fn runtime_mprotect_does_not_dirty_pages_without_writes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_116), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_117), 0),
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
        .create_vfs_node("/lib/libprot.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libprot.so".to_string(),
            0x2000,
            0xa000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x2000, true, true, false)
        .unwrap();

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("committed=2\tresident=0\tdirty=0\taccessed=0"));
    assert!(vmobjects.contains("faults=0(r=0,w=0,cow=0)\t/lib/libprot.so"));
}

#[test]
fn runtime_can_spawn_processes_with_copied_vm_state() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_200), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_201), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "bin",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/opt", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin/app", ObjectKind::File, bin)
        .unwrap();
    runtime.set_process_cwd(app, "/opt").unwrap();
    runtime
        .exec_process(
            app,
            "/bin/app",
            vec![String::from("app"), String::from("--fork")],
            vec![String::from("TERM=xterm")],
        )
        .unwrap();
    let child_scratch = runtime
        .map_anonymous_memory(app, 0x2000, true, true, false, "child-scratch")
        .unwrap();

    let child = runtime
        .spawn_process_copy_vm("forked", Some(init), SchedulerClass::Interactive, app)
        .unwrap();
    let info = runtime.process_info(child).unwrap();
    assert_eq!(info.image_path, "/bin/app");
    assert_eq!(info.cwd, "/opt");
    assert!(info.shared_memory_region_count >= 1);
    assert!(info.copy_on_write_region_count >= 1);
    let cmdline = runtime
        .read_procfs_path(&format!("/proc/{}/cmdline", child.raw()))
        .unwrap();
    assert_eq!(cmdline, b"app\0--fork\0");
    let environ = runtime
        .read_procfs_path(&format!("/proc/{}/environ", child.raw()))
        .unwrap();
    assert_eq!(environ, b"TERM=xterm\0");
    let parent_maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("/bin/app"));
    assert!(maps.contains("[anon:child-scratch]"));
    assert!(maps.contains("refs=2"));
    assert!(maps.contains("cow"));
    assert!(parent_maps.contains("cow"));
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("owners=2"));
    let touch = runtime
        .touch_memory(child, child_scratch, 0x1000, true)
        .unwrap();
    assert_eq!(touch.pages_touched, 1);
    assert_eq!(touch.faulted_pages, 1);
    assert_eq!(touch.cow_faulted_pages, 1);
    let child_vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(child_vmobjects.contains("[cow]"));
    assert!(child_vmobjects.contains("shadow="));
    assert!(child_vmobjects.contains("faults=1(r=0,w=0,cow=1)"));

    let layouts = runtime.inspect_vm_object_layouts(child).unwrap();
    let cow_layout = layouts
        .into_iter()
        .find(|layout| layout.shadow_source_id.is_some())
        .expect("cow-derived vm object must expose a shadow source");
    assert_eq!(cow_layout.shadow_source_offset, 0);
}

#[test]
fn runtime_tracks_shadow_depth_across_multiple_cow_generations() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = runtime
        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x2000, true, true, false, "shadow-chain")
        .unwrap();

    let child = runtime
        .spawn_process_copy_vm("child", Some(init), SchedulerClass::Interactive, parent)
        .unwrap();
    runtime.touch_memory(child, scratch, 0x1000, true).unwrap();

    let grandchild = runtime
        .spawn_process_copy_vm("grandchild", Some(init), SchedulerClass::Interactive, child)
        .unwrap();
    runtime
        .touch_memory(grandchild, scratch, 0x1000, true)
        .unwrap();

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", grandchild.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("depth=2"));

    let layouts = runtime.inspect_vm_object_layouts(grandchild).unwrap();
    let nested = layouts
        .into_iter()
        .find(|layout| layout.shadow_depth == 2)
        .expect("nested cow object must report shadow depth 2");
    assert!(nested.shadow_source_id.is_some());
}

#[test]
fn runtime_tracks_nonzero_shadow_offsets_across_multiple_cow_generations() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = runtime
        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x3000, true, true, false, "shadow-offset-chain")
        .unwrap();

    let child = runtime
        .spawn_process_copy_vm("child", Some(init), SchedulerClass::Interactive, parent)
        .unwrap();
    runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();

    let grandchild = runtime
        .spawn_process_copy_vm("grandchild", Some(init), SchedulerClass::Interactive, child)
        .unwrap();
    runtime
        .touch_memory(grandchild, scratch + 0x1000, 0x1000, true)
        .unwrap();

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", grandchild.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("shadow="));
    assert!(vmobjects.contains("@00001000/depth=2"));

    let layouts = runtime.inspect_vm_object_layouts(grandchild).unwrap();
    let nested = layouts
        .into_iter()
        .find(|layout| layout.shadow_depth == 2 && layout.shadow_source_offset == 0x1000)
        .expect("nested cow object must retain the non-zero shadow offset");
    assert!(nested.shadow_source_id.is_some());
}

#[test]
fn runtime_reuses_shadow_objects_for_adjacent_partial_cow_faults() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = runtime
        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x3000, true, true, false, "shadow-reuse")
        .unwrap();
    let child = runtime
        .spawn_process_copy_vm("child", Some(init), SchedulerClass::Interactive, parent)
        .unwrap();

    runtime.touch_memory(child, scratch, 0x1000, true).unwrap();
    runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();

    let layouts = runtime.inspect_vm_object_layouts(child).unwrap();
    let shadow_layouts = layouts
        .into_iter()
        .filter(|layout| layout.shadow_source_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(shadow_layouts.len(), 1);
    assert_eq!(shadow_layouts[0].shadow_source_offset, 0);
    assert_eq!(shadow_layouts[0].shadow_depth, 1);
    assert_eq!(
        shadow_layouts[0]
            .segments
            .iter()
            .map(|segment| segment.byte_len)
            .sum::<u64>(),
        0x2000
    );

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(vmobjects.matches("[cow]").count(), 1);
    assert!(vmobjects.contains("committed=2\tresident=2\tdirty=2\taccessed=2"));
    assert!(vmobjects.contains("faults=2(r=0,w=0,cow=2)"));
}

#[test]
fn runtime_reuses_shadow_objects_for_reverse_adjacent_partial_cow_faults() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = runtime
        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x3000, true, true, false, "shadow-reuse-reverse")
        .unwrap();
    let child = runtime
        .spawn_process_copy_vm("child", Some(init), SchedulerClass::Interactive, parent)
        .unwrap();

    runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();
    runtime.touch_memory(child, scratch, 0x1000, true).unwrap();

    let layouts = runtime.inspect_vm_object_layouts(child).unwrap();
    let shadow_layouts = layouts
        .into_iter()
        .filter(|layout| layout.shadow_source_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(shadow_layouts.len(), 1);
    assert_eq!(shadow_layouts[0].shadow_source_offset, 0);
    assert_eq!(shadow_layouts[0].shadow_depth, 1);
    assert_eq!(shadow_layouts[0].segments[0].byte_offset, 0);
    assert_eq!(
        shadow_layouts[0]
            .segments
            .iter()
            .map(|segment| segment.byte_len)
            .sum::<u64>(),
        0x2000
    );

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(vmobjects.matches("[cow]").count(), 1);
    assert!(vmobjects.contains("shadow="));
    assert!(vmobjects.contains("@00000000/depth=1"));
}

#[test]
fn runtime_merges_shadow_objects_when_a_fault_bridges_both_sides() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = runtime
        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let scratch = runtime
        .map_anonymous_memory(parent, 0x3000, true, true, false, "shadow-bridge")
        .unwrap();
    let child = runtime
        .spawn_process_copy_vm("child", Some(init), SchedulerClass::Interactive, parent)
        .unwrap();

    runtime.touch_memory(child, scratch, 0x1000, true).unwrap();
    runtime
        .touch_memory(child, scratch + 0x2000, 0x1000, true)
        .unwrap();
    runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();

    let layouts = runtime.inspect_vm_object_layouts(child).unwrap();
    let shadow_layouts = layouts
        .into_iter()
        .filter(|layout| layout.shadow_source_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(shadow_layouts.len(), 1);
    assert_eq!(shadow_layouts[0].shadow_source_offset, 0);
    assert_eq!(shadow_layouts[0].shadow_depth, 1);
    assert_eq!(shadow_layouts[0].segments[0].byte_offset, 0);
    assert_eq!(
        shadow_layouts[0]
            .segments
            .iter()
            .map(|segment| segment.byte_len)
            .sum::<u64>(),
        0x3000
    );

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(vmobjects.matches("[cow]").count(), 1);
    assert!(vmobjects.contains("committed=3\tresident=3\tdirty=3\taccessed=3"));
    assert!(vmobjects.contains("faults=3(r=0,w=0,cow=3)"));
}

#[test]
fn runtime_vm_quarantine_blocks_touch_until_release_and_exposes_state() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_300), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_301), 0),
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
        .create_vfs_node("/lib/libquarantine.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libquarantine.so".to_string(),
            0x2000,
            0xb000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x2000, true, true, false)
        .unwrap();

    let vm_object_id = runtime.resolve_vm_object_id(app, mapped, 0x2000).unwrap();
    runtime.quarantine_vm_object(app, vm_object_id, 77).unwrap();

    let layouts = runtime.inspect_vm_object_layouts(app).unwrap();
    let quarantined = layouts
        .into_iter()
        .find(|layout| layout.object_id == vm_object_id)
        .unwrap();
    assert!(quarantined.quarantined);
    assert_eq!(quarantined.quarantine_reason, 77);

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("quarantined=1\treason=77"));

    assert_eq!(
        runtime.touch_memory(app, mapped, 0x1000, true),
        Err(RuntimeError::Process(ProcessError::MemoryQuarantined {
            vm_object_id
        }))
    );

    let vmdecisions = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmdecisions", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmdecisions.contains("agent=quarantine-state"));
    assert!(vmdecisions.contains("agent=quarantine-block"));

    runtime
        .release_vm_object_quarantine(app, vm_object_id)
        .unwrap();
    let layouts = runtime.inspect_vm_object_layouts(app).unwrap();
    let released = layouts
        .into_iter()
        .find(|layout| layout.object_id == vm_object_id)
        .unwrap();
    assert!(!released.quarantined);
    assert_eq!(released.quarantine_reason, 0);

    let touch = runtime.touch_memory(app, mapped, 0x1000, true).unwrap();
    assert_eq!(touch.vm_object_id, vm_object_id);
    assert_eq!(touch.pages_touched, 1);

    let vmepisodes = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmepisodes", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmepisodes.contains("kind=quarantine"));
    assert!(vmepisodes.contains("blocked=yes"));
    assert!(vmepisodes.contains("released=yes"));
}

#[test]
fn runtime_vm_reclaim_pressure_skips_quarantine_then_syncs_and_evicts_file_pages() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_302), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(9_303), 0),
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
        .create_vfs_node("/lib/libpressure.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_file_memory(
            app,
            "/lib/libpressure.so".to_string(),
            0x3000,
            0xc000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app, mapped, 0x3000, true, true, false)
        .unwrap();
    runtime.touch_memory(app, mapped, 0x3000, true).unwrap();

    let vm_object_id = runtime.resolve_vm_object_id(app, mapped, 0x3000).unwrap();
    runtime.quarantine_vm_object(app, vm_object_id, 99).unwrap();
    assert_eq!(runtime.reclaim_memory_pressure(app, 2).unwrap(), 0);

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("resident=3\tdirty=3\taccessed=3"));
    assert!(vmobjects.contains("quarantined=1\treason=99"));

    runtime
        .release_vm_object_quarantine(app, vm_object_id)
        .unwrap();
    let reclaimed = runtime.reclaim_memory_pressure(app, 2).unwrap();
    assert!(reclaimed >= 2);

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("dirty=0"));
    assert!(vmobjects.contains("resident=0"));

    let reread = runtime.touch_memory(app, mapped, 0x3000, false).unwrap();
    assert_eq!(reread.pages_touched, 3);
    assert_eq!(reread.faulted_pages, 3);

    let vmdecisions = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmdecisions", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmdecisions.contains("agent=pressure-trigger"));
    assert!(vmdecisions.contains("agent=sync"));
    assert!(vmdecisions.contains("agent=advice"));
    assert!(vmdecisions.contains("agent=pressure-victim"));
}

#[test]
fn runtime_vm_global_reclaim_pressure_targets_cross_process_file_residency() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app_a = runtime
        .spawn_process("app-a", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let app_b = runtime
        .spawn_process("app-b", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(9_304), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(9_305), 0),
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
        .create_vfs_node("/lib/libglobal-a.so", ObjectKind::File, lib)
        .unwrap();
    runtime
        .create_vfs_node("/lib/libglobal-b.so", ObjectKind::File, lib)
        .unwrap();

    let mapped_a = runtime
        .map_file_memory(
            app_a,
            "/lib/libglobal-a.so".to_string(),
            0x3000,
            0xe000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app_a, mapped_a, 0x3000, true, true, false)
        .unwrap();
    runtime.touch_memory(app_a, mapped_a, 0x3000, true).unwrap();

    let mapped_b = runtime
        .map_file_memory(
            app_b,
            "/lib/libglobal-b.so".to_string(),
            0x1000,
            0xf000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(app_b, mapped_b, 0x1000, true, true, false)
        .unwrap();
    runtime.touch_memory(app_b, mapped_b, 0x1000, true).unwrap();

    let reclaimed = runtime.reclaim_memory_pressure_global(3).unwrap();
    assert!(reclaimed >= 3);

    let vmobjects_a = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app_a.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects_a.contains("/lib/libglobal-a.so"));
    assert!(vmobjects_a.contains("resident=0"));
    assert!(vmobjects_a.contains("dirty=0"));

    let vmobjects_b = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app_b.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects_b.contains("/lib/libglobal-b.so"));
    assert!(vmobjects_b.contains("resident=1"));
    assert!(vmobjects_b.contains("dirty=1"));

    let vmdecisions_a = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmdecisions", app_a.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmdecisions_a.contains("agent=sync"));
    assert!(vmdecisions_a.contains("agent=advice"));
    assert!(vmdecisions_a.contains("agent=pressure-victim"));

    let vmepisodes_a = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmepisodes", app_a.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmepisodes_a.contains("kind=reclaim"));
    assert!(vmepisodes_a.contains("evicted=yes"));

    let vmdecisions_b = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmdecisions", app_b.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(!vmdecisions_b.contains("agent=pressure-victim"));
}

#[test]
fn observe_contract_gates_process_vm_procfs_reads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("vm-target", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("vm-observer", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            target,
            ObjectHandle::new(Handle::new(9_401), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            target,
            ObjectHandle::new(Handle::new(9_402), 0),
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
        .create_vfs_node("/lib/libinspect.so", ObjectKind::File, lib)
        .unwrap();
    let mapped = runtime
        .map_file_memory(
            target,
            "/lib/libinspect.so".to_string(),
            0x2000,
            0xd000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(target, mapped, 0x2000, true, true, false)
        .unwrap();
    runtime.touch_memory(target, mapped, 0x2000, true).unwrap();

    let vm_object_id = runtime
        .resolve_vm_object_id(target, mapped, 0x2000)
        .unwrap();
    runtime
        .quarantine_vm_object(target, vm_object_id, 77)
        .unwrap();
    assert_eq!(
        runtime.touch_memory(target, mapped, 0x1000, true),
        Err(RuntimeError::Process(ProcessError::MemoryQuarantined {
            vm_object_id
        }))
    );

    for path in [
        format!("/proc/{}/vmobjects", target.raw()),
        format!("/proc/{}/vmdecisions", target.raw()),
        format!("/proc/{}/vmepisodes", target.raw()),
    ] {
        let denied = runtime.read_procfs_path_for(observer, &path).unwrap_err();
        assert_eq!(
            denied,
            RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
                kind: ContractKind::Observe
            })
        );
    }

    let domain = runtime.create_domain(observer, None, "obs").unwrap();
    let resource = runtime
        .create_resource(observer, domain, ResourceKind::Namespace, "inspect")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Observe)
        .unwrap();
    let contract = runtime
        .create_contract(observer, domain, resource, ContractKind::Observe, "observe")
        .unwrap();
    runtime.bind_process_contract(observer, contract).unwrap();

    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/vmobjects", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("quarantined=1"));
    assert!(vmobjects.contains("libinspect.so"));

    let vmdecisions = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/vmdecisions", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmdecisions.contains("agent=quarantine-state"));
    assert!(vmdecisions.contains("agent=quarantine-block"));

    let vmepisodes = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/vmepisodes", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmepisodes.contains("kind=quarantine"));
    assert!(vmepisodes.contains("blocked=yes"));
}
