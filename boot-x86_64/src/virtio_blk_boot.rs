extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use ngos_user_abi::{
    Errno, NATIVE_BLOCK_IO_MAGIC, NATIVE_BLOCK_IO_OP_READ, NATIVE_BLOCK_IO_OP_WRITE,
    NATIVE_BLOCK_IO_VERSION, NativeBlockIoRequest, NativeDeviceRecord, NativeDriverRecord, POLLIN,
    POLLOUT,
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
