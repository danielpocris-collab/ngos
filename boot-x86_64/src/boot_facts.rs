#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::__cpuid;

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
}
