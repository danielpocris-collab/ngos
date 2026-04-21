use super::*;
#[test]
fn runtime_orchestrates_processes_capabilities_and_scheduler() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.default_thread_cpu_extended_state = ThreadCpuExtendedStateProfile {
        owned: true,
        xsave_managed: true,
        save_area_bytes: 4096,
        xcr0_mask: 0xe7,
        boot_probed: true,
        boot_seed_marker: 0x55aa_33cc,
        active_in_cpu: false,
        save_count: 0,
        restore_count: 0,
        last_saved_tick: 0,
        last_restored_tick: 0,
        save_area_buffer_bytes: 0,
        save_area_alignment_bytes: 0,
        save_area_generation: 0,
        last_save_marker: 0,
    };
    let mut runtime = KernelRuntime::new(policy);
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
    let shell_thread = runtime.thread_infos(shell).unwrap();
    assert_eq!(shell_thread.len(), 1);
    assert!(shell_thread[0].cpu_extended_state.active_in_cpu);
    assert_eq!(shell_thread[0].cpu_extended_state.restore_count, 1);
    assert_eq!(shell_thread[0].cpu_extended_state.last_restored_tick, 1);
    assert_eq!(
        shell_thread[0].cpu_extended_state.save_area_buffer_bytes,
        4096
    );
    assert_eq!(
        shell_thread[0].cpu_extended_state.save_area_alignment_bytes,
        64
    );
    assert_eq!(shell_thread[0].cpu_extended_state.save_area_generation, 1);
    assert_eq!(
        shell_thread[0].cpu_extended_state.last_save_marker,
        0x55aa_33cc
    );
    let active = runtime.active_cpu_extended_state().unwrap();
    assert_eq!(active.owner_pid, shell);
    assert_eq!(active.owner_tid, first.tid);
    assert_eq!(active.image.bytes.len(), 4096);

    let init_thread = runtime.thread_infos(init).unwrap();
    assert_eq!(init_thread.len(), 1);
    assert!(!init_thread[0].cpu_extended_state.active_in_cpu);
    assert_eq!(init_thread[0].cpu_extended_state.save_count, 0);
    assert_eq!(
        init_thread[0].cpu_extended_state.save_area_buffer_bytes,
        4096
    );
    assert_eq!(
        init_thread[0].cpu_extended_state.save_area_alignment_bytes,
        64
    );
    assert_eq!(
        init_thread[0].cpu_extended_state.last_save_marker,
        0x55aa_33cc
    );
}

#[test]
fn runtime_exposes_process_info_and_process_list() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.default_thread_cpu_extended_state = ThreadCpuExtendedStateProfile {
        owned: true,
        xsave_managed: true,
        save_area_bytes: 4096,
        xcr0_mask: 0xe7,
        boot_probed: true,
        boot_seed_marker: 0xfeed_cafe,
        active_in_cpu: false,
        save_count: 0,
        restore_count: 0,
        last_saved_tick: 0,
        last_restored_tick: 0,
        save_area_buffer_bytes: 0,
        save_area_alignment_bytes: 0,
        save_area_generation: 0,
        last_save_marker: 0,
    };
    let mut runtime = KernelRuntime::new(policy);
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

    let threads = runtime.thread_infos(app).unwrap();
    assert_eq!(threads.len(), 1);
    assert!(threads[0].cpu_extended_state.owned);
    assert!(threads[0].cpu_extended_state.xsave_managed);
    assert_eq!(threads[0].cpu_extended_state.save_area_bytes, 4096);
    assert_eq!(threads[0].cpu_extended_state.xcr0_mask, 0xe7);
    assert!(threads[0].cpu_extended_state.boot_probed);
    assert_eq!(threads[0].cpu_extended_state.save_area_buffer_bytes, 4096);
    assert_eq!(threads[0].cpu_extended_state.save_area_alignment_bytes, 64);
    assert_eq!(threads[0].cpu_extended_state.save_area_generation, 1);
    assert_eq!(threads[0].cpu_extended_state.last_save_marker, 0xfeed_cafe);
}

#[test]
fn runtime_exports_aligned_cpu_extended_state_image() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.default_thread_cpu_extended_state = ThreadCpuExtendedStateProfile {
        owned: true,
        xsave_managed: true,
        save_area_bytes: 2048,
        xcr0_mask: 0xe7,
        boot_probed: true,
        boot_seed_marker: 0xdead_beef,
        active_in_cpu: false,
        save_count: 0,
        restore_count: 0,
        last_saved_tick: 0,
        last_restored_tick: 0,
        save_area_buffer_bytes: 0,
        save_area_alignment_bytes: 0,
        save_area_generation: 0,
        last_save_marker: 0,
    };
    let mut runtime = KernelRuntime::new(policy);
    let init = runtime
        .spawn_process("init", None, SchedulerClass::BestEffort)
        .unwrap();
    let scheduled = runtime.tick().unwrap();
    assert_eq!(scheduled.pid, init);

    let active = runtime.active_cpu_extended_state().unwrap();
    assert!(active.image.bytes.is_aligned());
    assert_eq!(active.image.profile.save_area_alignment_bytes, 64);

    let exported = runtime
        .export_thread_cpu_extended_state_image(init, scheduled.tid)
        .unwrap();
    assert!(exported.bytes.is_aligned());
    assert_eq!(exported.profile.save_area_alignment_bytes, 64);
}

#[test]
fn runtime_restores_thread_cpu_extended_state_from_boot_seed() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.default_thread_cpu_extended_state = ThreadCpuExtendedStateProfile {
        owned: true,
        xsave_managed: true,
        save_area_bytes: 4096,
        xcr0_mask: 0xe7,
        boot_probed: true,
        boot_seed_marker: 0xabcd_1234,
        active_in_cpu: false,
        save_count: 0,
        restore_count: 0,
        last_saved_tick: 0,
        last_restored_tick: 0,
        save_area_buffer_bytes: 0,
        save_area_alignment_bytes: 0,
        save_area_generation: 0,
        last_save_marker: 0,
    };
    let mut runtime = KernelRuntime::new(policy);
    let init = runtime
        .spawn_process("init", None, SchedulerClass::BestEffort)
        .unwrap();
    let shell = runtime
        .spawn_process("shell", Some(init), SchedulerClass::Interactive)
        .unwrap();

    let shell_tid = runtime
        .processes()
        .get(shell)
        .unwrap()
        .main_thread()
        .unwrap();
    assert_eq!(
        runtime.thread_infos(shell).unwrap()[0]
            .cpu_extended_state
            .last_save_marker,
        0xabcd_1234
    );

    assert_eq!(runtime.tick().unwrap().tid, shell_tid);
    runtime
        .processes
        .mark_thread_cpu_extended_state_saved(shell_tid, 9)
        .unwrap();

    let changed = runtime.thread_infos(shell).unwrap()[0].cpu_extended_state;
    assert_ne!(changed.last_save_marker, 0xabcd_1234);
    assert!(changed.save_count >= 1);

    runtime
        .restore_thread_cpu_extended_state_boot_seed(shell, shell_tid)
        .unwrap();
    let restored = runtime.thread_infos(shell).unwrap()[0].cpu_extended_state;
    assert_eq!(restored.last_save_marker, 0xabcd_1234);
    assert_eq!(restored.save_area_generation, 1);
    assert!(!restored.active_in_cpu);

    let missing = runtime.restore_thread_cpu_extended_state_boot_seed(init, shell_tid);
    assert_eq!(
        missing,
        Err(RuntimeError::Process(ProcessError::InvalidTid))
    );
}

#[test]
fn runtime_policy_handoff_builds_default_thread_profile() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 8192,
        xcr0_mask: 0x1ff,
        boot_probed: true,
        boot_seed_marker: 0xdead_beef,
    });
    let mut runtime = KernelRuntime::new(policy);
    let pid = runtime
        .spawn_process("handoff", None, SchedulerClass::Interactive)
        .unwrap();
    let thread = runtime.thread_infos(pid).unwrap();
    assert_eq!(thread.len(), 1);
    assert!(thread[0].cpu_extended_state.xsave_managed);
    assert_eq!(thread[0].cpu_extended_state.save_area_bytes, 8192);
    assert_eq!(thread[0].cpu_extended_state.xcr0_mask, 0x1ff);
    assert!(thread[0].cpu_extended_state.boot_probed);
    assert_eq!(thread[0].cpu_extended_state.boot_seed_marker, 0xdead_beef);
}

#[test]
fn runtime_can_apply_cpu_handoff_for_future_threads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let before = runtime
        .spawn_process("before", None, SchedulerClass::Interactive)
        .unwrap();
    let before_thread = runtime.thread_infos(before).unwrap();
    assert!(!before_thread[0].cpu_extended_state.xsave_managed);

    let applied = runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 2048,
        xcr0_mask: 0x27,
        boot_probed: true,
        boot_seed_marker: 0xcafe_babe,
    });
    assert!(applied.xsave_managed);
    assert_eq!(applied.save_area_bytes, 2048);
    assert_eq!(
        runtime.default_thread_cpu_extended_state().boot_seed_marker,
        0xcafe_babe
    );

    let after = runtime
        .spawn_process("after", None, SchedulerClass::Interactive)
        .unwrap();
    let after_thread = runtime.thread_infos(after).unwrap();
    assert!(after_thread[0].cpu_extended_state.xsave_managed);
    assert_eq!(after_thread[0].cpu_extended_state.save_area_bytes, 2048);
    assert_eq!(after_thread[0].cpu_extended_state.xcr0_mask, 0x27);
    assert!(after_thread[0].cpu_extended_state.boot_probed);
    assert_eq!(
        after_thread[0].cpu_extended_state.boot_seed_marker,
        0xcafe_babe
    );
    assert_eq!(after_thread[0].cpu_extended_state.save_area_generation, 1);
    assert_eq!(
        after_thread[0].cpu_extended_state.last_save_marker,
        0xcafe_babe
    );

    let before_again = runtime.thread_infos(before).unwrap();
    assert!(!before_again[0].cpu_extended_state.xsave_managed);
}

#[test]
fn runtime_can_apply_cpu_handoff_to_existing_process_threads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let pid = runtime
        .spawn_process("existing", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime.processes().get(pid).unwrap().main_thread().unwrap();

    let initial = runtime.thread_infos(pid).unwrap();
    assert!(!initial[0].cpu_extended_state.xsave_managed);

    let applied = runtime
        .apply_cpu_handoff_to_process_threads(
            pid,
            CpuExtendedStateHandoff {
                xsave_managed: true,
                save_area_bytes: 3072,
                xcr0_mask: 0x67,
                boot_probed: true,
                boot_seed_marker: 0x1357_2468,
            },
        )
        .unwrap();
    assert_eq!(applied, 1);

    let updated = runtime.thread_infos(pid).unwrap();
    assert_eq!(updated[0].tid, tid);
    assert!(updated[0].cpu_extended_state.xsave_managed);
    assert_eq!(updated[0].cpu_extended_state.save_area_bytes, 3072);
    assert_eq!(updated[0].cpu_extended_state.xcr0_mask, 0x67);
    assert!(updated[0].cpu_extended_state.boot_probed);
    assert_eq!(updated[0].cpu_extended_state.boot_seed_marker, 0x1357_2468);
    assert_eq!(updated[0].cpu_extended_state.save_area_buffer_bytes, 3072);
    assert_eq!(updated[0].cpu_extended_state.save_area_generation, 1);
    assert_eq!(updated[0].cpu_extended_state.last_save_marker, 0x1357_2468);

    let missing = runtime.apply_cpu_handoff_to_process_threads(
        ProcessId::from_handle(ObjectHandle::new(Handle::new(99_999), 0)),
        CpuExtendedStateHandoff {
            xsave_managed: true,
            save_area_bytes: 1024,
            xcr0_mask: 0x7,
            boot_probed: false,
            boot_seed_marker: 1,
        },
    );
    assert_eq!(
        missing,
        Err(RuntimeError::Process(ProcessError::InvalidPid))
    );
}

#[test]
fn runtime_can_restore_existing_process_threads_to_default_cpu_handoff() {
    let mut runtime = KernelRuntime::host_runtime_default();
    runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 1536,
        xcr0_mask: 0x17,
        boot_probed: true,
        boot_seed_marker: 0x1111_2222,
    });

    let pid = runtime
        .spawn_process("restore-target", None, SchedulerClass::Interactive)
        .unwrap();

    runtime
        .apply_cpu_handoff_to_process_threads(
            pid,
            CpuExtendedStateHandoff {
                xsave_managed: true,
                save_area_bytes: 3072,
                xcr0_mask: 0x67,
                boot_probed: true,
                boot_seed_marker: 0x3333_4444,
            },
        )
        .unwrap();
    let changed = runtime.thread_infos(pid).unwrap();
    assert_eq!(changed[0].cpu_extended_state.save_area_bytes, 3072);
    assert_eq!(changed[0].cpu_extended_state.boot_seed_marker, 0x3333_4444);
    assert_eq!(changed[0].cpu_extended_state.last_save_marker, 0x3333_4444);

    let restored = runtime
        .restore_process_threads_to_default_cpu_handoff(pid)
        .unwrap();
    assert_eq!(restored, 1);

    let thread = runtime.thread_infos(pid).unwrap();
    assert!(thread[0].cpu_extended_state.xsave_managed);
    assert_eq!(thread[0].cpu_extended_state.save_area_bytes, 1536);
    assert_eq!(thread[0].cpu_extended_state.xcr0_mask, 0x17);
    assert!(thread[0].cpu_extended_state.boot_probed);
    assert_eq!(thread[0].cpu_extended_state.boot_seed_marker, 0x1111_2222);
    assert_eq!(thread[0].cpu_extended_state.save_area_buffer_bytes, 1536);
    assert_eq!(thread[0].cpu_extended_state.save_area_generation, 1);
    assert_eq!(thread[0].cpu_extended_state.last_save_marker, 0x1111_2222);

    let missing = runtime.restore_process_threads_to_default_cpu_handoff(ProcessId::from_handle(
        ObjectHandle::new(Handle::new(88_888), 0),
    ));
    assert_eq!(
        missing,
        Err(RuntimeError::Process(ProcessError::InvalidPid))
    );
}

#[test]
fn runtime_can_export_and_import_thread_cpu_extended_state_image() {
    let mut runtime = KernelRuntime::host_runtime_default();
    runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 1024,
        xcr0_mask: 0x27,
        boot_probed: true,
        boot_seed_marker: 0xaaaa_5555,
    });

    let source = runtime
        .spawn_process("source", None, SchedulerClass::Interactive)
        .unwrap();
    let target = runtime
        .spawn_process("target", None, SchedulerClass::Interactive)
        .unwrap();

    let source_tid = runtime
        .processes()
        .get(source)
        .unwrap()
        .main_thread()
        .unwrap();
    let target_tid = runtime
        .processes()
        .get(target)
        .unwrap()
        .main_thread()
        .unwrap();

    runtime
        .processes
        .mark_thread_cpu_extended_state_saved(source_tid, 42)
        .unwrap();
    let exported = runtime
        .export_thread_cpu_extended_state_image(source, source_tid)
        .unwrap();
    assert_eq!(exported.profile.save_area_bytes, 1024);
    assert_eq!(
        exported.profile.last_save_marker,
        42 ^ 0x27 ^ source_tid.raw()
    );
    assert_eq!(exported.bytes.len(), 1024);

    runtime
        .import_thread_cpu_extended_state_image(target, target_tid, exported.clone())
        .unwrap();
    let imported = runtime.thread_infos(target).unwrap();
    assert_eq!(imported[0].cpu_extended_state.save_area_bytes, 1024);
    assert_eq!(
        imported[0].cpu_extended_state.last_save_marker,
        exported.profile.last_save_marker
    );
    assert_eq!(
        imported[0].cpu_extended_state.save_area_generation,
        exported.profile.save_area_generation
    );

    let invalid_owner = runtime.export_thread_cpu_extended_state_image(target, source_tid);
    assert_eq!(
        invalid_owner,
        Err(RuntimeError::Process(ProcessError::InvalidTid))
    );

    let invalid_import = runtime.import_thread_cpu_extended_state_image(
        source,
        source_tid,
        ThreadCpuExtendedStateImage {
            profile: ThreadCpuExtendedStateProfile::bootstrap_default(),
            bytes: AlignedCpuExtendedStateBuffer::new(),
        },
    );
    assert_eq!(
        invalid_import,
        Err(RuntimeError::Process(
            ProcessError::CpuExtendedStateUnavailable
        ))
    );
}

#[test]
fn runtime_can_clone_thread_cpu_extended_state_between_threads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 2048,
        xcr0_mask: 0x67,
        boot_probed: true,
        boot_seed_marker: 0x2468_1357,
    });

    let source = runtime
        .spawn_process("source-clone", None, SchedulerClass::Interactive)
        .unwrap();
    let target = runtime
        .spawn_process("target-clone", None, SchedulerClass::Interactive)
        .unwrap();

    let source_tid = runtime
        .processes()
        .get(source)
        .unwrap()
        .main_thread()
        .unwrap();
    let target_tid = runtime
        .processes()
        .get(target)
        .unwrap()
        .main_thread()
        .unwrap();

    runtime
        .processes
        .mark_thread_cpu_extended_state_saved(source_tid, 77)
        .unwrap();

    let cloned = runtime
        .clone_thread_cpu_extended_state_image(source, source_tid, target, target_tid)
        .unwrap();
    assert_eq!(cloned.profile.save_area_bytes, 2048);
    assert_eq!(
        cloned.profile.last_save_marker,
        77 ^ 0x67 ^ source_tid.raw()
    );

    let target_thread = runtime.thread_infos(target).unwrap();
    assert_eq!(
        target_thread[0].cpu_extended_state.last_save_marker,
        cloned.profile.last_save_marker
    );
    assert_eq!(
        target_thread[0].cpu_extended_state.save_area_generation,
        cloned.profile.save_area_generation
    );

    let wrong_owner =
        runtime.clone_thread_cpu_extended_state_image(target, source_tid, target, target_tid);
    assert_eq!(
        wrong_owner,
        Err(RuntimeError::Process(ProcessError::InvalidTid))
    );

    let plain = runtime
        .spawn_process("plain", None, SchedulerClass::Interactive)
        .unwrap();
    let plain_tid = runtime
        .processes()
        .get(plain)
        .unwrap()
        .main_thread()
        .unwrap();
    runtime
        .apply_cpu_handoff_to_process_threads(
            plain,
            CpuExtendedStateHandoff {
                xsave_managed: false,
                save_area_bytes: 0,
                xcr0_mask: 0,
                boot_probed: false,
                boot_seed_marker: 0,
            },
        )
        .unwrap();
    let unavailable =
        runtime.clone_thread_cpu_extended_state_image(plain, plain_tid, target, target_tid);
    assert_eq!(
        unavailable,
        Err(RuntimeError::Process(
            ProcessError::CpuExtendedStateUnavailable
        ))
    );
}

#[test]
fn runtime_can_release_thread_cpu_extended_state_image() {
    let mut runtime = KernelRuntime::host_runtime_default();
    runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 1024,
        xcr0_mask: 0x27,
        boot_probed: true,
        boot_seed_marker: 0xbeef_cafe,
    });

    let pid = runtime
        .spawn_process("release", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime.processes().get(pid).unwrap().main_thread().unwrap();

    let before = runtime.thread_infos(pid).unwrap();
    assert!(before[0].cpu_extended_state.xsave_managed);
    assert_eq!(before[0].cpu_extended_state.save_area_buffer_bytes, 1024);

    runtime
        .release_thread_cpu_extended_state_image(pid, tid)
        .unwrap();

    let after = runtime.thread_infos(pid).unwrap();
    assert!(!after[0].cpu_extended_state.xsave_managed);
    assert_eq!(after[0].cpu_extended_state.save_area_bytes, 0);
    assert_eq!(after[0].cpu_extended_state.xcr0_mask, 0);
    assert_eq!(after[0].cpu_extended_state.boot_seed_marker, 0);
    assert_eq!(after[0].cpu_extended_state.save_area_buffer_bytes, 0);
    assert_eq!(after[0].cpu_extended_state.save_area_generation, 0);
    assert_eq!(after[0].cpu_extended_state.last_save_marker, 0);

    let already_released = runtime.release_thread_cpu_extended_state_image(pid, tid);
    assert_eq!(
        already_released,
        Err(RuntimeError::Process(
            ProcessError::CpuExtendedStateUnavailable
        ))
    );

    let wrong_owner = runtime.release_thread_cpu_extended_state_image(
        ProcessId::from_handle(ObjectHandle::new(Handle::new(77_777), 0)),
        tid,
    );
    assert_eq!(
        wrong_owner,
        Err(RuntimeError::Process(ProcessError::InvalidTid))
    );
}

#[test]
fn runtime_procfs_cpu_renders_extended_state_lifecycle() {
    let mut runtime = KernelRuntime::host_runtime_default();
    runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 2048,
        xcr0_mask: 0x67,
        boot_probed: true,
        boot_seed_marker: 0x1234_abcd,
    });

    let pid = runtime
        .spawn_process("cpu-procfs", None, SchedulerClass::Interactive)
        .unwrap();
    let tid = runtime.processes().get(pid).unwrap().main_thread().unwrap();
    runtime
        .processes
        .mark_thread_cpu_extended_state_saved(tid, 33)
        .unwrap();

    let cpu = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/cpu", pid.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(cpu.contains("xsave-managed=true"));
    assert!(cpu.contains("save-area=2048"));
    assert!(cpu.contains("xcr0=0x67"));
    assert!(cpu.contains("boot-probed=true"));
    assert!(cpu.contains("boot-seed=0x1234abcd"));
    assert!(cpu.contains("generation=2"));

    runtime
        .release_thread_cpu_extended_state_image(pid, tid)
        .unwrap();
    let released = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/cpu", pid.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(released.contains("xsave-managed=false"));
    assert!(released.contains("save-area=0"));
    assert!(released.contains("marker=0x0"));
}

#[test]
fn runtime_tracks_active_cpu_extended_state_slot_across_thread_switches() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.default_thread_cpu_extended_state = ThreadCpuExtendedStateProfile {
        owned: true,
        xsave_managed: true,
        save_area_bytes: 2048,
        xcr0_mask: 0x67,
        boot_probed: true,
        boot_seed_marker: 0x9999_aaaa,
        active_in_cpu: false,
        save_count: 0,
        restore_count: 0,
        last_saved_tick: 0,
        last_restored_tick: 0,
        save_area_buffer_bytes: 0,
        save_area_alignment_bytes: 0,
        save_area_generation: 0,
        last_save_marker: 0,
    };
    let mut runtime = KernelRuntime::new(policy);
    let init = runtime
        .spawn_process("init", None, SchedulerClass::BestEffort)
        .unwrap();
    let shell = runtime
        .spawn_process("shell", Some(init), SchedulerClass::Interactive)
        .unwrap();

    let shell_tid = runtime
        .processes()
        .get(shell)
        .unwrap()
        .main_thread()
        .unwrap();
    let init_tid = runtime
        .processes()
        .get(init)
        .unwrap()
        .main_thread()
        .unwrap();

    let first = runtime.tick().unwrap();
    assert_eq!(first.tid, shell_tid);
    let active_first = runtime.active_cpu_extended_state().unwrap();
    assert_eq!(active_first.owner_pid, shell);
    assert_eq!(active_first.owner_tid, shell_tid);

    assert_eq!(runtime.block_running().unwrap(), shell);
    let second = runtime.tick().unwrap();
    assert_eq!(second.tid, init_tid);
    let active_second = runtime.active_cpu_extended_state().unwrap();
    assert_eq!(active_second.owner_pid, init);
    assert_eq!(active_second.owner_tid, init_tid);

    let shell_thread = runtime.thread_infos(shell).unwrap();
    assert!(shell_thread[0].cpu_extended_state.save_count >= 1);
    assert!(!shell_thread[0].cpu_extended_state.active_in_cpu);

    let system_cpu =
        String::from_utf8(runtime.read_procfs_path("/proc/system/cpu").unwrap()).unwrap();
    assert!(system_cpu.contains(&format!(
        "active-slot:\tpid={} tid={}",
        init.raw(),
        init_tid.raw()
    )));
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

    let cpu = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/cpu", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(cpu.contains(&format!("pid:\t{}", app.raw())));
    assert!(cpu.contains("xsave-managed=false"));
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
fn runtime_exec_process_rejects_directories_then_executes_image_and_refreshes_procfs_views() {
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
        .create_vfs_node("/bin/tool", ObjectKind::File, bin)
        .unwrap();

    let denied = runtime.exec_process(app, "/bin", vec![String::from("bin")], vec![]);
    assert!(matches!(
        denied,
        Err(RuntimeError::Vfs(VfsError::NotExecutable))
    ));

    let closed = runtime
        .exec_process(
            app,
            "/bin/tool",
            vec![String::from("tool"), String::from("--inspect")],
            vec![String::from("LANG=C")],
        )
        .unwrap();
    assert_eq!(closed.len(), 0);

    let info = runtime.process_info(app).unwrap();
    assert_eq!(info.name, "tool");
    assert_eq!(info.image_path, "/bin/tool");
    assert_eq!(info.environment_count, 1);

    let status = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/status", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(status.contains("Name:\ttool"));
    assert!(status.contains("Image:\t/bin/tool"));
    assert!(status.contains(&format!("Pid:\t{}", app.raw())));

    let cmdline = runtime
        .read_procfs_path(&format!("/proc/{}/cmdline", app.raw()))
        .unwrap();
    assert_eq!(cmdline, b"tool\0--inspect\0");

    let cwd = runtime
        .read_procfs_path(&format!("/proc/{}/cwd", app.raw()))
        .unwrap();
    assert_eq!(cwd, b"/");

    let environ = runtime
        .read_procfs_path(&format!("/proc/{}/environ", app.raw()))
        .unwrap();
    assert_eq!(environ, b"LANG=C\0");

    let exe = runtime
        .read_procfs_path(&format!("/proc/{}/exe", app.raw()))
        .unwrap();
    assert_eq!(exe, b"/bin/tool");
}

#[test]
fn runtime_exec_process_rejects_missing_path_then_executes_after_creation_and_refreshes_procfs_views()
 {
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
            ObjectHandle::new(Handle::new(7_320), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(7_321), 0),
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

    let missing = runtime.exec_process(app, "/bin/late", vec![String::from("late")], vec![]);
    assert!(matches!(
        missing,
        Err(RuntimeError::Vfs(VfsError::NotFound))
    ));

    runtime
        .create_vfs_node("/bin/late", ObjectKind::File, bin)
        .unwrap();

    let closed = runtime
        .exec_process(
            app,
            "/bin/late",
            vec![String::from("late"), String::from("--boot")],
            vec![String::from("LANG=C"), String::from("MODE=recovery")],
        )
        .unwrap();
    assert_eq!(closed.len(), 0);

    let info = runtime.process_info(app).unwrap();
    assert_eq!(info.name, "late");
    assert_eq!(info.image_path, "/bin/late");
    assert_eq!(info.environment_count, 2);

    let status = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/status", app.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(status.contains("Name:\tlate"));
    assert!(status.contains("Image:\t/bin/late"));
    assert!(status.contains(&format!("Pid:\t{}", app.raw())));

    let cmdline = runtime
        .read_procfs_path(&format!("/proc/{}/cmdline", app.raw()))
        .unwrap();
    assert_eq!(cmdline, b"late\0--boot\0");

    let environ = runtime
        .read_procfs_path(&format!("/proc/{}/environ", app.raw()))
        .unwrap();
    assert_eq!(environ, b"LANG=C\0MODE=recovery\0");

    let exe = runtime
        .read_procfs_path(&format!("/proc/{}/exe", app.raw()))
        .unwrap();
    assert_eq!(exe, b"/bin/late");
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

#[test]
fn runtime_verified_core_reports_hard_kernel_invariants() {
    let mut runtime = KernelRuntime::host_runtime_default();
    runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 4096,
        xcr0_mask: 0xe7,
        boot_probed: true,
        boot_seed_marker: 0xfeed_cafe,
    });
    let init = runtime
        .spawn_process("init", None, SchedulerClass::Interactive)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::BestEffort)
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(8_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin", ObjectKind::Directory, root)
        .unwrap();
    let _ = runtime.tick().unwrap();

    let report = runtime.verify_core();
    assert!(report.is_verified(), "{report:#?}");
    assert!(report.capability_model_verified);
    assert!(report.vfs_invariants_verified);
    assert!(report.scheduler_state_machine_verified);
    assert!(report.cpu_extended_state_lifecycle_verified);
    assert!(report.bus_integrity_verified);
    assert!(report.violations.is_empty());
}

#[test]
fn runtime_verified_core_reports_corrupted_kernel_invariants() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("broken-init", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(8_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin", ObjectKind::Directory, root)
        .unwrap();

    let _dangling_cap = runtime
        .capabilities
        .grant(
            &runtime.processes,
            init,
            ObjectHandle::new(Handle::new(8_101), 0),
            CapabilityRights::READ,
            "tmp",
        )
        .unwrap();
    runtime.capabilities.revoke(root).unwrap();

    let init_tid = runtime.processes.get(init).unwrap().main_thread().unwrap();
    runtime
        .processes
        .set_state(init, ProcessState::Blocked)
        .unwrap();
    runtime
        .processes
        .threads
        .get_mut(init_tid.handle())
        .unwrap()
        .set_cpu_extended_state(ThreadCpuExtendedStateProfile {
            owned: true,
            xsave_managed: true,
            save_area_bytes: 128,
            xcr0_mask: 0,
            boot_probed: true,
            boot_seed_marker: 0,
            active_in_cpu: true,
            save_count: 0,
            restore_count: 0,
            last_saved_tick: 0,
            last_restored_tick: 0,
            save_area_buffer_bytes: 0,
            save_area_alignment_bytes: 0,
            save_area_generation: 0,
            last_save_marker: 0,
        });
    runtime.processes.objects.remove(init.handle()).unwrap();

    let report = runtime.verify_core();
    assert!(!report.is_verified());
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.family == VerifiedCoreFamily::CapabilityModel)
    );
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.family == VerifiedCoreFamily::VfsInvariants)
    );
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.family == VerifiedCoreFamily::SchedulerStateMachine)
    );
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.family == VerifiedCoreFamily::CpuExtendedStateLifecycle)
    );
}

#[test]
fn runtime_verified_core_reports_bus_integrity_corruption() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("bus-init", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(8_300), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render", ObjectKind::Channel, root)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    let peer = runtime.create_bus_peer(owner, domain, "renderer").unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/render")
        .unwrap();
    runtime.attach_bus_peer(peer, endpoint).unwrap();
    runtime
        .bus_endpoints
        .get_mut(endpoint)
        .unwrap()
        .attached_peers
        .clear();

    let report = runtime.verify_core();
    assert!(!report.bus_integrity_verified);
    assert!(report.violations.iter().any(|entry| {
        entry.family == VerifiedCoreFamily::BusIntegrity
            && entry.code == "bus-peer-attachment-not-reciprocated"
    }));
}

#[test]
fn runtime_verified_core_reports_scheduler_policy_state_corruption() {
    let runtime = &mut KernelRuntime::host_runtime_default();
    runtime
        .scheduler
        .inject_wait_ticks_for_test(SchedulerClass::Background, 3);
    runtime
        .scheduler
        .inject_lag_debt_for_test(SchedulerClass::Interactive, 2);

    let report = runtime.verify_core();
    assert!(!report.scheduler_state_machine_verified);
    assert!(report.violations.iter().any(|entry| {
        entry.family == VerifiedCoreFamily::SchedulerStateMachine
            && entry.code == "scheduler-empty-class-has-wait-ticks"
    }));
    assert!(report.violations.iter().any(|entry| {
        entry.family == VerifiedCoreFamily::SchedulerStateMachine
            && entry.code == "scheduler-empty-class-has-lag-debt"
    }));
}

#[test]
fn runtime_verified_core_reports_scheduler_service_accounting_corruption() {
    let runtime = &mut KernelRuntime::host_runtime_default();
    runtime
        .scheduler
        .inject_wait_ticks_for_test(SchedulerClass::Background, 1);
    runtime
        .scheduler
        .inject_lag_debt_for_test(SchedulerClass::Background, 0);
    runtime.scheduler.class_runtime_ticks_mut_for_test()[SchedulerClass::Interactive.index()] = 9;

    let report = runtime.verify_core();
    assert!(!report.scheduler_state_machine_verified);
    assert!(report.violations.iter().any(|entry| {
        entry.family == VerifiedCoreFamily::SchedulerStateMachine
            && entry.code == "scheduler-runtime-without-dispatch"
    }));
    assert!(report.violations.iter().any(|entry| {
        entry.family == VerifiedCoreFamily::SchedulerStateMachine
            && entry.code == "scheduler-runtime-exceeds-dispatch-budget"
    }));
}

#[test]
fn runtime_verified_core_reports_scheduler_cpu_affinity_corruption() {
    let runtime = &mut KernelRuntime::host_runtime_default();
    let pid = runtime
        .spawn_process("cpu-corrupt", None, SchedulerClass::BestEffort)
        .unwrap();
    let tid = runtime.processes.get(pid).unwrap().main_thread().unwrap();
    runtime
        .scheduler
        .inject_thread_assignment_for_test(tid, 3, 0b01);

    let report = runtime.verify_core();
    assert!(!report.scheduler_state_machine_verified);
    assert!(report.violations.iter().any(|entry| {
        entry.family == VerifiedCoreFamily::SchedulerStateMachine
            && entry.code == "scheduler-thread-assigned-cpu-invalid"
    }));
}

#[test]
fn runtime_set_process_affinity_rejects_empty_mask_and_recovers_with_valid_mask() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.scheduler_logical_cpu_count = 2;
    let runtime = &mut KernelRuntime::new(policy);
    let pid = runtime
        .spawn_process("cpu-affinity", None, SchedulerClass::BestEffort)
        .unwrap();
    let tid = runtime.processes.get(pid).unwrap().main_thread().unwrap();

    assert!(matches!(
        runtime.set_process_affinity(pid, 0),
        Err(RuntimeError::Scheduler(SchedulerError::InvalidCpuAffinity))
    ));

    runtime.set_process_affinity(pid, 0b10).unwrap();
    let (assigned_cpu, affinity_mask) = runtime.scheduler.thread_assignment(tid).unwrap();
    assert_eq!(assigned_cpu, 1);
    assert_eq!(affinity_mask, 0b10);

    runtime.set_process_affinity(pid, 0b11).unwrap();
    let (assigned_cpu, affinity_mask) = runtime.scheduler.thread_assignment(tid).unwrap();
    assert_eq!(assigned_cpu, 1);
    assert_eq!(affinity_mask, 0b11);
}

#[test]
fn runtime_set_process_affinity_prefers_nearest_topology_cpu_when_load_is_equal() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.apply_scheduler_cpu_topology(vec![
        SchedulerCpuTopologyEntry {
            apic_id: 10,
            package_id: 0,
            core_group: 0,
            sibling_group: 0,
            inferred: false,
        },
        SchedulerCpuTopologyEntry {
            apic_id: 11,
            package_id: 0,
            core_group: 0,
            sibling_group: 1,
            inferred: false,
        },
        SchedulerCpuTopologyEntry {
            apic_id: 12,
            package_id: 0,
            core_group: 1,
            sibling_group: 0,
            inferred: false,
        },
        SchedulerCpuTopologyEntry {
            apic_id: 13,
            package_id: 1,
            core_group: 0,
            sibling_group: 0,
            inferred: false,
        },
    ]);
    let runtime = &mut KernelRuntime::new(policy);
    let pid = runtime
        .spawn_process("cpu-topology-affinity", None, SchedulerClass::BestEffort)
        .unwrap();
    let tid = runtime.processes.get(pid).unwrap().main_thread().unwrap();

    runtime.set_process_affinity(pid, 0b0001).unwrap();
    let (assigned_cpu, affinity_mask) = runtime.scheduler.thread_assignment(tid).unwrap();
    assert_eq!(assigned_cpu, 0);
    assert_eq!(affinity_mask, 0b0001);

    runtime.set_process_affinity(pid, 0b1110).unwrap();
    let (assigned_cpu, affinity_mask) = runtime.scheduler.thread_assignment(tid).unwrap();
    assert_eq!(assigned_cpu, 1);
    assert_eq!(affinity_mask, 0b1110);
}
