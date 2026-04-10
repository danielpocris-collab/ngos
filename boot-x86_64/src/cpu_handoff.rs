//! Canonical subsystem role:
//! - subsystem: boot CPU handoff
//! - owner layer: Layer 0 to Layer 1 transition
//! - semantic owner: `boot-x86_64`
//! - truth path role: canonical handoff of boot CPU extended-state facts into
//!   `kernel-core` runtime policy
//!
//! Canonical contract families handled here:
//! - CPU extended-state handoff contracts
//! - runtime policy installation contracts
//!
//! This module may package and apply boot-owned CPU facts into runtime policy,
//! but it must not replace the long-term CPU ownership model defined in
//! `kernel-core`.

use alloc::vec;
use alloc::vec::Vec;

pub fn boot_cpu_extended_state_handoff() -> kernel_core::CpuExtendedStateHandoff {
    let cpu = crate::cpu_runtime_status::snapshot();
    kernel_core::CpuExtendedStateHandoff {
        xsave_managed: cpu.xsave_enabled,
        save_area_bytes: cpu.save_area_bytes,
        xcr0_mask: cpu.xcr0,
        boot_probed: cpu.probe_attempted && cpu.probe_saved && cpu.probe_restored,
        boot_seed_marker: cpu.probe_seed_marker,
    }
}

pub fn apply_boot_cpu_extended_state_handoff(policy: &mut kernel_core::RuntimePolicy) {
    policy.apply_cpu_extended_state_handoff(boot_cpu_extended_state_handoff());
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn boot_scheduler_cpu_topology_handoff(
    boot_info: &platform_x86_64::BootInfo<'_>,
) -> Vec<kernel_core::SchedulerCpuTopologyEntry> {
    let bootstrap_apic_id = crate::smp::bootstrap_apic_id();
    let Some(topology) = platform_x86_64::apic_topology(boot_info, bootstrap_apic_id) else {
        return vec![kernel_core::SchedulerCpuTopologyEntry {
            apic_id: bootstrap_apic_id,
            package_id: 0,
            core_group: 0,
            sibling_group: 0,
            inferred: true,
        }];
    };
    topology
        .processors
        .iter()
        .filter(|processor| processor.enabled)
        .enumerate()
        .map(
            |(cpu_index, processor)| kernel_core::SchedulerCpuTopologyEntry {
                apic_id: processor.apic_id,
                package_id: 0,
                core_group: cpu_index / 2,
                sibling_group: cpu_index % 2,
                inferred: true,
            },
        )
        .collect()
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn apply_boot_scheduler_topology_handoff(
    policy: &mut kernel_core::RuntimePolicy,
    boot_info: &platform_x86_64::BootInfo<'_>,
) {
    policy.apply_scheduler_cpu_topology(boot_scheduler_cpu_topology_handoff(boot_info));
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_x86_64::{BootInfo, BootMemoryRegion, BootMemoryRegionKind};

    #[test]
    fn boot_cpu_handoff_reflects_runtime_snapshot() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 4096, true, true, true, true, true, true, true, 0xe7,
        );
        crate::cpu_runtime_status::record_probe(true, true, true, 4096, 0, 0x4444_aaaa);

        let handoff = boot_cpu_extended_state_handoff();
        assert!(handoff.xsave_managed);
        assert_eq!(handoff.save_area_bytes, 4096);
        assert_eq!(handoff.xcr0_mask, 0xe7);
        assert!(handoff.boot_probed);
        assert_eq!(handoff.boot_seed_marker, 0x4444_aaaa);
    }

    #[test]
    fn boot_scheduler_cpu_topology_handoff_falls_back_to_bootstrap_cpu() {
        let boot_info = BootInfo {
            protocol: platform_x86_64::BootProtocol::LoaderDefined,
            command_line: None,
            rsdp: None,
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0,
                kind: BootMemoryRegionKind::Reserved,
            },
        };

        let topology = boot_scheduler_cpu_topology_handoff(&boot_info);
        assert_eq!(topology.len(), 1);
        assert_eq!(topology[0].core_group, 0);
        assert_eq!(topology[0].sibling_group, 0);
        assert!(topology[0].inferred);
    }
}
