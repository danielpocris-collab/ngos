//! Canonical subsystem role:
//! - subsystem: host runtime session assembly
//! - owner layer: auxiliary execution layer
//! - semantic owner: `host-runtime`
//! - truth path role: auxiliary host-side assembly of native runtime sessions
//!   for development and reporting
//!
//! Canonical contract families handled here:
//! - host session fixture contracts
//! - host report assembly contracts
//! - auxiliary stdin-script execution contracts
//!
//! This module may assemble host-side sessions for validation, but it must not
//! be treated as the product truth path for subsystem closure.

use crate::backend::HostRuntimeKernelBackend;
use crate::report::{
    HostRuntimeNativeSessionReport, extract_chronoscope_summary,
    extract_resource_agent_report_summary, extract_vm_agent_report_summary,
    extract_vm_episode_report_summary,
};
use kernel_core::{
    CapabilityId, CapabilityRights, ContractKind, ContractState, Descriptor, Handle, KernelRuntime,
    MemoryAdvice, ObjectHandle, ObjectKind, ProcessId, ResourceContractPolicy, ResourceKind,
    SchedulerClass,
};
use ngos_boot_x86_64::diagnostics::chronoscope_snapshot;
use ngos_ui::UiPresenter;
use user_abi::{AuxvEntry, BootstrapArgs};
use user_runtime::Runtime as UserRuntime;

pub const NATIVE_STDIN_SCRIPT: &[u8] = b"help\nvm-probe-map-anon 2 4096 rw- shell-map-gap\nproc 2 maps\nvm-probe-map-anon 2 0 rw- shell-map-gap-invalid\nproc 2 vmobjects\nvm-load-word 2 $VM_FILE_ADDR\nvm-advise 2 $VM_FILE_ADDR 4096 dontneed\nproc 2 vmobjects\nvm-load-word 2 $VM_FILE_ADDR\nproc 2 vmobjects\nproc 2 maps\nvm-brk 2 $VM_HEAP_GROW\nproc 2 maps\nvm-brk 2 $VM_HEAP_SHRINK\nproc 2 maps\nvm-probe-brk 2 $VM_HEAP_INVALID\nproc 2 vmepisodes\nexit 0\n";
pub const HOST_SCRATCH_LEN: u64 = 0x20_000;

pub struct NativeHostTestFixture {
    pub runtime: KernelRuntime,
    pub init: ProcessId,
    pub app: ProcessId,
    pub scratch: usize,
}

struct NativeSessionLaunchContext {
    runtime: KernelRuntime,
    app: ProcessId,
    scratch: usize,
    bootstrap: BootstrapArgs<'static>,
}

struct NativeSessionObservability {
    process: kernel_core::ProcessInfo,
    stdout: String,
    system: kernel_core::SystemIntrospection,
}

struct VmProbeFamilyResult {
    vm_probe: usize,
    vm_file_probe: usize,
    vm_region_probe: usize,
    vm_probe_child: ProcessId,
    vm_probe_grandchild: ProcessId,
    vm_probe_grandchild_depth: u64,
    vm_shared_writer: ProcessId,
    vm_shared_observer: ProcessId,
    vm_shared_value: u32,
    vm_shared_restored_value: u32,
    vm_shared_live_a: ProcessId,
    vm_shared_live_b: ProcessId,
    vm_shared_live_b_map: u64,
    vm_shared_live_value: u32,
    vm_shared_live_owner_count: u64,
}

struct VmFaultFamilyResult {
    vm_cow_shadow_before_write: u64,
    vm_cow_shadow_after_write: u64,
    vm_cow_cow_faults_after_write: u64,
    vm_split_read_faults: u64,
    vm_split_write_faults: u64,
    vm_split_total_faults: u64,
    vm_offset_segment_count: u64,
    vm_offset_first_segment_offset: u64,
    vm_offset_second_segment_offset: u64,
    vm_read_fault_resident: u64,
    vm_read_fault_dirty: u64,
    vm_read_fault_accessed: u64,
    vm_read_fault_reads: u64,
    vm_read_fault_writes: u64,
    vm_mprotect_clean_faults: u64,
    vm_mprotect_clean_dirty: u64,
    vm_range_split_count: u64,
    vm_range_coalesced_count: u64,
    vm_range_dirty_after_sync: u64,
    vm_range_faults: u64,
}

struct VmPressureContractResult {
    vm_pressure_a: usize,
    vm_pressure_b: usize,
    vm_pressure_a_object: u64,
    vm_pressure_b_object: u64,
    vm_contract_app: ProcessId,
    vm_contract: u64,
    vm_contract_first_map: u64,
    vm_contract_blocked_state: u64,
    vm_pressure_global_reclaimed: u64,
    vm_pressure_global_victims: u64,
    vm_pressure_global_policy_blocks: u64,
    vm_pressure_global_cow_events: u64,
    vm_shared_live_restored_value: u32,
}

struct VmSessionAddressContext {
    probe_vm_object: u64,
    file_probe_vm_object: u64,
    heap_grow: u64,
    heap_shrink: u64,
    heap_invalid: u64,
}

struct NativeHostCapabilityContext {
    root: CapabilityId,
    bin: CapabilityId,
}

pub fn build_native_host_test_fixture_and_configure<F>(
    configure_runtime: F,
) -> NativeHostTestFixture
where
    F: FnOnce(&mut KernelRuntime),
{
    let mut runtime = KernelRuntime::host_runtime_default();
    configure_runtime(&mut runtime);
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .expect("native host runtime init spawn must succeed");
    let app = runtime
        .spawn_process(
            "ngos-userland-native",
            Some(init),
            SchedulerClass::Interactive,
        )
        .expect("native host runtime app spawn must succeed");
    let capabilities = grant_native_host_capabilities(&mut runtime, app);
    install_native_host_vfs_topology(&mut runtime, capabilities.root, capabilities.bin);
    install_native_host_device_graph(&mut runtime, capabilities.root);
    install_native_host_userland_bootstrap(&mut runtime, app);
    let scratch = runtime
        .map_anonymous_memory(
            app,
            HOST_SCRATCH_LEN,
            true,
            true,
            false,
            "host-runtime-scratch",
        )
        .expect("native scratch mapping must succeed") as usize;
    NativeHostTestFixture {
        runtime,
        init,
        app,
        scratch,
    }
}

fn grant_native_host_capabilities(
    runtime: &mut KernelRuntime,
    app: ProcessId,
) -> NativeHostCapabilityContext {
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(30_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .expect("native root capability grant must succeed");
    let bin = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(30_002), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "bin",
        )
        .expect("native bin capability grant must succeed");
    NativeHostCapabilityContext { root, bin }
}

fn install_native_host_vfs_topology(
    runtime: &mut KernelRuntime,
    root: CapabilityId,
    bin: CapabilityId,
) {
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/bin", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/etc", ObjectKind::Directory, root)
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
        .create_vfs_node("/bin/ngos-userland-native", ObjectKind::File, bin)
        .unwrap();
    runtime
        .create_vfs_node("/bin/worker", ObjectKind::File, bin)
        .unwrap();
    runtime
        .create_vfs_node("/etc/motd", ObjectKind::File, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/net0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/storage0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/audio0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/input0", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/dev/gpu-unbound", ObjectKind::Device, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/net0", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/storage0", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/gpu0", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/gpu1", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/audio0", ObjectKind::Driver, root)
        .unwrap();
    runtime
        .create_vfs_node("/drv/input0", ObjectKind::Driver, root)
        .unwrap();
}

fn install_native_host_device_graph(runtime: &mut KernelRuntime, root: CapabilityId) {
    runtime
        .bind_device_to_driver("/dev/net0", "/drv/net0")
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/storage0", "/drv/storage0")
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/gpu0", "/drv/gpu0")
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/audio0", "/drv/audio0")
        .unwrap();
    runtime
        .bind_device_to_driver("/dev/input0", "/drv/input0")
        .unwrap();
    runtime
        .configure_device_geometry("/dev/storage0", 512, 128 * 1024 * 1024)
        .unwrap();
    runtime
        .create_vfs_symlink("/motd", "/etc/motd", root)
        .unwrap();
}

fn install_native_host_userland_bootstrap(runtime: &mut KernelRuntime, app: ProcessId) {
    let motd_fd = runtime.open_path(app, "/etc/motd").unwrap();
    runtime.write_io(app, motd_fd, b"ngos host motd\n").unwrap();
    runtime.close_descriptor(app, motd_fd).unwrap();
    runtime
        .exec_process(
            app,
            "/bin/ngos-userland-native",
            vec![String::from(userland_native::PROGRAM_NAME)],
            vec![],
        )
        .expect("native exec must succeed");
}

pub fn build_native_session_report() -> HostRuntimeNativeSessionReport {
    build_native_session_report_with_script(NATIVE_STDIN_SCRIPT)
}

pub fn build_native_session_report_with_script(script: &[u8]) -> HostRuntimeNativeSessionReport {
    build_native_session_report_with_script_and_configure(script, |_| {})
}

fn prepare_native_session_launch(
    mut runtime: KernelRuntime,
    app: ProcessId,
    scratch: usize,
    script: &[u8],
) -> NativeSessionLaunchContext {
    let launch = runtime
        .prepare_user_launch(app)
        .expect("native launch plan must succeed");
    let argv = launch
        .bootstrap
        .argv
        .iter()
        .map(|value| value.clone().into_boxed_str())
        .collect::<Vec<_>>();
    let envp = launch
        .bootstrap
        .envp
        .iter()
        .map(|value| value.clone().into_boxed_str())
        .collect::<Vec<_>>();
    let auxv = launch
        .bootstrap
        .auxv
        .iter()
        .map(|entry| AuxvEntry {
            key: entry.key as usize,
            value: entry.value as usize,
        })
        .collect::<Vec<_>>();
    let leaked_argv_storage = Box::leak(argv.into_boxed_slice());
    let leaked_envp_storage = Box::leak(envp.into_boxed_slice());
    let leaked_argv = Box::leak(
        leaked_argv_storage
            .iter()
            .map(|value| value.as_ref())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let leaked_envp = Box::leak(
        leaked_envp_storage
            .iter()
            .map(|value| value.as_ref())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let leaked_auxv = Box::leak(auxv.into_boxed_slice());
    let bootstrap = BootstrapArgs::new(leaked_argv, leaked_envp, leaked_auxv);
    runtime
        .seed_standard_input(app, script)
        .expect("native stdin seeding must succeed");
    NativeSessionLaunchContext {
        runtime,
        app,
        scratch,
        bootstrap,
    }
}

fn execute_native_session(launch: NativeSessionLaunchContext) -> (i32, NativeSessionObservability) {
    let backend = HostRuntimeKernelBackend::new(
        launch.runtime,
        launch.app,
        launch.scratch,
        HOST_SCRATCH_LEN as usize,
    );
    let user_runtime = UserRuntime::new(backend);
    let exit_code = userland_native::main(&user_runtime, &launch.bootstrap);
    let mut runtime = user_runtime.backend().runtime_mut();
    runtime
        .exit(launch.app, exit_code)
        .expect("native process exit must succeed");
    let process = runtime
        .process_info(launch.app)
        .expect("native process info must succeed");
    let stdout = String::from_utf8(
        runtime
            .inspect_io(launch.app, Descriptor::new(1))
            .expect("native stdout inspection must succeed")
            .payload()
            .to_vec(),
    )
    .expect("native stdout must be utf8");
    let system = runtime.inspect_system();
    (
        exit_code,
        NativeSessionObservability {
            process,
            stdout,
            system,
        },
    )
}

fn build_native_session_report_from_observability(
    app: ProcessId,
    exit_code: i32,
    observability: NativeSessionObservability,
) -> HostRuntimeNativeSessionReport {
    HostRuntimeNativeSessionReport {
        pid: app.raw(),
        exit_code,
        stdout_bytes: observability.stdout.len(),
        ui_presentation_backend: match UiPresenter::backend() {
            ngos_ui::UiPresentationBackend::Skia => "skia",
        },
        session_reported: observability.process.session_reported,
        session_report_count: observability.process.session_report_count,
        session_status: observability.process.session_status,
        session_stage: observability.process.session_stage,
        session_code: observability.process.session_code,
        session_detail: observability.process.session_detail,
        domain_count: observability.system.domains.len(),
        resource_count: observability.system.resources.len(),
        contract_count: observability.system.contracts.len(),
        chronoscope: extract_chronoscope_summary(&chronoscope_snapshot()),
        resource_agents: extract_resource_agent_report_summary(
            &observability.system.resource_agent_decisions,
        ),
        vm_agents: extract_vm_agent_report_summary(&observability.system.vm_agent_decisions),
        vm_episodes: extract_vm_episode_report_summary(&observability.system.vm_agent_decisions),
        stdout: observability.stdout,
    }
}

fn append_export<T: core::fmt::Display>(script: &mut Vec<u8>, key: &str, value: T) {
    script.extend_from_slice(format!("set {key} {value}\n").as_bytes());
}

fn append_vm_probe_exports(
    script: &mut Vec<u8>,
    vm_probe_family: &VmProbeFamilyResult,
    probe_vm_object: u64,
    file_probe_vm_object: u64,
) {
    append_export(script, "VM_ADDR0", vm_probe_family.vm_probe);
    append_export(script, "VM_ADDR1", vm_probe_family.vm_probe + 0x1000);
    append_export(script, "VM_ADDR2", vm_probe_family.vm_probe + 0x2000);
    append_export(script, "VM_PROBE", probe_vm_object);
    append_export(
        script,
        "VM_PROBE_COPY",
        vm_probe_family.vm_probe_child.raw(),
    );
    append_export(
        script,
        "VM_PROBE_GRANDCHILD",
        vm_probe_family.vm_probe_grandchild.raw(),
    );
    append_export(
        script,
        "VM_PROBE_GRANDCHILD_DEPTH",
        vm_probe_family.vm_probe_grandchild_depth,
    );
    append_export(script, "VM_FILE_ADDR", vm_probe_family.vm_file_probe);
    append_export(script, "VM_FILE_PROBE", file_probe_vm_object);
    append_export(script, "VM_REGION0", vm_probe_family.vm_region_probe);
    append_export(
        script,
        "VM_REGION1",
        vm_probe_family.vm_region_probe + 0x1000,
    );
    append_export(
        script,
        "VM_REGION2",
        vm_probe_family.vm_region_probe + 0x2000,
    );
    append_export(
        script,
        "VM_SHARED_WRITER",
        vm_probe_family.vm_shared_writer.raw(),
    );
    append_export(
        script,
        "VM_SHARED_OBSERVER",
        vm_probe_family.vm_shared_observer.raw(),
    );
    append_export(script, "VM_SHARED_VALUE", vm_probe_family.vm_shared_value);
    append_export(
        script,
        "VM_SHARED_RESTORED",
        vm_probe_family.vm_shared_restored_value,
    );
    append_export(
        script,
        "VM_SHARED_LIVE_A",
        vm_probe_family.vm_shared_live_a.raw(),
    );
    append_export(
        script,
        "VM_SHARED_LIVE_B",
        vm_probe_family.vm_shared_live_b.raw(),
    );
    append_export(
        script,
        "VM_SHARED_LIVE_VALUE",
        vm_probe_family.vm_shared_live_value,
    );
    append_export(
        script,
        "VM_SHARED_LIVE_OWNERS",
        vm_probe_family.vm_shared_live_owner_count,
    );
}

fn append_vm_fault_exports(script: &mut Vec<u8>, vm_fault_family: &VmFaultFamilyResult) {
    append_export(
        script,
        "VM_COW_SHADOW_BEFORE",
        vm_fault_family.vm_cow_shadow_before_write,
    );
    append_export(
        script,
        "VM_COW_SHADOW_AFTER",
        vm_fault_family.vm_cow_shadow_after_write,
    );
    append_export(
        script,
        "VM_COW_COW_FAULTS",
        vm_fault_family.vm_cow_cow_faults_after_write,
    );
    append_export(
        script,
        "VM_SPLIT_READ_FAULTS",
        vm_fault_family.vm_split_read_faults,
    );
    append_export(
        script,
        "VM_SPLIT_WRITE_FAULTS",
        vm_fault_family.vm_split_write_faults,
    );
    append_export(
        script,
        "VM_SPLIT_TOTAL_FAULTS",
        vm_fault_family.vm_split_total_faults,
    );
    append_export(
        script,
        "VM_OFFSET_SEGMENTS",
        vm_fault_family.vm_offset_segment_count,
    );
    append_export(
        script,
        "VM_OFFSET_FIRST",
        vm_fault_family.vm_offset_first_segment_offset,
    );
    append_export(
        script,
        "VM_OFFSET_SECOND",
        vm_fault_family.vm_offset_second_segment_offset,
    );
    append_export(
        script,
        "VM_READ_RESIDENT",
        vm_fault_family.vm_read_fault_resident,
    );
    append_export(script, "VM_READ_DIRTY", vm_fault_family.vm_read_fault_dirty);
    append_export(
        script,
        "VM_READ_ACCESSED",
        vm_fault_family.vm_read_fault_accessed,
    );
    append_export(
        script,
        "VM_READ_READFAULTS",
        vm_fault_family.vm_read_fault_reads,
    );
    append_export(
        script,
        "VM_READ_WRITEFAULTS",
        vm_fault_family.vm_read_fault_writes,
    );
    append_export(
        script,
        "VM_MPROTECT_FAULTS",
        vm_fault_family.vm_mprotect_clean_faults,
    );
    append_export(
        script,
        "VM_MPROTECT_DIRTY",
        vm_fault_family.vm_mprotect_clean_dirty,
    );
    append_export(
        script,
        "VM_RANGE_SPLIT_COUNT",
        vm_fault_family.vm_range_split_count,
    );
    append_export(
        script,
        "VM_RANGE_COALESCED_COUNT",
        vm_fault_family.vm_range_coalesced_count,
    );
    append_export(
        script,
        "VM_RANGE_DIRTY_AFTER_SYNC",
        vm_fault_family.vm_range_dirty_after_sync,
    );
    append_export(script, "VM_RANGE_FAULTS", vm_fault_family.vm_range_faults);
}

fn append_vm_pressure_exports(script: &mut Vec<u8>, vm_pressure_family: &VmPressureContractResult) {
    append_export(
        script,
        "VM_SHARED_LIVE_RESTORED",
        vm_pressure_family.vm_shared_live_restored_value,
    );
    append_export(script, "VM_PRESSURE_A", vm_pressure_family.vm_pressure_a);
    append_export(script, "VM_PRESSURE_B", vm_pressure_family.vm_pressure_b);
    append_export(
        script,
        "VM_PRESSURE_A_OBJECT",
        vm_pressure_family.vm_pressure_a_object,
    );
    append_export(
        script,
        "VM_PRESSURE_B_OBJECT",
        vm_pressure_family.vm_pressure_b_object,
    );
    append_export(
        script,
        "VM_CONTRACT_PID",
        vm_pressure_family.vm_contract_app.raw(),
    );
    append_export(script, "VM_CONTRACT_ID", vm_pressure_family.vm_contract);
    append_export(
        script,
        "VM_CONTRACT_ALLOWED_MAP",
        vm_pressure_family.vm_contract_first_map,
    );
    append_export(
        script,
        "VM_CONTRACT_BLOCKED_STATE",
        vm_pressure_family.vm_contract_blocked_state,
    );
    append_export(
        script,
        "VM_PRESSURE_GLOBAL_RECLAIMED",
        vm_pressure_family.vm_pressure_global_reclaimed,
    );
    append_export(
        script,
        "VM_PRESSURE_GLOBAL_VICTIMS",
        vm_pressure_family.vm_pressure_global_victims,
    );
    append_export(
        script,
        "VM_PRESSURE_GLOBAL_POLICY_BLOCKS",
        vm_pressure_family.vm_pressure_global_policy_blocks,
    );
    append_export(
        script,
        "VM_PRESSURE_GLOBAL_COW_EVENTS",
        vm_pressure_family.vm_pressure_global_cow_events,
    );
}

fn build_runtime_probe_script(
    vm_probe_family: &VmProbeFamilyResult,
    vm_fault_family: &VmFaultFamilyResult,
    vm_pressure_family: &VmPressureContractResult,
    vm_addresses: &VmSessionAddressContext,
    scratch: usize,
    script: &[u8],
) -> Vec<u8> {
    let mut runtime_script = Vec::new();
    append_vm_probe_exports(
        &mut runtime_script,
        vm_probe_family,
        vm_addresses.probe_vm_object,
        vm_addresses.file_probe_vm_object,
    );
    append_export(&mut runtime_script, "VM_HEAP_GROW", vm_addresses.heap_grow);
    append_export(
        &mut runtime_script,
        "VM_HEAP_SHRINK",
        vm_addresses.heap_shrink,
    );
    append_export(
        &mut runtime_script,
        "VM_HEAP_INVALID",
        vm_addresses.heap_invalid,
    );
    append_vm_fault_exports(&mut runtime_script, vm_fault_family);
    append_vm_pressure_exports(&mut runtime_script, vm_pressure_family);
    append_export(&mut runtime_script, "VM_SCRATCH", scratch);
    runtime_script.extend_from_slice(script);
    runtime_script
}

fn script_requires_vm_exports(script: &[u8]) -> bool {
    script.windows(4).any(|window| window == b"$VM_")
}

fn seed_non_vm_session_contract_baseline(runtime: &mut KernelRuntime, app: ProcessId) {
    let domain = runtime
        .create_domain(app, None, "host-runtime-fast-path")
        .expect("native non-vm seed domain must succeed");
    let resource = runtime
        .create_resource(app, domain, ResourceKind::Device, "fast-path-seed")
        .expect("native non-vm seed resource must succeed");
    runtime
        .create_contract(
            app,
            domain,
            resource,
            ContractKind::Display,
            "fast-path-seed",
        )
        .expect("native non-vm seed contract must succeed");
}

fn run_vm_probe_family(
    runtime: &mut KernelRuntime,
    init: ProcessId,
    app: ProcessId,
) -> VmProbeFamilyResult {
    let vm_probe = runtime
        .map_anonymous_memory(app, 0x3000, true, true, false, "host-runtime-vm-probe")
        .expect("native vm probe mapping must succeed") as usize;
    let vm_file_probe = runtime
        .map_file_memory(app, "/etc/motd", 0x1000, 0, true, true, false, true)
        .expect("native file-backed vm probe mapping must succeed")
        as usize;
    let vm_region_probe = runtime
        .map_anonymous_memory(app, 0x3000, true, true, false, "host-runtime-vm-region")
        .expect("native vm region probe mapping must succeed") as usize;
    let vm_probe_child = runtime
        .spawn_process_copy_vm("vm-probe-copy", Some(init), SchedulerClass::Background, app)
        .expect("native vm probe copy spawn must succeed");
    runtime
        .touch_memory(vm_probe_child, (vm_probe + 0x1000) as u64, 0x1000, true)
        .expect("native vm probe child cow touch must succeed");
    let vm_probe_grandchild = runtime
        .spawn_process_copy_vm(
            "vm-probe-grandchild",
            Some(init),
            SchedulerClass::Background,
            vm_probe_child,
        )
        .expect("native vm probe grandchild spawn must succeed");
    runtime
        .touch_memory(
            vm_probe_grandchild,
            (vm_probe + 0x1000) as u64,
            0x1000,
            true,
        )
        .expect("native vm probe grandchild cow touch must succeed");
    let vm_probe_grandchild_depth = runtime
        .inspect_vm_object_layouts(vm_probe_grandchild)
        .expect("native vm probe grandchild layouts must succeed")
        .into_iter()
        .map(|layout| layout.shadow_depth)
        .max()
        .unwrap_or(0);
    let vm_shared_writer = runtime
        .spawn_process("vm-shared-writer", Some(init), SchedulerClass::Background)
        .expect("native vm shared writer spawn must succeed");
    let vm_shared_writer_map = runtime
        .map_file_memory(
            vm_shared_writer,
            "/etc/motd",
            0x1000,
            0,
            true,
            true,
            false,
            false,
        )
        .expect("native shared writer mapping must succeed");
    runtime
        .store_memory_word(vm_shared_writer, vm_shared_writer_map, 0x1122_3344)
        .expect("native shared writer store must succeed");
    runtime
        .sync_memory(vm_shared_writer, vm_shared_writer_map, 0x1000)
        .expect("native shared writer sync must succeed");
    runtime
        .unmap_memory(vm_shared_writer, vm_shared_writer_map, 0x1000)
        .expect("native shared writer unmap must succeed");
    let vm_shared_observer = runtime
        .spawn_process("vm-shared-observer", Some(init), SchedulerClass::Background)
        .expect("native vm shared observer spawn must succeed");
    let vm_shared_observer_map = runtime
        .map_file_memory(
            vm_shared_observer,
            "/etc/motd",
            0x1000,
            0,
            true,
            true,
            false,
            false,
        )
        .expect("native shared observer mapping must succeed");
    let vm_shared_value = runtime
        .load_memory_word(vm_shared_observer, vm_shared_observer_map)
        .expect("native shared observer load must succeed");
    runtime
        .advise_memory(
            vm_shared_observer,
            vm_shared_observer_map,
            0x1000,
            MemoryAdvice::DontNeed,
        )
        .expect("native shared observer reclaim must succeed");
    let vm_shared_restored_value = runtime
        .load_memory_word(vm_shared_observer, vm_shared_observer_map)
        .expect("native shared observer reload must succeed");
    let vm_shared_live_a = runtime
        .spawn_process("vm-shared-live-a", Some(init), SchedulerClass::Background)
        .expect("native vm shared live-a spawn must succeed");
    let vm_shared_live_b = runtime
        .spawn_process("vm-shared-live-b", Some(init), SchedulerClass::Background)
        .expect("native vm shared live-b spawn must succeed");
    let vm_shared_live_a_map = runtime
        .map_file_memory(
            vm_shared_live_a,
            "/etc/motd",
            0x1000,
            0,
            true,
            true,
            false,
            false,
        )
        .expect("native shared live-a mapping must succeed");
    let vm_shared_live_b_map = runtime
        .map_file_memory(
            vm_shared_live_b,
            "/etc/motd",
            0x1000,
            0,
            true,
            true,
            false,
            false,
        )
        .expect("native shared live-b mapping must succeed");
    runtime
        .store_memory_word(vm_shared_live_a, vm_shared_live_a_map, 0x5566_7788)
        .expect("native shared live-a store must succeed");
    let vm_shared_live_value = runtime
        .load_memory_word(vm_shared_live_b, vm_shared_live_b_map)
        .expect("native shared live-b load must succeed");
    runtime
        .sync_memory(vm_shared_live_b, vm_shared_live_b_map, 0x1000)
        .expect("native shared live-b sync must succeed");
    let vm_shared_live_owner_count = runtime
        .inspect_vm_object_layouts(vm_shared_live_b)
        .expect("native shared live-b layouts must succeed")
        .into_iter()
        .find(|layout| {
            layout.kind == kernel_core::VmObjectKind::File
                && !layout.private
                && layout.backing_offset == 0
        })
        .map(|layout| layout.owner_count as u64)
        .unwrap_or(0);
    VmProbeFamilyResult {
        vm_probe,
        vm_file_probe,
        vm_region_probe,
        vm_probe_child,
        vm_probe_grandchild,
        vm_probe_grandchild_depth: vm_probe_grandchild_depth.into(),
        vm_shared_writer,
        vm_shared_observer,
        vm_shared_value,
        vm_shared_restored_value,
        vm_shared_live_a,
        vm_shared_live_b,
        vm_shared_live_b_map,
        vm_shared_live_value,
        vm_shared_live_owner_count,
    }
}

fn run_vm_fault_family(runtime: &mut KernelRuntime, init: ProcessId) -> VmFaultFamilyResult {
    let vm_cow_read_parent = runtime
        .spawn_process("vm-cow-read-parent", Some(init), SchedulerClass::Background)
        .expect("native vm cow read parent spawn must succeed");
    let vm_cow_read_addr = runtime
        .map_anonymous_memory(vm_cow_read_parent, 0x2000, true, true, false, "vm-cow-read")
        .expect("native vm cow read mapping must succeed");
    let vm_cow_read_child = runtime
        .spawn_process_copy_vm(
            "vm-cow-read-child",
            Some(init),
            SchedulerClass::Background,
            vm_cow_read_parent,
        )
        .expect("native vm cow read child spawn must succeed");
    runtime
        .load_memory_word(vm_cow_read_child, vm_cow_read_addr)
        .expect("native vm cow read load must succeed");
    let vm_cow_shadow_before_write = runtime
        .inspect_vm_object_layouts(vm_cow_read_child)
        .expect("native vm cow read child layouts must succeed")
        .into_iter()
        .filter(|layout| layout.shadow_source_id.is_some())
        .count() as u64;
    runtime
        .store_memory_word(vm_cow_read_child, vm_cow_read_addr, 0x99aa_bbcc)
        .expect("native vm cow read child store must succeed");
    let vm_cow_layouts_after = runtime
        .inspect_vm_object_layouts(vm_cow_read_child)
        .expect("native vm cow read child layouts after write must succeed");
    let vm_cow_shadow_after_write = vm_cow_layouts_after
        .iter()
        .filter(|layout| layout.shadow_source_id.is_some())
        .count() as u64;
    let vm_cow_cow_faults_after_write = vm_cow_layouts_after
        .iter()
        .filter(|layout| layout.shadow_source_id.is_some())
        .map(|layout| layout.cow_fault_count)
        .sum::<u64>();
    let vm_split_fault_app = runtime
        .spawn_process("vm-split-fault-app", Some(init), SchedulerClass::Background)
        .expect("native vm split fault app spawn must succeed");
    let vm_split_fault_map = runtime
        .map_file_memory(
            vm_split_fault_app,
            "/etc/motd",
            0x3000,
            0,
            true,
            false,
            false,
            true,
        )
        .expect("native vm split fault mapping must succeed");
    runtime
        .protect_memory(
            vm_split_fault_app,
            vm_split_fault_map,
            0x3000,
            true,
            true,
            false,
        )
        .expect("native vm split fault protect must succeed");
    runtime
        .load_memory_word(vm_split_fault_app, vm_split_fault_map)
        .expect("native vm split first read must succeed");
    runtime
        .load_memory_word(vm_split_fault_app, vm_split_fault_map + 0x1000)
        .expect("native vm split second read must succeed");
    runtime
        .store_memory_word(vm_split_fault_app, vm_split_fault_map + 0x2000, 0x1234_5678)
        .expect("native vm split write must succeed");
    let vm_split_fault_layout = runtime
        .inspect_vm_object_layouts(vm_split_fault_app)
        .expect("native vm split layouts must succeed")
        .into_iter()
        .find(|layout| layout.kind == kernel_core::VmObjectKind::File)
        .expect("native vm split file layout must exist");
    let vm_offset_app = runtime
        .spawn_process("vm-offset-app", Some(init), SchedulerClass::Background)
        .expect("native vm offset app spawn must succeed");
    let vm_offset_map = runtime
        .map_file_memory(
            vm_offset_app,
            "/etc/motd",
            0x2000,
            0x3000,
            true,
            false,
            true,
            true,
        )
        .expect("native vm offset mapping must succeed");
    runtime
        .protect_memory(vm_offset_app, vm_offset_map, 0x2000, true, true, false)
        .expect("native vm offset protect must succeed");
    runtime
        .store_memory_word(vm_offset_app, vm_offset_map, 0xaabb_ccdd)
        .expect("native vm offset store must succeed");
    let vm_offset_layout = runtime
        .inspect_vm_object_layouts(vm_offset_app)
        .expect("native vm offset layouts must succeed")
        .into_iter()
        .find(|layout| {
            layout.kind == kernel_core::VmObjectKind::File && layout.backing_offset == 0x3000
        })
        .expect("native vm offset file layout must exist");
    let vm_read_fault_app = runtime
        .spawn_process("vm-read-fault-app", Some(init), SchedulerClass::Background)
        .expect("native vm read fault app spawn must succeed");
    let vm_read_fault_map = runtime
        .map_file_memory(
            vm_read_fault_app,
            "/etc/motd",
            0x2000,
            0x2000,
            true,
            false,
            false,
            true,
        )
        .expect("native vm read fault mapping must succeed");
    runtime
        .load_memory_word(vm_read_fault_app, vm_read_fault_map)
        .expect("native vm read fault load must succeed");
    let vm_read_fault_layout = runtime
        .inspect_vm_object_layouts(vm_read_fault_app)
        .expect("native vm read fault layouts must succeed")
        .into_iter()
        .find(|layout| layout.kind == kernel_core::VmObjectKind::File)
        .expect("native vm read fault file layout must exist");
    let vm_mprotect_clean_app = runtime
        .spawn_process(
            "vm-mprotect-clean-app",
            Some(init),
            SchedulerClass::Background,
        )
        .expect("native vm mprotect clean app spawn must succeed");
    let vm_mprotect_clean_map = runtime
        .map_file_memory(
            vm_mprotect_clean_app,
            "/etc/motd",
            0x2000,
            0xa000,
            true,
            false,
            true,
            true,
        )
        .expect("native vm mprotect clean mapping must succeed");
    runtime
        .protect_memory(
            vm_mprotect_clean_app,
            vm_mprotect_clean_map,
            0x2000,
            true,
            true,
            false,
        )
        .expect("native vm mprotect clean protect must succeed");
    let vm_mprotect_clean_layout = runtime
        .inspect_vm_object_layouts(vm_mprotect_clean_app)
        .expect("native vm mprotect clean layouts must succeed")
        .into_iter()
        .find(|layout| layout.kind == kernel_core::VmObjectKind::File)
        .expect("native vm mprotect clean file layout must exist");
    let vm_range_app = runtime
        .spawn_process("vm-range-app", Some(init), SchedulerClass::Background)
        .expect("native vm range app spawn must succeed");
    let vm_range_map = runtime
        .map_file_memory(
            vm_range_app,
            "/etc/motd",
            0x3000,
            0x6000,
            true,
            false,
            true,
            true,
        )
        .expect("native vm range mapping must succeed");
    runtime
        .protect_memory(
            vm_range_app,
            vm_range_map + 0x1000,
            0x1000,
            true,
            true,
            false,
        )
        .expect("native vm range partial protect must succeed");
    let vm_range_split_maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", vm_range_app.raw()))
            .expect("native vm range split maps must succeed"),
    )
    .expect("native vm range split maps must be utf8");
    runtime
        .protect_memory(vm_range_app, vm_range_map, 0x3000, true, true, false)
        .expect("native vm range full protect must succeed");
    runtime
        .advise_memory(
            vm_range_app,
            vm_range_map + 0x1000,
            0x1000,
            MemoryAdvice::DontNeed,
        )
        .expect("native vm range partial advise must succeed");
    runtime
        .advise_memory(vm_range_app, vm_range_map, 0x3000, MemoryAdvice::Normal)
        .expect("native vm range full advise must succeed");
    let vm_range_coalesced_maps = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/maps", vm_range_app.raw()))
            .expect("native vm range coalesced maps must succeed"),
    )
    .expect("native vm range coalesced maps must be utf8");
    runtime
        .store_memory_word(vm_range_app, vm_range_map + 0x1000, 0x0bad_f00d)
        .expect("native vm range store must succeed");
    runtime
        .sync_memory(vm_range_app, vm_range_map, 0x3000)
        .expect("native vm range sync must succeed");
    let vm_range_layout = runtime
        .inspect_vm_object_layouts(vm_range_app)
        .expect("native vm range layouts must succeed")
        .into_iter()
        .find(|layout| layout.kind == kernel_core::VmObjectKind::File)
        .expect("native vm range file layout must exist");
    VmFaultFamilyResult {
        vm_cow_shadow_before_write,
        vm_cow_shadow_after_write,
        vm_cow_cow_faults_after_write,
        vm_split_read_faults: vm_split_fault_layout.read_fault_count,
        vm_split_write_faults: vm_split_fault_layout.write_fault_count,
        vm_split_total_faults: vm_split_fault_layout.fault_count,
        vm_offset_segment_count: vm_offset_layout.segment_count as u64,
        vm_offset_first_segment_offset: vm_offset_layout
            .segments
            .first()
            .map(|segment| segment.byte_offset)
            .unwrap_or(0),
        vm_offset_second_segment_offset: vm_offset_layout
            .segments
            .get(1)
            .map(|segment| segment.byte_offset)
            .unwrap_or(0),
        vm_read_fault_resident: vm_read_fault_layout.resident_pages,
        vm_read_fault_dirty: vm_read_fault_layout.dirty_pages,
        vm_read_fault_accessed: vm_read_fault_layout.accessed_pages,
        vm_read_fault_reads: vm_read_fault_layout.read_fault_count,
        vm_read_fault_writes: vm_read_fault_layout.write_fault_count,
        vm_mprotect_clean_faults: vm_mprotect_clean_layout.fault_count,
        vm_mprotect_clean_dirty: vm_mprotect_clean_layout.dirty_pages,
        vm_range_split_count: vm_range_split_maps.matches("/etc/motd").count() as u64,
        vm_range_coalesced_count: vm_range_coalesced_maps.matches("/etc/motd").count() as u64,
        vm_range_dirty_after_sync: vm_range_layout.dirty_pages,
        vm_range_faults: vm_range_layout.fault_count,
    }
}

fn run_vm_pressure_contract_family(
    runtime: &mut KernelRuntime,
    init: ProcessId,
    app: ProcessId,
    vm_shared_live_b: ProcessId,
    vm_shared_live_b_map: u64,
) -> VmPressureContractResult {
    let vm_pressure_a = runtime
        .map_file_memory(app, "/etc/motd", 0x2000, 0xb000, true, true, false, true)
        .expect("native vm pressure-a mapping must succeed") as usize;
    runtime
        .store_memory_word(app, vm_pressure_a as u64, 0xface_cafe)
        .expect("native vm pressure-a first store must succeed");
    runtime
        .store_memory_word(app, (vm_pressure_a + 0x1000) as u64, 0xfeed_beef)
        .expect("native vm pressure-a second store must succeed");
    let vm_pressure_b = runtime
        .map_file_memory(app, "/etc/motd", 0x1000, 0xd000, true, true, false, true)
        .expect("native vm pressure-b mapping must succeed") as usize;
    runtime
        .store_memory_word(app, vm_pressure_b as u64, 0x1234_abcd)
        .expect("native vm pressure-b store must succeed");
    let vm_pressure_a_object = runtime
        .resolve_vm_object_id(app, vm_pressure_a as u64, 0x2000)
        .expect("native vm pressure-a object resolution must succeed");
    let vm_pressure_b_object = runtime
        .resolve_vm_object_id(app, vm_pressure_b as u64, 0x1000)
        .expect("native vm pressure-b object resolution must succeed");
    let vm_contract_app = runtime
        .spawn_process("vm-contract-app", Some(init), SchedulerClass::Background)
        .expect("native vm contract app spawn must succeed");
    let vm_contract_domain = runtime
        .create_domain(vm_contract_app, None, "vm-guard")
        .expect("native vm contract domain must succeed");
    let vm_contract_resource = runtime
        .create_resource(
            vm_contract_app,
            vm_contract_domain,
            ResourceKind::Memory,
            "vm-guard-budget",
        )
        .expect("native vm contract resource must succeed");
    runtime
        .set_resource_contract_policy(vm_contract_resource, ResourceContractPolicy::Memory)
        .expect("native vm contract policy must succeed");
    let vm_contract = runtime
        .create_contract(
            vm_contract_app,
            vm_contract_domain,
            vm_contract_resource,
            ContractKind::Memory,
            "vm-guard",
        )
        .expect("native vm memory contract must succeed");
    runtime
        .bind_process_contract(vm_contract_app, vm_contract)
        .expect("native vm memory contract bind must succeed");
    let vm_contract_first_map = runtime
        .map_anonymous_memory(
            vm_contract_app,
            0x2000,
            true,
            true,
            false,
            "vm-contract-allowed",
        )
        .expect("native vm contract allowed map must succeed");
    runtime
        .transition_contract_state(vm_contract, ContractState::Suspended)
        .expect("native vm contract suspend must succeed");
    let vm_contract_blocked_state = match runtime.map_anonymous_memory(
        vm_contract_app,
        0x1000,
        true,
        true,
        false,
        "vm-contract-blocked",
    ) {
        Err(kernel_core::RuntimeError::NativeModel(
            kernel_core::NativeModelError::ContractNotActive { state },
        )) => match state {
            ContractState::Active => 0,
            ContractState::Suspended => 1,
            ContractState::Revoked => 2,
        },
        Ok(_) => 99,
        Err(_) => 98,
    };
    let composed_before = runtime.inspect_system().vm_agent_decisions.len();
    let vm_pressure_global_reclaimed = runtime
        .reclaim_memory_pressure_global(3)
        .expect("native global vm pressure reclaim must succeed");
    let vm_shared_live_restored_value = runtime
        .load_memory_word(vm_shared_live_b, vm_shared_live_b_map)
        .expect("native shared live-b restore after pressure must succeed");
    let composed_after = runtime.inspect_system();
    let composed_vm_agent_slice = &composed_after.vm_agent_decisions[composed_before..];
    VmPressureContractResult {
        vm_pressure_a,
        vm_pressure_b,
        vm_pressure_a_object,
        vm_pressure_b_object,
        vm_contract_app,
        vm_contract: vm_contract.raw(),
        vm_contract_first_map,
        vm_contract_blocked_state,
        vm_pressure_global_reclaimed,
        vm_pressure_global_victims: composed_vm_agent_slice
            .iter()
            .filter(|entry| matches!(entry.agent, kernel_core::VmAgentKind::PressureVictimAgent))
            .count() as u64,
        vm_pressure_global_policy_blocks: composed_after
            .vm_agent_decisions
            .iter()
            .filter(|entry| matches!(entry.agent, kernel_core::VmAgentKind::PolicyBlockAgent))
            .count() as u64,
        vm_pressure_global_cow_events: composed_after
            .vm_agent_decisions
            .iter()
            .filter(|entry| {
                matches!(
                    entry.agent,
                    kernel_core::VmAgentKind::ShadowReuseAgent
                        | kernel_core::VmAgentKind::CowPopulateAgent
                        | kernel_core::VmAgentKind::ShadowBridgeAgent
                )
            })
            .count() as u64,
        vm_shared_live_restored_value,
    }
}

fn resolve_vm_session_address_context(
    runtime: &KernelRuntime,
    app: ProcessId,
    vm_probe_family: &VmProbeFamilyResult,
) -> VmSessionAddressContext {
    let heap_region = runtime
        .address_space_info(app)
        .expect("native address space inspection must succeed")
        .regions
        .into_iter()
        .find(|region| region.label == " [heap]")
        .expect("native heap region must exist");
    let probe_vm_object = runtime
        .resolve_vm_object_id(app, vm_probe_family.vm_probe as u64, 0x3000)
        .expect("native vm probe object resolution must succeed");
    let file_probe_vm_object = runtime
        .resolve_vm_object_id(app, vm_probe_family.vm_file_probe as u64, 0x1000)
        .expect("native file-backed vm probe object resolution must succeed");
    VmSessionAddressContext {
        probe_vm_object,
        file_probe_vm_object,
        heap_grow: heap_region.end + 0x2000,
        heap_shrink: heap_region.end,
        heap_invalid: u64::MAX - 0x7ff,
    }
}

pub fn build_native_session_report_with_script_and_configure<F>(
    script: &[u8],
    configure_runtime: F,
) -> HostRuntimeNativeSessionReport
where
    F: FnOnce(&mut KernelRuntime),
{
    let NativeHostTestFixture {
        mut runtime,
        init,
        app,
        scratch,
    } = build_native_host_test_fixture_and_configure(configure_runtime);
    let runtime_script = if script_requires_vm_exports(script) {
        let vm_probe_family = run_vm_probe_family(&mut runtime, init, app);
        let vm_fault_family = run_vm_fault_family(&mut runtime, init);
        let vm_pressure_family = run_vm_pressure_contract_family(
            &mut runtime,
            init,
            app,
            vm_probe_family.vm_shared_live_b,
            vm_probe_family.vm_shared_live_b_map,
        );
        let vm_addresses = resolve_vm_session_address_context(&runtime, app, &vm_probe_family);
        build_runtime_probe_script(
            &vm_probe_family,
            &vm_fault_family,
            &vm_pressure_family,
            &vm_addresses,
            scratch,
            script,
        )
    } else {
        seed_non_vm_session_contract_baseline(&mut runtime, app);
        script.to_vec()
    };
    let launch = prepare_native_session_launch(runtime, app, scratch, &runtime_script);
    let (exit_code, observability) = execute_native_session(launch);
    build_native_session_report_from_observability(app, exit_code, observability)
}

#[cfg(test)]
mod tests {
    use super::script_requires_vm_exports;

    #[test]
    fn script_requires_vm_exports_only_when_vm_placeholders_are_present() {
        assert!(script_requires_vm_exports(
            b"vm-load-word 2 $VM_FILE_ADDR\nexit 0\n"
        ));
        assert!(!script_requires_vm_exports(
            b"gpu-submit /dev/gpu0 draw:a\ngpu-read /dev/gpu0\nexit 0\n"
        ));
    }
}
