//! Canonical subsystem role:
//! - subsystem: boot CPU feature activation
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: hardware-facing activation and probe of CPU features
//!   before runtime handoff
//!
//! Canonical contract families handled here:
//! - CPU feature activation contracts
//! - XSAVE/XCR0 setup contracts
//! - boot CPU probe contracts
//!
//! This module may activate and probe boot-stage CPU features, but it must not
//! replace long-term CPU ownership that belongs to `kernel-core`.

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{_xgetbv, _xsetbv};

use crate::boot_facts::CpuFeaturePolicy;
use crate::cpu_extended_state_buffer::{
    AlignedExtendedStateBuffer, restore_extended_state_from_buffer, save_extended_state_to_buffer,
};

const CR4_UMIP: u64 = 1 << 11;
const CR4_FSGSBASE: u64 = 1 << 16;
const CR4_PCIDE: u64 = 1 << 17;
const CR4_OSXSAVE: u64 = 1 << 18;
const CR4_SMEP: u64 = 1 << 20;
const CR4_SMAP: u64 = 1 << 21;
const CR4_PKE: u64 = 1 << 22;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuExtendedStateStatus {
    pub sse_ready: bool,
    pub xsave_enabled: bool,
    pub save_area_bytes: u32,
    pub fsgsbase_enabled: bool,
    pub pcid_enabled: bool,
    pub invpcid_available: bool,
    pub pku_enabled: bool,
    pub smep_enabled: bool,
    pub smap_enabled: bool,
    pub umip_enabled: bool,
    pub xcr0: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuExtendedStateProbeRefusal {
    None,
    XsaveDisabled,
    SaveAreaTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuExtendedStateProbeStatus {
    pub attempted: bool,
    pub saved: bool,
    pub restored: bool,
    pub required_bytes: u32,
    pub refusal: CpuExtendedStateProbeRefusal,
    pub seed_marker: u64,
}

pub fn enable_cpu_extended_state() -> CpuExtendedStateStatus {
    let mut cr0 = read_cr0_local();
    cr0 &= !(1 << 2);
    cr0 |= 1 << 1;
    let cr3 = read_cr3_local();
    let mut cr4 = read_cr4_local();
    cr4 |= (1 << 9) | (1 << 10);
    let cpu = crate::boot_facts::cpu_facts();
    let policy = crate::boot_facts::cpu_feature_policy(&cpu);
    let activation = plan_feature_activation(&policy, cr3);
    let mut xsave_enabled = false;
    let mut xcr0 = 0u64;
    cr4 |= activation.cr4_bits;
    unsafe {
        write_cr0_local(cr0);
        write_cr4_local(cr4);
        if activation.xsave_enabled {
            _xsetbv(0, activation.xcr0_mask);
            xcr0 = _xgetbv(0);
            xsave_enabled = true;
        }
        core::arch::asm!("fninit", options(nomem, nostack, preserves_flags));
    }
    CpuExtendedStateStatus {
        sse_ready: true,
        xsave_enabled,
        save_area_bytes: extended_state_save_area_bytes(&cpu, xsave_enabled),
        fsgsbase_enabled: activation.fsgsbase_enabled,
        pcid_enabled: activation.pcid_enabled,
        invpcid_available: policy.enable_invpcid,
        pku_enabled: activation.pku_enabled,
        smep_enabled: activation.smep_enabled,
        smap_enabled: activation.smap_enabled,
        umip_enabled: activation.umip_enabled,
        xcr0,
    }
}

pub fn probe_boot_extended_state_roundtrip(
    status: &CpuExtendedStateStatus,
) -> CpuExtendedStateProbeStatus {
    if !status.xsave_enabled {
        return CpuExtendedStateProbeStatus {
            attempted: false,
            saved: false,
            restored: false,
            required_bytes: status.save_area_bytes,
            refusal: CpuExtendedStateProbeRefusal::XsaveDisabled,
            seed_marker: 0,
        };
    }
    if status.save_area_bytes as usize > 8192 {
        return CpuExtendedStateProbeStatus {
            attempted: false,
            saved: false,
            restored: false,
            required_bytes: status.save_area_bytes,
            refusal: CpuExtendedStateProbeRefusal::SaveAreaTooLarge,
            seed_marker: 0,
        };
    }
    let mut probe = AlignedExtendedStateBuffer::<8192>::zeroed();
    let saved_io = save_extended_state_to_buffer(status, probe.as_mut_slice()).ok();
    let saved = saved_io.is_some();
    let restored = restore_extended_state_from_buffer(status, probe.as_mut_slice()).is_ok();

    CpuExtendedStateProbeStatus {
        attempted: true,
        saved,
        restored,
        required_bytes: status.save_area_bytes,
        refusal: CpuExtendedStateProbeRefusal::None,
        seed_marker: saved_io.map(|io| io.seed_marker).unwrap_or(0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FeatureActivationPlan {
    cr4_bits: u64,
    xsave_enabled: bool,
    xcr0_mask: u64,
    fsgsbase_enabled: bool,
    pcid_enabled: bool,
    pku_enabled: bool,
    smep_enabled: bool,
    smap_enabled: bool,
    umip_enabled: bool,
}

fn plan_feature_activation(policy: &CpuFeaturePolicy, cr3: u64) -> FeatureActivationPlan {
    let xsave_enabled = policy.enable_xsave;
    let fsgsbase_enabled = policy.enable_fsgsbase;
    let pcid_enabled = policy.enable_pcid && (cr3 & 0xfff) == 0;
    let pku_enabled = policy.enable_pku;
    let smep_enabled = policy.enable_smep;
    let smap_enabled = policy.enable_smap;
    let umip_enabled = policy.enable_umip;
    let mut cr4_bits = 0u64;
    if xsave_enabled {
        cr4_bits |= CR4_OSXSAVE;
    }
    if fsgsbase_enabled {
        cr4_bits |= CR4_FSGSBASE;
    }
    if pcid_enabled {
        cr4_bits |= CR4_PCIDE;
    }
    if pku_enabled {
        cr4_bits |= CR4_PKE;
    }
    if smep_enabled {
        cr4_bits |= CR4_SMEP;
    }
    if smap_enabled {
        cr4_bits |= CR4_SMAP;
    }
    if umip_enabled {
        cr4_bits |= CR4_UMIP;
    }
    FeatureActivationPlan {
        cr4_bits,
        xsave_enabled,
        xcr0_mask: xcr0_mask(policy),
        fsgsbase_enabled,
        pcid_enabled,
        pku_enabled,
        smep_enabled,
        smap_enabled,
        umip_enabled,
    }
}

fn xcr0_mask(policy: &CpuFeaturePolicy) -> u64 {
    if !policy.enable_xsave {
        return 0;
    }
    let mut mask = 0x3u64;
    if policy.allow_avx_user_state {
        mask |= 1 << 2;
    }
    if policy.allow_avx512_user_state {
        mask |= (1 << 5) | (1 << 6) | (1 << 7);
    }
    mask
}

fn extended_state_save_area_bytes(cpu: &crate::boot_facts::CpuFacts, xsave_enabled: bool) -> u32 {
    if xsave_enabled && cpu.max_xsave_bytes != 0 {
        cpu.max_xsave_bytes
    } else {
        512
    }
}

pub fn read_cr0_local() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

pub fn read_cr4_local() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

pub fn read_cr3_local() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

unsafe fn write_cr0_local(value: u64) {
    unsafe {
        core::arch::asm!("mov cr0, {}", in(reg) value, options(nostack, preserves_flags));
    }
}

unsafe fn write_cr4_local(value: u64) {
    unsafe {
        core::arch::asm!("mov cr4, {}", in(reg) value, options(nostack, preserves_flags));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot_facts::CpuVendor;

    #[test]
    fn xcr0_mask_tracks_avx_levels() {
        let mut policy = CpuFeaturePolicy {
            vendor: CpuVendor::Intel,
            enable_xsave: true,
            enable_x2apic: false,
            enable_fsgsbase: false,
            enable_pcid: false,
            enable_invpcid: false,
            enable_smep: false,
            enable_smap: false,
            enable_umip: false,
            enable_pku: false,
            enable_la57: false,
            allow_avx_user_state: false,
            allow_avx512_user_state: false,
        };
        assert_eq!(xcr0_mask(&policy), 0x3);
        policy.allow_avx_user_state = true;
        assert_eq!(xcr0_mask(&policy), 0x7);
        policy.allow_avx512_user_state = true;
        assert_eq!(xcr0_mask(&policy), 0xE7);
    }

    #[test]
    fn activation_plan_only_enables_pcid_on_clean_cr3() {
        let policy = CpuFeaturePolicy {
            vendor: CpuVendor::Intel,
            enable_xsave: true,
            enable_x2apic: false,
            enable_fsgsbase: true,
            enable_pcid: true,
            enable_invpcid: true,
            enable_smep: true,
            enable_smap: true,
            enable_umip: true,
            enable_pku: true,
            enable_la57: false,
            allow_avx_user_state: true,
            allow_avx512_user_state: false,
        };
        let clean = plan_feature_activation(&policy, 0x1000);
        assert!(clean.pcid_enabled);
        assert_ne!(clean.cr4_bits & CR4_PCIDE, 0);
        let tagged = plan_feature_activation(&policy, 0x1001);
        assert!(!tagged.pcid_enabled);
        assert_eq!(tagged.cr4_bits & CR4_PCIDE, 0);
    }

    #[test]
    fn extended_state_save_area_uses_xsave_size_when_enabled() {
        let mut cpu = crate::boot_facts::cpu_facts();
        cpu.max_xsave_bytes = 4096;
        assert_eq!(extended_state_save_area_bytes(&cpu, true), 4096);
        assert_eq!(extended_state_save_area_bytes(&cpu, false), 512);
        cpu.max_xsave_bytes = 0;
        assert_eq!(extended_state_save_area_bytes(&cpu, true), 512);
    }

    #[test]
    fn extended_state_probe_refuses_when_xsave_is_disabled() {
        let status = CpuExtendedStateStatus {
            sse_ready: true,
            xsave_enabled: false,
            save_area_bytes: 512,
            fsgsbase_enabled: false,
            pcid_enabled: false,
            invpcid_available: false,
            pku_enabled: false,
            smep_enabled: false,
            smap_enabled: false,
            umip_enabled: false,
            xcr0: 0,
        };
        let probe = probe_boot_extended_state_roundtrip(&status);
        assert!(!probe.attempted);
        assert_eq!(probe.refusal, CpuExtendedStateProbeRefusal::XsaveDisabled);
    }

    #[test]
    fn extended_state_probe_refuses_when_save_area_exceeds_probe_capacity() {
        let status = CpuExtendedStateStatus {
            sse_ready: true,
            xsave_enabled: true,
            save_area_bytes: (8192 + 64) as u32,
            fsgsbase_enabled: false,
            pcid_enabled: false,
            invpcid_available: false,
            pku_enabled: false,
            smep_enabled: false,
            smap_enabled: false,
            umip_enabled: false,
            xcr0: 0x3,
        };
        let probe = probe_boot_extended_state_roundtrip(&status);
        assert!(!probe.attempted);
        assert_eq!(
            probe.refusal,
            CpuExtendedStateProbeRefusal::SaveAreaTooLarge
        );
    }
}
