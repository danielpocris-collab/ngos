#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod block_admin_agent;

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

use ngos_shell_proc::fixed_text_field;
use ngos_shell_types::{ShellVariable, resolve_shell_path, shell_set_variable};
use ngos_shell_vfs::{shell_emit_lines, write_line};
use ngos_user_abi::{
    Errno, ExitCode, NATIVE_BLOCK_IO_MAGIC, NATIVE_BLOCK_IO_VERSION, NativeBlockIoRequest,
    NativeMountPropagationMode, NativeReadinessRecord, NativeStorageVolumeRecord, SyscallBackend,
};
use ngos_user_runtime::Runtime;

use crate::block_admin_agent::try_handle_block_admin_command;

pub fn try_handle_surface_front_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    variables: &mut Vec<ShellVariable>,
    line: &str,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("fd-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: fd-watch <path> <read|write|priority|readwrite|readpriority|writepriority|all>",
            );
            return Some(Err(2));
        };
        let Some(mode) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: fd-watch <path> <read|write|priority|readwrite|readpriority|writepriority|all>",
            );
            return Some(Err(2));
        };
        let Some((readable, writable, priority)) = parse_readiness_interest(mode) else {
            let _ = write_line(
                runtime,
                "usage: fd-watch <path> <read|write|priority|readwrite|readpriority|writepriority|all>",
            );
            return Some(Err(2));
        };
        *last_status = match shell_watch_fd_readiness(
            runtime,
            &resolve_shell_path(cwd, path),
            readable,
            writable,
            priority,
        ) {
            Ok(fd) => {
                shell_set_variable(variables, "LAST_WATCH_FD", fd.to_string());
                0
            }
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if line == "fd-ready" {
        *last_status = match shell_collect_readiness(runtime) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(result) = try_handle_block_admin_command(runtime, cwd, line, last_status) {
        return Some(result);
    }
    if let Some(path) = line.strip_prefix("storage-volume ") {
        *last_status =
            match shell_render_storage_volume(runtime, &resolve_shell_path(cwd, path.trim())) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("storage-lineage ") {
        *last_status =
            match shell_render_storage_lineage(runtime, &resolve_shell_path(cwd, path.trim())) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("storage-history ") {
        *last_status =
            match shell_render_storage_history(runtime, &resolve_shell_path(cwd, path.trim())) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("storage-history-range ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: storage-history-range <device> <start> <count>",
            );
            return Some(Err(2));
        };
        let Some(start) = parts.next().and_then(|value| value.parse::<usize>().ok()) else {
            let _ = write_line(
                runtime,
                "usage: storage-history-range <device> <start> <count>",
            );
            return Some(Err(2));
        };
        let Some(count) = parts.next().and_then(|value| value.parse::<usize>().ok()) else {
            let _ = write_line(
                runtime,
                "usage: storage-history-range <device> <start> <count>",
            );
            return Some(Err(2));
        };
        *last_status = match shell_render_storage_history_range(
            runtime,
            &resolve_shell_path(cwd, device_path),
            start,
            count,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("storage-history-tail ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: storage-history-tail <device> <count>");
            return Some(Err(2));
        };
        let Some(count) = parts.next().and_then(|value| value.parse::<usize>().ok()) else {
            let _ = write_line(runtime, "usage: storage-history-tail <device> <count>");
            return Some(Err(2));
        };
        *last_status = match shell_render_storage_history_tail(
            runtime,
            &resolve_shell_path(cwd, device_path),
            count,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("storage-history-entry ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: storage-history-entry <device> <index>");
            return Some(Err(2));
        };
        let Some(index) = parts.next().and_then(|value| value.parse::<usize>().ok()) else {
            let _ = write_line(runtime, "usage: storage-history-entry <device> <index>");
            return Some(Err(2));
        };
        *last_status = match shell_render_storage_history_entry(
            runtime,
            &resolve_shell_path(cwd, device_path),
            index,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("storage-prepare ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: storage-prepare <device> <tag> <payload>");
            return Some(Err(2));
        };
        let Some(tag) = parts.next() else {
            let _ = write_line(runtime, "usage: storage-prepare <device> <tag> <payload>");
            return Some(Err(2));
        };
        let payload = parts.collect::<Vec<_>>().join(" ");
        if payload.is_empty() {
            let _ = write_line(runtime, "usage: storage-prepare <device> <tag> <payload>");
            return Some(Err(2));
        }
        *last_status = match shell_prepare_storage_commit(
            runtime,
            &resolve_shell_path(cwd, device_path),
            tag,
            &payload,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("storage-recover ") {
        *last_status =
            match shell_recover_storage_volume(runtime, &resolve_shell_path(cwd, path.trim())) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("storage-repair ") {
        *last_status =
            match shell_repair_storage_snapshot(runtime, &resolve_shell_path(cwd, path.trim())) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("storage-mount ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: storage-mount <device> <mount>");
            return Some(Err(2));
        };
        let Some(mount_path) = parts.next() else {
            let _ = write_line(runtime, "usage: storage-mount <device> <mount>");
            return Some(Err(2));
        };
        *last_status = match shell_mount_storage_volume(
            runtime,
            &resolve_shell_path(cwd, device_path),
            &resolve_shell_path(cwd, mount_path),
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("storage-unmount ") {
        *last_status =
            match shell_unmount_storage_volume(runtime, &resolve_shell_path(cwd, path.trim())) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("mount-info ") {
        *last_status = match shell_render_mount_info(runtime, &resolve_shell_path(cwd, path.trim()))
        {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("mount-propagation ") {
        let mut parts = rest.split_whitespace();
        let Some(path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: mount-propagation <mount> <private|shared|slave>",
            );
            return Some(Err(2));
        };
        let Some(mode) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: mount-propagation <mount> <private|shared|slave>",
            );
            return Some(Err(2));
        };
        let mode = match mode {
            "private" => NativeMountPropagationMode::Private,
            "shared" => NativeMountPropagationMode::Shared,
            "slave" => NativeMountPropagationMode::Slave,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: mount-propagation <mount> <private|shared|slave>",
                );
                return Some(Err(2));
            }
        };
        *last_status =
            match shell_set_mount_propagation(runtime, &resolve_shell_path(cwd, path), mode) {
                Ok(()) => 0,
                Err(code) => code,
            };
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("driver-read ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(shell_driver_read(runtime, &resolved).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("echo ") {
        return Some(write_line(runtime, rest).map_err(|_| 197));
    }
    None
}

fn parse_readiness_interest(token: &str) -> Option<(bool, bool, bool)> {
    match token {
        "read" => Some((true, false, false)),
        "write" => Some((false, true, false)),
        "priority" => Some((false, false, true)),
        "readwrite" => Some((true, true, false)),
        "readpriority" => Some((true, false, true)),
        "writepriority" => Some((false, true, true)),
        "all" => Some((true, true, true)),
        _ => None,
    }
}

fn shell_watch_fd_readiness<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    readable: bool,
    writable: bool,
    priority: bool,
) -> Result<usize, ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 234)?;
    runtime
        .register_readiness(fd, readable, writable, priority)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "fd-watch fd={} path={} readable={} writable={} priority={}",
            fd, path, readable as u8, writable as u8, priority as u8
        ),
    )?;
    Ok(fd)
}

fn shell_collect_readiness<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut records = [NativeReadinessRecord {
        owner: 0,
        fd: 0,
        readable: 0,
        writable: 0,
        priority: 0,
        reserved: 0,
    }; 16];
    let count = runtime.collect_readiness(&mut records).map_err(|_| 246)?;
    if count == 0 {
        return write_line(runtime, "fd-ready count=0");
    }
    for record in &records[..count] {
        write_line(
            runtime,
            &format!(
                "fd-ready owner={} fd={} readable={} writable={} priority={}",
                record.owner, record.fd, record.readable, record.writable, record.priority
            ),
        )?;
    }
    Ok(())
}

fn try_decode_block_request(bytes: &[u8]) -> Option<NativeBlockIoRequest> {
    if bytes.len() < core::mem::size_of::<NativeBlockIoRequest>() {
        return None;
    }
    let request = unsafe { (bytes.as_ptr() as *const NativeBlockIoRequest).read_unaligned() };
    if request.magic != NATIVE_BLOCK_IO_MAGIC || request.version != NATIVE_BLOCK_IO_VERSION {
        return None;
    }
    Some(request)
}

fn shell_render_storage_volume<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeStorageVolumeRecord = runtime
        .inspect_storage_volume(device_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "storage-volume path={} valid={} dirty={} generation={} parent-generation={} replay-generation={} prepares={} recovers={} repairs={} payload-len={} checksum={} superblock={} journal={} data={} alloc-total={} alloc-used={} files={} dirs={} symlinks={} extents={} volume={} state={} tag={} preview={}",
            device_path,
            record.valid,
            record.dirty,
            record.generation,
            record.parent_generation,
            record.replay_generation,
            record.prepared_commit_count,
            record.recovered_commit_count,
            record.repaired_snapshot_count,
            record.payload_len,
            record.payload_checksum,
            record.superblock_sector,
            record.journal_sector,
            record.data_sector,
            record.allocation_total_blocks,
            record.allocation_used_blocks,
            record.mapped_file_count,
            record.mapped_directory_count,
            record.mapped_symlink_count,
            record.mapped_extent_count,
            fixed_text_field(&record.volume_id),
            fixed_text_field(&record.state_label),
            fixed_text_field(&record.last_commit_tag),
            fixed_text_field(&record.payload_preview)
        ),
    )
}

fn shell_render_storage_lineage<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeStorageVolumeRecord = runtime
        .inspect_storage_volume(device_path)
        .map_err(|_| 246)?;
    let lineage = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "storage-lineage path={} generation={} parent={} replay={} prepares={} recovers={} repairs={} newest={} oldest={} contiguous={} state={} tag={}",
            device_path,
            record.generation,
            record.parent_generation,
            record.replay_generation,
            record.prepared_commit_count,
            record.recovered_commit_count,
            record.repaired_snapshot_count,
            lineage.newest_generation,
            lineage.oldest_generation,
            if lineage.lineage_contiguous != 0 {
                "yes"
            } else {
                "no"
            },
            fixed_text_field(&record.state_label),
            fixed_text_field(&record.last_commit_tag)
        ),
    )
}

fn shell_render_storage_history<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    if record.valid == 0 || record.count == 0 {
        return write_line(
            runtime,
            &format!(
                "storage-history path={} count=0 newest=0 oldest=0 contiguous=yes events=empty",
                device_path
            ),
        );
    }
    let mut parts = Vec::new();
    for entry in record.entries.iter().take(record.count as usize) {
        parts.push(format!(
            "{}:{}:{}<-{}:{}:{}",
            entry.generation,
            fixed_text_field(&entry.kind_label),
            fixed_text_field(&entry.state_label),
            entry.parent_generation,
            entry.payload_checksum,
            fixed_text_field(&entry.tag_label)
        ));
    }
    write_line(
        runtime,
        &format!(
            "storage-history path={} count={} newest={} oldest={} contiguous={} events={}",
            device_path,
            record.count,
            record.newest_generation,
            record.oldest_generation,
            if record.lineage_contiguous != 0 {
                "yes"
            } else {
                "no"
            },
            parts.join("|")
        ),
    )
}

fn shell_render_storage_history_range<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    start: usize,
    count: usize,
) -> Result<(), ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    if record.valid == 0 || count == 0 || start >= record.count as usize {
        return write_line(
            runtime,
            &format!(
                "storage-history-range path={} start={} count={} entries=empty",
                device_path, start, count
            ),
        );
    }
    let mut parts = Vec::new();
    for (offset, entry) in record
        .entries
        .iter()
        .take(record.count as usize)
        .skip(start)
        .take(count)
        .enumerate()
    {
        parts.push(format!(
            "{}:{}:{}<-{}:{}:{}",
            start + offset,
            fixed_text_field(&entry.kind_label),
            fixed_text_field(&entry.state_label),
            entry.parent_generation,
            entry.payload_checksum,
            fixed_text_field(&entry.tag_label)
        ));
    }
    write_line(
        runtime,
        &format!(
            "storage-history-range path={} start={} count={} entries={}",
            device_path,
            start,
            count,
            parts.join("|")
        ),
    )
}

fn shell_render_storage_history_tail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    count: usize,
) -> Result<(), ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    let total = record.count as usize;
    let start = total.saturating_sub(count);
    if record.valid == 0 || total == 0 || count == 0 || start >= total {
        return write_line(
            runtime,
            &format!(
                "storage-history-tail path={} count={} entries=empty",
                device_path, count
            ),
        );
    }
    let mut parts = Vec::new();
    for offset in start..total {
        let entry = &record.entries[offset];
        parts.push(format!(
            "{}:{}:{}<-{}:{}:{}",
            offset - start,
            fixed_text_field(&entry.kind_label),
            fixed_text_field(&entry.state_label),
            entry.parent_generation,
            entry.payload_checksum,
            fixed_text_field(&entry.tag_label)
        ));
    }
    write_line(
        runtime,
        &format!(
            "storage-history-tail path={} count={} entries={}",
            device_path,
            count,
            parts.join("|")
        ),
    )
}

fn shell_render_storage_history_entry<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    index: usize,
) -> Result<(), ExitCode> {
    let record = runtime
        .inspect_storage_lineage(device_path)
        .map_err(|_| 246)?;
    if record.valid == 0 || index >= record.count as usize {
        return write_line(
            runtime,
            &format!(
                "storage-history-entry path={} index={} status=missing",
                device_path, index
            ),
        );
    }
    let entry = &record.entries[index];
    write_line(
        runtime,
        &format!(
            "storage-history-entry path={} index={} generation={} parent={} kind={} state={} checksum={} tag={}",
            device_path,
            index,
            entry.generation,
            entry.parent_generation,
            fixed_text_field(&entry.kind_label),
            fixed_text_field(&entry.state_label),
            entry.payload_checksum,
            fixed_text_field(&entry.tag_label)
        ),
    )
}

fn shell_prepare_storage_commit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    tag: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let generation = runtime
        .prepare_storage_commit(device_path, tag, payload.as_bytes())
        .map_err(|errno| match errno {
            Errno::TooBig => 247,
            Errno::Inval => 248,
            Errno::Nxio => 249,
            _ => 246,
        })?;
    write_line(
        runtime,
        &format!(
            "storage-prepare path={} generation={} tag={} bytes={}",
            device_path,
            generation,
            tag,
            payload.len()
        ),
    )
}

fn shell_recover_storage_volume<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let generation = runtime
        .recover_storage_volume(device_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "storage-recover path={} generation={}",
            device_path, generation
        ),
    )
}

fn shell_repair_storage_snapshot<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let generation = runtime
        .repair_storage_snapshot(device_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "storage-repair path={} generation={}",
            device_path, generation
        ),
    )
}

fn shell_mount_storage_volume<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    mount_path: &str,
) -> Result<(), ExitCode> {
    let loaded = runtime
        .mount_storage_volume(device_path, mount_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "storage-mount device={} mount={} entries={}",
            device_path, mount_path, loaded
        ),
    )
}

fn shell_unmount_storage_volume<B: SyscallBackend>(
    runtime: &Runtime<B>,
    mount_path: &str,
) -> Result<(), ExitCode> {
    runtime
        .unmount_storage_volume(mount_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("storage-unmount mount={} generation=1", mount_path),
    )
}

fn mount_propagation_name(mode: u32) -> &'static str {
    match NativeMountPropagationMode::from_raw(mode) {
        Some(NativeMountPropagationMode::Private) => "private",
        Some(NativeMountPropagationMode::Shared) => "shared",
        Some(NativeMountPropagationMode::Slave) => "slave",
        None => "unknown",
    }
}

fn shell_render_mount_info<B: SyscallBackend>(
    runtime: &Runtime<B>,
    mount_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_mount(mount_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "mount-info path={} id={} parent={} peer-group={} master-group={} layer={} entries={} mode={} created-root={}",
            mount_path,
            record.id,
            record.parent_mount_id,
            record.peer_group,
            record.master_group,
            record.layer,
            record.entry_count,
            mount_propagation_name(record.propagation_mode),
            if record.created_mount_root != 0 {
                "yes"
            } else {
                "no"
            },
        ),
    )
}

fn shell_set_mount_propagation<B: SyscallBackend>(
    runtime: &Runtime<B>,
    mount_path: &str,
    mode: NativeMountPropagationMode,
) -> Result<(), ExitCode> {
    runtime
        .set_mount_propagation(mount_path, mode)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "mount-propagation path={} mode={}",
            mount_path,
            mount_propagation_name(mode as u32)
        ),
    )
}

pub fn shell_driver_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    let prefix_len = buffer[..count]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(count);
    let text = core::str::from_utf8(&buffer[..prefix_len]).map_err(|_| 239)?;
    shell_emit_lines(runtime, text)?;
    if prefix_len < count {
        let payload_bytes = &buffer[prefix_len..count];
        if !payload_bytes.is_empty() {
            if let Some(request) = try_decode_block_request(payload_bytes) {
                write_line(
                    runtime,
                    &format!(
                        "block-request path={} op={} sector={} sectors={} block-size={}",
                        driver_path,
                        match request.op {
                            ngos_user_abi::NATIVE_BLOCK_IO_OP_READ => "read",
                            _ => "unknown",
                        },
                        request.sector,
                        request.sector_count,
                        request.block_size
                    ),
                )?;
            } else if let Ok(payload) = core::str::from_utf8(payload_bytes) {
                write_line(
                    runtime,
                    &format!(
                        "driver-payload path={} bytes={} text={}",
                        driver_path,
                        payload_bytes.len(),
                        payload
                    ),
                )?;
            } else {
                write_line(
                    runtime,
                    &format!(
                        "driver-payload path={} bytes={} encoding=binary",
                        driver_path,
                        payload_bytes.len()
                    ),
                )?;
            }
        }
    }
    Ok(())
}
