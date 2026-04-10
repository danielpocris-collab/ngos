//! Canonical subsystem role:
//! - subsystem: x86_64 Intel GPU platform mediation
//! - owner layer: platform mediation
//! - semantic owner: `platform-x86_64`
//! - truth path role: platform-specific Intel GPU hardware mechanics (Xe/Arc)

use platform_hal::{
    BarId, DeviceLocator, DeviceIdentity, HalError, DevicePlatform
};

pub const INTEL_VENDOR_ID: u16 = 0x8086;

// GuC Doorbell Offset
pub const GUC_DOORBELL_OFFSET: usize = 0x2000;
// Intel Xe2 / Battlemage Specifics
pub const INTEL_DEVICE_ID_BATTLEMAGE: u16 = 0xE200; // Gama Arc B-series
pub const INTEL_GUC_HOST_INTERRUPT_OFFSET: usize = 0x190000;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IntelGucContextDescriptor {
    pub desc_flags: u32,
    pub context_id: u32,
    pub ring_vaddr: u64,
    pub ring_size: u32,
    pub work_queue_vaddr: u64,
    pub work_queue_size: u32,
}

pub struct IntelGpu {
    pub locator: DeviceLocator,
    pub identity: DeviceIdentity,
    pub mmio_bar: BarId,
    pub gtt_bar: BarId,
    pub mmio_base: u64,
}

impl IntelGpu {
    pub fn try_detect<P: DevicePlatform>(
        platform: &P,
        locator: DeviceLocator,
    ) -> Result<Option<Self>, HalError> {
        let record = platform.enumerate_devices()?
            .into_iter()
            .find(|r| r.locator == locator)
            .ok_or(HalError::InvalidDevice)?;

        if record.identity.vendor_id != INTEL_VENDOR_ID {
            return Ok(None);
        }

        Ok(Some(Self {
            locator,
            identity: record.identity,
            mmio_bar: record.bars[0].id,
            gtt_bar: record.bars[1].id,
            mmio_base: record.bars[0].base,
        }))
    }
}

pub struct IntelGucAgent {
    pub firmware_loaded: bool,
    pub doorbell_vaddr: u64,
}

impl IntelGucAgent {
    pub fn new(mmio_vaddr: u64) -> Self {
        Self {
            firmware_loaded: false,
            doorbell_vaddr: mmio_vaddr + GUC_DOORBELL_OFFSET as u64,
        }
    }

    pub unsafe fn ring_doorbell(&self, context_id: u32) {
        let ptr = self.doorbell_vaddr as *mut u32;
        unsafe {
            core::ptr::write_volatile(ptr.add(context_id as usize), 1);
        }
    }
}

