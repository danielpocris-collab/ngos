//! Canonical subsystem role:
//! - subsystem: boot CPU hardware-provider installation
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-owned hardware hook for CPU extended-state save and
//!   restore on the real x86 path
//!
//! Canonical contract families handled here:
//! - CPU hardware-provider contracts
//! - XSAVE/XRSTOR bridge contracts
//! - boot-to-runtime CPU install contracts
//!
//! This module may attach boot-owned CPU hardware operations into the runtime,
//! but it must not redefine the higher-level CPU ownership model that belongs
//! to `kernel-core`.

#[cfg(not(test))]
use core::arch::asm;

use alloc::boxed::Box;
use alloc::vec::Vec;

use kernel_core::{HardwareProvider, ProcessId, ThreadCpuExtendedStateImage, ThreadId};
use platform_hal::HalError;

const XSAVE_ALIGNMENT: usize = 64;
const HOST_CPU_EXTENDED_STATE_BUFFER_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootCpuHardwareProviderRefusal {
    None = 0,
    XsaveDisabled = 1,
    SaveAreaUnavailable = 2,
    Xcr0Unavailable = 3,
    SaveAreaTooLarge = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootCpuHardwareProvider {
    xsave_enabled: bool,
    save_area_bytes: u32,
    xcr0: u64,
}

impl BootCpuHardwareProvider {
    pub fn from_runtime_snapshot() -> Self {
        let snapshot = crate::cpu_runtime_status::snapshot();
        Self {
            xsave_enabled: snapshot.xsave_enabled,
            save_area_bytes: snapshot.save_area_bytes,
            xcr0: snapshot.xcr0,
        }
    }

    pub fn install_into_runtime(runtime: &mut kernel_core::KernelRuntime) -> bool {
        let snapshot = crate::cpu_runtime_status::snapshot();
        let refusal = install_refusal_from_snapshot(snapshot);
        if refusal != BootCpuHardwareProviderRefusal::None {
            crate::cpu_runtime_status::record_hardware_provider_install(
                false,
                true,
                refusal as u32,
            );
            crate::boot_locator::event(
                crate::boot_locator::BootLocatorStage::User,
                crate::boot_locator::BootLocatorKind::Transition,
                crate::boot_locator::BootLocatorSeverity::Warn,
                0x572,
                crate::boot_locator::BootPayloadLabel::Status,
                refusal as u64,
                crate::boot_locator::BootPayloadLabel::Length,
                snapshot.save_area_bytes as u64,
            );
            return false;
        }
        runtime.install_hardware_provider(Box::new(Self::from_runtime_snapshot()));
        crate::cpu_runtime_status::record_hardware_provider_install(true, false, 0);
        crate::boot_locator::event(
            crate::boot_locator::BootLocatorStage::User,
            crate::boot_locator::BootLocatorKind::Transition,
            crate::boot_locator::BootLocatorSeverity::Info,
            0x571,
            crate::boot_locator::BootPayloadLabel::Status,
            1,
            crate::boot_locator::BootPayloadLabel::Length,
            snapshot.save_area_bytes as u64,
        );
        true
    }
}

impl HardwareProvider for BootCpuHardwareProvider {
    fn submit_gpu_command(&mut self, _rpc_id: u32, _payload: &[u8]) -> Result<Vec<u8>, HalError> {
        Err(HalError::Unsupported)
    }

    fn allocate_gpu_memory(
        &mut self,
        _kind: platform_hal::GpuMemoryKind,
        _size: u64,
    ) -> Result<u64, HalError> {
        Err(HalError::Unsupported)
    }

    fn set_primary_gpu_power_state(&mut self, _pstate: u32) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn start_primary_gpu_media_session(
        &mut self,
        _width: u32,
        _height: u32,
        _bitrate_kbps: u32,
        _codec: u32,
    ) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn inject_primary_gpu_neural_semantic(
        &mut self,
        _semantic_label: &str,
    ) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn commit_primary_gpu_neural_frame(&mut self) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn dispatch_primary_gpu_tensor_kernel(&mut self, _kernel_id: u32) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn gpu_binding_evidence(
        &mut self,
        _device: platform_hal::DeviceLocator,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_binding_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_vbios_window(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosWindowEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_vbios_bytes(&mut self, _max_len: usize) -> Result<Vec<u8>, HalError> {
        Err(HalError::Unsupported)
    }

    fn primary_gpu_vbios_image_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosImageEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_gsp_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuGspEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_interrupt_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuInterruptEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_display_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuDisplayEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_power_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuPowerEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_media_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuMediaEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_neural_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuNeuralEvidence>, HalError> {
        Ok(None)
    }

    fn primary_gpu_tensor_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuTensorEvidence>, HalError> {
        Ok(None)
    }

    fn save_cpu_extended_state(
        &mut self,
        _owner_pid: ProcessId,
        _owner_tid: ThreadId,
        image: &mut ThreadCpuExtendedStateImage,
    ) -> Result<(), HalError> {
        validate_image(self, image.bytes.len())?;
        let len = image.bytes.len();
        let io = if image.bytes.is_aligned() {
            save_xsave_to_buffer(self, image.bytes.as_mut_slice())?
        } else {
            let mut aligned =
                AlignedExtendedStateBuffer::<HOST_CPU_EXTENDED_STATE_BUFFER_BYTES>::zeroed();
            let io = save_xsave_to_buffer(self, aligned.as_mut_slice())?;
            image.bytes[..len].copy_from_slice(&aligned.as_mut_slice()[..len]);
            io
        };
        image.profile.save_area_buffer_bytes = len as u32;
        image.profile.save_area_alignment_bytes = if image.bytes.is_empty() { 0 } else { 64 };
        image.profile.save_area_generation = image.profile.save_area_generation.saturating_add(1);
        image.profile.last_save_marker = buffer_seed_marker(&image.bytes, len);
        if io != 0 {
            image.profile.last_save_marker = io;
        }
        Ok(())
    }

    fn restore_cpu_extended_state(
        &mut self,
        _owner_pid: ProcessId,
        _owner_tid: ThreadId,
        image: &ThreadCpuExtendedStateImage,
    ) -> Result<(), HalError> {
        validate_image(self, image.bytes.len())?;
        if image.bytes.is_aligned() {
            let mut direct = image.bytes.clone();
            restore_xsave_from_buffer(self, direct.as_mut_slice())
        } else {
            let mut aligned =
                AlignedExtendedStateBuffer::<HOST_CPU_EXTENDED_STATE_BUFFER_BYTES>::zeroed();
            let len = image.bytes.len();
            aligned.as_mut_slice()[..len].copy_from_slice(&image.bytes);
            restore_xsave_from_buffer(self, aligned.as_mut_slice())
        }
    }
}

#[repr(align(64))]
struct AlignedExtendedStateBuffer<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> AlignedExtendedStateBuffer<N> {
    const fn zeroed() -> Self {
        Self { bytes: [0; N] }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

fn validate_image(provider: &BootCpuHardwareProvider, image_len: usize) -> Result<(), HalError> {
    if !provider.xsave_enabled || provider.xcr0 == 0 {
        return Err(HalError::Unsupported);
    }
    let expected = provider.save_area_bytes as usize;
    if expected == 0 || image_len != expected || expected > HOST_CPU_EXTENDED_STATE_BUFFER_BYTES {
        return Err(HalError::InvalidDmaBuffer);
    }
    Ok(())
}

fn install_refusal_from_snapshot(
    snapshot: crate::cpu_runtime_status::CpuRuntimeStatus,
) -> BootCpuHardwareProviderRefusal {
    if !snapshot.xsave_enabled {
        BootCpuHardwareProviderRefusal::XsaveDisabled
    } else if snapshot.save_area_bytes == 0 {
        BootCpuHardwareProviderRefusal::SaveAreaUnavailable
    } else if snapshot.xcr0 == 0 {
        BootCpuHardwareProviderRefusal::Xcr0Unavailable
    } else if snapshot.save_area_bytes as usize > HOST_CPU_EXTENDED_STATE_BUFFER_BYTES {
        BootCpuHardwareProviderRefusal::SaveAreaTooLarge
    } else {
        BootCpuHardwareProviderRefusal::None
    }
}

fn save_xsave_to_buffer(
    provider: &BootCpuHardwareProvider,
    buffer: &mut [u8],
) -> Result<u64, HalError> {
    validate_buffer_alignment(buffer)?;
    #[cfg(test)]
    {
        let used_len = provider.save_area_bytes as usize;
        let seed = provider.xcr0.to_le_bytes();
        for (index, byte) in buffer.iter_mut().take(used_len).enumerate() {
            *byte = seed[index % seed.len()] ^ (index as u8);
        }
        Ok(buffer_seed_marker(buffer, used_len))
    }
    #[cfg(not(test))]
    unsafe {
        let mask_low = provider.xcr0 as u32;
        let mask_high = (provider.xcr0 >> 32) as u32;
        asm!(
            "xsave64 [{}]",
            in(reg) buffer.as_mut_ptr(),
            in("eax") mask_low,
            in("edx") mask_high,
            options(nostack)
        );
        Ok(buffer_seed_marker(
            buffer,
            provider.save_area_bytes as usize,
        ))
    }
}

fn restore_xsave_from_buffer(
    provider: &BootCpuHardwareProvider,
    buffer: &mut [u8],
) -> Result<(), HalError> {
    validate_buffer_alignment(buffer)?;
    #[cfg(test)]
    {
        let used_len = provider.save_area_bytes as usize;
        if used_len == 0 || buffer.len() < used_len {
            return Err(HalError::InvalidDmaBuffer);
        }
        Ok(())
    }
    #[cfg(not(test))]
    unsafe {
        let mask_low = provider.xcr0 as u32;
        let mask_high = (provider.xcr0 >> 32) as u32;
        asm!(
            "xrstor64 [{}]",
            in(reg) buffer.as_mut_ptr(),
            in("eax") mask_low,
            in("edx") mask_high,
            options(nostack)
        );
        Ok(())
    }
}

fn validate_buffer_alignment(buffer: &mut [u8]) -> Result<(), HalError> {
    let address = buffer.as_mut_ptr() as usize;
    if address % XSAVE_ALIGNMENT != 0 {
        return Err(HalError::InvalidDmaBuffer);
    }
    Ok(())
}

fn buffer_seed_marker(buffer: &[u8], used_len: usize) -> u64 {
    let bounded = core::cmp::min(buffer.len(), used_len);
    let mut marker = bounded as u64;
    for (index, byte) in buffer.iter().take(core::cmp::min(bounded, 64)).enumerate() {
        marker ^= u64::from(*byte) << ((index % 8) * 8);
        marker = marker.rotate_left(5) ^ ((index as u64).wrapping_mul(0x9E37));
    }
    marker
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_cpu_hardware_provider_uses_runtime_snapshot_for_xsave_status() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 512, true, true, true, false, true, true, true, 0x3,
        );
        let provider = BootCpuHardwareProvider::from_runtime_snapshot();
        assert!(provider.xsave_enabled);
        assert_eq!(provider.save_area_bytes, 512);
        assert_eq!(provider.xcr0, 0x3);
    }

    #[test]
    fn boot_cpu_hardware_provider_classifies_install_refusals() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, false, 512, true, true, true, false, true, true, true, 0x3,
        );
        assert_eq!(
            install_refusal_from_snapshot(crate::cpu_runtime_status::snapshot()),
            BootCpuHardwareProviderRefusal::XsaveDisabled
        );

        crate::cpu_runtime_status::record(
            true, true, 0, true, true, true, false, true, true, true, 0x3,
        );
        assert_eq!(
            install_refusal_from_snapshot(crate::cpu_runtime_status::snapshot()),
            BootCpuHardwareProviderRefusal::SaveAreaUnavailable
        );

        crate::cpu_runtime_status::record(
            true, true, 512, true, true, true, false, true, true, true, 0,
        );
        assert_eq!(
            install_refusal_from_snapshot(crate::cpu_runtime_status::snapshot()),
            BootCpuHardwareProviderRefusal::Xcr0Unavailable
        );

        crate::cpu_runtime_status::record(
            true,
            true,
            (HOST_CPU_EXTENDED_STATE_BUFFER_BYTES as u32) + 64,
            true,
            true,
            true,
            false,
            true,
            true,
            true,
            0x3,
        );
        assert_eq!(
            install_refusal_from_snapshot(crate::cpu_runtime_status::snapshot()),
            BootCpuHardwareProviderRefusal::SaveAreaTooLarge
        );
    }
}
