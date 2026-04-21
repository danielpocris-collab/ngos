//! Canonical subsystem role:
//! - subsystem: first-user ELF parsing
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-stage parsing of the initial native user image for
//!   launch materialization
//!
//! Canonical contract families handled here:
//! - ELF parsing contracts
//! - user image segment extraction contracts
//! - first-user image validation contracts
//!
//! This module may parse the initial user ELF for boot launch, but it must not
//! redefine the long-term executable or VM ownership model of the kernel.

use super::*;

pub(super) struct ParsedUserElf<'a> {
    pub(super) base_addr: u64,
    pub(super) entry_point: u64,
    pub(super) phdr_addr: u64,
    pub(super) phent_size: u64,
    pub(super) phnum: u64,
    pub(super) stack_top: u64,
    pub(super) segments: Vec<UserImageSegment<'a>>,
}

pub(super) fn parse_user_elf<'a>(
    module_name: &'a str,
    image: &'a [u8],
) -> Result<ParsedUserElf<'a>, UserProcessError> {
    if image.len() < size_of::<Elf64Header>() {
        return Err(UserProcessError::InvalidElf);
    }

    let header = unsafe { ptr::read_unaligned(image.as_ptr().cast::<Elf64Header>()) };
    if header.e_ident[..4] != [0x7f, b'E', b'L', b'F']
        || header.e_ident[4] != 2
        || header.e_ident[5] != 1
    {
        return Err(UserProcessError::InvalidElf);
    }
    if header.e_machine != 0x3e {
        return Err(UserProcessError::UnsupportedElf);
    }

    let phoff = header.e_phoff as usize;
    let phentsize = header.e_phentsize as usize;
    let phnum = header.e_phnum as usize;
    let total_ph_size = phentsize
        .checked_mul(phnum)
        .ok_or(UserProcessError::SegmentOverflow)?;
    let ph_end = phoff
        .checked_add(total_ph_size)
        .ok_or(UserProcessError::SegmentOverflow)?;
    if ph_end > image.len() || phentsize != size_of::<Elf64ProgramHeader>() {
        return Err(UserProcessError::InvalidElf);
    }

    let mut segments = Vec::new();
    for index in 0..phnum {
        let ph_ptr = unsafe { image.as_ptr().add(phoff + index * phentsize) };
        let ph = unsafe { ptr::read_unaligned(ph_ptr.cast::<Elf64ProgramHeader>()) };
        if ph.p_type != 1 {
            continue;
        }
        if segments.len() >= MAX_USER_SEGMENTS {
            return Err(UserProcessError::TooManySegments {
                count: segments.len() + 1,
                capacity: MAX_USER_SEGMENTS,
            });
        }
        let file_end = ph
            .p_offset
            .checked_add(ph.p_filesz)
            .ok_or(UserProcessError::SegmentOverflow)?;
        if file_end as usize > image.len() || ph.p_filesz > ph.p_memsz {
            return Err(UserProcessError::InvalidElf);
        }

        let mapping_vaddr = align_down(ph.p_vaddr, PAGE_SIZE_4K);
        let copy_offset = (ph.p_vaddr - mapping_vaddr) as usize;
        let mapping_len = align_up(ph.p_memsz.saturating_add(copy_offset as u64), PAGE_SIZE_4K);
        let file_bytes = &image[ph.p_offset as usize..file_end as usize];
        segments.push(UserImageSegment {
            mapping: PageMapping {
                vaddr: mapping_vaddr,
                paddr: 0,
                len: mapping_len,
                perms: MemoryPermissions {
                    read: (ph.p_flags & 0x4) != 0,
                    write: (ph.p_flags & 0x2) != 0,
                    execute: (ph.p_flags & 0x1) != 0,
                },
                cache: CachePolicy::WriteBack,
                user: true,
            },
            file_bytes,
            copy_offset,
        });
    }

    segments.sort_by_key(|segment| segment.mapping.vaddr);
    if segments.is_empty() {
        return Err(UserProcessError::InvalidElf);
    }
    let base_addr = segments[0].mapping.vaddr;

    let stack_top = 0x0000_7fff_ffff_0000u64;
    serial::print(format_args!(
        "ngos/x86_64: exec userland-native module=\"{}\" entry={:#x} segments={}\n",
        module_name,
        header.e_entry,
        segments.len()
    ));

    Ok(ParsedUserElf {
        base_addr,
        entry_point: header.e_entry,
        phdr_addr: header.e_phoff,
        phent_size: header.e_phentsize as u64,
        phnum: header.e_phnum as u64,
        stack_top,
        segments,
    })
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}
