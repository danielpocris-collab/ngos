extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use ngos_user_abi::{
    Errno, NATIVE_BLOCK_IO_MAGIC, NATIVE_BLOCK_IO_OP_READ, NATIVE_BLOCK_IO_OP_WRITE,
    NATIVE_BLOCK_IO_VERSION, NATIVE_STORAGE_LINEAGE_DEPTH, NativeBlockIoRequest,
    NativeDeviceRecord, NativeDriverRecord, NativeStorageLineageEntry, NativeStorageLineageRecord,
    NativeStorageVolumeRecord, POLLIN, POLLOUT,
};
use platform_hal::{BarKind, CachePolicy, DevicePlatform, MemoryPermissions, PageMapping};
use platform_x86_64::{
    DmaWindow, PciLegacyPortBackend, VirtioBlkDriver, X86_64DevicePlatform,
    X86_64DevicePlatformConfig,
};

use crate::diagnostics::{self, DiagnosticsPath, GuardKind, TraceChannel, TraceKind, WatchKind};
use crate::paging::ActivePageTables;
use crate::phys_alloc::{BootFrameAllocator, frame_bytes};
use crate::{EarlyBootState, pic, serial};

static mut PLATFORM: MaybeUninit<X86_64DevicePlatform<PciLegacyPortBackend>> =
    MaybeUninit::uninit();
static mut DRIVER: MaybeUninit<VirtioBlkDriver> = MaybeUninit::uninit();
static mut DRIVER_ONLINE: bool = false;
static mut IRQ_LINE: u8 = 0;

const DMA_WINDOW_FRAMES: usize = 64;
pub const STORAGE_DEVICE_PATH: &str = "/dev/storage0";
pub const STORAGE_DRIVER_PATH: &str = "/drv/storage0";
const STORAGE_DEVICE_INODE: u64 = 0x5354_4f52_4147_4530;
const STORAGE_DRIVER_INODE: u64 = 0x4452_5653_544f_5230;
const STORAGE_QUEUE_CAPACITY: u64 = 128;
const STORAGE_DEVICE_CLASS: u32 = 2;
const STORAGE_DEVICE_STATE_REGISTERED: u32 = 0;
const STORAGE_DRIVER_STATE_ACTIVE: u32 = 1;
const STORAGE_VOLUME_MAGIC: u64 = 0x4e47_4f53_53544f52;
const STORAGE_VOLUME_VERSION: u32 = 1;
const STORAGE_MAX_PERSIST_PAYLOAD: usize = 512;
const STORAGE_VOLUME_ID: &str = "ngos-storage0";
const STORAGE_STATE_UNINITIALIZED: &str = "uninitialized";
const STORAGE_STATE_PREPARED: &str = "prepared";
const STORAGE_STATE_RECOVERED: &str = "recovered";
const STORAGE_SNAPSHOT_MAGIC: u64 = 0x4e47_4f53_50455232;
const STORAGE_SNAPSHOT_VERSION: u32 = 2;
const STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT: usize = 4;
const STORAGE_SNAPSHOT_MAX_NAME: usize = 32;
const STORAGE_SNAPSHOT_MAX_FILES: usize = 36;
const STORAGE_SNAPSHOT_MAX_EXTENTS: usize = 62;
const STORAGE_SNAPSHOT_MAX_BLOCKS_PER_EXTENT: u32 = 1;
const STORAGE_LINEAGE_DEPTH: usize = NATIVE_STORAGE_LINEAGE_DEPTH;
const STORAGE_PERSIST_LINEAGE_DEPTH: usize = 3;
pub const STORAGE_SNAPSHOT_ENTRY_DIRECTORY: u32 = 1;
pub const STORAGE_SNAPSHOT_ENTRY_FILE: u32 = 2;
pub const STORAGE_SNAPSHOT_ENTRY_SYMLINK: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistSuperblock {
    magic: u64,
    version: u32,
    dirty: u32,
    generation: u64,
    parent_generation: u64,
    replay_generation: u64,
    prepared_commit_count: u64,
    recovered_commit_count: u64,
    repaired_snapshot_count: u64,
    lineage_head: u32,
    lineage_count: u32,
    payload_len: u64,
    payload_checksum: u64,
    superblock_sector: u64,
    journal_sector: u64,
    data_sector: u64,
    volume_id: [u8; 32],
    state_label: [u8; 32],
    last_commit_tag: [u8; 32],
    payload_preview: [u8; 32],
    lineage_events: [PersistLineageEvent; STORAGE_PERSIST_LINEAGE_DEPTH],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistLineageEvent {
    generation: u64,
    parent_generation: u64,
    payload_checksum: u64,
    kind_label: [u8; 16],
    state_label: [u8; 16],
    tag_label: [u8; 32],
}

impl PersistLineageEvent {
    const fn empty() -> Self {
        Self {
            generation: 0,
            parent_generation: 0,
            payload_checksum: 0,
            kind_label: [0; 16],
            state_label: [0; 16],
            tag_label: [0; 32],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistSnapshotEntry {
    name: [u8; STORAGE_SNAPSHOT_MAX_NAME],
    kind: u32,
    reserved: u32,
    data_len: u64,
    first_extent: u32,
    extent_count: u32,
}

impl PersistSnapshotEntry {
    const fn empty() -> Self {
        Self {
            name: [0; STORAGE_SNAPSHOT_MAX_NAME],
            kind: 0,
            reserved: 0,
            data_len: 0,
            first_extent: 0,
            extent_count: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistSnapshotExtent {
    start_block: u32,
    block_count: u32,
}

impl PersistSnapshotExtent {
    const fn empty() -> Self {
        Self {
            start_block: 0,
            block_count: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistSnapshotIndexHeader {
    magic: u64,
    version: u32,
    entry_count: u32,
    total_blocks: u32,
    used_blocks: u32,
    mapped_extents: u32,
    reserved: u32,
    alloc_bitmap_checksum: u64,
}

impl PersistSnapshotIndexHeader {
    fn encode_sector(&self) -> [u8; 512] {
        let mut sector = [0u8; 512];
        unsafe {
            ptr::copy_nonoverlapping(
                (self as *const PersistSnapshotIndexHeader).cast::<u8>(),
                sector.as_mut_ptr(),
                core::mem::size_of::<PersistSnapshotIndexHeader>(),
            );
        }
        sector
    }

    fn decode_sector(bytes: &[u8; 512]) -> Option<Self> {
        let value =
            unsafe { (bytes.as_ptr() as *const PersistSnapshotIndexHeader).read_unaligned() };
        if value.magic != STORAGE_SNAPSHOT_MAGIC || value.version != STORAGE_SNAPSHOT_VERSION {
            return None;
        }
        Some(value)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistSnapshotExtentTable {
    magic: u64,
    version: u32,
    extent_count: u32,
    extents: [PersistSnapshotExtent; STORAGE_SNAPSHOT_MAX_EXTENTS],
}

impl PersistSnapshotExtentTable {
    fn encode_sector(&self) -> [u8; 512] {
        let mut sector = [0u8; 512];
        unsafe {
            ptr::copy_nonoverlapping(
                (self as *const PersistSnapshotExtentTable).cast::<u8>(),
                sector.as_mut_ptr(),
                core::mem::size_of::<PersistSnapshotExtentTable>(),
            );
        }
        sector
    }

    fn decode_sector(bytes: &[u8; 512]) -> Option<Self> {
        let value =
            unsafe { (bytes.as_ptr() as *const PersistSnapshotExtentTable).read_unaligned() };
        if value.magic != STORAGE_SNAPSHOT_MAGIC || value.version != STORAGE_SNAPSHOT_VERSION {
            return None;
        }
        Some(value)
    }
}

#[derive(Debug, Clone)]
pub struct StorageSnapshotEntry {
    pub name: alloc::string::String,
    pub kind: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SnapshotLayout {
    superblock_sector: u64,
    journal_sector: u64,
    data_sector: u64,
    index_sector: u64,
    entry_sector: u64,
    extent_sector: u64,
    alloc_sector: u64,
    alloc_sector_count: u64,
    data_start_sector: u64,
    data_block_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SnapshotStats {
    total_blocks: u64,
    used_blocks: u64,
    file_count: u64,
    extent_count: u64,
    directory_count: u64,
    symlink_count: u64,
}

impl PersistSuperblock {
    fn encode_sector(&self) -> [u8; 512] {
        let mut sector = [0u8; 512];
        unsafe {
            ptr::copy_nonoverlapping(
                (self as *const PersistSuperblock).cast::<u8>(),
                sector.as_mut_ptr(),
                core::mem::size_of::<PersistSuperblock>(),
            );
        }
        sector
    }

    fn decode_sector(bytes: &[u8; 512]) -> Option<Self> {
        let value = unsafe { (bytes.as_ptr() as *const PersistSuperblock).read_unaligned() };
        if value.magic != STORAGE_VOLUME_MAGIC || value.version != STORAGE_VOLUME_VERSION {
            return None;
        }
        Some(value)
    }
}

fn push_lineage_event(superblock: &mut PersistSuperblock, kind: &str, tag: &str) {
    let index = (superblock.lineage_head as usize) % STORAGE_PERSIST_LINEAGE_DEPTH;
    let mut event = PersistLineageEvent::empty();
    event.generation = superblock.generation;
    event.parent_generation = superblock.parent_generation;
    event.payload_checksum = superblock.payload_checksum;
    fill_fixed(&mut event.kind_label, kind);
    let state_label = fixed_text(&superblock.state_label);
    fill_fixed(&mut event.state_label, &state_label);
    fill_fixed(&mut event.tag_label, tag);
    superblock.lineage_events[index] = event;
    superblock.lineage_head = ((index + 1) % STORAGE_PERSIST_LINEAGE_DEPTH) as u32;
    superblock.lineage_count =
        (superblock.lineage_count as usize + 1).min(STORAGE_PERSIST_LINEAGE_DEPTH) as u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageEndpointKind {
    Device,
    Driver,
}

#[derive(Default)]
struct StorageRuntimeState {
    driver_queue: VecDeque<Vec<u8>>,
    completion_queue: VecDeque<Vec<u8>>,
    submitted_requests: u64,
    completed_requests: u64,
    in_flight_requests: u64,
    last_request_id: u64,
    last_completion_id: u64,
    last_irq_id: u64,
}

struct StorageRuntimeCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<StorageRuntimeState>>,
}

unsafe impl Sync for StorageRuntimeCell {}

impl StorageRuntimeCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn initialize(&self) {
        self.with_mut(|state| {
            if state.is_none() {
                *state = Some(StorageRuntimeState::default());
            }
        });
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Option<StorageRuntimeState>) -> R) -> R {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let result = unsafe { f(&mut *self.state.get()) };
        self.locked.store(false, Ordering::Release);
        result
    }
}

static STORAGE_RUNTIME: StorageRuntimeCell = StorageRuntimeCell::new();

pub fn bring_up(
    state: &EarlyBootState<'static>,
    paging: &ActivePageTables,
    allocator: &mut BootFrameAllocator,
) -> Result<bool, &'static str> {
    let dma_run = allocator
        .allocate_frames(DMA_WINDOW_FRAMES)
        .map_err(|_| "virtio-blk dma window allocation failed")?;
    let mut direct_map_size = state.layout.direct_map_size;
    let config = X86_64DevicePlatformConfig {
        direct_map_base: state.boot_info.physical_memory_offset,
        direct_map_size,
        dma_window: DmaWindow {
            physical_start: dma_run.start,
            len: frame_bytes(DMA_WINDOW_FRAMES),
        },
        interrupt_vector_base: pic::IRQ_BASE_PRIMARY,
        interrupt_vector_count: 16,
    };
    let mut platform = X86_64DevicePlatform::with_legacy_pci_ports(config);
    let devices = platform
        .enumerate_devices()
        .map_err(|_| "virtio-blk pci enumeration failed")?;
    let Some(record) = devices
        .iter()
        .find(|record| matches!(VirtioBlkDriver::probe(&mut platform, record), Ok(Some(_))))
        .cloned()
    else {
        serial::print(format_args!(
            "ngos/x86_64: virtio-blk: no matching pci device\n"
        ));
        return Ok(false);
    };

    serial::print(format_args!(
        "ngos/x86_64: virtio-blk candidate vendor={:#06x} device={:#06x} subsystem={:#06x}:{:#06x} class={:#04x}:{:#04x} pi={:#04x} bars={} irqs={}\n",
        record.identity.vendor_id,
        record.identity.device_id,
        record.identity.subsystem_vendor_id,
        record.identity.subsystem_device_id,
        record.identity.base_class,
        record.identity.sub_class,
        record.identity.programming_interface,
        record.bars.len(),
        record.interrupts.len(),
    ));
    for (index, bar) in record.bars.iter().enumerate() {
        serial::print(format_args!(
            "ngos/x86_64: virtio-blk bar[{}] kind={:?} base={:#x} size={:#x}\n",
            index, bar.kind, bar.base, bar.size
        ));
    }

    for bar in &record.bars {
        if !matches!(bar.kind, BarKind::Memory32 | BarKind::Memory64) {
            continue;
        }
        let bar_end = bar.base.saturating_add(bar.size);
        if bar_end <= direct_map_size {
            continue;
        }
        let alias = state
            .boot_info
            .physical_memory_offset
            .saturating_add(bar.base);
        let len = (bar.size + 0xfff) & !0xfff;
        paging
            .map_existing_physical(
                allocator,
                PageMapping {
                    vaddr: alias,
                    paddr: bar.base,
                    len,
                    perms: MemoryPermissions::read_write(),
                    cache: CachePolicy::Uncacheable,
                    user: false,
                },
            )
            .map_err(|_| "virtio-blk mmio alias map failed")?;
        direct_map_size = direct_map_size.max(bar_end);
        serial::print(format_args!(
            "ngos/x86_64: virtio-blk mmio alias base={:#x} len={:#x} virt={:#x}\n",
            bar.base, len, alias
        ));
    }
    paging.flush_tlb();

    let mut platform = X86_64DevicePlatform::with_legacy_pci_ports(X86_64DevicePlatformConfig {
        direct_map_base: state.boot_info.physical_memory_offset,
        direct_map_size,
        dma_window: config.dma_window,
        interrupt_vector_base: config.interrupt_vector_base,
        interrupt_vector_count: config.interrupt_vector_count,
    });
    let devices = platform
        .enumerate_devices()
        .map_err(|_| "virtio-blk pci re-enumeration failed")?;
    let record = devices
        .into_iter()
        .find(|candidate| candidate.locator == record.locator)
        .ok_or("virtio-blk record disappeared after mmio alias setup")?;

    let driver = VirtioBlkDriver::initialize(&mut platform, &record).map_err(|error| {
        serial::print(format_args!(
            "ngos/x86_64: virtio-blk initialize error: {:?}\n",
            error
        ));
        "virtio-blk initialize failed"
    })?;
    let route = driver.interrupt_route();
    let Some(line) = route.line else {
        return Err("virtio-blk legacy interrupt line missing");
    };
    if !crate::irq_registry::register_irq_handler(line, handle_irq) {
        return Err("virtio-blk irq registry registration failed");
    }
    pic::unmask_irq_line(line);

    let capacity_sectors = driver.capacity_sectors();
    let block_size = driver.block_size();
    let read_only = driver.is_read_only();
    unsafe {
        ptr::addr_of_mut!(PLATFORM)
            .cast::<X86_64DevicePlatform<PciLegacyPortBackend>>()
            .write(platform);
        ptr::addr_of_mut!(DRIVER)
            .cast::<VirtioBlkDriver>()
            .write(driver);
        DRIVER_ONLINE = true;
        IRQ_LINE = line;
    }
    STORAGE_RUNTIME.initialize();
    serial::print(format_args!(
        "ngos/x86_64: virtio-blk online sectors={} block_size={} read_only={} irq_line={} irq_vector={}\n",
        capacity_sectors, block_size, read_only, line, route.vector
    ));

    let sector0 = unsafe {
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioBlkDriver>();
        driver
            .read_sector(platform, 0)
            .map_err(|_| "virtio-blk read sector0 failed")?
    };
    serial::print(format_args!(
        "ngos/x86_64: virtio-blk sector0 preview={:?}\n",
        &sector0[..32]
    ));
    Ok(true)
}

pub fn endpoint_for_path(path: &str) -> Option<StorageEndpointKind> {
    match path {
        STORAGE_DEVICE_PATH => Some(StorageEndpointKind::Device),
        STORAGE_DRIVER_PATH => Some(StorageEndpointKind::Driver),
        _ => None,
    }
}

pub fn device_record(path: &str) -> Option<NativeDeviceRecord> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return None;
    }
    STORAGE_RUNTIME.with_mut(|runtime| {
        let runtime = runtime.as_mut()?;
        let (block_size, capacity_bytes) = device_geometry()?;
        Some(NativeDeviceRecord {
            class: STORAGE_DEVICE_CLASS,
            state: STORAGE_DEVICE_STATE_REGISTERED,
            reserved0: 0,
            queue_depth: runtime.driver_queue.len() as u64,
            queue_capacity: STORAGE_QUEUE_CAPACITY,
            submitted_requests: runtime.submitted_requests,
            completed_requests: runtime.completed_requests,
            total_latency_ticks: 0,
            max_latency_ticks: 0,
            total_queue_wait_ticks: 0,
            max_queue_wait_ticks: 0,
            link_up: 1,
            reserved1: 0,
            block_size,
            reserved2: 0,
            capacity_bytes,
            last_completed_request_id: runtime.last_completion_id,
            last_completed_frame_tag: [0; 64],
            last_completed_source_api_name: [0; 24],
            last_completed_translation_label: [0; 32],
            last_terminal_request_id: runtime.last_completion_id,
            last_terminal_state: 2,
            reserved3: 0,
            last_terminal_frame_tag: [0; 64],
            last_terminal_source_api_name: [0; 24],
            last_terminal_translation_label: [0; 32],
        })
    })
}

pub fn driver_record(path: &str) -> Option<NativeDriverRecord> {
    if path != STORAGE_DRIVER_PATH || !is_online() {
        return None;
    }
    STORAGE_RUNTIME.with_mut(|runtime| {
        let runtime = runtime.as_mut()?;
        Some(NativeDriverRecord {
            state: STORAGE_DRIVER_STATE_ACTIVE,
            reserved: 0,
            bound_device_count: 1,
            queued_requests: runtime.driver_queue.len() as u64,
            in_flight_requests: runtime.in_flight_requests,
            completed_requests: runtime.completed_requests,
            last_completed_request_id: runtime.last_completion_id,
            last_completed_frame_tag: [0; 64],
            last_completed_source_api_name: [0; 24],
            last_completed_translation_label: [0; 32],
            last_terminal_request_id: runtime.last_completion_id,
            last_terminal_state: 2,
            reserved1: 0,
            last_terminal_frame_tag: [0; 64],
            last_terminal_source_api_name: [0; 24],
            last_terminal_translation_label: [0; 32],
        })
    })
}

pub fn poll(endpoint: StorageEndpointKind, interest: u32) -> usize {
    serial::print(format_args!(
        "ngos/x86_64: virtio-blk poll enter endpoint={:?} interest={:#x}\n",
        endpoint, interest
    ));
    if !is_online() {
        return 0;
    }
    let ready = STORAGE_RUNTIME.with_mut(|runtime| {
        let Some(runtime) = runtime.as_mut() else {
            return 0;
        };
        match endpoint {
            StorageEndpointKind::Device => {
                let mut ready = 0;
                if !runtime.completion_queue.is_empty() {
                    ready |= POLLIN;
                }
                ready |= POLLOUT;
                (ready & interest) as usize
            }
            StorageEndpointKind::Driver => {
                let mut ready = 0;
                if !runtime.driver_queue.is_empty() {
                    ready |= POLLIN;
                }
                (ready & interest) as usize
            }
        }
    });
    serial::print(format_args!(
        "ngos/x86_64: virtio-blk poll handled endpoint={:?} ready={:#x}\n",
        endpoint, ready
    ));
    ready
}

pub fn read(
    endpoint: StorageEndpointKind,
    buffer: *mut u8,
    len: usize,
    nonblock: bool,
) -> Result<usize, Errno> {
    serial::print(format_args!(
        "ngos/x86_64: virtio-blk read enter endpoint={:?} buffer={:#x} len={} nonblock={}\n",
        endpoint, buffer as usize, len, nonblock
    ));
    if len == 0 {
        return Ok(0);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    loop {
        let maybe_payload = STORAGE_RUNTIME.with_mut(|runtime| {
            let runtime = runtime.as_mut()?;
            match endpoint {
                StorageEndpointKind::Device => runtime.completion_queue.pop_front(),
                StorageEndpointKind::Driver => {
                    let payload = runtime.driver_queue.pop_front();
                    if payload.is_some() {
                        runtime.in_flight_requests = runtime.in_flight_requests.saturating_add(1);
                    }
                    payload
                }
            }
        });
        if let Some(payload) = maybe_payload {
            serial::print(format_args!(
                "ngos/x86_64: virtio-blk read payload endpoint={:?} payload_len={}\n",
                endpoint,
                payload.len()
            ));
            let copy_len = payload.len().min(len);
            let completion_id = diagnostics::next_completion_id();
            let _ = diagnostics::guard_register(
                GuardKind::CompletionBuffer,
                DiagnosticsPath::Completion,
                payload.as_ptr() as u64,
                payload.len() as u64,
                32,
                diagnostics::replay_ids().request_id,
                completion_id,
            );
            let _ = diagnostics::watch_register(
                WatchKind::Read,
                DiagnosticsPath::Completion,
                payload.as_ptr() as u64,
                payload.len() as u64,
                diagnostics::replay_ids().request_id,
                completion_id,
            );
            unsafe {
                ptr::copy_nonoverlapping(payload.as_ptr(), buffer, copy_len);
            }
            diagnostics::watch_touch(WatchKind::Read, payload.as_ptr() as u64, copy_len as u64);
            diagnostics::trace_emit(
                TraceKind::Transition,
                TraceChannel::Transition,
                crate::diagnostics::BootTraceStage::DeviceBringup as u16,
                completion_id,
                copy_len as u64,
                endpoint as u64,
                0,
            );
            serial::print(format_args!(
                "ngos/x86_64: virtio-blk read return endpoint={:?} copied={}\n",
                endpoint, copy_len
            ));
            return Ok(copy_len);
        }
        if nonblock {
            serial::print(format_args!(
                "ngos/x86_64: virtio-blk read return endpoint={:?} err=Again\n",
                endpoint
            ));
            return Err(Errno::Again);
        }
        core::hint::spin_loop();
    }
}

pub fn write(endpoint: StorageEndpointKind, bytes: &[u8]) -> Result<usize, Errno> {
    match endpoint {
        StorageEndpointKind::Device => submit_device_request(bytes),
        StorageEndpointKind::Driver => publish_driver_completion(bytes),
    }
}

pub fn inode_for_path(path: &str) -> Option<u64> {
    match path {
        STORAGE_DEVICE_PATH => Some(STORAGE_DEVICE_INODE),
        STORAGE_DRIVER_PATH => Some(STORAGE_DRIVER_INODE),
        _ => None,
    }
}

fn submit_device_request(bytes: &[u8]) -> Result<usize, Errno> {
    if !is_online() {
        return Err(Errno::Nxio);
    }
    serial::print(format_args!(
        "ngos/x86_64: virtio-blk submit enter bytes={} header_size={}\n",
        bytes.len(),
        core::mem::size_of::<NativeBlockIoRequest>()
    ));
    let request = decode_block_request(bytes)?;
    let request_id = diagnostics::next_request_id();
    diagnostics::set_active_window(
        1,
        0,
        request.op as u64,
        STORAGE_DEVICE_INODE,
        0,
        DiagnosticsPath::Block,
        request_id,
        0,
    );
    let _ = diagnostics::guard_register(
        GuardKind::RequestBuffer,
        DiagnosticsPath::Block,
        bytes.as_ptr() as u64,
        bytes.len() as u64,
        32,
        request_id,
        0,
    );
    let _ = diagnostics::watch_register(
        WatchKind::Touch,
        DiagnosticsPath::Block,
        bytes.as_ptr() as u64,
        bytes.len() as u64,
        request_id,
        0,
    );
    diagnostics::watch_touch(WatchKind::Read, bytes.as_ptr() as u64, bytes.len() as u64);
    let expected_payload = (request.sector_count as usize)
        .checked_mul(request.block_size as usize)
        .ok_or(Errno::TooBig)?;
    let payload = &bytes[core::mem::size_of::<NativeBlockIoRequest>()..];
    if request.op == NATIVE_BLOCK_IO_OP_WRITE && payload.len() != expected_payload {
        return Err(Errno::Inval);
    }
    if request.op == NATIVE_BLOCK_IO_OP_READ && !payload.is_empty() {
        return Err(Errno::Inval);
    }
    let completion_id = diagnostics::next_completion_id();
    let driver_view = build_driver_view(&request);
    let _ = diagnostics::guard_register(
        GuardKind::QueueMetadata,
        DiagnosticsPath::Block,
        driver_view.as_ptr() as u64,
        driver_view.len() as u64,
        16,
        request_id,
        completion_id,
    );
    unsafe {
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioBlkDriver>();
        if request.block_size != driver.block_size() || request.sector_count == 0 {
            return Err(Errno::Inval);
        }
        for sector_offset in 0..request.sector_count as usize {
            let sector_index = request
                .sector
                .checked_add(sector_offset as u64)
                .ok_or(Errno::Range)?;
            if sector_index >= driver.capacity_sectors() {
                return Err(Errno::Range);
            }
            if request.op == NATIVE_BLOCK_IO_OP_READ {
                let _sector = driver
                    .read_sector(platform, sector_index)
                    .map_err(|_| Errno::Io)?;
            } else if request.op == NATIVE_BLOCK_IO_OP_WRITE {
                let offset = sector_offset * request.block_size as usize;
                let sector_bytes: [u8; 512] = payload[offset..offset + 512]
                    .try_into()
                    .map_err(|_| Errno::Inval)?;
                driver
                    .write_sector(platform, sector_index, &sector_bytes)
                    .map_err(|_| Errno::Io)?;
            } else {
                return Err(Errno::Inval);
            }
        }
    }

    STORAGE_RUNTIME.with_mut(|runtime| {
        let Some(runtime) = runtime.as_mut() else {
            return;
        };
        runtime.submitted_requests = runtime.submitted_requests.saturating_add(1);
        runtime.last_request_id = request_id;
        runtime.driver_queue.push_back(driver_view);
    });
    diagnostics::trace_emit(
        TraceKind::Device,
        TraceChannel::Device,
        crate::diagnostics::BootTraceStage::DeviceBringup as u16,
        request_id,
        completion_id,
        request.op as u64,
        expected_payload as u64,
    );
    Ok(bytes.len())
}

fn publish_driver_completion(bytes: &[u8]) -> Result<usize, Errno> {
    if !is_online() {
        return Err(Errno::Nxio);
    }
    STORAGE_RUNTIME.with_mut(|runtime| {
        let Some(runtime) = runtime.as_mut() else {
            return Err(Errno::Nxio);
        };
        if runtime.in_flight_requests == 0 {
            return Err(Errno::Again);
        }
        runtime.in_flight_requests -= 1;
        let completion_id = diagnostics::next_completion_id();
        runtime.completed_requests = runtime.completed_requests.saturating_add(1);
        runtime.last_completion_id = completion_id;
        runtime.completion_queue.push_back(bytes.to_vec());
        diagnostics::trace_emit(
            TraceKind::Transition,
            TraceChannel::Transition,
            crate::diagnostics::BootTraceStage::DeviceBringup as u16,
            runtime.last_request_id,
            completion_id,
            StorageEndpointKind::Driver as u64,
            bytes.len() as u64,
        );
        Ok(bytes.len())
    })
}

fn decode_block_request(bytes: &[u8]) -> Result<NativeBlockIoRequest, Errno> {
    if bytes.len() < core::mem::size_of::<NativeBlockIoRequest>() {
        serial::print(format_args!(
            "ngos/x86_64: virtio-blk decode short bytes={}\n",
            bytes.len()
        ));
        return Err(Errno::Inval);
    }
    let request = unsafe { (bytes.as_ptr() as *const NativeBlockIoRequest).read_unaligned() };
    if request.magic != NATIVE_BLOCK_IO_MAGIC
        || request.version != NATIVE_BLOCK_IO_VERSION
        || request.block_size != 512
    {
        serial::print(format_args!(
            "ngos/x86_64: virtio-blk decode invalid magic={:#x} version={} block_size={}\n",
            request.magic, request.version, request.block_size
        ));
        return Err(Errno::Inval);
    }
    serial::print(format_args!(
        "ngos/x86_64: virtio-blk decode ok op={} sector={} sectors={}\n",
        request.op, request.sector, request.sector_count
    ));
    Ok(request)
}

fn build_driver_view(request: &NativeBlockIoRequest) -> Vec<u8> {
    let mut bytes = Vec::new();
    let op = match request.op {
        NATIVE_BLOCK_IO_OP_READ => "Read",
        NATIVE_BLOCK_IO_OP_WRITE => "Write",
        _ => "Unknown",
    };
    let request_index = STORAGE_RUNTIME.with_mut(|runtime| {
        runtime
            .as_ref()
            .map(|state| state.submitted_requests.saturating_add(1))
            .unwrap_or(1)
    });
    let header = alloc::format!(
        "request:{} kind={} device={} opcode={}\n",
        request_index,
        op,
        STORAGE_DEVICE_PATH,
        request.op
    );
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(unsafe {
        core::slice::from_raw_parts(
            (request as *const NativeBlockIoRequest).cast::<u8>(),
            core::mem::size_of::<NativeBlockIoRequest>(),
        )
    });
    bytes
}

fn device_geometry() -> Option<(u32, u64)> {
    unsafe {
        if !DRIVER_ONLINE {
            return None;
        }
        let driver = &*ptr::addr_of!(DRIVER).cast::<VirtioBlkDriver>();
        Some((
            driver.block_size(),
            driver
                .capacity_sectors()
                .saturating_mul(driver.block_size() as u64),
        ))
    }
}

fn is_online() -> bool {
    unsafe { DRIVER_ONLINE }
}

fn checksum64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0u64, |acc, byte| {
        acc.wrapping_mul(131).wrapping_add(u64::from(*byte))
    })
}

fn checksum64_extend(acc: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(acc, |value, byte| {
        value.wrapping_mul(131).wrapping_add(u64::from(*byte))
    })
}

fn fill_fixed<const N: usize>(dst: &mut [u8; N], value: &str) {
    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(N.saturating_sub(1));
    if copy_len != 0 {
        dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
    }
    if copy_len < N {
        dst[copy_len] = 0;
    }
}

fn sector_layout() -> Option<(u64, u64, u64)> {
    unsafe {
        if !DRIVER_ONLINE {
            return None;
        }
        let driver = &*ptr::addr_of!(DRIVER).cast::<VirtioBlkDriver>();
        let capacity = driver.capacity_sectors();
        if capacity < 4 {
            return None;
        }
        Some((capacity - 1, capacity - 2, capacity - 3))
    }
}

fn snapshot_layout() -> Option<SnapshotLayout> {
    unsafe {
        if !DRIVER_ONLINE {
            return None;
        }
        let driver = &*ptr::addr_of!(DRIVER).cast::<VirtioBlkDriver>();
        let capacity = driver.capacity_sectors();
        let fixed_reserved = 4u64 + STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT as u64 + 1u64;
        if capacity <= fixed_reserved {
            return None;
        }
        let mut alloc_sector_count = 1u64;
        loop {
            let data_block_count = capacity.checked_sub(fixed_reserved + alloc_sector_count)?;
            let required_alloc = data_block_count.div_ceil(4096);
            if required_alloc == alloc_sector_count {
                let alloc_sector = capacity - fixed_reserved - alloc_sector_count;
                return Some(SnapshotLayout {
                    superblock_sector: capacity - 1,
                    journal_sector: capacity - 2,
                    data_sector: capacity - 3,
                    index_sector: capacity - 4,
                    entry_sector: capacity - 4 - STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT as u64,
                    extent_sector: capacity - 5 - STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT as u64,
                    alloc_sector,
                    alloc_sector_count,
                    data_start_sector: 0,
                    data_block_count,
                });
            }
            alloc_sector_count = required_alloc;
            if capacity <= fixed_reserved + alloc_sector_count {
                return None;
            }
        }
    }
}

fn fixed_text(bytes: &[u8]) -> alloc::string::String {
    let len = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    alloc::string::String::from_utf8_lossy(&bytes[..len]).into_owned()
}

fn load_snapshot_header(
    layout: SnapshotLayout,
) -> Result<Option<PersistSnapshotIndexHeader>, Errno> {
    let sector = read_storage_sector(layout.index_sector)?;
    Ok(PersistSnapshotIndexHeader::decode_sector(&sector))
}

fn load_snapshot_extent_table(
    layout: SnapshotLayout,
) -> Result<Option<PersistSnapshotExtentTable>, Errno> {
    let sector = read_storage_sector(layout.extent_sector)?;
    Ok(PersistSnapshotExtentTable::decode_sector(&sector))
}

fn encode_snapshot_entry_sectors(
    entries: &[PersistSnapshotEntry],
) -> Result<[[u8; 512]; STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT], Errno> {
    if entries.len() > STORAGE_SNAPSHOT_MAX_FILES {
        return Err(Errno::TooBig);
    }
    let mut sectors = [[0u8; 512]; STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT];
    for (index, entry) in entries.iter().enumerate() {
        let sector_index = index / 9;
        let slot_index = index % 9;
        let offset = slot_index * core::mem::size_of::<PersistSnapshotEntry>();
        unsafe {
            ptr::copy_nonoverlapping(
                (entry as *const PersistSnapshotEntry).cast::<u8>(),
                sectors[sector_index][offset..].as_mut_ptr(),
                core::mem::size_of::<PersistSnapshotEntry>(),
            );
        }
    }
    Ok(sectors)
}

fn decode_snapshot_entry_sectors(
    sectors: &[[u8; 512]; STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT],
    entry_count: usize,
) -> Result<Vec<PersistSnapshotEntry>, Errno> {
    if entry_count > STORAGE_SNAPSHOT_MAX_FILES {
        return Err(Errno::TooBig);
    }
    let mut entries = Vec::with_capacity(entry_count);
    for index in 0..entry_count {
        let sector_index = index / 9;
        let slot_index = index % 9;
        let offset = slot_index * core::mem::size_of::<PersistSnapshotEntry>();
        let entry = unsafe {
            (sectors[sector_index][offset..].as_ptr() as *const PersistSnapshotEntry)
                .read_unaligned()
        };
        entries.push(entry);
    }
    Ok(entries)
}

fn load_snapshot_entries(
    layout: SnapshotLayout,
    entry_count: usize,
) -> Result<Vec<PersistSnapshotEntry>, Errno> {
    let mut sectors = [[0u8; 512]; STORAGE_SNAPSHOT_ENTRY_SECTOR_COUNT];
    for (sector_index, sector) in sectors.iter_mut().enumerate() {
        *sector = read_storage_sector(layout.entry_sector + sector_index as u64)?;
    }
    decode_snapshot_entry_sectors(&sectors, entry_count)
}

fn write_snapshot_entries(
    layout: SnapshotLayout,
    entries: &[PersistSnapshotEntry],
) -> Result<(), Errno> {
    let sectors = encode_snapshot_entry_sectors(entries)?;
    for (sector_index, sector) in sectors.iter().enumerate() {
        write_storage_sector(layout.entry_sector + sector_index as u64, sector)?;
    }
    Ok(())
}

fn render_alloc_bitmap_sector(
    start_block: u64,
    extents: &[PersistSnapshotExtent],
) -> Result<[u8; 512], Errno> {
    let mut bytes = [0u8; 512];
    let end_block = start_block.saturating_add(4096);
    for extent in extents {
        if extent.block_count == 0 {
            return Err(Errno::Inval);
        }
        let extent_start = extent.start_block as u64;
        let extent_end = extent_start
            .checked_add(extent.block_count as u64)
            .ok_or(Errno::Range)?;
        if extent_end <= start_block || extent_start >= end_block {
            continue;
        }
        let overlap_start = extent_start.max(start_block);
        let overlap_end = extent_end.min(end_block);
        for block in overlap_start..overlap_end {
            let relative = (block - start_block) as usize;
            bytes[relative / 8] |= 1u8 << (relative % 8);
        }
    }
    Ok(bytes)
}

fn alloc_bitmap_checksum_from_disk(layout: SnapshotLayout) -> Result<u64, Errno> {
    let mut checksum = 0u64;
    for sector in 0..layout.alloc_sector_count {
        let bytes = read_storage_sector(layout.alloc_sector + sector)?;
        checksum = checksum64_extend(checksum, &bytes);
    }
    Ok(checksum)
}

fn write_alloc_bitmap_for_extents(
    layout: SnapshotLayout,
    extents: &[PersistSnapshotExtent],
) -> Result<u64, Errno> {
    let mut checksum = 0u64;
    for sector in 0..layout.alloc_sector_count {
        let start_block = sector * 4096;
        let bytes = render_alloc_bitmap_sector(start_block, extents)?;
        checksum = checksum64_extend(checksum, &bytes);
        write_storage_sector(layout.alloc_sector + sector, &bytes)?;
    }
    Ok(checksum)
}

fn snapshot_stats(layout: SnapshotLayout) -> Result<SnapshotStats, Errno> {
    let Some(header) = load_snapshot_header(layout)? else {
        return Ok(SnapshotStats {
            total_blocks: layout.data_block_count,
            used_blocks: 0,
            file_count: 0,
            extent_count: 0,
            directory_count: 0,
            symlink_count: 0,
        });
    };
    let entries = load_snapshot_entries(layout, header.entry_count as usize)?;
    let mut file_count = 0u64;
    let mut directory_count = 0u64;
    let mut symlink_count = 0u64;
    for raw in entries.iter() {
        match raw.kind {
            STORAGE_SNAPSHOT_ENTRY_DIRECTORY => directory_count += 1,
            STORAGE_SNAPSHOT_ENTRY_FILE => file_count += 1,
            STORAGE_SNAPSHOT_ENTRY_SYMLINK => symlink_count += 1,
            _ => return Err(Errno::Inval),
        }
    }
    Ok(SnapshotStats {
        total_blocks: header.total_blocks as u64,
        used_blocks: header.used_blocks as u64,
        file_count,
        extent_count: header.mapped_extents as u64,
        directory_count,
        symlink_count,
    })
}

pub fn read_storage_snapshot(path: &str) -> Result<Vec<StorageSnapshotEntry>, Errno> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return Err(Errno::Nxio);
    }
    let layout = snapshot_layout().ok_or(Errno::Nxio)?;
    let Some(header) = load_snapshot_header(layout)? else {
        return Ok(Vec::new());
    };
    if header.entry_count as usize > STORAGE_SNAPSHOT_MAX_FILES
        || header.total_blocks as u64 != layout.data_block_count
    {
        return Err(Errno::Inval);
    }
    let Some(extent_table) = load_snapshot_extent_table(layout)? else {
        return Err(Errno::Inval);
    };
    if extent_table.extent_count as usize > STORAGE_SNAPSHOT_MAX_EXTENTS {
        return Err(Errno::Inval);
    }
    if alloc_bitmap_checksum_from_disk(layout)? != header.alloc_bitmap_checksum {
        return Err(Errno::Inval);
    }
    let raw_entries = load_snapshot_entries(layout, header.entry_count as usize)?;
    let mut entries = Vec::new();
    for raw in raw_entries.iter() {
        let name = fixed_text(&raw.name);
        if name.is_empty() {
            return Err(Errno::Inval);
        }
        let mut bytes = Vec::new();
        match raw.kind {
            STORAGE_SNAPSHOT_ENTRY_DIRECTORY => {
                if raw.data_len != 0 || raw.extent_count != 0 {
                    return Err(Errno::Inval);
                }
            }
            STORAGE_SNAPSHOT_ENTRY_FILE | STORAGE_SNAPSHOT_ENTRY_SYMLINK => {
                if raw.extent_count == 0 {
                    return Err(Errno::Inval);
                }
                let extent_end = raw
                    .first_extent
                    .checked_add(raw.extent_count)
                    .ok_or(Errno::Range)?;
                if extent_end as usize > extent_table.extent_count as usize {
                    return Err(Errno::Inval);
                }
                bytes = Vec::with_capacity(raw.data_len as usize);
                let mut capacity = 0usize;
                for extent in
                    extent_table.extents[raw.first_extent as usize..extent_end as usize].iter()
                {
                    if extent.block_count == 0 {
                        return Err(Errno::Inval);
                    }
                    let block_end = extent
                        .start_block
                        .checked_add(extent.block_count)
                        .ok_or(Errno::Range)?;
                    if block_end as u64 > layout.data_block_count {
                        return Err(Errno::Range);
                    }
                    capacity = capacity.saturating_add(extent.block_count as usize * 512);
                    for block in 0..extent.block_count as u64 {
                        let sector = read_storage_sector(
                            layout.data_start_sector + extent.start_block as u64 + block,
                        )?;
                        let remaining = raw.data_len as usize - bytes.len();
                        let copy_len = remaining.min(512);
                        bytes.extend_from_slice(&sector[..copy_len]);
                        if bytes.len() == raw.data_len as usize {
                            break;
                        }
                    }
                    if bytes.len() == raw.data_len as usize {
                        break;
                    }
                }
                if raw.data_len as usize > capacity {
                    return Err(Errno::Inval);
                }
                if raw.kind == STORAGE_SNAPSHOT_ENTRY_SYMLINK && bytes.is_empty() {
                    return Err(Errno::Inval);
                }
            }
            _ => return Err(Errno::Inval),
        }
        entries.push(StorageSnapshotEntry {
            name,
            kind: raw.kind,
            bytes,
        });
    }
    Ok(entries)
}

pub fn write_storage_snapshot(
    path: &str,
    tag: &str,
    entries: &[StorageSnapshotEntry],
) -> Result<usize, Errno> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return Err(Errno::Nxio);
    }
    if tag.is_empty() {
        return Err(Errno::Inval);
    }
    let layout = snapshot_layout().ok_or(Errno::Nxio)?;
    if entries.len() > STORAGE_SNAPSHOT_MAX_FILES {
        return Err(Errno::TooBig);
    }
    let previous = load_superblock()?.unwrap_or(PersistSuperblock {
        magic: STORAGE_VOLUME_MAGIC,
        version: STORAGE_VOLUME_VERSION,
        dirty: 0,
        generation: 0,
        parent_generation: 0,
        replay_generation: 0,
        prepared_commit_count: 0,
        recovered_commit_count: 0,
        repaired_snapshot_count: 0,
        lineage_head: 0,
        lineage_count: 0,
        payload_len: 0,
        payload_checksum: 0,
        superblock_sector: layout.superblock_sector,
        journal_sector: layout.journal_sector,
        data_sector: layout.data_sector,
        volume_id: [0; 32],
        state_label: [0; 32],
        last_commit_tag: [0; 32],
        payload_preview: [0; 32],
        lineage_events: [PersistLineageEvent::empty(); STORAGE_PERSIST_LINEAGE_DEPTH],
    });

    let mut header = PersistSnapshotIndexHeader {
        magic: STORAGE_SNAPSHOT_MAGIC,
        version: STORAGE_SNAPSHOT_VERSION,
        entry_count: entries.len() as u32,
        total_blocks: layout.data_block_count as u32,
        used_blocks: 0,
        mapped_extents: 0,
        reserved: 0,
        alloc_bitmap_checksum: 0,
    };
    let mut extent_table = PersistSnapshotExtentTable {
        magic: STORAGE_SNAPSHOT_MAGIC,
        version: STORAGE_SNAPSHOT_VERSION,
        extent_count: 0,
        extents: [PersistSnapshotExtent::empty(); STORAGE_SNAPSHOT_MAX_EXTENTS],
    };
    let mut snapshot_entries = Vec::with_capacity(entries.len());
    let mut next_block = 0u32;
    let mut total_bytes = 0usize;
    let mut preview_name = alloc::string::String::new();
    let mut extent_count = 0u32;

    for entry in entries.iter() {
        if entry.name.is_empty() || entry.name.len() >= STORAGE_SNAPSHOT_MAX_NAME {
            return Err(Errno::Inval);
        }
        let mut name = [0u8; STORAGE_SNAPSHOT_MAX_NAME];
        name[..entry.name.len()].copy_from_slice(entry.name.as_bytes());
        let (first_extent, entry_extent_count) = match entry.kind {
            STORAGE_SNAPSHOT_ENTRY_DIRECTORY => {
                if !entry.bytes.is_empty() {
                    return Err(Errno::Inval);
                }
                (0, 0)
            }
            STORAGE_SNAPSHOT_ENTRY_FILE | STORAGE_SNAPSHOT_ENTRY_SYMLINK => {
                if entry.kind == STORAGE_SNAPSHOT_ENTRY_SYMLINK && entry.bytes.is_empty() {
                    return Err(Errno::Inval);
                }
                let block_count_total = entry.bytes.len().div_ceil(512).max(1) as u32;
                let chunk_count =
                    block_count_total.div_ceil(STORAGE_SNAPSHOT_MAX_BLOCKS_PER_EXTENT);
                if extent_table.extent_count as usize + chunk_count as usize
                    > STORAGE_SNAPSHOT_MAX_EXTENTS
                {
                    return Err(Errno::TooBig);
                }
                if next_block as u64 + block_count_total as u64 > layout.data_block_count {
                    return Err(Errno::TooBig);
                }
                let first_extent = extent_table.extent_count;
                let mut remaining_blocks = block_count_total;
                let mut payload_offset = 0usize;
                while remaining_blocks != 0 {
                    let block_count = remaining_blocks.min(STORAGE_SNAPSHOT_MAX_BLOCKS_PER_EXTENT);
                    extent_table.extents[extent_table.extent_count as usize] =
                        PersistSnapshotExtent {
                            start_block: next_block,
                            block_count,
                        };
                    extent_table.extent_count += 1;
                    extent_count = extent_count.saturating_add(1);
                    for block in 0..block_count {
                        let mut sector = [0u8; 512];
                        let end = (payload_offset + 512).min(entry.bytes.len());
                        if end > payload_offset {
                            sector[..end - payload_offset]
                                .copy_from_slice(&entry.bytes[payload_offset..end]);
                            payload_offset = end;
                        }
                        write_storage_sector(
                            layout.data_start_sector + next_block as u64 + block as u64,
                            &sector,
                        )?;
                    }
                    next_block += block_count;
                    remaining_blocks -= block_count;
                }
                (first_extent, chunk_count)
            }
            _ => return Err(Errno::Inval),
        };
        snapshot_entries.push(PersistSnapshotEntry {
            name,
            kind: entry.kind,
            reserved: 0,
            data_len: entry.bytes.len() as u64,
            first_extent,
            extent_count: entry_extent_count,
        });
        total_bytes = total_bytes.saturating_add(entry.bytes.len());
        if preview_name.is_empty() {
            preview_name = entry.name.clone();
        }
    }

    header.used_blocks = next_block;
    header.mapped_extents = extent_count;
    header.alloc_bitmap_checksum = write_alloc_bitmap_for_extents(
        layout,
        &extent_table.extents[..extent_table.extent_count as usize],
    )?;
    write_snapshot_entries(layout, &snapshot_entries)?;
    write_storage_sector(layout.extent_sector, &extent_table.encode_sector())?;
    write_storage_sector(layout.index_sector, &header.encode_sector())?;

    let mut superblock = previous;
    superblock.magic = STORAGE_VOLUME_MAGIC;
    superblock.version = STORAGE_VOLUME_VERSION;
    superblock.dirty = 0;
    superblock.parent_generation = previous.generation;
    superblock.generation = previous.generation.saturating_add(1);
    superblock.replay_generation = superblock.generation;
    superblock.prepared_commit_count = previous.prepared_commit_count.saturating_add(1);
    superblock.payload_len = total_bytes as u64;
    superblock.payload_checksum = checksum64(
        &entries
            .iter()
            .flat_map(|entry| entry.bytes.iter().copied())
            .collect::<Vec<_>>(),
    );
    superblock.superblock_sector = layout.superblock_sector;
    superblock.journal_sector = layout.journal_sector;
    superblock.data_sector = layout.data_sector;
    superblock.payload_preview = [0; 32];
    fill_fixed(&mut superblock.payload_preview, &preview_name);
    fill_fixed(&mut superblock.volume_id, STORAGE_VOLUME_ID);
    fill_fixed(&mut superblock.state_label, STORAGE_STATE_RECOVERED);
    fill_fixed(&mut superblock.last_commit_tag, tag);
    push_lineage_event(&mut superblock, "snapshot", tag);
    write_storage_sector(layout.superblock_sector, &superblock.encode_sector())?;
    Ok(superblock.generation as usize)
}

pub fn repair_storage_snapshot(path: &str) -> Result<usize, Errno> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return Err(Errno::Nxio);
    }
    let layout = snapshot_layout().ok_or(Errno::Nxio)?;
    let mut header = load_snapshot_header(layout)?.unwrap_or(PersistSnapshotIndexHeader {
        magic: STORAGE_SNAPSHOT_MAGIC,
        version: STORAGE_SNAPSHOT_VERSION,
        entry_count: 0,
        total_blocks: layout.data_block_count as u32,
        used_blocks: 0,
        mapped_extents: 0,
        reserved: 0,
        alloc_bitmap_checksum: 0,
    });
    if header.entry_count as usize > STORAGE_SNAPSHOT_MAX_FILES
        || header.total_blocks as u64 != layout.data_block_count
    {
        return Err(Errno::Inval);
    }
    let entries = load_snapshot_entries(layout, header.entry_count as usize)?;
    let extent_table = load_snapshot_extent_table(layout)?.unwrap_or(PersistSnapshotExtentTable {
        magic: STORAGE_SNAPSHOT_MAGIC,
        version: STORAGE_SNAPSHOT_VERSION,
        extent_count: 0,
        extents: [PersistSnapshotExtent::empty(); STORAGE_SNAPSHOT_MAX_EXTENTS],
    });
    if extent_table.extent_count as usize > STORAGE_SNAPSHOT_MAX_EXTENTS {
        return Err(Errno::Inval);
    }
    let mut used_blocks = 0u32;
    let mut extent_count = 0u32;
    for raw in entries.iter() {
        let name = fixed_text(&raw.name);
        if name.is_empty() {
            return Err(Errno::Inval);
        }
        match raw.kind {
            STORAGE_SNAPSHOT_ENTRY_DIRECTORY => {
                if raw.data_len != 0 || raw.extent_count != 0 {
                    return Err(Errno::Inval);
                }
            }
            STORAGE_SNAPSHOT_ENTRY_FILE | STORAGE_SNAPSHOT_ENTRY_SYMLINK => {
                if raw.extent_count == 0 {
                    return Err(Errno::Inval);
                }
                let extent_end = raw
                    .first_extent
                    .checked_add(raw.extent_count)
                    .ok_or(Errno::Range)?;
                if extent_end as usize > extent_table.extent_count as usize {
                    return Err(Errno::Inval);
                }
                let mut capacity = 0usize;
                for extent in
                    extent_table.extents[raw.first_extent as usize..extent_end as usize].iter()
                {
                    if extent.block_count == 0 {
                        return Err(Errno::Inval);
                    }
                    let block_end = extent
                        .start_block
                        .checked_add(extent.block_count)
                        .ok_or(Errno::Range)?;
                    if block_end as u64 > layout.data_block_count {
                        return Err(Errno::Range);
                    }
                    used_blocks = used_blocks.saturating_add(extent.block_count);
                    capacity = capacity.saturating_add(extent.block_count as usize * 512);
                    extent_count = extent_count.saturating_add(1);
                }
                if raw.data_len as usize > capacity {
                    return Err(Errno::Inval);
                }
            }
            _ => return Err(Errno::Inval),
        }
    }
    header.alloc_bitmap_checksum = write_alloc_bitmap_for_extents(
        layout,
        &extent_table.extents[..extent_table.extent_count as usize],
    )?;
    write_snapshot_entries(layout, &entries)?;
    header.used_blocks = used_blocks;
    header.mapped_extents = extent_count;
    write_storage_sector(layout.extent_sector, &extent_table.encode_sector())?;
    write_storage_sector(layout.index_sector, &header.encode_sector())?;

    let Some(mut superblock) = load_superblock()? else {
        return Err(Errno::NoEnt);
    };
    superblock.replay_generation = superblock.generation;
    superblock.recovered_commit_count = superblock.recovered_commit_count.saturating_add(1);
    fill_fixed(&mut superblock.state_label, STORAGE_STATE_RECOVERED);
    fill_fixed(&mut superblock.last_commit_tag, "storage-repair");
    superblock.repaired_snapshot_count = superblock.repaired_snapshot_count.saturating_add(1);
    push_lineage_event(&mut superblock, "repair", "storage-repair");
    write_storage_sector(layout.superblock_sector, &superblock.encode_sector())?;
    Ok(superblock.generation as usize)
}

fn read_storage_sector(sector: u64) -> Result<[u8; 512], Errno> {
    unsafe {
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioBlkDriver>();
        driver.read_sector(platform, sector).map_err(|_| Errno::Io)
    }
}

fn write_storage_sector(sector: u64, bytes: &[u8; 512]) -> Result<(), Errno> {
    unsafe {
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioBlkDriver>();
        driver
            .write_sector(platform, sector, bytes)
            .map_err(|_| Errno::Io)
    }
}

fn load_superblock() -> Result<Option<PersistSuperblock>, Errno> {
    let (superblock_sector, _, _) = sector_layout().ok_or(Errno::Nxio)?;
    let sector = read_storage_sector(superblock_sector)?;
    Ok(PersistSuperblock::decode_sector(&sector))
}

pub fn inspect_volume(path: &str) -> Option<NativeStorageVolumeRecord> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return None;
    }
    let snapshot = snapshot_layout()?;
    let stats = snapshot_stats(snapshot).ok()?;
    let superblock = load_superblock().ok().flatten();
    let mut record = NativeStorageVolumeRecord {
        valid: 0,
        dirty: 0,
        payload_len: 0,
        generation: 0,
        parent_generation: 0,
        replay_generation: 0,
        payload_checksum: 0,
        superblock_sector: snapshot.superblock_sector,
        journal_sector: snapshot.journal_sector,
        data_sector: snapshot.data_sector,
        index_sector: snapshot.index_sector,
        alloc_sector: snapshot.alloc_sector,
        data_start_sector: snapshot.data_start_sector,
        prepared_commit_count: 0,
        recovered_commit_count: 0,
        repaired_snapshot_count: 0,
        allocation_total_blocks: stats.total_blocks,
        allocation_used_blocks: stats.used_blocks,
        mapped_file_count: stats.file_count,
        mapped_extent_count: stats.extent_count,
        mapped_directory_count: stats.directory_count,
        mapped_symlink_count: stats.symlink_count,
        volume_id: [0; 32],
        state_label: [0; 32],
        last_commit_tag: [0; 32],
        payload_preview: [0; 32],
    };
    fill_fixed(&mut record.volume_id, STORAGE_VOLUME_ID);
    fill_fixed(&mut record.state_label, STORAGE_STATE_UNINITIALIZED);
    if let Some(superblock) = superblock {
        record.valid = 1;
        record.dirty = superblock.dirty;
        record.payload_len = superblock.payload_len;
        record.generation = superblock.generation;
        record.parent_generation = superblock.parent_generation;
        record.replay_generation = superblock.replay_generation;
        record.prepared_commit_count = superblock.prepared_commit_count;
        record.recovered_commit_count = superblock.recovered_commit_count;
        record.repaired_snapshot_count = superblock.repaired_snapshot_count;
        record.payload_checksum = superblock.payload_checksum;
        record.volume_id = superblock.volume_id;
        record.state_label = superblock.state_label;
        record.last_commit_tag = superblock.last_commit_tag;
        record.payload_preview = superblock.payload_preview;
    }
    Some(record)
}

pub fn inspect_lineage(path: &str) -> Option<NativeStorageLineageRecord> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return None;
    }
    let Some(superblock) = load_superblock().ok().flatten() else {
        return Some(NativeStorageLineageRecord {
            valid: 0,
            lineage_contiguous: 1,
            count: 0,
            newest_generation: 0,
            oldest_generation: 0,
            entries: [NativeStorageLineageEntry {
                generation: 0,
                parent_generation: 0,
                payload_checksum: 0,
                kind_label: [0; 16],
                state_label: [0; 16],
                tag_label: [0; 32],
            }; STORAGE_LINEAGE_DEPTH],
        });
    };
    let count = (superblock.lineage_count as usize).min(STORAGE_PERSIST_LINEAGE_DEPTH);
    let mut entries = [NativeStorageLineageEntry {
        generation: 0,
        parent_generation: 0,
        payload_checksum: 0,
        kind_label: [0; 16],
        state_label: [0; 16],
        tag_label: [0; 32],
    }; STORAGE_LINEAGE_DEPTH];
    let mut lineage_contiguous = true;
    for (out_index, slot_back) in (0..count).enumerate() {
        let slot =
            (superblock.lineage_head as usize + STORAGE_PERSIST_LINEAGE_DEPTH - 1 - slot_back)
                % STORAGE_PERSIST_LINEAGE_DEPTH;
        let event = superblock.lineage_events[slot];
        entries[out_index] = NativeStorageLineageEntry {
            generation: event.generation,
            parent_generation: event.parent_generation,
            payload_checksum: event.payload_checksum,
            kind_label: event.kind_label,
            state_label: event.state_label,
            tag_label: event.tag_label,
        };
        if out_index > 0 && entries[out_index - 1].parent_generation != event.generation {
            lineage_contiguous = false;
        }
    }
    Some(NativeStorageLineageRecord {
        valid: 1,
        lineage_contiguous: u32::from(lineage_contiguous),
        count: count as u64,
        newest_generation: entries[0].generation,
        oldest_generation: entries[count.saturating_sub(1)].generation,
        entries,
    })
}

pub fn prepare_storage_commit(path: &str, tag: &str, payload: &[u8]) -> Result<usize, Errno> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return Err(Errno::Nxio);
    }
    if tag.is_empty() {
        return Err(Errno::Inval);
    }
    if payload.len() > STORAGE_MAX_PERSIST_PAYLOAD {
        return Err(Errno::TooBig);
    }
    let (superblock_sector, journal_sector, data_sector) = sector_layout().ok_or(Errno::Nxio)?;
    let previous = load_superblock()?.unwrap_or(PersistSuperblock {
        magic: STORAGE_VOLUME_MAGIC,
        version: STORAGE_VOLUME_VERSION,
        dirty: 0,
        generation: 0,
        parent_generation: 0,
        replay_generation: 0,
        prepared_commit_count: 0,
        recovered_commit_count: 0,
        repaired_snapshot_count: 0,
        lineage_head: 0,
        lineage_count: 0,
        payload_len: 0,
        payload_checksum: 0,
        superblock_sector,
        journal_sector,
        data_sector,
        volume_id: [0; 32],
        state_label: [0; 32],
        last_commit_tag: [0; 32],
        payload_preview: [0; 32],
        lineage_events: [PersistLineageEvent::empty(); STORAGE_PERSIST_LINEAGE_DEPTH],
    });
    let mut journal = [0u8; 512];
    journal[..payload.len()].copy_from_slice(payload);
    write_storage_sector(journal_sector, &journal)?;

    let mut superblock = previous;
    superblock.magic = STORAGE_VOLUME_MAGIC;
    superblock.version = STORAGE_VOLUME_VERSION;
    superblock.dirty = 1;
    superblock.parent_generation = previous.generation;
    superblock.generation = previous.generation.saturating_add(1);
    superblock.prepared_commit_count = previous.prepared_commit_count.saturating_add(1);
    superblock.payload_len = payload.len() as u64;
    superblock.payload_checksum = checksum64(payload);
    superblock.superblock_sector = superblock_sector;
    superblock.journal_sector = journal_sector;
    superblock.data_sector = data_sector;
    superblock.payload_preview = [0; 32];
    superblock.payload_preview[..payload.len().min(32)]
        .copy_from_slice(&payload[..payload.len().min(32)]);
    fill_fixed(&mut superblock.volume_id, STORAGE_VOLUME_ID);
    fill_fixed(&mut superblock.state_label, STORAGE_STATE_PREPARED);
    fill_fixed(&mut superblock.last_commit_tag, tag);
    push_lineage_event(&mut superblock, "prepare", tag);
    write_storage_sector(superblock_sector, &superblock.encode_sector())?;
    Ok(superblock.generation as usize)
}

pub fn recover_storage_volume(path: &str) -> Result<usize, Errno> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return Err(Errno::Nxio);
    }
    let Some(mut superblock) = load_superblock()? else {
        return Err(Errno::NoEnt);
    };
    if superblock.dirty == 0 {
        return Ok(superblock.generation as usize);
    }
    let journal = read_storage_sector(superblock.journal_sector)?;
    write_storage_sector(superblock.data_sector, &journal)?;
    superblock.dirty = 0;
    superblock.replay_generation = superblock.generation;
    superblock.recovered_commit_count = superblock.recovered_commit_count.saturating_add(1);
    fill_fixed(&mut superblock.state_label, STORAGE_STATE_RECOVERED);
    let recovery_tag = fixed_text(&superblock.last_commit_tag);
    push_lineage_event(&mut superblock, "recover", &recovery_tag);
    write_storage_sector(superblock.superblock_sector, &superblock.encode_sector())?;
    Ok(superblock.generation as usize)
}

pub fn active_storage_payload(path: &str) -> Result<Vec<u8>, Errno> {
    if path != STORAGE_DEVICE_PATH || !is_online() {
        return Err(Errno::Nxio);
    }
    let Some(superblock) = load_superblock()? else {
        return Ok(Vec::new());
    };
    let source_sector = if superblock.dirty != 0 {
        superblock.journal_sector
    } else {
        superblock.data_sector
    };
    let sector = read_storage_sector(source_sector)?;
    let payload_len = (superblock.payload_len as usize).min(STORAGE_MAX_PERSIST_PAYLOAD);
    Ok(sector[..payload_len].to_vec())
}

pub fn commit_storage_snapshot(path: &str, tag: &str, payload: &[u8]) -> Result<usize, Errno> {
    let generation = prepare_storage_commit(path, tag, payload)?;
    let _ = recover_storage_volume(path)?;
    Ok(generation)
}

fn handle_irq(_line: u8) {
    unsafe {
        if !DRIVER_ONLINE {
            return;
        }
        let irq_id = diagnostics::next_irq_id();
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioBlkDriver>();
        if let Ok(summary) = driver.service_interrupt(platform) {
            STORAGE_RUNTIME.with_mut(|runtime| {
                if let Some(runtime) = runtime.as_mut() {
                    runtime.last_irq_id = irq_id;
                }
            });
            serial::print(format_args!(
                "ngos/x86_64: virtio-blk irq completed={} isr={:#x}\n",
                summary.completed, summary.isr_status
            ));
            diagnostics::trace_emit(
                TraceKind::Irq,
                TraceChannel::Irq,
                crate::diagnostics::BootTraceStage::DeviceBringup as u16,
                irq_id,
                summary.completed as u64,
                summary.isr_status as u64,
                0,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_entry_sectors_round_trip_more_than_legacy_limit() {
        let entries = (0..12)
            .map(|index| {
                let mut name = [0u8; STORAGE_SNAPSHOT_MAX_NAME];
                let text = format!("file-{index}.txt");
                name[..text.len()].copy_from_slice(text.as_bytes());
                PersistSnapshotEntry {
                    name,
                    kind: STORAGE_SNAPSHOT_ENTRY_FILE,
                    reserved: 0,
                    data_len: (index + 1) as u64,
                    first_extent: index as u32,
                    extent_count: 1,
                }
            })
            .collect::<Vec<_>>();
        let sectors = encode_snapshot_entry_sectors(&entries).unwrap();
        let decoded = decode_snapshot_entry_sectors(&sectors, entries.len()).unwrap();
        assert_eq!(decoded.len(), 12);
        assert_eq!(fixed_text(&decoded[11].name), "file-11.txt");
        assert_eq!(decoded[11].first_extent, 11);
    }

    #[test]
    fn snapshot_entry_sectors_reject_overflow() {
        let entries = vec![PersistSnapshotEntry::empty(); STORAGE_SNAPSHOT_MAX_FILES + 1];
        assert_eq!(encode_snapshot_entry_sectors(&entries), Err(Errno::TooBig));
    }
}
