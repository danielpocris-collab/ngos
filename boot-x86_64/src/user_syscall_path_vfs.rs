use super::*;

pub(super) fn dispatch_path_vfs_syscall(frame: &SyscallFrame) -> Option<Result<usize, Errno>> {
    let result = match frame.number {
        SYS_MKDIR_PATH => mkdir_path_syscall(frame.arg0, frame.arg1),
        SYS_MKDIR_PATH_AT => mkdir_path_at_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_MKFILE_PATH => mkfile_path_syscall(frame.arg0, frame.arg1),
        SYS_MKFILE_PATH_AT => mkfile_path_at_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_MKCHAN_PATH => mkchan_path_syscall(frame.arg0, frame.arg1),
        SYS_MKSOCK_PATH => mksock_path_syscall(frame.arg0, frame.arg1),
        SYS_MOUNT_STORAGE_VOLUME => {
            mount_storage_volume_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_UNMOUNT_STORAGE_VOLUME => unmount_storage_volume_syscall(frame.arg0, frame.arg1),
        SYS_INSPECT_MOUNT => {
            inspect_mount_syscall(frame.arg0, frame.arg1, frame.arg2 as *mut NativeMountRecord)
        }
        SYS_SET_MOUNT_PROPAGATION => {
            set_mount_propagation_syscall(frame.arg0, frame.arg1, frame.arg2 as u32)
        }
        SYS_SYMLINK_PATH => symlink_path_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3),
        SYS_SYMLINK_PATH_AT => {
            symlink_path_at_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3, frame.arg4)
        }
        SYS_CHMOD_PATH_AT => {
            chmod_path_at_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3 as u32)
        }
        SYS_CHOWN_PATH_AT => chown_path_at_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3 as u32,
            frame.arg4 as u32,
        ),
        SYS_CHMOD_PATH => chmod_path_syscall(frame.arg0, frame.arg1, frame.arg2 as u32),
        SYS_CHOWN_PATH => {
            chown_path_syscall(frame.arg0, frame.arg1, frame.arg2 as u32, frame.arg3 as u32)
        }
        SYS_RENAME_PATH => rename_path_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3),
        SYS_RENAME_PATH_AT => rename_path_at_syscall(
            frame.arg0, frame.arg1, frame.arg2, frame.arg3, frame.arg4, frame.arg5,
        ),
        SYS_LINK_PATH => link_path_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3),
        SYS_LINK_PATH_AT => link_path_at_syscall(
            frame.arg0, frame.arg1, frame.arg2, frame.arg3, frame.arg4, frame.arg5,
        ),
        SYS_UNLINK_PATH => unlink_path_syscall(frame.arg0, frame.arg1),
        SYS_UNLINK_PATH_AT => unlink_path_at_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_TRUNCATE_PATH => truncate_path_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_TRUNCATE_PATH_AT => {
            truncate_path_at_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_READLINK_PATH => {
            readlink_path_syscall(frame.arg0, frame.arg1, frame.arg2 as *mut u8, frame.arg3)
        }
        SYS_READLINK_PATH_AT => readlink_path_at_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3 as *mut u8,
            frame.arg4,
        ),
        SYS_LIST_PATH => {
            list_path_syscall(frame.arg0, frame.arg1, frame.arg2 as *mut u8, frame.arg3)
        }
        SYS_LIST_PATH_AT => list_path_at_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3 as *mut u8,
            frame.arg4,
        ),
        _ => return None,
    };
    Some(result)
}
