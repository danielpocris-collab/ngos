use super::*;
use platform_hal::{DeviceIdentity, DevicePlatform, GpuPlatform};
use platform_x86_64::device_platform::PciAddress;
use platform_x86_64::{
    SyntheticPciConfigBackend, X86_64DevicePlatform, X86_64DevicePlatformConfig,
};
#[test]
fn descriptor_namespace_opens_duplicates_and_closes_objects() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let capability = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(5_000), 1),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "vfs-root",
        )
        .unwrap();
    let mut namespace = DescriptorNamespace::new();

    let fd0 = namespace
        .open(
            runtime.processes(),
            runtime.capabilities(),
            owner,
            capability,
            ObjectKind::Directory,
            "/",
        )
        .unwrap();
    let fd1 = namespace
        .dup(runtime.processes(), runtime.capabilities(), fd0)
        .unwrap();

    assert_eq!(fd0.raw(), 0);
    assert_eq!(fd1.raw(), 1);
    assert_eq!(namespace.get(fd1).unwrap().kind(), ObjectKind::Directory);
    assert_eq!(namespace.by_owner(owner), vec![fd0, fd1]);

    let closed = namespace.close(fd0).unwrap();
    assert_eq!(closed.name(), "/");
    assert_eq!(namespace.get(fd0), Err(DescriptorError::InvalidDescriptor));
}

#[test]
fn descriptor_namespace_honors_cloexec_and_rights() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("game", None, SchedulerClass::Interactive)
        .unwrap();
    let readonly = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(6_000), 0),
            CapabilityRights::READ,
            "asset-pack",
        )
        .unwrap();
    let duplicable = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(6_001), 0),
            CapabilityRights::READ | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();
    let mut namespace = DescriptorNamespace::new();

    let file = namespace
        .open(
            runtime.processes(),
            runtime.capabilities(),
            owner,
            readonly,
            ObjectKind::File,
            "assets.pak",
        )
        .unwrap();
    namespace.set_cloexec(file, true).unwrap();

    let socket = namespace
        .open(
            runtime.processes(),
            runtime.capabilities(),
            owner,
            duplicable,
            ObjectKind::Socket,
            "render.sock",
        )
        .unwrap();
    assert_eq!(
        namespace.dup(runtime.processes(), runtime.capabilities(), file),
        Err(DescriptorError::RightDenied {
            required: CapabilityRights::DUPLICATE,
            actual: CapabilityRights::READ,
        })
    );
    assert_eq!(
        namespace
            .dup(runtime.processes(), runtime.capabilities(), socket)
            .unwrap()
            .raw(),
        2
    );

    let closed = namespace.close_on_exec(owner);
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].fd(), file);
    assert_eq!(namespace.get(file), Err(DescriptorError::InvalidDescriptor));
    assert!(namespace.get(socket).is_ok());
}

#[test]
fn runtime_integrates_vfs_mount_create_and_open() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime.mount("/compat/foreign", "foreign-root").unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();

    let fd = runtime.open_path(owner, "/dev/gpu0").unwrap();
    assert_eq!(fd.raw(), 0);
    assert_eq!(runtime.vfs().mounts().len(), 2);
    assert_eq!(runtime.descriptors_for(owner).unwrap(), vec![fd]);
    let io = runtime.inspect_io(owner, fd).unwrap();
    assert_eq!(io.kind(), ObjectKind::Device);
    assert!(io.capabilities().contains(IoCapabilities::CONTROL));
    assert_eq!(runtime.io_registry().by_owner(owner), vec![fd]);
}

#[test]
fn runtime_integrates_driver_nodes_and_control_surface() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_010), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_011), 0),
            CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "render-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/render", ObjectKind::Driver, driver)
        .unwrap();

    let fd = runtime.open_path(owner, "/drv/render").unwrap();
    let io = runtime.inspect_io(owner, fd).unwrap();
    assert_eq!(io.kind(), ObjectKind::Driver);
    assert_eq!(io.name(), "/drv/render");
    assert!(!io.capabilities().contains(IoCapabilities::READ));
    assert!(io.capabilities().contains(IoCapabilities::WRITE));
    assert!(io.capabilities().contains(IoCapabilities::CONTROL));

    let events = runtime.poll_io(owner, fd).unwrap();
    assert!(events.contains(IoPollEvents::WRITABLE));
    assert!(events.contains(IoPollEvents::PRIORITY));
    assert_eq!(runtime.control_io(owner, fd, 0x90).unwrap(), 0x91);

    let status = runtime.stat_path("/drv/render").unwrap();
    assert_eq!(status.kind, ObjectKind::Driver);
    assert!(status.inode > 0);
}

#[test]
fn runtime_integrates_memory_and_channel_nodes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_020), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let memory = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_021), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "shared-mem",
        )
        .unwrap();
    let channel = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_022), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "ipc-chan",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/mem", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/mem/shared", ObjectKind::Memory, memory)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render", ObjectKind::Channel, channel)
        .unwrap();

    let mem_fd = runtime.open_path(owner, "/mem/shared").unwrap();
    let chan_fd = runtime.open_path(owner, "/ipc/render").unwrap();

    assert_eq!(runtime.write_io(owner, mem_fd, b":delta").unwrap(), 6);
    let mem_read = runtime.read_io(owner, mem_fd, 64).unwrap();
    assert!(String::from_utf8_lossy(&mem_read).contains("object:/mem/shared"));

    assert_eq!(runtime.write_io(owner, chan_fd, b":frame").unwrap(), 6);
    let chan_events = runtime.poll_io(owner, chan_fd).unwrap();
    assert!(chan_events.contains(IoPollEvents::READABLE));
    assert!(chan_events.contains(IoPollEvents::WRITABLE));
    let chan_read = runtime.read_io(owner, chan_fd, 64).unwrap();
    assert_eq!(chan_read, b":frame");
    let chan_events = runtime.poll_io(owner, chan_fd).unwrap();
    assert!(!chan_events.contains(IoPollEvents::READABLE));
    assert!(chan_events.contains(IoPollEvents::WRITABLE));

    let mem_status = runtime.stat_path("/mem/shared").unwrap();
    assert_eq!(mem_status.kind, ObjectKind::Memory);
    let chan_status = runtime.stat_path("/ipc/render").unwrap();
    assert_eq!(chan_status.kind, ObjectKind::Channel);
}

#[test]
fn runtime_reports_introspection_metadata_for_driver_memory_and_channel() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_030), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_031), 0),
            CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "render-driver",
        )
        .unwrap();
    let memory = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_032), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "shared-mem",
        )
        .unwrap();
    let channel = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(10_033), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "ipc-chan",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/mem", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/render", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .create_vfs_node("/mem/shared", ObjectKind::Memory, memory)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render", ObjectKind::Channel, channel)
        .unwrap();

    let driver_fd = runtime.open_path(owner, "/drv/render").unwrap();
    let mem_fd = runtime.open_path(owner, "/mem/shared").unwrap();
    let chan_fd = runtime.open_path(owner, "/ipc/render").unwrap();

    let entries = runtime.filedesc_entries(owner).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].fd, driver_fd);
    assert_eq!(entries[0].kind_code, 12);
    assert_eq!(entries[1].fd, mem_fd);
    assert_eq!(entries[1].kind_code, 1);
    assert_eq!(entries[2].fd, chan_fd);
    assert_eq!(entries[2].kind_code, 2);

    let kinfo = runtime.kinfo_file_entries(owner).unwrap();
    assert_eq!(kinfo.len(), 3);
    assert_eq!(kinfo[0].fd, driver_fd);
    assert_eq!(kinfo[0].kind_code, 12);
    assert_eq!(kinfo[0].socket_type, None);
    assert_eq!(kinfo[1].fd, mem_fd);
    assert_eq!(kinfo[1].kind_code, 1);
    assert_eq!(kinfo[1].socket_type, None);
    assert_eq!(kinfo[2].fd, chan_fd);
    assert_eq!(kinfo[2].kind_code, 2);
    assert_eq!(kinfo[2].socket_domain, Some(1));
    assert_eq!(kinfo[2].socket_type, Some(5));
    assert_eq!(kinfo[2].socket_protocol, Some(0));
}

#[test]
fn vfs_namespace_mounts_creates_and_resolves_nodes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(9_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let games = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(9_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "games",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(9_002), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu",
        )
        .unwrap();
    let mut vfs = VfsNamespace::new();

    vfs.create_node("/", ObjectKind::Directory, root).unwrap();
    assert_eq!(
        vfs.create_node("/", ObjectKind::Directory, root),
        Err(VfsError::AlreadyExists)
    );
    vfs.create_node("/games", ObjectKind::Directory, games)
        .unwrap();
    vfs.create_node("/games/doom", ObjectKind::File, games)
        .unwrap();
    vfs.create_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    vfs.create_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    vfs.mount("/compat/foreign", "foreign-root").unwrap();

    let fd = vfs.resolve(&mut runtime, owner, "/dev/gpu0").unwrap();
    assert_eq!(fd.raw(), 0);
    assert_eq!(vfs.node("/games/doom").unwrap().kind(), ObjectKind::File);
    assert_eq!(vfs.mounts().len(), 2);
}

#[test]
fn vfs_namespace_can_rename_subtrees_and_unlink_leaf_nodes() {
    let cap = CapabilityId::from_handle(ObjectHandle::new(Handle::new(1), 0));
    let mut vfs = VfsNamespace::new();

    vfs.create_node("/", ObjectKind::Directory, cap).unwrap();
    vfs.create_node("/games", ObjectKind::Directory, cap)
        .unwrap();
    vfs.create_node("/games/doom", ObjectKind::Directory, cap)
        .unwrap();
    vfs.create_node("/games/doom/doom.wad", ObjectKind::File, cap)
        .unwrap();

    vfs.rename_node("/games/doom", "/games/idtech").unwrap();
    assert_eq!(
        vfs.node("/games/idtech/doom.wad").unwrap().kind(),
        ObjectKind::File
    );
    assert_eq!(vfs.node("/games/doom"), Err(VfsError::NotFound));

    assert_eq!(
        vfs.remove_node("/games/idtech"),
        Err(VfsError::DirectoryNotEmpty)
    );

    let removed = vfs.remove_node("/games/idtech/doom.wad").unwrap();
    assert_eq!(removed.kind(), ObjectKind::File);
    assert_eq!(vfs.node("/games/idtech/doom.wad"), Err(VfsError::NotFound));
}

#[test]
fn vfs_namespace_can_create_and_resolve_symlinks() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let cap = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(9_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let mut vfs = VfsNamespace::new();

    vfs.create_node("/", ObjectKind::Directory, cap).unwrap();
    vfs.create_node("/bin", ObjectKind::Directory, cap).unwrap();
    vfs.create_node("/bin/sh", ObjectKind::File, cap).unwrap();
    vfs.create_symlink("/usr-bin-sh", "/bin/sh", cap).unwrap();

    assert_eq!(vfs.readlink("/usr-bin-sh").unwrap(), "/bin/sh");
    assert_eq!(vfs.node("/usr-bin-sh").unwrap().kind(), ObjectKind::Symlink);

    let fd = vfs.resolve(&mut runtime, owner, "/usr-bin-sh").unwrap();
    let io = runtime.inspect_io(owner, fd).unwrap();
    assert_eq!(io.kind(), ObjectKind::File);
    assert_eq!(io.name(), "/bin/sh");
}

#[test]
fn vfs_namespace_rejects_invalid_paths_and_missing_parents() {
    let mut vfs = VfsNamespace::new();
    let cap = CapabilityId::from_handle(ObjectHandle::new(Handle::new(1), 0));

    assert_eq!(
        vfs.create_node("relative", ObjectKind::File, cap),
        Err(VfsError::InvalidPath)
    );
    assert_eq!(
        vfs.create_node("/missing/file", ObjectKind::File, cap),
        Err(VfsError::NotDirectory)
    );
    assert_eq!(vfs.mount("/bad/../path", "bad"), Err(VfsError::InvalidPath));
    vfs.create_node("/", ObjectKind::Directory, cap).unwrap();
    vfs.create_node("/games", ObjectKind::Directory, cap)
        .unwrap();
    vfs.create_node("/games/doom", ObjectKind::Directory, cap)
        .unwrap();
    vfs.mount("/compat/foreign", "foreign-root").unwrap();
    assert_eq!(
        vfs.rename_node("/games", "/games/doom/subdir"),
        Err(VfsError::InvalidPath)
    );
    assert_eq!(
        vfs.rename_node("/games", "/compat/foreign/games"),
        Err(VfsError::CrossMountRename)
    );
}

#[test]
fn syscall_surface_handles_vfs_operations() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(11_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(11_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu",
        )
        .unwrap();

    assert!(matches!(
        surface
            .dispatch(
                context.clone(),
                Syscall::CreateVfsNode {
                    path: String::from("/"),
                    kind: ObjectKind::Directory,
                    capability: root,
                },
            )
            .unwrap(),
        SyscallResult::VfsNodeCreated
    ));
    assert!(matches!(
        surface
            .dispatch(
                context.clone(),
                Syscall::Mount {
                    mount_path: String::from("/compat/foreign"),
                    name: String::from("foreign-root"),
                },
            )
            .unwrap(),
        SyscallResult::Mounted
    ));
    let _ = surface
        .dispatch(
            context.clone(),
            Syscall::CreateVfsNode {
                path: String::from("/dev"),
                kind: ObjectKind::Directory,
                capability: root,
            },
        )
        .unwrap();
    let _ = surface
        .dispatch(
            context.clone(),
            Syscall::CreateVfsNode {
                path: String::from("/dev/gpu0"),
                kind: ObjectKind::Device,
                capability: device,
            },
        )
        .unwrap();
    assert!(matches!(
        surface
            .dispatch(
                context.clone(),
                Syscall::CreateVfsSymlink {
                    path: String::from("/dev/gpu-link"),
                    target: String::from("/dev/gpu0"),
                    capability: root,
                },
            )
            .unwrap(),
        SyscallResult::VfsSymlinkCreated
    ));
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadLink {
                path: String::from("/dev/gpu-link"),
            },
        )
        .unwrap()
    {
        SyscallResult::LinkTarget(target) => assert_eq!(target, "/dev/gpu0"),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    assert!(matches!(
        surface
            .dispatch(
                context.clone(),
                Syscall::RenamePath {
                    from: String::from("/dev/gpu0"),
                    to: String::from("/dev/gpu-render"),
                },
            )
            .unwrap(),
        SyscallResult::VfsNodeRenamed
    ));

    match surface
        .dispatch(
            context.clone(),
            Syscall::OpenPath {
                owner: app,
                path: String::from("/dev/gpu-render"),
            },
        )
        .unwrap()
    {
        SyscallResult::PathOpened(fd) => assert_eq!(fd.raw(), 0),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    assert_eq!(
        surface.runtime.open_path(app, "/dev/gpu-link"),
        Err(RuntimeError::Vfs(VfsError::NotFound))
    );
    assert!(matches!(
        surface
            .dispatch(
                context,
                Syscall::UnlinkPath {
                    path: String::from("/dev/gpu-render"),
                },
            )
            .unwrap(),
        SyscallResult::VfsNodeRemoved
    ));
    assert_eq!(
        surface.runtime.open_path(app, "/dev/gpu-render"),
        Err(RuntimeError::Vfs(VfsError::NotFound))
    );
}

#[test]
fn runtime_tracks_io_objects_through_descriptor_lifecycle() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let capability = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(12_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    let fd0 = runtime
        .open_descriptor(owner, capability, ObjectKind::Socket, "render.sock")
        .unwrap();
    let fd1 = runtime.duplicate_descriptor(owner, fd0).unwrap();

    let io0 = runtime.inspect_io(owner, fd0).unwrap();
    assert_eq!(io0.kind(), ObjectKind::Socket);
    assert!(io0.capabilities().contains(IoCapabilities::POLL));
    assert_eq!(runtime.io_registry().by_owner(owner), vec![fd0, fd1]);

    let _ = runtime.close_descriptor(owner, fd0).unwrap();
    assert_eq!(
        runtime.inspect_io(owner, fd0),
        Err(RuntimeError::Descriptor(DescriptorError::InvalidDescriptor))
    );
}

#[test]
fn runtime_supports_dup2_style_remap_and_descriptor_flags() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("dup", None, SchedulerClass::Interactive)
        .unwrap();
    let capability = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(16_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "dup.sock",
        )
        .unwrap();

    let fd0 = runtime
        .open_descriptor(owner, capability, ObjectKind::Socket, "dup.sock")
        .unwrap();
    runtime.set_descriptor_nonblock(owner, fd0, true).unwrap();
    runtime.set_descriptor_cloexec(owner, fd0, true).unwrap();

    let remapped = runtime
        .duplicate_descriptor_to(owner, fd0, Descriptor::new(7))
        .unwrap();
    assert_eq!(remapped.raw(), 7);

    let flags = runtime.descriptor_flags(owner, remapped).unwrap();
    assert!(flags.cloexec);
    assert!(flags.nonblock);
    let io = runtime.inspect_io(owner, remapped).unwrap();
    assert!(io.nonblock());
    let system = runtime.inspect_system();
    assert!(system.io_agent_decisions.iter().any(|entry| {
        entry.agent == IoAgentKind::DuplicateDescriptorAgent
            && entry.owner == owner.raw()
            && entry.fd == u64::from(remapped.raw())
            && entry.detail0 == u64::from(fd0.raw())
    }));
}

#[test]
fn runtime_exposes_stat_and_fstat_metadata() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("stat", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(18_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let file = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(18_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "save",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/data", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/data/save.dat", ObjectKind::File, file)
        .unwrap();
    let fd = runtime.open_path(owner, "/data/save.dat").unwrap();
    runtime.write_io(owner, fd, b":sync").unwrap();
    runtime.set_descriptor_cloexec(owner, fd, true).unwrap();

    let stat = runtime.stat_path("/data/save.dat").unwrap();
    assert_eq!(stat.kind, ObjectKind::File);
    assert!(stat.inode > 0);

    let fstat = runtime.fstat_descriptor(owner, fd).unwrap();
    assert_eq!(fstat.path, "/data/save.dat");
    assert!(fstat.cloexec);
    assert!(fstat.size >= stat.size);
}

#[test]
fn runtime_persists_vfs_file_content_across_open_close_and_reopen() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("persist", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(18_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let file = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(18_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "script",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/tmp", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/tmp/script.ngs", ObjectKind::File, file)
        .unwrap();

    let writer = runtime.open_path(owner, "/tmp/script.ngs").unwrap();
    assert_eq!(runtime.read_io(owner, writer, 64).unwrap(), b"");
    runtime.write_io(owner, writer, b"echo persisted").unwrap();
    runtime.close_descriptor(owner, writer).unwrap();

    let stat = runtime.stat_path("/tmp/script.ngs").unwrap();
    assert_eq!(stat.size, b"echo persisted".len() as u64);

    let reader = runtime.open_path(owner, "/tmp/script.ngs").unwrap();
    let bytes = runtime.read_io(owner, reader, 64).unwrap();
    assert_eq!(bytes, b"echo persisted");
    let system = runtime.inspect_system();
    assert!(system.io_agent_decisions.iter().any(|entry| {
        entry.agent == IoAgentKind::WriteAgent
            && entry.owner == owner.raw()
            && entry.fd == u64::from(writer.raw())
            && entry.detail0 == b"echo persisted".len() as u64
    }));
    assert!(system.io_agent_decisions.iter().any(|entry| {
        entry.agent == IoAgentKind::ReadAgent
            && entry.owner == owner.raw()
            && entry.fd == u64::from(reader.raw())
    }));
    assert!(system.io_agent_decisions.iter().any(|entry| {
        entry.agent == IoAgentKind::CloseDescriptorAgent
            && entry.owner == owner.raw()
            && entry.fd == u64::from(writer.raw())
    }));
}

#[test]
fn syscall_surface_exposes_dup2_and_descriptor_flags() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let capability = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(17_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "flags.sock",
        )
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let fd = surface
        .runtime
        .open_descriptor(app, capability, ObjectKind::Socket, "flags.sock")
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::SetNonblock {
                owner: app,
                fd,
                nonblock: true,
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    let remapped = match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateDescriptorTo {
                owner: app,
                fd,
                target: Descriptor::new(11),
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorDuplicatedTo(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    assert_eq!(remapped.raw(), 11);

    match surface
        .dispatch(
            context,
            Syscall::GetDescriptorFlags {
                owner: app,
                fd: remapped,
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlags(flags) => assert!(flags.nonblock),
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_updates_cloexec_independently_after_duplication() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let capability = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(17_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "cloexec.sock",
        )
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let fd = surface
        .runtime
        .open_descriptor(app, capability, ObjectKind::Socket, "cloexec.sock")
        .unwrap();

    let remapped = match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateDescriptorTo {
                owner: app,
                fd,
                target: Descriptor::new(13),
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorDuplicatedTo(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    assert_eq!(remapped.raw(), 13);

    match surface
        .dispatch(
            context.clone(),
            Syscall::SetCloexec {
                owner: app,
                fd: remapped,
                cloexec: true,
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::GetDescriptorFlags {
                owner: app,
                fd: remapped,
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlags(flags) => assert!(flags.cloexec),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context, Syscall::GetDescriptorFlags { owner: app, fd })
        .unwrap()
    {
        SyscallResult::DescriptorFlags(flags) => assert!(!flags.cloexec),
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_exposes_stat_and_fstat() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let cap = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(19_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "doom",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, cap)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/doom.wad", ObjectKind::File, cap)
        .unwrap();
    let fd = surface.runtime.open_path(app, "/doom.wad").unwrap();
    let context = SyscallContext::kernel(bootstrap);

    match surface
        .dispatch(
            context.clone(),
            Syscall::StatPath {
                path: String::from("/doom.wad"),
            },
        )
        .unwrap()
    {
        SyscallResult::FileStatus(status) => {
            assert_eq!(status.kind, ObjectKind::File);
            assert!(status.inode > 0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface
        .runtime
        .create_vfs_symlink("/doom-current", "/doom.wad", cap)
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::StatPath {
                path: String::from("/doom-current"),
            },
        )
        .unwrap()
    {
        SyscallResult::FileStatus(status) => {
            assert_eq!(status.kind, ObjectKind::File);
            assert_eq!(status.path, "/doom.wad");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::LstatPath {
                path: String::from("/doom-current"),
            },
        )
        .unwrap()
    {
        SyscallResult::FileStatus(status) => {
            assert_eq!(status.kind, ObjectKind::Symlink);
            assert_eq!(status.path, "/doom-current");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context, Syscall::StatDescriptor { owner: app, fd })
        .unwrap()
    {
        SyscallResult::FileStatus(status) => {
            assert_eq!(status.path, "/doom.wad");
            assert_eq!(status.kind, ObjectKind::File);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn runtime_supports_statfs_fcntl_and_readiness() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("runtime", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(20_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(20_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime.mount("/compat/foreign", "foreign-root").unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/render.sock", ObjectKind::Socket, socket)
        .unwrap();

    let statfs = runtime.statfs("/compat/foreign/game").unwrap();
    assert_eq!(statfs.mount_name, "foreign-root");
    assert_eq!(statfs.path, "/compat/foreign");
    assert_eq!(statfs.mount_count, 2);

    let fd = runtime.open_path(owner, "/run/render.sock").unwrap();
    match runtime
        .fcntl(owner, fd, FcntlCmd::SetFl { nonblock: true })
        .unwrap()
    {
        FcntlResult::Updated(flags) => assert!(flags.nonblock),
        other => panic!("unexpected fcntl result: {other:?}"),
    }
    match runtime
        .fcntl(owner, fd, FcntlCmd::SetFd { cloexec: true })
        .unwrap()
    {
        FcntlResult::Updated(flags) => assert!(flags.cloexec),
        other => panic!("unexpected fcntl result: {other:?}"),
    }

    runtime
        .register_readiness(
            owner,
            fd,
            ReadinessInterest {
                readable: true,
                writable: true,
                priority: false,
            },
        )
        .unwrap();
    let ready = runtime.collect_ready().unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].fd, fd);
    assert!(ready[0].interest.readable);
    assert!(ready[0].interest.writable);

    let _closed = runtime.close_descriptor(owner, fd).unwrap();
    assert!(runtime.collect_ready().unwrap().is_empty());
}

#[test]
fn runtime_close_range_clears_readiness_registrations_for_closed_descriptors() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("close-range-readiness", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(20_050), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(20_051), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/ready.sock", ObjectKind::Socket, socket)
        .unwrap();

    let fd = runtime.open_path(owner, "/run/ready.sock").unwrap();
    runtime
        .register_readiness(
            owner,
            fd,
            ReadinessInterest {
                readable: true,
                writable: false,
                priority: false,
            },
        )
        .unwrap();
    assert_eq!(runtime.collect_ready().unwrap().len(), 1);

    runtime
        .close_range(owner, fd, Some(fd), CloseRangeMode::Close)
        .unwrap();
    assert!(runtime.collect_ready().unwrap().is_empty());
    assert!(!runtime.descriptors_for(owner).unwrap().contains(&fd));
}

#[test]
fn runtime_readiness_registration_replaces_existing_interest() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(20_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(20_101), 0),
            CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "render-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/render", ObjectKind::Driver, driver)
        .unwrap();
    let fd = runtime.open_path(owner, "/drv/render").unwrap();

    runtime
        .register_readiness(
            owner,
            fd,
            ReadinessInterest {
                readable: false,
                writable: true,
                priority: false,
            },
        )
        .unwrap();
    let first = runtime.collect_ready().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].fd, fd);
    assert!(first[0].interest.writable);

    runtime
        .register_readiness(
            owner,
            fd,
            ReadinessInterest {
                readable: true,
                writable: false,
                priority: false,
            },
        )
        .unwrap();
    let replaced = runtime.collect_ready().unwrap();
    assert!(replaced.is_empty());
    let io_decisions = runtime.recent_io_agent_decisions();
    assert!(io_decisions.iter().any(|entry| {
        entry.agent == IoAgentKind::ReadinessAgent
            && entry.owner == owner.raw()
            && entry.fd == u64::from(fd.raw())
            && entry.detail0 == 0b010
    }));
    assert!(io_decisions.iter().any(|entry| {
        entry.agent == IoAgentKind::ReadinessAgent
            && entry.owner == owner.raw()
            && entry.fd == u64::from(fd.raw())
            && entry.detail0 == 0b001
    }));
}

#[test]
fn syscall_surface_exposes_statfs_fcntl_and_readiness() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(21_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(21_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "socket",
        )
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/run/render.sock", ObjectKind::Socket, socket)
        .unwrap();
    surface
        .runtime
        .mount("/compat/foreign", "foreign-root")
        .unwrap();
    let fd = surface.runtime.open_path(app, "/run/render.sock").unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::StatFs {
                path: String::from("/compat/foreign/game"),
            },
        )
        .unwrap()
    {
        SyscallResult::FileSystemStatus(status) => {
            assert_eq!(status.mount_name, "foreign-root");
            assert_eq!(status.path, "/compat/foreign");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::FcntlDescriptor {
                owner: app,
                fd,
                cmd: FcntlCmd::SetFl { nonblock: true },
            },
        )
        .unwrap()
    {
        SyscallResult::FcntlResult(FcntlResult::Updated(flags)) => assert!(flags.nonblock),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::RegisterReadiness {
                owner: app,
                fd,
                interest: ReadinessInterest {
                    readable: true,
                    writable: true,
                    priority: false,
                },
            },
        )
        .unwrap()
    {
        SyscallResult::ReadinessRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context, Syscall::CollectReadiness)
        .unwrap()
    {
        SyscallResult::ReadinessEvents(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].fd, fd);
            assert!(events[0].interest.writable);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_replaces_existing_readiness_interest() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(21_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let driver = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(21_101), 0),
            CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "render-driver",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/drv/render", ObjectKind::Driver, driver)
        .unwrap();
    let fd = surface.runtime.open_path(app, "/drv/render").unwrap();
    let context = SyscallContext::kernel(bootstrap);

    match surface
        .dispatch(
            context.clone(),
            Syscall::RegisterReadiness {
                owner: app,
                fd,
                interest: ReadinessInterest {
                    readable: false,
                    writable: true,
                    priority: false,
                },
            },
        )
        .unwrap()
    {
        SyscallResult::ReadinessRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::CollectReadiness)
        .unwrap()
    {
        SyscallResult::ReadinessEvents(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].fd, fd);
            assert!(events[0].interest.writable);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::RegisterReadiness {
                owner: app,
                fd,
                interest: ReadinessInterest {
                    readable: true,
                    writable: false,
                    priority: false,
                },
            },
        )
        .unwrap()
    {
        SyscallResult::ReadinessRegistered => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context, Syscall::CollectReadiness)
        .unwrap()
    {
        SyscallResult::ReadinessEvents(events) => assert!(events.is_empty()),
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn device_driver_registry_binds_and_completes_requests_end_to_end() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("io-stack", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "nic0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_002), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "nic-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net0", ObjectKind::Device, device)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/net0", "/drv/net")
        .unwrap();

    let device_fd = runtime.open_path(owner, "/dev/net0").unwrap();
    let driver_fd = runtime.open_path(owner, "/drv/net").unwrap();

    assert_eq!(
        runtime.write_io(owner, device_fd, b"packet:hello").unwrap(),
        12
    );
    let driver_events = runtime.poll_io(owner, driver_fd).unwrap();
    assert!(driver_events.contains(IoPollEvents::READABLE));
    assert!(driver_events.contains(IoPollEvents::PRIORITY));

    let request = runtime.read_io(owner, driver_fd, 256).unwrap();
    let request_text = String::from_utf8_lossy(&request);
    assert!(request_text.contains("request:"));
    assert!(request_text.contains("device=/dev/net0"));
    assert!(request_text.contains("packet:hello"));

    assert_eq!(runtime.write_io(owner, driver_fd, b"ack:hello").unwrap(), 9);
    let device_events = runtime.poll_io(owner, device_fd).unwrap();
    assert!(device_events.contains(IoPollEvents::READABLE));
    let completion = runtime.read_io(owner, device_fd, 64).unwrap();
    assert_eq!(completion, b"ack:hello");

    let driver_info = runtime.driver_info_by_path("/drv/net").unwrap();
    assert_eq!(driver_info.bound_devices, vec![String::from("/dev/net0")]);
    assert_eq!(driver_info.completed_requests, 1);

    let device_info = runtime.device_info_by_path("/dev/net0").unwrap();
    assert_eq!(device_info.class, DeviceClass::Network);
    assert_eq!(device_info.driver.as_deref(), Some("/drv/net"));
    assert_eq!(device_info.submitted_requests, 1);
    assert_eq!(device_info.completed_requests, 1);
}

#[test]
fn device_driver_registry_tracks_rename_and_unlink_lifecycle() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("device-admin", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "disk0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_102), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "block-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/storage0", ObjectKind::Device, device)
        .unwrap();
    runtime
        .create_vfs_node("/drv/storage", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/storage0", "/drv/storage")
        .unwrap();
    runtime
        .configure_device_geometry("/dev/storage0", 512, 128 * 1024 * 1024)
        .unwrap();

    runtime
        .rename_path("/dev/storage0", "/dev/storage-main")
        .unwrap();
    runtime.rename_path("/drv/storage", "/drv/block").unwrap();

    let device_info = runtime.device_info_by_path("/dev/storage-main").unwrap();
    assert_eq!(device_info.class, DeviceClass::Storage);
    assert_eq!(device_info.driver.as_deref(), Some("/drv/block"));
    assert_eq!(device_info.block_size, 512);
    assert_eq!(device_info.capacity_bytes, 128 * 1024 * 1024);
    assert_eq!(
        runtime.device_info_by_path("/dev/storage0"),
        Err(RuntimeError::DeviceModel(DeviceModelError::InvalidDevice))
    );

    let driver_info = runtime.driver_info_by_path("/drv/block").unwrap();
    assert_eq!(
        driver_info.bound_devices,
        vec![String::from("/dev/storage-main")]
    );

    runtime.unlink_path("/drv/block").unwrap();
    assert_eq!(
        runtime.driver_info_by_path("/drv/block"),
        Err(RuntimeError::DeviceModel(DeviceModelError::InvalidDriver))
    );
    let device_info = runtime.device_info_by_path("/dev/storage-main").unwrap();
    assert_eq!(device_info.driver, None);
    assert_eq!(device_info.state, DeviceState::Registered);
}

#[test]
fn storage_device_driver_registry_completes_requests_with_geometry_intact() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("blk-stack", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_200), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_201), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "storage0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_202), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "storage-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/storage0", ObjectKind::Device, device)
        .unwrap();
    runtime
        .create_vfs_node("/drv/storage0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/storage0", "/drv/storage0")
        .unwrap();
    runtime
        .configure_device_geometry("/dev/storage0", 512, 128 * 1024 * 1024)
        .unwrap();

    let device_fd = runtime.open_path(owner, "/dev/storage0").unwrap();
    let driver_fd = runtime.open_path(owner, "/drv/storage0").unwrap();

    assert_eq!(
        runtime
            .write_io(owner, device_fd, b"read:lba=0 count=1")
            .unwrap(),
        18
    );
    let request = runtime.read_io(owner, driver_fd, 256).unwrap();
    let request_text = String::from_utf8_lossy(&request);
    assert!(request_text.contains("request:"));
    assert!(request_text.contains("kind=Write"));
    assert!(request_text.contains("device=/dev/storage0"));
    assert!(request_text.contains("read:lba=0 count=1"));

    assert_eq!(
        runtime
            .write_io(owner, driver_fd, b"sector0:eb58904d5357494e")
            .unwrap(),
        24
    );
    let completion = runtime.read_io(owner, device_fd, 64).unwrap();
    assert_eq!(completion, b"sector0:eb58904d5357494e");

    let driver_info = runtime.driver_info_by_path("/drv/storage0").unwrap();
    assert_eq!(driver_info.completed_requests, 1);
    assert_eq!(
        driver_info.bound_devices,
        vec![String::from("/dev/storage0")]
    );

    let device_info = runtime.device_info_by_path("/dev/storage0").unwrap();
    assert_eq!(device_info.class, DeviceClass::Storage);
    assert_eq!(device_info.block_size, 512);
    assert_eq!(device_info.capacity_bytes, 128 * 1024 * 1024);
    assert_eq!(device_info.completed_requests, 1);
}

#[test]
fn storage_device_completion_remains_pollable_after_reopening_device_path() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("blk-reopen", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_210), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_211), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "storage0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_212), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "storage-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/storage0", ObjectKind::Device, device)
        .unwrap();
    runtime
        .create_vfs_node("/drv/storage0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/storage0", "/drv/storage0")
        .unwrap();
    runtime
        .configure_device_geometry("/dev/storage0", 512, 128 * 1024 * 1024)
        .unwrap();

    let device_fd = runtime.open_path(owner, "/dev/storage0").unwrap();
    let driver_fd = runtime.open_path(owner, "/drv/storage0").unwrap();

    assert_eq!(
        runtime
            .write_io(owner, device_fd, b"read:lba=0 count=1")
            .unwrap(),
        18
    );
    assert!(
        runtime
            .poll_io(owner, driver_fd)
            .unwrap()
            .contains(IoPollEvents::READABLE)
    );
    let _request = runtime.read_io(owner, driver_fd, 256).unwrap();
    assert_eq!(
        runtime
            .write_io(owner, driver_fd, b"sector0:eb58904d5357494e")
            .unwrap(),
        24
    );

    runtime.close_descriptor(owner, device_fd).unwrap();

    let reopened_fd = runtime.open_path(owner, "/dev/storage0").unwrap();
    let reopened_events = runtime.poll_io(owner, reopened_fd).unwrap();
    assert!(
        reopened_events.contains(IoPollEvents::READABLE),
        "{reopened_events:?}"
    );
    let completion = runtime.read_io(owner, reopened_fd, 64).unwrap();
    assert_eq!(completion, b"sector0:eb58904d5357494e");
}

#[test]
fn graphics_device_queue_capacity_limits_pending_requests_and_recovers_after_completion() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-queue", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_300), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_301), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_302), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "gpu-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, device)
        .unwrap();
    runtime
        .create_vfs_node("/drv/gpu0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/gpu0", "/drv/gpu0")
        .unwrap();
    runtime.configure_device_queue("/dev/gpu0", 2).unwrap();

    let domain = runtime.create_domain(owner, None, "graphics").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    runtime
        .set_resource_governance_mode(resource, ResourceGovernanceMode::ExclusiveLease)
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Display)
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    runtime.claim_resource_via_contract(contract).unwrap();

    let device_fd = runtime.open_path(owner, "/dev/gpu0").unwrap();
    let driver_fd = runtime.open_path(owner, "/drv/gpu0").unwrap();

    assert_eq!(runtime.write_io(owner, device_fd, b"draw:a").unwrap(), 6);
    assert_eq!(
        runtime.control_io(owner, device_fd, 0x4750_0001).unwrap(),
        0x4750_0003
    );
    assert_eq!(
        runtime.write_io(owner, device_fd, b"draw:c"),
        Err(RuntimeError::DeviceModel(DeviceModelError::QueueFull))
    );

    let driver_info = runtime.driver_info_by_path("/drv/gpu0").unwrap();
    assert_eq!(driver_info.queued_requests, 2);
    assert_eq!(driver_info.in_flight_requests, 0);

    let request_one = String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
    assert!(request_one.contains("request:2"));
    assert!(request_one.contains("kind=Control"));
    assert_eq!(runtime.write_io(owner, driver_fd, b"present:b").unwrap(), 9);

    let request_two = String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
    assert!(request_two.contains("request:1"));
    assert!(request_two.contains("kind=Write"));
    assert!(request_two.contains("draw:a"));
    assert_eq!(runtime.write_io(owner, driver_fd, b"fence:a").unwrap(), 7);
    assert_eq!(runtime.write_io(owner, device_fd, b"draw:c").unwrap(), 6);

    let request_three = String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
    assert!(request_three.contains("request:3"));
    assert!(request_three.contains("kind=Write"));
    assert!(request_three.contains("draw:c"));
    assert_eq!(runtime.write_io(owner, driver_fd, b"fence:c").unwrap(), 7);

    assert_eq!(runtime.read_io(owner, device_fd, 32).unwrap(), b"present:b");
    assert_eq!(runtime.read_io(owner, device_fd, 32).unwrap(), b"fence:a");
    assert_eq!(runtime.read_io(owner, device_fd, 32).unwrap(), b"fence:c");

    let device_info = runtime.device_info_by_path("/dev/gpu0").unwrap();
    assert_eq!(device_info.queue_capacity, 2);
    assert_eq!(device_info.queue_depth, 0);
    assert_eq!(device_info.completed_requests, 3);
}

#[test]
fn graphics_device_accepts_semantic_surface_display_contracts() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-surface", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_311), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_312), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_313), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "gpu-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, device)
        .unwrap();
    runtime
        .create_vfs_node("/drv/gpu0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/gpu0", "/drv/gpu0")
        .unwrap();

    let domain = runtime.create_domain(owner, None, "graphics").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Surface, "orbit-runner-gfx")
        .unwrap();
    runtime
        .set_resource_governance_mode(resource, ResourceGovernanceMode::ExclusiveLease)
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Display)
        .unwrap();
    let contract = runtime
        .create_contract(
            owner,
            domain,
            resource,
            ContractKind::Display,
            "frame-pace-display",
        )
        .unwrap();
    runtime.claim_resource_via_contract(contract).unwrap();

    let device_fd = runtime.open_path(owner, "/dev/gpu0").unwrap();
    let driver_fd = runtime.open_path(owner, "/drv/gpu0").unwrap();

    assert_eq!(
        runtime.write_io(owner, device_fd, b"draw:surface").unwrap(),
        12
    );

    let request = String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
    assert!(request.contains("kind=Write"));
    assert!(request.contains("draw:surface"));
}

#[test]
fn audio_device_writes_complete_immediately_without_driver_queue_residue() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/audio0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/audio0", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/audio0", "/drv/audio0")
        .unwrap();

    let device_fd = runtime.open_path(owner, "/dev/audio0").unwrap();
    assert_eq!(runtime.write_io(owner, device_fd, b"tone:lead").unwrap(), 9);

    let device_info = runtime.device_info_by_path("/dev/audio0").unwrap();
    assert_eq!(device_info.class, DeviceClass::Audio);
    assert_eq!(device_info.queue_depth, 0);
    assert_eq!(device_info.submitted_requests, 1);
    assert_eq!(device_info.completed_requests, 1);

    let driver_info = runtime.driver_info_by_path("/drv/audio0").unwrap();
    assert_eq!(driver_info.queued_requests, 0);
    assert_eq!(driver_info.in_flight_requests, 0);
    assert_eq!(driver_info.completed_requests, 1);
}

#[test]
fn input_device_writes_complete_immediately_without_driver_queue_residue() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/input0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/input0", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/input0", "/drv/input0")
        .unwrap();

    let device_fd = runtime.open_path(owner, "/dev/input0").unwrap();
    assert_eq!(
        runtime.write_io(owner, device_fd, b"button:cross").unwrap(),
        12
    );

    let device_info = runtime.device_info_by_path("/dev/input0").unwrap();
    assert_eq!(device_info.class, DeviceClass::Input);
    assert_eq!(device_info.queue_depth, 0);
    assert_eq!(device_info.submitted_requests, 1);
    assert_eq!(device_info.completed_requests, 1);

    let driver_info = runtime.driver_info_by_path("/drv/input0").unwrap();
    assert_eq!(driver_info.queued_requests, 0);
    assert_eq!(driver_info.in_flight_requests, 0);
    assert_eq!(driver_info.completed_requests, 1);
}

#[derive(Default)]
struct RecordingGpuHardware {
    fail_submit: bool,
    calls: std::sync::Arc<std::sync::Mutex<Vec<(u32, Vec<u8>)>>>,
}

impl HardwareProvider for RecordingGpuHardware {
    fn submit_gpu_command(&mut self, rpc_id: u32, payload: &[u8]) -> Result<Vec<u8>, HalError> {
        self.calls.lock().unwrap().push((rpc_id, payload.to_vec()));
        if self.fail_submit {
            Err(HalError::InvalidDevice)
        } else {
            Ok(Vec::new())
        }
    }

    fn allocate_gpu_memory(
        &mut self,
        _kind: platform_hal::GpuMemoryKind,
        size: u64,
    ) -> Result<u64, HalError> {
        Ok(size)
    }

    fn set_primary_gpu_power_state(&mut self, _pstate: u32) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn gpu_binding_evidence(
        &mut self,
        _device: platform_hal::DeviceLocator,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_binding_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_vbios_window(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosWindowEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_vbios_image_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosImageEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_vbios_bytes(&mut self, _max_len: usize) -> Result<Vec<u8>, HalError> {
        Err(HalError::Unsupported)
    }

    fn primary_gpu_gsp_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuGspEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_interrupt_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuInterruptEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_display_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuDisplayEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_power_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuPowerEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_media_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuMediaEvidence>, HalError> {
        Ok(None)
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

    fn primary_gpu_neural_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuNeuralEvidence>, HalError> {
        Ok(None)
    }

    fn dispatch_primary_gpu_tensor_kernel(&mut self, _kernel_id: u32) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn primary_gpu_tensor_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuTensorEvidence>, HalError> {
        Ok(None)
    }
}

#[test]
fn graphics_buffer_submit_uses_installed_hardware_provider_when_available() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-hw", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_400), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_401), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, device)
        .unwrap();

    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    runtime.install_hardware_provider(Box::new(RecordingGpuHardware {
        fail_submit: false,
        calls: calls.clone(),
    }));

    let buffer_id = runtime.create_graphics_buffer(owner, 32).unwrap();
    assert_eq!(
        runtime
            .write_graphics_buffer(owner, buffer_id, 0, b"draw:hardware")
            .unwrap(),
        13
    );
    assert_eq!(
        runtime
            .submit_graphics_buffer(owner, "/dev/gpu0", buffer_id)
            .unwrap(),
        13
    );

    let recorded = calls.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].0, 0x100);
    assert_eq!(recorded[0].1, b"draw:hardware");

    let info = runtime.graphics_buffer_info(buffer_id).unwrap();
    assert_eq!(info.used_len, 13);
}

#[test]
fn graphics_buffer_submit_falls_back_to_driver_queue_when_hardware_provider_refuses() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-hw-fallback", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_410), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let device = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_411), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_412), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "gpu-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, device)
        .unwrap();
    runtime
        .create_vfs_node("/drv/gpu0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/gpu0", "/drv/gpu0")
        .unwrap();

    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    runtime.install_hardware_provider(Box::new(RecordingGpuHardware {
        fail_submit: true,
        calls: calls.clone(),
    }));

    let buffer_id = runtime.create_graphics_buffer(owner, 32).unwrap();
    runtime
        .write_graphics_buffer(owner, buffer_id, 0, b"draw:fallback")
        .unwrap();

    let request_id = runtime
        .submit_graphics_buffer(owner, "/dev/gpu0", buffer_id)
        .unwrap();
    assert!(request_id > 0);

    let driver_fd = runtime.open_path(owner, "/drv/gpu0").unwrap();
    let request = String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
    assert!(request.contains("kind=Write"));
    assert!(request.contains(&format!("buffer={buffer_id}")));
    assert!(request.contains("draw:fallback"));

    let recorded = calls.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].1, b"draw:fallback");
}

#[test]
fn graphics_buffer_submit_uses_platform_x86_64_gpu_provider_when_agent_is_initialized() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    assert_eq!(devices.len(), 1);
    let device = devices.remove(0);
    assert!(device.bars.len() >= 2);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-x86-hw", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_420), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_421), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_422), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "gpu-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime
        .create_vfs_node("/drv/gpu0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/gpu0", "/drv/gpu0")
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    let buffer_id = runtime.create_graphics_buffer(owner, 32).unwrap();
    runtime
        .write_graphics_buffer(owner, buffer_id, 0, b"draw:x86-hardware")
        .unwrap();
    let completed_len = runtime
        .submit_graphics_buffer(owner, "/dev/gpu0", buffer_id)
        .unwrap();
    assert_eq!(completed_len, 17);

    let scanout = runtime.graphics_scanout_info("/dev/gpu0").unwrap();
    assert_eq!(scanout.presented_frames, 1);
    assert_eq!(scanout.last_frame_len, 17);
    assert_eq!(
        runtime
            .read_graphics_scanout_frame("/dev/gpu0", 64)
            .unwrap(),
        b"draw:x86-hardware"
    );
}

#[test]
fn graphics_buffer_submit_falls_back_when_platform_x86_64_gpu_provider_is_not_initialized() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    assert_eq!(devices.len(), 1);
    let device = devices.remove(0);
    assert!(device.bars.len() >= 2);

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-x86-fallback", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_430), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_431), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(30_432), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "gpu-driver",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime
        .create_vfs_node("/drv/gpu0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/gpu0", "/drv/gpu0")
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    let buffer_id = runtime.create_graphics_buffer(owner, 32).unwrap();
    runtime
        .write_graphics_buffer(owner, buffer_id, 0, b"draw:x86-fallback")
        .unwrap();
    let request_id = runtime
        .submit_graphics_buffer(owner, "/dev/gpu0", buffer_id)
        .unwrap();
    assert!(request_id > 0);

    let driver_fd = runtime.open_path(owner, "/drv/gpu0").unwrap();
    let request = String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
    assert!(request.contains("kind=Write"));
    assert!(request.contains(&format!("buffer={buffer_id}")));
    assert!(request.contains("draw:x86-fallback"));
}

#[test]
fn networking_interface_moves_packets_between_socket_driver_and_rx_path() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-stack", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let nic = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "nic0",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_002), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "nic-driver",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_003), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "udp0",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net0", ObjectKind::Device, nic)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net0", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .create_vfs_node("/run/net0.sock", ObjectKind::Socket, socket)
        .unwrap();

    runtime
        .bind_device_to_driver("/dev/net0", "/drv/net0")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net0",
            [10, 1, 0, 2],
            [255, 255, 255, 0],
            [10, 1, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net0.sock",
            owner,
            "/dev/net0",
            4000,
            [10, 1, 0, 9],
            5000,
        )
        .unwrap();

    let sock_fd = runtime.open_path(owner, "/run/net0.sock").unwrap();
    let drv_fd = runtime.open_path(owner, "/drv/net0").unwrap();

    let sock_ready = runtime.poll_io(owner, sock_fd).unwrap();
    assert!(sock_ready.contains(IoPollEvents::WRITABLE));
    assert!(sock_ready.contains(IoPollEvents::PRIORITY));

    runtime.write_io(owner, sock_fd, b"frame:tx").unwrap();
    let tx = runtime.read_io(owner, drv_fd, 256).unwrap();
    let tx_text = String::from_utf8_lossy(&tx);
    assert!(tx_text.contains("net-tx iface=/dev/net0"));
    assert!(tx_text.contains("socket=/run/net0.sock"));
    assert!(tx_text.contains("sport=4000"));
    let header_end = tx.iter().position(|byte| *byte == b'\n').unwrap() + 1;
    let frame = &tx[header_end..];
    assert_eq!(&frame[12..14], &0x0800u16.to_be_bytes());
    assert_eq!(frame[23], 17);
    assert_eq!(u16::from_be_bytes([frame[34], frame[35]]), 4000);
    assert_eq!(u16::from_be_bytes([frame[36], frame[37]]), 5000);
    assert_eq!(&frame[42..], b"frame:tx");

    let injected = {
        let mut bytes = vec![0xff; 6];
        bytes.extend_from_slice(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        bytes.extend_from_slice(&0x0800u16.to_be_bytes());
        let payload = b"frame:rx";
        let total_len = 20 + 8 + payload.len();
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&[10, 1, 0, 9]);
        ip[16..20].copy_from_slice(&[10, 1, 0, 2]);
        bytes.extend_from_slice(&ip);
        bytes.extend_from_slice(&5000u16.to_be_bytes());
        bytes.extend_from_slice(&4000u16.to_be_bytes());
        bytes.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    };
    runtime.write_io(owner, drv_fd, &injected).unwrap();
    let rx_ready = runtime.poll_io(owner, sock_fd).unwrap();
    assert!(rx_ready.contains(IoPollEvents::READABLE));
    let rx = runtime.read_io(owner, sock_fd, 64).unwrap();
    assert_eq!(rx, b"frame:rx");

    let iface = runtime.network_interface_info("/dev/net0").unwrap();
    assert!(iface.link_up);
    assert_eq!(iface.driver_path, "/drv/net0");
    assert_eq!(iface.tx_packets, 1);
    assert_eq!(iface.rx_packets, 1);
    assert_eq!(iface.attached_sockets, vec![String::from("/run/net0.sock")]);
    assert_eq!(iface.ipv4_addr, [10, 1, 0, 2]);

    let socket_info = runtime.network_socket_info("/run/net0.sock").unwrap();
    assert_eq!(socket_info.local_port, 4000);
    assert_eq!(socket_info.remote_port, 5000);
    assert_eq!(socket_info.tx_packets, 1);
    assert_eq!(socket_info.rx_packets, 1);
    assert_eq!(socket_info.rx_depth, 0);
}

#[test]
fn networking_interface_rename_preserves_socket_attachment_and_interface_metadata() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-admin", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let nic = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "nic1",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_102), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "nic1-driver",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_103), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "udp1",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net1", ObjectKind::Device, nic)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net1", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .create_vfs_node("/run/net1.sock", ObjectKind::Socket, socket)
        .unwrap();

    runtime
        .bind_device_to_driver("/dev/net1", "/drv/net1")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net1",
            [10, 2, 0, 2],
            [255, 255, 255, 0],
            [10, 2, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net1.sock",
            owner,
            "/dev/net1",
            4100,
            [10, 2, 0, 9],
            5100,
        )
        .unwrap();

    runtime.rename_path("/dev/net1", "/dev/uplink0").unwrap();
    runtime.rename_path("/drv/net1", "/drv/uplink0").unwrap();
    runtime
        .rename_path("/run/net1.sock", "/run/uplink.sock")
        .unwrap();

    let iface = runtime.network_interface_info("/dev/uplink0").unwrap();
    assert_eq!(iface.driver_path, "/drv/uplink0");
    assert_eq!(
        iface.attached_sockets,
        vec![String::from("/run/uplink.sock")]
    );
    assert!(runtime.network_interface_info("/dev/net1").is_err());

    let socket_info = runtime.network_socket_info("/run/uplink.sock").unwrap();
    assert_eq!(socket_info.interface, "/dev/uplink0");
    assert_eq!(socket_info.local_port, 4100);
    assert!(runtime.network_socket_info("/run/net1.sock").is_err());
}

#[test]
fn procfs_system_network_views_render_interfaces_and_sockets() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-observe", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(13_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net0", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .create_vfs_node("/run/net0.sock", ObjectKind::Socket, root)
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/net0", "/drv/net0")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net0",
            [10, 1, 0, 2],
            [255, 255, 255, 0],
            [10, 1, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net0.sock",
            owner,
            "/dev/net0",
            4000,
            [10, 1, 0, 9],
            5000,
        )
        .unwrap();

    let interfaces = String::from_utf8(
        runtime
            .read_procfs_path("/proc/system/network/interfaces")
            .unwrap(),
    )
    .unwrap();
    assert!(interfaces.contains("/dev/net0"));
    assert!(interfaces.contains("driver=/drv/net0"));
    assert!(interfaces.contains("link=up"));
    assert!(interfaces.contains("addr=10.1.0.2"));
    assert!(interfaces.contains("sockets=1"));

    let sockets = String::from_utf8(
        runtime
            .read_procfs_path("/proc/system/network/sockets")
            .unwrap(),
    )
    .unwrap();
    assert!(sockets.contains("/run/net0.sock"));
    assert!(sockets.contains("iface=/dev/net0"));
    assert!(sockets.contains("local=10.1.0.2:4000"));
    assert!(sockets.contains("remote=10.1.0.9:5000"));
}

#[test]
fn networking_interface_demultiplexes_multiple_udp_sockets_on_one_interface() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-multi", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let nic = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "nic2",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_102), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "nic2-driver",
        )
        .unwrap();
    let socket_a = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_103), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "udp-a",
        )
        .unwrap();
    let socket_b = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_104), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "udp-b",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net2", ObjectKind::Device, nic)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net2", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .create_vfs_node("/run/net2-a.sock", ObjectKind::Socket, socket_a)
        .unwrap();
    runtime
        .create_vfs_node("/run/net2-b.sock", ObjectKind::Socket, socket_b)
        .unwrap();

    runtime
        .bind_device_to_driver("/dev/net2", "/drv/net2")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net2",
            [10, 2, 0, 2],
            [255, 255, 255, 0],
            [10, 2, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net2-a.sock",
            owner,
            "/dev/net2",
            4100,
            [10, 2, 0, 9],
            5100,
        )
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net2-b.sock",
            owner,
            "/dev/net2",
            4200,
            [10, 2, 0, 10],
            5200,
        )
        .unwrap();

    let drv_fd = runtime.open_path(owner, "/drv/net2").unwrap();
    let sock_a_fd = runtime.open_path(owner, "/run/net2-a.sock").unwrap();
    let sock_b_fd = runtime.open_path(owner, "/run/net2-b.sock").unwrap();

    let frame_a = {
        let mut bytes = vec![0xff; 6];
        bytes.extend_from_slice(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        bytes.extend_from_slice(&0x0800u16.to_be_bytes());
        let payload = b"rx-a";
        let total_len = 20 + 8 + payload.len();
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&[10, 2, 0, 9]);
        ip[16..20].copy_from_slice(&[10, 2, 0, 2]);
        bytes.extend_from_slice(&ip);
        bytes.extend_from_slice(&5100u16.to_be_bytes());
        bytes.extend_from_slice(&4100u16.to_be_bytes());
        bytes.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    };
    let frame_b = {
        let mut bytes = vec![0xff; 6];
        bytes.extend_from_slice(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        bytes.extend_from_slice(&0x0800u16.to_be_bytes());
        let payload = b"rx-b";
        let total_len = 20 + 8 + payload.len();
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&[10, 2, 0, 10]);
        ip[16..20].copy_from_slice(&[10, 2, 0, 2]);
        bytes.extend_from_slice(&ip);
        bytes.extend_from_slice(&5200u16.to_be_bytes());
        bytes.extend_from_slice(&4200u16.to_be_bytes());
        bytes.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    };

    runtime.write_io(owner, drv_fd, &frame_a).unwrap();
    runtime.write_io(owner, drv_fd, &frame_b).unwrap();

    assert_eq!(runtime.read_io(owner, sock_a_fd, 64).unwrap(), b"rx-a");
    assert_eq!(runtime.read_io(owner, sock_b_fd, 64).unwrap(), b"rx-b");

    let socket_a_info = runtime.network_socket_info("/run/net2-a.sock").unwrap();
    assert_eq!(socket_a_info.rx_packets, 1);
    assert_eq!(socket_a_info.rx_depth, 0);

    let socket_b_info = runtime.network_socket_info("/run/net2-b.sock").unwrap();
    assert_eq!(socket_b_info.rx_packets, 1);
    assert_eq!(socket_b_info.rx_depth, 0);
}

#[test]
fn networking_interface_enforces_mtu_on_tx_and_rx() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-mtu", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_200), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let nic = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_201), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "nic3",
        )
        .unwrap();
    let driver = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_202), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "nic3-driver",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_203), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "udp3",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/run", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net3", ObjectKind::Device, nic)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net3", ObjectKind::Driver, driver)
        .unwrap();
    runtime
        .create_vfs_node("/run/net3.sock", ObjectKind::Socket, socket)
        .unwrap();

    runtime
        .bind_device_to_driver("/dev/net3", "/drv/net3")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net3",
            [10, 3, 0, 2],
            [255, 255, 255, 0],
            [10, 3, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net3.sock",
            owner,
            "/dev/net3",
            4300,
            [10, 3, 0, 9],
            5300,
        )
        .unwrap();
    runtime.set_network_interface_mtu("/dev/net3", 96).unwrap();

    let sock_fd = runtime.open_path(owner, "/run/net3.sock").unwrap();
    let drv_fd = runtime.open_path(owner, "/drv/net3").unwrap();
    let oversized_payload = vec![0x41; 80];
    assert_eq!(
        runtime.write_io(owner, sock_fd, &oversized_payload),
        Err(RuntimeError::DeviceModel(DeviceModelError::PacketTooLarge))
    );

    let oversized_frame = {
        let mut bytes = vec![0xff; 6];
        bytes.extend_from_slice(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        bytes.extend_from_slice(&0x0800u16.to_be_bytes());
        let payload = vec![0x42; 80];
        let total_len = 20 + 8 + payload.len();
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&[10, 3, 0, 9]);
        ip[16..20].copy_from_slice(&[10, 3, 0, 2]);
        bytes.extend_from_slice(&ip);
        bytes.extend_from_slice(&5300u16.to_be_bytes());
        bytes.extend_from_slice(&4300u16.to_be_bytes());
        bytes.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&payload);
        bytes
    };
    assert_eq!(
        runtime.write_io(owner, drv_fd, &oversized_frame),
        Err(RuntimeError::DeviceModel(DeviceModelError::PacketTooLarge))
    );
}

#[test]
fn networking_tx_backpressure_requires_driver_completion_and_recycles_buffers() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-backpressure", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_300), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    for (path, kind) in [
        ("/", ObjectKind::Directory),
        ("/dev", ObjectKind::Directory),
        ("/drv", ObjectKind::Directory),
        ("/run", ObjectKind::Directory),
        ("/dev/net4", ObjectKind::Device),
        ("/drv/net4", ObjectKind::Driver),
        ("/run/net4.sock", ObjectKind::Socket),
    ] {
        runtime.create_vfs_node(path, kind, root).unwrap();
    }
    runtime
        .bind_device_to_driver("/dev/net4", "/drv/net4")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net4",
            [10, 4, 0, 2],
            [255, 255, 255, 0],
            [10, 4, 0, 1],
        )
        .unwrap();
    runtime
        .configure_network_interface_admin("/dev/net4", true, false, 1500, 2, 4, 1)
        .unwrap();
    runtime
        .bind_udp_socket(
            "/run/net4.sock",
            owner,
            "/dev/net4",
            4400,
            [10, 4, 0, 9],
            5400,
        )
        .unwrap();

    let sock_fd = runtime.open_path(owner, "/run/net4.sock").unwrap();
    let drv_fd = runtime.open_path(owner, "/drv/net4").unwrap();

    runtime.write_io(owner, sock_fd, b"pkt0").unwrap();
    let tx0_bytes = runtime.read_io(owner, drv_fd, 512).unwrap();
    let tx0 = String::from_utf8_lossy(&tx0_bytes);
    assert!(tx0.contains("buffer="));
    assert_eq!(
        runtime.write_io(owner, sock_fd, b"pkt1"),
        Err(RuntimeError::DeviceModel(DeviceModelError::QueueFull))
    );

    assert_eq!(runtime.complete_network_tx("/drv/net4", 1).unwrap(), 1);
    runtime.write_io(owner, sock_fd, b"pkt1").unwrap();
    let tx1_bytes = runtime.read_io(owner, drv_fd, 512).unwrap();
    let tx1 = String::from_utf8_lossy(&tx1_bytes);
    assert!(tx1.contains("queued=0 inflight=1"));
    assert_eq!(runtime.complete_network_tx("/drv/net4", 2).unwrap(), 1);

    let iface = runtime.network_interface_info("/dev/net4").unwrap();
    assert_eq!(iface.tx_packets, 2);
    assert_eq!(iface.tx_completions, 2);
    assert_eq!(iface.free_buffer_count, 1);
}

#[test]
fn networking_sendto_and_recvfrom_support_multi_peer_udp_flows() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-peers", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_400), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    for (path, kind) in [
        ("/", ObjectKind::Directory),
        ("/dev", ObjectKind::Directory),
        ("/drv", ObjectKind::Directory),
        ("/run", ObjectKind::Directory),
        ("/dev/net5", ObjectKind::Device),
        ("/drv/net5", ObjectKind::Driver),
        ("/run/net5.sock", ObjectKind::Socket),
    ] {
        runtime.create_vfs_node(path, kind, root).unwrap();
    }
    runtime
        .bind_device_to_driver("/dev/net5", "/drv/net5")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net5",
            [10, 5, 0, 2],
            [255, 255, 255, 0],
            [10, 5, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket("/run/net5.sock", owner, "/dev/net5", 4500, [0, 0, 0, 0], 0)
        .unwrap();

    runtime
        .send_udp_socket_to("/run/net5.sock", owner, [10, 5, 0, 9], 5501, b"peer-a")
        .unwrap();
    runtime
        .send_udp_socket_to("/run/net5.sock", owner, [10, 5, 0, 10], 5502, b"peer-b")
        .unwrap();

    let drv_fd = runtime.open_path(owner, "/drv/net5").unwrap();
    let tx_a_bytes = runtime.read_io(owner, drv_fd, 512).unwrap();
    let tx_a = String::from_utf8_lossy(&tx_a_bytes);
    let tx_b_bytes = runtime.read_io(owner, drv_fd, 512).unwrap();
    let tx_b = String::from_utf8_lossy(&tx_b_bytes);
    assert!(tx_a.contains("dport=5501"));
    assert!(tx_b.contains("dport=5502"));
    assert_eq!(runtime.complete_network_tx("/drv/net5", 2).unwrap(), 2);

    let inject = |src_ip: [u8; 4], src_port: u16, payload: &[u8]| {
        let mut bytes = vec![0xff; 6];
        bytes.extend_from_slice(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        bytes.extend_from_slice(&0x0800u16.to_be_bytes());
        let total_len = 20 + 8 + payload.len();
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&src_ip);
        ip[16..20].copy_from_slice(&[10, 5, 0, 2]);
        bytes.extend_from_slice(&ip);
        bytes.extend_from_slice(&src_port.to_be_bytes());
        bytes.extend_from_slice(&4500u16.to_be_bytes());
        bytes.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    };
    runtime
        .write_io(owner, drv_fd, &inject([10, 5, 0, 9], 5501, b"reply-a"))
        .unwrap();
    runtime
        .write_io(owner, drv_fd, &inject([10, 5, 0, 10], 5502, b"reply-b"))
        .unwrap();

    let (a_payload, a_ip, a_port) = runtime
        .recv_udp_socket_from("/run/net5.sock", owner, 64)
        .unwrap();
    let (b_payload, b_ip, b_port) = runtime
        .recv_udp_socket_from("/run/net5.sock", owner, 64)
        .unwrap();
    assert_eq!(a_payload, b"reply-a");
    assert_eq!(a_ip, [10, 5, 0, 9]);
    assert_eq!(a_port, 5501);
    assert_eq!(b_payload, b"reply-b");
    assert_eq!(b_ip, [10, 5, 0, 10]);
    assert_eq!(b_port, 5502);
}

#[test]
fn networking_interface_admin_controls_promiscuous_mode_and_queue_limits() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("net-admin", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_500), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    for (path, kind) in [
        ("/", ObjectKind::Directory),
        ("/dev", ObjectKind::Directory),
        ("/drv", ObjectKind::Directory),
        ("/run", ObjectKind::Directory),
        ("/dev/net6", ObjectKind::Device),
        ("/drv/net6", ObjectKind::Driver),
        ("/run/net6.sock", ObjectKind::Socket),
    ] {
        runtime.create_vfs_node(path, kind, root).unwrap();
    }
    runtime
        .bind_device_to_driver("/dev/net6", "/drv/net6")
        .unwrap();
    runtime
        .configure_network_interface_ipv4(
            "/dev/net6",
            [10, 6, 0, 2],
            [255, 255, 255, 0],
            [10, 6, 0, 1],
        )
        .unwrap();
    runtime
        .bind_udp_socket("/run/net6.sock", owner, "/dev/net6", 4600, [0, 0, 0, 0], 0)
        .unwrap();
    runtime
        .configure_network_interface_admin("/dev/net6", false, false, 1500, 4, 1, 2)
        .unwrap();

    assert_eq!(
        runtime.send_udp_socket_to("/run/net6.sock", owner, [10, 6, 0, 9], 5600, b"blocked"),
        Err(RuntimeError::DeviceModel(DeviceModelError::NotBound))
    );

    runtime
        .configure_network_interface_admin("/dev/net6", true, true, 1500, 4, 1, 2)
        .unwrap();
    let drv_fd = runtime.open_path(owner, "/drv/net6").unwrap();
    let foreign_dst = {
        let mut bytes = vec![0xff; 6];
        bytes.extend_from_slice(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        bytes.extend_from_slice(&0x0800u16.to_be_bytes());
        let payload = b"promisc-hit";
        let total_len = 20 + 8 + payload.len();
        let mut ip = [0u8; 20];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        ip[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 17;
        ip[12..16].copy_from_slice(&[10, 6, 0, 9]);
        ip[16..20].copy_from_slice(&[10, 6, 0, 99]);
        bytes.extend_from_slice(&ip);
        bytes.extend_from_slice(&5600u16.to_be_bytes());
        bytes.extend_from_slice(&4600u16.to_be_bytes());
        bytes.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    };
    runtime.write_io(owner, drv_fd, &foreign_dst).unwrap();
    let (payload, _, _) = runtime
        .recv_udp_socket_from("/run/net6.sock", owner, 64)
        .unwrap();
    assert_eq!(payload, b"promisc-hit");

    runtime.write_io(owner, drv_fd, &foreign_dst).unwrap();
    assert_eq!(
        runtime.write_io(owner, drv_fd, &foreign_dst),
        Err(RuntimeError::DeviceModel(DeviceModelError::QueueFull))
    );
    let socket_info = runtime.network_socket_info("/run/net6.sock").unwrap();
    assert_eq!(socket_info.rx_depth, 1);
    assert_eq!(socket_info.rx_queue_limit, 1);
}

#[test]
fn graphics_vbios_image_evidence_reports_parsed_rom_identity_when_platform_provider_exposes_rom() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    backend.define_config_dword(address, 0x30, 0x00c0_0001);
    let mut rom = vec![0; 0x400];
    rom[0..8].copy_from_slice(&[0x55, 0xaa, 0x4e, 0x56, 0x49, 0x44, 0x49, 0x41]);
    rom[0x40..0x44].copy_from_slice(b"NVFW");
    rom[0x120..0x124].copy_from_slice(b"PCIR");
    rom[0x124..0x126].copy_from_slice(&0x10deu16.to_le_bytes());
    rom[0x126..0x128].copy_from_slice(&0x2d04u16.to_le_bytes());
    rom[0x1c0..0x1da].copy_from_slice(b"NVIDIA GeForce RTX 5060 Ti");
    rom[0x220..0x22e].copy_from_slice(b"P14N:506T301FB");
    rom[0x280..0x296].copy_from_slice(b"Version 98.06.1F.00.DC");
    rom[0x320..0x323].copy_from_slice(b"BIT");
    backend.define_rom(0x00c0_0000, &rom);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    let device = devices.remove(0);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-vbios-evidence", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_600), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_601), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    let evidence = runtime
        .graphics_vbios_image_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert_eq!(evidence.vendor_id, 0x10de);
    assert_eq!(evidence.device_id, 0x2d04);
    assert_eq!(evidence.board_name, "NVIDIA GeForce RTX 5060 Ti");
    assert_eq!(evidence.board_code, "P14N:506T301FB");
    assert_eq!(evidence.version, "Version 98.06.1F.00.DC");
    assert_eq!(evidence.nvfw_offset, Some(0x40));
    assert_eq!(evidence.bit_offset, Some(0x320));
}

#[test]
fn graphics_vbios_image_evidence_refuses_when_platform_provider_has_no_rom_backing() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    backend.define_config_dword(address, 0x30, 0x00c0_0001);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    let device = devices.remove(0);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-vbios-no-rom", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_610), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_611), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    assert!(runtime.graphics_vbios_image_evidence("/dev/gpu0").is_err());
}

#[test]
fn graphics_power_evidence_reports_local_rtx_5060_ti_clock_evidence() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    let device = devices.remove(0);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-power-evidence", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_620), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_621), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    let evidence = runtime
        .graphics_power_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert!(evidence.present);
    assert_eq!(evidence.pstate, 8);
    assert_eq!(evidence.graphics_clock_mhz, 1200);
    assert_eq!(evidence.memory_clock_mhz, 900);
    assert_eq!(evidence.boost_clock_mhz, 1500);
    assert!(!evidence.hardware_power_management_confirmed);
}

#[test]
fn graphics_set_power_state_updates_primary_gpu_power_evidence() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    let device = devices.remove(0);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-power-set", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_622), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_623), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    runtime.graphics_set_power_state("/dev/gpu0", 0).unwrap();
    let after = runtime
        .graphics_power_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert_eq!(after.pstate, 0);
    assert_eq!(after.graphics_clock_mhz, 2407);
    assert_eq!(after.memory_clock_mhz, 1750);
    assert_eq!(after.boost_clock_mhz, 2602);

    runtime.graphics_set_power_state("/dev/gpu0", 12).unwrap();
    let idle = runtime
        .graphics_power_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert_eq!(idle.pstate, 12);
    assert_eq!(idle.graphics_clock_mhz, 300);
    assert_eq!(idle.memory_clock_mhz, 405);
    assert_eq!(idle.boost_clock_mhz, 600);
}

#[test]
fn graphics_start_media_session_updates_primary_gpu_media_evidence() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    let device = devices.remove(0);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-media", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_624), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_625), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    let before = runtime
        .graphics_media_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert_eq!(before.sessions, 0);
    assert_eq!(before.codec, 0);
    assert_eq!(before.width, 0);

    runtime
        .graphics_start_media_session("/dev/gpu0", 1920, 1080, 12_000, 2)
        .unwrap();
    let after = runtime
        .graphics_media_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert_eq!(after.sessions, 1);
    assert_eq!(after.codec, 2);
    assert_eq!(after.width, 1920);
    assert_eq!(after.height, 1080);
    assert_eq!(after.bitrate_kbps, 12_000);
    assert!(!after.hardware_media_confirmed);
}

#[test]
fn graphics_neural_evidence_updates_after_inject_and_commit() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    let device = devices.remove(0);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-neural", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_626), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_627), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    let before = runtime
        .graphics_neural_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert!(!before.model_loaded);
    assert_eq!(before.active_semantics, 0);
    assert!(!before.last_commit_completed);

    runtime
        .graphics_inject_neural_semantic("/dev/gpu0", "enemy-vehicle")
        .unwrap();
    let after_inject = runtime
        .graphics_neural_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert!(after_inject.model_loaded);
    assert_eq!(after_inject.active_semantics, 1);
    assert!(!after_inject.last_commit_completed);

    runtime.graphics_commit_neural_frame("/dev/gpu0").unwrap();
    let after_commit = runtime
        .graphics_neural_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert!(after_commit.last_commit_completed);
    assert!(!after_commit.hardware_neural_confirmed);
}

#[test]
fn graphics_tensor_evidence_updates_after_dispatch() {
    let mut backend = SyntheticPciConfigBackend::new();
    let address = PciAddress {
        segment: 0,
        bus: 0,
        device: 5,
        function: 0,
    };
    backend.define_device(
        address,
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
    backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
    backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
    backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
    let mut platform = X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
    let mut devices = platform.enumerate_devices().unwrap();
    let device = devices.remove(0);
    platform.setup_gpu_agent(device.locator).unwrap();

    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("gpu-tensor", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_628), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let gpu = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(31_629), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, gpu)
        .unwrap();
    runtime.install_hardware_provider(Box::new(platform));

    let before = runtime
        .graphics_tensor_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert_eq!(before.active_jobs, 0);
    assert_eq!(before.last_kernel_id, 0);

    runtime
        .graphics_dispatch_tensor_kernel("/dev/gpu0", 77)
        .unwrap();
    let after = runtime
        .graphics_tensor_evidence("/dev/gpu0")
        .unwrap()
        .unwrap();
    assert_eq!(after.active_jobs, 1);
    assert_eq!(after.last_kernel_id, 77);
    assert!(!after.hardware_tensor_confirmed);
}
