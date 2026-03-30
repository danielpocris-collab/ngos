use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::fmt::Write;
use core::ptr;
use core::slice;
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use ngos_user_abi::POLLPRI;
use ngos_user_abi::{
    BootSessionReport, BootSessionStage, BootSessionStatus, Errno, NativeContractKind,
    NativeContractRecord, NativeContractState, NativeDeviceRecord, NativeDomainRecord,
    NativeDriverRecord, NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind,
    NativeFileStatusRecord, NativeFileSystemStatusRecord, NativeObjectKind, NativeProcessRecord,
    NativeResourceArbitrationPolicy, NativeResourceCancelRecord, NativeResourceClaimRecord,
    NativeResourceContractPolicy, NativeResourceEventWatchConfig, NativeResourceGovernanceMode,
    NativeResourceIssuerPolicy, NativeResourceKind, NativeResourceRecord,
    NativeResourceReleaseRecord, NativeResourceState, NativeSchedulerClass,
    NativeSpawnProcessConfig, POLLIN, POLLOUT, SYS_ACQUIRE_RESOURCE, SYS_ADVISE_MEMORY_RANGE,
    SYS_BIND_PROCESS_CONTRACT, SYS_BOOT_REPORT, SYS_CANCEL_RESOURCE_CLAIM, SYS_CLAIM_RESOURCE,
    SYS_CLOSE, SYS_CREATE_CONTRACT, SYS_CREATE_DOMAIN, SYS_CREATE_EVENT_QUEUE, SYS_CREATE_RESOURCE,
    SYS_DUP, SYS_EXIT, SYS_FCNTL, SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME, SYS_GET_PROCESS_CWD,
    SYS_GET_PROCESS_IMAGE_PATH, SYS_GET_PROCESS_NAME, SYS_GET_RESOURCE_NAME, SYS_INSPECT_CONTRACT,
    SYS_INSPECT_DEVICE, SYS_INSPECT_DOMAIN, SYS_INSPECT_DRIVER, SYS_INSPECT_PROCESS,
    SYS_INSPECT_RESOURCE, SYS_INVOKE_CONTRACT, SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS,
    SYS_LIST_PROCESSES, SYS_LIST_RESOURCE_WAITERS, SYS_LIST_RESOURCES, SYS_LOAD_MEMORY_WORD,
    SYS_LSTAT_PATH, SYS_MAP_ANONYMOUS_MEMORY, SYS_MAP_FILE_MEMORY, SYS_MKCHAN_PATH, SYS_MKDIR_PATH,
    SYS_MKFILE_PATH, SYS_OPEN_PATH, SYS_POLL, SYS_PROTECT_MEMORY_RANGE, SYS_QUARANTINE_VM_OBJECT,
    SYS_READ, SYS_READ_PROCFS, SYS_READLINK_PATH, SYS_READV, SYS_REAP_PROCESS,
    SYS_RECLAIM_MEMORY_PRESSURE, SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL, SYS_RELEASE_CLAIMED_RESOURCE,
    SYS_RELEASE_RESOURCE, SYS_RELEASE_VM_OBJECT, SYS_REMOVE_RESOURCE_EVENTS, SYS_RENAME_PATH,
    SYS_SEND_SIGNAL, SYS_SET_CONTRACT_STATE, SYS_SET_PROCESS_ARGS, SYS_SET_PROCESS_BREAK,
    SYS_SET_PROCESS_CWD, SYS_SET_PROCESS_ENV, SYS_SET_RESOURCE_CONTRACT_POLICY,
    SYS_SET_RESOURCE_GOVERNANCE, SYS_SET_RESOURCE_ISSUER_POLICY, SYS_SET_RESOURCE_POLICY,
    SYS_SET_RESOURCE_STATE, SYS_SPAWN_CONFIGURED_PROCESS, SYS_SPAWN_PATH_PROCESS,
    SYS_SPAWN_PROCESS_COPY_VM, SYS_STAT_PATH, SYS_STATFS_PATH, SYS_STORE_MEMORY_WORD,
    SYS_SYMLINK_PATH, SYS_SYNC_MEMORY_RANGE, SYS_TRANSFER_RESOURCE, SYS_UNLINK_PATH,
    SYS_UNMAP_MEMORY_RANGE, SYS_WAIT_EVENT_QUEUE, SYS_WATCH_RESOURCE_EVENTS, SYS_WRITE, SYS_WRITEV,
    SyscallFrame, SyscallReturn, UserIoVec,
};

use crate::diagnostics::{self, DiagnosticsPath, GuardKind, WatchKind};
use crate::serial;
use crate::tty;
use crate::user_runtime_status;

#[cfg(not(test))]
fn syscall_trace(_args: core::fmt::Arguments<'_>) {}

#[cfg(test)]
fn syscall_trace(_args: core::fmt::Arguments<'_>) {}

const MAX_DESCRIPTOR_COUNT: usize = 8;
const MAX_DOMAIN_COUNT: usize = 16;
const MAX_RESOURCE_COUNT: usize = 16;
const MAX_CONTRACT_COUNT: usize = 32;
const MAX_NAME_LEN: usize = 32;
const MAX_PROCESS_COUNT: usize = 16;
const MAX_EVENT_QUEUE_COUNT: usize = 8;
const MAX_EVENT_QUEUE_WATCH_COUNT: usize = 16;
const MAX_EVENT_QUEUE_PENDING: usize = 32;
const BOOT_OWNER_ID: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
enum SyscallDisposition {
    Return = 0,
    Halt = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DescriptorTarget {
    Stdin,
    Stdout,
    Stderr,
    EventQueue(usize),
    StorageDevice,
    StorageDriver,
    GpuDevice,
    AudioDevice,
    InputDevice,
    BootFile(usize),
    BootChannel(usize),
    Procfs(BootProcfsNode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootProcfsNodeKind {
    Status,
    Maps,
    VmObjects,
    VmDecisions,
    VmEpisodes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootProcfsNode {
    pid: u64,
    kind: BootProcfsNodeKind,
}

struct ProcfsLineBuffer {
    bytes: [u8; 384],
    len: usize,
}

impl ProcfsLineBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; 384],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl Write for ProcfsLineBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        if self.len + bytes.len() > self.bytes.len() {
            return Err(core::fmt::Error);
        }
        self.bytes[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DescriptorFlags {
    nonblock: bool,
    cloexec: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DescriptorState {
    target: DescriptorTarget,
    flags: DescriptorFlags,
    offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DescriptorTable {
    slots: [Option<DescriptorState>; MAX_DESCRIPTOR_COUNT],
}

struct DescriptorTableCell(UnsafeCell<DescriptorTable>);
struct NativeRegistryCell(UnsafeCell<NativeRegistry>);
struct BootEventQueueRegistryCell(UnsafeCell<BootEventQueueRegistry>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodedFcntl {
    GetFl,
    GetFd,
    SetFl { nonblock: bool },
    SetFd { cloexec: bool },
}

#[repr(C)]
pub struct SyscallDispatchResult {
    pub raw_return: usize,
    pub disposition: u64,
}

static mut PROCESS_EXIT_CODE: i32 = 0;
static mut PROCESS_EXITED: bool = false;
static DESCRIPTORS: DescriptorTableCell = DescriptorTableCell::new();
static NATIVE_REGISTRY: NativeRegistryCell = NativeRegistryCell::new();
static BOOT_VFS: BootVfsCell = BootVfsCell::new();
static BOOT_PROCESSES: BootProcessRegistryCell = BootProcessRegistryCell::new();
static BOOT_EVENT_QUEUES: BootEventQueueRegistryCell = BootEventQueueRegistryCell::new();

const FN_CLAIM_RESOURCE: u64 = 1;
const FN_RELEASE_CLAIMED_RESOURCE: u64 = 2;
const FN_SET_RESOURCE_GOVERNANCE: u64 = 3;
const FN_SET_RESOURCE_CONTRACT_POLICY: u64 = 4;
const FN_SET_RESOURCE_STATE: u64 = 5;
const FN_CREATE_CONTRACT: u64 = 6;
const FN_SET_RESOURCE_ISSUER_POLICY: u64 = 7;
const GPU_DEVICE_PATH: &str = "/dev/gpu0";
const AUDIO_DEVICE_PATH: &str = "/dev/audio0";
const INPUT_DEVICE_PATH: &str = "/dev/input0";

unsafe impl Sync for DescriptorTableCell {}
unsafe impl Sync for NativeRegistryCell {}
unsafe impl Sync for BootVfsCell {}
unsafe impl Sync for BootProcessRegistryCell {}
unsafe impl Sync for BootEventQueueRegistryCell {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum BootResourceEventKind {
    Claimed = 0,
    Queued = 1,
    Canceled = 2,
    Released = 3,
    HandedOff = 4,
    Revoked = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResourceEventWatch {
    resource: u64,
    token: u64,
    events: u32,
    claimed: bool,
    queued: bool,
    canceled: bool,
    released: bool,
    handed_off: bool,
    revoked: bool,
}

#[derive(Debug, Clone)]
struct BootEventQueueEntry {
    id: usize,
    mode: NativeEventQueueMode,
    pending: Vec<NativeEventRecord>,
    resource_watches: Vec<ResourceEventWatch>,
}

#[derive(Debug)]
struct BootEventQueueRegistry {
    next_id: usize,
    queues: [Option<BootEventQueueEntry>; MAX_EVENT_QUEUE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootNodeKind {
    Directory,
    File,
    Channel,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootNode {
    path: String,
    kind: BootNodeKind,
    inode: u64,
    bytes: Vec<u8>,
    link_target: Option<String>,
}

#[derive(Debug, Default)]
struct BootVfs {
    next_inode: u64,
    nodes: Vec<BootNode>,
}

struct BootVfsCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<BootVfs>>,
}

impl BootVfsCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootVfs) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let state = unsafe { &mut *self.state.get() };
        if state.is_none() {
            *state = Some(BootVfs::new());
        }
        let result = f(state.as_mut().unwrap());
        self.locked.store(false, Ordering::Release);
        result
    }
}

impl BootEventQueueRegistryCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(BootEventQueueRegistry::new()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootEventQueueRegistry) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }

    fn with<R>(&self, f: impl FnOnce(&BootEventQueueRegistry) -> R) -> R {
        unsafe { f(&*self.0.get()) }
    }
}

impl BootVfs {
    fn new() -> Self {
        Self {
            next_inode: 0x424f_4f54_5653_0001,
            nodes: vec![BootNode {
                path: String::from("/"),
                kind: BootNodeKind::Directory,
                inode: 0x424f_4f54_5653_0000,
                bytes: Vec::new(),
                link_target: None,
            }],
        }
    }

    fn find_node(&self, path: &str) -> Option<usize> {
        self.nodes.iter().position(|node| node.path == path)
    }

    fn normalize_path(path: &str) -> Result<String, Errno> {
        if path.is_empty() || !path.starts_with('/') {
            return Err(Errno::Inval);
        }
        if path == "/" {
            return Ok(String::from("/"));
        }
        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() {
            Ok(String::from("/"))
        } else {
            Ok(String::from(trimmed))
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

    fn ensure_parent_directory(&self, path: &str) -> Result<(), Errno> {
        let parent = Self::parent_path(path);
        let Some(index) = self.find_node(parent) else {
            return Err(Errno::NoEnt);
        };
        if self.nodes[index].kind != BootNodeKind::Directory {
            return Err(Errno::NotDir);
        }
        Ok(())
    }

    fn create(&mut self, path: &str, kind: BootNodeKind) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        if path == "/" {
            return Err(Errno::Exist);
        }
        self.ensure_parent_directory(&path)?;
        if self.find_node(&path).is_some() {
            return Err(Errno::Exist);
        }
        let inode = self.next_inode;
        self.next_inode = self.next_inode.saturating_add(1);
        self.nodes.push(BootNode {
            path,
            kind,
            inode,
            bytes: Vec::new(),
            link_target: None,
        });
        Ok(())
    }

    fn create_symlink(&mut self, path: &str, target: &str) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        let target = Self::normalize_path(target)?;
        if path == "/" {
            return Err(Errno::Exist);
        }
        self.ensure_parent_directory(&path)?;
        if self.find_node(&path).is_some() {
            return Err(Errno::Exist);
        }
        let inode = self.next_inode;
        self.next_inode = self.next_inode.saturating_add(1);
        self.nodes.push(BootNode {
            path,
            kind: BootNodeKind::Symlink,
            inode,
            bytes: Vec::new(),
            link_target: Some(target),
        });
        Ok(())
    }

    fn resolve_node_index(&self, path: &str, follow_symlink: bool) -> Result<usize, Errno> {
        self.resolve_node_index_depth(path, follow_symlink, 0)
    }

    fn resolve_node_index_depth(
        &self,
        path: &str,
        follow_symlink: bool,
        depth: usize,
    ) -> Result<usize, Errno> {
        if depth > 8 {
            return Err(Errno::Inval);
        }
        let path = Self::normalize_path(path)?;
        let Some(index) = self.find_node(&path) else {
            return Err(Errno::NoEnt);
        };
        let node = &self.nodes[index];
        if follow_symlink && node.kind == BootNodeKind::Symlink {
            let target = node.link_target.as_deref().ok_or(Errno::Inval)?;
            return self.resolve_node_index_depth(target, true, depth + 1);
        }
        Ok(index)
    }

    fn stat(&self, path: &str, follow_symlink: bool) -> Option<NativeFileStatusRecord> {
        let path = Self::normalize_path(path).ok()?;
        let index = self.resolve_node_index(&path, follow_symlink).ok()?;
        let node = self.nodes.get(index)?;
        let (kind, readable, writable) = match node.kind {
            BootNodeKind::Directory => (NativeObjectKind::Directory as u32, 1, 0),
            BootNodeKind::File => (NativeObjectKind::File as u32, 1, 1),
            BootNodeKind::Channel => (NativeObjectKind::Channel as u32, 1, 1),
            BootNodeKind::Symlink => (NativeObjectKind::Symlink as u32, 1, 0),
        };
        Some(NativeFileStatusRecord {
            inode: node.inode,
            size: node
                .link_target
                .as_ref()
                .map(|target| target.len())
                .unwrap_or_else(|| node.bytes.len()) as u64,
            kind,
            cloexec: 0,
            nonblock: 0,
            readable,
            writable,
        })
    }

    fn file_size(&self, path: &str) -> Result<usize, Errno> {
        let index = self.resolve_node_index(path, true)?;
        let node = &self.nodes[index];
        match node.kind {
            BootNodeKind::Directory => Err(Errno::IsDir),
            BootNodeKind::File | BootNodeKind::Channel => Ok(node.bytes.len()),
            BootNodeKind::Symlink => Err(Errno::Inval),
        }
    }

    fn readlink(&self, path: &str) -> Result<&str, Errno> {
        let path = Self::normalize_path(path)?;
        let Some(index) = self.find_node(&path) else {
            return Err(Errno::NoEnt);
        };
        let node = &self.nodes[index];
        if node.kind != BootNodeKind::Symlink {
            return Err(Errno::Inval);
        }
        node.link_target.as_deref().ok_or(Errno::Inval)
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), Errno> {
        let from = Self::normalize_path(from)?;
        let to = Self::normalize_path(to)?;
        if from == "/" || to == "/" {
            return Err(Errno::Inval);
        }
        if to == from || to.starts_with(&(from.clone() + "/")) {
            return Err(Errno::Inval);
        }
        self.ensure_parent_directory(&to)?;
        let Some(index) = self.find_node(&from) else {
            return Err(Errno::NoEnt);
        };
        if self.find_node(&to).is_some() {
            return Err(Errno::Exist);
        }
        if self.nodes[index].kind == BootNodeKind::Directory {
            let from_prefix = from.clone() + "/";
            let to_prefix = to.clone() + "/";
            for node in &mut self.nodes {
                if node.path == from {
                    node.path = to.clone();
                } else if node.path.starts_with(&from_prefix) {
                    node.path = format!("{}{}", to_prefix, &node.path[from_prefix.len()..]);
                }
            }
        } else {
            self.nodes[index].path = to;
        }
        Ok(())
    }

    fn unlink(&mut self, path: &str) -> Result<(), Errno> {
        let path = Self::normalize_path(path)?;
        if path == "/" {
            return Err(Errno::Inval);
        }
        let Some(index) = self.find_node(&path) else {
            return Err(Errno::NoEnt);
        };
        if self.nodes[index].kind == BootNodeKind::Directory {
            let prefix = path.clone() + "/";
            if self.nodes.iter().any(|node| node.path.starts_with(&prefix)) {
                return Err(Errno::Busy);
            }
        }
        self.nodes.remove(index);
        Ok(())
    }
}

impl BootEventQueueRegistry {
    const fn new() -> Self {
        Self {
            next_id: 1,
            queues: [None, None, None, None, None, None, None, None],
        }
    }

    fn create_queue(&mut self, mode: NativeEventQueueMode) -> Result<usize, Errno> {
        let Some(slot) = self.queues.iter().position(Option::is_none) else {
            return Err(Errno::Again);
        };
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.queues[slot] = Some(BootEventQueueEntry {
            id,
            mode,
            pending: Vec::new(),
            resource_watches: Vec::new(),
        });
        Ok(id)
    }

    fn queue(&self, id: usize) -> Result<&BootEventQueueEntry, Errno> {
        self.queues
            .iter()
            .flatten()
            .find(|entry| entry.id == id)
            .ok_or(Errno::Badf)
    }

    fn queue_mut(&mut self, id: usize) -> Result<&mut BootEventQueueEntry, Errno> {
        self.queues
            .iter_mut()
            .flatten()
            .find(|entry| entry.id == id)
            .ok_or(Errno::Badf)
    }

    fn remove_queue(&mut self, id: usize) {
        if let Some(slot) = self
            .queues
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|queue| queue.id == id))
        {
            self.queues[slot] = None;
        }
    }
}

fn boot_vfs_stat(path: &str) -> Option<NativeFileStatusRecord> {
    BOOT_VFS.with_mut(|vfs| vfs.stat(path, true))
}

fn boot_vfs_lstat(path: &str) -> Option<NativeFileStatusRecord> {
    BOOT_VFS.with_mut(|vfs| vfs.stat(path, false))
}

fn boot_vfs_create(path: &str, kind: BootNodeKind) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.create(path, kind))
}

fn boot_vfs_symlink(path: &str, target: &str) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.create_symlink(path, target))
}

fn boot_vfs_file_size(path: &str) -> Result<usize, Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.file_size(path))
}

fn boot_vfs_readlink(path: &str) -> Result<String, Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.readlink(path).map(String::from))
}

fn boot_vfs_rename(from: &str, to: &str) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.rename(from, to))
}

fn boot_vfs_unlink(path: &str) -> Result<(), Errno> {
    BOOT_VFS.with_mut(|vfs| vfs.unlink(path))
}

fn boot_vfs_lookup_target(path: &str) -> Result<DescriptorTarget, Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let index = vfs.resolve_node_index(path, true)?;
        match vfs.nodes[index].kind {
            BootNodeKind::Directory => Err(Errno::IsDir),
            BootNodeKind::File => Ok(DescriptorTarget::BootFile(index)),
            BootNodeKind::Channel => Ok(DescriptorTarget::BootChannel(index)),
            BootNodeKind::Symlink => Err(Errno::Inval),
        }
    })
}

fn boot_procfs_node(path: &str) -> Result<Option<BootProcfsNode>, Errno> {
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    if segments.next() != Some("proc") {
        return Ok(None);
    }
    let pid = segments
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(Errno::Inval)?;
    let kind = match segments.next().ok_or(Errno::Inval)? {
        "status" => BootProcfsNodeKind::Status,
        "maps" => BootProcfsNodeKind::Maps,
        "vmobjects" => BootProcfsNodeKind::VmObjects,
        "vmdecisions" => BootProcfsNodeKind::VmDecisions,
        "vmepisodes" => BootProcfsNodeKind::VmEpisodes,
        _ => return Err(Errno::NoEnt),
    };
    if segments.next().is_some() {
        return Err(Errno::NoEnt);
    }
    Ok(Some(BootProcfsNode { pid, kind }))
}

fn boot_procfs_payload(pid: u64, kind: BootProcfsNodeKind) -> Result<String, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        let text = match kind {
            BootProcfsNodeKind::Status => format!(
                "Name:\t{}\nState:\t{}\nPid:\t{}\nCwd:\t{}\nVmObjects:\t{}\n",
                entry.name,
                if entry.state == 2 { "Running" } else { "Exited" },
                entry.pid,
                entry.cwd,
                entry.vm_objects.len()
            ),
            BootProcfsNodeKind::Maps => {
                let mut text = String::new();
                for object in &entry.vm_objects {
                    let perms = if object.quarantined {
                        "----"
                    } else {
                        match (object.readable, object.writable, object.executable) {
                            (true, true, true) => "rwxp",
                            (true, true, false) => "rw-p",
                            (true, false, true) => "r-xp",
                            (true, false, false) => "r--p",
                            (false, true, false) => "-w-p",
                            (false, true, true) => "-wxp",
                            (false, false, true) => "--xp",
                            (false, false, false) => "----",
                        }
                    };
                    text.push_str(&format!(
                        "{:016x}-{:016x} {} {:08x} {}\n",
                        object.start,
                        object.start.saturating_add(object.len),
                        perms,
                        object.file_offset,
                        object.name
                    ));
                }
                text
            }
            BootProcfsNodeKind::VmObjects => {
                let mut text = String::new();
                for object in &entry.vm_objects {
                    let owners = boot_vm_owner_count(registry, object.share_key);
                    let (segment_count, resident_segment_count) = boot_vm_segment_counts(object);
                    let shadow = object
                        .shadow_source_id
                        .map(|source| {
                            format!(
                                "\tshadow={:08x}@{:08x}/depth={}",
                                source, object.shadow_source_offset, object.shadow_depth
                            )
                        })
                        .unwrap_or_default();
                    text.push_str(&format!(
                        "{:08x}\t{}\tprivate={}\towners={}\toffset={:08x}\tcommitted={}\tresident={}\tdirty={}\taccessed={}\tsegments={}\tresident-segments={}\tfaults={}(r={},w={},cow={})\t{}\treadable={}\twritable={}\texecutable={}\tquarantined={}\treason={}{}\n",
                        object.id,
                        object.kind,
                        object.private_mapping,
                        owners,
                        object.file_offset,
                        object.committed_pages,
                        object.resident_pages,
                        object.dirty_pages,
                        object.accessed_pages,
                        segment_count,
                        resident_segment_count,
                        object.read_fault_count
                            .saturating_add(object.write_fault_count)
                            .saturating_add(object.cow_fault_count),
                        object.read_fault_count,
                        object.write_fault_count,
                        object.cow_fault_count,
                        object.name,
                        object.readable as u8,
                        object.writable as u8,
                        object.executable as u8,
                        object.quarantined as u8,
                        object.quarantine_reason,
                        shadow,
                    ));
                }
                text
            }
            BootProcfsNodeKind::VmDecisions => {
                let mut text = String::new();
                for (tick, decision) in entry.vm_decisions.iter().enumerate() {
                    push_vm_decision_line(&mut text, entry, tick, decision).map_err(|_| Errno::Io)?;
                }
                text
            }
            BootProcfsNodeKind::VmEpisodes => {
                let mut text = String::new();
                boot_procfs_write_vm_episodes(&mut text, entry).map_err(|_| Errno::Io)?;
                text
            }
        };
        Ok(text)
    })
}

fn boot_procfs_read(
    node: BootProcfsNode,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    if len == 0 {
        return Ok(0);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    match node.kind {
        BootProcfsNodeKind::Maps => {
            return boot_procfs_read_maps(node.pid, offset, buffer, len);
        }
        BootProcfsNodeKind::VmObjects => {
            return boot_procfs_read_vm_objects(node.pid, offset, buffer, len);
        }
        BootProcfsNodeKind::VmDecisions => {
            return boot_procfs_read_vm_decisions(node.pid, offset, buffer, len);
        }
        BootProcfsNodeKind::VmEpisodes => {
            return boot_procfs_read_vm_episodes(node.pid, offset, buffer, len);
        }
        _ => {}
    }
    let payload = boot_procfs_payload(node.pid, node.kind)?;
    let bytes = payload.as_bytes();
    let count = bytes.len().saturating_sub(*offset).min(len);
    if count == 0 {
        return Ok(0);
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes[*offset..*offset + count].as_ptr(), buffer, count);
    }
    *offset += count;
    Ok(count)
}

fn boot_procfs_read_maps(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|table| {
        let Some(index) = table.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &table.entries[index];
        let mut absolute_offset = 0usize;
        let mut written = 0usize;
        for object in &entry.vm_objects {
            let perms = if object.quarantined {
                "----"
            } else {
                match (object.readable, object.writable, object.executable) {
                    (true, true, true) => "rwxp",
                    (true, true, false) => "rw-p",
                    (true, false, true) => "r-xp",
                    (true, false, false) => "r--p",
                    (false, true, false) => "-w-p",
                    (false, true, true) => "-wxp",
                    (false, false, true) => "--xp",
                    (false, false, false) => "----",
                }
            };
            let mut line = ProcfsLineBuffer::new();
            write!(
                &mut line,
                "{:016x}-{:016x} {} {:08x} {}\n",
                object.start,
                object.start.saturating_add(object.len),
                perms,
                object.file_offset,
                object.name
            )
            .map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }
        Ok(written)
    })
}

fn boot_procfs_read_vm_decisions(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|table| {
        let Some(index) = table.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &table.entries[index];
        let mut absolute_offset = 0usize;
        let mut written = 0usize;
        for (tick, decision) in entry.vm_decisions.iter().enumerate() {
            let mut line = ProcfsLineBuffer::new();
            push_vm_decision_line(&mut line, entry, tick, decision).map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }
        Ok(written)
    })
}

fn push_vm_object_line(
    line: &mut ProcfsLineBuffer,
    registry: &BootProcessRegistry,
    object: &BootVmObject,
) -> core::fmt::Result {
    let owners = boot_vm_owner_count(registry, object.share_key);
    let (segment_count, resident_segment_count) = boot_vm_segment_counts(object);
    write!(
        line,
        "{:08x}\t{}\tprivate={}\towners={}\toffset={:08x}\tcommitted={}\tresident={}\tdirty={}\taccessed={}\tsegments={}\tresident-segments={}\tfaults={}(r={},w={},cow={})\t{}\treadable={}\twritable={}\texecutable={}\tquarantined={}\treason={}",
        object.id,
        object.kind,
        object.private_mapping,
        owners,
        object.file_offset,
        object.committed_pages,
        object.resident_pages,
        object.dirty_pages,
        object.accessed_pages,
        segment_count,
        resident_segment_count,
        object
            .read_fault_count
            .saturating_add(object.write_fault_count)
            .saturating_add(object.cow_fault_count),
        object.read_fault_count,
        object.write_fault_count,
        object.cow_fault_count,
        object.name,
        object.readable as u8,
        object.writable as u8,
        object.executable as u8,
        object.quarantined as u8,
        object.quarantine_reason,
    )?;
    if let Some(source) = object.shadow_source_id {
        write!(
            line,
            "\tshadow={:08x}@{:08x}/depth={}",
            source, object.shadow_source_offset, object.shadow_depth
        )?;
    }
    line.write_str("\n")
}

fn boot_procfs_read_vm_objects(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &registry.entries[index];
        let mut absolute_offset = 0usize;
        let mut written = 0usize;
        for object in &entry.vm_objects {
            let mut line = ProcfsLineBuffer::new();
            push_vm_object_line(&mut line, registry, object).map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }
        Ok(written)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootVmEpisodeRecord {
    start_tick: usize,
    vm_object_id: u64,
    kind: &'static str,
    fields: String,
}

fn boot_vm_episode_object_ids(entry: &BootProcessEntry) -> Vec<u64> {
    let mut ids = Vec::new();
    for object in &entry.vm_objects {
        if !ids.contains(&object.id) {
            ids.push(object.id);
        }
    }
    for decision in &entry.vm_decisions {
        if decision.vm_object_id == 0 || ids.contains(&decision.vm_object_id) {
            continue;
        }
        ids.push(decision.vm_object_id);
    }
    ids
}

fn boot_vm_episode_mapped_kind(decision: &BootVmDecision) -> &'static str {
    match decision.agent {
        "map" => "anon",
        "map-file" if (decision.detail1 & (1 << 3)) != 0 => "file-private",
        "map-file" => "file-shared",
        _ => "unknown",
    }
}

fn boot_collect_vm_episode_records(entry: &BootProcessEntry) -> Vec<BootVmEpisodeRecord> {
    let mut records = Vec::<BootVmEpisodeRecord>::new();

    let mut policy_start = None;
    let mut policy_end = 0usize;
    let mut policy_state = 0u64;
    let mut policy_operation = 0u64;
    let mut policy_blocked = false;
    let mut policy_decisions = 0usize;
    for (tick, decision) in entry.vm_decisions.iter().enumerate() {
        if decision.agent != "policy-block" {
            continue;
        }
        if policy_start.is_none() {
            policy_start = Some(tick + 1);
        }
        policy_end = tick + 1;
        policy_state = decision.detail0;
        policy_operation = decision.detail1;
        policy_blocked = true;
        policy_decisions += 1;
    }
    if let Some(start_tick) = policy_start {
        records.push(BootVmEpisodeRecord {
            start_tick,
            vm_object_id: 0,
            kind: "policy",
            fields: format!(
                "start-tick={}\tend-tick={}\tstate={}\toperation={}\tblocked={}\tdecisions={}\tlast=policy-block",
                start_tick,
                policy_end.max(start_tick),
                policy_state,
                policy_operation,
                if policy_blocked { "yes" } else { "no" },
                policy_decisions.max(1),
            ),
        });
    }

    for object_id in boot_vm_episode_object_ids(entry) {
        let resident_pages = entry
            .vm_objects
            .iter()
            .find(|object| object.id == object_id)
            .map(|object| object.resident_pages)
            .unwrap_or(0);

        let mut map_start = None;
        let mut map_end = 0usize;
        let mut map_decisions = 0usize;
        let mut mapped_kind = "unknown";
        let mut map_last = "";

        let mut heap_start = None;
        let mut heap_end = 0usize;
        let mut heap_decisions = 0usize;
        let mut heap_old_end = 0u64;
        let mut heap_new_end = 0u64;
        let mut heap_grew = false;
        let mut heap_shrank = false;
        let mut heap_last = "";

        let mut quarantine_start = None;
        let mut quarantine_end = 0usize;
        let mut quarantine_reason = 0u64;
        let mut quarantine_blocked = false;
        let mut quarantine_released = false;
        let mut quarantine_decisions = 0usize;
        let mut quarantine_last = "";

        let mut reclaim_start = None;
        let mut reclaim_end = 0usize;
        let mut reclaim_decisions = 0usize;
        let mut reclaim_evicted = false;
        let mut reclaim_restored = false;
        let mut reclaim_last = "";

        let mut fault_start = None;
        let mut fault_end = 0usize;
        let mut fault_decisions = 0usize;
        let mut faulted = false;
        let mut cow = false;
        let mut bridged = false;
        let mut touched = false;
        let mut synced = false;
        let mut advised = false;
        let mut fault_last = "";

        let mut region_start = None;
        let mut region_end = 0usize;
        let mut region_decisions = 0usize;
        let mut region_protected = false;
        let mut region_unmapped = false;
        let mut region_last = "";

        for (tick, decision) in entry.vm_decisions.iter().enumerate() {
            if decision.vm_object_id != object_id {
                continue;
            }
            match decision.agent {
                "map" | "map-file" => {
                    if map_start.is_none() {
                        map_start = Some(tick + 1);
                    }
                    map_end = tick + 1;
                    map_decisions += 1;
                    mapped_kind = boot_vm_episode_mapped_kind(decision);
                    map_last = decision.agent;
                }
                "brk" => {
                    if heap_start.is_none() {
                        heap_start = Some(tick + 1);
                        heap_old_end = decision.detail0;
                    }
                    heap_end = tick + 1;
                    heap_decisions += 1;
                    heap_old_end = heap_old_end.min(decision.detail0);
                    heap_new_end = decision.detail1;
                    heap_grew |= decision.detail1 > decision.detail0;
                    heap_shrank |= decision.detail1 < decision.detail0;
                    heap_last = decision.agent;
                }
                "quarantine-state" => {
                    if decision.detail1 == 1 && quarantine_start.is_none() {
                        quarantine_start = Some(tick + 1);
                        quarantine_reason = decision.detail0;
                    }
                    if quarantine_start.is_some() {
                        quarantine_end = tick + 1;
                        quarantine_decisions += 1;
                        quarantine_last = "quarantine-state";
                        if decision.detail1 == 0 {
                            quarantine_released = true;
                        }
                    }
                }
                "quarantine-block" => {
                    if quarantine_start.is_some() {
                        quarantine_end = tick + 1;
                        quarantine_decisions += 1;
                        quarantine_last = "quarantine-block";
                        quarantine_blocked = true;
                    }
                }
                "pressure-victim" => {
                    if reclaim_start.is_none() {
                        reclaim_start = Some(tick + 1);
                    }
                    reclaim_end = tick + 1;
                    reclaim_decisions += 1;
                    reclaim_last = "pressure-victim";
                }
                "advice" if reclaim_start.is_some() => {
                    reclaim_end = tick + 1;
                    reclaim_decisions += 1;
                    reclaim_last = "advice";
                    if decision.detail0 == 4 {
                        reclaim_evicted = true;
                    } else if decision.detail0 == 3 {
                        reclaim_restored = true;
                    }
                }
                "fault-classifier" | "page-touch" | "sync" | "advice" | "cow-populate"
                | "shadow-reuse" | "shadow-bridge" => {
                    if fault_start.is_none() {
                        fault_start = Some(tick + 1);
                    }
                    fault_end = tick + 1;
                    fault_decisions += 1;
                    fault_last = decision.agent;
                    if decision.agent == "fault-classifier" {
                        faulted = true;
                        if reclaim_start.is_some() {
                            reclaim_restored = true;
                        }
                    } else if decision.agent == "page-touch" {
                        touched = true;
                        if reclaim_start.is_some() {
                            reclaim_restored = true;
                        }
                    } else if decision.agent == "sync" {
                        synced = true;
                        if reclaim_start.is_some() {
                            reclaim_restored = true;
                        }
                    } else if decision.agent == "advice" {
                        advised = true;
                    } else if decision.agent == "shadow-bridge" {
                        bridged = true;
                    } else {
                        cow = true;
                    }
                    if reclaim_start.is_some() {
                        reclaim_end = tick + 1;
                        reclaim_decisions += 1;
                        reclaim_last = decision.agent;
                    }
                }
                "protect" | "unmap" => {
                    if region_start.is_none() {
                        region_start = Some(tick + 1);
                    }
                    region_end = tick + 1;
                    region_decisions += 1;
                    region_last = decision.agent;
                    if decision.agent == "protect" {
                        region_protected = true;
                    } else {
                        region_unmapped = true;
                    }
                }
                _ => {}
            }
        }

        if let Some(start_tick) = map_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "map",
                fields: format!(
                    "start-tick={}\tend-tick={}\tmapped={}\tdecisions={}\tlast={}",
                    start_tick,
                    map_end.max(start_tick),
                    mapped_kind,
                    map_decisions.max(1),
                    if map_last.is_empty() { "map" } else { map_last },
                ),
            });
        }

        if let Some(start_tick) = heap_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "heap",
                fields: format!(
                    "start-tick={}\tend-tick={}\tgrew={}\tshrank={}\told-end={}\tnew-end={}\tdecisions={}\tlast={}",
                    start_tick,
                    heap_end.max(start_tick),
                    if heap_grew { "yes" } else { "no" },
                    if heap_shrank { "yes" } else { "no" },
                    heap_old_end,
                    heap_new_end,
                    heap_decisions.max(1),
                    if heap_last.is_empty() { "brk" } else { heap_last },
                ),
            });
        }

        if let Some(start_tick) = quarantine_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "quarantine",
                fields: format!(
                    "start-tick={}\tend-tick={}\treason={}\tblocked={}\treleased={}\tdecisions={}\tlast={}",
                    start_tick,
                    quarantine_end.max(start_tick),
                    quarantine_reason,
                    if quarantine_blocked { "yes" } else { "no" },
                    if quarantine_released { "yes" } else { "no" },
                    quarantine_decisions.max(1),
                    if quarantine_last.is_empty() {
                        "quarantine-state"
                    } else {
                        quarantine_last
                    },
                ),
            });
        }

        if let Some(start_tick) = reclaim_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "reclaim",
                fields: format!(
                    "start-tick={}\tend-tick={}\tevicted={}\trestored={}\tdecisions={}\tlast={}",
                    start_tick,
                    reclaim_end.max(start_tick),
                    if reclaim_evicted || resident_pages == 0 {
                        "yes"
                    } else {
                        "no"
                    },
                    if reclaim_restored { "yes" } else { "no" },
                    reclaim_decisions.max(1),
                    if reclaim_last.is_empty() {
                        "pressure-victim"
                    } else {
                        reclaim_last
                    },
                ),
            });
        }

        if let Some(start_tick) = fault_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "fault",
                fields: format!(
                    "start-tick={}\tend-tick={}\tfaulted={}\tcow={}\tbridged={}\ttouched={}\tsynced={}\tadvised={}\tdecisions={}\tlast={}",
                    start_tick,
                    fault_end.max(start_tick),
                    if faulted { "yes" } else { "no" },
                    if cow { "yes" } else { "no" },
                    if bridged { "yes" } else { "no" },
                    if touched { "yes" } else { "no" },
                    if synced { "yes" } else { "no" },
                    if advised { "yes" } else { "no" },
                    fault_decisions.max(1),
                    if fault_last.is_empty() {
                        "fault-classifier"
                    } else {
                        fault_last
                    },
                ),
            });
        }

        if let Some(start_tick) = region_start {
            records.push(BootVmEpisodeRecord {
                start_tick,
                vm_object_id: object_id,
                kind: "region",
                fields: format!(
                    "start-tick={}\tend-tick={}\tprotected={}\tunmapped={}\tdecisions={}\tlast={}",
                    start_tick,
                    region_end.max(start_tick),
                    if region_protected { "yes" } else { "no" },
                    if region_unmapped { "yes" } else { "no" },
                    region_decisions.max(1),
                    if region_last.is_empty() {
                        "protect"
                    } else {
                        region_last
                    },
                ),
            });
        }
    }

    records.sort_by_key(|record| (record.start_tick, record.vm_object_id));
    records
}

fn boot_procfs_write_vm_episodes(
    output: &mut impl Write,
    entry: &BootProcessEntry,
) -> core::fmt::Result {
    let records = boot_collect_vm_episode_records(entry);
    for (episode, record) in records.iter().enumerate() {
        write!(
            output,
            "episode={}\tkind={}\tvm-object={:08x}\t{}\n",
            episode + 1,
            record.kind,
            record.vm_object_id,
            record.fields,
        )?;
    }

    Ok(())
}

fn boot_procfs_read_vm_episodes(
    pid: u64,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid) else {
            return Err(Errno::NoEnt);
        };
        let entry = &registry.entries[index];
        let records = boot_collect_vm_episode_records(entry);
        let mut absolute_offset = 0usize;
        let mut written = 0usize;

        for (episode, record) in records.iter().enumerate() {
            let mut line = ProcfsLineBuffer::new();
            write!(
                &mut line,
                "episode={}\tkind={}\tvm-object={:08x}\t{}\n",
                episode + 1,
                record.kind,
                record.vm_object_id,
                record.fields,
            )
            .map_err(|_| Errno::Io)?;
            let line_bytes = line.as_bytes();
            let line_end = absolute_offset + line_bytes.len();
            if *offset < line_end {
                let start_in_line = (*offset).saturating_sub(absolute_offset);
                let remaining = &line_bytes[start_in_line..];
                let to_copy = remaining.len().min(len - written);
                unsafe {
                    ptr::copy_nonoverlapping(remaining.as_ptr(), buffer.add(written), to_copy);
                }
                written += to_copy;
                *offset += to_copy;
                if written == len {
                    return Ok(written);
                }
            }
            absolute_offset = line_end;
        }

        Ok(written)
    })
}

fn push_vm_decision_line(
    output: &mut impl Write,
    entry: &BootProcessEntry,
    tick: usize,
    decision: &BootVmDecision,
) -> core::fmt::Result {
    let object_name = entry
        .vm_objects
        .iter()
        .find(|object| object.id == decision.vm_object_id)
        .map(|object| object.name.as_str())
        .unwrap_or("-");
    write!(
        output,
        "tick={}\tagent={}\tvm-object={:08x}\tstart={:08x}\tlen={:08x}\tdetail0={}\tdetail1={}\tname={}\n",
        tick + 1,
        decision.agent,
        decision.vm_object_id,
        decision.start,
        decision.len,
        decision.detail0,
        decision.detail1,
        object_name,
    )
}

fn boot_vm_page_count_for_len(len: u64) -> usize {
    (len / 0x1000) as usize
}

fn boot_vm_recount_object_pages(object: &mut BootVmObject) {
    object.committed_pages = object.page_states.len() as u64;
    object.resident_pages = object
        .page_states
        .iter()
        .filter(|page| page.resident)
        .count() as u64;
    object.dirty_pages = object.page_states.iter().filter(|page| page.dirty).count() as u64;
    object.accessed_pages = object
        .page_states
        .iter()
        .filter(|page| page.accessed)
        .count() as u64;
}

fn boot_vm_segment_counts(object: &BootVmObject) -> (usize, usize) {
    if object.page_states.is_empty() {
        return (0, 0);
    }

    let mut segment_count = 0usize;
    let mut resident_segment_count = 0usize;
    let mut current: Option<BootVmPageState> = None;

    for page in &object.page_states {
        if current != Some(*page) {
            segment_count = segment_count.saturating_add(1);
            if page.resident {
                resident_segment_count = resident_segment_count.saturating_add(1);
            }
            current = Some(*page);
        }
    }

    (segment_count, resident_segment_count)
}

fn boot_vm_shadow_label(name: &str) -> String {
    format!("{} [cow]", name)
}

fn boot_vm_page_range(object: &BootVmObject, page_index: usize) -> (u64, u64) {
    let start = object.start + ((page_index as u64) * 0x1000);
    (start, start + 0x1000)
}

fn boot_vm_find_adjacent_shadow_neighbors(
    entry: &BootProcessEntry,
    object_index: usize,
    shadow_source_id: u64,
    shadow_depth: u32,
    shadow_source_offset: u64,
) -> (Option<usize>, Option<usize>) {
    let mut left = None;
    let mut right = None;
    if object_index > 0 {
        let candidate = &entry.vm_objects[object_index - 1];
        if candidate.shadow_source_id == Some(shadow_source_id)
            && candidate.shadow_depth == shadow_depth
            && candidate.start.saturating_add(candidate.len) == entry.vm_objects[object_index].start
            && candidate.shadow_source_offset.saturating_add(candidate.len) == shadow_source_offset
        {
            left = Some(object_index - 1);
        }
    }
    if object_index + 1 < entry.vm_objects.len() {
        let candidate = &entry.vm_objects[object_index + 1];
        if candidate.shadow_source_id == Some(shadow_source_id)
            && candidate.shadow_depth == shadow_depth
            && entry.vm_objects[object_index]
                .start
                .saturating_add(entry.vm_objects[object_index].len)
                == candidate.start
            && shadow_source_offset.saturating_add(entry.vm_objects[object_index].len)
                == candidate.shadow_source_offset
        {
            right = Some(object_index + 1);
        }
    }
    (left, right)
}

fn boot_vm_merge_object_range(left: &mut BootVmObject, right: BootVmObject) {
    left.len = left.len.saturating_add(right.len);
    left.page_states.extend(right.page_states);
    left.read_fault_count = left.read_fault_count.saturating_add(right.read_fault_count);
    left.write_fault_count = left
        .write_fault_count
        .saturating_add(right.write_fault_count);
    left.cow_fault_count = left.cow_fault_count.saturating_add(right.cow_fault_count);
    boot_vm_recount_object_pages(left);
}

fn boot_vm_touch_object_page(
    entry: &mut BootProcessEntry,
    mut object_index: usize,
    addr: u64,
    is_write: bool,
    owners: u64,
) -> Result<(), Errno> {
    let Some(object) = entry.vm_objects.get(object_index) else {
        return Err(Errno::Fault);
    };
    if addr < object.start || addr >= object.start.saturating_add(object.len) {
        return Err(Errno::Fault);
    }
    let page_index = ((addr - object.start) / 0x1000) as usize;
    let (absolute_page_start, absolute_page_end) = boot_vm_page_range(object, page_index);
    let mut replacement = None;
    if is_write && owners > 1 {
        if absolute_page_start > object.start {
            object_index = split_vm_object_at(entry, object_index, absolute_page_start)
                .unwrap_or(object_index);
        }
        if absolute_page_end
            < entry.vm_objects[object_index]
                .start
                .saturating_add(entry.vm_objects[object_index].len)
        {
            split_vm_object_at(entry, object_index, absolute_page_end);
        }
        let object = entry.vm_objects[object_index].clone();
        let shadow_source_id = object.shadow_source_id.unwrap_or(object.share_key);
        let mut shadow = object.clone();
        shadow.id = entry.next_vm_object_id;
        entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
        shadow.share_key = shadow.id;
        shadow.kind = "Anonymous";
        shadow.name = boot_vm_shadow_label(&object.name);
        shadow.shadow_source_id = Some(shadow_source_id);
        shadow.shadow_source_offset = object.shadow_source_offset;
        shadow.shadow_depth = object.shadow_depth.saturating_add(1);
        shadow.cow_fault_count = 0;
        replacement = Some(shadow);
    }
    let object = if let Some(shadow) = replacement {
        let (left_shadow, right_shadow) = boot_vm_find_adjacent_shadow_neighbors(
            entry,
            object_index,
            shadow.shadow_source_id.unwrap_or(0),
            shadow.shadow_depth,
            shadow.shadow_source_offset,
        );
        entry.vm_decisions.push(BootVmDecision {
            agent: "shadow-reuse",
            vm_object_id: shadow.id,
            start: absolute_page_start,
            len: 0x1000,
            detail0: shadow.shadow_source_id.unwrap_or(0),
            detail1: shadow.shadow_depth as u64,
        });
        entry.vm_objects[object_index] = shadow;
        if let Some(left_index) = left_shadow {
            let current = entry.vm_objects.remove(object_index);
            object_index = left_index;
            let left = &mut entry.vm_objects[left_index];
            boot_vm_merge_object_range(left, current);
        }
        if let Some(right_index) = right_shadow {
            let adjusted_right = if left_shadow.is_some() {
                right_index - 1
            } else {
                right_index
            };
            let right = entry.vm_objects.remove(adjusted_right);
            let left = &mut entry.vm_objects[object_index];
            boot_vm_merge_object_range(left, right);
            if left_shadow.is_some() {
                entry.vm_decisions.push(BootVmDecision {
                    agent: "shadow-bridge",
                    vm_object_id: entry.vm_objects[object_index].id,
                    start: absolute_page_start,
                    len: 0x1000,
                    detail0: 1,
                    detail1: 1,
                });
            }
        }
        let object = &entry.vm_objects[object_index];
        let local_page_index = ((absolute_page_start - object.start) / 0x1000) as u64;
        entry.vm_decisions.push(BootVmDecision {
            agent: "cow-populate",
            vm_object_id: object.id,
            start: absolute_page_start,
            len: 0x1000,
            detail0: 1,
            detail1: local_page_index,
        });
        let object = &mut entry.vm_objects[object_index];
        object.cow_fault_count = object.cow_fault_count.saturating_add(1);
        object
    } else {
        &mut entry.vm_objects[object_index]
    };
    let page_index = ((absolute_page_start - object.start) / 0x1000) as usize;
    let Some(page) = object.page_states.get_mut(page_index) else {
        return Err(Errno::Fault);
    };
    let was_resident = page.resident;
    page.resident = true;
    page.accessed = true;
    if is_write {
        page.dirty = true;
    }
    if !was_resident {
        if is_write {
            object.write_fault_count = object.write_fault_count.saturating_add(1);
        } else {
            object.read_fault_count = object.read_fault_count.saturating_add(1);
        }
        entry.vm_decisions.push(BootVmDecision {
            agent: "fault-classifier",
            vm_object_id: object.id,
            start: absolute_page_start,
            len: 0x1000,
            detail0: if is_write { 1 } else { 0 },
            detail1: page_index as u64,
        });
    }
    entry.vm_decisions.push(BootVmDecision {
        agent: "page-touch",
        vm_object_id: object.id,
        start: absolute_page_start,
        len: 0x1000,
        detail0: page_index as u64,
        detail1: if is_write { 1 } else { 0 },
    });
    boot_vm_recount_object_pages(object);
    Ok(())
}

fn boot_vm_owner_count(registry: &BootProcessRegistry, share_key: u64) -> u64 {
    registry
        .entries
        .iter()
        .filter(|entry| !entry.reaped)
        .filter(|entry| {
            entry
                .vm_objects
                .iter()
                .any(|object| object.share_key == share_key)
        })
        .count() as u64
}

fn boot_vm_clone_for_copy(entry: &mut BootProcessEntry, source: &BootVmObject) -> BootVmObject {
    let object_id = entry.next_vm_object_id;
    entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
    BootVmObject {
        id: object_id,
        start: source.start,
        len: source.len,
        name: source.name.clone(),
        kind: source.kind,
        share_key: source.share_key,
        shadow_source_id: source.shadow_source_id,
        shadow_source_offset: source.shadow_source_offset,
        shadow_depth: source.shadow_depth,
        private_mapping: source.private_mapping,
        file_offset: source.file_offset,
        readable: source.readable,
        writable: source.writable,
        executable: source.executable,
        read_fault_count: source.read_fault_count,
        write_fault_count: source.write_fault_count,
        cow_fault_count: source.cow_fault_count,
        committed_pages: source.committed_pages,
        resident_pages: source.resident_pages,
        dirty_pages: source.dirty_pages,
        accessed_pages: source.accessed_pages,
        quarantined: source.quarantined,
        quarantine_reason: source.quarantine_reason,
        page_states: source.page_states.clone(),
    }
}

fn boot_copy_vm_state(source_pid: u64, target_pid: u64) -> Result<(), Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(source_index) = registry.find_index(source_pid) else {
            return Err(Errno::Srch);
        };
        let Some(target_index) = registry.find_index(target_pid) else {
            return Err(Errno::Srch);
        };
        let source_objects = registry.entries[source_index].vm_objects.clone();
        let source_next_vm_addr = registry.entries[source_index].next_vm_addr;
        let target = &mut registry.entries[target_index];
        target.next_vm_addr = source_next_vm_addr;
        target.vm_objects = source_objects
            .iter()
            .map(|object| boot_vm_clone_for_copy(target, object))
            .collect();
        target.vm_decisions.push(BootVmDecision {
            agent: "fork",
            vm_object_id: 0,
            start: source_pid,
            len: target_pid,
            detail0: source_objects.len() as u64,
            detail1: 0,
        });
        Ok(())
    })
}

fn boot_procfs_poll(node: BootProcfsNode, offset: usize, interest: u32) -> u32 {
    let Ok(payload) = boot_procfs_payload(node.pid, node.kind) else {
        return 0;
    };
    let mut ready = POLLOUT;
    if offset < payload.len() {
        ready |= POLLIN;
    }
    ready & interest
}

fn boot_vfs_read(
    node_index: usize,
    offset: &mut usize,
    buffer: *mut u8,
    len: usize,
) -> Result<usize, Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let Some(node) = vfs.nodes.get(node_index) else {
            return Err(Errno::Badf);
        };
        let available = node.bytes.len().saturating_sub(*offset);
        let count = available.min(len);
        if count == 0 {
            return Ok(0);
        }
        unsafe {
            ptr::copy_nonoverlapping(node.bytes[*offset..*offset + count].as_ptr(), buffer, count);
        }
        *offset += count;
        Ok(count)
    })
}

fn boot_vfs_write(node_index: usize, offset: &mut usize, bytes: &[u8]) -> Result<usize, Errno> {
    BOOT_VFS.with_mut(|vfs| {
        let Some(node) = vfs.nodes.get_mut(node_index) else {
            return Err(Errno::Badf);
        };
        if matches!(node.kind, BootNodeKind::Directory) {
            return Err(Errno::IsDir);
        }
        if matches!(node.kind, BootNodeKind::Channel) {
            node.bytes.extend_from_slice(bytes);
            *offset = node.bytes.len();
            return Ok(bytes.len());
        }
        if *offset > node.bytes.len() {
            node.bytes.resize(*offset, 0);
        }
        let end = offset.saturating_add(bytes.len());
        if end > node.bytes.len() {
            node.bytes.resize(end, 0);
        }
        node.bytes[*offset..end].copy_from_slice(bytes);
        *offset = end;
        Ok(bytes.len())
    })
}

fn boot_vfs_poll(node_index: usize, offset: usize, interest: u32) -> u32 {
    BOOT_VFS.with_mut(|vfs| {
        let Some(node) = vfs.nodes.get(node_index) else {
            return 0;
        };
        let mut ready = 0;
        if offset < node.bytes.len() {
            ready |= POLLIN;
        }
        if !matches!(node.kind, BootNodeKind::Directory) {
            ready |= POLLOUT;
        }
        ready & interest
    })
}

fn boot_stream_target(path: &str) -> Option<DescriptorTarget> {
    match path {
        GPU_DEVICE_PATH => Some(DescriptorTarget::GpuDevice),
        AUDIO_DEVICE_PATH => Some(DescriptorTarget::AudioDevice),
        INPUT_DEVICE_PATH => Some(DescriptorTarget::InputDevice),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootProcessEntry {
    pid: u64,
    parent: u64,
    name: String,
    image_path: String,
    cwd: String,
    argv_count: u64,
    env_count: u64,
    state: u32,
    exit_code: i32,
    pending_signal_count: u64,
    contract_bindings: BootProcessContractBindings,
    next_vm_addr: u64,
    next_vm_object_id: u64,
    vm_objects: Vec<BootVmObject>,
    vm_decisions: Vec<BootVmDecision>,
    reaped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BootProcessContractBindings {
    execution: u64,
    memory: u64,
    io: u64,
    observe: u64,
}

impl BootProcessContractBindings {
    fn bind(&mut self, kind: NativeContractKind, contract: u64) {
        match kind {
            NativeContractKind::Execution => self.execution = contract,
            NativeContractKind::Memory => self.memory = contract,
            NativeContractKind::Io => self.io = contract,
            NativeContractKind::Observe => self.observe = contract,
            NativeContractKind::Device | NativeContractKind::Display => {}
        }
    }
}

struct BootProcessContractBindAgent;

impl BootProcessContractBindAgent {
    fn execute(contract_id: usize) -> Result<NativeContractKind, Errno> {
        let pid = BOOT_OWNER_ID;
        let kind = NATIVE_REGISTRY.with(|registry| {
            let contract = registry.contract(contract_id)?;
            if contract.issuer != pid {
                return Err(Errno::Access);
            }
            Ok(contract.kind)
        })?;
        BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid) else {
                return Err(Errno::Srch);
            };
            registry.entries[index]
                .contract_bindings
                .bind(kind, contract_id as u64);
            Ok(())
        })?;
        Ok(kind)
    }
}

struct BootVmPolicyBlockAgent;

impl BootVmPolicyBlockAgent {
    fn record(pid: usize, start: usize, len: usize, state_code: u64, operation_code: u64) {
        let _ = BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid as u64) else {
                return Err(Errno::Srch);
            };
            registry.entries[index].vm_decisions.push(BootVmDecision {
                agent: "policy-block",
                vm_object_id: 0,
                start: start as u64,
                len: len as u64,
                detail0: state_code,
                detail1: operation_code,
            });
            Ok(())
        });
    }
}

struct BootVmPolicyEnforcementAgent;

impl BootVmPolicyEnforcementAgent {
    fn enforce(pid: usize, start: usize, len: usize, operation_code: u64) -> Result<(), Errno> {
        let contract_id = BOOT_PROCESSES.with_mut(|registry| {
            let Some(index) = registry.find_index(pid as u64) else {
                return Err(Errno::Srch);
            };
            Ok(registry.entries[index].contract_bindings.memory)
        })?;
        if contract_id == 0 {
            return Ok(());
        }

        let state = NATIVE_REGISTRY.with(|registry| {
            let contract = registry.contract(contract_id as usize)?;
            let resource = registry.resource(contract.resource as usize)?;
            if !contract_kind_allowed(resource.contract_policy, NativeContractKind::Memory) {
                return Err(Errno::Access);
            }
            Ok(contract.state)
        })?;

        if state != NativeContractState::Active {
            let state_code = match state {
                NativeContractState::Active => 0,
                NativeContractState::Suspended => 1,
                NativeContractState::Revoked => 2,
            };
            BootVmPolicyBlockAgent::record(pid, start, len, state_code, operation_code);
            return Err(Errno::Access);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootVmObject {
    id: u64,
    start: u64,
    len: u64,
    name: String,
    kind: &'static str,
    share_key: u64,
    shadow_source_id: Option<u64>,
    shadow_source_offset: u64,
    shadow_depth: u32,
    private_mapping: bool,
    file_offset: u64,
    readable: bool,
    writable: bool,
    executable: bool,
    read_fault_count: u64,
    write_fault_count: u64,
    cow_fault_count: u64,
    committed_pages: u64,
    resident_pages: u64,
    dirty_pages: u64,
    accessed_pages: u64,
    quarantined: bool,
    quarantine_reason: u64,
    page_states: Vec<BootVmPageState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootVmDecision {
    agent: &'static str,
    vm_object_id: u64,
    start: u64,
    len: u64,
    detail0: u64,
    detail1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BootVmPageState {
    resident: bool,
    dirty: bool,
    accessed: bool,
}

#[derive(Debug, Default)]
struct BootProcessRegistry {
    next_pid: u64,
    entries: Vec<BootProcessEntry>,
}

struct BootProcessRegistryCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<BootProcessRegistry>>,
}

impl BootProcessRegistryCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut BootProcessRegistry) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let state = unsafe { &mut *self.state.get() };
        if state.is_none() {
            *state = Some(BootProcessRegistry::new());
        }
        let result = f(state.as_mut().unwrap());
        self.locked.store(false, Ordering::Release);
        result
    }
}

impl BootProcessRegistry {
    fn new() -> Self {
        Self {
            next_pid: 2,
            entries: vec![BootProcessEntry {
                pid: 1,
                parent: 0,
                name: String::from("ngos-userland-native"),
                image_path: String::from("/kernel/ngos-userland-native"),
                cwd: String::from("/"),
                argv_count: 1,
                env_count: 0,
                state: 2,
                exit_code: 0,
                pending_signal_count: 0,
                contract_bindings: BootProcessContractBindings::default(),
                next_vm_addr: 0x6000_0000,
                next_vm_object_id: 2,
                vm_objects: vec![BootVmObject {
                    id: 1,
                    start: 0x4000_0000,
                    len: 0x4000,
                    name: String::from("[heap]"),
                    kind: "Heap",
                    share_key: 1,
                    shadow_source_id: None,
                    shadow_source_offset: 0,
                    shadow_depth: 0,
                    private_mapping: true,
                    file_offset: 0,
                    readable: true,
                    writable: true,
                    executable: false,
                    read_fault_count: 0,
                    write_fault_count: 0,
                    cow_fault_count: 0,
                    committed_pages: 4,
                    resident_pages: 4,
                    dirty_pages: 1,
                    accessed_pages: 1,
                    quarantined: false,
                    quarantine_reason: 0,
                    page_states: vec![
                        BootVmPageState {
                            resident: true,
                            dirty: true,
                            accessed: true,
                        },
                        BootVmPageState {
                            resident: true,
                            dirty: false,
                            accessed: false,
                        },
                        BootVmPageState {
                            resident: true,
                            dirty: false,
                            accessed: false,
                        },
                        BootVmPageState {
                            resident: true,
                            dirty: false,
                            accessed: false,
                        },
                    ],
                }],
                vm_decisions: Vec::new(),
                reaped: false,
            }],
        }
    }

    fn find_index(&self, pid: u64) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.pid == pid && !entry.reaped)
    }

    fn spawn(
        &mut self,
        name: String,
        image_path: String,
        cwd: String,
        argv_count: usize,
        env_count: usize,
    ) -> Result<u64, Errno> {
        if self.entries.iter().filter(|entry| !entry.reaped).count() >= MAX_PROCESS_COUNT {
            return Err(Errno::Again);
        }
        let pid = self.next_pid;
        self.next_pid = self.next_pid.saturating_add(1);
        self.entries.push(BootProcessEntry {
            pid,
            parent: 1,
            name,
            image_path,
            cwd,
            argv_count: argv_count as u64,
            env_count: env_count as u64,
            state: 1,
            exit_code: 0,
            pending_signal_count: 0,
            contract_bindings: BootProcessContractBindings::default(),
            next_vm_addr: 0x6000_0000,
            next_vm_object_id: 2,
            vm_objects: vec![BootVmObject {
                id: 1,
                start: 0x4000_0000,
                len: 0x2000,
                name: String::from("[heap]"),
                kind: "Heap",
                share_key: 1,
                shadow_source_id: None,
                shadow_source_offset: 0,
                shadow_depth: 0,
                private_mapping: true,
                file_offset: 0,
                readable: true,
                writable: true,
                executable: false,
                read_fault_count: 0,
                write_fault_count: 0,
                cow_fault_count: 0,
                committed_pages: 2,
                resident_pages: 2,
                dirty_pages: 0,
                accessed_pages: 0,
                quarantined: false,
                quarantine_reason: 0,
                page_states: vec![
                    BootVmPageState {
                        resident: true,
                        dirty: false,
                        accessed: false,
                    };
                    2
                ],
            }],
            vm_decisions: Vec::new(),
            reaped: false,
        });
        Ok(pid)
    }
}

fn copy_struct_from_user<T: Copy>(ptr_value: usize) -> Result<T, Errno> {
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    Ok(unsafe { ptr::read(ptr_value as *const T) })
}

fn string_from_user(ptr_value: usize, len: usize) -> Result<String, Errno> {
    let text = path_from_user(ptr_value, len)?;
    Ok(String::from(text))
}

fn string_table_from_user(
    ptr_value: usize,
    len: usize,
    count: usize,
) -> Result<Vec<String>, Errno> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    let bytes = unsafe { slice::from_raw_parts(ptr_value as *const u8, len) };
    let mut values = Vec::with_capacity(count);
    let mut start = 0usize;
    for index in 0..len {
        if bytes[index] != 0 {
            continue;
        }
        let value = core::str::from_utf8(&bytes[start..index]).map_err(|_| Errno::Inval)?;
        values.push(String::from(value));
        start = index + 1;
        if values.len() == count {
            break;
        }
    }
    if values.len() != count {
        return Err(Errno::Inval);
    }
    Ok(values)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InlineName {
    len: usize,
    bytes: [u8; MAX_NAME_LEN],
}

impl InlineName {
    const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; MAX_NAME_LEN],
        }
    }

    fn from_text(text: &str) -> Result<Self, Errno> {
        if text.is_empty() || text.len() > MAX_NAME_LEN {
            return Err(Errno::Inval);
        }
        let mut name = Self::empty();
        name.len = text.len();
        name.bytes[..text.len()].copy_from_slice(text.as_bytes());
        Ok(name)
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DomainEntry {
    id: u64,
    owner: u64,
    parent: u64,
    name: InlineName,
    resource_count: u64,
    contract_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResourceEntry {
    id: u64,
    domain: u64,
    creator: u64,
    name: InlineName,
    kind: NativeResourceKind,
    state: NativeResourceState,
    arbitration: NativeResourceArbitrationPolicy,
    governance: NativeResourceGovernanceMode,
    contract_policy: NativeResourceContractPolicy,
    issuer_policy: NativeResourceIssuerPolicy,
    holder_contract: u64,
    waiting_count: usize,
    waiters: [u64; MAX_CONTRACT_COUNT],
    acquire_count: u64,
    handoff_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContractEntry {
    id: u64,
    domain: u64,
    resource: u64,
    issuer: u64,
    kind: NativeContractKind,
    state: NativeContractState,
    label: InlineName,
    invocation_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeRegistry {
    domains: [Option<DomainEntry>; MAX_DOMAIN_COUNT],
    resources: [Option<ResourceEntry>; MAX_RESOURCE_COUNT],
    contracts: [Option<ContractEntry>; MAX_CONTRACT_COUNT],
}

impl NativeRegistry {
    const fn new() -> Self {
        Self {
            domains: [None; MAX_DOMAIN_COUNT],
            resources: [None; MAX_RESOURCE_COUNT],
            contracts: [None; MAX_CONTRACT_COUNT],
        }
    }

    fn create_domain(&mut self, parent: u64, name: InlineName) -> Result<usize, Errno> {
        if parent != 0 && self.domain(parent as usize).is_err() {
            return Err(Errno::Inval);
        }
        let slot = self
            .domains
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.domains[slot] = Some(DomainEntry {
            id,
            owner: BOOT_OWNER_ID,
            parent,
            name,
            resource_count: 0,
            contract_count: 0,
        });
        Ok(id as usize)
    }

    fn create_resource(
        &mut self,
        domain: usize,
        kind: NativeResourceKind,
        name: InlineName,
    ) -> Result<usize, Errno> {
        let domain_slot = self.domain_slot(domain)?;
        let slot = self
            .resources
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.resources[slot] = Some(ResourceEntry {
            id,
            domain: domain as u64,
            creator: BOOT_OWNER_ID,
            name,
            kind,
            state: NativeResourceState::Active,
            arbitration: NativeResourceArbitrationPolicy::Fifo,
            governance: NativeResourceGovernanceMode::Queueing,
            contract_policy: NativeResourceContractPolicy::Any,
            issuer_policy: NativeResourceIssuerPolicy::AnyIssuer,
            holder_contract: 0,
            waiting_count: 0,
            waiters: [0; MAX_CONTRACT_COUNT],
            acquire_count: 0,
            handoff_count: 0,
        });
        self.domains[domain_slot].as_mut().unwrap().resource_count += 1;
        Ok(id as usize)
    }

    fn create_contract(
        &mut self,
        domain: usize,
        resource: usize,
        kind: NativeContractKind,
        label: InlineName,
    ) -> Result<usize, Errno> {
        let domain_slot = self.domain_slot(domain)?;
        let resource_slot = self.resource_slot(resource)?;
        {
            let resource_entry = self.resources[resource_slot].as_ref().unwrap();
            if resource_entry.domain != domain as u64 {
                return Err(Errno::Inval);
            }
            if resource_entry.state != NativeResourceState::Active {
                return Err(Errno::Access);
            }
            if !contract_kind_allowed(resource_entry.contract_policy, kind) {
                return Err(Errno::Access);
            }
            if !issuer_allowed(
                resource_entry.issuer_policy,
                resource_entry.creator,
                self.domains[domain_slot].as_ref().unwrap().owner,
                BOOT_OWNER_ID,
            ) {
                return Err(Errno::Access);
            }
        }
        let slot = self
            .contracts
            .iter()
            .position(Option::is_none)
            .ok_or(Errno::Again)?;
        let id = (slot + 1) as u64;
        self.contracts[slot] = Some(ContractEntry {
            id,
            domain: domain as u64,
            resource: resource as u64,
            issuer: BOOT_OWNER_ID,
            kind,
            state: NativeContractState::Active,
            label,
            invocation_count: 0,
        });
        self.domains[domain_slot].as_mut().unwrap().contract_count += 1;
        Ok(id as usize)
    }

    fn domain_slot(&self, id: usize) -> Result<usize, Errno> {
        if id == 0 || id > MAX_DOMAIN_COUNT {
            return Err(Errno::Inval);
        }
        self.domains[id - 1].ok_or(Errno::Inval).map(|_| id - 1)
    }

    fn resource_slot(&self, id: usize) -> Result<usize, Errno> {
        if id == 0 || id > MAX_RESOURCE_COUNT {
            return Err(Errno::Inval);
        }
        self.resources[id - 1].ok_or(Errno::Inval).map(|_| id - 1)
    }

    fn contract_slot(&self, id: usize) -> Result<usize, Errno> {
        if id == 0 || id > MAX_CONTRACT_COUNT {
            return Err(Errno::Inval);
        }
        self.contracts[id - 1].ok_or(Errno::Inval).map(|_| id - 1)
    }

    fn domain(&self, id: usize) -> Result<&DomainEntry, Errno> {
        Ok(self.domains[self.domain_slot(id)?].as_ref().unwrap())
    }

    fn resource(&self, id: usize) -> Result<&ResourceEntry, Errno> {
        Ok(self.resources[self.resource_slot(id)?].as_ref().unwrap())
    }

    fn contract(&self, id: usize) -> Result<&ContractEntry, Errno> {
        Ok(self.contracts[self.contract_slot(id)?].as_ref().unwrap())
    }

    fn contract_and_resource_slots(&self, contract: usize) -> Result<(usize, usize), Errno> {
        let contract_slot = self.contract_slot(contract)?;
        let resource = self.contracts[contract_slot].as_ref().unwrap().resource as usize;
        let resource_slot = self.resource_slot(resource)?;
        Ok((contract_slot, resource_slot))
    }

    fn remove_waiter_at(resource: &mut ResourceEntry, index: usize) {
        let count = resource.waiting_count;
        let mut cursor = index;
        while cursor + 1 < count {
            resource.waiters[cursor] = resource.waiters[cursor + 1];
            cursor += 1;
        }
        if count > 0 {
            resource.waiters[count - 1] = 0;
            resource.waiting_count -= 1;
        }
    }

    fn remove_waiter(resource: &mut ResourceEntry, contract: u64) -> bool {
        if let Some(index) = resource.waiters[..resource.waiting_count]
            .iter()
            .position(|id| *id == contract)
        {
            Self::remove_waiter_at(resource, index);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn next_waiter_index(&self, resource: &ResourceEntry) -> Option<usize> {
        if resource.waiting_count == 0 {
            return None;
        }
        match resource.arbitration {
            NativeResourceArbitrationPolicy::Fifo => Some(0),
            NativeResourceArbitrationPolicy::Lifo => Some(resource.waiting_count - 1),
        }
    }

    fn select_handoff_waiter(&mut self, resource_slot: usize) -> Option<u64> {
        loop {
            let (waiting_count, arbitration, state) = {
                let resource = self.resources[resource_slot].as_ref().unwrap();
                (resource.waiting_count, resource.arbitration, resource.state)
            };
            let index = if waiting_count == 0 {
                return None;
            } else {
                match arbitration {
                    NativeResourceArbitrationPolicy::Fifo => 0,
                    NativeResourceArbitrationPolicy::Lifo => waiting_count - 1,
                }
            };
            let contract_id = {
                let resource = self.resources[resource_slot].as_mut().unwrap();
                let id = resource.waiters[index];
                Self::remove_waiter_at(resource, index);
                id
            };
            let Ok(contract_slot) = self.contract_slot(contract_id as usize) else {
                continue;
            };
            let contract = self.contracts[contract_slot].as_ref().unwrap();
            if contract.state != NativeContractState::Active {
                continue;
            }
            if state != NativeResourceState::Active {
                return None;
            }
            let contract_allowed = {
                let resource = self.resources[resource_slot].as_ref().unwrap();
                contract_kind_allowed(resource.contract_policy, contract.kind)
            };
            if !contract_allowed {
                continue;
            }
            return Some(contract_id);
        }
    }
}

impl NativeRegistryCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(NativeRegistry::new()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut NativeRegistry) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }

    fn with<R>(&self, f: impl FnOnce(&NativeRegistry) -> R) -> R {
        unsafe { f(&*self.0.get()) }
    }
}

fn contract_kind_allowed(policy: NativeResourceContractPolicy, kind: NativeContractKind) -> bool {
    match policy {
        NativeResourceContractPolicy::Any => true,
        NativeResourceContractPolicy::Execution => kind == NativeContractKind::Execution,
        NativeResourceContractPolicy::Memory => kind == NativeContractKind::Memory,
        NativeResourceContractPolicy::Io => kind == NativeContractKind::Io,
        NativeResourceContractPolicy::Device => kind == NativeContractKind::Device,
        NativeResourceContractPolicy::Display => kind == NativeContractKind::Display,
        NativeResourceContractPolicy::Observe => kind == NativeContractKind::Observe,
    }
}

fn issuer_allowed(
    policy: NativeResourceIssuerPolicy,
    creator: u64,
    domain_owner: u64,
    issuer: u64,
) -> bool {
    match policy {
        NativeResourceIssuerPolicy::AnyIssuer => true,
        NativeResourceIssuerPolicy::CreatorOnly => issuer == creator,
        NativeResourceIssuerPolicy::DomainOwnerOnly => issuer == domain_owner,
    }
}

fn read_inline_name(ptr_value: usize, len: usize) -> Result<InlineName, Errno> {
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    if len == 0 || len > MAX_NAME_LEN {
        return Err(Errno::Inval);
    }
    let source = unsafe { slice::from_raw_parts(ptr_value as *const u8, len) };
    let mut name = InlineName::empty();
    name.len = len;
    name.bytes[..len].copy_from_slice(source);
    Ok(name)
}

fn copy_ids_to_user(ids: &[u64], buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    if capacity == 0 {
        return Ok(ids.len());
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let copy_len = ids.len().min(capacity);
    unsafe {
        ptr::copy_nonoverlapping(ids.as_ptr(), buffer, copy_len);
    }
    Ok(ids.len())
}

fn copy_name_to_user(name: &InlineName, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    if capacity == 0 {
        return Ok(name.len);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let copy_len = name.len.min(capacity);
    unsafe {
        ptr::copy_nonoverlapping(name.as_bytes().as_ptr(), buffer, copy_len);
    }
    Ok(copy_len)
}

fn copy_text_to_user(text: &str, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    if capacity == 0 {
        return Ok(text.len());
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    let copy_len = text.len().min(capacity);
    unsafe {
        ptr::copy_nonoverlapping(text.as_ptr(), buffer, copy_len);
    }
    Ok(copy_len)
}

fn write_record<T: Copy>(dst: *mut T, value: T) -> Result<(), Errno> {
    if dst.is_null() {
        return Err(Errno::Fault);
    }
    unsafe {
        ptr::write(dst, value);
    }
    Ok(())
}

fn resource_watch_matches(watch: &ResourceEventWatch, kind: BootResourceEventKind) -> bool {
    match kind {
        BootResourceEventKind::Claimed => watch.claimed,
        BootResourceEventKind::Queued => watch.queued,
        BootResourceEventKind::Canceled => watch.canceled,
        BootResourceEventKind::Released => watch.released,
        BootResourceEventKind::HandedOff => watch.handed_off,
        BootResourceEventKind::Revoked => watch.revoked,
    }
}

fn queue_pending_mask(queue_id: usize) -> Result<u32, Errno> {
    BOOT_EVENT_QUEUES.with(|registry| {
        let queue = registry.queue(queue_id)?;
        Ok(queue
            .pending
            .iter()
            .fold(0u32, |mask, event| mask | event.events))
    })
}

fn emit_resource_event(resource: u64, contract: u64, kind: BootResourceEventKind) {
    BOOT_EVENT_QUEUES.with_mut(|registry| {
        for queue in registry.queues.iter_mut().flatten() {
            let matching = queue
                .resource_watches
                .iter()
                .filter(|watch| watch.resource == resource && resource_watch_matches(watch, kind))
                .map(|watch| NativeEventRecord {
                    token: watch.token,
                    events: watch.events,
                    source_kind: NativeEventSourceKind::Resource as u32,
                    source_arg0: resource,
                    source_arg1: contract,
                    source_arg2: 0,
                    detail0: kind as u32,
                    detail1: 0,
                })
                .collect::<Vec<_>>();
            for event in matching {
                if queue.pending.len() >= MAX_EVENT_QUEUE_PENDING {
                    queue.pending.remove(0);
                }
                queue.pending.push(event);
            }
        }
    });
}

impl DescriptorTable {
    const fn new() -> Self {
        Self {
            slots: [
                Some(DescriptorState {
                    target: DescriptorTarget::Stdin,
                    flags: DescriptorFlags {
                        nonblock: false,
                        cloexec: false,
                    },
                    offset: 0,
                }),
                Some(DescriptorState {
                    target: DescriptorTarget::Stdout,
                    flags: DescriptorFlags {
                        nonblock: false,
                        cloexec: false,
                    },
                    offset: 0,
                }),
                Some(DescriptorState {
                    target: DescriptorTarget::Stderr,
                    flags: DescriptorFlags {
                        nonblock: false,
                        cloexec: false,
                    },
                    offset: 0,
                }),
                None,
                None,
                None,
                None,
                None,
            ],
        }
    }

    fn descriptor(&self, fd: usize) -> Result<DescriptorState, Errno> {
        self.slots
            .get(fd)
            .and_then(|entry| *entry)
            .ok_or(Errno::Badf)
    }

    fn descriptor_mut(&mut self, fd: usize) -> Result<&mut DescriptorState, Errno> {
        self.slots
            .get_mut(fd)
            .and_then(Option::as_mut)
            .ok_or(Errno::Badf)
    }

    fn duplicate(&mut self, fd: usize) -> Result<usize, Errno> {
        let descriptor = self.descriptor(fd)?;
        let free_fd = self
            .slots
            .iter()
            .enumerate()
            .skip(3)
            .find_map(|(index, slot)| slot.is_none().then_some(index))
            .ok_or(Errno::Again)?;
        self.slots[free_fd] = Some(descriptor);
        Ok(free_fd)
    }

    fn close(&mut self, fd: usize) -> Result<(), Errno> {
        let slot = self.slots.get_mut(fd).ok_or(Errno::Badf)?;
        let target = slot.ok_or(Errno::Badf)?.target;
        *slot = None;
        if let DescriptorTarget::EventQueue(queue_id) = target {
            let still_open = self
                .slots
                .iter()
                .flatten()
                .any(|descriptor| descriptor.target == DescriptorTarget::EventQueue(queue_id));
            if !still_open {
                BOOT_EVENT_QUEUES.with_mut(|registry| registry.remove_queue(queue_id));
            }
        }
        Ok(())
    }

    fn fcntl(&mut self, fd: usize, encoded: usize) -> Result<usize, Errno> {
        let command = decode_fcntl(encoded).ok_or(Errno::Inval)?;
        let descriptor = self.descriptor_mut(fd)?;
        match command {
            DecodedFcntl::GetFl => Ok(encode_flags(DescriptorFlags {
                nonblock: descriptor.flags.nonblock,
                cloexec: false,
            })),
            DecodedFcntl::GetFd => Ok(encode_flags(DescriptorFlags {
                nonblock: false,
                cloexec: descriptor.flags.cloexec,
            })),
            DecodedFcntl::SetFl { nonblock } => {
                descriptor.flags.nonblock = nonblock;
                Ok(encode_flags(DescriptorFlags {
                    nonblock: descriptor.flags.nonblock,
                    cloexec: false,
                }))
            }
            DecodedFcntl::SetFd { cloexec } => {
                descriptor.flags.cloexec = cloexec;
                Ok(encode_flags(DescriptorFlags {
                    nonblock: false,
                    cloexec: descriptor.flags.cloexec,
                }))
            }
        }
    }

    fn poll(&self, fd: usize, interest: u32) -> Result<usize, Errno> {
        let descriptor = self.descriptor(fd)?;
        serial::print(format_args!(
            "ngos/x86_64: descriptor-poll fd={} target={:?} interest={:#x}\n",
            fd, descriptor.target, interest
        ));
        let available = match descriptor.target {
            DescriptorTarget::Stdin => tty::poll_mask_for_stdin(interest) as u32,
            DescriptorTarget::Stdout | DescriptorTarget::Stderr => {
                tty::poll_mask_for_output(interest) as u32
            }
            DescriptorTarget::EventQueue(queue_id) => queue_pending_mask(queue_id)?,
            DescriptorTarget::GpuDevice
            | DescriptorTarget::AudioDevice
            | DescriptorTarget::InputDevice => POLLOUT & interest,
            DescriptorTarget::StorageDevice => crate::virtio_blk_boot::poll(
                crate::virtio_blk_boot::StorageEndpointKind::Device,
                interest,
            ) as u32,
            DescriptorTarget::StorageDriver => crate::virtio_blk_boot::poll(
                crate::virtio_blk_boot::StorageEndpointKind::Driver,
                interest,
            ) as u32,
            DescriptorTarget::BootFile(node) | DescriptorTarget::BootChannel(node) => {
                boot_vfs_poll(node, descriptor.offset, interest)
            }
            DescriptorTarget::Procfs(node) => boot_procfs_poll(node, descriptor.offset, interest),
        };
        Ok((available & interest) as usize)
    }

    fn create_event_queue(&mut self, mode: NativeEventQueueMode) -> Result<usize, Errno> {
        let queue_id = BOOT_EVENT_QUEUES.with_mut(|registry| registry.create_queue(mode))?;
        let free_fd = self
            .slots
            .iter()
            .enumerate()
            .skip(3)
            .find_map(|(index, slot)| slot.is_none().then_some(index))
            .ok_or(Errno::Again)?;
        self.slots[free_fd] = Some(DescriptorState {
            target: DescriptorTarget::EventQueue(queue_id),
            flags: DescriptorFlags::default(),
            offset: 0,
        });
        Ok(free_fd)
    }

    fn event_queue_descriptor(&self, fd: usize) -> Result<(usize, DescriptorFlags), Errno> {
        let descriptor = self.descriptor(fd)?;
        match descriptor.target {
            DescriptorTarget::EventQueue(queue_id) => Ok((queue_id, descriptor.flags)),
            _ => Err(Errno::Badf),
        }
    }

    fn watch_resource_events(
        &mut self,
        fd: usize,
        resource: usize,
        config: NativeResourceEventWatchConfig,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            if let Some(existing) = queue
                .resource_watches
                .iter_mut()
                .find(|watch| watch.resource == resource as u64 && watch.token == config.token)
            {
                *existing = ResourceEventWatch {
                    resource: resource as u64,
                    token: config.token,
                    events: config.poll_events,
                    claimed: config.claimed != 0,
                    queued: config.queued != 0,
                    canceled: config.canceled != 0,
                    released: config.released != 0,
                    handed_off: config.handed_off != 0,
                    revoked: config.revoked != 0,
                };
                return Ok(());
            }
            if queue.resource_watches.len() >= MAX_EVENT_QUEUE_WATCH_COUNT {
                return Err(Errno::Again);
            }
            queue.resource_watches.push(ResourceEventWatch {
                resource: resource as u64,
                token: config.token,
                events: config.poll_events,
                claimed: config.claimed != 0,
                queued: config.queued != 0,
                canceled: config.canceled != 0,
                released: config.released != 0,
                handed_off: config.handed_off != 0,
                revoked: config.revoked != 0,
            });
            Ok(())
        })
    }

    fn remove_resource_events(
        &mut self,
        fd: usize,
        resource: usize,
        token: u64,
    ) -> Result<(), Errno> {
        let (queue_id, _) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let original = queue.resource_watches.len();
            queue
                .resource_watches
                .retain(|watch| !(watch.resource == resource as u64 && watch.token == token));
            if queue.resource_watches.len() == original {
                return Err(Errno::NoEnt);
            }
            Ok(())
        })
    }

    fn wait_event_queue(
        &mut self,
        fd: usize,
        buffer: *mut NativeEventRecord,
        capacity: usize,
    ) -> Result<usize, Errno> {
        if buffer.is_null() && capacity != 0 {
            return Err(Errno::Fault);
        }
        let (queue_id, flags) = self.event_queue_descriptor(fd)?;
        BOOT_EVENT_QUEUES.with_mut(|registry| {
            let queue = registry.queue_mut(queue_id)?;
            let _ = queue.mode;
            if queue.pending.is_empty() {
                return Err(Errno::Again);
            }
            if capacity == 0 {
                return Ok(0);
            }
            let count = capacity.min(queue.pending.len());
            for (index, event) in queue.pending.drain(..count).enumerate() {
                unsafe {
                    ptr::write(buffer.add(index), event);
                }
            }
            if flags.nonblock && queue.pending.is_empty() {
                return Ok(count);
            }
            Ok(count)
        })
    }

    fn open_path(&mut self, path: &str) -> Result<usize, Errno> {
        serial::debug_marker(b'y');
        serial::print(format_args!(
            "ngos/x86_64: descriptor-open enter path={}\n",
            path
        ));
        let target = match crate::virtio_blk_boot::endpoint_for_path(path) {
            Some(crate::virtio_blk_boot::StorageEndpointKind::Device) => {
                serial::debug_marker(b'z');
                serial::print(format_args!(
                    "ngos/x86_64: descriptor-open endpoint=device path={}\n",
                    path
                ));
                DescriptorTarget::StorageDevice
            }
            Some(crate::virtio_blk_boot::StorageEndpointKind::Driver) => {
                serial::debug_marker(b'z');
                serial::print(format_args!(
                    "ngos/x86_64: descriptor-open endpoint=driver path={}\n",
                    path
                ));
                DescriptorTarget::StorageDriver
            }
            None => match boot_stream_target(path) {
                Some(target) => target,
                None => match boot_procfs_node(path)? {
                    Some(node) => DescriptorTarget::Procfs(node),
                    None => boot_vfs_lookup_target(path)?,
                },
            },
        };
        serial::debug_marker(b'v');
        let free_fd = self
            .slots
            .iter()
            .enumerate()
            .skip(3)
            .find_map(|(index, slot)| slot.is_none().then_some(index))
            .ok_or_else(|| {
                serial::debug_marker(b'w');
                Errno::Again
            })?;
        serial::debug_marker(b'w');
        serial::print(format_args!(
            "ngos/x86_64: descriptor-open slot path={} fd={} target={:?}\n",
            path, free_fd, target
        ));
        self.slots[free_fd] = Some(DescriptorState {
            target,
            flags: DescriptorFlags::default(),
            offset: 0,
        });
        serial::debug_marker(b'v');
        Ok(free_fd)
    }
}

impl DescriptorTableCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(DescriptorTable::new()))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut DescriptorTable) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }

    fn with<R>(&self, f: impl FnOnce(&DescriptorTable) -> R) -> R {
        unsafe { f(&*self.0.get()) }
    }
}

#[unsafe(no_mangle)]
extern "C" fn x86_64_syscall_dispatch(
    frame: *const SyscallFrame,
    user_rip: u64,
    user_rsp: u64,
    user_rflags: u64,
    result: *mut SyscallDispatchResult,
) {
    let frame = unsafe { &*frame };
    let result = unsafe { &mut *result };

    serial::debug_marker(b'T');
    syscall_trace(format_args!(
        "ngos/x86_64: syscall entry reached rip={:#x} rsp={:#x} rflags={:#x}\n",
        user_rip, user_rsp, user_rflags
    ));
    serial::debug_marker(b'U');
    syscall_trace(format_args!(
        "ngos/x86_64: syscall number decoded nr={} a0={:#x} a1={:#x} a2={:#x}\n",
        frame.number, frame.arg0, frame.arg1, frame.arg2
    ));
    user_runtime_status::record_syscall(frame.number);
    diagnostics::record_syscall_enter(
        frame.number as u64,
        frame.arg0 as u64,
        frame.arg1 as u64,
        frame.arg2 as u64,
    );

    let syscall_result = match frame.number {
        SYS_EXIT => handle_exit(frame.arg0 as i32, result),
        SYS_READ => read_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2),
        SYS_READV => readv_syscall(frame.arg0, frame.arg1 as *const UserIoVec, frame.arg2),
        SYS_WRITE => write_syscall(frame.arg0, frame.arg1 as *const u8, frame.arg2),
        SYS_WRITEV => writev_syscall(frame.arg0, frame.arg1 as *const UserIoVec, frame.arg2),
        SYS_DUP => duplicate_syscall(frame.arg0),
        SYS_CLOSE => close_syscall(frame.arg0),
        SYS_FCNTL => fcntl_syscall(frame.arg0, frame.arg1),
        SYS_POLL => poll_syscall(frame.arg0, frame.arg1 as u32),
        SYS_STAT_PATH => stat_path_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeFileStatusRecord,
        ),
        SYS_LSTAT_PATH => lstat_path_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeFileStatusRecord,
        ),
        SYS_STATFS_PATH => statfs_path_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeFileSystemStatusRecord,
        ),
        SYS_OPEN_PATH => open_path_syscall(frame.arg0, frame.arg1),
        SYS_LIST_PROCESSES => list_processes_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_PROCESS => {
            inspect_process_syscall(frame.arg0, frame.arg1 as *mut NativeProcessRecord)
        }
        SYS_GET_PROCESS_NAME => {
            get_process_name_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_PROCESS_IMAGE_PATH => {
            get_process_image_path_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_PROCESS_CWD => {
            get_process_cwd_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_SEND_SIGNAL => send_signal_syscall(frame.arg0, frame.arg1 as u8),
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
        SYS_MKDIR_PATH => mkdir_path_syscall(frame.arg0, frame.arg1),
        SYS_MKFILE_PATH => mkfile_path_syscall(frame.arg0, frame.arg1),
        SYS_MKCHAN_PATH => mkchan_path_syscall(frame.arg0, frame.arg1),
        SYS_SYMLINK_PATH => symlink_path_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3),
        SYS_RENAME_PATH => rename_path_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3),
        SYS_UNLINK_PATH => unlink_path_syscall(frame.arg0, frame.arg1),
        SYS_READLINK_PATH => {
            readlink_path_syscall(frame.arg0, frame.arg1, frame.arg2 as *mut u8, frame.arg3)
        }
        SYS_CREATE_EVENT_QUEUE => create_event_queue_syscall(frame.arg0 as u32),
        SYS_WAIT_EVENT_QUEUE => {
            wait_event_queue_syscall(frame.arg0, frame.arg1 as *mut NativeEventRecord, frame.arg2)
        }
        SYS_WATCH_RESOURCE_EVENTS => watch_resource_events_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeResourceEventWatchConfig,
        ),
        SYS_REMOVE_RESOURCE_EVENTS => {
            remove_resource_events_syscall(frame.arg0, frame.arg1, frame.arg2 as u64)
        }
        SYS_INSPECT_DEVICE => inspect_device_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeDeviceRecord,
        ),
        SYS_INSPECT_DRIVER => inspect_driver_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeDriverRecord,
        ),
        SYS_CREATE_DOMAIN => create_domain_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_CREATE_RESOURCE => {
            create_resource_syscall(frame.arg0, frame.arg1 as u32, frame.arg2, frame.arg3)
        }
        SYS_CREATE_CONTRACT => create_contract_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as u32,
            frame.arg3,
            frame.arg4,
        ),
        SYS_BIND_PROCESS_CONTRACT => bind_process_contract_syscall(frame.arg0),
        SYS_LIST_DOMAINS => list_domains_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_DOMAIN => {
            inspect_domain_syscall(frame.arg0, frame.arg1 as *mut NativeDomainRecord)
        }
        SYS_LIST_RESOURCES => list_resources_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_RESOURCE => {
            inspect_resource_syscall(frame.arg0, frame.arg1 as *mut NativeResourceRecord)
        }
        SYS_LIST_CONTRACTS => list_contracts_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_CONTRACT => {
            syscall_trace(format_args!(
                "ngos/x86_64: dispatch inspect_contract id={} out={:p}\n",
                frame.arg0, frame.arg1 as *mut NativeContractRecord
            ));
            inspect_contract_syscall(frame.arg0, frame.arg1 as *mut NativeContractRecord)
        }
        SYS_GET_DOMAIN_NAME => {
            get_domain_name_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_RESOURCE_NAME => {
            get_resource_name_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_CONTRACT_LABEL => {
            get_contract_label_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_SET_CONTRACT_STATE => set_contract_state_syscall(frame.arg0, frame.arg1 as u32),
        SYS_INVOKE_CONTRACT => {
            syscall_trace(format_args!(
                "ngos/x86_64: dispatch invoke_contract id={}\n",
                frame.arg0
            ));
            invoke_contract_syscall(frame.arg0)
        }
        SYS_RELEASE_RESOURCE => release_resource_syscall(frame.arg0),
        SYS_TRANSFER_RESOURCE => transfer_resource_syscall(frame.arg0, frame.arg1),
        SYS_SET_RESOURCE_POLICY => set_resource_policy_syscall(frame.arg0, frame.arg1 as u32),
        SYS_SET_RESOURCE_GOVERNANCE => {
            set_resource_governance_syscall(frame.arg0, frame.arg1 as u32)
        }
        SYS_SET_RESOURCE_CONTRACT_POLICY => {
            set_resource_contract_policy_syscall(frame.arg0, frame.arg1 as u32)
        }
        SYS_SET_RESOURCE_ISSUER_POLICY => {
            set_resource_issuer_policy_syscall(frame.arg0, frame.arg1 as u32)
        }
        SYS_SET_RESOURCE_STATE => set_resource_state_syscall(frame.arg0, frame.arg1 as u32),
        SYS_ACQUIRE_RESOURCE => acquire_resource_syscall(frame.arg0),
        SYS_CLAIM_RESOURCE => {
            claim_resource_syscall(frame.arg0, frame.arg1 as *mut NativeResourceClaimRecord)
        }
        SYS_RELEASE_CLAIMED_RESOURCE => {
            syscall_trace(format_args!(
                "ngos/x86_64: dispatch release_claimed_resource contract={} out={:p}\n",
                frame.arg0, frame.arg1 as *mut NativeResourceReleaseRecord
            ));
            release_claimed_resource_syscall(
                frame.arg0,
                frame.arg1 as *mut NativeResourceReleaseRecord,
            )
        }
        SYS_LIST_RESOURCE_WAITERS => {
            list_resource_waiters_syscall(frame.arg0, frame.arg1 as *mut u64, frame.arg2)
        }
        SYS_CANCEL_RESOURCE_CLAIM => {
            cancel_resource_claim_syscall(frame.arg0, frame.arg1 as *mut NativeResourceCancelRecord)
        }
        SYS_BOOT_REPORT => boot_report_syscall(
            frame.arg0 as u32,
            frame.arg1 as u32,
            frame.arg2 as i32,
            frame.arg3 as u64,
        ),
        _ => Err(Errno::Inval),
    };

    if frame.number != SYS_EXIT {
        let (ok, errno) = match &syscall_result {
            Ok(_) => (true, 0),
            Err(errno) => (false, *errno as u16),
        };
        diagnostics::record_syscall_exit(
            frame.number as u64,
            frame.arg0 as u64,
            frame.arg1 as u64,
            frame.arg2 as u64,
            ok,
            errno,
        );
        result.raw_return = encode_syscall_result(syscall_result);
        result.disposition = SyscallDisposition::Return as u64;
    }
}

fn handle_exit(code: i32, result: &mut SyscallDispatchResult) -> Result<usize, Errno> {
    unsafe {
        PROCESS_EXIT_CODE = code;
        PROCESS_EXITED = true;
    }
    user_runtime_status::mark_exit(code);
    serial::debug_marker(b'V');
    syscall_trace(format_args!(
        "ngos/x86_64: exit syscall handled code={}\n",
        code
    ));
    serial::debug_marker(b'W');
    syscall_trace(format_args!(
        "ngos/x86_64: process exit propagated code={} exited={}\n",
        code,
        unsafe { PROCESS_EXITED }
    ));
    user_runtime_status::emit_final_report_if_terminal();
    let _ = user_runtime_status::apply_configured_boot_outcome_policy();
    result.raw_return = 0;
    result.disposition = SyscallDisposition::Halt as u64;
    Ok(0)
}

fn duplicate_syscall(fd: usize) -> Result<usize, Errno> {
    let duplicated = DESCRIPTORS.with_mut(|descriptors| descriptors.duplicate(fd))?;
    serial::print(format_args!(
        "ngos/x86_64: dup handled fd={} -> {}\n",
        fd, duplicated
    ));
    Ok(duplicated)
}

fn close_syscall(fd: usize) -> Result<usize, Errno> {
    DESCRIPTORS.with_mut(|descriptors| descriptors.close(fd))?;
    serial::print(format_args!("ngos/x86_64: close handled fd={}\n", fd));
    Ok(0)
}

fn fcntl_syscall(fd: usize, encoded: usize) -> Result<usize, Errno> {
    let flags = DESCRIPTORS.with_mut(|descriptors| descriptors.fcntl(fd, encoded))?;
    serial::print(format_args!(
        "ngos/x86_64: fcntl handled fd={} encoded={:#x} result={:#x}\n",
        fd, encoded, flags
    ));
    Ok(flags)
}

fn poll_syscall(fd: usize, interest: u32) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: poll enter fd={} interest={:#x}\n",
        fd, interest
    ));
    let ready = DESCRIPTORS.with(|descriptors| descriptors.poll(fd, interest))?;
    serial::print(format_args!(
        "ngos/x86_64: poll handled fd={} interest={:#x} ready={:#x}\n",
        fd, interest, ready
    ));
    Ok(ready)
}

fn create_event_queue_syscall(mode_raw: u32) -> Result<usize, Errno> {
    let mode = NativeEventQueueMode::from_raw(mode_raw).ok_or(Errno::Inval)?;
    let fd = DESCRIPTORS.with_mut(|descriptors| descriptors.create_event_queue(mode))?;
    serial::print(format_args!(
        "ngos/x86_64: create_event_queue handled mode={} fd={}\n",
        mode_raw, fd
    ));
    Ok(fd)
}

fn wait_event_queue_syscall(
    fd: usize,
    buffer: *mut NativeEventRecord,
    capacity: usize,
) -> Result<usize, Errno> {
    let count =
        DESCRIPTORS.with_mut(|descriptors| descriptors.wait_event_queue(fd, buffer, capacity))?;
    serial::print(format_args!(
        "ngos/x86_64: wait_event_queue handled fd={} count={}\n",
        fd, count
    ));
    Ok(count)
}

fn watch_resource_events_syscall(
    fd: usize,
    resource: usize,
    config_ptr: *const NativeResourceEventWatchConfig,
) -> Result<usize, Errno> {
    if config_ptr.is_null() {
        return Err(Errno::Fault);
    }
    let config = unsafe { ptr::read(config_ptr) };
    DESCRIPTORS.with_mut(|descriptors| descriptors.watch_resource_events(fd, resource, config))?;
    serial::print(format_args!(
        "ngos/x86_64: watch_resource_events handled fd={} resource={} token={}\n",
        fd, resource, config.token
    ));
    Ok(0)
}

fn remove_resource_events_syscall(fd: usize, resource: usize, token: u64) -> Result<usize, Errno> {
    DESCRIPTORS.with_mut(|descriptors| descriptors.remove_resource_events(fd, resource, token))?;
    serial::print(format_args!(
        "ngos/x86_64: remove_resource_events handled fd={} resource={} token={}\n",
        fd, resource, token
    ));
    Ok(0)
}

fn read_syscall(fd: usize, buffer: *mut u8, len: usize) -> Result<usize, Errno> {
    let descriptor = DESCRIPTORS.with(|descriptors| descriptors.descriptor(fd))?;
    syscall_trace(format_args!(
        "ngos/x86_64: read enter fd={} target={:?} buffer={:#x} len={}\n",
        fd, descriptor.target, buffer as usize, len
    ));
    let path = match descriptor.target {
        DescriptorTarget::StorageDevice => DiagnosticsPath::Completion,
        DescriptorTarget::StorageDriver => DiagnosticsPath::Completion,
        DescriptorTarget::EventQueue(_) => DiagnosticsPath::Syscall,
        DescriptorTarget::GpuDevice
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::InputDevice => DiagnosticsPath::Syscall,
        _ => DiagnosticsPath::Syscall,
    };
    diagnostics::set_active_window(
        SYS_READ as u64,
        fd as u64,
        0,
        0x5354_4f52_4147_4530,
        0,
        path,
        diagnostics::replay_ids().request_id,
        diagnostics::replay_ids().completion_id,
    );
    let read = match descriptor.target {
        DescriptorTarget::Stdin => tty::read_stdin(buffer, len, descriptor.flags.nonblock)?,
        DescriptorTarget::StorageDevice => crate::virtio_blk_boot::read(
            crate::virtio_blk_boot::StorageEndpointKind::Device,
            buffer,
            len,
            descriptor.flags.nonblock,
        )?,
        DescriptorTarget::StorageDriver => crate::virtio_blk_boot::read(
            crate::virtio_blk_boot::StorageEndpointKind::Driver,
            buffer,
            len,
            descriptor.flags.nonblock,
        )?,
        DescriptorTarget::EventQueue(_) => return Err(Errno::Badf),
        DescriptorTarget::GpuDevice
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::InputDevice => 0,
        DescriptorTarget::BootFile(node) | DescriptorTarget::BootChannel(node) => DESCRIPTORS
            .with_mut(|descriptors| {
                let descriptor = descriptors.descriptor_mut(fd)?;
                boot_vfs_read(node, &mut descriptor.offset, buffer, len)
            })?,
        DescriptorTarget::Procfs(node) => DESCRIPTORS.with_mut(|descriptors| {
            let descriptor = descriptors.descriptor_mut(fd)?;
            boot_procfs_read(node, &mut descriptor.offset, buffer, len)
        })?,
        DescriptorTarget::Stdout | DescriptorTarget::Stderr => return Err(Errno::Badf),
    };

    syscall_trace(format_args!(
        "ngos/x86_64: read handled fd={} len={} read={}\n",
        fd, len, read
    ));
    if read != 0 {
        diagnostics::watch_touch(WatchKind::Read, buffer as u64, read as u64);
    }
    diagnostics::clear_active_window();
    Ok(read)
}

fn write_syscall(fd: usize, buffer: *const u8, len: usize) -> Result<usize, Errno> {
    let descriptor = DESCRIPTORS.with(|descriptors| descriptors.descriptor(fd))?;
    let path = match descriptor.target {
        DescriptorTarget::StorageDevice => DiagnosticsPath::Block,
        DescriptorTarget::StorageDriver => DiagnosticsPath::Block,
        DescriptorTarget::EventQueue(_) => DiagnosticsPath::Syscall,
        DescriptorTarget::GpuDevice
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::InputDevice => DiagnosticsPath::Syscall,
        _ => DiagnosticsPath::Syscall,
    };
    diagnostics::set_active_window(
        SYS_WRITE as u64,
        fd as u64,
        0,
        0x5354_4f52_4147_4530,
        0,
        path,
        0,
        0,
    );
    syscall_trace(format_args!(
        "ngos/x86_64: write enter fd={} target={:?} buffer={:#x} len={}\n",
        fd, descriptor.target, buffer as usize, len
    ));
    if len == 0 {
        syscall_trace(format_args!(
            "ngos/x86_64: write short-circuit fd={} len=0\n",
            fd
        ));
        return Ok(0);
    }
    if buffer.is_null() {
        syscall_trace(format_args!(
            "ngos/x86_64: write fault fd={} null-buffer\n",
            fd
        ));
        return Err(Errno::Fault);
    }

    let bytes = unsafe { slice::from_raw_parts(buffer, len) };
    let _ = diagnostics::guard_register(
        GuardKind::RequestBuffer,
        path,
        buffer as u64,
        len as u64,
        32,
        0,
        0,
    );
    let _ = diagnostics::watch_register(WatchKind::Touch, path, buffer as u64, len as u64, 0, 0);
    let _ = diagnostics::guard_check(buffer as u64, len as u64);
    diagnostics::watch_touch(WatchKind::Read, buffer as u64, len as u64);
    syscall_trace(format_args!(
        "ngos/x86_64: write bytes ready fd={} len={} first8={:?}\n",
        fd,
        len,
        &bytes[..bytes.len().min(8)]
    ));
    match descriptor.target {
        DescriptorTarget::Stdout => {
            let _ = tty::write_stdout(bytes);
        }
        DescriptorTarget::Stderr => {
            let _ = tty::write_stderr(bytes);
        }
        DescriptorTarget::StorageDevice => {
            syscall_trace(format_args!(
                "ngos/x86_64: write dispatch storage-device fd={} len={}\n",
                fd, len
            ));
            let result = crate::virtio_blk_boot::write(
                crate::virtio_blk_boot::StorageEndpointKind::Device,
                bytes,
            );
            syscall_trace(format_args!(
                "ngos/x86_64: write return storage-device fd={} result={:?}\n",
                fd, result
            ));
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::StorageDriver => {
            syscall_trace(format_args!(
                "ngos/x86_64: write dispatch storage-driver fd={} len={}\n",
                fd, len
            ));
            let result = crate::virtio_blk_boot::write(
                crate::virtio_blk_boot::StorageEndpointKind::Driver,
                bytes,
            );
            syscall_trace(format_args!(
                "ngos/x86_64: write return storage-driver fd={} result={:?}\n",
                fd, result
            ));
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::EventQueue(_) => return Err(Errno::Badf),
        DescriptorTarget::GpuDevice
        | DescriptorTarget::AudioDevice
        | DescriptorTarget::InputDevice => {
            diagnostics::clear_active_window();
            return Ok(bytes.len());
        }
        DescriptorTarget::BootFile(node) | DescriptorTarget::BootChannel(node) => {
            let result = DESCRIPTORS.with_mut(|descriptors| {
                let descriptor = descriptors.descriptor_mut(fd)?;
                boot_vfs_write(node, &mut descriptor.offset, bytes)
            });
            diagnostics::clear_active_window();
            return result;
        }
        DescriptorTarget::Procfs(_) => return Err(Errno::Badf),
        DescriptorTarget::Stdin => return Err(Errno::Badf),
    }
    if bytes
        .windows(b"desktop.boot stage=presented".len())
        .any(|window| window == b"desktop.boot stage=presented")
    {
        serial::disable_framebuffer_mirror();
        serial::print(format_args!(
            "ngos/x86_64: framebuffer serial mirror disabled reason=desktop-presented\n"
        ));
    }
    user_runtime_status::record_write(fd, bytes.len());
    syscall_trace(format_args!(
        "ngos/x86_64: write complete fd={} len={}\n",
        fd,
        bytes.len()
    ));
    diagnostics::clear_active_window();
    Ok(bytes.len())
}

fn readv_syscall(fd: usize, iovecs: *const UserIoVec, count: usize) -> Result<usize, Errno> {
    if count == 0 {
        return Ok(0);
    }
    if iovecs.is_null() {
        return Err(Errno::Fault);
    }
    let iovecs = unsafe { slice::from_raw_parts(iovecs, count) };
    let mut total = 0usize;
    for iov in iovecs {
        let read = read_syscall(fd, iov.base as *mut u8, iov.len)?;
        total += read;
        if read < iov.len {
            break;
        }
    }
    Ok(total)
}

fn writev_syscall(fd: usize, iovecs: *const UserIoVec, count: usize) -> Result<usize, Errno> {
    if count == 0 {
        return Ok(0);
    }
    if iovecs.is_null() {
        return Err(Errno::Fault);
    }
    let iovecs = unsafe { slice::from_raw_parts(iovecs, count) };
    let mut total = 0usize;
    for iov in iovecs {
        let written = write_syscall(fd, iov.base as *const u8, iov.len)?;
        total += written;
        if written < iov.len {
            break;
        }
    }
    Ok(total)
}

fn path_from_user<'a>(ptr_value: usize, len: usize) -> Result<&'a str, Errno> {
    if ptr_value == 0 {
        return Err(Errno::Fault);
    }
    let bytes = unsafe { slice::from_raw_parts(ptr_value as *const u8, len) };
    core::str::from_utf8(bytes).map_err(|_| Errno::Inval)
}

fn stat_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileStatusRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    if let Some(record) = boot_vfs_stat(path) {
        write_record(out, record)?;
        return Ok(0);
    }
    if let Some(node) = boot_procfs_node(path)? {
        let payload = boot_procfs_payload(node.pid, node.kind)?;
        write_record(
            out,
            NativeFileStatusRecord {
                inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                    acc.wrapping_mul(131).wrapping_add(*byte as u64)
                }),
                size: payload.len() as u64,
                kind: NativeObjectKind::File as u32,
                cloexec: 0,
                nonblock: 0,
                readable: 1,
                writable: 0,
            },
        )?;
        return Ok(0);
    }
    if matches!(boot_stream_target(path), Some(_)) {
        write_record(
            out,
            NativeFileStatusRecord {
                inode: path.as_bytes().iter().fold(0u64, |acc, byte| {
                    acc.wrapping_mul(131).wrapping_add(*byte as u64)
                }),
                size: 0,
                kind: NativeObjectKind::Device as u32,
                cloexec: 0,
                nonblock: 0,
                readable: 0,
                writable: 1,
            },
        )?;
        return Ok(0);
    }
    let inode = crate::virtio_blk_boot::inode_for_path(path).ok_or(Errno::NoEnt)?;
    let (size, kind, readable, writable) = if path == crate::virtio_blk_boot::STORAGE_DEVICE_PATH {
        let info = crate::virtio_blk_boot::device_record(path).ok_or(Errno::Nxio)?;
        (info.capacity_bytes, NativeObjectKind::Device as u32, 1, 1)
    } else {
        (0, NativeObjectKind::Driver as u32, 1, 0)
    };
    write_record(
        out,
        NativeFileStatusRecord {
            inode,
            size,
            kind,
            cloexec: 0,
            nonblock: 0,
            readable,
            writable,
        },
    )?;
    Ok(0)
}

fn lstat_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileStatusRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    if let Some(record) = boot_vfs_lstat(path) {
        write_record(out, record)?;
        return Ok(0);
    }
    stat_path_syscall(path_ptr, path_len, out)
}

fn statfs_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeFileSystemStatusRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    if boot_vfs_stat(path).is_some() {
        write_record(
            out,
            NativeFileSystemStatusRecord {
                mount_count: 1,
                node_count: BOOT_VFS.with_mut(|vfs| vfs.nodes.len()) as u64,
                read_only: 0,
                reserved: 0,
            },
        )?;
        return Ok(0);
    }
    if crate::virtio_blk_boot::inode_for_path(path).is_none() {
        return Err(Errno::NoEnt);
    }
    write_record(
        out,
        NativeFileSystemStatusRecord {
            mount_count: 1,
            node_count: 2,
            read_only: 0,
            reserved: 0,
        },
    )?;
    Ok(0)
}

fn mkdir_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    boot_vfs_create(path, BootNodeKind::Directory)?;
    Ok(0)
}

fn mkfile_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    boot_vfs_create(path, BootNodeKind::File)?;
    Ok(0)
}

fn mkchan_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    boot_vfs_create(path, BootNodeKind::Channel)?;
    Ok(0)
}

fn symlink_path_syscall(
    path_ptr: usize,
    path_len: usize,
    target_ptr: usize,
    target_len: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let target = path_from_user(target_ptr, target_len)?;
    boot_vfs_symlink(path, target)?;
    Ok(0)
}

fn rename_path_syscall(
    from_ptr: usize,
    from_len: usize,
    to_ptr: usize,
    to_len: usize,
) -> Result<usize, Errno> {
    let from = path_from_user(from_ptr, from_len)?;
    let to = path_from_user(to_ptr, to_len)?;
    boot_vfs_rename(from, to)?;
    Ok(0)
}

fn unlink_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    boot_vfs_unlink(path)?;
    Ok(0)
}

fn readlink_path_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let target = boot_vfs_readlink(path)?;
    copy_text_to_user(&target, out, capacity)
}

fn open_path_syscall(path_ptr: usize, path_len: usize) -> Result<usize, Errno> {
    serial::debug_marker(b'x');
    serial::print(format_args!(
        "ngos/x86_64: open_path enter path_ptr={:#x} path_len={}\n",
        path_ptr, path_len
    ));
    let path = path_from_user(path_ptr, path_len)?;
    serial::print(format_args!(
        "ngos/x86_64: open_path path decoded={}\n",
        path
    ));
    let fd = DESCRIPTORS.with_mut(|descriptors| descriptors.open_path(path))?;
    serial::print(format_args!(
        "ngos/x86_64: open_path handled path={} fd={}\n",
        path, fd
    ));
    Ok(fd)
}

fn list_processes_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let ids = registry
            .entries
            .iter()
            .filter(|entry| !entry.reaped)
            .map(|entry| entry.pid)
            .collect::<Vec<_>>();
        copy_ids_to_user(&ids, buffer, capacity)
    })
}

fn inspect_process_syscall(pid: usize, out: *mut NativeProcessRecord) -> Result<usize, Errno> {
    let record = BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &registry.entries[index];
        Ok(NativeProcessRecord {
            pid: entry.pid,
            parent: entry.parent,
            address_space: entry.pid,
            main_thread: entry.pid,
            state: entry.state,
            exit_code: entry.exit_code,
            descriptor_count: 3,
            capability_count: 0,
            environment_count: entry.env_count,
            memory_region_count: entry.vm_objects.len() as u64,
            thread_count: 1,
            pending_signal_count: entry.pending_signal_count,
            session_reported: 0,
            session_status: 0,
            session_stage: 0,
            scheduler_class: NativeSchedulerClass::Interactive as u32,
            scheduler_budget: 2,
            cpu_runtime_ticks: 0,
            execution_contract: entry.contract_bindings.execution,
            memory_contract: entry.contract_bindings.memory,
            io_contract: entry.contract_bindings.io,
            observe_contract: entry.contract_bindings.observe,
            reserved: 0,
        })
    })?;
    write_record(out, record)?;
    Ok(0)
}

fn get_process_name_syscall(pid: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        copy_name_to_user(
            &InlineName::from_text(&registry.entries[index].name)?,
            buffer,
            capacity,
        )
    })
}

fn get_process_image_path_syscall(
    pid: usize,
    buffer: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        copy_text_to_user(&registry.entries[index].image_path, buffer, capacity)
    })
}

fn get_process_cwd_syscall(pid: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        copy_text_to_user(&registry.entries[index].cwd, buffer, capacity)
    })
}

fn send_signal_syscall(pid: usize, signal: u8) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        entry.pending_signal_count = entry.pending_signal_count.saturating_add(1);
        if signal != 0 {
            entry.state = 4;
            entry.exit_code = 128 + signal as i32;
            entry.pending_signal_count = 0;
        }
        Ok(0)
    })
}

fn spawn_path_process_syscall(
    name_ptr: usize,
    name_len: usize,
    path_ptr: usize,
    path_len: usize,
) -> Result<usize, Errno> {
    let name = string_from_user(name_ptr, name_len)?;
    let path = string_from_user(path_ptr, path_len)?;
    BOOT_PROCESSES
        .with_mut(|registry| registry.spawn(name, path, String::from("/"), 1, 0))
        .map(|pid| pid as usize)
}

fn spawn_process_copy_vm_syscall(
    name_ptr: usize,
    name_len: usize,
    path_ptr: usize,
    path_len: usize,
    source_pid: usize,
) -> Result<usize, Errno> {
    let name = string_from_user(name_ptr, name_len)?;
    let path = string_from_user(path_ptr, path_len)?;
    let pid =
        BOOT_PROCESSES.with_mut(|registry| registry.spawn(name, path, String::from("/"), 1, 0))?;
    boot_copy_vm_state(source_pid as u64, pid)?;
    Ok(pid as usize)
}

fn spawn_configured_process_syscall(config_ptr: usize) -> Result<usize, Errno> {
    let config = copy_struct_from_user::<NativeSpawnProcessConfig>(config_ptr)?;
    let name = string_from_user(config.name_ptr, config.name_len)?;
    let path = string_from_user(config.path_ptr, config.path_len)?;
    let cwd = string_from_user(config.cwd_ptr, config.cwd_len)?;
    let argv = string_table_from_user(config.argv_ptr, config.argv_len, config.argv_count)?;
    let envp = string_table_from_user(config.envp_ptr, config.envp_len, config.envp_count)?;
    BOOT_PROCESSES
        .with_mut(|registry| registry.spawn(name, path, cwd, argv.len(), envp.len()))
        .map(|pid| pid as usize)
}

fn set_process_args_syscall(
    pid: usize,
    argv_ptr: usize,
    argv_len: usize,
    argv_count: usize,
) -> Result<usize, Errno> {
    let argv = string_table_from_user(argv_ptr, argv_len, argv_count)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].argv_count = argv.len() as u64;
        Ok(0)
    })
}

fn set_process_env_syscall(
    pid: usize,
    env_ptr: usize,
    env_len: usize,
    env_count: usize,
) -> Result<usize, Errno> {
    let envp = string_table_from_user(env_ptr, env_len, env_count)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].env_count = envp.len() as u64;
        Ok(0)
    })
}

fn set_process_cwd_syscall(pid: usize, cwd_ptr: usize, cwd_len: usize) -> Result<usize, Errno> {
    let cwd = string_from_user(cwd_ptr, cwd_len)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        registry.entries[index].cwd = cwd;
        Ok(0)
    })
}

fn reap_process_syscall(pid: usize) -> Result<usize, Errno> {
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if entry.state != 4 {
            return Err(Errno::Again);
        }
        entry.reaped = true;
        Ok(entry.exit_code as usize)
    })
}

fn read_procfs_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut u8,
    capacity: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let node = boot_procfs_node(path)?.ok_or(Errno::NoEnt)?;
    let payload = boot_procfs_payload(node.pid, node.kind)?;
    copy_text_to_user(&payload, out, capacity)
}

fn load_memory_word_syscall(pid: usize, addr: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, addr, 4, 9)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let share_key = {
            let entry = &registry.entries[index];
            let Some(object_index) = entry.vm_objects.iter().position(|object| {
                !object.quarantined
                    && object.readable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            }) else {
                let entry = &mut registry.entries[index];
                if let Some(object) = entry.vm_objects.iter().find(|object| {
                    object.quarantined
                        && (addr as u64) >= object.start
                        && (addr as u64) < object.start.saturating_add(object.len)
                }) {
                    entry.vm_decisions.push(BootVmDecision {
                        agent: "quarantine-block",
                        vm_object_id: object.id,
                        start: addr as u64,
                        len: 8,
                        detail0: object.quarantine_reason,
                        detail1: 0,
                    });
                    return Err(Errno::Fault);
                }
                return Err(Errno::Fault);
            };
            entry.vm_objects[object_index].share_key
        };
        let owners = boot_vm_owner_count(registry, share_key);
        let entry = &mut registry.entries[index];
        let object_index = entry
            .vm_objects
            .iter()
            .position(|object| {
                !object.quarantined
                    && object.readable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            })
            .ok_or(Errno::Fault)?;
        boot_vm_touch_object_page(entry, object_index, addr as u64, false, owners)?;
        Ok(0)
    })
}

fn store_memory_word_syscall(pid: usize, addr: usize, _value: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, addr, 4, 11)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let share_key = {
            let entry = &registry.entries[index];
            let Some(object_index) = entry.vm_objects.iter().position(|object| {
                !object.quarantined
                    && object.writable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            }) else {
                let entry = &mut registry.entries[index];
                if let Some(object) = entry.vm_objects.iter().find(|object| {
                    object.quarantined
                        && (addr as u64) >= object.start
                        && (addr as u64) < object.start.saturating_add(object.len)
                }) {
                    entry.vm_decisions.push(BootVmDecision {
                        agent: "quarantine-block",
                        vm_object_id: object.id,
                        start: addr as u64,
                        len: 8,
                        detail0: object.quarantine_reason,
                        detail1: 1,
                    });
                    return Err(Errno::Fault);
                }
                return Err(Errno::Fault);
            };
            entry.vm_objects[object_index].share_key
        };
        let owners = boot_vm_owner_count(registry, share_key);
        let entry = &mut registry.entries[index];
        let object_index = entry
            .vm_objects
            .iter()
            .position(|object| {
                !object.quarantined
                    && object.writable
                    && (addr as u64) >= object.start
                    && (addr as u64) < object.start.saturating_add(object.len)
            })
            .ok_or(Errno::Fault)?;
        boot_vm_touch_object_page(entry, object_index, addr as u64, true, owners)?;
        Ok(0)
    })
}

fn quarantine_vm_object_syscall(
    pid: usize,
    vm_object_id: usize,
    reason: usize,
) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 7)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let object = entry
            .vm_objects
            .iter_mut()
            .find(|object| object.id == vm_object_id as u64)
            .ok_or(Errno::NoEnt)?;
        object.quarantined = true;
        object.quarantine_reason = reason as u64;
        entry.vm_decisions.push(BootVmDecision {
            agent: "quarantine-state",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: reason as u64,
            detail1: 1,
        });
        Ok(0)
    })
}

fn release_vm_object_syscall(pid: usize, vm_object_id: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 8)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let object = entry
            .vm_objects
            .iter_mut()
            .find(|object| object.id == vm_object_id as u64)
            .ok_or(Errno::NoEnt)?;
        let reason = object.quarantine_reason;
        object.quarantined = false;
        object.quarantine_reason = 0;
        entry.vm_decisions.push(BootVmDecision {
            agent: "quarantine-state",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: reason,
            detail1: 0,
        });
        Ok(0)
    })
}

fn map_anonymous_memory_syscall(
    pid: usize,
    length: usize,
    label_ptr: usize,
    label_len: usize,
) -> Result<usize, Errno> {
    let label = string_from_user(label_ptr, label_len)?;
    BootVmPolicyEnforcementAgent::enforce(pid, 0, length, 0)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = entry.next_vm_addr;
        let len = (length as u64).max(0x1000).next_multiple_of(0x1000);
        let object_id = entry.next_vm_object_id;
        entry.next_vm_addr = entry
            .next_vm_addr
            .saturating_add(len)
            .saturating_add(0x1000);
        entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
        entry.vm_objects.push(BootVmObject {
            id: object_id,
            start,
            len,
            name: format!("[anon:{}]", label),
            kind: "Anonymous",
            share_key: object_id,
            shadow_source_id: None,
            shadow_source_offset: 0,
            shadow_depth: 0,
            private_mapping: true,
            file_offset: 0,
            readable: true,
            writable: true,
            executable: false,
            read_fault_count: 0,
            write_fault_count: 0,
            cow_fault_count: 0,
            committed_pages: len / 0x1000,
            resident_pages: 0,
            dirty_pages: 0,
            accessed_pages: 0,
            quarantined: false,
            quarantine_reason: 0,
            page_states: vec![BootVmPageState::default(); boot_vm_page_count_for_len(len)],
        });
        entry.vm_decisions.push(BootVmDecision {
            agent: "map",
            vm_object_id: object_id,
            start,
            len,
            detail0: 1,
            detail1: entry.vm_objects.len() as u64,
        });
        Ok(start as usize)
    })
}

fn map_file_backed_memory_boot(
    pid: usize,
    path_ptr: usize,
    path_len: usize,
    length: usize,
    offset: usize,
    readable: usize,
    writable: usize,
    executable: usize,
    private_mapping: usize,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let len = (length as u64).max(0x1000).next_multiple_of(0x1000);
    let offset = offset as u64;
    let file_size = boot_vfs_file_size(path)? as u64;
    if offset >= file_size || offset.saturating_add(len) > file_size {
        return Err(Errno::Inval);
    }
    BootVmPolicyEnforcementAgent::enforce(pid, 0, length, 1)?;

    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = entry.next_vm_addr;
        let object_id = entry.next_vm_object_id;
        entry.next_vm_addr = entry
            .next_vm_addr
            .saturating_add(len)
            .saturating_add(0x1000);
        entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
        entry.vm_objects.push(BootVmObject {
            id: object_id,
            start,
            len,
            name: String::from(path),
            kind: "File",
            share_key: object_id,
            shadow_source_id: None,
            shadow_source_offset: 0,
            shadow_depth: 0,
            private_mapping: private_mapping != 0,
            file_offset: offset,
            readable: readable != 0,
            writable: writable != 0,
            executable: executable != 0,
            read_fault_count: 0,
            write_fault_count: 0,
            cow_fault_count: 0,
            committed_pages: len / 0x1000,
            resident_pages: 0,
            dirty_pages: 0,
            accessed_pages: 0,
            quarantined: false,
            quarantine_reason: 0,
            page_states: vec![BootVmPageState::default(); boot_vm_page_count_for_len(len)],
        });
        entry.vm_decisions.push(BootVmDecision {
            agent: "map-file",
            vm_object_id: object_id,
            start,
            len,
            detail0: offset,
            detail1: ((readable != 0) as u64)
                | (((writable != 0) as u64) << 1)
                | (((executable != 0) as u64) << 2)
                | (((private_mapping != 0) as u64) << 3),
        });
        Ok(start as usize)
    })
}

fn split_vm_object_at(
    entry: &mut BootProcessEntry,
    object_index: usize,
    split_at: u64,
) -> Option<usize> {
    if object_index >= entry.vm_objects.len() {
        return None;
    }
    let object = entry.vm_objects[object_index].clone();
    if split_at <= object.start || split_at >= object.start.saturating_add(object.len) {
        return None;
    }

    let left_len = split_at.saturating_sub(object.start);
    let right_len = object.len.saturating_sub(left_len);
    let left_pages = left_len / 0x1000;
    let right_pages = right_len / 0x1000;
    let left_page_count = left_pages as usize;

    entry.vm_objects[object_index].len = left_len;
    entry.vm_objects[object_index].page_states = object.page_states[..left_page_count].to_vec();
    boot_vm_recount_object_pages(&mut entry.vm_objects[object_index]);

    let right = BootVmObject {
        id: entry.next_vm_object_id,
        start: split_at,
        len: right_len,
        name: object.name,
        kind: object.kind,
        share_key: object.share_key,
        shadow_source_id: object.shadow_source_id,
        shadow_source_offset: object.shadow_source_offset.saturating_add(left_len),
        shadow_depth: object.shadow_depth,
        private_mapping: object.private_mapping,
        file_offset: if object.kind == "File" {
            object.file_offset.saturating_add(left_len)
        } else {
            object.file_offset
        },
        readable: object.readable,
        writable: object.writable,
        executable: object.executable,
        read_fault_count: object.read_fault_count,
        write_fault_count: object.write_fault_count,
        cow_fault_count: object.cow_fault_count,
        committed_pages: right_pages,
        resident_pages: 0,
        dirty_pages: 0,
        accessed_pages: 0,
        quarantined: object.quarantined,
        quarantine_reason: object.quarantine_reason,
        page_states: object.page_states[left_page_count..].to_vec(),
    };
    let left_committed = entry.vm_objects[object_index].committed_pages.max(1);
    let right_committed = right.committed_pages.max(1);
    let total_committed = left_committed.saturating_add(right_committed).max(1);
    entry.vm_objects[object_index].read_fault_count =
        object.read_fault_count.saturating_mul(left_committed) / total_committed;
    entry.vm_objects[object_index].write_fault_count =
        object.write_fault_count.saturating_mul(left_committed) / total_committed;
    entry.vm_objects[object_index].cow_fault_count =
        object.cow_fault_count.saturating_mul(left_committed) / total_committed;
    let mut right = right;
    right.read_fault_count = object
        .read_fault_count
        .saturating_sub(entry.vm_objects[object_index].read_fault_count);
    right.write_fault_count = object
        .write_fault_count
        .saturating_sub(entry.vm_objects[object_index].write_fault_count);
    right.cow_fault_count = object
        .cow_fault_count
        .saturating_sub(entry.vm_objects[object_index].cow_fault_count);
    boot_vm_recount_object_pages(&mut right);
    entry.next_vm_object_id = entry.next_vm_object_id.saturating_add(1);
    entry.vm_objects.insert(object_index + 1, right);
    Some(object_index + 1)
}

fn vm_object_index_for_range(entry: &BootProcessEntry, start: u64, len: u64) -> Option<usize> {
    if len == 0 {
        return None;
    }
    entry.vm_objects.iter().position(|object| {
        let object_end = object.start.saturating_add(object.len);
        let range_end = start.saturating_add(len);
        start < object_end && range_end > object.start
    })
}

fn protect_memory_range_syscall(
    pid: usize,
    start: usize,
    len: usize,
    readable: usize,
    writable: usize,
    executable: usize,
) -> Result<usize, Errno> {
    if len == 0 {
        return Err(Errno::Inval);
    }
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 3)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = start as u64;
        let len = (len as u64).next_multiple_of(0x1000);
        let end = start.saturating_add(len);
        let Some(mut object_index) = vm_object_index_for_range(entry, start, len) else {
            return Err(Errno::Fault);
        };

        if start > entry.vm_objects[object_index].start {
            object_index = split_vm_object_at(entry, object_index, start).unwrap_or(object_index);
        }

        let mut cursor = object_index;
        let mut touched = 0usize;
        while cursor < entry.vm_objects.len() {
            let object_start = entry.vm_objects[cursor].start;
            if object_start >= end {
                break;
            }
            let object_end = object_start.saturating_add(entry.vm_objects[cursor].len);
            if end < object_end {
                split_vm_object_at(entry, cursor, end);
            }
            let object = &mut entry.vm_objects[cursor];
            object.readable = readable != 0;
            object.writable = writable != 0;
            object.executable = executable != 0;
            let object_id = object.id;
            let object_start = object.start;
            let object_len = object.len;
            entry.vm_decisions.push(BootVmDecision {
                agent: "protect",
                vm_object_id: object_id,
                start: object_start,
                len: object_len,
                detail0: ((readable != 0) as u64)
                    | (((writable != 0) as u64) << 1)
                    | (((executable != 0) as u64) << 2),
                detail1: 0,
            });
            touched += 1;
            cursor += 1;
        }

        if touched == 0 {
            Err(Errno::Fault)
        } else {
            Ok(0)
        }
    })
}

fn unmap_memory_range_syscall(pid: usize, start: usize, len: usize) -> Result<usize, Errno> {
    if len == 0 {
        return Err(Errno::Inval);
    }
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 2)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let start = start as u64;
        let len = (len as u64).next_multiple_of(0x1000);
        let end = start.saturating_add(len);
        let Some(mut object_index) = vm_object_index_for_range(entry, start, len) else {
            return Err(Errno::Fault);
        };

        if start > entry.vm_objects[object_index].start {
            object_index = split_vm_object_at(entry, object_index, start).unwrap_or(object_index);
        }

        let mut removed = 0usize;
        while object_index < entry.vm_objects.len() {
            let object_start = entry.vm_objects[object_index].start;
            if object_start >= end {
                break;
            }
            let object_end = object_start.saturating_add(entry.vm_objects[object_index].len);
            if end < object_end {
                split_vm_object_at(entry, object_index, end);
            }
            let object = entry.vm_objects.remove(object_index);
            entry.vm_decisions.push(BootVmDecision {
                agent: "unmap",
                vm_object_id: object.id,
                start: object.start,
                len: object.len,
                detail0: object.committed_pages,
                detail1: 0,
            });
            removed += 1;
        }

        if removed == 0 {
            Err(Errno::Fault)
        } else {
            Ok(0)
        }
    })
}

fn set_process_break_vm_syscall(pid: usize, new_end: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 13)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        let object = entry
            .vm_objects
            .iter_mut()
            .find(|object| object.kind == "Heap")
            .ok_or(Errno::NoEnt)?;
        let old_end = object.start.saturating_add(object.len);
        let requested = new_end as u64;
        if requested <= object.start {
            return Err(Errno::Inval);
        }
        object.len = requested
            .saturating_sub(object.start)
            .next_multiple_of(0x1000);
        object.page_states.resize(
            boot_vm_page_count_for_len(object.len),
            BootVmPageState::default(),
        );
        boot_vm_recount_object_pages(object);
        entry.vm_decisions.push(BootVmDecision {
            agent: "brk",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: old_end,
            detail1: object.start.saturating_add(object.len),
        });
        Ok((object.start.saturating_add(object.len)) as usize)
    })
}

fn sync_memory_range_syscall(pid: usize, start: usize, len: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 5)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if let Some(object) = entry.vm_objects.iter_mut().find(|object| {
            (start as u64) >= object.start
                && (start as u64) < object.start.saturating_add(object.len)
        }) {
            for page in &mut object.page_states {
                page.dirty = false;
            }
            boot_vm_recount_object_pages(object);
            entry.vm_decisions.push(BootVmDecision {
                agent: "sync",
                vm_object_id: object.id,
                start: start as u64,
                len: len as u64,
                detail0: object.committed_pages,
                detail1: 1,
            });
            Ok(0)
        } else {
            Err(Errno::Fault)
        }
    })
}

fn advise_memory_range_syscall(
    pid: usize,
    start: usize,
    len: usize,
    advice: usize,
) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, start, len, 4)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        if let Some(object) = entry.vm_objects.iter_mut().find(|object| {
            (start as u64) >= object.start
                && (start as u64) < object.start.saturating_add(object.len)
        }) {
            if advice == 4 {
                for page in &mut object.page_states {
                    page.resident = false;
                    page.dirty = false;
                    page.accessed = false;
                }
            } else if advice == 3 {
                for page in &mut object.page_states {
                    page.resident = true;
                    page.accessed = true;
                }
            }
            boot_vm_recount_object_pages(object);
            entry.vm_decisions.push(BootVmDecision {
                agent: "advice",
                vm_object_id: object.id,
                start: start as u64,
                len: len as u64,
                detail0: advice as u64,
                detail1: object.resident_pages,
            });
            Ok(0)
        } else {
            Err(Errno::Fault)
        }
    })
}

fn reclaim_memory_pressure_syscall(pid: usize, target_pages: usize) -> Result<usize, Errno> {
    BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 14)?;
    BOOT_PROCESSES.with_mut(|registry| {
        let Some(index) = registry.find_index(pid as u64) else {
            return Err(Errno::Srch);
        };
        let entry = &mut registry.entries[index];
        entry.vm_decisions.push(BootVmDecision {
            agent: "pressure-trigger",
            vm_object_id: 0,
            start: 0,
            len: target_pages as u64,
            detail0: target_pages as u64,
            detail1: entry.vm_objects.len() as u64,
        });
        let mut reclaimed = 0u64;
        for object in &mut entry.vm_objects {
            if reclaimed >= target_pages as u64 || object.quarantined {
                break;
            }
            if object.dirty_pages > 0 {
                for page in &mut object.page_states {
                    page.dirty = false;
                }
                boot_vm_recount_object_pages(object);
                entry.vm_decisions.push(BootVmDecision {
                    agent: "sync",
                    vm_object_id: object.id,
                    start: object.start,
                    len: object.len,
                    detail0: object.committed_pages,
                    detail1: 1,
                });
            }
            let victim = object
                .resident_pages
                .min((target_pages as u64).saturating_sub(reclaimed));
            let mut evicted = 0u64;
            let mut cursor = object.page_states.len();
            while cursor > 0 && evicted < victim {
                cursor -= 1;
                if object.page_states[cursor].resident {
                    object.page_states[cursor].resident = false;
                    object.page_states[cursor].dirty = false;
                    object.page_states[cursor].accessed = false;
                    evicted += 1;
                }
            }
            boot_vm_recount_object_pages(object);
            entry.vm_decisions.push(BootVmDecision {
                agent: "pressure-victim",
                vm_object_id: object.id,
                start: object.start,
                len: object.len,
                detail0: evicted,
                detail1: object.committed_pages,
            });
            reclaimed = reclaimed.saturating_add(evicted);
        }
        Ok(reclaimed as usize)
    })
}

fn reclaim_memory_pressure_global_syscall(target_pages: usize) -> Result<usize, Errno> {
    if let Some(pid) = BOOT_PROCESSES.with_mut(|registry| {
        registry
            .entries
            .iter()
            .find(|entry| !entry.reaped)
            .map(|entry| entry.pid as usize)
    }) {
        BootVmPolicyEnforcementAgent::enforce(pid, 0, 0, 14)?;
    }
    BOOT_PROCESSES.with_mut(|registry| {
        let mut best_pid = None;
        let mut best_index = None;
        let mut best_pages = 0u64;
        for entry in &registry.entries {
            if entry.reaped {
                continue;
            }
            if let Some((index, object)) = entry
                .vm_objects
                .iter()
                .enumerate()
                .filter(|(_, object)| !object.quarantined)
                .max_by_key(|(_, object)| object.resident_pages)
            {
                if object.resident_pages > best_pages {
                    best_pages = object.resident_pages;
                    best_pid = Some(entry.pid);
                    best_index = Some(index);
                }
            }
        }
        let Some(pid) = best_pid else {
            return Ok(0);
        };
        let entry = registry
            .entries
            .iter_mut()
            .find(|entry| entry.pid == pid && !entry.reaped)
            .ok_or(Errno::Srch)?;
        entry.vm_decisions.push(BootVmDecision {
            agent: "pressure-trigger",
            vm_object_id: 0,
            start: 0,
            len: target_pages as u64,
            detail0: target_pages as u64,
            detail1: 1,
        });
        let object = &mut entry.vm_objects[best_index.unwrap()];
        if object.dirty_pages > 0 {
            for page in &mut object.page_states {
                page.dirty = false;
            }
            boot_vm_recount_object_pages(object);
            entry.vm_decisions.push(BootVmDecision {
                agent: "sync",
                vm_object_id: object.id,
                start: object.start,
                len: object.len,
                detail0: object.committed_pages,
                detail1: 1,
            });
        }
        let reclaimed = object.resident_pages.min(target_pages as u64);
        let mut evicted = 0u64;
        let mut cursor = object.page_states.len();
        while cursor > 0 && evicted < reclaimed {
            cursor -= 1;
            if object.page_states[cursor].resident {
                object.page_states[cursor].resident = false;
                object.page_states[cursor].dirty = false;
                object.page_states[cursor].accessed = false;
                evicted += 1;
            }
        }
        boot_vm_recount_object_pages(object);
        entry.vm_decisions.push(BootVmDecision {
            agent: "pressure-victim",
            vm_object_id: object.id,
            start: object.start,
            len: object.len,
            detail0: evicted,
            detail1: object.committed_pages,
        });
        Ok(evicted as usize)
    })
}

fn inspect_device_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeDeviceRecord,
) -> Result<usize, Errno> {
    let path = path_from_user(path_ptr, path_len)?;
    let record = crate::virtio_blk_boot::device_record(path).ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    serial::print(format_args!(
        "ngos/x86_64: inspect_device handled path={} class={} block_size={} capacity={}\n",
        path, record.class, record.block_size, record.capacity_bytes
    ));
    Ok(0)
}

fn inspect_driver_syscall(
    path_ptr: usize,
    path_len: usize,
    out: *mut NativeDriverRecord,
) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: inspect_driver enter path_ptr={:#x} path_len={} out={:#x}\n",
        path_ptr, path_len, out as usize
    ));
    let path = path_from_user(path_ptr, path_len)?;
    serial::print(format_args!(
        "ngos/x86_64: inspect_driver path decoded={}\n",
        path
    ));
    let record = crate::virtio_blk_boot::driver_record(path).ok_or(Errno::NoEnt)?;
    write_record(out, record)?;
    serial::print(format_args!(
        "ngos/x86_64: inspect_driver handled path={} bound={} queued={} completed={}\n",
        path, record.bound_device_count, record.queued_requests, record.completed_requests
    ));
    Ok(0)
}

fn boot_report_syscall(status: u32, stage: u32, code: i32, detail: u64) -> Result<usize, Errno> {
    if BootSessionStatus::from_raw(status).is_none() {
        return Err(Errno::Inval);
    }
    if BootSessionStage::from_raw(stage).is_none() {
        return Err(Errno::Inval);
    }
    user_runtime_status::record_boot_report(BootSessionReport {
        status,
        stage,
        code,
        reserved: 0,
        detail,
    })
    .map_err(|_| Errno::Inval)?;
    serial::print(format_args!(
        "ngos/x86_64: boot report handled status={} stage={} code={} detail={:#x}\n",
        status, stage, code, detail
    ));
    Ok(0)
}

fn create_domain_syscall(parent: usize, name_ptr: usize, name_len: usize) -> Result<usize, Errno> {
    let name = read_inline_name(name_ptr, name_len)?;
    let id = NATIVE_REGISTRY.with_mut(|registry| registry.create_domain(parent as u64, name))?;
    serial::print(format_args!(
        "ngos/x86_64: create_domain handled parent={} id={} name={}\n",
        parent,
        id,
        core::str::from_utf8(name.as_bytes()).unwrap_or("<bin>")
    ));
    Ok(id)
}

fn create_resource_syscall(
    domain: usize,
    kind_raw: u32,
    name_ptr: usize,
    name_len: usize,
) -> Result<usize, Errno> {
    let kind = NativeResourceKind::from_raw(kind_raw).ok_or(Errno::Inval)?;
    let name = read_inline_name(name_ptr, name_len)?;
    let id = NATIVE_REGISTRY.with_mut(|registry| registry.create_resource(domain, kind, name))?;
    serial::print(format_args!(
        "ngos/x86_64: create_resource handled domain={} id={} kind={} name={}\n",
        domain,
        id,
        kind_raw,
        core::str::from_utf8(name.as_bytes()).unwrap_or("<bin>")
    ));
    Ok(id)
}

fn create_contract_syscall(
    domain: usize,
    resource: usize,
    kind_raw: u32,
    label_ptr: usize,
    label_len: usize,
) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_CREATE_CONTRACT,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    let kind = NativeContractKind::from_raw(kind_raw).ok_or(Errno::Inval)?;
    diagnostics::record_function_checkpoint(
        FN_CREATE_CONTRACT,
        1,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    let label = read_inline_name(label_ptr, label_len)?;
    diagnostics::record_function_checkpoint(
        FN_CREATE_CONTRACT,
        2,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    let id = NATIVE_REGISTRY
        .with_mut(|registry| registry.create_contract(domain, resource, kind, label))?;
    diagnostics::record_function_checkpoint(
        FN_CREATE_CONTRACT,
        4,
        kind_raw as u64,
        domain as u64,
        resource as u64,
    );
    diagnostics::record_function_exit(
        FN_CREATE_CONTRACT,
        6,
        kind_raw as u64,
        domain as u64,
        resource as u64,
        true,
        0,
    );
    serial::print(format_args!(
        "ngos/x86_64: create_contract handled domain={} resource={} id={} kind={} label={}\n",
        domain,
        resource,
        id,
        kind_raw,
        core::str::from_utf8(label.as_bytes()).unwrap_or("<bin>")
    ));
    Ok(id)
}

fn list_domains_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let mut ids = [0u64; MAX_DOMAIN_COUNT];
        let mut count = 0usize;
        for entry in registry.domains.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_domain_syscall(id: usize, out: *mut NativeDomainRecord) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let entry = registry.domain(id)?;
        write_record(
            out,
            NativeDomainRecord {
                id: entry.id,
                owner: entry.owner,
                parent: entry.parent,
                resource_count: entry.resource_count,
                contract_count: entry.contract_count,
            },
        )?;
        Ok(0)
    })
}

fn list_resources_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let mut ids = [0u64; MAX_RESOURCE_COUNT];
        let mut count = 0usize;
        for entry in registry.resources.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_resource_syscall(id: usize, out: *mut NativeResourceRecord) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let entry = registry.resource(id)?;
        write_record(
            out,
            NativeResourceRecord {
                id: entry.id,
                domain: entry.domain,
                creator: entry.creator,
                holder_contract: entry.holder_contract,
                kind: entry.kind as u32,
                state: entry.state as u32,
                arbitration: entry.arbitration as u32,
                governance: entry.governance as u32,
                contract_policy: entry.contract_policy as u32,
                issuer_policy: entry.issuer_policy as u32,
                waiting_count: entry.waiting_count as u64,
                acquire_count: entry.acquire_count,
                handoff_count: entry.handoff_count,
            },
        )?;
        Ok(0)
    })
}

fn list_contracts_syscall(buffer: *mut u64, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| {
        let mut ids = [0u64; MAX_CONTRACT_COUNT];
        let mut count = 0usize;
        for entry in registry.contracts.iter().flatten() {
            ids[count] = entry.id;
            count += 1;
        }
        copy_ids_to_user(&ids[..count], buffer, capacity)
    })
}

fn inspect_contract_syscall(id: usize, out: *mut NativeContractRecord) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: inspect_contract enter id={} out={:p}\n",
        id, out
    ));
    NATIVE_REGISTRY.with(|registry| {
        let entry = registry.contract(id)?;
        write_record(
            out,
            NativeContractRecord {
                id: entry.id,
                domain: entry.domain,
                resource: entry.resource,
                issuer: entry.issuer,
                kind: entry.kind as u32,
                state: entry.state as u32,
            },
        )?;
        serial::print(format_args!(
            "ngos/x86_64: inspect_contract handled id={} domain={} resource={} state={}\n",
            entry.id, entry.domain, entry.resource, entry.state as u32
        ));
        Ok(0)
    })
}

fn get_domain_name_syscall(id: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY.with(|registry| copy_name_to_user(&registry.domain(id)?.name, buffer, capacity))
}

fn get_resource_name_syscall(id: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY
        .with(|registry| copy_name_to_user(&registry.resource(id)?.name, buffer, capacity))
}

fn get_contract_label_syscall(id: usize, buffer: *mut u8, capacity: usize) -> Result<usize, Errno> {
    NATIVE_REGISTRY
        .with(|registry| copy_name_to_user(&registry.contract(id)?.label, buffer, capacity))
}

fn bind_process_contract_syscall(contract: usize) -> Result<usize, Errno> {
    BootProcessContractBindAgent::execute(contract)?;
    Ok(0)
}

fn set_contract_state_syscall(id: usize, state_raw: u32) -> Result<usize, Errno> {
    let state = NativeContractState::from_raw(state_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let contract_slot = registry.contract_slot(id)?;
        let resource_id = registry.contracts[contract_slot].as_ref().unwrap().resource as usize;
        let resource_slot = registry.resource_slot(resource_id)?;
        let contract_id = registry.contracts[contract_slot].as_ref().unwrap().id;
        registry.contracts[contract_slot].as_mut().unwrap().state = state;
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        let removed_waiter = NativeRegistry::remove_waiter(resource, contract_id);
        if state == NativeContractState::Revoked && resource.holder_contract == contract_id {
            resource.holder_contract = 0;
        }
        Ok((resource.id, contract_id, removed_waiter))
    });
    if let Ok((resource_id, contract_id, removed_waiter)) = result {
        if removed_waiter || state == NativeContractState::Revoked {
            emit_resource_event(resource_id, contract_id, BootResourceEventKind::Revoked);
        }
        serial::print(format_args!(
            "ngos/x86_64: set_contract_state handled id={} state={}\n",
            id, state_raw
        ));
        return Ok(0);
    }
    result.map(|_| 0)
}

fn invoke_contract_syscall(id: usize) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: invoke_contract enter id={}\n",
        id
    ));
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let contract_slot = registry.contract_slot(id)?;
        let resource_slot = registry
            .resource_slot(registry.contracts[contract_slot].as_ref().unwrap().resource as usize)?;
        let contract = registry.contracts[contract_slot].as_mut().unwrap();
        let resource = registry.resources[resource_slot].as_ref().unwrap();
        if contract.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
        {
            return Err(Errno::Access);
        }
        contract.invocation_count += 1;
        Ok(contract.invocation_count as usize)
    });
    match result {
        Ok(count) => {
            serial::print(format_args!(
                "ngos/x86_64: invoke_contract handled id={} count={}\n",
                id, count
            ));
            Ok(count)
        }
        Err(err) => {
            serial::print(format_args!(
                "ngos/x86_64: invoke_contract rejected id={} err={:?}\n",
                id, err
            ));
            Err(err)
        }
    }
}

fn set_resource_policy_syscall(resource: usize, policy_raw: u32) -> Result<usize, Errno> {
    let policy = NativeResourceArbitrationPolicy::from_raw(policy_raw).ok_or(Errno::Inval)?;
    NATIVE_REGISTRY.with_mut(|registry| {
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .arbitration = policy;
        Ok(0)
    })
}

fn claim_resource_syscall(
    contract: usize,
    out: *mut NativeResourceClaimRecord,
) -> Result<usize, Errno> {
    diagnostics::record_function_enter(FN_CLAIM_RESOURCE, 0, contract as u64, out as u64);
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            1,
            0,
            contract as u64,
            out as u64,
        );
        let (contract_slot, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let contract_entry = registry.contracts[contract_slot].as_ref().unwrap();
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            2,
            0,
            contract as u64,
            resource.id,
        );
        if contract_entry.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
            || !contract_kind_allowed(resource.contract_policy, contract_entry.kind)
        {
            return Err(Errno::Access);
        }
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            3,
            0,
            contract as u64,
            resource.id,
        );
        if resource.holder_contract == 0 {
            diagnostics::record_function_checkpoint(
                FN_CLAIM_RESOURCE,
                4,
                1,
                contract as u64,
                resource.id,
            );
            resource.holder_contract = contract as u64;
            resource.acquire_count += 1;
            diagnostics::record_function_checkpoint(
                FN_CLAIM_RESOURCE,
                5,
                1,
                contract as u64,
                resource.id,
            );
            write_record(
                out,
                NativeResourceClaimRecord {
                    resource: resource.id,
                    holder_contract: 0,
                    acquire_count: resource.acquire_count,
                    position: 0,
                    queued: 0,
                    reserved: 0,
                },
            )?;
            diagnostics::record_function_checkpoint(
                FN_CLAIM_RESOURCE,
                6,
                1,
                contract as u64,
                resource.id,
            );
            return Ok((
                0usize,
                Some((resource.id, contract as u64, BootResourceEventKind::Claimed)),
            ));
        }
        if resource.holder_contract == contract as u64 {
            return Err(Errno::Access);
        }
        if resource.governance == NativeResourceGovernanceMode::ExclusiveLease {
            return Err(Errno::Busy);
        }
        if resource.waiting_count >= MAX_CONTRACT_COUNT {
            return Err(Errno::Again);
        }
        if resource.waiters[..resource.waiting_count]
            .iter()
            .any(|id| *id == contract as u64)
        {
            return Err(Errno::Access);
        }
        resource.waiters[resource.waiting_count] = contract as u64;
        resource.waiting_count += 1;
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            4,
            2,
            contract as u64,
            resource.id,
        );
        write_record(
            out,
            NativeResourceClaimRecord {
                resource: resource.id,
                holder_contract: resource.holder_contract,
                acquire_count: resource.acquire_count,
                position: resource.waiting_count as u64,
                queued: 1,
                reserved: 0,
            },
        )?;
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            5,
            2,
            contract as u64,
            resource.id,
        );
        diagnostics::record_function_checkpoint(
            FN_CLAIM_RESOURCE,
            6,
            2,
            contract as u64,
            resource.id,
        );
        Ok((
            0usize,
            Some((resource.id, contract as u64, BootResourceEventKind::Queued)),
        ))
    });
    if let Ok((value, event)) = result {
        if let Some((resource, contract, kind)) = event {
            emit_resource_event(resource, contract, kind);
        }
        diagnostics::record_function_exit(FN_CLAIM_RESOURCE, 6, 0, contract as u64, 0, true, 0);
        serial::print(format_args!(
            "ngos/x86_64: claim_resource handled contract={}\n",
            contract
        ));
        return Ok(value);
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_CLAIM_RESOURCE,
            6,
            0,
            contract as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: claim_resource rejected contract={} err={:?}\n",
            contract, err
        ));
    }
    result.map(|(value, _)| value)
}

fn acquire_resource_syscall(contract: usize) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (contract_slot, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let contract_entry = registry.contracts[contract_slot].as_ref().unwrap();
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if contract_entry.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
            || !contract_kind_allowed(resource.contract_policy, contract_entry.kind)
        {
            return Err(Errno::Access);
        }
        if resource.holder_contract == 0 {
            resource.holder_contract = contract as u64;
            resource.acquire_count += 1;
            return Ok(resource.id as usize);
        }
        if resource.holder_contract == contract as u64 {
            return Ok(resource.id as usize);
        }
        Err(Errno::Busy)
    });
    if let Ok(resource) = result {
        serial::print(format_args!(
            "ngos/x86_64: acquire_resource handled contract={} resource={}\n",
            contract, resource
        ));
    } else if let Err(err) = result {
        serial::print(format_args!(
            "ngos/x86_64: acquire_resource rejected contract={} err={:?}\n",
            contract, err
        ));
    }
    result
}

fn list_resource_waiters_syscall(
    resource: usize,
    buffer: *mut u64,
    capacity: usize,
) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with(|registry| {
        let entry = registry.resource(resource)?;
        copy_ids_to_user(&entry.waiters[..entry.waiting_count], buffer, capacity)
    });
    if let Ok(count) = result {
        serial::print(format_args!(
            "ngos/x86_64: list_resource_waiters handled resource={} count={}\n",
            resource, count
        ));
    }
    result
}

fn cancel_resource_claim_syscall(
    contract: usize,
    out: *mut NativeResourceCancelRecord,
) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (_, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if !NativeRegistry::remove_waiter(resource, contract as u64) {
            return Err(Errno::Access);
        }
        write_record(
            out,
            NativeResourceCancelRecord {
                resource: resource.id,
                waiting_count: resource.waiting_count as u64,
            },
        )?;
        Ok((resource.id, contract as u64))
    });
    if let Ok((resource_id, contract_id)) = result {
        emit_resource_event(resource_id, contract_id, BootResourceEventKind::Canceled);
        serial::print(format_args!(
            "ngos/x86_64: cancel_resource_claim handled contract={}\n",
            contract
        ));
        return Ok(0);
    }
    result.map(|_| 0)
}

fn release_claimed_resource_syscall(
    contract: usize,
    out: *mut NativeResourceReleaseRecord,
) -> Result<usize, Errno> {
    diagnostics::record_function_enter(FN_RELEASE_CLAIMED_RESOURCE, 0, contract as u64, out as u64);
    serial::print(format_args!(
        "ngos/x86_64: release_claimed_resource enter contract={} out={:p}\n",
        contract, out
    ));
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_RELEASE_CLAIMED_RESOURCE,
            1,
            0,
            contract as u64,
            out as u64,
        );
        let (_, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let next_waiter = {
            let resource = registry.resources[resource_slot].as_ref().unwrap();
            diagnostics::record_function_checkpoint(
                FN_RELEASE_CLAIMED_RESOURCE,
                2,
                0,
                contract as u64,
                resource.id,
            );
            if resource.holder_contract != contract as u64 {
                return Err(Errno::Access);
            }
            if resource.governance == NativeResourceGovernanceMode::Queueing {
                registry.select_handoff_waiter(resource_slot)
            } else {
                None
            }
        };
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        diagnostics::record_function_checkpoint(
            FN_RELEASE_CLAIMED_RESOURCE,
            3,
            next_waiter.unwrap_or(0),
            contract as u64,
            resource.id,
        );
        match next_waiter {
            Some(next_contract) => {
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    4,
                    next_contract,
                    contract as u64,
                    resource.id,
                );
                resource.holder_contract = next_contract;
                resource.acquire_count += 1;
                resource.handoff_count += 1;
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    5,
                    next_contract,
                    contract as u64,
                    resource.id,
                );
                write_record(
                    out,
                    NativeResourceReleaseRecord {
                        resource: resource.id,
                        handoff_contract: next_contract,
                        acquire_count: resource.acquire_count,
                        handoff_count: resource.handoff_count,
                        handed_off: 1,
                        reserved: 0,
                    },
                )?;
                Ok((
                    0usize,
                    resource.id,
                    Some((contract as u64, BootResourceEventKind::Released)),
                    Some((next_contract, BootResourceEventKind::HandedOff)),
                ))
            }
            None => {
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    4,
                    0,
                    contract as u64,
                    resource.id,
                );
                resource.holder_contract = 0;
                diagnostics::record_function_checkpoint(
                    FN_RELEASE_CLAIMED_RESOURCE,
                    5,
                    0,
                    contract as u64,
                    resource.id,
                );
                write_record(
                    out,
                    NativeResourceReleaseRecord {
                        resource: resource.id,
                        handoff_contract: 0,
                        acquire_count: resource.acquire_count,
                        handoff_count: resource.handoff_count,
                        handed_off: 0,
                        reserved: 0,
                    },
                )?;
                Ok((
                    0usize,
                    resource.id,
                    Some((contract as u64, BootResourceEventKind::Released)),
                    None,
                ))
            }
        }
    });
    if let Ok((value, resource_id, released, handed_off)) = result {
        if let Some((event_contract, kind)) = released {
            emit_resource_event(resource_id, event_contract, kind);
        }
        if let Some((event_contract, kind)) = handed_off {
            emit_resource_event(resource_id, event_contract, kind);
        }
        diagnostics::record_function_exit(
            FN_RELEASE_CLAIMED_RESOURCE,
            6,
            0,
            contract as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: release_claimed_resource handled contract={}\n",
            contract
        ));
        return Ok(value);
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_RELEASE_CLAIMED_RESOURCE,
            6,
            0,
            contract as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: release_claimed_resource rejected contract={} err={:?}\n",
            contract, err
        ));
    }
    result.map(|(value, ..)| value)
}

fn transfer_resource_syscall(source: usize, target: usize) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (source_slot, resource_slot) = registry.contract_and_resource_slots(source)?;
        let target_slot = registry.contract_slot(target)?;
        let source_contract = registry.contracts[source_slot].as_ref().unwrap();
        let target_contract = registry.contracts[target_slot].as_ref().unwrap();
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if resource.holder_contract != source as u64
            || source_contract.resource != target_contract.resource
            || target_contract.state != NativeContractState::Active
            || resource.state != NativeResourceState::Active
            || !contract_kind_allowed(resource.contract_policy, target_contract.kind)
        {
            return Err(Errno::Access);
        }
        NativeRegistry::remove_waiter(resource, target as u64);
        resource.holder_contract = target as u64;
        resource.acquire_count += 1;
        resource.handoff_count += 1;
        Ok((resource.id as usize, source as u64, target as u64))
    });
    if let Ok((resource, _source, target)) = result {
        emit_resource_event(resource as u64, target, BootResourceEventKind::HandedOff);
        serial::print(format_args!(
            "ngos/x86_64: transfer_resource handled source={} target={} resource={}\n",
            source, target, resource
        ));
        return Ok(resource);
    }
    result.map(|(resource, ..)| resource)
}

fn release_resource_syscall(contract: usize) -> Result<usize, Errno> {
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        let (_, resource_slot) = registry.contract_and_resource_slots(contract)?;
        let resource = registry.resources[resource_slot].as_mut().unwrap();
        if resource.holder_contract != contract as u64 {
            return Err(Errno::Access);
        }
        resource.holder_contract = 0;
        Ok((resource.id as usize, contract as u64))
    });
    if let Ok((resource, contract_id)) = result {
        emit_resource_event(
            resource as u64,
            contract_id,
            BootResourceEventKind::Released,
        );
        serial::print(format_args!(
            "ngos/x86_64: release_resource handled contract={} resource={}\n",
            contract, resource
        ));
        return Ok(resource);
    }
    result.map(|(resource, _)| resource)
}

fn set_resource_governance_syscall(resource: usize, mode_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_SET_RESOURCE_GOVERNANCE,
        mode_raw as u64,
        resource as u64,
        0,
    );
    let mode = NativeResourceGovernanceMode::from_raw(mode_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_GOVERNANCE,
            1,
            mode_raw as u64,
            resource as u64,
            0,
        );
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .governance = mode;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_GOVERNANCE,
            4,
            mode_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_GOVERNANCE,
            6,
            mode_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_GOVERNANCE,
            6,
            mode_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_governance handled resource={} mode={}\n",
            resource, mode_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_GOVERNANCE,
            6,
            mode_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_governance rejected resource={} mode={} err={:?}\n",
            resource, mode_raw, err
        ));
    }
    result
}

fn set_resource_contract_policy_syscall(resource: usize, policy_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_SET_RESOURCE_CONTRACT_POLICY,
        policy_raw as u64,
        resource as u64,
        0,
    );
    let policy = NativeResourceContractPolicy::from_raw(policy_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            1,
            policy_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            3,
            policy_raw as u64,
            resource as u64,
            0,
        );
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .contract_policy = policy;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            4,
            policy_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_contract_policy handled resource={} policy={}\n",
            resource, policy_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_CONTRACT_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_contract_policy rejected resource={} policy={} err={:?}\n",
            resource, policy_raw, err
        ));
    }
    result
}

fn set_resource_issuer_policy_syscall(resource: usize, policy_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(
        FN_SET_RESOURCE_ISSUER_POLICY,
        policy_raw as u64,
        resource as u64,
        0,
    );
    let policy = NativeResourceIssuerPolicy::from_raw(policy_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_ISSUER_POLICY,
            1,
            policy_raw as u64,
            resource as u64,
            0,
        );
        registry.resources[registry.resource_slot(resource)?]
            .as_mut()
            .unwrap()
            .issuer_policy = policy;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_ISSUER_POLICY,
            4,
            policy_raw as u64,
            resource as u64,
            0,
        );
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_ISSUER_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_ISSUER_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_issuer_policy handled resource={} policy={}\n",
            resource, policy_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_ISSUER_POLICY,
            6,
            policy_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_issuer_policy rejected resource={} policy={} err={:?}\n",
            resource, policy_raw, err
        ));
    }
    result
}

fn set_resource_state_syscall(resource: usize, state_raw: u32) -> Result<usize, Errno> {
    diagnostics::record_function_enter(FN_SET_RESOURCE_STATE, state_raw as u64, resource as u64, 0);
    let state = NativeResourceState::from_raw(state_raw).ok_or(Errno::Inval)?;
    let result = NATIVE_REGISTRY.with_mut(|registry| {
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            1,
            state_raw as u64,
            resource as u64,
            0,
        );
        let resource_slot = registry.resource_slot(resource)?;
        let resource_entry = registry.resources[resource_slot].as_mut().unwrap();
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            2,
            state_raw as u64,
            resource as u64,
            0,
        );
        resource_entry.state = state;
        resource_entry.holder_contract = 0;
        resource_entry.waiters = [0; MAX_CONTRACT_COUNT];
        resource_entry.waiting_count = 0;
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            4,
            state_raw as u64,
            resource as u64,
            0,
        );
        if state == NativeResourceState::Retired {
            for contract in registry.contracts.iter_mut().flatten() {
                if contract.resource == resource as u64 {
                    contract.state = NativeContractState::Revoked;
                }
            }
        }
        diagnostics::record_function_checkpoint(
            FN_SET_RESOURCE_STATE,
            6,
            state_raw as u64,
            resource as u64,
            0,
        );
        Ok(0)
    });
    if result.is_ok() {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_STATE,
            6,
            state_raw as u64,
            resource as u64,
            0,
            true,
            0,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_state handled resource={} state={}\n",
            resource, state_raw
        ));
    } else if let Err(err) = result {
        diagnostics::record_function_exit(
            FN_SET_RESOURCE_STATE,
            6,
            state_raw as u64,
            resource as u64,
            0,
            false,
            err as u16,
        );
        serial::print(format_args!(
            "ngos/x86_64: set_resource_state rejected resource={} state={} err={:?}\n",
            resource, state_raw, err
        ));
    }
    result
}

fn encode_syscall_result(result: Result<usize, Errno>) -> usize {
    match result {
        Ok(value) => SyscallReturn::ok(value).raw() as usize,
        Err(errno) => SyscallReturn::err(errno).raw() as usize,
    }
}

fn decode_fcntl(encoded: usize) -> Option<DecodedFcntl> {
    let flag = ((encoded >> 8) & 0x1) != 0;
    let command = match encoded & 0xff {
        0 => DecodedFcntl::GetFl,
        1 => DecodedFcntl::GetFd,
        2 => DecodedFcntl::SetFl { nonblock: flag },
        3 => DecodedFcntl::SetFd { cloexec: flag },
        _ => return None,
    };
    Some(command)
}

fn encode_flags(flags: DescriptorFlags) -> usize {
    ((flags.cloexec as usize) << 1) | (flags.nonblock as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_runtime_status;
    use std::sync::{Mutex, OnceLock};

    fn user_syscall_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TestGuards {
        _state: std::sync::MutexGuard<'static, ()>,
        _io: std::sync::MutexGuard<'static, ()>,
    }

    fn lock_user_syscall_test_state() -> TestGuards {
        TestGuards {
            _state: user_syscall_test_lock()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            _io: crate::serial::lock_test_io(),
        }
    }

    fn reset_user_syscall_test_state() {
        crate::keyboard::reset_state();
        crate::serial::clear_input();
        crate::serial::clear_output();
        BOOT_PROCESSES.with_mut(|registry| *registry = BootProcessRegistry::new());
        BOOT_VFS.with_mut(|vfs| *vfs = BootVfs::new());
        BOOT_EVENT_QUEUES.with_mut(|queues| *queues = BootEventQueueRegistry::new());
        DESCRIPTORS.with_mut(|descriptors| *descriptors = DescriptorTable::new());
    }

    #[test]
    fn duplicate_close_and_poll_keep_stdio_subset_observable() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        let dup_fd = table.duplicate(1).unwrap();
        assert!(dup_fd >= 3);
        assert_eq!(table.poll(dup_fd, POLLOUT).unwrap(), POLLOUT as usize);
        table.close(dup_fd).unwrap();
        assert_eq!(table.poll(dup_fd, POLLOUT), Err(Errno::Badf));
    }

    #[test]
    fn fcntl_tracks_nonblock_and_cloexec_bits_separately() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        assert_eq!(table.fcntl(1, 0).unwrap(), 0);
        assert_eq!(table.fcntl(1, 2 | (1 << 8)).unwrap(), 1);
        assert_eq!(table.fcntl(1, 0).unwrap(), 1);
        assert_eq!(table.fcntl(1, 3 | (1 << 8)).unwrap(), 2);
        assert_eq!(table.fcntl(1, 1).unwrap(), 2);
    }

    #[test]
    fn resource_event_queues_observe_queue_handoff_revoke_and_recovery() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let domain = create_domain_syscall(0, b"eventing".as_ptr() as usize, 8).unwrap();
        let resource = create_resource_syscall(
            domain,
            NativeResourceKind::Device as u32,
            b"queue0".as_ptr() as usize,
            6,
        )
        .unwrap();
        let primary = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Display as u32,
            b"primary".as_ptr() as usize,
            7,
        )
        .unwrap();
        let mirror = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Display as u32,
            b"mirror".as_ptr() as usize,
            6,
        )
        .unwrap();

        let queue_fd = create_event_queue_syscall(NativeEventQueueMode::Epoll as u32).unwrap();
        assert_eq!(fcntl_syscall(queue_fd, 2 | (1 << 8)), Ok(1));

        let watch = NativeResourceEventWatchConfig {
            token: 515,
            poll_events: POLLPRI,
            claimed: 0,
            queued: 1,
            canceled: 0,
            released: 0,
            handed_off: 1,
            revoked: 1,
        };
        assert_eq!(
            watch_resource_events_syscall(queue_fd, resource, &watch),
            Ok(0)
        );

        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 4];
        assert_eq!(
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()),
            Err(Errno::Again)
        );

        let mut claim = NativeResourceClaimRecord {
            resource: 0,
            holder_contract: 0,
            acquire_count: 0,
            position: 0,
            queued: 0,
            reserved: 0,
        };
        assert_eq!(claim_resource_syscall(primary, &mut claim), Ok(0));
        assert_eq!(claim_resource_syscall(mirror, &mut claim), Ok(0));

        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert_eq!(count, 1);
        assert_eq!(events[0].token, 515);
        assert_eq!(events[0].events, POLLPRI);
        assert_eq!(
            events[0].source_kind,
            NativeEventSourceKind::Resource as u32
        );
        assert_eq!(events[0].source_arg0, resource as u64);
        assert_eq!(events[0].source_arg1, mirror as u64);
        assert_eq!(events[0].detail0, BootResourceEventKind::Queued as u32);

        let mut release = NativeResourceReleaseRecord {
            resource: 0,
            handoff_contract: 0,
            acquire_count: 0,
            handoff_count: 0,
            handed_off: 0,
            reserved: 0,
        };
        assert_eq!(
            release_claimed_resource_syscall(primary, &mut release),
            Ok(0)
        );
        assert_eq!(release.handoff_contract, mirror as u64);

        let count = wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()).unwrap();
        assert_eq!(count, 1);
        assert_eq!(events[0].source_arg0, resource as u64);
        assert_eq!(events[0].source_arg1, mirror as u64);
        assert_eq!(events[0].detail0, BootResourceEventKind::HandedOff as u32);

        assert_eq!(
            remove_resource_events_syscall(queue_fd, resource, 515),
            Ok(0)
        );
        assert_eq!(
            remove_resource_events_syscall(queue_fd, resource, 515),
            Err(Errno::NoEnt)
        );
        assert_eq!(
            set_contract_state_syscall(mirror, NativeContractState::Revoked as u32),
            Ok(0)
        );
        assert_eq!(
            wait_event_queue_syscall(queue_fd, events.as_mut_ptr(), events.len()),
            Err(Errno::Again)
        );

        let mut info = NativeResourceRecord {
            id: 0,
            domain: 0,
            creator: 0,
            holder_contract: 0,
            kind: 0,
            state: 0,
            arbitration: 0,
            governance: 0,
            contract_policy: 0,
            issuer_policy: 0,
            waiting_count: 0,
            acquire_count: 0,
            handoff_count: 0,
        };
        assert_eq!(inspect_resource_syscall(resource, &mut info), Ok(0));
        assert_eq!(info.holder_contract, 0);
        assert_eq!(info.waiting_count, 0);
        assert_eq!(info.acquire_count, 2);
        assert_eq!(info.handoff_count, 1);

        assert_eq!(close_syscall(queue_fd), Ok(0));
        assert_eq!(poll_syscall(queue_fd, POLLPRI), Err(Errno::Badf));
    }

    #[test]
    fn duplicate_exhaustion_maps_to_eagain() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        for _ in 0..5 {
            table.duplicate(1).unwrap();
        }
        assert_eq!(table.duplicate(1), Err(Errno::Again));
    }

    #[test]
    fn invalid_fcntl_command_maps_to_einval() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        assert_eq!(table.fcntl(1, 0xff), Err(Errno::Inval));
    }

    #[test]
    fn stdin_is_not_writable() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let descriptor = DescriptorTable::new().descriptor(0).unwrap();
        assert_eq!(descriptor.target, DescriptorTarget::Stdin);
    }

    #[test]
    fn stdin_poll_reports_readable_when_serial_input_is_pending() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        crate::serial::inject_input(b"help\n");
        let table = DescriptorTable::new();
        assert_eq!(table.poll(0, POLLIN).unwrap(), POLLIN as usize);
        crate::serial::clear_input();
    }

    #[test]
    fn read_syscall_consumes_serial_input_from_stdin() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        crate::serial::inject_input(b"echo boot\r\n");
        let mut buffer = [0u8; 32];
        let read = read_syscall(0, buffer.as_mut_ptr(), buffer.len()).unwrap();
        assert_eq!(&buffer[..read], b"echo boot\n");
        assert_eq!(DescriptorTable::new().poll(0, POLLIN).unwrap(), 0);
        crate::serial::clear_input();
    }

    #[test]
    fn readv_and_writev_syscalls_span_multiple_iovecs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        crate::serial::inject_input(b"hello\n");
        let mut a = [0u8; 2];
        let mut b = [0u8; 8];
        let read_iovecs = [
            UserIoVec {
                base: a.as_mut_ptr() as usize,
                len: a.len(),
            },
            UserIoVec {
                base: b.as_mut_ptr() as usize,
                len: b.len(),
            },
        ];
        let read = readv_syscall(0, read_iovecs.as_ptr(), read_iovecs.len()).unwrap();
        assert_eq!(read, 6);
        assert_eq!(&a, b"he");
        assert_eq!(&b[..4], b"llo\n");

        let left = b"ng";
        let right = b"os\n";
        let write_iovecs = [
            UserIoVec {
                base: left.as_ptr() as usize,
                len: left.len(),
            },
            UserIoVec {
                base: right.as_ptr() as usize,
                len: right.len(),
            },
        ];
        assert_eq!(
            writev_syscall(1, write_iovecs.as_ptr(), write_iovecs.len()).unwrap(),
            5
        );
        crate::serial::clear_input();
    }

    #[test]
    fn nonblocking_stdin_returns_eagain_without_input() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        let mut table = DescriptorTable::new();
        assert_eq!(table.fcntl(0, 2 | (1 << 8)).unwrap(), 1);
        DESCRIPTORS.with_mut(|descriptors| *descriptors = table);
        let mut buffer = [0u8; 8];
        assert_eq!(
            read_syscall(0, buffer.as_mut_ptr(), buffer.len()),
            Err(Errno::Again)
        );
        crate::serial::clear_input();
    }

    #[test]
    fn runtime_status_tracks_syscall_count_and_exit() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let frame = SyscallFrame::new(SYS_DUP, [1, 0, 0, 0, 0, 0]);
        let mut result = SyscallDispatchResult {
            raw_return: 0,
            disposition: 0,
        };
        x86_64_syscall_dispatch(&frame, 0x401000, 0x7fff_0000, 0x202, &mut result);
        let status = user_runtime_status::snapshot();
        assert_eq!(status.syscall_count, 1);
        assert_eq!(status.last_syscall, SYS_DUP);
        assert!(!status.exited);

        let exit = SyscallFrame::new(SYS_EXIT, [7, 0, 0, 0, 0, 0]);
        x86_64_syscall_dispatch(&exit, 0x401100, 0x7fff_0000, 0x202, &mut result);
        let status = user_runtime_status::snapshot();
        assert!(status.exited);
        assert_eq!(status.exit_code, 7);
    }

    #[test]
    fn write_syscall_records_stdout_runtime_telemetry() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let payload = b"boot-runtime";
        let result = write_syscall(1, payload.as_ptr(), payload.len());
        assert_eq!(result, Ok(payload.len()));
        assert_eq!(crate::serial::take_output(), payload);
        assert!(crate::serial::take_error_output().is_empty());

        let status = user_runtime_status::snapshot();
        assert_eq!(status.stdout_write_count, 1);
        assert_eq!(status.stderr_write_count, 0);
        assert_eq!(status.bytes_written, payload.len() as u64);
        assert_eq!(status.last_write_fd, 1);
        assert_eq!(status.last_write_len, payload.len() as u64);
    }

    #[test]
    fn write_syscall_records_stderr_runtime_telemetry() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let payload = b"fault-path";
        let result = write_syscall(2, payload.as_ptr(), payload.len());
        assert_eq!(result, Ok(payload.len()));
        assert_eq!(crate::serial::take_output(), payload);
        assert_eq!(crate::serial::take_error_output(), payload);

        let status = user_runtime_status::snapshot();
        assert_eq!(status.stdout_write_count, 0);
        assert_eq!(status.stderr_write_count, 1);
        assert_eq!(status.bytes_written, payload.len() as u64);
        assert_eq!(status.last_write_fd, 2);
        assert_eq!(status.last_write_len, payload.len() as u64);
    }

    #[test]
    fn boot_report_syscall_records_structured_boot_session_report() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        let result = boot_report_syscall(
            BootSessionStatus::Success as u32,
            BootSessionStage::Bootstrap as u32,
            0,
            0x1000,
        );
        assert_eq!(result, Ok(0));
        let result = boot_report_syscall(
            BootSessionStatus::Success as u32,
            BootSessionStage::NativeRuntime as u32,
            0,
            0x2000,
        );
        assert_eq!(result, Ok(0));
        let result = boot_report_syscall(
            BootSessionStatus::Success as u32,
            BootSessionStage::Complete as u32,
            0,
            0xfeed,
        );
        assert_eq!(result, Ok(0));

        let status = user_runtime_status::snapshot();
        assert!(status.boot_reported);
        assert_eq!(status.boot_report_status, BootSessionStatus::Success as u32);
        assert_eq!(status.boot_report_stage, BootSessionStage::Complete as u32);
        assert_eq!(status.boot_report_code, 0);
        assert_eq!(status.boot_report_detail, 0xfeed);
    }

    #[test]
    fn boot_report_syscall_rejects_stage_regression() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        user_runtime_status::reset();
        assert_eq!(
            boot_report_syscall(
                BootSessionStatus::Success as u32,
                BootSessionStage::NativeRuntime as u32,
                0,
                1,
            ),
            Ok(0)
        );
        assert_eq!(
            boot_report_syscall(
                BootSessionStatus::Success as u32,
                BootSessionStage::Bootstrap as u32,
                0,
                2,
            ),
            Err(Errno::Inval)
        );
    }

    #[test]
    fn boot_vm_procfs_reports_vmobjects_and_decisions_after_global_reclaim() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();
        assert_eq!(reclaim_memory_pressure_global_syscall(3), Ok(3));

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("[heap]"));
        assert!(text.contains("resident=1"));
        assert!(text.contains("dirty=0"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=pressure-trigger"));
        assert!(text.contains("agent=sync"));
        assert!(text.contains("agent=pressure-victim"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=reclaim"));
        assert!(text.contains("evicted=no"));
    }

    #[test]
    fn boot_vm_map_and_quarantine_are_observable_through_procfs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"boot-scratch";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(store_memory_word_syscall(1, mapped, 7), Ok(0));
        assert_eq!(quarantine_vm_object_syscall(1, 2, 44), Ok(0));
        assert_eq!(store_memory_word_syscall(1, mapped, 9), Err(Errno::Fault));
        assert_eq!(release_vm_object_syscall(1, 2), Ok(0));
        assert_eq!(store_memory_word_syscall(1, mapped, 9), Ok(0));

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("[anon:boot-scratch]"));
        assert!(text.contains("quarantined=0\treason=0"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=map"));
        assert!(text.contains("agent=quarantine-state"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=map"), "{text}");
        assert!(text.contains("mapped=anon"), "{text}");
        assert!(text.contains("kind=quarantine"));
        assert!(text.contains("reason=44"));
        assert!(text.contains("blocked=yes"));
        assert!(text.contains("released=yes"));
    }

    #[test]
    fn boot_vm_protect_and_unmap_are_observable_and_enforced() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"prot-range";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(
            protect_memory_range_syscall(1, mapped + 0x1000, 0x1000, 1, 0, 0),
            Ok(0)
        );
        assert_eq!(
            store_memory_word_syscall(1, mapped + 0x1000, 7),
            Err(Errno::Fault)
        );
        assert_eq!(load_memory_word_syscall(1, mapped + 0x1000), Ok(0));

        let path = b"/proc/1/maps";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("rw-p 00000000 [anon:prot-range]"));
        assert!(text.contains("r--p 00000000 [anon:prot-range]"), "{text}");

        let path = b"/proc/1/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("readable=1\twritable=0\texecutable=0"));

        assert_eq!(
            unmap_memory_range_syscall(1, mapped + 0x1000, 0x1000),
            Ok(0)
        );
        assert_eq!(
            load_memory_word_syscall(1, mapped + 0x1000),
            Err(Errno::Fault)
        );

        let path = b"/proc/1/maps";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[anon:prot-range]").count(), 2);

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=protect"));
        assert!(text.contains("agent=unmap"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=region"), "{text}");
        assert!(text.contains("protected=yes"), "{text}");
        assert!(text.contains("unmapped=yes"), "{text}");
    }

    #[test]
    fn boot_vm_protect_and_unmap_refuse_invalid_ranges() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(
            protect_memory_range_syscall(1, 0xdead_0000, 0x1000, 1, 0, 0),
            Err(Errno::Fault)
        );
        assert_eq!(
            unmap_memory_range_syscall(1, 0xdead_0000, 0x1000),
            Err(Errno::Fault)
        );
        assert_eq!(
            protect_memory_range_syscall(1, 0x4000_0000, 0, 1, 0, 0),
            Err(Errno::Inval)
        );
        assert_eq!(
            unmap_memory_range_syscall(1, 0x4000_0000, 0),
            Err(Errno::Inval)
        );
    }

    #[test]
    fn boot_vm_file_backed_mapping_is_observable_and_recoverable() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libboot.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x5au8; 0x3000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mapped = map_file_backed_memory_boot(
            1,
            lib_path.as_ptr() as usize,
            lib_path.len(),
            0x2000,
            0x1000,
            1,
            0,
            1,
            1,
        )
        .unwrap();

        assert_eq!(load_memory_word_syscall(1, mapped), Ok(0));
        assert_eq!(store_memory_word_syscall(1, mapped, 7), Err(Errno::Fault));
        assert_eq!(
            protect_memory_range_syscall(1, mapped, 0x2000, 1, 1, 0),
            Ok(0)
        );
        assert_eq!(store_memory_word_syscall(1, mapped, 9), Ok(0));
        assert_eq!(sync_memory_range_syscall(1, mapped, 0x2000), Ok(0));

        let mut buffer = [0u8; 2048];
        let path = b"/proc/1/maps";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("rw-p 00001000 /lib/libboot.so"));

        let path = b"/proc/1/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("File\tprivate=true\towners=1\toffset=00001000"));
        assert!(text.contains("/lib/libboot.so"));
        assert!(text.contains("dirty=0"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=map-file"));
        assert!(text.contains("agent=protect"));
        assert!(text.contains("agent=sync"));

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=map"), "{text}");
        assert!(text.contains("mapped=file-private"), "{text}");
    }

    #[test]
    fn boot_vm_file_backed_mapping_refuses_missing_directory_and_invalid_offset() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libboot.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x11u8; 0x2000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let missing = b"/lib/missing.so";
        assert_eq!(
            map_file_backed_memory_boot(
                1,
                missing.as_ptr() as usize,
                missing.len(),
                0x1000,
                0,
                1,
                0,
                0,
                1
            ),
            Err(Errno::NoEnt)
        );
        assert_eq!(
            map_file_backed_memory_boot(
                1,
                lib_dir.as_ptr() as usize,
                lib_dir.len(),
                0x1000,
                0,
                1,
                0,
                0,
                1
            ),
            Err(Errno::IsDir)
        );
        assert_eq!(
            map_file_backed_memory_boot(
                1,
                lib_path.as_ptr() as usize,
                lib_path.len(),
                0x1000,
                0x3000,
                1,
                0,
                0,
                1
            ),
            Err(Errno::Inval)
        );
    }

    #[test]
    fn boot_vm_faults_and_page_touch_are_observable_through_procfs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libfault.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x22u8; 0x3000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let mapped = map_file_backed_memory_boot(
            1,
            lib_path.as_ptr() as usize,
            lib_path.len(),
            0x3000,
            0,
            1,
            0,
            1,
            1,
        )
        .unwrap();
        assert_eq!(
            protect_memory_range_syscall(1, mapped, 0x3000, 1, 1, 0),
            Ok(0)
        );
        assert_eq!(store_memory_word_syscall(1, mapped, 7), Ok(0));
        assert_eq!(load_memory_word_syscall(1, mapped + 0x1000), Ok(0));
        assert_eq!(load_memory_word_syscall(1, mapped + 0x1000), Ok(0));

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 2048];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("resident=2\tdirty=1\taccessed=2"), "{text}");
        assert!(text.contains("segments=3\tresident-segments=2"), "{text}");
        assert!(text.contains("faults=2(r=1,w=1,cow=0)\t/lib/libfault.so"));

        let path = b"/proc/1/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=fault-classifier"));
        assert!(text.contains("agent=page-touch"));
        assert_eq!(text.matches("agent=fault-classifier").count(), 2);

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=fault"));
        assert!(text.contains("faulted=yes"));
        assert!(text.contains("touched=yes"));
    }

    #[test]
    fn boot_vm_copy_state_creates_cow_shadow_and_observability() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"cow-boot";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 9), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("owners=1"));
        assert!(text.contains("[anon:cow-boot] [cow]"));
        assert!(text.contains("shadow="));
        assert!(text.contains("depth=1"));
        assert!(text.contains("faults=2(r=0,w=1,cow=1)"));

        let path = b"/proc/2/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=shadow-reuse"));
        assert!(text.contains("agent=cow-populate"));

        let path = b"/proc/2/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=fault"));
        assert!(text.contains("cow=yes"));
    }

    #[test]
    fn boot_vm_copy_state_tracks_nested_shadow_depth() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"cow-depth";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();

        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 9), Ok(0));

        let grandchild_name = b"grandchild";
        let grandchild_path = b"/bin/grandchild";
        let grandchild = spawn_path_process_syscall(
            grandchild_name.as_ptr() as usize,
            grandchild_name.len(),
            grandchild_path.as_ptr() as usize,
            grandchild_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(child as u64, grandchild as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(grandchild, mapped, 11), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/3/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("depth=2"));
        assert!(text.contains("[anon:cow-depth] [cow] [cow]"));
        assert!(text.contains("faults=2(r=0,w=1,cow=1)"));

        let path = b"/proc/3/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=shadow-reuse"));
        assert!(text.contains("detail1=2"));
    }

    #[test]
    fn boot_vm_reuses_shadow_for_adjacent_partial_cow_faults() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"shadow-reuse";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 1), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 2), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[cow]").count(), 1);
        assert!(text.contains("@00000000/depth=1"));
        assert!(text.contains("committed=2\tresident=2\tdirty=2\taccessed=2"));

        let path = b"/proc/2/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.matches("agent=shadow-reuse").count() >= 2);
    }

    #[test]
    fn boot_vm_reuses_shadow_for_reverse_adjacent_partial_cow_faults() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"shadow-reuse-reverse";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 1), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped, 2), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[cow]").count(), 1);
        assert!(text.contains("@00000000/depth=1"));
        assert!(text.contains("committed=2\tresident=2\tdirty=2\taccessed=2"));
    }

    #[test]
    fn boot_vm_bridges_shadow_objects_for_middle_cow_fault() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"shadow-bridge";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped, 1), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x2000, 2), Ok(0));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 3), Ok(0));

        let mut buffer = [0u8; 4096];
        let path = b"/proc/2/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert_eq!(text.matches("[cow]").count(), 1);
        assert!(text.contains("committed=3\tresident=3\tdirty=3\taccessed=3"));

        let path = b"/proc/2/vmdecisions";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("agent=shadow-bridge"));

        let path = b"/proc/2/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=fault"));
        assert!(text.contains("cow=yes"));
        assert!(text.contains("bridged=yes"));
    }

    #[test]
    fn boot_vm_tracks_nonzero_shadow_offsets_across_generations() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"shadow-offset";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();

        let child_name = b"child";
        let child_path = b"/bin/child";
        let child = spawn_path_process_syscall(
            child_name.as_ptr() as usize,
            child_name.len(),
            child_path.as_ptr() as usize,
            child_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(1, child as u64), Ok(()));
        assert_eq!(store_memory_word_syscall(child, mapped + 0x1000, 1), Ok(0));

        let grandchild_name = b"grandchild";
        let grandchild_path = b"/bin/grandchild";
        let grandchild = spawn_path_process_syscall(
            grandchild_name.as_ptr() as usize,
            grandchild_name.len(),
            grandchild_path.as_ptr() as usize,
            grandchild_path.len(),
        )
        .unwrap();
        assert_eq!(boot_copy_vm_state(child as u64, grandchild as u64), Ok(()));
        assert_eq!(
            store_memory_word_syscall(grandchild, mapped + 0x1000, 2),
            Ok(0)
        );

        let mut buffer = [0u8; 4096];
        let path = b"/proc/3/vmobjects";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("@00001000/depth=2"));
        assert!(text.matches("[cow]").count() >= 1);
    }

    #[test]
    fn boot_vmobjects_report_real_segment_counts_for_sparse_page_state() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"segment-shape";
        let mapped =
            map_anonymous_memory_syscall(1, 0x3000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(store_memory_word_syscall(1, mapped, 1), Ok(0));
        assert_eq!(load_memory_word_syscall(1, mapped + 0x1000), Ok(0));

        let path = b"/proc/1/vmobjects";
        let mut buffer = [0u8; 4096];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("[anon:segment-shape]"));
        assert!(text.contains("segments=3\tresident-segments=2"), "{text}");
    }

    #[test]
    fn boot_vm_heap_growth_and_shrink_are_observable_through_procfs() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        assert_eq!(
            set_process_break_vm_syscall(1, 0x4000_7000),
            Ok(0x4000_7000)
        );
        assert_eq!(
            set_process_break_vm_syscall(1, 0x4000_3000),
            Ok(0x4000_3000)
        );

        let path = b"/proc/1/vmepisodes";
        let mut buffer = [0u8; 4096];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=heap"), "{text}");
        assert!(text.contains("grew=yes"), "{text}");
        assert!(text.contains("shrank=yes"), "{text}");
        assert!(text.contains("old-end=1073758208"), "{text}");
        assert!(text.contains("new-end=1073754112"), "{text}");
        assert!(text.contains("decisions=2"), "{text}");
        assert!(text.contains("last=brk"), "{text}");
    }

    #[test]
    fn boot_procfs_open_read_and_stat_work_through_descriptor_path() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let label = b"procfs-open";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(
            protect_memory_range_syscall(1, mapped + 0x1000, 0x1000, 1, 0, 0),
            Ok(0)
        );

        let path = b"/proc/1/maps";
        let fd = open_path_syscall(path.as_ptr() as usize, path.len()).unwrap();
        let mut buffer = [0u8; 2048];
        let read = read_syscall(fd, buffer.as_mut_ptr(), buffer.len()).unwrap();
        let text = core::str::from_utf8(&buffer[..read]).unwrap();
        assert!(text.contains("[anon:procfs-open]"));
        assert!(text.contains("rw-p 00000000 [anon:procfs-open]"));
        assert_eq!(
            read_syscall(fd, buffer.as_mut_ptr(), buffer.len()).unwrap(),
            0
        );

        let mut record = NativeFileStatusRecord {
            inode: 0,
            size: 0,
            kind: 0,
            cloexec: 0,
            nonblock: 0,
            readable: 0,
            writable: 0,
        };
        assert_eq!(
            stat_path_syscall(path.as_ptr() as usize, path.len(), &mut record as *mut _),
            Ok(0)
        );
        assert_eq!(record.kind, NativeObjectKind::File as u32);
        assert_eq!(record.readable, 1);
        assert_eq!(record.writable, 0);
        assert!(record.size >= read as u64);
    }

    #[test]
    fn boot_procfs_open_path_refuses_unknown_nodes() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let path = b"/proc/1/unknown";
        assert_eq!(
            open_path_syscall(path.as_ptr() as usize, path.len()),
            Err(Errno::NoEnt)
        );
    }

    #[test]
    fn boot_vfs_symlink_rename_unlink_and_readlink_work_on_real_boot_path() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let root = b"/vfs";
        let bin = b"/vfs/bin";
        let app = b"/vfs/bin/app";
        let app2 = b"/vfs/bin/app2";
        let link = b"/vfs/link";
        let invalid_subtree = b"/vfs/bin/subdir";

        assert_eq!(
            mkdir_path_syscall(root.as_ptr() as usize, root.len()),
            Ok(0)
        );
        assert_eq!(mkdir_path_syscall(bin.as_ptr() as usize, bin.len()), Ok(0));
        assert_eq!(mkfile_path_syscall(app.as_ptr() as usize, app.len()), Ok(0));
        assert_eq!(
            symlink_path_syscall(
                link.as_ptr() as usize,
                link.len(),
                app.as_ptr() as usize,
                app.len(),
            ),
            Ok(0)
        );

        let mut link_stat = NativeFileStatusRecord {
            inode: 0,
            size: 0,
            kind: 0,
            cloexec: 0,
            nonblock: 0,
            readable: 0,
            writable: 0,
        };
        assert_eq!(
            lstat_path_syscall(link.as_ptr() as usize, link.len(), &mut link_stat as *mut _),
            Ok(0)
        );
        assert_eq!(link_stat.kind, NativeObjectKind::Symlink as u32);

        let mut target = [0u8; 64];
        let copied = readlink_path_syscall(
            link.as_ptr() as usize,
            link.len(),
            target.as_mut_ptr(),
            target.len(),
        )
        .unwrap();
        assert_eq!(&target[..copied], app);

        let fd = open_path_syscall(link.as_ptr() as usize, link.len()).unwrap();
        assert!(fd >= 3);

        assert_eq!(
            rename_path_syscall(
                app.as_ptr() as usize,
                app.len(),
                app2.as_ptr() as usize,
                app2.len(),
            ),
            Ok(0)
        );
        assert_eq!(
            rename_path_syscall(
                bin.as_ptr() as usize,
                bin.len(),
                invalid_subtree.as_ptr() as usize,
                invalid_subtree.len(),
            ),
            Err(Errno::Inval)
        );
        assert_eq!(
            unlink_path_syscall(link.as_ptr() as usize, link.len()),
            Ok(0)
        );
        assert_eq!(
            readlink_path_syscall(
                link.as_ptr() as usize,
                link.len(),
                target.as_mut_ptr(),
                target.len(),
            ),
            Err(Errno::NoEnt)
        );
    }

    #[test]
    fn boot_vm_memory_contract_policy_blocks_vm_operations_and_exposes_policy_episodes() {
        let _guard = lock_user_syscall_test_state();
        reset_user_syscall_test_state();

        let domain = create_domain_syscall(0, b"vm".as_ptr() as usize, 2).unwrap();
        let resource = create_resource_syscall(
            domain,
            NativeResourceKind::Memory as u32,
            b"vm-budget".as_ptr() as usize,
            9,
        )
        .unwrap();
        assert_eq!(
            set_resource_contract_policy_syscall(
                resource,
                NativeResourceContractPolicy::Memory as u32
            ),
            Ok(0)
        );
        let contract = create_contract_syscall(
            domain,
            resource,
            NativeContractKind::Memory as u32,
            b"vm".as_ptr() as usize,
            2,
        )
        .unwrap();
        assert_eq!(bind_process_contract_syscall(contract), Ok(0));

        let lib_dir = b"/lib";
        assert_eq!(
            mkdir_path_syscall(lib_dir.as_ptr() as usize, lib_dir.len()),
            Ok(0)
        );
        let lib_path = b"/lib/libpolicy.so";
        assert_eq!(
            mkfile_path_syscall(lib_path.as_ptr() as usize, lib_path.len()),
            Ok(0)
        );
        let fd = open_path_syscall(lib_path.as_ptr() as usize, lib_path.len()).unwrap();
        let payload = [0x33u8; 0x2000];
        assert_eq!(
            write_syscall(fd, payload.as_ptr(), payload.len()),
            Ok(payload.len())
        );

        let label = b"policy-scratch";
        let mapped =
            map_anonymous_memory_syscall(1, 0x2000, label.as_ptr() as usize, label.len()).unwrap();
        assert_eq!(store_memory_word_syscall(1, mapped, 41), Ok(0));

        let vm_object_id = BOOT_PROCESSES
            .with_mut(|registry| {
                let entry = registry
                    .entries
                    .iter()
                    .find(|entry| entry.pid == 1)
                    .unwrap();
                entry
                    .vm_objects
                    .iter()
                    .find(|object| object.start == mapped as u64)
                    .map(|object| object.id as usize)
                    .ok_or(Errno::NoEnt)
            })
            .unwrap();

        assert_eq!(
            set_contract_state_syscall(contract, NativeContractState::Suspended as u32),
            Ok(0)
        );

        for result in [
            map_anonymous_memory_syscall(1, 0x1000, b"blocked-map".as_ptr() as usize, 11),
            map_file_backed_memory_boot(
                1,
                lib_path.as_ptr() as usize,
                lib_path.len(),
                0x1000,
                0,
                1,
                0,
                1,
                1,
            ),
            unmap_memory_range_syscall(1, mapped, 0x1000),
            protect_memory_range_syscall(1, mapped, 0x1000, 1, 0, 0),
            advise_memory_range_syscall(1, mapped, 0x1000, 4),
            sync_memory_range_syscall(1, mapped, 0x1000),
            quarantine_vm_object_syscall(1, vm_object_id, 7),
            release_vm_object_syscall(1, vm_object_id),
            load_memory_word_syscall(1, mapped),
            store_memory_word_syscall(1, mapped, 99),
            set_process_break_vm_syscall(1, mapped + 0x4000),
            reclaim_memory_pressure_syscall(1, 1),
            reclaim_memory_pressure_global_syscall(1),
        ] {
            assert_eq!(result, Err(Errno::Access));
        }

        let path = b"/proc/1/vmdecisions";
        let mut buffer = [0u8; 4096];
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        for operation in [
            "\tdetail1=0",
            "\tdetail1=1",
            "\tdetail1=2",
            "\tdetail1=3",
            "\tdetail1=4",
            "\tdetail1=5",
            "\tdetail1=7",
            "\tdetail1=8",
            "\tdetail1=9",
            "\tdetail1=11",
            "\tdetail1=13",
            "\tdetail1=14",
        ] {
            assert!(text.contains("agent=policy-block"), "{text}");
            assert!(text.contains(operation), "{text}");
        }

        let path = b"/proc/1/vmepisodes";
        let count = read_procfs_syscall(
            path.as_ptr() as usize,
            path.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
        )
        .unwrap();
        let text = core::str::from_utf8(&buffer[..count]).unwrap();
        assert!(text.contains("kind=policy"), "{text}");
        assert!(text.contains("state=1"), "{text}");
        assert!(text.contains("operation=14"), "{text}");
        assert!(text.contains("blocked=yes"), "{text}");

        assert_eq!(
            set_contract_state_syscall(contract, NativeContractState::Active as u32),
            Ok(0)
        );
        assert_eq!(load_memory_word_syscall(1, mapped), Ok(0));
        assert_eq!(store_memory_word_syscall(1, mapped, 41), Ok(0));
    }
}
