use super::*;

const PERSISTED_EXTRA_PATHS: [&str; 6] = [
    "/persist/extra-a",
    "/persist/extra-b",
    "/persist/extra-c",
    "/persist/extra-d",
    "/persist/extra-e",
    "/persist/extra-f",
];
const OVERFLOW_EXTRA_COUNT: usize = 26;

fn write_storage_text_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    payload: &[u8],
    create_error: ExitCode,
    open_error: ExitCode,
    write_error: ExitCode,
    close_error: ExitCode,
) -> Result<(), ExitCode> {
    if runtime.mkfile_path(path).is_err() {
        return Err(create_error);
    }
    let fd = runtime.open_path(path).map_err(|_| open_error)?;
    if shell_write_all(runtime, fd, payload).is_err() {
        let _ = runtime.close(fd);
        return Err(write_error);
    }
    runtime.close(fd).map_err(|_| close_error)?;
    Ok(())
}

#[inline(never)]
pub(crate) fn run_native_storage_commit_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let before = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 501,
    };
    if write_line(
        runtime,
        &format!(
            "storage.smoke.inspect.before valid={} dirty={} generation={} volume={} state={}",
            before.valid,
            before.dirty,
            before.generation,
            fixed_text_field(&before.volume_id),
            fixed_text_field(&before.state_label)
        ),
    )
    .is_err()
    {
        return 502;
    }

    let payload = "persist:qemu-storage-commit-001";
    let prepared_generation = match runtime.prepare_storage_commit(
        "/dev/storage0",
        "qemu-storage-commit-001",
        payload.as_bytes(),
    ) {
        Ok(value) => value,
        Err(_) => return 503,
    };
    let prepared = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 504,
    };
    if prepared.valid == 0
        || prepared.dirty == 0
        || prepared.generation != prepared_generation as u64
        || fixed_text_field(&prepared.last_commit_tag) != "qemu-storage-commit-001"
        || fixed_text_field(&prepared.payload_preview) != payload
    {
        return 505;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.success generation={} dirty={} volume={} state={} tag={} preview={} checksum={}",
            prepared.generation,
            prepared.dirty,
            fixed_text_field(&prepared.volume_id),
            fixed_text_field(&prepared.state_label),
            fixed_text_field(&prepared.last_commit_tag),
            fixed_text_field(&prepared.payload_preview),
            prepared.payload_checksum
        ),
    )
    .is_err()
    {
        return 506;
    }

    let oversized = [b'Z'; 513];
    match runtime.prepare_storage_commit("/dev/storage0", "oversized", &oversized) {
        Err(Errno::TooBig) => {}
        _ => return 507,
    }
    if write_line(
        runtime,
        "storage.smoke.refusal op=prepare errno=E2BIG outcome=expected",
    )
    .is_err()
    {
        return 508;
    }

    let recovered_generation = match runtime.recover_storage_volume("/dev/storage0") {
        Ok(value) => value,
        Err(_) => return 509,
    };
    let recovered = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 510,
    };
    if recovered.dirty != 0
        || recovered.generation != recovered_generation as u64
        || recovered.replay_generation != recovered.generation
        || fixed_text_field(&recovered.state_label) != "recovered"
        || fixed_text_field(&recovered.payload_preview) != payload
    {
        return 511;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.recovery generation={} replay={} state={} tag={} preview={} checksum={} outcome=ok",
            recovered.generation,
            recovered.replay_generation,
            fixed_text_field(&recovered.state_label),
            fixed_text_field(&recovered.last_commit_tag),
            fixed_text_field(&recovered.payload_preview),
            recovered.payload_checksum
        ),
    )
    .is_err()
    {
        return 512;
    }

    let cleared_generation = match runtime.prepare_storage_commit("/dev/storage0", "clear", &[]) {
        Ok(value) => value,
        Err(_) => return 513,
    };
    let cleared = match runtime.recover_storage_volume("/dev/storage0") {
        Ok(value) => value,
        Err(_) => return 514,
    };
    let cleared_record = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 515,
    };
    if cleared != cleared_generation
        || cleared_record.generation != cleared as u64
        || cleared_record.dirty != 0
        || cleared_record.payload_len != 0
    {
        return 516;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.clear generation={} state={} payload-len={} outcome=ok",
            cleared_record.generation,
            fixed_text_field(&cleared_record.state_label),
            cleared_record.payload_len
        ),
    )
    .is_err()
    {
        return 517;
    }

    let mounted_entries = match runtime.mount_storage_volume("/dev/storage0", "/persist") {
        Ok(value) => value,
        Err(_) => return 518,
    };
    if mounted_entries != 0 {
        return 519;
    }
    if runtime.mkdir_path("/persist/config").is_err() {
        return 520;
    }
    if runtime.mkdir_path("/persist/assets").is_err() {
        return 582;
    }
    if runtime.mkfile_path("/persist/config/session.txt").is_err() {
        return 583;
    }
    let session_payload = b"persist:qemu-vfs-session-001";
    let session_fd = match runtime.open_path("/persist/config/session.txt") {
        Ok(fd) => fd,
        Err(_) => return 521,
    };
    if shell_write_all(runtime, session_fd, session_payload).is_err() {
        let _ = runtime.close(session_fd);
        return 522;
    }
    if runtime.close(session_fd).is_err() {
        return 523;
    }
    if runtime
        .symlink_path("/persist/current-session", "/persist/config/session.txt")
        .is_err()
    {
        return 584;
    }
    if runtime.mkfile_path("/persist/assets/asset.bin").is_err() {
        return 542;
    }
    let asset_payload = [b'A'; 900];
    let asset_fd = match runtime.open_path("/persist/assets/asset.bin") {
        Ok(fd) => fd,
        Err(_) => return 543,
    };
    if shell_write_all(runtime, asset_fd, &asset_payload).is_err() {
        let _ = runtime.close(asset_fd);
        return 544;
    }
    if runtime.close(asset_fd).is_err() {
        return 545;
    }
    for (index, path) in PERSISTED_EXTRA_PATHS.iter().enumerate() {
        let payload = [b'a' + index as u8];
        if write_storage_text_file(
            runtime,
            path,
            &payload,
            546 + index as i32,
            552 + index as i32,
            558 + index as i32,
            564 + index as i32,
        )
        .is_err()
        {
            return 546 + index as i32;
        }
    }
    for index in 0..OVERFLOW_EXTRA_COUNT {
        let path = format!("/persist/overflow-{index:02}.tmp");
        let payload = [b'0' + (index % 10) as u8];
        if write_storage_text_file(
            runtime,
            &path,
            &payload,
            570 + index as i32,
            600 + index as i32,
            630 + index as i32,
            660 + index as i32,
        )
        .is_err()
        {
            return 570 + index as i32;
        }
    }
    match runtime.unmount_storage_volume("/persist") {
        Err(Errno::TooBig) => {}
        _ => return 562,
    }
    if write_line(
        runtime,
        "storage.smoke.mapping.refusal op=unmount errno=E2BIG outcome=expected",
    )
    .is_err()
    {
        return 563;
    }
    for index in 0..OVERFLOW_EXTRA_COUNT {
        let path = format!("/persist/overflow-{index:02}.tmp");
        if runtime.unlink_path(&path).is_err() {
            return 700 + index as i32;
        }
    }
    let unmounted_generation = match runtime.unmount_storage_volume("/persist") {
        Ok(value) => value,
        Err(_) => return 524,
    };
    match runtime.inspect_mount("/persist") {
        Err(Errno::NoEnt) => {}
        _ => return 526,
    }
    if let Ok(status) = runtime.stat_path("/persist") {
        if status.kind != NativeObjectKind::Directory as u32 {
            return 526;
        }
        let mut mountpoint_buffer = [0u8; 64];
        let mountpoint_len = match runtime.list_path("/persist", &mut mountpoint_buffer) {
            Ok(value) => value,
            Err(_) => return 526,
        };
        let mountpoint_listing = match core::str::from_utf8(&mountpoint_buffer[..mountpoint_len]) {
            Ok(text) => text,
            Err(_) => return 526,
        };
        if mountpoint_listing
            .lines()
            .any(|line| !line.trim().is_empty())
        {
            return 526;
        }
    }
    let prepared = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 504,
    };
    if write_line(
        runtime,
        &format!(
            "storage.smoke.inspect.after-unmount valid={} dirty={} generation={} replay={} state={} tag={} payload-len={} checksum={}",
            prepared.valid,
            prepared.dirty,
            prepared.generation,
            prepared.replay_generation,
            fixed_text_field(&prepared.state_label),
            fixed_text_field(&prepared.last_commit_tag),
            prepared.payload_len,
            prepared.payload_checksum
        ),
    )
    .is_err()
    {
        return 540;
    }
    if prepared.valid == 0
        || prepared.dirty != 0
        || prepared.generation != unmounted_generation as u64
        || fixed_text_field(&prepared.last_commit_tag) != "boot-vfs-unmount"
        || fixed_text_field(&prepared.state_label) != "recovered"
        || prepared.payload_len == 0
    {
        return 505;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.mount.success generation={} dirty={} volume={} state={} tag={} payload-len={} checksum={}",
            prepared.generation,
            prepared.dirty,
            fixed_text_field(&prepared.volume_id),
            fixed_text_field(&prepared.state_label),
            fixed_text_field(&prepared.last_commit_tag),
            prepared.payload_len,
            prepared.payload_checksum
        ),
    )
    .is_err()
    {
        return 506;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.mount.commit mount=/persist entries={} files=8 dirs=2 symlinks=1 session-bytes={} asset-bytes={} alloc-total={} generation={} outcome=ok",
            mounted_entries,
            session_payload.len(),
            asset_payload.len(),
            prepared.allocation_total_blocks,
            unmounted_generation
        ),
    )
    .is_err()
    {
        return 527;
    }

    match runtime.unmount_storage_volume("/persist") {
        Err(Errno::NoEnt) => {}
        _ => return 528,
    }
    if write_line(
        runtime,
        "storage.smoke.mount.refusal op=unmount errno=ENOENT outcome=expected",
    )
    .is_err()
    {
        return 529;
    }

    if write_line(runtime, "storage-commit-smoke-ok").is_err() {
        return 509;
    }
    0
}

#[inline(never)]
pub(crate) fn run_native_storage_recover_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let prepared = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 510,
    };
    if prepared.valid == 0 || prepared.dirty != 0 {
        return 511;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.prepared generation={} dirty={} tag={} payload-len={} state={}",
            prepared.generation,
            prepared.dirty,
            fixed_text_field(&prepared.last_commit_tag),
            prepared.payload_len,
            fixed_text_field(&prepared.state_label)
        ),
    )
    .is_err()
    {
        return 512;
    }
    let mounted_entries = match runtime.mount_storage_volume("/dev/storage0", "/persist") {
        Ok(value) => value,
        Err(_) => return 530,
    };
    if mounted_entries != 11 {
        return 531;
    }
    let session_fd = match runtime.open_path("/persist/current-session") {
        Ok(fd) => fd,
        Err(_) => return 532,
    };
    let mut payload = [0u8; 64];
    let payload_len = match runtime.read(session_fd, &mut payload) {
        Ok(value) => value,
        Err(_) => {
            let _ = runtime.close(session_fd);
            return 533;
        }
    };
    if runtime.close(session_fd).is_err() {
        return 534;
    }
    let payload_text = match core::str::from_utf8(&payload[..payload_len]) {
        Ok(value) => value,
        Err(_) => return 535,
    };
    if payload_text != "persist:qemu-vfs-session-001" {
        return 536;
    }
    let mut link_buffer = [0u8; 64];
    let link_len = match runtime.readlink_path("/persist/current-session", &mut link_buffer) {
        Ok(value) => value,
        Err(_) => return 585,
    };
    let link_target = match core::str::from_utf8(&link_buffer[..link_len]) {
        Ok(value) => value,
        Err(_) => return 587,
    };
    if link_target != "/persist/config/session.txt" {
        return 586;
    }
    let asset_fd = match runtime.open_path("/persist/assets/asset.bin") {
        Ok(fd) => fd,
        Err(_) => return 553,
    };
    let mut asset_buffer = [0u8; 1024];
    let asset_len = match runtime.read(asset_fd, &mut asset_buffer) {
        Ok(value) => value,
        Err(_) => {
            let _ = runtime.close(asset_fd);
            return 554;
        }
    };
    if runtime.close(asset_fd).is_err() {
        return 555;
    }
    if asset_len != 900 || asset_buffer[..asset_len].iter().any(|byte| *byte != b'A') {
        return 556;
    }
    let mounted_record = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 557,
    };
    if write_line(
        runtime,
        &format!(
            "storage.smoke.mount.inspect files={} dirs={} symlinks={} alloc-total={} alloc-used={} extents={}",
            mounted_record.mapped_file_count,
            mounted_record.mapped_directory_count,
            mounted_record.mapped_symlink_count,
            mounted_record.allocation_total_blocks,
            mounted_record.allocation_used_blocks,
            mounted_record.mapped_extent_count
        ),
    )
    .is_err()
    {
        return 588;
    }
    if mounted_record.allocation_total_blocks <= 8
        || mounted_record.allocation_used_blocks < 10
        || mounted_record.mapped_file_count != 8
        || mounted_record.mapped_directory_count != 2
        || mounted_record.mapped_symlink_count != 1
        || mounted_record.mapped_extent_count != 10
    {
        return 558;
    }
    let final_generation = match runtime.unmount_storage_volume("/persist") {
        Ok(value) => value,
        Err(_) => return 537,
    };
    if final_generation < prepared.generation as usize {
        return 538;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.mount.recovery mount=/persist entries={} files=8 dirs=2 symlinks=1 payload={} asset-bytes={} alloc-total={} alloc-used={} extents={} generation={} outcome=ok",
            mounted_entries,
            payload_text,
            asset_len,
            mounted_record.allocation_total_blocks,
            mounted_record.allocation_used_blocks,
            mounted_record.mapped_extent_count,
            final_generation
        ),
    )
    .is_err()
    {
        return 539;
    }
    if write_line(runtime, "storage-recover-smoke-ok").is_err() {
        return 517;
    }
    0
}

#[inline(never)]
pub(crate) fn run_native_storage_corrupt_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let volume = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 560,
    };
    if volume.valid == 0
        || volume.alloc_sector == 0
        || volume.mapped_file_count != 8
        || volume.mapped_directory_count != 2
        || volume.mapped_symlink_count != 1
    {
        return 561;
    }
    let zero_sector = [0u8; 512];
    if storage_submit_block_write(runtime, "/dev/storage0", volume.alloc_sector, &zero_sector)
        .is_err()
    {
        return 562;
    }
    let request = match storage_complete_driver_request(
        runtime,
        "/drv/storage0",
        b"alloc-sector-corrupted",
    ) {
        Ok(request) => request,
        Err(_) => return 563,
    };
    if request.op != ngos_user_abi::NATIVE_BLOCK_IO_OP_WRITE
        || request.sector != volume.alloc_sector
    {
        return 564;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.corruption sector={} kind=alloc-bitmap outcome=written",
            volume.alloc_sector
        ),
    )
    .is_err()
    {
        return 565;
    }

    match runtime.mount_storage_volume("/dev/storage0", "/persist") {
        Err(Errno::Inval) => {}
        _ => return 566,
    }
    if write_line(
        runtime,
        "storage.smoke.corruption.refusal op=mount errno=EINVAL outcome=expected",
    )
    .is_err()
    {
        return 567;
    }

    let repaired_generation = match runtime.repair_storage_snapshot("/dev/storage0") {
        Ok(value) => value,
        Err(_) => return 568,
    };
    let repaired = match runtime.inspect_storage_volume("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 569,
    };
    if repaired.generation != repaired_generation as u64
        || fixed_text_field(&repaired.last_commit_tag) != "storage-repair"
        || repaired.allocation_total_blocks <= 8
        || repaired.allocation_used_blocks < 10
        || repaired.mapped_file_count != 8
        || repaired.mapped_directory_count != 2
        || repaired.mapped_symlink_count != 1
        || repaired.mapped_extent_count != 10
    {
        return 570;
    }
    if write_line(
        runtime,
        &format!(
            "storage.smoke.corruption.repair generation={} alloc-total={} alloc-used={} files={} dirs={} symlinks={} extents={} outcome=ok",
            repaired.generation,
            repaired.allocation_total_blocks,
            repaired.allocation_used_blocks,
            repaired.mapped_file_count,
            repaired.mapped_directory_count,
            repaired.mapped_symlink_count,
            repaired.mapped_extent_count
        ),
    )
    .is_err()
    {
        return 571;
    }

    let remounted_entries = match runtime.mount_storage_volume("/dev/storage0", "/persist") {
        Ok(value) => value,
        Err(_) => return 572,
    };
    if remounted_entries != 11 {
        return 573;
    }
    let session_fd = match runtime.open_path("/persist/current-session") {
        Ok(fd) => fd,
        Err(_) => return 574,
    };
    let mut payload = [0u8; 64];
    let payload_len = match runtime.read(session_fd, &mut payload) {
        Ok(value) => value,
        Err(_) => {
            let _ = runtime.close(session_fd);
            return 575;
        }
    };
    if runtime.close(session_fd).is_err() {
        return 576;
    }
    let payload_text = match core::str::from_utf8(&payload[..payload_len]) {
        Ok(value) => value,
        Err(_) => return 577,
    };
    if payload_text != "persist:qemu-vfs-session-001" {
        return 578;
    }
    let final_generation = match runtime.unmount_storage_volume("/persist") {
        Ok(value) => value,
        Err(_) => return 579,
    };
    if write_line(
        runtime,
        &format!(
            "storage.smoke.corruption.recovery mount=/persist entries={} payload={} generation={} outcome=ok",
            remounted_entries,
            payload_text,
            final_generation
        ),
    )
    .is_err()
    {
        return 580;
    }
    if write_line(runtime, "storage-corrupt-smoke-ok").is_err() {
        return 581;
    }
    0
}
