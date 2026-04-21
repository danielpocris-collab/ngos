//! Canonical subsystem role:
//! - subsystem: x86_64 AMD GPU platform mediation
//! - owner layer: platform mediation
//! - semantic owner: `platform-x86_64`
//! - truth path role: platform-specific AMD GPU hardware mechanics (RDNA 3/4)

use alloc::vec::Vec;
use platform_hal::{
    BarId, DeviceLocator, DeviceIdentity, DmaCoherency, DmaConstraints, 
    DmaDirection, HalError, DevicePlatform
};

pub const AMD_VENDOR_ID: u16 = 0x1002;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmdArchitecture {
    Rdna3, // GFX11
    Rdna4, // GFX12
    Rdna5, // GFX13
}

// PM4 Packet Type 3 Header
pub const PM4_TYPE_3: u32 = 3;
pub fn pm4_header(opcode: u8, count: u16) -> u32 {
    (PM4_TYPE_3 << 30) | (((count as u32) & 0x3FFF) << 16) | (((opcode as u32) & 0xFF) << 8)
}

pub struct AmdGpu {
    pub locator: DeviceLocator,
    pub identity: DeviceIdentity,
    pub mmio_bar: BarId,
    pub vram_bar: BarId,
    pub mmio_base: u64,
    pub arch: AmdArchitecture,
}

impl AmdGpu {
    pub fn try_detect<P: DevicePlatform>(
        platform: &P,
        locator: DeviceLocator,
    ) -> Result<Option<Self>, HalError> {
        let record = platform.enumerate_devices()?
            .into_iter()
            .find(|r| r.locator == locator)
            .ok_or(HalError::InvalidDevice)?;

        if record.identity.vendor_id != AMD_VENDOR_ID {
            return Ok(None);
        }

        Ok(Some(Self {
            locator,
            identity: record.identity,
            mmio_bar: record.bars[0].id,
            vram_bar: record.bars[2].id, // BAR2 is usually VRAM on AMD
            mmio_base: record.bars[0].base,
        }))
    }
}

pub struct AmdPspAgent {
    pub firmware_loaded: bool,
}

impl AmdPspAgent {
    pub fn new() -> Self {
        Self { firmware_loaded: false }
    }

    pub unsafe fn load_firmware<P: DevicePlatform>(
        &mut self,
        _gpu: &AmdGpu,
        _platform: &mut P,
        _fw_blob: &[u8],
    ) -> Result<(), HalError> {
        // Logica de incarcare via PSP (similar cu GSP bootstrap)
        self.firmware_loaded = true;
        Ok(())
    }
}

pub struct AmdCpAgent {
    pub ring_vaddr: u64,
    pub ring_size: u32,
    pub wptr: u32,
}

impl AmdCpAgent {
    pub fn new(vaddr: u64, size: u32) -> Self {
        Self {
            ring_vaddr: vaddr,
            ring_size: size,
            wptr: 0,
        }
    }

    pub unsafe fn submit_packet(&mut self, header: u32, body: &[u32]) {
        let ring = self.ring_vaddr as *mut u32;
        unsafe {
            core::ptr::write_volatile(ring.add(self.wptr as usize), header);
            for (i, &word) in body.iter().enumerate() {
                core::ptr::write_volatile(ring.add((self.wptr + 1 + i as u32) as usize), word);
            }
        }
        self.wptr = (self.wptr + 1 + body.len() as u32) % (self.ring_size / 4);
    }
}

// SDMA 6.0 Opcodes
pub const SDMA_OP_COPY_LINEAR: u8 = 0x01;
pub const SDMA_OP_WRITE_UNTILED: u8 = 0x02;
pub const SDMA_OP_FILL_LINEAR: u8 = 0x03;
pub const SDMA_OP_FENCE: u8 = 0x07;

pub fn sdma_header(opcode: u8, sub_op: u8, count: u16) -> u32 {
    (opcode as u32) | ((sub_op as u32) << 8) | ((count as u32) << 16)
}

pub struct AmdSdmaAgent {
    pub ring_vaddr: u64,
    pub ring_size: u32,
    pub wptr: u32,
    pub doorbell_vaddr: u64,
}

impl AmdSdmaAgent {
    pub fn new(vaddr: u64, size: u32, doorbell: u64) -> Self {
        Self {
            ring_vaddr: vaddr,
            ring_size: size,
            wptr: 0,
            doorbell_vaddr: doorbell,
        }
    }

    /// Lanseaza o copie lineara via SDMA (RDNA 3)
    pub unsafe fn submit_copy(&mut self, src_paddr: u64, dst_paddr: u64, size: u32) {
        let ring = self.ring_vaddr as *mut u32;
        
        // SDMA Copy Linear Packet: Header + 7 DWORDs
        let header = sdma_header(SDMA_OP_COPY_LINEAR, 0, 7);
        
        unsafe {
            let base = self.wptr as usize;
            core::ptr::write_volatile(ring.add(base), header);
            core::ptr::write_volatile(ring.add(base + 1), size); // Bytes to copy
            core::ptr::write_volatile(ring.add(base + 2), 0);    // Reserved
            core::ptr::write_volatile(ring.add(base + 3), (src_paddr & 0xFFFFFFFF) as u32);
            core::ptr::write_volatile(ring.add(base + 4), (src_paddr >> 32) as u32);
            core::ptr::write_volatile(ring.add(base + 5), (dst_paddr & 0xFFFFFFFF) as u32);
            core::ptr::write_volatile(ring.add(base + 6), (dst_paddr >> 32) as u32);
            core::ptr::write_volatile(ring.add(base + 7), 0); // Control flags
        }
        
        self.wptr = (self.wptr + 8) % (self.ring_size / 4);
        
        // Ring the doorbell (64-bit write for RDNA 3)
        unsafe {
            core::ptr::write_volatile(self.doorbell_vaddr as *mut u64, self.wptr as u64);
        }
    }
}

