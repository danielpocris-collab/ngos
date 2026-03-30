# NVIDIA RTX 5060 Ti Local Observations

Date observed locally on this machine on 2026-03-29 from Windows PnP / CIM / registry.

These observations are direct host evidence. They are not inferred from the repo and they are not copied from vendor documentation.

## Confirmed Identity

- Device name: `NVIDIA GeForce RTX 5060 Ti`
- Die: `GB206`
- Vendor ID: `0x10DE`
- Device ID: `0x2D04`
- Subsystem ID: `0x205E1771`
- Revision: `A1`
- PCI location: `bus 1, device 0, function 0`
- Class: `0x03 / 0x00 / 0x00` (`Display / VGA compatible`)
- Windows service stack: `nvlddmkm -> ACPI -> pci`
- Driver INF: `oem55.inf`
- Driver section: `Section048`
- Driver version: `32.0.15.9579`
- Driver date: `2026-03-04`

## Confirmed PCI / PnP Resources

The current Windows device instance reports:

- IO range: `0xF000 - 0xF07F`
- IRQ resource: `0xFFFFFFAF`
- Memory range 0: `0xF0000000 - 0xF3FFFFFF`
- Memory range 1: `0xFC00000000 - 0xFC01FFFFFF`
- Device parameters present:
  - `VideoID`
  - `AOCID`
  - `_DISPLAY_ACPI_INFO`

These observations justify the current driver assumptions that:

- the device is real and present on this machine
- `DEV_2D04` is a valid match target for local RE work
- the platform should expect at least one low MMIO range and one high memory aperture

## Confirmed PCIe Capabilities Exposed By Windows

The Windows PnP property set reports:

- `InterruptSupport = 7`
- `InterruptMessageMaximum = 9`
- `CurrentLinkSpeed = 4`
- `MaxLinkSpeed = 5`
- `CurrentLinkWidth = 8`
- `MaxLinkWidth = 16`
- `ExpressSpecVersion = 2`
- `CurrentPayloadSize = 1`
- `MaxPayloadSize = 1`
- `MaxReadRequestSize = 2`
- `AERCapabilityPresent = true`
- `S0WakeupSupported = true`
- `ARI support = true`
- `SR-IOV support = 2`
- `ACS support = 2`
- `BarTypes = 16843009`
- `SerialNumber = 7901562430057984072`

These values are useful as host evidence that the device exposes message-signaled interrupt capability and PCIe extended capability surfaces, but they do not by themselves reveal the device-internal register protocol.

The repo now exposes this through `gpu-irq /dev/gpu0`, which reports:

- `msi-supported=1`
- `message-limit=1`
- `windows-max=9`
- `hardware-confirmed=0`

This means MSI capability and policy are confirmed locally, but end-to-end hardware interrupt servicing is still not confirmed.

## Confirmed Topology

- ACPI BIOS device path: `\_SB.PCI0.GPP0.VGA`
- Location path: `PCIROOT(0)#PCI(0101)#PCI(0000)`
- Parent PCI bridge instance:
  - `PCI\VEN_1022&DEV_14DB&SUBSYS_14531022&REV_00\3&11583659&0&09`
- Reported display child relation:
  - `DISPLAY\GSM5C87\5&167e8082&0&UID4353`

## Confirmed Windows Binding

- Display driver INF: `oem55.inf`
- Original INF: `nv_dispi.inf`
- Driver section: `Section048`
- Kernel service: `nvlddmkm`
- Class stack: `\Driver\nvlddmkm, \Driver\ACPI, \Driver\pci`

## Confirmed INF Section048 Details

Directly observed in `C:\Windows\System32\DriverStore\FileRepository\nv_dispi.inf_amd64_4bf4c17fa8a478b5\nv_dispi.inf`:

- `%NVIDIA_DEV.2D04% = Section048, PCI\VEN_10DE&DEV_2D04`
- `NVIDIA_DEV.2D04 = "NVIDIA GeForce RTX 5060 Ti"`
- `[Section048]` includes:
  - `nv_commonBase_addreg__01`
  - `nv_commonDisplayModes_addreg`
  - `nv_global_addreg`
  - `nv_miscBase_addreg__16`
  - `nv_opengl_addreg`
  - `nv_timingRestrictions_addreg__01`
- `[Section048.GeneralConfigData]`:
  - `MaximumDeviceMemoryConfiguration = 128`
  - `MaximumNumberOfDevices = 4`
- `[Section048.HW]`:
  - `AddReg = nv_msiSupport_addreg`
- `[Section048.Services]`:
  - `AddService = nvlddmkm, ...`
  - `AddService = NVDisplay.ContainerLocalSystem, ...`
- `[nv_msiSupport_addreg]`:
  - `MSISupported = 1`
  - `MessageNumberLimit = 1`

This is direct evidence that the installed Windows package for `DEV_2D04` expects MSI delivery and constrains it to one message in the registry policy it installs.

## Confirmed nvidia-smi Observations

Directly observed via local `nvidia-smi -q` on 2026-03-29:

- Product architecture: `Blackwell`
- GPU part number: `2D04-300-A1`
- VBIOS version: `98.06.1f.00.dc`
- Bus ID: `00000000:01:00.0`
- Subsystem ID: `0x205E1771`
- Driver version: `595.79`
- CUDA version: `13.2`
- Driver model: `WDDM`
- Performance state: `P0`
- BAR1 total: `16384 MiB`
- FB total: `16311 MiB`
- PCIe host max generation: `4`
- PCIe device max generation: `5`
- PCIe current generation: `4`
- Link width current: `8x`
- Link width max: `16x`
- GSP firmware version: `N/A`

Directly observed via local GPU-Z capture on 2026-03-29:

- Bus interface: `PCIe x8 5.0 @ x8 4.0`
- Resizable BAR: `Enabled`

The repo now exposes this through `gpu-evidence /dev/gpu0`, including:

- `die=GB206`
- `bus-interface=PCIe x8 5.0 @ x8 4.0`
- `resizable-bar=1`
- `display-engine-confirmed=0`

This means link and board identity are confirmed locally, but real display-engine register programming is still not confirmed.

These values confirm the local card family and memory aperture sizes, but they do not confirm a hardware GSP control protocol because the current tooling exposed no firmware version there.

## Confirmed Real-Hardware VBIOS Dump

Directly recovered from a local GPU-Z save on 2026-03-29:

- Dump path: `C:\Users\pocri\OneDrive\Desktop\GB206.rom`
- Dump size: `1,961,983 bytes`
- SHA-256: `9a294cebf93aa635acba0fe5f7cd9b2ced6b357eeef85a81d22fabb98923aef2`

Confirmed signatures and offsets inside the dump:

- `NVFW` at `0x1000`
- `PCIR` at `0x346E4`
- `vendor = 0x10DE`, `device = 0x2D04` in the `PCIR` structure
- board string near `0x360E0`:
  - `NVIDIA GeForce RTX 5060 TI VGA BIOS`
- board code near `0x3610B`:
  - `P14N:506T301FB`
- version string near `0x36131`:
  - `Version 98.06.1F.00.DC`
- `BIT` table anchor near `0x361F2`

This is direct byte-level evidence from the local adapter. It confirms that real VBIOS bytes for the active `DEV_2D04` card have been recovered and that the dump contains NVIDIA ROM structures and the expected board identity.

## Current GSP Status

Directly observed via local `nvidia-smi -q` on 2026-03-29:

- `GSP Firmware Version: N/A`
- under the PCIe section, `Firmware: N/A`

Directly observed in the installed Windows package:

- `gsp_ga10x.bin`
- `gsp_tu10x.bin`

No obvious local firmware blob matching `DEV_2D04` / Blackwell was identified in the installed package during this session.

This means:

- the current machine confirms the NVIDIA package is installed
- but the current local tooling does not confirm an active GSP firmware surface for this setup
- and the repo must keep `gsp-control on real hardware` open rather than treating the synthetic loopback path as hardware confirmation

The repo now exposes this state end-to-end through `gpu-gsp /dev/gpu0`, which reports:

- `firmware-known=0`
- `firmware-version=N/A`
- `blackwell-blob=0`
- `blobs=gsp_ga10x.bin,gsp_tu10x.bin`

This is intentional refusal-style evidence: the current setup exposes loopback submission state, but it does not expose a confirmed Blackwell GSP firmware surface.

## Boundaries Of This Evidence

This evidence confirms:

- local physical presence of the RTX 5060 Ti
- the exact PCI identity targeted by the driver
- current Windows resource assignment
- current Windows driver binding
- some PCIe capability exposure

This evidence does not confirm:

- GSP mailbox/ring layout
- display engine register programming
- firmware RPC payload layouts
- interrupt acknowledge protocol inside the GPU
- power/media/neural/tensor/ray-tracing command formats

Those remain separate reverse-engineering fronts and require hardware traces, register capture, ROM decoding, firmware response decoding, or equivalent direct evidence.
