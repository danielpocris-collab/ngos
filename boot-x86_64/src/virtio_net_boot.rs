use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use ngos_user_abi::{
    Errno, NativeDeviceRecord, NativeDriverRecord, NativeNetworkEventKind,
    NativeNetworkInterfaceRecord, POLLIN, POLLOUT,
};
use platform_hal::{BarKind, CachePolicy, DevicePlatform, MemoryPermissions, PageMapping};
use platform_x86_64::{
    DmaWindow, PciLegacyPortBackend, VirtioNetDriver, X86_64DevicePlatform,
    X86_64DevicePlatformConfig,
};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};
use crate::boot_network_runtime::{NETWORK_DEVICE_PATH, NETWORK_DRIVER_PATH};
use crate::paging::ActivePageTables;
use crate::phys_alloc::{BootFrameAllocator, frame_bytes};
use crate::{EarlyBootState, pic, serial, timer};

static mut PLATFORM: MaybeUninit<X86_64DevicePlatform<PciLegacyPortBackend>> =
    MaybeUninit::uninit();
static mut DRIVER: MaybeUninit<VirtioNetDriver> = MaybeUninit::uninit();
static mut DRIVER_ONLINE: bool = false;
static mut RX_FRAMES: usize = 0;
static mut TX_COMPLETIONS: usize = 0;
static mut TX_SUBMISSIONS: usize = 0;
static mut IRQ_LINE: u8 = 0;
static mut MAC_ADDRESS: [u8; 6] = [0; 6];
static NETWORK_IO_RUNTIME: NetworkIoRuntimeCell = NetworkIoRuntimeCell::new();

const DMA_WINDOW_FRAMES: usize = 128;
const TEST_ETHERTYPE: [u8; 2] = [0x88, 0xb5];
const NETWORK_DEVICE_CLASS: u32 = 6;
const NETWORK_DEVICE_STATE_REGISTERED: u32 = 0;
const NETWORK_DRIVER_STATE_ACTIVE: u32 = 1;
const NETWORK_REQUEST_STATE_INFLIGHT: u32 = 1;
const NETWORK_REQUEST_STATE_COMPLETED: u32 = 2;
const NETWORK_QUEUE_CAPACITY: u64 = 128;

#[derive(Clone)]
struct NetworkIoRuntimeState {
    completion_queue: VecDeque<Vec<u8>>,
    arp_cache: Vec<([u8; 4], [u8; 6])>,
    admin_up: bool,
    link_up: bool,
    promiscuous: bool,
    mtu: u64,
    tx_capacity: u64,
    rx_capacity: u64,
    tx_inflight_limit: u64,
    ipv4_addr: [u8; 4],
    ipv4_netmask: [u8; 4],
    ipv4_gateway: [u8; 4],
    tx_dropped: u64,
    rx_dropped: u64,
    tracked_udp_tx_baseline: u64,
    tracked_udp_tx_pending: u64,
    tracked_udp_tx_drained: u64,
}

impl Default for NetworkIoRuntimeState {
    fn default() -> Self {
        Self {
            completion_queue: VecDeque::new(),
            arp_cache: Vec::new(),
            admin_up: true,
            link_up: true,
            promiscuous: false,
            mtu: 1500,
            tx_capacity: NETWORK_QUEUE_CAPACITY,
            rx_capacity: NETWORK_QUEUE_CAPACITY,
            tx_inflight_limit: NETWORK_QUEUE_CAPACITY,
            ipv4_addr: [0; 4],
            ipv4_netmask: [0; 4],
            ipv4_gateway: [0; 4],
            tx_dropped: 0,
            rx_dropped: 0,
            tracked_udp_tx_baseline: 0,
            tracked_udp_tx_pending: 0,
            tracked_udp_tx_drained: 0,
        }
    }
}

struct NetworkIoRuntimeCell {
    locked: AtomicBool,
    state: UnsafeCell<Option<NetworkIoRuntimeState>>,
}

unsafe impl Sync for NetworkIoRuntimeCell {}

impl NetworkIoRuntimeCell {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(None),
        }
    }

    fn initialize(&self) {
        self.with_mut(|state| {
            if state.is_none() {
                *state = Some(NetworkIoRuntimeState::default());
            }
        });
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Option<NetworkIoRuntimeState>) -> R) -> R {
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

pub fn bring_up(
    state: &EarlyBootState<'static>,
    paging: &ActivePageTables,
    allocator: &mut BootFrameAllocator,
) -> Result<bool, &'static str> {
    boot_locator::event(
        BootLocatorStage::User,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x600,
        BootPayloadLabel::Length,
        frame_bytes(DMA_WINDOW_FRAMES),
        BootPayloadLabel::Count,
        DMA_WINDOW_FRAMES as u64,
    );
    let dma_run = allocator
        .allocate_frames(DMA_WINDOW_FRAMES)
        .map_err(|_| "virtio-net dma window allocation failed")?;
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
        .map_err(|_| "virtio-net pci enumeration failed")?;
    boot_locator::event(
        BootLocatorStage::User,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x610,
        BootPayloadLabel::Count,
        devices.len() as u64,
        BootPayloadLabel::None,
        0,
    );
    let Some(record) = devices
        .iter()
        .find(|record| matches!(VirtioNetDriver::probe(&mut platform, record), Ok(Some(_))))
        .cloned()
    else {
        serial::print(format_args!(
            "ngos/x86_64: virtio-net: no matching pci device\n"
        ));
        return Ok(false);
    };

    serial::print(format_args!(
        "ngos/x86_64: virtio-net candidate vendor={:#06x} device={:#06x} subsystem={:#06x}:{:#06x} class={:#04x}:{:#04x} pi={:#04x} bars={} irqs={}\n",
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
            "ngos/x86_64: virtio-net bar[{}] kind={:?} base={:#x} size={:#x} prefetchable={} cacheable={} readonly={}\n",
            index,
            bar.kind,
            bar.base,
            bar.size,
            bar.flags.prefetchable,
            bar.flags.cacheable,
            bar.flags.read_only,
        ));
    }
    for (index, interrupt) in record.interrupts.iter().enumerate() {
        serial::print(format_args!(
            "ngos/x86_64: virtio-net irq[{}] kind={:?} vectors={} line={:?} trigger={:?} polarity={:?}\n",
            index,
            interrupt.kind,
            interrupt.vectors,
            interrupt.line,
            interrupt.trigger,
            interrupt.polarity,
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
            .map_err(|_| "virtio-net mmio alias map failed")?;
        direct_map_size = direct_map_size.max(bar_end);
        serial::print(format_args!(
            "ngos/x86_64: virtio-net mmio alias base={:#x} len={:#x} virt={:#x}\n",
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
        .map_err(|_| "virtio-net pci re-enumeration failed")?;
    let record = devices
        .into_iter()
        .find(|candidate| candidate.locator == record.locator)
        .ok_or("virtio-net record disappeared after mmio alias setup")?;

    let driver = VirtioNetDriver::initialize(&mut platform, &record).map_err(|error| {
        serial::print(format_args!(
            "ngos/x86_64: virtio-net initialize error: {:?}\n",
            error
        ));
        "virtio-net initialize failed"
    })?;
    let route = driver.interrupt_route();
    let Some(line) = route.line else {
        return Err("virtio-net legacy interrupt line missing");
    };
    if !crate::irq_registry::register_irq_handler(line, handle_irq) {
        return Err("virtio-net irq registry registration failed");
    }
    pic::unmask_irq_line(line);

    let mac = driver.mac_address();
    unsafe {
        ptr::addr_of_mut!(PLATFORM)
            .cast::<X86_64DevicePlatform<PciLegacyPortBackend>>()
            .write(platform);
        ptr::addr_of_mut!(DRIVER)
            .cast::<VirtioNetDriver>()
            .write(driver);
        DRIVER_ONLINE = true;
        RX_FRAMES = 0;
        TX_COMPLETIONS = 0;
        TX_SUBMISSIONS = 0;
        IRQ_LINE = line;
        MAC_ADDRESS = mac;
    }
    NETWORK_IO_RUNTIME.initialize();

    serial::print(format_args!(
        "ngos/x86_64: virtio-net online mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} irq_line={} irq_vector={}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], line, route.vector
    ));
    boot_locator::event(
        BootLocatorStage::User,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x620,
        BootPayloadLabel::Status,
        line as u64,
        BootPayloadLabel::Status,
        route.vector as u64,
    );
    send_probe_burst();
    Ok(true)
}

pub fn is_online() -> bool {
    unsafe { DRIVER_ONLINE }
}

pub fn device_record(path: &str) -> Option<NativeDeviceRecord> {
    if path != NETWORK_DEVICE_PATH || !is_online() {
        return None;
    }
    unsafe {
        let submitted_requests = TX_SUBMISSIONS as u64;
        let completed_requests = TX_COMPLETIONS as u64;
        let queue_depth = submitted_requests.saturating_sub(completed_requests);
        let mut last_completed_frame_tag = [0u8; 64];
        let mut last_completed_source_api_name = [0u8; 24];
        let mut last_completed_translation_label = [0u8; 32];
        let mut last_terminal_frame_tag = [0u8; 64];
        let mut last_terminal_source_api_name = [0u8; 24];
        let mut last_terminal_translation_label = [0u8; 32];
        if completed_requests != 0 {
            fill_fixed(&mut last_completed_frame_tag, "virtio-net-probe");
            fill_fixed(&mut last_completed_source_api_name, "virtio-net");
            fill_fixed(&mut last_completed_translation_label, "qemu-hardware");
        }
        let (last_terminal_request_id, last_terminal_state) = if queue_depth != 0 {
            fill_fixed(&mut last_terminal_frame_tag, "virtio-net-probe");
            fill_fixed(&mut last_terminal_source_api_name, "virtio-net");
            fill_fixed(&mut last_terminal_translation_label, "qemu-hardware");
            (submitted_requests, NETWORK_REQUEST_STATE_INFLIGHT)
        } else if completed_requests != 0 {
            last_terminal_frame_tag = last_completed_frame_tag;
            last_terminal_source_api_name = last_completed_source_api_name;
            last_terminal_translation_label = last_completed_translation_label;
            (completed_requests, NETWORK_REQUEST_STATE_COMPLETED)
        } else {
            (0, 0)
        };
        Some(NativeDeviceRecord {
            class: NETWORK_DEVICE_CLASS,
            state: NETWORK_DEVICE_STATE_REGISTERED,
            reserved0: u64::from(u16::from_be_bytes([MAC_ADDRESS[0], MAC_ADDRESS[1]])),
            queue_depth,
            queue_capacity: NETWORK_QUEUE_CAPACITY,
            submitted_requests,
            completed_requests,
            total_latency_ticks: 0,
            max_latency_ticks: 0,
            total_queue_wait_ticks: 0,
            max_queue_wait_ticks: 0,
            link_up: interface_state().link_up as u32,
            reserved1: 0,
            block_size: 0,
            reserved2: 0,
            capacity_bytes: 0,
            last_completed_request_id: completed_requests,
            last_completed_frame_tag,
            last_completed_source_api_name,
            last_completed_translation_label,
            last_terminal_request_id,
            last_terminal_state,
            reserved3: 0,
            last_terminal_frame_tag,
            last_terminal_source_api_name,
            last_terminal_translation_label,
        })
    }
}

pub fn interface_record(path: &str) -> Option<NativeNetworkInterfaceRecord> {
    if path != NETWORK_DEVICE_PATH || !is_online() {
        return None;
    }
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let state = state.as_mut()?;
        let tx_inflight_depth = current_queue_depth();
        let rx_ring_depth = state.completion_queue.len() as u64;
        Some(NativeNetworkInterfaceRecord {
            admin_up: state.admin_up as u32,
            link_up: state.link_up as u32,
            promiscuous: state.promiscuous as u32,
            reserved: 0,
            mtu: state.mtu,
            tx_capacity: state.tx_capacity,
            rx_capacity: state.rx_capacity,
            tx_inflight_limit: state.tx_inflight_limit,
            tx_inflight_depth,
            free_buffer_count: state.rx_capacity.saturating_sub(rx_ring_depth),
            mac: unsafe { MAC_ADDRESS },
            mac_reserved: [0; 2],
            ipv4_addr: state.ipv4_addr,
            ipv4_netmask: state.ipv4_netmask,
            ipv4_gateway: state.ipv4_gateway,
            ipv4_reserved: [0; 4],
            rx_ring_depth,
            tx_ring_depth: tx_inflight_depth,
            tx_packets: unsafe { TX_SUBMISSIONS as u64 },
            rx_packets: unsafe { RX_FRAMES as u64 },
            tx_completions: unsafe { TX_COMPLETIONS as u64 },
            tx_dropped: state.tx_dropped,
            rx_dropped: state.rx_dropped,
            attached_socket_count: 0,
        })
    })
}

pub fn configure_interface_ipv4(
    path: &str,
    addr: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
) -> Result<(), Errno> {
    if path != NETWORK_DEVICE_PATH || !is_online() {
        return Err(Errno::NoEnt);
    }
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        state.ipv4_addr = addr;
        state.ipv4_netmask = netmask;
        state.ipv4_gateway = gateway;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub fn configure_interface_admin(
    path: &str,
    mtu: u64,
    tx_capacity: u64,
    rx_capacity: u64,
    tx_inflight_limit: u64,
    admin_up: bool,
    promiscuous: bool,
) -> Result<(), Errno> {
    if path != NETWORK_DEVICE_PATH || !is_online() {
        return Err(Errno::NoEnt);
    }
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        state.mtu = mtu.max(576);
        state.tx_capacity = tx_capacity.max(1);
        state.rx_capacity = rx_capacity.max(1);
        state.tx_inflight_limit = tx_inflight_limit.max(1);
        state.admin_up = admin_up;
        state.promiscuous = promiscuous;
        Ok(())
    })
}

pub fn set_link_state(path: &str, link_up: bool) -> Result<(), Errno> {
    if path != NETWORK_DEVICE_PATH || !is_online() {
        return Err(Errno::NoEnt);
    }
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        state.link_up = link_up;
        Ok(())
    })
}

pub fn driver_record(path: &str) -> Option<NativeDriverRecord> {
    if path != NETWORK_DRIVER_PATH || !is_online() {
        return None;
    }
    unsafe {
        let submitted_requests = TX_SUBMISSIONS as u64;
        let completed_requests = TX_COMPLETIONS as u64;
        let in_flight_requests = submitted_requests.saturating_sub(completed_requests);
        let mut last_completed_frame_tag = [0u8; 64];
        let mut last_completed_source_api_name = [0u8; 24];
        let mut last_completed_translation_label = [0u8; 32];
        let mut last_terminal_frame_tag = [0u8; 64];
        let mut last_terminal_source_api_name = [0u8; 24];
        let mut last_terminal_translation_label = [0u8; 32];
        if completed_requests != 0 {
            fill_fixed(&mut last_completed_frame_tag, "virtio-net-probe");
            fill_fixed(&mut last_completed_source_api_name, "virtio-net");
            fill_fixed(&mut last_completed_translation_label, "qemu-hardware");
        }
        let (last_terminal_request_id, last_terminal_state) = if in_flight_requests != 0 {
            fill_fixed(&mut last_terminal_frame_tag, "virtio-net-probe");
            fill_fixed(&mut last_terminal_source_api_name, "virtio-net");
            fill_fixed(&mut last_terminal_translation_label, "qemu-hardware");
            (submitted_requests, NETWORK_REQUEST_STATE_INFLIGHT)
        } else if completed_requests != 0 {
            last_terminal_frame_tag = last_completed_frame_tag;
            last_terminal_source_api_name = last_completed_source_api_name;
            last_terminal_translation_label = last_completed_translation_label;
            (completed_requests, NETWORK_REQUEST_STATE_COMPLETED)
        } else {
            (0, 0)
        };
        Some(NativeDriverRecord {
            state: NETWORK_DRIVER_STATE_ACTIVE,
            reserved: 0,
            bound_device_count: 1,
            queued_requests: in_flight_requests,
            in_flight_requests,
            completed_requests,
            last_completed_request_id: completed_requests,
            last_completed_frame_tag,
            last_completed_source_api_name,
            last_completed_translation_label,
            last_terminal_request_id,
            last_terminal_state,
            reserved1: 0,
            last_terminal_frame_tag,
            last_terminal_source_api_name,
            last_terminal_translation_label,
        })
    }
}

pub fn poll_device(interest: u32) -> usize {
    if !is_online() {
        return 0;
    }
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_mut() else {
            return 0;
        };
        let mut ready = POLLOUT;
        if !state.completion_queue.is_empty() {
            ready |= POLLIN;
        }
        (ready & interest) as usize
    })
}

pub fn read_device(buffer: *mut u8, len: usize, nonblock: bool) -> Result<usize, Errno> {
    if !is_online() {
        return Err(Errno::Nxio);
    }
    if len == 0 {
        return Ok(0);
    }
    if buffer.is_null() {
        return Err(Errno::Fault);
    }
    NETWORK_IO_RUNTIME.initialize();
    loop {
        let maybe_payload =
            NETWORK_IO_RUNTIME.with_mut(|state| state.as_mut()?.completion_queue.pop_front());
        if let Some(payload) = maybe_payload {
            let copy_len = payload.len().min(len);
            unsafe {
                ptr::copy_nonoverlapping(payload.as_ptr(), buffer, copy_len);
            }
            serial::print(format_args!(
                "ngos/x86_64: virtio-net raw read copied={}\n",
                copy_len
            ));
            return Ok(copy_len);
        }
        if nonblock {
            return Err(Errno::Again);
        }
        core::hint::spin_loop();
    }
}

pub fn write_device(bytes: &[u8]) -> Result<usize, Errno> {
    if !is_online() {
        return Err(Errno::Nxio);
    }
    if bytes.len() < 60 {
        return Err(Errno::Inval);
    }
    send_frame_internal(bytes)
}

pub fn send_udp_packet(
    local_ip: [u8; 4],
    local_port: u16,
    remote_ip: [u8; 4],
    remote_port: u16,
    payload: &[u8],
) -> Result<usize, Errno> {
    if !is_online() {
        return Err(Errno::Nxio);
    }
    let state = interface_state();
    if local_port == 0 || local_ip == [0, 0, 0, 0] {
        return Err(Errno::Inval);
    }
    let route_ip = select_route_target(local_ip, remote_ip, state.ipv4_netmask, state.ipv4_gateway);
    let dst_mac = resolve_arp_mac(route_ip, local_ip)?;
    NETWORK_IO_RUNTIME.with_mut(|runtime| {
        if let Some(runtime) = runtime.as_mut() {
            if runtime.tracked_udp_tx_pending == 0 {
                runtime.tracked_udp_tx_baseline = unsafe { TX_COMPLETIONS as u64 };
                runtime.tracked_udp_tx_drained = 0;
            }
            runtime.tracked_udp_tx_pending = runtime.tracked_udp_tx_pending.saturating_add(1);
        }
    });
    let frame = build_udp_ipv4_frame(
        unsafe { MAC_ADDRESS },
        dst_mac,
        local_ip,
        remote_ip,
        local_port,
        remote_port,
        payload,
    );
    if let Err(errno) = send_frame_internal(&frame) {
        NETWORK_IO_RUNTIME.with_mut(|runtime| {
            if let Some(runtime) = runtime.as_mut() {
                runtime.tracked_udp_tx_pending = runtime.tracked_udp_tx_pending.saturating_sub(1);
                if runtime.tracked_udp_tx_pending == 0 {
                    runtime.tracked_udp_tx_baseline = unsafe { TX_COMPLETIONS as u64 };
                    runtime.tracked_udp_tx_drained = 0;
                }
            }
        });
        return Err(errno);
    }
    Ok(payload.len())
}

pub fn complete_udp_tx(driver_path: &str, completions: usize) -> Result<usize, Errno> {
    if driver_path != NETWORK_DRIVER_PATH || !is_online() {
        return Err(Errno::NoEnt);
    }
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let state = state.as_mut().ok_or(Errno::NoEnt)?;
        let available = unsafe { TX_COMPLETIONS as u64 }
            .saturating_sub(state.tracked_udp_tx_baseline)
            .saturating_sub(state.tracked_udp_tx_drained);
        let count = available
            .min(state.tracked_udp_tx_pending)
            .min(completions as u64) as usize;
        state.tracked_udp_tx_drained = state.tracked_udp_tx_drained.saturating_add(count as u64);
        state.tracked_udp_tx_pending = state.tracked_udp_tx_pending.saturating_sub(count as u64);
        if state.tracked_udp_tx_pending == 0 {
            state.tracked_udp_tx_baseline = unsafe { TX_COMPLETIONS as u64 };
            state.tracked_udp_tx_drained = 0;
        }
        Ok(count)
    })
}

pub fn wait_for_external_traffic(timeout_ms: u64) {
    let start = timer::boot_uptime_micros().unwrap_or(0);
    loop {
        let elapsed_us = timer::boot_uptime_micros()
            .unwrap_or(start)
            .saturating_sub(start);
        if elapsed_us >= timeout_ms.saturating_mul(1000) {
            break;
        }
        unsafe {
            if DRIVER_ONLINE && RX_FRAMES != 0 {
                break;
            }
        }
        core::hint::spin_loop();
    }
    unsafe {
        if DRIVER_ONLINE {
            let tx_completions = TX_COMPLETIONS;
            let rx_frames = RX_FRAMES;
            let irq_dispatches = crate::irq_registry::irq_dispatch_count(IRQ_LINE);
            serial::print(format_args!(
                "ngos/x86_64: virtio-net summary tx_completions={} rx_frames={} irq_dispatches={}\n",
                tx_completions, rx_frames, irq_dispatches
            ));
        }
    }
}

fn fill_fixed<const N: usize>(dst: &mut [u8; N], text: &str) {
    *dst = [0; N];
    let bytes = text.as_bytes();
    let len = bytes.len().min(N);
    dst[..len].copy_from_slice(&bytes[..len]);
}

fn current_queue_depth() -> u64 {
    unsafe { (TX_SUBMISSIONS as u64).saturating_sub(TX_COMPLETIONS as u64) }
}

fn interface_state() -> NetworkIoRuntimeState {
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| state.as_ref().cloned().unwrap_or_default())
}

fn normalize_outbound_frame(bytes: &[u8]) -> Vec<u8> {
    let mut payload = bytes.to_vec();
    unsafe {
        if payload.len() >= 12 && payload[6..12].iter().all(|byte| *byte == 0) {
            payload[6..12].copy_from_slice(&MAC_ADDRESS);
        }
        if payload.len() >= 28
            && payload[12] == 0x08
            && payload[13] == 0x06
            && payload[22..28].iter().all(|byte| *byte == 0)
        {
            payload[22..28].copy_from_slice(&MAC_ADDRESS);
        }
    }
    payload
}

fn send_frame_internal(bytes: &[u8]) -> Result<usize, Errno> {
    let state = interface_state();
    if !state.admin_up || !state.link_up {
        NETWORK_IO_RUNTIME.with_mut(|runtime| {
            if let Some(runtime) = runtime.as_mut() {
                runtime.tx_dropped = runtime.tx_dropped.saturating_add(1);
            }
        });
        return Err(Errno::Access);
    }
    let payload = normalize_outbound_frame(bytes);
    unsafe {
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioNetDriver>();
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        driver.send(platform, &payload).map_err(|_| Errno::Busy)?;
        TX_SUBMISSIONS = TX_SUBMISSIONS.saturating_add(1);
    }
    serial::print(format_args!(
        "ngos/x86_64: virtio-net raw tx queued len={}\n",
        payload.len()
    ));
    Ok(payload.len())
}

fn select_route_target(
    local_ip: [u8; 4],
    remote_ip: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
) -> [u8; 4] {
    if gateway != [0, 0, 0, 0] && !same_subnet(local_ip, remote_ip, netmask) {
        gateway
    } else {
        remote_ip
    }
}

fn same_subnet(left: [u8; 4], right: [u8; 4], netmask: [u8; 4]) -> bool {
    left.iter()
        .zip(right.iter())
        .zip(netmask.iter())
        .all(|((left, right), mask)| (*left & *mask) == (*right & *mask))
}

fn resolve_arp_mac(target_ip: [u8; 4], source_ip: [u8; 4]) -> Result<[u8; 6], Errno> {
    if let Some(mac) = lookup_arp_cache(target_ip) {
        return Ok(mac);
    }
    let request = build_arp_request_frame(unsafe { MAC_ADDRESS }, source_ip, target_ip);
    let _ = send_frame_internal(&request)?;
    lookup_arp_cache(target_ip).ok_or(Errno::Again)
}

fn lookup_arp_cache(target_ip: [u8; 4]) -> Option<[u8; 6]> {
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let state = state.as_ref()?;
        state
            .arp_cache
            .iter()
            .find(|(ipv4, _)| *ipv4 == target_ip)
            .map(|(_, mac)| *mac)
    })
}

fn remember_arp_mapping(target_ip: [u8; 4], mac: [u8; 6]) {
    NETWORK_IO_RUNTIME.initialize();
    NETWORK_IO_RUNTIME.with_mut(|state| {
        let Some(state) = state.as_mut() else {
            return;
        };
        if let Some((_, cached_mac)) = state
            .arp_cache
            .iter_mut()
            .find(|(ipv4, _)| *ipv4 == target_ip)
        {
            *cached_mac = mac;
            return;
        }
        state.arp_cache.push((target_ip, mac));
    });
}

fn note_arp_frame(payload: &[u8]) {
    let Some((sender_ip, sender_mac)) = parse_arp_reply(payload) else {
        return;
    };
    remember_arp_mapping(sender_ip, sender_mac);
}

fn parse_arp_reply(frame: &[u8]) -> Option<([u8; 4], [u8; 6])> {
    if frame.len() < 42 {
        return None;
    }
    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    if ethertype != 0x0806 {
        return None;
    }
    let opcode = u16::from_be_bytes([frame[20], frame[21]]);
    if opcode != 2 {
        return None;
    }
    let sender_mac = <[u8; 6]>::try_from(&frame[22..28]).ok()?;
    let sender_ip = <[u8; 4]>::try_from(&frame[28..32]).ok()?;
    Some((sender_ip, sender_mac))
}

fn build_arp_request_frame(src_mac: [u8; 6], src_ip: [u8; 4], target_ip: [u8; 4]) -> [u8; 60] {
    let mut frame = [0u8; 60];
    frame[0..6].copy_from_slice(&[0xff; 6]);
    frame[6..12].copy_from_slice(&src_mac);
    frame[12..14].copy_from_slice(&0x0806u16.to_be_bytes());
    frame[14..16].copy_from_slice(&0x0001u16.to_be_bytes());
    frame[16..18].copy_from_slice(&0x0800u16.to_be_bytes());
    frame[18] = 6;
    frame[19] = 4;
    frame[20..22].copy_from_slice(&0x0001u16.to_be_bytes());
    frame[22..28].copy_from_slice(&src_mac);
    frame[28..32].copy_from_slice(&src_ip);
    frame[32..38].copy_from_slice(&[0; 6]);
    frame[38..42].copy_from_slice(&target_ip);
    frame
}

fn checksum16(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    let rem = chunks.remainder();
    if let Some(byte) = rem.first() {
        sum = sum.wrapping_add((*byte as u32) << 8);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn build_udp_ipv4_frame(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let udp_len = 8 + payload.len();
    let ip_len = 20 + udp_len;
    let mut frame = Vec::with_capacity(60.max(14 + ip_len));
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.extend_from_slice(&0x0800u16.to_be_bytes());

    let mut ip_header = [0u8; 20];
    ip_header[0] = 0x45;
    ip_header[2..4].copy_from_slice(&(ip_len as u16).to_be_bytes());
    ip_header[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
    ip_header[8] = 64;
    ip_header[9] = 17;
    ip_header[12..16].copy_from_slice(&src_ip);
    ip_header[16..20].copy_from_slice(&dst_ip);
    let ip_checksum = checksum16(&ip_header);
    ip_header[10..12].copy_from_slice(&ip_checksum.to_be_bytes());
    frame.extend_from_slice(&ip_header);

    let mut udp = Vec::with_capacity(udp_len);
    udp.extend_from_slice(&src_port.to_be_bytes());
    udp.extend_from_slice(&dst_port.to_be_bytes());
    udp.extend_from_slice(&(udp_len as u16).to_be_bytes());
    udp.extend_from_slice(&0u16.to_be_bytes());
    udp.extend_from_slice(payload);

    let mut pseudo = Vec::with_capacity(12 + udp.len());
    pseudo.extend_from_slice(&src_ip);
    pseudo.extend_from_slice(&dst_ip);
    pseudo.push(0);
    pseudo.push(17);
    pseudo.extend_from_slice(&(udp_len as u16).to_be_bytes());
    pseudo.extend_from_slice(&udp);
    let udp_checksum = checksum16(&pseudo);
    udp[6..8].copy_from_slice(&udp_checksum.to_be_bytes());
    frame.extend_from_slice(&udp);
    if frame.len() < 60 {
        frame.resize(60, 0);
    }
    frame
}

fn send_probe_burst() {
    unsafe {
        if !DRIVER_ONLINE {
            return;
        }
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioNetDriver>();
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        let mac = driver.mac_address();
        for sequence in 0..4u8 {
            let frame = build_probe_frame(mac, sequence);
            if driver.send(platform, &frame).is_ok() {
                TX_SUBMISSIONS = TX_SUBMISSIONS.saturating_add(1);
                serial::print(format_args!(
                    "ngos/x86_64: virtio-net tx queued seq={} len={}",
                    sequence,
                    frame.len()
                ));
            } else {
                serial::print(format_args!(
                    "ngos/x86_64: virtio-net tx backpressure seq={}\n",
                    sequence
                ));
            }
        }
    }
}

fn handle_irq(_line: u8) {
    unsafe {
        if !DRIVER_ONLINE {
            return;
        }
        let platform =
            &mut *ptr::addr_of_mut!(PLATFORM).cast::<X86_64DevicePlatform<PciLegacyPortBackend>>();
        let driver = &mut *ptr::addr_of_mut!(DRIVER).cast::<VirtioNetDriver>();
        match driver.service_interrupt(platform) {
            Ok(summary) => {
                TX_COMPLETIONS = TX_COMPLETIONS.saturating_add(summary.tx_completed);
                if summary.tx_completed != 0 {
                    boot_locator::event(
                        BootLocatorStage::User,
                        BootLocatorKind::Transition,
                        BootLocatorSeverity::Info,
                        0x630,
                        BootPayloadLabel::Count,
                        summary.tx_completed as u64,
                        BootPayloadLabel::Status,
                        summary.isr_status as u64,
                    );
                    serial::print(format_args!(
                        "ngos/x86_64: virtio-net irq tx_completed={} isr={:#x}",
                        summary.tx_completed, summary.isr_status
                    ));
                }
                if summary.rx_completed != 0 {
                    boot_locator::event(
                        BootLocatorStage::User,
                        BootLocatorKind::Transition,
                        BootLocatorSeverity::Info,
                        0x640,
                        BootPayloadLabel::Count,
                        summary.rx_completed as u64,
                        BootPayloadLabel::Status,
                        summary.isr_status as u64,
                    );
                    let frames = driver.receive(platform).unwrap_or_default();
                    RX_FRAMES = RX_FRAMES.saturating_add(frames.len());
                    for frame in frames {
                        note_arp_frame(&frame.payload);
                        NETWORK_IO_RUNTIME.initialize();
                        NETWORK_IO_RUNTIME.with_mut(|state| {
                            if let Some(state) = state.as_mut() {
                                state.completion_queue.push_back(frame.payload.clone());
                            }
                        });
                        if crate::boot_network_runtime::ingest_udp_ipv4_frame(&frame.payload) {
                            crate::user_syscall::emit_network_event(
                                NETWORK_DEVICE_PATH,
                                None,
                                NativeNetworkEventKind::RxReady,
                            );
                        }
                        log_rx_frame(&frame.payload);
                    }
                }
            }
            Err(_) => {
                serial::print(format_args!("ngos/x86_64: virtio-net irq service failed\n"));
            }
        }
    }
}

fn log_rx_frame(payload: &[u8]) {
    let preview_len = payload.len().min(32);
    let preview = &payload[..preview_len];
    serial::print(format_args!(
        "ngos/x86_64: virtio-net rx len={} preview={:?}\n",
        payload.len(),
        preview
    ));
}

fn build_probe_frame(mac: [u8; 6], sequence: u8) -> [u8; 64] {
    let mut frame = [0u8; 64];
    frame[0..6].copy_from_slice(&[0xff; 6]);
    frame[6..12].copy_from_slice(&mac);
    frame[12..14].copy_from_slice(&TEST_ETHERTYPE);
    frame[14..18].copy_from_slice(b"NGOS");
    frame[18] = sequence;
    frame[19..34].copy_from_slice(b"virtio-net-test");
    frame
}
