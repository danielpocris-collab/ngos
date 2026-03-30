use super::*;
#[test]
fn runtime_orchestrates_processes_capabilities_and_scheduler() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::BestEffort)
        .unwrap();
    let shell = runtime
        .spawn_process("shell", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(1000), 7),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "rootfs",
        )
        .unwrap();
    let child_cap = runtime
        .duplicate_capability(cap, shell, CapabilityRights::READ, "rootfs-ro")
        .unwrap();

    let first = runtime.tick().unwrap();
    assert_eq!(first.pid, shell);
    assert_eq!(
        first.tid,
        runtime
            .processes()
            .get(shell)
            .unwrap()
            .main_thread()
            .unwrap()
    );
    assert_eq!(runtime.snapshot().running, Some(shell));
    assert_eq!(runtime.snapshot().running_thread, Some(first.tid));
    assert_eq!(
        runtime.capabilities().get(child_cap).unwrap().rights(),
        CapabilityRights::READ
    );
}

#[test]
fn runtime_exposes_process_info_and_process_list() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "asset",
        )
        .unwrap();
    let _fd = runtime
        .open_descriptor(app, cap, ObjectKind::File, "/tmp/app.log")
        .unwrap();

    let info = runtime.process_info(app).unwrap();
    assert_eq!(info.pid, app);
    assert_eq!(info.parent, Some(init));
    assert_eq!(info.name, "app");
    assert_eq!(info.image_path, "app");
    assert_eq!(info.executable_image.path, "app");
    assert_eq!(info.cwd, "/");
    assert_eq!(info.thread_count, 1);
    assert!(info.main_thread.is_some());
    assert_eq!(info.descriptor_count, 1);
    assert_eq!(info.capability_count, 1);
    assert_eq!(info.environment_count, 0);
    assert_eq!(info.auxiliary_vector_count, 6);
    assert_eq!(info.memory_region_count, 5);
    assert_eq!(info.vm_object_count, 5);

    let processes = runtime.process_list();
    assert!(processes.iter().any(|process| process.pid == init));
    assert!(processes.iter().any(|process| process.pid == app));
}

#[test]
fn runtime_renders_procfs_views_for_processes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_process_args(
            app,
            vec![
                String::from("app"),
                String::from("--scene"),
                String::from("mars"),
            ],
        )
        .unwrap();
    runtime
        .set_process_env(
            app,
            vec![String::from("TERM=xterm"), String::from("HOME=/home/app")],
        )
        .unwrap();
    let cap = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "asset",
        )
        .unwrap();
    let _fd = runtime
        .open_descriptor(app, cap, ObjectKind::File, "/tmp/app.log")
        .unwrap();

    let status = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/status", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(status.contains("Name:\tapp"));
    assert!(status.contains("Image:\tapp"));
    assert!(status.contains("Entry:\t0x0"));
    assert!(status.contains("Auxv:\t6"));
    assert!(status.contains("Maps:\t5"));
    assert!(status.contains("VmObjects:\t5"));
    assert!(status.contains("Threads:\t1"));
    assert!(status.contains("Cwd:\t/"));
    assert!(status.contains("Envs:\t2"));
    assert!(status.contains(&format!("Pid:\t{}", app.raw())));

    let stat = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/stat", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(stat.contains(&format!("{} (app)", app.raw())));

    let fds = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/fd", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(fds.contains("/tmp/app.log"));

    let fdinfo = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/fdinfo/0", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(fdinfo.contains("path:\t/tmp/app.log"));

    let cmdline = runtime
        .read_procfs_path(&format!("/proc/{}/cmdline", app.raw()))
        .unwrap();
    assert_eq!(cmdline, b"app\0--scene\0mars\0");
    let cwd = runtime
        .read_procfs_path(&format!("/proc/{}/cwd", app.raw()))
        .unwrap();
    assert_eq!(cwd, b"/");
    let environ = runtime
        .read_procfs_path(&format!("/proc/{}/environ", app.raw()))
        .unwrap();
    assert_eq!(environ, b"TERM=xterm\0HOME=/home/app\0");
    let exe = runtime
        .read_procfs_path(&format!("/proc/{}/exe", app.raw()))
        .unwrap();
    assert_eq!(exe, b"app");
    let auxv = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/auxv", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(auxv.contains("3\t0x"));
    assert!(auxv.contains("9\t0x"));
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("r-xp 00000000 normal app"));
    assert!(maps.contains("rw-p 00000000 normal [stack]"));
    let vmobjects = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("private=true"));
    assert!(vmobjects.contains("owners=1"));
    assert!(vmobjects.contains("app"));

    let caps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/caps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(caps.contains("asset"));
}

#[test]
fn runtime_can_block_wake_exit_and_reap_running_processes() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let worker = runtime
        .spawn_process("worker", None, SchedulerClass::BestEffort)
        .unwrap();

    assert_eq!(runtime.tick().unwrap().pid, worker);
    assert_eq!(runtime.block_running().unwrap(), worker);
    assert_eq!(
        runtime.processes().get(worker).unwrap().state(),
        ProcessState::Blocked
    );

    runtime
        .wake_process(worker, SchedulerClass::Interactive)
        .unwrap();
    assert_eq!(runtime.tick().unwrap().pid, worker);
    let system = runtime.inspect_system();
    assert!(system.scheduler_agent_decisions.iter().any(|entry| {
        entry.agent == SchedulerAgentKind::BlockAgent && entry.pid == worker.raw()
    }));
    assert!(system.scheduler_agent_decisions.iter().any(|entry| {
        entry.agent == SchedulerAgentKind::WakeAgent && entry.pid == worker.raw()
    }));
    assert_eq!(runtime.exit_running(23).unwrap(), worker);

    let reaped = runtime.reap_process(worker).unwrap();
    assert_eq!(reaped.exit_code(), Some(23));
    assert!(!runtime.processes().contains(worker));
}

#[test]
fn runtime_manages_descriptor_namespaces_per_process() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let worker = runtime
        .spawn_process("worker", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            worker,
            ObjectHandle::new(Handle::new(7_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "worker-root",
        )
        .unwrap();

    let fd0 = runtime
        .open_descriptor(worker, cap, ObjectKind::Directory, "/srv")
        .unwrap();
    let fd1 = runtime.duplicate_descriptor(worker, fd0).unwrap();
    runtime.set_descriptor_cloexec(worker, fd0, true).unwrap();

    assert_eq!(runtime.descriptors_for(worker).unwrap(), vec![fd0, fd1]);
    assert_eq!(runtime.exec_transition(worker).unwrap().len(), 1);
    assert_eq!(runtime.descriptors_for(worker).unwrap(), vec![fd1]);

    runtime.exit(worker, 0).unwrap();
    let _ = runtime.reap_process(worker).unwrap();
    assert_eq!(
        runtime.descriptors_for(worker),
        Err(RuntimeError::Process(ProcessError::InvalidPid))
    );
}

#[test]
fn exec_transition_purges_event_queue_state_for_cloexec_descriptors() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("queue-owner", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(7_150), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(7_151), 0),
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
        .create_vfs_node("/run/exec.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = runtime.open_path(owner, "/run/exec.sock").unwrap();
    runtime.set_descriptor_cloexec(owner, fd, true).unwrap();

    let queue = runtime
        .create_event_queue(owner, EventQueueMode::Epoll)
        .unwrap();
    runtime
        .watch_event(
            owner,
            queue,
            fd,
            44,
            ReadinessInterest {
                readable: true,
                writable: true,
                priority: false,
            },
            EventWatchBehavior::LEVEL,
        )
        .unwrap();
    let _ = runtime.wait_event_queue(owner, queue).unwrap();

    let system_before = runtime.inspect_system();
    let queue_before = system_before
        .event_queues
        .iter()
        .find(|entry| entry.id == queue)
        .unwrap();
    assert_eq!(queue_before.watch_count, 1);

    let closed = runtime.exec_transition(owner).unwrap();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].fd(), fd);

    let system_after = runtime.inspect_system();
    let queue_after = system_after
        .event_queues
        .iter()
        .find(|entry| entry.id == queue)
        .unwrap();
    assert_eq!(queue_after.watch_count, 0);
    assert_eq!(queue_after.pending_count, 0);
    assert!(runtime.wait_event_queue(owner, queue).unwrap().is_empty());
    assert!(runtime.descriptors_for(owner).unwrap().is_empty());
}

#[test]
fn exec_transition_clears_fdshare_peer_deferred_watch_refresh_tasks() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let peer = runtime
        .spawn_process_share_fds("peer", Some(owner), SchedulerClass::Interactive, owner)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(7_160), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let socket = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(7_161), 0),
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
        .create_vfs_node("/run/shared-exec.sock", ObjectKind::Socket, socket)
        .unwrap();
    let fd = runtime.open_path(owner, "/run/shared-exec.sock").unwrap();
    runtime.set_descriptor_cloexec(owner, fd, true).unwrap();

    let peer_queue = runtime
        .create_event_queue(peer, EventQueueMode::Kqueue)
        .unwrap();
    runtime
        .watch_event(
            peer,
            peer_queue,
            fd,
            55,
            ReadinessInterest {
                readable: true,
                writable: true,
                priority: false,
            },
            EventWatchBehavior::LEVEL,
        )
        .unwrap();
    assert_eq!(runtime.snapshot().deferred_task_count, 1);

    let closed = runtime.exec_transition(owner).unwrap();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].fd(), fd);

    let system = runtime.inspect_system();
    let peer_queue_info = system
        .event_queues
        .iter()
        .find(|entry| entry.id == peer_queue)
        .unwrap();
    assert_eq!(peer_queue_info.watch_count, 0);
    assert_eq!(peer_queue_info.pending_count, 0);
    assert_eq!(system.snapshot.deferred_task_count, 0);
    assert!(runtime.descriptors_for(owner).unwrap().is_empty());
    assert!(runtime.descriptors_for(peer).unwrap().is_empty());
}

#[test]
fn runtime_exec_process_updates_image_cmdline_and_cloexec_state() {
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
            ObjectHandle::new(Handle::new(7_200), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_201), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
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
        .create_vfs_node("/home", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/home/game", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin/game", ObjectKind::File, bin)
        .unwrap();
    runtime.set_process_cwd(app, "/home/game").unwrap();
    let fd0 = runtime.open_path(app, "/bin/game").unwrap();
    runtime.duplicate_descriptor(app, fd0).unwrap();
    runtime.set_descriptor_cloexec(app, fd0, true).unwrap();

    let closed = runtime
        .exec_process(
            app,
            "/bin/game",
            vec![
                String::from("game"),
                String::from("--map"),
                String::from("mars"),
            ],
            vec![String::from("LANG=C"), String::from("SAVE_DIR=/home/game")],
        )
        .unwrap();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].fd(), fd0);
    let descriptors = runtime.descriptors_for(app).unwrap();
    assert_eq!(
        descriptors,
        vec![Descriptor::new(0), Descriptor::new(1), Descriptor::new(2)]
    );

    let info = runtime.process_info(app).unwrap();
    assert_eq!(info.name, "game");
    assert_eq!(info.image_path, "/bin/game");
    assert_eq!(info.executable_image.path, "/bin/game");
    assert!(info.executable_image.entry_point > info.executable_image.base_addr);
    assert_eq!(info.cwd, "/home/game");
    assert_eq!(info.descriptor_count, 3);
    assert_eq!(info.environment_count, 2);
    assert_eq!(info.auxiliary_vector_count, 6);
    assert_eq!(info.memory_region_count, 5);
    let cmdline = runtime
        .read_procfs_path(&format!("/proc/{}/cmdline", app.raw()))
        .unwrap();
    assert_eq!(cmdline, b"game\0--map\0mars\0");
    let cwd = runtime
        .read_procfs_path(&format!("/proc/{}/cwd", app.raw()))
        .unwrap();
    assert_eq!(cwd, b"/home/game");
    let environ = runtime
        .read_procfs_path(&format!("/proc/{}/environ", app.raw()))
        .unwrap();
    assert_eq!(environ, b"LANG=C\0SAVE_DIR=/home/game\0");
    let exe = runtime
        .read_procfs_path(&format!("/proc/{}/exe", app.raw()))
        .unwrap();
    assert_eq!(exe, b"/bin/game");
    let auxv = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/auxv", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(auxv.contains("5\t0x3"));
    let maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("r-xp 00000000 normal /bin/game"));
    assert!(maps.contains("rw-p 00000000 normal [heap]"));
}

#[test]
fn runtime_prepares_user_launch_from_exec_image() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_300), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_301), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
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
        .create_vfs_node("/bin/userland-native", ObjectKind::File, bin)
        .unwrap();

    runtime
        .exec_process(
            app,
            "/bin/userland-native",
            vec![String::from("userland-native"), String::from("--mode=test")],
            vec![String::from("USERLAND=1")],
        )
        .unwrap();

    let launch = runtime.prepare_user_launch(app).unwrap();
    assert_eq!(
        runtime.descriptors_for(app).unwrap(),
        vec![Descriptor::new(0), Descriptor::new(1), Descriptor::new(2)]
    );
    assert_eq!(launch.pid, app);
    assert_eq!(launch.bootstrap.argv[0], "userland-native");
    assert_eq!(launch.bootstrap.envp[0], "USERLAND=1");
    assert_eq!(launch.stack_image.argc, 2);
    assert_eq!(launch.stack_image.start_frame.argc, 2);
    assert_eq!(
        launch.stack_image.start_frame.stack_alignment,
        STACK_ALIGNMENT
    );
    assert_eq!(
        launch.registers.rip,
        launch.executable_image.entry_point as usize
    );
    assert_eq!(launch.registers.rsp, launch.stack_image.stack_top);
    assert_eq!(launch.registers.cs, AMD64_USER_CODE_SELECTOR);
    assert_eq!(launch.registers.ss, AMD64_USER_STACK_SELECTOR);
    assert_eq!(launch.registers.rdi, 2);
    assert_eq!(
        launch.registers.rsi,
        launch.stack_image.start_frame.argv as usize
    );
    assert_eq!(
        launch.registers.rdx,
        launch.stack_image.start_frame.envp as usize
    );
    assert_eq!(
        launch.stack_mapping.vaddr,
        launch.stack_image.stack_base as u64
    );
    assert!(launch.stack_mapping.user);
    assert!(launch.stack_mapping.perms.read);
    assert!(launch.stack_mapping.perms.write);
    assert_eq!(
        launch.stack_range.vaddr,
        launch.stack_image.stack_base as u64
    );
}

#[test]
fn runtime_exec_process_installs_standard_streams_and_accepts_seeded_input() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("shell", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_350), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_351), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
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
        .create_vfs_node("/bin/sh", ObjectKind::File, bin)
        .unwrap();

    runtime
        .exec_process(app, "/bin/sh", vec![String::from("sh")], vec![])
        .unwrap();
    runtime.seed_standard_input(app, b"help\nexit 0\n").unwrap();

    assert_eq!(
        runtime.descriptors_for(app).unwrap(),
        vec![Descriptor::new(0), Descriptor::new(1), Descriptor::new(2)]
    );
    assert_eq!(
        runtime.read_io(app, Descriptor::new(0), 5).unwrap(),
        b"help\n".to_vec()
    );
    assert_eq!(
        runtime.read_io(app, Descriptor::new(0), 16).unwrap(),
        b"exit 0\n".to_vec()
    );
    assert!(runtime.inspect_io(app, Descriptor::new(1)).is_ok());
    assert!(runtime.inspect_io(app, Descriptor::new(2)).is_ok());
}
