# Boot And Diagnostics Closure Status

`Subsystem Boot and diagnostics is closed on the real QEMU path.`

## Scope

Conform [docs/ngos-subsystem-closure-matrix.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-subsystem-closure-matrix.md), în scope intră:

- boot entry
- early memory
- platform facts
- CPU bring-up facts
- boot diagnostics
- boot locator / serial proof markers
- handoff spre `kernel-core`
- încărcare kernel și userland bootstrap

În afara scope-ului pentru acest document rămân:

- policy semantică finală de scheduler / VFS / VM
- logică de control userland
- closure globală a device runtime doar prin diagnostics

## Ce Este Închis

- success path real de boot pe `boot-x86_64 -> platform-x86_64 -> QEMU`
- diagnostics observabile pe serial pentru:
  - `stage0 entered`
  - bootloader detectat
  - `early_kernel_main`
  - framebuffer console online
  - protocol / `hhdm`
  - post-paging handoff
  - entering user mode module
  - boot report handled
  - first user process report
  - boot outcome policy final
- handoff real către `kernel-core` și bootstrap real pentru `userland-native`
- refusal families `ngos` pe path-ul real `QEMU` pentru:
  - `invalid-command-line-utf8`
  - `too-many-modules`
  - `missing-memory-map`
  - `missing-hhdm`
  - `too-many-memory-regions`
  - `InvalidBootInfo::UnalignedPhysicalMemoryOffset`
  - `InvalidBootInfo::KernelRangeMustBeKernelImage`
  - `InvalidBootInfo::KernelRangeMustBePageAligned`
  - `InvalidBootInfo::KernelRangeMustBeNonEmpty`
  - `InvalidBootInfo::MemoryRegionMustBePageAligned`
  - `InvalidBootInfo::MemoryRegionMustBeNonEmpty`
  - `InvalidBootInfo::MemoryRegionsOverlap`
- rejection families pre-`ngos` observate separat pe `QEMU` în Limine pentru:
  - `invalid-module-path-utf8`
  - `missing-executable-address`
  - `missing-base-revision`
  - `unsupported-base-revision`
- observabilitate și refusal/recovery locală în owner-ul real `boot-x86_64` pentru:
  - handoff proof corruption surface
  - boot report stage progression
  - boot report duplicate/regression refusal
  - first-user outcome policy

## Familii Rămase Deschise

- none pe truth path-ul actual `QEMU`

Scriptul repo-owned [tooling/x86_64/verify-boot-open-families-state.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-boot-open-families-state.ps1) afirmă explicit:

- `Still open: none`

## Dovezi Curente

- success path `QEMU`:
  - [tooling/x86_64/prove-qemu-boot.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-boot.ps1)
  - [tooling/x86_64/verify-qemu-boot-log.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-boot-log.ps1)
- refusal / rejection aggregate `QEMU`:
  - [docs/boot-qemu-refusal-evidence.md](/C:/Users/pocri/OneDrive/Desktop/experiment/docs/boot-qemu-refusal-evidence.md)
  - [tooling/x86_64/prove-qemu-boot-evidence.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/prove-qemu-boot-evidence.ps1)
  - [tooling/x86_64/verify-qemu-boot-evidence.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-qemu-boot-evidence.ps1)
  - [tooling/x86_64/verify-boot-subsystem-state.ps1](/C:/Users/pocri/OneDrive/Desktop/experiment/tooling/x86_64/verify-boot-subsystem-state.ps1)
- owner-local proofs:
  - [boot-x86_64/src/boot_handoff_proof.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/boot_handoff_proof.rs)
  - [boot-x86_64/src/limine.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/limine.rs)
  - [boot-x86_64/src/user_process.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_process.rs)
  - [boot-x86_64/src/user_runtime_status.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_runtime_status.rs)
  - [boot-x86_64/src/user_syscall.rs](/C:/Users/pocri/OneDrive/Desktop/experiment/boot-x86_64/src/user_syscall.rs)

## Verificare

- `cargo test -p ngos-platform-x86_64`
- `cargo test -p ngos-boot-x86_64 --lib`
- `powershell -ExecutionPolicy Bypass -File .\\tooling\\x86_64\\verify-boot-subsystem-state.ps1 -IncludeQemu`

## Comportament Observabil

- success path:
  - `ngos/x86_64: stage0 entered`
  - `ngos/x86_64: early_kernel_main reached`
  - `ngos/x86_64: entering user mode module="/kernel/ngos-userland-native"`
  - `ngos/x86_64: boot report handled status=0 stage=2 code=0`
  - `ngos/x86_64: first user process boot outcome policy=RequireZeroExit outcome=success action=halt-success exit_code=0`
- refusal path:
  - `ngos/x86_64: limine handoff refusal detail=... status=...`
  - `ngos/x86_64: post-handoff corruption applied mode=...`

Pe scope-ul actual de closure pentru `Boot and diagnostics`, subsistemul este închis pe `QEMU`.
