use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;
use core::ptr::{read_volatile, write_volatile};

use platform_hal::{
    BarKind, ConfigAccess, ConfigSpaceKind, ConfigWidth, DeviceLocator, DevicePlatform,
    DeviceRecord, DmaBufferInfo, DmaCoherency, DmaConstraints, DmaDirection, HalError,
    InterruptHandle, InterruptRoute, MmioCachePolicy, MmioMapping, MmioPermissions,
};

const PCI_VENDOR_VIRTIO: u16 = 0x1af4;
const PCI_CAP_ID_VENDOR: u8 = 0x09;
const PCI_STATUS_CAPABILITIES: u16 = 1 << 4;
const PCI_CAP_PTR_OFFSET: u16 = 0x34;
const PCI_STATUS_OFFSET: u16 = 0x06;
const PCI_BAR0_OFFSET: u16 = 0x10;
const PCI_SUBSYSTEM_DEVICE_OFFSET: u16 = 0x2e;
const PCI_SUBSYSTEM_VENDOR_OFFSET: u16 = 0x2c;

const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

const VIRTIO_NET_F_MAC: u64 = 1 << 5;
const VIRTIO_F_VERSION_1: u64 = 1 << 32;

const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
const VIRTIO_STATUS_FAILED: u8 = 128;

const VIRTQ_DESC_F_WRITE: u16 = 2;

const VIRTIO_ISR_QUEUE: u8 = 1;

const RX_QUEUE_INDEX: u16 = 0;
const TX_QUEUE_INDEX: u16 = 1;
const RX_QUEUE_SIZE: u16 = 8;
const TX_QUEUE_SIZE: u16 = 8;
const RX_PACKET_BUFFER_SIZE: u64 = 2048;
const TX_PACKET_BUFFER_SIZE: u64 = 2048;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtioNetHeader {
    flags: u8,
    gso_type: u8,
    hdr_len: u16,
    gso_size: u16,
    csum_start: u16,
    csum_offset: u16,
    num_buffers: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtioNetPciMatch {
    pub device: DeviceLocator,
    pub subsystem_vendor_id: u16,
    pub subsystem_device_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioNetFrame {
    pub payload: Vec<u8>,
    pub used_len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VirtioNetInterruptSummary {
    pub isr_status: u8,
    pub tx_completed: usize,
    pub rx_completed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioNetError {
    Hal(HalError),
    DeviceNotVirtioNet,
    MissingBar,
    MissingCapability,
    QueueUnavailable,
    QueueTooSmall,
    FeaturesRejected,
    TxBackpressure,
    PacketTooLarge,
    InvalidQueueState,
}

impl From<HalError> for VirtioNetError {
    fn from(value: HalError) -> Self {
        Self::Hal(value)
    }
}

#[derive(Debug, Clone, Copy)]
struct VirtioPciCapability {
    cfg_type: u8,
    bar: u8,
    offset: u32,
    length: u32,
    notify_multiplier: u32,
}

#[derive(Debug, Clone, Copy)]
struct CommonCfg {
    base: *mut u8,
}

#[derive(Debug, Clone, Copy)]
struct NotifyCfg {
    base: *mut u8,
    multiplier: u32,
}

#[derive(Debug, Clone, Copy)]
struct IsrCfg {
    base: *mut u8,
}

#[derive(Debug, Clone, Copy)]
struct DeviceCfg {
    base: *mut u8,
}

#[derive(Debug)]
struct VirtQueue {
    size: u16,
    avail_shadow: *mut u16,
    avail_idx_ptr: *mut u16,
    used_idx_ptr: *mut u16,
    desc_ptr: *mut VirtqDesc,
    used_ring_ptr: *mut VirtqUsedElem,
    last_used_idx: u16,
}

#[derive(Debug)]
struct RxSlot {
    buffer: DmaBufferInfo,
}

#[derive(Debug)]
struct TxSlot {
    buffer: DmaBufferInfo,
    busy: bool,
}

pub struct VirtioNetDriver {
    device: DeviceLocator,
    match_info: VirtioNetPciMatch,
    mapping: MmioMapping,
    interrupt_handle: InterruptHandle,
    interrupt_route: InterruptRoute,
    common_cfg: CommonCfg,
    notify_cfg: NotifyCfg,
    isr_cfg: IsrCfg,
    device_cfg: DeviceCfg,
    mac_address: [u8; 6],
    rx_queue: VirtQueue,
    tx_queue: VirtQueue,
    rx_slots: Vec<RxSlot>,
    tx_slots: Vec<TxSlot>,
    free_tx: VecDeque<u16>,
    received_frames: VecDeque<VirtioNetFrame>,
}

impl VirtioNetDriver {
    pub fn probe<P: DevicePlatform>(
        platform: &mut P,
        record: &DeviceRecord,
    ) -> Result<Option<VirtioNetPciMatch>, VirtioNetError> {
        if record.identity.vendor_id != PCI_VENDOR_VIRTIO || record.identity.base_class != 0x02 {
            return Ok(None);
        }
        let subsystem_vendor =
            read_config_u16(platform, record.locator, PCI_SUBSYSTEM_VENDOR_OFFSET)?;
        let subsystem_device =
            read_config_u16(platform, record.locator, PCI_SUBSYSTEM_DEVICE_OFFSET)?;
        if subsystem_vendor != PCI_VENDOR_VIRTIO {
            return Ok(None);
        }
        Ok(Some(VirtioNetPciMatch {
            device: record.locator,
            subsystem_vendor_id: subsystem_vendor,
            subsystem_device_id: subsystem_device,
        }))
    }

    pub fn initialize<P: DevicePlatform>(
        platform: &mut P,
        record: &DeviceRecord,
    ) -> Result<Self, VirtioNetError> {
        let Some(match_info) = Self::probe(platform, record)? else {
            return Err(VirtioNetError::DeviceNotVirtioNet);
        };

        let caps = read_virtio_caps(platform, record.locator)?;
        let common_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_COMMON_CFG)
            .copied()
            .ok_or(VirtioNetError::MissingCapability)?;
        let notify_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_NOTIFY_CFG)
            .copied()
            .ok_or(VirtioNetError::MissingCapability)?;
        let isr_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_ISR_CFG)
            .copied()
            .ok_or(VirtioNetError::MissingCapability)?;
        let device_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_DEVICE_CFG)
            .copied()
            .ok_or(VirtioNetError::MissingCapability)?;

        let bar = select_capability_bar(platform, record, common_cap)?;
        let region = platform.claim_bar(record.locator, bar.id)?;
        let mapping = platform.map_mmio(
            region,
            MmioPermissions::read_write(),
            MmioCachePolicy::Uncacheable,
        )?;
        let mmio_base = mapping.virtual_base as *mut u8;

        validate_bar_cap(&bar, common_cap)?;
        validate_bar_cap(&bar, notify_cap)?;
        validate_bar_cap(&bar, isr_cap)?;
        validate_bar_cap(&bar, device_cap)?;

        let common_cfg = CommonCfg {
            base: unsafe { mmio_base.add(common_cap.offset as usize) },
        };
        let notify_cfg = NotifyCfg {
            base: unsafe { mmio_base.add(notify_cap.offset as usize) },
            multiplier: notify_cap.notify_multiplier,
        };
        let isr_cfg = IsrCfg {
            base: unsafe { mmio_base.add(isr_cap.offset as usize) },
        };
        let device_cfg = DeviceCfg {
            base: unsafe { mmio_base.add(device_cap.offset as usize) },
        };

        common_cfg.write_status(0);
        common_cfg.write_status(VIRTIO_STATUS_ACKNOWLEDGE);
        common_cfg.write_status(VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

        let device_features = read_device_features(common_cfg);
        let negotiated = negotiate_features(device_features)?;
        write_driver_features(common_cfg, negotiated);
        let features_ok =
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK;
        common_cfg.write_status(features_ok);
        if (common_cfg.read_status() & VIRTIO_STATUS_FEATURES_OK) == 0 {
            common_cfg.write_status(VIRTIO_STATUS_FAILED);
            return Err(VirtioNetError::FeaturesRejected);
        }

        let (interrupt_handle, interrupt_route) = platform.claim_interrupt(record.locator, 0)?;
        platform.enable_interrupt(interrupt_handle)?;

        let mut rx_queue = configure_queue(platform, common_cfg, RX_QUEUE_INDEX, RX_QUEUE_SIZE)?;
        let mut tx_queue = configure_queue(platform, common_cfg, TX_QUEUE_INDEX, TX_QUEUE_SIZE)?;
        let rx_slots = allocate_rx_slots(platform, &mut rx_queue)?;
        let (tx_slots, free_tx) = allocate_tx_slots(platform, &mut tx_queue)?;

        let mut mac_address = [0u8; 6];
        for (index, byte) in mac_address.iter_mut().enumerate() {
            *byte = device_cfg.read_u8(index);
        }

        let driver_ok = features_ok | VIRTIO_STATUS_DRIVER_OK;
        common_cfg.write_status(driver_ok);

        Ok(Self {
            device: record.locator,
            match_info,
            mapping,
            interrupt_handle,
            interrupt_route,
            common_cfg,
            notify_cfg,
            isr_cfg,
            device_cfg,
            mac_address,
            rx_queue,
            tx_queue,
            rx_slots,
            tx_slots,
            free_tx,
            received_frames: VecDeque::new(),
        })
    }

    pub fn device(&self) -> DeviceLocator {
        self.device
    }

    pub fn interrupt_handle(&self) -> InterruptHandle {
        self.interrupt_handle
    }

    pub fn interrupt_route(&self) -> InterruptRoute {
        self.interrupt_route
    }

    pub fn mac_address(&self) -> [u8; 6] {
        self.mac_address
    }

    pub fn mapping(&self) -> MmioMapping {
        self.mapping
    }

    pub fn pci_match(&self) -> VirtioNetPciMatch {
        self.match_info
    }

    pub fn negotiated_mac_config(&self) -> [u8; 6] {
        let mut mac = [0u8; 6];
        for (index, byte) in mac.iter_mut().enumerate() {
            *byte = self.device_cfg.read_u8(index);
        }
        mac
    }

    pub fn send<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        payload: &[u8],
    ) -> Result<(), VirtioNetError> {
        if payload.len() as u64 > TX_PACKET_BUFFER_SIZE - size_of::<VirtioNetHeader>() as u64 {
            return Err(VirtioNetError::PacketTooLarge);
        }
        let Some(slot_index) = self.free_tx.pop_front() else {
            return Err(VirtioNetError::TxBackpressure);
        };
        let slot = self
            .tx_slots
            .get_mut(slot_index as usize)
            .ok_or(VirtioNetError::InvalidQueueState)?;
        if slot.busy {
            return Err(VirtioNetError::InvalidQueueState);
        }

        unsafe {
            let header_ptr = slot.buffer.cpu_virtual as *mut VirtioNetHeader;
            write_volatile(header_ptr, VirtioNetHeader::default());
            let payload_ptr =
                (slot.buffer.cpu_virtual + size_of::<VirtioNetHeader>() as u64) as *mut u8;
            core::ptr::copy_nonoverlapping(payload.as_ptr(), payload_ptr, payload.len());
        }
        platform.prepare_dma_for_device(slot.buffer.id)?;
        queue_submit(
            &mut self.tx_queue,
            slot_index,
            slot.buffer.device_address,
            (size_of::<VirtioNetHeader>() + payload.len()) as u32,
            0,
        )?;
        slot.busy = true;
        notify_queue(self.notify_cfg, self.common_cfg, TX_QUEUE_INDEX);
        Ok(())
    }

    pub fn service_interrupt<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
    ) -> Result<VirtioNetInterruptSummary, VirtioNetError> {
        platform.acknowledge_interrupt(self.interrupt_handle)?;
        let isr_status = self.isr_cfg.read_u8();
        let mut summary = VirtioNetInterruptSummary {
            isr_status,
            ..VirtioNetInterruptSummary::default()
        };
        if (isr_status & VIRTIO_ISR_QUEUE) != 0 {
            summary.tx_completed = self.process_tx_completions(platform)?;
            let (frames, completed) = self.process_rx_completions(platform)?;
            for frame in frames {
                self.received_frames.push_back(frame);
            }
            summary.rx_completed = completed;
        }
        Ok(summary)
    }

    pub fn receive<P: DevicePlatform>(
        &mut self,
        _platform: &mut P,
    ) -> Result<Vec<VirtioNetFrame>, VirtioNetError> {
        let mut frames = Vec::with_capacity(self.received_frames.len());
        while let Some(frame) = self.received_frames.pop_front() {
            frames.push(frame);
        }
        Ok(frames)
    }

    fn process_tx_completions<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
    ) -> Result<usize, VirtioNetError> {
        let mut completed = 0usize;
        while self.tx_queue.last_used_idx != self.tx_queue.used_idx() {
            let elem = self.tx_queue.used_elem(self.tx_queue.last_used_idx);
            self.tx_queue.last_used_idx = self.tx_queue.last_used_idx.wrapping_add(1);
            let slot_index = elem.id as usize;
            let slot = self
                .tx_slots
                .get_mut(slot_index)
                .ok_or(VirtioNetError::InvalidQueueState)?;
            if !slot.busy {
                return Err(VirtioNetError::InvalidQueueState);
            }
            platform.complete_dma_from_device(slot.buffer.id)?;
            slot.busy = false;
            self.free_tx.push_back(slot_index as u16);
            completed += 1;
        }
        Ok(completed)
    }

    fn process_rx_completions<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
    ) -> Result<(Vec<VirtioNetFrame>, usize), VirtioNetError> {
        let mut frames = Vec::new();
        let mut completed = 0usize;
        while self.rx_queue.last_used_idx != self.rx_queue.used_idx() {
            let elem = self.rx_queue.used_elem(self.rx_queue.last_used_idx);
            self.rx_queue.last_used_idx = self.rx_queue.last_used_idx.wrapping_add(1);
            let slot_index = elem.id as usize;
            let slot = self
                .rx_slots
                .get_mut(slot_index)
                .ok_or(VirtioNetError::InvalidQueueState)?;
            platform.complete_dma_from_device(slot.buffer.id)?;

            let total_len = elem.len as usize;
            let header_len = size_of::<VirtioNetHeader>();
            if total_len >= header_len {
                let payload_len = total_len - header_len;
                let mut payload = vec![0u8; payload_len];
                unsafe {
                    let payload_ptr = (slot.buffer.cpu_virtual
                        + size_of::<VirtioNetHeader>() as u64)
                        as *const u8;
                    core::ptr::copy_nonoverlapping(payload_ptr, payload.as_mut_ptr(), payload_len);
                }
                frames.push(VirtioNetFrame {
                    payload,
                    used_len: elem.len,
                });
            }

            platform.prepare_dma_for_device(slot.buffer.id)?;
            queue_submit(
                &mut self.rx_queue,
                slot_index as u16,
                slot.buffer.device_address,
                RX_PACKET_BUFFER_SIZE as u32,
                VIRTQ_DESC_F_WRITE,
            )?;
            completed += 1;
        }
        Ok((frames, completed))
    }
}

fn validate_bar_cap(
    bar: &platform_hal::BarInfo,
    cap: VirtioPciCapability,
) -> Result<(), VirtioNetError> {
    if cap.offset as u64 + cap.length as u64 > bar.size {
        return Err(VirtioNetError::MissingCapability);
    }
    Ok(())
}

fn select_capability_bar<P: DevicePlatform>(
    platform: &mut P,
    record: &DeviceRecord,
    cap: VirtioPciCapability,
) -> Result<platform_hal::BarInfo, VirtioNetError> {
    let offset = PCI_BAR0_OFFSET + u16::from(cap.bar) * 4;
    let low = read_config_u32(platform, record.locator, offset)?;
    if (low & 0x1) != 0 {
        return Err(VirtioNetError::MissingBar);
    }

    let memory_type = (low >> 1) & 0x3;
    let (kind, base) = if memory_type == 0x2 {
        let high = read_config_u32(platform, record.locator, offset + 4)?;
        (
            BarKind::Memory64,
            ((high as u64) << 32) | (low as u64 & 0xffff_fff0),
        )
    } else if memory_type == 0x0 {
        (BarKind::Memory32, low as u64 & 0xffff_fff0)
    } else {
        return Err(VirtioNetError::MissingBar);
    };

    record
        .bars
        .iter()
        .find(|bar| bar.kind == kind && bar.base == base)
        .copied()
        .ok_or(VirtioNetError::MissingBar)
}

fn negotiate_features(device_features: u64) -> Result<u64, VirtioNetError> {
    let required = VIRTIO_F_VERSION_1;
    if (device_features & required) != required {
        return Err(VirtioNetError::FeaturesRejected);
    }
    let optional = device_features & VIRTIO_NET_F_MAC;
    Ok(required | optional)
}

fn read_device_features(common_cfg: CommonCfg) -> u64 {
    common_cfg.write_u32(0x00, 0);
    let low = common_cfg.read_u32(0x04) as u64;
    common_cfg.write_u32(0x00, 1);
    let high = common_cfg.read_u32(0x04) as u64;
    low | (high << 32)
}

fn write_driver_features(common_cfg: CommonCfg, features: u64) {
    common_cfg.write_u32(0x08, 0);
    common_cfg.write_u32(0x0c, features as u32);
    common_cfg.write_u32(0x08, 1);
    common_cfg.write_u32(0x0c, (features >> 32) as u32);
}

fn configure_queue<P: DevicePlatform>(
    platform: &mut P,
    common_cfg: CommonCfg,
    queue_index: u16,
    requested_size: u16,
) -> Result<VirtQueue, VirtioNetError> {
    common_cfg.write_u16(0x16, queue_index);
    let queue_size = common_cfg.read_u16(0x18);
    if queue_size < requested_size {
        return Err(VirtioNetError::QueueTooSmall);
    }
    let queue_size = requested_size;
    common_cfg.write_u16(0x18, queue_size);
    let allocation_len = virtqueue_layout_len(queue_size);
    let dma = platform.allocate_dma(
        allocation_len,
        DmaDirection::Bidirectional,
        DmaCoherency::Coherent,
        DmaConstraints {
            alignment: 4096,
            max_address_bits: 64,
            segment_boundary: u64::MAX,
            contiguous: true,
        },
    )?;
    unsafe {
        core::ptr::write_bytes(dma.cpu_virtual as *mut u8, 0, allocation_len as usize);
    }
    let desc_size = (size_of::<VirtqDesc>() * queue_size as usize) as u64;
    let avail_offset = desc_size;
    let avail_ring_offset = avail_offset + 4;
    let avail_len = 4 + (queue_size as u64 * 2) + 2;
    let used_offset = align_up(avail_offset + avail_len, 4);
    let used_ring_offset = used_offset + 4;

    common_cfg.write_u64(0x20, dma.device_address);
    common_cfg.write_u64(0x28, dma.device_address + avail_offset);
    common_cfg.write_u64(0x30, dma.device_address + used_offset);
    common_cfg.write_u16(0x1c, 1);

    Ok(VirtQueue {
        size: queue_size,
        avail_shadow: (dma.cpu_virtual + avail_ring_offset) as *mut u16,
        avail_idx_ptr: (dma.cpu_virtual + avail_offset + 2) as *mut u16,
        used_idx_ptr: (dma.cpu_virtual + used_offset + 2) as *mut u16,
        desc_ptr: dma.cpu_virtual as *mut VirtqDesc,
        used_ring_ptr: (dma.cpu_virtual + used_ring_offset) as *mut VirtqUsedElem,
        last_used_idx: 0,
    })
}

fn allocate_rx_slots<P: DevicePlatform>(
    platform: &mut P,
    queue: &mut VirtQueue,
) -> Result<Vec<RxSlot>, VirtioNetError> {
    let mut slots = Vec::with_capacity(queue.size as usize);
    for index in 0..queue.size {
        let buffer = platform.allocate_dma(
            RX_PACKET_BUFFER_SIZE,
            DmaDirection::FromDevice,
            DmaCoherency::Coherent,
            DmaConstraints::platform_default(),
        )?;
        unsafe {
            write_volatile(
                buffer.cpu_virtual as *mut VirtioNetHeader,
                VirtioNetHeader::default(),
            );
        }
        platform.prepare_dma_for_device(buffer.id)?;
        queue_submit(
            queue,
            index,
            buffer.device_address,
            RX_PACKET_BUFFER_SIZE as u32,
            VIRTQ_DESC_F_WRITE,
        )?;
        slots.push(RxSlot { buffer });
    }
    Ok(slots)
}

fn allocate_tx_slots<P: DevicePlatform>(
    platform: &mut P,
    _queue: &mut VirtQueue,
) -> Result<(Vec<TxSlot>, VecDeque<u16>), VirtioNetError> {
    let mut slots = Vec::with_capacity(TX_QUEUE_SIZE as usize);
    let mut free = VecDeque::with_capacity(TX_QUEUE_SIZE as usize);
    for index in 0..TX_QUEUE_SIZE {
        let buffer = platform.allocate_dma(
            TX_PACKET_BUFFER_SIZE,
            DmaDirection::ToDevice,
            DmaCoherency::Coherent,
            DmaConstraints::platform_default(),
        )?;
        slots.push(TxSlot {
            buffer,
            busy: false,
        });
        free.push_back(index);
    }
    Ok((slots, free))
}

fn queue_submit(
    queue: &mut VirtQueue,
    descriptor_index: u16,
    address: u64,
    len: u32,
    flags: u16,
) -> Result<(), VirtioNetError> {
    if descriptor_index >= queue.size {
        return Err(VirtioNetError::InvalidQueueState);
    }
    unsafe {
        write_volatile(
            queue.desc_ptr.add(descriptor_index as usize),
            VirtqDesc {
                addr: address,
                len,
                flags,
                next: 0,
            },
        );
        let avail_idx = read_volatile(queue.avail_idx_ptr);
        write_volatile(
            queue.avail_shadow.add((avail_idx % queue.size) as usize),
            descriptor_index,
        );
        write_volatile(queue.avail_idx_ptr, avail_idx.wrapping_add(1));
    }
    Ok(())
}

fn notify_queue(notify: NotifyCfg, common_cfg: CommonCfg, queue_index: u16) {
    common_cfg.write_u16(0x16, queue_index);
    let offset = common_cfg.read_u16(0x1e) as u32;
    let notify_ptr = unsafe {
        notify
            .base
            .add((offset.saturating_mul(notify.multiplier)) as usize) as *mut u16
    };
    unsafe {
        write_volatile(notify_ptr, queue_index);
    }
}

fn read_virtio_caps<P: DevicePlatform>(
    platform: &mut P,
    device: DeviceLocator,
) -> Result<Vec<VirtioPciCapability>, VirtioNetError> {
    let status = read_config_u16(platform, device, PCI_STATUS_OFFSET)?;
    if (status & PCI_STATUS_CAPABILITIES) == 0 {
        return Err(VirtioNetError::MissingCapability);
    }
    let mut offset = read_config_u8(platform, device, PCI_CAP_PTR_OFFSET)? as u16;
    let mut caps = Vec::new();
    let mut guard = 0usize;
    while offset >= 0x40 && guard < 64 {
        guard += 1;
        let cap_id = read_config_u8(platform, device, offset)?;
        let next = read_config_u8(platform, device, offset + 1)? as u16;
        if cap_id == PCI_CAP_ID_VENDOR {
            let cfg_type = read_config_u8(platform, device, offset + 3)?;
            let bar = read_config_u8(platform, device, offset + 4)?;
            let cap_offset = read_config_u32(platform, device, offset + 8)?;
            let length = read_config_u32(platform, device, offset + 12)?;
            let notify_multiplier = if cfg_type == VIRTIO_PCI_CAP_NOTIFY_CFG {
                read_config_u32(platform, device, offset + 16)?
            } else {
                0
            };
            caps.push(VirtioPciCapability {
                cfg_type,
                bar,
                offset: cap_offset,
                length,
                notify_multiplier,
            });
        }
        if next == 0 || next == offset {
            break;
        }
        offset = next;
    }
    Ok(caps)
}

fn read_config_u8<P: DevicePlatform>(
    platform: &mut P,
    device: DeviceLocator,
    offset: u16,
) -> Result<u8, VirtioNetError> {
    Ok(platform.read_config_u32(
        device,
        ConfigAccess {
            kind: ConfigSpaceKind::Pci,
            offset,
            width: ConfigWidth::U8,
        },
    )? as u8)
}

fn read_config_u16<P: DevicePlatform>(
    platform: &mut P,
    device: DeviceLocator,
    offset: u16,
) -> Result<u16, VirtioNetError> {
    Ok(platform.read_config_u32(
        device,
        ConfigAccess {
            kind: ConfigSpaceKind::Pci,
            offset,
            width: ConfigWidth::U16,
        },
    )? as u16)
}

fn read_config_u32<P: DevicePlatform>(
    platform: &mut P,
    device: DeviceLocator,
    offset: u16,
) -> Result<u32, VirtioNetError> {
    Ok(platform.read_config_u32(
        device,
        ConfigAccess {
            kind: ConfigSpaceKind::Pci,
            offset,
            width: ConfigWidth::U32,
        },
    )?)
}

fn virtqueue_layout_len(size: u16) -> u64 {
    let desc = size_of::<VirtqDesc>() as u64 * size as u64;
    let avail = 4 + size as u64 * 2 + 2;
    let used = 4 + size_of::<VirtqUsedElem>() as u64 * size as u64 + 2;
    align_up(desc + avail, 4) + used
}

fn align_up(value: u64, align: u64) -> u64 {
    if align == 0 {
        value
    } else {
        let rem = value % align;
        if rem == 0 {
            value
        } else {
            value + (align - rem)
        }
    }
}

impl CommonCfg {
    fn read_status(self) -> u8 {
        self.read_u8(0x14)
    }

    fn write_status(self, value: u8) {
        self.write_u8(0x14, value);
    }

    fn read_u8(self, offset: usize) -> u8 {
        unsafe { read_volatile(self.base.add(offset)) }
    }

    fn write_u8(self, offset: usize, value: u8) {
        unsafe {
            write_volatile(self.base.add(offset), value);
        }
    }

    fn read_u16(self, offset: usize) -> u16 {
        unsafe { read_volatile(self.base.add(offset) as *const u16) }
    }

    fn write_u16(self, offset: usize, value: u16) {
        unsafe {
            write_volatile(self.base.add(offset) as *mut u16, value);
        }
    }

    fn read_u32(self, offset: usize) -> u32 {
        unsafe { read_volatile(self.base.add(offset) as *const u32) }
    }

    fn write_u32(self, offset: usize, value: u32) {
        unsafe {
            write_volatile(self.base.add(offset) as *mut u32, value);
        }
    }

    fn write_u64(self, offset: usize, value: u64) {
        unsafe {
            write_volatile(self.base.add(offset) as *mut u64, value);
        }
    }
}

impl IsrCfg {
    fn read_u8(self) -> u8 {
        unsafe { read_volatile(self.base as *const u8) }
    }
}

impl DeviceCfg {
    fn read_u8(self, offset: usize) -> u8 {
        unsafe { read_volatile(self.base.add(offset) as *const u8) }
    }
}

impl VirtQueue {
    fn used_idx(&self) -> u16 {
        unsafe { read_volatile(self.used_idx_ptr) }
    }

    fn used_elem(&self, used_index: u16) -> VirtqUsedElem {
        unsafe { read_volatile(self.used_ring_ptr.add((used_index % self.size) as usize)) }
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use std::boxed::Box;

    use crate::{SyntheticPciConfigBackend, X86_64DevicePlatform, X86_64DevicePlatformConfig};

    struct TestVirtioDevice {
        mmio: Box<[u8]>,
        _dma: Box<[u8]>,
        record: DeviceRecord,
        platform: X86_64DevicePlatform<SyntheticPciConfigBackend>,
        tx_seen: Vec<Vec<u8>>,
        last_tx_avail_idx: u16,
        last_rx_avail_idx: u16,
    }

    impl TestVirtioDevice {
        fn new() -> Self {
            let mut mmio = vec![0u8; 0x4000].into_boxed_slice();
            let mut dma = vec![0u8; 0x20000].into_boxed_slice();
            let mmio_base = mmio.as_mut_ptr() as u64;
            let dma_base = dma.as_mut_ptr() as u64;

            let common_offset = 0x0000u32;
            let notify_offset = 0x1000u32;
            let isr_offset = 0x2000u32;
            let device_offset = 0x3000u32;

            let common = mmio.as_mut_ptr();
            unsafe {
                write_volatile(common.add(0x04) as *mut u32, 0x20);
                write_volatile(common.add(0x12) as *mut u16, 2);
                write_volatile(common.add(0x18) as *mut u16, 8);
                write_volatile(common.add(0x14), 0);
                write_volatile(
                    common.add(0x04) as *mut u32,
                    (VIRTIO_NET_F_MAC | VIRTIO_F_VERSION_1) as u32,
                );
                write_volatile(common.add(0x00) as *mut u32, 0);
                write_volatile(
                    common.add(0x04) as *mut u32,
                    (VIRTIO_NET_F_MAC | VIRTIO_F_VERSION_1) as u32,
                );
                write_volatile(common.add(0x00) as *mut u32, 1);
                write_volatile(
                    common.add(0x04) as *mut u32,
                    ((VIRTIO_NET_F_MAC | VIRTIO_F_VERSION_1) >> 32) as u32,
                );
                write_volatile(mmio.as_mut_ptr().add(device_offset as usize), 0x52u8);
                write_volatile(mmio.as_mut_ptr().add(device_offset as usize + 1), 0x54u8);
                write_volatile(mmio.as_mut_ptr().add(device_offset as usize + 2), 0x00u8);
                write_volatile(mmio.as_mut_ptr().add(device_offset as usize + 3), 0x12u8);
                write_volatile(mmio.as_mut_ptr().add(device_offset as usize + 4), 0x34u8);
                write_volatile(mmio.as_mut_ptr().add(device_offset as usize + 5), 0x56u8);
            }

            let mut backend = SyntheticPciConfigBackend::new();
            let address = crate::device_platform::PciAddress {
                segment: 0,
                bus: 0,
                device: 1,
                function: 0,
            };
            let identity = platform_hal::DeviceIdentity {
                vendor_id: PCI_VENDOR_VIRTIO,
                device_id: 0x1041,
                subsystem_vendor_id: PCI_VENDOR_VIRTIO,
                subsystem_device_id: 1,
                revision_id: 1,
                base_class: 0x02,
                sub_class: 0,
                programming_interface: 0,
            };
            backend.define_device(address, identity, PCI_VENDOR_VIRTIO, 1, false, 11, 1);
            backend.define_bar(address, 0, ((mmio_base as u32) & !0xf) | 0x4, 0xffff_c000);
            backend.define_bar(address, 1, (mmio_base >> 32) as u32, 0xffff_ffff);
            define_virtio_cap(
                &mut backend,
                address,
                0x50,
                VIRTIO_PCI_CAP_COMMON_CFG,
                0,
                common_offset,
                0x100,
                0,
                0x60,
            );
            define_virtio_cap(
                &mut backend,
                address,
                0x60,
                VIRTIO_PCI_CAP_NOTIFY_CFG,
                0,
                notify_offset,
                0x100,
                4,
                0x70,
            );
            define_virtio_cap(
                &mut backend,
                address,
                0x70,
                VIRTIO_PCI_CAP_ISR_CFG,
                0,
                isr_offset,
                0x20,
                0,
                0x80,
            );
            define_virtio_cap(
                &mut backend,
                address,
                0x80,
                VIRTIO_PCI_CAP_DEVICE_CFG,
                0,
                device_offset,
                0x100,
                0,
                0,
            );

            let config = X86_64DevicePlatformConfig {
                direct_map_base: 0,
                direct_map_size: u64::MAX,
                dma_window: crate::DmaWindow {
                    physical_start: dma_base,
                    len: dma.len() as u64,
                },
                interrupt_vector_base: 64,
                interrupt_vector_count: 16,
            };
            let mut platform = X86_64DevicePlatform::new(backend, config);
            let record = platform.enumerate_devices().unwrap().remove(0);
            Self {
                mmio,
                _dma: dma,
                record,
                platform,
                tx_seen: Vec::new(),
                last_tx_avail_idx: 0,
                last_rx_avail_idx: 0,
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn define_virtio_cap(
        backend: &mut SyntheticPciConfigBackend,
        address: crate::device_platform::PciAddress,
        offset: u16,
        cfg_type: u8,
        bar: u8,
        cap_offset: u32,
        length: u32,
        notify_multiplier: u32,
        next: u8,
    ) {
        backend.define_capability(
            address,
            offset,
            u32::from(PCI_CAP_ID_VENDOR) | (u32::from(cfg_type) << 24),
            next,
        );
        backend.define_config_dword(address, offset + 4, u32::from(bar));
        backend.define_config_dword(address, offset + 8, cap_offset);
        backend.define_config_dword(address, offset + 12, length);
        if cfg_type == VIRTIO_PCI_CAP_NOTIFY_CFG {
            backend.define_config_dword(address, offset + 16, notify_multiplier);
        }
    }

    fn emulate_tx(test: &mut TestVirtioDevice, driver: &VirtioNetDriver) {
        let tx_desc = driver.tx_queue.desc_ptr as u64;
        let tx_avail = tx_desc + (size_of::<VirtqDesc>() * driver.tx_queue.size as usize) as u64;
        let tx_used = align_up(tx_avail + 4 + (driver.tx_queue.size as u64 * 2) + 2, 4);
        let avail_idx = unsafe { read_volatile((tx_avail + 2) as *const u16) };
        while test.last_tx_avail_idx != avail_idx {
            let ring_slot = test.last_tx_avail_idx % TX_QUEUE_SIZE;
            let desc_index =
                unsafe { read_volatile((tx_avail + 4 + u64::from(ring_slot) * 2) as *const u16) };
            let desc =
                unsafe { read_volatile((tx_desc as *const VirtqDesc).add(desc_index as usize)) };
            let payload_len = desc.len as usize - size_of::<VirtioNetHeader>();
            let mut payload = vec![0u8; payload_len];
            unsafe {
                core::ptr::copy_nonoverlapping(
                    (desc.addr + size_of::<VirtioNetHeader>() as u64) as *const u8,
                    payload.as_mut_ptr(),
                    payload_len,
                );
            }
            test.tx_seen.push(payload);
            let used_idx_ptr = (tx_used + 2) as *mut u16;
            let used_idx = unsafe { read_volatile(used_idx_ptr) };
            unsafe {
                write_volatile(
                    (tx_used
                        + 4
                        + u64::from(used_idx % TX_QUEUE_SIZE) * size_of::<VirtqUsedElem>() as u64)
                        as *mut VirtqUsedElem,
                    VirtqUsedElem {
                        id: desc_index as u32,
                        len: desc.len,
                    },
                );
                write_volatile(used_idx_ptr, used_idx.wrapping_add(1));
                write_volatile(test.mmio.as_mut_ptr().add(0x2000), VIRTIO_ISR_QUEUE);
            }
            test.last_tx_avail_idx = test.last_tx_avail_idx.wrapping_add(1);
        }
    }

    fn emulate_rx(test: &mut TestVirtioDevice, driver: &VirtioNetDriver, payload: &[u8]) {
        let rx_desc = driver.rx_queue.desc_ptr as u64;
        let rx_avail = rx_desc + (size_of::<VirtqDesc>() * driver.rx_queue.size as usize) as u64;
        let rx_used = align_up(rx_avail + 4 + (driver.rx_queue.size as u64 * 2) + 2, 4);
        let avail_idx = unsafe { read_volatile((rx_avail + 2) as *const u16) };
        if test.last_rx_avail_idx == avail_idx {
            panic!("no rx descriptors available");
        }
        let ring_slot = test.last_rx_avail_idx % RX_QUEUE_SIZE;
        let desc_index =
            unsafe { read_volatile((rx_avail + 4 + u64::from(ring_slot) * 2) as *const u16) };
        let desc = unsafe { read_volatile((rx_desc as *const VirtqDesc).add(desc_index as usize)) };
        unsafe {
            write_volatile(
                desc.addr as *mut VirtioNetHeader,
                VirtioNetHeader::default(),
            );
            core::ptr::copy_nonoverlapping(
                payload.as_ptr(),
                (desc.addr + size_of::<VirtioNetHeader>() as u64) as *mut u8,
                payload.len(),
            );
        }
        let used_idx_ptr = (rx_used + 2) as *mut u16;
        let used_idx = unsafe { read_volatile(used_idx_ptr) };
        unsafe {
            write_volatile(
                (rx_used
                    + 4
                    + u64::from(used_idx % RX_QUEUE_SIZE) * size_of::<VirtqUsedElem>() as u64)
                    as *mut VirtqUsedElem,
                VirtqUsedElem {
                    id: desc_index as u32,
                    len: (size_of::<VirtioNetHeader>() + payload.len()) as u32,
                },
            );
            write_volatile(used_idx_ptr, used_idx.wrapping_add(1));
            write_volatile(test.mmio.as_mut_ptr().add(0x2000), VIRTIO_ISR_QUEUE);
        }
        test.last_rx_avail_idx = test.last_rx_avail_idx.wrapping_add(1);
    }

    #[test]
    fn virtio_net_initializes_over_device_platform_and_reads_mac() {
        let mut test = TestVirtioDevice::new();
        let driver = VirtioNetDriver::initialize(&mut test.platform, &test.record).unwrap();
        assert_eq!(driver.mapping().virtual_base, test.mmio.as_ptr() as u64);
        assert_eq!(
            &test.mmio[0x3000..0x3006],
            &[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
        );
        assert_eq!(
            driver.device_cfg.base as u64,
            test.mmio.as_ptr() as u64 + 0x3000
        );
        assert_eq!(
            unsafe { read_volatile(driver.device_cfg.base as *const u8) },
            0x52
        );
        assert_eq!(driver.device(), test.record.locator);
        assert_eq!(driver.mac_address(), [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
        assert_eq!(
            driver.negotiated_mac_config(),
            [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
        );
    }

    #[test]
    fn virtio_net_transmits_and_recycles_tx_buffers_via_interrupts() {
        let mut test = TestVirtioDevice::new();
        let mut driver = VirtioNetDriver::initialize(&mut test.platform, &test.record).unwrap();
        driver.send(&mut test.platform, b"packet-one").unwrap();
        driver.send(&mut test.platform, b"packet-two").unwrap();
        emulate_tx(&mut test, &driver);
        let handle = driver.interrupt_handle();
        let route = test.platform.claim_interrupt(test.record.locator, 0);
        assert!(route.is_err());
        let claim = test
            .platform
            .dispatch_interrupt_vector(driver.interrupt_route().vector)
            .unwrap()
            .unwrap();
        assert_eq!(claim.handle, handle);
        let summary = driver.service_interrupt(&mut test.platform).unwrap();
        assert_eq!(summary.tx_completed, 2);
        assert_eq!(
            test.tx_seen,
            vec![b"packet-one".to_vec(), b"packet-two".to_vec()]
        );
        for index in 0..TX_QUEUE_SIZE as usize {
            assert!(!driver.tx_slots[index].busy);
        }
    }

    #[test]
    fn virtio_net_receives_frames_and_requeues_rx_buffers() {
        let mut test = TestVirtioDevice::new();
        let mut driver = VirtioNetDriver::initialize(&mut test.platform, &test.record).unwrap();
        emulate_rx(&mut test, &driver, b"hello-virtio");
        let _ = test
            .platform
            .dispatch_interrupt_vector(driver.interrupt_route().vector)
            .unwrap()
            .unwrap();
        let summary = driver.service_interrupt(&mut test.platform).unwrap();
        assert_eq!(summary.rx_completed, 1);
        let frames = driver.receive(&mut test.platform).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].payload, b"hello-virtio".to_vec());
        emulate_rx(&mut test, &driver, b"second-frame");
        let _ = test
            .platform
            .dispatch_interrupt_vector(driver.interrupt_route().vector)
            .unwrap()
            .unwrap();
        let _ = driver.service_interrupt(&mut test.platform).unwrap();
        let frames = driver.receive(&mut test.platform).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].payload, b"second-frame".to_vec());
    }
}
