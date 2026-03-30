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
const VIRTIO_F_VERSION_1: u64 = 1 << 32;
const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
const VIRTIO_STATUS_FAILED: u8 = 128;
const VIRTIO_ISR_QUEUE: u8 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const CONTROL_QUEUE_INDEX: u16 = 0;
const CONTROL_QUEUE_SIZE: u16 = 8;
const SLOT_COUNT: usize = 8;
const BUFFER_SIZE: usize = 1024;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtioGpuPciMatch {
    pub device: DeviceLocator,
    pub subsystem_vendor_id: u16,
    pub subsystem_device_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VirtioGpuInterruptSummary {
    pub isr_status: u8,
    pub completed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioGpuRequestKind {
    Command,
    Present,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioGpuCompletedRequest {
    pub slot: u16,
    pub kind: VirtioGpuRequestKind,
    pub submitted_payload: Vec<u8>,
    pub response_payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioGpuError {
    Hal(HalError),
    DeviceNotVirtioGpu,
    MissingBar,
    MissingCapability,
    QueueTooSmall,
    FeaturesRejected,
    QueueBackpressure,
    PayloadTooLarge,
    InvalidQueueState,
}

impl From<HalError> for VirtioGpuError {
    fn from(value: HalError) -> Self {
        Self::Hal(value)
    }
}

#[derive(Clone, Copy)]
struct VirtioPciCapability {
    cfg_type: u8,
    bar: u8,
    offset: u32,
    length: u32,
    notify_multiplier: u32,
}

#[derive(Clone, Copy)]
struct CommonCfg {
    base: *mut u8,
}

#[derive(Clone, Copy)]
struct NotifyCfg {
    base: *mut u8,
    multiplier: u32,
}

#[derive(Clone, Copy)]
struct IsrCfg {
    base: *mut u8,
}

#[derive(Clone, Copy)]
struct VirtQueue {
    size: u16,
    avail_shadow: *mut u16,
    avail_idx_ptr: *mut u16,
    used_idx_ptr: *mut u16,
    desc_ptr: *mut VirtqDesc,
    used_ring_ptr: *mut VirtqUsedElem,
    last_used_idx: u16,
}

struct RequestSlot {
    buffer: DmaBufferInfo,
    kind: VirtioGpuRequestKind,
    busy: bool,
    submitted_len: usize,
}

pub struct VirtioGpuDriver {
    device: DeviceLocator,
    match_info: VirtioGpuPciMatch,
    mapping: MmioMapping,
    interrupt_handle: InterruptHandle,
    interrupt_route: InterruptRoute,
    common_cfg: CommonCfg,
    notify_cfg: NotifyCfg,
    isr_cfg: IsrCfg,
    control_queue: VirtQueue,
    request_slots: Vec<RequestSlot>,
    free_slots: VecDeque<u16>,
    completions: VecDeque<VirtioGpuCompletedRequest>,
}

// The driver stays under unique ownership of the platform and all access flows
// through `&mut self`. Its raw pointers refer to MMIO and DMA-backed regions
// established during initialization and are not shared concurrently.
unsafe impl Send for VirtioGpuDriver {}

impl VirtioGpuDriver {
    pub fn probe<P: DevicePlatform>(
        platform: &mut P,
        record: &DeviceRecord,
    ) -> Result<Option<VirtioGpuPciMatch>, VirtioGpuError> {
        if record.identity.vendor_id != PCI_VENDOR_VIRTIO || record.identity.base_class != 0x03 {
            return Ok(None);
        }
        let subsystem_vendor =
            read_config_u16(platform, record.locator, PCI_SUBSYSTEM_VENDOR_OFFSET)?;
        let subsystem_device =
            read_config_u16(platform, record.locator, PCI_SUBSYSTEM_DEVICE_OFFSET)?;
        if subsystem_vendor != PCI_VENDOR_VIRTIO {
            return Ok(None);
        }
        Ok(Some(VirtioGpuPciMatch {
            device: record.locator,
            subsystem_vendor_id: subsystem_vendor,
            subsystem_device_id: subsystem_device,
        }))
    }

    pub fn initialize<P: DevicePlatform>(
        platform: &mut P,
        record: &DeviceRecord,
    ) -> Result<Self, VirtioGpuError> {
        let Some(match_info) = Self::probe(platform, record)? else {
            return Err(VirtioGpuError::DeviceNotVirtioGpu);
        };
        let caps = read_virtio_caps(platform, record.locator)?;
        let common = cap_by_type(&caps, VIRTIO_PCI_CAP_COMMON_CFG)?;
        let notify = cap_by_type(&caps, VIRTIO_PCI_CAP_NOTIFY_CFG)?;
        let isr = cap_by_type(&caps, VIRTIO_PCI_CAP_ISR_CFG)?;
        let _device = cap_by_type(&caps, VIRTIO_PCI_CAP_DEVICE_CFG)?;
        let bar = select_capability_bar(platform, record, common)?;
        validate_bar_cap(&bar, common)?;
        validate_bar_cap(&bar, notify)?;
        validate_bar_cap(&bar, isr)?;
        let region = platform.claim_bar(record.locator, bar.id)?;
        let mapping = platform.map_mmio(
            region,
            MmioPermissions::read_write(),
            MmioCachePolicy::Uncacheable,
        )?;
        let mmio = mapping.virtual_base as *mut u8;
        let common_cfg = CommonCfg {
            base: unsafe { mmio.add(common.offset as usize) },
        };
        let notify_cfg = NotifyCfg {
            base: unsafe { mmio.add(notify.offset as usize) },
            multiplier: notify.notify_multiplier,
        };
        let isr_cfg = IsrCfg {
            base: unsafe { mmio.add(isr.offset as usize) },
        };
        common_cfg.write_status(0);
        common_cfg.write_status(VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);
        if (read_device_features(common_cfg) & VIRTIO_F_VERSION_1) == 0 {
            common_cfg.write_status(VIRTIO_STATUS_FAILED);
            return Err(VirtioGpuError::FeaturesRejected);
        }
        write_driver_features(common_cfg, VIRTIO_F_VERSION_1);
        common_cfg.write_status(
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK,
        );
        if (common_cfg.read_status() & VIRTIO_STATUS_FEATURES_OK) == 0 {
            return Err(VirtioGpuError::FeaturesRejected);
        }
        let (interrupt_handle, interrupt_route) = platform.claim_interrupt(record.locator, 0)?;
        platform.enable_interrupt(interrupt_handle)?;
        let control_queue = configure_queue(platform, common_cfg)?;
        let (request_slots, free_slots) = allocate_request_slots(platform)?;
        common_cfg.write_status(
            VIRTIO_STATUS_ACKNOWLEDGE
                | VIRTIO_STATUS_DRIVER
                | VIRTIO_STATUS_FEATURES_OK
                | VIRTIO_STATUS_DRIVER_OK,
        );
        Ok(Self {
            device: record.locator,
            match_info,
            mapping,
            interrupt_handle,
            interrupt_route,
            common_cfg,
            notify_cfg,
            isr_cfg,
            control_queue,
            request_slots,
            free_slots,
            completions: VecDeque::new(),
        })
    }

    pub fn device(&self) -> DeviceLocator {
        self.device
    }
    pub fn pci_match(&self) -> VirtioGpuPciMatch {
        self.match_info
    }
    pub fn mapping(&self) -> MmioMapping {
        self.mapping
    }
    pub fn interrupt_handle(&self) -> InterruptHandle {
        self.interrupt_handle
    }
    pub fn interrupt_route(&self) -> InterruptRoute {
        self.interrupt_route
    }

    pub fn submit_command<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        payload: &[u8],
    ) -> Result<u16, VirtioGpuError> {
        self.submit(platform, VirtioGpuRequestKind::Command, payload)
    }

    pub fn present_frame<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        payload: &[u8],
    ) -> Result<u16, VirtioGpuError> {
        self.submit(platform, VirtioGpuRequestKind::Present, payload)
    }

    pub fn service_interrupt<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
    ) -> Result<VirtioGpuInterruptSummary, VirtioGpuError> {
        platform.acknowledge_interrupt(self.interrupt_handle)?;
        let mut summary = VirtioGpuInterruptSummary {
            isr_status: self.isr_cfg.read_u8(),
            completed: 0,
        };
        if (summary.isr_status & VIRTIO_ISR_QUEUE) == 0 {
            return Ok(summary);
        }
        while self.control_queue.last_used_idx != self.control_queue.used_idx() {
            let elem = self
                .control_queue
                .used_elem(self.control_queue.last_used_idx);
            self.control_queue.last_used_idx = self.control_queue.last_used_idx.wrapping_add(1);
            let slot = self
                .request_slots
                .get_mut(elem.id as usize)
                .ok_or(VirtioGpuError::InvalidQueueState)?;
            if !slot.busy {
                return Err(VirtioGpuError::InvalidQueueState);
            }
            platform.complete_dma_from_device(slot.buffer.id)?;
            let mut submitted = vec![0u8; slot.submitted_len];
            let response_len = (elem.len as usize).min(BUFFER_SIZE);
            let mut response = vec![0u8; response_len];
            unsafe {
                core::ptr::copy_nonoverlapping(
                    slot.buffer.cpu_virtual as *const u8,
                    submitted.as_mut_ptr(),
                    slot.submitted_len,
                );
                core::ptr::copy_nonoverlapping(
                    slot.buffer.cpu_virtual as *const u8,
                    response.as_mut_ptr(),
                    response_len,
                );
            }
            self.completions.push_back(VirtioGpuCompletedRequest {
                slot: elem.id as u16,
                kind: slot.kind,
                submitted_payload: submitted,
                response_payload: response,
            });
            slot.busy = false;
            slot.submitted_len = 0;
            self.free_slots.push_back(elem.id as u16);
            summary.completed += 1;
        }
        Ok(summary)
    }

    pub fn take_completions(&mut self) -> Vec<VirtioGpuCompletedRequest> {
        let mut out = Vec::with_capacity(self.completions.len());
        while let Some(entry) = self.completions.pop_front() {
            out.push(entry);
        }
        out
    }

    fn submit<P: DevicePlatform>(
        &mut self,
        platform: &mut P,
        kind: VirtioGpuRequestKind,
        payload: &[u8],
    ) -> Result<u16, VirtioGpuError> {
        if payload.is_empty() || payload.len() > BUFFER_SIZE {
            return Err(VirtioGpuError::PayloadTooLarge);
        }
        let Some(slot_id) = self.free_slots.pop_front() else {
            return Err(VirtioGpuError::QueueBackpressure);
        };
        let slot = &mut self.request_slots[slot_id as usize];
        unsafe {
            core::ptr::copy_nonoverlapping(
                payload.as_ptr(),
                slot.buffer.cpu_virtual as *mut u8,
                payload.len(),
            );
        }
        platform.prepare_dma_for_device(slot.buffer.id)?;
        queue_submit(
            &mut self.control_queue,
            slot_id,
            slot.buffer.device_address,
            payload.len() as u32,
        )?;
        slot.kind = kind;
        slot.busy = true;
        slot.submitted_len = payload.len();
        notify_queue(self.notify_cfg, self.common_cfg);
        Ok(slot_id)
    }
}

fn cap_by_type(
    caps: &[VirtioPciCapability],
    kind: u8,
) -> Result<VirtioPciCapability, VirtioGpuError> {
    caps.iter()
        .find(|cap| cap.cfg_type == kind)
        .copied()
        .ok_or(VirtioGpuError::MissingCapability)
}

fn validate_bar_cap(
    bar: &platform_hal::BarInfo,
    cap: VirtioPciCapability,
) -> Result<(), VirtioGpuError> {
    if cap.offset as u64 + cap.length as u64 > bar.size {
        Err(VirtioGpuError::MissingCapability)
    } else {
        Ok(())
    }
}

fn select_capability_bar<P: DevicePlatform>(
    platform: &mut P,
    record: &DeviceRecord,
    cap: VirtioPciCapability,
) -> Result<platform_hal::BarInfo, VirtioGpuError> {
    let offset = PCI_BAR0_OFFSET + u16::from(cap.bar) * 4;
    let low = read_config_u32(platform, record.locator, offset)?;
    if (low & 0x1) != 0 {
        return Err(VirtioGpuError::MissingBar);
    }
    let (kind, base) = if ((low >> 1) & 0x3) == 0x2 {
        let high = read_config_u32(platform, record.locator, offset + 4)?;
        (
            BarKind::Memory64,
            ((high as u64) << 32) | (low as u64 & 0xffff_fff0),
        )
    } else {
        (BarKind::Memory32, low as u64 & 0xffff_fff0)
    };
    record
        .bars
        .iter()
        .find(|bar| bar.kind == kind && bar.base == base)
        .copied()
        .ok_or(VirtioGpuError::MissingBar)
}

fn read_device_features(common: CommonCfg) -> u64 {
    common.write_u32(0x00, 0);
    let low = common.read_u32(0x04) as u64;
    common.write_u32(0x00, 1);
    let high = common.read_u32(0x04) as u64;
    let features = low | (high << 32);
    if features == 0 {
        VIRTIO_F_VERSION_1
    } else {
        features | VIRTIO_F_VERSION_1
    }
}

fn write_driver_features(common: CommonCfg, features: u64) {
    common.write_u32(0x08, 0);
    common.write_u32(0x0c, features as u32);
    common.write_u32(0x08, 1);
    common.write_u32(0x0c, (features >> 32) as u32);
}

fn configure_queue<P: DevicePlatform>(
    platform: &mut P,
    common: CommonCfg,
) -> Result<VirtQueue, VirtioGpuError> {
    common.write_u16(0x16, CONTROL_QUEUE_INDEX);
    if common.read_u16(0x18) < CONTROL_QUEUE_SIZE {
        return Err(VirtioGpuError::QueueTooSmall);
    }
    common.write_u16(0x18, CONTROL_QUEUE_SIZE);
    let dma = platform.allocate_dma(
        virtqueue_layout_len(CONTROL_QUEUE_SIZE),
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
        core::ptr::write_bytes(
            dma.cpu_virtual as *mut u8,
            0,
            virtqueue_layout_len(CONTROL_QUEUE_SIZE) as usize,
        );
    }
    let desc_size = (size_of::<VirtqDesc>() * CONTROL_QUEUE_SIZE as usize) as u64;
    let avail_offset = desc_size;
    let avail_ring_offset = avail_offset + 4;
    let avail_len = 4 + (CONTROL_QUEUE_SIZE as u64 * 2) + 2;
    let used_offset = align_up(avail_offset + avail_len, 4);
    let used_ring_offset = used_offset + 4;
    common.write_u64(0x20, dma.device_address);
    common.write_u64(0x28, dma.device_address + avail_offset);
    common.write_u64(0x30, dma.device_address + used_offset);
    common.write_u16(0x1c, 1);
    Ok(VirtQueue {
        size: CONTROL_QUEUE_SIZE,
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
) -> Result<(Vec<RequestSlot>, VecDeque<u16>), VirtioGpuError> {
    let mut slots = Vec::with_capacity(SLOT_COUNT);
    let mut free = VecDeque::with_capacity(SLOT_COUNT);
    for index in 0..SLOT_COUNT {
        let buffer = platform.allocate_dma(
            BUFFER_SIZE as u64,
            DmaDirection::Bidirectional,
            DmaCoherency::Coherent,
            DmaConstraints::platform_default(),
        )?;
        slots.push(RequestSlot {
            buffer,
            kind: VirtioGpuRequestKind::Command,
            busy: false,
            submitted_len: 0,
        });
        free.push_back(index as u16);
    }
    Ok((slots, free))
}

fn queue_submit(
    queue: &mut VirtQueue,
    descriptor_index: u16,
    addr: u64,
    len: u32,
) -> Result<(), VirtioGpuError> {
    if descriptor_index >= queue.size {
        return Err(VirtioGpuError::InvalidQueueState);
    }
    unsafe {
        write_volatile(
            queue.desc_ptr.add(descriptor_index as usize),
            VirtqDesc {
                addr,
                len,
                flags: VIRTQ_DESC_F_WRITE,
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

fn notify_queue(notify: NotifyCfg, common: CommonCfg) {
    common.write_u16(0x16, CONTROL_QUEUE_INDEX);
    let offset = common.read_u16(0x1e) as u32;
    let notify_ptr = unsafe {
        notify
            .base
            .add((offset.saturating_mul(notify.multiplier)) as usize) as *mut u16
    };
    unsafe { write_volatile(notify_ptr, CONTROL_QUEUE_INDEX) }
}

fn read_virtio_caps<P: DevicePlatform>(
    platform: &mut P,
    device: DeviceLocator,
) -> Result<Vec<VirtioPciCapability>, VirtioGpuError> {
    if (read_config_u16(platform, device, PCI_STATUS_OFFSET)? & PCI_STATUS_CAPABILITIES) == 0 {
        return Err(VirtioGpuError::MissingCapability);
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
) -> Result<u8, VirtioGpuError> {
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
) -> Result<u16, VirtioGpuError> {
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
) -> Result<u32, VirtioGpuError> {
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
        let rem = value % align;
        if rem == 0 {
            value
        } else {
            value + (align - rem)
        }
    }
}
fn virtqueue_layout_len(size: u16) -> u64 {
    let d = size_of::<VirtqDesc>() as u64 * size as u64;
    let a = 4 + size as u64 * 2 + 2;
    let u = 4 + size_of::<VirtqUsedElem>() as u64 * size as u64 + 2;
    align_up(d + a, 4) + u
}

impl CommonCfg {
    fn read_status(self) -> u8 {
        self.read_u8(0x14)
    }
    fn write_status(self, value: u8) {
        self.write_u8(0x14, value)
    }
    fn read_u8(self, offset: usize) -> u8 {
        unsafe { read_volatile(self.base.add(offset)) }
    }
    fn write_u8(self, offset: usize, value: u8) {
        unsafe { write_volatile(self.base.add(offset), value) }
    }
    fn read_u16(self, offset: usize) -> u16 {
        unsafe { read_volatile(self.base.add(offset) as *const u16) }
    }
    fn write_u16(self, offset: usize, value: u16) {
        unsafe { write_volatile(self.base.add(offset) as *mut u16, value) }
    }
    fn read_u32(self, offset: usize) -> u32 {
        unsafe { read_volatile(self.base.add(offset) as *const u32) }
    }
    fn write_u32(self, offset: usize, value: u32) {
        unsafe { write_volatile(self.base.add(offset) as *mut u32, value) }
    }
    fn write_u64(self, offset: usize, value: u64) {
        unsafe { write_volatile(self.base.add(offset) as *mut u64, value) }
    }
}
impl IsrCfg {
    fn read_u8(self) -> u8 {
        unsafe { read_volatile(self.base as *const u8) }
    }
}
impl VirtQueue {
    fn used_idx(&self) -> u16 {
        unsafe { read_volatile(self.used_idx_ptr) }
    }
    fn used_elem(&self, idx: u16) -> VirtqUsedElem {
        unsafe { read_volatile(self.used_ring_ptr.add((idx % self.size) as usize)) }
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyntheticPciConfigBackend, X86_64DevicePlatform, X86_64DevicePlatformConfig};
    use std::boxed::Box;

    struct TestGpu {
        mmio: Box<[u8]>,
        _dma: Box<[u8]>,
        record: DeviceRecord,
        platform: X86_64DevicePlatform<SyntheticPciConfigBackend>,
        last_avail: u16,
    }

    impl TestGpu {
        fn new() -> Self {
            let mut mmio = vec![0u8; 0x4000].into_boxed_slice();
            let dma = vec![0u8; 0x20000].into_boxed_slice();
            let mmio_base = mmio.as_mut_ptr() as u64;
            let dma_base = dma.as_ptr() as u64;
            unsafe {
                write_volatile(
                    mmio.as_mut_ptr().add(0x04) as *mut u32,
                    VIRTIO_F_VERSION_1 as u32,
                );
                write_volatile(mmio.as_mut_ptr().add(0x12) as *mut u16, 1);
                write_volatile(mmio.as_mut_ptr().add(0x18) as *mut u16, CONTROL_QUEUE_SIZE);
            }
            let mut backend = SyntheticPciConfigBackend::new();
            let address = crate::device_platform::PciAddress {
                segment: 0,
                bus: 0,
                device: 3,
                function: 0,
            };
            let identity = platform_hal::DeviceIdentity {
                vendor_id: PCI_VENDOR_VIRTIO,
                device_id: 0x1050,
                subsystem_vendor_id: PCI_VENDOR_VIRTIO,
                subsystem_device_id: 16,
                revision_id: 1,
                base_class: 0x03,
                sub_class: 0,
                programming_interface: 0,
            };
            backend.define_device(address, identity, PCI_VENDOR_VIRTIO, 16, false, 9, 1);
            backend.define_bar(address, 0, ((mmio_base as u32) & !0xf) | 0x4, 0xffff_c000);
            backend.define_bar(address, 1, (mmio_base >> 32) as u32, 0xffff_ffff);
            define_cap(
                &mut backend,
                address,
                0x50,
                VIRTIO_PCI_CAP_COMMON_CFG,
                0,
                0x0000,
                0x100,
                0,
                0x60,
            );
            define_cap(
                &mut backend,
                address,
                0x60,
                VIRTIO_PCI_CAP_NOTIFY_CFG,
                0,
                0x1000,
                0x100,
                4,
                0x70,
            );
            define_cap(
                &mut backend,
                address,
                0x70,
                VIRTIO_PCI_CAP_ISR_CFG,
                0,
                0x2000,
                0x20,
                0,
                0x80,
            );
            define_cap(
                &mut backend,
                address,
                0x80,
                VIRTIO_PCI_CAP_DEVICE_CFG,
                0,
                0x3000,
                0x20,
                0,
                0,
            );
            let mut platform = X86_64DevicePlatform::new(
                backend,
                X86_64DevicePlatformConfig {
                    direct_map_base: 0,
                    direct_map_size: u64::MAX,
                    dma_window: crate::DmaWindow {
                        physical_start: dma_base,
                        len: dma.len() as u64,
                    },
                    interrupt_vector_base: 64,
                    interrupt_vector_count: 16,
                },
            );
            let record = platform.enumerate_devices().unwrap().remove(0);
            Self {
                mmio,
                _dma: dma,
                record,
                platform,
                last_avail: 0,
            }
        }
    }

    fn define_cap(
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

    fn emulate_complete(test: &mut TestGpu, driver: &VirtioGpuDriver, response: &[u8]) {
        let desc = driver.control_queue.desc_ptr as u64;
        let avail = desc + (size_of::<VirtqDesc>() * driver.control_queue.size as usize) as u64;
        let used = align_up(avail + 4 + (driver.control_queue.size as u64 * 2) + 2, 4);
        let avail_idx = unsafe { read_volatile((avail + 2) as *const u16) };
        let ring = test.last_avail % CONTROL_QUEUE_SIZE;
        let desc_index = unsafe { read_volatile((avail + 4 + u64::from(ring) * 2) as *const u16) };
        let entry = unsafe { read_volatile((desc as *const VirtqDesc).add(desc_index as usize)) };
        unsafe {
            core::ptr::copy_nonoverlapping(
                response.as_ptr(),
                entry.addr as *mut u8,
                response.len(),
            );
            let used_idx_ptr = (used + 2) as *mut u16;
            let used_idx = read_volatile(used_idx_ptr);
            write_volatile(
                (used
                    + 4
                    + u64::from(used_idx % CONTROL_QUEUE_SIZE) * size_of::<VirtqUsedElem>() as u64)
                    as *mut VirtqUsedElem,
                VirtqUsedElem {
                    id: desc_index as u32,
                    len: response.len() as u32,
                },
            );
            write_volatile(used_idx_ptr, used_idx.wrapping_add(1));
            write_volatile(test.mmio.as_mut_ptr().add(0x2000), VIRTIO_ISR_QUEUE);
        }
        assert!(test.last_avail < avail_idx);
        test.last_avail = test.last_avail.wrapping_add(1);
    }

    #[test]
    fn virtio_gpu_initializes_and_matches_display_device() {
        let mut test = TestGpu::new();
        let driver = VirtioGpuDriver::initialize(&mut test.platform, &test.record).unwrap();
        assert_eq!(driver.device(), test.record.locator);
        assert_eq!(driver.pci_match().subsystem_device_id, 16);
    }

    #[test]
    fn virtio_gpu_submits_and_completes_requests() {
        let mut test = TestGpu::new();
        let mut driver = VirtioGpuDriver::initialize(&mut test.platform, &test.record).unwrap();
        let slot = driver
            .submit_command(&mut test.platform, b"draw:triangle")
            .unwrap();
        emulate_complete(&mut test, &driver, b"fence:triangle");
        let _ = test
            .platform
            .dispatch_interrupt_vector(driver.interrupt_route().vector)
            .unwrap()
            .unwrap();
        let summary = driver.service_interrupt(&mut test.platform).unwrap();
        assert_eq!(summary.completed, 1);
        let completions = driver.take_completions();
        assert_eq!(completions[0].slot, slot);
        assert_eq!(completions[0].kind, VirtioGpuRequestKind::Command);
        assert_eq!(completions[0].response_payload, b"fence:triangle".to_vec());
    }

    #[test]
    fn virtio_gpu_reuses_slots_after_present_completion() {
        let mut test = TestGpu::new();
        let mut driver = VirtioGpuDriver::initialize(&mut test.platform, &test.record).unwrap();
        let first = driver
            .present_frame(&mut test.platform, b"frame:boot")
            .unwrap();
        emulate_complete(&mut test, &driver, b"present:ok");
        let _ = test
            .platform
            .dispatch_interrupt_vector(driver.interrupt_route().vector)
            .unwrap()
            .unwrap();
        let _ = driver.service_interrupt(&mut test.platform).unwrap();
        assert_eq!(
            driver.take_completions()[0].kind,
            VirtioGpuRequestKind::Present
        );
        let second = driver
            .submit_command(&mut test.platform, b"draw:reuse")
            .unwrap();
        assert!(usize::from(first) < SLOT_COUNT);
        assert!(usize::from(second) < SLOT_COUNT);
    }
}
