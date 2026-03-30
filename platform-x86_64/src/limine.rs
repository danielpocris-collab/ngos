#[cfg(all(target_arch = "x86_64", not(test)))]
use core::arch::asm;
use core::ffi::CStr;

use limine::{
    file::File,
    framebuffer::Framebuffer,
    memory_map::{Entry, EntryType},
};

use crate::{
    BootMemoryRegion, BootMemoryRegionKind, BootModule, BootProtocol, FramebufferInfo,
    LoaderDefinedBootHandoff, PAGE_SIZE_4K, align_up,
};

pub const MAX_LIMINE_MEMORY_REGIONS: usize = 256;
pub const MAX_LIMINE_MODULES: usize = 32;

pub struct LimineBootSnapshot<'a> {
    pub command_line: Option<&'a CStr>,
    pub rsdp: Option<u64>,
    pub memory_map: &'a [&'a Entry],
    pub modules: &'a [&'a File],
    pub framebuffer: Option<Framebuffer<'a>>,
    pub physical_memory_offset: u64,
    pub kernel_physical_base: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimineHandoffError {
    TooManyMemoryRegions { count: usize, capacity: usize },
    TooManyModules { count: usize, capacity: usize },
    InvalidCommandLineUtf8,
    InvalidModulePathUtf8 { index: usize },
}

#[derive(Debug, Clone)]
pub struct LimineBootBuffers {
    memory_regions: [BootMemoryRegion; MAX_LIMINE_MEMORY_REGIONS],
    modules: [BootModule<'static>; MAX_LIMINE_MODULES],
}

impl LimineBootBuffers {
    pub const fn new() -> Self {
        const EMPTY_REGION: BootMemoryRegion = BootMemoryRegion {
            start: 0,
            len: 0,
            kind: BootMemoryRegionKind::Reserved,
        };
        const EMPTY_MODULE: BootModule<'static> = BootModule {
            name: "",
            physical_start: 0,
            len: 0,
        };

        Self {
            memory_regions: [EMPTY_REGION; MAX_LIMINE_MEMORY_REGIONS],
            modules: [EMPTY_MODULE; MAX_LIMINE_MODULES],
        }
    }

    pub fn build_loader_defined_handoff<'a>(
        &'a mut self,
        snapshot: LimineBootSnapshot<'static>,
        kernel_image_len: u64,
    ) -> Result<LoaderDefinedBootHandoff<'a>, LimineHandoffError> {
        debug_marker(b'M');
        if snapshot.memory_map.len() > self.memory_regions.len() {
            return Err(LimineHandoffError::TooManyMemoryRegions {
                count: snapshot.memory_map.len(),
                capacity: self.memory_regions.len(),
            });
        }
        if snapshot.modules.len() > self.modules.len() {
            return Err(LimineHandoffError::TooManyModules {
                count: snapshot.modules.len(),
                capacity: self.modules.len(),
            });
        }
        debug_marker(b'N');

        for (slot, entry) in self
            .memory_regions
            .iter_mut()
            .zip(snapshot.memory_map.iter().copied())
        {
            *slot = BootMemoryRegion {
                start: entry.base,
                len: entry.length,
                kind: limine_region_kind(entry.entry_type),
            };
        }
        debug_marker(b'O');

        for (index, (slot, module)) in self
            .modules
            .iter_mut()
            .zip(snapshot.modules.iter().copied())
            .enumerate()
        {
            let name = module
                .path()
                .to_str()
                .map_err(|_| LimineHandoffError::InvalidModulePathUtf8 { index })?;
            *slot = BootModule {
                name,
                physical_start: (module.addr() as u64)
                    .saturating_sub(snapshot.physical_memory_offset),
                len: module.size(),
            };
        }
        debug_marker(b'P');

        let command_line = snapshot
            .command_line
            .filter(|cmdline| !cmdline.to_bytes().is_empty())
            .map(|cmdline| {
                cmdline
                    .to_str()
                    .map_err(|_| LimineHandoffError::InvalidCommandLineUtf8)
            })
            .transpose()?;
        debug_marker(b'Q');

        let framebuffer = snapshot.framebuffer.map(|fb| FramebufferInfo {
            physical_start: (fb.addr() as u64).saturating_sub(snapshot.physical_memory_offset),
            width: fb.width().min(u32::MAX as u64) as u32,
            height: fb.height().min(u32::MAX as u64) as u32,
            pitch: fb.pitch().min(u32::MAX as u64) as u32,
            bpp: fb.bpp(),
            red_mask_size: fb.red_mask_size(),
            red_mask_shift: fb.red_mask_shift(),
            green_mask_size: fb.green_mask_size(),
            green_mask_shift: fb.green_mask_shift(),
            blue_mask_size: fb.blue_mask_size(),
            blue_mask_shift: fb.blue_mask_shift(),
        });
        debug_marker(b'R');

        let rsdp = snapshot.rsdp;

        Ok(LoaderDefinedBootHandoff::from_protocol(
            BootProtocol::Limine,
            command_line,
            rsdp,
            &self.memory_regions[..snapshot.memory_map.len()],
            &self.modules[..snapshot.modules.len()],
            framebuffer,
            snapshot.physical_memory_offset,
            BootMemoryRegion {
                start: snapshot.kernel_physical_base,
                len: align_up(kernel_image_len, PAGE_SIZE_4K),
                kind: BootMemoryRegionKind::KernelImage,
            },
        ))
    }
}

impl Default for LimineBootBuffers {
    fn default() -> Self {
        Self::new()
    }
}

fn limine_region_kind(entry_type: EntryType) -> BootMemoryRegionKind {
    if entry_type == EntryType::USABLE {
        BootMemoryRegionKind::Usable
    } else if entry_type == EntryType::RESERVED {
        BootMemoryRegionKind::Reserved
    } else if entry_type == EntryType::ACPI_RECLAIMABLE {
        BootMemoryRegionKind::AcpiReclaimable
    } else if entry_type == EntryType::ACPI_NVS {
        BootMemoryRegionKind::AcpiNvs
    } else if entry_type == EntryType::BAD_MEMORY {
        BootMemoryRegionKind::BadMemory
    } else if entry_type == EntryType::BOOTLOADER_RECLAIMABLE {
        BootMemoryRegionKind::BootloaderReclaimable
    } else if entry_type == EntryType::EXECUTABLE_AND_MODULES {
        BootMemoryRegionKind::KernelImage
    } else if entry_type == EntryType::FRAMEBUFFER {
        BootMemoryRegionKind::Framebuffer
    } else {
        BootMemoryRegionKind::Reserved
    }
}

#[cfg(all(target_arch = "x86_64", not(test)))]
fn debug_marker(byte: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") 0x00e9u16,
            in("al") byte,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[cfg(any(not(target_arch = "x86_64"), test))]
fn debug_marker(_byte: u8) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_defined_handoff_preserves_physical_rsdp_address() {
        let mut buffers = LimineBootBuffers::new();
        let physical_memory_offset = 0xffff_8000_0000_0000u64;
        let rsdp_phys = 0x1f77_e014u64;
        let snapshot = LimineBootSnapshot {
            command_line: None,
            rsdp: Some(rsdp_phys),
            memory_map: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset,
            kernel_physical_base: 0x1932_c000,
        };

        let handoff = buffers
            .build_loader_defined_handoff(snapshot, 0x160000)
            .expect("limine snapshot should build handoff");

        assert_eq!(handoff.rsdp, Some(rsdp_phys));
        assert_eq!(handoff.physical_memory_offset, physical_memory_offset);
    }
}
