use super::*;

fn vm_object_id_for_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    procfs_path: &str,
    path_marker: &str,
) -> Result<u64, ExitCode> {
    let bytes = read_procfs_all(runtime, procfs_path).map_err(|_| 270)?;
    let text = String::from_utf8(bytes).map_err(|_| 271)?;
    for line in text.lines() {
        if line.contains(path_marker) {
            let Some((object_id, _)) = line.split_once('\t') else {
                continue;
            };
            let value = u64::from_str_radix(object_id, 16).map_err(|_| 272)?;
            return Ok(value);
        }
    }
    Err(273)
}

fn boot_bind_vm_memory_contract<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<usize, ExitCode> {
    let domain = runtime
        .create_domain(None, "boot-vm-memory")
        .map_err(|_| 308)?;
    let resource = runtime
        .create_resource(domain, NativeResourceKind::Memory, "boot-vm-memory")
        .map_err(|_| 308)?;
    runtime
        .set_resource_contract_policy(resource, NativeResourceContractPolicy::Memory)
        .map_err(|_| 308)?;
    let contract = runtime
        .create_contract(
            domain,
            resource,
            NativeContractKind::Memory,
            "boot-vm-memory",
        )
        .map_err(|_| 308)?;
    runtime.bind_process_contract(contract).map_err(|_| 308)?;
    Ok(contract)
}

pub(crate) fn ensure_vm_smoke_backing_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    byte_len: usize,
    fill: u8,
    open_error: ExitCode,
    write_error: ExitCode,
    close_error: ExitCode,
) -> Result<(), ExitCode> {
    if runtime.mkdir_path("/lib").is_err() && runtime.stat_path("/lib").is_err() {
        return Err(open_error);
    }
    if runtime.mkfile_path(path).is_err() && runtime.stat_path(path).is_err() {
        return Err(open_error);
    }
    let fd = runtime.open_path(path).map_err(|_| open_error)?;
    let chunk = [fill; 256];
    let mut written = 0usize;
    while written < byte_len {
        if runtime.write(fd, &chunk).is_err() {
            let _ = runtime.close(fd);
            return Err(write_error);
        }
        written += chunk.len();
    }
    runtime.close(fd).map_err(|_| close_error)?;
    Ok(())
}

pub(crate) fn run_vm_stress_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    let mut cycles = 0u64;
    let mut refusals = 0u64;
    while cycles < 12 {
        let label = format!("boot-vm-stress-{cycles}");
        let start = runtime
            .map_anonymous_memory(pid, 0x2000, true, true, false, &label)
            .map_err(|_| 206)?;
        runtime
            .store_memory_word(pid, start, (cycles + 1) as u32)
            .map_err(|_| 207)?;
        runtime
            .protect_memory_range(pid, start + 0x1000, 0x1000, true, false, false)
            .map_err(|_| 208)?;
        if runtime.store_memory_word(pid, start + 0x1000, 0x55) != Err(Errno::Fault) {
            return Err(209);
        }
        refusals += 1;
        runtime
            .unmap_memory_range(pid, start + 0x1000, 0x1000)
            .map_err(|_| 210)?;
        runtime
            .unmap_memory_range(pid, start, 0x1000)
            .map_err(|_| 211)?;
        cycles += 1;
    }

    write_line(
        runtime,
        &format!("vm.smoke.stress pid={pid} cycles={cycles} refusals={refusals} outcome=ok"),
    )
}

pub(crate) fn run_vm_pressure_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    ensure_vm_smoke_backing_file(
        runtime,
        "/lib/libvm-pressure.so",
        0x4000,
        0x37,
        212,
        213,
        214,
    )?;
    let mapped = runtime
        .map_file_memory(
            pid,
            "/lib/libvm-pressure.so",
            0x3000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 215)?;
    runtime
        .protect_memory_range(pid, mapped, 0x3000, true, true, false)
        .map_err(|_| 216)?;
    for (page, value) in [(0u64, 31u32), (0x1000, 32u32), (0x2000, 33u32)] {
        runtime
            .store_memory_word(pid, mapped + page, value)
            .map_err(|_| 217)?;
    }

    let target_pages = 3u64;
    let reclaimed = runtime
        .reclaim_memory_pressure(pid, target_pages)
        .map_err(|_| 218)?;
    if reclaimed < 3 {
        return Err(219);
    }

    let vmdecisions = path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &["agent=pressure-trigger", "agent=pressure-victim"],
    )?;
    if !vmdecisions {
        return Err(220);
    }

    let vmobjects = path_contains_all_markers(
        runtime,
        "/proc/1/vmobjects",
        &["/lib/libvm-pressure.so", "resident=0", "dirty=0"],
    )?;
    if !vmobjects {
        return Err(221);
    }

    for page in [0u64, 0x1000, 0x2000] {
        runtime
            .load_memory_word(pid, mapped + page)
            .map_err(|_| 222)?;
    }

    write_line(
        runtime,
        &format!(
            "vm.smoke.pressure pid={pid} target-pages={target_pages} reclaimed-pages={reclaimed} restored=yes outcome=ok"
        ),
    )
}

pub(crate) fn run_vm_global_pressure_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    ensure_vm_smoke_backing_file(
        runtime,
        "/lib/libvm-global-a.so",
        0x4000,
        0x41,
        224,
        225,
        226,
    )?;
    ensure_vm_smoke_backing_file(
        runtime,
        "/lib/libvm-global-b.so",
        0x2000,
        0x52,
        227,
        228,
        229,
    )?;

    let mapped_a = runtime
        .map_file_memory(
            pid,
            "/lib/libvm-global-a.so",
            0x3000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 230)?;
    runtime
        .protect_memory_range(pid, mapped_a, 0x3000, true, true, false)
        .map_err(|_| 231)?;
    for (page, value) in [(0u64, 41u32), (0x1000, 42u32), (0x2000, 43u32)] {
        runtime
            .store_memory_word(pid, mapped_a + page, value)
            .map_err(|_| 232)?;
    }

    let child = runtime
        .spawn_process_copy_vm("g", "/g", pid)
        .map_err(|_| 233)?;
    let mapped_b = runtime
        .map_file_memory(
            child,
            "/lib/libvm-global-b.so",
            0x1000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 234)?;
    runtime
        .protect_memory_range(child, mapped_b, 0x1000, true, true, false)
        .map_err(|_| 235)?;
    runtime
        .store_memory_word(child, mapped_b, 51)
        .map_err(|_| 236)?;

    let reclaimed = runtime.reclaim_memory_pressure_global(3).map_err(|_| 237)?;
    if reclaimed < 3 {
        return Err(238);
    }

    let parent_vmobjects = path_contains_all_markers(
        runtime,
        "/proc/1/vmobjects",
        &["/lib/libvm-global-a.so", "resident=0", "dirty=0"],
    )?;
    if !parent_vmobjects {
        return Err(239);
    }

    let child_decisions = path_contains_all_markers(
        runtime,
        &format!("/proc/{child}/vmdecisions"),
        &["/lib/libvm-global-b.so", "agent=map-file", "agent=protect"],
    )?;
    if !child_decisions {
        return Err(242);
    }

    let parent_decisions = path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &[
            "/lib/libvm-global-a.so",
            "agent=pressure-trigger",
            "agent=pressure-victim",
        ],
    )?;
    if !parent_decisions {
        return Err(243);
    }

    let child_vmobjects = path_contains_all_markers(
        runtime,
        &format!("/proc/{child}/vmobjects"),
        &["/lib/libvm-global-b.so", "resident=1", "dirty=1"],
    )?;
    if !child_vmobjects {
        return Err(245);
    }

    write_line(
        runtime,
        &format!(
            "vm.smoke.pressure.global pid={pid} child={child} target-pages=3 reclaimed-pages={reclaimed} victim=libvm-global-a survivor=libvm-global-b outcome=ok"
        ),
    )
}

pub(crate) fn run_vm_advise_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    ensure_vm_smoke_backing_file(runtime, "/lib/libvm-advise.so", 0x4000, 0x61, 274, 275, 276)?;
    let mapped = runtime
        .map_file_memory(
            pid,
            "/lib/libvm-advise.so",
            0x3000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 277)?;
    runtime
        .protect_memory_range(pid, mapped, 0x3000, true, true, false)
        .map_err(|_| 278)?;
    for (page, value) in [(0u64, 71u32), (0x1000, 72u32), (0x2000, 73u32)] {
        runtime
            .store_memory_word(pid, mapped + page, value)
            .map_err(|_| 279)?;
    }
    runtime
        .advise_memory_range(pid, mapped + 0x1000, 0x1000, 4)
        .map_err(|_| 280)?;
    runtime
        .advise_memory_range(pid, mapped, 0x3000, 3)
        .map_err(|_| 281)?;

    let vmdecisions = path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &["agent=advice", "/lib/libvm-advise.so"],
    )?;
    if !vmdecisions {
        return Err(282);
    }
    let vmepisodes = path_contains_all_markers(
        runtime,
        "/proc/1/vmepisodes",
        &["kind=fault", "advised=yes", "last=advice"],
    )?;
    if !vmepisodes {
        return Err(283);
    }
    runtime.load_memory_word(pid, mapped).map_err(|_| 284)?;
    write_line(
        runtime,
        &format!("vm.smoke.advise pid={pid} path=/lib/libvm-advise.so advised=yes outcome=ok"),
    )
}

pub(crate) fn run_vm_quarantine_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    ensure_vm_smoke_backing_file(
        runtime,
        "/lib/libvm-quarantine.so",
        0x3000,
        0x73,
        285,
        286,
        287,
    )?;
    let mapped = runtime
        .map_file_memory(
            pid,
            "/lib/libvm-quarantine.so",
            0x2000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 288)?;
    runtime
        .protect_memory_range(pid, mapped, 0x2000, true, true, false)
        .map_err(|_| 289)?;
    runtime
        .store_memory_word(pid, mapped, 81)
        .map_err(|_| 290)?;
    let vm_object_id =
        vm_object_id_for_path(runtime, "/proc/1/vmobjects", "/lib/libvm-quarantine.so")?;
    runtime
        .quarantine_vm_object(pid, vm_object_id, 77)
        .map_err(|_| 291)?;
    let blocked_store = runtime.store_memory_word(pid, mapped, 82);
    if blocked_store != Err(Errno::Access) && blocked_store != Err(Errno::Fault) {
        return Err(292);
    }
    runtime
        .release_vm_object(pid, vm_object_id)
        .map_err(|_| 293)?;
    runtime
        .store_memory_word(pid, mapped, 83)
        .map_err(|_| 294)?;

    let vmobjects = path_contains_all_markers(
        runtime,
        "/proc/1/vmobjects",
        &["/lib/libvm-quarantine.so", "quarantined=0", "reason=0"],
    )?;
    if !vmobjects {
        return Err(295);
    }
    let vmdecisions = path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &["agent=quarantine-state", "agent=quarantine-block"],
    )?;
    if !vmdecisions {
        return Err(296);
    }
    let vmepisodes = path_contains_all_markers(
        runtime,
        "/proc/1/vmepisodes",
        &["kind=quarantine", "blocked=yes", "released=yes"],
    )?;
    if !vmepisodes {
        return Err(297);
    }
    write_line(
        runtime,
        &format!(
            "vm.smoke.quarantine pid={pid} path=/lib/libvm-quarantine.so blocked=yes released=yes outcome=ok"
        ),
    )
}

pub(crate) fn run_vm_policy_block_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    contract: usize,
) -> Result<(), ExitCode> {
    let mapped = runtime
        .map_anonymous_memory(pid, 0x1000, true, true, false, "boot-vm-policy")
        .map_err(|_| 298)?;
    runtime
        .store_memory_word(pid, mapped, 91)
        .map_err(|_| 299)?;
    runtime
        .set_contract_state(contract, NativeContractState::Suspended)
        .map_err(|_| 300)?;

    let blocked = [
        runtime
            .map_anonymous_memory(pid, 0x1000, true, true, false, "blocked-map")
            .is_err(),
        runtime
            .protect_memory_range(pid, mapped, 0x1000, true, false, false)
            .is_err(),
        runtime.advise_memory_range(pid, mapped, 0x1000, 4).is_err(),
        runtime.unmap_memory_range(pid, mapped, 0x1000).is_err(),
        runtime.load_memory_word(pid, mapped).is_err(),
        runtime.store_memory_word(pid, mapped, 99).is_err(),
        runtime.reclaim_memory_pressure(pid, 1).is_err(),
        runtime.reclaim_memory_pressure_global(1).is_err(),
    ];
    if blocked.iter().all(|blocked| !blocked) {
        let _ = runtime.set_contract_state(contract, NativeContractState::Active);
        return Err(301);
    }

    let vmdecisions =
        path_contains_all_markers(runtime, "/proc/1/vmdecisions", &["agent=policy-block"])?;
    if !vmdecisions {
        let _ = runtime.set_contract_state(contract, NativeContractState::Active);
        return Err(302);
    }
    let vmepisodes = path_contains_all_markers(
        runtime,
        "/proc/1/vmepisodes",
        &["kind=policy", "blocked=yes", "last=policy-block"],
    )?;
    if !vmepisodes {
        let _ = runtime.set_contract_state(contract, NativeContractState::Active);
        return Err(303);
    }

    runtime
        .set_contract_state(contract, NativeContractState::Active)
        .map_err(|_| 304)?;
    runtime.load_memory_word(pid, mapped).map_err(|_| 305)?;
    runtime
        .store_memory_word(pid, mapped, 92)
        .map_err(|_| 306)?;
    runtime
        .unmap_memory_range(pid, mapped, 0x1000)
        .map_err(|_| 307)?;

    write_line(
        runtime,
        &format!(
            "vm.smoke.policy pid={pid} contract={contract} blocked=yes resumed=yes outcome=ok"
        ),
    )
}

fn run_vm_required_vfs_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    let root = "/vfs";
    let bin_dir = "/vfs/bin";
    let app_path = "/vfs/bin/app";
    let renamed_app_path = "/vfs/bin/app2";
    let link_path = "/vfs/link";

    if runtime.mkdir_path(root).is_err() && runtime.stat_path(root).is_err() {
        return Err(247);
    }
    if runtime.mkdir_path(bin_dir).is_err() && runtime.stat_path(bin_dir).is_err() {
        return Err(248);
    }
    if runtime.mkfile_path(app_path).is_err() && runtime.stat_path(app_path).is_err() {
        return Err(249);
    }
    if let Err(error) = runtime.symlink_path(link_path, app_path) {
        if !matches!(error, Errno::Exist) {
            return Err(250);
        }
    }

    let mount = runtime.statfs_path(root).map_err(|_| 251)?;
    let kind = runtime.stat_path(app_path).map_err(|_| 252)?;
    let link = runtime.lstat_path(link_path).map_err(|_| 253)?;
    let mut target = [0u8; 64];
    let target_len = runtime
        .readlink_path(link_path, &mut target)
        .map_err(|_| 254)?;
    if &target[..target_len] != app_path.as_bytes() {
        return Err(255);
    }

    let fd = runtime.open_path(link_path).map_err(|_| 256)?;
    runtime.close(fd).map_err(|_| 257)?;
    runtime
        .rename_path(app_path, renamed_app_path)
        .map_err(|_| 258)?;
    runtime.stat_path(renamed_app_path).map_err(|_| 259)?;
    runtime.open_path(renamed_app_path).map_err(|_| 260)?;
    if runtime.rename_path(bin_dir, "/vfs/bin/subdir").is_ok() {
        return Err(261);
    }
    runtime.unlink_path(link_path).map_err(|_| 262)?;
    if runtime.readlink_path(link_path, &mut target).is_ok() {
        return Err(263);
    }

    write_line(
        runtime,
        &format!(
            "vfs.smoke.mount pid={pid} path={root} mounts={} nodes={} read_only={} outcome=ok",
            mount.mount_count, mount.node_count, mount.read_only
        ),
    )
    .map_err(|_| 264)?;
    write_line(
        runtime,
        &format!(
            "vfs.smoke.create pid={pid} path={app_path} kind={} inode={} outcome=ok",
            kind.kind, kind.inode
        ),
    )
    .map_err(|_| 265)?;
    write_line(
        runtime,
        &format!(
            "vfs.smoke.symlink pid={pid} path={link_path} target={app_path} kind={} inode={} outcome=ok",
            link.kind, link.inode
        ),
    )
    .map_err(|_| 266)?;
    write_line(
        runtime,
        &format!(
            "vfs.smoke.rename pid={pid} from={app_path} to={renamed_app_path} refusal=invalid-subtree yes outcome=ok"
        ),
    )
    .map_err(|_| 267)?;
    write_line(
        runtime,
        &format!("vfs.smoke.unlink pid={pid} path={link_path} after-unlink=missing outcome=ok"),
    )
    .map_err(|_| 268)?;
    write_line(
        runtime,
        &format!("vfs.smoke.coherence pid={pid} descriptor=open-path-open readlink=stable statfs=ok outcome=ok"),
    )
    .map_err(|_| 269)?;

    Ok(())
}

pub(crate) fn run_native_vm_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let pid = 1u64;
    if let Err(code) = boot_bind_observe_contract(runtime) {
        return code;
    }
    let memory_contract = match boot_bind_vm_memory_contract(runtime) {
        Ok(contract) => contract,
        Err(code) => return code,
    };
    if let Err(code) =
        ensure_vm_smoke_backing_file(runtime, "/lib/libvm-smoke.so", 0x3000, 0x5a, 160, 163, 164)
    {
        return code;
    }
    let mapped = match runtime.map_anonymous_memory(pid, 0x2000, true, true, false, "boot-vm-smoke")
    {
        Ok(start) => start,
        Err(_) => return 170,
    };
    if write_line(
        runtime,
        &format!("vm.smoke.map pid={pid} start={mapped} len=8192"),
    )
    .is_err()
    {
        return 171;
    }

    if runtime.store_memory_word(pid, mapped, 7).is_err() {
        return 172;
    }
    if runtime
        .protect_memory_range(pid, mapped + 0x1000, 0x1000, true, false, false)
        .is_err()
    {
        return 173;
    }
    if runtime.load_memory_word(pid, mapped + 0x1000).is_err() {
        return 174;
    }
    if runtime.store_memory_word(pid, mapped + 0x1000, 9) != Err(Errno::Fault) {
        return 175;
    }
    if write_line(
        runtime,
        &format!(
            "vm.smoke.protect pid={pid} addr={} perms=r--",
            mapped + 0x1000
        ),
    )
    .is_err()
    {
        return 176;
    }

    let maps = match path_contains_all_markers(
        runtime,
        "/proc/1/maps",
        &["[anon:boot-vm-smoke]", "r--p 00000000 [anon:boot-vm-smoke]"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !maps {
        return 177;
    }

    let heap_grow = match runtime.set_process_break(pid, 0x4000_7000) {
        Ok(end) => end,
        Err(_) => return 199,
    };
    if heap_grow != 0x4000_7000 {
        return 200;
    }
    let heap_shrink = match runtime.set_process_break(pid, 0x4000_3000) {
        Ok(end) => end,
        Err(_) => return 201,
    };
    if heap_shrink != 0x4000_3000 {
        return 202;
    }
    let heap_vmdecisions =
        match path_contains_all_markers(runtime, "/proc/1/vmdecisions", &["agent=brk"]) {
            Ok(value) => value,
            Err(code) => return code,
        };
    if !heap_vmdecisions {
        return 203;
    }

    if runtime.sync_memory_range(pid, mapped, 0x1000).is_err() {
        return 179;
    }
    if runtime
        .unmap_memory_range(pid, mapped + 0x1000, 0x1000)
        .is_err()
    {
        return 181;
    }
    if runtime.load_memory_word(pid, mapped + 0x1000) != Err(Errno::Fault) {
        return 182;
    }
    if write_line(
        runtime,
        &format!(
            "vm.smoke.unmap pid={pid} addr={} len=4096 outcome=ok",
            mapped + 0x1000
        ),
    )
    .is_err()
    {
        return 184;
    }
    let vmdecisions = match path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &["agent=map", "agent=protect", "agent=unmap"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !vmdecisions {
        return 178;
    }
    let vmepisodes = match path_contains_all_markers(
        runtime,
        "/proc/1/vmepisodes",
        &[
            "kind=heap",
            "grew=yes",
            "shrank=yes",
            "kind=region",
            "protected=yes",
            "unmapped=yes",
        ],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !vmepisodes {
        return 204;
    }
    if write_line(
        runtime,
        &format!("vm.smoke.heap pid={pid} kind=heap grew=yes shrank=yes"),
    )
    .is_err()
    {
        return 205;
    }
    if write_line(
        runtime,
        &format!("vm.smoke.region pid={pid} kind=region protected=yes unmapped=yes"),
    )
    .is_err()
    {
        return 198;
    }

    let cow_child = match runtime.spawn_process_copy_vm("c", "/c", pid) {
        Ok(child) => child,
        Err(_) => return 192,
    };
    if runtime.store_memory_word(cow_child, mapped, 21).is_err() {
        return 193;
    }
    let child_vmobjects =
        match path_contains_all_markers(runtime, "/proc/2/vmobjects", &["[cow]", "depth=1"]) {
            Ok(value) => value,
            Err(code) => return code,
        };
    if !child_vmobjects {
        return 194;
    }
    let child_vmdecisions = match path_contains_all_markers(
        runtime,
        "/proc/2/vmdecisions",
        &["agent=shadow-reuse", "agent=cow-populate"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !child_vmdecisions {
        return 195;
    }
    if write_line(
        runtime,
        &format!(
            "vm.smoke.cow.observe pid={cow_child} source={pid} object=[cow] depth=1 kind=fault cow=yes"
        ),
    )
    .is_err()
    {
        return 197;
    }
    if write_line(
        runtime,
        &format!("vm.smoke.cow pid={cow_child} source={pid} addr={mapped} outcome=ok"),
    )
    .is_err()
    {
        return 196;
    }

    let file_mapped = match runtime.map_file_memory(
        pid,
        "/lib/libvm-smoke.so",
        0x2000,
        0x1000,
        true,
        false,
        true,
        true,
    ) {
        Ok(start) => start,
        Err(_) => return 185,
    };
    if runtime.store_memory_word(pid, file_mapped, 13) != Err(Errno::Fault) {
        return 186;
    }
    if runtime
        .protect_memory_range(pid, file_mapped, 0x2000, true, true, false)
        .is_err()
    {
        return 187;
    }
    if runtime.store_memory_word(pid, file_mapped, 13).is_err() {
        return 188;
    }
    if runtime.sync_memory_range(pid, file_mapped, 0x2000).is_err() {
        return 189;
    }
    let file_maps = match path_contains_all_markers(
        runtime,
        "/proc/1/maps",
        &["rw-p 00001000 /lib/libvm-smoke.so"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !file_maps {
        return 183;
    }
    let file_vmdecisions = match path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &[
            "agent=map-file",
            "agent=protect",
            "agent=sync",
            "/lib/libvm-smoke.so",
        ],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !file_vmdecisions {
        return 191;
    }
    if runtime
        .unmap_memory_range(pid, file_mapped, 0x2000)
        .is_err()
    {
        return 192;
    }

    if let Err(code) = run_vm_stress_hardening(runtime, pid) {
        return code;
    }
    if let Err(code) = run_vm_pressure_hardening(runtime, pid) {
        return code;
    }
    if let Err(code) = run_vm_global_pressure_hardening(runtime, pid) {
        return code;
    }
    if let Err(code) = run_vm_advise_hardening(runtime, pid) {
        return code;
    }
    if let Err(code) = run_vm_quarantine_hardening(runtime, pid) {
        return code;
    }
    if let Err(code) = run_vm_policy_block_hardening(runtime, pid, memory_contract) {
        return code;
    }

    if write_line(
        runtime,
        &format!(
            "vm.smoke.production pid={pid} stress=yes pressure=yes global-pressure=yes advise=yes quarantine=yes policy=yes workloads=anon,cow,file,heap,region outcome=ok"
        ),
    )
    .is_err()
    {
        return 246;
    }

    if let Err(code) = run_vm_required_vfs_smoke(runtime, pid) {
        return code;
    }

    0
}
