//! Canonical subsystem role:
//! - subsystem: x86_64 virtio block transport mediation
//! - owner layer: platform mediation
//! - semantic owner: `platform-x86_64`
//! - truth path role: platform-specific virtio block hardware mechanics for
//!   the real x86 path
//!
//! Canonical contract families handled here:
//! - virtio block transport contracts
//! - MMIO/config-space mediation contracts
//! - block queue and interrupt mechanism contracts
//!
//! This module may mediate virtio block hardware behavior, but it must not
//! redefine higher-level storage or device-runtime truth owned by `kernel-core`.

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

const VIRTIO_F_VERSION_1: u64 = 1 << 32;
const VIRTIO_BLK_F_RO: u64 = 1 << 5;
const VIRTIO_BLK_F_BLK_SIZE: u64 = 1 << 6;

const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
const VIRTIO_STATUS_FAILED: u8 = 128;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTIO_ISR_QUEUE: u8 = 1;

const REQUEST_QUEUE_INDEX: u16 = 0;
const REQUEST_QUEUE_SIZE: u16 = 8;
const REQUEST_SLOT_COUNT: usize = 2;
const REQUEST_DESCRIPTOR_STRIDE: u16 = 3;
const SECTOR_SIZE: usize = 512;

const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;
const VIRTIO_BLK_S_OK: u8 = 0;

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
struct VirtioBlkReqHeader {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtioBlkPciMatch {
    pub device: DeviceLocator,
    pub subsystem_vendor_id: u16,
    pub subsystem_device_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VirtioBlkInterruptSummary {
    pub isr_status: u8,
    pub completed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioBlkError {
    Hal(HalError),
    DeviceNotVirtioBlk,
    MissingBar,
    MissingCapability,
    QueueUnavailable,
    QueueTooSmall,
    FeaturesRejected,
    InvalidQueueState,
    InvalidRequest,
    ReadOnly,
    IoFailed,
}

impl From<HalError> for VirtioBlkError {
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
struct RequestSlot {
    dma: DmaBufferInfo,
    in_use: bool,
    data_len: usize,
}

pub struct VirtioBlkDriver {
    #[allow(dead_code)]
    device: DeviceLocator,
    match_info: VirtioBlkPciMatch,
    mapping: MmioMapping,
    interrupt_handle: InterruptHandle,
    interrupt_route: InterruptRoute,
    common_cfg: CommonCfg,
    notify_cfg: NotifyCfg,
    isr_cfg: IsrCfg,
    _device_cfg: DeviceCfg,
    request_queue: VirtQueue,
    request_slots: [RequestSlot; REQUEST_SLOT_COUNT],
    capacity_sectors: u64,
    block_size: u32,
    read_only: bool,
}

impl VirtioBlkDriver {
    pub fn probe<P: DevicePlatform>(
        platform: &mut P,
        record: &DeviceRecord,
    ) -> Result<Option<VirtioBlkPciMatch>, VirtioBlkError> {
        if record.identity.vendor_id != PCI_VENDOR_VIRTIO || record.identity.base_class != 0x01 {
            return Ok(None);
        }
        let subsystem_vendor =
            read_config_u16(platform, record.locator, PCI_SUBSYSTEM_VENDOR_OFFSET)?;
        let subsystem_device =
            read_config_u16(platform, record.locator, PCI_SUBSYSTEM_DEVICE_OFFSET)?;
        if subsystem_vendor != PCI_VENDOR_VIRTIO {
            return Ok(None);
        }
        Ok(Some(VirtioBlkPciMatch {
            device: record.locator,
            subsystem_vendor_id: subsystem_vendor,
            subsystem_device_id: subsystem_device,
        }))
    }

    pub fn initialize<P: DevicePlatform>(
        platform: &mut P,
        record: &DeviceRecord,
    ) -> Result<Self, VirtioBlkError> {
        let Some(match_info) = Self::probe(platform, record)? else {
            return Err(VirtioBlkError::DeviceNotVirtioBlk);
        };

        let caps = read_virtio_caps(platform, record.locator)?;
        let common_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_COMMON_CFG)
            .copied()
            .ok_or(VirtioBlkError::MissingCapability)?;
        let notify_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_NOTIFY_CFG)
            .copied()
            .ok_or(VirtioBlkError::MissingCapability)?;
        let isr_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_ISR_CFG)
            .copied()
            .ok_or(VirtioBlkError::MissingCapability)?;
        let device_cap = caps
            .iter()
            .find(|cap| cap.cfg_type == VIRTIO_PCI_CAP_DEVICE_CFG)
            .copied()
            .ok_or(VirtioBlkError::MissingCapability)?;

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
            return Err(VirtioBlkError::FeaturesRejected);
        }

        let (interrupt_handle, interrupt_route) = platform.claim_interrupt(record.locator, 0)?;
        platform.enable_interrupt(interrupt_handle)?;

        let request_queue = configure_queue(
            platform,
            common_cfg,
            REQUEST_QUEUE_INDEX,
            REQUEST_QUEUE_SIZE,
        )?;

        let capacity_sectors = device_cfg.read_u64(0x00);
        let block_size = if (negotiated & VIRTIO_BLK_F_BLK_SIZE) != 0 {
            device_cfg.read_u32(0x14)
        } else {
            SECTOR_SIZE as u32
        };
        let read_only = (negotiated & VIRTIO_BLK_F_RO) != 0;

        let request_slots = allocate_request_slots(platform)?;

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
            _device_cfg: device_cfg,
            request_queue,
            request_slots,
            capacity_sectors,
            block_size,
            read_only,
        })
    }

    pub fn interrupt_route(&self) -> InterruptRoute {
        self.interrupt_route
    }

    pub fn interrupt_handle(&self) -> InterruptHandle {
        self.interrupt_handle
    }

    pub fn mapping(&self) -> MmioMapping {
        self.mapping
    }

    pub fn pci_match(&self) -> VirtioBlkPciMatch {
        self.match_info
    }

    pub fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }

    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn read_sector<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        sector: u64,
    ) -> Result<[u8; SECTOR_SIZE], VirtioBlkError> {
        let slot_index = self.submit_request(platform, VIRTIO_BLK_T_IN, sector, None)?;
        let completed = self.complete_request(platform, slot_index)?;
        Ok(completed)
    }

    pub fn write_sector<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        sector: u64,
        payload: &[u8; SECTOR_SIZE],
    ) -> Result<(), VirtioBlkError> {
        if self.read_only {
            return Err(VirtioBlkError::ReadOnly);
        }
        let slot_index = self.submit_request(platform, VIRTIO_BLK_T_OUT, sector, Some(payload))?;
        let _ = self.complete_request(platform, slot_index)?;
        Ok(())
    }

    pub fn service_interrupt<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
    ) -> Result<VirtioBlkInterruptSummary, VirtioBlkError> {
        platform.acknowledge_interrupt(self.interrupt_handle)?;
        let isr_status = self.isr_cfg.read_u8();
        let mut summary = VirtioBlkInterruptSummary {
            isr_status,
            ..VirtioBlkInterruptSummary::default()
        };
        if (isr_status & VIRTIO_ISR_QUEUE) != 0 {
            let used_idx = self.request_queue.used_idx();
            summary.completed = used_idx.wrapping_sub(self.request_queue.last_used_idx) as usize;
        }
        Ok(summary)
    }

    fn submit_request<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        request_type: u32,
        sector: u64,
        payload: Option<&[u8; SECTOR_SIZE]>,
    ) -> Result<usize, VirtioBlkError> {
        let Some(slot_index) = self.request_slots.iter().position(|slot| !slot.in_use) else {
            return Err(VirtioBlkError::InvalidQueueState);
        };
        let slot = &mut self.request_slots[slot_index];
        let header_ptr = slot.dma.cpu_virtual as *mut VirtioBlkReqHeader;
        let data_ptr = (slot.dma.cpu_virtual + size_of::<VirtioBlkReqHeader>() as u64) as *mut u8;
        let status_ptr = unsafe { data_ptr.add(SECTOR_SIZE) };

        unsafe {
            write_volatile(
                header_ptr,
                VirtioBlkReqHeader {
                    request_type,
                    reserved: 0,
                    sector,
                },
            );
            if let Some(bytes) = payload {
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), data_ptr, SECTOR_SIZE);
            } else {
                core::ptr::write_bytes(data_ptr, 0, SECTOR_SIZE);
            }
            write_volatile(status_ptr, 0xff);
        }

        platform.prepare_dma_for_device(slot.dma.id)?;
        let descriptor_base = (slot_index as u16) * REQUEST_DESCRIPTOR_STRIDE;
        unsafe {
            write_volatile(
                self.request_queue.desc_ptr.add(descriptor_base as usize),
                VirtqDesc {
                    addr: slot.dma.device_address,
                    len: size_of::<VirtioBlkReqHeader>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: descriptor_base + 1,
                },
            );
            write_volatile(
                self.request_queue
                    .desc_ptr
                    .add((descriptor_base + 1) as usize),
                VirtqDesc {
                    addr: slot.dma.device_address + size_of::<VirtioBlkReqHeader>() as u64,
                    len: SECTOR_SIZE as u32,
                    flags: if request_type == VIRTIO_BLK_T_IN {
                        VIRTQ_DESC_F_NEXT | VIRTQ_DESC_F_WRITE
                    } else {
                        VIRTQ_DESC_F_NEXT
                    },
                    next: descriptor_base + 2,
                },
            );
            write_volatile(
                self.request_queue
                    .desc_ptr
                    .add((descriptor_base + 2) as usize),
                VirtqDesc {
                    addr: slot.dma.device_address
                        + size_of::<VirtioBlkReqHeader>() as u64
                        + SECTOR_SIZE as u64,
                    len: 1,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0,
                },
            );

            let avail_idx = read_volatile(self.request_queue.avail_idx_ptr);
            write_volatile(
                self.request_queue
                    .avail_shadow
                    .add((avail_idx % self.request_queue.size) as usize),
                descriptor_base,
            );
            write_volatile(self.request_queue.avail_idx_ptr, avail_idx.wrapping_add(1));
        }
        slot.in_use = true;
        slot.data_len = SECTOR_SIZE;
        notify_queue(self.notify_cfg, self.common_cfg, REQUEST_QUEUE_INDEX);
        Ok(slot_index)
    }

    fn complete_request<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        slot_index: usize,
    ) -> Result<[u8; SECTOR_SIZE], VirtioBlkError> {
        while self.request_queue.last_used_idx == self.request_queue.used_idx() {
            core::hint::spin_loop();
        }
        let elem = self
            .request_queue
            .used_elem(self.request_queue.last_used_idx);
        self.request_queue.last_used_idx = self.request_queue.last_used_idx.wrapping_add(1);
        let expected = (slot_index as u32) * (REQUEST_DESCRIPTOR_STRIDE as u32);
        if elem.id != expected {
            return Err(VirtioBlkError::InvalidRequest);
        }

        let slot = &mut self.request_slots[slot_index];
        platform.complete_dma_from_device(slot.dma.id)?;
        let data_ptr = (slot.dma.cpu_virtual + size_of::<VirtioBlkReqHeader>() as u64) as *const u8;
        let status_ptr = unsafe { data_ptr.add(SECTOR_SIZE) };
        let status = unsafe { read_volatile(status_ptr) };
        if status != VIRTIO_BLK_S_OK {
            slot.in_use = false;
            return Err(VirtioBlkError::IoFailed);
        }
        let mut sector = [0u8; SECTOR_SIZE];
        unsafe {
            core::ptr::copy_nonoverlapping(data_ptr, sector.as_mut_ptr(), SECTOR_SIZE);
        }
        slot.in_use = false;
        Ok(sector)
    }
}

fn validate_bar_cap(
    bar: &platform_hal::BarInfo,
    cap: VirtioPciCapability,
) -> Result<(), VirtioBlkError> {
    if cap.offset as u64 + cap.length as u64 > bar.size {
        return Err(VirtioBlkError::MissingCapability);
    }
    Ok(())
}

fn select_capability_bar<P: DevicePlatform>(
    platform: &mut P,
    record: &DeviceRecord,
    cap: VirtioPciCapability,
) -> Result<platform_hal::BarInfo, VirtioBlkError> {
    let offset = PCI_BAR0_OFFSET + u16::from(cap.bar) * 4;
    let low = read_config_u32(platform, record.locator, offset)?;
    if (low & 0x1) != 0 {
        return Err(VirtioBlkError::MissingBar);
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
        return Err(VirtioBlkError::MissingBar);
    };
    record
        .bars
        .iter()
        .find(|bar| bar.kind == kind && bar.base == base)
        .copied()
        .ok_or(VirtioBlkError::MissingBar)
}

fn negotiate_features(device_features: u64) -> Result<u64, VirtioBlkError> {
    let required = VIRTIO_F_VERSION_1;
    if (device_features & required) != required {
        return Err(VirtioBlkError::FeaturesRejected);
    }
    let optional = device_features & (VIRTIO_BLK_F_RO | VIRTIO_BLK_F_BLK_SIZE);
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
) -> Result<VirtQueue, VirtioBlkError> {
    common_cfg.write_u16(0x16, queue_index);
    let queue_size = common_cfg.read_u16(0x18);
    if queue_size < requested_size {
        return Err(VirtioBlkError::QueueTooSmall);
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

fn allocate_request_slots<P: DevicePlatform>(
    platform: &mut P,
) -> Result<[RequestSlot; REQUEST_SLOT_COUNT], VirtioBlkError> {
    let mut slots = Vec::with_capacity(REQUEST_SLOT_COUNT);
    for _ in 0..REQUEST_SLOT_COUNT {
        let dma = platform.allocate_dma(
            (size_of::<VirtioBlkReqHeader>() + SECTOR_SIZE + 1) as u64,
            DmaDirection::Bidirectional,
            DmaCoherency::Coherent,
            DmaConstraints::platform_default(),
        )?;
        slots.push(RequestSlot {
            dma,
            in_use: false,
            data_len: 0,
        });
    }
    slots
        .try_into()
        .map_err(|_| VirtioBlkError::InvalidQueueState)
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
) -> Result<Vec<VirtioPciCapability>, VirtioBlkError> {
    let status = read_config_u16(platform, device, PCI_STATUS_OFFSET)?;
    if (status & PCI_STATUS_CAPABILITIES) == 0 {
        return Err(VirtioBlkError::MissingCapability);
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
) -> Result<u8, VirtioBlkError> {
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
) -> Result<u16, VirtioBlkError> {
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
) -> Result<u32, VirtioBlkError> {
    Ok(platform.read_config_u32(
        device,
        ConfigAccess {
            kind: ConfigSpaceKind::Pci,
            offset,
            width: ConfigWidth::U32,
        },
    )?)
}

fn align_up(value: u64, align: u64) -> u64 {
    if align == 0 {
        value
    } else {
        let remainder = value % align;
        if remainder == 0 {
            value
        } else {
            value + (align - remainder)
        }
    }
}

fn virtqueue_layout_len(queue_size: u16) -> u64 {
    let desc_len = (size_of::<VirtqDesc>() * queue_size as usize) as u64;
    let avail_len = 4 + (queue_size as u64 * 2) + 2;
    let used_offset = align_up(desc_len + avail_len, 4);
    let used_len = 4 + (queue_size as u64 * size_of::<VirtqUsedElem>() as u64) + 2;
    align_up(used_offset + used_len, 4096)
}

impl VirtQueue {
    fn used_idx(&self) -> u16 {
        unsafe { read_volatile(self.used_idx_ptr) }
    }

    fn used_elem(&self, used_index: u16) -> VirtqUsedElem {
        unsafe { read_volatile(self.used_ring_ptr.add((used_index % self.size) as usize)) }
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
        unsafe { write_volatile(self.base.add(offset), value) };
    }

    fn read_u16(self, offset: usize) -> u16 {
        unsafe { read_volatile(self.base.add(offset) as *const u16) }
    }

    fn write_u16(self, offset: usize, value: u16) {
        unsafe { write_volatile(self.base.add(offset) as *mut u16, value) };
    }

    fn read_u32(self, offset: usize) -> u32 {
        unsafe { read_volatile(self.base.add(offset) as *const u32) }
    }

    fn write_u32(self, offset: usize, value: u32) {
        unsafe { write_volatile(self.base.add(offset) as *mut u32, value) };
    }

    fn write_u64(self, offset: usize, value: u64) {
        unsafe { write_volatile(self.base.add(offset) as *mut u64, value) };
    }
}

impl IsrCfg {
    fn read_u8(self) -> u8 {
        unsafe { read_volatile(self.base) }
    }
}

impl DeviceCfg {
    fn read_u32(self, offset: usize) -> u32 {
        unsafe { read_volatile(self.base.add(offset) as *const u32) }
    }

    fn read_u64(self, offset: usize) -> u64 {
        unsafe { read_volatile(self.base.add(offset) as *const u64) }
    }
}
