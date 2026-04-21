# Boot QEMU Refusal Evidence

Acest document fixeaza evidenta actuala pentru refusal family-urile din `boot-x86_64` pe path-ul real `QEMU`.

## Refusal Family Status

### Dovedite pe `QEMU` ca refusal `ngos`

- `invalid-command-line-utf8`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-invalid-command-line-utf8.ps1`
  - verificator: `tooling/x86_64/verify-qemu-boot-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=invalid-command-line-utf8 status=0x30`

- `too-many-modules`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-too-many-modules.ps1`
  - verificator: `tooling/x86_64/verify-qemu-boot-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=too-many-modules status=0x21`

- `missing-memory-map`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-missing-loader-response.ps1 -Mode missing-memory-map`
  - verificator: `tooling/x86_64/verify-qemu-boot-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=missing-memory-map status=0x10`

- `missing-hhdm`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-missing-loader-response.ps1 -Mode missing-hhdm`
  - verificator: `tooling/x86_64/verify-qemu-boot-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=missing-hhdm status=0x11`

- `too-many-memory-regions`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode too-many-memory-regions`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: post-handoff corruption applied mode=too-many-memory-regions`

- `InvalidBootInfo::UnalignedPhysicalMemoryOffset`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode invalid-hhdm-offset`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=invalid-hhdm-offset status=0x40`

- `InvalidBootInfo::KernelRangeMustBeKernelImage`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode invalid-kernel-range-kind`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=invalid-kernel-range-kind status=0x41`

- `InvalidBootInfo::KernelRangeMustBePageAligned`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode invalid-kernel-range-alignment`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=invalid-kernel-range-alignment status=0x42`

- `InvalidBootInfo::KernelRangeMustBeNonEmpty`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode empty-kernel-range`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=empty-kernel-range status=0x43`

- `InvalidBootInfo::MemoryRegionMustBePageAligned`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode invalid-memory-region-alignment`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=invalid-memory-region-alignment status=0x44`

- `InvalidBootInfo::MemoryRegionMustBeNonEmpty`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode empty-memory-region`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=empty-memory-region status=0x45`

- `InvalidBootInfo::MemoryRegionsOverlap`
  - script: `tooling/x86_64/prove-qemu-boot-refusal-post-handoff-corruption.ps1 -Mode overlapping-memory-regions`
  - verificator: `tooling/x86_64/verify-qemu-boot-post-handoff-refusal-log.ps1`
  - marker: `ngos/x86_64: limine handoff refusal detail=overlapping-memory-regions status=0x46`

### Dovedite pe `QEMU` ca rejection pre-`ngos` in Limine

- `invalid-module-path-utf8`
  - inspect script: `tooling/x86_64/inspect-qemu-limine-preboot-module-path-rejection.ps1`
  - verificator: `tooling/x86_64/verify-qemu-limine-preboot-module-path-rejection-log.ps1`

- `missing-executable-address`
  - inspect script: `tooling/x86_64/inspect-qemu-limine-preboot-executable-path-rejection.ps1`
  - verificator: `tooling/x86_64/verify-qemu-limine-preboot-executable-path-rejection-log.ps1`

- `missing-base-revision`
  - inspect script: `tooling/x86_64/inspect-qemu-limine-preboot-base-revision-rejection.ps1 -Mode missing-base-revision`
  - verificator: `tooling/x86_64/verify-qemu-limine-preboot-base-revision-rejection-log.ps1 -Mode missing-base-revision`

- `unsupported-base-revision`
  - inspect script: `tooling/x86_64/inspect-qemu-limine-preboot-base-revision-rejection.ps1 -Mode unsupported-base-revision`
  - verificator: `tooling/x86_64/verify-qemu-limine-preboot-base-revision-rejection-log.ps1 -Mode unsupported-base-revision`

## Mechanism

Mecanismul repo-owned de corupere post-handoff traieste in:

- [boot_handoff_proof.rs](C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/boot_handoff_proof.rs)
- [limine.rs](C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/limine.rs)

Fluxul real este:

1. `platform-x86_64::limine::build_loader_defined_handoff(...)`
2. `boot-x86_64::boot_handoff_proof::apply(...)`
3. `LoaderDefinedBootHandoff::as_boot_info()`

Selectorul real de activare este:

- `ngos.boot.handoff_corrupt=<mode>`

Verificatorul source-owned pentru suprafata mecanismului este:

- `tooling/x86_64/verify-qemu-boot-post-handoff-corruption-surface.ps1`

## Aggregates

- dovada agregata `QEMU`: `tooling/x86_64/prove-qemu-boot-evidence.ps1`
- verificare agregata `QEMU`: `tooling/x86_64/verify-qemu-boot-evidence.ps1`
- verificare agregata subsistem: `tooling/x86_64/verify-boot-subsystem-state.ps1 -IncludeQemu`

## Interpretation Rule

- daca refusal-ul intra in `ngos`, tinta minima este proof `QEMU` cu marker-ele `ngos`
- daca refusal-ul este respins de bootloader inainte de handoff, trebuie pastrat separat ca evidence pre-`ngos`
- familia `InvalidBootInfo(...)` nu mai este local-only: este acum dovedita pe `QEMU` prin mecanismul repo-owned de corupere post-handoff
