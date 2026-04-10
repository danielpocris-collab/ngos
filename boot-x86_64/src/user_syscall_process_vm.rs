use super::*;

pub(super) fn dispatch_process_vm_syscall(frame: &SyscallFrame) -> Option<Result<usize, Errno>> {
    let result = match frame.number {
        SYS_INSPECT_PROCESS => {
            inspect_process_syscall(frame.arg0, frame.arg1 as *mut NativeProcessRecord)
        }
        SYS_INSPECT_PROCESS_COMPAT => {
            inspect_process_compat_syscall(frame.arg0, frame.arg1 as *mut NativeProcessCompatRecord)
        }
        SYS_INSPECT_STORAGE_VOLUME => inspect_storage_volume_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeStorageVolumeRecord,
        ),
        SYS_INSPECT_STORAGE_LINEAGE => inspect_storage_lineage_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeStorageLineageRecord,
        ),
        SYS_PREPARE_STORAGE_COMMIT => prepare_storage_commit_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3,
            frame.arg4 as *const u8,
            frame.arg5,
        ),
        SYS_RECOVER_STORAGE_VOLUME => recover_storage_volume_syscall(frame.arg0, frame.arg1),
        SYS_REPAIR_STORAGE_SNAPSHOT => repair_storage_snapshot_syscall(frame.arg0, frame.arg1),
        SYS_GET_PROCESS_NAME => {
            get_process_name_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_PROCESS_IMAGE_PATH => {
            get_process_image_path_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_PROCESS_CWD => {
            get_process_cwd_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_PROCESS_IDENTITY => inspect_process_identity_syscall(
            frame.arg0,
            frame.arg1 as *mut NativeProcessIdentityRecord,
        ),
        SYS_GET_PROCESS_SECURITY_LABEL => {
            inspect_process_security_label_syscall(frame.arg0, frame.arg1 as *mut SecurityLabel)
        }
        SYS_GET_PROCESS_ROOT => {
            get_process_root_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_SEND_SIGNAL => send_signal_syscall(frame.arg0, frame.arg1 as u8),
        SYS_PENDING_SIGNALS => {
            pending_signals_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_BLOCKED_PENDING_SIGNALS => {
            blocked_pending_signals_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_SPAWN_PATH_PROCESS => {
            spawn_path_process_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_SPAWN_PROCESS_COPY_VM => spawn_process_copy_vm_syscall(
            frame.arg0, frame.arg1, frame.arg2, frame.arg3, frame.arg4,
        ),
        SYS_SPAWN_CONFIGURED_PROCESS => spawn_configured_process_syscall(frame.arg0),
        SYS_SET_PROCESS_ARGS => {
            set_process_args_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_SET_PROCESS_ENV => {
            set_process_env_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_SET_PROCESS_CWD => set_process_cwd_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_SET_PROCESS_IDENTITY => set_process_identity_syscall(
            frame.arg0,
            frame.arg1 as *const NativeProcessIdentityRecord,
        ),
        SYS_SET_FD_RIGHTS => set_fd_rights_syscall(frame.arg0, BlockRightsMask(frame.arg1 as u64)),
        SYS_SET_PROCESS_SECURITY_LABEL => {
            set_process_security_label_syscall(frame.arg0, frame.arg1 as *const SecurityLabel)
        }
        SYS_SET_PROCESS_ROOT => set_process_root_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_SET_PROCESS_AFFINITY => set_process_affinity_syscall(frame.arg0, frame.arg1),
        SYS_PAUSE_PROCESS => pause_process_syscall(frame.arg0),
        SYS_RESUME_PROCESS => resume_process_syscall(frame.arg0),
        SYS_RENICE_PROCESS => renice_process_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_INSPECT_PATH_SECURITY_CONTEXT => inspect_path_security_context_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut ObjectSecurityContext,
        ),
        SYS_SET_PATH_SECURITY_LABEL => set_path_security_label_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const SecurityLabel,
        ),
        SYS_REAP_PROCESS => reap_process_syscall(frame.arg0),
        SYS_READ_PROCFS => {
            read_procfs_syscall(frame.arg0, frame.arg1, frame.arg2 as *mut u8, frame.arg3)
        }
        SYS_LOAD_MEMORY_WORD => load_memory_word_syscall(frame.arg0, frame.arg1),
        SYS_STORE_MEMORY_WORD => store_memory_word_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_QUARANTINE_VM_OBJECT => {
            quarantine_vm_object_syscall(frame.arg0, frame.arg1, frame.arg2)
        }
        SYS_RELEASE_VM_OBJECT => release_vm_object_syscall(frame.arg0, frame.arg1),
        SYS_MAP_ANONYMOUS_MEMORY => {
            map_anonymous_memory_syscall(frame.arg0, frame.arg1, frame.arg3, frame.arg4)
        }
        SYS_MAP_FILE_MEMORY => {
            let flags = frame.arg5;
            map_file_backed_memory_boot(
                frame.arg0,
                frame.arg1,
                frame.arg2,
                frame.arg3,
                frame.arg4,
                flags & 0x1,
                (flags >> 1) & 0x1,
                (flags >> 2) & 0x1,
                (flags >> 3) & 0x1,
            )
        }
        SYS_SET_PROCESS_BREAK => set_process_break_vm_syscall(frame.arg0, frame.arg1),
        SYS_RECLAIM_MEMORY_PRESSURE => reclaim_memory_pressure_syscall(frame.arg0, frame.arg1),
        SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL => reclaim_memory_pressure_global_syscall(frame.arg0),
        SYS_SYNC_MEMORY_RANGE => sync_memory_range_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_ADVISE_MEMORY_RANGE => {
            advise_memory_range_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_PROTECT_MEMORY_RANGE => protect_memory_range_syscall(
            frame.arg0, frame.arg1, frame.arg2, frame.arg3, frame.arg4, frame.arg5,
        ),
        SYS_UNMAP_MEMORY_RANGE => unmap_memory_range_syscall(frame.arg0, frame.arg1, frame.arg2),
        _ => return None,
    };
    Some(result)
}
