//! VFS operation implementations: stat, cat, write, copy, grep, mkdir, chmod, …

use alloc::format;

use ngos_user_abi::{ExitCode, NativeObjectKind, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::io::{
    object_kind_name, shell_emit_lines, shell_read_file_text, shell_write_all, write_line,
};

fn shell_close_fd_best_effort<B: SyscallBackend>(runtime: &Runtime<B>, fd: usize) {
    let _ = runtime.close(fd);
}

fn shell_prepare_writable_file_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    match runtime.stat_path(path) {
        Ok(status) if status.kind == NativeObjectKind::File as u32 => {
            runtime.unlink_path(path).map_err(|_| 245)?;
            runtime.mkfile_path(path).map_err(|_| 242)?;
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(_) => runtime.mkfile_path(path).map_err(|_| 242),
    }
}

pub fn shell_render_stat_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    follow_symlink: bool,
) -> Result<(), ExitCode> {
    let status = if follow_symlink {
        runtime.stat_path(path).map_err(|_| 231)?
    } else {
        runtime.lstat_path(path).map_err(|_| 232)?
    };
    write_line(
        runtime,
        &format!(
            "path={} kind={} inode={} size={} readable={} writable={} executable={} owner={} group={} mode={:o} cloexec={} nonblock={}",
            path,
            object_kind_name(status.kind),
            status.inode,
            status.size,
            status.readable,
            status.writable,
            status.executable,
            status.owner_uid,
            status.group_gid,
            status.mode,
            status.cloexec,
            status.nonblock
        ),
    )
}

pub fn shell_render_statfs_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let status = runtime.statfs_path(path).map_err(|_| 233)?;
    write_line(
        runtime,
        &format!(
            "path={} mounts={} nodes={} read_only={}",
            path, status.mount_count, status.node_count, status.read_only
        ),
    )
}

pub fn shell_open_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 234)?;
    write_line(runtime, &format!("opened path={} fd={fd}", path))
}

pub fn shell_readlink_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let mut buffer = [0u8; 256];
    let count = runtime.readlink_path(path, &mut buffer).map_err(|_| 235)?;
    let target = core::str::from_utf8(&buffer[..count]).map_err(|_| 236)?;
    write_line(runtime, &format!("link {} -> {}", path, target))
}

pub fn shell_cat_file<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    let mut buffer = [0u8; 256];
    loop {
        let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        let text = match core::str::from_utf8(&buffer[..count]) {
            Ok(text) => text,
            Err(_) => {
                shell_close_fd_best_effort(runtime, fd);
                return Err(239);
            }
        };
        if let Err(code) = shell_emit_lines(runtime, text) {
            shell_close_fd_best_effort(runtime, fd);
            return Err(code);
        }
    }
    runtime.close(fd).map_err(|_| 240)?;
    Ok(())
}

pub fn shell_write_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    text: &str,
) -> Result<(), ExitCode> {
    shell_prepare_writable_file_path(runtime, path)?;
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    shell_write_all(runtime, fd, text.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("file-written path={path} bytes={}", text.len()),
    )
}

pub fn shell_append_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    text: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    let mut drain = [0u8; 256];
    loop {
        let count = runtime.read(fd, &mut drain).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
    }
    shell_write_all(runtime, fd, text.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("file-appended path={path} bytes={}", text.len()),
    )
}

pub fn shell_truncate_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    len: usize,
) -> Result<(), ExitCode> {
    runtime.truncate_path(path, len).map_err(|_| 243)?;
    write_line(runtime, &format!("file-truncated path={path} size={len}"))
}

pub fn shell_link_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    source: &str,
    destination: &str,
) -> Result<(), ExitCode> {
    runtime.link_path(source, destination).map_err(|_| 244)?;
    write_line(
        runtime,
        &format!("path-linked from={source} to={destination}"),
    )
}

pub fn shell_copy_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    source: &str,
    destination: &str,
) -> Result<(), ExitCode> {
    let src = runtime.open_path(source).map_err(|_| 237)?;
    if let Err(code) = shell_prepare_writable_file_path(runtime, destination) {
        shell_close_fd_best_effort(runtime, src);
        return Err(code);
    }
    let dst = match runtime.open_path(destination).map_err(|_| 237) {
        Ok(fd) => fd,
        Err(code) => {
            shell_close_fd_best_effort(runtime, src);
            return Err(code);
        }
    };
    let mut buffer = [0u8; 256];
    let mut total = 0usize;
    loop {
        let count = match runtime.read(src, &mut buffer).map_err(|_| 238) {
            Ok(count) => count,
            Err(code) => {
                shell_close_fd_best_effort(runtime, src);
                shell_close_fd_best_effort(runtime, dst);
                return Err(code);
            }
        };
        if count == 0 {
            break;
        }
        if let Err(code) = shell_write_all(runtime, dst, &buffer[..count]) {
            shell_close_fd_best_effort(runtime, src);
            shell_close_fd_best_effort(runtime, dst);
            return Err(code);
        }
        total += count;
    }
    runtime.close(src).map_err(|_| 240)?;
    runtime.close(dst).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("file-copied from={source} to={destination} bytes={total}"),
    )
}

pub fn shell_compare_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    left: &str,
    right: &str,
) -> Result<(), ExitCode> {
    let left_text = shell_read_file_text(runtime, left)?;
    let right_text = shell_read_file_text(runtime, right)?;
    if left_text == right_text {
        return write_line(
            runtime,
            &format!(
                "files-match left={left} right={right} bytes={}",
                left_text.len()
            ),
        );
    }
    let left_bytes = left_text.as_bytes();
    let right_bytes = right_text.as_bytes();
    let mismatch = left_bytes
        .iter()
        .zip(right_bytes.iter())
        .position(|(l, r)| l != r)
        .unwrap_or_else(|| left_bytes.len().min(right_bytes.len()));
    write_line(
        runtime,
        &format!(
            "files-differ left={left} right={right} offset={mismatch} left-bytes={} right-bytes={}",
            left_bytes.len(),
            right_bytes.len()
        ),
    )
}

pub fn shell_grep_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let mut matched = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line.contains(needle) {
            matched += 1;
            write_line(runtime, &format!("grep {path}:{} {}", index + 1, line))?;
        }
    }
    write_line(
        runtime,
        &format!("grep-summary path={path} needle={needle} matches={matched}"),
    )
}

pub fn shell_assert_file_contains<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    if text.contains(needle) {
        write_line(
            runtime,
            &format!("assert-file-contains-ok path={path} needle={needle}"),
        )
    } else {
        let _ = write_line(
            runtime,
            &format!("assert-file-contains-failed path={path} needle={needle}"),
        );
        Err(248)
    }
}

pub fn shell_mkdir_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    runtime.mkdir_path(path).map_err(|_| 241)?;
    write_line(runtime, &format!("directory-created path={path}"))
}

pub fn shell_mkfile_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    runtime.mkfile_path(path).map_err(|_| 242)?;
    write_line(runtime, &format!("file-created path={path}"))
}

pub fn shell_mksock_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    runtime.mksock_path(path).map_err(|_| 242)?;
    write_line(runtime, &format!("socket-created path={path}"))
}

pub fn shell_symlink_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    target: &str,
) -> Result<(), ExitCode> {
    runtime.symlink_path(path, target).map_err(|_| 243)?;
    write_line(
        runtime,
        &format!("symlink-created path={path} target={target}"),
    )
}

pub fn shell_chmod_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    mode: &str,
) -> Result<(), ExitCode> {
    let mode = u32::from_str_radix(mode, 8).map_err(|_| 243)?;
    runtime.chmod_path(path, mode).map_err(|_| 243)?;
    write_line(runtime, &format!("path-chmod path={path} mode={mode:o}"))
}

pub fn shell_chown_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    owner: &str,
    group: &str,
) -> Result<(), ExitCode> {
    let owner_uid = owner.parse::<u32>().map_err(|_| 243)?;
    let group_gid = group.parse::<u32>().map_err(|_| 243)?;
    runtime
        .chown_path(path, owner_uid, group_gid)
        .map_err(|_| 243)?;
    write_line(
        runtime,
        &format!("path-chown path={path} owner={owner_uid} group={group_gid}"),
    )
}

pub fn shell_rename_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    from: &str,
    to: &str,
) -> Result<(), ExitCode> {
    runtime.rename_path(from, to).map_err(|_| 244)?;
    write_line(runtime, &format!("path-renamed from={from} to={to}"))
}

pub fn shell_unlink_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    runtime.unlink_path(path).map_err(|_| 245)?;
    write_line(runtime, &format!("path-unlinked path={path}"))
}

pub fn shell_list_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let mut buffer = alloc::vec![0u8; 512];
    loop {
        let count = runtime.list_path(path, &mut buffer).map_err(|_| 246)?;
        if count < buffer.len() {
            let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 247)?;
            if text.is_empty() {
                return write_line(runtime, &format!("path={path} entries=0"));
            }
            return shell_emit_lines(runtime, text);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}
