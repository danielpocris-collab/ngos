//! Canonical subsystem role:
//! - subsystem: x86_64 bootloader mediation
//! - owner layer: platform mediation
//! - semantic owner: `platform-x86_64`
//! - truth path role: platform-specific Limine handoff mediation into canonical
//!   boot structures
//!
//! Canonical contract families handled here:
//! - bootloader handoff contracts
//! - memory map mediation contracts
//! - framebuffer/module handoff contracts
//!
//! This module may mediate Limine-provided boot mechanics, but it must not
//! redefine higher-level boot or kernel semantic ownership.

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
    use alloc::{boxed::Box, vec::Vec};
    use core::{
        ffi::{CStr, c_char, c_void},
        mem::{MaybeUninit, size_of},
        num::NonZeroU32,
    };
    use limine::file::{MediaType, Uuid};

    #[repr(C)]
    struct TestFile {
        revision: u64,
        addr: *mut c_void,
        size: u64,
        path: *const c_char,
        string: *const c_char,
        media_type: MediaType,
        unused: MaybeUninit<u32>,
        tftp_ip: Option<NonZeroU32>,
        tftp_port: Option<NonZeroU32>,
        partition_idx: Option<NonZeroU32>,
        mbr_disk_id: Option<NonZeroU32>,
        gpt_disk_id: Uuid,
        gpt_partition_id: Uuid,
        partition_uuid: Uuid,
    }

    fn test_uuid() -> Uuid {
        Uuid {
            a: 0,
            b: 0,
            c: 0,
            d: [0; 8],
        }
    }

    fn test_file(path: &'static CStr) -> &'static File {
        assert_eq!(size_of::<TestFile>(), size_of::<File>());
        let file = Box::new(TestFile {
            revision: 0,
            addr: core::ptr::null_mut(),
            size: 0x1000,
            path: path.as_ptr(),
            string: c"".as_ptr(),
            media_type: MediaType::GENERIC,
            unused: MaybeUninit::new(0),
            tftp_ip: None,
            tftp_port: None,
            partition_idx: None,
            mbr_disk_id: None,
            gpt_disk_id: test_uuid(),
            gpt_partition_id: test_uuid(),
            partition_uuid: test_uuid(),
        });
        unsafe { &*(Box::leak(file) as *mut TestFile as *mut File) }
    }

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

    #[test]
    fn loader_defined_handoff_normalizes_empty_command_line_to_none() {
        let mut buffers = LimineBootBuffers::new();
        let snapshot = LimineBootSnapshot {
            command_line: Some(c""),
            rsdp: None,
            memory_map: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0xffff_8000_0000_0000,
            kernel_physical_base: 0x20_0000,
        };

        let handoff = buffers
            .build_loader_defined_handoff(snapshot, PAGE_SIZE_4K)
            .expect("empty cmdline should normalize cleanly");

        assert_eq!(handoff.command_line, None);
    }

    #[test]
    fn loader_defined_handoff_preserves_module_path_and_physical_translation() {
        let mut buffers = LimineBootBuffers::new();
        let physical_memory_offset = 0u64;
        let module_bytes = Box::leak(Vec::from([0u8; 0x2000]).into_boxed_slice());
        let module = Box::new(TestFile {
            revision: 0,
            addr: module_bytes.as_mut_ptr().cast(),
            size: module_bytes.len() as u64,
            path: c"/kernel/ngos-userland-native".as_ptr(),
            string: c"render3d".as_ptr(),
            media_type: MediaType::GENERIC,
            unused: MaybeUninit::new(0),
            tftp_ip: None,
            tftp_port: None,
            partition_idx: None,
            mbr_disk_id: None,
            gpt_disk_id: test_uuid(),
            gpt_partition_id: test_uuid(),
            partition_uuid: test_uuid(),
        });
        let module = unsafe { &*(Box::leak(module) as *mut TestFile as *mut File) };
        let modules: &'static [&'static File] = Box::leak(Vec::from([module]).into_boxed_slice());
        let snapshot = LimineBootSnapshot {
            command_line: Some(c"console=ttyS0"),
            rsdp: None,
            memory_map: &[],
            modules,
            framebuffer: None,
            physical_memory_offset,
            kernel_physical_base: 0x20_0000,
        };

        let handoff = buffers
            .build_loader_defined_handoff(snapshot, 0x6000)
            .expect("module handoff should be preserved");

        assert_eq!(handoff.command_line, Some("console=ttyS0"));
        assert_eq!(handoff.modules.len(), 1);
        assert_eq!(handoff.modules[0].name, "/kernel/ngos-userland-native");
        assert_eq!(
            handoff.modules[0].physical_start,
            module_bytes.as_ptr() as u64 - physical_memory_offset
        );
        assert_eq!(handoff.modules[0].len, 0x2000);
    }

    #[test]
    fn loader_defined_handoff_rejects_too_many_memory_regions() {
        let mut buffers = LimineBootBuffers::new();
        let entry = Box::leak(Box::new(Entry {
            base: 0,
            length: PAGE_SIZE_4K,
            entry_type: EntryType::USABLE,
        }));
        let memory_map: &'static [&'static Entry] = Box::leak(
            (0..(MAX_LIMINE_MEMORY_REGIONS + 1))
                .map(|_| &*entry)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let snapshot = LimineBootSnapshot {
            command_line: None,
            rsdp: None,
            memory_map,
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0xffff_8000_0000_0000,
            kernel_physical_base: 0x20_0000,
        };

        assert_eq!(
            buffers.build_loader_defined_handoff(snapshot, PAGE_SIZE_4K),
            Err(LimineHandoffError::TooManyMemoryRegions {
                count: MAX_LIMINE_MEMORY_REGIONS + 1,
                capacity: MAX_LIMINE_MEMORY_REGIONS,
            })
        );
    }

    #[test]
    fn loader_defined_handoff_rejects_too_many_modules() {
        let mut buffers = LimineBootBuffers::new();
        let module = test_file(c"/kernel/ngos-userland-native");
        let modules: &'static [&'static File] = Box::leak(
            (0..(MAX_LIMINE_MODULES + 1))
                .map(|_| module)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let snapshot = LimineBootSnapshot {
            command_line: None,
            rsdp: None,
            memory_map: &[],
            modules,
            framebuffer: None,
            physical_memory_offset: 0xffff_8000_0000_0000,
            kernel_physical_base: 0x20_0000,
        };

        assert_eq!(
            buffers.build_loader_defined_handoff(snapshot, PAGE_SIZE_4K),
            Err(LimineHandoffError::TooManyModules {
                count: MAX_LIMINE_MODULES + 1,
                capacity: MAX_LIMINE_MODULES,
            })
        );
    }

    #[test]
    fn loader_defined_handoff_rejects_invalid_command_line_utf8() {
        let mut buffers = LimineBootBuffers::new();
        let invalid_cmdline = unsafe { CStr::from_bytes_with_nul_unchecked(b"console=\xc3(\0") };
        let snapshot = LimineBootSnapshot {
            command_line: Some(invalid_cmdline),
            rsdp: None,
            memory_map: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0xffff_8000_0000_0000,
            kernel_physical_base: 0x20_0000,
        };

        assert_eq!(
            buffers.build_loader_defined_handoff(snapshot, PAGE_SIZE_4K),
            Err(LimineHandoffError::InvalidCommandLineUtf8)
        );
    }

    #[test]
    fn loader_defined_handoff_rejects_invalid_module_path_utf8() {
        let mut buffers = LimineBootBuffers::new();
        let invalid_path =
            unsafe { CStr::from_bytes_with_nul_unchecked(b"/kernel/ngos-userland-native\xc3(\0") };
        let module = test_file(invalid_path);
        let modules: &'static [&'static File] = Box::leak(Vec::from([module]).into_boxed_slice());
        let snapshot = LimineBootSnapshot {
            command_line: None,
            rsdp: None,
            memory_map: &[],
            modules,
            framebuffer: None,
            physical_memory_offset: 0xffff_8000_0000_0000,
            kernel_physical_base: 0x20_0000,
        };

        assert_eq!(
            buffers.build_loader_defined_handoff(snapshot, PAGE_SIZE_4K),
            Err(LimineHandoffError::InvalidModulePathUtf8 { index: 0 })
        );
    }
}
