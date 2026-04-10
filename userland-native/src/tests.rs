use super::*;
use core::cell::{Cell, RefCell};
use ngos_semantic_runtime::{SemanticObservation, semantic_for_channel};
use ngos_user_abi::{
    AT_ENTRY, AT_PAGESZ, BootSessionStage, BootSessionStatus, Errno, NATIVE_STORAGE_LINEAGE_DEPTH,
    NativeBusEndpointRecord, NativeBusEventWatchConfig, NativeBusPeerRecord, NativeContractKind,
    NativeContractRecord, NativeContractState, NativeDeviceRecord, NativeDeviceRequestRecord,
    NativeDomainRecord, NativeDriverRecord, NativeEventRecord, NativeEventSourceKind,
    NativeFileStatusRecord, NativeFileSystemStatusRecord, NativeGpuDisplayRecord,
    NativeGpuScanoutRecord, NativeMountPropagationMode, NativeNetworkEventWatchConfig,
    NativeNetworkInterfaceRecord, NativeNetworkSocketRecord, NativeObjectKind,
    NativeProcessIdentityRecord, NativeProcessRecord, NativeResourceArbitrationPolicy,
    NativeResourceCancelRecord, NativeResourceClaimRecord, NativeResourceContractPolicy,
    NativeResourceEventWatchConfig, NativeResourceGovernanceMode, NativeResourceIssuerPolicy,
    NativeResourceKind, NativeResourceRecord, NativeResourceReleaseRecord, NativeResourceState,
    NativeSchedulerClass, NativeStorageLineageEntry, NativeStorageLineageRecord,
    NativeStorageVolumeRecord, NativeSystemSnapshotRecord, SYS_ACQUIRE_RESOURCE,
    SYS_ADVISE_MEMORY_RANGE, SYS_ATTACH_BUS_PEER, SYS_BIND_UDP_SOCKET,
    SYS_BLOCKED_PENDING_SIGNALS, SYS_BOOT_REPORT, SYS_CANCEL_RESOURCE_CLAIM, SYS_CHDIR_PATH,
    SYS_CHMOD_PATH, SYS_CHOWN_PATH, SYS_CLAIM_RESOURCE,
    SYS_CLOSE, SYS_COMPLETE_NET_TX, SYS_CONFIGURE_NETIF_ADMIN, SYS_CONFIGURE_NETIF_IPV4,
    SYS_CONNECT_UDP_SOCKET, SYS_CREATE_BUS_ENDPOINT, SYS_CREATE_BUS_PEER, SYS_CREATE_CONTRACT,
    SYS_CREATE_DOMAIN, SYS_CREATE_EVENT_QUEUE, SYS_CREATE_RESOURCE, SYS_DETACH_BUS_PEER, SYS_DUP,
    SYS_FCNTL, SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME, SYS_GET_PROCESS_CWD,
    SYS_GET_PROCESS_IDENTITY, SYS_GET_PROCESS_IMAGE_PATH, SYS_GET_PROCESS_NAME,
    SYS_GET_PROCESS_ROOT, SYS_GET_RESOURCE_NAME, SYS_INSPECT_BUS_ENDPOINT, SYS_INSPECT_BUS_PEER,
    SYS_INSPECT_CONTRACT, SYS_INSPECT_DEVICE, SYS_INSPECT_DEVICE_REQUEST, SYS_INSPECT_DOMAIN,
    SYS_INSPECT_DRIVER, SYS_INSPECT_GPU_DISPLAY, SYS_INSPECT_GPU_SCANOUT, SYS_INSPECT_MOUNT,
    SYS_INSPECT_NETIF, SYS_INSPECT_NETSOCK, SYS_INSPECT_PROCESS, SYS_INSPECT_PROCESS_COMPAT,
    SYS_INSPECT_RESOURCE, SYS_INSPECT_STORAGE_LINEAGE, SYS_INSPECT_STORAGE_VOLUME,
    SYS_INSPECT_SYSTEM_SNAPSHOT, SYS_INVOKE_CONTRACT, SYS_LINK_PATH, SYS_LIST_BUS_ENDPOINTS,
    SYS_LIST_BUS_PEERS, SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS, SYS_LIST_PATH, SYS_LIST_PROCESSES,
    SYS_LIST_RESOURCE_WAITERS, SYS_LIST_RESOURCES, SYS_LOAD_MEMORY_WORD, SYS_LSTAT_PATH,
    SYS_MAP_ANONYMOUS_MEMORY, SYS_MAP_FILE_MEMORY, SYS_MKCHAN_PATH, SYS_MKDIR_PATH,
    SYS_MKFILE_PATH, SYS_MKSOCK_PATH, SYS_MOUNT_STORAGE_VOLUME, SYS_OPEN_PATH, SYS_PAUSE_PROCESS,
    SYS_PENDING_SIGNALS, SYS_POLL, SYS_PREPARE_STORAGE_COMMIT, SYS_PRESENT_GPU_FRAME,
    SYS_PROTECT_MEMORY_RANGE, SYS_PUBLISH_BUS_MESSAGE, SYS_QUARANTINE_VM_OBJECT, SYS_READ,
    SYS_READ_GPU_SCANOUT_FRAME, SYS_READ_PROCFS, SYS_READLINK_PATH, SYS_REAP_PROCESS,
    SYS_RECEIVE_BUS_MESSAGE, SYS_RECLAIM_MEMORY_PRESSURE, SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL,
    SYS_RECOVER_STORAGE_VOLUME, SYS_RELEASE_VM_OBJECT,
    SYS_RECVFROM_UDP_SOCKET, SYS_RELEASE_CLAIMED_RESOURCE, SYS_RELEASE_RESOURCE,
    SYS_REMOVE_BUS_EVENTS, SYS_REMOVE_GRAPHICS_EVENTS, SYS_REMOVE_NET_EVENTS,
    SYS_REMOVE_PROCESS_EVENTS, SYS_REMOVE_RESOURCE_EVENTS, SYS_RENAME_PATH, SYS_RENICE_PROCESS,
    SYS_REPAIR_STORAGE_SNAPSHOT, SYS_RESUME_PROCESS, SYS_SEEK, SYS_SEND_SIGNAL,
    SYS_SENDTO_UDP_SOCKET, SYS_SET_CONTRACT_STATE, SYS_SET_MOUNT_PROPAGATION,
    SYS_SET_NETIF_LINK_STATE, SYS_SET_PROCESS_AFFINITY, SYS_SET_PROCESS_ARGS,
    SYS_SET_PROCESS_BREAK, SYS_SET_PROCESS_CWD, SYS_SET_PROCESS_ENV,
    SYS_SET_RESOURCE_CONTRACT_POLICY, SYS_SET_RESOURCE_GOVERNANCE, SYS_SET_RESOURCE_ISSUER_POLICY,
    SYS_SET_RESOURCE_POLICY, SYS_SET_RESOURCE_STATE, SYS_SPAWN_CONFIGURED_PROCESS,
    SYS_SPAWN_PATH_PROCESS, SYS_SPAWN_PROCESS_COPY_VM, SYS_STAT_PATH, SYS_STATFS_PATH,
    SYS_STORE_MEMORY_WORD, SYS_SYMLINK_PATH, SYS_SYNC_MEMORY_RANGE, SYS_TRANSFER_RESOURCE,
    SYS_TRUNCATE_PATH, SYS_UNLINK_PATH, SYS_UNMAP_MEMORY_RANGE, SYS_UNMOUNT_STORAGE_VOLUME,
    SYS_WAIT_EVENT_QUEUE, SYS_WATCH_BUS_EVENTS, SYS_WATCH_GRAPHICS_EVENTS, SYS_WATCH_NET_EVENTS,
    SYS_WATCH_PROCESS_EVENTS, SYS_WATCH_RESOURCE_EVENTS, SYS_WRITE, SYS_WRITEV, SyscallFrame,
    SyscallReturn, UserIoVec,
};
use ngos_user_runtime::Runtime as UserRuntime;

mod bootproof_tests;
mod bootstrap_entry_tests;
mod bus_agent_tests;
mod compat_primitives_tests;
mod compat_runtime_tests;
mod compat_smoke_tests;
mod event_queue_tests;
mod fd_agent_tests;
mod game_audio_input_tests;
mod game_gfx_alias_tests;
mod game_gfx_translate_tests;
mod game_graphics_tests;
mod game_session_watch_tests;
mod network_agent_tests;
mod nextmind_runtime_tests;
mod repair_ai_tests;
mod resource_agent_tests;
mod semantic_runtime_tests;
mod surface_command_tests;
mod surface_runtime_tests;

fn mount_propagation_name(mode: u32) -> &'static str {
    match NativeMountPropagationMode::from_raw(mode) {
        Some(NativeMountPropagationMode::Private) => "private",
        Some(NativeMountPropagationMode::Shared) => "shared",
        Some(NativeMountPropagationMode::Slave) => "slave",
        None => "unknown",
    }
}

#[derive(Clone, Debug)]
struct RecordedProcessBootstrap {
    pid: u64,
    name: String,
    image_path: String,
    cwd: String,
    argv: Vec<String>,
    envp: Vec<String>,
}

#[derive(Clone, Debug)]
struct VmMappingRecord {
    pid: u64,
    start: u64,
    len: u64,
    readable: bool,
    writable: bool,
    executable: bool,
    label: String,
    file_path: Option<String>,
    file_inode: Option<u64>,
    file_offset: u64,
    private: bool,
    cow: bool,
    present: bool,
    reclaimed: bool,
    quarantined: bool,
    quarantine_reason: u64,
    orphan_bytes: Vec<u8>,
    words: Vec<(u64, u32)>,
}

#[derive(Clone, Debug)]
struct RecordedGpuRequest {
    id: u64,
    record: NativeDeviceRequestRecord,
    payload: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RecordedAudioRequest {
    id: u64,
    record: NativeDeviceRequestRecord,
    payload: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RecordedInputRequest {
    id: u64,
    record: NativeDeviceRequestRecord,
    payload: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RecordedNetworkInterfaceState {
    path: String,
    record: NativeNetworkInterfaceRecord,
}

#[derive(Clone, Debug)]
struct RecordedNetworkSocketState {
    path: String,
    device_path: String,
    record: NativeNetworkSocketRecord,
    pending_rx_payload: Vec<u8>,
    pending_rx_ipv4: [u8; 4],
    pending_rx_port: u16,
}

#[derive(Clone, Debug)]
struct RecordedNetworkEventWatch {
    queue_fd: usize,
    interface_path: String,
    socket_path: Option<String>,
    config: NativeNetworkEventWatchConfig,
}

#[derive(Clone, Debug)]
struct RecordedBusPeerState {
    record: NativeBusPeerRecord,
    name: String,
}

#[derive(Clone, Debug)]
struct RecordedBusEndpointState {
    record: NativeBusEndpointRecord,
    path: String,
    messages: Vec<Vec<u8>>,
}

#[derive(Clone, Debug)]
struct OpenFileRecord {
    fd: usize,
    description_id: usize,
    path: String,
    inode: u64,
    kind: NativeObjectKind,
    deleted: bool,
    orphan_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct LocalMountRecord {
    id: u64,
    device_path: String,
    mount_path: String,
    parent_mount_id: u64,
    peer_group: u64,
    master_group: u64,
    propagation_mode: u32,
    created_mount_root: bool,
}

#[derive(Clone, Copy, Debug)]
struct RecordedStorageLineageEntry {
    generation: u64,
    parent_generation: u64,
    payload_checksum: u64,
    kind_label: [u8; 16],
    state_label: [u8; 16],
    tag_label: [u8; 32],
}

#[derive(Clone, Debug)]
struct RecordedStorageSnapshotEntry {
    relative_path: String,
    kind: NativeObjectKind,
    bytes: Vec<u8>,
    symlink_target: Option<String>,
}

#[derive(Clone, Debug)]
struct RecordedStorageState {
    valid: bool,
    dirty: bool,
    generation: u64,
    parent_generation: u64,
    replay_generation: u64,
    payload_checksum: u64,
    payload_len: u64,
    prepared_commit_count: u64,
    recovered_commit_count: u64,
    repaired_snapshot_count: u64,
    allocation_total_blocks: u64,
    allocation_used_blocks: u64,
    mapped_file_count: u64,
    mapped_extent_count: u64,
    mapped_directory_count: u64,
    mapped_symlink_count: u64,
    volume_id: [u8; 32],
    state_label: [u8; 32],
    last_commit_tag: [u8; 32],
    payload_preview: [u8; 32],
    lineage: [Option<RecordedStorageLineageEntry>; NATIVE_STORAGE_LINEAGE_DEPTH],
    lineage_head: usize,
    lineage_count: usize,
    persisted_entries: Vec<RecordedStorageSnapshotEntry>,
}

struct RecordingBackend {
    frames: RefCell<Vec<SyscallFrame>>,
    stdin: RefCell<Vec<u8>>,
    stdin_offset: Cell<usize>,
    stdout: RefCell<Vec<u8>>,
    next_fd: Cell<usize>,
    next_description_id: Cell<usize>,
    next_inode: Cell<u64>,
    next_pid: Cell<u64>,
    process_bootstraps: RefCell<Vec<RecordedProcessBootstrap>>,
    open_files: RefCell<Vec<OpenFileRecord>>,
    fd_flags: RefCell<Vec<(usize, bool, bool)>>,
    description_nonblock: RefCell<Vec<(usize, bool)>>,
    fd_locks: RefCell<Vec<(String, usize, u16, bool)>>,
    read_offsets: RefCell<Vec<(usize, usize)>>,
    created_paths: RefCell<Vec<(String, NativeObjectKind)>>,
    mounts: RefCell<Vec<LocalMountRecord>>,
    next_mount_id: Cell<u64>,
    path_inodes: RefCell<Vec<(String, u64)>>,
    path_metadata: RefCell<Vec<(String, u32, u32, u32)>>,
    subject_uid: Cell<u32>,
    subject_gid: Cell<u32>,
    symlink_targets: RefCell<Vec<(String, String)>>,
    channel_messages: RefCell<Vec<(String, Vec<Vec<u8>>)>>,
    file_contents: RefCell<Vec<(String, Vec<u8>)>>,
    storage_state: RefCell<Option<RecordedStorageState>>,
    vm_mappings: RefCell<Vec<VmMappingRecord>>,
    vm_decisions: RefCell<Vec<(u64, String)>>,
    vm_episodes: RefCell<Vec<(u64, String)>>,
    next_vm_addr: Cell<u64>,
    system_snapshot_override: RefCell<Option<NativeSystemSnapshotRecord>>,
    event_queue_pending: RefCell<Vec<(usize, Vec<NativeEventRecord>)>>,
    event_queue_modes: RefCell<Vec<(usize, NativeEventQueueMode)>>,
    bus_event_watches: RefCell<Vec<(usize, u64, NativeBusEventWatchConfig)>>,
    resource_event_watches: RefCell<Vec<(usize, u64, NativeResourceEventWatchConfig)>>,
    network_event_watches: RefCell<Vec<RecordedNetworkEventWatch>>,
    event_queue_nonblock: RefCell<Vec<(usize, bool)>>,
    resource_event_queues: RefCell<Vec<usize>>,
    network_interfaces: RefCell<Vec<RecordedNetworkInterfaceState>>,
    network_sockets: RefCell<Vec<RecordedNetworkSocketState>>,
    next_device_request_id: Cell<u64>,
    next_bus_peer_id: Cell<u64>,
    next_bus_endpoint_id: Cell<u64>,
    gpu_requests: RefCell<Vec<RecordedGpuRequest>>,
    bus_peers: RefCell<Vec<RecordedBusPeerState>>,
    bus_endpoints: RefCell<Vec<RecordedBusEndpointState>>,
    gpu_scanout_payload: RefCell<Vec<u8>>,
    audio_requests: RefCell<Vec<RecordedAudioRequest>>,
    audio_completion_payload: RefCell<Vec<u8>>,
    input_requests: RefCell<Vec<RecordedInputRequest>>,
    input_completion_payload: RefCell<Vec<u8>>,
}

impl Default for RecordingBackend {
    fn default() -> Self {
        Self {
            frames: RefCell::new(Vec::new()),
            stdin: RefCell::new(Vec::new()),
            stdin_offset: Cell::new(0),
            stdout: RefCell::new(Vec::new()),
            next_fd: Cell::new(7),
            next_description_id: Cell::new(1),
            next_inode: Cell::new(100),
            next_pid: Cell::new(77),
            process_bootstraps: RefCell::new(Vec::new()),
            open_files: RefCell::new(Vec::new()),
            fd_flags: RefCell::new(Vec::new()),
            description_nonblock: RefCell::new(Vec::new()),
            fd_locks: RefCell::new(Vec::new()),
            read_offsets: RefCell::new(Vec::new()),
            created_paths: RefCell::new(Vec::new()),
            mounts: RefCell::new(Vec::new()),
            next_mount_id: Cell::new(1),
            path_inodes: RefCell::new(Vec::new()),
            path_metadata: RefCell::new(Vec::new()),
            subject_uid: Cell::new(1000),
            subject_gid: Cell::new(1000),
            symlink_targets: RefCell::new(Vec::new()),
            channel_messages: RefCell::new(Vec::new()),
            file_contents: RefCell::new(vec![
                (String::from("/motd"), b"ngos host motd\n".to_vec()),
                (String::from("/etc/motd"), b"ngos host motd\n".to_vec()),
                (
                    String::from("/proc/1/status"),
                    b"Name:\tngos-userland-native\nState:\tRunning\nPid:\t1\nCwd:\t/\n".to_vec(),
                ),
            ]),
            storage_state: RefCell::new(Some(Self::default_storage_state())),
            vm_mappings: RefCell::new(Vec::new()),
            vm_decisions: RefCell::new(Vec::new()),
            vm_episodes: RefCell::new(Vec::new()),
            next_vm_addr: Cell::new(0x1000_0000),
            system_snapshot_override: RefCell::new(None),
            event_queue_pending: RefCell::new(Vec::new()),
            event_queue_modes: RefCell::new(Vec::new()),
            bus_event_watches: RefCell::new(Vec::new()),
            resource_event_watches: RefCell::new(Vec::new()),
            network_event_watches: RefCell::new(Vec::new()),
            event_queue_nonblock: RefCell::new(Vec::new()),
            resource_event_queues: RefCell::new(Vec::new()),
            network_interfaces: RefCell::new(Vec::new()),
            network_sockets: RefCell::new(Vec::new()),
            next_device_request_id: Cell::new(1),
            next_bus_peer_id: Cell::new(51),
            next_bus_endpoint_id: Cell::new(61),
            gpu_requests: RefCell::new(Vec::new()),
            bus_peers: RefCell::new(Vec::new()),
            bus_endpoints: RefCell::new(Vec::new()),
            gpu_scanout_payload: RefCell::new(Vec::new()),
            audio_requests: RefCell::new(Vec::new()),
            audio_completion_payload: RefCell::new(Vec::new()),
            input_requests: RefCell::new(Vec::new()),
            input_completion_payload: RefCell::new(Vec::new()),
        }
    }
}

impl RecordingBackend {
    fn default_storage_state() -> RecordedStorageState {
        let mut volume_id = [0u8; 32];
        let mut state_label = [0u8; 32];
        let mut last_commit_tag = [0u8; 32];
        let mut payload_preview = [0u8; 32];
        fill_fixed_field(&mut volume_id, "ngos-storage0");
        fill_fixed_field(&mut state_label, "clean");
        fill_fixed_field(&mut last_commit_tag, "boot");
        fill_fixed_field(&mut payload_preview, "persist:bootstrap");
        let mut state = RecordedStorageState {
            valid: true,
            dirty: false,
            generation: 1,
            parent_generation: 0,
            replay_generation: 1,
            payload_checksum: Self::storage_checksum(b"persist:bootstrap"),
            payload_len: b"persist:bootstrap".len() as u64,
            prepared_commit_count: 0,
            recovered_commit_count: 0,
            repaired_snapshot_count: 0,
            allocation_total_blocks: 16,
            allocation_used_blocks: 1,
            mapped_file_count: 0,
            mapped_extent_count: 1,
            mapped_directory_count: 0,
            mapped_symlink_count: 0,
            volume_id,
            state_label,
            last_commit_tag,
            payload_preview,
            lineage: [None; NATIVE_STORAGE_LINEAGE_DEPTH],
            lineage_head: 0,
            lineage_count: 0,
            persisted_entries: Vec::new(),
        };
        Self::push_storage_lineage_event(&mut state, "snapshot", "clean", "boot");
        state
    }

    fn storage_checksum(bytes: &[u8]) -> u64 {
        let mut checksum = 0xcbf2_9ce4_8422_2325u64;
        for byte in bytes {
            checksum ^= *byte as u64;
            checksum = checksum.wrapping_mul(0x1000_0000_01b3);
        }
        checksum
    }

    fn push_storage_lineage_event(
        state: &mut RecordedStorageState,
        kind: &str,
        status: &str,
        tag: &str,
    ) {
        let mut entry = RecordedStorageLineageEntry {
            generation: state.generation,
            parent_generation: state.parent_generation,
            payload_checksum: state.payload_checksum,
            kind_label: [0; 16],
            state_label: [0; 16],
            tag_label: [0; 32],
        };
        fill_fixed_field(&mut entry.kind_label, kind);
        fill_fixed_field(&mut entry.state_label, status);
        fill_fixed_field(&mut entry.tag_label, tag);
        state.lineage[state.lineage_head] = Some(entry);
        state.lineage_head = (state.lineage_head + 1) % state.lineage.len();
        state.lineage_count = state
            .lineage_count
            .saturating_add(1)
            .min(state.lineage.len());
    }

    fn with_storage_state<T>(
        &self,
        f: impl FnOnce(&mut RecordedStorageState) -> Result<T, Errno>,
    ) -> Result<T, Errno> {
        let mut storage = self.storage_state.borrow_mut();
        let Some(state) = storage.as_mut() else {
            return Err(Errno::Nxio);
        };
        f(state)
    }

    fn mounted_storage_roots(&self) -> Vec<String> {
        self.mounts
            .borrow()
            .iter()
            .filter(|record| record.device_path == "/dev/storage0")
            .map(|record| record.mount_path.clone())
            .collect()
    }

    fn storage_tree_metrics_for_roots(&self, roots: &[String]) -> (u64, u64, u64, u64) {
        let created = self.created_paths.borrow();
        let mut files = 0u64;
        let mut dirs = 0u64;
        let mut symlinks = 0u64;
        for (path, kind) in created.iter() {
            if !roots
                .iter()
                .any(|root| path == root || path.starts_with(&(root.clone() + "/")))
            {
                continue;
            }
            if roots.iter().any(|root| path == root) {
                continue;
            }
            match kind {
                NativeObjectKind::File => files += 1,
                NativeObjectKind::Directory => dirs += 1,
                NativeObjectKind::Symlink => symlinks += 1,
                _ => {}
            }
        }
        let extents = files.saturating_mul(2).saturating_add(symlinks).max(1);
        (files, dirs, symlinks, extents)
    }

    fn storage_payload_preview(payload: &[u8]) -> [u8; 32] {
        let mut preview = [0u8; 32];
        let count = payload.len().min(preview.len());
        preview[..count].copy_from_slice(&payload[..count]);
        preview
    }

    fn normalize_absolute_path(path: &str) -> Result<String, Errno> {
        if path.is_empty() || !path.starts_with('/') {
            return Err(Errno::Inval);
        }
        let mut segments = Vec::<&str>::new();
        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(Errno::Inval);
                    }
                }
                value => segments.push(value),
            }
        }
        if segments.is_empty() {
            Ok(String::from("/"))
        } else {
            Ok(format!("/{}", segments.join("/")))
        }
    }

    fn normalize_relative_target(target: &str) -> Result<String, Errno> {
        if target.is_empty() {
            return Err(Errno::Inval);
        }
        if target.starts_with('/') {
            return Self::normalize_absolute_path(target);
        }
        let mut segments = Vec::<&str>::new();
        for segment in target.split('/') {
            match segment {
                "" | "." => {}
                ".." => segments.push(".."),
                value => segments.push(value),
            }
        }
        if segments.is_empty() {
            Ok(String::from("."))
        } else {
            Ok(segments.join("/"))
        }
    }

    fn parent_path(path: &str) -> &str {
        if path == "/" {
            return "/";
        }
        match path.rfind('/') {
            Some(0) => "/",
            Some(index) => &path[..index],
            None => "/",
        }
    }

    fn join_relative_target(base_path: &str, target: &str) -> Result<String, Errno> {
        if target.starts_with('/') {
            return Self::normalize_absolute_path(target);
        }
        let base = Self::normalize_absolute_path(Self::parent_path(base_path))?;
        let mut segments = base
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        for segment in target.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(Errno::Inval);
                    }
                }
                value => segments.push(value.to_string()),
            }
        }
        if segments.is_empty() {
            Ok(String::from("/"))
        } else {
            Ok(format!("/{}", segments.join("/")))
        }
    }

    fn with_stdin(input: &[u8]) -> Self {
        Self {
            frames: RefCell::new(Vec::new()),
            stdin: RefCell::new(input.to_vec()),
            stdin_offset: Cell::new(0),
            stdout: RefCell::new(Vec::new()),
            next_fd: Cell::new(7),
            next_description_id: Cell::new(1),
            next_inode: Cell::new(100),
            next_pid: Cell::new(77),
            process_bootstraps: RefCell::new(Vec::new()),
            open_files: RefCell::new(Vec::new()),
            fd_flags: RefCell::new(Vec::new()),
            description_nonblock: RefCell::new(Vec::new()),
            fd_locks: RefCell::new(Vec::new()),
            read_offsets: RefCell::new(Vec::new()),
            next_bus_peer_id: Cell::new(51),
            next_bus_endpoint_id: Cell::new(61),
            created_paths: RefCell::new(Vec::new()),
            bus_peers: RefCell::new(Vec::new()),
            bus_endpoints: RefCell::new(Vec::new()),
            mounts: RefCell::new(Vec::new()),
            next_mount_id: Cell::new(1),
            path_inodes: RefCell::new(Vec::new()),
            path_metadata: RefCell::new(Vec::new()),
            subject_uid: Cell::new(1000),
            subject_gid: Cell::new(1000),
            symlink_targets: RefCell::new(Vec::new()),
            channel_messages: RefCell::new(Vec::new()),
            file_contents: RefCell::new(vec![
                (String::from("/motd"), b"ngos host motd\n".to_vec()),
                (String::from("/etc/motd"), b"ngos host motd\n".to_vec()),
                (
                    String::from("/proc/1/status"),
                    b"Name:\tngos-userland-native\nState:\tRunning\nPid:\t1\nCwd:\t/\n".to_vec(),
                ),
            ]),
            storage_state: RefCell::new(Some(Self::default_storage_state())),
            vm_mappings: RefCell::new(Vec::new()),
            vm_decisions: RefCell::new(Vec::new()),
            vm_episodes: RefCell::new(Vec::new()),
            next_vm_addr: Cell::new(0x1000_0000),
            system_snapshot_override: RefCell::new(None),
            event_queue_pending: RefCell::new(Vec::new()),
            event_queue_modes: RefCell::new(Vec::new()),
            bus_event_watches: RefCell::new(Vec::new()),
            resource_event_watches: RefCell::new(Vec::new()),
            network_event_watches: RefCell::new(Vec::new()),
            event_queue_nonblock: RefCell::new(Vec::new()),
            resource_event_queues: RefCell::new(Vec::new()),
            network_interfaces: RefCell::new(Vec::new()),
            network_sockets: RefCell::new(Vec::new()),
            next_device_request_id: Cell::new(1),
            gpu_requests: RefCell::new(Vec::new()),
            gpu_scanout_payload: RefCell::new(Vec::new()),
            audio_requests: RefCell::new(Vec::new()),
            audio_completion_payload: RefCell::new(Vec::new()),
            input_requests: RefCell::new(Vec::new()),
            input_completion_payload: RefCell::new(Vec::new()),
        }
    }

    fn open_record(&self, fd: usize) -> Option<OpenFileRecord> {
        self.open_files
            .borrow()
            .iter()
            .rev()
            .find(|record| record.fd == fd)
            .cloned()
    }

    fn set_system_snapshot_override(&self, record: NativeSystemSnapshotRecord) {
        self.system_snapshot_override.replace(Some(record));
    }

    fn opened_path_raw(&self, fd: usize) -> Option<String> {
        self.open_record(fd).map(|record| record.path)
    }

    fn opened_path(&self, fd: usize) -> Option<String> {
        self.open_record(fd).map(|record| {
            if record.deleted {
                format!("{} (deleted)", record.path)
            } else {
                record.path
            }
        })
    }

    fn fd_flags(&self, fd: usize) -> (bool, bool) {
        let cloexec = self
            .fd_flags
            .borrow()
            .iter()
            .rev()
            .find(|(open_fd, _, _)| *open_fd == fd)
            .map(|(_, _, cloexec)| *cloexec)
            .unwrap_or(false);
        let nonblock = self
            .open_record(fd)
            .map(|record| self.description_nonblock(record.description_id))
            .unwrap_or_else(|| {
                self.fd_flags
                    .borrow()
                    .iter()
                    .rev()
                    .find(|(open_fd, _, _)| *open_fd == fd)
                    .map(|(_, nonblock, _)| *nonblock)
                    .unwrap_or(false)
            });
        (nonblock, cloexec)
    }

    fn set_fd_nonblock(&self, fd: usize, nonblock: bool) {
        if let Some(record) = self.open_record(fd) {
            self.set_description_nonblock(record.description_id, nonblock);
            return;
        }
        if let Some((_, value)) = self
            .event_queue_nonblock
            .borrow_mut()
            .iter_mut()
            .find(|(queue_fd, _)| *queue_fd == fd)
        {
            *value = nonblock;
        }
        let mut flags = self.fd_flags.borrow_mut();
        if let Some((_, value, _)) = flags.iter_mut().find(|(open_fd, _, _)| *open_fd == fd) {
            *value = nonblock;
        } else {
            flags.push((fd, nonblock, false));
        }
    }

    fn set_fd_cloexec(&self, fd: usize, cloexec: bool) {
        let mut flags = self.fd_flags.borrow_mut();
        if let Some((_, _, value)) = flags.iter_mut().find(|(open_fd, _, _)| *open_fd == fd) {
            *value = cloexec;
        } else {
            flags.push((fd, false, cloexec));
        }
    }

    fn description_nonblock(&self, description_id: usize) -> bool {
        self.description_nonblock
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| *candidate == description_id)
            .map(|(_, nonblock)| *nonblock)
            .unwrap_or(false)
    }

    fn set_description_nonblock(&self, description_id: usize, nonblock: bool) {
        let mut flags = self.description_nonblock.borrow_mut();
        if let Some((_, value)) = flags
            .iter_mut()
            .find(|(candidate, _)| *candidate == description_id)
        {
            *value = nonblock;
        } else {
            flags.push((description_id, nonblock));
        }
    }

    fn description_is_open(&self, description_id: usize) -> bool {
        self.open_files
            .borrow()
            .iter()
            .any(|record| record.description_id == description_id)
    }

    fn query_fd_lock(&self, fd: usize) -> usize {
        let Some(record) = self.open_record(fd) else {
            return 0;
        };
        self.fd_locks
            .borrow()
            .iter()
            .find(|(candidate, _, _, _)| candidate == &record.path)
            .map(|(_, _, token, _)| *token as usize)
            .unwrap_or(0)
    }

    fn try_lock_fd(&self, fd: usize, token: u16) -> Result<usize, Errno> {
        if token == 0 {
            return Err(Errno::Inval);
        }
        let record = self.open_record(fd).ok_or(Errno::Badf)?;
        let mut locks = self.fd_locks.borrow_mut();
        if let Some((_, owner_fd, owner_token, exclusive)) = locks
            .iter()
            .find(|(candidate, _, _, _)| candidate == &record.path)
        {
            if *exclusive && *owner_fd == record.description_id && *owner_token == token {
                return Ok(token as usize);
            }
            return Err(Errno::Busy);
        }
        locks.push((record.path, record.description_id, token, true));
        Ok(token as usize)
    }

    fn unlock_fd(&self, fd: usize, token: u16) -> Result<usize, Errno> {
        let record = self.open_record(fd).ok_or(Errno::Badf)?;
        let mut locks = self.fd_locks.borrow_mut();
        let Some(index) = locks
            .iter()
            .position(|(candidate, _, _, exclusive)| candidate == &record.path && *exclusive)
        else {
            return Err(Errno::NoEnt);
        };
        let (_, owner_fd, owner_token, _) = locks[index];
        if owner_fd != record.description_id || owner_token != token {
            return Err(Errno::Perm);
        }
        locks.remove(index);
        Ok(token as usize)
    }

    fn try_lock_fd_shared(&self, fd: usize, token: u16) -> Result<usize, Errno> {
        if token == 0 {
            return Err(Errno::Inval);
        }
        let record = self.open_record(fd).ok_or(Errno::Badf)?;
        let mut locks = self.fd_locks.borrow_mut();
        if locks
            .iter()
            .any(|(candidate, owner_fd, owner_token, exclusive)| {
                candidate == &record.path
                    && *exclusive
                    && !(*owner_fd == record.description_id && *owner_token == token)
            })
        {
            return Err(Errno::Busy);
        }
        if let Some((_, _, existing_token, _)) =
            locks.iter().find(|(candidate, owner_fd, _, exclusive)| {
                candidate == &record.path && !*exclusive && *owner_fd == record.description_id
            })
        {
            if *existing_token == token {
                return Ok(token as usize);
            }
        }
        locks.push((record.path, record.description_id, token, false));
        Ok(token as usize)
    }

    fn unlock_fd_shared(&self, fd: usize, token: u16) -> Result<usize, Errno> {
        let record = self.open_record(fd).ok_or(Errno::Badf)?;
        let mut locks = self.fd_locks.borrow_mut();
        let Some(index) = locks
            .iter()
            .position(|(candidate, owner_fd, owner_token, exclusive)| {
                candidate == &record.path
                    && !*exclusive
                    && *owner_fd == record.description_id
                    && *owner_token == token
            })
        else {
            return Err(Errno::NoEnt);
        };
        locks.remove(index);
        Ok(token as usize)
    }

    fn object_lock_conflict(
        &self,
        path: &str,
        actor_description: Option<usize>,
    ) -> Result<(), Errno> {
        let path = Self::normalize_absolute_path(path)?;
        let protected_paths = if self.created_kind(&path) == Some(NativeObjectKind::File) {
            let inode = self.inode_for_path(&path);
            let mut linked = self.all_paths_for_inode(inode);
            if linked.is_empty() {
                linked.push(path.clone());
            }
            linked
        } else {
            vec![path.clone()]
        };
        let locks = self.fd_locks.borrow();
        if locks.iter().any(|(candidate, owner_description, _, _)| {
            protected_paths
                .iter()
                .any(|protected| protected == candidate)
                && actor_description.is_none_or(|actor| actor != *owner_description)
        }) {
            Err(Errno::Busy)
        } else {
            Ok(())
        }
    }

    fn procfs_fd_listing(&self) -> String {
        let mut entries = vec![
            String::from("0 [stdio:stdin]\tFile\t[stdio:stdin]\tcloexec=false\tnonblock=false"),
            String::from("1 [stdio:stdout]\tFile\t[stdio:stdout]\tcloexec=false\tnonblock=false"),
            String::from("2 [stdio:stderr]\tFile\t[stdio:stderr]\tcloexec=false\tnonblock=false"),
        ];
        let open_files = self.open_files.borrow();
        for record in open_files.iter() {
            let kind = match record.kind {
                NativeObjectKind::Directory => "Directory",
                NativeObjectKind::Channel => "Channel",
                NativeObjectKind::Symlink => "Symlink",
                _ => "File",
            };
            let (nonblock, cloexec) = self.fd_flags(record.fd);
            let path = if record.deleted {
                format!("{} (deleted)", record.path)
            } else {
                record.path.clone()
            };
            entries.push(format!(
                "{}\t{}\t{}\tcloexec={}\tnonblock={}",
                record.fd, kind, path, cloexec, nonblock
            ));
        }
        entries.sort();
        let mut text = entries.join("\n");
        text.push('\n');
        text
    }

    fn procfs_fdinfo_payload(&self, fd: u64) -> Option<Vec<u8>> {
        let (path, kind, pos, nonblock, cloexec, rights) = match fd {
            0 => ("stdin".to_string(), "File", 0usize, false, false, 0x3u32),
            1 => ("stdout".to_string(), "File", 0usize, false, false, 0x2u32),
            2 => ("stderr".to_string(), "File", 0usize, false, false, 0x2u32),
            _ => {
                let record = self.open_record(fd as usize)?;
                let kind = match record.kind {
                    NativeObjectKind::Directory => "Directory",
                    NativeObjectKind::Channel => "Channel",
                    NativeObjectKind::Symlink => "Symlink",
                    _ => "File",
                };
                let pos = self.read_offset(fd as usize);
                let (nonblock, cloexec) = self.fd_flags(fd as usize);
                let path = if record.deleted {
                    format!("{} (deleted)", record.path)
                } else {
                    record.path
                };
                (path, kind, pos, nonblock, cloexec, 0x3u32)
            }
        };
        Some(
            format!(
                "fd:\t{fd}\npath:\t{path}\nkind:\t{kind}\npos:\t{pos}\nflags:\tcloexec={cloexec} nonblock={nonblock}\nrights:\t0x{rights:x}\n"
            )
            .into_bytes(),
        )
    }

    fn procfs_cap_listing(&self, pid: u64) -> String {
        let count = if pid == 1 { 2 } else { 0 };
        let mut text = (0..count)
            .map(|index| format!("capability:{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        text
    }

    fn procfs_auxv_listing(&self, pid: u64) -> String {
        let image_path = self
            .recorded_process(pid)
            .map(|process| process.image_path)
            .unwrap_or_else(|| String::from("/bin/ngos-userland-native"));
        format!("AT_PAGESZ=4096\nAT_ENTRY=0x401000\nAT_EXECFN={image_path}\n")
    }

    fn procfs_mount_listing(&self, pid: u64) -> String {
        if pid != 1 {
            return String::new();
        }
        let mut lines = vec![String::from(
            "/\tdevice=rootfs\tmode=private\tpeer_group=0\tmaster_group=0\tcreated_root=yes",
        )];
        let mut mounts = self.mounts.borrow().clone();
        mounts.sort_by(|left, right| left.mount_path.cmp(&right.mount_path));
        for record in mounts {
            lines.push(format!(
                "{}\tdevice={}\tmode={}\tpeer_group={}\tmaster_group={}\tcreated_root={}",
                record.mount_path,
                record.device_path,
                mount_propagation_name(record.propagation_mode),
                record.peer_group,
                record.master_group,
                if record.created_mount_root {
                    "yes"
                } else {
                    "no"
                },
            ));
        }
        lines.join("\n") + "\n"
    }

    fn file_content(&self, path: &str) -> Vec<u8> {
        if let Some(payload) = self.recorded_procfs_payload(path) {
            return payload;
        }
        match path {
            "/proc/1/cwd" => return b"/".to_vec(),
            "/proc/1/exe" => return b"/bin/ngos-userland-native".to_vec(),
            "/proc/1/root" => return b"/".to_vec(),
            "/proc/1/fd" => return self.procfs_fd_listing().into_bytes(),
            "/proc/1/auxv" => return self.procfs_auxv_listing(1).into_bytes(),
            "/proc/1/mounts" => return self.procfs_mount_listing(1).into_bytes(),
            "/proc/1/vfslocks" => return Vec::new(),
            "/proc/1/vfswatches" => return Vec::new(),
            "/proc/1/vfsstats" => {
                return b"live: nodes=1 orphans=0 locks=0 watches=0 mounts=1\n".to_vec()
            }
            "/proc/1/caps" => return self.procfs_cap_listing(1).into_bytes(),
            "/proc/1/queues" => return self.procfs_queue_listing(1).into_bytes(),
            "/proc/1/fdinfo/0" => {
                return b"fd:\t0\npath:\t[stdio:stdin]\nkind:\tFile\npos:\t0\nflags:\tcloexec=false nonblock=false\nrights:\t0x3\n"
                    .to_vec()
            }
            "/proc/1/maps" => return self.render_proc_maps(1),
            "/proc/1/vmdecisions" => return self.render_vm_decisions(1),
            "/proc/1/vmobjects" => return self.render_vm_objects(1),
            "/proc/1/vmepisodes" => return self.render_vm_episodes(1),
            "/proc/2/vmdecisions" => return self.render_vm_decisions(2),
            "/proc/2/vmobjects" => return self.render_vm_objects(2),
            "/proc/2/vmepisodes" => return self.render_vm_episodes(2),
            "/proc/3/vmdecisions" => return self.render_vm_decisions(3),
            "/proc/3/vmobjects" => return self.render_vm_objects(3),
            "/proc/3/vmepisodes" => return self.render_vm_episodes(3),
            "/proc/1/cmdline" => return b"ngos-userland-native\0".to_vec(),
            "/proc/1/environ" => {
                return b"NGOS_SESSION=1\0NGOS_SESSION_PROTOCOL=kernel-launch\0".to_vec()
            }
            _ => {}
        }
        self.file_contents
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == path)
            .map(|(_, bytes)| bytes.clone())
            .unwrap_or_default()
    }

    fn file_content_for_fd(&self, fd: usize) -> Option<Vec<u8>> {
        let record = self.open_record(fd)?;
        if record.deleted {
            return Some(record.orphan_bytes);
        }
        Some(self.file_content(&record.path))
    }

    fn created_kind(&self, path: &str) -> Option<NativeObjectKind> {
        if path == "/" {
            return Some(NativeObjectKind::Directory);
        }
        let path = Self::normalize_absolute_path(path).ok()?;
        self.created_paths
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == &path)
            .map(|(_, kind)| *kind)
    }

    fn symlink_target(&self, path: &str) -> Option<String> {
        let path = Self::normalize_absolute_path(path).ok()?;
        self.symlink_targets
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == &path)
            .map(|(_, target)| target.clone())
    }

    fn path_exists(&self, path: &str) -> bool {
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return false;
        };
        if path == "/" {
            return true;
        }
        self.created_kind(&path).is_some()
            || self
                .file_contents
                .borrow()
                .iter()
                .any(|(candidate, _)| candidate == &path)
            || self.symlink_target(&path).is_some()
            || path.starts_with("/proc/")
            || path.starts_with("/dev/")
            || path.starts_with("/drv/")
    }

    fn parent_directory_exists(&self, path: &str) -> bool {
        let parent = Self::parent_path(path);
        self.created_kind(parent) == Some(NativeObjectKind::Directory)
    }

    fn metadata_for_path(&self, path: &str) -> (u32, u32, u32) {
        if path == "/" {
            return (1000, 1000, 0o755);
        }
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return (0, 0, 0);
        };
        self.path_metadata
            .borrow()
            .iter()
            .find(|(candidate, _, _, _)| candidate == &path)
            .map(|(_, owner, group, mode)| (*owner, *group, *mode))
            .unwrap_or((1000, 1000, 0o644))
    }

    fn next_inode(&self) -> u64 {
        let inode = self.next_inode.get();
        self.next_inode.set(inode.saturating_add(1));
        inode
    }

    fn inode_for_path(&self, path: &str) -> u64 {
        self.path_inodes
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == path)
            .map(|(_, inode)| *inode)
            .unwrap_or(99)
    }

    fn set_path_inode(&self, path: &str, inode: u64) {
        let mut inodes = self.path_inodes.borrow_mut();
        if let Some((_, value)) = inodes.iter_mut().find(|(candidate, _)| candidate == path) {
            *value = inode;
        } else {
            inodes.push((path.to_string(), inode));
        }
    }

    fn remove_path_inode(&self, path: &str) {
        self.path_inodes
            .borrow_mut()
            .retain(|(candidate, _)| candidate != path);
    }

    fn all_paths_for_inode(&self, inode: u64) -> Vec<String> {
        self.path_inodes
            .borrow()
            .iter()
            .filter(|(_, candidate_inode)| *candidate_inode == inode)
            .map(|(path, _)| path.clone())
            .collect()
    }

    fn next_mount_id(&self) -> u64 {
        let id = self.next_mount_id.get();
        self.next_mount_id.set(id.saturating_add(1));
        id
    }

    fn mount_by_path(&self, path: &str) -> Option<LocalMountRecord> {
        let path = Self::normalize_absolute_path(path).ok()?;
        self.mounts
            .borrow()
            .iter()
            .find(|record| record.mount_path == path)
            .cloned()
    }

    fn mount_owner_for_path(&self, path: &str) -> Option<LocalMountRecord> {
        let path = Self::normalize_absolute_path(path).ok()?;
        self.mounts
            .borrow()
            .iter()
            .filter(|record| {
                path == record.mount_path || path.starts_with(&(record.mount_path.clone() + "/"))
            })
            .max_by_key(|record| record.mount_path.len())
            .cloned()
    }

    fn mount_parent(&self, path: &str) -> Option<LocalMountRecord> {
        let path = Self::normalize_absolute_path(path).ok()?;
        self.mounts
            .borrow()
            .iter()
            .filter(|record| {
                path != record.mount_path && path.starts_with(&(record.mount_path.clone() + "/"))
            })
            .max_by_key(|record| record.mount_path.len())
            .cloned()
    }

    fn mount_relative_suffix(parent_mount_path: &str, child_mount_path: &str) -> Option<String> {
        if child_mount_path == parent_mount_path {
            return Some(String::new());
        }
        let prefix = format!("{parent_mount_path}/");
        child_mount_path
            .strip_prefix(&prefix)
            .map(|suffix| format!("/{suffix}"))
    }

    fn mount_propagation_clones(
        &self,
        parent: &LocalMountRecord,
        child_mount_path: &str,
    ) -> Vec<(String, NativeMountPropagationMode)> {
        let Some(relative_suffix) =
            Self::mount_relative_suffix(&parent.mount_path, child_mount_path)
        else {
            return Vec::new();
        };
        self.mounts
            .borrow()
            .iter()
            .filter(|peer| peer.id != parent.id)
            .filter_map(|peer| {
                if peer.peer_group == parent.peer_group
                    && peer.peer_group != 0
                    && peer.propagation_mode == NativeMountPropagationMode::Shared as u32
                {
                    Some((
                        format!("{}{}", peer.mount_path, relative_suffix),
                        NativeMountPropagationMode::Shared,
                    ))
                } else if peer.master_group == parent.peer_group
                    && parent.peer_group != 0
                    && peer.propagation_mode == NativeMountPropagationMode::Slave as u32
                {
                    Some((
                        format!("{}{}", peer.mount_path, relative_suffix),
                        NativeMountPropagationMode::Slave,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    fn mount_unmount_ids(&self, record: &LocalMountRecord) -> Vec<u64> {
        self.mounts
            .borrow()
            .iter()
            .filter(|candidate| {
                if candidate.id == record.id {
                    return true;
                }
                match NativeMountPropagationMode::from_raw(record.propagation_mode) {
                    Some(NativeMountPropagationMode::Shared) => {
                        (candidate.peer_group != 0 && candidate.peer_group == record.peer_group)
                            || (candidate.master_group != 0
                                && candidate.master_group == record.peer_group)
                    }
                    _ => false,
                }
            })
            .map(|candidate| candidate.id)
            .collect()
    }

    fn mount_has_nested_child_outside(&self, mount_path: &str, active_ids: &[u64]) -> bool {
        let nested_prefix = format!("{mount_path}/");
        self.mounts.borrow().iter().any(|record| {
            !active_ids.iter().any(|id| *id == record.id)
                && record.mount_path.starts_with(&nested_prefix)
        })
    }

    fn mount_entry_count(&self, mount_path: &str) -> usize {
        let Ok(mount_path) = Self::normalize_absolute_path(mount_path) else {
            return 0;
        };
        let prefix = format!("{mount_path}/");
        self.created_paths
            .borrow()
            .iter()
            .filter(|(candidate, _)| candidate == &mount_path || candidate.starts_with(&prefix))
            .count()
    }

    fn snapshot_storage_entries(&self, mount_path: &str) -> Vec<RecordedStorageSnapshotEntry> {
        let Ok(mount_path) = Self::normalize_absolute_path(mount_path) else {
            return Vec::new();
        };
        let prefix = format!("{mount_path}/");
        let mut entries = self
            .created_paths
            .borrow()
            .iter()
            .filter(|(candidate, _)| candidate.starts_with(&prefix))
            .filter_map(|(candidate, kind)| {
                let relative_path = candidate.strip_prefix(&mount_path)?.to_string();
                Some(RecordedStorageSnapshotEntry {
                    relative_path,
                    kind: *kind,
                    bytes: if *kind == NativeObjectKind::File {
                        self.file_content(candidate)
                    } else {
                        Vec::new()
                    },
                    symlink_target: if *kind == NativeObjectKind::Symlink {
                        self.symlink_target(candidate)
                    } else {
                        None
                    },
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| match entry.kind {
            NativeObjectKind::Directory => (0u8, entry.relative_path.clone()),
            NativeObjectKind::File => (1u8, entry.relative_path.clone()),
            NativeObjectKind::Symlink => (2u8, entry.relative_path.clone()),
            _ => (3u8, entry.relative_path.clone()),
        });
        entries
    }

    fn restore_storage_entries(&self, mount_path: &str, entries: &[RecordedStorageSnapshotEntry]) {
        for entry in entries {
            let full_path = format!("{mount_path}{}", entry.relative_path);
            match entry.kind {
                NativeObjectKind::Directory => {
                    self.record_created_path(&full_path, NativeObjectKind::Directory)
                }
                NativeObjectKind::File => {
                    self.record_created_path(&full_path, NativeObjectKind::File);
                    if let Some((_, bytes)) = self
                        .file_contents
                        .borrow_mut()
                        .iter_mut()
                        .find(|(candidate, _)| candidate == &full_path)
                    {
                        *bytes = entry.bytes.clone();
                    } else {
                        self.file_contents
                            .borrow_mut()
                            .push((full_path.clone(), entry.bytes.clone()));
                    }
                }
                NativeObjectKind::Symlink => {
                    if let Some(target) = entry.symlink_target.as_deref() {
                        self.record_symlink_path(&full_path, target);
                    }
                }
                _ => {}
            }
        }
    }

    fn link_count_for_inode(&self, inode: u64) -> u64 {
        self.path_inodes
            .borrow()
            .iter()
            .filter(|(_, candidate_inode)| *candidate_inode == inode)
            .count() as u64
    }

    fn set_path_metadata(&self, path: &str, owner_uid: u32, group_gid: u32, mode: u32) {
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return;
        };
        let mut metadata = self.path_metadata.borrow_mut();
        if let Some((_, owner, group, perms)) = metadata
            .iter_mut()
            .find(|(candidate, _, _, _)| candidate == &path)
        {
            *owner = owner_uid;
            *group = group_gid;
            *perms = mode & 0o7777;
        } else {
            metadata.push((path, owner_uid, group_gid, mode & 0o7777));
        }
    }

    fn remove_path_metadata(&self, path: &str) {
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return;
        };
        self.path_metadata
            .borrow_mut()
            .retain(|(candidate, _, _, _)| candidate != &path);
    }

    fn current_subject(&self) -> (u32, u32) {
        (self.subject_uid.get(), self.subject_gid.get())
    }

    fn set_current_subject(&self, uid: u32, gid: u32) {
        self.subject_uid.set(uid);
        self.subject_gid.set(gid);
    }

    fn permission_mask(mode: u32, owner_uid: u32, group_gid: u32, uid: u32, gid: u32) -> u32 {
        if uid == 0 {
            return 0o7;
        }
        if uid == owner_uid {
            (mode >> 6) & 0o7
        } else if gid == group_gid {
            (mode >> 3) & 0o7
        } else {
            mode & 0o7
        }
    }

    fn require_access(
        &self,
        path: &str,
        read: bool,
        write: bool,
        execute: bool,
    ) -> Result<(), Errno> {
        let (owner_uid, group_gid, mode) = self.metadata_for_path(path);
        let (uid, gid) = self.current_subject();
        let mask = Self::permission_mask(mode, owner_uid, group_gid, uid, gid);
        let needed =
            (u32::from(read) * 0o4) | (u32::from(write) * 0o2) | (u32::from(execute) * 0o1);
        if (mask & needed) == needed {
            Ok(())
        } else {
            Err(Errno::Access)
        }
    }

    fn require_parent_mutation_access(&self, path: &str) -> Result<(), Errno> {
        let parent = Self::parent_path(path);
        self.require_access(parent, false, true, true)
    }

    fn require_sticky_mutation_access(&self, path: &str) -> Result<(), Errno> {
        let parent = Self::parent_path(path);
        let (parent_owner, _, parent_mode) = self.metadata_for_path(parent);
        if parent_mode & 0o1000 == 0 {
            return Ok(());
        }
        let (uid, _) = self.current_subject();
        if uid == 0 || uid == parent_owner {
            return Ok(());
        }
        let (target_owner, _, _) = self.metadata_for_path(path);
        if uid == target_owner {
            Ok(())
        } else {
            Err(Errno::Access)
        }
    }

    fn inherited_metadata_for_new_path(
        &self,
        path: &str,
        kind: NativeObjectKind,
    ) -> (u32, u32, u32) {
        let parent = Self::parent_path(path);
        let (_, parent_gid, parent_mode) = self.metadata_for_path(parent);
        let (uid, gid) = self.current_subject();
        let inherited_gid = if (parent_mode & 0o2000) != 0 {
            parent_gid
        } else {
            gid
        };
        let mut mode = match kind {
            NativeObjectKind::Directory => 0o755,
            NativeObjectKind::File => 0o644,
            NativeObjectKind::Channel => 0o660,
            NativeObjectKind::Symlink => 0o777,
            _ => 0o644,
        };
        if kind == NativeObjectKind::Directory && (parent_mode & 0o2000) != 0 {
            mode |= 0o2000;
        }
        (uid, inherited_gid, mode)
    }

    fn require_traversal_access(
        &self,
        path: &str,
        include_self_directory: bool,
    ) -> Result<(), Errno> {
        let path = Self::normalize_absolute_path(path)?;
        if path == "/" {
            return if include_self_directory {
                self.require_access("/", false, false, true)
            } else {
                Ok(())
            };
        }
        let final_is_directory = self.created_kind(&path) == Some(NativeObjectKind::Directory);
        let segments = path
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        let mut probe = String::from("/");
        for (index, segment) in segments.iter().enumerate() {
            if probe != "/" {
                probe.push('/');
            }
            probe.push_str(segment);
            let is_last = index + 1 == segments.len();
            if !is_last || (include_self_directory && final_is_directory) {
                self.require_access(&probe, false, false, true)?;
            }
        }
        Ok(())
    }

    fn record_created_path(&self, path: &str, kind: NativeObjectKind) {
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return;
        };
        let metadata_path = path.clone();
        let inode = self.next_inode();
        let mut created = self.created_paths.borrow_mut();
        if let Some((_, existing_kind)) =
            created.iter_mut().find(|(candidate, _)| candidate == &path)
        {
            *existing_kind = kind;
        } else {
            created.push((path.clone(), kind));
        }
        let (owner_uid, group_gid, mode) = self.inherited_metadata_for_new_path(&path, kind);
        drop(created);
        self.set_path_inode(&path, inode);
        self.set_path_metadata(&metadata_path, owner_uid, group_gid, mode);
    }

    fn record_symlink_path(&self, path: &str, target: &str) {
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return;
        };
        let Ok(target) = Self::normalize_relative_target(target) else {
            return;
        };
        self.record_created_path(&path, NativeObjectKind::Symlink);
        let mut symlinks = self.symlink_targets.borrow_mut();
        if let Some((_, existing_target)) = symlinks
            .iter_mut()
            .find(|(candidate, _)| candidate == &path)
        {
            *existing_target = target;
        } else {
            symlinks.push((path, target));
        }
    }

    fn rewrite_path_prefix(path: &str, from: &str, to: &str) -> String {
        if path == from {
            return to.to_string();
        }
        let prefix = format!("{from}/");
        if let Some(rest) = path.strip_prefix(&prefix) {
            return format!("{to}/{rest}");
        }
        path.to_string()
    }

    fn list_path_payload(&self, path: &str) -> Vec<u8> {
        if path == "/dev" {
            return b"net0\tDevice\nstorage0\tDevice\ngpu0\tDevice\naudio0\tDevice\ninput0\tDevice\n"
                .to_vec();
        }
        if path == "/drv" {
            return b"net0\tDriver\nstorage0\tDriver\ngpu0\tDriver\naudio0\tDriver\ninput0\tDriver\n"
                .to_vec();
        }

        let normalized = path.trim_end_matches('/');
        let prefix = if normalized.is_empty() || normalized == "/" {
            String::from("/")
        } else {
            format!("{normalized}/")
        };
        let mut entries = Vec::<(String, &'static str)>::new();
        let mut seen = Vec::<String>::new();

        let mut push_entry = |candidate_path: &str, kind: &'static str| {
            let maybe_name = if prefix == "/" {
                candidate_path.trim_start_matches('/').split('/').next()
            } else {
                candidate_path
                    .strip_prefix(&prefix)
                    .and_then(|rest| rest.split('/').next())
            };
            let Some(name) = maybe_name else {
                return;
            };
            if name.is_empty() {
                return;
            }
            if seen.iter().any(|entry| entry == name) {
                return;
            }
            seen.push(name.to_string());
            entries.push((name.to_string(), kind));
        };

        {
            let created = self.created_paths.borrow();
            for (candidate, kind) in created.iter() {
                let label = match kind {
                    NativeObjectKind::Directory => "Directory",
                    NativeObjectKind::File => "File",
                    NativeObjectKind::Symlink => "Symlink",
                    NativeObjectKind::Channel => "Channel",
                    _ => "File",
                };
                push_entry(candidate, label);
            }
        }

        {
            let files = self.file_contents.borrow();
            for (candidate, _) in files.iter() {
                push_entry(candidate, "File");
            }
        }

        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let mut out = String::new();
        for (name, kind) in entries {
            out.push_str(&name);
            out.push('\t');
            out.push_str(kind);
            out.push('\n');
        }
        out.into_bytes()
    }

    fn push_channel_message(&self, path: &str, bytes: &[u8]) {
        let mut channels = self.channel_messages.borrow_mut();
        if let Some((_, queue)) = channels.iter_mut().find(|(candidate, _)| candidate == path) {
            queue.push(bytes.to_vec());
        } else {
            channels.push((path.to_string(), vec![bytes.to_vec()]));
        }
    }

    fn pop_channel_message(&self, path: &str) -> Option<Vec<u8>> {
        let mut channels = self.channel_messages.borrow_mut();
        let (_, queue) = channels
            .iter_mut()
            .find(|(candidate, _)| candidate == path)?;
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    }

    fn write_file_content_at(
        &self,
        path: &str,
        offset: usize,
        bytes: &[u8],
        actor_description: Option<usize>,
    ) -> Result<(), Errno> {
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return Err(Errno::Inval);
        };
        self.object_lock_conflict(&path, actor_description)?;
        let inode = self.inode_for_path(&path);
        let linked_paths = self.all_paths_for_inode(inode);
        let mut files = self.file_contents.borrow_mut();
        let required_len = offset.saturating_add(bytes.len());
        for linked_path in linked_paths {
            let content = if let Some((_, content)) = files
                .iter_mut()
                .find(|(candidate, _)| candidate == &linked_path)
            {
                content
            } else {
                files.push((linked_path, Vec::new()));
                &mut files.last_mut().expect("just pushed").1
            };
            if content.len() < offset {
                content.resize(offset, 0);
            }
            if content.len() < required_len {
                content.resize(required_len, 0);
            }
            content[offset..required_len].copy_from_slice(bytes);
        }
        Ok(())
    }

    fn truncate_file_content(&self, path: &str, len: usize) -> Result<(), Errno> {
        let path = Self::normalize_absolute_path(path)?;
        if !self.path_exists(&path) {
            return Err(Errno::NoEnt);
        }
        self.require_traversal_access(&path, false)?;
        self.require_access(&path, false, true, false)?;
        match self.created_kind(&path).unwrap_or(NativeObjectKind::File) {
            NativeObjectKind::Directory => return Err(Errno::IsDir),
            NativeObjectKind::Symlink => return Err(Errno::Inval),
            _ => {}
        }
        let mut files = self.file_contents.borrow_mut();
        let inode = self.inode_for_path(&path);
        self.object_lock_conflict(&path, None)?;
        let linked_paths = self.all_paths_for_inode(inode);
        for linked_path in linked_paths {
            if let Some((_, content)) = files
                .iter_mut()
                .find(|(candidate, _)| candidate == &linked_path)
            {
                content.resize(len, 0);
            } else {
                files.push((linked_path, vec![0; len]));
            }
        }
        let refreshed = files
            .iter()
            .find(|(candidate, _)| candidate == &path)
            .map(|(_, content)| content.clone())
            .unwrap_or_else(|| vec![0; len]);
        drop(files);
        self.recompute_mapping_words_from_backing(inode, &refreshed);
        Ok(())
    }

    fn resolve_open_path(&self, path: &str, depth: usize) -> Result<String, Errno> {
        if depth > 8 {
            return Err(Errno::Inval);
        }
        let path = Self::normalize_absolute_path(path)?;
        if let Some(target) = self.symlink_target(&path) {
            let resolved = Self::join_relative_target(&path, &target)?;
            return self.resolve_open_path(&resolved, depth + 1);
        }
        Ok(path)
    }

    fn record_gpu_present(&self, payload: &[u8]) -> u64 {
        let request_id = self.next_device_request_id.get();
        self.next_device_request_id
            .set(request_id.saturating_add(1));
        let payload_text = core::str::from_utf8(payload).unwrap_or_default();
        let frame_tag = find_payload_value(payload_text, "frame=").unwrap_or_default();
        let source_api = find_payload_value(payload_text, "source-api=").unwrap_or_default();
        let translation = find_payload_value(payload_text, "translation=").unwrap_or_default();

        let mut record = NativeDeviceRequestRecord {
            issuer: 1,
            kind: 2,
            state: 2,
            opcode: 0x4750_0001,
            buffer_id: 0,
            payload_len: payload.len() as u64,
            response_len: payload.len() as u64,
            submitted_tick: request_id.saturating_mul(3).saturating_sub(2),
            started_tick: request_id.saturating_mul(3).saturating_sub(1),
            completed_tick: request_id.saturating_mul(3),
            frame_tag: [0; 64],
            source_api_name: [0; 24],
            translation_label: [0; 32],
        };
        fill_fixed_field(&mut record.frame_tag, frame_tag);
        fill_fixed_field(&mut record.source_api_name, source_api);
        fill_fixed_field(&mut record.translation_label, translation);

        self.gpu_requests.borrow_mut().push(RecordedGpuRequest {
            id: request_id,
            record,
            payload: payload.to_vec(),
        });
        *self.gpu_scanout_payload.borrow_mut() = payload.to_vec();
        request_id
    }

    fn gpu_request(&self, request_id: u64) -> Option<RecordedGpuRequest> {
        self.gpu_requests
            .borrow()
            .iter()
            .find(|request| request.id == request_id)
            .cloned()
    }

    fn latest_gpu_request(&self) -> Option<RecordedGpuRequest> {
        self.gpu_requests.borrow().last().cloned()
    }

    fn complete_gpu_request(&self, request_id: u64, payload: &[u8], state: u32) -> bool {
        let mut requests = self.gpu_requests.borrow_mut();
        let Some(request) = requests.iter_mut().find(|request| request.id == request_id) else {
            return false;
        };
        request.record.state = state;
        request.record.response_len = if state == 4 { 0 } else { payload.len() as u64 };
        request.record.completed_tick = request_id.saturating_mul(3);
        if state != 4 {
            request.payload = payload.to_vec();
            *self.gpu_scanout_payload.borrow_mut() = payload.to_vec();
        }
        true
    }

    fn record_audio_submit(&self, payload: &[u8]) -> u64 {
        let request_id = self.next_device_request_id.get();
        self.next_device_request_id
            .set(request_id.saturating_add(1));
        let payload_text = core::str::from_utf8(payload).unwrap_or_default();
        let stream_tag = find_payload_value(payload_text, "stream=").unwrap_or_default();
        let source_api = find_payload_value(payload_text, "source-api=").unwrap_or_default();
        let translation = find_payload_value(payload_text, "translation=").unwrap_or_default();

        let mut record = NativeDeviceRequestRecord {
            issuer: 1,
            kind: 1,
            state: 1,
            opcode: 0x4155_0001,
            buffer_id: 0,
            payload_len: payload.len() as u64,
            response_len: 0,
            submitted_tick: request_id.saturating_mul(3).saturating_sub(2),
            started_tick: request_id.saturating_mul(3).saturating_sub(1),
            completed_tick: 0,
            frame_tag: [0; 64],
            source_api_name: [0; 24],
            translation_label: [0; 32],
        };
        fill_fixed_field(&mut record.frame_tag, stream_tag);
        fill_fixed_field(&mut record.source_api_name, source_api);
        fill_fixed_field(&mut record.translation_label, translation);

        self.audio_requests.borrow_mut().push(RecordedAudioRequest {
            id: request_id,
            record,
            payload: payload.to_vec(),
        });
        request_id
    }

    fn complete_audio_request(&self, request_id: u64, payload: &[u8], state: u32) -> bool {
        let mut requests = self.audio_requests.borrow_mut();
        let Some(request) = requests.iter_mut().find(|request| request.id == request_id) else {
            return false;
        };
        request.record.state = state;
        request.record.response_len = if state == 4 { 0 } else { payload.len() as u64 };
        request.record.completed_tick = request_id.saturating_mul(3);
        if state != 4 {
            request.payload = payload.to_vec();
            *self.audio_completion_payload.borrow_mut() = payload.to_vec();
        }
        true
    }

    fn audio_request(&self, request_id: u64) -> Option<RecordedAudioRequest> {
        self.audio_requests
            .borrow()
            .iter()
            .find(|request| request.id == request_id)
            .cloned()
    }

    fn latest_audio_request(&self) -> Option<RecordedAudioRequest> {
        self.audio_requests.borrow().last().cloned()
    }

    fn record_input_submit(&self, payload: &[u8]) -> u64 {
        let request_id = self.next_device_request_id.get();
        self.next_device_request_id
            .set(request_id.saturating_add(1));
        let payload_text = core::str::from_utf8(payload).unwrap_or_default();
        let frame_tag = find_payload_value(payload_text, "frame=").unwrap_or_default();
        let source_api = find_payload_value(payload_text, "source-api=").unwrap_or_default();
        let translation = find_payload_value(payload_text, "translation=").unwrap_or_default();

        let mut record = NativeDeviceRequestRecord {
            issuer: 1,
            kind: 1,
            state: 1,
            opcode: 0x494e_0001,
            buffer_id: 0,
            payload_len: payload.len() as u64,
            response_len: 0,
            submitted_tick: request_id.saturating_mul(3).saturating_sub(2),
            started_tick: request_id.saturating_mul(3).saturating_sub(1),
            completed_tick: 0,
            frame_tag: [0; 64],
            source_api_name: [0; 24],
            translation_label: [0; 32],
        };
        fill_fixed_field(&mut record.frame_tag, frame_tag);
        fill_fixed_field(&mut record.source_api_name, source_api);
        fill_fixed_field(&mut record.translation_label, translation);

        self.input_requests.borrow_mut().push(RecordedInputRequest {
            id: request_id,
            record,
            payload: payload.to_vec(),
        });
        request_id
    }

    fn complete_input_request(&self, request_id: u64, payload: &[u8], state: u32) -> bool {
        let mut requests = self.input_requests.borrow_mut();
        let Some(request) = requests.iter_mut().find(|request| request.id == request_id) else {
            return false;
        };
        request.record.state = state;
        request.record.response_len = if state == 4 { 0 } else { payload.len() as u64 };
        request.record.completed_tick = request_id.saturating_mul(3);
        if state != 4 {
            request.payload = payload.to_vec();
            *self.input_completion_payload.borrow_mut() = payload.to_vec();
        }
        true
    }

    fn input_request(&self, request_id: u64) -> Option<RecordedInputRequest> {
        self.input_requests
            .borrow()
            .iter()
            .find(|request| request.id == request_id)
            .cloned()
    }

    fn latest_input_request(&self) -> Option<RecordedInputRequest> {
        self.input_requests.borrow().last().cloned()
    }

    fn read_offset(&self, fd: usize) -> usize {
        let Some(record) = self.open_record(fd) else {
            return 0;
        };
        self.read_offsets
            .borrow()
            .iter()
            .find(|(description_id, _)| *description_id == record.description_id)
            .map(|(_, offset)| *offset)
            .unwrap_or(0)
    }

    fn service_block_request(&self, path: &str, bytes: &[u8]) -> bool {
        if path != "/dev/storage0" {
            return false;
        }
        let Some(request) = try_decode_block_request(bytes) else {
            return false;
        };
        let header = format!(
            "request:1 kind={} device={} opcode={}\n",
            if request.op == NATIVE_BLOCK_IO_OP_READ {
                "Read"
            } else {
                "Write"
            },
            "/dev/storage0",
            request.op
        );
        let mut driver_payload = header.into_bytes();
        driver_payload.extend_from_slice(encode_block_request_bytes(&request));
        let block_payload = if request.op == NATIVE_BLOCK_IO_OP_READ {
            let total_len = request.sector_count as usize * request.block_size as usize;
            let mut sector = vec![0u8; total_len];
            if total_len >= 3 {
                sector[0] = 0xeb;
                sector[1] = 0x58;
                sector[2] = 0x90;
            }
            if total_len >= 11 {
                sector[3..11].copy_from_slice(b"MSWIN4.1");
            }
            sector
        } else {
            Vec::new()
        };
        let mut files = self.file_contents.borrow_mut();
        files.retain(|(candidate, _)| candidate != "/drv/storage0" && candidate != "/dev/storage0");
        files.push((String::from("/drv/storage0"), driver_payload));
        files.push((String::from("/dev/storage0"), block_payload));
        true
    }

    fn service_block_completion(&self, path: &str, bytes: &[u8]) -> bool {
        if path != "/drv/storage0" {
            return false;
        }
        let mut files = self.file_contents.borrow_mut();
        files.retain(|(candidate, _)| candidate != "/dev/storage0");
        files.push((String::from("/dev/storage0"), bytes.to_vec()));
        true
    }

    fn prepare_storage_commit_local(
        &self,
        device_path: &str,
        tag: &str,
        payload: &[u8],
    ) -> Result<usize, Errno> {
        if device_path != "/dev/storage0" {
            return Err(Errno::Nxio);
        }
        if payload.len() > 512 {
            return Err(Errno::TooBig);
        }
        self.with_storage_state(|state| {
            let next_generation = state.generation.saturating_add(1);
            state.parent_generation = state.generation;
            state.generation = next_generation;
            state.dirty = true;
            state.payload_checksum = Self::storage_checksum(payload);
            state.payload_len = payload.len() as u64;
            state.prepared_commit_count = state.prepared_commit_count.saturating_add(1);
            fill_fixed_field(&mut state.state_label, "prepared");
            fill_fixed_field(&mut state.last_commit_tag, tag);
            state.payload_preview = Self::storage_payload_preview(payload);
            Self::push_storage_lineage_event(state, "prepare", "prepared", tag);
            Ok(next_generation as usize)
        })
    }

    fn recover_storage_volume_local(&self, device_path: &str) -> Result<usize, Errno> {
        if device_path != "/dev/storage0" {
            return Err(Errno::Nxio);
        }
        self.with_storage_state(|state| {
            let tag = fixed_text_field(&state.last_commit_tag).to_string();
            let clear_recovery = state.payload_len == 0 && tag == "clear";
            let next_generation = if clear_recovery {
                state.generation
            } else {
                state.generation.saturating_add(1)
            };
            if !clear_recovery {
                state.parent_generation = state.generation;
                state.generation = next_generation;
            }
            state.replay_generation = next_generation;
            state.dirty = false;
            if clear_recovery {
                state.payload_checksum = Self::storage_checksum(&[]);
                state.payload_len = 0;
                state.payload_preview = Self::storage_payload_preview(&[]);
            }
            state.recovered_commit_count = state.recovered_commit_count.saturating_add(1);
            fill_fixed_field(&mut state.state_label, "recovered");
            Self::push_storage_lineage_event(state, "recover", "recovered", &tag);
            Ok(next_generation as usize)
        })
    }

    fn repair_storage_snapshot_local(&self, device_path: &str) -> Result<usize, Errno> {
        if device_path != "/dev/storage0" {
            return Err(Errno::Nxio);
        }
        self.with_storage_state(|state| {
            let next_generation = state.generation.saturating_add(1);
            state.parent_generation = state.generation;
            state.generation = next_generation;
            state.replay_generation = next_generation;
            state.dirty = false;
            state.repaired_snapshot_count = state.repaired_snapshot_count.saturating_add(1);
            state.allocation_total_blocks = state.allocation_total_blocks.max(16);
            state.allocation_used_blocks = state.allocation_used_blocks.max(4);
            state.mapped_file_count = state.mapped_file_count.max(2);
            state.mapped_directory_count = state.mapped_directory_count.max(2);
            state.mapped_symlink_count = state.mapped_symlink_count.max(1);
            state.mapped_extent_count = state.mapped_extent_count.max(4);
            fill_fixed_field(&mut state.state_label, "repaired");
            fill_fixed_field(&mut state.last_commit_tag, "storage-repair");
            Self::push_storage_lineage_event(state, "repair", "repaired", "storage-repair");
            Ok(next_generation as usize)
        })
    }

    fn inspect_storage_volume_local(
        &self,
        device_path: &str,
    ) -> Result<NativeStorageVolumeRecord, Errno> {
        if device_path != "/dev/storage0" {
            return Err(Errno::NoEnt);
        }
        self.with_storage_state(|state| {
            let roots = self.mounted_storage_roots();
            let (files, dirs, symlinks, extents) = self.storage_tree_metrics_for_roots(&roots);
            if !roots.is_empty() {
                state.mapped_file_count = files;
                state.mapped_directory_count = dirs;
                state.mapped_symlink_count = symlinks;
                state.mapped_extent_count = extents;
                state.allocation_total_blocks = state
                    .allocation_total_blocks
                    .max(8 + files + dirs + symlinks + extents);
                state.allocation_used_blocks = state
                    .allocation_used_blocks
                    .max(2 + files + dirs + symlinks);
            }
            Ok(NativeStorageVolumeRecord {
                valid: u32::from(state.valid),
                dirty: u32::from(state.dirty),
                payload_len: state.payload_len,
                generation: state.generation,
                parent_generation: state.parent_generation,
                replay_generation: state.replay_generation,
                payload_checksum: state.payload_checksum,
                superblock_sector: 1,
                journal_sector: 2,
                data_sector: 3,
                index_sector: 4,
                alloc_sector: 5,
                data_start_sector: 6,
                prepared_commit_count: state.prepared_commit_count,
                recovered_commit_count: state.recovered_commit_count,
                repaired_snapshot_count: state.repaired_snapshot_count,
                allocation_total_blocks: state.allocation_total_blocks,
                allocation_used_blocks: state.allocation_used_blocks,
                mapped_file_count: state.mapped_file_count,
                mapped_extent_count: state.mapped_extent_count,
                mapped_directory_count: state.mapped_directory_count,
                mapped_symlink_count: state.mapped_symlink_count,
                volume_id: state.volume_id,
                state_label: state.state_label,
                last_commit_tag: state.last_commit_tag,
                payload_preview: state.payload_preview,
            })
        })
    }

    fn inspect_storage_lineage_local(
        &self,
        device_path: &str,
    ) -> Result<NativeStorageLineageRecord, Errno> {
        if device_path != "/dev/storage0" {
            return Err(Errno::NoEnt);
        }
        self.with_storage_state(|state| {
            let mut entries = [NativeStorageLineageEntry {
                generation: 0,
                parent_generation: 0,
                payload_checksum: 0,
                kind_label: [0; 16],
                state_label: [0; 16],
                tag_label: [0; 32],
            }; NATIVE_STORAGE_LINEAGE_DEPTH];
            let mut newest_generation = 0u64;
            let mut oldest_generation = 0u64;
            let mut contiguous = true;
            let count = state.lineage_count.min(state.lineage.len());
            let mut previous_parent_generation = None;
            for index in 0..count {
                let ring_index =
                    (state.lineage_head + state.lineage.len() - 1 - index) % state.lineage.len();
                let Some(entry) = state.lineage[ring_index] else {
                    continue;
                };
                if index == 0 {
                    newest_generation = entry.generation;
                }
                oldest_generation = entry.generation;
                if let Some(previous_parent) = previous_parent_generation {
                    contiguous &= previous_parent == entry.generation;
                }
                previous_parent_generation = Some(entry.parent_generation);
                entries[index] = NativeStorageLineageEntry {
                    generation: entry.generation,
                    parent_generation: entry.parent_generation,
                    payload_checksum: entry.payload_checksum,
                    kind_label: entry.kind_label,
                    state_label: entry.state_label,
                    tag_label: entry.tag_label,
                };
            }
            Ok(NativeStorageLineageRecord {
                valid: u32::from(state.valid),
                lineage_contiguous: u32::from(contiguous),
                count: count as u64,
                newest_generation,
                oldest_generation,
                entries,
            })
        })
    }

    fn rename_path(&self, from: &str, to: &str) -> Result<(), Errno> {
        let from = Self::normalize_absolute_path(from)?;
        let to = Self::normalize_absolute_path(to)?;
        if from == to {
            return Err(Errno::Inval);
        }
        if to.starts_with(&(from.to_string() + "/")) {
            return Err(Errno::Inval);
        }
        if !self.path_exists(&from) {
            return Err(Errno::NoEnt);
        }
        if !self.parent_directory_exists(&to) {
            return Err(Errno::NoEnt);
        }
        self.require_parent_mutation_access(&from)?;
        self.require_parent_mutation_access(&to)?;
        self.object_lock_conflict(&from, None)?;
        self.require_sticky_mutation_access(&from)?;
        let source_kind = self.created_kind(&from).unwrap_or(NativeObjectKind::File);
        if self.path_exists(&to) {
            self.object_lock_conflict(&to, None)?;
            self.require_sticky_mutation_access(&to)?;
            let target_kind = self.created_kind(&to).unwrap_or(NativeObjectKind::File);
            if source_kind == NativeObjectKind::Directory
                || target_kind == NativeObjectKind::Directory
            {
                if source_kind != target_kind {
                    return Err(Errno::Inval);
                }
                let prefix = format!("{to}/");
                let has_children = self
                    .created_paths
                    .borrow()
                    .iter()
                    .any(|(candidate, _)| candidate.starts_with(&prefix))
                    || self
                        .file_contents
                        .borrow()
                        .iter()
                        .any(|(candidate, _)| candidate.starts_with(&prefix))
                    || self
                        .symlink_targets
                        .borrow()
                        .iter()
                        .any(|(candidate, _)| candidate.starts_with(&prefix));
                if has_children {
                    return Err(Errno::Busy);
                }
            }
            self.detach_path_entry(&to);
        }
        let from_mount = self.mount_owner_for_path(&from).map(|record| record.id);
        let to_mount = self.mount_owner_for_path(&to).map(|record| record.id);
        if from_mount != to_mount {
            return Err(Errno::Busy);
        }

        let mut created = self.created_paths.borrow_mut();
        for entry in created.iter_mut() {
            entry.0 = Self::rewrite_path_prefix(&entry.0, &from, &to);
        }
        created.sort_by(|a, b| a.0.cmp(&b.0));

        let mut symlinks = self.symlink_targets.borrow_mut();
        for entry in symlinks.iter_mut() {
            entry.0 = Self::rewrite_path_prefix(&entry.0, &from, &to);
        }
        symlinks.sort_by(|a, b| a.0.cmp(&b.0));

        let mut files = self.file_contents.borrow_mut();
        for entry in files.iter_mut() {
            entry.0 = Self::rewrite_path_prefix(&entry.0, &from, &to);
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let mut metadata = self.path_metadata.borrow_mut();
        for entry in metadata.iter_mut() {
            entry.0 = Self::rewrite_path_prefix(&entry.0, &from, &to);
        }
        metadata.sort_by(|a, b| a.0.cmp(&b.0));
        let mut inodes = self.path_inodes.borrow_mut();
        for entry in inodes.iter_mut() {
            entry.0 = Self::rewrite_path_prefix(&entry.0, &from, &to);
        }
        inodes.sort_by(|a, b| a.0.cmp(&b.0));
        let mut open_files = self.open_files.borrow_mut();
        for record in open_files.iter_mut() {
            if !record.deleted {
                record.path = Self::rewrite_path_prefix(&record.path, &from, &to);
            }
        }
        Ok(())
    }

    fn detach_path_entry(&self, path: &str) {
        let Ok(path) = Self::normalize_absolute_path(path) else {
            return;
        };
        if !self.path_exists(&path) {
            return;
        }
        let inode = self.inode_for_path(&path);
        let last_link = self.link_count_for_inode(inode) == 1;
        let orphan_bytes = self.file_content(&path);
        let orphan_kind = self.created_kind(&path).unwrap_or(NativeObjectKind::File);
        for record in self.open_files.borrow_mut().iter_mut() {
            if record.path == path {
                record.deleted = true;
                record.kind = orphan_kind;
                record.orphan_bytes = orphan_bytes.clone();
            }
        }
        if last_link && orphan_kind == NativeObjectKind::File {
            let mut mappings = self.vm_mappings.borrow_mut();
            for mapping in mappings
                .iter_mut()
                .filter(|mapping| mapping.file_inode == Some(inode))
            {
                mapping.orphan_bytes = orphan_bytes.clone();
            }
        }
        self.created_paths
            .borrow_mut()
            .retain(|(candidate, _)| candidate != &path);
        self.symlink_targets
            .borrow_mut()
            .retain(|(candidate, _)| candidate != &path);
        self.file_contents
            .borrow_mut()
            .retain(|(candidate, _)| candidate != &path);
        let removed_device_paths = {
            let mut sockets = self.network_sockets.borrow_mut();
            let removed = sockets
                .iter()
                .filter(|entry| entry.path == path)
                .map(|entry| entry.device_path.clone())
                .collect::<Vec<_>>();
            sockets.retain(|entry| entry.path != path);
            removed
        };
        for device_path in removed_device_paths {
            self.recount_attached_sockets(&device_path);
        }
        self.remove_path_inode(&path);
        self.remove_path_metadata(&path);
    }

    fn unlink_path(&self, path: &str) -> Result<(), Errno> {
        let path = Self::normalize_absolute_path(path)?;
        if !self.path_exists(&path) {
            return Err(Errno::NoEnt);
        }
        self.require_parent_mutation_access(&path)?;
        self.object_lock_conflict(&path, None)?;
        self.require_sticky_mutation_access(&path)?;
        if self.created_kind(&path) == Some(NativeObjectKind::Directory) {
            let prefix = format!("{path}/");
            let has_children = self
                .created_paths
                .borrow()
                .iter()
                .any(|(candidate, _)| candidate.starts_with(&prefix))
                || self
                    .file_contents
                    .borrow()
                    .iter()
                    .any(|(candidate, _)| candidate.starts_with(&prefix))
                || self
                    .symlink_targets
                    .borrow()
                    .iter()
                    .any(|(candidate, _)| candidate.starts_with(&prefix));
            if has_children {
                return Err(Errno::Busy);
            }
        }
        self.detach_path_entry(&path);
        Ok(())
    }

    fn link_path(&self, source: &str, destination: &str) -> Result<(), Errno> {
        let source = Self::normalize_absolute_path(source)?;
        let destination = Self::normalize_absolute_path(destination)?;
        if source == destination || source == "/" || destination == "/" {
            return Err(Errno::Inval);
        }
        if !self.path_exists(&source) {
            return Err(Errno::NoEnt);
        }
        if !self.parent_directory_exists(&destination) {
            return Err(Errno::NoEnt);
        }
        self.require_access(&source, true, false, false)?;
        self.require_parent_mutation_access(&destination)?;
        self.object_lock_conflict(&source, None)?;
        if self.path_exists(&destination) {
            return Err(Errno::Exist);
        }
        let source_mount = self.mount_owner_for_path(&source).map(|record| record.id);
        let destination_mount = self
            .mount_owner_for_path(&destination)
            .map(|record| record.id);
        if source_mount != destination_mount {
            return Err(Errno::Busy);
        }
        if self.created_kind(&source) != Some(NativeObjectKind::File) {
            return Err(Errno::Inval);
        }
        let inode = self.inode_for_path(&source);
        let (_, owner_uid, group_gid, mode) = self
            .path_metadata
            .borrow()
            .iter()
            .find(|(candidate, _, _, _)| candidate == &source)
            .cloned()
            .unwrap_or((source.clone(), 1000, 1000, 0o644));
        self.created_paths
            .borrow_mut()
            .push((destination.clone(), NativeObjectKind::File));
        self.set_path_inode(&destination, inode);
        self.set_path_metadata(&destination, owner_uid, group_gid, mode);
        let content = self.file_content(&source);
        self.file_contents.borrow_mut().push((destination, content));
        Ok(())
    }

    fn mount_storage_volume_local(
        &self,
        device_path: &str,
        mount_path: &str,
    ) -> Result<usize, Errno> {
        let mount_path = Self::normalize_absolute_path(mount_path)?;
        if self.mount_by_path(&mount_path).is_some() {
            return Err(Errno::Exist);
        }
        let parent_mount = self.mount_parent(&mount_path);
        let parent_mount_id = parent_mount.as_ref().map(|record| record.id).unwrap_or(0);
        let default_mode = match parent_mount
            .as_ref()
            .and_then(|record| NativeMountPropagationMode::from_raw(record.propagation_mode))
        {
            Some(NativeMountPropagationMode::Shared) => NativeMountPropagationMode::Shared,
            Some(NativeMountPropagationMode::Slave) => NativeMountPropagationMode::Slave,
            _ => NativeMountPropagationMode::Private,
        };
        let mut planned = vec![(self.next_mount_id(), mount_path.clone(), default_mode)];
        if matches!(default_mode, NativeMountPropagationMode::Shared) {
            if let Some(parent) = &parent_mount {
                for (clone_path, clone_mode) in self.mount_propagation_clones(parent, &mount_path) {
                    planned.push((self.next_mount_id(), clone_path, clone_mode));
                }
            }
        }
        if planned
            .iter()
            .any(|(_, path, _)| self.mount_by_path(path).is_some())
        {
            return Err(Errno::Exist);
        }
        let primary_peer_group = if matches!(default_mode, NativeMountPropagationMode::Shared) {
            planned[0].0
        } else {
            0
        };
        let primary_master_group = if matches!(default_mode, NativeMountPropagationMode::Slave) {
            parent_mount
                .as_ref()
                .map(|record| record.master_group.max(record.peer_group))
                .unwrap_or(0)
        } else {
            0
        };
        for (_, path, _) in &planned {
            if !self.path_exists(path) {
                self.record_created_path(path, NativeObjectKind::Directory);
            }
        }
        let planned_parent_ids = planned
            .iter()
            .map(|(_, path, _)| {
                self.mount_parent(path)
                    .map(|record| record.id)
                    .unwrap_or(parent_mount_id)
            })
            .collect::<Vec<_>>();
        let mut mounts = self.mounts.borrow_mut();
        for ((id, path, mode), parent_id) in planned.iter().zip(planned_parent_ids.iter()) {
            let (peer_group, master_group) = match mode {
                NativeMountPropagationMode::Shared => (primary_peer_group, 0),
                NativeMountPropagationMode::Slave => {
                    (0, primary_peer_group.max(primary_master_group))
                }
                NativeMountPropagationMode::Private => (0, 0),
            };
            mounts.push(LocalMountRecord {
                id: *id,
                device_path: device_path.to_string(),
                mount_path: path.clone(),
                parent_mount_id: *parent_id,
                peer_group,
                master_group,
                propagation_mode: *mode as u32,
                created_mount_root: true,
            });
        }
        if device_path == "/dev/storage0" {
            let persisted_entries = self
                .storage_state
                .borrow()
                .as_ref()
                .map(|state| state.persisted_entries.clone())
                .unwrap_or_default();
            if !persisted_entries.is_empty() {
                self.restore_storage_entries(&mount_path, &persisted_entries);
            }
        }
        Ok(0)
    }

    fn unmount_storage_volume_local(&self, mount_path: &str) -> Result<usize, Errno> {
        let mount_path = Self::normalize_absolute_path(mount_path)?;
        let Some(record) = self.mount_by_path(&mount_path) else {
            return Err(Errno::NoEnt);
        };
        let active_ids = self.mount_unmount_ids(&record);
        for target in self
            .mounts
            .borrow()
            .iter()
            .filter(|candidate| active_ids.iter().any(|id| *id == candidate.id))
        {
            if self.mount_has_nested_child_outside(&target.mount_path, &active_ids) {
                return Err(Errno::Busy);
            }
        }
        let targets = self
            .mounts
            .borrow()
            .iter()
            .filter(|candidate| active_ids.iter().any(|id| *id == candidate.id))
            .cloned()
            .collect::<Vec<_>>();
        let prefix = format!("{mount_path}/");
        let files = self
            .created_paths
            .borrow()
            .iter()
            .filter(|(candidate, kind)| {
                (candidate == &mount_path || candidate.starts_with(&prefix))
                    && *kind == NativeObjectKind::File
            })
            .count() as u64;
        let dirs = self
            .created_paths
            .borrow()
            .iter()
            .filter(|(candidate, kind)| {
                candidate.starts_with(&prefix) && *kind == NativeObjectKind::Directory
            })
            .count() as u64;
        let symlinks = self
            .created_paths
            .borrow()
            .iter()
            .filter(|(candidate, kind)| {
                (candidate == &mount_path || candidate.starts_with(&prefix))
                    && *kind == NativeObjectKind::Symlink
            })
            .count() as u64;
        let mut payload = Vec::new();
        let persisted_entries = self.snapshot_storage_entries(&mount_path);
        // Allow the real persisted baseline used by storage smoke while still
        // refusing oversized snapshots that exceed the modeled mount budget.
        if persisted_entries.len() > 16 {
            return Err(Errno::TooBig);
        }
        payload.extend_from_slice(
            format!(
                "mount={} files={} dirs={} symlinks={} entries={}",
                mount_path,
                files,
                dirs,
                symlinks,
                persisted_entries.len()
            )
            .as_bytes(),
        );
        for target in &targets {
            let prefix = format!("{}/", target.mount_path);
            self.created_paths.borrow_mut().retain(|(candidate, _)| {
                candidate != &target.mount_path && !candidate.starts_with(&prefix)
            });
            self.file_contents.borrow_mut().retain(|(candidate, _)| {
                candidate != &target.mount_path && !candidate.starts_with(&prefix)
            });
            self.symlink_targets.borrow_mut().retain(|(candidate, _)| {
                candidate != &target.mount_path && !candidate.starts_with(&prefix)
            });
            self.path_inodes.borrow_mut().retain(|(candidate, _)| {
                candidate != &target.mount_path && !candidate.starts_with(&prefix)
            });
            self.path_metadata
                .borrow_mut()
                .retain(|(candidate, _, _, _)| {
                    candidate != &target.mount_path && !candidate.starts_with(&prefix)
                });
            if target.created_mount_root {
                self.created_paths
                    .borrow_mut()
                    .retain(|(candidate, _)| candidate != &target.mount_path);
            }
        }
        self.mounts
            .borrow_mut()
            .retain(|candidate| !active_ids.iter().any(|id| *id == candidate.id));
        self.with_storage_state(|state| {
            let next_generation = state.generation.saturating_add(1);
            state.parent_generation = state.generation;
            state.generation = next_generation;
            state.replay_generation = next_generation;
            state.dirty = false;
            state.payload_checksum = Self::storage_checksum(&payload);
            state.payload_len = payload.len() as u64;
            state.payload_preview = Self::storage_payload_preview(&payload);
            state.persisted_entries = persisted_entries;
            state.mapped_file_count = files;
            state.mapped_directory_count = dirs;
            state.mapped_symlink_count = symlinks;
            state.mapped_extent_count = files.saturating_mul(2).max(1);
            state.allocation_total_blocks = state
                .allocation_total_blocks
                .max(8 + files + dirs + symlinks);
            state.allocation_used_blocks =
                (2 + files + dirs + symlinks).min(state.allocation_total_blocks);
            fill_fixed_field(&mut state.state_label, "recovered");
            fill_fixed_field(&mut state.last_commit_tag, "boot-vfs-unmount");
            Self::push_storage_lineage_event(state, "snapshot", "recovered", "boot-vfs-unmount");
            Ok(next_generation as usize)
        })
    }

    fn set_read_offset(&self, fd: usize, offset: usize) {
        let Some(record) = self.open_record(fd) else {
            return;
        };
        let mut offsets = self.read_offsets.borrow_mut();
        if let Some((_, current)) = offsets
            .iter_mut()
            .find(|(description_id, _)| *description_id == record.description_id)
        {
            *current = offset;
        } else {
            offsets.push((record.description_id, offset));
        }
    }

    fn record_spawned_process(&self, pid: u64, name: &str, path: &str) {
        let mut bootstraps = self.process_bootstraps.borrow_mut();
        if let Some(existing) = bootstraps.iter_mut().find(|entry| entry.pid == pid) {
            existing.name = name.to_string();
            existing.image_path = path.to_string();
            if existing.argv.is_empty() {
                existing.argv.push(name.to_string());
            }
            return;
        }
        bootstraps.push(RecordedProcessBootstrap {
            pid,
            name: name.to_string(),
            image_path: path.to_string(),
            cwd: String::from("/"),
            argv: vec![name.to_string()],
            envp: Vec::new(),
        });
    }

    fn with_recorded_process_mut<F>(&self, pid: u64, update: F) -> Result<(), Errno>
    where
        F: FnOnce(&mut RecordedProcessBootstrap),
    {
        let mut bootstraps = self.process_bootstraps.borrow_mut();
        let Some(process) = bootstraps.iter_mut().find(|entry| entry.pid == pid) else {
            return Err(Errno::Srch);
        };
        update(process);
        Ok(())
    }

    fn recorded_process(&self, pid: u64) -> Option<RecordedProcessBootstrap> {
        self.process_bootstraps
            .borrow()
            .iter()
            .find(|entry| entry.pid == pid)
            .cloned()
    }

    fn decode_string_table(
        &self,
        ptr: usize,
        len: usize,
        count: usize,
    ) -> Result<Vec<String>, Errno> {
        if count == 0 {
            return if len == 0 {
                Ok(Vec::new())
            } else {
                Err(Errno::Inval)
            };
        }
        let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
        if bytes.last().copied() != Some(0) {
            return Err(Errno::Inval);
        }
        let values = bytes
            .split(|byte| *byte == 0)
            .take(count)
            .map(|segment| String::from_utf8(segment.to_vec()).map_err(|_| Errno::Inval))
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() != count {
            return Err(Errno::Inval);
        }
        let expected = values
            .iter()
            .fold(0usize, |acc, value| acc + value.len() + 1);
        if expected != len {
            return Err(Errno::Inval);
        }
        Ok(values)
    }

    fn parse_proc_pid_section(path: &str) -> Option<(u64, &str)> {
        let rest = path.strip_prefix("/proc/")?;
        let (pid, section) = rest.split_once('/')?;
        Some((pid.parse::<u64>().ok()?, section))
    }

    fn recorded_procfs_payload(&self, path: &str) -> Option<Vec<u8>> {
        let (pid, section) = Self::parse_proc_pid_section(path)?;
        let process = self.recorded_process(pid)?;
        match section {
            "cmdline" => Some(process.argv.join("\n").into_bytes()),
            "environ" => Some(process.envp.join("\n").into_bytes()),
            "root" => Some(b"/".to_vec()),
            "cwd" => Some(process.cwd.into_bytes()),
            "exe" => Some(process.image_path.into_bytes()),
            "auxv" => Some(self.procfs_auxv_listing(pid).into_bytes()),
            "mounts" => Some(self.procfs_mount_listing(pid).into_bytes()),
            "fd" => Some(self.procfs_fd_listing().into_bytes()),
            "vfslocks" => Some(Vec::new()),
            "vfswatches" => Some(Vec::new()),
            "vfsstats" => Some(b"live: nodes=1 orphans=0 locks=0 watches=0 mounts=1\n".to_vec()),
            "caps" => Some(self.procfs_cap_listing(pid).into_bytes()),
            "queues" => Some(self.procfs_queue_listing(pid).into_bytes()),
            section if section.starts_with("fdinfo/") => {
                let fd = section.strip_prefix("fdinfo/")?;
                let fd = fd.parse::<u64>().ok()?;
                self.procfs_fdinfo_payload(fd)
            }
            "status" => Some(
                format!(
                    "Name:\t{}\nState:\tExited\nPid:\t{}\nCwd:\t{}\n",
                    process.name, process.pid, process.cwd
                )
                .into_bytes(),
            ),
            _ => None,
        }
    }

    fn push_stdout_line(&self, text: &str) {
        let mut stdout = self.stdout.borrow_mut();
        stdout.extend_from_slice(text.as_bytes());
        stdout.push(b'\n');
    }

    fn emit_compat_proc_probe_output(&self, pid: u64) {
        let Some(process) = self.recorded_process(pid) else {
            return;
        };
        let expected_exe =
            find_env_value(&process.envp, "NGOS_COMPAT_EXPECT_EXE").unwrap_or(&process.image_path);
        let expected_cwd =
            find_env_value(&process.envp, "NGOS_COMPAT_EXPECT_CWD").unwrap_or(&process.cwd);
        let proc_fd_text = self.procfs_fd_listing();
        let proc_fd_lines = proc_fd_text
            .lines()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        let proc_fd_count = proc_fd_lines.len();
        self.push_stdout_line(&format!(
            "compat.abi.smoke.proc.step pid={pid} path=/proc/{pid}/fd"
        ));
        self.push_stdout_line(&format!(
            "compat.abi.smoke.proc.step pid={pid} path=/proc/{pid}/cwd"
        ));
        self.push_stdout_line(&format!(
            "compat.abi.smoke.proc.step pid={pid} path=/proc/{pid}/exe"
        ));
        self.push_stdout_line(&format!(
            "compat.abi.smoke.proc.step pid={pid} path=/proc/{pid}/cmdline"
        ));
        if let Some(target) = find_env_value(&process.envp, "NGOS_COMPAT_TARGET") {
            self.push_stdout_line(&format!(
                "compat.abi.smoke.proc.step pid={pid} path=/proc/{pid}/environ"
            ));
            self.push_stdout_line(&format!(
                "compat.abi.smoke.proc.environ pid={pid} outcome=ok marker=NGOS_COMPAT_TARGET={target}"
            ));
        }
        self.push_stdout_line(&format!(
            "compat.abi.smoke.proc.success pid={} fd-count={} fd0=present fd1=present fd2=present cwd={} exe={} cmdline=present",
            pid, proc_fd_count, expected_cwd, expected_exe
        ));
        self.push_stdout_line(&format!(
            "compat.abi.smoke.proc.refusal pid={} path=/proc/{pid}/fd/9999 outcome=expected",
            pid
        ));
        self.push_stdout_line(&format!(
            "compat.abi.smoke.proc.recovery pid={} fd-list=ok outcome=ok",
            pid
        ));
    }

    fn render_bus_procfs(&self) -> Vec<u8> {
        let peers = self.bus_peers.borrow();
        let endpoints = self.bus_endpoints.borrow();
        let attached_endpoint_total = peers
            .iter()
            .map(|peer| peer.record.attached_endpoint_count)
            .sum::<u64>();
        let mut lines = vec![format!(
            "bus-peers:\t{}\nbus-endpoints:\t{}\nattached-endpoints:\t{}",
            peers.len(),
            endpoints.len(),
            attached_endpoint_total
        )];
        for peer in peers.iter() {
            lines.push(format!(
                "peer\tid={}\towner={}\tdomain={}\tname={}\tattachments={}\tpublishes={}\treceives={}\tlast-endpoint={}",
                peer.record.id,
                peer.record.owner,
                peer.record.domain,
                peer.name,
                peer.record.attached_endpoint_count,
                peer.record.publish_count,
                peer.record.receive_count,
                peer.record.last_endpoint,
            ));
        }
        for endpoint in endpoints.iter() {
            let attached = peers
                .iter()
                .filter(|peer| peer.record.last_endpoint == endpoint.record.id)
                .map(|peer| peer.record.id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            lines.push(format!(
                "endpoint\tid={}\tdomain={}\tresource={}\tkind=channel\tpath={}\tcontract-policy=none\tissuer-policy=none\tdelegated-caps=0\tqueue-depth={}\tqueue-capacity={}\tqueue-peak={}\toverflows={}\tbytes={}\tpublishes={}\treceives={}\tlast-peer={}\tpeers=[{}]",
                endpoint.record.id,
                endpoint.record.domain,
                endpoint.record.resource,
                endpoint.path,
                endpoint.record.queue_depth,
                endpoint.record.queue_capacity,
                endpoint.record.peak_queue_depth,
                endpoint.record.overflow_count,
                endpoint.record.byte_count,
                endpoint.record.publish_count,
                endpoint.record.receive_count,
                endpoint.record.last_peer,
                attached,
            ));
        }
        (lines.join("\n") + "\n").into_bytes()
    }

    fn procfs_queue_listing(&self, pid: u64) -> String {
        if pid != 1 {
            return String::new();
        }
        let mut lines = Vec::new();
        for (fd, mode) in self.event_queue_modes.borrow().iter() {
            lines.push(format!(
                "event\t{}\t{:?}\twatches=0\ttimers=0\tprocwatches=0\tsigwatches=0\tmemwatches=0\tresourcewatches=0\tpending=0\twaiters=0\tdescriptors=1\tdeferred=0",
                fd, mode
            ));
        }
        if lines.is_empty() {
            String::new()
        } else {
            lines.join("\n") + "\n"
        }
    }

    fn alloc_vm_addr(&self, len: u64) -> u64 {
        let start = self.next_vm_addr.get();
        let advance = len.max(0x1000).next_multiple_of(0x1000);
        self.next_vm_addr.set(start + advance + 0x1000);
        start
    }

    fn push_vm_decision(&self, pid: u64, line: String) {
        self.vm_decisions.borrow_mut().push((pid, line));
    }

    fn push_vm_episode(&self, pid: u64, line: String) {
        self.vm_episodes.borrow_mut().push((pid, line));
    }

    fn memory_policy_blocked(&self) -> bool {
        let frames = self.frames.borrow();
        let states = frames
            .iter()
            .filter(|entry| entry.number == SYS_SET_CONTRACT_STATE)
            .map(|entry| (entry.arg0 as u64, entry.arg1 as u32))
            .collect::<Vec<_>>();
        let mut next_contract = 43u64;
        for entry in frames.iter() {
            if entry.number != SYS_CREATE_CONTRACT {
                continue;
            }
            let contract = next_contract;
            next_contract += 1;
            if entry.arg2 as u32 == NativeContractKind::Memory as u32
                && replayed_contract_state(&states, contract)
                    == NativeContractState::Suspended as u32
            {
                return true;
            }
        }
        false
    }

    fn emit_vm_policy_block(&self, pid: u64) {
        self.push_vm_decision(pid, format!("agent=policy-block pid={pid}"));
        self.push_vm_episode(pid, format!("kind=policy pid={pid} blocked=yes last=policy-block"));
    }

    fn ensure_heap_mapping(&self, pid: u64) {
        let mut mappings = self.vm_mappings.borrow_mut();
        if mappings
            .iter()
            .any(|mapping| mapping.pid == pid && mapping.present && mapping.label == "[heap]")
        {
            return;
        }
        mappings.push(VmMappingRecord {
            pid,
            start: 0x4000_0000,
            len: 0x4000,
            readable: true,
            writable: true,
            executable: false,
            label: String::from("[heap]"),
            file_path: None,
            file_inode: None,
            file_offset: 0,
            private: true,
            cow: false,
            present: true,
            reclaimed: false,
            quarantined: false,
            quarantine_reason: 0,
            orphan_bytes: Vec::new(),
            words: Vec::new(),
        });
    }

    fn mapping_resident_pages(mapping: &VmMappingRecord) -> u64 {
        let mut pages = Vec::new();
        for (addr, _) in &mapping.words {
            let page = addr.saturating_sub(mapping.start) / 0x1000;
            if !pages.contains(&page) {
                pages.push(page);
            }
        }
        pages.len() as u64
    }

    fn read_word_from_bytes(bytes: &[u8], offset: usize) -> u32 {
        let mut word = [0u8; 4];
        for (index, slot) in word.iter_mut().enumerate() {
            if let Some(byte) = bytes.get(offset.saturating_add(index)) {
                *slot = *byte;
            }
        }
        u32::from_le_bytes(word)
    }

    fn write_word_to_bytes(bytes: &mut Vec<u8>, offset: usize, value: u32) {
        let end = offset.saturating_add(4);
        if bytes.len() < end {
            bytes.resize(end, 0);
        }
        bytes[offset..end].copy_from_slice(&value.to_le_bytes());
    }

    fn current_mapping_backing_bytes(&self, mapping: &VmMappingRecord) -> Vec<u8> {
        if let Some(inode) = mapping.file_inode {
            if let Some(path) = self.all_paths_for_inode(inode).into_iter().next() {
                return self.file_content(&path);
            }
        }
        mapping.orphan_bytes.clone()
    }

    fn recompute_mapping_words_from_backing(&self, inode: u64, backing: &[u8]) {
        let mut mappings = self.vm_mappings.borrow_mut();
        for mapping in mappings
            .iter_mut()
            .filter(|mapping| mapping.file_inode == Some(inode))
        {
            for (addr, value) in mapping.words.iter_mut() {
                let relative = addr.saturating_sub(mapping.start) as usize;
                let backing_offset = mapping.file_offset as usize + relative;
                *value = Self::read_word_from_bytes(backing, backing_offset);
            }
            mapping.orphan_bytes = backing.to_vec();
        }
    }

    fn flush_file_mapping_range(&self, pid: u64, start: u64, len: u64) -> Result<(), Errno> {
        let end = start.saturating_add(len);
        let snapshots = self
            .vm_mappings
            .borrow()
            .iter()
            .filter(|mapping| {
                mapping.pid == pid
                    && mapping.present
                    && mapping.file_path.is_some()
                    && start < mapping.start.saturating_add(mapping.len)
                    && end > mapping.start
            })
            .cloned()
            .collect::<Vec<_>>();
        for mapping in snapshots {
            let mut backing = self.current_mapping_backing_bytes(&mapping);
            for (addr, value) in mapping.words.iter() {
                if *addr < start || *addr >= end {
                    continue;
                }
                let relative = addr.saturating_sub(mapping.start) as usize;
                let backing_offset = mapping.file_offset as usize + relative;
                Self::write_word_to_bytes(&mut backing, backing_offset, *value);
            }
            if let Some(inode) = mapping.file_inode {
                let linked_paths = self.all_paths_for_inode(inode);
                if linked_paths.is_empty() {
                    let mut mappings = self.vm_mappings.borrow_mut();
                    if let Some(target) = mappings.iter_mut().find(|candidate| {
                        candidate.pid == mapping.pid && candidate.start == mapping.start
                    }) {
                        target.orphan_bytes = backing.clone();
                    }
                } else {
                    let mut files = self.file_contents.borrow_mut();
                    for linked_path in linked_paths {
                        if let Some((_, content)) = files
                            .iter_mut()
                            .find(|(candidate, _)| candidate == &linked_path)
                        {
                            *content = backing.clone();
                        } else {
                            files.push((linked_path, backing.clone()));
                        }
                    }
                    drop(files);
                    self.recompute_mapping_words_from_backing(inode, &backing);
                }
            }
        }
        Ok(())
    }

    fn load_vm_word(&self, pid: u64, addr: u64) -> Result<(u32, bool), Errno> {
        if self.memory_policy_blocked() {
            self.emit_vm_policy_block(pid);
            return Err(Errno::Access);
        }
        let mut mappings = self.vm_mappings.borrow_mut();
        let Some(mapping) = mappings.iter_mut().find(|mapping| {
            mapping.pid == pid
                && mapping.present
                && addr >= mapping.start
                && addr < mapping.start.saturating_add(mapping.len)
        }) else {
            return Err(Errno::Fault);
        };
        if !mapping.readable {
            return Err(Errno::Fault);
        }
        if mapping.quarantined {
            return Err(Errno::Access);
        }
        let restored = mapping.reclaimed;
        mapping.reclaimed = false;
        if mapping.file_path.is_some()
            && !mapping
                .words
                .iter()
                .any(|(word_addr, _)| *word_addr == addr)
        {
            let relative = addr.saturating_sub(mapping.start) as usize;
            let backing_offset = mapping.file_offset as usize + relative;
            let value = Self::read_word_from_bytes(
                &self.current_mapping_backing_bytes(mapping),
                backing_offset,
            );
            mapping.words.push((addr, value));
        }
        Ok((
            mapping
                .words
                .iter()
                .find(|(word_addr, _)| *word_addr == addr)
                .map(|(_, value)| *value)
                .unwrap_or(0),
            restored,
        ))
    }

    fn store_vm_word(&self, pid: u64, addr: u64, value: u32) -> Result<bool, Errno> {
        if self.memory_policy_blocked() {
            self.emit_vm_policy_block(pid);
            return Err(Errno::Access);
        }
        let mut mappings = self.vm_mappings.borrow_mut();
        let Some(mapping) = mappings.iter_mut().find(|mapping| {
            mapping.pid == pid
                && mapping.present
                && addr >= mapping.start
                && addr < mapping.start.saturating_add(mapping.len)
        }) else {
            return Err(Errno::Fault);
        };
        if !mapping.writable {
            return Err(Errno::Fault);
        }
        if mapping.quarantined {
            return Err(Errno::Access);
        }
        let was_cow = mapping.cow;
        mapping.reclaimed = false;
        if let Some((_, current)) = mapping
            .words
            .iter_mut()
            .find(|(word_addr, _)| *word_addr == addr)
        {
            *current = value;
        } else {
            mapping.words.push((addr, value));
        }
        Ok(was_cow)
    }

    fn protect_vm_range(
        &self,
        pid: u64,
        start: u64,
        len: u64,
        readable: bool,
        writable: bool,
        executable: bool,
    ) -> Result<(), Errno> {
        if self.memory_policy_blocked() {
            self.emit_vm_policy_block(pid);
            return Err(Errno::Access);
        }
        let mut mappings = self.vm_mappings.borrow_mut();
        let Some(index) = mappings.iter().position(|mapping| {
            mapping.pid == pid
                && mapping.present
                && start >= mapping.start
                && start.saturating_add(len) <= mapping.start.saturating_add(mapping.len)
        }) else {
            return Err(Errno::Fault);
        };
        let mapping = mappings.remove(index);
        let end = start + len;
        let mapping_end = mapping.start + mapping.len;
        let mut replacements = Vec::new();
        if start > mapping.start {
            let mut before = mapping.clone();
            before.len = start - mapping.start;
            before.words.retain(|(addr, _)| *addr < start);
            replacements.push(before);
        }
        let mut middle = mapping.clone();
        middle.start = start;
        middle.len = len;
        middle.readable = readable;
        middle.writable = writable;
        middle.executable = executable;
        middle
            .words
            .retain(|(addr, _)| *addr >= start && *addr < end);
        replacements.push(middle);
        if end < mapping_end {
            let mut after = mapping;
            after.start = end;
            after.len = mapping_end - end;
            after.words.retain(|(addr, _)| *addr >= end);
            replacements.push(after);
        }
        mappings.splice(index..index, replacements);
        Ok(())
    }

    fn unmap_vm_range(&self, pid: u64, start: u64, len: u64) -> Result<(), Errno> {
        if self.memory_policy_blocked() {
            self.emit_vm_policy_block(pid);
            return Err(Errno::Access);
        }
        let mut mappings = self.vm_mappings.borrow_mut();
        let Some(index) = mappings.iter().position(|mapping| {
            mapping.pid == pid
                && mapping.present
                && start >= mapping.start
                && start.saturating_add(len) <= mapping.start.saturating_add(mapping.len)
        }) else {
            return Err(Errno::Fault);
        };
        let mapping = mappings.remove(index);
        let end = start + len;
        let mapping_end = mapping.start + mapping.len;
        let mut replacements = Vec::new();
        if start > mapping.start {
            let mut before = mapping.clone();
            before.len = start - mapping.start;
            before.words.retain(|(addr, _)| *addr < start);
            replacements.push(before);
        }
        if end < mapping_end {
            let mut after = mapping;
            after.start = end;
            after.len = mapping_end - end;
            after.words.retain(|(addr, _)| *addr >= end);
            replacements.push(after);
        }
        mappings.splice(index..index, replacements);
        Ok(())
    }

    fn advise_vm_range(&self, pid: u64, start: u64, len: u64, advice: u32) -> Result<(), Errno> {
        if self.memory_policy_blocked() {
            self.emit_vm_policy_block(pid);
            return Err(Errno::Access);
        }
        let end = start.saturating_add(len);
        let mut mappings = self.vm_mappings.borrow_mut();
        let mut coverage = 0u64;
        let mut touched = false;
        for mapping in mappings.iter_mut().filter(|mapping| {
            mapping.pid == pid
                && mapping.present
                && start < mapping.start.saturating_add(mapping.len)
                && end > mapping.start
        }) {
            let overlap_start = start.max(mapping.start);
            let overlap_end = end.min(mapping.start.saturating_add(mapping.len));
            if overlap_end <= overlap_start {
                continue;
            }
            coverage = coverage.saturating_add(overlap_end.saturating_sub(overlap_start));
            touched = true;
            if mapping.file_path.is_some() {
                mapping.words.clear();
                mapping.reclaimed = true;
            }
        }
        if !touched || coverage < len {
            return Err(Errno::Fault);
        }
        drop(mappings);
        self.push_vm_decision(
            pid,
            format!("agent=advice pid={pid} start={start} len={len} advice={advice}"),
        );
        self.push_vm_episode(
            pid,
            format!("kind=fault pid={pid} advised=yes last=advice advice={advice}"),
        );
        Ok(())
    }

    fn reclaim_vm_pressure(&self, scope: Option<u64>, target_pages: u64) -> u64 {
        let actor_pid = scope.unwrap_or(1);
        if self.memory_policy_blocked() {
            self.emit_vm_policy_block(actor_pid);
            return 0;
        }
        let mut mappings = self.vm_mappings.borrow_mut();
        let mut candidates = mappings
            .iter_mut()
            .filter(|mapping| {
                mapping.present
                    && mapping.file_path.is_some()
                    && scope.is_none_or(|pid| mapping.pid == pid)
                    && Self::mapping_resident_pages(mapping) > 0
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            Self::mapping_resident_pages(right)
                .cmp(&Self::mapping_resident_pages(left))
                .then_with(|| left.start.cmp(&right.start))
        });
        self.push_vm_decision(
            actor_pid,
            format!("agent=pressure-trigger pid={actor_pid} target-pages={target_pages}"),
        );
        let mut reclaimed = 0u64;
        for mapping in candidates {
            if reclaimed >= target_pages {
                break;
            }
            let resident = Self::mapping_resident_pages(mapping);
            if resident == 0 {
                continue;
            }
            if let Some(path) = mapping.file_path.as_ref() {
                self.push_vm_decision(
                    mapping.pid,
                    format!(
                        "agent=sync pid={} start={} len={} path={}",
                        mapping.pid, mapping.start, mapping.len, path
                    ),
                );
                self.push_vm_decision(
                    mapping.pid,
                    format!(
                        "agent=advice pid={} start={} len={} path={}",
                        mapping.pid, mapping.start, mapping.len, path
                    ),
                );
                self.push_vm_decision(
                    mapping.pid,
                    format!(
                        "agent=pressure-victim pid={} reclaimed-pages={} path={}",
                        mapping.pid, resident, path
                    ),
                );
            }
            mapping.words.clear();
            mapping.reclaimed = true;
            reclaimed = reclaimed.saturating_add(resident);
            self.push_vm_episode(
                mapping.pid,
                format!("kind=reclaim pid={} evicted=yes restored=no", mapping.pid),
            );
        }
        reclaimed.min(target_pages)
    }

    fn quarantine_vm_object(
        &self,
        pid: u64,
        vm_object_id: u64,
        reason: u64,
    ) -> Result<(), Errno> {
        let mut mappings = self.vm_mappings.borrow_mut();
        let Some(mapping) = mappings.iter_mut().find(|mapping| {
            mapping.pid == pid
                && mapping.present
                && mapping.file_inode.unwrap_or(mapping.start) == vm_object_id
        }) else {
            return Err(Errno::NoEnt);
        };
        mapping.quarantined = true;
        mapping.quarantine_reason = reason;
        drop(mappings);
        self.push_vm_decision(
            pid,
            format!("agent=quarantine-state pid={pid} object={vm_object_id} reason={reason}"),
        );
        self.push_vm_decision(
            pid,
            format!("agent=quarantine-block pid={pid} object={vm_object_id} reason={reason}"),
        );
        self.push_vm_episode(
            pid,
            format!("kind=quarantine pid={pid} object={vm_object_id} blocked=yes released=no"),
        );
        Ok(())
    }

    fn release_vm_object(&self, pid: u64, vm_object_id: u64) -> Result<(), Errno> {
        let mut mappings = self.vm_mappings.borrow_mut();
        let Some(mapping) = mappings.iter_mut().find(|mapping| {
            mapping.pid == pid
                && mapping.present
                && mapping.file_inode.unwrap_or(mapping.start) == vm_object_id
        }) else {
            return Err(Errno::NoEnt);
        };
        mapping.quarantined = false;
        mapping.quarantine_reason = 0;
        drop(mappings);
        self.push_vm_episode(
            pid,
            format!("kind=quarantine pid={pid} object={vm_object_id} blocked=yes released=yes"),
        );
        Ok(())
    }

    fn render_proc_maps(&self, pid: u64) -> Vec<u8> {
        let mut lines = vec![
            String::from("0000000000400000-0000000000401000 r-x /bin/ngos-userland-native"),
            String::from("00007ffffffd0000-0000800000000000 rw- [stack]"),
        ];
        for mapping in self
            .vm_mappings
            .borrow()
            .iter()
            .filter(|mapping| mapping.pid == pid && mapping.present)
        {
            let perms = format!(
                "{}{}{}{}",
                if mapping.readable { "r" } else { "-" },
                if mapping.writable { "w" } else { "-" },
                if mapping.executable { "x" } else { "-" },
                if mapping.private { "p" } else { "-" }
            );
            let tail = match &mapping.file_path {
                Some(path) => path.clone(),
                None => format!("[anon:{}]", mapping.label),
            };
            let offset = if mapping.file_path.is_some() {
                0x1000
            } else {
                0
            };
            lines.push(format!(
                "{:016x}-{:016x} {} {:08x} {}",
                mapping.start,
                mapping.start + mapping.len,
                perms,
                offset,
                tail
            ));
        }
        lines.join("\n").into_bytes()
    }

    fn render_vm_decisions(&self, pid: u64) -> Vec<u8> {
        self.vm_decisions
            .borrow()
            .iter()
            .filter(|(entry_pid, _)| *entry_pid == pid)
            .map(|(_, line)| line.clone())
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes()
    }

    fn render_vm_objects(&self, pid: u64) -> Vec<u8> {
        self.vm_mappings
            .borrow()
            .iter()
            .filter(|mapping| mapping.pid == pid && mapping.present)
            .map(|mapping| {
                let resident = Self::mapping_resident_pages(mapping);
                let dirty = resident;
                let object_id = mapping.file_inode.unwrap_or(mapping.start);
                format!(
                    "{object_id:016x}\tpid={} start={} len={} kind={} depth={} {} path={} resident={} dirty={} quarantined={} reason={}",
                    pid,
                    mapping.start,
                    mapping.len,
                    if mapping.file_path.is_some() {
                        "file"
                    } else {
                        "anon"
                    },
                    if mapping.cow { 1 } else { 0 },
                    if mapping.cow { "[cow]" } else { "[owned]" },
                    mapping
                        .file_path
                        .as_deref()
                        .unwrap_or(mapping.label.as_str()),
                    resident,
                    dirty,
                    if mapping.quarantined { 1 } else { 0 },
                    mapping.quarantine_reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes()
    }

    fn render_vm_episodes(&self, pid: u64) -> Vec<u8> {
        self.vm_episodes
            .borrow()
            .iter()
            .filter(|(entry_pid, _)| *entry_pid == pid)
            .map(|(_, line)| line.clone())
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes()
    }

    fn push_event_queue_record(&self, queue_fd: usize, record: NativeEventRecord) {
        let mut pending = self.event_queue_pending.borrow_mut();
        if let Some((_, records)) = pending.iter_mut().find(|(fd, _)| *fd == queue_fd) {
            records.push(record);
        } else {
            pending.push((queue_fd, vec![record]));
        }
    }

    fn take_event_queue_records(
        &self,
        queue_fd: usize,
        capacity: usize,
    ) -> Result<Vec<NativeEventRecord>, Errno> {
        let mut pending = self.event_queue_pending.borrow_mut();
        let Some((_, records)) = pending.iter_mut().find(|(fd, _)| *fd == queue_fd) else {
            return Err(Errno::Again);
        };
        if records.is_empty() {
            let nonblock = self
                .event_queue_nonblock
                .borrow()
                .iter()
                .find(|(fd, _)| *fd == queue_fd)
                .map(|(_, value)| *value)
                .unwrap_or(false);
            let has_resource_watch = self
                .resource_event_watches
                .borrow()
                .iter()
                .any(|(fd, _, _)| *fd == queue_fd);
            let has_bus_watch = self
                .bus_event_watches
                .borrow()
                .iter()
                .any(|(fd, _, _)| *fd == queue_fd);
            let has_network_watch = self
                .network_event_watches
                .borrow()
                .iter()
                .any(|watch| watch.queue_fd == queue_fd);
            let is_resource_queue = self.resource_event_queues.borrow().contains(&queue_fd);
            if nonblock
                && (has_resource_watch || has_bus_watch || has_network_watch || is_resource_queue)
            {
                return Err(Errno::Again);
            }
            if capacity == 0 {
                return Ok(Vec::new());
            }
            return Ok(vec![NativeEventRecord {
                token: queue_fd as u64,
                events: POLLPRI,
                source_kind: if has_resource_watch {
                    NativeEventSourceKind::Resource as u32
                } else if has_bus_watch {
                    NativeEventSourceKind::Bus as u32
                } else {
                    NativeEventSourceKind::Network as u32
                },
                source_arg0: if has_resource_watch {
                    42
                } else if has_bus_watch {
                    11
                } else {
                    99
                },
                source_arg1: if has_resource_watch {
                    43
                } else if has_bus_watch {
                    22
                } else {
                    0
                },
                source_arg2: 0,
                detail0: if has_resource_watch {
                    4
                } else if has_bus_watch {
                    0
                } else {
                    1
                },
                detail1: 0,
            }]);
        }
        let count = capacity.min(records.len());
        Ok(records.drain(..count).collect())
    }

    fn register_resource_event_watch(
        &self,
        queue_fd: usize,
        resource: u64,
        config: NativeResourceEventWatchConfig,
    ) {
        let mut resource_queues = self.resource_event_queues.borrow_mut();
        if !resource_queues.contains(&queue_fd) {
            resource_queues.push(queue_fd);
        }
        let mut watches = self.resource_event_watches.borrow_mut();
        if let Some((_, _, existing)) = watches.iter_mut().find(|(fd, watch_resource, existing)| {
            *fd == queue_fd && *watch_resource == resource && existing.token == config.token
        }) {
            *existing = config;
        } else {
            watches.push((queue_fd, resource, config));
        }
    }

    fn register_bus_event_watch(
        &self,
        queue_fd: usize,
        endpoint: u64,
        config: NativeBusEventWatchConfig,
    ) {
        let mut watches = self.bus_event_watches.borrow_mut();
        if let Some((_, _, existing)) = watches.iter_mut().find(|(fd, watch_endpoint, existing)| {
            *fd == queue_fd && *watch_endpoint == endpoint && existing.token == config.token
        }) {
            *existing = config;
        } else {
            watches.push((queue_fd, endpoint, config));
        }
    }

    fn remove_bus_event_watch(
        &self,
        queue_fd: usize,
        endpoint: u64,
        token: u64,
    ) -> Result<(), Errno> {
        let mut watches = self.bus_event_watches.borrow_mut();
        let original = watches.len();
        watches.retain(|(fd, watch_endpoint, config)| {
            !(*fd == queue_fd && *watch_endpoint == endpoint && config.token == token)
        });
        if watches.len() == original {
            return Err(Errno::NoEnt);
        }
        Ok(())
    }

    fn emit_bus_event(&self, peer: u64, endpoint: u64, kind: u32) {
        let watches = self.bus_event_watches.borrow().clone();
        for (queue_fd, watch_endpoint, config) in watches {
            if watch_endpoint != endpoint {
                continue;
            }
            let interested = match kind {
                0 => config.attached != 0,
                1 => config.detached != 0,
                2 => config.published != 0,
                3 => config.received != 0,
                _ => false,
            };
            if !interested {
                continue;
            }
            self.push_event_queue_record(
                queue_fd,
                NativeEventRecord {
                    token: config.token,
                    events: config.poll_events,
                    source_kind: NativeEventSourceKind::Bus as u32,
                    source_arg0: peer,
                    source_arg1: endpoint,
                    source_arg2: 0,
                    detail0: kind,
                    detail1: 0,
                },
            );
        }
    }

    fn remove_resource_event_watch(
        &self,
        queue_fd: usize,
        resource: u64,
        token: u64,
    ) -> Result<(), Errno> {
        let mut watches = self.resource_event_watches.borrow_mut();
        let original = watches.len();
        watches.retain(|(fd, watch_resource, config)| {
            !(*fd == queue_fd && *watch_resource == resource && config.token == token)
        });
        if watches.len() == original {
            return Err(Errno::NoEnt);
        }
        Ok(())
    }

    fn register_network_event_watch(
        &self,
        queue_fd: usize,
        interface_path: &str,
        socket_path: Option<&str>,
        config: NativeNetworkEventWatchConfig,
    ) {
        let mut watches = self.network_event_watches.borrow_mut();
        if let Some(existing) = watches.iter_mut().find(|watch| {
            watch.queue_fd == queue_fd
                && watch.interface_path == interface_path
                && watch.socket_path.as_deref() == socket_path
                && watch.config.token == config.token
        }) {
            existing.config = config;
            return;
        }
        watches.push(RecordedNetworkEventWatch {
            queue_fd,
            interface_path: interface_path.to_string(),
            socket_path: socket_path.map(str::to_string),
            config,
        });
    }

    fn remove_network_event_watch(
        &self,
        queue_fd: usize,
        interface_path: &str,
        socket_path: Option<&str>,
        token: u64,
    ) -> Result<(), Errno> {
        let mut watches = self.network_event_watches.borrow_mut();
        let original_len = watches.len();
        watches.retain(|watch| {
            !(watch.queue_fd == queue_fd
                && watch.interface_path == interface_path
                && watch.socket_path.as_deref() == socket_path
                && watch.config.token == token)
        });
        if watches.len() == original_len {
            return Err(Errno::NoEnt);
        }
        Ok(())
    }

    fn default_network_interface_record() -> NativeNetworkInterfaceRecord {
        NativeNetworkInterfaceRecord {
            admin_up: 0,
            link_up: 1,
            promiscuous: 0,
            reserved: 0,
            mtu: 1500,
            tx_capacity: 4,
            rx_capacity: 4,
            tx_inflight_limit: 2,
            tx_inflight_depth: 0,
            free_buffer_count: 4,
            mac: [0x02, 0x11, 0x22, 0x33, 0x44, 0x55],
            mac_reserved: [0; 2],
            ipv4_addr: [0, 0, 0, 0],
            ipv4_netmask: [0, 0, 0, 0],
            ipv4_gateway: [0, 0, 0, 0],
            ipv4_reserved: [0; 4],
            rx_ring_depth: 0,
            tx_ring_depth: 0,
            tx_packets: 0,
            rx_packets: 0,
            tx_completions: 0,
            tx_dropped: 0,
            rx_dropped: 0,
            attached_socket_count: 0,
        }
    }

    fn ensure_network_interface_state(&self, path: &str) -> Result<usize, Errno> {
        if path != "/dev/net0" && path != "/dev/net1" {
            return Err(Errno::NoEnt);
        }
        let mut interfaces = self.network_interfaces.borrow_mut();
        if let Some(index) = interfaces.iter().position(|entry| entry.path == path) {
            return Ok(index);
        }
        interfaces.push(RecordedNetworkInterfaceState {
            path: path.to_string(),
            record: Self::default_network_interface_record(),
        });
        Ok(interfaces.len() - 1)
    }

    fn network_device_path_for_driver(driver_path: &str) -> Option<&'static str> {
        match driver_path {
            "/drv/net0" => Some("/dev/net0"),
            "/drv/net1" => Some("/dev/net1"),
            _ => None,
        }
    }

    fn network_socket_path_for_device(device_path: &str) -> Option<&'static str> {
        match device_path {
            "/dev/net0" => Some("/run/net0.sock"),
            "/dev/net1" => Some("/run/net1.sock"),
            _ => None,
        }
    }

    fn network_driver_path_for_device(device_path: &str) -> Option<&'static str> {
        match device_path {
            "/dev/net0" => Some("/drv/net0"),
            "/dev/net1" => Some("/drv/net1"),
            _ => None,
        }
    }

    fn recount_attached_sockets(&self, device_path: &str) {
        let attached = self
            .network_sockets
            .borrow()
            .iter()
            .filter(|socket| socket.device_path == device_path)
            .count() as u64;
        if let Some(interface) = self
            .network_interfaces
            .borrow_mut()
            .iter_mut()
            .find(|entry| entry.path == device_path)
        {
            interface.record.attached_socket_count = attached;
        }
    }

    fn parse_udp_ipv4_frame(bytes: &[u8]) -> Option<([u8; 4], u16, u16, Vec<u8>)> {
        const ETH_HEADER_LEN: usize = 14;
        const IPV4_MIN_HEADER_LEN: usize = 20;
        const UDP_HEADER_LEN: usize = 8;
        if bytes.len() < ETH_HEADER_LEN + IPV4_MIN_HEADER_LEN + UDP_HEADER_LEN {
            return None;
        }
        let ethertype = u16::from_be_bytes([bytes[12], bytes[13]]);
        if ethertype != 0x0800 {
            return None;
        }
        let ip_start = ETH_HEADER_LEN;
        let version_ihl = bytes[ip_start];
        if version_ihl >> 4 != 4 {
            return None;
        }
        let ipv4_header_len = ((version_ihl & 0x0f) as usize) * 4;
        if ipv4_header_len < IPV4_MIN_HEADER_LEN
            || bytes.len() < ETH_HEADER_LEN + ipv4_header_len + UDP_HEADER_LEN
        {
            return None;
        }
        if bytes[ip_start + 9] != 17 {
            return None;
        }
        let src_ipv4 = [
            bytes[ip_start + 12],
            bytes[ip_start + 13],
            bytes[ip_start + 14],
            bytes[ip_start + 15],
        ];
        let udp_start = ETH_HEADER_LEN + ipv4_header_len;
        let src_port = u16::from_be_bytes([bytes[udp_start], bytes[udp_start + 1]]);
        let dst_port = u16::from_be_bytes([bytes[udp_start + 2], bytes[udp_start + 3]]);
        let udp_len = u16::from_be_bytes([bytes[udp_start + 4], bytes[udp_start + 5]]) as usize;
        if udp_len < UDP_HEADER_LEN {
            return None;
        }
        let payload_start = udp_start + UDP_HEADER_LEN;
        let payload_end = payload_start
            .saturating_add(udp_len - UDP_HEADER_LEN)
            .min(bytes.len());
        if payload_end < payload_start {
            return None;
        }
        Some((
            src_ipv4,
            src_port,
            dst_port,
            bytes[payload_start..payload_end].to_vec(),
        ))
    }

    fn emit_resource_event(&self, resource: u64, contract: u64, detail0: u32) {
        let watches = self.resource_event_watches.borrow().clone();
        for (queue_fd, watch_resource, config) in watches {
            if watch_resource != resource {
                continue;
            }
            let interested = match detail0 {
                0 => config.claimed != 0,
                1 => config.queued != 0,
                2 => config.canceled != 0,
                3 => config.released != 0,
                4 => config.handed_off != 0,
                5 => config.revoked != 0,
                _ => false,
            };
            if !interested {
                continue;
            }
            self.push_event_queue_record(
                queue_fd,
                NativeEventRecord {
                    token: config.token,
                    events: config.poll_events,
                    source_kind: NativeEventSourceKind::Resource as u32,
                    source_arg0: resource,
                    source_arg1: contract,
                    source_arg2: 0,
                    detail0,
                    detail1: 0,
                },
            );
        }
    }

    fn emit_network_event(
        &self,
        interface_path: &str,
        socket_path: Option<&str>,
        kind: NativeNetworkEventKind,
    ) {
        let watches = self.network_event_watches.borrow().clone();
        for watch in watches {
            if watch.interface_path != interface_path {
                continue;
            }
            if let Some(expected_socket) = watch.socket_path.as_deref() {
                if socket_path != Some(expected_socket) {
                    continue;
                }
            }
            let interested = match kind {
                NativeNetworkEventKind::LinkChanged => watch.config.link_changed != 0,
                NativeNetworkEventKind::RxReady => watch.config.rx_ready != 0,
                NativeNetworkEventKind::TxDrained => watch.config.tx_drained != 0,
            };
            if !interested {
                continue;
            }
            self.push_event_queue_record(
                watch.queue_fd,
                NativeEventRecord {
                    token: watch.config.token,
                    events: watch.config.poll_events,
                    source_kind: NativeEventSourceKind::Network as u32,
                    source_arg0: 99,
                    source_arg1: if socket_path.is_some() { 7 } else { 0 },
                    source_arg2: 0,
                    detail0: u32::from(socket_path.is_some()),
                    detail1: kind as u32,
                },
            );
        }
    }
}

fn set_replayed_contract_state(states: &mut Vec<(u64, u32)>, contract: u64, state: u32) {
    if let Some((_, current)) = states.iter_mut().find(|(id, _)| *id == contract) {
        *current = state;
    } else {
        states.push((contract, state));
    }
}

fn replayed_contract_state(states: &[(u64, u32)], contract: u64) -> u32 {
    states
        .iter()
        .rev()
        .find(|(id, _)| *id == contract)
        .map(|(_, state)| *state)
        .unwrap_or(NativeContractState::Active as u32)
}

fn replayed_process_scheduler(frames: &[SyscallFrame], pid: u64) -> (u32, u32) {
    frames
        .iter()
        .rev()
        .find(|entry| entry.number == SYS_RENICE_PROCESS && entry.arg0 as u64 == pid)
        .map(|entry| (entry.arg1 as u32, entry.arg2 as u32))
        .unwrap_or_else(|| {
            if pid == 1 {
                (NativeSchedulerClass::Interactive as u32, 2)
            } else {
                (NativeSchedulerClass::LatencyCritical as u32, 4)
            }
        })
}

fn replayed_process_state(frames: &[SyscallFrame], pid: u64) -> u32 {
    if pid >= 77 {
        return 4;
    }
    frames
        .iter()
        .rev()
        .find_map(|entry| {
            if entry.arg0 as u64 != pid {
                return None;
            }
            match entry.number {
                SYS_PAUSE_PROCESS => Some(3),
                SYS_RESUME_PROCESS => Some(1),
                _ => None,
            }
        })
        .unwrap_or(2)
}

fn replayed_system_snapshot(frames: &[SyscallFrame]) -> NativeSystemSnapshotRecord {
    let renice_count = frames
        .iter()
        .filter(|entry| entry.number == SYS_RENICE_PROCESS)
        .count() as u64;
    let pause_count = frames
        .iter()
        .filter(|entry| entry.number == SYS_PAUSE_PROCESS)
        .count() as u64;
    let net_admin_count = frames
        .iter()
        .filter(|entry| entry.number == SYS_CONFIGURE_NETIF_ADMIN)
        .count() as u64;
    let run_queue = 5u64
        .saturating_sub(renice_count.saturating_mul(2))
        .saturating_sub(pause_count);
    let cpu_busy = if run_queue > 0 { 96 } else { 54 };
    let socket_limit: u64 = if net_admin_count == 0 { 16 } else { 32 };
    let socket_depth: u64 = if net_admin_count == 0 { 15 } else { 6 };
    let saturated = u64::from(socket_depth >= socket_limit.saturating_sub(1));
    NativeSystemSnapshotRecord {
        current_tick: 100 + frames.len() as u64,
        busy_ticks: 80 + (frames.len() as u64).saturating_mul(cpu_busy) / 100,
        process_count: 3,
        active_process_count: 3u64.saturating_sub(pause_count.min(1)),
        blocked_process_count: pause_count.min(1),
        queued_processes: run_queue,
        queued_latency_critical: 1,
        queued_interactive: 1u64.min(run_queue),
        queued_normal: run_queue.saturating_sub(2).min(2),
        queued_background: run_queue.saturating_sub(3),
        queued_urgent_latency_critical: 0,
        queued_urgent_interactive: u64::from(run_queue >= 2),
        queued_urgent_normal: 0,
        queued_urgent_background: 0,
        lag_debt_latency_critical: 0,
        lag_debt_interactive: i64::from(run_queue >= 2) * 4,
        lag_debt_normal: 0,
        lag_debt_background: i64::from(run_queue >= 4) * 3,
        dispatch_count_latency_critical: 0,
        dispatch_count_interactive: run_queue.min(3),
        dispatch_count_normal: u64::from(run_queue >= 2),
        dispatch_count_background: u64::from(run_queue >= 4),
        runtime_ticks_latency_critical: 0,
        runtime_ticks_interactive: run_queue.min(3),
        runtime_ticks_normal: u64::from(run_queue >= 2),
        runtime_ticks_background: u64::from(run_queue >= 4),
        scheduler_cpu_count: 2,
        scheduler_running_cpu: 0,
        scheduler_cpu_load_imbalance: run_queue.saturating_sub(2),
        starved_latency_critical: NativeSystemSnapshotRecord::SCHEDULER_POLICY_FALSE,
        starved_interactive: NativeSystemSnapshotRecord::SCHEDULER_POLICY_FALSE,
        starved_normal: NativeSystemSnapshotRecord::SCHEDULER_POLICY_FALSE,
        starved_background: u64::from(run_queue >= 4),
        deferred_task_count: 1,
        sleeping_processes: pause_count.min(1),
        total_event_queue_count: frames
            .iter()
            .filter(|entry| entry.number == SYS_WATCH_PROCESS_EVENTS)
            .count() as u64
            + frames
                .iter()
                .filter(|entry| entry.number == SYS_WATCH_RESOURCE_EVENTS)
                .count() as u64,
        total_event_queue_pending: if run_queue > 2 { 10 } else { 2 },
        total_event_queue_waiters: if run_queue > 2 { 3 } else { 1 },
        total_socket_count: 2,
        saturated_socket_count: saturated,
        total_socket_rx_depth: socket_depth,
        total_socket_rx_limit: socket_limit,
        max_socket_rx_depth: socket_depth.saturating_sub(2),
        total_network_tx_dropped: 4,
        total_network_rx_dropped: 3,
        running_pid: 1,
        reserved0: NativeSystemSnapshotRecord::VERIFIED_CORE_OK_TRUE,
        reserved1: 0,
    }
}

fn pop_next_active_waiter(waiters: &mut Vec<u64>, states: &[(u64, u32)]) -> Option<u64> {
    while let Some(waiter) = waiters.first().copied() {
        waiters.remove(0);
        if replayed_contract_state(states, waiter) == NativeContractState::Active as u32 {
            return Some(waiter);
        }
    }
    None
}

fn replayed_contract_kind(frames: &[SyscallFrame], contract: u64) -> u32 {
    let mut next_contract = 43u64;
    for entry in frames {
        if entry.number == SYS_CREATE_CONTRACT {
            if next_contract == contract {
                return entry.arg2 as u32;
            }
            next_contract += 1;
        }
    }
    NativeContractKind::Display as u32
}

fn replayed_contract_resource(frames: &[SyscallFrame], contract: u64) -> u64 {
    let mut next_contract = 43u64;
    for entry in frames {
        if entry.number == SYS_CREATE_CONTRACT {
            if next_contract == contract {
                return entry.arg1 as u64;
            }
            next_contract += 1;
        }
    }
    42
}

fn replayed_resource_contract_policy(frames: &[SyscallFrame], resource: u64) -> u32 {
    frames
        .iter()
        .rev()
        .find(|entry| {
            entry.number == SYS_SET_RESOURCE_CONTRACT_POLICY && entry.arg0 as u64 == resource
        })
        .map(|entry| entry.arg1 as u32)
        .unwrap_or(NativeResourceContractPolicy::Any as u32)
}

fn replayed_resource_operational_state(frames: &[SyscallFrame], resource: u64) -> u32 {
    frames
        .iter()
        .rev()
        .find(|entry| entry.number == SYS_SET_RESOURCE_STATE && entry.arg0 as u64 == resource)
        .map(|entry| entry.arg1 as u32)
        .unwrap_or(NativeResourceState::Active as u32)
}

fn replayed_resource_governance(frames: &[SyscallFrame], resource: u64) -> u32 {
    frames
        .iter()
        .rev()
        .find(|entry| entry.number == SYS_SET_RESOURCE_GOVERNANCE && entry.arg0 as u64 == resource)
        .map(|entry| entry.arg1 as u32)
        .unwrap_or(NativeResourceGovernanceMode::Queueing as u32)
}

fn replay_resource_state(frames: &[SyscallFrame], resource: u64) -> (u64, u64, u64, Vec<u64>) {
    let mut holder_contract = 0u64;
    let mut acquire_count = 0u64;
    let mut handoff_count = 0u64;
    let mut fifo = true;
    let mut exclusive = false;
    let mut resource_state = NativeResourceState::Active as u32;
    let mut waiters = Vec::new();
    let mut states = Vec::new();
    for entry in frames {
        match entry.number {
            SYS_SET_RESOURCE_POLICY if entry.arg0 as u64 == resource => {
                fifo = entry.arg1 == NativeResourceArbitrationPolicy::Fifo as usize;
            }
            SYS_SET_RESOURCE_GOVERNANCE if entry.arg0 as u64 == resource => {
                exclusive = entry.arg1 == NativeResourceGovernanceMode::ExclusiveLease as usize;
                if exclusive {
                    waiters.clear();
                }
            }
            SYS_SET_CONTRACT_STATE => {
                let contract = entry.arg0 as u64;
                let state = entry.arg1 as u32;
                set_replayed_contract_state(&mut states, contract, state);
                waiters.retain(|waiter| *waiter != contract);
                if holder_contract == contract && state != NativeContractState::Active as u32 {
                    holder_contract = pop_next_active_waiter(&mut waiters, &states).unwrap_or(0);
                    if holder_contract != 0 {
                        acquire_count += 1;
                        handoff_count += 1;
                    }
                }
            }
            SYS_SET_RESOURCE_STATE if entry.arg0 as u64 == resource => {
                resource_state = entry.arg1 as u32;
                if resource_state != NativeResourceState::Active as u32 {
                    holder_contract = 0;
                    waiters.clear();
                }
            }
            SYS_CLAIM_RESOURCE => {
                if replayed_contract_resource(frames, entry.arg0 as u64) != resource {
                    continue;
                }
                if resource_state != NativeResourceState::Active as u32 {
                    continue;
                }
                let contract = entry.arg0 as u64;
                if replayed_contract_state(&states, contract) != NativeContractState::Active as u32
                {
                    continue;
                }
                if holder_contract == 0 {
                    holder_contract = contract;
                    acquire_count += 1;
                } else if exclusive {
                    continue;
                } else if holder_contract != contract && !waiters.contains(&contract) {
                    if fifo {
                        waiters.push(contract);
                    } else {
                        waiters.insert(0, contract);
                    }
                }
            }
            SYS_CANCEL_RESOURCE_CLAIM => {
                let contract = entry.arg0 as u64;
                if replayed_contract_resource(frames, contract) != resource {
                    continue;
                }
                waiters.retain(|waiter| *waiter != contract);
            }
            SYS_TRANSFER_RESOURCE
                if replayed_contract_resource(frames, entry.arg0 as u64) == resource
                    && holder_contract == entry.arg0 as u64 =>
            {
                waiters.retain(|waiter| *waiter != entry.arg1 as u64);
                holder_contract = entry.arg1 as u64;
                acquire_count += 1;
                handoff_count += 1;
            }
            SYS_RELEASE_CLAIMED_RESOURCE
                if replayed_contract_resource(frames, entry.arg0 as u64) == resource
                    && holder_contract == entry.arg0 as u64 =>
            {
                if let Some(next) = pop_next_active_waiter(&mut waiters, &states) {
                    holder_contract = next;
                    acquire_count += 1;
                    handoff_count += 1;
                } else {
                    holder_contract = 0;
                }
            }
            SYS_RELEASE_RESOURCE
                if replayed_contract_resource(frames, entry.arg0 as u64) == resource
                    && holder_contract == entry.arg0 as u64 =>
            {
                holder_contract = 0;
            }
            _ => {}
        }
    }
    (holder_contract, acquire_count, handoff_count, waiters)
}

fn find_payload_value<'a>(payload: &'a str, prefix: &str) -> Option<&'a str> {
    payload
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(str::trim)
}

fn find_env_value<'a>(envp: &'a [String], key: &str) -> Option<&'a str> {
    envp.iter().rev().find_map(|entry| {
        entry
            .split_once('=')
            .and_then(|(candidate_key, value)| (candidate_key == key).then_some(value))
    })
}

fn fill_fixed_field<const N: usize>(dst: &mut [u8; N], value: &str) {
    dst.fill(0);
    let bytes = value.as_bytes();
    let count = bytes.len().min(N);
    dst[..count].copy_from_slice(&bytes[..count]);
}

impl ngos_user_abi::SyscallBackend for RecordingBackend {
    unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
        self.frames.borrow_mut().push(frame);
        match frame.number {
            SYS_READ => {
                if frame.arg0 != 0 {
                    let Some(path) = self.opened_path_raw(frame.arg0) else {
                        return SyscallReturn::err(Errno::Badf);
                    };
                    if path == "/drv/gpu0" {
                        let payload = self
                            .latest_gpu_request()
                            .map(|request| request.payload)
                            .unwrap_or_default();
                        let read = payload.len().min(frame.arg2);
                        if read != 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    payload.as_ptr(),
                                    frame.arg1 as *mut u8,
                                    read,
                                );
                            }
                        }
                        return SyscallReturn::ok(read);
                    }
                    if path == "/drv/audio0" {
                        let payload = self
                            .latest_audio_request()
                            .map(|request| {
                                let mut bytes = format!("request:{}\n", request.id).into_bytes();
                                bytes.extend_from_slice(&request.payload);
                                bytes
                            })
                            .unwrap_or_default();
                        let read = payload.len().min(frame.arg2);
                        if read != 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    payload.as_ptr(),
                                    frame.arg1 as *mut u8,
                                    read,
                                );
                            }
                        }
                        return SyscallReturn::ok(read);
                    }
                    if path == "/drv/input0" {
                        let payload = self
                            .latest_input_request()
                            .map(|request| {
                                let mut bytes = format!("request:{}\n", request.id).into_bytes();
                                bytes.extend_from_slice(&request.payload);
                                bytes
                            })
                            .unwrap_or_default();
                        let read = payload.len().min(frame.arg2);
                        if read != 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    payload.as_ptr(),
                                    frame.arg1 as *mut u8,
                                    read,
                                );
                            }
                        }
                        return SyscallReturn::ok(read);
                    }
                    if path == "/dev/audio0" {
                        let payload = self.audio_completion_payload.borrow().clone();
                        let read = payload.len().min(frame.arg2);
                        if read != 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    payload.as_ptr(),
                                    frame.arg1 as *mut u8,
                                    read,
                                );
                            }
                        }
                        return SyscallReturn::ok(read);
                    }
                    if path == "/dev/input0" {
                        let payload = self.input_completion_payload.borrow().clone();
                        let read = payload.len().min(frame.arg2);
                        if read != 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    payload.as_ptr(),
                                    frame.arg1 as *mut u8,
                                    read,
                                );
                            }
                        }
                        return SyscallReturn::ok(read);
                    }
                    if self.created_kind(&path) == Some(NativeObjectKind::Channel) {
                        let payload = self.pop_channel_message(&path).unwrap_or_default();
                        let read = payload.len().min(frame.arg2);
                        if read != 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    payload.as_ptr(),
                                    frame.arg1 as *mut u8,
                                    read,
                                );
                            }
                        }
                        return SyscallReturn::ok(read);
                    }
                    let payload = self.file_content_for_fd(frame.arg0).unwrap_or_default();
                    let offset = self.read_offset(frame.arg0);
                    let read = payload.len().saturating_sub(offset).min(frame.arg2);
                    if read != 0 {
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                payload.as_ptr().add(offset),
                                frame.arg1 as *mut u8,
                                read,
                            );
                        }
                        self.set_read_offset(frame.arg0, offset + read);
                    }
                    return SyscallReturn::ok(read);
                }
                let stdin = self.stdin.borrow();
                let offset = self.stdin_offset.get();
                let remaining = stdin.len().saturating_sub(offset);
                let read = remaining.min(frame.arg2);
                if read != 0 {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            stdin.as_ptr().add(offset),
                            frame.arg1 as *mut u8,
                            read,
                        );
                    }
                    self.stdin_offset.set(offset + read);
                }
                SyscallReturn::ok(read)
            }
            SYS_CREATE_DOMAIN => {
                let count = self
                    .frames
                    .borrow()
                    .iter()
                    .filter(|entry| entry.number == SYS_CREATE_DOMAIN)
                    .count();
                SyscallReturn::ok(40 + count)
            }
            SYS_CREATE_RESOURCE => {
                let count = self
                    .frames
                    .borrow()
                    .iter()
                    .filter(|entry| entry.number == SYS_CREATE_RESOURCE)
                    .count();
                SyscallReturn::ok(41 + count)
            }
            SYS_CREATE_CONTRACT => {
                let resource = frame.arg1 as u64;
                let frames = self.frames.borrow();
                if replayed_resource_operational_state(&frames, resource)
                    != NativeResourceState::Active as u32
                {
                    return SyscallReturn::err(Errno::Access);
                }
                let policy = replayed_resource_contract_policy(
                    &frames[..frames.len().saturating_sub(1)],
                    resource,
                );
                if policy != NativeResourceContractPolicy::Any as u32
                    && frame.arg2 as u32 != policy - 1
                {
                    return SyscallReturn::err(Errno::Access);
                }
                let count = self
                    .frames
                    .borrow()
                    .iter()
                    .filter(|entry| entry.number == SYS_CREATE_CONTRACT)
                    .count();
                SyscallReturn::ok(42 + count)
            }
            SYS_CREATE_BUS_PEER => {
                let id = self.next_bus_peer_id.get();
                self.next_bus_peer_id.set(id + 1);
                let name = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg1 as *const u8,
                        frame.arg2,
                    ))
                };
                self.bus_peers.borrow_mut().push(RecordedBusPeerState {
                    record: NativeBusPeerRecord {
                        id,
                        owner: 1,
                        domain: frame.arg0 as u64,
                        attached_endpoint_count: 0,
                        publish_count: 0,
                        receive_count: 0,
                        last_endpoint: 0,
                    },
                    name: name.to_string(),
                });
                SyscallReturn::ok(id as usize)
            }
            SYS_CREATE_BUS_ENDPOINT => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                if self.created_kind(path) != Some(NativeObjectKind::Channel) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let id = self.next_bus_endpoint_id.get();
                self.next_bus_endpoint_id.set(id + 1);
                self.bus_endpoints
                    .borrow_mut()
                    .push(RecordedBusEndpointState {
                        record: NativeBusEndpointRecord {
                            id,
                            domain: frame.arg0 as u64,
                            resource: frame.arg1 as u64,
                            kind: 0,
                            reserved: 0,
                            attached_peer_count: 0,
                            publish_count: 0,
                            receive_count: 0,
                            byte_count: 0,
                            queue_depth: 0,
                            queue_capacity: 64,
                            peak_queue_depth: 0,
                            overflow_count: 0,
                            last_peer: 0,
                        },
                        path: path.to_string(),
                        messages: Vec::new(),
                    });
                SyscallReturn::ok(id as usize)
            }
            SYS_ATTACH_BUS_PEER => {
                let peer = frame.arg0 as u64;
                let endpoint = frame.arg1 as u64;
                let mut peers = self.bus_peers.borrow_mut();
                let Some(peer_entry) = peers.iter_mut().find(|entry| entry.record.id == peer)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let mut endpoints = self.bus_endpoints.borrow_mut();
                let Some(endpoint_entry) = endpoints
                    .iter_mut()
                    .find(|entry| entry.record.id == endpoint)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                peer_entry.record.attached_endpoint_count =
                    peer_entry.record.attached_endpoint_count.saturating_add(1);
                peer_entry.record.last_endpoint = endpoint;
                endpoint_entry.record.attached_peer_count =
                    endpoint_entry.record.attached_peer_count.saturating_add(1);
                endpoint_entry.record.last_peer = peer;
                drop(endpoints);
                drop(peers);
                self.emit_bus_event(peer, endpoint, 0);
                SyscallReturn::ok(0)
            }
            SYS_DETACH_BUS_PEER => {
                let peer = frame.arg0 as u64;
                let endpoint = frame.arg1 as u64;
                let mut peers = self.bus_peers.borrow_mut();
                let Some(peer_entry) = peers.iter_mut().find(|entry| entry.record.id == peer)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let mut endpoints = self.bus_endpoints.borrow_mut();
                let Some(endpoint_entry) = endpoints
                    .iter_mut()
                    .find(|entry| entry.record.id == endpoint)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                peer_entry.record.attached_endpoint_count =
                    peer_entry.record.attached_endpoint_count.saturating_sub(1);
                endpoint_entry.record.attached_peer_count =
                    endpoint_entry.record.attached_peer_count.saturating_sub(1);
                drop(endpoints);
                drop(peers);
                self.emit_bus_event(peer, endpoint, 1);
                SyscallReturn::ok(0)
            }
            SYS_LIST_DOMAINS => {
                let ptr = frame.arg0 as *mut u64;
                unsafe {
                    *ptr = 41;
                }
                SyscallReturn::ok(1)
            }
            SYS_LIST_RESOURCES => {
                let ptr = frame.arg0 as *mut u64;
                unsafe {
                    *ptr = 42;
                }
                SyscallReturn::ok(1)
            }
            SYS_LIST_CONTRACTS => {
                let ptr = frame.arg0 as *mut u64;
                unsafe {
                    *ptr = 43;
                    *ptr.add(1) = 44;
                    *ptr.add(2) = 45;
                }
                SyscallReturn::ok(3)
            }
            SYS_LIST_BUS_PEERS => {
                let ptr = frame.arg0 as *mut u64;
                let peers = self.bus_peers.borrow();
                for (index, peer) in peers.iter().take(frame.arg1).enumerate() {
                    unsafe {
                        *ptr.add(index) = peer.record.id;
                    }
                }
                SyscallReturn::ok(peers.len().min(frame.arg1))
            }
            SYS_LIST_BUS_ENDPOINTS => {
                let ptr = frame.arg0 as *mut u64;
                let endpoints = self.bus_endpoints.borrow();
                for (index, endpoint) in endpoints.iter().take(frame.arg1).enumerate() {
                    unsafe {
                        *ptr.add(index) = endpoint.record.id;
                    }
                }
                SyscallReturn::ok(endpoints.len().min(frame.arg1))
            }
            SYS_INSPECT_BUS_PEER => {
                let id = frame.arg0 as u64;
                let ptr = frame.arg1 as *mut NativeBusPeerRecord;
                let peers = self.bus_peers.borrow();
                let Some(peer) = peers.iter().find(|entry| entry.record.id == id) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                unsafe {
                    *ptr = peer.record;
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_BUS_ENDPOINT => {
                let id = frame.arg0 as u64;
                let ptr = frame.arg1 as *mut NativeBusEndpointRecord;
                let endpoints = self.bus_endpoints.borrow();
                let Some(endpoint) = endpoints.iter().find(|entry| entry.record.id == id) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                unsafe {
                    *ptr = endpoint.record;
                }
                SyscallReturn::ok(0)
            }
            SYS_PUBLISH_BUS_MESSAGE => {
                let peer = frame.arg0 as u64;
                let endpoint = frame.arg1 as u64;
                let payload =
                    unsafe { core::slice::from_raw_parts(frame.arg2 as *const u8, frame.arg3) };
                let mut peers = self.bus_peers.borrow_mut();
                let Some(peer_entry) = peers.iter_mut().find(|entry| entry.record.id == peer)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let mut endpoints = self.bus_endpoints.borrow_mut();
                let Some(endpoint_entry) = endpoints
                    .iter_mut()
                    .find(|entry| entry.record.id == endpoint)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                if peer_entry.record.attached_endpoint_count == 0
                    || peer_entry.record.last_endpoint != endpoint
                    || endpoint_entry.record.attached_peer_count == 0
                {
                    return SyscallReturn::err(Errno::Inval);
                }
                if endpoint_entry.messages.len() >= endpoint_entry.record.queue_capacity as usize {
                    endpoint_entry.record.overflow_count =
                        endpoint_entry.record.overflow_count.saturating_add(1);
                    return SyscallReturn::err(Errno::Again);
                }
                peer_entry.record.publish_count = peer_entry.record.publish_count.saturating_add(1);
                peer_entry.record.last_endpoint = endpoint;
                endpoint_entry.record.publish_count =
                    endpoint_entry.record.publish_count.saturating_add(1);
                endpoint_entry.record.byte_count = endpoint_entry
                    .record
                    .byte_count
                    .saturating_add(payload.len() as u64);
                endpoint_entry.messages.push(payload.to_vec());
                endpoint_entry.record.queue_depth = endpoint_entry.messages.len() as u64;
                endpoint_entry.record.peak_queue_depth = endpoint_entry
                    .record
                    .peak_queue_depth
                    .max(endpoint_entry.record.queue_depth);
                endpoint_entry.record.last_peer = peer;
                drop(endpoints);
                drop(peers);
                self.emit_bus_event(peer, endpoint, 2);
                SyscallReturn::ok(payload.len())
            }
            SYS_RECEIVE_BUS_MESSAGE => {
                let peer = frame.arg0 as u64;
                let endpoint = frame.arg1 as u64;
                let buffer = frame.arg2 as *mut u8;
                let buffer_len = frame.arg3;
                let mut peers = self.bus_peers.borrow_mut();
                let Some(peer_entry) = peers.iter_mut().find(|entry| entry.record.id == peer)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let mut endpoints = self.bus_endpoints.borrow_mut();
                let Some(endpoint_entry) = endpoints
                    .iter_mut()
                    .find(|entry| entry.record.id == endpoint)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                if peer_entry.record.attached_endpoint_count == 0
                    || peer_entry.record.last_endpoint != endpoint
                    || endpoint_entry.record.attached_peer_count == 0
                {
                    return SyscallReturn::err(Errno::Inval);
                }
                let Some(message) = endpoint_entry.messages.first().cloned() else {
                    return SyscallReturn::err(Errno::Again);
                };
                let copied = message.len().min(buffer_len);
                unsafe {
                    core::ptr::copy_nonoverlapping(message.as_ptr(), buffer, copied);
                }
                endpoint_entry.messages.remove(0);
                endpoint_entry.record.queue_depth = endpoint_entry.messages.len() as u64;
                endpoint_entry.record.receive_count =
                    endpoint_entry.record.receive_count.saturating_add(1);
                endpoint_entry.record.last_peer = peer;
                peer_entry.record.receive_count = peer_entry.record.receive_count.saturating_add(1);
                peer_entry.record.last_endpoint = endpoint;
                drop(endpoints);
                drop(peers);
                self.emit_bus_event(peer, endpoint, 3);
                SyscallReturn::ok(copied)
            }
            SYS_LIST_PROCESSES => {
                let ptr = frame.arg0 as *mut u64;
                let spawned = self.next_pid.get().saturating_sub(77) as usize;
                unsafe {
                    *ptr = 1;
                    for index in 0..spawned {
                        *ptr.add(1 + index) = 77 + index as u64;
                    }
                    *ptr.add(1 + spawned) = 88;
                }
                SyscallReturn::ok(spawned + 2)
            }
            SYS_INSPECT_PROCESS => {
                let ptr = frame.arg1 as *mut NativeProcessRecord;
                let pid = frame.arg0 as u64;
                let frames = self.frames.borrow();
                let state = replayed_process_state(&frames, pid);
                let (scheduler_class, scheduler_budget) = replayed_process_scheduler(&frames, pid);
                let exit_code = if pid >= 77 && pid < self.next_pid.get() {
                    137
                } else {
                    0
                };
                unsafe {
                    ptr.write(NativeProcessRecord {
                        pid,
                        parent: 0,
                        address_space: 4,
                        main_thread: pid,
                        state,
                        exit_code,
                        descriptor_count: 3,
                        capability_count: 2,
                        environment_count: 1,
                        memory_region_count: 2,
                        thread_count: 1,
                        pending_signal_count: if pid == 1 { 1 } else { 0 },
                        session_reported: 0,
                        session_status: 0,
                        session_stage: 0,
                        scheduler_class,
                        scheduler_budget,
                        cpu_runtime_ticks: match pid {
                            88 => 91,
                            77.. => 48 + pid.saturating_sub(77),
                            _ => 12,
                        },
                        execution_contract: 0,
                        memory_contract: 0,
                        io_contract: 0,
                        observe_contract: 0,
                        reserved: 0,
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_PROCESS_COMPAT => {
                let ptr = frame.arg1 as *mut NativeProcessCompatRecord;
                let pid = frame.arg0 as u64;
                let process =
                    self.recorded_process(pid)
                        .unwrap_or_else(|| RecordedProcessBootstrap {
                            pid,
                            name: String::from("ngos-userland-native"),
                            image_path: String::from("/bin/ngos-userland-native"),
                            cwd: String::from("/"),
                            argv: Vec::new(),
                            envp: Vec::new(),
                        });
                let target =
                    find_env_value(&process.envp, "NGOS_COMPAT_TARGET").unwrap_or("native");
                let route_class = find_env_value(&process.envp, "NGOS_COMPAT_ABI_ROUTE_CLASS")
                    .unwrap_or("native-process-abi");
                let handle_profile =
                    find_env_value(&process.envp, "NGOS_COMPAT_ABI_HANDLE_PROFILE")
                        .unwrap_or("native-handles");
                let path_profile = find_env_value(&process.envp, "NGOS_COMPAT_ABI_PATH_PROFILE")
                    .unwrap_or("native-paths");
                let scheduler_profile =
                    find_env_value(&process.envp, "NGOS_COMPAT_ABI_SCHEDULER_PROFILE")
                        .unwrap_or("native-scheduler");
                let sync_profile = find_env_value(&process.envp, "NGOS_COMPAT_ABI_SYNC_PROFILE")
                    .unwrap_or("native-sync");
                let timer_profile = find_env_value(&process.envp, "NGOS_COMPAT_ABI_TIMER_PROFILE")
                    .unwrap_or("native-timer");
                let module_profile =
                    find_env_value(&process.envp, "NGOS_COMPAT_ABI_MODULE_PROFILE")
                        .unwrap_or("native-module");
                let event_profile = find_env_value(&process.envp, "NGOS_COMPAT_ABI_EVENT_PROFILE")
                    .unwrap_or("native-event");
                let requires_kernel_abi_shims =
                    find_env_value(&process.envp, "NGOS_COMPAT_ABI_REQUIRES_SHIMS")
                        .map(|value| value != "0")
                        .unwrap_or(false);
                let prefix = find_env_value(&process.envp, "NGOS_COMPAT_PREFIX").unwrap_or("/");
                let executable_path = &process.image_path;
                let working_dir = &process.cwd;
                let loader_route_class = find_env_value(&process.envp, "NGOS_COMPAT_ROUTE_CLASS")
                    .unwrap_or("native-direct");
                let loader_launch_mode = find_env_value(&process.envp, "NGOS_COMPAT_LAUNCH_MODE")
                    .unwrap_or("native-direct");
                let loader_entry_profile =
                    find_env_value(&process.envp, "NGOS_COMPAT_ENTRY_PROFILE")
                        .unwrap_or("native-entry");
                let loader_requires_compat_shims =
                    find_env_value(&process.envp, "NGOS_COMPAT_REQUIRES_SHIMS")
                        .map(|value| value != "0")
                        .unwrap_or(false);
                unsafe {
                    ptr.write(NativeProcessCompatRecord {
                        pid,
                        target: {
                            let mut field = [0; 16];
                            fill_fixed_field(&mut field, target);
                            field
                        },
                        route_class: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, route_class);
                            field
                        },
                        handle_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, handle_profile);
                            field
                        },
                        path_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, path_profile);
                            field
                        },
                        scheduler_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, scheduler_profile);
                            field
                        },
                        sync_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, sync_profile);
                            field
                        },
                        timer_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, timer_profile);
                            field
                        },
                        module_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, module_profile);
                            field
                        },
                        event_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, event_profile);
                            field
                        },
                        requires_kernel_abi_shims: u32::from(requires_kernel_abi_shims),
                        prefix: {
                            let mut field = [0; 64];
                            fill_fixed_field(&mut field, prefix);
                            field
                        },
                        executable_path: {
                            let mut field = [0; 64];
                            fill_fixed_field(&mut field, executable_path);
                            field
                        },
                        working_dir: {
                            let mut field = [0; 64];
                            fill_fixed_field(&mut field, working_dir);
                            field
                        },
                        loader_route_class: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, loader_route_class);
                            field
                        },
                        loader_launch_mode: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, loader_launch_mode);
                            field
                        },
                        loader_entry_profile: {
                            let mut field = [0; 32];
                            fill_fixed_field(&mut field, loader_entry_profile);
                            field
                        },
                        loader_requires_compat_shims: u32::from(loader_requires_compat_shims),
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_SYSTEM_SNAPSHOT => {
                let ptr = frame.arg0 as *mut NativeSystemSnapshotRecord;
                let override_record = self.system_snapshot_override.borrow().clone();
                unsafe {
                    ptr.write(match override_record {
                        Some(record) => record,
                        None => {
                            let frames = self.frames.borrow();
                            replayed_system_snapshot(&frames)
                        }
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_GET_PROCESS_NAME => {
                let ptr = frame.arg1 as *mut u8;
                let payload = match frame.arg0 as u64 {
                    pid if pid >= 77 && pid < self.next_pid.get() => self
                        .recorded_process(pid)
                        .map(|process| process.name.into_bytes())
                        .unwrap_or_else(|| b"worker".to_vec()),
                    88 => b"bg-net-pump".to_vec(),
                    _ => b"ngos-userland-native".to_vec(),
                };
                let count = payload.len().min(frame.arg2);
                unsafe {
                    core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                }
                SyscallReturn::ok(count)
            }
            SYS_GET_PROCESS_IMAGE_PATH => {
                let ptr = frame.arg1 as *mut u8;
                let payload = match frame.arg0 as u64 {
                    pid if pid >= 77 && pid < self.next_pid.get() => self
                        .recorded_process(pid)
                        .map(|process| process.image_path.into_bytes())
                        .unwrap_or_else(|| b"/bin/worker".to_vec()),
                    88 => b"/bin/bg-net-pump".to_vec(),
                    _ => b"/bin/ngos-userland-native".to_vec(),
                };
                let count = payload.len().min(frame.arg2);
                unsafe {
                    core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                }
                SyscallReturn::ok(count)
            }
            SYS_GET_PROCESS_CWD => {
                let ptr = frame.arg1 as *mut u8;
                let payload = match frame.arg0 as u64 {
                    pid if pid >= 77 && pid < self.next_pid.get() => self
                        .recorded_process(pid)
                        .map(|process| process.cwd.into_bytes())
                        .unwrap_or_else(|| b"/".to_vec()),
                    _ => b"/".to_vec(),
                };
                let count = payload.len().min(frame.arg2);
                unsafe {
                    core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                }
                SyscallReturn::ok(count)
            }
            SYS_GET_PROCESS_ROOT => {
                let ptr = frame.arg1 as *mut u8;
                let payload = b"/".to_vec();
                let count = payload.len().min(frame.arg2);
                unsafe {
                    core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                }
                SyscallReturn::ok(count)
            }
            SYS_GET_PROCESS_IDENTITY => {
                let ptr = frame.arg1 as *mut NativeProcessIdentityRecord;
                unsafe {
                    ptr.write(NativeProcessIdentityRecord {
                        uid: 1000,
                        gid: 1000,
                        umask: 0o022,
                        supplemental_count: 0,
                        supplemental_gids: [0; 8],
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_CONFIGURE_NETIF_IPV4 => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let config = unsafe {
                    (frame.arg2 as *const ngos_user_abi::NativeNetworkInterfaceConfig)
                        .read_unaligned()
                };
                let Ok(index) = self.ensure_network_interface_state(path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let mut interfaces = self.network_interfaces.borrow_mut();
                interfaces[index].record.ipv4_addr = config.addr;
                interfaces[index].record.ipv4_netmask = config.netmask;
                interfaces[index].record.ipv4_gateway = config.gateway;
                SyscallReturn::ok(0)
            }
            SYS_CONFIGURE_NETIF_ADMIN => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Ok(index) = self.ensure_network_interface_state(path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let config = unsafe {
                    (frame.arg2 as *const ngos_user_abi::NativeNetworkAdminConfig).read_unaligned()
                };
                let mut interfaces = self.network_interfaces.borrow_mut();
                let record = &mut interfaces[index].record;
                record.mtu = config.mtu;
                record.tx_capacity = config.tx_capacity;
                record.rx_capacity = config.rx_capacity;
                record.tx_inflight_limit = config.tx_inflight_limit;
                record.admin_up = config.admin_up;
                record.promiscuous = config.promiscuous;
                record.free_buffer_count = record.rx_capacity;
                SyscallReturn::ok(0)
            }
            SYS_SET_NETIF_LINK_STATE => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let config = unsafe {
                    (frame.arg2 as *const ngos_user_abi::NativeNetworkLinkStateConfig)
                        .read_unaligned()
                };
                let Ok(index) = self.ensure_network_interface_state(path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let mut interfaces = self.network_interfaces.borrow_mut();
                interfaces[index].record.link_up = config.link_up;
                drop(interfaces);
                self.emit_network_event(path, None, NativeNetworkEventKind::LinkChanged);
                SyscallReturn::ok(0)
            }
            SYS_BIND_UDP_SOCKET => {
                let socket_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let device_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                let config = unsafe {
                    (frame.arg4 as *const ngos_user_abi::NativeUdpBindConfig).read_unaligned()
                };
                let Ok(_) = self.ensure_network_interface_state(device_path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let local_ipv4 = self
                    .network_interfaces
                    .borrow()
                    .iter()
                    .find(|entry| entry.path == device_path)
                    .map(|entry| entry.record.ipv4_addr)
                    .unwrap_or([0, 0, 0, 0]);
                if self.created_kind(socket_path) != Some(NativeObjectKind::Socket) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let mut sockets = self.network_sockets.borrow_mut();
                let record = NativeNetworkSocketRecord {
                    local_ipv4,
                    remote_ipv4: config.remote_ipv4,
                    local_port: config.local_port,
                    remote_port: config.remote_port,
                    connected: 0,
                    reserved: 0,
                    rx_depth: 0,
                    rx_queue_limit: 32,
                    tx_packets: 0,
                    rx_packets: 0,
                    dropped_packets: 0,
                };
                if let Some(existing) = sockets.iter_mut().find(|entry| entry.path == socket_path) {
                    existing.device_path = device_path.to_string();
                    existing.record = record;
                    existing.pending_rx_payload.clear();
                    existing.pending_rx_ipv4 = [0; 4];
                    existing.pending_rx_port = 0;
                } else {
                    sockets.push(RecordedNetworkSocketState {
                        path: socket_path.to_string(),
                        device_path: device_path.to_string(),
                        record,
                        pending_rx_payload: Vec::new(),
                        pending_rx_ipv4: [0; 4],
                        pending_rx_port: 0,
                    });
                }
                drop(sockets);
                self.recount_attached_sockets(device_path);
                SyscallReturn::ok(0)
            }
            SYS_CONNECT_UDP_SOCKET => {
                let socket_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let config = unsafe {
                    (frame.arg2 as *const ngos_user_abi::NativeUdpConnectConfig).read_unaligned()
                };
                let mut sockets = self.network_sockets.borrow_mut();
                let Some(socket) = sockets.iter_mut().find(|entry| entry.path == socket_path)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                socket.record.remote_ipv4 = config.remote_ipv4;
                socket.record.remote_port = config.remote_port;
                socket.record.connected = 1;
                SyscallReturn::ok(0)
            }
            SYS_SENDTO_UDP_SOCKET => {
                let socket_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let config = unsafe {
                    (frame.arg2 as *const ngos_user_abi::NativeUdpSendToConfig).read_unaligned()
                };
                let payload =
                    unsafe { core::slice::from_raw_parts(frame.arg3 as *const u8, frame.arg4) };
                let device_path = {
                    let mut sockets = self.network_sockets.borrow_mut();
                    let Some(socket) = sockets.iter_mut().find(|entry| entry.path == socket_path)
                    else {
                        return SyscallReturn::err(Errno::NoEnt);
                    };
                    socket.record.remote_ipv4 = config.remote_ipv4;
                    socket.record.remote_port = config.remote_port;
                    socket.record.connected = 1;
                    socket.record.tx_packets = socket.record.tx_packets.saturating_add(1);
                    socket.device_path.clone()
                };
                let Ok(index) = self.ensure_network_interface_state(&device_path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                {
                    let interfaces = self.network_interfaces.borrow();
                    let record = interfaces[index].record;
                    if record.admin_up == 0 || record.link_up == 0 {
                        return SyscallReturn::err(Errno::Access);
                    }
                }
                {
                    let mut interfaces = self.network_interfaces.borrow_mut();
                    interfaces[index].record.tx_packets =
                        interfaces[index].record.tx_packets.saturating_add(1);
                }
                let request = format!(
                    "tx path={} remote={}:{} bytes={}\n{}",
                    socket_path,
                    render_ipv4(config.remote_ipv4),
                    config.remote_port,
                    payload.len(),
                    core::str::from_utf8(payload).unwrap_or_default()
                )
                .into_bytes();
                let Some(driver_path) = Self::network_driver_path_for_device(&device_path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                if let Some((_, bytes)) = self
                    .file_contents
                    .borrow_mut()
                    .iter_mut()
                    .find(|(path, _)| path == driver_path)
                {
                    *bytes = request;
                } else {
                    self.file_contents
                        .borrow_mut()
                        .push((String::from(driver_path), request));
                }
                SyscallReturn::ok(payload.len())
            }
            SYS_RECVFROM_UDP_SOCKET => {
                let socket_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let mut payload = b"hello-qemu".to_vec();
                let (remote_ipv4, remote_port, device_path) = {
                    let mut sockets = self.network_sockets.borrow_mut();
                    let Some(socket) = sockets.iter_mut().find(|entry| entry.path == socket_path)
                    else {
                        return SyscallReturn::err(Errno::NoEnt);
                    };
                    socket.record.rx_packets = socket.record.rx_packets.saturating_add(1);
                    if !socket.pending_rx_payload.is_empty() {
                        payload = core::mem::take(&mut socket.pending_rx_payload);
                    }
                    (
                        if socket.pending_rx_port != 0 {
                            socket.pending_rx_ipv4
                        } else {
                            socket.record.remote_ipv4
                        },
                        if socket.pending_rx_port != 0 {
                            socket.pending_rx_port
                        } else {
                            socket.record.remote_port
                        },
                        socket.device_path.clone(),
                    )
                };
                let buffer_len = frame.arg3.min(payload.len());
                if let Ok(index) = self.ensure_network_interface_state(&device_path) {
                    let mut interfaces = self.network_interfaces.borrow_mut();
                    interfaces[index].record.rx_packets =
                        interfaces[index].record.rx_packets.saturating_add(1);
                    interfaces[index].record.rx_ring_depth =
                        interfaces[index].record.rx_ring_depth.saturating_sub(1);
                }
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        payload.as_ptr(),
                        frame.arg2 as *mut u8,
                        buffer_len,
                    );
                    (frame.arg4 as *mut ngos_user_abi::NativeUdpRecvMeta).write(
                        ngos_user_abi::NativeUdpRecvMeta {
                            remote_ipv4,
                            remote_port,
                            reserved: 0,
                        },
                    );
                }
                SyscallReturn::ok(buffer_len)
            }
            SYS_COMPLETE_NET_TX => {
                let driver_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Some(device_path) = Self::network_device_path_for_driver(driver_path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                if let Some((_, bytes)) = self
                    .file_contents
                    .borrow_mut()
                    .iter_mut()
                    .find(|(path, _)| path == driver_path)
                {
                    bytes.clear();
                }
                if let Ok(index) = self.ensure_network_interface_state(device_path) {
                    let mut interfaces = self.network_interfaces.borrow_mut();
                    interfaces[index].record.tx_completions = interfaces[index]
                        .record
                        .tx_completions
                        .saturating_add(frame.arg2 as u64);
                }
                self.emit_network_event(
                    device_path,
                    Self::network_socket_path_for_device(device_path),
                    NativeNetworkEventKind::TxDrained,
                );
                SyscallReturn::ok(frame.arg2)
            }
            SYS_INSPECT_NETIF => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Ok(index) = self.ensure_network_interface_state(path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let ptr = frame.arg2 as *mut NativeNetworkInterfaceRecord;
                unsafe {
                    ptr.write(self.network_interfaces.borrow()[index].record);
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_NETSOCK => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let ptr = frame.arg2 as *mut NativeNetworkSocketRecord;
                let sockets = self.network_sockets.borrow();
                let Some(socket) = sockets.iter().find(|entry| entry.path == path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                unsafe {
                    ptr.write(socket.record);
                }
                SyscallReturn::ok(0)
            }
            SYS_CHDIR_PATH
            | ngos_user_abi::SYS_BIND_PROCESS_CONTRACT
            | SYS_PAUSE_PROCESS
            | SYS_RESUME_PROCESS
            | SYS_RENICE_PROCESS
            | SYS_SET_PROCESS_AFFINITY
            | SYS_WATCH_GRAPHICS_EVENTS
            | SYS_REMOVE_PROCESS_EVENTS
            | SYS_REMOVE_GRAPHICS_EVENTS => SyscallReturn::ok(0),
            SYS_WATCH_PROCESS_EVENTS => {
                let config = unsafe {
                    (frame.arg2 as *const ngos_user_abi::NativeProcessEventWatchConfig)
                        .read_unaligned()
                };
                self.push_event_queue_record(
                    frame.arg0,
                    NativeEventRecord {
                        token: config.token,
                        events: config.poll_events,
                        source_kind: NativeEventSourceKind::Process as u32,
                        source_arg0: frame.arg1 as u64,
                        source_arg1: 0,
                        source_arg2: 0,
                        detail0: 1,
                        detail1: 0,
                    },
                );
                SyscallReturn::ok(0)
            }
            SYS_WATCH_BUS_EVENTS => {
                let config =
                    unsafe { (frame.arg2 as *const NativeBusEventWatchConfig).read_unaligned() };
                self.register_bus_event_watch(frame.arg0, frame.arg1 as u64, config);
                self.push_event_queue_record(
                    frame.arg0,
                    NativeEventRecord {
                        token: config.token,
                        events: config.poll_events,
                        source_kind: NativeEventSourceKind::Bus as u32,
                        source_arg0: 11,
                        source_arg1: frame.arg1 as u64,
                        source_arg2: 0,
                        detail0: 0,
                        detail1: 0,
                    },
                );
                SyscallReturn::ok(0)
            }
            SYS_WATCH_NET_EVENTS => {
                let interface_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg1 as *const u8,
                        frame.arg2,
                    ))
                };
                let socket_path = if frame.arg3 == 0 || frame.arg4 == 0 {
                    None
                } else {
                    Some(unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg3 as *const u8,
                            frame.arg4,
                        ))
                    })
                };
                let config = unsafe {
                    (frame.arg5 as *const NativeNetworkEventWatchConfig).read_unaligned()
                };
                self.register_network_event_watch(frame.arg0, interface_path, socket_path, config);
                self.push_event_queue_record(
                    frame.arg0,
                    NativeEventRecord {
                        token: config.token,
                        events: config.poll_events,
                        source_kind: NativeEventSourceKind::Network as u32,
                        source_arg0: 99,
                        source_arg1: 0,
                        source_arg2: 0,
                        detail0: 1,
                        detail1: NativeNetworkEventKind::LinkChanged as u32,
                    },
                );
                SyscallReturn::ok(0)
            }
            SYS_REMOVE_NET_EVENTS => {
                let interface_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg1 as *const u8,
                        frame.arg2,
                    ))
                };
                let socket_path = if frame.arg3 == 0 || frame.arg4 == 0 {
                    None
                } else {
                    Some(unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg3 as *const u8,
                            frame.arg4,
                        ))
                    })
                };
                match self.remove_network_event_watch(
                    frame.arg0,
                    interface_path,
                    socket_path,
                    frame.arg5 as u64,
                ) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_REMOVE_BUS_EVENTS => {
                match self.remove_bus_event_watch(frame.arg0, frame.arg1 as u64, frame.arg2 as u64)
                {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_WATCH_RESOURCE_EVENTS => {
                let config = unsafe {
                    (frame.arg2 as *const NativeResourceEventWatchConfig).read_unaligned()
                };
                self.register_resource_event_watch(frame.arg0, frame.arg1 as u64, config);
                SyscallReturn::ok(0)
            }
            SYS_REMOVE_RESOURCE_EVENTS => {
                match self.remove_resource_event_watch(
                    frame.arg0,
                    frame.arg1 as u64,
                    frame.arg2 as u64,
                ) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_RECLAIM_MEMORY_PRESSURE => {
                if self.memory_policy_blocked() {
                    self.emit_vm_policy_block(frame.arg0 as u64);
                    return SyscallReturn::err(Errno::Access);
                }
                let reclaimed =
                    self.reclaim_vm_pressure(Some(frame.arg0 as u64), frame.arg1 as u64);
                SyscallReturn::ok(reclaimed as usize)
            }
            SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL => {
                if self.memory_policy_blocked() {
                    self.emit_vm_policy_block(1);
                    return SyscallReturn::err(Errno::Access);
                }
                let reclaimed = self.reclaim_vm_pressure(None, frame.arg0 as u64);
                SyscallReturn::ok(reclaimed as usize)
            }
            SYS_MAP_ANONYMOUS_MEMORY => {
                let pid = frame.arg0 as u64;
                if self.memory_policy_blocked() {
                    self.emit_vm_policy_block(pid);
                    return SyscallReturn::err(Errno::Access);
                }
                let len = frame.arg1 as u64;
                let perms = frame.arg2;
                let label = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg3 as *const u8,
                        frame.arg4,
                    ))
                };
                let start = self.alloc_vm_addr(len);
                self.vm_mappings.borrow_mut().push(VmMappingRecord {
                    pid,
                    start,
                    len,
                    readable: (perms & 1) != 0,
                    writable: (perms & 2) != 0,
                    executable: (perms & 4) != 0,
                    label: label.to_string(),
                    file_path: None,
                    file_inode: None,
                    file_offset: 0,
                    private: true,
                    cow: false,
                    present: true,
                    reclaimed: false,
                    quarantined: false,
                    quarantine_reason: 0,
                    orphan_bytes: Vec::new(),
                    words: Vec::new(),
                });
                self.push_vm_decision(
                    pid,
                    format!("agent=map pid={pid} start={start} len={len} label={label}"),
                );
                SyscallReturn::ok(start as usize)
            }
            SYS_LOAD_MEMORY_WORD => match self.load_vm_word(frame.arg0 as u64, frame.arg1 as u64) {
                Ok((value, restored)) => {
                    if restored {
                        self.push_vm_episode(
                            frame.arg0 as u64,
                            format!(
                                "kind=reclaim pid={} evicted=yes restored=yes",
                                frame.arg0 as u64
                            ),
                        );
                    }
                    SyscallReturn::ok(value as usize)
                }
                Err(errno) => SyscallReturn::err(errno),
            },
            SYS_STORE_MEMORY_WORD => {
                let pid = frame.arg0 as u64;
                let addr = frame.arg1 as u64;
                let value = frame.arg2 as u32;
                match self.store_vm_word(pid, addr, value) {
                    Ok(was_cow) => {
                        if was_cow {
                            self.push_vm_episode(
                                pid,
                                format!("kind=fault pid={pid} addr={addr} cow=yes"),
                            );
                        }
                        SyscallReturn::ok(0)
                    }
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_QUARANTINE_VM_OBJECT => {
                match self.quarantine_vm_object(frame.arg0 as u64, frame.arg1 as u64, frame.arg2 as u64)
                {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_RELEASE_VM_OBJECT => {
                match self.release_vm_object(frame.arg0 as u64, frame.arg1 as u64) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_SYNC_MEMORY_RANGE => {
                let pid = frame.arg0 as u64;
                match self.flush_file_mapping_range(pid, frame.arg1 as u64, frame.arg2 as u64) {
                    Ok(()) => {
                        self.push_vm_decision(
                            pid,
                            format!(
                                "agent=sync pid={pid} start={} len={}",
                                frame.arg1, frame.arg2
                            ),
                        );
                        SyscallReturn::ok(0)
                    }
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_PROTECT_MEMORY_RANGE => {
                let pid = frame.arg0 as u64;
                let start = frame.arg1 as u64;
                let len = frame.arg2 as u64;
                let readable = frame.arg3 != 0;
                let writable = frame.arg4 != 0;
                let executable = frame.arg5 != 0;
                match self.protect_vm_range(pid, start, len, readable, writable, executable) {
                    Ok(()) => {
                        self.push_vm_decision(
                            pid,
                            format!("agent=protect pid={pid} start={start} len={len}"),
                        );
                        self.push_vm_episode(
                            pid,
                            format!("kind=region pid={pid} protected=yes unmapped=no"),
                        );
                        SyscallReturn::ok(0)
                    }
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_ADVISE_MEMORY_RANGE => {
                let pid = frame.arg0 as u64;
                let start = frame.arg1 as u64;
                let len = frame.arg2 as u64;
                let advice = frame.arg3 as u32;
                match self.advise_vm_range(pid, start, len, advice) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_UNMAP_MEMORY_RANGE => {
                let pid = frame.arg0 as u64;
                let start = frame.arg1 as u64;
                let len = frame.arg2 as u64;
                match self.unmap_vm_range(pid, start, len) {
                    Ok(()) => {
                        self.push_vm_decision(
                            pid,
                            format!("agent=unmap pid={pid} start={start} len={len}"),
                        );
                        self.push_vm_episode(
                            pid,
                            format!("kind=region pid={pid} protected=no unmapped=yes"),
                        );
                        SyscallReturn::ok(0)
                    }
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_SET_PROCESS_BREAK => {
                let pid = frame.arg0 as u64;
                let requested_end = frame.arg1 as u64;
                self.ensure_heap_mapping(pid);
                let mut mappings = self.vm_mappings.borrow_mut();
                let Some(mapping) = mappings.iter_mut().find(|mapping| {
                    mapping.pid == pid && mapping.present && mapping.label == "[heap]"
                }) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                if requested_end <= mapping.start {
                    return SyscallReturn::err(Errno::Inval);
                }
                let old_end = mapping.start.saturating_add(mapping.len);
                mapping.len = requested_end
                    .saturating_sub(mapping.start)
                    .max(0x1000)
                    .next_multiple_of(0x1000);
                let new_end = mapping.start.saturating_add(mapping.len);
                let grew = new_end > old_end;
                let shrank = new_end < old_end;
                drop(mappings);
                self.push_vm_decision(
                    pid,
                    format!("agent=brk pid={pid} old-end={old_end} new-end={new_end}"),
                );
                self.push_vm_episode(
                    pid,
                    format!(
                        "kind=heap pid={pid} grew={} shrank={} old-end={} new-end={}",
                        if grew { "yes" } else { "no" },
                        if shrank { "yes" } else { "no" },
                        old_end,
                        new_end,
                    ),
                );
                SyscallReturn::ok(new_end as usize)
            }
            SYS_MAP_FILE_MEMORY => {
                let pid = frame.arg0 as u64;
                if self.memory_policy_blocked() {
                    self.emit_vm_policy_block(pid);
                    return SyscallReturn::err(Errno::Access);
                }
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg1 as *const u8,
                        frame.arg2,
                    ))
                };
                let path = match Self::normalize_absolute_path(path) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                if !self.path_exists(&path) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if self.created_kind(&path) != Some(NativeObjectKind::File) {
                    return SyscallReturn::err(Errno::Inval);
                }
                let len = frame.arg3 as u64;
                let file_offset = frame.arg4 as u64;
                let perms = frame.arg5;
                if self
                    .require_access(
                        &path,
                        (perms & 0x1) != 0 || (perms & 0x4) != 0,
                        (perms & 0x2) != 0,
                        false,
                    )
                    .is_err()
                {
                    return SyscallReturn::err(Errno::Perm);
                }
                let start = self.alloc_vm_addr(len);
                let inode = self.inode_for_path(&path);
                let backing = self.file_content(&path);
                self.vm_mappings.borrow_mut().push(VmMappingRecord {
                    pid,
                    start,
                    len,
                    readable: (perms & 1) != 0,
                    writable: (perms & 2) != 0,
                    executable: (perms & 4) != 0,
                    label: path.to_string(),
                    file_path: Some(path.to_string()),
                    file_inode: Some(inode),
                    file_offset,
                    private: true,
                    cow: true,
                    present: true,
                    reclaimed: false,
                    quarantined: false,
                    quarantine_reason: 0,
                    orphan_bytes: backing,
                    words: Vec::new(),
                });
                self.push_vm_decision(
                    pid,
                    format!(
                        "agent=map-file pid={pid} start={start} len={len} path={path} inode={inode} offset={file_offset}"
                    ),
                );
                SyscallReturn::ok(start as usize)
            }
            SYS_SPAWN_PROCESS_COPY_VM => {
                let source_pid = frame.arg4 as u64;
                let child_pid = if self
                    .vm_mappings
                    .borrow()
                    .iter()
                    .any(|mapping| mapping.pid == 2)
                {
                    3u64
                } else {
                    2u64
                };
                let clones = self
                    .vm_mappings
                    .borrow()
                    .iter()
                    .filter(|mapping| mapping.pid == source_pid && mapping.present)
                    .cloned()
                    .map(|mut mapping| {
                        mapping.pid = child_pid;
                        mapping.cow = true;
                        mapping
                    })
                    .collect::<Vec<_>>();
                self.vm_mappings.borrow_mut().extend(clones);
                self.push_vm_decision(
                    child_pid,
                    format!("agent=shadow-reuse pid={child_pid} source-pid={source_pid}"),
                );
                self.push_vm_decision(
                    child_pid,
                    format!("agent=cow-populate pid={child_pid} source-pid={source_pid}"),
                );
                self.push_vm_episode(
                    child_pid,
                    format!("kind=fault pid={child_pid} source_pid={source_pid} cow=yes"),
                );
                SyscallReturn::ok(child_pid as usize)
            }
            SYS_CREATE_EVENT_QUEUE => {
                let fd = self.next_fd.get();
                self.next_fd.set(fd + 1);
                self.event_queue_pending.borrow_mut().push((fd, Vec::new()));
                let mode = match frame.arg0 as u32 {
                    1 => NativeEventQueueMode::Kqueue,
                    _ => NativeEventQueueMode::Epoll,
                };
                self.event_queue_modes.borrow_mut().push((fd, mode));
                self.event_queue_nonblock.borrow_mut().push((fd, false));
                SyscallReturn::ok(fd)
            }
            SYS_WAIT_EVENT_QUEUE => match self.take_event_queue_records(frame.arg0, frame.arg2) {
                Ok(records) => {
                    let ptr = frame.arg1 as *mut NativeEventRecord;
                    for (index, record) in records.iter().enumerate() {
                        unsafe {
                            ptr.add(index).write(*record);
                        }
                    }
                    SyscallReturn::ok(records.len())
                }
                Err(errno) => SyscallReturn::err(errno),
            },
            SYS_SEND_SIGNAL => SyscallReturn::ok(0),
            SYS_PENDING_SIGNALS => {
                let ptr = frame.arg1 as *mut u8;
                unsafe {
                    *ptr = 9;
                }
                SyscallReturn::ok(1)
            }
            SYS_BLOCKED_PENDING_SIGNALS => SyscallReturn::ok(0),
            SYS_SPAWN_PATH_PROCESS => {
                let name = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                let pid = self.next_pid.get();
                self.next_pid.set(pid + 1);
                self.record_spawned_process(pid, name, path);
                SyscallReturn::ok(pid as usize)
            }
            SYS_SPAWN_CONFIGURED_PROCESS => {
                let config = unsafe {
                    (frame.arg0 as *const ngos_user_abi::NativeSpawnProcessConfig).read_unaligned()
                };
                let name = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        config.name_ptr as *const u8,
                        config.name_len,
                    ))
                };
                let name = match name {
                    Ok(name) => name,
                    Err(_) => return SyscallReturn::err(Errno::Inval),
                };
                let path = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        config.path_ptr as *const u8,
                        config.path_len,
                    ))
                };
                let path = match path {
                    Ok(path) => path,
                    Err(_) => return SyscallReturn::err(Errno::Inval),
                };
                let cwd = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        config.cwd_ptr as *const u8,
                        config.cwd_len,
                    ))
                };
                let cwd = match cwd {
                    Ok(cwd) => cwd.to_string(),
                    Err(_) => return SyscallReturn::err(Errno::Inval),
                };
                let argv = match self.decode_string_table(
                    config.argv_ptr,
                    config.argv_len,
                    config.argv_count,
                ) {
                    Ok(argv) => argv,
                    Err(error) => return SyscallReturn::err(error),
                };
                let envp = match self.decode_string_table(
                    config.envp_ptr,
                    config.envp_len,
                    config.envp_count,
                ) {
                    Ok(envp) => envp,
                    Err(error) => return SyscallReturn::err(error),
                };
                let pid = self.next_pid.get();
                self.next_pid.set(pid + 1);
                self.record_spawned_process(pid, name, path);
                if self
                    .with_recorded_process_mut(pid, |process| {
                        process.cwd = cwd;
                        process.argv = argv;
                        process.envp = envp;
                    })
                    .is_err()
                {
                    return SyscallReturn::err(Errno::Srch);
                }
                if self.recorded_process(pid).is_some_and(|process| {
                    process.argv.iter().any(|arg| arg == "--compat-proc-probe")
                }) {
                    self.emit_compat_proc_probe_output(pid);
                }
                SyscallReturn::ok(pid as usize)
            }
            SYS_SET_PROCESS_ARGS => {
                let pid = frame.arg0 as u64;
                let argv = match self.decode_string_table(frame.arg1, frame.arg2, frame.arg3) {
                    Ok(argv) => argv,
                    Err(error) => return SyscallReturn::err(error),
                };
                if self
                    .with_recorded_process_mut(pid, |process| process.argv = argv)
                    .is_err()
                {
                    return SyscallReturn::err(Errno::Srch);
                }
                SyscallReturn::ok(0)
            }
            SYS_SET_PROCESS_ENV => {
                let pid = frame.arg0 as u64;
                let envp = match self.decode_string_table(frame.arg1, frame.arg2, frame.arg3) {
                    Ok(envp) => envp,
                    Err(error) => return SyscallReturn::err(error),
                };
                if self
                    .with_recorded_process_mut(pid, |process| process.envp = envp)
                    .is_err()
                {
                    return SyscallReturn::err(Errno::Srch);
                }
                SyscallReturn::ok(0)
            }
            SYS_SET_PROCESS_CWD => {
                let pid = frame.arg0 as u64;
                let cwd = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        frame.arg1 as *const u8,
                        frame.arg2,
                    ))
                };
                let cwd = match cwd {
                    Ok(cwd) => cwd.to_string(),
                    Err(_) => return SyscallReturn::err(Errno::Inval),
                };
                if self
                    .with_recorded_process_mut(pid, |process| process.cwd = cwd)
                    .is_err()
                {
                    return SyscallReturn::err(Errno::Srch);
                }
                SyscallReturn::ok(0)
            }
            SYS_PREPARE_STORAGE_COMMIT => {
                let device_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let tag = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                let payload = if frame.arg5 == 0 {
                    &[][..]
                } else {
                    unsafe { core::slice::from_raw_parts(frame.arg4 as *const u8, frame.arg5) }
                };
                match self.prepare_storage_commit_local(device_path, tag, payload) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_RECOVER_STORAGE_VOLUME => {
                let device_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                match self.recover_storage_volume_local(device_path) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_REPAIR_STORAGE_SNAPSHOT => {
                let device_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                match self.repair_storage_snapshot_local(device_path) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_INSPECT_STORAGE_VOLUME => {
                let device_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                match self.inspect_storage_volume_local(device_path) {
                    Ok(record) => {
                        unsafe {
                            (frame.arg2 as *mut NativeStorageVolumeRecord).write(record);
                        }
                        SyscallReturn::ok(0)
                    }
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_INSPECT_STORAGE_LINEAGE => {
                let device_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                match self.inspect_storage_lineage_local(device_path) {
                    Ok(record) => {
                        unsafe {
                            (frame.arg2 as *mut NativeStorageLineageRecord).write(record);
                        }
                        SyscallReturn::ok(0)
                    }
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_REAP_PROCESS => {
                let pid = frame.arg0 as u64;
                if self.recorded_process(pid).is_some_and(|process| {
                    process.argv.iter().any(|arg| arg == "--compat-proc-probe")
                }) {
                    SyscallReturn::ok(0)
                } else {
                    SyscallReturn::ok(137)
                }
            }
            SYS_READ_PROCFS => {
                let path =
                    unsafe { core::slice::from_raw_parts(frame.arg0 as *const u8, frame.arg1) };
                let path = match core::str::from_utf8(path) {
                    Ok(path) => path,
                    Err(_) => return SyscallReturn::err(Errno::Inval),
                };
                if path == "/proc/system/bus"
                    && !self
                        .frames
                        .borrow()
                        .iter()
                        .any(|record| record.number == ngos_user_abi::SYS_BIND_PROCESS_CONTRACT)
                {
                    return SyscallReturn::err(Errno::Access);
                }
                let payload = if let Some(payload) = self.recorded_procfs_payload(path) {
                    payload
                } else {
                    match path {
                        "/proc/system/bus" => self.render_bus_procfs(),
                        "/proc/1/status" => {
                            b"Name:\tngos-userland-native\nState:\tRunning\nPid:\t1\nCwd:\t/\n"
                                .to_vec()
                        }
                        "/proc/1/cwd" => b"/".to_vec(),
                        "/proc/1/exe" => b"/bin/ngos-userland-native".to_vec(),
                        "/proc/1/root" => b"/".to_vec(),
                        "/proc/1/auxv" => self.procfs_auxv_listing(1).into_bytes(),
                        "/proc/1/mounts" => self.procfs_mount_listing(1).into_bytes(),
                        "/proc/1/caps" => self.procfs_cap_listing(1).into_bytes(),
                        "/proc/1/fd" => self.procfs_fd_listing().into_bytes(),
                        "/proc/1/queues" => self.procfs_queue_listing(1).into_bytes(),
                        "/proc/1/vfslocks" => Vec::new(),
                        "/proc/1/vfswatches" => Vec::new(),
                        "/proc/1/vfsstats" => {
                            b"live: nodes=1 orphans=0 locks=0 watches=0 mounts=1\n".to_vec()
                        }
                        "/proc/1/maps" => self.render_proc_maps(1),
                        "/proc/1/vmdecisions" => self.render_vm_decisions(1),
                        "/proc/1/vmobjects" => self.render_vm_objects(1),
                        "/proc/1/vmepisodes" => self.render_vm_episodes(1),
                        "/proc/2/vmdecisions" => self.render_vm_decisions(2),
                        "/proc/2/vmobjects" => self.render_vm_objects(2),
                        "/proc/2/vmepisodes" => self.render_vm_episodes(2),
                        "/proc/3/vmdecisions" => self.render_vm_decisions(3),
                        "/proc/3/vmobjects" => self.render_vm_objects(3),
                        "/proc/3/vmepisodes" => self.render_vm_episodes(3),
                        "/proc/1/cmdline" => b"ngos-userland-native\0".to_vec(),
                        "/proc/1/environ" => {
                            b"NGOS_SESSION=1\0NGOS_SESSION_PROTOCOL=kernel-launch\0".to_vec()
                        }
                        _ if path.starts_with("/proc/1/fdinfo/") => {
                            let Some(fd) = path
                                .strip_prefix("/proc/1/fdinfo/")
                                .and_then(|value| value.parse::<u64>().ok())
                            else {
                                return SyscallReturn::err(Errno::Inval);
                            };
                            let Some(payload) = self.procfs_fdinfo_payload(fd) else {
                                return SyscallReturn::err(Errno::NoEnt);
                            };
                            payload
                        }
                        _ => return SyscallReturn::err(Errno::NoEnt),
                    }
                };
                if frame.arg3 == 0 {
                    return SyscallReturn::ok(payload.len());
                }
                let count = payload.len().min(frame.arg3);
                unsafe {
                    core::ptr::copy_nonoverlapping(payload.as_ptr(), frame.arg2 as *mut u8, count);
                }
                SyscallReturn::ok(count)
            }
            SYS_OPEN_PATH => {
                let path =
                    unsafe { core::slice::from_raw_parts(frame.arg0 as *const u8, frame.arg1) };
                let path = match core::str::from_utf8(path) {
                    Ok(path) => path,
                    Err(_) => return SyscallReturn::err(Errno::Inval),
                };
                let resolved_path = match self.resolve_open_path(path, 0) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                if !self.path_exists(&resolved_path)
                    && resolved_path != "/proc/1/status"
                    && resolved_path != "/proc/1/cwd"
                    && !resolved_path.starts_with("/dev/")
                    && !resolved_path.starts_with("/drv/")
                {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if !resolved_path.starts_with("/proc/")
                    && !resolved_path.starts_with("/dev/")
                    && !resolved_path.starts_with("/drv/")
                {
                    if self
                        .require_traversal_access(&resolved_path, false)
                        .is_err()
                        || self
                            .require_access(&resolved_path, true, false, false)
                            .is_err()
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                }
                let fd = self.next_fd.get();
                self.next_fd.set(fd + 1);
                let description_id = self.next_description_id.get();
                self.next_description_id.set(description_id + 1);
                let kind = self.created_kind(&resolved_path).unwrap_or_else(|| {
                    if resolved_path.starts_with("/dev/") || resolved_path.starts_with("/drv/") {
                        NativeObjectKind::Device
                    } else {
                        NativeObjectKind::File
                    }
                });
                let inode = self.inode_for_path(&resolved_path);
                self.open_files.borrow_mut().push(OpenFileRecord {
                    fd,
                    description_id,
                    path: resolved_path,
                    inode,
                    kind,
                    deleted: false,
                    orphan_bytes: Vec::new(),
                });
                self.fd_flags.borrow_mut().push((fd, false, false));
                self.description_nonblock
                    .borrow_mut()
                    .push((description_id, false));
                self.set_read_offset(fd, 0);
                SyscallReturn::ok(fd)
            }
            SYS_MKDIR_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let path = match Self::normalize_absolute_path(path) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                if path == "/" || self.path_exists(&path) {
                    return SyscallReturn::err(Errno::Exist);
                }
                if !self.parent_directory_exists(&path) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if let Err(errno) = self.require_parent_mutation_access(&path) {
                    return SyscallReturn::err(errno);
                }
                self.record_created_path(&path, NativeObjectKind::Directory);
                SyscallReturn::ok(0)
            }
            SYS_MKFILE_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let path = match Self::normalize_absolute_path(path) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                if path == "/" || self.path_exists(&path) {
                    return SyscallReturn::err(Errno::Exist);
                }
                if !self.parent_directory_exists(&path) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if let Err(errno) = self.require_parent_mutation_access(&path) {
                    return SyscallReturn::err(errno);
                }
                self.record_created_path(&path, NativeObjectKind::File);
                SyscallReturn::ok(0)
            }
            SYS_MKSOCK_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Ok(path) = Self::normalize_absolute_path(path) else {
                    return SyscallReturn::err(Errno::Inval);
                };
                if self.path_exists(&path) {
                    return SyscallReturn::err(Errno::Exist);
                }
                if self.require_traversal_access(&path, false).is_err()
                    || self.require_parent_mutation_access(&path).is_err()
                {
                    return SyscallReturn::err(Errno::Access);
                }
                self.record_created_path(&path, NativeObjectKind::Socket);
                SyscallReturn::ok(0)
            }
            SYS_MKCHAN_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let path = match Self::normalize_absolute_path(path) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                if path == "/" || self.path_exists(&path) {
                    return SyscallReturn::err(Errno::Exist);
                }
                if !self.parent_directory_exists(&path) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if let Err(errno) = self.require_parent_mutation_access(&path) {
                    return SyscallReturn::err(errno);
                }
                self.record_created_path(&path, NativeObjectKind::Channel);
                SyscallReturn::ok(0)
            }
            SYS_SYMLINK_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let target = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                let path = match Self::normalize_absolute_path(path) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                if path == "/" || self.path_exists(&path) {
                    return SyscallReturn::err(Errno::Exist);
                }
                if !self.parent_directory_exists(&path) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if let Err(errno) = self.require_parent_mutation_access(&path) {
                    return SyscallReturn::err(errno);
                }
                self.record_symlink_path(&path, target);
                SyscallReturn::ok(0)
            }
            SYS_CHMOD_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                if !self.path_exists(path) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let (owner_uid, group_gid, _) = self.metadata_for_path(path);
                let (uid, _) = self.current_subject();
                if uid != 0 && uid != owner_uid {
                    return SyscallReturn::err(Errno::Perm);
                }
                self.set_path_metadata(path, owner_uid, group_gid, frame.arg2 as u32);
                SyscallReturn::ok(0)
            }
            SYS_CHOWN_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                if !self.path_exists(path) {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if self.current_subject().0 != 0 {
                    return SyscallReturn::err(Errno::Perm);
                }
                let (_, _, mode) = self.metadata_for_path(path);
                self.set_path_metadata(path, frame.arg2 as u32, frame.arg3 as u32, mode);
                SyscallReturn::ok(0)
            }
            SYS_RENAME_PATH => {
                let from = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let to = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                match self.rename_path(from, to) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_LINK_PATH => {
                let from = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let to = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                match self.link_path(from, to) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_UNLINK_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                match self.unlink_path(path) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_TRUNCATE_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                match self.truncate_file_content(path, frame.arg2) {
                    Ok(()) => SyscallReturn::ok(0),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_MOUNT_STORAGE_VOLUME => {
                let device_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let mount_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg2 as *const u8,
                        frame.arg3,
                    ))
                };
                match self.mount_storage_volume_local(device_path, mount_path) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_UNMOUNT_STORAGE_VOLUME => {
                let mount_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                match self.unmount_storage_volume_local(mount_path) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                }
            }
            SYS_INSPECT_MOUNT => {
                let mount_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Some(record) = self.mount_by_path(mount_path) else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                unsafe {
                    (frame.arg2 as *mut ngos_user_abi::NativeMountRecord).write(
                        ngos_user_abi::NativeMountRecord {
                            id: record.id,
                            parent_mount_id: record.parent_mount_id,
                            peer_group: record.peer_group,
                            master_group: record.master_group,
                            layer: record.id,
                            entry_count: self.mount_entry_count(mount_path) as u64,
                            propagation_mode: record.propagation_mode,
                            created_mount_root: u32::from(record.created_mount_root),
                        },
                    );
                }
                SyscallReturn::ok(0)
            }
            SYS_SET_MOUNT_PROPAGATION => {
                let mount_path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Some(mode) = NativeMountPropagationMode::from_raw(frame.arg2 as u32) else {
                    return SyscallReturn::err(Errno::Inval);
                };
                let Some(index) = self
                    .mounts
                    .borrow()
                    .iter()
                    .position(|record| record.mount_path == mount_path)
                else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                let (record_id, device_path, parent_mount_id) = {
                    let mounts = self.mounts.borrow();
                    let record = &mounts[index];
                    (
                        record.id,
                        record.device_path.clone(),
                        record.parent_mount_id,
                    )
                };
                let shared_group_candidate = self
                    .mounts
                    .borrow()
                    .iter()
                    .find(|candidate| {
                        candidate.id != record_id
                            && candidate.device_path == device_path
                            && candidate.parent_mount_id == parent_mount_id
                            && candidate.propagation_mode
                                == NativeMountPropagationMode::Shared as u32
                            && candidate.peer_group != 0
                    })
                    .map(|candidate| candidate.peer_group);
                let mut mounts = self.mounts.borrow_mut();
                let record = &mut mounts[index];
                match mode {
                    NativeMountPropagationMode::Private => {
                        record.propagation_mode = NativeMountPropagationMode::Private as u32;
                        record.peer_group = 0;
                        record.master_group = 0;
                    }
                    NativeMountPropagationMode::Shared => {
                        let group = shared_group_candidate.unwrap_or(record.id);
                        record.propagation_mode = NativeMountPropagationMode::Shared as u32;
                        record.peer_group = group;
                        record.master_group = 0;
                    }
                    NativeMountPropagationMode::Slave => {
                        let Some(group) = shared_group_candidate else {
                            return SyscallReturn::err(Errno::NoEnt);
                        };
                        record.propagation_mode = NativeMountPropagationMode::Slave as u32;
                        record.peer_group = 0;
                        record.master_group = group;
                    }
                }
                SyscallReturn::ok(0)
            }
            SYS_LIST_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let path = match Self::normalize_absolute_path(path) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                if path != "/proc"
                    && !path.starts_with("/proc/")
                    && path != "/dev"
                    && !path.starts_with("/dev/")
                    && path != "/drv"
                    && !path.starts_with("/drv/")
                {
                    if !self.path_exists(&path) {
                        return SyscallReturn::err(Errno::NoEnt);
                    }
                    if self.require_traversal_access(&path, true).is_err()
                        || self.require_access(&path, true, false, true).is_err()
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                }
                let ptr = frame.arg2 as *mut u8;
                let payload = self.list_path_payload(&path);
                let count = payload.len().min(frame.arg3);
                unsafe {
                    core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                }
                SyscallReturn::ok(count)
            }
            SYS_READLINK_PATH => {
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let path = match Self::normalize_absolute_path(path) {
                    Ok(path) => path,
                    Err(errno) => return SyscallReturn::err(errno),
                };
                let payload = if let Some((pid, section)) = Self::parse_proc_pid_section(&path) {
                    let process = if pid == 1 {
                        self.recorded_process(pid)
                            .unwrap_or(RecordedProcessBootstrap {
                                pid,
                                name: String::from("ngos-userland-native"),
                                image_path: String::from("/bin/ngos-userland-native"),
                                cwd: String::from("/"),
                                argv: vec![String::from("ngos-userland-native")],
                                envp: Vec::new(),
                            })
                    } else if let Some(process) = self.recorded_process(pid) {
                        process
                    } else {
                        return SyscallReturn::err(Errno::NoEnt);
                    };
                    match section {
                        "cwd" => process.cwd.into_bytes(),
                        "exe" => process.image_path.into_bytes(),
                        _ => return SyscallReturn::err(Errno::NoEnt),
                    }
                } else {
                    if self.require_traversal_access(&path, false).is_err()
                        || self.require_access(&path, true, false, false).is_err()
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                    if let Some(target) = self.symlink_target(&path) {
                        target.into_bytes()
                    } else {
                        return SyscallReturn::err(Errno::NoEnt);
                    }
                };
                let ptr = frame.arg2 as *mut u8;
                let count = payload.len().min(frame.arg3);
                unsafe {
                    core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                }
                SyscallReturn::ok(count)
            }
            SYS_STAT_PATH | SYS_LSTAT_PATH => {
                let ptr = frame.arg2 as *mut NativeFileStatusRecord;
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                if !self.path_exists(path) && !path.starts_with("/proc/") {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                if !path.starts_with("/proc/")
                    && !path.starts_with("/dev/")
                    && !path.starts_with("/drv/")
                {
                    let normalized = match Self::normalize_absolute_path(path) {
                        Ok(path) => path,
                        Err(errno) => return SyscallReturn::err(errno),
                    };
                    let include_self_directory =
                        self.created_kind(&normalized) == Some(NativeObjectKind::Directory);
                    if self
                        .require_traversal_access(&normalized, include_self_directory)
                        .is_err()
                        || self
                            .require_access(&normalized, true, false, include_self_directory)
                            .is_err()
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                }
                unsafe {
                    let (owner_uid, group_gid, mode) = self.metadata_for_path(path);
                    let inode = self.inode_for_path(path);
                    let kind = self.created_kind(path).unwrap_or(NativeObjectKind::File);
                    let size = if path.starts_with("/proc/") {
                        4096
                    } else {
                        match kind {
                            NativeObjectKind::Directory => 0,
                            NativeObjectKind::Symlink => self
                                .symlink_target(path)
                                .map(|target| target.len())
                                .unwrap_or(0),
                            _ => self.file_content(path).len(),
                        }
                    } as u64;
                    ptr.write(NativeFileStatusRecord {
                        inode,
                        link_count: self.link_count_for_inode(inode),
                        size,
                        kind: kind as u32,
                        cloexec: 0,
                        nonblock: 0,
                        readable: 1,
                        writable: 1,
                        executable: u32::from((mode & 0o111) != 0),
                        owner_uid,
                        group_gid,
                        mode,
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_STATFS_PATH => {
                let ptr = frame.arg2 as *mut NativeFileSystemStatusRecord;
                unsafe {
                    ptr.write(NativeFileSystemStatusRecord {
                        mount_count: 1,
                        node_count: 3,
                        read_only: 0,
                        reserved: 0,
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_DEVICE => {
                let ptr = frame.arg2 as *mut NativeDeviceRecord;
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                if path == "/dev/gpu0" {
                    let latest = self.latest_gpu_request();
                    let mut record = NativeDeviceRecord {
                        class: 3,
                        state: 1,
                        reserved0: 0,
                        queue_depth: 0,
                        queue_capacity: 128,
                        submitted_requests: self.gpu_requests.borrow().len() as u64,
                        completed_requests: self.gpu_requests.borrow().len() as u64,
                        total_latency_ticks: 0,
                        max_latency_ticks: 0,
                        total_queue_wait_ticks: 0,
                        max_queue_wait_ticks: 0,
                        link_up: 1,
                        reserved1: 0,
                        block_size: 0,
                        reserved2: 0,
                        capacity_bytes: 0,
                        last_completed_request_id: 0,
                        last_completed_frame_tag: [0; 64],
                        last_completed_source_api_name: [0; 24],
                        last_completed_translation_label: [0; 32],
                        last_terminal_request_id: 0,
                        last_terminal_state: 0,
                        reserved3: 0,
                        last_terminal_frame_tag: [0; 64],
                        last_terminal_source_api_name: [0; 24],
                        last_terminal_translation_label: [0; 32],
                    };
                    if let Some(latest) = latest {
                        record.last_completed_request_id = latest.id;
                        record.last_completed_frame_tag = latest.record.frame_tag;
                        record.last_completed_source_api_name = latest.record.source_api_name;
                        record.last_completed_translation_label = latest.record.translation_label;
                        record.last_terminal_request_id = latest.id;
                        record.last_terminal_state = latest.record.state;
                        record.last_terminal_frame_tag = latest.record.frame_tag;
                        record.last_terminal_source_api_name = latest.record.source_api_name;
                        record.last_terminal_translation_label = latest.record.translation_label;
                    }
                    unsafe {
                        ptr.write(record);
                    }
                    return SyscallReturn::ok(0);
                }
                unsafe {
                    ptr.write(if path == "/dev/storage0" {
                        NativeDeviceRecord {
                            class: 2,
                            state: 1,
                            reserved0: 0,
                            queue_depth: 0,
                            queue_capacity: 128,
                            submitted_requests: 4,
                            completed_requests: 4,
                            total_latency_ticks: 0,
                            max_latency_ticks: 0,
                            total_queue_wait_ticks: 0,
                            max_queue_wait_ticks: 0,
                            link_up: 1,
                            reserved1: 0,
                            block_size: 512,
                            reserved2: 0,
                            capacity_bytes: 128 * 1024 * 1024,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved3: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        }
                    } else if path == "/dev/audio0" {
                        let latest = self.latest_audio_request();
                        let mut record = NativeDeviceRecord {
                            class: 4,
                            state: 1,
                            reserved0: 0,
                            queue_depth: 0,
                            queue_capacity: 128,
                            submitted_requests: self.audio_requests.borrow().len() as u64,
                            completed_requests: self
                                .audio_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 2)
                                .count() as u64,
                            total_latency_ticks: 0,
                            max_latency_ticks: 0,
                            total_queue_wait_ticks: 0,
                            max_queue_wait_ticks: 0,
                            link_up: 1,
                            reserved1: 0,
                            block_size: 0,
                            reserved2: 0,
                            capacity_bytes: self.audio_completion_payload.borrow().len() as u64,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved3: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        };
                        if let Some(latest) = latest {
                            if latest.record.state == 2 {
                                record.last_completed_request_id = latest.id;
                                record.last_completed_frame_tag = latest.record.frame_tag;
                                record.last_completed_source_api_name =
                                    latest.record.source_api_name;
                                record.last_completed_translation_label =
                                    latest.record.translation_label;
                            }
                            record.last_terminal_request_id = latest.id;
                            record.last_terminal_state = latest.record.state;
                            record.last_terminal_frame_tag = latest.record.frame_tag;
                            record.last_terminal_source_api_name = latest.record.source_api_name;
                            record.last_terminal_translation_label =
                                latest.record.translation_label;
                        }
                        record
                    } else if path == "/dev/input0" {
                        let latest = self.latest_input_request();
                        let mut record = NativeDeviceRecord {
                            class: 5,
                            state: 1,
                            reserved0: 0,
                            queue_depth: self
                                .input_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 1)
                                .count() as u64,
                            queue_capacity: 64,
                            submitted_requests: self.input_requests.borrow().len() as u64,
                            completed_requests: self
                                .input_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 2)
                                .count() as u64,
                            total_latency_ticks: 0,
                            max_latency_ticks: 0,
                            total_queue_wait_ticks: 0,
                            max_queue_wait_ticks: 0,
                            link_up: 1,
                            reserved1: 0,
                            block_size: 0,
                            reserved2: 0,
                            capacity_bytes: self.input_completion_payload.borrow().len() as u64,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved3: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        };
                        if let Some(latest) = latest {
                            if latest.record.state == 2 {
                                record.last_completed_request_id = latest.id;
                                record.last_completed_frame_tag = latest.record.frame_tag;
                                record.last_completed_source_api_name =
                                    latest.record.source_api_name;
                                record.last_completed_translation_label =
                                    latest.record.translation_label;
                            }
                            record.last_terminal_request_id = latest.id;
                            record.last_terminal_state = latest.record.state;
                            record.last_terminal_frame_tag = latest.record.frame_tag;
                            record.last_terminal_source_api_name = latest.record.source_api_name;
                            record.last_terminal_translation_label =
                                latest.record.translation_label;
                        }
                        record
                    } else {
                        NativeDeviceRecord {
                            class: 1,
                            state: 1,
                            reserved0: 0,
                            queue_depth: 0,
                            queue_capacity: 256,
                            submitted_requests: 2,
                            completed_requests: 2,
                            total_latency_ticks: 6,
                            max_latency_ticks: 3,
                            total_queue_wait_ticks: 2,
                            max_queue_wait_ticks: 1,
                            link_up: 1,
                            reserved1: 0,
                            block_size: 0,
                            reserved2: 0,
                            capacity_bytes: 0,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved3: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        }
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_DRIVER => {
                let ptr = frame.arg2 as *mut NativeDriverRecord;
                let path = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                if path == "/drv/gpu0" {
                    let latest = self.latest_gpu_request();
                    let mut record = NativeDriverRecord {
                        state: 1,
                        reserved: 0,
                        bound_device_count: 1,
                        queued_requests: 0,
                        in_flight_requests: 0,
                        completed_requests: self.gpu_requests.borrow().len() as u64,
                        last_completed_request_id: 0,
                        last_completed_frame_tag: [0; 64],
                        last_completed_source_api_name: [0; 24],
                        last_completed_translation_label: [0; 32],
                        last_terminal_request_id: 0,
                        last_terminal_state: 0,
                        reserved1: 0,
                        last_terminal_frame_tag: [0; 64],
                        last_terminal_source_api_name: [0; 24],
                        last_terminal_translation_label: [0; 32],
                    };
                    if let Some(latest) = latest {
                        record.last_completed_request_id = latest.id;
                        record.last_completed_frame_tag = latest.record.frame_tag;
                        record.last_completed_source_api_name = latest.record.source_api_name;
                        record.last_completed_translation_label = latest.record.translation_label;
                        record.last_terminal_request_id = latest.id;
                        record.last_terminal_state = latest.record.state;
                        record.last_terminal_frame_tag = latest.record.frame_tag;
                        record.last_terminal_source_api_name = latest.record.source_api_name;
                        record.last_terminal_translation_label = latest.record.translation_label;
                    }
                    unsafe {
                        ptr.write(record);
                    }
                    return SyscallReturn::ok(0);
                }
                unsafe {
                    ptr.write(if path == "/drv/audio0" {
                        let latest = self.latest_audio_request();
                        let mut record = NativeDriverRecord {
                            state: 1,
                            reserved: 0,
                            bound_device_count: 1,
                            queued_requests: 0,
                            in_flight_requests: self
                                .audio_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 1)
                                .count() as u64,
                            completed_requests: self
                                .audio_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 2)
                                .count() as u64,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved1: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        };
                        if let Some(latest) = latest {
                            if latest.record.state == 2 {
                                record.last_completed_request_id = latest.id;
                                record.last_completed_frame_tag = latest.record.frame_tag;
                                record.last_completed_source_api_name =
                                    latest.record.source_api_name;
                                record.last_completed_translation_label =
                                    latest.record.translation_label;
                            }
                            record.last_terminal_request_id = latest.id;
                            record.last_terminal_state = latest.record.state;
                            record.last_terminal_frame_tag = latest.record.frame_tag;
                            record.last_terminal_source_api_name = latest.record.source_api_name;
                            record.last_terminal_translation_label =
                                latest.record.translation_label;
                        }
                        record
                    } else if path == "/drv/input0" {
                        let latest = self.latest_input_request();
                        let mut record = NativeDriverRecord {
                            state: 1,
                            reserved: 0,
                            bound_device_count: 1,
                            queued_requests: self
                                .input_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 1)
                                .count() as u64,
                            in_flight_requests: self
                                .input_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 1)
                                .count() as u64,
                            completed_requests: self
                                .input_requests
                                .borrow()
                                .iter()
                                .filter(|request| request.record.state == 2)
                                .count() as u64,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved1: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        };
                        if let Some(latest) = latest {
                            if latest.record.state == 2 {
                                record.last_completed_request_id = latest.id;
                                record.last_completed_frame_tag = latest.record.frame_tag;
                                record.last_completed_source_api_name =
                                    latest.record.source_api_name;
                                record.last_completed_translation_label =
                                    latest.record.translation_label;
                            }
                            record.last_terminal_request_id = latest.id;
                            record.last_terminal_state = latest.record.state;
                            record.last_terminal_frame_tag = latest.record.frame_tag;
                            record.last_terminal_source_api_name = latest.record.source_api_name;
                            record.last_terminal_translation_label =
                                latest.record.translation_label;
                        }
                        record
                    } else {
                        NativeDriverRecord {
                            state: 1,
                            reserved: 0,
                            bound_device_count: 1,
                            queued_requests: 0,
                            in_flight_requests: 0,
                            completed_requests: 4,
                            last_completed_request_id: 0,
                            last_completed_frame_tag: [0; 64],
                            last_completed_source_api_name: [0; 24],
                            last_completed_translation_label: [0; 32],
                            last_terminal_request_id: 0,
                            last_terminal_state: 0,
                            reserved1: 0,
                            last_terminal_frame_tag: [0; 64],
                            last_terminal_source_api_name: [0; 24],
                            last_terminal_translation_label: [0; 32],
                        }
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_DOMAIN => {
                let ptr = frame.arg1 as *mut NativeDomainRecord;
                unsafe {
                    ptr.write(NativeDomainRecord {
                        id: 41,
                        owner: 1,
                        parent: 0,
                        resource_count: 1,
                        contract_count: 3,
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_RESOURCE => {
                let ptr = frame.arg1 as *mut NativeResourceRecord;
                let frames = self.frames.borrow();
                let (holder_contract, acquire_count, handoff_count, waiters) =
                    replay_resource_state(&frames, frame.arg0 as u64);
                unsafe {
                    ptr.write(NativeResourceRecord {
                        id: 42,
                        domain: 41,
                        creator: 1,
                        holder_contract,
                        kind: NativeResourceKind::Device as u32,
                        state: replayed_resource_operational_state(&frames, frame.arg0 as u64),
                        arbitration: NativeResourceArbitrationPolicy::Fifo as u32,
                        governance: replayed_resource_governance(&frames, frame.arg0 as u64),
                        contract_policy: replayed_resource_contract_policy(
                            &frames,
                            frame.arg0 as u64,
                        ),
                        issuer_policy: frames
                            .iter()
                            .rev()
                            .find(|entry| entry.number == SYS_SET_RESOURCE_ISSUER_POLICY)
                            .map(|entry| entry.arg1 as u32)
                            .unwrap_or(NativeResourceIssuerPolicy::AnyIssuer as u32),
                        waiting_count: waiters.len() as u64,
                        acquire_count,
                        handoff_count,
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_LIST_RESOURCE_WAITERS => {
                let ptr = frame.arg1 as *mut u64;
                let frames = self.frames.borrow();
                let (_, _, _, waiters) = replay_resource_state(&frames, frame.arg0 as u64);
                for (index, waiter) in waiters.iter().take(frame.arg2).enumerate() {
                    unsafe {
                        *ptr.add(index) = *waiter;
                    }
                }
                SyscallReturn::ok(waiters.len())
            }
            SYS_SET_RESOURCE_POLICY => SyscallReturn::ok(0),
            SYS_SET_RESOURCE_CONTRACT_POLICY => SyscallReturn::ok(0),
            SYS_SET_RESOURCE_ISSUER_POLICY => SyscallReturn::ok(0),
            SYS_SET_RESOURCE_STATE => SyscallReturn::ok(0),
            SYS_ACQUIRE_RESOURCE => SyscallReturn::ok(1),
            SYS_CLAIM_RESOURCE => {
                let ptr = frame.arg1 as *mut NativeResourceClaimRecord;
                let frames = self.frames.borrow();
                let states = frames
                    .iter()
                    .filter(|entry| entry.number == SYS_SET_CONTRACT_STATE)
                    .map(|entry| (entry.arg0 as u64, entry.arg1 as u32))
                    .collect::<Vec<_>>();
                if replayed_contract_state(&states, frame.arg0 as u64)
                    != NativeContractState::Active as u32
                {
                    return SyscallReturn::err(Errno::Access);
                }
                let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                if replayed_resource_operational_state(&frames, resource)
                    != NativeResourceState::Active as u32
                {
                    return SyscallReturn::err(Errno::Access);
                }
                let (holder_contract, acquire_count, _, waiters) =
                    replay_resource_state(&frames, resource);
                let contract = frame.arg0 as u64;
                let policy = replayed_resource_contract_policy(&frames, resource);
                let contract_kind = replayed_contract_kind(&frames, contract);
                let allowed_kind = match policy {
                    x if x == NativeResourceContractPolicy::Any as u32 => None,
                    x => Some(x - 1),
                };
                if let Some(expected) = allowed_kind
                    && contract_kind != expected
                {
                    return SyscallReturn::err(Errno::Access);
                }
                if holder_contract != 0
                    && holder_contract != contract
                    && replayed_resource_governance(&frames, resource)
                        == NativeResourceGovernanceMode::ExclusiveLease as u32
                {
                    return SyscallReturn::err(Errno::Busy);
                }
                unsafe {
                    if holder_contract == contract {
                        ptr.write(NativeResourceClaimRecord {
                            resource,
                            holder_contract: contract,
                            acquire_count,
                            position: 0,
                            queued: 0,
                            reserved: 0,
                        });
                        self.emit_resource_event(resource, contract, 0);
                    } else {
                        let position = waiters
                            .iter()
                            .position(|waiter| *waiter == contract)
                            .map(|index| index as u64 + 1)
                            .unwrap_or(0);
                        ptr.write(NativeResourceClaimRecord {
                            resource,
                            holder_contract,
                            acquire_count: 0,
                            position,
                            queued: 1,
                            reserved: 0,
                        });
                        self.emit_resource_event(resource, contract, 1);
                    }
                }
                SyscallReturn::ok(0)
            }
            SYS_CANCEL_RESOURCE_CLAIM => {
                let ptr = frame.arg1 as *mut NativeResourceCancelRecord;
                let frames = self.frames.borrow();
                let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                let (holder_contract, _, _, waiters) =
                    replay_resource_state(&frames[..frames.len().saturating_sub(1)], resource);
                let contract = frame.arg0 as u64;
                if holder_contract == contract {
                    return SyscallReturn::err(Errno::Busy);
                }
                if !waiters.contains(&contract) {
                    return SyscallReturn::err(Errno::Inval);
                }
                let (_, _, _, new_waiters) = replay_resource_state(&frames, resource);
                unsafe {
                    ptr.write(NativeResourceCancelRecord {
                        resource,
                        waiting_count: new_waiters.len() as u64,
                    });
                }
                self.emit_resource_event(resource, contract, 2);
                SyscallReturn::ok(0)
            }
            SYS_SET_RESOURCE_GOVERNANCE => SyscallReturn::ok(0),
            SYS_RELEASE_CLAIMED_RESOURCE => {
                let ptr = frame.arg1 as *mut NativeResourceReleaseRecord;
                let frames = self.frames.borrow();
                let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                let (old_holder, _, old_handoffs, _) =
                    replay_resource_state(&frames[..frames.len().saturating_sub(1)], resource);
                let (holder_contract, acquire_count, handoff_count, _) =
                    replay_resource_state(&frames, resource);
                unsafe {
                    if old_holder == frame.arg0 as u64
                        && holder_contract != 0
                        && holder_contract != frame.arg0 as u64
                        && handoff_count > old_handoffs
                    {
                        ptr.write(NativeResourceReleaseRecord {
                            resource,
                            handoff_contract: holder_contract,
                            acquire_count,
                            handoff_count,
                            handed_off: 1,
                            reserved: 0,
                        });
                    } else {
                        ptr.write(NativeResourceReleaseRecord {
                            resource,
                            handoff_contract: 0,
                            acquire_count: 0,
                            handoff_count,
                            handed_off: 0,
                            reserved: 0,
                        });
                    }
                }
                self.emit_resource_event(resource, frame.arg0 as u64, 3);
                if old_holder == frame.arg0 as u64
                    && holder_contract != 0
                    && holder_contract != frame.arg0 as u64
                    && handoff_count > old_handoffs
                {
                    self.emit_resource_event(resource, holder_contract, 4);
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_CONTRACT => {
                let ptr = frame.arg1 as *mut NativeContractRecord;
                let frames = self.frames.borrow();
                let mut state = replayed_contract_state(
                    &frames
                        .iter()
                        .filter(|entry| entry.number == SYS_SET_CONTRACT_STATE)
                        .map(|entry| (entry.arg0 as u64, entry.arg1 as u32))
                        .collect::<Vec<_>>(),
                    frame.arg0 as u64,
                );
                let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                if replayed_resource_operational_state(&frames, resource)
                    == NativeResourceState::Retired as u32
                {
                    state = NativeContractState::Revoked as u32;
                }
                unsafe {
                    ptr.write(NativeContractRecord {
                        id: frame.arg0 as u64,
                        domain: 41,
                        resource,
                        issuer: 1,
                        kind: replayed_contract_kind(&self.frames.borrow(), frame.arg0 as u64),
                        state,
                    });
                }
                SyscallReturn::ok(0)
            }
            SYS_SET_CONTRACT_STATE => {
                if frame.arg1 as u32 == NativeContractState::Revoked as u32 {
                    let resource =
                        replayed_contract_resource(&self.frames.borrow(), frame.arg0 as u64);
                    self.emit_resource_event(resource, frame.arg0 as u64, 5);
                }
                SyscallReturn::ok(0)
            }
            SYS_INVOKE_CONTRACT => {
                let frames = self.frames.borrow();
                let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                if replayed_resource_operational_state(&frames, resource)
                    != NativeResourceState::Active as u32
                {
                    return SyscallReturn::err(Errno::Access);
                }
                let state = replayed_contract_state(
                    &frames
                        .iter()
                        .filter(|entry| entry.number == SYS_SET_CONTRACT_STATE)
                        .map(|entry| (entry.arg0 as u64, entry.arg1 as u32))
                        .collect::<Vec<_>>(),
                    frame.arg0 as u64,
                );
                if state != NativeContractState::Active as u32 {
                    SyscallReturn::err(Errno::Access)
                } else {
                    let start = frames
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, entry)| {
                            entry.number == SYS_SET_CONTRACT_STATE
                                && entry.arg1 == NativeContractState::Active as usize
                        })
                        .map(|(index, _)| index)
                        .unwrap_or(0);
                    let count = frames
                        .iter()
                        .skip(start)
                        .filter(|entry| entry.number == SYS_INVOKE_CONTRACT)
                        .count();
                    SyscallReturn::ok(count)
                }
            }
            SYS_RELEASE_RESOURCE => {
                let frames = self.frames.borrow();
                let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                SyscallReturn::ok(resource as usize)
            }
            SYS_TRANSFER_RESOURCE => {
                let frames = self.frames.borrow();
                let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                let (holder_contract, _, _, _) =
                    replay_resource_state(&frames[..frames.len().saturating_sub(1)], resource);
                if holder_contract == frame.arg0 as u64 {
                    SyscallReturn::ok(resource as usize)
                } else {
                    SyscallReturn::err(Errno::Inval)
                }
            }
            SYS_GET_DOMAIN_NAME => {
                let bytes = b"graphics";
                let ptr = frame.arg1 as *mut u8;
                unsafe {
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                }
                SyscallReturn::ok(bytes.len())
            }
            SYS_GET_RESOURCE_NAME => {
                let bytes = b"gpu0";
                let ptr = frame.arg1 as *mut u8;
                unsafe {
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                }
                SyscallReturn::ok(bytes.len())
            }
            SYS_GET_CONTRACT_LABEL => {
                let bytes = b"scanout";
                let ptr = frame.arg1 as *mut u8;
                unsafe {
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                }
                SyscallReturn::ok(bytes.len())
            }
            SYS_FCNTL => match frame.arg1 & 0xff {
                0 => {
                    let (nonblock, _) = self.fd_flags(frame.arg0);
                    SyscallReturn::ok(usize::from(nonblock))
                }
                1 => {
                    let (_, cloexec) = self.fd_flags(frame.arg0);
                    SyscallReturn::ok(usize::from(cloexec) << 1)
                }
                2 => {
                    let nonblock = (frame.arg1 >> 8) != 0;
                    self.set_fd_nonblock(frame.arg0, nonblock);
                    SyscallReturn::ok(usize::from(nonblock))
                }
                3 => {
                    let cloexec = (frame.arg1 >> 8) != 0;
                    self.set_fd_cloexec(frame.arg0, cloexec);
                    SyscallReturn::ok(usize::from(cloexec) << 1)
                }
                4 => SyscallReturn::ok(self.query_fd_lock(frame.arg0)),
                5 => match self.try_lock_fd(frame.arg0, ((frame.arg1 >> 8) & 0xffff) as u16) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                },
                6 => match self.unlock_fd(frame.arg0, ((frame.arg1 >> 8) & 0xffff) as u16) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                },
                7 => match self.try_lock_fd_shared(frame.arg0, ((frame.arg1 >> 8) & 0xffff) as u16)
                {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                },
                8 => match self.unlock_fd_shared(frame.arg0, ((frame.arg1 >> 8) & 0xffff) as u16) {
                    Ok(value) => SyscallReturn::ok(value),
                    Err(errno) => SyscallReturn::err(errno),
                },
                _ => SyscallReturn::err(Errno::Inval),
            },
            SYS_POLL => {
                if self
                    .event_queue_modes
                    .borrow()
                    .iter()
                    .any(|(fd, _)| *fd == frame.arg0)
                {
                    let ready = self
                        .event_queue_pending
                        .borrow()
                        .iter()
                        .find(|(fd, _)| *fd == frame.arg0)
                        .map(|(_, records)| {
                            if records.is_empty() {
                                0usize
                            } else {
                                (POLLIN | POLLPRI) as usize
                            }
                        })
                        .unwrap_or(0);
                    return SyscallReturn::ok(ready & frame.arg1);
                }
                if self.opened_path_raw(frame.arg0).is_some() {
                    let payload = self.file_content_for_fd(frame.arg0).unwrap_or_default();
                    let offset = self.read_offset(frame.arg0);
                    let mut ready = 0usize;
                    if offset < payload.len() {
                        ready |= POLLIN as usize;
                    }
                    ready |= POLLOUT as usize;
                    return SyscallReturn::ok(ready & frame.arg1);
                }
                SyscallReturn::ok(frame.arg1)
            }
            SYS_DUP => {
                let new_fd = self.next_fd.get();
                self.next_fd.set(new_fd + 1);
                if let Some(record) = self.open_record(frame.arg0) {
                    self.open_files.borrow_mut().push(OpenFileRecord {
                        fd: new_fd,
                        description_id: record.description_id,
                        path: record.path,
                        inode: record.inode,
                        kind: record.kind,
                        deleted: record.deleted,
                        orphan_bytes: record.orphan_bytes,
                    });
                    self.fd_flags.borrow_mut().push((new_fd, false, false));
                } else {
                    let (nonblock, _) = self.fd_flags(frame.arg0);
                    self.fd_flags.borrow_mut().push((new_fd, nonblock, false));
                }
                SyscallReturn::ok(new_fd)
            }
            SYS_SEEK => {
                let Some(record) = self.open_record(frame.arg0) else {
                    return SyscallReturn::err(Errno::Badf);
                };
                let Some(whence) = SeekWhence::from_raw(frame.arg2 as u32) else {
                    return SyscallReturn::err(Errno::Inval);
                };
                let length = if record.deleted {
                    record.orphan_bytes.len()
                } else {
                    self.file_content(&record.path).len()
                };
                let base = match whence {
                    SeekWhence::Set => 0i64,
                    SeekWhence::Cur => self.read_offset(frame.arg0) as i64,
                    SeekWhence::End => length as i64,
                };
                let Some(new_offset) = base.checked_add(frame.arg1 as i64) else {
                    return SyscallReturn::err(Errno::Range);
                };
                if new_offset < 0 {
                    return SyscallReturn::err(Errno::Inval);
                }
                self.set_read_offset(frame.arg0, new_offset as usize);
                SyscallReturn::ok(new_offset as usize)
            }
            SYS_CLOSE => {
                let closed_description_id = self
                    .open_record(frame.arg0)
                    .map(|record| record.description_id);
                self.open_files
                    .borrow_mut()
                    .retain(|record| record.fd != frame.arg0);
                self.fd_flags
                    .borrow_mut()
                    .retain(|(open_fd, _, _)| *open_fd != frame.arg0);
                self.event_queue_pending
                    .borrow_mut()
                    .retain(|(queue_fd, _)| *queue_fd != frame.arg0);
                self.event_queue_modes
                    .borrow_mut()
                    .retain(|(queue_fd, _)| *queue_fd != frame.arg0);
                self.event_queue_nonblock
                    .borrow_mut()
                    .retain(|(queue_fd, _)| *queue_fd != frame.arg0);
                self.resource_event_queues
                    .borrow_mut()
                    .retain(|queue_fd| *queue_fd != frame.arg0);
                self.resource_event_watches
                    .borrow_mut()
                    .retain(|(queue_fd, _, _)| *queue_fd != frame.arg0);
                if let Some(description_id) = closed_description_id {
                    if !self.description_is_open(description_id) {
                        self.fd_locks
                            .borrow_mut()
                            .retain(|(_, owner_id, _, _)| *owner_id != description_id);
                        self.read_offsets
                            .borrow_mut()
                            .retain(|(candidate, _)| *candidate != description_id);
                        self.description_nonblock
                            .borrow_mut()
                            .retain(|(candidate, _)| *candidate != description_id);
                    }
                }
                SyscallReturn::ok(0)
            }
            SYS_BOOT_REPORT => SyscallReturn::ok(0),
            SYS_PRESENT_GPU_FRAME => {
                let path = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Ok(path) = path else {
                    return SyscallReturn::err(Errno::Inval);
                };
                if path != "/dev/gpu0" {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let payload =
                    unsafe { core::slice::from_raw_parts(frame.arg2 as *const u8, frame.arg3) };
                let request_id = self.record_gpu_present(payload);
                SyscallReturn::ok(request_id as usize)
            }
            SYS_INSPECT_DEVICE_REQUEST => {
                let ptr = frame.arg1 as *mut NativeDeviceRequestRecord;
                let request = self
                    .gpu_request(frame.arg0 as u64)
                    .map(|request| request.record)
                    .or_else(|| {
                        self.audio_request(frame.arg0 as u64)
                            .map(|request| request.record)
                    })
                    .or_else(|| {
                        self.input_request(frame.arg0 as u64)
                            .map(|request| request.record)
                    });
                let Some(request) = request else {
                    return SyscallReturn::err(Errno::NoEnt);
                };
                unsafe {
                    ptr.write(request);
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_GPU_DISPLAY => {
                let ptr = frame.arg2 as *mut NativeGpuDisplayRecord;
                let path = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Ok(path) = path else {
                    return SyscallReturn::err(Errno::Inval);
                };
                if path != "/dev/gpu0" {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let latest = self.latest_gpu_request();
                let record = NativeGpuDisplayRecord {
                    present: u32::from(latest.is_some()),
                    active_pipes: if latest.is_some() { 1 } else { 0 },
                    planned_frames: self.gpu_requests.borrow().len() as u64,
                    last_present_offset: 0,
                    last_present_len: latest
                        .as_ref()
                        .map(|request| request.payload.len() as u64)
                        .unwrap_or(0),
                    hardware_programming_confirmed: 0,
                };
                unsafe {
                    ptr.write(record);
                }
                SyscallReturn::ok(0)
            }
            SYS_INSPECT_GPU_SCANOUT => {
                let ptr = frame.arg2 as *mut NativeGpuScanoutRecord;
                let path = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Ok(path) = path else {
                    return SyscallReturn::err(Errno::Inval);
                };
                if path != "/dev/gpu0" {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let latest = self.latest_gpu_request();
                let record = if let Some(latest) = latest {
                    NativeGpuScanoutRecord {
                        presented_frames: self.gpu_requests.borrow().len() as u64,
                        last_frame_len: latest.payload.len() as u64,
                        last_frame_tag: latest.record.frame_tag,
                        last_source_api_name: latest.record.source_api_name,
                        last_translation_label: latest.record.translation_label,
                    }
                } else {
                    NativeGpuScanoutRecord {
                        presented_frames: 0,
                        last_frame_len: 0,
                        last_frame_tag: [0; 64],
                        last_source_api_name: [0; 24],
                        last_translation_label: [0; 32],
                    }
                };
                unsafe {
                    ptr.write(record);
                }
                SyscallReturn::ok(0)
            }
            SYS_READ_GPU_SCANOUT_FRAME => {
                let path = unsafe {
                    core::str::from_utf8(core::slice::from_raw_parts(
                        frame.arg0 as *const u8,
                        frame.arg1,
                    ))
                };
                let Ok(path) = path else {
                    return SyscallReturn::err(Errno::Inval);
                };
                if path != "/dev/gpu0" {
                    return SyscallReturn::err(Errno::NoEnt);
                }
                let payload = self.gpu_scanout_payload.borrow();
                let count = payload.len().min(frame.arg3);
                if count != 0 {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            payload.as_ptr(),
                            frame.arg2 as *mut u8,
                            count,
                        );
                    }
                }
                SyscallReturn::ok(count)
            }
            SYS_WRITE => {
                if frame.arg0 == 1 || frame.arg0 == 2 {
                    let bytes =
                        unsafe { core::slice::from_raw_parts(frame.arg1 as *const u8, frame.arg2) };
                    self.stdout.borrow_mut().extend_from_slice(bytes);
                } else if let Some(path) = self.opened_path(frame.arg0) {
                    let bytes =
                        unsafe { core::slice::from_raw_parts(frame.arg1 as *const u8, frame.arg2) };
                    if self.created_kind(&path) == Some(NativeObjectKind::Channel) {
                        self.push_channel_message(&path, bytes);
                        return SyscallReturn::ok(bytes.len());
                    }
                    if path == "/dev/gpu0" {
                        return SyscallReturn::ok(bytes.len());
                    }
                    if path == "/dev/audio0" {
                        self.record_audio_submit(bytes);
                        return SyscallReturn::ok(bytes.len());
                    }
                    if path == "/dev/input0" {
                        self.record_input_submit(bytes);
                        return SyscallReturn::ok(bytes.len());
                    }
                    if path == "/drv/gpu0" {
                        let text = core::str::from_utf8(bytes).unwrap_or_default();
                        let mut lines = text.splitn(2, '\n');
                        let header = lines.next().unwrap_or_default();
                        let payload = lines.next().unwrap_or_default().as_bytes();
                        let completed =
                            if let Some(value) = header.strip_prefix("complete-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_gpu_request(id, payload, 2))
                                    .unwrap_or(false)
                            } else if let Some(value) = header.strip_prefix("failed-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_gpu_request(id, payload, 3))
                                    .unwrap_or(false)
                            } else if let Some(value) = header.strip_prefix("cancel-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_gpu_request(id, payload, 4))
                                    .unwrap_or(false)
                            } else {
                                false
                            };
                        if !completed {
                            return SyscallReturn::err(Errno::NoEnt);
                        }
                        return SyscallReturn::ok(bytes.len());
                    }
                    if path == "/drv/audio0" {
                        let text = core::str::from_utf8(bytes).unwrap_or_default();
                        let mut lines = text.splitn(2, '\n');
                        let header = lines.next().unwrap_or_default();
                        let payload = lines.next().unwrap_or_default().as_bytes();
                        let completed =
                            if let Some(value) = header.strip_prefix("complete-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_audio_request(id, payload, 2))
                                    .unwrap_or(false)
                            } else if let Some(value) = header.strip_prefix("failed-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_audio_request(id, payload, 3))
                                    .unwrap_or(false)
                            } else if let Some(value) = header.strip_prefix("cancel-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_audio_request(id, payload, 4))
                                    .unwrap_or(false)
                            } else {
                                false
                            };
                        if !completed {
                            return SyscallReturn::err(Errno::NoEnt);
                        }
                        return SyscallReturn::ok(bytes.len());
                    }
                    if path == "/drv/input0" {
                        let text = core::str::from_utf8(bytes).unwrap_or_default();
                        let mut lines = text.splitn(2, '\n');
                        let header = lines.next().unwrap_or_default();
                        let payload = lines.next().unwrap_or_default().as_bytes();
                        let completed =
                            if let Some(value) = header.strip_prefix("complete-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_input_request(id, payload, 2))
                                    .unwrap_or(false)
                            } else if let Some(value) = header.strip_prefix("failed-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_input_request(id, payload, 3))
                                    .unwrap_or(false)
                            } else if let Some(value) = header.strip_prefix("cancel-request:") {
                                value
                                    .parse::<u64>()
                                    .ok()
                                    .map(|id| self.complete_input_request(id, payload, 4))
                                    .unwrap_or(false)
                            } else {
                                false
                            };
                        if !completed {
                            return SyscallReturn::err(Errno::NoEnt);
                        }
                        return SyscallReturn::ok(bytes.len());
                    }
                    if let Some(device_path) = Self::network_device_path_for_driver(&path) {
                        if let Some((_, payload)) = self
                            .file_contents
                            .borrow_mut()
                            .iter_mut()
                            .find(|(candidate, _)| candidate == &path)
                        {
                            *payload = bytes.to_vec();
                        } else {
                            self.file_contents
                                .borrow_mut()
                                .push((path.clone(), bytes.to_vec()));
                        }
                        if let Some((remote_ipv4, remote_port, local_port, payload)) =
                            Self::parse_udp_ipv4_frame(bytes)
                        {
                            if let Some(socket) =
                                self.network_sockets.borrow_mut().iter_mut().find(|entry| {
                                    entry.device_path == device_path
                                        && entry.record.local_port == local_port
                                })
                            {
                                socket.pending_rx_ipv4 = remote_ipv4;
                                socket.pending_rx_port = remote_port;
                                socket.pending_rx_payload = payload;
                            }
                        }
                        if let Ok(index) = self.ensure_network_interface_state(device_path) {
                            let mut interfaces = self.network_interfaces.borrow_mut();
                            interfaces[index].record.rx_ring_depth =
                                interfaces[index].record.rx_ring_depth.saturating_add(1);
                        }
                        self.emit_network_event(
                            device_path,
                            Self::network_socket_path_for_device(device_path),
                            NativeNetworkEventKind::RxReady,
                        );
                        return SyscallReturn::ok(bytes.len());
                    }
                    if !self.service_block_request(&path, bytes)
                        && !self.service_block_completion(&path, bytes)
                    {
                        if self.require_traversal_access(&path, false).is_err()
                            || self.require_access(&path, false, true, false).is_err()
                        {
                            return SyscallReturn::err(Errno::Access);
                        }
                        let offset = self.read_offset(frame.arg0);
                        let actor_description = self
                            .open_record(frame.arg0)
                            .map(|record| record.description_id);
                        if let Err(errno) =
                            self.write_file_content_at(&path, offset, bytes, actor_description)
                        {
                            return SyscallReturn::err(errno);
                        }
                        self.set_read_offset(frame.arg0, offset.saturating_add(bytes.len()));
                    }
                }
                SyscallReturn::ok(frame.arg2)
            }
            SYS_WRITEV => {
                if frame.arg0 == 1 || frame.arg0 == 2 {
                    let iovecs = unsafe {
                        core::slice::from_raw_parts(frame.arg1 as *const UserIoVec, frame.arg2)
                    };
                    let mut total = 0usize;
                    for iov in iovecs {
                        let bytes =
                            unsafe { core::slice::from_raw_parts(iov.base as *const u8, iov.len) };
                        self.stdout.borrow_mut().extend_from_slice(bytes);
                        total += bytes.len();
                    }
                    SyscallReturn::ok(total)
                } else {
                    SyscallReturn::err(Errno::Badf)
                }
            }
            _ => SyscallReturn::err(Errno::Inval),
        }
    }
}

#[test]
fn boot_desktop_frame_uses_framebuffer_dimensions_and_presents_graphics_queue() {
    let framebuffer = ngos_user_abi::bootstrap::FramebufferContext {
        width: 1920,
        height: 1080,
        pitch: 7680,
        bpp: 32,
    };

    let frame = build_boot_desktop_frame(&framebuffer).expect("desktop frame should build");

    assert_eq!(frame.width, 1920);
    assert_eq!(frame.height, 1080);
    assert_eq!(frame.frame_tag, "ngos-desktop-boot");
    assert_eq!(frame.queue, "graphics");
    assert_eq!(frame.present_mode, "mailbox");
    assert_eq!(frame.completion, "wait-present");
    assert!(frame.ops.len() >= 35);
}

#[test]
fn native_program_reports_kernel_launch_session_stages() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let frames = runtime.backend().frames.borrow();
    let reports = frames
        .iter()
        .filter(|frame| frame.number == SYS_BOOT_REPORT)
        .collect::<Vec<_>>();
    assert_eq!(reports.len(), 3);
    assert_eq!(reports[0].arg0, BootSessionStatus::Success as usize);
    assert_eq!(reports[0].arg1, BootSessionStage::Bootstrap as usize);
    assert_eq!(reports[1].arg1, BootSessionStage::NativeRuntime as usize);
    assert_eq!(reports[2].arg0, BootSessionStatus::Success as usize);
    assert_eq!(reports[2].arg1, BootSessionStage::Complete as usize);
}

#[test]
fn native_program_runs_kernel_launch_shell_script_from_stdin() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"help\n# comment line\npwd; env\nset NOTE shell-note\nvars\nalias ll list-path .\nalias say echo $NOTE\naliases\nsession\nps\nprocess-info 1\nkill 1 9\npending-signals 1\nblocked-signals 1\nspawn-path worker /bin/worker\njobs\njob-info 77\nfg 77\njobs\nproc 1 maps\nstatus\nfd\nfdinfo 0\nstat-path /proc/1/status\nlstat-path /proc/1/status\nstatfs-path /proc/1/status\nopen-path /proc/1/status\nreadlink-path /proc/1/cwd\ncat-file /motd\nmkdir-path /shell-tmp\ncd /shell-tmp\npwd\nmkfile-path note\nwrite-file note $NOTE\nappend-file note -extra\ngrep-file note shell\nassert-file-contains note shell-note-extra\ncat-file note\nmkfile-path copy\ncopy-file note copy\ncmp-file note copy\ncat-file copy\nmkfile-path script\nwrite-file script echo sourced-script\nsource-file script\nfalse || echo recovered\nassert-status 0\nrepeat 2 echo looped\ntrue && echo chained\nfalse && echo skipped\ntrue || echo skipped-2\nlast-status\nsymlink-path current-note note\nrename-path note note-2\nll\nreadlink-path current-note\nunalias ll\nunset NOTE\nhistory\nunlink-path current-note\ncd /\ndomains\ndomain 41\nresources\nresource 42\nwaiters 42\ncontracts\ncontract 43\nmkdomain render\nmkresource 41 device gpu1\nmkcontract 41 42 display mirror-2\nresource-policy 42 lifo\nresource-governance 42 exclusive-lease\nresource-contract-policy 42 display\nresource-issuer-policy 42 creator-only\nclaim 43\nwaiters 42\nreleaseclaim 43\ncontract-state 43 suspended\ncontract-state 43 active\nresource-state 42 suspended\nresource-state 42 active\ncat /proc/1/fd\nsay\necho hello-kernel\nexit 7\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 7);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("ngos shell"));
    assert!(stdout.contains("help session mode pwd env cd alias unalias"));
    assert!(stdout.contains("find-symbol crate-find-symbol refs crate-refs outline crate-outline"));
    assert!(stdout.contains("game-manifest game-plan game-launch game-simulate"));
    assert!(stdout.contains("game-simulate game-sessions"));
    assert!(stdout.contains("game-next"));
    assert!(stdout.contains(
        "game-gfx-plan game-gfx-submit game-gfx-status game-gfx-driver-read game-gfx-request"
    ));
    assert!(stdout.contains("game-gfx-next"));
    assert!(stdout.contains("game-audio-plan game-audio-submit game-audio-status"));
    assert!(stdout.contains("game-audio-next"));
    assert!(stdout.contains("game-input-plan game-input-submit game-input-status"));
    assert!(
        stdout.contains("game-watch-start game-watch-status game-watch-status-all game-watch-poll-all game-watch-wait game-watch-stop")
    );
    assert!(stdout.contains("game-watch-start"));
    assert!(stdout.contains("game-watch-status"));
    assert!(stdout.contains("game-watch-status-all"));
    assert!(stdout.contains("game-watch-poll-all"));
    assert!(stdout.contains("game-watch-wait"));
    assert!(stdout.contains("game-watch-stop"));
    assert!(stdout.contains("var NOTE=shell-note"));
    assert!(stdout.contains("alias ll='list-path .'"));
    assert!(stdout.contains("alias say='echo shell-note'"));
    assert!(stdout.contains("pid=1 name=ngos-userland-native image=/bin/ngos-userland-native cwd=/ parent=0 address-space=4 thread=1 state=Running exit=0 fds=3 caps=2 env=1 regions=2 threads=1 pending=1 session-reported=0 session-status=0 session-stage=0 scheduler-class=interactive scheduler-budget=2"));
    assert!(stdout.contains("signal-sent pid=1 signal=9"));
    assert!(stdout.contains("pid=1 pending-signals=9"));
    assert!(stdout.contains("pid=1 blocked-pending-signals=-"));
    assert!(stdout.contains("process-spawned pid=77 name=worker path=/bin/worker"));
    assert!(stdout.contains("job pid=77 name=worker path=/bin/worker state=live:Exited signals=0"));
    assert!(stdout.contains("job-info pid=77 name=worker path=/bin/worker state=live:Exited signals=0 exit=137 pending=0"));
    assert!(stdout.contains("foreground-complete pid=77 exit=137"));
    assert!(stdout.contains("job pid=77 name=worker path=/bin/worker state=reaped:137 signals=0"));
    assert!(stdout.contains("\n/\n"));
    assert!(stdout.contains("outcome_policy=require-zero-exit"));
    assert!(stdout.contains("protocol=kernel-launch"));
    assert!(stdout.contains("pid=1 name=ngos-userland-native state=Running cwd=/"));
    assert!(stdout.contains("/bin/ngos-userland-native"));
    assert!(stdout.contains("fd:\t0"));
    assert!(stdout.contains("path=/proc/1/status kind=file inode=99 size=4096"));
    assert!(stdout.contains("path=/proc/1/status mounts=1 nodes=3 read_only=0"));
    assert!(stdout.contains("opened path=/proc/1/status fd=7"));
    assert!(stdout.contains("link /shell-tmp/current-note -> /shell-tmp/note"));
    assert!(stdout.contains("ngos host motd"));
    assert!(stdout.contains("directory-created path=/shell-tmp"));
    assert!(stdout.contains("cwd-updated path=/shell-tmp"));
    assert!(stdout.contains("file-created path=/shell-tmp/note"));
    assert!(stdout.contains("file-written path=/shell-tmp/note bytes=10"));
    assert!(stdout.contains("file-appended path=/shell-tmp/note bytes=6"));
    assert!(stdout.contains("grep-summary path=/shell-tmp/note needle=shell matches=1"));
    assert!(
        stdout.contains("assert-file-contains-ok path=/shell-tmp/note needle=shell-note-extra")
    );
    assert!(stdout.contains("shell-note-extra"));
    assert!(stdout.contains("file-created path=/shell-tmp/copy"));
    assert!(stdout.contains("file-copied from=/shell-tmp/note to=/shell-tmp/copy bytes=16"));
    assert!(stdout.contains("files-match left=/shell-tmp/note right=/shell-tmp/copy bytes=16"));
    assert!(stdout.contains("file-created path=/shell-tmp/script"));
    assert!(stdout.contains("file-written path=/shell-tmp/script bytes=19"));
    assert!(stdout.contains("script-loaded path=/shell-tmp/script"));
    assert!(stdout.contains("sourced-script"));
    assert!(stdout.contains("recovered"));
    assert!(stdout.contains("assert-status-ok expected=0"));
    assert!(stdout.contains("repeat-expanded count=2"));
    assert!(stdout.matches("looped").count() >= 2);
    assert!(stdout.contains("chained"));
    assert!(!stdout.contains("skipped\n"));
    assert!(!stdout.contains("skipped-2\n"));
    assert!(stdout.contains("last-status=0"));
    assert!(stdout.contains("symlink-created path=/shell-tmp/current-note target=/shell-tmp/note"));
    assert!(stdout.contains("path-renamed from=/shell-tmp/note to=/shell-tmp/note-2"));
    assert!(stdout.contains("note-2\tFile"));
    assert!(stdout.contains("link /shell-tmp/current-note -> /shell-tmp/note"));
    assert!(stdout.contains("history 1 help"));
    assert!(stdout.contains("history 2 pwd"));
    assert!(stdout.contains("history "));
    assert!(stdout.contains("path-unlinked path=/shell-tmp/current-note"));
    assert!(stdout.contains("domain id=41 owner=1 resources=1 contracts=3 name=graphics"));
    assert!(stdout.contains("domain id=41 owner=1 parent=0 resources=1 contracts=3 name=graphics"));
    assert!(stdout.contains(
        "resource id=42 domain=41 kind=device state=active holder=0 waiters=0 name=gpu0"
    ));
    assert!(stdout.contains("resource id=42 domain=41 creator=1 kind=device state=active arbitration=fifo governance=queueing"));
    assert!(stdout.contains("resource=42 waiters=-"));
    assert!(stdout.contains(
        "contract id=43 domain=41 resource=42 issuer=1 kind=display state=active label=scanout"
    ));
    assert!(stdout.contains("domain-created id=41 name=render"));
    assert!(stdout.contains("resource-created id=42 domain=41 kind=device name=gpu1"));
    assert!(
        stdout.contains("contract-created id=43 domain=41 resource=42 kind=display label=mirror-2")
    );
    assert!(stdout.contains("resource-policy-updated id=42 policy=lifo"));
    assert!(stdout.contains("resource-governance-updated id=42 mode=exclusive-lease"));
    assert!(stdout.contains("resource-contract-policy-updated id=42 policy=display"));
    assert!(stdout.contains("resource-issuer-policy-updated id=42 policy=creator-only"));
    assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
    assert!(stdout.contains("claim-released contract=43 resource=42"));
    assert!(stdout.contains("contract-state-updated id=43 state=suspended"));
    assert!(stdout.contains("contract-state-updated id=43 state=active"));
    assert!(stdout.contains("resource-state-updated id=42 state=suspended"));
    assert!(stdout.contains("resource-state-updated id=42 state=active"));
    assert!(stdout.contains("0 [stdio:stdin]"));
    assert!(stdout.contains("hello-kernel"));
}

#[test]
fn recording_backend_accepts_empty_storage_commit_payload() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let before = runtime.inspect_storage_volume("/dev/storage0").unwrap();
    let generation = runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("empty payload storage commit should succeed");
    let after = runtime.inspect_storage_volume("/dev/storage0").unwrap();
    assert_eq!(generation as u64, before.generation + 1);
    assert_eq!(after.payload_len, 0);
    assert_eq!(fixed_text_field(&after.last_commit_tag), "clear");
}

#[test]
fn recording_backend_accepts_empty_storage_commit_after_recovery() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime
        .prepare_storage_commit(
            "/dev/storage0",
            "qemu-storage-commit-001",
            b"persist:qemu-storage-commit-001",
        )
        .unwrap();
    runtime.recover_storage_volume("/dev/storage0").unwrap();
    let generation = runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("empty payload storage commit after recovery should succeed");
    let after = runtime.inspect_storage_volume("/dev/storage0").unwrap();
    assert_eq!(generation as u64, after.generation);
    assert_eq!(after.payload_len, 0);
    assert_eq!(fixed_text_field(&after.last_commit_tag), "clear");
}

#[test]
fn native_storage_commit_boot_smoke_succeeds_in_recording_backend() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    let code = run_native_storage_commit_boot_smoke(&runtime);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
}

#[test]
fn native_storage_commit_boot_smoke_succeeds_after_device_primitives() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    assert_eq!(run_native_compat_graphics_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_audio_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_input_boot_smoke(&runtime), 0);
    assert_eq!(run_native_network_boot_smoke(&runtime), 0);
    let code = run_native_storage_commit_boot_smoke(&runtime);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
}

#[test]
fn recording_backend_accepts_clear_commit_after_device_primitives() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    assert_eq!(run_native_compat_graphics_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_audio_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_input_boot_smoke(&runtime), 0);
    assert_eq!(run_native_network_boot_smoke(&runtime), 0);
    runtime
        .prepare_storage_commit(
            "/dev/storage0",
            "qemu-storage-commit-001",
            b"persist:qemu-storage-commit-001",
        )
        .unwrap();
    runtime.recover_storage_volume("/dev/storage0").unwrap();
    runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("clear commit should still succeed after device primitives");
}

#[test]
fn recording_backend_reports_first_primitive_that_breaks_clear_commit() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("clear commit should succeed before device primitives");

    let runtime = UserRuntime::new(RecordingBackend::default());
    assert_eq!(run_native_compat_graphics_boot_smoke(&runtime), 0);
    runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("clear commit should succeed after graphics primitive");

    let runtime = UserRuntime::new(RecordingBackend::default());
    assert_eq!(run_native_compat_graphics_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_audio_boot_smoke(&runtime), 0);
    runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("clear commit should succeed after audio primitive");

    let runtime = UserRuntime::new(RecordingBackend::default());
    assert_eq!(run_native_compat_graphics_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_audio_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_input_boot_smoke(&runtime), 0);
    runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("clear commit should succeed after input primitive");

    let runtime = UserRuntime::new(RecordingBackend::default());
    assert_eq!(run_native_compat_graphics_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_audio_boot_smoke(&runtime), 0);
    assert_eq!(run_native_compat_input_boot_smoke(&runtime), 0);
    assert_eq!(run_native_network_boot_smoke(&runtime), 0);
    runtime
        .prepare_storage_commit("/dev/storage0", "clear", &[])
        .expect("clear commit should succeed after network primitive");
}

#[test]
fn native_shell_readlink_preserves_symlink_target_after_rename() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /shell-tmp\ncd /shell-tmp\nmkfile-path note\nsymlink-path current-note note\nrename-path note note-2\nreadlink-path current-note\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("link /shell-tmp/current-note -> /shell-tmp/note"),
        "{stdout}"
    );
}

#[test]
fn native_shell_reports_rust_symbol_navigation_commands_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /code\nmkfile-path /code/sample.rs\nappend-line /code/sample.rs pub struct Widget\nappend-line /code/sample.rs impl Widget\nappend-line /code/sample.rs     pub fn new()\nappend-line /code/sample.rs let created = Widget::new();\nfind-symbol /code Widget 2\nrefs /code Widget 2\noutline /code 2\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    println!("{stdout}");
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("symbol-match kind=struct path=/code/sample.rs line=1 name=Widget"));
    assert!(stdout.contains("symbol-match kind=impl path=/code/sample.rs line=2 name=Widget"));
    assert!(stdout.contains("find-symbol-summary path=/code needle=Widget depth=2"));
    assert!(stdout.contains("rust-ref kind=definition path=/code/sample.rs line=1"));
    assert!(stdout.contains("rust-ref kind=definition path=/code/sample.rs line=2"));
    assert!(stdout.contains("rust-ref kind=reference path=/code/sample.rs line=4"));
    assert!(stdout.contains("refs-summary path=/code needle=Widget depth=2"));
    assert!(
        stdout.contains(
            "outline-symbol path=/code/sample.rs line=1 indent=0 kind=struct name=Widget"
        )
    );
    assert!(
        stdout
            .contains("outline-symbol path=/code/sample.rs line=2 indent=0 kind=impl name=Widget")
    );
    assert!(
        stdout.contains("outline-symbol path=/code/sample.rs line=3 indent=0 kind=fn name=new")
    );
    assert!(stdout.contains("outline-summary path=/code depth=2"));
}

#[test]
fn native_shell_reports_build_diagnostics_and_test_failures_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /logs\nmkfile-path /logs/build.log\nappend-line /logs/build.log error[E0425]: cannot find value `answer` in this scope\nappend-line /logs/build.log   --> src/main.rs:7:13\nappend-line /logs/build.log warning: unused variable: `temp`\nappend-line /logs/build.log   --> src/lib.rs:11:9\nmkfile-path /logs/test.log\nappend-line /logs/test.log failures:\nappend-line /logs/test.log ---- tests::it_renders_frame stdout ----\nappend-line /logs/test.log thread 'tests::it_renders_frame' panicked at src/render.rs:22:5: assertion failed: frame_ready\nbuild-diagnostics /logs/build.log\ntest-failures /logs/test.log\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let result = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(result, 0, "stdout:\n{stdout}");
    assert!(stdout.contains(
        "build-diagnostic severity=error code=E0425 path=src/main.rs line=7 column=13 message=cannot find value `answer` in this scope"
    ));
    assert!(stdout.contains(
        "build-diagnostic severity=warning code=- path=src/lib.rs line=11 column=9 message=unused variable: `temp`"
    ));
    assert!(stdout.contains(
        "build-diagnostics-summary path=/logs/build.log diagnostics=2 errors=1 warnings=1 notes=0"
    ));
    assert!(stdout.contains(
        "test-failure name=tests::it_renders_frame path=src/render.rs line=22 column=5 reason=assertion failed: frame_ready"
    ));
    assert!(stdout.contains("test-failures-summary path=/logs/test.log failures=1"));
}

#[test]
fn native_shell_reports_call_set_and_coding_explain_tools_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /logs\nfn make-name(value) { if $value == lang { return shell-lang } else { return invalid } }\ncall-set REPORT_NAME make-name lang\nprint $REPORT_NAME\nmkfile-path /logs/build.log\nappend-line /logs/build.log error[E0425]: cannot find value `answer` in this scope\nappend-line /logs/build.log   --> src/main.rs:7:13\nappend-line /logs/build.log warning: unused variable: `temp`\nappend-line /logs/build.log   --> src/lib.rs:11:9\nmkfile-path /logs/test.log\nappend-line /logs/test.log failures:\nappend-line /logs/test.log ---- tests::it_renders_frame stdout ----\nappend-line /logs/test.log thread 'tests::it_renders_frame' panicked at src/render.rs:22:5: assertion failed: frame_ready\ndiagnostic-files /logs/build.log\nexplain-test-failures /logs/test.log\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let result = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(result, 0, "stdout:\n{stdout}");
    assert!(stdout.contains("call-set-expanded target=REPORT_NAME name=make-name lines=1 args=1"));
    assert!(stdout.contains("call-finished name=make-name target=REPORT_NAME value=shell-lang"));
    assert!(stdout.contains("shell-lang"));
    assert!(
        stdout
            .contains("diagnostic-file path=src/main.rs diagnostics=1 errors=1 warnings=0 notes=0")
    );
    assert!(
        stdout
            .contains("diagnostic-file path=src/lib.rs diagnostics=1 errors=0 warnings=1 notes=0")
    );
    assert!(stdout.contains("diagnostic-files-summary path=/logs/build.log files=2 diagnostics=2"));
    assert!(stdout.contains(
        "explain-test-failure name=tests::it_renders_frame path=src/render.rs line=22 column=5 kind=assertion hint=inspect-assertion-and-fixture"
    ));
    assert!(stdout.contains(
        "explain-test-failures path=/logs/test.log failures=1 files=1 verdict=red next=inspect-src/render.rs:22:5"
    ));
}

#[test]
fn native_shell_reports_match_and_review_tools_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /review\nmkfile-path /review/match.note\nset MODE coding\nmatch $MODE {\ncase coding {\nwrite-file /review/match.note semantic-review\nset MATCH_OUTCOME ok\n}\nelse {\nset MATCH_OUTCOME invalid\n}\n}\nprint $MATCH_OUTCOME\nmkfile-path /review/left.rs\nappend-line /review/left.rs fn render()\nappend-line /review/left.rs let mode = old\nmkfile-path /review/right.rs\nappend-line /review/right.rs fn render()\nappend-line /review/right.rs let mode = new\nappend-line /review/right.rs let frames = 3\nimpact-summary /review/left.rs /review/right.rs\nrollback-preview /review/left.rs /review/right.rs\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("match-result matched=coding value=coding"));
    assert!(stdout.contains("file-written path=/review/match.note bytes=15"));
    assert!(stdout.contains("ok"));
    assert!(stdout.contains(
        "impact-summary left=/review/left.rs right=/review/right.rs impact=behavior-edit risk=medium touched=2 changed=1 added=1 removed=0 unchanged=1 review=review-replaced-lines-first rollback=rollback-preview"
    ));
    assert!(stdout.contains(
        "rollback-preview left=/review/left.rs right=/review/right.rs apply=/review/right.rs=>/review/left.rs"
    ));
    assert!(stdout.contains("@@ -2,1 +2,1 @@"));
    assert!(stdout.contains("-let mode = new"));
    assert!(stdout.contains("+let mode = old"));
    assert!(stdout.contains("-let frames = 3"));
    assert!(stdout.contains(
        "rollback-preview-summary left=/review/left.rs right=/review/right.rs changed=1 added=0 removed=1"
    ));
}

#[test]
fn native_shell_reports_semantic_record_values() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"record-set BUILD_RESULT kind=diagnostic severity=warning path=src/lib.rs line=11\nvalue-type BUILD_RESULT\nvalue-show BUILD_RESULT\nrecord-get BUILD_RESULT path BUILD_PATH\nprint $BUILD_PATH\nvars\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("record-set name=BUILD_RESULT fields=4"));
    assert!(stdout.contains("value-type name=BUILD_RESULT type=record"));
    assert!(stdout.contains(
        "value-show name=BUILD_RESULT type=record value={kind=diagnostic, severity=warning, path=src/lib.rs, line=11}"
    ));
    assert!(stdout.contains("value-field name=BUILD_RESULT field=kind value=diagnostic"));
    assert!(stdout.contains("value-field name=BUILD_RESULT field=path value=src/lib.rs"));
    assert!(
        stdout.contains(
            "record-field name=BUILD_RESULT field=path target=BUILD_PATH value=src/lib.rs"
        )
    );
    assert!(stdout.contains("src/lib.rs"));
    assert!(stdout.contains(
        "var BUILD_RESULT={kind=diagnostic, severity=warning, path=src/lib.rs, line=11} type=record"
    ));
    assert!(stdout.contains("var BUILD_PATH=src/lib.rs type=string"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_record_values() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"record kind=diagnostic severity=warning path=src/lib.rs line=11 |> record-get path PIPE_PATH |> value-show\nprint $PIPE_PATH\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("pipeline-source stage=record type=record fields=4"));
    assert!(stdout.contains("pipeline-record-field field=path target=PIPE_PATH value=src/lib.rs"));
    assert!(stdout.contains("pipeline-show type=string value=src/lib.rs"));
    assert!(stdout.contains("pipeline-complete stages=3 type=string"));
    assert!(stdout.contains("src/lib.rs"));
}

#[test]
fn native_shell_runs_semantic_pipeline_query_over_record_keys_and_values() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"record name=shell kind=proof mode=semantic |> record-keys |> list-find kind RECORD_KIND_KEY |> value-show\nrecord name=shell kind=proof mode=semantic |> record-values |> filter-eq proof |> list-count RECORD_VALUE_MATCH |> value-show\nrecord name=shell kind=proof mode=semantic |> record-values |> list-find-eq proof RECORD_PROOF_VALUE |> value-show\nrecord name=shell kind=proof mode=semantic |> record-has mode |> value-show\nprint $RECORD_KIND_KEY\nprint $RECORD_VALUE_MATCH\nprint $RECORD_PROOF_VALUE\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("pipeline-record-keys count=3"));
    assert!(stdout.contains("pipeline-list-find needle=kind target=RECORD_KIND_KEY value=kind"));
    assert!(stdout.contains("pipeline-record-values count=3"));
    assert!(stdout.contains("pipeline-filter-eq value=proof count=1"));
    assert!(stdout.contains("pipeline-list-count target=RECORD_VALUE_MATCH count=1"));
    assert!(
        stdout.contains("pipeline-list-find-eq value=proof target=RECORD_PROOF_VALUE value=proof")
    );
    assert!(stdout.contains("pipeline-record-has field=mode present=true"));
    assert!(stdout.contains("pipeline-show type=string value=kind"));
    assert!(stdout.contains("pipeline-show type=int value=1"));
    assert!(stdout.contains("pipeline-show type=bool value=true"));
    assert!(stdout.contains("\nkind\n"));
    assert!(stdout.contains("\n1\n"));
    assert!(stdout.contains("\nproof\n"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_real_session_and_resource_surfaces() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"session |> record-get protocol SESSION_PROTOCOL |> value-show\nprocess-info 1 |> record-get state INIT_STATE |> value-show\nmkdomain render\ndomain 41 |> record-get name RENDER_DOMAIN_NAME |> value-show\nmkresource 41 device gpu1\nmkcontract 41 42 display mirror-2\nresource 42 |> record-get state RESOURCE_STATE |> value-show\nprint $SESSION_PROTOCOL\nprint $INIT_STATE\nprint $RENDER_DOMAIN_NAME\nprint $RESOURCE_STATE\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("pipeline-source stage=session type=record fields=4"));
    assert!(stdout.contains(
        "pipeline-record-field field=protocol target=SESSION_PROTOCOL value=kernel-launch"
    ));
    assert!(stdout.contains("pipeline-source stage=process-info pid=1 type=record fields=12"));
    assert!(stdout.contains("pipeline-record-field field=state target=INIT_STATE value="));
    assert!(stdout.contains("pipeline-source stage=domain id=41 type=record fields=6"));
    assert!(stdout.contains("pipeline-record-field field=name target=RENDER_DOMAIN_NAME value="));
    assert!(stdout.contains("pipeline-source stage=resource id=42 type=record fields=10"));
    assert!(
        stdout.contains("pipeline-record-field field=state target=RESOURCE_STATE value=active")
    );
    assert!(stdout.contains("kernel-launch"));
    assert!(stdout.contains("active"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_waiters_list_surface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdomain render\nset WAIT_DOMAIN $LAST_DOMAIN_ID\nmkresource $WAIT_DOMAIN device gpu1\nset WAIT_RESOURCE $LAST_RESOURCE_ID\nmkcontract $WAIT_DOMAIN $WAIT_RESOURCE display mirror-2\nset WAIT_PRIMARY $LAST_CONTRACT_ID\nmkcontract $WAIT_DOMAIN $WAIT_RESOURCE display mirror-3\nset WAIT_MIRROR $LAST_CONTRACT_ID\nclaim $WAIT_PRIMARY\nclaim $WAIT_MIRROR\nwaiters $WAIT_RESOURCE |> list-count WAITER_COUNT |> value-show\nwaiters $WAIT_RESOURCE |> list-first FIRST_WAITER |> value-show\nreleaseclaim $WAIT_PRIMARY\nrelease $WAIT_MIRROR\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("domain-created id="));
    assert!(stdout.contains("var-set WAIT_DOMAIN="));
    assert!(stdout.contains("resource-created id="));
    assert!(stdout.contains("var-set WAIT_RESOURCE="));
    assert!(stdout.contains("contract-created id="));
    assert!(stdout.contains("var-set WAIT_PRIMARY="));
    assert!(stdout.contains("var-set WAIT_MIRROR="));
    assert!(stdout.contains("claim-acquired contract="));
    assert!(stdout.contains("claim-queued contract="));
    assert!(stdout.contains("pipeline-source stage=waiters resource="));
    assert!(stdout.contains("type=list items=1"));
    assert!(stdout.contains("pipeline-list-count count=1"));
    assert!(stdout.contains("pipeline-list-first value="));
    assert!(stdout.contains("pipeline-show type=int value=1"));
    assert!(stdout.contains("pipeline-show type=int value="));
    assert!(stdout.contains("claim-handed-off resource="));
    assert!(stdout.contains("resource-released contract="));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_mount_record_surface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"storage-mount /dev/storage0 /mnt/pipeline\nmount-info /mnt/pipeline |> record-get path MOUNT_PATH |> value-show\nmount-info /mnt/pipeline |> record-get mode MOUNT_MODE |> value-show\nstorage-unmount /mnt/pipeline\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("storage-mount device=/dev/storage0 mount=/mnt/pipeline entries=0"));
    assert!(
        stdout.contains("pipeline-source stage=mount-info path=/mnt/pipeline type=record fields=9")
    );
    assert!(
        stdout.contains("pipeline-record-field field=path target=MOUNT_PATH value=/mnt/pipeline")
    );
    assert!(stdout.contains("pipeline-record-field field=mode target=MOUNT_MODE value="));
    assert!(stdout.contains("pipeline-show type=string value=/mnt/pipeline"));
    assert!(stdout.contains("storage-unmount mount=/mnt/pipeline generation=1"));
}

#[test]
fn native_shell_filters_mount_record_fields_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"storage-mount /dev/storage0 /mnt/filter\nmount-info /mnt/filter |> record-fields |> filter-contains mode= |> list-count MODE_COUNT |> value-show\nstorage-unmount /mnt/filter\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(
        stdout.contains("pipeline-source stage=mount-info path=/mnt/filter type=record fields=9")
    );
    assert!(stdout.contains("pipeline-record-fields count=9"));
    assert!(stdout.contains("pipeline-filter-contains needle=mode= count=1"));
    assert!(stdout.contains("pipeline-list-count target=MODE_COUNT count=1"));
    assert!(stdout.contains("pipeline-show type=int value=1"));
    assert!(stdout.contains("storage-unmount mount=/mnt/filter generation=1"));
}

#[test]
fn native_shell_filters_domain_resource_and_contract_inventory_lists() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdomain inventory\nset INV_DOMAIN $LAST_DOMAIN_ID\nmkresource $INV_DOMAIN device inv-gpu\nset INV_RESOURCE $LAST_RESOURCE_ID\nmkcontract $INV_DOMAIN $INV_RESOURCE display inv-primary\nset INV_CONTRACT $LAST_CONTRACT_ID\ndomains |> filter-contains $INV_DOMAIN |> list-count DOMAIN_COUNT |> value-show\nresources |> filter-contains $INV_RESOURCE |> list-count RESOURCE_COUNT |> value-show\ncontracts |> filter-contains $INV_CONTRACT |> list-count CONTRACT_COUNT |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("pipeline-source stage=domains type=list items="));
    assert!(stdout.contains("pipeline-source stage=resources type=list items="));
    assert!(stdout.contains("pipeline-source stage=contracts type=list items="));
    assert!(stdout.contains("pipeline-filter-contains needle="));
    assert!(stdout.contains("pipeline-list-count target=DOMAIN_COUNT count=1"));
    assert!(stdout.contains("pipeline-list-count target=RESOURCE_COUNT count=1"));
    assert!(stdout.contains("pipeline-list-count target=CONTRACT_COUNT count=1"));
    assert!(stdout.contains("pipeline-show type=int value=1"));
}

#[test]
fn native_shell_filters_queue_inventory_lists_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"queue-create epoll\nqueue-create kqueue\nqueues |> filter-contains Epoll |> list-count EPOLL_COUNT |> value-show\nqueues |> filter-contains Kqueue |> list-count KQUEUE_COUNT |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("queue-created fd="));
    assert!(stdout.contains("pipeline-source stage=queues type=list items=2"));
    assert!(stdout.contains("pipeline-filter-contains needle=Epoll count=1"));
    assert!(stdout.contains("pipeline-filter-contains needle=Kqueue count=1"));
    assert!(stdout.contains("pipeline-list-count target=EPOLL_COUNT count=1"));
    assert!(stdout.contains("pipeline-list-count target=KQUEUE_COUNT count=1"));
    assert!(stdout.contains("pipeline-show type=int value=1"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_network_socket_record_surface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /run\nmksock-path /run/net0.sock\nnet-config /dev/net0 10.1.0.2 255.255.255.0 10.1.0.1\nnet-admin /dev/net0 1500 4 4 2 up promisc\nudp-bind /run/net0.sock /dev/net0 4020 0.0.0.0 0\nudp-connect /run/net0.sock 10.1.0.9 5000\nnetsock /run/net0.sock |> record-get local_port NETSOCK_PORT |> value-show\nnetsock /run/net0.sock |> record-get connected NETSOCK_CONNECTED |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains(
        "udp-bound socket=/run/net0.sock device=/dev/net0 local-port=4020 remote=0.0.0.0:0"
    ));
    assert!(stdout.contains("udp-connected socket=/run/net0.sock remote=10.1.0.9:5000"));
    assert!(
        stdout.contains("pipeline-source stage=netsock path=/run/net0.sock type=record fields=11")
    );
    assert!(
        stdout.contains("pipeline-record-field field=local_port target=NETSOCK_PORT value=4020")
    );
    assert!(
        stdout.contains("pipeline-record-field field=connected target=NETSOCK_CONNECTED value=yes")
    );
    assert!(stdout.contains("pipeline-complete stages=3 type=string"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_network_interface_record_surface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"net-config /dev/net0 10.1.0.2 255.255.255.0 10.1.0.1\nnet-admin /dev/net0 1500 4 4 2 up promisc\nnetif /dev/net0 |> record-get addr NETIF_ADDR |> value-show\nnetif /dev/net0 |> record-get admin NETIF_ADMIN |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains(
        "netif-configured path=/dev/net0 addr=10.1.0.2 netmask=255.255.255.0 gateway=10.1.0.1"
    ));
    assert!(stdout.contains(
        "net-admin path=/dev/net0 mtu=1500 tx-cap=4 rx-cap=4 inflight-limit=2 admin=up promisc=on"
    ));
    assert!(stdout.contains("pipeline-source stage=netif path=/dev/net0 type=record fields=14"));
    assert!(stdout.contains("pipeline-record-field field=addr target=NETIF_ADDR value=10.1.0.2"));
    assert!(stdout.contains("pipeline-record-field field=admin target=NETIF_ADMIN value=up"));
    assert!(stdout.contains("pipeline-complete stages=3 type=string"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_fd_and_fdinfo_surfaces() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"fd |> into FD_LIST |> value-type\nfdinfo 0 |> record-get kind FD0_KIND |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-source stage=fd type=list items=3"));
    assert!(stdout.contains("pipeline-store name=FD_LIST type=list"));
    assert!(stdout.contains("pipeline-type type=list"));
    assert!(stdout.contains("pipeline-source stage=fdinfo fd=0 type=record fields=6"));
    assert!(stdout.contains("pipeline-record-field field=kind target=FD0_KIND value=File"));
    assert!(stdout.contains("pipeline-show type=string value=File"));
    assert!(stdout.contains("pipeline-complete stages=3 type=list"));
    assert!(stdout.contains("pipeline-complete stages=3 type=string"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_maps_surface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"maps 1 |> list-count MAP_COUNT |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-source stage=maps pid=1 type=list items="));
    assert!(stdout.contains("pipeline-list-count target=MAP_COUNT count="));
    assert!(stdout.contains("pipeline-show type=int value="));
    assert!(stdout.contains("pipeline-complete stages=3 type=int"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_vm_procfs_surfaces() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"vm-map-anon 1 8192 rw- vm-proof\nvmobjects 1 |> list-count VMOBJECT_COUNT |> value-show\nvmdecisions 1 |> list-count VMDECISION_COUNT |> value-show\nvmepisodes 1 |> list-count VMEPISODE_COUNT |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("vm-map-anon pid=1"));
    assert!(stdout.contains("pipeline-source stage=vmobjects pid=1 type=list items="));
    assert!(stdout.contains("pipeline-source stage=vmdecisions pid=1 type=list items="));
    assert!(stdout.contains("pipeline-source stage=vmepisodes pid=1 type=list items="));
    assert!(stdout.contains("pipeline-list-count target=VMOBJECT_COUNT count="));
    assert!(stdout.contains("pipeline-list-count target=VMDECISION_COUNT count="));
    assert!(stdout.contains("pipeline-list-count target=VMEPISODE_COUNT count="));
    assert!(stdout.contains("pipeline-show type=int value="));
    assert!(stdout.contains("pipeline-complete stages=3 type=int"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_caps_surface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"caps 1 |> list-count CAP_COUNT |> value-show\ncaps 1 |> list-first FIRST_CAP |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-source stage=caps pid=1 type=list items=2"));
    assert!(stdout.contains("pipeline-list-count target=CAP_COUNT count=2"));
    assert!(stdout.contains("pipeline-list-first target=FIRST_CAP value=capability:0"));
    assert!(stdout.contains("pipeline-show type=int value=2"));
    assert!(stdout.contains("pipeline-show type=string value=capability:0"));
}

#[test]
fn native_shell_runs_record_merge_and_set_field_pipeline() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"record owner=ngos shell=semantic |> into BASE\nprocess-info 1 |> record-select pid state caps |> record-merge BASE |> record-set-field cap-source capability:0 |> record-get owner OWNER |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-store name=BASE type=record"));
    assert!(stdout.contains("pipeline-record-select count=3"));
    assert!(stdout.contains("pipeline-record-merge source=BASE count=5"));
    assert!(stdout.contains("pipeline-record-set-field field=cap-source count=6"));
    assert!(stdout.contains("pipeline-record-field field=owner target=OWNER value=ngos"));
    assert!(stdout.contains("pipeline-show type=string value=ngos"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_procfs_process_views() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"status-of 1 |> record-get Name STATUS_NAME |> value-show\ncmdline-of 1 |> list-count CMDLINE_COUNT |> value-show\nenviron-of 1 |> list-find NGOS_SESSION= SESSION_ENV |> value-show\nroot-of 1 |> into ROOT_PATH |> value-show\ncwd-of 1 |> into CWD_PATH |> value-show\nexe-of 1 |> into EXE_PATH |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-record-field field=Name target=STATUS_NAME value="));
    assert!(stdout.contains("pipeline-list-count target=CMDLINE_COUNT count="));
    assert!(stdout.contains(
        "pipeline-list-find needle=NGOS_SESSION= target=SESSION_ENV value=NGOS_SESSION=1"
    ));
    assert!(stdout.contains("value-show target=_PIPE type=string value=/bin/ngos-userland-native"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_auxv_process_view() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"auxv-of 1 |> list-count AUXV_COUNT |> value-show\nauxv-of 1 |> list-first AUXV_FIRST |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-list-count target=AUXV_COUNT count="));
    assert!(stdout.contains("pipeline-list-first target=AUXV_FIRST value=AT_PAGESZ=4096"));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_process_compat_view() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"compat-of 1 |> record-get route COMPAT_ROUTE |> value-show\ncompat-of 1 |> record-get target COMPAT_TARGET |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-source stage=compat-of pid=1 type=record fields="));
    assert!(stdout.contains("pipeline-record-field field=route target=COMPAT_ROUTE value="));
    assert!(stdout.contains("pipeline-record-field field=target target=COMPAT_TARGET value="));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_process_identity_view() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"identity-of 1 |> record-get uid UID |> value-show\nidentity-of 1 |> record-get root ROOT |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-source stage=identity-of pid=1 type=record fields=5"));
    assert!(stdout.contains("pipeline-record-field field=uid target=UID value=1000"));
    assert!(stdout.contains("pipeline-record-field field=root target=ROOT value=/"));
}

#[test]
fn native_shell_runs_record_predicates_over_identity_and_compat() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"identity-of 1 |> record-eq uid 1000 |> value-show\ncompat-of 1 |> record-contains route native |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-record-eq field=uid value=1000 present=true"));
    assert!(stdout.contains("pipeline-record-contains field=route needle=native present=true"));
}

#[test]
fn native_shell_can_reload_saved_semantic_values_into_pipeline() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"record name=shell kind=proof mode=semantic |> into SAVED_RECORD\nvalue-load SAVED_RECORD |> record-get kind RECORD_KIND |> value-show\nstring alpha,beta |> string-split , |> into SAVED_LIST\nvalue-load SAVED_LIST |> list-first FIRST_ITEM |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-show type=string value=proof"));
    assert!(stdout.contains("pipeline-show type=string value=alpha"));
}

#[test]
fn native_shell_reports_ux_discovery_and_unknown_command_feedback() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"help-ux\nhelp-topic PIPELINE\nhelp-topic session\nhelp-topic recovery\nwhereami\ncommand-card Identity-Of\nexamples Identity-Of\nsuggest-next review\necho shell-needle\nrepeat-last\nrerun-find shell-needle\nrecent-work 4\nhistory-tail 4\nsuggest Pro\napropos Mount\nexplain-command Identity-Of\nhistory-find shell-needle\nmissing-helper\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("help-ux topics="));
    assert!(stdout.contains("help-topic topic=pipeline summary="));
    assert!(stdout.contains("help-topic topic=session summary="));
    assert!(stdout.contains("help-topic topic=recovery summary="));
    assert!(stdout.contains("whereami protocol=kernel-launch cwd=/"));
    assert!(stdout.contains("command-card command=identity-of topic="));
    assert!(stdout.contains("example identity identity-of 1 |> record-get uid UID |> value-show"));
    assert!(stdout.contains("suggest-next topic=review count=3"));
    assert!(stdout.contains("recent-work count="));
    assert!(stdout.contains("history-tail count="));
    assert!(stdout.contains("repeat-last queued=echo shell-needle"));
    assert!(stdout.contains("rerun-find needle=shell-needle queued=echo shell-needle"));
    assert!(stdout.contains("suggest prefix=pro"));
    assert!(stdout.contains("apropos needle=mount"));
    assert!(stdout.contains("command=identity-of summary="));
    assert!(stdout.contains("history-match "));
    assert!(stdout.contains("unknown-command"));
    assert!(stdout.contains("suggestion "));
    assert!(stdout.contains("suggest prefix=missing-helper count="));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_vfs_observability_views() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"vfsstats-of 1 |> record-get nodes VFS_NODE_COUNT |> value-show\nvfslocks-of 1 |> list-count VFS_LOCK_COUNT |> value-show\nvfswatches-of 1 |> list-count VFS_WATCH_COUNT |> value-show\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-record-field field=nodes target=VFS_NODE_COUNT value="));
    assert!(stdout.contains("pipeline-list-count target=VFS_LOCK_COUNT count="));
    assert!(stdout.contains("pipeline-list-count target=VFS_WATCH_COUNT count="));
}

#[test]
fn native_shell_runs_semantic_pipeline_over_mount_inventory_surface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"storage-mount /dev/storage0 /mnt/semantic\nmounts |> filter-contains /mnt/semantic |> list-count MOUNT_COUNT |> value-show\nmounts |> list-first FIRST_MOUNT |> value-show\nstorage-unmount /mnt/semantic\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-list-count target=MOUNT_COUNT count=1"));
    assert!(stdout.contains("pipeline-list-first target=FIRST_MOUNT value=/"));
}

#[test]
fn native_shell_runs_semantic_list_field_queries_over_mounts_and_auxv() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"storage-mount /dev/storage0 /mnt/fields\nmounts |> filter-field-eq device /dev/storage0 |> list-count DEVICE_MATCH_COUNT |> value-show\nmounts |> list-field mode |> filter-eq private |> list-count MODE_MATCH_COUNT |> value-show\nauxv-of 1 |> list-field AT_EXECFN |> list-first EXECFN |> value-show\nstorage-unmount /mnt/fields\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("pipeline-list-count target=MODE_MATCH_COUNT count=2"));
    assert!(stdout.contains("pipeline-list-first target=EXECFN value=/bin/ngos-userland-native"));
}

#[test]
fn native_shell_runs_semantic_list_predicates_over_mounts() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"storage-mount /dev/storage0 /mnt/preds\nmounts |> list-any-contains /mnt/preds |> value-show\nmounts |> list-all-contains mode= |> value-show\nstorage-unmount /mnt/preds\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("value-show target=_PIPE type=bool value=true"));
}

#[test]
fn native_shell_reports_diff_patch_and_explain_tools_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /diff\nmkfile-path /diff/left.rs\nappend-line /diff/left.rs fn render()\nappend-line /diff/left.rs let mode = old;\nmkfile-path /diff/right.rs\nappend-line /diff/right.rs fn render()\nappend-line /diff/right.rs let mode = new;\nappend-line /diff/right.rs let frames = 3;\ndiff-files /diff/left.rs /diff/right.rs\npatch-preview /diff/left.rs /diff/right.rs\nexplain-diff /diff/left.rs /diff/right.rs\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(
        stdout.contains(
            "diff-change left-line=2 right-line=2 left=let mode = old right=let mode = new"
        )
    );
    assert!(stdout.contains("diff-add right-line=3 text=let frames = 3"));
    assert!(stdout.contains(
        "diff-files-summary left=/diff/left.rs right=/diff/right.rs changed=1 added=1 removed=0 unchanged=1"
    ));
    assert!(stdout.contains("patch-preview left=/diff/left.rs right=/diff/right.rs"));
    assert!(stdout.contains("@@ -2,1 +2,1 @@"));
    assert!(stdout.contains("-let mode = old"));
    assert!(stdout.contains("+let mode = new"));
    assert!(stdout.contains("+let frames = 3"));
    assert!(stdout.contains(
        "patch-preview-summary left=/diff/left.rs right=/diff/right.rs changed=1 added=1 removed=0"
    ));
    assert!(stdout.contains(
        "explain-diff left=/diff/left.rs right=/diff/right.rs impact=behavior-edit changed=1 added=1 removed=0 unchanged=1"
    ));
    assert!(stdout.contains("explain-diff-change left-line=2 right-line=2 effect=replaced-line"));
    assert!(stdout.contains("explain-diff-add right-line=3 effect=inserted-line"));
}

#[test]
fn native_program_runs_vm_pressure_global_command_from_shell() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"vm-pressure-global 5\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    let exit = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    println!("seek-shell stdout:\n{stdout}");
    assert_eq!(exit, 0, "stdout:\n{stdout}");
    assert!(stdout.contains("vm-pressure-global target-pages=5 reclaimed-pages=0"));

    let frames = runtime.backend().frames.borrow();
    assert!(
        frames
            .iter()
            .any(|frame| { frame.number == SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL && frame.arg0 == 5 })
    );
}

#[test]
fn recording_backend_mount_propagation_clones_child_mounts_and_blocks_parent_unmount() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime
        .mount_storage_volume("/dev/storage0", "/mnt/shared")
        .unwrap();
    runtime
        .mount_storage_volume("/dev/storage0", "/mnt/peer")
        .unwrap();
    runtime
        .set_mount_propagation("/mnt/shared", NativeMountPropagationMode::Shared)
        .unwrap();
    runtime
        .set_mount_propagation("/mnt/peer", NativeMountPropagationMode::Slave)
        .unwrap();

    let shared = runtime.inspect_mount("/mnt/shared").unwrap();
    let peer = runtime.inspect_mount("/mnt/peer").unwrap();
    assert_eq!(
        NativeMountPropagationMode::from_raw(shared.propagation_mode),
        Some(NativeMountPropagationMode::Shared)
    );
    assert_ne!(shared.peer_group, 0);
    assert_eq!(
        NativeMountPropagationMode::from_raw(peer.propagation_mode),
        Some(NativeMountPropagationMode::Slave)
    );
    assert_eq!(peer.master_group, shared.peer_group);

    runtime
        .mount_storage_volume("/dev/storage0", "/mnt/shared/child")
        .unwrap();
    let child = runtime.inspect_mount("/mnt/shared/child").unwrap();
    let clone = runtime.inspect_mount("/mnt/peer/child").unwrap();
    assert_eq!(
        NativeMountPropagationMode::from_raw(child.propagation_mode),
        Some(NativeMountPropagationMode::Shared)
    );
    assert_ne!(child.peer_group, 0);
    assert_eq!(
        NativeMountPropagationMode::from_raw(clone.propagation_mode),
        Some(NativeMountPropagationMode::Slave)
    );
    assert_eq!(clone.master_group, child.peer_group);

    assert_eq!(
        runtime.unmount_storage_volume("/mnt/shared"),
        Err(Errno::Busy)
    );
    runtime.unmount_storage_volume("/mnt/shared/child").unwrap();
    assert_eq!(runtime.inspect_mount("/mnt/peer/child"), Err(Errno::NoEnt));
    runtime.unmount_storage_volume("/mnt/shared").unwrap();
    assert_eq!(runtime.inspect_mount("/mnt/peer"), Err(Errno::NoEnt));
}

#[test]
fn recording_backend_refuses_cross_mount_rename_and_link() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime
        .mount_storage_volume("/dev/storage0", "/mnt/a")
        .unwrap();
    runtime
        .mount_storage_volume("/dev/storage0", "/mnt/b")
        .unwrap();
    runtime.mkfile_path("/mnt/a/file.txt").unwrap();
    runtime.mkfile_path("/mnt/b/other.txt").unwrap();

    assert_eq!(
        runtime.rename_path("/mnt/a/file.txt", "/mnt/b/file.txt"),
        Err(Errno::Busy)
    );
    assert_eq!(
        runtime.link_path("/mnt/a/file.txt", "/mnt/b/link.txt"),
        Err(Errno::Busy)
    );
}

#[test]
fn recording_backend_file_mapping_tracks_sync_truncate_and_unlink_lifecycle() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime.mkfile_path("/mapped.bin").unwrap();
    let fd = runtime.open_path("/mapped.bin").unwrap();
    runtime.write(fd, b"abcdwxyz").unwrap();
    runtime.close(fd).unwrap();

    let mapped = runtime
        .map_file_memory(1, "/mapped.bin", 0x1000, 0, true, true, false, true)
        .unwrap();
    assert_eq!(
        runtime.load_memory_word(1, mapped).unwrap(),
        u32::from_le_bytes(*b"abcd")
    );
    runtime
        .store_memory_word(1, mapped + 4, u32::from_le_bytes(*b"4321"))
        .unwrap();
    runtime.sync_memory_range(1, mapped, 0x1000).unwrap();
    let synced = shell_read_file_text(&runtime, "/mapped.bin").unwrap();
    assert_eq!(synced, "abcd4321");

    runtime.truncate_path("/mapped.bin", 2).unwrap();
    assert_eq!(runtime.load_memory_word(1, mapped + 4).unwrap(), 0);

    runtime.unlink_path("/mapped.bin").unwrap();
    assert_eq!(
        runtime.load_memory_word(1, mapped).unwrap(),
        u32::from_le_bytes([b'a', b'b', 0, 0])
    );
    runtime.sync_memory_range(1, mapped, 0x1000).unwrap();
    runtime.unmap_memory_range(1, mapped, 0x1000).unwrap();
}

#[test]
fn recording_backend_locks_block_mutations_until_release() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime.mkfile_path("/locked.txt").unwrap();
    let primary = runtime.open_path("/locked.txt").unwrap();
    let secondary = runtime.open_path("/locked.txt").unwrap();
    runtime
        .fcntl(primary, FcntlCmd::TryLockShared { token: 0x11 })
        .unwrap();
    runtime
        .fcntl(secondary, FcntlCmd::TryLockShared { token: 0x22 })
        .unwrap();

    assert_eq!(runtime.write(secondary, b"x"), Err(Errno::Busy));
    assert_eq!(runtime.truncate_path("/locked.txt", 1), Err(Errno::Busy));
    assert_eq!(
        runtime.rename_path("/locked.txt", "/locked-renamed.txt"),
        Err(Errno::Busy)
    );
    assert_eq!(
        runtime.link_path("/locked.txt", "/locked-link.txt"),
        Err(Errno::Busy)
    );
    assert_eq!(runtime.unlink_path("/locked.txt"), Err(Errno::Busy));

    runtime
        .fcntl(primary, FcntlCmd::UnlockShared { token: 0x11 })
        .unwrap();
    runtime
        .fcntl(secondary, FcntlCmd::UnlockShared { token: 0x22 })
        .unwrap();
    runtime
        .link_path("/locked.txt", "/locked-link.txt")
        .unwrap();
    runtime.unlink_path("/locked-link.txt").unwrap();
}

#[test]
fn recording_backend_rename_replaces_existing_target_and_preserves_deleted_fd() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime.mkfile_path("/replace-src").unwrap();
    runtime.mkfile_path("/replace-dst").unwrap();
    let src_fd = runtime.open_path("/replace-src").unwrap();
    runtime.write(src_fd, b"new-bytes").unwrap();
    runtime.close(src_fd).unwrap();
    let dst_fd = runtime.open_path("/replace-dst").unwrap();
    runtime.write(dst_fd, b"old-bytes").unwrap();
    assert_eq!(runtime.seek(dst_fd, 0, SeekWhence::Set).unwrap(), 0);

    runtime.rename_path("/replace-src", "/replace-dst").unwrap();
    assert_eq!(
        shell_read_file_text(&runtime, "/replace-dst").unwrap(),
        "new-bytes"
    );
    let fdinfo = String::from_utf8(read_procfs_all(&runtime, "/proc/1/fdinfo/8").unwrap()).unwrap();
    assert!(fdinfo.contains("path:\t/replace-dst (deleted)"));
    let mut buffer = [0u8; 16];
    let count = runtime.read(dst_fd, &mut buffer).unwrap();
    assert_eq!(&buffer[..count], b"old-bytes");
    runtime.close(dst_fd).unwrap();

    runtime.mkdir_path("/replace-dir-src").unwrap();
    runtime.mkdir_path("/replace-dir-dst").unwrap();
    runtime.mkfile_path("/replace-dir-dst/child").unwrap();
    assert_eq!(
        runtime.rename_path("/replace-dir-src", "/replace-dir-dst"),
        Err(Errno::Busy)
    );

    runtime.mkdir_path("/replace-dir-empty-src").unwrap();
    runtime.mkdir_path("/replace-dir-empty-dst").unwrap();
    runtime
        .rename_path("/replace-dir-empty-src", "/replace-dir-empty-dst")
        .unwrap();
    assert_eq!(
        runtime.stat_path("/replace-dir-empty-src"),
        Err(Errno::NoEnt)
    );
    assert!(runtime.stat_path("/replace-dir-empty-dst").is_ok());

    runtime.mkdir_path("/replace-kind-dir").unwrap();
    runtime.mkfile_path("/replace-kind-file").unwrap();
    assert_eq!(
        runtime.rename_path("/replace-kind-dir", "/replace-kind-file"),
        Err(Errno::Inval)
    );
    assert_eq!(
        runtime.rename_path("/replace-kind-file", "/replace-kind-dir"),
        Err(Errno::Inval)
    );
}

#[test]
fn recording_backend_sticky_directory_blocks_rename_and_unlink_for_non_owner() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime.backend().set_current_subject(0, 0);
    runtime.mkdir_path("/sticky").unwrap();
    runtime.mkfile_path("/sticky/other.txt").unwrap();
    runtime.chmod_path("/sticky", 0o1777).unwrap();
    runtime.chown_path("/sticky/other.txt", 2000, 2000).unwrap();
    runtime.backend().set_current_subject(1000, 1000);

    assert_eq!(runtime.unlink_path("/sticky/other.txt"), Err(Errno::Access));
    assert_eq!(
        runtime.rename_path("/sticky/other.txt", "/sticky/renamed.txt"),
        Err(Errno::Access)
    );

    runtime.backend().set_current_subject(0, 0);
    runtime.chown_path("/sticky/other.txt", 1000, 1000).unwrap();
    runtime.backend().set_current_subject(1000, 1000);
    runtime
        .rename_path("/sticky/other.txt", "/sticky/renamed.txt")
        .unwrap();
    runtime.unlink_path("/sticky/renamed.txt").unwrap();
}

#[test]
fn recording_backend_sgid_directory_inherits_group_and_directory_bit() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime.backend().set_current_subject(0, 0);
    runtime.mkdir_path("/shared").unwrap();
    runtime.chown_path("/shared", 0, 4242).unwrap();
    runtime.chmod_path("/shared", 0o2775).unwrap();
    runtime.backend().set_current_subject(1000, 4242);

    runtime.mkfile_path("/shared/file.txt").unwrap();
    runtime.mkdir_path("/shared/subdir").unwrap();

    let file_status = runtime.stat_path("/shared/file.txt").unwrap();
    let dir_status = runtime.stat_path("/shared/subdir").unwrap();
    assert_eq!(file_status.group_gid, 4242);
    assert_eq!(dir_status.group_gid, 4242);
    assert_eq!(dir_status.mode & 0o2000, 0o2000);
}

#[test]
fn native_shell_can_transfer_and_release_resource_through_contract_commands() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"claim 43\nclaim 44\ntransfer 43 44\nresource 42\nrelease 44\nresource 42\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
    assert!(stdout.contains("claim-queued contract=44 resource=42 holder=43 position=1"));
    assert!(stdout.contains("resource-transferred source=43 target=44 resource=42"));
    assert!(stdout.contains("resource id=42 domain=41 creator=1 kind=device state=active arbitration=fifo governance=queueing"));
    assert!(stdout.contains("holder=44"));
    assert!(stdout.contains("resource-released contract=44 resource=42"));
    assert!(stdout.contains("holder=0"));
}

#[test]
fn native_shell_reports_resource_governance_refusal_and_recovery_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"claim 43\nresource-governance 42 exclusive-lease\nclaim 44\nlast-status\nresource-governance 42 queueing\nclaim 44\nreleaseclaim 43\nresource 42\nrelease 44\nresource 42\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
    assert!(stdout.contains("resource-governance-updated id=42 mode=exclusive-lease"));
    assert!(stdout.contains("claim-refused contract=44 errno=EBUSY code=16"));
    assert!(stdout.contains("last-status=241"));
    assert!(stdout.contains("resource-governance-updated id=42 mode=queueing"));
    assert!(stdout.contains("claim-queued contract=44 resource=42 holder=43 position=1"));
    assert!(stdout.contains("claim-handed-off resource=42 to=44 acquire_count=2 handoff_count=1"));
    assert!(stdout.contains("holder=44"));
    assert!(stdout.contains("resource-released contract=44 resource=42"));
    assert!(stdout.contains("holder=0"));
}

#[test]
fn native_shell_reports_contract_and_resource_state_refusal_and_recovery_semantically() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"contract-state 43 suspended\ninvoke 43\nlast-status\ncontract-state 43 active\ninvoke 43\nresource-state 42 suspended\nclaim 43\nlast-status\nresource-state 42 active\nclaim 43\nreleaseclaim 43\nresource 42\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("contract-state-updated id=43 state=suspended"));
    assert!(stdout.contains("invoke-refused contract=43 errno=EACCES code=13"));
    assert!(stdout.contains("last-status=244"));
    assert!(stdout.contains("contract-state-updated id=43 state=active"));
    assert!(stdout.contains("invoked contract=43 count=1"));
    assert!(stdout.contains("resource-state-updated id=42 state=suspended"));
    assert!(stdout.contains("claim-refused contract=43 errno=EACCES code=13"));
    assert!(stdout.contains("resource-state-updated id=42 state=active"));
    assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
    assert!(stdout.contains("claim-released contract=43 resource=42"));
    assert!(stdout.contains("holder=0"));
}

#[test]
fn native_program_runs_signal_commands_from_stdin() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"process-info 1\nkill 1 9\npending-signals 1\nblocked-signals 1\nprocess-info 1\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("pid=1 name=ngos-userland-native"));
    assert!(stdout.contains("signal-sent pid=1 signal=9"));
    assert!(stdout.contains("pid=1 pending-signals=9"));
    assert!(stdout.contains("pid=1 blocked-pending-signals=-"));
}

#[test]
fn native_shell_unifies_direct_and_semantic_control_modes() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mode semantic\nobserve system\nrenice 1 background 1\npause 1\nresume 1\nintent optimize process 1\nlearn\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("mode=semantic"));
    assert!(stdout.contains("fact process pid=1 name=ngos-userland-native state=Running class=interactive budget=2 cwd=/ image=/bin/ngos-userland-native"));
    assert!(stdout.contains("fact device path=/dev/net0"));
    assert!(stdout.contains("process-control pid=1"));
    assert!(stdout.contains("class=background budget=1"));
    assert!(stdout.contains("class=latency-critical budget=4"));
    assert!(stdout.contains("learn subject=process:1 action=renice policy-epoch="));
    assert!(stdout.contains("learn subject=process:1 action=pause policy-epoch="));
    assert!(stdout.contains("learn subject=process:1 action=resume policy-epoch="));
    assert!(stdout.contains("learn subject=process:1 action=optimize policy-epoch="));
}

#[test]
fn native_shell_semantic_affinity_controls_process_cpu_mask() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mode semantic\naffinity 1 0x3\nlearn\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("process-control pid=1"));
    assert!(stdout.contains("cpu-mask=0x3"));
    assert!(stdout.contains("learn subject=process:1 action=affinity policy-epoch="));
    let frames = runtime.backend().frames.borrow();
    let affinity = frames
        .iter()
        .find(|entry| entry.number == SYS_SET_PROCESS_AFFINITY && entry.arg0 == 1)
        .copied()
        .unwrap();
    assert_eq!(affinity.arg1, 0x3);
}

#[test]
fn native_shell_nextmind_optimizes_mixed_pressure_and_explains_actions() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    let session = parse_session_context(&bootstrap).unwrap();
    assert_eq!(
        run_session_shell_script(
            &runtime,
            &session,
            "mode semantic\nnextmind.optimize\nnextmind.observe\nnextmind.explain last\nnextmind.auto on\ntrue\nnextmind.auto off\nexit 0\n",
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("nextmind.metrics label=before state="));
    assert!(stdout.contains("nextmind.metrics label=after state="));
    assert!(stdout.contains("nextmind.metrics label=current state="));
    assert!(stdout.contains("verified-core=true"));
    assert!(stdout.contains("violations=0"));
    assert!(stdout.contains("nextmind.action reason="));
    assert!(stdout.contains("nextmind.verdict=improved"));
    assert!(stdout.contains("nextmind.explain trigger="));
    assert!(stdout.contains("verdict=improved thresholds=runq>3,cpu>=75,socket>=80,event>=75"));
    assert!(stdout.contains("channel=proc::"));
    assert!(stdout.contains("nextmind.auto=on"));
    assert!(stdout.contains("nextmind.auto trigger="));
    assert!(stdout.contains("nextmind.auto=off"));
}

#[test]
fn native_shell_reports_empty_global_watch_poll_after_all_sessions_are_stopped() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /games/comet\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/comet.manifest\nappend-line /games/comet.manifest title=Comet Arena\nappend-line /games/comet.manifest slug=comet-arena\nappend-line /games/comet.manifest exec=/bin/worker\nappend-line /games/comet.manifest cwd=/games/comet\nappend-line /games/comet.manifest arg=--windowed\nappend-line /games/comet.manifest gfx.backend=vulkan\nappend-line /games/comet.manifest gfx.profile=frame-pace\nappend-line /games/comet.manifest audio.backend=native-mixer\nappend-line /games/comet.manifest audio.profile=arena-mix\nappend-line /games/comet.manifest input.backend=native-input\nappend-line /games/comet.manifest input.profile=kbm-first\nappend-line /games/comet.manifest shim.prefix=/compat/comet\nappend-line /games/comet.manifest shim.saves=/saves/comet\nappend-line /games/comet.manifest shim.cache=/cache/comet\ngame-launch /games/orbit.manifest\ngame-launch /games/comet.manifest\ngame-watch-start 77 audio\ngame-watch-start 78 input\ngame-stop 77\ngame-stop 78\ngame-watch-status-all\ngame-watch-poll-all\nlast-status\ngame-sessions\nexit 0\n",
    ));
    let argv = ["ngos-userland-native"];
    let envp = [
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

    assert_eq!(main(&runtime, &bootstrap), 0);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains(
        "game.watch.summary pid=77 slug=orbit-runner kind=audio queue=inactive token=inactive"
    ));
    assert!(stdout.contains(
        "game.watch.summary pid=78 slug=comet-arena kind=input queue=inactive token=inactive"
    ));
    assert!(stdout.contains("last-status=299"));
    assert!(stdout.contains(
        "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=true exit="
    ));
    assert!(stdout.contains(
        "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=true exit="
    ));
}
