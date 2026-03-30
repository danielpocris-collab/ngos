# x86_64 Boot Foundation

Acest document descrie fundatia initiala pentru target-ul kernel `x86_64`, separata de runtime-ul `host-runtime`.

## Ce exista acum

- crate nou: `platform-x86_64`
- target spec: `platform-x86_64/targets/x86_64-ngos-kernel.json`
- linker script: `platform-x86_64/linker/kernel-x86_64.ld`
- layout canonic pentru higher-half kernel, direct map si boot stack
- tipuri de handoff neutre fata de boot protocol:
  - `BootInfo`
  - `BootMemoryRegion`
  - `BootModule`
  - `FramebufferInfo`
- plan de mapare timpurie:
  - identity map
  - higher-half kernel image mapping
  - direct map window
  - boot stack mapping

## Scopul acestui strat

Nu incearca sa fie deja bootloader final sau ABI final de boot.
Rolul lui este sa fixeze fundatia comuna pe care se pot lega ulterior:

- un loader Limine sau Multiboot2
- initializarea paginarii
- intrarea reala `_start`
- handoff-ul catre subsistemele de arhitectura si apoi catre kernel runtime

## Directia urmatoare recomandata

1. Adaugare crate `kernel-x86_64` sau `boot-x86_64` cu `_start`, zeroing `.bss` si stack switch.
2. Definire handoff concret din boot protocol in `BootInfo`.
3. Construire page tables initiale pe baza `BootstrapMappingPlan`.
4. Stabilire contract clar intre `platform-x86_64` si viitorul backend kernel non-host-runtime.

## Build de baza

Exemplu de verificare a crate-ului:

```powershell
cargo test -p ngos-platform-x86_64
```

Exemplu de build freestanding dupa aparitia unui binar kernel real:

```powershell
cargo build -p <kernel-crate> --target platform-x86_64/targets/x86_64-ngos-kernel.json
```
