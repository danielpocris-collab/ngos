#![cfg_attr(not(target_os = "none"), allow(dead_code))]

//! Canonical subsystem role:
//! - subsystem: boot local APIC mode activation
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-stage local APIC mode selection and activation for
//!   the real x86 path
//!
//! Canonical contract families handled here:
//! - local APIC mode contracts
//! - x2APIC activation contracts
//!
//! This module may activate boot-stage APIC mode, but it must not redefine the
//! higher-level scheduler or interrupt ownership model.

const IA32_APIC_BASE: u32 = 0x1b;
const IA32_APIC_BASE_GLOBAL_ENABLE: u64 = 1 << 11;
const IA32_APIC_BASE_X2APIC_ENABLE: u64 = 1 << 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalApicMode {
    XApic = 1,
    X2Apic = 2,
}

pub fn enable_preferred_local_apic_mode() -> LocalApicMode {
    #[cfg(target_os = "none")]
    {
        let cpu = crate::boot_facts::cpu_facts();
        let policy = crate::boot_facts::cpu_feature_policy(&cpu);
        if !policy.enable_x2apic {
            return LocalApicMode::XApic;
        }
        let mut apic_base = read_msr(IA32_APIC_BASE);
        apic_base |= IA32_APIC_BASE_GLOBAL_ENABLE | IA32_APIC_BASE_X2APIC_ENABLE;
        write_msr(IA32_APIC_BASE, apic_base);
        let observed = read_msr(IA32_APIC_BASE);
        if (observed & IA32_APIC_BASE_X2APIC_ENABLE) != 0 {
            return LocalApicMode::X2Apic;
        }
        LocalApicMode::XApic
    }
    #[cfg(not(target_os = "none"))]
    {
        LocalApicMode::XApic
    }
}

#[cfg(target_os = "none")]
fn read_msr(msr: u32) -> u64 {
    use core::arch::asm;
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

#[cfg(target_os = "none")]
fn write_msr(msr: u32, value: u64) {
    use core::arch::asm;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") value as u32,
            in("edx") (value >> 32) as u32,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_apic_mode_values_remain_stable() {
        assert_eq!(LocalApicMode::XApic as u32, 1);
        assert_eq!(LocalApicMode::X2Apic as u32, 2);
    }
}
