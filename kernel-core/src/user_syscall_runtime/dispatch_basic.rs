//! Canonical subsystem role:
//! - subsystem: syscall transport and ABI dispatch
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical syscall-number dispatch and ABI record
//!   materialization over kernel truth
//!
//! Canonical contract families handled here:
//! - syscall dispatch contracts
//! - ABI snapshot/inspection transport contracts
//! - kernel-to-user serialization contracts
//!
//! This module may serialize and dispatch canonical kernel truth into ABI
//! records, but it must not define a shadow semantic model independent of
//! `kernel-core`.

use super::*;
use crate::eventing_model::{
    BusEventInterest, BusEventKind, GraphicsEventInterest, GraphicsEventKind,
};
use ngos_user_abi::{
    BootSessionReport, NativeBusEventWatchConfig, NativeDeviceRecord, NativeDeviceRequestRecord,
    NativeDriverRecord, NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind,
    NativeFileStatusRecord, NativeFileSystemStatusRecord, NativeGpuBindingRecord,
    NativeGpuBufferRecord, NativeGpuDisplayRecord, NativeGpuGspRecord, NativeGpuInterruptRecord,
    NativeGpuMediaRecord, NativeGpuNeuralRecord, NativeGpuPowerRecord, NativeGpuScanoutRecord,
    NativeGpuTensorRecord, NativeGpuVbiosRecord, NativeGraphicsEventKind,
    NativeGraphicsEventWatchConfig, NativeNetworkAdminConfig, NativeNetworkEventKind,
    NativeNetworkEventWatchConfig, NativeNetworkInterfaceConfig, NativeNetworkInterfaceRecord,
    NativeNetworkLinkStateConfig, NativeNetworkSocketRecord, NativeProcessCompatRecord,
    NativeProcessEventWatchConfig, NativeProcessRecord, NativeReadinessRecord,
    NativeResourceEventWatchConfig, NativeSchedulerClass, NativeSpawnProcessConfig,
    NativeSystemSnapshotRecord, NativeUdpBindConfig, NativeUdpConnectConfig, NativeUdpRecvMeta,
    NativeUdpSendToConfig, SYS_ADVISE_MEMORY_RANGE, SYS_BIND_DEVICE_DRIVER, SYS_BIND_UDP_SOCKET,
    SYS_BLOCKED_PENDING_SIGNALS, SYS_BOOT_REPORT, SYS_CHDIR_PATH, SYS_COLLECT_READINESS,
    SYS_COMMIT_GPU_NEURAL_FRAME, SYS_COMPLETE_NET_TX, SYS_CONFIGURE_DEVICE_QUEUE,
    SYS_CONFIGURE_NETIF_ADMIN, SYS_CONFIGURE_NETIF_IPV4, SYS_CONNECT_UDP_SOCKET,
    SYS_CONTROL_DESCRIPTOR, SYS_CREATE_EVENT_QUEUE, SYS_CREATE_GPU_BUFFER,
    SYS_DISPATCH_GPU_TENSOR_KERNEL, SYS_GET_PROCESS_CWD, SYS_GET_PROCESS_IMAGE_PATH,
    SYS_GET_PROCESS_NAME, SYS_GET_PROCESS_ROOT, SYS_INJECT_GPU_NEURAL_SEMANTIC, SYS_INSPECT_DEVICE,
    SYS_INSPECT_DEVICE_REQUEST, SYS_INSPECT_DRIVER, SYS_INSPECT_GPU_BINDING,
    SYS_INSPECT_GPU_BUFFER, SYS_INSPECT_GPU_DISPLAY, SYS_INSPECT_GPU_GSP,
    SYS_INSPECT_GPU_INTERRUPT, SYS_INSPECT_GPU_MEDIA, SYS_INSPECT_GPU_NEURAL,
    SYS_INSPECT_GPU_POWER, SYS_INSPECT_GPU_SCANOUT, SYS_INSPECT_GPU_TENSOR, SYS_INSPECT_GPU_VBIOS,
    SYS_INSPECT_NETIF, SYS_INSPECT_NETSOCK, SYS_INSPECT_PROCESS, SYS_INSPECT_PROCESS_COMPAT,
    SYS_INSPECT_SYSTEM_SNAPSHOT, SYS_LIST_PATH, SYS_LIST_PROCESSES, SYS_LOAD_MEMORY_WORD,
    SYS_LSTAT_PATH, SYS_MAP_ANONYMOUS_MEMORY, SYS_MAP_FILE_MEMORY, SYS_MKCHAN_PATH, SYS_MKDIR_PATH,
    SYS_MKFILE_PATH, SYS_MKSOCK_PATH, SYS_OPEN_PATH, SYS_PAUSE_PROCESS, SYS_PENDING_SIGNALS,
    SYS_PRESENT_GPU_FRAME, SYS_PROTECT_MEMORY_RANGE, SYS_QUARANTINE_VM_OBJECT,
    SYS_READ_GPU_SCANOUT_FRAME, SYS_READ_PROCFS, SYS_READLINK_PATH, SYS_READV, SYS_REAP_PROCESS,
    SYS_RECLAIM_MEMORY_PRESSURE, SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL, SYS_RECVFROM_UDP_SOCKET,
    SYS_REGISTER_READINESS, SYS_RELEASE_VM_OBJECT, SYS_REMOVE_BUS_EVENTS,
    SYS_REMOVE_GRAPHICS_EVENTS, SYS_REMOVE_NET_EVENTS, SYS_REMOVE_PROCESS_EVENTS,
    SYS_REMOVE_RESOURCE_EVENTS, SYS_RENAME_PATH, SYS_RENICE_PROCESS, SYS_RESUME_PROCESS,
    SYS_SEND_SIGNAL, SYS_SENDTO_UDP_SOCKET, SYS_SET_GPU_POWER_STATE, SYS_SET_NETIF_LINK_STATE,
    SYS_SET_PROCESS_AFFINITY, SYS_SET_PROCESS_ARGS, SYS_SET_PROCESS_BREAK, SYS_SET_PROCESS_CWD,
    SYS_SET_PROCESS_ENV, SYS_SET_PROCESS_ROOT, SYS_SPAWN_CONFIGURED_PROCESS,
    SYS_SPAWN_PATH_PROCESS, SYS_SPAWN_PROCESS_COPY_VM, SYS_START_GPU_MEDIA_SESSION, SYS_STAT_PATH,
    SYS_STATFS_PATH, SYS_STORE_MEMORY_WORD, SYS_SUBMIT_GPU_BUFFER, SYS_SYMLINK_PATH,
    SYS_SYNC_MEMORY_RANGE, SYS_TCP_ACCEPT, SYS_TCP_CLOSE, SYS_TCP_CONNECT, SYS_TCP_LISTEN,
    SYS_TCP_RECV, SYS_TCP_RESET, SYS_TCP_SEND, SYS_ICMP_ECHO_REQUEST, SYS_CPU_ONLINE, SYS_CPU_OFFLINE,
    SYS_CPU_INFO, SYS_UNBIND_DEVICE_DRIVER,
    SYS_UNLINK_PATH,
    SYS_UNMAP_MEMORY_RANGE, SYS_WAIT_EVENT_QUEUE, SYS_WATCH_BUS_EVENTS, SYS_WATCH_GRAPHICS_EVENTS,
    SYS_WATCH_NET_EVENTS, SYS_WATCH_PROCESS_EVENTS, SYS_WATCH_RESOURCE_EVENTS, SYS_WRITE_GPU_BUFFER,
    SYS_WRITEV,
};

// Canonical subsystem role:
// - subsystem: syscall surface transport
// - owner layer: Layer 1 + Layer 2
// - semantic owner: `kernel-core` for behavior, `user-abi` for record transport
// - truth path role: moves canonical kernel truth into stable ABI records
//
// Canonical contract families handled here:
// - syscall contracts
// - runtime snapshot contracts
// - process/device/network inspection contracts
//
// This module may transport and serialize truth. It must not drift from the
// canonical records defined in `user-abi`.

fn copy_text_field(dst: &mut [u8], text: &str) {
    dst.fill(0);
    let bytes = text.as_bytes();
    let copy_len = core::cmp::min(dst.len().saturating_sub(1), bytes.len());
    dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
}

impl KernelRuntime {
    pub fn dispatch_user_syscall_frame(
        &mut self,
        caller: ProcessId,
        frame: SyscallFrame,
    ) -> SyscallReturn {
        match self.dispatch_user_syscall_frame_result(caller, frame) {
            Ok(result) => result,
            Err(error) => SyscallReturn::err(map_runtime_error_to_errno(error)),
        }
    }

    fn dispatch_user_syscall_frame_result(
        &mut self,
        caller: ProcessId,
        frame: SyscallFrame,
    ) -> Result<SyscallReturn, RuntimeError> {
        if let Some(result) = self.dispatch_native_model_syscall(caller, frame)? {
            return Ok(result);
        }
        match frame.number {
            SYS_READ => {
                let fd = frame_fd(frame.arg0)?;
                let bytes = self.read_io(caller, fd, frame.arg2)?;
                if let Err(error) = self.copy_to_user(caller, frame.arg1, &bytes) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(bytes.len()))
            }
            SYS_WRITE => {
                let fd = frame_fd(frame.arg0)?;
                let bytes = match self.copy_from_user(caller, frame.arg1, frame.arg2) {
                    Ok(bytes) => bytes,
                    Err(error) => return Ok(SyscallReturn::err(error.errno())),
                };
                let written = self.write_io(caller, fd, &bytes)?;
                Ok(SyscallReturn::ok(written))
            }
            SYS_READV => {
                let fd = frame_fd(frame.arg0)?;
                let iovecs = match frame_iovecs(self, caller, frame.arg1, frame.arg2) {
                    Ok(iovecs) => iovecs,
                    Err(error) => return Ok(error),
                };
                let lengths = iovecs.iter().map(|iov| iov.len).collect::<Vec<_>>();
                let segments = self.read_io_vectored(caller, fd, &lengths)?;
                let mut total = 0usize;
                for (iov, bytes) in iovecs.into_iter().zip(segments) {
                    let count = iov.len.min(bytes.len());
                    if let Err(error) = self.copy_to_user(caller, iov.base, &bytes[..count]) {
                        return Ok(SyscallReturn::err(error.errno()));
                    }
                    total += count;
                    if count < iov.len {
                        break;
                    }
                }
                Ok(SyscallReturn::ok(total))
            }
            SYS_WRITEV => {
                let fd = frame_fd(frame.arg0)?;
                let iovecs = match frame_iovecs(self, caller, frame.arg1, frame.arg2) {
                    Ok(iovecs) => iovecs,
                    Err(error) => return Ok(error),
                };
                let mut segments = Vec::with_capacity(iovecs.len());
                for iov in iovecs {
                    let bytes = match self.copy_from_user(caller, iov.base, iov.len) {
                        Ok(bytes) => bytes,
                        Err(error) => return Ok(SyscallReturn::err(error.errno())),
                    };
                    segments.push(bytes);
                }
                let written = self.write_io_vectored(caller, fd, &segments)?;
                Ok(SyscallReturn::ok(written))
            }
            SYS_CLOSE => {
                let fd = frame_fd(frame.arg0)?;
                let _ = self.close_descriptor(caller, fd)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_DUP => {
                let fd = frame_fd(frame.arg0)?;
                let dup = self.duplicate_descriptor(caller, fd)?;
                Ok(SyscallReturn::ok(dup.raw() as usize))
            }
            SYS_FCNTL => {
                let fd = frame_fd(frame.arg0)?;
                let cmd = match decode_fcntl(frame.arg1) {
                    Some(cmd) => cmd,
                    None => return Ok(SyscallReturn::err(Errno::Inval)),
                };
                let value = self.fcntl(caller, fd, cmd)?;
                let raw = match value {
                    FcntlResult::Flags(flags) | FcntlResult::Updated(flags) => {
                        ((flags.cloexec as usize) << 1) | (flags.nonblock as usize)
                    }
                };
                Ok(SyscallReturn::ok(raw))
            }
            SYS_POLL => {
                let fd = frame_fd(frame.arg0)?;
                let polled = self.poll_io(caller, fd)?;
                let mut raw = 0u32;
                if polled.contains(IoPollEvents::READABLE) {
                    raw |= IOPOLL_READABLE;
                }
                if polled.contains(IoPollEvents::WRITABLE) {
                    raw |= IOPOLL_WRITABLE;
                }
                if polled.contains(IoPollEvents::PRIORITY) {
                    raw |= IOPOLL_PRIORITY;
                }
                if polled.contains(IoPollEvents::HANGUP) {
                    raw |= IOPOLL_HANGUP;
                }
                let masked = raw & (frame.arg1 as u32);
                Ok(SyscallReturn::ok(masked as usize))
            }
            SYS_CONTROL_DESCRIPTOR => {
                let fd = frame_fd(frame.arg0)?;
                let response = self.control_io(caller, fd, frame.arg1 as u32)?;
                Ok(SyscallReturn::ok(response as usize))
            }
            SYS_REGISTER_READINESS => {
                let fd = frame_fd(frame.arg0)?;
                let raw = frame.arg1 as u32;
                self.register_readiness(
                    caller,
                    fd,
                    ReadinessInterest {
                        readable: (raw & 0x1) != 0,
                        writable: (raw & 0x2) != 0,
                        priority: (raw & 0x4) != 0,
                    },
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_COLLECT_READINESS => {
                let ready = self.collect_ready()?;
                let records = ready
                    .into_iter()
                    .map(|entry| NativeReadinessRecord {
                        owner: entry.owner.raw(),
                        fd: u64::from(entry.fd.raw()),
                        readable: entry.interest.readable as u32,
                        writable: entry.interest.writable as u32,
                        priority: entry.interest.priority as u32,
                        reserved: 0,
                    })
                    .collect::<Vec<_>>();
                let byte_len = frame.arg1 * core::mem::size_of::<NativeReadinessRecord>();
                let bytes = unsafe {
                    core::slice::from_raw_parts(
                        records.as_ptr() as *const u8,
                        records.len() * core::mem::size_of::<NativeReadinessRecord>(),
                    )
                };
                let copy_len = bytes.len().min(byte_len);
                if let Err(error) = self.copy_to_user(caller, frame.arg0, &bytes[..copy_len]) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(records.len()))
            }
            SYS_EXIT => {
                self.exit(caller, frame.arg0 as i32)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_BOOT_REPORT => {
                let report = BootSessionReport {
                    status: frame.arg0 as u32,
                    stage: frame.arg1 as u32,
                    code: frame.arg2 as i32,
                    reserved: 0,
                    detail: frame.arg3 as u64,
                };
                self.processes.record_session_report(caller, report)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_LIST_PROCESSES => {
                let ids = self
                    .process_list()
                    .into_iter()
                    .map(|info| info.pid.raw())
                    .collect::<Vec<_>>();
                if let Err(result) =
                    copy_u64_slice_to_user(self, caller, frame.arg0, frame.arg1, &ids)
                {
                    return Ok(result);
                }
                Ok(SyscallReturn::ok(ids.len()))
            }
            SYS_INSPECT_PROCESS => {
                let pid = frame_pid(frame.arg0)?;
                let info = self.process_info(pid)?;
                let record = NativeProcessRecord {
                    pid: info.pid.raw(),
                    parent: info.parent.map(|pid| pid.raw()).unwrap_or(0),
                    address_space: info.address_space.map(|id| id.raw()).unwrap_or(0),
                    main_thread: info.main_thread.map(|id| id.raw()).unwrap_or(0),
                    state: encode_native_process_state(info.state),
                    exit_code: info.exit_code.unwrap_or(0),
                    descriptor_count: info.descriptor_count as u64,
                    capability_count: info.capability_count as u64,
                    environment_count: info.environment_count as u64,
                    memory_region_count: info.memory_region_count as u64,
                    thread_count: info.thread_count as u64,
                    pending_signal_count: info.pending_signals.len() as u64,
                    session_reported: info.session_reported as u32,
                    session_status: info.session_status,
                    session_stage: info.session_stage,
                    scheduler_class: encode_native_scheduler_class(info.scheduler_policy.class)
                        as u32,
                    scheduler_budget: info.scheduler_policy.budget,
                    cpu_runtime_ticks: info.cpu_runtime_ticks,
                    execution_contract: info
                        .contract_bindings
                        .execution
                        .map(|id| id.raw())
                        .unwrap_or(0),
                    memory_contract: info
                        .contract_bindings
                        .memory
                        .map(|id| id.raw())
                        .unwrap_or(0),
                    io_contract: info.contract_bindings.io.map(|id| id.raw()).unwrap_or(0),
                    observe_contract: info
                        .contract_bindings
                        .observe
                        .map(|id| id.raw())
                        .unwrap_or(0),
                    reserved: 0,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_PROCESS_COMPAT => {
                let pid = frame_pid(frame.arg0)?;
                let info = self.process_info(pid)?;
                let abi = info.abi_profile;
                let mut record = NativeProcessCompatRecord {
                    pid: info.pid.raw(),
                    target: [0; 16],
                    route_class: [0; 32],
                    handle_profile: [0; 32],
                    path_profile: [0; 32],
                    scheduler_profile: [0; 32],
                    sync_profile: [0; 32],
                    timer_profile: [0; 32],
                    module_profile: [0; 32],
                    event_profile: [0; 32],
                    requires_kernel_abi_shims: if abi.requires_kernel_abi_shims { 1 } else { 0 },
                    prefix: [0; 64],
                    executable_path: [0; 64],
                    working_dir: [0; 64],
                    loader_route_class: [0; 32],
                    loader_launch_mode: [0; 32],
                    loader_entry_profile: [0; 32],
                    loader_requires_compat_shims: if abi.loader_requires_compat_shims {
                        1
                    } else {
                        0
                    },
                };
                copy_text_field(&mut record.target, &abi.target);
                copy_text_field(&mut record.route_class, &abi.route_class);
                copy_text_field(&mut record.handle_profile, &abi.handle_profile);
                copy_text_field(&mut record.path_profile, &abi.path_profile);
                copy_text_field(&mut record.scheduler_profile, &abi.scheduler_profile);
                copy_text_field(&mut record.sync_profile, &abi.sync_profile);
                copy_text_field(&mut record.timer_profile, &abi.timer_profile);
                copy_text_field(&mut record.module_profile, &abi.module_profile);
                copy_text_field(&mut record.event_profile, &abi.event_profile);
                copy_text_field(&mut record.prefix, &abi.prefix);
                copy_text_field(&mut record.executable_path, &abi.executable_path);
                copy_text_field(&mut record.working_dir, &abi.working_dir);
                copy_text_field(&mut record.loader_route_class, &abi.loader_route_class);
                copy_text_field(&mut record.loader_launch_mode, &abi.loader_launch_mode);
                copy_text_field(&mut record.loader_entry_profile, &abi.loader_entry_profile);
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_LOAD_MEMORY_WORD => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                let value = self.load_memory_word(pid, frame.arg1 as u64)?;
                Ok(SyscallReturn::ok(value as usize))
            }
            SYS_STORE_MEMORY_WORD => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                self.store_memory_word(pid, frame.arg1 as u64, frame.arg2 as u32)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_MAP_ANONYMOUS_MEMORY => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                let label = match frame_string(self, caller, frame.arg3, frame.arg4) {
                    Ok(label) => label,
                    Err(error) => return Ok(error),
                };
                let perms = frame.arg2 as u64;
                let start = self.map_anonymous_memory(
                    pid,
                    frame.arg1 as u64,
                    (perms & 0x1) != 0,
                    (perms & 0x2) != 0,
                    (perms & 0x4) != 0,
                    label,
                )?;
                Ok(SyscallReturn::ok(start as usize))
            }
            SYS_MAP_FILE_MEMORY => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                let path = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let flags = frame.arg5 as u64;
                let start = self.map_file_memory(
                    pid,
                    path,
                    frame.arg3 as u64,
                    frame.arg4 as u64,
                    (flags & 0x1) != 0,
                    (flags & 0x2) != 0,
                    (flags & 0x4) != 0,
                    (flags & 0x8) != 0,
                )?;
                Ok(SyscallReturn::ok(start as usize))
            }
            SYS_QUARANTINE_VM_OBJECT => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                self.quarantine_vm_object(pid, frame.arg1 as u64, frame.arg2 as u64)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_RELEASE_VM_OBJECT => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                self.release_vm_object_quarantine(pid, frame.arg1 as u64)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SYNC_MEMORY_RANGE => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                self.sync_memory(pid, frame.arg1 as u64, frame.arg2 as u64)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_ADVISE_MEMORY_RANGE => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                let advice = match frame.arg3 as u64 {
                    0 => MemoryAdvice::Normal,
                    1 => MemoryAdvice::Sequential,
                    2 => MemoryAdvice::Random,
                    3 => MemoryAdvice::WillNeed,
                    4 => MemoryAdvice::DontNeed,
                    _ => return Ok(SyscallReturn::err(Errno::Inval)),
                };
                self.advise_memory(pid, frame.arg1 as u64, frame.arg2 as u64, advice)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_PROTECT_MEMORY_RANGE => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                self.protect_memory(
                    pid,
                    frame.arg1 as u64,
                    frame.arg2 as u64,
                    frame.arg3 != 0,
                    frame.arg4 != 0,
                    frame.arg5 != 0,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_UNMAP_MEMORY_RANGE => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                self.unmap_memory(pid, frame.arg1 as u64, frame.arg2 as u64)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SET_PROCESS_BREAK => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                let end = self.set_process_break(pid, frame.arg1 as u64)?;
                Ok(SyscallReturn::ok(end as usize))
            }
            SYS_RECLAIM_MEMORY_PRESSURE => {
                let pid = frame_pid(frame.arg0)?;
                if pid != caller {
                    return Ok(SyscallReturn::err(Errno::Access));
                }
                let reclaimed = self.reclaim_memory_pressure(pid, frame.arg1 as u64)?;
                Ok(SyscallReturn::ok(reclaimed as usize))
            }
            SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL => {
                let reclaimed = self.reclaim_memory_pressure_global(frame.arg0 as u64)?;
                Ok(SyscallReturn::ok(reclaimed as usize))
            }
            SYS_INSPECT_SYSTEM_SNAPSHOT => {
                let snapshot = self.snapshot();
                let record = NativeSystemSnapshotRecord {
                    current_tick: snapshot.current_tick,
                    busy_ticks: snapshot.busy_ticks,
                    process_count: snapshot.process_count as u64,
                    active_process_count: snapshot.active_process_count as u64,
                    blocked_process_count: snapshot.blocked_process_count as u64,
                    queued_processes: snapshot.queued_processes as u64,
                    queued_latency_critical: snapshot.queued_latency_critical as u64,
                    queued_interactive: snapshot.queued_interactive as u64,
                    queued_normal: snapshot.queued_normal as u64,
                    queued_background: snapshot.queued_background as u64,
                    queued_urgent_latency_critical: snapshot.queued_urgent_latency_critical as u64,
                    queued_urgent_interactive: snapshot.queued_urgent_interactive as u64,
                    queued_urgent_normal: snapshot.queued_urgent_normal as u64,
                    queued_urgent_background: snapshot.queued_urgent_background as u64,
                    lag_debt_latency_critical: snapshot.lag_debt_latency_critical as i64,
                    lag_debt_interactive: snapshot.lag_debt_interactive as i64,
                    lag_debt_normal: snapshot.lag_debt_normal as i64,
                    lag_debt_background: snapshot.lag_debt_background as i64,
                    dispatch_count_latency_critical: snapshot.dispatch_count_latency_critical,
                    dispatch_count_interactive: snapshot.dispatch_count_interactive,
                    dispatch_count_normal: snapshot.dispatch_count_normal,
                    dispatch_count_background: snapshot.dispatch_count_background,
                    runtime_ticks_latency_critical: snapshot.runtime_ticks_latency_critical,
                    runtime_ticks_interactive: snapshot.runtime_ticks_interactive,
                    runtime_ticks_normal: snapshot.runtime_ticks_normal,
                    runtime_ticks_background: snapshot.runtime_ticks_background,
                    scheduler_cpu_count: snapshot.scheduler_cpu_count as u64,
                    scheduler_running_cpu: snapshot
                        .scheduler_running_cpu
                        .map(|cpu| cpu as u64)
                        .unwrap_or(u64::MAX),
                    scheduler_cpu_load_imbalance: snapshot.scheduler_cpu_load_imbalance as u64,
                    starved_latency_critical: u64::from(snapshot.starved_latency_critical),
                    starved_interactive: u64::from(snapshot.starved_interactive),
                    starved_normal: u64::from(snapshot.starved_normal),
                    starved_background: u64::from(snapshot.starved_background),
                    deferred_task_count: snapshot.deferred_task_count as u64,
                    sleeping_processes: snapshot.sleeping_processes as u64,
                    total_event_queue_count: snapshot.total_event_queue_count as u64,
                    total_event_queue_pending: snapshot.total_event_queue_pending as u64,
                    total_event_queue_waiters: snapshot.total_event_queue_waiters as u64,
                    total_socket_count: snapshot.total_socket_count as u64,
                    saturated_socket_count: snapshot.saturated_socket_count as u64,
                    total_socket_rx_depth: snapshot.total_socket_rx_depth as u64,
                    total_socket_rx_limit: snapshot.total_socket_rx_limit as u64,
                    max_socket_rx_depth: snapshot.max_socket_rx_depth as u64,
                    total_network_tx_dropped: snapshot.total_network_tx_dropped,
                    total_network_rx_dropped: snapshot.total_network_rx_dropped,
                    running_pid: snapshot.running.map(|pid| pid.raw()).unwrap_or(0),
                    reserved0: if snapshot.verified_core_ok {
                        NativeSystemSnapshotRecord::VERIFIED_CORE_OK_TRUE
                    } else {
                        NativeSystemSnapshotRecord::VERIFIED_CORE_OK_FALSE
                    },
                    reserved1: snapshot.verified_core_violation_count as u64,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg0, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_DEVICE => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let info = self.device_info_by_path(&device_path)?;
                let record = NativeDeviceRecord {
                    class: info.class as u32,
                    state: info.state as u32,
                    reserved0: if info.class == DeviceClass::Graphics {
                        u64::from(
                            self.device_registry
                                .devices
                                .iter()
                                .find(|device| device.path == device_path)
                                .is_some_and(|device| device.graphics_control_reserve_armed),
                        )
                    } else {
                        0
                    },
                    queue_depth: info.queue_depth as u64,
                    queue_capacity: info.queue_capacity as u64,
                    submitted_requests: info.submitted_requests,
                    completed_requests: info.completed_requests,
                    total_latency_ticks: info.total_latency_ticks,
                    max_latency_ticks: info.max_latency_ticks,
                    total_queue_wait_ticks: info.total_queue_wait_ticks,
                    max_queue_wait_ticks: info.max_queue_wait_ticks,
                    link_up: info.link_up as u32,
                    reserved1: 0,
                    block_size: info.block_size,
                    reserved2: 0,
                    capacity_bytes: info.capacity_bytes,
                    last_completed_request_id: info.last_completed_request_id,
                    last_completed_frame_tag: [0; 64],
                    last_completed_source_api_name: [0; 24],
                    last_completed_translation_label: [0; 32],
                    last_terminal_request_id: info.last_terminal_request_id,
                    last_terminal_state: info.last_terminal_state as u32,
                    reserved3: 0,
                    last_terminal_frame_tag: [0; 64],
                    last_terminal_source_api_name: [0; 24],
                    last_terminal_translation_label: [0; 32],
                };
                let mut record = record;
                copy_text_field(
                    &mut record.last_completed_frame_tag,
                    &info.last_completed_frame_tag,
                );
                copy_text_field(
                    &mut record.last_completed_source_api_name,
                    &info.last_completed_source_api_name,
                );
                copy_text_field(
                    &mut record.last_completed_translation_label,
                    &info.last_completed_translation_label,
                );
                copy_text_field(
                    &mut record.last_terminal_frame_tag,
                    &info.last_terminal_frame_tag,
                );
                copy_text_field(
                    &mut record.last_terminal_source_api_name,
                    &info.last_terminal_source_api_name,
                );
                copy_text_field(
                    &mut record.last_terminal_translation_label,
                    &info.last_terminal_translation_label,
                );
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_DEVICE_REQUEST => {
                let info = self.device_request_info(frame.arg0 as u64)?;
                let record = NativeDeviceRequestRecord {
                    issuer: info.issuer.raw(),
                    kind: info.kind as u32,
                    state: info.state as u32,
                    opcode: info.opcode.unwrap_or(0) as u64,
                    buffer_id: info.graphics_buffer_id.unwrap_or(0),
                    payload_len: info.payload_len as u64,
                    response_len: info.response_len as u64,
                    submitted_tick: info.submitted_tick,
                    started_tick: info.started_tick.unwrap_or(0),
                    completed_tick: info.completed_tick.unwrap_or(0),
                    frame_tag: [0; 64],
                    source_api_name: [0; 24],
                    translation_label: [0; 32],
                };
                let mut record = record;
                copy_text_field(&mut record.frame_tag, &info.frame_tag);
                copy_text_field(&mut record.source_api_name, &info.source_api_name);
                copy_text_field(&mut record.translation_label, &info.translation_label);
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_CREATE_GPU_BUFFER => {
                let buffer_id = self.create_graphics_buffer(caller, frame.arg0)?;
                Ok(SyscallReturn::ok(buffer_id as usize))
            }
            SYS_WRITE_GPU_BUFFER => {
                let bytes = match self.copy_from_user(caller, frame.arg2, frame.arg3) {
                    Ok(bytes) => bytes,
                    Err(error) => return Ok(SyscallReturn::err(error.errno())),
                };
                let written =
                    self.write_graphics_buffer(caller, frame.arg0 as u64, frame.arg1, &bytes)?;
                Ok(SyscallReturn::ok(written))
            }
            SYS_INSPECT_GPU_BUFFER => {
                let info = self.graphics_buffer_info(frame.arg0 as u64)?;
                let record = NativeGpuBufferRecord {
                    owner: info.owner.raw(),
                    length: info.length as u64,
                    used_len: info.used_len as u64,
                    reserved: 0,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_SUBMIT_GPU_BUFFER => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let written =
                    self.submit_graphics_buffer(caller, &device_path, frame.arg2 as u64)?;
                Ok(SyscallReturn::ok(written))
            }
            SYS_PRESENT_GPU_FRAME => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let frame_bytes = match self.copy_from_user(caller, frame.arg2, frame.arg3) {
                    Ok(bytes) => bytes,
                    Err(error) => return Ok(SyscallReturn::err(error.errno())),
                };
                let response = self.present_graphics_frame(caller, &device_path, &frame_bytes)?;
                Ok(SyscallReturn::ok(response as usize))
            }
            SYS_INSPECT_GPU_SCANOUT => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let info = self.graphics_scanout_info(&device_path)?;
                let mut last_frame_tag = [0u8; 64];
                copy_text_field(&mut last_frame_tag, &info.last_frame_tag);
                let mut last_source_api_name = [0u8; 24];
                copy_text_field(&mut last_source_api_name, &info.last_source_api_name);
                let mut last_translation_label = [0u8; 32];
                copy_text_field(&mut last_translation_label, &info.last_translation_label);
                let record = NativeGpuScanoutRecord {
                    presented_frames: info.presented_frames,
                    last_frame_len: info.last_frame_len as u64,
                    last_frame_tag,
                    last_source_api_name,
                    last_translation_label,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_BINDING => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuBindingRecord {
                    present: 0,
                    msi_supported: 0,
                    msi_message_limit: 0,
                    resizable_bar_enabled: 0,
                    subsystem_id: 0,
                    bar1_total_mib: 0,
                    framebuffer_total_mib: 0,
                    display_engine_confirmed: 0,
                    architecture_name: [0; 32],
                    product_name: [0; 64],
                    die_name: [0; 16],
                    bus_interface: [0; 32],
                    inf_section: [0; 32],
                    kernel_service: [0; 32],
                    vbios_version: [0; 32],
                    part_number: [0; 32],
                    msi_source_name: [0; 32],
                };
                if let Some(info) = self.graphics_binding_evidence(&device_path)? {
                    record.present = 1;
                    record.msi_supported = u32::from(info.msi_policy.supported);
                    record.msi_message_limit = info.msi_policy.message_limit;
                    record.resizable_bar_enabled = u32::from(info.resizable_bar_enabled);
                    record.subsystem_id = info.subsystem_id;
                    record.bar1_total_mib = info.bar1_total_mib;
                    record.framebuffer_total_mib = info.framebuffer_total_mib;
                    record.display_engine_confirmed = u32::from(info.display_engine_confirmed);
                    copy_text_field(&mut record.architecture_name, info.architecture_name);
                    copy_text_field(&mut record.product_name, info.product_name);
                    copy_text_field(&mut record.die_name, info.die_name);
                    copy_text_field(&mut record.bus_interface, info.bus_interface);
                    copy_text_field(&mut record.inf_section, info.inf_section);
                    copy_text_field(&mut record.kernel_service, info.kernel_service);
                    copy_text_field(&mut record.vbios_version, info.vbios_version);
                    copy_text_field(&mut record.part_number, info.part_number);
                    copy_text_field(&mut record.msi_source_name, info.msi_policy.source_name);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_VBIOS => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuVbiosRecord {
                    present: 0,
                    enabled: 0,
                    vendor_id: 0,
                    rom_bar_raw: 0,
                    device_id: 0,
                    physical_base: 0,
                    image_len: 0,
                    header_len: 0,
                    pcir_offset: 0,
                    bit_offset: 0,
                    nvfw_offset: 0,
                    header: [0; 16],
                    board_name: [0; 64],
                    board_code: [0; 32],
                    version: [0; 32],
                };
                if let Some(window) = self.graphics_vbios_window(&device_path)? {
                    record.present = 1;
                    record.enabled = u32::from(window.enabled);
                    record.rom_bar_raw = window.rom_bar_raw;
                    record.physical_base = window.physical_base;
                    if let Ok(bytes) = self.read_graphics_vbios(&device_path, record.header.len()) {
                        record.image_len = bytes.len() as u64;
                        record.header_len = bytes.len() as u32;
                        record.header[..bytes.len()].copy_from_slice(&bytes);
                    }
                }
                if let Some(info) = self.graphics_vbios_image_evidence(&device_path)? {
                    record.vendor_id = u32::from(info.vendor_id);
                    record.device_id = u32::from(info.device_id);
                    record.pcir_offset = info.pcir_offset as u32;
                    record.bit_offset = info.bit_offset.unwrap_or(0) as u32;
                    record.nvfw_offset = info.nvfw_offset.unwrap_or(0) as u32;
                    copy_text_field(&mut record.board_name, &info.board_name);
                    copy_text_field(&mut record.board_code, &info.board_code);
                    copy_text_field(&mut record.version, &info.version);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_GSP => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuGspRecord {
                    present: 0,
                    loopback_ready: 0,
                    firmware_known: 0,
                    blackwell_blob_present: 0,
                    hardware_ready: 0,
                    driver_model_wddm: 0,
                    loopback_completions: 0,
                    loopback_failures: 0,
                    firmware_version: [0; 16],
                    blob_summary: [0; 48],
                    refusal_reason: [0; 48],
                };
                if let Some(info) = self.graphics_gsp_evidence(&device_path)? {
                    record.present = u32::from(info.present);
                    record.loopback_ready = u32::from(info.loopback_ready);
                    record.firmware_known = u32::from(info.firmware_known);
                    record.blackwell_blob_present = u32::from(info.blackwell_blob_present);
                    record.loopback_completions = info.loopback_completions;
                    record.loopback_failures = info.loopback_failures;
                    copy_text_field(&mut record.firmware_version, info.firmware_version);
                    copy_text_field(&mut record.blob_summary, info.blob_summary);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_INTERRUPT => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuInterruptRecord {
                    present: 0,
                    vector: 0,
                    delivered_count: 0,
                    msi_supported: 0,
                    message_limit: 0,
                    windows_interrupt_message_maximum: 0,
                    hardware_servicing_confirmed: 0,
                };
                if let Some(info) = self.graphics_interrupt_evidence(&device_path)? {
                    record.present = u32::from(info.present);
                    record.vector = u32::from(info.vector);
                    record.delivered_count = info.delivered_count;
                    record.msi_supported = u32::from(info.msi_supported);
                    record.message_limit = info.message_limit;
                    record.windows_interrupt_message_maximum =
                        info.windows_interrupt_message_maximum;
                    record.hardware_servicing_confirmed =
                        u32::from(info.hardware_servicing_confirmed);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_DISPLAY => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuDisplayRecord {
                    present: 0,
                    active_pipes: 0,
                    planned_frames: 0,
                    last_present_offset: 0,
                    last_present_len: 0,
                    hardware_programming_confirmed: 0,
                };
                if let Some(info) = self.graphics_display_evidence(&device_path)? {
                    record.present = u32::from(info.present);
                    record.active_pipes = info.active_pipes;
                    record.planned_frames = info.planned_frames;
                    record.last_present_offset = info.last_present_offset;
                    record.last_present_len = info.last_present_len;
                    record.hardware_programming_confirmed =
                        u32::from(info.hardware_programming_confirmed);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_POWER => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuPowerRecord {
                    present: 0,
                    pstate: 0,
                    graphics_clock_mhz: 0,
                    memory_clock_mhz: 0,
                    boost_clock_mhz: 0,
                    hardware_power_management_confirmed: 0,
                };
                if let Some(info) = self.graphics_power_evidence(&device_path)? {
                    record.present = u32::from(info.present);
                    record.pstate = info.pstate;
                    record.graphics_clock_mhz = info.graphics_clock_mhz;
                    record.memory_clock_mhz = info.memory_clock_mhz;
                    record.boost_clock_mhz = info.boost_clock_mhz;
                    record.hardware_power_management_confirmed =
                        u32::from(info.hardware_power_management_confirmed);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_SET_GPU_POWER_STATE => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let pstate = frame.arg2 as u32;
                match pstate {
                    0 | 5 | 8 | 12 => {}
                    _ => return Ok(SyscallReturn::err(Errno::Inval)),
                }
                self.graphics_set_power_state(&device_path, pstate)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_MEDIA => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuMediaRecord::default();
                if let Some(info) = self.graphics_media_evidence(&device_path)? {
                    record.present = u32::from(info.present);
                    record.sessions = info.sessions;
                    record.codec = info.codec;
                    record.width = info.width;
                    record.height = info.height;
                    record.bitrate_kbps = info.bitrate_kbps;
                    record.hardware_media_confirmed = u32::from(info.hardware_media_confirmed);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_START_GPU_MEDIA_SESSION => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let width = frame.arg2 as u32;
                let height = frame.arg3 as u32;
                let bitrate_kbps = frame.arg4 as u32;
                let codec = frame.arg5 as u32;
                if width == 0 || height == 0 || bitrate_kbps == 0 || codec > 2 {
                    return Ok(SyscallReturn::err(Errno::Inval));
                }
                self.graphics_start_media_session(
                    &device_path,
                    width,
                    height,
                    bitrate_kbps,
                    codec,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_NEURAL => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuNeuralRecord::default();
                if let Some(info) = self.graphics_neural_evidence(&device_path)? {
                    record.present = u32::from(info.present);
                    record.model_loaded = u32::from(info.model_loaded);
                    record.active_semantics = info.active_semantics;
                    record.last_commit_completed = u32::from(info.last_commit_completed);
                    record.hardware_neural_confirmed = u32::from(info.hardware_neural_confirmed);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INJECT_GPU_NEURAL_SEMANTIC => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let semantic_label = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                if semantic_label.is_empty() {
                    return Ok(SyscallReturn::err(Errno::Inval));
                }
                self.graphics_inject_neural_semantic(&device_path, &semantic_label)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_COMMIT_GPU_NEURAL_FRAME => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.graphics_commit_neural_frame(&device_path)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_GPU_TENSOR => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let mut record = NativeGpuTensorRecord::default();
                if let Some(info) = self.graphics_tensor_evidence(&device_path)? {
                    record.present = u32::from(info.present);
                    record.active_jobs = info.active_jobs;
                    record.last_kernel_id = info.last_kernel_id;
                    record.hardware_tensor_confirmed = u32::from(info.hardware_tensor_confirmed);
                }
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_DISPATCH_GPU_TENSOR_KERNEL => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let kernel_id = frame.arg2 as u32;
                if kernel_id == 0 {
                    return Ok(SyscallReturn::err(Errno::Inval));
                }
                self.graphics_dispatch_tensor_kernel(&device_path, kernel_id)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_READ_GPU_SCANOUT_FRAME => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let bytes = self.read_graphics_scanout_frame(&device_path, frame.arg3)?;
                if let Err(error) = self.copy_to_user(caller, frame.arg2, &bytes) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(bytes.len()))
            }
            SYS_INSPECT_DRIVER => {
                let driver_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let info = self.driver_info_by_path(&driver_path)?;
                let record = NativeDriverRecord {
                    state: info.state as u32,
                    reserved: 0,
                    bound_device_count: info.bound_devices.len() as u64,
                    queued_requests: info.queued_requests as u64,
                    in_flight_requests: info.in_flight_requests as u64,
                    completed_requests: info.completed_requests,
                    last_completed_request_id: info.last_completed_request_id,
                    last_completed_frame_tag: [0; 64],
                    last_completed_source_api_name: [0; 24],
                    last_completed_translation_label: [0; 32],
                    last_terminal_request_id: info.last_terminal_request_id,
                    last_terminal_state: info.last_terminal_state as u32,
                    reserved1: 0,
                    last_terminal_frame_tag: [0; 64],
                    last_terminal_source_api_name: [0; 24],
                    last_terminal_translation_label: [0; 32],
                };
                let mut record = record;
                copy_text_field(
                    &mut record.last_completed_frame_tag,
                    &info.last_completed_frame_tag,
                );
                copy_text_field(
                    &mut record.last_completed_source_api_name,
                    &info.last_completed_source_api_name,
                );
                copy_text_field(
                    &mut record.last_completed_translation_label,
                    &info.last_completed_translation_label,
                );
                copy_text_field(
                    &mut record.last_terminal_frame_tag,
                    &info.last_terminal_frame_tag,
                );
                copy_text_field(
                    &mut record.last_terminal_source_api_name,
                    &info.last_terminal_source_api_name,
                );
                copy_text_field(
                    &mut record.last_terminal_translation_label,
                    &info.last_terminal_translation_label,
                );
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_GET_PROCESS_NAME => {
                let pid = frame_pid(frame.arg0)?;
                let info = self.process_info(pid)?;
                let copied = copy_string_to_user(self, caller, frame.arg1, frame.arg2, &info.name)?;
                Ok(SyscallReturn::ok(copied))
            }
            SYS_GET_PROCESS_IMAGE_PATH => {
                let pid = frame_pid(frame.arg0)?;
                let info = self.process_info(pid)?;
                let copied =
                    copy_string_to_user(self, caller, frame.arg1, frame.arg2, &info.image_path)?;
                Ok(SyscallReturn::ok(copied))
            }
            SYS_GET_PROCESS_CWD => {
                let pid = frame_pid(frame.arg0)?;
                let info = self.process_info(pid)?;
                let copied = copy_string_to_user(self, caller, frame.arg1, frame.arg2, &info.cwd)?;
                Ok(SyscallReturn::ok(copied))
            }
            SYS_GET_PROCESS_ROOT => {
                let pid = frame_pid(frame.arg0)?;
                let info = self.process_info(pid)?;
                let copied = copy_string_to_user(self, caller, frame.arg1, frame.arg2, &info.root)?;
                Ok(SyscallReturn::ok(copied))
            }
            SYS_CHDIR_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.set_process_cwd(caller, path)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SEND_SIGNAL => {
                let pid = frame_pid(frame.arg0)?;
                let signal = u8::try_from(frame.arg1)
                    .map_err(|_| RuntimeError::from(ProcessError::InvalidSignal))?;
                let sender_tid = self
                    .processes
                    .get(caller)?
                    .main_thread()
                    .ok_or(RuntimeError::from(ProcessError::InvalidTid))?;
                self.send_signal(
                    PendingSignalSender {
                        pid: caller,
                        tid: sender_tid,
                    },
                    pid,
                    signal,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_PENDING_SIGNALS => {
                let pid = frame_pid(frame.arg0)?;
                let signals = self.pending_signals(pid)?;
                if let Err(result) =
                    copy_signal_slice_to_user(self, caller, frame.arg1, frame.arg2, &signals)
                {
                    return Ok(result);
                }
                Ok(SyscallReturn::ok(signals.len()))
            }
            SYS_BLOCKED_PENDING_SIGNALS => {
                let pid = frame_pid(frame.arg0)?;
                let signals = self.blocked_pending_signals(pid)?;
                if let Err(result) =
                    copy_signal_slice_to_user(self, caller, frame.arg1, frame.arg2, &signals)
                {
                    return Ok(result);
                }
                Ok(SyscallReturn::ok(signals.len()))
            }
            SYS_SPAWN_PATH_PROCESS => {
                let name = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(name) => name,
                    Err(error) => return Ok(error),
                };
                let path = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let pid =
                    self.spawn_process(name.clone(), Some(caller), SchedulerClass::Interactive)?;
                self.set_process_args(pid, vec![name])?;
                self.exec_process(pid, path, vec![], vec![])?;
                Ok(SyscallReturn::ok(pid.raw() as usize))
            }
            SYS_SPAWN_CONFIGURED_PROCESS => {
                let config = match copy_struct_from_user::<NativeSpawnProcessConfig>(
                    self, caller, frame.arg0,
                ) {
                    Ok(config) => config,
                    Err(error) => return Ok(error),
                };
                let name = match frame_string(self, caller, config.name_ptr, config.name_len) {
                    Ok(name) => name,
                    Err(error) => return Ok(error),
                };
                let path = match frame_string(self, caller, config.path_ptr, config.path_len) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let cwd = match frame_string(self, caller, config.cwd_ptr, config.cwd_len) {
                    Ok(cwd) => cwd,
                    Err(error) => return Ok(error),
                };
                let argv = match frame_string_table(
                    self,
                    caller,
                    config.argv_ptr,
                    config.argv_len,
                    config.argv_count,
                ) {
                    Ok(argv) => argv,
                    Err(error) => return Ok(error),
                };
                let envp = match frame_string_table(
                    self,
                    caller,
                    config.envp_ptr,
                    config.envp_len,
                    config.envp_count,
                ) {
                    Ok(envp) => envp,
                    Err(error) => return Ok(error),
                };
                let pid =
                    self.spawn_process(name.clone(), Some(caller), SchedulerClass::Interactive)?;
                self.set_process_cwd(pid, cwd)?;
                self.exec_process(pid, path, argv, envp)?;
                Ok(SyscallReturn::ok(pid.raw() as usize))
            }
            SYS_SPAWN_PROCESS_COPY_VM => {
                let name = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(name) => name,
                    Err(error) => return Ok(error),
                };
                let path = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let source = match frame_pid(frame.arg4) {
                    Ok(pid) => pid,
                    Err(error) => return Ok(SyscallReturn::err(map_runtime_error_to_errno(error))),
                };
                let pid = self.spawn_process_copy_vm(
                    name.clone(),
                    Some(caller),
                    SchedulerClass::Interactive,
                    source,
                )?;
                self.set_process_args(pid, vec![name])?;
                self.exec_process(pid, path, vec![], vec![])?;
                Ok(SyscallReturn::ok(pid.raw() as usize))
            }
            SYS_SET_PROCESS_ARGS => {
                let pid = frame_pid(frame.arg0)?;
                let argv =
                    match frame_string_table(self, caller, frame.arg1, frame.arg2, frame.arg3) {
                        Ok(argv) => argv,
                        Err(error) => return Ok(error),
                    };
                self.set_process_args(pid, argv)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SET_PROCESS_ENV => {
                let pid = frame_pid(frame.arg0)?;
                let envp =
                    match frame_string_table(self, caller, frame.arg1, frame.arg2, frame.arg3) {
                        Ok(envp) => envp,
                        Err(error) => return Ok(error),
                    };
                self.set_process_env(pid, envp)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SET_PROCESS_CWD => {
                let pid = frame_pid(frame.arg0)?;
                let cwd = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(cwd) => cwd,
                    Err(error) => return Ok(error),
                };
                self.set_process_cwd(pid, cwd)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SET_PROCESS_ROOT => {
                let pid = frame_pid(frame.arg0)?;
                let root = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(root) => root,
                    Err(error) => return Ok(error),
                };
                self.set_process_root(pid, root)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_REAP_PROCESS => {
                let pid = frame_pid(frame.arg0)?;
                let process = self.reap_process(pid)?;
                Ok(SyscallReturn::ok(process.exit_code().unwrap_or(0) as usize))
            }
            SYS_READ_PROCFS => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let bytes = self.read_procfs_path_for(caller, &path)?;
                let count = frame.arg3.min(bytes.len());
                if let Err(error) = self.copy_to_user(caller, frame.arg2, &bytes[..count]) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(count))
            }
            SYS_STAT_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let status = self.stat_path(&path)?;
                let record = NativeFileStatusRecord {
                    inode: status.inode,
                    link_count: 1,
                    size: status.size,
                    kind: encode_native_object_kind(status.kind) as u32,
                    cloexec: status.cloexec as u32,
                    nonblock: status.nonblock as u32,
                    readable: status.readable as u32,
                    writable: status.writable as u32,
                    executable: 0,
                    owner_uid: 0,
                    group_gid: 0,
                    mode: ((status.readable as u32) * 0o444) | ((status.writable as u32) * 0o222),
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_LSTAT_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let status = self.lstat_path(&path)?;
                let record = NativeFileStatusRecord {
                    inode: status.inode,
                    link_count: 1,
                    size: status.size,
                    kind: encode_native_object_kind(status.kind) as u32,
                    cloexec: status.cloexec as u32,
                    nonblock: status.nonblock as u32,
                    readable: status.readable as u32,
                    writable: status.writable as u32,
                    executable: 0,
                    owner_uid: 0,
                    group_gid: 0,
                    mode: ((status.readable as u32) * 0o444) | ((status.writable as u32) * 0o222),
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_STATFS_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let status = self.statfs(&path)?;
                let record = NativeFileSystemStatusRecord {
                    mount_count: status.mount_count as u64,
                    node_count: status.node_count as u64,
                    read_only: status.read_only as u32,
                    reserved: 0,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_OPEN_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let fd = self.open_path(caller, &path)?;
                Ok(SyscallReturn::ok(fd.raw() as usize))
            }
            SYS_MKDIR_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.create_owned_vfs_node(caller, path, ObjectKind::Directory)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_MKFILE_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.create_owned_vfs_node(caller, path, ObjectKind::File)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_MKSOCK_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.create_owned_vfs_node(caller, path, ObjectKind::Socket)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_MKCHAN_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.create_owned_vfs_node(caller, path, ObjectKind::Channel)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SYMLINK_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let target = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.create_owned_vfs_symlink(caller, path, target)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_RENAME_PATH => {
                let from = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let to = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.rename_path(&from, &to)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_UNLINK_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.unlink_path(&path)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_READLINK_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let target = self.readlink_path(&path)?;
                let bytes = target.as_bytes();
                let count = frame.arg3.min(bytes.len());
                if let Err(error) = self.copy_to_user(caller, frame.arg2, &bytes[..count]) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(count))
            }
            SYS_LIST_PATH => {
                let path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let bytes = self.list_path(&path)?;
                let count = frame.arg3.min(bytes.len());
                if let Err(error) = self.copy_to_user(caller, frame.arg2, &bytes[..count]) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(count))
            }
            SYS_CONFIGURE_NETIF_IPV4 => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let config: NativeNetworkInterfaceConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.configure_network_interface_ipv4(
                    &device_path,
                    config.addr,
                    config.netmask,
                    config.gateway,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_BIND_UDP_SOCKET => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let device_path = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let config: NativeUdpBindConfig =
                    match copy_struct_from_user(self, caller, frame.arg4) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.bind_udp_socket(
                    &socket_path,
                    caller,
                    &device_path,
                    config.local_port,
                    config.remote_ipv4,
                    config.remote_port,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_NETIF => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let info = self.network_interface_info(&device_path)?;
                let record = NativeNetworkInterfaceRecord {
                    admin_up: info.admin_up as u32,
                    link_up: info.link_up as u32,
                    promiscuous: info.promiscuous as u32,
                    reserved: 0,
                    mtu: info.mtu as u64,
                    tx_capacity: info.tx_capacity as u64,
                    rx_capacity: info.rx_capacity as u64,
                    tx_inflight_limit: info.tx_inflight_limit as u64,
                    tx_inflight_depth: info.tx_inflight_depth as u64,
                    free_buffer_count: info.free_buffer_count as u64,
                    mac: info.mac,
                    mac_reserved: [0; 2],
                    ipv4_addr: info.ipv4_addr,
                    ipv4_netmask: info.ipv4_netmask,
                    ipv4_gateway: info.ipv4_gateway,
                    ipv4_reserved: [0; 4],
                    rx_ring_depth: info.rx_ring_depth as u64,
                    tx_ring_depth: info.tx_ring_depth as u64,
                    tx_packets: info.tx_packets,
                    rx_packets: info.rx_packets,
                    tx_completions: info.tx_completions,
                    tx_dropped: info.tx_dropped,
                    rx_dropped: info.rx_dropped,
                    attached_socket_count: info.attached_sockets.len() as u64,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_INSPECT_NETSOCK => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let info = self.network_socket_info(&socket_path)?;
                let record = NativeNetworkSocketRecord {
                    local_ipv4: info.local_ipv4,
                    remote_ipv4: info.remote_ipv4,
                    local_port: info.local_port,
                    remote_port: info.remote_port,
                    connected: info.connected as u32,
                    reserved: 0,
                    rx_depth: info.rx_depth as u64,
                    rx_queue_limit: info.rx_queue_limit as u64,
                    tx_packets: info.tx_packets,
                    rx_packets: info.rx_packets,
                    dropped_packets: info.dropped_packets,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg2, &record) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(0))
            }
            SYS_SET_NETIF_LINK_STATE => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let config: NativeNetworkLinkStateConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.set_network_interface_link_state(&device_path, config.link_up != 0)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_CONFIGURE_NETIF_ADMIN => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let config: NativeNetworkAdminConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.configure_network_interface_admin(
                    &device_path,
                    config.admin_up != 0,
                    config.promiscuous != 0,
                    config.mtu as usize,
                    config.tx_capacity as usize,
                    config.rx_capacity as usize,
                    config.tx_inflight_limit as usize,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_CONFIGURE_DEVICE_QUEUE => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.configure_device_queue(&device_path, frame.arg2)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_BIND_DEVICE_DRIVER => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let driver_path = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.bind_device_to_driver(&device_path, &driver_path)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_UNBIND_DEVICE_DRIVER => {
                let device_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.unbind_device_driver(&device_path)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_CONNECT_UDP_SOCKET => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let config: NativeUdpConnectConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.connect_udp_socket(
                    &socket_path,
                    caller,
                    config.remote_ipv4,
                    config.remote_port,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SENDTO_UDP_SOCKET => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let config: NativeUdpSendToConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                let bytes = match self.copy_from_user(caller, frame.arg3, frame.arg4) {
                    Ok(bytes) => bytes,
                    Err(error) => return Ok(SyscallReturn::err(error.errno())),
                };
                let written = self.send_udp_socket_to(
                    &socket_path,
                    caller,
                    config.remote_ipv4,
                    config.remote_port,
                    &bytes,
                )?;
                Ok(SyscallReturn::ok(written))
            }
            SYS_RECVFROM_UDP_SOCKET => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let (payload, remote_ipv4, remote_port) =
                    self.recv_udp_socket_from(&socket_path, caller, frame.arg3)?;
                if let Err(error) = self.copy_to_user(caller, frame.arg2, &payload) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                let meta = NativeUdpRecvMeta {
                    remote_ipv4,
                    remote_port,
                    reserved: 0,
                };
                if let Err(error) = copy_struct_to_user(self, caller, frame.arg4, &meta) {
                    return Ok(error);
                }
                Ok(SyscallReturn::ok(payload.len()))
            }
            SYS_COMPLETE_NET_TX => {
                let driver_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let count = self.complete_network_tx(&driver_path, frame.arg2)?;
                Ok(SyscallReturn::ok(count))
            }
            SYS_TCP_LISTEN => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let device_path = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let local_port = frame.arg4 as u16;
                let backlog = frame.arg5;
                self.tcp_listen(&socket_path, caller, &device_path, local_port, backlog)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_TCP_CONNECT => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let remote_ipv4 = [
                    ((frame.arg2 >> 24) & 0xff) as u8,
                    ((frame.arg2 >> 16) & 0xff) as u8,
                    ((frame.arg2 >> 8) & 0xff) as u8,
                    (frame.arg2 & 0xff) as u8,
                ];
                let remote_port = frame.arg3 as u16;
                self.tcp_connect(&socket_path, caller, remote_ipv4, remote_port, self.current_tick)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_TCP_ACCEPT => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let (accepted_path, remote_ipv4, remote_port) =
                    self.tcp_accept(&socket_path, caller, self.current_tick)?;
                if let Err(error) = self.copy_to_user(caller, frame.arg2, accepted_path.as_bytes()) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                let ipv4_u32 = u32::from_be_bytes(remote_ipv4);
                let result = ((remote_port as usize) << 32) | (ipv4_u32 as usize);
                Ok(SyscallReturn::ok(result))
            }
            SYS_TCP_SEND => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let buffer = match self.copy_from_user(caller, frame.arg2, frame.arg3) {
                    Ok(bytes) => bytes,
                    Err(error) => return Ok(SyscallReturn::err(error.errno())),
                };
                let count = self.tcp_send(&socket_path, caller, &buffer, self.current_tick)?;
                Ok(SyscallReturn::ok(count))
            }
            SYS_TCP_RECV => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let data = self.tcp_recv(&socket_path, caller, frame.arg3, self.current_tick)?;
                if let Err(error) = self.copy_to_user(caller, frame.arg2, &data) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(data.len()))
            }
            SYS_TCP_CLOSE => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.tcp_close(&socket_path, caller, self.current_tick)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_TCP_RESET => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                self.tcp_send_reset(&socket_path, caller, self.current_tick)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_ICMP_ECHO_REQUEST => {
                let socket_path = match frame_string(self, caller, frame.arg0, frame.arg1) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let target_ipv4 = [
                    ((frame.arg2 >> 24) & 0xff) as u8,
                    ((frame.arg2 >> 16) & 0xff) as u8,
                    ((frame.arg2 >> 8) & 0xff) as u8,
                    (frame.arg2 & 0xff) as u8,
                ];
                let identifier = (frame.arg3 >> 16) as u16;
                let sequence = (frame.arg3 & 0xffff) as u16;
                let count = frame.arg4;
                let payload = b"ngos icmp echo request payload data 0123456789ABCDEF";
                self.icmp_echo_request(
                    &socket_path,
                    caller,
                    target_ipv4,
                    identifier,
                    sequence,
                    payload,
                    self.current_tick,
                )?;
                Ok(SyscallReturn::ok(count))
            }
            SYS_CPU_ONLINE => {
                let cpu = frame.arg0;
                self.set_cpu_online(cpu, true)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_CPU_OFFLINE => {
                let cpu = frame.arg0;
                self.set_cpu_online(cpu, false)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_CPU_INFO => {
                let logical_count = self.logical_cpu_count();
                let online_count = self.cpu_online_count();
                let result = ((online_count as usize) << 32) | (logical_count as usize);
                Ok(SyscallReturn::ok(result))
            }
            SYS_CREATE_EVENT_QUEUE => {
                let mode = match NativeEventQueueMode::from_raw(frame.arg0 as u32) {
                    Some(NativeEventQueueMode::Kqueue) => EventQueueMode::Kqueue,
                    Some(NativeEventQueueMode::Epoll) => EventQueueMode::Epoll,
                    None => return Ok(SyscallReturn::err(Errno::Inval)),
                };
                let fd = self.create_event_queue_descriptor(caller, mode)?;
                Ok(SyscallReturn::ok(fd.raw() as usize))
            }
            SYS_WAIT_EVENT_QUEUE => {
                let queue_fd = frame_fd(frame.arg0)?;
                let tid = self
                    .processes
                    .get(caller)?
                    .main_thread()
                    .ok_or(RuntimeError::from(ProcessError::InvalidTid))?;
                let events = match self.wait_event_queue_descriptor(caller, queue_fd, tid)? {
                    EventQueueWaitResult::Ready(events) => events,
                    EventQueueWaitResult::Blocked(_) => {
                        return Ok(SyscallReturn::err(Errno::Again));
                    }
                };
                let count = frame.arg2.min(events.len());
                let records = events
                    .into_iter()
                    .take(count)
                    .map(encode_native_event_record)
                    .collect::<Vec<_>>();
                let bytes = records.len() * core::mem::size_of::<NativeEventRecord>();
                if let Err(error) = self.copy_to_user(caller, frame.arg1, unsafe {
                    core::slice::from_raw_parts(records.as_ptr() as *const u8, bytes)
                }) {
                    return Ok(SyscallReturn::err(error.errno()));
                }
                Ok(SyscallReturn::ok(records.len()))
            }
            SYS_WATCH_PROCESS_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let pid = frame_pid(frame.arg1)?;
                let config: NativeProcessEventWatchConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.watch_process_events_descriptor(
                    caller,
                    queue_fd,
                    pid,
                    config.token,
                    ProcessLifecycleInterest {
                        exited: config.exited != 0,
                        reaped: config.reaped != 0,
                    },
                    decode_iopoll_events(config.poll_events),
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_REMOVE_PROCESS_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let pid = frame_pid(frame.arg1)?;
                self.remove_process_events_descriptor(caller, queue_fd, pid, frame.arg2 as u64)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_WATCH_RESOURCE_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let resource = self.find_resource_id_by_raw(frame.arg1 as u64)?;
                let config: NativeResourceEventWatchConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.watch_resource_events_descriptor(
                    caller,
                    queue_fd,
                    resource,
                    config.token,
                    ResourceEventInterest {
                        claimed: config.claimed != 0,
                        queued: config.queued != 0,
                        canceled: config.canceled != 0,
                        released: config.released != 0,
                        handed_off: config.handed_off != 0,
                        revoked: config.revoked != 0,
                    },
                    decode_iopoll_events(config.poll_events),
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_REMOVE_RESOURCE_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let resource = self.find_resource_id_by_raw(frame.arg1 as u64)?;
                self.remove_resource_events_descriptor(
                    caller,
                    queue_fd,
                    resource,
                    frame.arg2 as u64,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_WATCH_BUS_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let endpoint = self.find_bus_endpoint_id_by_raw(frame.arg1 as u64)?;
                let config: NativeBusEventWatchConfig =
                    match copy_struct_from_user(self, caller, frame.arg2) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                self.watch_bus_events_descriptor(
                    caller,
                    queue_fd,
                    endpoint,
                    config.token,
                    BusEventInterest {
                        attached: config.attached != 0,
                        detached: config.detached != 0,
                        published: config.published != 0,
                        received: config.received != 0,
                    },
                    decode_iopoll_events(config.poll_events),
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_WATCH_NET_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let interface_path = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let socket_path = if frame.arg3 == 0 || frame.arg4 == 0 {
                    None
                } else {
                    match frame_string(self, caller, frame.arg3, frame.arg4) {
                        Ok(path) => Some(path),
                        Err(error) => return Ok(error),
                    }
                };
                let config: NativeNetworkEventWatchConfig =
                    match copy_struct_from_user(self, caller, frame.arg5) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                let interface_inode = self.stat_path(&interface_path)?.inode;
                let socket_inode = socket_path
                    .as_deref()
                    .map(|path| self.stat_path(path).map(|status| status.inode))
                    .transpose()?;
                self.watch_network_events_descriptor(
                    caller,
                    queue_fd,
                    interface_inode,
                    socket_inode,
                    config.token,
                    NetworkEventInterest {
                        link_changed: config.link_changed != 0,
                        rx_ready: config.rx_ready != 0,
                        tx_drained: config.tx_drained != 0,
                    },
                    decode_iopoll_events(config.poll_events),
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_WATCH_GRAPHICS_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let device_path = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let config: NativeGraphicsEventWatchConfig =
                    match copy_struct_from_user(self, caller, frame.arg3) {
                        Ok(config) => config,
                        Err(error) => return Ok(error),
                    };
                let device_inode = self.stat_path(&device_path)?.inode;
                self.watch_graphics_events_descriptor(
                    caller,
                    queue_fd,
                    device_inode,
                    config.token,
                    GraphicsEventInterest {
                        submitted: config.submitted != 0,
                        completed: config.completed != 0,
                        failed: config.failed != 0,
                        drained: config.drained != 0,
                        canceled: config.canceled != 0,
                        faulted: config.faulted != 0,
                        recovered: config.recovered != 0,
                        retired: config.retired != 0,
                        lease_released: config.lease_released != 0,
                        lease_acquired: config.lease_acquired != 0,
                    },
                    decode_iopoll_events(config.poll_events),
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_REMOVE_NET_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let interface_path = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let socket_path = if frame.arg3 == 0 || frame.arg4 == 0 {
                    None
                } else {
                    match frame_string(self, caller, frame.arg3, frame.arg4) {
                        Ok(path) => Some(path),
                        Err(error) => return Ok(error),
                    }
                };
                let interface_inode = self.stat_path(&interface_path)?.inode;
                let socket_inode = socket_path
                    .as_deref()
                    .map(|path| self.stat_path(path).map(|status| status.inode))
                    .transpose()?;
                self.remove_network_events_descriptor(
                    caller,
                    queue_fd,
                    interface_inode,
                    socket_inode,
                    frame.arg5 as u64,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_REMOVE_BUS_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let endpoint = self.find_bus_endpoint_id_by_raw(frame.arg1 as u64)?;
                self.remove_bus_events_descriptor(caller, queue_fd, endpoint, frame.arg2 as u64)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_REMOVE_GRAPHICS_EVENTS => {
                let queue_fd = frame_fd(frame.arg0)?;
                let device_path = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(path) => path,
                    Err(error) => return Ok(error),
                };
                let device_inode = self.stat_path(&device_path)?.inode;
                self.remove_graphics_events_descriptor(
                    caller,
                    queue_fd,
                    device_inode,
                    frame.arg3 as u64,
                )?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_PAUSE_PROCESS => {
                let pid = frame_pid(frame.arg0)?;
                self.pause_process(pid)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_RESUME_PROCESS => {
                let pid = frame_pid(frame.arg0)?;
                self.resume_process(pid)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_RENICE_PROCESS => {
                let pid = frame_pid(frame.arg0)?;
                let class = match NativeSchedulerClass::from_raw(frame.arg1 as u32) {
                    Some(class) => decode_native_scheduler_class(class),
                    None => return Ok(SyscallReturn::err(Errno::Inval)),
                };
                self.renice_process(pid, class, frame.arg2 as u32)?;
                Ok(SyscallReturn::ok(0))
            }
            SYS_SET_PROCESS_AFFINITY => {
                let pid = frame_pid(frame.arg0)?;
                self.set_process_affinity(pid, frame.arg1 as u64)?;
                Ok(SyscallReturn::ok(0))
            }
            _ => Ok(SyscallReturn::err(Errno::Inval)),
        }
    }
}

fn decode_iopoll_events(raw: u32) -> IoPollEvents {
    let mut events = IoPollEvents::empty();
    if raw & IOPOLL_READABLE != 0 {
        events = events | IoPollEvents::READABLE;
    }
    if raw & IOPOLL_WRITABLE != 0 {
        events = events | IoPollEvents::WRITABLE;
    }
    if raw & IOPOLL_PRIORITY != 0 {
        events = events | IoPollEvents::PRIORITY;
    }
    if raw & IOPOLL_HANGUP != 0 {
        events = events | IoPollEvents::HANGUP;
    }
    events
}

fn encode_native_scheduler_class(class: SchedulerClass) -> NativeSchedulerClass {
    match class {
        SchedulerClass::LatencyCritical => NativeSchedulerClass::LatencyCritical,
        SchedulerClass::Interactive => NativeSchedulerClass::Interactive,
        SchedulerClass::BestEffort => NativeSchedulerClass::BestEffort,
        SchedulerClass::Background => NativeSchedulerClass::Background,
    }
}

fn decode_native_scheduler_class(class: NativeSchedulerClass) -> SchedulerClass {
    match class {
        NativeSchedulerClass::LatencyCritical => SchedulerClass::LatencyCritical,
        NativeSchedulerClass::Interactive => SchedulerClass::Interactive,
        NativeSchedulerClass::BestEffort => SchedulerClass::BestEffort,
        NativeSchedulerClass::Background => SchedulerClass::Background,
    }
}

fn encode_native_event_record(event: EventRecord) -> NativeEventRecord {
    match event.source {
        EventSource::Descriptor(fd) => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Descriptor as u32,
            source_arg0: fd.raw() as u64,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        },
        EventSource::Timer(timer) => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Timer as u32,
            source_arg0: timer.raw(),
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        },
        EventSource::Process { pid, kind } => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Process as u32,
            source_arg0: pid.raw(),
            source_arg1: 0,
            source_arg2: 0,
            detail0: match kind {
                ProcessLifecycleEventKind::Exited => 0,
                ProcessLifecycleEventKind::Reaped => 1,
            },
            detail1: 0,
        },
        EventSource::Signal { pid, tid, signal } => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Signal as u32,
            source_arg0: pid.raw(),
            source_arg1: tid.map(ThreadId::raw).unwrap_or(0),
            source_arg2: signal as u64,
            detail0: 0,
            detail1: 0,
        },
        EventSource::MemoryWait { domain, addr, kind } => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::MemoryWait as u32,
            source_arg0: match domain {
                MemoryWaitDomain::Shared => 0,
                MemoryWaitDomain::Process(pid) => pid.raw(),
            },
            source_arg1: addr,
            source_arg2: 0,
            detail0: match domain {
                MemoryWaitDomain::Shared => 0,
                MemoryWaitDomain::Process(_) => 1,
            },
            detail1: match kind {
                MemoryWaitEventKind::Woken => 0,
                MemoryWaitEventKind::Requeued => 1,
            },
        },
        EventSource::Resource {
            resource,
            contract,
            kind,
        } => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Resource as u32,
            source_arg0: resource.raw(),
            source_arg1: contract.raw(),
            source_arg2: 0,
            detail0: match kind {
                ResourceEventKind::Claimed => 0,
                ResourceEventKind::Queued => 1,
                ResourceEventKind::Canceled => 2,
                ResourceEventKind::Released => 3,
                ResourceEventKind::HandedOff => 4,
                ResourceEventKind::Revoked => 5,
            },
            detail1: 0,
        },
        EventSource::Network {
            interface_inode,
            socket_inode,
            kind,
        } => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Network as u32,
            source_arg0: interface_inode,
            source_arg1: socket_inode.unwrap_or(0),
            source_arg2: 0,
            detail0: socket_inode.is_some() as u32,
            detail1: match kind {
                NetworkEventKind::LinkChanged => NativeNetworkEventKind::LinkChanged as u32,
                NetworkEventKind::RxReady => NativeNetworkEventKind::RxReady as u32,
                NetworkEventKind::TxDrained => NativeNetworkEventKind::TxDrained as u32,
            },
        },
        EventSource::Graphics {
            device_inode,
            request_id,
            kind,
        } => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Graphics as u32,
            source_arg0: device_inode,
            source_arg1: request_id,
            source_arg2: 0,
            detail0: 0,
            detail1: match kind {
                GraphicsEventKind::Submitted => NativeGraphicsEventKind::Submitted as u32,
                GraphicsEventKind::Completed => NativeGraphicsEventKind::Completed as u32,
                GraphicsEventKind::Failed => NativeGraphicsEventKind::Failed as u32,
                GraphicsEventKind::Drained => NativeGraphicsEventKind::Drained as u32,
                GraphicsEventKind::Canceled => NativeGraphicsEventKind::Canceled as u32,
                GraphicsEventKind::Faulted => NativeGraphicsEventKind::Faulted as u32,
                GraphicsEventKind::Recovered => NativeGraphicsEventKind::Recovered as u32,
                GraphicsEventKind::Retired => NativeGraphicsEventKind::Retired as u32,
                GraphicsEventKind::LeaseReleased => NativeGraphicsEventKind::LeaseReleased as u32,
                GraphicsEventKind::LeaseAcquired => NativeGraphicsEventKind::LeaseAcquired as u32,
            },
        },
        EventSource::Bus {
            peer,
            endpoint,
            kind,
        } => NativeEventRecord {
            token: event.token,
            events: event.events.0,
            source_kind: NativeEventSourceKind::Bus as u32,
            source_arg0: peer.raw(),
            source_arg1: endpoint.raw(),
            source_arg2: 0,
            detail0: match kind {
                BusEventKind::Attached => 0,
                BusEventKind::Detached => 1,
                BusEventKind::Published => 2,
                BusEventKind::Received => 3,
            },
            detail1: 0,
        },
    }
}
