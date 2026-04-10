use super::*;

const VFS_SMOKE_PID: u64 = 1;
const VFS_ROOT: &str = "/vfs";
const VFS_BIN: &str = "/vfs/bin";
const VFS_APP: &str = "/vfs/bin/app";
const VFS_APP_COPY: &str = "/vfs/bin/app-copy";
const VFS_LINK: &str = "/vfs/link";

fn vfs_write_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    code: ExitCode,
) -> Result<(), ExitCode> {
    write_line(runtime, line).map_err(|_| code)
}

fn vfs_step<B: SyscallBackend>(
    runtime: &Runtime<B>,
    step: &str,
    code: ExitCode,
) -> Result<(), ExitCode> {
    let _ = (runtime, step, code);
    Ok(())
}

fn mkdir_if_missing<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    code: ExitCode,
) -> Result<(), ExitCode> {
    match runtime.mkdir_path(path) {
        Ok(()) | Err(Errno::Exist) => Ok(()),
        Err(_) if runtime.stat_path(path).is_ok() => Ok(()),
        Err(_) => Err(code),
    }
}

fn mkfile_if_missing<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    code: ExitCode,
) -> Result<(), ExitCode> {
    match runtime.mkfile_path(path) {
        Ok(()) | Err(Errno::Exist) => Ok(()),
        Err(_) if runtime.stat_path(path).is_ok() => Ok(()),
        Err(_) => Err(code),
    }
}

fn path_missing<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> bool {
    matches!(runtime.lstat_path(path), Err(Errno::NoEnt))
}

fn vfs_list_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    code: ExitCode,
) -> Result<String, ExitCode> {
    let mut buffer = [0u8; 2048];
    let count = runtime.list_path(path, &mut buffer).map_err(|_| code)?;
    core::str::from_utf8(&buffer[..count])
        .map(|text| text.to_string())
        .map_err(|_| code)
}

fn vfs_readlink_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    code: ExitCode,
) -> Result<String, ExitCode> {
    let mut buffer = [0u8; 256];
    let count = runtime.readlink_path(path, &mut buffer).map_err(|_| code)?;
    core::str::from_utf8(&buffer[..count])
        .map(|text| text.to_string())
        .map_err(|_| code)
}

fn read_open_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
    buffer: &mut [u8],
    code: ExitCode,
) -> Result<usize, ExitCode> {
    runtime.read(fd, buffer).map_err(|_| code)
}

fn run_vfs_core_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "core", 600)?;
    mkdir_if_missing(runtime, VFS_ROOT, 601)?;
    mkdir_if_missing(runtime, VFS_BIN, 602)?;
    shell_write_file(runtime, VFS_APP, "seed-vfs-app")?;
    runtime.symlink_path(VFS_LINK, VFS_APP).map_err(|_| 603)?;

    let fs = runtime.statfs_path(VFS_ROOT).map_err(|_| 604)?;
    let app = runtime.stat_path(VFS_APP).map_err(|_| 605)?;
    let link = runtime.lstat_path(VFS_LINK).map_err(|_| 606)?;
    let link_target = vfs_readlink_text(runtime, VFS_LINK, 607)?;
    if link_target != VFS_APP {
        return Err(608);
    }

    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.mount pid={} path={} mounts={} nodes={} read_only={} outcome=ok",
            VFS_SMOKE_PID, VFS_ROOT, fs.mount_count, fs.node_count, fs.read_only
        ),
        609,
    )?;
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.create pid={} path={} kind={} inode={} outcome=ok",
            VFS_SMOKE_PID, VFS_APP, app.kind, app.inode
        ),
        610,
    )?;
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.symlink pid={} path={} target={} kind={} inode={} outcome=ok",
            VFS_SMOKE_PID, VFS_LINK, VFS_APP, link.kind, link.inode
        ),
        611,
    )?;

    runtime
        .rename_path(VFS_APP, "/vfs/bin/app2")
        .map_err(|_| 612)?;
    if runtime.stat_path("/vfs/bin/app2").is_err() || runtime.open_path("/vfs/bin/app2").is_err() {
        return Err(613);
    }
    if runtime.rename_path(VFS_BIN, "/vfs/bin/subdir").is_ok() {
        return Err(614);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.rename pid={} from={} to=/vfs/bin/app2 refusal=invalid-subtree yes outcome=ok",
            VFS_SMOKE_PID, VFS_APP
        ),
        615,
    )?;

    runtime.unlink_path(VFS_LINK).map_err(|_| 616)?;
    if !path_missing(runtime, VFS_LINK) {
        return Err(617);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.unlink pid={} path={} after-unlink=missing outcome=ok",
            VFS_SMOKE_PID, VFS_LINK
        ),
        618,
    )?;

    runtime.symlink_path(VFS_LINK, VFS_APP).map_err(|_| 619)?;
    runtime
        .rename_path("/vfs/bin/app2", VFS_APP)
        .map_err(|_| 620)?;
    if vfs_readlink_text(runtime, VFS_LINK, 621)? != VFS_APP {
        return Err(622);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.recovery pid={} path={} target={} rename-restored=yes readlink=stable outcome=ok",
            VFS_SMOKE_PID, VFS_LINK, VFS_APP
        ),
        623,
    )?;

    let missing_parent = matches!(
        runtime.mkfile_path("/vfs/missing-parent/child"),
        Err(Errno::NoEnt)
    );
    let unlink_nonempty_dir = runtime.unlink_path(VFS_BIN).is_err();
    if !missing_parent || !unlink_nonempty_dir {
        return Err(624);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.refusal pid={} create-missing-parent=yes unlink-nonempty-dir=yes outcome=ok",
            VFS_SMOKE_PID
        ),
        625,
    )?;

    runtime
        .symlink_path("/vfs/loop-a", "/vfs/loop-b")
        .map_err(|_| 626)?;
    runtime
        .symlink_path("/vfs/loop-b", "/vfs/loop-a")
        .map_err(|_| 627)?;
    if runtime.open_path("/vfs/loop-a").is_ok() {
        return Err(628);
    }
    runtime.unlink_path("/vfs/loop-a").map_err(|_| 629)?;
    runtime.unlink_path("/vfs/loop-b").map_err(|_| 630)?;
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.symlink-loop pid={} refusal=loop-detected yes recovery=unlink outcome=ok",
            VFS_SMOKE_PID
        ),
        631,
    )?;

    Ok(())
}

fn run_vfs_file_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "file", 632)?;
    shell_append_file(runtime, VFS_APP, "-ok!")?;
    mkfile_if_missing(runtime, VFS_APP_COPY, 633)?;
    shell_copy_file(runtime, VFS_APP, VFS_APP_COPY)?;
    let app_text = shell_read_file_text(runtime, VFS_APP)?;
    let copy_text = shell_read_file_text(runtime, VFS_APP_COPY)?;
    if app_text.len() != 16 || app_text != copy_text {
        return Err(634);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.file pid={} path={} copy={} bytes={} append=yes copy-match=yes outcome=ok",
            VFS_SMOKE_PID,
            VFS_APP,
            VFS_APP_COPY,
            app_text.len()
        ),
        635,
    )?;
    Ok(())
}

fn run_vfs_link_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "link", 636)?;
    let link_path = "/vfs/bin/app-link";
    runtime.link_path(VFS_APP, link_path).map_err(|_| 637)?;
    let before = runtime.stat_path(VFS_APP).map_err(|_| 638)?;
    let linked = runtime.stat_path(link_path).map_err(|_| 639)?;
    if before.link_count < 2 || before.inode != linked.inode {
        return Err(640);
    }
    shell_append_file(runtime, link_path, "!")?;
    if !shell_read_file_text(runtime, VFS_APP)?.ends_with('!') {
        return Err(641);
    }
    runtime.unlink_path(link_path).map_err(|_| 642)?;
    let after = runtime.stat_path(VFS_APP).map_err(|_| 643)?;
    if after.link_count != 1 || !path_missing(runtime, link_path) {
        return Err(644);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.link pid={} source={} link={} shared-inode=yes shared-write=yes links-before={} links-after={} unlink-released=yes outcome=ok",
            VFS_SMOKE_PID, VFS_APP, link_path, before.link_count, after.link_count
        ),
        645,
    )?;
    Ok(())
}

fn run_vfs_truncate_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "truncate", 646)?;
    runtime.truncate_path(VFS_APP_COPY, 5).map_err(|_| 647)?;
    runtime.truncate_path(VFS_APP_COPY, 8).map_err(|_| 648)?;
    let fd = runtime.open_path(VFS_APP_COPY).map_err(|_| 649)?;
    let mut buffer = [0u8; 8];
    let count = read_open_file(runtime, fd, &mut buffer, 650)?;
    runtime.close(fd).map_err(|_| 651)?;
    if count != 8 || &buffer[..5] != b"seed-" || buffer[5] != 0 || buffer[6] != 0 || buffer[7] != 0
    {
        return Err(652);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.truncate pid={} path={} shrink=5 extend=8 zero-fill=yes outcome=ok",
            VFS_SMOKE_PID, VFS_APP_COPY
        ),
        653,
    )?;
    Ok(())
}

fn run_vfs_list_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "list", 654)?;
    let listing = vfs_list_text(runtime, VFS_BIN, 655)?;
    let mut saw_app = false;
    let mut saw_copy = false;
    let mut entries = 0usize;
    for line in listing.lines() {
        let entry = line.split('\t').next().unwrap_or("").trim();
        if entry.is_empty() {
            continue;
        }
        entries += 1;
        if entry == "app" {
            saw_app = true;
        }
        if entry == "app-copy" {
            saw_copy = true;
        }
    }
    if entries != 2 || !saw_app || !saw_copy {
        return Err(656);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.list pid={} path={} entries=2 names=app,app-copy outcome=ok",
            VFS_SMOKE_PID, VFS_BIN
        ),
        657,
    )?;
    Ok(())
}

fn run_vfs_unlink_open_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "unlink-open", 658)?;
    let path = "/vfs/bin/live";
    shell_write_file(runtime, path, "live-data")?;
    let fd = runtime.open_path(path).map_err(|_| 659)?;
    runtime.unlink_path(path).map_err(|_| 660)?;
    let mut buffer = [0u8; 32];
    let count = read_open_file(runtime, fd, &mut buffer, 663)?;
    let deleted_fdinfo =
        read_procfs_all(runtime, &format!("/proc/{}/fdinfo/{}", VFS_SMOKE_PID, fd))
            .map_err(|_| 664)?;
    runtime.close(fd).map_err(|_| 664)?;
    if &buffer[..count] != b"live-data"
        || !core::str::from_utf8(&deleted_fdinfo)
            .map(|text| text.contains("(deleted)"))
            .unwrap_or(false)
    {
        return Err(665);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.unlink-open pid={} path={} fd={} retained=expected outcome=ok",
            VFS_SMOKE_PID, path, fd
        ),
        666,
    )?;
    Ok(())
}

fn run_vfs_vm_file_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "vm-file", 667)?;
    let path = "/vfs/bin/vm-file";
    shell_write_file(runtime, path, "abcdwxyz")?;
    let mapped = runtime
        .map_file_memory(VFS_SMOKE_PID, path, 0x1000, 0, true, true, false, true)
        .map_err(|_| 668)?;
    if runtime
        .load_memory_word(VFS_SMOKE_PID, mapped)
        .map_err(|_| 669)?
        != u32::from_le_bytes(*b"abcd")
    {
        return Err(670);
    }
    runtime
        .store_memory_word(VFS_SMOKE_PID, mapped + 4, u32::from_le_bytes(*b"4321"))
        .map_err(|_| 671)?;
    runtime
        .sync_memory_range(VFS_SMOKE_PID, mapped, 0x1000)
        .map_err(|_| 672)?;
    if shell_read_file_text(runtime, path)? != "abcd4321" {
        return Err(673);
    }
    runtime.truncate_path(path, 2).map_err(|_| 674)?;
    if runtime
        .load_memory_word(VFS_SMOKE_PID, mapped + 4)
        .map_err(|_| 675)?
        != 0
    {
        return Err(676);
    }
    runtime.unlink_path(path).map_err(|_| 677)?;
    if runtime
        .load_memory_word(VFS_SMOKE_PID, mapped)
        .map_err(|_| 678)?
        != u32::from_le_bytes([b'a', b'b', 0, 0])
    {
        return Err(679);
    }
    runtime
        .sync_memory_range(VFS_SMOKE_PID, mapped, 0x1000)
        .map_err(|_| 680)?;
    runtime
        .unmap_memory_range(VFS_SMOKE_PID, mapped, 0x1000)
        .map_err(|_| 681)?;
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.vm-file pid={} path={} sync=yes truncate-reflects=yes unlink-survives=yes unmap=yes outcome=ok",
            VFS_SMOKE_PID, path
        ),
        682,
    )?;
    Ok(())
}

fn run_vfs_permissions_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "permissions", 683)?;
    let dir = "/vfs/secure";
    let file = "/vfs/secure/secret.txt";
    mkdir_if_missing(runtime, dir, 684)?;
    shell_write_file(runtime, file, "secret-data")?;
    runtime.chmod_path(file, 0o000).map_err(|_| 685)?;
    runtime.chmod_path(dir, 0o600).map_err(|_| 686)?;

    let list_blocked = runtime.list_path(dir, &mut [0u8; 64]).is_err();
    let traverse_blocked = matches!(runtime.open_path(file), Err(Errno::Access));
    let rename_blocked = matches!(
        runtime.rename_path(file, "/vfs/secure/secret-2.txt"),
        Err(Errno::Access)
    );
    let unlink_blocked = matches!(runtime.unlink_path(file), Err(Errno::Access));
    if !list_blocked || !traverse_blocked || !rename_blocked || !unlink_blocked {
        return Err(687);
    }

    runtime.chmod_path(dir, 0o700).map_err(|_| 688)?;
    let file_read_blocked = matches!(runtime.open_path(file), Err(Errno::Access));
    if !file_read_blocked {
        return Err(689);
    }
    runtime.chmod_path(file, 0o600).map_err(|_| 690)?;
    if shell_read_file_text(runtime, file)? != "secret-data" {
        return Err(691);
    }

    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.permissions pid={} dir={} file={} list-blocked=yes traverse-blocked=yes rename-blocked=yes unlink-blocked=yes file-read-blocked=yes recovery=yes outcome=ok",
            VFS_SMOKE_PID, dir, file
        ),
        692,
    )?;
    Ok(())
}

fn run_vfs_replace_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "replace", 693)?;
    let source = "/vfs/bin/replace-src";
    let target = "/vfs/bin/replace-dst";
    shell_write_file(runtime, source, "new-bytes")?;
    shell_write_file(runtime, target, "old-bytes")?;
    let target_fd = runtime.open_path(target).map_err(|_| 694)?;
    runtime
        .seek(target_fd, 0, SeekWhence::Set)
        .map_err(|_| 695)?;
    runtime.rename_path(source, target).map_err(|_| 696)?;
    if shell_read_file_text(runtime, target)? != "new-bytes" {
        return Err(697);
    }
    let mut target_buffer = [0u8; 16];
    let count = read_open_file(runtime, target_fd, &mut target_buffer, 698)?;
    runtime.close(target_fd).map_err(|_| 699)?;
    if &target_buffer[..count] != b"old-bytes" {
        return Err(700);
    }

    mkdir_if_missing(runtime, "/vfs/bin/replace-dir-src", 701)?;
    mkdir_if_missing(runtime, "/vfs/bin/replace-dir-dst", 702)?;
    shell_write_file(runtime, "/vfs/bin/replace-dir-dst/child", "child")?;
    let nonempty_dir_refusal = matches!(
        runtime.rename_path("/vfs/bin/replace-dir-src", "/vfs/bin/replace-dir-dst"),
        Err(Errno::Busy)
    );

    mkdir_if_missing(runtime, "/vfs/bin/replace-dir-empty-src", 703)?;
    mkdir_if_missing(runtime, "/vfs/bin/replace-dir-empty-dst", 704)?;
    runtime
        .rename_path(
            "/vfs/bin/replace-dir-empty-src",
            "/vfs/bin/replace-dir-empty-dst",
        )
        .map_err(|_| 705)?;
    let empty_dir_replaced = path_missing(runtime, "/vfs/bin/replace-dir-empty-src")
        && runtime.stat_path("/vfs/bin/replace-dir-empty-dst").is_ok();

    mkdir_if_missing(runtime, "/vfs/bin/replace-kind-dir", 706)?;
    shell_write_file(runtime, "/vfs/bin/replace-kind-file", "kind")?;
    let kind_mismatch_refusal = matches!(
        runtime.rename_path("/vfs/bin/replace-kind-dir", "/vfs/bin/replace-kind-file"),
        Err(Errno::Busy | Errno::Inval)
    ) && matches!(
        runtime.rename_path("/vfs/bin/replace-kind-file", "/vfs/bin/replace-kind-dir"),
        Err(Errno::Busy | Errno::Inval)
    );

    if !nonempty_dir_refusal || !empty_dir_replaced || !kind_mismatch_refusal {
        return Err(707);
    }

    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.replace pid={} source={} target={} file-replaced=yes open-target-survives=yes nonempty-dir-refusal=yes empty-dir-replaced=yes kind-mismatch-refusal=yes outcome=ok",
            VFS_SMOKE_PID, source, target
        ),
        708,
    )?;
    Ok(())
}

fn run_vfs_tree_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "tree", 709)?;
    let source = "/vfs/tree-src";
    let nested = "/vfs/tree-src/nested";
    let copy = "/vfs/tree-dst";
    let mirror = "/vfs/tree-mirror";
    mkdir_if_missing(runtime, source, 710)?;
    mkdir_if_missing(runtime, nested, 711)?;
    shell_write_file(runtime, "/vfs/tree-src/base.txt", "base")?;
    shell_write_file(runtime, "/vfs/tree-src/nested/leaf.txt", "leaf")?;
    runtime
        .symlink_path("/vfs/tree-src/link.txt", "/vfs/tree-src/nested/leaf.txt")
        .map_err(|_| 712)?;

    match workflow_agents::try_handle_workflow_agent_command(
        runtime,
        "/",
        &format!("copy-tree {source} {copy} 4"),
    ) {
        Some(Ok(())) => {}
        _ => return Err(713),
    }
    if shell_read_file_text(runtime, "/vfs/tree-dst/base.txt")? != "base"
        || shell_read_file_text(runtime, "/vfs/tree-dst/nested/leaf.txt")? != "leaf"
    {
        return Err(714);
    }
    if vfs_readlink_text(runtime, "/vfs/tree-dst/link.txt", 715)? != "/vfs/tree-src/nested/leaf.txt"
    {
        return Err(716);
    }
    let self_nest_refusal = !matches!(
        workflow_agents::try_handle_workflow_agent_command(
            runtime,
            "/",
            "copy-tree /vfs/tree-src /vfs/tree-src/nested/inside 4"
        ),
        Some(Ok(()))
    );

    mkdir_if_missing(runtime, mirror, 717)?;
    shell_write_file(runtime, "/vfs/tree-mirror/stale.txt", "stale")?;
    shell_write_file(runtime, "/vfs/tree-src/base.txt", "base-updated")?;
    match workflow_agents::try_handle_workflow_agent_command(
        runtime,
        "/",
        &format!("mirror-tree {source} {mirror} 4"),
    ) {
        Some(Ok(())) => {}
        _ => return Err(718),
    }
    let pruned = path_missing(runtime, "/vfs/tree-mirror/stale.txt");
    if shell_read_file_text(runtime, "/vfs/tree-mirror/base.txt")? != "base-updated"
        || !self_nest_refusal
        || !pruned
        || vfs_readlink_text(runtime, "/vfs/tree-mirror/link.txt", 719)?
            != "/vfs/tree-src/nested/leaf.txt"
    {
        return Err(720);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.tree pid={} source={} copy={} mirror={} refusal=self-nest yes symlink=stable pruned=yes outcome=ok",
            VFS_SMOKE_PID, source, copy, mirror
        ),
        721,
    )?;
    Ok(())
}

fn run_vfs_mount_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "mount-propagation", 722)?;
    let shared = "/vfs/mount-shared";
    let peer = "/vfs/mount-peer";
    let child = "/vfs/mount-shared/child";
    let clone = "/vfs/mount-peer/child";
    runtime
        .mount_storage_volume("/dev/storage0", shared)
        .map_err(|_| 723)?;
    runtime
        .mount_storage_volume("/dev/storage0", peer)
        .map_err(|_| 724)?;
    runtime
        .set_mount_propagation(shared, NativeMountPropagationMode::Shared)
        .map_err(|_| 725)?;
    runtime
        .set_mount_propagation(peer, NativeMountPropagationMode::Slave)
        .map_err(|_| 726)?;
    runtime
        .mount_storage_volume("/dev/storage0", child)
        .map_err(|_| 727)?;
    let child_mount = runtime.inspect_mount(child).map_err(|_| 728)?;
    let clone_mount = runtime.inspect_mount(clone).map_err(|_| 729)?;
    if NativeMountPropagationMode::from_raw(child_mount.propagation_mode)
        != Some(NativeMountPropagationMode::Shared)
        || NativeMountPropagationMode::from_raw(clone_mount.propagation_mode)
            != Some(NativeMountPropagationMode::Slave)
        || clone_mount.master_group != child_mount.peer_group
    {
        return Err(730);
    }

    shell_write_file(runtime, "/vfs/mount-shared/child/file.txt", "mounted")?;
    let cross_mount_rename = matches!(
        runtime.rename_path(
            "/vfs/mount-shared/child/file.txt",
            "/vfs/mount-peer/child/file.txt"
        ),
        Err(Errno::Busy)
    );
    let cross_mount_link = matches!(
        runtime.link_path(
            "/vfs/mount-shared/child/file.txt",
            "/vfs/mount-peer/child/link.txt"
        ),
        Err(Errno::Busy)
    );
    let parent_unmount_blocked = matches!(runtime.unmount_storage_volume(shared), Err(Errno::Busy));
    runtime.unmount_storage_volume(child).map_err(|_| 731)?;
    if runtime.inspect_mount(clone) != Err(Errno::NoEnt) {
        return Err(732);
    }
    runtime.unmount_storage_volume(shared).map_err(|_| 733)?;
    if runtime.inspect_mount(peer) != Err(Errno::NoEnt) {
        return Err(734);
    }
    if !cross_mount_rename || !cross_mount_link || !parent_unmount_blocked {
        return Err(735);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.mount-propagation pid={} shared={} peer={} child={} clone={} cross-mount-rename=blocked cross-mount-link=blocked parent-unmount-blocked=yes recovery=yes outcome=ok",
            VFS_SMOKE_PID, shared, peer, child, clone
        ),
        736,
    )?;
    Ok(())
}

fn run_vfs_fd_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "fd", 737)?;
    let path = "/vfs/fd-target";
    shell_write_file(runtime, path, "abcdef")?;

    let fd = runtime.open_path(path).map_err(|_| 738)?;
    let mut probe = [0u8; 8];
    let first = read_open_file(runtime, fd, &mut probe, 739)?;
    if &probe[..first] != b"abcdef" {
        return Err(740);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.fd pid={} fd={} path={} outcome=ok",
            VFS_SMOKE_PID, fd, path
        ),
        741,
    )?;

    let dup_fd = runtime.dup(fd).map_err(|_| 742)?;
    if runtime.seek(dup_fd, 0, SeekWhence::End).map_err(|_| 743)? != 6 {
        return Err(744);
    }
    runtime.write(dup_fd, b"g").map_err(|_| 745)?;
    runtime.seek(fd, 0, SeekWhence::Set).map_err(|_| 746)?;
    let mut full = [0u8; 8];
    let second = read_open_file(runtime, fd, &mut full, 747)?;
    if &full[..second] != b"abcdefg" {
        return Err(748);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.dup pid={} fd={} dup-fd={} shared-offset=yes outcome=ok",
            VFS_SMOKE_PID, fd, dup_fd
        ),
        749,
    )?;

    runtime
        .fcntl(dup_fd, FcntlCmd::SetFl { nonblock: true })
        .map_err(|_| 750)?;
    runtime
        .fcntl(dup_fd, FcntlCmd::SetFd { cloexec: true })
        .map_err(|_| 751)?;
    let getfl = runtime.fcntl(dup_fd, FcntlCmd::GetFl).map_err(|_| 752)?;
    let getfd = runtime.fcntl(dup_fd, FcntlCmd::GetFd).map_err(|_| 753)?;
    if getfl == 0 || getfd == 0 {
        return Err(754);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.fcntl pid={} fd={} nonblock=yes cloexec=yes outcome=ok",
            VFS_SMOKE_PID, dup_fd
        ),
        755,
    )?;
    runtime.close(dup_fd).map_err(|_| 756)?;
    runtime.close(fd).map_err(|_| 757)?;
    Ok(())
}

fn run_vfs_lock_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "lock", 758)?;
    let path = "/vfs/locked.txt";
    shell_write_file(runtime, path, "locked")?;
    let primary = runtime.open_path(path).map_err(|_| 759)?;
    let secondary = runtime.open_path(path).map_err(|_| 760)?;
    runtime
        .fcntl(primary, FcntlCmd::TryLockShared { token: 0x11 })
        .map_err(|_| 761)?;
    runtime
        .fcntl(secondary, FcntlCmd::TryLockShared { token: 0x22 })
        .map_err(|_| 762)?;
    let shared_refusal = matches!(
        runtime.fcntl(primary, FcntlCmd::TryLockExclusive { token: 0x33 }),
        Err(Errno::Busy)
    );
    let mutation_blocked = matches!(runtime.write(secondary, b"x"), Err(Errno::Busy))
        && matches!(runtime.truncate_path(path, 1), Err(Errno::Busy))
        && matches!(
            runtime.rename_path(path, "/vfs/locked-renamed.txt"),
            Err(Errno::Busy)
        )
        && matches!(
            runtime.link_path(path, "/vfs/locked-link.txt"),
            Err(Errno::Busy)
        )
        && matches!(runtime.unlink_path(path), Err(Errno::Busy));
    runtime
        .fcntl(primary, FcntlCmd::UnlockShared { token: 0x11 })
        .map_err(|_| 763)?;
    runtime
        .fcntl(secondary, FcntlCmd::UnlockShared { token: 0x22 })
        .map_err(|_| 764)?;
    runtime
        .fcntl(primary, FcntlCmd::TryLockExclusive { token: 0x44 })
        .map_err(|_| 765)?;
    runtime
        .fcntl(primary, FcntlCmd::UnlockExclusive { token: 0x44 })
        .map_err(|_| 766)?;
    runtime
        .link_path(path, "/vfs/locked-link.txt")
        .map_err(|_| 767)?;
    runtime
        .unlink_path("/vfs/locked-link.txt")
        .map_err(|_| 768)?;
    runtime.close(secondary).map_err(|_| 769)?;
    runtime.close(primary).map_err(|_| 770)?;
    if !shared_refusal || !mutation_blocked {
        return Err(771);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.lock pid={} primary-fd={} secondary-fd={} shared=yes shared-refusal=busy mutation-blocked=yes mutation-recovery=yes shared-recovery=yes outcome=ok",
            VFS_SMOKE_PID, primary, secondary
        ),
        772,
    )?;
    Ok(())
}

fn run_vfs_coherence_phase<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    vfs_step(runtime, "coherence", 773)?;
    let app_fd = runtime.open_path(VFS_APP).map_err(|_| 774)?;
    let link_fd = runtime.open_path(VFS_LINK).map_err(|_| 775)?;
    let statfs = runtime.statfs_path(VFS_ROOT).map_err(|_| 776)?;
    let readlink = vfs_readlink_text(runtime, VFS_LINK, 777)?;
    let app_text = shell_read_file_text(runtime, VFS_APP)?;
    let link_text = shell_read_file_text(runtime, VFS_LINK)?;
    runtime.close(link_fd).map_err(|_| 778)?;
    runtime.close(app_fd).map_err(|_| 779)?;
    if readlink != VFS_APP || app_text != link_text || statfs.node_count == 0 {
        return Err(780);
    }
    vfs_write_line(
        runtime,
        &format!(
            "vfs.smoke.coherence pid={} descriptor=open-path-open readlink=stable statfs=ok outcome=ok",
            VFS_SMOKE_PID
        ),
        781,
    )?;
    Ok(())
}

pub(crate) fn run_native_vfs_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    if let Err(code) = run_vfs_core_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_file_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_link_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_truncate_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_list_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_unlink_open_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_vm_file_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_permissions_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_replace_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_tree_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_mount_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_fd_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_lock_phase(runtime) {
        return code;
    }
    if let Err(code) = run_vfs_coherence_phase(runtime) {
        return code;
    }
    if vfs_write_line(runtime, "vfs-smoke-ok", 782).is_err() {
        return 782;
    }
    0
}
