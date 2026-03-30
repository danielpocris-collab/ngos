use core::mem::MaybeUninit;
use core::ptr;

use platform_hal::{BarKind, CachePolicy, DevicePlatform, MemoryPermissions, PageMapping};
use platform_x86_64::{
    DmaWindow, PciLegacyPortBackend, VirtioNetDriver, X86_64DevicePlatform,
    X86_64DevicePlatformConfig,
};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};
use crate::paging::ActivePageTables;
use crate::phys_alloc::{BootFrameAllocator, frame_bytes};
use crate::{EarlyBootState, pic, serial, timer};

static mut PLATFORM: MaybeUninit<X86_64DevicePlatform<PciLegacyPortBackend>> =
    MaybeUninit::uninit();
static mut DRIVER: MaybeUninit<VirtioNetDriver> = MaybeUninit::uninit();
static mut DRIVER_ONLINE: bool = false;
static mut RX_FRAMES: usize = 0;
static mut TX_COMPLETIONS: usize = 0;
static mut IRQ_LINE: u8 = 0;

const DMA_WINDOW_FRAMES: usize = 128;
const TEST_ETHERTYPE: [u8; 2] = [0x88, 0xb5];

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
        IRQ_LINE = line;
    }

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
