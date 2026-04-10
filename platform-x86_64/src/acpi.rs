//! Canonical subsystem role:
//! - subsystem: x86_64 firmware and ACPI mediation
//! - owner layer: platform mediation
//! - semantic owner: `platform-x86_64`
//! - truth path role: platform-specific discovery of ACPI and firmware table
//!   mechanics on the real x86 path
//!
//! Canonical contract families handled here:
//! - ACPI root discovery contracts
//! - firmware table probe contracts
//! - topology/firmware mediation contracts
//!
//! This module may mediate firmware and ACPI mechanics, but it must not
//! redefine higher-level kernel ownership of topology or device semantics.

use alloc::vec::Vec;
use core::{mem::size_of, ptr};

use crate::BootInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiRootInfo {
    pub revision: u8,
    pub rsdt_address: u32,
    pub xsdt_address: u64,
    pub uses_xsdt: bool,
    pub table_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiProbeInfo {
    pub rsdp_supplied: u64,
    pub rsdp_resolved: Option<u64>,
    pub rsdp_revision: u8,
    pub rsdt_address: u32,
    pub xsdt_address: u64,
    pub used_direct_map: bool,
    pub xsdt_signature: Option<[u8; 4]>,
    pub xsdt_length: Option<u32>,
    pub supplied_signature: Option<[u8; 8]>,
    pub physical_signature: Option<[u8; 8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessorTopologyEntry {
    pub processor_uid: u32,
    pub apic_id: u32,
    pub enabled: bool,
    pub online_capable: bool,
    pub is_bootstrap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoApicEntry {
    pub io_apic_id: u8,
    pub address: u32,
    pub gsi_base: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptSourceOverride {
    pub bus: u8,
    pub source: u8,
    pub gsi: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApicTopologyInfo {
    pub local_apic_address: u64,
    pub processors: Vec<ProcessorTopologyEntry>,
    pub io_apics: Vec<IoApicEntry>,
    pub interrupt_overrides: Vec<InterruptSourceOverride>,
}

pub fn acpi_root_info(boot_info: &BootInfo<'_>) -> Option<AcpiRootInfo> {
    let rsdp = resolve_rsdp(boot_info)?;

    let uses_xsdt = rsdp.revision >= 2 && rsdp.xsdt_address != 0;
    if uses_xsdt {
        let xsdt_virt = rsdp
            .xsdt_address
            .checked_add(boot_info.physical_memory_offset)?;
        let header = read_unaligned_copy::<AcpiSdtHeader>(xsdt_virt)?;
        if header.signature != *b"XSDT" || header.length < size_of::<AcpiSdtHeader>() as u32 {
            return None;
        }
        let payload = usize::try_from(header.length).ok()? - size_of::<AcpiSdtHeader>();
        return Some(AcpiRootInfo {
            revision: rsdp.revision,
            rsdt_address: rsdp.rsdt_address,
            xsdt_address: rsdp.xsdt_address,
            uses_xsdt: true,
            table_count: payload / size_of::<u64>(),
        });
    }

    if rsdp.rsdt_address == 0 {
        return None;
    }
    let rsdt_virt = u64::from(rsdp.rsdt_address).checked_add(boot_info.physical_memory_offset)?;
    let header = read_unaligned_copy::<AcpiSdtHeader>(rsdt_virt)?;
    if header.signature != *b"RSDT" || header.length < size_of::<AcpiSdtHeader>() as u32 {
        return None;
    }
    let payload = usize::try_from(header.length).ok()? - size_of::<AcpiSdtHeader>();
    Some(AcpiRootInfo {
        revision: rsdp.revision,
        rsdt_address: rsdp.rsdt_address,
        xsdt_address: rsdp.xsdt_address,
        uses_xsdt: false,
        table_count: payload / size_of::<u32>(),
    })
}

pub fn acpi_probe_info(boot_info: &BootInfo<'_>) -> Option<AcpiProbeInfo> {
    let rsdp_supplied = boot_info.rsdp?;
    let physical_candidate = rsdp_supplied.checked_sub(boot_info.physical_memory_offset);
    let supplied_signature = read_unaligned_copy::<[u8; 8]>(rsdp_supplied);
    let physical_signature = physical_candidate.and_then(read_unaligned_copy::<[u8; 8]>);
    let (rsdp, used_direct_map, rsdp_resolved) = resolve_rsdp_with_source(boot_info)?;
    let mut probe = AcpiProbeInfo {
        rsdp_supplied,
        rsdp_resolved: Some(rsdp_resolved),
        rsdp_revision: rsdp.revision,
        rsdt_address: rsdp.rsdt_address,
        xsdt_address: rsdp.xsdt_address,
        used_direct_map,
        xsdt_signature: None,
        xsdt_length: None,
        supplied_signature,
        physical_signature,
    };
    if rsdp.revision >= 2
        && rsdp.xsdt_address != 0
        && let Some(xsdt_virt) = rsdp
            .xsdt_address
            .checked_add(boot_info.physical_memory_offset)
        && let Some(header) = read_unaligned_copy::<AcpiSdtHeader>(xsdt_virt)
    {
        probe.xsdt_signature = Some(header.signature);
        probe.xsdt_length = Some(header.length);
    }
    Some(probe)
}

pub fn apic_topology(boot_info: &BootInfo<'_>, bootstrap_apic_id: u32) -> Option<ApicTopologyInfo> {
    let rsdp = resolve_rsdp(boot_info)?;

    let madt_virt = find_table(boot_info, &rsdp, *b"APIC")?;
    let madt = read_unaligned_copy::<AcpiSdtHeader>(madt_virt)?;
    if madt.length < size_of::<MadtHeader>() as u32 {
        return None;
    }

    let madt_len = usize::try_from(madt.length).ok()?;
    let mut local_apic_address = unsafe {
        ptr_read_unaligned_u32(
            (usize::try_from(madt_virt).ok()? as *const u8).add(size_of::<AcpiSdtHeader>()),
        )
    } as u64;
    let mut processors = Vec::new();
    let mut io_apics = Vec::new();
    let mut interrupt_overrides = Vec::new();

    let mut offset = size_of::<MadtHeader>();
    let madt_ptr = usize::try_from(madt_virt).ok()? as *const u8;
    while offset + size_of::<MadtEntryHeader>() <= madt_len {
        let entry_ptr = unsafe { madt_ptr.add(offset) };
        let entry = unsafe { &*(entry_ptr as *const MadtEntryHeader) };
        let entry_len = usize::from(entry.length);
        if entry_len < size_of::<MadtEntryHeader>() || offset + entry_len > madt_len {
            break;
        }

        match entry.entry_type {
            0 if entry_len >= size_of::<MadtLocalApic>() => {
                let local = unsafe { &*(entry_ptr as *const MadtLocalApic) };
                let enabled = (local.flags & 0x1) != 0;
                let online_capable = (local.flags & 0x2) != 0;
                processors.push(ProcessorTopologyEntry {
                    processor_uid: u32::from(local.processor_uid),
                    apic_id: u32::from(local.apic_id),
                    enabled,
                    online_capable,
                    is_bootstrap: u32::from(local.apic_id) == bootstrap_apic_id,
                });
            }
            1 if entry_len >= size_of::<MadtIoApic>() => {
                let io_apic = unsafe { &*(entry_ptr as *const MadtIoApic) };
                io_apics.push(IoApicEntry {
                    io_apic_id: io_apic.io_apic_id,
                    address: io_apic.address,
                    gsi_base: io_apic.gsi_base,
                });
            }
            2 if entry_len >= size_of::<MadtInterruptSourceOverride>() => {
                let iso = unsafe { &*(entry_ptr as *const MadtInterruptSourceOverride) };
                interrupt_overrides.push(InterruptSourceOverride {
                    bus: iso.bus,
                    source: iso.source,
                    gsi: iso.gsi,
                    flags: iso.flags,
                });
            }
            5 if entry_len >= size_of::<MadtLocalApicAddressOverride>() => {
                let override_entry =
                    unsafe { &*(entry_ptr as *const MadtLocalApicAddressOverride) };
                local_apic_address = override_entry.address;
            }
            9 if entry_len >= size_of::<MadtLocalX2Apic>() => {
                let x2apic = unsafe { &*(entry_ptr as *const MadtLocalX2Apic) };
                let enabled = (x2apic.flags & 0x1) != 0;
                let online_capable = (x2apic.flags & 0x2) != 0;
                processors.push(ProcessorTopologyEntry {
                    processor_uid: x2apic.processor_uid,
                    apic_id: x2apic.x2apic_id,
                    enabled,
                    online_capable,
                    is_bootstrap: x2apic.x2apic_id == bootstrap_apic_id,
                });
            }
            _ => {}
        }

        offset += entry_len;
    }

    Some(ApicTopologyInfo {
        local_apic_address,
        processors,
        io_apics,
        interrupt_overrides,
    })
}

fn resolve_rsdp(boot_info: &BootInfo<'_>) -> Option<RsdpV2> {
    resolve_rsdp_with_source(boot_info).map(|(rsdp, _, _)| rsdp)
}

fn resolve_rsdp_with_source(boot_info: &BootInfo<'_>) -> Option<(RsdpV2, bool, u64)> {
    let rsdp = boot_info.rsdp?;
    let raw = read_unaligned_copy::<RsdpV2>(rsdp)?;
    if raw.signature == *b"RSD PTR " {
        return Some((raw, false, rsdp));
    }
    let direct_mapped = rsdp
        .checked_add(boot_info.physical_memory_offset)
        .and_then(read_unaligned_copy::<RsdpV2>);
    if let Some(candidate) = direct_mapped
        && candidate.signature == *b"RSD PTR "
    {
        return Some((
            candidate,
            true,
            rsdp.checked_add(boot_info.physical_memory_offset)?,
        ));
    }
    None
}

pub fn acpi_probe_signatures(
    boot_info: &BootInfo<'_>,
) -> Option<(Option<[u8; 8]>, Option<[u8; 8]>)> {
    let rsdp_supplied = boot_info.rsdp?;
    let physical_candidate = rsdp_supplied.checked_sub(boot_info.physical_memory_offset);
    Some((
        read_unaligned_copy::<[u8; 8]>(rsdp_supplied),
        physical_candidate.and_then(read_unaligned_copy::<[u8; 8]>),
    ))
}

fn find_table(boot_info: &BootInfo<'_>, rsdp: &RsdpV2, signature: [u8; 4]) -> Option<u64> {
    if rsdp.revision >= 2 && rsdp.xsdt_address != 0 {
        let xsdt_virt = rsdp
            .xsdt_address
            .checked_add(boot_info.physical_memory_offset)?;
        let xsdt = read_unaligned_copy::<AcpiSdtHeader>(xsdt_virt)?;
        if xsdt.signature != *b"XSDT" || xsdt.length < size_of::<AcpiSdtHeader>() as u32 {
            return None;
        }
        let payload = usize::try_from(xsdt.length).ok()? - size_of::<AcpiSdtHeader>();
        let count = payload / size_of::<u64>();
        let entries_ptr = unsafe {
            (usize::try_from(xsdt_virt).ok()? as *const u8).add(size_of::<AcpiSdtHeader>())
                as *const u64
        };
        for index in 0..count {
            let table_phys = unsafe { ptr_read_unaligned_u64(entries_ptr.add(index) as *const u8) };
            let table_virt = table_phys.checked_add(boot_info.physical_memory_offset)?;
            let header = read_unaligned_copy::<AcpiSdtHeader>(table_virt)?;
            if header.signature == signature {
                return Some(table_virt);
            }
        }
        return None;
    }

    if rsdp.rsdt_address == 0 {
        return None;
    }
    let rsdt_virt = u64::from(rsdp.rsdt_address).checked_add(boot_info.physical_memory_offset)?;
    let rsdt = read_unaligned_copy::<AcpiSdtHeader>(rsdt_virt)?;
    if rsdt.signature != *b"RSDT" || rsdt.length < size_of::<AcpiSdtHeader>() as u32 {
        return None;
    }
    let payload = usize::try_from(rsdt.length).ok()? - size_of::<AcpiSdtHeader>();
    let count = payload / size_of::<u32>();
    let entries_ptr = unsafe {
        (usize::try_from(rsdt_virt).ok()? as *const u8).add(size_of::<AcpiSdtHeader>())
            as *const u32
    };
    for index in 0..count {
        let table_phys =
            u64::from(unsafe { ptr_read_unaligned_u32(entries_ptr.add(index) as *const u8) });
        let table_virt = table_phys.checked_add(boot_info.physical_memory_offset)?;
        let header = read_unaligned_copy::<AcpiSdtHeader>(table_virt)?;
        if header.signature == signature {
            return Some(table_virt);
        }
    }
    None
}

unsafe fn ptr_read_unaligned_u32(ptr: *const u8) -> u32 {
    unsafe { core::ptr::read_unaligned(ptr as *const u32) }
}

unsafe fn ptr_read_unaligned_u64(ptr: *const u8) -> u64 {
    unsafe { core::ptr::read_unaligned(ptr as *const u64) }
}

fn read_unaligned_copy<T: Copy>(address: u64) -> Option<T> {
    let ptr = usize::try_from(address).ok()? as *const T;
    Some(unsafe { ptr::read_unaligned(ptr) })
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
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

#[repr(C, packed)]
#[derive(Clone, Copy)]
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

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtHeader {
    header: AcpiSdtHeader,
    local_apic_address: u32,
    flags: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtEntryHeader {
    entry_type: u8,
    length: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtLocalApic {
    header: MadtEntryHeader,
    processor_uid: u8,
    apic_id: u8,
    flags: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtIoApic {
    header: MadtEntryHeader,
    io_apic_id: u8,
    reserved: u8,
    address: u32,
    gsi_base: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtInterruptSourceOverride {
    header: MadtEntryHeader,
    bus: u8,
    source: u8,
    gsi: u32,
    flags: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtLocalApicAddressOverride {
    header: MadtEntryHeader,
    reserved: u16,
    address: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MadtLocalX2Apic {
    header: MadtEntryHeader,
    reserved: u16,
    x2apic_id: u32,
    flags: u32,
    processor_uid: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BootMemoryRegion, BootMemoryRegionKind, BootProtocol};
    use core::ptr;

    #[test]
    fn acpi_root_info_accepts_hhdm_mapped_rsdp_pointer() {
        let mut backing = [0u8; 256];
        let base = backing.as_mut_ptr() as usize;
        let hhdm = base as u64;
        let rsdp_phys = 0x40u64;
        let xsdt_phys = 0x80u64;
        let rsdp_ptr = (base + rsdp_phys as usize) as *mut RsdpV2;
        let xsdt_ptr = (base + xsdt_phys as usize) as *mut AcpiSdtHeader;

        unsafe {
            ptr::write(
                rsdp_ptr,
                RsdpV2 {
                    signature: *b"RSD PTR ",
                    checksum: 0,
                    oem_id: *b"NGOS  ",
                    revision: 2,
                    rsdt_address: 0,
                    length: size_of::<RsdpV2>() as u32,
                    xsdt_address: xsdt_phys,
                    extended_checksum: 0,
                    reserved: [0; 3],
                },
            );
            ptr::write(
                xsdt_ptr,
                AcpiSdtHeader {
                    signature: *b"XSDT",
                    length: size_of::<AcpiSdtHeader>() as u32,
                    revision: 1,
                    checksum: 0,
                    oem_id: *b"NGOS  ",
                    oem_table_id: *b"ACPITEST",
                    oem_revision: 1,
                    creator_id: 0,
                    creator_revision: 0,
                },
            );
        }

        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: Some(hhdm + rsdp_phys),
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset: hhdm,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };

        let root = acpi_root_info(&boot_info).expect("expected XSDT via hhdm-mapped rsdp");
        assert!(root.uses_xsdt);
        assert_eq!(root.xsdt_address, xsdt_phys);
    }

    #[test]
    fn apic_topology_parses_processors_ioapic_and_overrides() {
        let mut backing = [0u8; 512];
        let base = backing.as_mut_ptr() as usize;
        let rsdp_ptr = base;
        let xsdt_ptr = base + 64;
        let madt_ptr = base + 128;

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
                    oem_table_id: *b"SMPTEST ",
                    oem_revision: 1,
                    creator_id: 0,
                    creator_revision: 0,
                },
            );
            ptr::write(
                (xsdt_ptr + size_of::<AcpiSdtHeader>()) as *mut u64,
                madt_ptr as u64,
            );

            ptr::write(
                madt_ptr as *mut MadtHeader,
                MadtHeader {
                    header: AcpiSdtHeader {
                        signature: *b"APIC",
                        length: (size_of::<MadtHeader>()
                            + size_of::<MadtLocalApic>()
                            + size_of::<MadtLocalApic>()
                            + size_of::<MadtIoApic>()
                            + size_of::<MadtInterruptSourceOverride>()
                            + size_of::<MadtLocalApicAddressOverride>())
                            as u32,
                        revision: 1,
                        checksum: 0,
                        oem_id: *b"NGOS  ",
                        oem_table_id: *b"SMPTEST ",
                        oem_revision: 1,
                        creator_id: 0,
                        creator_revision: 0,
                    },
                    local_apic_address: 0xfee0_0000,
                    flags: 1,
                },
            );

            let mut offset = madt_ptr + size_of::<MadtHeader>();
            ptr::write(
                offset as *mut MadtLocalApic,
                MadtLocalApic {
                    header: MadtEntryHeader {
                        entry_type: 0,
                        length: size_of::<MadtLocalApic>() as u8,
                    },
                    processor_uid: 0,
                    apic_id: 2,
                    flags: 1,
                },
            );
            offset += size_of::<MadtLocalApic>();
            ptr::write(
                offset as *mut MadtLocalApic,
                MadtLocalApic {
                    header: MadtEntryHeader {
                        entry_type: 0,
                        length: size_of::<MadtLocalApic>() as u8,
                    },
                    processor_uid: 1,
                    apic_id: 4,
                    flags: 3,
                },
            );
            offset += size_of::<MadtLocalApic>();
            ptr::write(
                offset as *mut MadtIoApic,
                MadtIoApic {
                    header: MadtEntryHeader {
                        entry_type: 1,
                        length: size_of::<MadtIoApic>() as u8,
                    },
                    io_apic_id: 9,
                    reserved: 0,
                    address: 0xfec0_0000,
                    gsi_base: 0,
                },
            );
            offset += size_of::<MadtIoApic>();
            ptr::write(
                offset as *mut MadtInterruptSourceOverride,
                MadtInterruptSourceOverride {
                    header: MadtEntryHeader {
                        entry_type: 2,
                        length: size_of::<MadtInterruptSourceOverride>() as u8,
                    },
                    bus: 0,
                    source: 1,
                    gsi: 33,
                    flags: 0x0d,
                },
            );
            offset += size_of::<MadtInterruptSourceOverride>();
            ptr::write(
                offset as *mut MadtLocalApicAddressOverride,
                MadtLocalApicAddressOverride {
                    header: MadtEntryHeader {
                        entry_type: 5,
                        length: size_of::<MadtLocalApicAddressOverride>() as u8,
                    },
                    reserved: 0,
                    address: 0xfee0_1000,
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

        let topology = apic_topology(&boot_info, 2).expect("expected MADT topology");
        assert_eq!(topology.local_apic_address, 0xfee0_1000);
        assert_eq!(topology.processors.len(), 2);
        assert!(topology.processors[0].is_bootstrap);
        assert_eq!(topology.processors[1].processor_uid, 1);
        assert!(topology.processors[1].online_capable);
        assert_eq!(topology.io_apics[0].address, 0xfec0_0000);
        assert_eq!(topology.interrupt_overrides[0].gsi, 33);
    }
}
