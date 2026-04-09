use platform_x86_64::{
    BootMemoryRegion, BootMemoryRegionKind, LoaderDefinedBootHandoff, PAGE_SIZE_4K,
};

const PROOF_TOO_MANY_MEMORY_REGIONS: &str = "too-many-memory-regions";
const PROOF_INVALID_HHDM_OFFSET: &str = "invalid-hhdm-offset";
const PROOF_INVALID_KERNEL_RANGE_KIND: &str = "invalid-kernel-range-kind";
const PROOF_INVALID_KERNEL_RANGE_ALIGNMENT: &str = "invalid-kernel-range-alignment";
const PROOF_EMPTY_KERNEL_RANGE: &str = "empty-kernel-range";
const PROOF_INVALID_MEMORY_REGION_ALIGNMENT: &str = "invalid-memory-region-alignment";
const PROOF_EMPTY_MEMORY_REGION: &str = "empty-memory-region";
const PROOF_OVERLAPPING_MEMORY_REGIONS: &str = "overlapping-memory-regions";
const MAX_PROOF_MEMORY_REGIONS: usize = 257;

const EMPTY_MEMORY_REGION: BootMemoryRegion = BootMemoryRegion {
    start: 0,
    len: 0,
    kind: BootMemoryRegionKind::Reserved,
};

static mut PROOF_MEMORY_REGIONS: [BootMemoryRegion; MAX_PROOF_MEMORY_REGIONS] =
    [EMPTY_MEMORY_REGION; MAX_PROOF_MEMORY_REGIONS];

pub fn apply<'a>(handoff: &mut LoaderDefinedBootHandoff<'a>) -> Option<&'static str> {
    let mode = proof_mode(handoff.command_line)?;
    let applied_mode = match mode {
        PROOF_TOO_MANY_MEMORY_REGIONS => {
            apply_too_many_memory_regions(handoff);
            PROOF_TOO_MANY_MEMORY_REGIONS
        }
        PROOF_INVALID_HHDM_OFFSET => {
            handoff.physical_memory_offset = handoff.physical_memory_offset.saturating_add(1);
            PROOF_INVALID_HHDM_OFFSET
        }
        PROOF_INVALID_KERNEL_RANGE_KIND => {
            handoff.kernel_phys_range.kind = BootMemoryRegionKind::Reserved;
            PROOF_INVALID_KERNEL_RANGE_KIND
        }
        PROOF_INVALID_KERNEL_RANGE_ALIGNMENT => {
            handoff.kernel_phys_range.start = handoff.kernel_phys_range.start.saturating_add(1);
            PROOF_INVALID_KERNEL_RANGE_ALIGNMENT
        }
        PROOF_EMPTY_KERNEL_RANGE => {
            handoff.kernel_phys_range.len = 0;
            PROOF_EMPTY_KERNEL_RANGE
        }
        PROOF_INVALID_MEMORY_REGION_ALIGNMENT => {
            apply_invalid_memory_region_alignment(handoff);
            PROOF_INVALID_MEMORY_REGION_ALIGNMENT
        }
        PROOF_EMPTY_MEMORY_REGION => {
            apply_empty_memory_region(handoff);
            PROOF_EMPTY_MEMORY_REGION
        }
        PROOF_OVERLAPPING_MEMORY_REGIONS => {
            apply_overlapping_memory_regions(handoff);
            PROOF_OVERLAPPING_MEMORY_REGIONS
        }
        _ => return None,
    };
    Some(applied_mode)
}

fn proof_mode(command_line: Option<&str>) -> Option<&str> {
    command_line.and_then(|command_line| {
        command_line.split_whitespace().find_map(|token| {
            token.strip_prefix("ngos.boot.handoff_corrupt=")
        })
    })
}

fn apply_too_many_memory_regions<'a>(handoff: &mut LoaderDefinedBootHandoff<'a>) {
    let original_count = handoff.memory_regions.len();
    if original_count == 0 {
        return;
    }
    let proof_regions = proof_region_storage();
    proof_regions[..original_count].copy_from_slice(handoff.memory_regions);
    let mut next_start = proof_regions[original_count - 1].end();
    let mut index = original_count;
    while index < MAX_PROOF_MEMORY_REGIONS {
        proof_regions[index] = BootMemoryRegion {
            start: next_start,
            len: PAGE_SIZE_4K,
            kind: BootMemoryRegionKind::Reserved,
        };
        next_start = next_start.saturating_add(PAGE_SIZE_4K);
        index += 1;
    }
    handoff.memory_regions = &proof_regions[..MAX_PROOF_MEMORY_REGIONS];
}

fn apply_invalid_memory_region_alignment<'a>(handoff: &mut LoaderDefinedBootHandoff<'a>) {
    let count = handoff.memory_regions.len();
    if count == 0 {
        return;
    }
    let proof_regions = proof_region_storage();
    proof_regions[..count].copy_from_slice(handoff.memory_regions);
    proof_regions[0].start = proof_regions[0].start.saturating_add(1);
    handoff.memory_regions = &proof_regions[..count];
}

fn apply_empty_memory_region<'a>(handoff: &mut LoaderDefinedBootHandoff<'a>) {
    let count = handoff.memory_regions.len();
    if count == 0 {
        return;
    }
    let proof_regions = proof_region_storage();
    proof_regions[..count].copy_from_slice(handoff.memory_regions);
    proof_regions[0].len = 0;
    handoff.memory_regions = &proof_regions[..count];
}

fn apply_overlapping_memory_regions<'a>(handoff: &mut LoaderDefinedBootHandoff<'a>) {
    let original = handoff.memory_regions;
    if original.is_empty() {
        return;
    }
    let proof_regions = proof_region_storage();
    let count = original.len().max(2).min(MAX_PROOF_MEMORY_REGIONS);
    proof_regions[..original.len()].copy_from_slice(original);
    if original.len() == 1 {
        proof_regions[1] = BootMemoryRegion {
            start: original[0].start.saturating_add(PAGE_SIZE_4K),
            len: PAGE_SIZE_4K,
            kind: BootMemoryRegionKind::Reserved,
        };
    } else {
        proof_regions[1].start = proof_regions[0].start.saturating_add(PAGE_SIZE_4K);
        proof_regions[1].len = proof_regions[1].len.max(PAGE_SIZE_4K);
    }
    handoff.memory_regions = &proof_regions[..count];
}

fn proof_region_storage() -> &'static mut [BootMemoryRegion] {
    unsafe {
        core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(PROOF_MEMORY_REGIONS).cast::<BootMemoryRegion>(),
            MAX_PROOF_MEMORY_REGIONS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_x86_64::{
        BootInfoValidationError, BootProtocol, LoaderDefinedHandoffError,
    };

    fn sample_handoff<'a>(
        command_line: Option<&'a str>,
        memory_regions: &'a [BootMemoryRegion],
    ) -> LoaderDefinedBootHandoff<'a> {
        LoaderDefinedBootHandoff::from_protocol(
            BootProtocol::Limine,
            command_line,
            None,
            memory_regions,
            &[],
            None,
            0xffff_8000_0000_0000,
            BootMemoryRegion {
                start: 0x20_0000,
                len: PAGE_SIZE_4K,
                kind: BootMemoryRegionKind::KernelImage,
            },
        )
    }

    fn sample_regions() -> [BootMemoryRegion; 2] {
        [
            BootMemoryRegion {
                start: 0,
                len: 0x20_0000,
                kind: BootMemoryRegionKind::Usable,
            },
            BootMemoryRegion {
                start: 0x40_0000,
                len: 0x20_0000,
                kind: BootMemoryRegionKind::Reserved,
            },
        ]
    }

    #[test]
    fn handoff_proof_can_inflate_memory_region_count() {
        let regions = sample_regions();
        let mut handoff = sample_handoff(
            Some("console=ttyS0 ngos.boot.handoff_corrupt=too-many-memory-regions"),
            &regions,
        );

        let applied = apply(&mut handoff);

        assert_eq!(applied, Some("too-many-memory-regions"));
        assert_eq!(handoff.memory_regions.len(), MAX_PROOF_MEMORY_REGIONS);
    }

    #[test]
    fn handoff_proof_can_force_invalid_boot_info_variants() {
        let cases = [
            (
                "invalid-hhdm-offset",
                BootInfoValidationError::UnalignedPhysicalMemoryOffset,
            ),
            (
                "invalid-kernel-range-kind",
                BootInfoValidationError::KernelRangeMustBeKernelImage,
            ),
            (
                "invalid-kernel-range-alignment",
                BootInfoValidationError::KernelRangeMustBePageAligned,
            ),
            (
                "empty-kernel-range",
                BootInfoValidationError::KernelRangeMustBeNonEmpty,
            ),
            (
                "invalid-memory-region-alignment",
                BootInfoValidationError::MemoryRegionMustBePageAligned,
            ),
            (
                "empty-memory-region",
                BootInfoValidationError::MemoryRegionMustBeNonEmpty,
            ),
            (
                "overlapping-memory-regions",
                BootInfoValidationError::MemoryRegionsOverlap,
            ),
        ];

        for (mode, expected) in cases {
            let regions = sample_regions();
            let mut handoff = sample_handoff(
                Some(mode),
                &regions,
            );
            handoff.command_line = Some(Box::leak(format!(
                "console=ttyS0 ngos.boot.handoff_corrupt={mode}"
            )
            .into_boxed_str()));

            assert_eq!(apply(&mut handoff), Some(mode));
            assert_eq!(
                handoff.as_boot_info(),
                Err(LoaderDefinedHandoffError::InvalidBootInfo(expected))
            );
        }
    }

    #[test]
    fn handoff_proof_ignores_absent_or_unknown_modes() {
        let regions = sample_regions();
        let mut handoff = sample_handoff(Some("console=ttyS0"), &regions);
        assert_eq!(apply(&mut handoff), None);

        let mut handoff = sample_handoff(
            Some("console=ttyS0 ngos.boot.handoff_corrupt=unknown"),
            &regions,
        );
        assert_eq!(apply(&mut handoff), None);
        assert!(handoff.as_boot_info().is_ok());
    }
}
