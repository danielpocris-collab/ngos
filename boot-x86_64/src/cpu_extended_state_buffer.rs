//! Canonical subsystem role:
//! - subsystem: boot CPU extended-state buffer mediation
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: hardware-facing XSAVE/XRSTOR buffer handling for the
//!   real x86 boot path
//!
//! Canonical contract families handled here:
//! - XSAVE buffer contracts
//! - aligned CPU state buffer contracts
//! - CPU state save/restore buffer I/O contracts
//!
//! This module may perform hardware-facing CPU state buffer operations, but it
//! must not redefine the higher-level CPU ownership model from `kernel-core`.

use core::arch::asm;

use crate::cpu_features::CpuExtendedStateStatus;

pub const XSAVE_BUFFER_ALIGNMENT: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuExtendedStateBufferError {
    XsaveDisabled,
    BufferTooSmall {
        required: u32,
        provided: usize,
    },
    BufferMisaligned {
        required_alignment: usize,
        address: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuExtendedStateBufferIo {
    pub bytes: u32,
    pub xcr0_mask: u64,
    pub seed_marker: u64,
}

#[repr(align(64))]
pub struct AlignedExtendedStateBuffer<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> AlignedExtendedStateBuffer<N> {
    pub const fn zeroed() -> Self {
        Self { bytes: [0; N] }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

pub fn save_extended_state_to_buffer(
    status: &CpuExtendedStateStatus,
    buffer: &mut [u8],
) -> Result<CpuExtendedStateBufferIo, CpuExtendedStateBufferError> {
    validate_buffer(status, buffer)?;
    let mask_low = status.xcr0 as u32;
    let mask_high = (status.xcr0 >> 32) as u32;
    unsafe {
        asm!(
            "xsave64 [{}]",
            in(reg) buffer.as_mut_ptr(),
            in("eax") mask_low,
            in("edx") mask_high,
            options(nostack)
        );
    }
    Ok(CpuExtendedStateBufferIo {
        bytes: status.save_area_bytes,
        xcr0_mask: status.xcr0,
        seed_marker: buffer_seed_marker(buffer, status.save_area_bytes as usize),
    })
}

pub fn restore_extended_state_from_buffer(
    status: &CpuExtendedStateStatus,
    buffer: &mut [u8],
) -> Result<CpuExtendedStateBufferIo, CpuExtendedStateBufferError> {
    validate_buffer(status, buffer)?;
    let mask_low = status.xcr0 as u32;
    let mask_high = (status.xcr0 >> 32) as u32;
    unsafe {
        asm!(
            "xrstor64 [{}]",
            in(reg) buffer.as_mut_ptr(),
            in("eax") mask_low,
            in("edx") mask_high,
            options(nostack)
        );
    }
    Ok(CpuExtendedStateBufferIo {
        bytes: status.save_area_bytes,
        xcr0_mask: status.xcr0,
        seed_marker: buffer_seed_marker(buffer, status.save_area_bytes as usize),
    })
}

pub fn buffer_seed_marker(buffer: &[u8], used_len: usize) -> u64 {
    let bounded = core::cmp::min(buffer.len(), used_len);
    let mut marker = bounded as u64;
    for (index, byte) in buffer.iter().take(core::cmp::min(bounded, 64)).enumerate() {
        marker ^= u64::from(*byte) << ((index % 8) * 8);
        marker = marker.rotate_left(5) ^ ((index as u64).wrapping_mul(0x9E37));
    }
    marker
}

fn validate_buffer(
    status: &CpuExtendedStateStatus,
    buffer: &mut [u8],
) -> Result<(), CpuExtendedStateBufferError> {
    if !status.xsave_enabled {
        return Err(CpuExtendedStateBufferError::XsaveDisabled);
    }
    if buffer.len() < status.save_area_bytes as usize {
        return Err(CpuExtendedStateBufferError::BufferTooSmall {
            required: status.save_area_bytes,
            provided: buffer.len(),
        });
    }
    let address = buffer.as_mut_ptr() as usize;
    if address % XSAVE_BUFFER_ALIGNMENT != 0 {
        return Err(CpuExtendedStateBufferError::BufferMisaligned {
            required_alignment: XSAVE_BUFFER_ALIGNMENT,
            address,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_status() -> CpuExtendedStateStatus {
        CpuExtendedStateStatus {
            sse_ready: true,
            xsave_enabled: true,
            save_area_bytes: 4096,
            fsgsbase_enabled: true,
            pcid_enabled: true,
            invpcid_available: true,
            pku_enabled: true,
            smep_enabled: true,
            smap_enabled: true,
            umip_enabled: true,
            xcr0: 0xe7,
        }
    }

    #[test]
    fn extended_state_buffer_refuses_small_slice() {
        let mut bytes = [0u8; 128];
        let error = save_extended_state_to_buffer(&test_status(), &mut bytes).unwrap_err();
        assert_eq!(
            error,
            CpuExtendedStateBufferError::BufferTooSmall {
                required: 4096,
                provided: 128,
            }
        );
    }

    #[test]
    fn extended_state_buffer_accepts_aligned_storage() {
        let mut buffer = AlignedExtendedStateBuffer::<4096>::zeroed();
        let io = save_extended_state_to_buffer(&test_status(), buffer.as_mut_slice()).unwrap();
        assert_eq!(io.bytes, 4096);
        assert_eq!(io.xcr0_mask, 0xe7);
        assert_ne!(io.seed_marker, 0);
    }

    #[test]
    fn extended_state_buffer_restore_refuses_disabled_xsave() {
        let mut status = test_status();
        status.xsave_enabled = false;
        let mut buffer = AlignedExtendedStateBuffer::<4096>::zeroed();
        let error = restore_extended_state_from_buffer(&status, buffer.as_mut_slice()).unwrap_err();
        assert_eq!(error, CpuExtendedStateBufferError::XsaveDisabled);
    }
}
