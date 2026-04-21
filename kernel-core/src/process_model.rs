//! Canonical subsystem role:
//! - subsystem: process and thread object model
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical process/thread state and identity model for
//!   the kernel
//!
//! Canonical contract families defined here:
//! - process model contracts
//! - thread model contracts
//! - CPU extended-state ownership contracts
//! - process inspection record contracts
//!
//! This module may define canonical process and thread truth. Higher layers may
//! inspect or transport it, but they must not redefine it.

use super::*;
use ngos_user_abi::{
    AT_ENTRY, AT_PAGESZ, AT_PHDR, AT_PHENT, AT_PHNUM, AT_PLATFORM, BootSessionReport,
    BootSessionStage, BootSessionStatus,
};

const DEFAULT_USER_STACK_BYTES: u64 = 128 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxiliaryVectorEntry {
    pub key: u64,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableImage {
    pub path: String,
    pub inode: u64,
    pub entry_point: u64,
    pub phdr_addr: u64,
    pub phent_size: u64,
    pub phnum: u64,
    pub base_addr: u64,
    pub stack_top: u64,
}

impl ExecutableImage {
    pub(crate) fn from_path_defaults(path: &str) -> Self {
        Self {
            path: path.to_string(),
            inode: 0,
            entry_point: 0,
            phdr_addr: 0,
            phent_size: 56,
            phnum: 0,
            base_addr: 0,
            stack_top: 0x7fff_ffff_0000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessMemoryRegion {
    pub start: u64,
    pub end: u64,
    pub vm_object_id: u64,
    pub share_count: u32,
    pub copy_on_write: bool,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub private: bool,
    pub file_offset: u64,
    pub advice: MemoryAdvice,
    pub dirty: bool,
    pub label: String,
}

impl ProcessMemoryRegion {
    pub(crate) fn contains_range(&self, start: u64, end: u64) -> bool {
        self.start <= start && end <= self.end
    }

    pub(crate) fn overlaps(&self, start: u64, end: u64) -> bool {
        self.start < end && start < self.end
    }

    pub(crate) fn slice(&self, start: u64, end: u64) -> Self {
        let mut sliced = self.clone();
        sliced.file_offset = self.file_offset + (start - self.start);
        sliced.start = start;
        sliced.end = end;
        sliced
    }

    pub(crate) fn can_merge_with(&self, next: &Self) -> bool {
        self.end == next.start
            && self.vm_object_id == next.vm_object_id
            && self.share_count == next.share_count
            && self.copy_on_write == next.copy_on_write
            && self.readable == next.readable
            && self.writable == next.writable
            && self.executable == next.executable
            && self.private == next.private
            && self.file_offset + (self.end - self.start) == next.file_offset
            && self.advice == next.advice
            && self.dirty == next.dirty
            && self.label == next.label
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessAbiProfile {
    pub target: String,
    pub route_class: String,
    pub handle_profile: String,
    pub path_profile: String,
    pub scheduler_profile: String,
    pub sync_profile: String,
    pub timer_profile: String,
    pub module_profile: String,
    pub event_profile: String,
    pub requires_kernel_abi_shims: bool,
    pub prefix: String,
    pub executable_path: String,
    pub working_dir: String,
    pub loader_route_class: String,
    pub loader_launch_mode: String,
    pub loader_entry_profile: String,
    pub loader_requires_compat_shims: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: ProcessId,
    pub parent: Option<ProcessId>,
    pub address_space: Option<AddressSpaceId>,
    pub main_thread: Option<ThreadId>,
    pub name: String,
    pub image_path: String,
    pub executable_image: ExecutableImage,
    pub root: String,
    pub cwd: String,
    pub state: ProcessState,
    pub exit_code: Option<i32>,
    pub pending_signals: Vec<u8>,
    pub descriptor_count: usize,
    pub capability_count: usize,
    pub environment_count: usize,
    pub auxiliary_vector_count: usize,
    pub memory_region_count: usize,
    pub thread_count: usize,
    pub vm_object_count: usize,
    pub shared_memory_region_count: usize,
    pub copy_on_write_region_count: usize,
    pub session_reported: bool,
    pub session_report_count: u64,
    pub session_status: u32,
    pub session_stage: u32,
    pub session_code: i32,
    pub session_detail: u64,
    pub abi_profile: ProcessAbiProfile,
    pub contract_bindings: ProcessContractBindings,
    pub scheduler_override: Option<SchedulerPolicyInfo>,
    pub scheduler_policy: SchedulerPolicyInfo,
    pub cpu_runtime_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadInfo {
    pub tid: ThreadId,
    pub owner: ProcessId,
    pub name: String,
    pub state: ThreadState,
    pub is_main: bool,
    pub exit_code: Option<i32>,
    pub cpu_extended_state: ThreadCpuExtendedStateProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadCpuExtendedStateProfile {
    pub owned: bool,
    pub xsave_managed: bool,
    pub save_area_bytes: u32,
    pub xcr0_mask: u64,
    pub boot_probed: bool,
    pub boot_seed_marker: u64,
    pub active_in_cpu: bool,
    pub save_count: u64,
    pub restore_count: u64,
    pub last_saved_tick: u64,
    pub last_restored_tick: u64,
    pub save_area_buffer_bytes: u32,
    pub save_area_alignment_bytes: u32,
    pub save_area_generation: u64,
    pub last_save_marker: u64,
}

#[derive(Debug)]
pub struct AlignedCpuExtendedStateBuffer {
    storage: Vec<u8>,
    offset: usize,
    len: usize,
}

impl AlignedCpuExtendedStateBuffer {
    pub const ALIGNMENT: usize = 64;

    pub const fn new() -> Self {
        Self {
            storage: Vec::new(),
            offset: 0,
            len: 0,
        }
    }

    pub fn zeroed(len: usize) -> Self {
        if len == 0 {
            return Self::new();
        }
        let storage = vec![0; len.saturating_add(Self::ALIGNMENT - 1)];
        let base = storage.as_ptr() as usize;
        let aligned = (base + (Self::ALIGNMENT - 1)) & !(Self::ALIGNMENT - 1);
        let offset = aligned.saturating_sub(base);
        Self {
            storage,
            offset,
            len,
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Self {
        let mut buffer = Self::zeroed(bytes.len());
        if !bytes.is_empty() {
            buffer.as_mut_slice().copy_from_slice(bytes);
        }
        buffer
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.storage.clear();
        self.offset = 0;
        self.len = 0;
    }

    pub fn fill(&mut self, value: u8) {
        self.as_mut_slice().fill(value);
    }

    pub fn as_slice(&self) -> &[u8] {
        let end = self.offset + self.len;
        &self.storage[self.offset..end]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        let end = self.offset + self.len;
        &mut self.storage[self.offset..end]
    }

    pub fn is_aligned(&self) -> bool {
        !self.is_empty() && (self.as_slice().as_ptr() as usize) % Self::ALIGNMENT == 0
    }
}

impl Clone for AlignedCpuExtendedStateBuffer {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice())
    }
}

impl Default for AlignedCpuExtendedStateBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for AlignedCpuExtendedStateBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for AlignedCpuExtendedStateBuffer {}

impl core::ops::Deref for AlignedCpuExtendedStateBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl core::ops::DerefMut for AlignedCpuExtendedStateBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadCpuExtendedStateImage {
    pub profile: ThreadCpuExtendedStateProfile,
    pub bytes: AlignedCpuExtendedStateBuffer,
}

impl ThreadCpuExtendedStateProfile {
    pub const fn bootstrap_default() -> Self {
        Self {
            owned: true,
            xsave_managed: false,
            save_area_bytes: 0,
            xcr0_mask: 0,
            boot_probed: false,
            boot_seed_marker: 0,
            active_in_cpu: false,
            save_count: 0,
            restore_count: 0,
            last_saved_tick: 0,
            last_restored_tick: 0,
            save_area_buffer_bytes: 0,
            save_area_alignment_bytes: 0,
            save_area_generation: 0,
            last_save_marker: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressSpaceInfo {
    pub id: AddressSpaceId,
    pub owner: ProcessId,
    pub region_count: usize,
    pub vm_object_count: usize,
    pub shared_region_count: usize,
    pub copy_on_write_region_count: usize,
    pub mapped_bytes: u64,
    pub regions: Vec<AddressSpaceRegionInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressSpaceRegionInfo {
    pub start: u64,
    pub end: u64,
    pub vm_object_id: u64,
    pub share_count: u32,
    pub copy_on_write: bool,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub private: bool,
    pub file_offset: u64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIntrospection {
    pub process: ProcessInfo,
    pub address_space: AddressSpaceInfo,
    pub threads: Vec<ThreadInfo>,
    pub filedesc_entries: Vec<FiledescEntry>,
    pub kinfo_file_entries: Vec<KinfoFileEntry>,
    pub vm_object_layouts: Vec<VmObjectLayoutInfo>,
}

pub fn project_hal_page_mappings(
    introspection: &ProcessIntrospection,
) -> Result<Vec<PageMapping>, HalError> {
    let mut mappings = Vec::new();
    for region in &introspection.address_space.regions {
        let layout = introspection
            .vm_object_layouts
            .iter()
            .find(|layout| layout.object_id == region.vm_object_id)
            .ok_or(HalError::MappingNotFound)?;
        mappings.extend(project_hal_region_mappings(region, layout)?);
    }
    Ok(mappings)
}

pub fn project_hal_address_space_layout(
    introspection: &ProcessIntrospection,
    id: HalAddressSpaceId,
    active: bool,
) -> Result<HalAddressSpaceLayout, HalError> {
    Ok(HalAddressSpaceLayout {
        id,
        active,
        mappings: project_hal_page_mappings(introspection)?,
    })
}

fn project_hal_region_mappings(
    region: &AddressSpaceRegionInfo,
    layout: &VmObjectLayoutInfo,
) -> Result<Vec<PageMapping>, HalError> {
    if region.end <= region.start {
        return Err(HalError::InvalidMapping);
    }

    let region_len = region.end - region.start;
    let region_backing_end = region
        .file_offset
        .checked_add(region_len)
        .ok_or(HalError::InvalidMapping)?;
    let perms = MemoryPermissions {
        read: region.readable,
        write: region.writable,
        execute: region.executable,
    };
    let mut mappings = Vec::new();

    for segment in &layout.segments {
        let segment_end = segment
            .byte_offset
            .checked_add(segment.byte_len)
            .ok_or(HalError::InvalidMapping)?;
        let overlap_start = region.file_offset.max(segment.byte_offset);
        let overlap_end = region_backing_end.min(segment_end);
        if overlap_start >= overlap_end {
            continue;
        }

        let region_delta = overlap_start - region.file_offset;
        let segment_delta = overlap_start - segment.byte_offset;
        mappings.push(PageMapping {
            vaddr: region.start + region_delta,
            paddr: segment.paddr + segment_delta,
            len: overlap_end - overlap_start,
            perms,
            cache: CachePolicy::WriteBack,
            user: true,
        });
    }

    if mappings.is_empty() {
        return Err(HalError::MappingNotFound);
    }

    Ok(mappings)
}

pub(crate) fn executable_name(path: &str) -> String {
    path.rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(path)
        .to_string()
}

pub(crate) fn executable_image_from_status(status: &FileStatus) -> ExecutableImage {
    let base_addr = 0x0040_0000 + status.inode.saturating_mul(0x1000);
    let phnum = match status.kind {
        ObjectKind::File | ObjectKind::Memory => 3,
        ObjectKind::Device | ObjectKind::Driver => 2,
        _ => 1,
    };
    ExecutableImage {
        path: status.path.clone(),
        inode: status.inode,
        entry_point: base_addr + 0x1000,
        phdr_addr: base_addr + 0x40,
        phent_size: 56,
        phnum,
        base_addr,
        stack_top: 0x7fff_ffff_0000 - status.inode.saturating_mul(0x1000),
    }
}

pub(crate) fn default_auxiliary_vector(path: &str, phnum: u64) -> Vec<AuxiliaryVectorEntry> {
    let path_hash = path.bytes().fold(0u64, |acc, byte| {
        acc.wrapping_mul(131).wrapping_add(u64::from(byte))
    });
    vec![
        AuxiliaryVectorEntry {
            key: AT_PHDR as u64,
            value: 0x40_0000 + (path_hash & 0xffff),
        },
        AuxiliaryVectorEntry {
            key: AT_PHENT as u64,
            value: 56,
        },
        AuxiliaryVectorEntry {
            key: AT_PHNUM as u64,
            value: phnum,
        },
        AuxiliaryVectorEntry {
            key: AT_PAGESZ as u64,
            value: 4096,
        },
        AuxiliaryVectorEntry {
            key: AT_ENTRY as u64,
            value: 0x40_1000 + (path_hash & 0xffff),
        },
        AuxiliaryVectorEntry {
            key: AT_PLATFORM as u64,
            value: 0x7fff_0000 + (path_hash & 0xffff),
        },
    ]
}

pub(crate) fn default_memory_map(image: &ExecutableImage) -> Vec<ProcessMemoryRegion> {
    let text_start = image.base_addr.max(0x0040_0000);
    let rodata_start = text_start + 0x2000;
    let data_start = rodata_start + 0x1000;
    let heap_start = data_start + 0x1000;
    let stack_start = image.stack_top.saturating_sub(DEFAULT_USER_STACK_BYTES);

    vec![
        ProcessMemoryRegion {
            start: text_start,
            end: text_start + 0x2000,
            vm_object_id: text_start,
            share_count: 1,
            copy_on_write: false,
            readable: true,
            writable: false,
            executable: true,
            private: true,
            file_offset: 0,
            advice: MemoryAdvice::Normal,
            dirty: false,
            label: compose_labeled_name(" ", &image.path, ""),
        },
        ProcessMemoryRegion {
            start: rodata_start,
            end: rodata_start + 0x1000,
            vm_object_id: rodata_start,
            share_count: 1,
            copy_on_write: false,
            readable: true,
            writable: false,
            executable: false,
            private: true,
            file_offset: 0x2000,
            advice: MemoryAdvice::Normal,
            dirty: false,
            label: compose_labeled_name(" ", &image.path, ".rodata"),
        },
        ProcessMemoryRegion {
            start: data_start,
            end: data_start + 0x1000,
            vm_object_id: data_start,
            share_count: 1,
            copy_on_write: false,
            readable: true,
            writable: true,
            executable: false,
            private: true,
            file_offset: 0x3000,
            advice: MemoryAdvice::Normal,
            dirty: true,
            label: compose_labeled_name(" ", &image.path, ".data"),
        },
        ProcessMemoryRegion {
            start: heap_start,
            end: heap_start + 0x4000,
            vm_object_id: heap_start,
            share_count: 1,
            copy_on_write: false,
            readable: true,
            writable: true,
            executable: false,
            private: true,
            file_offset: 0,
            advice: MemoryAdvice::Normal,
            dirty: true,
            label: String::from(" [heap]"),
        },
        ProcessMemoryRegion {
            start: stack_start,
            end: image.stack_top,
            vm_object_id: stack_start,
            share_count: 1,
            copy_on_write: false,
            readable: true,
            writable: true,
            executable: false,
            private: true,
            file_offset: 0,
            advice: MemoryAdvice::Normal,
            dirty: true,
            label: String::from(" [stack]"),
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessId(ObjectHandle);

impl ProcessId {
    pub const fn from_handle(handle: ObjectHandle) -> Self {
        Self(handle)
    }

    pub const fn handle(self) -> ObjectHandle {
        self.0
    }

    pub const fn raw(self) -> u64 {
        self.0.id().raw()
    }

    pub const fn generation(self) -> u32 {
        self.0.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(ObjectHandle);

impl ThreadId {
    pub const fn from_handle(handle: ObjectHandle) -> Self {
        Self(handle)
    }

    pub const fn from_process_id(pid: ProcessId) -> Self {
        Self(pid.handle())
    }

    pub const fn handle(self) -> ObjectHandle {
        self.0
    }

    pub const fn raw(self) -> u64 {
        self.0.id().raw()
    }

    pub const fn generation(self) -> u32 {
        self.0.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddressSpaceId(ObjectHandle);

impl AddressSpaceId {
    pub const fn from_handle(handle: ObjectHandle) -> Self {
        Self(handle)
    }

    pub const fn handle(self) -> ObjectHandle {
        self.0
    }

    pub const fn raw(self) -> u64 {
        self.0.id().raw()
    }

    pub const fn generation(self) -> u32 {
        self.0.generation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Created,
    Ready,
    Running,
    Blocked,
    Exited,
}

impl ProcessState {
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Ready)
                | (Self::Created, Self::Exited)
                | (Self::Ready, Self::Running)
                | (Self::Ready, Self::Blocked)
                | (Self::Ready, Self::Exited)
                | (Self::Running, Self::Ready)
                | (Self::Running, Self::Blocked)
                | (Self::Running, Self::Exited)
                | (Self::Blocked, Self::Ready)
                | (Self::Blocked, Self::Exited)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Created,
    Ready,
    Running,
    Blocked,
    Exited,
}

impl ThreadState {
    const fn from_process_state(state: ProcessState) -> Self {
        match state {
            ProcessState::Created => Self::Created,
            ProcessState::Ready => Self::Ready,
            ProcessState::Running => Self::Running,
            ProcessState::Blocked => Self::Blocked,
            ProcessState::Exited => Self::Exited,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub(crate) pid: ProcessId,
    pub(crate) parent: Option<ProcessId>,
    pub(crate) address_space: Option<AddressSpaceId>,
    pub(crate) name: String,
    pub(crate) image_path: String,
    pub(crate) executable_image: ExecutableImage,
    pub(crate) root: String,
    pub(crate) cwd: String,
    pub(crate) argv: Vec<String>,
    pub(crate) envp: Vec<String>,
    pub(crate) auxv: Vec<AuxiliaryVectorEntry>,
    pub(crate) threads: Vec<ThreadId>,
    pub(crate) state: ProcessState,
    pub(crate) exit_code: Option<i32>,
    pub(crate) pending_signals: u64,
    pub(crate) blocked_signals: u64,
    pub(crate) signal_dispositions: BTreeMap<u8, SignalDisposition>,
    pub(crate) signal_action_masks: BTreeMap<u8, u64>,
    pub(crate) signal_action_restarts: BTreeMap<u8, bool>,
    pub(crate) signal_senders: BTreeMap<u8, PendingSignalSender>,
    pub(crate) signal_values: BTreeMap<u8, u64>,
    pub(crate) thread_pending_signals: BTreeMap<u64, u64>,
    pub(crate) thread_signal_senders: BTreeMap<u64, BTreeMap<u8, PendingSignalSender>>,
    pub(crate) thread_signal_values: BTreeMap<u64, BTreeMap<u8, u64>>,
    pub(crate) session_reported: bool,
    pub(crate) session_report_count: u64,
    pub(crate) session_status: u32,
    pub(crate) session_stage: u32,
    pub(crate) session_code: i32,
    pub(crate) session_detail: u64,
    pub(crate) contract_bindings: ProcessContractBindings,
    pub(crate) scheduler_override: Option<SchedulerPolicyInfo>,
    pub(crate) cpu_runtime_ticks: u64,
}

impl Process {
    pub(crate) fn new_unbound(name: impl Into<String>, parent: Option<ProcessId>) -> Self {
        let name = name.into();
        Self {
            pid: ProcessId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            parent,
            address_space: None,
            image_path: name.clone(),
            executable_image: ExecutableImage::from_path_defaults(&name),
            root: String::from("/"),
            cwd: String::from("/"),
            argv: vec![name.clone()],
            envp: Vec::new(),
            auxv: default_auxiliary_vector(&name, 0),
            threads: Vec::new(),
            name,
            state: ProcessState::Created,
            exit_code: None,
            pending_signals: 0,
            blocked_signals: 0,
            signal_dispositions: BTreeMap::new(),
            signal_action_masks: BTreeMap::new(),
            signal_action_restarts: BTreeMap::new(),
            signal_senders: BTreeMap::new(),
            signal_values: BTreeMap::new(),
            thread_pending_signals: BTreeMap::new(),
            thread_signal_senders: BTreeMap::new(),
            thread_signal_values: BTreeMap::new(),
            session_reported: false,
            session_report_count: 0,
            session_status: BootSessionStatus::Failure as u32,
            session_stage: BootSessionStage::Bootstrap as u32,
            session_code: 0,
            session_detail: 0,
            contract_bindings: ProcessContractBindings::default(),
            scheduler_override: None,
            cpu_runtime_ticks: 0,
        }
    }

    pub(crate) fn attach_pid(&mut self, pid: ProcessId) {
        self.pid = pid;
    }

    pub const fn pid(&self) -> ProcessId {
        self.pid
    }

    pub const fn parent(&self) -> Option<ProcessId> {
        self.parent
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn image_path(&self) -> &str {
        &self.image_path
    }

    pub const fn address_space(&self) -> Option<AddressSpaceId> {
        self.address_space
    }

    pub const fn executable_image(&self) -> &ExecutableImage {
        &self.executable_image
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    pub fn envp(&self) -> &[String] {
        &self.envp
    }

    pub fn auxv(&self) -> &[AuxiliaryVectorEntry] {
        &self.auxv
    }

    pub fn threads(&self) -> &[ThreadId] {
        &self.threads
    }

    pub const fn session_reported(&self) -> bool {
        self.session_reported
    }

    pub const fn session_report_count(&self) -> u64 {
        self.session_report_count
    }

    pub const fn session_status(&self) -> u32 {
        self.session_status
    }

    pub const fn session_stage(&self) -> u32 {
        self.session_stage
    }

    pub const fn session_code(&self) -> i32 {
        self.session_code
    }

    pub const fn session_detail(&self) -> u64 {
        self.session_detail
    }

    pub fn main_thread(&self) -> Option<ThreadId> {
        self.threads.first().copied()
    }

    pub const fn contract_bindings(&self) -> ProcessContractBindings {
        self.contract_bindings
    }

    pub const fn scheduler_override(&self) -> Option<SchedulerPolicyInfo> {
        self.scheduler_override
    }

    pub fn set_scheduler_override(&mut self, override_policy: Option<SchedulerPolicyInfo>) {
        self.scheduler_override = override_policy;
    }

    pub const fn cpu_runtime_ticks(&self) -> u64 {
        self.cpu_runtime_ticks
    }

    pub fn account_runtime_tick(&mut self) {
        self.cpu_runtime_ticks = self.cpu_runtime_ticks.saturating_add(1);
    }

    pub const fn state(&self) -> ProcessState {
        self.state
    }

    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    pub fn queue_signal(
        &mut self,
        signal: u8,
        sender: PendingSignalSender,
    ) -> Result<(), ProcessError> {
        self.queue_signal_with_value(signal, sender, None)
    }

    pub fn queue_signal_with_value(
        &mut self,
        signal: u8,
        sender: PendingSignalSender,
        value: Option<u64>,
    ) -> Result<(), ProcessError> {
        if signal == 0 || signal > 64 {
            return Err(ProcessError::InvalidSignal);
        }
        self.pending_signals |= 1u64 << (signal - 1);
        self.signal_senders.insert(signal, sender);
        match value {
            Some(value) => {
                self.signal_values.insert(signal, value);
            }
            None => {
                self.signal_values.remove(&signal);
            }
        }
        Ok(())
    }

    pub fn queue_thread_signal(
        &mut self,
        tid: ThreadId,
        signal: u8,
        sender: PendingSignalSender,
    ) -> Result<(), ProcessError> {
        self.queue_thread_signal_with_value(tid, signal, sender, None)
    }

    pub fn queue_thread_signal_with_value(
        &mut self,
        tid: ThreadId,
        signal: u8,
        sender: PendingSignalSender,
        value: Option<u64>,
    ) -> Result<(), ProcessError> {
        if signal == 0 || signal > 64 {
            return Err(ProcessError::InvalidSignal);
        }
        if !self.threads.contains(&tid) {
            return Err(ProcessError::InvalidTid);
        }
        let entry = self.thread_pending_signals.entry(tid.raw()).or_insert(0);
        *entry |= 1u64 << (signal - 1);
        self.thread_signal_senders
            .entry(tid.raw())
            .or_default()
            .insert(signal, sender);
        match value {
            Some(value) => {
                self.thread_signal_values
                    .entry(tid.raw())
                    .or_default()
                    .insert(signal, value);
            }
            None => {
                if let Some(values) = self.thread_signal_values.get_mut(&tid.raw()) {
                    values.remove(&signal);
                    if values.is_empty() {
                        self.thread_signal_values.remove(&tid.raw());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn pending_signals(&self) -> Vec<u8> {
        (1..=64)
            .filter(|signal| self.pending_signals & (1u64 << (signal - 1)) != 0)
            .map(|signal| signal as u8)
            .collect()
    }

    pub fn pending_thread_signals(&self, tid: ThreadId) -> Result<Vec<u8>, ProcessError> {
        if !self.threads.contains(&tid) {
            return Err(ProcessError::InvalidTid);
        }
        let pending = self
            .thread_pending_signals
            .get(&tid.raw())
            .copied()
            .unwrap_or(0);
        Ok((1..=64)
            .filter(|signal| pending & (1u64 << (signal - 1)) != 0)
            .map(|signal| signal as u8)
            .collect())
    }

    pub fn take_thread_pending_signal_in_mask(
        &mut self,
        tid: ThreadId,
        mask: u64,
        blocked_only: bool,
    ) -> Result<Option<(u8, PendingSignalSender, Option<u64>)>, ProcessError> {
        if !self.threads.contains(&tid) {
            return Err(ProcessError::InvalidTid);
        }
        let pending = self
            .thread_pending_signals
            .get(&tid.raw())
            .copied()
            .unwrap_or(0);
        for signal in 1..=64 {
            let bit = 1u64 << (signal - 1);
            if mask & bit == 0 || pending & bit == 0 {
                continue;
            }
            if blocked_only && self.blocked_signals & bit == 0 {
                continue;
            }
            let updated = pending & !bit;
            if updated == 0 {
                self.thread_pending_signals.remove(&tid.raw());
            } else {
                self.thread_pending_signals.insert(tid.raw(), updated);
            }
            let signal = signal as u8;
            let sender = if let Some(senders) = self.thread_signal_senders.get_mut(&tid.raw()) {
                let sender = senders.remove(&signal);
                let empty = senders.is_empty();
                let sender = sender.unwrap_or(PendingSignalSender { pid: self.pid, tid });
                if empty {
                    self.thread_signal_senders.remove(&tid.raw());
                }
                sender
            } else {
                PendingSignalSender { pid: self.pid, tid }
            };
            let value = if let Some(values) = self.thread_signal_values.get_mut(&tid.raw()) {
                let value = values.remove(&signal);
                if values.is_empty() {
                    self.thread_signal_values.remove(&tid.raw());
                }
                value
            } else {
                None
            };
            return Ok(Some((signal, sender, value)));
        }
        Ok(None)
    }

    pub fn blocked_signals(&self) -> Vec<u8> {
        (1..=64)
            .filter(|signal| self.blocked_signals & (1u64 << (signal - 1)) != 0)
            .map(|signal| signal as u8)
            .collect()
    }

    pub fn pending_blocked_signals(&self) -> Vec<u8> {
        (1..=64)
            .filter(|signal| {
                let bit = 1u64 << (signal - 1);
                self.pending_signals & bit != 0 && self.blocked_signals & bit != 0
            })
            .map(|signal| signal as u8)
            .collect()
    }

    pub fn take_pending_signal_in_mask(
        &mut self,
        mask: u64,
        blocked_only: bool,
    ) -> Option<(u8, PendingSignalSender, Option<u64>)> {
        for signal in 1..=64 {
            let bit = 1u64 << (signal - 1);
            if mask & bit == 0 || self.pending_signals & bit == 0 {
                continue;
            }
            if blocked_only && self.blocked_signals & bit == 0 {
                continue;
            }
            self.pending_signals &= !bit;
            let signal = signal as u8;
            let sender = self
                .signal_senders
                .remove(&signal)
                .unwrap_or(PendingSignalSender {
                    pid: self.pid,
                    tid: self
                        .main_thread()
                        .unwrap_or(ThreadId::from_process_id(self.pid)),
                });
            let value = self.signal_values.remove(&signal);
            return Some((signal, sender, value));
        }
        None
    }

    pub fn signal_blocked(&self, signal: u8) -> Result<bool, ProcessError> {
        if signal == 0 || signal > 64 {
            return Err(ProcessError::InvalidSignal);
        }
        Ok(self.blocked_signals & (1u64 << (signal - 1)) != 0)
    }

    pub fn signal_mask_raw(&self) -> u64 {
        self.blocked_signals
    }

    pub fn set_signal_mask_raw(&mut self, mask: u64) {
        self.blocked_signals = mask;
    }

    pub fn signal_disposition(
        &self,
        signal: u8,
    ) -> Result<Option<SignalDisposition>, ProcessError> {
        if signal == 0 || signal > 64 {
            return Err(ProcessError::InvalidSignal);
        }
        Ok(self.signal_dispositions.get(&signal).copied())
    }

    pub fn signal_action_mask(&self, signal: u8) -> Result<u64, ProcessError> {
        if signal == 0 || signal > 64 {
            return Err(ProcessError::InvalidSignal);
        }
        Ok(self.signal_action_masks.get(&signal).copied().unwrap_or(0))
    }

    pub fn signal_action_restart(&self, signal: u8) -> Result<bool, ProcessError> {
        if signal == 0 || signal > 64 {
            return Err(ProcessError::InvalidSignal);
        }
        Ok(self
            .signal_action_restarts
            .get(&signal)
            .copied()
            .unwrap_or(false))
    }

    pub fn set_signal_disposition(
        &mut self,
        signal: u8,
        disposition: Option<SignalDisposition>,
        action_mask: u64,
        restart: bool,
    ) -> Result<(Option<SignalDisposition>, u64, bool), ProcessError> {
        if signal == 0 || signal > 64 {
            return Err(ProcessError::InvalidSignal);
        }
        let previous = self.signal_dispositions.get(&signal).copied();
        let previous_mask = self.signal_action_masks.get(&signal).copied().unwrap_or(0);
        let previous_restart = self
            .signal_action_restarts
            .get(&signal)
            .copied()
            .unwrap_or(false);
        if let Some(disposition) = disposition {
            self.signal_dispositions.insert(signal, disposition);
            self.signal_action_masks.insert(signal, action_mask);
            self.signal_action_restarts.insert(signal, restart);
        } else {
            self.signal_dispositions.remove(&signal);
            self.signal_action_masks.remove(&signal);
            self.signal_action_restarts.remove(&signal);
        }
        Ok((previous, previous_mask, previous_restart))
    }

    pub(crate) fn set_argv(&mut self, argv: Vec<String>) {
        if let Some(first) = argv.first() {
            self.name = first.clone();
            self.argv = argv;
        }
    }

    pub(crate) fn set_envp(&mut self, envp: Vec<String>) {
        self.envp = envp;
    }

    pub(crate) fn set_cwd(&mut self, cwd: String) {
        self.cwd = cwd;
    }

    pub(crate) fn set_root(&mut self, root: String) {
        self.root = root;
    }

    pub(crate) fn attach_address_space(&mut self, id: AddressSpaceId) {
        self.address_space = Some(id);
    }

    pub(crate) fn attach_main_thread(&mut self, tid: ThreadId) {
        if self.threads.is_empty() {
            self.threads.push(tid);
        } else {
            self.threads[0] = tid;
        }
    }

    pub(crate) fn set_exec_image(
        &mut self,
        image_path: String,
        executable_image: ExecutableImage,
        argv: Vec<String>,
        envp: Vec<String>,
        auxv: Vec<AuxiliaryVectorEntry>,
    ) {
        self.image_path = image_path.clone();
        self.executable_image = executable_image;
        self.name = executable_name(&image_path);
        self.argv = if argv.is_empty() {
            vec![image_path]
        } else {
            argv
        };
        self.envp = envp;
        self.auxv = auxv;
        self.exit_code = None;
        self.session_reported = false;
        self.session_report_count = 0;
        self.session_status = BootSessionStatus::Failure as u32;
        self.session_stage = BootSessionStage::Bootstrap as u32;
        self.session_code = 0;
        self.session_detail = 0;
        self.cpu_runtime_ticks = 0;
    }

    pub(crate) fn bind_contract(&mut self, kind: ContractKind, contract: ContractId) {
        match kind {
            ContractKind::Execution => self.contract_bindings.execution = Some(contract),
            ContractKind::Memory => self.contract_bindings.memory = Some(contract),
            ContractKind::Io => self.contract_bindings.io = Some(contract),
            ContractKind::Observe => self.contract_bindings.observe = Some(contract),
            ContractKind::Device | ContractKind::Display => {}
        }
    }

    pub(crate) fn record_session_report(
        &mut self,
        report: BootSessionReport,
    ) -> Result<(), ProcessError> {
        let _status =
            BootSessionStatus::from_raw(report.status).ok_or(ProcessError::InvalidSessionReport)?;
        let stage =
            BootSessionStage::from_raw(report.stage).ok_or(ProcessError::InvalidSessionReport)?;
        if self.session_report_count != 0 {
            let prior_stage = BootSessionStage::from_raw(self.session_stage)
                .ok_or(ProcessError::InvalidSessionReport)?;
            if prior_stage == BootSessionStage::Complete {
                return Err(ProcessError::InvalidSessionReport);
            }
            let prior_rank = match prior_stage {
                BootSessionStage::Bootstrap => 0,
                BootSessionStage::NativeRuntime => 1,
                BootSessionStage::Complete => 2,
            };
            let next_rank = match stage {
                BootSessionStage::Bootstrap => 0,
                BootSessionStage::NativeRuntime => 1,
                BootSessionStage::Complete => 2,
            };
            if next_rank < prior_rank {
                return Err(ProcessError::InvalidSessionReport);
            }
        }
        self.session_reported = true;
        self.session_report_count += 1;
        self.session_status = report.status;
        self.session_stage = report.stage;
        self.session_code = report.code;
        self.session_detail = report.detail;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressSpace {
    pub(crate) id: AddressSpaceId,
    pub(crate) owner: ProcessId,
    pub(crate) memory_map: Vec<ProcessMemoryRegion>,
}

impl AddressSpace {
    pub(crate) fn new_unbound(owner: ProcessId, image: &ExecutableImage) -> Self {
        Self {
            id: AddressSpaceId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            owner,
            memory_map: default_memory_map(image),
        }
    }

    pub(crate) fn attach_id(&mut self, id: AddressSpaceId) {
        self.id = id;
    }

    pub const fn id(&self) -> AddressSpaceId {
        self.id
    }

    pub const fn owner(&self) -> ProcessId {
        self.owner
    }

    pub fn memory_map(&self) -> &[ProcessMemoryRegion] {
        &self.memory_map
    }

    pub(crate) fn coalesce_memory_map(&mut self) {
        if self.memory_map.len() < 2 {
            return;
        }
        self.memory_map.sort_by_key(|region| region.start);
        let mut merged: Vec<ProcessMemoryRegion> = Vec::with_capacity(self.memory_map.len());
        for region in self.memory_map.drain(..) {
            if let Some(last) = merged.last_mut()
                && last.can_merge_with(&region)
            {
                last.end = region.end;
                continue;
            }
            merged.push(region);
        }
        self.memory_map = merged;
    }

    pub(crate) fn map_anonymous_memory(
        &mut self,
        vm_object_id: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: String,
    ) -> Result<u64, ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let aligned_len = align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?;
        let base = self
            .memory_map
            .iter()
            .map(|region| region.end)
            .max()
            .unwrap_or(0x1000_0000);
        let start = align_up(base.saturating_add(0x1000), 0x1000)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let end = start
            .checked_add(aligned_len)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        if self
            .memory_map
            .iter()
            .any(|region| region.overlaps(start, end))
        {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.memory_map.push(ProcessMemoryRegion {
            start,
            end,
            vm_object_id,
            share_count: 1,
            copy_on_write: false,
            readable,
            writable,
            executable,
            private: true,
            file_offset: 0,
            advice: MemoryAdvice::Normal,
            dirty: writable,
            label,
        });
        self.coalesce_memory_map();
        Ok(start)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn map_file_memory(
        &mut self,
        vm_object_id: u64,
        path: String,
        length: u64,
        file_offset: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        private: bool,
    ) -> Result<u64, ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let aligned_len = align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?;
        let base = self
            .memory_map
            .iter()
            .map(|region| region.end)
            .max()
            .unwrap_or(0x1000_0000);
        let start = align_up(base.saturating_add(0x1000), 0x1000)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let end = start
            .checked_add(aligned_len)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        if self
            .memory_map
            .iter()
            .any(|region| region.overlaps(start, end))
        {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.memory_map.push(ProcessMemoryRegion {
            start,
            end,
            vm_object_id,
            share_count: 1,
            copy_on_write: false,
            readable,
            writable,
            executable,
            private,
            file_offset,
            advice: MemoryAdvice::Normal,
            dirty: writable,
            label: format!(" {path}"),
        });
        self.coalesce_memory_map();
        Ok(start)
    }

    pub(crate) fn unmap_memory(&mut self, start: u64, length: u64) -> Result<(), ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let end = start
            .checked_add(align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let original = self.memory_map.clone();
        let mut rebuilt = Vec::with_capacity(original.len());
        let mut covered_until = start;
        let mut overlapped = false;

        for region in original {
            if !region.overlaps(start, end) {
                rebuilt.push(region);
                continue;
            }
            if region.start > covered_until {
                return Err(ProcessError::InvalidMemoryLayout);
            }
            overlapped = true;
            let overlap_start = start.max(region.start);
            let overlap_end = end.min(region.end);
            covered_until = overlap_end;
            if region.start < overlap_start {
                rebuilt.push(region.slice(region.start, overlap_start));
            }
            if overlap_end < region.end {
                rebuilt.push(region.slice(overlap_end, region.end));
            }
        }

        if !overlapped || covered_until < end {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.memory_map = rebuilt;
        self.coalesce_memory_map();
        Ok(())
    }

    pub(crate) fn range_chunks(
        &self,
        start: u64,
        length: u64,
    ) -> Result<Vec<ProcessMemoryRegion>, ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let end = start
            .checked_add(align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let mut chunks = Vec::new();
        let mut covered_until = start;

        for region in &self.memory_map {
            if !region.overlaps(start, end) {
                continue;
            }
            if region.start > covered_until {
                return Err(ProcessError::InvalidMemoryLayout);
            }
            let overlap_start = start.max(region.start);
            let overlap_end = end.min(region.end);
            covered_until = overlap_end;
            chunks.push(region.slice(overlap_start, overlap_end));
            if covered_until == end {
                break;
            }
        }

        if chunks.is_empty() || covered_until < end {
            return Err(ProcessError::InvalidMemoryLayout);
        }

        Ok(chunks)
    }

    pub(crate) fn resolve_range(
        &self,
        start: u64,
        length: u64,
    ) -> Result<ProcessMemoryRegion, ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let end = start
            .checked_add(length)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        self.memory_map
            .iter()
            .find(|region| region.contains_range(start, end))
            .cloned()
            .ok_or(ProcessError::InvalidMemoryLayout)
    }

    pub(crate) fn protect_memory(
        &mut self,
        start: u64,
        length: u64,
        readable: bool,
        writable: bool,
        executable: bool,
    ) -> Result<(), ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let end = start
            .checked_add(align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let original = self.memory_map.clone();
        let mut rebuilt = Vec::with_capacity(original.len());
        let mut covered_until = start;
        let mut overlapped = false;

        for region in original {
            if !region.overlaps(start, end) {
                rebuilt.push(region);
                continue;
            }
            if region.start > covered_until {
                return Err(ProcessError::InvalidMemoryLayout);
            }
            overlapped = true;
            let overlap_start = start.max(region.start);
            let overlap_end = end.min(region.end);
            covered_until = overlap_end;
            if region.start < overlap_start {
                rebuilt.push(region.slice(region.start, overlap_start));
            }
            let mut middle = region.slice(overlap_start, overlap_end);
            middle.readable = readable;
            middle.writable = writable;
            middle.executable = executable;
            rebuilt.push(middle);
            if overlap_end < region.end {
                rebuilt.push(region.slice(overlap_end, region.end));
            }
        }

        if !overlapped || covered_until < end {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.memory_map = rebuilt;
        self.coalesce_memory_map();
        Ok(())
    }

    pub(crate) fn advise_memory(
        &mut self,
        start: u64,
        length: u64,
        advice: MemoryAdvice,
    ) -> Result<Vec<(u64, u64, u64)>, ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let end = start
            .checked_add(align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let original = self.memory_map.clone();
        let mut rebuilt = Vec::with_capacity(original.len());
        let mut advised = Vec::new();
        let mut covered_until = start;
        let mut overlapped = false;

        for region in original {
            if !region.overlaps(start, end) {
                rebuilt.push(region);
                continue;
            }
            if region.start > covered_until {
                return Err(ProcessError::InvalidMemoryLayout);
            }
            overlapped = true;
            let overlap_start = start.max(region.start);
            let overlap_end = end.min(region.end);
            covered_until = overlap_end;
            if region.start < overlap_start {
                rebuilt.push(region.slice(region.start, overlap_start));
            }
            let mut middle = region.slice(overlap_start, overlap_end);
            middle.advice = advice;
            advised.push((
                middle.vm_object_id,
                middle.file_offset,
                overlap_end.saturating_sub(overlap_start),
            ));
            rebuilt.push(middle);
            if overlap_end < region.end {
                rebuilt.push(region.slice(overlap_end, region.end));
            }
        }

        if !overlapped || covered_until < end {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.memory_map = rebuilt;
        self.coalesce_memory_map();
        Ok(advised)
    }

    pub(crate) fn sync_memory(
        &mut self,
        start: u64,
        length: u64,
    ) -> Result<Vec<(u64, u64, u64)>, ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let end = start
            .checked_add(align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let original = self.memory_map.clone();
        let mut rebuilt = Vec::with_capacity(original.len());
        let mut synced = Vec::new();
        let mut covered_until = start;
        let mut overlapped = false;

        for region in original {
            if !region.overlaps(start, end) {
                rebuilt.push(region);
                continue;
            }
            if region.start > covered_until {
                return Err(ProcessError::InvalidMemoryLayout);
            }
            overlapped = true;
            let overlap_start = start.max(region.start);
            let overlap_end = end.min(region.end);
            covered_until = overlap_end;
            if region.start < overlap_start {
                rebuilt.push(region.slice(region.start, overlap_start));
            }
            let mut middle = region.slice(overlap_start, overlap_end);
            middle.dirty = false;
            synced.push((
                middle.vm_object_id,
                middle.file_offset,
                overlap_end.saturating_sub(overlap_start),
            ));
            rebuilt.push(middle);
            if overlap_end < region.end {
                rebuilt.push(region.slice(overlap_end, region.end));
            }
        }

        if !overlapped || covered_until < end {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        self.memory_map = rebuilt;
        self.coalesce_memory_map();
        Ok(synced)
    }

    pub(crate) fn touch_memory(
        &mut self,
        start: u64,
        length: u64,
        write: bool,
        replacement_vm_object_id: Option<u64>,
    ) -> Result<(u64, u64, u64), ProcessError> {
        if length == 0 {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        let aligned_len = align_up(length, 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?;
        let end = start
            .checked_add(aligned_len)
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let index = self
            .memory_map
            .iter()
            .position(|region| region.contains_range(start, end))
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let region = self.memory_map.remove(index);
        if write && !region.writable {
            self.memory_map.push(region);
            self.memory_map.sort_by_key(|candidate| candidate.start);
            return Err(ProcessError::InvalidMemoryLayout);
        }
        if !write && !region.readable {
            self.memory_map.push(region);
            self.memory_map.sort_by_key(|candidate| candidate.start);
            return Err(ProcessError::InvalidMemoryLayout);
        }
        if region.start < start {
            self.memory_map.push(region.slice(region.start, start));
        }
        let mut middle = region.slice(start, end);
        let cow_faulted_pages = if replacement_vm_object_id.is_some() {
            middle.vm_object_id = replacement_vm_object_id.unwrap_or(middle.vm_object_id);
            middle.share_count = 1;
            middle.copy_on_write = false;
            aligned_len / 0x1000
        } else {
            0
        };
        if write {
            middle.dirty = true;
        }
        let vm_object_id = middle.vm_object_id;
        self.memory_map.push(middle);
        if end < region.end {
            self.memory_map.push(region.slice(end, region.end));
        }
        self.coalesce_memory_map();
        Ok((vm_object_id, aligned_len / 0x1000, cow_faulted_pages))
    }

    pub(crate) fn set_brk(&mut self, new_end: u64) -> Result<u64, ProcessError> {
        let heap_index = self
            .memory_map
            .iter()
            .position(|region| region.label == " [heap]")
            .ok_or(ProcessError::InvalidMemoryLayout)?;
        let mut heap = self.memory_map[heap_index].clone();
        let min_end = heap.start + 0x1000;
        let aligned_end =
            align_up(new_end.max(min_end), 0x1000).ok_or(ProcessError::InvalidMemoryLayout)?;
        let next_start = self
            .memory_map
            .iter()
            .filter(|region| region.start > heap.start)
            .map(|region| region.start)
            .min()
            .unwrap_or(u64::MAX);
        if aligned_end >= next_start {
            return Err(ProcessError::InvalidMemoryLayout);
        }
        heap.end = aligned_end;
        self.memory_map[heap_index] = heap.clone();
        Ok(heap.end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    tid: ThreadId,
    owner: ProcessId,
    name: String,
    state: ThreadState,
    is_main: bool,
    exit_code: Option<i32>,
    cpu_extended_state: ThreadCpuExtendedStateProfile,
    cpu_extended_state_buffer: AlignedCpuExtendedStateBuffer,
}

impl Thread {
    pub(crate) fn new_main_unbound(owner: ProcessId, name: impl Into<String>) -> Self {
        Self {
            tid: ThreadId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            owner,
            name: name.into(),
            state: ThreadState::Created,
            is_main: true,
            exit_code: None,
            cpu_extended_state: ThreadCpuExtendedStateProfile::bootstrap_default(),
            cpu_extended_state_buffer: AlignedCpuExtendedStateBuffer::new(),
        }
    }

    pub(crate) fn set_cpu_extended_state(&mut self, profile: ThreadCpuExtendedStateProfile) {
        self.cpu_extended_state = profile;
        let buffer_len = if profile.xsave_managed && profile.save_area_bytes != 0 {
            profile.save_area_bytes as usize
        } else {
            0
        };
        self.cpu_extended_state_buffer = AlignedCpuExtendedStateBuffer::zeroed(buffer_len);
        self.cpu_extended_state.save_area_buffer_bytes = buffer_len as u32;
        self.cpu_extended_state.save_area_alignment_bytes = if buffer_len == 0 {
            0
        } else {
            AlignedCpuExtendedStateBuffer::ALIGNMENT as u32
        };
        self.cpu_extended_state.save_area_generation = 0;
        self.cpu_extended_state.last_save_marker = 0;
        if buffer_len != 0 && profile.boot_seed_marker != 0 {
            let seed_bytes = profile.boot_seed_marker.to_le_bytes();
            let copy_len = core::cmp::min(seed_bytes.len(), self.cpu_extended_state_buffer.len());
            self.cpu_extended_state_buffer[..copy_len].copy_from_slice(&seed_bytes[..copy_len]);
            self.cpu_extended_state.save_area_generation = 1;
            self.cpu_extended_state.last_save_marker = profile.boot_seed_marker;
        }
    }

    pub(crate) fn attach_tid(&mut self, tid: ThreadId) {
        self.tid = tid;
    }

    pub(crate) fn sync_from_process(&mut self, process: &Process) {
        self.name = process.name.clone();
        self.state = ThreadState::from_process_state(process.state);
        self.exit_code = process.exit_code;
    }

    pub const fn tid(&self) -> ThreadId {
        self.tid
    }

    pub const fn owner(&self) -> ProcessId {
        self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn state(&self) -> ThreadState {
        self.state
    }

    pub const fn is_main(&self) -> bool {
        self.is_main
    }

    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    pub const fn cpu_extended_state(&self) -> ThreadCpuExtendedStateProfile {
        self.cpu_extended_state
    }

    pub fn cpu_extended_state_image(&self) -> Result<ThreadCpuExtendedStateImage, ProcessError> {
        if !self.cpu_extended_state.xsave_managed || self.cpu_extended_state_buffer.is_empty() {
            return Err(ProcessError::CpuExtendedStateUnavailable);
        }
        Ok(ThreadCpuExtendedStateImage {
            profile: self.cpu_extended_state,
            bytes: self.cpu_extended_state_buffer.clone(),
        })
    }

    pub(crate) fn mark_cpu_extended_state_saved(&mut self, tick: u64) {
        self.cpu_extended_state.active_in_cpu = false;
        self.cpu_extended_state.save_count = self.cpu_extended_state.save_count.saturating_add(1);
        self.cpu_extended_state.last_saved_tick = tick;
        if !self.cpu_extended_state_buffer.is_empty() {
            let marker = tick ^ self.cpu_extended_state.xcr0_mask ^ u64::from(self.tid.raw());
            let marker_bytes = marker.to_le_bytes();
            let copy_len = core::cmp::min(marker_bytes.len(), self.cpu_extended_state_buffer.len());
            self.cpu_extended_state_buffer[..copy_len].copy_from_slice(&marker_bytes[..copy_len]);
            self.cpu_extended_state.save_area_generation = self
                .cpu_extended_state
                .save_area_generation
                .saturating_add(1);
            self.cpu_extended_state.last_save_marker = marker;
        }
    }

    pub(crate) fn mark_cpu_extended_state_restored(&mut self, tick: u64) {
        self.cpu_extended_state.active_in_cpu = true;
        self.cpu_extended_state.restore_count =
            self.cpu_extended_state.restore_count.saturating_add(1);
        self.cpu_extended_state.last_restored_tick = tick;
        self.cpu_extended_state.save_area_buffer_bytes =
            self.cpu_extended_state_buffer.len() as u32;
        self.cpu_extended_state.save_area_alignment_bytes =
            if self.cpu_extended_state_buffer.is_empty() {
                0
            } else {
                AlignedCpuExtendedStateBuffer::ALIGNMENT as u32
            };
    }

    pub(crate) fn restore_cpu_extended_state_boot_seed(&mut self) -> Result<(), ProcessError> {
        if !self.cpu_extended_state.xsave_managed || self.cpu_extended_state_buffer.is_empty() {
            return Err(ProcessError::CpuExtendedStateUnavailable);
        }
        if self.cpu_extended_state.boot_seed_marker == 0 {
            return Err(ProcessError::CpuExtendedStateUnavailable);
        }
        self.cpu_extended_state_buffer.fill(0);
        let seed_bytes = self.cpu_extended_state.boot_seed_marker.to_le_bytes();
        let copy_len = core::cmp::min(seed_bytes.len(), self.cpu_extended_state_buffer.len());
        self.cpu_extended_state_buffer[..copy_len].copy_from_slice(&seed_bytes[..copy_len]);
        self.cpu_extended_state.active_in_cpu = false;
        self.cpu_extended_state.save_area_generation = 1;
        self.cpu_extended_state.last_save_marker = self.cpu_extended_state.boot_seed_marker;
        self.cpu_extended_state.save_area_buffer_bytes =
            self.cpu_extended_state_buffer.len() as u32;
        Ok(())
    }

    pub(crate) fn import_cpu_extended_state_image(
        &mut self,
        image: ThreadCpuExtendedStateImage,
    ) -> Result<(), ProcessError> {
        if !image.profile.xsave_managed || image.bytes.is_empty() {
            return Err(ProcessError::CpuExtendedStateUnavailable);
        }
        if image.bytes.len() != image.profile.save_area_bytes as usize {
            return Err(ProcessError::CpuExtendedStateUnavailable);
        }
        self.cpu_extended_state = image.profile;
        self.cpu_extended_state_buffer = image.bytes;
        self.cpu_extended_state.save_area_buffer_bytes =
            self.cpu_extended_state_buffer.len() as u32;
        self.cpu_extended_state.save_area_alignment_bytes =
            if self.cpu_extended_state_buffer.is_empty() {
                0
            } else {
                AlignedCpuExtendedStateBuffer::ALIGNMENT as u32
            };
        Ok(())
    }

    pub(crate) fn release_cpu_extended_state_image(&mut self) -> Result<(), ProcessError> {
        if !self.cpu_extended_state.xsave_managed || self.cpu_extended_state_buffer.is_empty() {
            return Err(ProcessError::CpuExtendedStateUnavailable);
        }
        self.cpu_extended_state_buffer.clear();
        self.cpu_extended_state.xsave_managed = false;
        self.cpu_extended_state.save_area_bytes = 0;
        self.cpu_extended_state.xcr0_mask = 0;
        self.cpu_extended_state.boot_seed_marker = 0;
        self.cpu_extended_state.active_in_cpu = false;
        self.cpu_extended_state.save_area_buffer_bytes = 0;
        self.cpu_extended_state.save_area_alignment_bytes = 0;
        self.cpu_extended_state.save_area_generation = 0;
        self.cpu_extended_state.last_save_marker = 0;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    Exhausted,
    InvalidPid,
    StalePid,
    InvalidTid,
    StaleTid,
    InvalidMemoryLayout,
    MemoryQuarantined {
        vm_object_id: u64,
    },
    InvalidSignal,
    InvalidSessionReport,
    CpuExtendedStateUnavailable,
    InvalidTransition {
        from: ProcessState,
        to: ProcessState,
    },
    NotExited,
}

impl ProcessError {
    pub(crate) fn from_object_error(error: ObjectError) -> Self {
        match error {
            ObjectError::Exhausted => Self::Exhausted,
            ObjectError::InvalidHandle => Self::InvalidPid,
            ObjectError::StaleHandle => Self::StalePid,
        }
    }

    pub(crate) fn from_thread_object_error(error: ObjectError) -> Self {
        match error {
            ObjectError::Exhausted => Self::Exhausted,
            ObjectError::InvalidHandle => Self::InvalidTid,
            ObjectError::StaleHandle => Self::StaleTid,
        }
    }
}
