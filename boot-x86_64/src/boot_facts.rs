//! Canonical subsystem role:
//! - subsystem: boot hardware facts
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: authoritative boot-stage discovery of CPU, memory, and
//!   firmware facts for handoff into the real system path
//!
//! Canonical contract families produced here:
//! - CPU fact contracts
//! - memory region summary contracts
//! - firmware and topology fact contracts
//!
//! This module may discover and report authoritative boot-stage hardware facts,
//! but it must not redefine the long-term kernel ownership model that consumes
//! those facts.

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, __cpuid_count};

use platform_x86_64::{
    BootInfo, BootMemoryRegionKind, acpi_probe_info, acpi_probe_signatures, acpi_root_info,
    apic_topology,
};

use crate::serial;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegionSummary {
    pub usable_regions: usize,
    pub usable_bytes: u64,
    pub reserved_regions: usize,
    pub reserved_bytes: u64,
    pub reclaimable_regions: usize,
    pub reclaimable_bytes: u64,
    pub acpi_regions: usize,
    pub acpi_bytes: u64,
    pub bad_regions: usize,
    pub bad_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuFacts {
    pub vendor_kind: CpuVendor,
    pub vendor: [u8; 12],
    pub max_basic_leaf: u32,
    pub max_extended_leaf: u32,
    pub family: u8,
    pub model: u8,
    pub stepping: u8,
    pub apic_id: u8,
    pub has_tsc: bool,
    pub has_invariant_tsc: bool,
    pub has_syscall_sysret: bool,
    pub has_nx: bool,
    pub has_1g_pages: bool,
    pub has_xsave: bool,
    pub has_osxsave: bool,
    pub has_x2apic: bool,
    pub has_pcid: bool,
    pub has_invpcid: bool,
    pub has_fsgsbase: bool,
    pub has_smep: bool,
    pub has_smap: bool,
    pub has_umip: bool,
    pub has_pku: bool,
    pub has_ospke: bool,
    pub has_la57: bool,
    pub has_rdpid: bool,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512f: bool,
    pub max_xsave_bytes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    Intel,
    Amd,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFeaturePolicy {
    pub vendor: CpuVendor,
    pub enable_xsave: bool,
    pub enable_x2apic: bool,
    pub enable_fsgsbase: bool,
    pub enable_pcid: bool,
    pub enable_invpcid: bool,
    pub enable_smep: bool,
    pub enable_smap: bool,
    pub enable_umip: bool,
    pub enable_pku: bool,
    pub enable_la57: bool,
    pub allow_avx_user_state: bool,
    pub allow_avx512_user_state: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiFacts {
    pub revision: u8,
    pub rsdt_address: u32,
    pub xsdt_address: u64,
    pub uses_xsdt: bool,
    pub table_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmpFacts {
    pub bootstrap_apic_id: u32,
    pub local_apic_address: u64,
    pub processor_count: usize,
    pub enabled_processor_count: usize,
    pub online_capable_count: usize,
    pub io_apic_count: usize,
    pub interrupt_override_count: usize,
}

pub fn emit_boot_facts(boot_info: &BootInfo<'_>) {
    let mut memory = MemoryRegionSummary {
        usable_regions: 0,
        usable_bytes: 0,
        reserved_regions: 0,
        reserved_bytes: 0,
        reclaimable_regions: 0,
        reclaimable_bytes: 0,
        acpi_regions: 0,
        acpi_bytes: 0,
        bad_regions: 0,
        bad_bytes: 0,
    };
    summarize_memory_map_into(boot_info, &mut memory);
    serial::print(format_args!(
        "ngos/x86_64: memory usable={} regions/{:#x} bytes reserved={} regions/{:#x} bytes acpi={} regions/{:#x} bytes reclaimable={} regions/{:#x} bytes bad={} regions/{:#x} bytes\n",
        memory.usable_regions,
        memory.usable_bytes,
        memory.reserved_regions,
        memory.reserved_bytes,
        memory.acpi_regions,
        memory.acpi_bytes,
        memory.reclaimable_regions,
        memory.reclaimable_bytes,
        memory.bad_regions,
        memory.bad_bytes
    ));

    #[cfg(target_arch = "x86_64")]
    {
        let cpu = cpu_facts();
        let policy = cpu_feature_policy(&cpu);
        let vendor = core::str::from_utf8(&cpu.vendor).unwrap_or("unknown");
        serial::print(format_args!(
            "ngos/x86_64: cpu vendor={} family={} model={} stepping={} apic_id={} cpuid_basic={:#x} cpuid_ext={:#x}\n",
            vendor,
            cpu.family,
            cpu.model,
            cpu.stepping,
            cpu.apic_id,
            cpu.max_basic_leaf,
            cpu.max_extended_leaf
        ));
        serial::print(format_args!(
            "ngos/x86_64: cpu features tsc={} invariant_tsc={} syscall={} nx={} page1g={}\n",
            cpu.has_tsc,
            cpu.has_invariant_tsc,
            cpu.has_syscall_sysret,
            cpu.has_nx,
            cpu.has_1g_pages
        ));
        serial::print(format_args!(
            "ngos/x86_64: cpu modern xsave={} osxsave={} xsave_max={} x2apic={} pcid={} invpcid={} fsgsbase={} smep={} smap={} umip={} pku={} ospke={} la57={} rdpid={} avx={} avx2={} avx512f={}\n",
            cpu.has_xsave,
            cpu.has_osxsave,
            cpu.max_xsave_bytes,
            cpu.has_x2apic,
            cpu.has_pcid,
            cpu.has_invpcid,
            cpu.has_fsgsbase,
            cpu.has_smep,
            cpu.has_smap,
            cpu.has_umip,
            cpu.has_pku,
            cpu.has_ospke,
            cpu.has_la57,
            cpu.has_rdpid,
            cpu.has_avx,
            cpu.has_avx2,
            cpu.has_avx512f
        ));
        serial::print(format_args!(
            "ngos/x86_64: cpu policy vendor={:?} xsave={} x2apic={} fsgsbase={} pcid={} invpcid={} smep={} smap={} umip={} pku={} la57={} avx={} avx512={}\n",
            policy.vendor,
            policy.enable_xsave,
            policy.enable_x2apic,
            policy.enable_fsgsbase,
            policy.enable_pcid,
            policy.enable_invpcid,
            policy.enable_smep,
            policy.enable_smap,
            policy.enable_umip,
            policy.enable_pku,
            policy.enable_la57,
            policy.allow_avx_user_state,
            policy.allow_avx512_user_state
        ));
    }

    if let Some(acpi) = acpi_facts(boot_info) {
        serial::print(format_args!(
            "ngos/x86_64: acpi revision={} rsdt={:#x} xsdt={:#x} root={} tables={}\n",
            acpi.revision,
            acpi.rsdt_address,
            acpi.xsdt_address,
            if acpi.uses_xsdt { "xsdt" } else { "rsdt" },
            acpi.table_count
        ));
    } else if let Some(rsdp) = boot_info.rsdp {
        serial::print(format_args!(
            "ngos/x86_64: acpi rsdp present at {:#x} but root table is unavailable\n",
            rsdp
        ));
        if let Some((supplied_signature, physical_signature)) = acpi_probe_signatures(boot_info) {
            serial::print(format_args!(
                "ngos/x86_64: acpi probe raw supplied_sig={:?} physical_sig={:?} physical_candidate={:#x}\n",
                supplied_signature,
                physical_signature,
                rsdp.saturating_sub(boot_info.physical_memory_offset),
            ));
        }
        if let Some(probe) = acpi_probe_info(boot_info) {
            let rsdp_resolved = probe.rsdp_resolved.unwrap_or(0);
            serial::print(format_args!(
                "ngos/x86_64: acpi probe rsdp_supplied={:#x} rsdp_resolved={:#x} direct_map={} supplied_sig={:?} physical_sig={:?} rev={} rsdt={:#x} xsdt={:#x} xsdt_sig={:?} xsdt_len={:#x}\n",
                probe.rsdp_supplied,
                rsdp_resolved,
                probe.used_direct_map,
                probe.supplied_signature,
                probe.physical_signature,
                probe.rsdp_revision,
                probe.rsdt_address,
                probe.xsdt_address,
                probe.xsdt_signature,
                probe.xsdt_length.unwrap_or(0),
            ));
        }
    }
}

pub fn emit_smp_facts(boot_info: &BootInfo<'_>) {
    if let Some(smp) = smp_facts(boot_info) {
        serial::print(format_args!(
            "ngos/x86_64: smp bootstrap_apic={} cpus={} enabled={} online_capable={} lapic={:#x} ioapics={} overrides={}\n",
            smp.bootstrap_apic_id,
            smp.processor_count,
            smp.enabled_processor_count,
            smp.online_capable_count,
            smp.local_apic_address,
            smp.io_apic_count,
            smp.interrupt_override_count
        ));
    }
}

#[allow(dead_code)]
pub fn summarize_memory_map(boot_info: &BootInfo<'_>) -> MemoryRegionSummary {
    let mut summary = MemoryRegionSummary {
        usable_regions: 0,
        usable_bytes: 0,
        reserved_regions: 0,
        reserved_bytes: 0,
        reclaimable_regions: 0,
        reclaimable_bytes: 0,
        acpi_regions: 0,
        acpi_bytes: 0,
        bad_regions: 0,
        bad_bytes: 0,
    };
    summarize_memory_map_into(boot_info, &mut summary);
    summary
}

pub fn summarize_memory_map_into(boot_info: &BootInfo<'_>, summary: &mut MemoryRegionSummary) {
    summary.usable_regions = 0;
    summary.usable_bytes = 0;
    summary.reserved_regions = 0;
    summary.reserved_bytes = 0;
    summary.reclaimable_regions = 0;
    summary.reclaimable_bytes = 0;
    summary.acpi_regions = 0;
    summary.acpi_bytes = 0;
    summary.bad_regions = 0;
    summary.bad_bytes = 0;
    for region in boot_info.memory_regions {
        match region.kind {
            BootMemoryRegionKind::Usable => {
                summary.usable_regions += 1;
                summary.usable_bytes = summary.usable_bytes.saturating_add(region.len);
            }
            BootMemoryRegionKind::Reserved
            | BootMemoryRegionKind::KernelImage
            | BootMemoryRegionKind::Framebuffer => {
                summary.reserved_regions += 1;
                summary.reserved_bytes = summary.reserved_bytes.saturating_add(region.len);
            }
            BootMemoryRegionKind::BootloaderReclaimable => {
                summary.reclaimable_regions += 1;
                summary.reclaimable_bytes = summary.reclaimable_bytes.saturating_add(region.len);
            }
            BootMemoryRegionKind::AcpiReclaimable | BootMemoryRegionKind::AcpiNvs => {
                summary.acpi_regions += 1;
                summary.acpi_bytes = summary.acpi_bytes.saturating_add(region.len);
            }
            BootMemoryRegionKind::BadMemory | BootMemoryRegionKind::Mmio => {
                summary.bad_regions += 1;
                summary.bad_bytes = summary.bad_bytes.saturating_add(region.len);
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub fn cpu_facts() -> CpuFacts {
    let basic = __cpuid(0);
    let extended = __cpuid(0x8000_0000);
    let signature = __cpuid(1);
    let structured_features = if basic.eax >= 7 {
        Some(__cpuid_count(7, 0))
    } else {
        None
    };
    let xsave_leaf0 = if (signature.ecx & (1 << 26)) != 0 && basic.eax >= 0x0d {
        Some(__cpuid_count(0x0d, 0))
    } else {
        None
    };
    let extended_features = if extended.eax >= 0x8000_0001 {
        Some(__cpuid(0x8000_0001))
    } else {
        None
    };
    let power = if extended.eax >= 0x8000_0007 {
        Some(__cpuid(0x8000_0007))
    } else {
        None
    };
    let family_id = ((signature.eax >> 8) & 0x0f) as u8;
    let model_id = ((signature.eax >> 4) & 0x0f) as u8;
    let ext_family = ((signature.eax >> 20) & 0xff) as u8;
    let ext_model = ((signature.eax >> 16) & 0x0f) as u8;
    let family = if family_id == 0x0f {
        family_id.wrapping_add(ext_family)
    } else {
        family_id
    };
    let model = if family_id == 0x06 || family_id == 0x0f {
        (ext_model << 4) | model_id
    } else {
        model_id
    };

    CpuFacts {
        vendor_kind: cpu_vendor(vendor_bytes(basic.ebx, basic.edx, basic.ecx)),
        vendor: vendor_bytes(basic.ebx, basic.edx, basic.ecx),
        max_basic_leaf: basic.eax,
        max_extended_leaf: extended.eax,
        family,
        model,
        stepping: (signature.eax & 0x0f) as u8,
        apic_id: (signature.ebx >> 24) as u8,
        has_tsc: (signature.edx & (1 << 4)) != 0,
        has_invariant_tsc: power.is_some_and(|leaf| (leaf.edx & (1 << 8)) != 0),
        has_syscall_sysret: extended_features.is_some_and(|leaf| (leaf.edx & (1 << 11)) != 0),
        has_nx: extended_features.is_some_and(|leaf| (leaf.edx & (1 << 20)) != 0),
        has_1g_pages: extended_features.is_some_and(|leaf| (leaf.edx & (1 << 26)) != 0),
        has_xsave: (signature.ecx & (1 << 26)) != 0,
        has_osxsave: (signature.ecx & (1 << 27)) != 0,
        has_x2apic: (signature.ecx & (1 << 21)) != 0,
        has_pcid: (signature.ecx & (1 << 17)) != 0,
        has_invpcid: structured_features.is_some_and(|leaf| (leaf.ebx & (1 << 10)) != 0),
        has_fsgsbase: structured_features.is_some_and(|leaf| (leaf.ebx & (1 << 0)) != 0),
        has_smep: structured_features.is_some_and(|leaf| (leaf.ebx & (1 << 7)) != 0),
        has_smap: structured_features.is_some_and(|leaf| (leaf.ebx & (1 << 20)) != 0),
        has_umip: structured_features.is_some_and(|leaf| (leaf.ecx & (1 << 2)) != 0),
        has_pku: structured_features.is_some_and(|leaf| (leaf.ecx & (1 << 3)) != 0),
        has_ospke: structured_features.is_some_and(|leaf| (leaf.ecx & (1 << 4)) != 0),
        has_la57: structured_features.is_some_and(|leaf| (leaf.ecx & (1 << 16)) != 0),
        has_rdpid: structured_features.is_some_and(|leaf| (leaf.ecx & (1 << 22)) != 0),
        has_avx: (signature.ecx & (1 << 28)) != 0,
        has_avx2: structured_features.is_some_and(|leaf| (leaf.ebx & (1 << 5)) != 0),
        has_avx512f: structured_features.is_some_and(|leaf| (leaf.ebx & (1 << 16)) != 0),
        max_xsave_bytes: xsave_leaf0.map_or(0, |leaf| leaf.ebx),
    }
}

#[cfg(target_arch = "x86_64")]
pub fn cpu_feature_policy(cpu: &CpuFacts) -> CpuFeaturePolicy {
    let enable_xsave = cpu.has_xsave && cpu.has_osxsave && cpu.max_xsave_bytes != 0;
    let allow_avx_user_state = enable_xsave && cpu.has_avx;
    let allow_avx512_user_state = allow_avx_user_state && cpu.has_avx512f;
    match cpu.vendor_kind {
        CpuVendor::Intel => CpuFeaturePolicy {
            vendor: cpu.vendor_kind,
            enable_xsave,
            enable_x2apic: cpu.has_x2apic,
            enable_fsgsbase: cpu.has_fsgsbase,
            enable_pcid: cpu.has_pcid,
            enable_invpcid: cpu.has_pcid && cpu.has_invpcid,
            enable_smep: cpu.has_smep,
            enable_smap: cpu.has_smap,
            enable_umip: cpu.has_umip,
            enable_pku: cpu.has_pku && cpu.has_ospke,
            enable_la57: false,
            allow_avx_user_state,
            allow_avx512_user_state,
        },
        CpuVendor::Amd => CpuFeaturePolicy {
            vendor: cpu.vendor_kind,
            enable_xsave,
            enable_x2apic: cpu.has_x2apic,
            enable_fsgsbase: cpu.has_fsgsbase,
            enable_pcid: cpu.has_pcid,
            enable_invpcid: cpu.has_pcid && cpu.has_invpcid,
            enable_smep: cpu.has_smep,
            enable_smap: cpu.has_smap,
            enable_umip: cpu.has_umip,
            enable_pku: cpu.has_pku && cpu.has_ospke,
            enable_la57: false,
            allow_avx_user_state,
            allow_avx512_user_state,
        },
        CpuVendor::Unknown => CpuFeaturePolicy {
            vendor: cpu.vendor_kind,
            enable_xsave,
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
        },
    }
}

#[cfg(target_arch = "x86_64")]
const fn vendor_bytes(ebx: u32, edx: u32, ecx: u32) -> [u8; 12] {
    let ebx = ebx.to_le_bytes();
    let edx = edx.to_le_bytes();
    let ecx = ecx.to_le_bytes();
    [
        ebx[0], ebx[1], ebx[2], ebx[3], edx[0], edx[1], edx[2], edx[3], ecx[0], ecx[1], ecx[2],
        ecx[3],
    ]
}

#[cfg(target_arch = "x86_64")]
const fn vendor_matches(vendor: [u8; 12], expected: &[u8; 12]) -> bool {
    let mut index = 0;
    while index < 12 {
        if vendor[index] != expected[index] {
            return false;
        }
        index += 1;
    }
    true
}

#[cfg(target_arch = "x86_64")]
const fn cpu_vendor(vendor: [u8; 12]) -> CpuVendor {
    if vendor_matches(vendor, b"GenuineIntel") {
        CpuVendor::Intel
    } else if vendor_matches(vendor, b"AuthenticAMD") {
        CpuVendor::Amd
    } else {
        CpuVendor::Unknown
    }
}

pub fn acpi_facts(boot_info: &BootInfo<'_>) -> Option<AcpiFacts> {
    let root = acpi_root_info(boot_info)?;
    Some(AcpiFacts {
        revision: root.revision,
        rsdt_address: root.rsdt_address,
        xsdt_address: root.xsdt_address,
        uses_xsdt: root.uses_xsdt,
        table_count: root.table_count,
    })
}

#[cfg(target_arch = "x86_64")]
pub fn smp_facts(boot_info: &BootInfo<'_>) -> Option<SmpFacts> {
    let cpu = cpu_facts();
    let topology = apic_topology(boot_info, u32::from(cpu.apic_id))?;
    let enabled_processor_count = topology.processors.iter().filter(|cpu| cpu.enabled).count();
    let online_capable_count = topology
        .processors
        .iter()
        .filter(|cpu| cpu.online_capable)
        .count();
    Some(SmpFacts {
        bootstrap_apic_id: u32::from(cpu.apic_id),
        local_apic_address: topology.local_apic_address,
        processor_count: topology.processors.len(),
        enabled_processor_count,
        online_capable_count,
        io_apic_count: topology.io_apics.len(),
        interrupt_override_count: topology.interrupt_overrides.len(),
    })
}

#[allow(dead_code)]
#[repr(C, packed)]
struct RsdpV2 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[allow(dead_code)]
#[repr(C, packed)]
struct AcpiSdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_x86_64::{BootInfo, BootMemoryRegion, BootProtocol};

    #[test]
    fn summarize_memory_map_tracks_region_classes() {
        let regions = [
            BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::Usable,
            },
            BootMemoryRegion {
                start: 0x1000,
                len: 0x1000,
                kind: BootMemoryRegionKind::Reserved,
            },
            BootMemoryRegion {
                start: 0x2000,
                len: 0x3000,
                kind: BootMemoryRegionKind::BootloaderReclaimable,
            },
            BootMemoryRegion {
                start: 0x5000,
                len: 0x2000,
                kind: BootMemoryRegionKind::AcpiNvs,
            },
            BootMemoryRegion {
                start: 0x7000,
                len: 0x1000,
                kind: BootMemoryRegionKind::BadMemory,
            },
        ];
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: None,
            memory_regions: &regions,
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: BootMemoryRegion {
                start: 0x9000,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };

        let summary = summarize_memory_map(&boot_info);
        assert_eq!(summary.usable_regions, 1);
        assert_eq!(summary.usable_bytes, 0x1000);
        assert_eq!(summary.reserved_regions, 1);
        assert_eq!(summary.reclaimable_regions, 1);
        assert_eq!(summary.reclaimable_bytes, 0x3000);
        assert_eq!(summary.acpi_regions, 1);
        assert_eq!(summary.acpi_bytes, 0x2000);
        assert_eq!(summary.bad_regions, 1);
        assert_eq!(summary.bad_bytes, 0x1000);
    }

    #[test]
    fn acpi_facts_parse_xsdt_entry_count() {
        let mut backing = [0u8; 256];
        let rsdp_ptr = backing.as_mut_ptr() as usize;
        let xsdt_ptr = rsdp_ptr + 64;

        unsafe {
            ptr::write(
                rsdp_ptr as *mut RsdpV2,
                RsdpV2 {
                    signature: *b"RSD PTR ",
                    checksum: 0,
                    oem_id: *b"NGOS  ",
                    revision: 2,
                    rsdt_address: 0,
                    length: size_of::<RsdpV2>() as u32,
                    xsdt_address: xsdt_ptr as u64,
                    extended_checksum: 0,
                    reserved: [0; 3],
                },
            );
            ptr::write(
                xsdt_ptr as *mut AcpiSdtHeader,
                AcpiSdtHeader {
                    signature: *b"XSDT",
                    length: (size_of::<AcpiSdtHeader>() + 3 * size_of::<u64>()) as u32,
                    revision: 1,
                    checksum: 0,
                    oem_id: *b"NGOS  ",
                    oem_table_id: *b"BOOTFACT",
                    oem_revision: 1,
                    creator_id: 0,
                    creator_revision: 0,
                },
            );
        }

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: Some(rsdp_ptr as u64),
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };

        let facts = acpi_facts(&boot_info).expect("expected valid xsdt");
        assert!(facts.uses_xsdt);
        assert_eq!(facts.table_count, 3);
    }

    #[test]
    fn smp_facts_reports_processor_and_ioapic_counts() {
        let mut backing = [0u8; 512];
        let rsdp_ptr = backing.as_mut_ptr() as usize;
        let xsdt_ptr = rsdp_ptr + 64;
        let madt_ptr = rsdp_ptr + 128;

        unsafe {
            ptr::write(
                rsdp_ptr as *mut RsdpV2,
                RsdpV2 {
                    signature: *b"RSD PTR ",
                    checksum: 0,
                    oem_id: *b"NGOS  ",
                    revision: 2,
                    rsdt_address: 0,
                    length: size_of::<RsdpV2>() as u32,
                    xsdt_address: xsdt_ptr as u64,
                    extended_checksum: 0,
                    reserved: [0; 3],
                },
            );
            ptr::write(
                xsdt_ptr as *mut AcpiSdtHeader,
                AcpiSdtHeader {
                    signature: *b"XSDT",
                    length: (size_of::<AcpiSdtHeader>() + size_of::<u64>()) as u32,
                    revision: 1,
                    checksum: 0,
                    oem_id: *b"NGOS  ",
                    oem_table_id: *b"BOOTFACT",
                    oem_revision: 1,
                    creator_id: 0,
                    creator_revision: 0,
                },
            );
            ptr::write(
                (xsdt_ptr + size_of::<AcpiSdtHeader>()) as *mut u64,
                madt_ptr as u64,
            );
            ptr::write_bytes(madt_ptr as *mut u8, 0, 128);
            ptr::copy_nonoverlapping(b"APIC".as_ptr(), madt_ptr as *mut u8, 4);
            ptr::write((madt_ptr + 4) as *mut u32, (44 + 8 + 8 + 12 + 10) as u32);
            ptr::write((madt_ptr + 36) as *mut u32, 0xfee0_0000);
            ptr::write((madt_ptr + 40) as *mut u32, 1);

            let mut entry = madt_ptr + 44;
            ptr::write(entry as *mut u8, 0);
            ptr::write((entry + 1) as *mut u8, 8);
            ptr::write((entry + 2) as *mut u8, 0);
            ptr::write((entry + 3) as *mut u8, 0);
            ptr::write((entry + 4) as *mut u32, 1);
            entry += 8;
            ptr::write(entry as *mut u8, 0);
            ptr::write((entry + 1) as *mut u8, 8);
            ptr::write((entry + 2) as *mut u8, 1);
            ptr::write((entry + 3) as *mut u8, 1);
            ptr::write((entry + 4) as *mut u32, 3);
            entry += 8;
            ptr::write(entry as *mut u8, 1);
            ptr::write((entry + 1) as *mut u8, 12);
            ptr::write((entry + 2) as *mut u8, 7);
            ptr::write((entry + 3) as *mut u8, 0);
            ptr::write((entry + 4) as *mut u32, 0xfec0_0000);
            ptr::write((entry + 8) as *mut u32, 0);
            entry += 12;
            ptr::write(entry as *mut u8, 2);
            ptr::write((entry + 1) as *mut u8, 10);
            ptr::write((entry + 2) as *mut u8, 0);
            ptr::write((entry + 3) as *mut u8, 1);
            ptr::write((entry + 4) as *mut u32, 33);
            ptr::write((entry + 8) as *mut u16, 0x0d);
        }

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: Some(rsdp_ptr as u64),
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };

        let facts = smp_facts(&boot_info).expect("expected smp facts");
        assert_eq!(facts.bootstrap_apic_id, 0);
        assert_eq!(facts.processor_count, 2);
        assert_eq!(facts.enabled_processor_count, 2);
        assert_eq!(facts.online_capable_count, 1);
        assert_eq!(facts.io_apic_count, 1);
        assert_eq!(facts.interrupt_override_count, 1);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn cpu_vendor_classifies_intel_and_amd_signatures() {
        assert_eq!(cpu_vendor(*b"GenuineIntel"), CpuVendor::Intel);
        assert_eq!(cpu_vendor(*b"AuthenticAMD"), CpuVendor::Amd);
        assert_eq!(cpu_vendor(*b"UnknownVendr"), CpuVendor::Unknown);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn cpu_feature_policy_is_vendor_explicit_and_conservative() {
        let cpu = CpuFacts {
            vendor_kind: CpuVendor::Amd,
            vendor: *b"AuthenticAMD",
            max_basic_leaf: 7,
            max_extended_leaf: 0x8000_0007,
            family: 0x19,
            model: 0x61,
            stepping: 1,
            apic_id: 0,
            has_tsc: true,
            has_invariant_tsc: true,
            has_syscall_sysret: true,
            has_nx: true,
            has_1g_pages: true,
            has_xsave: true,
            has_osxsave: true,
            has_x2apic: true,
            has_pcid: true,
            has_invpcid: true,
            has_fsgsbase: true,
            has_smep: true,
            has_smap: true,
            has_umip: true,
            has_pku: true,
            has_ospke: true,
            has_la57: true,
            has_rdpid: true,
            has_avx: true,
            has_avx2: true,
            has_avx512f: false,
            max_xsave_bytes: 4096,
        };
        let policy = cpu_feature_policy(&cpu);
        assert_eq!(policy.vendor, CpuVendor::Amd);
        assert!(policy.enable_xsave);
        assert!(policy.enable_pcid);
        assert!(policy.enable_invpcid);
        assert!(policy.enable_smep);
        assert!(policy.enable_smap);
        assert!(policy.enable_umip);
        assert!(policy.enable_pku);
        assert!(!policy.enable_la57);
        assert!(policy.allow_avx_user_state);
        assert!(!policy.allow_avx512_user_state);
    }
}
