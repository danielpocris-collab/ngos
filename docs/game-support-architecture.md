# Game Support Architecture for NGOS

## Document Purpose

Acest document descrie cum jocurile video se integreaza în kernelul NGOS. Nu este o propunere teoretică. Este o hartă de integrare în codul existent, respectând arhitectura kernel, modelul Domain/Resource/Contract, modelul device-platform și principiile nano-semantic.

## Principiu Fundamental

Jocurile sunt **procese ca oricare altele**. Nu necesită:
- Clase de scheduler speciale
- Privilege escalation
- API-uri dedicate
- Tratament special în kernel

Ele necesită:
- Acces la GPU (dispozitiv hardware real)
- Acces la audio (dispozitiv hardware real)
- Acces la input (dispozitiv hardware real)
- Performanță deterministă (deja în scheduler)
- Izolare memorie (deja în vm_model)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│  JOCUL (Game Process, user mode)                        │
│  - Vulkan userland library                              │
│  - OpenAL audio library                                 │
│  - Input handling library                               │
└──────────────┬──────────────────────────────────────────┘
               │
               │ syscall (descriptor I/O, mmap, events)
               │
┌──────────────▼──────────────────────────────────────────┐
│ KERNEL NGOS (kernel-core)                               │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Game Process Object                                 │ │
│ │  - AddressSpace (vm_model)                          │ │
│ │  - DescriptorNamespace (filedescs)                  │ │
│ │  - EventQueue (game waits for events)               │ │
│ │  - Scheduler entry (class: Interactive/Latency)    │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Device I/O Runtime                                  │ │
│ │  - GPU device descriptor (DeviceEndpoint)           │ │
│ │  - Audio device descriptor (DeviceEndpoint)         │ │
│ │  - Input device descriptor (DeviceEndpoint)         │ │
│ │  - Request queuing & completion                     │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Native Model (Domain/Resource/Contract)             │ │
│ │  - GPU Domain (owns GPU hardware resource)          │ │
│ │  - GPU Resource (capability to submit commands)     │ │
│ │  - Contract (game claims GPU, gets lease)           │ │
│ │  - State machine (Idle → InUse → Idle)              │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
└──────────────┬──────────────────────────────────────────┘
               │
               │ device request submit
               │
┌──────────────▼──────────────────────────────────────────┐
│ PLATFORM LAYER (platform-x86_64)                        │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Device Platform Model                               │ │
│ │  - PCI discovery (enumerates GPU/audio/input)       │ │
│ │  - BAR lifecycle (claim/map/unmap GPU MMIO)         │ │
│ │  - Interrupt routing (GPU interrupts)               │ │
│ │  - DMA mapping (GPU command buffers)                │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
└──────────────┬──────────────────────────────────────────┘
               │
               │ hardware access (MMIO, interrupts, DMA)
               │
┌──────────────▼──────────────────────────────────────────┐
│ REAL HARDWARE                                           │
│ - GPU (NVIDIA, AMD, Intel)                             │
│ - Audio Controller (HDA, USB)                          │
│ - Input Devices (USB, PS/2, Touchscreen)               │
└─────────────────────────────────────────────────────────┘
```

## GPU Support - Vertical Slice

### Subsistem: GPU I/O Device

**Responsabilități:**
- PCI device enumeration → GPU discovery
- BAR mapping → Kernel-space MMIO access for GPU registers
- Command buffer submission from user-space
- Interrupt handling for frame completion
- DMA for GPU-accessible memory

### Kernel Changes Required

#### 1. Extend `device_model.rs`

Add GPU device type to existing device classes:

```rust
// Current: DeviceClass::Network, DeviceClass::Storage, DeviceClass::Generic
// Add:
pub enum DeviceClass {
    Network,
    Storage,
    Graphics,  // NEW
    Audio,     // NEW
    Input,     // NEW
    Generic,
}
```

#### 2. Extend `device_runtime.rs`

GPU device handling alongside existing networking/storage:

```rust
// DeviceEndpoint already has:
// - path, owner, capability, class, state, driver
// - queue_capacity, pending_requests, completion_queue
// - submitted_requests, completed_requests

// GPU needs additional fields:
pub struct GpuDeviceEndpoint {
    // ... inherit from DeviceEndpoint ...
    vram_size: u64,
    vram_allocated: u64,
    current_frame_deadline: u64,
    frame_completion_count: u64,
    vram_mappings: Vec<VramMapping>,
}

// VramMapping tracks GPU memory regions
pub struct VramMapping {
    gpu_virtual_addr: u64,
    cpu_physical_addr: u64,
    size: u64,
    owner: ProcessId,
}
```

Operations:

```rust
pub enum GpuIoOperation {
    SubmitCommandBuffer {
        buffer_id: u64,
        size: u64,
        deadline_ns: u64,  // Frame deadline
    },
    MapVram {
        size: u64,
        flags: MmapFlags,
    },
    UnmapVram {
        addr: u64,
    },
    WaitForFrame,
    QueryStatus,
}
```

#### 3. Extend `platform-x86_64`

GPU device discovery (similar to VirtIO net):

```rust
// In device_platform.rs, add GPU discovery
pub fn enumerate_pci_gpus() -> Vec<DeviceRecord> {
    // Scan PCI for graphics devices
    // Typical PCI class: 0x03 (Display controller)
    // Devices: NVIDIA, AMD, Intel GPUs
    // Return DeviceRecord with:
    // - BAR for registers
    // - Interrupt configuration
    // - VRAM base address
}

// GPU driver init
pub fn init_gpu_driver(device: &PciDeviceRecord) -> Result<GpuDriver> {
    // 1. Claim BAR (MMIO region)
    let mmio_bar = device.bar(0)?;  // Typically BAR0
    let mmio_mapping = claim_and_map_bar(mmio_bar)?;
    
    // 2. Allocate DMA buffers for command submission
    let cmd_buffer_pool = allocate_dma_buffers(4 * 1024 * 1024)?;  // 4MB
    
    // 3. Setup interrupts
    let interrupt = allocate_interrupt(device)?;
    
    // 4. Initialize GPU state
    Ok(GpuDriver {
        mmio_mapping,
        cmd_buffer_pool,
        interrupt,
        frame_count: 0,
    })
}
```

#### 4. Extend `platform-hal`

GPU contract in HAL:

```rust
// In platform-hal/lib.rs, add:
pub trait GpuDevice {
    fn submit_command_buffer(&mut self, buffer: &[u8]) -> Result<()>;
    fn wait_for_completion(&mut self, timeout_ms: u32) -> Result<bool>;
    fn map_vram(&mut self, size: u64) -> Result<GpuMemoryHandle>;
    fn unmap_vram(&mut self, handle: GpuMemoryHandle) -> Result<()>;
}
```

#### 5. VFS Device Entry

Game opens `/dev/gpu/0`:

```rust
// In vfs_model.rs VFS tree:
// /dev/gpu/0 → GPU device descriptor
// Open returns descriptor fd (e.g., fd=3)

// Game uses mmap + ioctl on fd=3 for GPU interaction
```

### Userland Library: libngos-graphics

```rust
// user-runtime/src/lib.rs add:

pub struct GpuDevice {
    fd: i32,  // File descriptor to /dev/gpu/0
    vram_mapped: u64,
    frame_deadline: u64,
}

impl GpuDevice {
    pub fn open() -> Result<Self> {
        let fd = syscall::open("/dev/gpu/0", O_RDWR)?;
        Ok(Self {
            fd,
            vram_mapped: 0,
            frame_deadline: 16_666_667,  // 60 FPS in nanos
        })
    }
    
    pub fn submit_commands(&self, buffer: &[u8]) -> Result<()> {
        // ioctl on fd to submit command buffer
        unsafe {
            syscall::ioctl(self.fd, GPU_SUBMIT_CMD, buffer.as_ptr())
        }
    }
    
    pub fn wait_frame(&self) -> Result<()> {
        // Block on event queue until GPU signals completion
        syscall::wait_event(self.fd)
    }
}
```

### Game Usage Example

```rust
// Example game code (Rust, C, Vulkan driver)
use libngos_graphics::GpuDevice;

fn main() {
    let gpu = GpuDevice::open().expect("GPU not found");
    let vram = gpu.mmap_vram(256 * 1024 * 1024).expect("VRAM alloc fail");
    
    loop {
        // Game logic
        update_game_state();
        
        // Render
        let cmd_buffer = build_command_buffer(&vram);
        gpu.submit_commands(&cmd_buffer).expect("Submit fail");
        
        // Wait for frame
        gpu.wait_frame().expect("Frame wait fail");
    }
}
```

### Observability

Introspection syscalls expose GPU state:

```rust
// syscall_surface.rs
pub fn inspect_gpu(gpu_id: u32) -> Result<GpuIntrospection> {
    Ok(GpuIntrospection {
        device_path: "/dev/gpu/0",
        owner_pid: Some(1234),
        vram_total: 8_000_000_000,
        vram_used: 2_500_000_000,
        frame_count: 60_000,
        frame_deadline_ns: 16_666_667,
        pending_requests: 0,
        last_interrupt: Some(12_345_678),
    })
}
```

Host runtime reports:

```
== gpu-devices ==
/dev/gpu/0
  Owner: PID 1234 (game)
  VRAM: 2.5 GB / 8.0 GB
  Frames: 60000
  Deadline: 16.67ms (60 FPS)
  Status: ACTIVE
```

## Audio Support - Vertical Slice

### Subsistem: Audio I/O Device

**Similar to GPU but simpler:**

Audio device is real-time I/O:

```rust
pub enum AudioIoOperation {
    SubmitFrames {
        format: AudioFormat,  // PCM, 48kHz, stereo
        buffer_id: u64,
        frame_count: u32,
    },
    QueryLatency,
    SetPriority(SchedulerClass),  // Audio thread can be LatencyCritical
}

pub struct AudioDeviceEndpoint {
    // ... from DeviceEndpoint ...
    sample_rate: u32,
    channels: u8,
    latency_target_ms: u32,
    underrun_count: u64,
}
```

**Platform implementation:** HDA (High Definition Audio) driver

**Userland library:** OpenAL-compatible audio system

## Input Support - Vertical Slice

### Subsistem: Input Devices

Input devices (keyboard, mouse, gamepad, touch) map to `/dev/input/*`:

```rust
pub enum InputIoOperation {
    GetEvent,          // Poll for input
    SetDeadzone { axis: u8, percent: u8 },
    QueryCapabilities,
}

// Game reads events through event queue
```

Devices:
- `/dev/input/kbd0` - Keyboard (HID)
- `/dev/input/mouse0` - Mouse (USB)
- `/dev/input/gamepad0` - Gamepad (HID)
- `/dev/input/touch0` - Touchscreen (HID)

## Scheduler Integration

**No changes needed.** Scheduler already supports:

```rust
pub enum SchedulerClass {
    LatencyCritical,  // Audio/input threads
    Interactive,      // Game main thread (default)
    BestEffort,       // Background tasks
    Background,       // Cleanup/profiling
}
```

Game process:
- Main logic thread: `Interactive`
- Render thread: `Interactive`
- Audio thread: `LatencyCritical`
- Physics thread: `Interactive` or `BestEffort`

## Memory Management

**No changes needed.** Already supported:

- Process address space: `AddressSpace` in `process_model`
- Shared GPU memory: `mmap` with shared flags
- Copy-on-Write: Already in `vm_model`

Game benefits from:
- Large address space (64-bit only)
- Deterministic page faults (no unpredictable GC)
- Protected memory (no other process can access)

## Execution Contract Validation

**Complete vertical slice for GPU:**

✅ Real logic (GPU device discovery, command submission)
✅ Integration (device_model → platform-x86_64 → real hardware)
✅ Visible runtime effect (GPU processes frames)
✅ Observability (introspection syscalls)
✅ Exposed interface (VFS `/dev/gpu/0`, syscalls)
✅ Testing (test PCI enumeration, command submission, interrupts)

**Not a stub.** Not a demo. Real subsystem.

## Definition of Done

GPU subsystem is done when:

1. ✅ PCI enumeration discovers real GPUs in QEMU
2. ✅ Game opens `/dev/gpu/0`
3. ✅ Game submits command buffers via ioctl
4. ✅ GPU processes commands in sequence
5. ✅ Interrupts signal completion
6. ✅ Game waits on event queue for frame completion
7. ✅ VRAM mapping works for texture data
8. ✅ introspect_gpu() returns accurate state
9. ✅ host-runtime reports GPU activity
10. ✅ Tests cover PCI → driver → game pipeline
11. ✅ Tests cover error cases (invalid commands, out of VRAM, etc.)
12. ✅ Godot/Unreal can use GPU to render frames

## Implementation Order

### Phase 1: GPU Platform (2 weeks)
- PCI enumeration for GPUs
- BAR mapping for MMIO
- Interrupt routing
- Basic driver skeleton
- Tests for discovery

**Definition of done:** `enumerate_pci_gpus()` finds GPUs, maps BAR, handles interrupts

### Phase 2: GPU Kernel Integration (3 weeks)
- GPU device type in device_model
- GPU operations in device_runtime
- VFS `/dev/gpu/0` entry
- Device request queue
- Tests for kernel integration

**Definition of done:** Game opens `/dev/gpu/0`, submits requests, gets responses

### Phase 3: GPU Driver Logic (4 weeks)
- Command buffer parsing
- GPU state machine
- Frame deadline tracking
- VRAM allocation/mapping
- Tests for real command execution

**Definition of done:** Game submits Vulkan commands, GPU executes, frames render

### Phase 4: Userland Libraries (3 weeks)
- libngos-graphics (GPU abstraction)
- Vulkan driver wrapper
- libngos-audio (OpenAL)
- libngos-input (controller API)

**Definition of done:** Godot runs test game, renders 3D scene

### Phase 5: Engine Integration (2 weeks)
- Godot NGOS backend
- Test game execution
- Performance profiling
- Documentation

**Definition of done:** Unmodified Godot game runs on NGOS at 60 FPS

---

## What Changes

**kernel-core:**
- Add DeviceClass::Graphics, Audio, Input
- Extend GpuDeviceEndpoint, AudioDeviceEndpoint, InputDeviceEndpoint
- Syscalls for GPU/audio/input operations

**platform-x86_64:**
- GPU driver (vendor-specific or generic like VESA or VirtIO)
- Audio driver (HDA/USB audio)
- Input driver (USB HID)

**user-runtime:**
- libngos-graphics
- libngos-audio
- libngos-input

**userland-native:**
- Device enumeration service
- Input mapping service
- Audio mixer service (optional)

## What Doesn't Change

- Scheduler (already supports priorities)
- Process model (games are regular processes)
- Memory model (already supports large allocations)
- VFS (already supports device nodes)
- ABI (games use standard syscalls)
- Execution contract (games follow nano-semantic rules)

## Proofs of Concept

**Current codebase already validates:**

1. **Device subsystem** - Networking (VirtIO) works end-to-end
   - PCI discovery ✅
   - BAR mapping ✅
   - Interrupt handling ✅
   - DMA buffers ✅

2. **I/O runtime** - Device requests queue/complete
   - device_runtime.rs ✅
   - Completion tracking ✅

3. **Scheduler** - Multiple priority levels exist
   - 4 scheduler classes ✅

4. **Introspection** - Host runtime reports internal state
   - inspect_system() ✅
   - ProcessIntrospection ✅

**No speculative architecture.** Blueprint exists, validated by VirtIO.

## Risks

**CPU-bound games:** If game physics/logic is slow, GPU will sit idle. Kernel can't help. Game must optimize.

**VRAM pressure:** If game requests more VRAM than available, allocation fails. Game must reduce quality or reject.

**Real-time miss:** If render time > frame deadline, game misses frame. Kernel doesn't guarantee GPU completes in time (GPU is independent unit).

**Driver reliability:** GPU driver bugs can crash. Kernel must quarantine bad processes (already in process_model via isolation).

## No Special Cases

Games don't get:
- Kernel-space rendering
- Privileged GPU access
- Special memory permissions
- Bypass of standard I/O
- Different security model

Games get:
- What every process gets
- Standard device access
- Standard scheduler priorities
- Standard memory protection
- Standard observability

This is **NGOS principle**: No exceptions for marketing.

---

## Implementation Status

**Status:** Not started (architecture defined, ready for implementation)

**Blocker:** None. Platform-x86_64 GPU driver needs vendor-specific work (real GPU support or QEMU simulation).

**Next:** Pick real GPU (QEMU virtio-gpu, NVIDIA, AMD) and implement driver following Phase 1 plan.

---

## Conclusion

Game support in NGOS is straightforward device I/O integration. GPU/audio/input are hardware devices, no different from network cards that already work. 

Games don't need special treatment. They're processes. Processes already work.

The arch supports what games need. Build the drivers, not the mythology.
