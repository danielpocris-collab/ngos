use kernel_core::{KernelRuntime, ProcessId};
use std::{cell::RefCell, slice};
use user_abi::{
    SYS_BIND_DEVICE_DRIVER, SYS_BIND_UDP_SOCKET, SYS_BLOCKED_PENDING_SIGNALS, SYS_BOOT_REPORT,
    SYS_CANCEL_RESOURCE_CLAIM, SYS_CHDIR_PATH, SYS_CLAIM_RESOURCE, SYS_COLLECT_READINESS,
    SYS_COMMIT_GPU_NEURAL_FRAME, SYS_COMPLETE_NET_TX, SYS_CONFIGURE_DEVICE_QUEUE,
    SYS_CONFIGURE_NETIF_ADMIN, SYS_CONFIGURE_NETIF_IPV4, SYS_CONNECT_UDP_SOCKET,
    SYS_CREATE_CONTRACT, SYS_CREATE_DOMAIN, SYS_CREATE_EVENT_QUEUE, SYS_CREATE_GPU_BUFFER,
    SYS_CREATE_RESOURCE, SYS_DISPATCH_GPU_TENSOR_KERNEL, SYS_GET_CONTRACT_LABEL,
    SYS_GET_DOMAIN_NAME, SYS_GET_PROCESS_CWD, SYS_GET_PROCESS_IMAGE_PATH, SYS_GET_PROCESS_NAME,
    SYS_GET_RESOURCE_NAME, SYS_INJECT_GPU_NEURAL_SEMANTIC, SYS_INSPECT_CONTRACT,
    SYS_INSPECT_DEVICE, SYS_INSPECT_DEVICE_REQUEST, SYS_INSPECT_DOMAIN, SYS_INSPECT_DRIVER,
    SYS_INSPECT_GPU_BINDING, SYS_INSPECT_GPU_BUFFER, SYS_INSPECT_GPU_DISPLAY, SYS_INSPECT_GPU_GSP,
    SYS_INSPECT_GPU_INTERRUPT, SYS_INSPECT_GPU_MEDIA, SYS_INSPECT_GPU_NEURAL,
    SYS_INSPECT_GPU_POWER, SYS_INSPECT_GPU_SCANOUT, SYS_INSPECT_GPU_TENSOR, SYS_INSPECT_GPU_VBIOS,
    SYS_INSPECT_NETIF, SYS_INSPECT_NETSOCK, SYS_INSPECT_PROCESS, SYS_INSPECT_RESOURCE,
    SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS, SYS_LIST_PATH, SYS_LIST_PROCESSES,
    SYS_LIST_RESOURCE_WAITERS, SYS_LIST_RESOURCES, SYS_LSTAT_PATH, SYS_MAP_ANONYMOUS_MEMORY,
    SYS_MKCHAN_PATH, SYS_MKDIR_PATH, SYS_MKFILE_PATH, SYS_MKSOCK_PATH, SYS_OPEN_PATH,
    SYS_PENDING_SIGNALS, SYS_POLL, SYS_PRESENT_GPU_FRAME, SYS_READ, SYS_READ_GPU_SCANOUT_FRAME,
    SYS_READ_PROCFS, SYS_READLINK_PATH, SYS_READV, SYS_RECVFROM_UDP_SOCKET, SYS_REGISTER_READINESS,
    SYS_RELEASE_CLAIMED_RESOURCE, SYS_REMOVE_GRAPHICS_EVENTS, SYS_REMOVE_NET_EVENTS,
    SYS_RENAME_PATH, SYS_SEND_SIGNAL, SYS_SENDTO_UDP_SOCKET, SYS_SET_GPU_POWER_STATE,
    SYS_SET_NETIF_LINK_STATE, SYS_SET_PROCESS_ARGS, SYS_SET_PROCESS_CWD, SYS_SET_PROCESS_ENV,
    SYS_SPAWN_CONFIGURED_PROCESS, SYS_SPAWN_PATH_PROCESS, SYS_START_GPU_MEDIA_SESSION,
    SYS_STAT_PATH, SYS_STATFS_PATH, SYS_SUBMIT_GPU_BUFFER, SYS_SYMLINK_PATH,
    SYS_UNBIND_DEVICE_DRIVER, SYS_UNLINK_PATH, SYS_WAIT_EVENT_QUEUE, SYS_WATCH_GRAPHICS_EVENTS,
    SYS_WATCH_NET_EVENTS, SYS_WATCH_RESOURCE_EVENTS, SYS_WRITE, SYS_WRITE_GPU_BUFFER, SYS_WRITEV,
    SyscallBackend, SyscallFrame, SyscallReturn, UserIoVec,
};

pub struct HostRuntimeKernelBackend {
    runtime: RefCell<KernelRuntime>,
    pid: ProcessId,
    scratch_base: usize,
    scratch_len: usize,
}

impl HostRuntimeKernelBackend {
    pub fn new(
        runtime: KernelRuntime,
        pid: ProcessId,
        scratch_base: usize,
        scratch_len: usize,
    ) -> Self {
        Self {
            runtime: RefCell::new(runtime),
            pid,
            scratch_base,
            scratch_len,
        }
    }

    pub fn runtime_mut(&self) -> std::cell::RefMut<'_, KernelRuntime> {
        self.runtime.borrow_mut()
    }
}

impl SyscallBackend for HostRuntimeKernelBackend {
    unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
        let mut runtime = self.runtime.borrow_mut();
        let mut scratch =
            ScratchSpace::new(&mut runtime, self.pid, self.scratch_base, self.scratch_len);
        match scratch.dispatch(frame) {
            Ok(result) => result,
            Err(errno) => SyscallReturn::err(errno),
        }
    }
}

struct ScratchSpace<'a> {
    runtime: &'a mut KernelRuntime,
    pid: ProcessId,
    base: usize,
    limit: usize,
    cursor: usize,
    post: Vec<PostCopy>,
}

enum PostCopy {
    Buffer {
        user_ptr: usize,
        host_ptr: usize,
        len: usize,
        copy_len: CopyLen,
    },
    Readv {
        iovs: Vec<(usize, usize, usize)>,
        total_len: usize,
    },
}

enum CopyLen {
    Exact(usize),
    ResultBytes,
    ResultU64s,
    ResultRecords(usize),
}

impl<'a> ScratchSpace<'a> {
    fn new(runtime: &'a mut KernelRuntime, pid: ProcessId, base: usize, limit: usize) -> Self {
        Self {
            runtime,
            pid,
            base,
            limit,
            cursor: 0,
            post: Vec::new(),
        }
    }

    fn marshal_frame(&mut self, frame: SyscallFrame) -> Result<SyscallFrame, user_abi::Errno> {
        let mut args = [
            frame.arg0, frame.arg1, frame.arg2, frame.arg3, frame.arg4, frame.arg5,
        ];
        match frame.number {
            SYS_WRITE => {
                args[1] = self.copy_in(args[1], args[2])?;
            }
            SYS_READ => {
                let user_ptr = self.alloc(args[2], 1)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[1],
                    len: args[2],
                    copy_len: CopyLen::ResultBytes,
                });
                args[1] = user_ptr;
            }
            SYS_WRITEV => {
                args[1] = self.marshal_writev(args[1], args[2])?;
            }
            SYS_READV => {
                args[1] = self.marshal_readv(args[1], args[2])?;
            }
            SYS_OPEN_PATH | SYS_MKDIR_PATH | SYS_MKFILE_PATH | SYS_MKSOCK_PATH
            | SYS_MKCHAN_PATH | SYS_UNLINK_PATH | SYS_CHDIR_PATH => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_CONFIGURE_DEVICE_QUEUE => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_CREATE_GPU_BUFFER => {}
            SYS_WRITE_GPU_BUFFER => {
                args[2] = self.copy_in(args[2], args[3])?;
            }
            SYS_INSPECT_GPU_BUFFER => {
                args[1] = self.marshal_record_out::<user_abi::NativeGpuBufferRecord>(args[1])?;
            }
            SYS_SUBMIT_GPU_BUFFER => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_PRESENT_GPU_FRAME => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(args[2], args[3])?;
            }
            SYS_BIND_DEVICE_DRIVER => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(args[2], args[3])?;
            }
            SYS_UNBIND_DEVICE_DRIVER => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_LIST_PATH | SYS_READLINK_PATH | SYS_READ_PROCFS => {
                args[0] = self.copy_in(args[0], args[1])?;
                let user_ptr = self.alloc(args[3], 1)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[2],
                    len: args[3],
                    copy_len: CopyLen::ResultBytes,
                });
                args[2] = user_ptr;
            }
            SYS_STAT_PATH | SYS_LSTAT_PATH | SYS_STATFS_PATH => {
                args[0] = self.copy_in(args[0], args[1])?;
                let size = if frame.number == SYS_STATFS_PATH {
                    std::mem::size_of::<user_abi::NativeFileSystemStatusRecord>()
                } else {
                    std::mem::size_of::<user_abi::NativeFileStatusRecord>()
                };
                let user_ptr = self.alloc(size, 8)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[2],
                    len: size,
                    copy_len: CopyLen::Exact(size),
                });
                args[2] = user_ptr;
            }
            SYS_SYMLINK_PATH | SYS_RENAME_PATH | SYS_SPAWN_PATH_PROCESS => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(args[2], args[3])?;
            }
            SYS_SPAWN_CONFIGURED_PROCESS => {
                let host_config = unsafe {
                    (args[0] as *const user_abi::NativeSpawnProcessConfig).read_unaligned()
                };
                let scratch_config = user_abi::NativeSpawnProcessConfig {
                    name_ptr: self.copy_in(host_config.name_ptr, host_config.name_len)?,
                    name_len: host_config.name_len,
                    path_ptr: self.copy_in(host_config.path_ptr, host_config.path_len)?,
                    path_len: host_config.path_len,
                    cwd_ptr: self.copy_in(host_config.cwd_ptr, host_config.cwd_len)?,
                    cwd_len: host_config.cwd_len,
                    argv_ptr: self.copy_in(host_config.argv_ptr, host_config.argv_len)?,
                    argv_len: host_config.argv_len,
                    argv_count: host_config.argv_count,
                    envp_ptr: self.copy_in(host_config.envp_ptr, host_config.envp_len)?,
                    envp_len: host_config.envp_len,
                    envp_count: host_config.envp_count,
                };
                args[0] = self.copy_structs_in(&[scratch_config])?;
            }
            SYS_SET_PROCESS_ARGS | SYS_SET_PROCESS_ENV => {
                args[1] = self.copy_in(args[1], args[2])?;
            }
            SYS_SET_PROCESS_CWD => {
                args[1] = self.copy_in(args[1], args[2])?;
            }
            SYS_MAP_ANONYMOUS_MEMORY => {
                args[3] = self.copy_in(args[3], args[4])?;
            }
            SYS_LIST_PROCESSES | SYS_LIST_DOMAINS | SYS_LIST_RESOURCES | SYS_LIST_CONTRACTS => {
                let user_ptr = self.alloc(args[1] * std::mem::size_of::<u64>(), 8)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[0],
                    len: args[1] * std::mem::size_of::<u64>(),
                    copy_len: CopyLen::ResultU64s,
                });
                args[0] = user_ptr;
            }
            SYS_PENDING_SIGNALS
            | SYS_BLOCKED_PENDING_SIGNALS
            | SYS_GET_PROCESS_NAME
            | SYS_GET_PROCESS_IMAGE_PATH
            | SYS_GET_PROCESS_CWD
            | SYS_GET_DOMAIN_NAME
            | SYS_GET_RESOURCE_NAME
            | SYS_GET_CONTRACT_LABEL => {
                let user_ptr = self.alloc(args[2], 1)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[1],
                    len: args[2],
                    copy_len: CopyLen::ResultBytes,
                });
                args[1] = user_ptr;
            }
            SYS_INSPECT_PROCESS => {
                args[1] = self.marshal_record_out::<user_abi::NativeProcessRecord>(args[1])?;
            }
            SYS_INSPECT_DOMAIN => {
                args[1] = self.marshal_record_out::<user_abi::NativeDomainRecord>(args[1])?;
            }
            SYS_INSPECT_RESOURCE => {
                args[1] = self.marshal_record_out::<user_abi::NativeResourceRecord>(args[1])?;
            }
            SYS_INSPECT_CONTRACT => {
                args[1] = self.marshal_record_out::<user_abi::NativeContractRecord>(args[1])?;
            }
            SYS_INSPECT_NETIF => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] =
                    self.marshal_record_out::<user_abi::NativeNetworkInterfaceRecord>(args[2])?;
            }
            SYS_INSPECT_NETSOCK => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] =
                    self.marshal_record_out::<user_abi::NativeNetworkSocketRecord>(args[2])?;
            }
            SYS_INSPECT_DEVICE => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeDeviceRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_BINDING => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuBindingRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_VBIOS => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuVbiosRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_GSP => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuGspRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_INTERRUPT => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuInterruptRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_DISPLAY => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuDisplayRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_POWER => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuPowerRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_MEDIA => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuMediaRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_NEURAL => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuNeuralRecord>(args[2])?;
            }
            SYS_INSPECT_GPU_TENSOR => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuTensorRecord>(args[2])?;
            }
            SYS_SET_GPU_POWER_STATE => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_START_GPU_MEDIA_SESSION => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_INJECT_GPU_NEURAL_SEMANTIC => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(args[2], args[3])?;
            }
            SYS_COMMIT_GPU_NEURAL_FRAME => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_DISPATCH_GPU_TENSOR_KERNEL => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_INSPECT_GPU_SCANOUT => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeGpuScanoutRecord>(args[2])?;
            }
            SYS_INSPECT_DEVICE_REQUEST => {
                args[1] =
                    self.marshal_record_out::<user_abi::NativeDeviceRequestRecord>(args[1])?;
            }
            SYS_INSPECT_DRIVER => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.marshal_record_out::<user_abi::NativeDriverRecord>(args[2])?;
            }
            SYS_CREATE_DOMAIN => {
                args[1] = self.copy_in(args[1], args[2])?;
            }
            SYS_CREATE_RESOURCE => {
                args[2] = self.copy_in(args[2], args[3])?;
            }
            SYS_CREATE_CONTRACT => {
                args[3] = self.copy_in(args[3], args[4])?;
            }
            SYS_LIST_RESOURCE_WAITERS => {
                let user_ptr = self.alloc(args[2] * std::mem::size_of::<u64>(), 8)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[1],
                    len: args[2] * std::mem::size_of::<u64>(),
                    copy_len: CopyLen::ResultU64s,
                });
                args[1] = user_ptr;
            }
            SYS_CLAIM_RESOURCE => {
                args[1] =
                    self.marshal_record_out::<user_abi::NativeResourceClaimRecord>(args[1])?;
            }
            SYS_RELEASE_CLAIMED_RESOURCE => {
                args[1] =
                    self.marshal_record_out::<user_abi::NativeResourceReleaseRecord>(args[1])?;
            }
            SYS_CANCEL_RESOURCE_CLAIM => {
                args[1] =
                    self.marshal_record_out::<user_abi::NativeResourceCancelRecord>(args[1])?;
            }
            SYS_CONFIGURE_NETIF_IPV4 => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(
                    args[2],
                    std::mem::size_of::<user_abi::NativeNetworkInterfaceConfig>(),
                )?;
            }
            SYS_BIND_UDP_SOCKET => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(args[2], args[3])?;
                args[4] = self.copy_in(
                    args[4],
                    std::mem::size_of::<user_abi::NativeUdpBindConfig>(),
                )?;
            }
            SYS_SET_NETIF_LINK_STATE => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(
                    args[2],
                    std::mem::size_of::<user_abi::NativeNetworkLinkStateConfig>(),
                )?;
            }
            SYS_CONFIGURE_NETIF_ADMIN => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(
                    args[2],
                    std::mem::size_of::<user_abi::NativeNetworkAdminConfig>(),
                )?;
            }
            SYS_CONNECT_UDP_SOCKET => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(
                    args[2],
                    std::mem::size_of::<user_abi::NativeUdpConnectConfig>(),
                )?;
            }
            SYS_SENDTO_UDP_SOCKET => {
                args[0] = self.copy_in(args[0], args[1])?;
                args[2] = self.copy_in(
                    args[2],
                    std::mem::size_of::<user_abi::NativeUdpSendToConfig>(),
                )?;
                args[3] = self.copy_in(args[3], args[4])?;
            }
            SYS_RECVFROM_UDP_SOCKET => {
                args[0] = self.copy_in(args[0], args[1])?;
                let user_ptr = self.alloc(args[3], 1)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[2],
                    len: args[3],
                    copy_len: CopyLen::ResultBytes,
                });
                args[2] = user_ptr;
                args[4] = self.marshal_record_out::<user_abi::NativeUdpRecvMeta>(args[4])?;
            }
            SYS_READ_GPU_SCANOUT_FRAME => {
                args[0] = self.copy_in(args[0], args[1])?;
                let user_ptr = self.alloc(args[3], 1)?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[2],
                    len: args[3],
                    copy_len: CopyLen::ResultBytes,
                });
                args[2] = user_ptr;
            }
            SYS_COMPLETE_NET_TX => {
                args[0] = self.copy_in(args[0], args[1])?;
            }
            SYS_CREATE_EVENT_QUEUE => {}
            SYS_WAIT_EVENT_QUEUE => {
                let user_ptr = self.alloc(
                    args[2] * std::mem::size_of::<user_abi::NativeEventRecord>(),
                    8,
                )?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[1],
                    len: args[2] * std::mem::size_of::<user_abi::NativeEventRecord>(),
                    copy_len: CopyLen::ResultRecords(std::mem::size_of::<
                        user_abi::NativeEventRecord,
                    >()),
                });
                args[1] = user_ptr;
            }
            SYS_COLLECT_READINESS => {
                let user_ptr = self.alloc(
                    args[1] * std::mem::size_of::<user_abi::NativeReadinessRecord>(),
                    8,
                )?;
                self.post.push(PostCopy::Buffer {
                    user_ptr,
                    host_ptr: args[0],
                    len: args[1] * std::mem::size_of::<user_abi::NativeReadinessRecord>(),
                    copy_len: CopyLen::ResultRecords(std::mem::size_of::<
                        user_abi::NativeReadinessRecord,
                    >()),
                });
                args[0] = user_ptr;
            }
            SYS_WATCH_NET_EVENTS => {
                args[1] = self.copy_in(args[1], args[2])?;
                if args[3] != 0 && args[4] != 0 {
                    args[3] = self.copy_in(args[3], args[4])?;
                }
                args[5] = self.copy_in(
                    args[5],
                    std::mem::size_of::<user_abi::NativeNetworkEventWatchConfig>(),
                )?;
            }
            SYS_WATCH_GRAPHICS_EVENTS => {
                args[1] = self.copy_in(args[1], args[2])?;
                args[3] = self.copy_in(
                    args[3],
                    std::mem::size_of::<user_abi::NativeGraphicsEventWatchConfig>(),
                )?;
            }
            SYS_WATCH_RESOURCE_EVENTS => {
                args[2] = self.copy_in(
                    args[2],
                    std::mem::size_of::<user_abi::NativeResourceEventWatchConfig>(),
                )?;
            }
            SYS_REGISTER_READINESS => {}
            SYS_REMOVE_NET_EVENTS | SYS_REMOVE_GRAPHICS_EVENTS => {
                args[1] = self.copy_in(args[1], args[2])?;
                if args[3] != 0 && args[4] != 0 {
                    args[3] = self.copy_in(args[3], args[4])?;
                }
            }
            SYS_BOOT_REPORT | SYS_SEND_SIGNAL | SYS_POLL => {}
            _ => {}
        }
        Ok(SyscallFrame::new(frame.number, args))
    }

    fn dispatch(&mut self, frame: SyscallFrame) -> Result<SyscallReturn, user_abi::Errno> {
        let frame = self.marshal_frame(frame)?;
        let result = self.runtime.dispatch_user_syscall_frame(self.pid, frame);
        self.finish(result)
    }

    fn finish(&mut self, result: SyscallReturn) -> Result<SyscallReturn, user_abi::Errno> {
        let Ok(value) = result.into_result() else {
            return Ok(result);
        };
        for post in &self.post {
            match *post {
                PostCopy::Buffer {
                    user_ptr,
                    host_ptr,
                    len,
                    ref copy_len,
                } => {
                    let bytes = match copy_len {
                        CopyLen::Exact(exact) => *exact,
                        CopyLen::ResultBytes => value.min(len),
                        CopyLen::ResultU64s => {
                            value.saturating_mul(std::mem::size_of::<u64>()).min(len)
                        }
                        CopyLen::ResultRecords(record_size) => {
                            value.saturating_mul(*record_size).min(len)
                        }
                    };
                    let copied = self
                        .runtime
                        .copy_from_user(self.pid, user_ptr, bytes)
                        .map_err(|e| e.errno())?;
                    unsafe {
                        slice::from_raw_parts_mut(host_ptr as *mut u8, bytes)
                            .copy_from_slice(&copied);
                    }
                }
                PostCopy::Readv {
                    ref iovs,
                    total_len,
                } => {
                    let mut remaining = value.min(total_len);
                    for &(user_ptr, host_ptr, len) in iovs {
                        let chunk = remaining.min(len);
                        let copied = self
                            .runtime
                            .copy_from_user(self.pid, user_ptr, chunk)
                            .map_err(|e| e.errno())?;
                        unsafe {
                            slice::from_raw_parts_mut(host_ptr as *mut u8, chunk)
                                .copy_from_slice(&copied);
                        }
                        remaining -= chunk;
                        if remaining == 0 {
                            break;
                        }
                    }
                }
            }
        }
        Ok(SyscallReturn::ok(value))
    }

    fn marshal_record_out<T>(&mut self, host_ptr: usize) -> Result<usize, user_abi::Errno> {
        let size = std::mem::size_of::<T>();
        let user_ptr = self.alloc(size, 8)?;
        self.post.push(PostCopy::Buffer {
            user_ptr,
            host_ptr,
            len: size,
            copy_len: CopyLen::Exact(size),
        });
        Ok(user_ptr)
    }

    fn marshal_writev(
        &mut self,
        host_iov_ptr: usize,
        count: usize,
    ) -> Result<usize, user_abi::Errno> {
        let host_iovs = unsafe { slice::from_raw_parts(host_iov_ptr as *const UserIoVec, count) };
        let mut scratch_iovs = Vec::with_capacity(count);
        for iov in host_iovs {
            let base = self.copy_in(iov.base, iov.len)?;
            scratch_iovs.push(UserIoVec { base, len: iov.len });
        }
        self.copy_structs_in(&scratch_iovs)
    }

    fn marshal_readv(
        &mut self,
        host_iov_ptr: usize,
        count: usize,
    ) -> Result<usize, user_abi::Errno> {
        let host_iovs = unsafe { slice::from_raw_parts(host_iov_ptr as *const UserIoVec, count) };
        let mut scratch_iovs = Vec::with_capacity(count);
        let mut copies = Vec::with_capacity(count);
        let mut total_len = 0usize;
        for iov in host_iovs {
            let user_ptr = self.alloc(iov.len, 1)?;
            scratch_iovs.push(UserIoVec {
                base: user_ptr,
                len: iov.len,
            });
            copies.push((user_ptr, iov.base, iov.len));
            total_len = total_len.saturating_add(iov.len);
        }
        self.post.push(PostCopy::Readv {
            iovs: copies,
            total_len,
        });
        self.copy_structs_in(&scratch_iovs)
    }

    fn copy_structs_in<T: Copy>(&mut self, values: &[T]) -> Result<usize, user_abi::Errno> {
        let len = std::mem::size_of_val(values);
        let user_ptr = self.alloc(len, std::mem::align_of::<T>())?;
        let bytes = unsafe { slice::from_raw_parts(values.as_ptr() as *const u8, len) };
        self.runtime
            .copy_to_user(self.pid, user_ptr, bytes)
            .map_err(|e| e.errno())?;
        Ok(user_ptr)
    }

    fn copy_in(&mut self, host_ptr: usize, len: usize) -> Result<usize, user_abi::Errno> {
        let user_ptr = self.alloc(len, 1)?;
        if len != 0 {
            let bytes = unsafe { slice::from_raw_parts(host_ptr as *const u8, len) };
            self.runtime
                .copy_to_user(self.pid, user_ptr, bytes)
                .map_err(|e| e.errno())?;
        }
        Ok(user_ptr)
    }

    fn alloc(&mut self, len: usize, align: usize) -> Result<usize, user_abi::Errno> {
        let align = align.max(1);
        let start = (self.base + self.cursor + (align - 1)) & !(align - 1);
        let end = start.saturating_add(len);
        if end > self.base + self.limit {
            return Err(user_abi::Errno::NoMem);
        }
        self.cursor = end - self.base;
        Ok(start)
    }
}
