use super::*;
use ngos_user_abi::{
    NativeEventRecord, NativeEventSourceKind, NativeNetworkEventKind,
    NativeNetworkEventWatchConfig, NativeNetworkLinkStateConfig, POLLPRI, SYS_CREATE_EVENT_QUEUE,
    SYS_MAP_ANONYMOUS_MEMORY, SYS_QUARANTINE_VM_OBJECT, SYS_READ_PROCFS,
    SYS_RECLAIM_MEMORY_PRESSURE, SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL, SYS_RELEASE_VM_OBJECT,
    SYS_SET_NETIF_LINK_STATE, SYS_SET_PROCESS_ARGS, SYS_SET_PROCESS_CWD, SYS_SET_PROCESS_ENV,
    SYS_STORE_MEMORY_WORD, SYS_WAIT_EVENT_QUEUE, SYS_WATCH_NET_EVENTS,
};

fn checksum16(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    if let Some(byte) = chunks.remainder().first() {
        sum = sum.wrapping_add((*byte as u32) << 8);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn build_udp_ipv4_frame(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let udp_len = 8 + payload.len();
    let ip_len = 20 + udp_len;
    let mut frame = Vec::with_capacity(14 + ip_len);
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.extend_from_slice(&0x0800u16.to_be_bytes());

    let mut ip = [0u8; 20];
    ip[0] = 0x45;
    ip[2..4].copy_from_slice(&(ip_len as u16).to_be_bytes());
    ip[8] = 64;
    ip[9] = 17;
    ip[12..16].copy_from_slice(&src_ip);
    ip[16..20].copy_from_slice(&dst_ip);
    let ip_checksum = checksum16(&ip);
    ip[10..12].copy_from_slice(&ip_checksum.to_be_bytes());
    frame.extend_from_slice(&ip);

    let mut udp = Vec::with_capacity(udp_len);
    udp.extend_from_slice(&src_port.to_be_bytes());
    udp.extend_from_slice(&dst_port.to_be_bytes());
    udp.extend_from_slice(&(udp_len as u16).to_be_bytes());
    udp.extend_from_slice(&0u16.to_be_bytes());
    udp.extend_from_slice(payload);

    let mut pseudo = Vec::with_capacity(12 + udp.len());
    pseudo.extend_from_slice(&src_ip);
    pseudo.extend_from_slice(&dst_ip);
    pseudo.push(0);
    pseudo.push(17);
    pseudo.extend_from_slice(&(udp_len as u16).to_be_bytes());
    pseudo.extend_from_slice(&udp);
    let udp_checksum = checksum16(&pseudo);
    udp[6..8].copy_from_slice(&udp_checksum.to_be_bytes());
    frame.extend_from_slice(&udp);
    frame
}
#[test]
fn unknown_syscall_number_maps_to_einval() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let pid = runtime
        .spawn_process("user", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let result = runtime.dispatch_user_syscall_frame(pid, SyscallFrame::new(9_999, [0; 6]));
    assert_eq!(result, SyscallReturn::err(Errno::Inval));
}

#[test]
fn write_with_invalid_fd_maps_to_ebadf() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    let frame = SyscallFrame::new(SYS_WRITE, [1, mapped as usize, 4, 0, 0, 0]);
    let result = runtime.dispatch_user_syscall_frame(pid, frame);
    assert_eq!(result, SyscallReturn::err(Errno::Badf));
}

#[test]
fn write_with_null_pointer_and_non_zero_len_maps_to_efault() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let pid = runtime
        .spawn_process("user", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let frame = SyscallFrame::new(SYS_WRITE, [1, 0, 8, 0, 0, 0]);
    let result = runtime.dispatch_user_syscall_frame(pid, frame);
    assert_eq!(result, SyscallReturn::err(Errno::Fault));
}

#[test]
fn vm_quarantine_user_syscalls_block_then_release_touch_flow() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"user-vm-quarantine")
        .unwrap();

    let scratch = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_MAP_ANONYMOUS_MEMORY,
                [pid.raw() as usize, 0x2000, 0x3, mapped as usize, 18, 0],
            ),
        )
        .into_result()
        .unwrap() as u64;
    let vm_object_id = runtime.resolve_vm_object_id(pid, scratch, 0x2000).unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_QUARANTINE_VM_OBJECT,
                [pid.raw() as usize, vm_object_id as usize, 123, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_STORE_MEMORY_WORD,
                [pid.raw() as usize, scratch as usize, 7, 0, 0, 0],
            ),
        ),
        SyscallReturn::err(Errno::Busy)
    );

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_RELEASE_VM_OBJECT,
                [pid.raw() as usize, vm_object_id as usize, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_STORE_MEMORY_WORD,
                [pid.raw() as usize, scratch as usize, 7, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
}

#[test]
fn reclaim_memory_pressure_user_syscall_evicts_dirty_file_pages() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(12_950), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(12_951), 0),
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

    let mapped_file = runtime
        .map_file_memory(
            pid,
            "/lib/libpressure.so".to_string(),
            0x3000,
            0xd000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    runtime
        .protect_memory(pid, mapped_file, 0x3000, true, true, false)
        .unwrap();
    runtime
        .touch_memory(pid, mapped_file, 0x3000, true)
        .unwrap();

    let reclaimed = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_RECLAIM_MEMORY_PRESSURE,
                [pid.raw() as usize, 2, 0, 0, 0, 0],
            ),
        )
        .into_result()
        .unwrap();
    assert!(reclaimed >= 2);

    let vmobjects_path = format!("/proc/{}/vmobjects", pid.raw());
    runtime
        .copy_to_user(pid, mapped as usize, vmobjects_path.as_bytes())
        .unwrap();
    let len = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_READ_PROCFS,
                [
                    mapped as usize,
                    vmobjects_path.len(),
                    mapped as usize + 0x100,
                    512,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let vmobjects = String::from_utf8(
        runtime
            .copy_from_user(pid, mapped as usize + 0x100, len)
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects.contains("dirty=0"));
    assert!(vmobjects.contains("resident=0"));
}

#[test]
fn reclaim_memory_pressure_global_user_syscall_evicts_largest_cross_process_target() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let a = runtime
        .spawn_process("a", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let b = runtime
        .spawn_process("b", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(12_952), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(12_953), 0),
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

    let map_a = runtime
        .map_file_memory(a, "/lib/libglobal-a.so", 0x3000, 0, true, false, true, true)
        .unwrap();
    runtime
        .protect_memory(a, map_a, 0x3000, true, true, false)
        .unwrap();
    runtime.touch_memory(a, map_a, 0x3000, true).unwrap();

    let map_b = runtime
        .map_file_memory(b, "/lib/libglobal-b.so", 0x1000, 0, true, false, true, true)
        .unwrap();
    runtime
        .protect_memory(b, map_b, 0x1000, true, true, false)
        .unwrap();
    runtime.touch_memory(b, map_b, 0x1000, true).unwrap();

    let reclaimed = runtime
        .dispatch_user_syscall_frame(
            a,
            SyscallFrame::new(SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL, [3, 0, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert!(reclaimed >= 3);

    let vmobjects_a = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", a.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects_a.contains("resident=0"));
    assert!(vmobjects_a.contains("dirty=0"));

    let vmobjects_b = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmobjects", b.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmobjects_b.contains("resident=1"));
    assert!(vmobjects_b.contains("dirty=1"));
}

#[test]
fn exit_syscall_sets_exit_status_for_reap() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let pid = runtime
        .spawn_process("user", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let result = runtime
        .dispatch_user_syscall_frame(pid, SyscallFrame::new(SYS_EXIT, [77usize, 0, 0, 0, 0, 0]));
    assert_eq!(result, SyscallReturn::ok(0));

    let info = runtime.inspect_process(pid).unwrap();
    assert_eq!(info.process.exit_code, Some(77));
    assert_eq!(info.process.state, ProcessState::Exited);
}

#[test]
fn boot_report_syscall_tracks_process_session_progress_and_rejects_regression() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_BOOT_REPORT,
                [
                    BootSessionStatus::Success as usize,
                    BootSessionStage::Bootstrap as usize,
                    0,
                    0x400000,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_BOOT_REPORT,
                [
                    BootSessionStatus::Success as usize,
                    BootSessionStage::NativeRuntime as usize,
                    0,
                    0x401000,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_BOOT_REPORT,
                [
                    BootSessionStatus::Success as usize,
                    BootSessionStage::Bootstrap as usize,
                    0,
                    0x400000,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::err(Errno::Inval)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_BOOT_REPORT,
                [
                    BootSessionStatus::Success as usize,
                    BootSessionStage::Complete as usize,
                    0,
                    2,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let info = runtime.inspect_process(pid).unwrap();
    assert!(info.process.session_reported);
    assert_eq!(info.process.session_report_count, 3);
    assert_eq!(
        info.process.session_status,
        BootSessionStatus::Success as u32
    );
    assert_eq!(
        info.process.session_stage,
        BootSessionStage::Complete as u32
    );
    assert_eq!(info.process.session_detail, 2);
}

#[test]
fn process_listing_and_procfs_user_syscalls_copy_results_into_user_memory() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_002), 0),
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
        .create_vfs_node("/bin/user", ObjectKind::File, bin)
        .unwrap();
    runtime
        .exec_process(
            pid,
            "/bin/user",
            vec![String::from("user")],
            vec![String::from("TERM=kernel")],
        )
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "user-procfs-syscall")
        .unwrap();
    let list_ptr = mapped as usize;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_LIST_PROCESSES, [list_ptr, 8, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(1)
    );
    let list_bytes = runtime.copy_from_user(pid, list_ptr, 8).unwrap();
    let listed_pid = u64::from_ne_bytes(list_bytes.try_into().unwrap());
    assert_eq!(listed_pid, pid.raw());

    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/proc/1/status")
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_READ_PROCFS,
                [
                    mapped as usize + 0x80,
                    14,
                    mapped as usize + 0x100,
                    256,
                    0,
                    0
                ],
            ),
        ),
        SyscallReturn::ok(256)
    );
    let status = String::from_utf8(
        runtime
            .copy_from_user(pid, mapped as usize + 0x100, 256)
            .unwrap(),
    )
    .unwrap();
    assert!(status.contains("Name:\tuser"));
    assert!(status.contains("SessionReported:\tfalse"));
}

#[test]
fn process_metadata_user_syscalls_update_cmdline_environ_and_cwd() {
    fn encode_table(values: &[&str]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in values {
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(0);
        }
        bytes
    }

    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_102), 0),
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
        .create_vfs_node("/games", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/games/orbit", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin/worker", ObjectKind::File, bin)
        .unwrap();
    runtime
        .exec_process(
            pid,
            "/bin/worker",
            vec![String::from("worker")],
            vec![String::from("TERM=kernel")],
        )
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x4000, true, true, false, "user-process-meta-syscall")
        .unwrap();
    let argv = encode_table(&["/bin/worker", "--fullscreen", "--vsync"]);
    let envp = encode_table(&[
        "NGOS_GAME_TITLE=Orbit Runner",
        "NGOS_GFX_BACKEND=vulkan",
        "NGOS_COMPAT_PREFIX=/compat/orbit",
    ]);
    let cwd = b"/games/orbit";

    runtime.copy_to_user(pid, mapped as usize, &argv).unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x400, &envp)
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x800, cwd)
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_PROCESS_ARGS,
                [pid.raw() as usize, mapped as usize, argv.len(), 3, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_PROCESS_ENV,
                [
                    pid.raw() as usize,
                    mapped as usize + 0x400,
                    envp.len(),
                    3,
                    0,
                    0
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_PROCESS_CWD,
                [
                    pid.raw() as usize,
                    mapped as usize + 0x800,
                    cwd.len(),
                    0,
                    0,
                    0
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let cwd_out = mapped as usize + 0xc00;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_GET_PROCESS_CWD,
                [pid.raw() as usize, cwd_out, 64, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(cwd.len())
    );
    let cwd_text =
        String::from_utf8(runtime.copy_from_user(pid, cwd_out, cwd.len()).unwrap()).unwrap();
    assert_eq!(cwd_text, "/games/orbit");

    let cmdline_path = format!("/proc/{}/cmdline", pid.raw());
    let environ_path = format!("/proc/{}/environ", pid.raw());
    runtime
        .copy_to_user(pid, mapped as usize + 0x1000, cmdline_path.as_bytes())
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x1400, environ_path.as_bytes())
        .unwrap();

    let cmdline_out = mapped as usize + 0x1800;
    let environ_out = mapped as usize + 0x2000;
    let cmdline_count = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_READ_PROCFS,
                [
                    mapped as usize + 0x1000,
                    cmdline_path.len(),
                    cmdline_out,
                    256,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let environ_count = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_READ_PROCFS,
                [
                    mapped as usize + 0x1400,
                    environ_path.len(),
                    environ_out,
                    256,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let cmdline_bytes = runtime
        .copy_from_user(pid, cmdline_out, cmdline_count)
        .unwrap();
    let environ_bytes = runtime
        .copy_from_user(pid, environ_out, environ_count)
        .unwrap();
    let cmdline_text = String::from_utf8(cmdline_bytes).unwrap();
    let environ_text = String::from_utf8(environ_bytes).unwrap();
    assert!(cmdline_text.contains("/bin/worker"));
    assert!(cmdline_text.contains("--fullscreen"));
    assert!(cmdline_text.contains("--vsync"));
    assert!(environ_text.contains("NGOS_GAME_TITLE=Orbit Runner"));
    assert!(environ_text.contains("NGOS_GFX_BACKEND=vulkan"));
    assert!(environ_text.contains("NGOS_COMPAT_PREFIX=/compat/orbit"));
}

#[test]
fn signal_user_syscalls_queue_and_copy_pending_signals() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    let target = runtime
        .spawn_process("target", Some(pid), SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_signal_disposition(target, 9, Some(SignalDisposition::Catch), 0, false)
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "user-signal-syscall")
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_SEND_SIGNAL, [target.raw() as usize, 9, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(runtime.pending_signals(target).unwrap(), vec![9]);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_PENDING_SIGNALS,
                [target.raw() as usize, mapped as usize, 8, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(1)
    );
    assert_eq!(
        runtime.copy_from_user(pid, mapped as usize, 1).unwrap(),
        vec![9]
    );
}

#[test]
fn inspect_process_user_syscall_copies_structured_process_record() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    runtime
        .set_process_args(
            pid,
            vec![String::from("user"), String::from("--interactive")],
        )
        .unwrap();
    runtime
        .set_process_env(
            pid,
            vec![String::from("TERM=kernel"), String::from("HOME=/")],
        )
        .unwrap();
    runtime
        .set_signal_disposition(pid, 17, Some(SignalDisposition::Catch), 0, false)
        .unwrap();
    runtime
        .send_signal(
            PendingSignalSender {
                pid,
                tid: runtime.processes.get(pid).unwrap().main_thread().unwrap(),
            },
            pid,
            17,
        )
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "user-process-record")
        .unwrap();
    let record_ptr = mapped as usize + 0x100;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_PROCESS,
                [pid.raw() as usize, record_ptr, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    let bytes = runtime
        .copy_from_user(pid, record_ptr, core::mem::size_of::<NativeProcessRecord>())
        .unwrap();
    let record = unsafe { core::ptr::read_unaligned(bytes.as_ptr().cast::<NativeProcessRecord>()) };
    assert_eq!(record.pid, pid.raw());
    assert_eq!(record.parent, 0);
    assert_ne!(record.address_space, 0);
    assert_ne!(record.main_thread, 0);
    assert_eq!(record.state, 1);
    assert_eq!(record.exit_code, 0);
    assert_eq!(record.environment_count, 2);
    assert_eq!(record.pending_signal_count, 1);
    assert_eq!(record.session_reported, 0);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_GET_PROCESS_NAME,
                [pid.raw() as usize, mapped as usize + 0x200, 32, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(4)
    );
    assert_eq!(
        String::from_utf8(
            runtime
                .copy_from_user(pid, mapped as usize + 0x200, 4)
                .unwrap()
        )
        .unwrap(),
        "user"
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_GET_PROCESS_IMAGE_PATH,
                [pid.raw() as usize, mapped as usize + 0x240, 64, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(4)
    );
    assert_eq!(
        String::from_utf8(
            runtime
                .copy_from_user(pid, mapped as usize + 0x240, 4)
                .unwrap()
        )
        .unwrap(),
        "user"
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_GET_PROCESS_CWD,
                [pid.raw() as usize, mapped as usize + 0x280, 32, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(1)
    );
    assert_eq!(
        String::from_utf8(
            runtime
                .copy_from_user(pid, mapped as usize + 0x280, 1)
                .unwrap()
        )
        .unwrap(),
        "/"
    );
}

#[test]
fn chdir_user_syscall_updates_process_cwd() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_310), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/workspace", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x300, b"/workspace")
        .unwrap();

    let result = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_CHDIR_PATH,
            [mapped as usize + 0x300, "/workspace".len(), 0, 0, 0, 0],
        ),
    );
    assert_eq!(result.into_result().unwrap(), 0);
    assert_eq!(runtime.process_info(pid).unwrap().cwd, "/workspace");
}

#[test]
fn spawn_and_reap_user_syscalls_create_exec_kill_and_reap_process() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_201), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_202), 0),
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
        .create_vfs_node("/bin/worker", ObjectKind::File, bin)
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "user-spawn-syscall")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"worker")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/bin/worker")
        .unwrap();

    let spawned = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_SPAWN_PATH_PROCESS,
            [mapped as usize + 0x40, 6, mapped as usize + 0x80, 11, 0, 0],
        ),
    );
    let spawned_pid = spawned.into_result().unwrap() as u64;
    let spawned_pid = ProcessId::from_handle(ObjectHandle::new(Handle::new(spawned_pid), 0));
    let info = runtime.inspect_process(spawned_pid).unwrap();
    assert_eq!(info.process.name, "worker");
    assert_eq!(info.process.image_path.as_str(), "/bin/worker");

    runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_SEND_SIGNAL, [spawned_pid.raw() as usize, 9, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let info = runtime.inspect_process(spawned_pid).unwrap();
    assert_eq!(info.process.state, ProcessState::Exited);

    let reaped = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_REAP_PROCESS,
            [spawned_pid.raw() as usize, 0, 0, 0, 0, 0],
        ),
    );
    assert_eq!(reaped.into_result().unwrap() as i32, 137);
}

#[test]
fn spawned_process_can_update_cwd_args_env_then_exit_and_reap() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_301), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_302), 0),
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
        .create_vfs_node("/games", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/games/orbit", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin/worker", ObjectKind::File, bin)
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x2000, true, true, false, "user-spawn-config")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"worker")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/bin/worker")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x100, b"/games/orbit")
        .unwrap();

    let spawned = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_SPAWN_PATH_PROCESS,
            [mapped as usize + 0x40, 6, mapped as usize + 0x80, 11, 0, 0],
        ),
    );
    let spawned_pid = spawned.into_result().unwrap() as u64;
    let spawned_pid = ProcessId::from_handle(ObjectHandle::new(Handle::new(spawned_pid), 0));

    let argv = ["/bin/worker", "--fullscreen"];
    let envp = ["NGOS_GAME_CHANNEL=/compat/orbit/session.chan"];
    let argv_payload = argv.join("\0") + "\0";
    let env_payload = envp.join("\0") + "\0";
    runtime
        .copy_to_user(pid, mapped as usize + 0x180, argv_payload.as_bytes())
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x280, env_payload.as_bytes())
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_PROCESS_CWD,
                [
                    spawned_pid.raw() as usize,
                    mapped as usize + 0x100,
                    "/games/orbit".len(),
                    0,
                    0,
                    0
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_PROCESS_ARGS,
                [
                    spawned_pid.raw() as usize,
                    mapped as usize + 0x180,
                    argv_payload.len(),
                    argv.len(),
                    0,
                    0
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_PROCESS_ENV,
                [
                    spawned_pid.raw() as usize,
                    mapped as usize + 0x280,
                    env_payload.len(),
                    envp.len(),
                    0,
                    0
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SEND_SIGNAL,
                [spawned_pid.raw() as usize, 15, 0, 0, 0, 0],
            ),
        )
        .into_result()
        .unwrap();
    let reaped = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_REAP_PROCESS,
            [spawned_pid.raw() as usize, 0, 0, 0, 0, 0],
        ),
    );
    assert_eq!(reaped.into_result().unwrap() as i32, 143);
}

#[test]
fn stat_and_statfs_user_syscalls_copy_structured_records_into_user_memory() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let file = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_102), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "file",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/data.bin", ObjectKind::File, file)
        .unwrap();
    runtime.mount("/compat/foreign", "foreign-root").unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "user-stat-syscall")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"/data.bin")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/compat/foreign")
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_STAT_PATH,
                [mapped as usize + 0x40, 9, mapped as usize + 0x100, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let stat_bytes = runtime
        .copy_from_user(
            pid,
            mapped as usize + 0x100,
            core::mem::size_of::<NativeFileStatusRecord>(),
        )
        .unwrap();
    let stat_record =
        unsafe { core::ptr::read_unaligned(stat_bytes.as_ptr().cast::<NativeFileStatusRecord>()) };
    assert_eq!(stat_record.kind, NativeObjectKind::File as u32);
    assert!(stat_record.inode > 0);
    assert!(stat_record.writable != 0);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_LSTAT_PATH,
                [mapped as usize + 0x40, 9, mapped as usize + 0x180, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let lstat_bytes = runtime
        .copy_from_user(
            pid,
            mapped as usize + 0x180,
            core::mem::size_of::<NativeFileStatusRecord>(),
        )
        .unwrap();
    let lstat_record =
        unsafe { core::ptr::read_unaligned(lstat_bytes.as_ptr().cast::<NativeFileStatusRecord>()) };
    assert_eq!(lstat_record.kind, NativeObjectKind::File as u32);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_STATFS_PATH,
                [mapped as usize + 0x80, 15, mapped as usize + 0x200, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let statfs_bytes = runtime
        .copy_from_user(
            pid,
            mapped as usize + 0x200,
            core::mem::size_of::<NativeFileSystemStatusRecord>(),
        )
        .unwrap();
    let statfs_record = unsafe {
        core::ptr::read_unaligned(statfs_bytes.as_ptr().cast::<NativeFileSystemStatusRecord>())
    };
    assert_eq!(statfs_record.mount_count, 2);
    assert!(statfs_record.node_count >= 1);
}

#[test]
fn open_and_readlink_user_syscalls_roundtrip_path_results() {
    let (mut runtime, pid, _) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_201), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let file = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(9_202), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "file",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/motd", ObjectKind::File, file)
        .unwrap();
    runtime
        .create_vfs_symlink("/motd-link", "/motd", root)
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "user-open-syscall")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"/motd")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/motd-link")
        .unwrap();

    let opened = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_OPEN_PATH, [mapped as usize + 0x40, 5, 0, 0, 0, 0]),
    );
    let fd = opened.into_result().unwrap() as u32;
    assert_eq!(fd, 0);

    let readlink = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_READLINK_PATH,
            [
                mapped as usize + 0x80,
                10,
                mapped as usize + 0x100,
                32,
                0,
                0,
            ],
        ),
    );
    assert_eq!(readlink, SyscallReturn::ok(5));
    let target = String::from_utf8(
        runtime
            .copy_from_user(pid, mapped as usize + 0x100, 5)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(target, "/motd");
}

#[test]
fn path_mutation_user_syscalls_create_rename_and_unlink_nodes() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime.copy_to_user(pid, mapped as usize, b"/tmp").unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"/tmp/note")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/tmp/current")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0xc0, b"/tmp/note-2")
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_MKDIR_PATH, [mapped as usize, 4, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_MKFILE_PATH, [mapped as usize + 0x40, 9, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SYMLINK_PATH,
                [mapped as usize + 0x80, 12, mapped as usize + 0x40, 9, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_RENAME_PATH,
                [mapped as usize + 0x40, 9, mapped as usize + 0xc0, 11, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let stat = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_STAT_PATH,
            [mapped as usize + 0xc0, 11, mapped as usize + 0x180, 0, 0, 0],
        ),
    );
    assert_eq!(stat, SyscallReturn::ok(0));
    let stat_bytes = runtime
        .copy_from_user(
            pid,
            mapped as usize + 0x180,
            core::mem::size_of::<NativeFileStatusRecord>(),
        )
        .unwrap();
    let stat_record =
        unsafe { core::ptr::read_unaligned(stat_bytes.as_ptr().cast::<NativeFileStatusRecord>()) };
    assert_eq!(stat_record.kind, NativeObjectKind::File as u32);

    let readlink = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_READLINK_PATH,
            [
                mapped as usize + 0x80,
                12,
                mapped as usize + 0x200,
                32,
                0,
                0,
            ],
        ),
    );
    assert_eq!(readlink, SyscallReturn::ok(9));
    let target = String::from_utf8(
        runtime
            .copy_from_user(pid, mapped as usize + 0x200, 9)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(target, "/tmp/note");

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_UNLINK_PATH, [mapped as usize + 0x80, 12, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_UNLINK_PATH, [mapped as usize + 0xc0, 11, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_STAT_PATH,
                [mapped as usize + 0xc0, 11, mapped as usize + 0x180, 0, 0, 0]
            ),
        ),
        SyscallReturn::err(Errno::NoEnt)
    );
}

#[test]
fn networking_user_syscalls_configure_bind_inspect_and_move_udp_traffic() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(12_001), 0),
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
        .bind_device_to_driver("/dev/net0", "/drv/net0")
        .unwrap();

    runtime
        .copy_to_user(pid, mapped as usize, b"/run/net0.sock")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"/dev/net0")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/drv/net0")
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_MKSOCK_PATH, [mapped as usize, 14, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    runtime
        .copy_to_user(pid, mapped as usize + 0x20, b"/run/game.chan")
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_MKCHAN_PATH, [mapped as usize + 0x20, 14, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );

    let iface_config = NativeNetworkInterfaceConfig {
        addr: [10, 1, 0, 2],
        netmask: [255, 255, 255, 0],
        gateway: [10, 1, 0, 1],
    };
    let iface_bytes = unsafe {
        core::slice::from_raw_parts(
            (&iface_config as *const NativeNetworkInterfaceConfig).cast::<u8>(),
            core::mem::size_of::<NativeNetworkInterfaceConfig>(),
        )
    };
    runtime
        .copy_to_user(pid, mapped as usize + 0x100, iface_bytes)
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CONFIGURE_NETIF_IPV4,
                [mapped as usize + 0x40, 9, mapped as usize + 0x100, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let udp_bind = NativeUdpBindConfig {
        remote_ipv4: [10, 1, 0, 9],
        local_port: 4000,
        remote_port: 5000,
    };
    let bind_bytes = unsafe {
        core::slice::from_raw_parts(
            (&udp_bind as *const NativeUdpBindConfig).cast::<u8>(),
            core::mem::size_of::<NativeUdpBindConfig>(),
        )
    };
    runtime
        .copy_to_user(pid, mapped as usize + 0x140, bind_bytes)
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_BIND_UDP_SOCKET,
                [
                    mapped as usize,
                    14,
                    mapped as usize + 0x40,
                    9,
                    mapped as usize + 0x140,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let socket_info_ptr = mapped as usize + 0x180;
    let iface_info_ptr = mapped as usize + 0x200;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_NETSOCK,
                [mapped as usize, 14, socket_info_ptr, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_NETIF,
                [mapped as usize + 0x40, 9, iface_info_ptr, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let socket_record = unsafe {
        core::ptr::read_unaligned(
            runtime
                .copy_from_user(
                    pid,
                    socket_info_ptr,
                    core::mem::size_of::<NativeNetworkSocketRecord>(),
                )
                .unwrap()
                .as_ptr()
                .cast::<NativeNetworkSocketRecord>(),
        )
    };
    assert_eq!(socket_record.local_port, 4000);
    assert_eq!(socket_record.remote_port, 5000);
    assert_eq!(socket_record.remote_ipv4, [10, 1, 0, 9]);

    let iface_record = unsafe {
        core::ptr::read_unaligned(
            runtime
                .copy_from_user(
                    pid,
                    iface_info_ptr,
                    core::mem::size_of::<NativeNetworkInterfaceRecord>(),
                )
                .unwrap()
                .as_ptr()
                .cast::<NativeNetworkInterfaceRecord>(),
        )
    };
    assert_eq!(iface_record.ipv4_addr, [10, 1, 0, 2]);
    assert_eq!(iface_record.link_up, 1);
    assert_eq!(iface_record.attached_socket_count, 1);

    let socket_fd = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_OPEN_PATH, [mapped as usize, 14, 0, 0, 0, 0]),
    );
    let socket_fd = socket_fd.into_result().unwrap();
    let driver_fd = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_OPEN_PATH, [mapped as usize + 0x80, 9, 0, 0, 0, 0]),
    );
    let driver_fd = driver_fd.into_result().unwrap();

    runtime
        .copy_to_user(pid, mapped as usize + 0x280, b"hello-net")
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_WRITE, [socket_fd, mapped as usize + 0x280, 9, 0, 0, 0],),
        ),
        SyscallReturn::ok(9)
    );
    let tx_poll = runtime
        .dispatch_user_syscall_frame(pid, SyscallFrame::new(SYS_POLL, [driver_fd, 1, 0, 0, 0, 0]));
    assert_eq!(tx_poll, SyscallReturn::ok(1));

    let driver_read = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_READ, [driver_fd, mapped as usize + 0x300, 128, 0, 0, 0]),
    );
    assert!(driver_read.into_result().unwrap() > 0);
    let driver_bytes = runtime
        .copy_from_user(pid, mapped as usize + 0x300, 128)
        .unwrap();
    let transcript_len = driver_bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(driver_bytes.len());
    let transcript = String::from_utf8(driver_bytes[..transcript_len].to_vec()).unwrap();
    assert!(transcript.contains("net-tx iface=/dev/net0"));
    assert!(transcript.contains("sport=4000"));
    assert!(transcript.contains("dport=5000"));

    let frame = build_udp_ipv4_frame(
        [0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee],
        [0x02, 0x2b, 0x19, 0x44, 0x55, 0x66],
        [10, 1, 0, 9],
        [10, 1, 0, 2],
        5000,
        4000,
        b"reply-net",
    );
    runtime
        .copy_to_user(pid, mapped as usize + 0x400, &frame)
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_WRITE,
                [driver_fd, mapped as usize + 0x400, frame.len(), 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(frame.len())
    );

    let socket_poll = runtime
        .dispatch_user_syscall_frame(pid, SyscallFrame::new(SYS_POLL, [socket_fd, 1, 0, 0, 0, 0]));
    assert_eq!(socket_poll, SyscallReturn::ok(1));
    let socket_read = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_READ, [socket_fd, mapped as usize + 0x500, 64, 0, 0, 0]),
    );
    assert_eq!(socket_read, SyscallReturn::ok(9));
    let rx_payload = runtime
        .copy_from_user(pid, mapped as usize + 0x500, 9)
        .unwrap();
    assert_eq!(rx_payload, b"reply-net");
}

#[test]
fn networking_user_syscalls_expose_link_control_and_event_queue_delivery() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    let root = runtime
        .grant_capability(
            pid,
            ObjectHandle::new(Handle::new(12_100), 0),
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
        .copy_to_user(pid, mapped as usize, b"/dev/net0")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"/run/net0.sock")
        .unwrap();
    let netif_config = NativeNetworkInterfaceConfig {
        addr: [10, 1, 0, 2],
        netmask: [255, 255, 255, 0],
        gateway: [10, 1, 0, 1],
    };
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, unsafe {
            core::slice::from_raw_parts(
                (&netif_config as *const NativeNetworkInterfaceConfig).cast::<u8>(),
                core::mem::size_of::<NativeNetworkInterfaceConfig>(),
            )
        })
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CONFIGURE_NETIF_IPV4,
                [mapped as usize, 9, mapped as usize + 0x80, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let udp_bind = NativeUdpBindConfig {
        remote_ipv4: [10, 1, 0, 9],
        local_port: 4000,
        remote_port: 5000,
    };
    runtime
        .copy_to_user(pid, mapped as usize + 0x100, unsafe {
            core::slice::from_raw_parts(
                (&udp_bind as *const NativeUdpBindConfig).cast::<u8>(),
                core::mem::size_of::<NativeUdpBindConfig>(),
            )
        })
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_BIND_UDP_SOCKET,
                [
                    mapped as usize + 0x40,
                    14,
                    mapped as usize,
                    9,
                    mapped as usize + 0x100,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let queue_fd = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_EVENT_QUEUE, [0, 0, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let watch = NativeNetworkEventWatchConfig {
        token: 733,
        poll_events: POLLPRI,
        link_changed: 1,
        rx_ready: 1,
        tx_drained: 1,
        reserved: 0,
    };
    runtime
        .copy_to_user(pid, mapped as usize + 0x180, unsafe {
            core::slice::from_raw_parts(
                (&watch as *const NativeNetworkEventWatchConfig).cast::<u8>(),
                core::mem::size_of::<NativeNetworkEventWatchConfig>(),
            )
        })
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_WATCH_NET_EVENTS,
                [
                    queue_fd,
                    mapped as usize,
                    9,
                    mapped as usize + 0x40,
                    14,
                    mapped as usize + 0x180,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let socket_fd = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_OPEN_PATH, [mapped as usize + 0x40, 14, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let driver_path_ptr = mapped as usize + 0x220;
    runtime
        .copy_to_user(pid, driver_path_ptr, b"/drv/net0")
        .unwrap();
    let driver_fd = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_OPEN_PATH, [driver_path_ptr, 9, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();

    runtime
        .copy_to_user(pid, mapped as usize + 0x260, b"user-event")
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_WRITE, [socket_fd, mapped as usize + 0x260, 10, 0, 0, 0],),
        ),
        SyscallReturn::ok(10)
    );
    assert!(
        runtime
            .read_io(pid, Descriptor::new(driver_fd as u32), 256)
            .is_ok()
    );

    let events_ptr = mapped as usize + 0x300;
    let waited = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_WAIT_EVENT_QUEUE, [queue_fd, events_ptr, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert!(waited >= 1);
    let first = unsafe {
        core::ptr::read_unaligned(
            runtime
                .copy_from_user(pid, events_ptr, core::mem::size_of::<NativeEventRecord>())
                .unwrap()
                .as_ptr()
                .cast::<NativeEventRecord>(),
        )
    };
    assert_eq!(first.token, 733);
    assert_eq!(first.source_kind, NativeEventSourceKind::Network as u32);

    let link_state = NativeNetworkLinkStateConfig {
        link_up: 0,
        reserved: 0,
    };
    runtime
        .copy_to_user(pid, mapped as usize + 0x3c0, unsafe {
            core::slice::from_raw_parts(
                (&link_state as *const NativeNetworkLinkStateConfig).cast::<u8>(),
                core::mem::size_of::<NativeNetworkLinkStateConfig>(),
            )
        })
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_NETIF_LINK_STATE,
                [mapped as usize, 9, mapped as usize + 0x3c0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let waited = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_WAIT_EVENT_QUEUE, [queue_fd, events_ptr, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert!(waited >= 1);
    let second = unsafe {
        core::ptr::read_unaligned(
            runtime
                .copy_from_user(pid, events_ptr, core::mem::size_of::<NativeEventRecord>())
                .unwrap()
                .as_ptr()
                .cast::<NativeEventRecord>(),
        )
    };
    assert_eq!(second.detail1, NativeNetworkEventKind::LinkChanged as u32);
}

#[test]
fn list_path_user_syscall_renders_directory_entries_into_user_memory() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime.copy_to_user(pid, mapped as usize, b"/tmp").unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x40, b"/tmp/a")
        .unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x80, b"/tmp/b")
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_MKDIR_PATH, [mapped as usize, 4, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_MKFILE_PATH, [mapped as usize + 0x40, 6, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_MKFILE_PATH, [mapped as usize + 0x80, 6, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );

    let listed = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_LIST_PATH,
            [mapped as usize, 4, mapped as usize + 0x200, 128, 0, 0],
        ),
    );
    let count = listed.into_result().unwrap();
    let text = String::from_utf8(
        runtime
            .copy_from_user(pid, mapped as usize + 0x200, count)
            .unwrap(),
    )
    .unwrap();
    assert!(text.contains("a\tFile"));
    assert!(text.contains("b\tFile"));
}

#[test]
fn write_and_read_use_user_memory_copy_semantics() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime.copy_to_user(pid, mapped as usize, b"abc").unwrap();
    let fd = open_file_descriptor(&mut runtime, pid, "roundtrip");

    let wrote = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_WRITE, [fd.raw() as usize, mapped as usize, 3, 0, 0, 0]),
    );
    assert_eq!(wrote, SyscallReturn::ok(3));

    let read_ptr = mapped as usize + 0x80;
    let read = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_READ, [fd.raw() as usize, read_ptr, 16, 0, 0, 0]),
    );
    assert_eq!(read.into_result().unwrap(), 16);
    let bytes = runtime.copy_from_user(pid, read_ptr, 3).unwrap();
    assert_eq!(bytes, b"obj");
}

#[test]
fn readv_and_writev_use_user_iovec_arrays_and_segment_buffers() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    let fd = open_file_descriptor(&mut runtime, pid, "vectored");
    runtime.copy_to_user(pid, mapped as usize, b"ab").unwrap();
    runtime
        .copy_to_user(pid, mapped as usize + 0x20, b"cdef")
        .unwrap();
    let write_iovecs = [
        UserIoVec {
            base: mapped as usize,
            len: 2,
        },
        UserIoVec {
            base: mapped as usize + 0x20,
            len: 4,
        },
    ];
    let write_iovec_ptr = mapped as usize + 0x80;
    let write_iovec_bytes = unsafe {
        core::slice::from_raw_parts(
            write_iovecs.as_ptr().cast::<u8>(),
            core::mem::size_of_val(&write_iovecs),
        )
    };
    runtime
        .copy_to_user(pid, write_iovec_ptr, write_iovec_bytes)
        .unwrap();

    let wrote = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_WRITEV,
            [
                fd.raw() as usize,
                write_iovec_ptr,
                write_iovecs.len(),
                0,
                0,
                0,
            ],
        ),
    );
    assert_eq!(wrote, SyscallReturn::ok(6));

    let read_iovecs = [
        UserIoVec {
            base: mapped as usize + 0x100,
            len: 3,
        },
        UserIoVec {
            base: mapped as usize + 0x120,
            len: 3,
        },
    ];
    let read_iovec_ptr = mapped as usize + 0x180;
    let read_iovec_bytes = unsafe {
        core::slice::from_raw_parts(
            read_iovecs.as_ptr().cast::<u8>(),
            core::mem::size_of_val(&read_iovecs),
        )
    };
    runtime
        .copy_to_user(pid, read_iovec_ptr, read_iovec_bytes)
        .unwrap();

    let read = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_READV,
            [
                fd.raw() as usize,
                read_iovec_ptr,
                read_iovecs.len(),
                0,
                0,
                0,
            ],
        ),
    );
    assert_eq!(read, SyscallReturn::ok(6));
    assert_eq!(
        runtime
            .copy_from_user(pid, mapped as usize + 0x100, 3)
            .unwrap(),
        b"obj"
    );
    assert_eq!(
        runtime
            .copy_from_user(pid, mapped as usize + 0x120, 3)
            .unwrap(),
        b"ect"
    );
}

#[test]
fn close_dup_fcntl_and_poll_are_routed_and_validated() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime.copy_to_user(pid, mapped as usize, b"x").unwrap();
    let fd = open_file_descriptor(&mut runtime, pid, "ops");

    let dup = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_DUP, [fd.raw() as usize, 0, 0, 0, 0, 0]),
    );
    let dup_fd = dup.into_result().unwrap() as u32;
    assert_ne!(dup_fd, fd.raw());

    let fcntl = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_FCNTL, [fd.raw() as usize, 0, 0, 0, 0, 0]),
    );
    assert!(fcntl.into_result().is_ok());

    let poll = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_POLL,
            [
                fd.raw() as usize,
                (IOPOLL_READABLE | IOPOLL_WRITABLE) as usize,
                0,
                0,
                0,
                0,
            ],
        ),
    );
    assert!(poll.into_result().is_ok());

    let close_dup = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_CLOSE, [dup_fd as usize, 0, 0, 0, 0, 0]),
    );
    assert_eq!(close_dup, SyscallReturn::ok(0));

    let close_main = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_CLOSE, [fd.raw() as usize, 0, 0, 0, 0, 0]),
    );
    assert_eq!(close_main, SyscallReturn::ok(0));
}

#[test]
fn read_and_write_fail_with_efault_for_unmapped_user_buffers() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    let fd = open_file_descriptor(&mut runtime, pid, "faults");

    let write_fault = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_WRITE, [fd.raw() as usize, 0, 8, 0, 0, 0]),
    );
    assert_eq!(write_fault, SyscallReturn::err(Errno::Fault));

    let read_fault = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(
            SYS_READ,
            [
                fd.raw() as usize,
                (mapped as usize) + 0x1000 - 1,
                2,
                0,
                0,
                0,
            ],
        ),
    );
    assert_eq!(read_fault, SyscallReturn::err(Errno::Fault));
}
