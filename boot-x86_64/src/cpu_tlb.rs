//! Canonical subsystem role:
//! - subsystem: boot TLB invalidation mediation
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-stage hardware choice for TLB invalidation on the
//!   real x86 path
//!
//! Canonical contract families handled here:
//! - TLB invalidation contracts
//! - INVPCID/CR3 fallback contracts
//!
//! This module may choose and execute hardware TLB invalidation at boot/runtime
//! bridge time, but it must not redefine VM ownership or address-space truth.

use core::arch::asm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbFlushMethod {
    None = 0,
    Cr3Reload = 1,
    InvpcidSingleContext = 2,
}

#[repr(C, align(16))]
struct InvpcidDescriptor {
    pcid: u64,
    linear_address: u64,
}

pub fn invalidate_address_space(root_phys: u64) -> TlbFlushMethod {
    let cpu = crate::cpu_runtime_status::snapshot();
    if cpu.invpcid_available {
        let descriptor = InvpcidDescriptor {
            pcid: root_phys & 0xfff,
            linear_address: 0,
        };
        unsafe {
            asm!(
                "invpcid {kind}, xmmword ptr [{descriptor}]",
                kind = in(reg) 1u64,
                descriptor = in(reg) &descriptor,
                options(nostack, preserves_flags)
            );
        }
        return TlbFlushMethod::InvpcidSingleContext;
    }

    unsafe {
        asm!("mov cr3, {}", in(reg) root_phys, options(nostack, preserves_flags));
    }
    TlbFlushMethod::Cr3Reload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tlb_flush_method_values_remain_stable() {
        assert_eq!(TlbFlushMethod::None as u32, 0);
        assert_eq!(TlbFlushMethod::Cr3Reload as u32, 1);
        assert_eq!(TlbFlushMethod::InvpcidSingleContext as u32, 2);
    }
}
