# NVIDIA Reverse Engineering Evidence Index

## Confirmed Local Evidence

- [RTX 5060 Ti local observations](C:/Users/pocri/OneDrive/Desktop/experiment/docs/nvidia-rtx-5060ti-local-observations.md)

## Interpretation Rules

- `confirmed` means directly observed through local hardware, PCI config space, ROM bytes, or runtime-owned state.
- `inferred` means reconstructed from observed topology or controlled synthetic execution but not yet confirmed against real device protocol.
- `experimental` means semantic scaffolding without hardware confirmation.

## Current Mapping

- `probe`: confirmed
- `local pci topology for DEV_2D04`: confirmed
- `windows resource assignment for DEV_2D04`: confirmed
- `windows driver binding for DEV_2D04`: confirmed
- `windows Section048 binding for DEV_2D04`: confirmed
- `windows MSI registry policy for DEV_2D04`:
  - `nv_msiSupport_addreg`
  - `MSISupported = 1`
  - `MessageNumberLimit = 1`
  - confirmed
- `local architecture / vbios / part number for DEV_2D04`:
  - `Blackwell`
  - `98.06.1f.00.dc`
  - `2D04-300-A1`
  - confirmed
- `local BAR1 / framebuffer aperture sizes for DEV_2D04`:
  - `BAR1 = 16384 MiB`
  - `FB = 16311 MiB`
  - confirmed
- `vbios-window`: confirmed
- `vbios-bytes`:
  - confirmed on synthetic backend when ROM backing exists
  - confirmed on real hardware from local GPU-Z dump artifact:
    - `C:\Users\pocri\OneDrive\Desktop\GB206.rom`
    - `SHA-256 = 9a294cebf93aa635acba0fe5f7cd9b2ced6b357eeef85a81d22fabb98923aef2`
    - contains `PCIR`, `NVFW`, `BIT`, `10DE:2D04`, `Version 98.06.1F.00.DC`
- `msi-x capability enable`: confirmed on synthetic PCI model and supported by local Windows capability evidence
- `gsp generic submit`: inferred
- `gsp firmware surface on current Windows setup`:
  - `nvidia-smi` reports `GSP Firmware Version: N/A`
  - installed package exposes `gsp_ga10x.bin` and `gsp_tu10x.bin`
  - no confirmed local Blackwell / `DEV_2D04` firmware blob identified
  - shell exposure now reports:
    - `firmware-known=0`
    - `firmware-version=N/A`
    - `blackwell-blob=0`
    - `blobs=gsp_ga10x.bin,gsp_tu10x.bin`
  - current real-hardware status remains open
- `display present planning`: inferred/semantic
- `power`, `media`, `neural`, `tensor`, `ray-tracing`: experimental
