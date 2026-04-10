# Unified Compatibility Layer for NGOS

This document defines the single compatibility layer direction for NGOS.

The goal is not to create separate "Wine" and "Proton" products.
The goal is one NGOS compatibility layer that can host:

- Windows application compatibility
- game-oriented graphics compatibility
- source API translation for DirectX, OpenGL, Metal, and Vulkan
- one runtime, one manifest format, one observability path

The layer remains an adapter, not a kernel identity.
It maps foreign-facing behavior into NGOS-owned runtime and device contracts.

## High-Level Shape

```
foreign app / game
    -> compatibility manifest
    -> translation plan
    -> runtime/session orchestration
    -> graphics/audio/input lanes
    -> NGOS device/resource/contract model
    -> kernel/device/runtime execution
```

## Responsibilities

### 1. Source API classification

Every compatibility target declares:

- source API
- backend execution target
- profile
- shims / paths / environment

Examples:

- `gfx.api=directx11`
- `gfx.api=directx12`
- `gfx.api=opengl`
- `gfx.api=metal`
- `gfx.api=vulkan`

The source API is not the backend.
The source API is the foreign-facing contract.
The backend is the NGOS execution target.

### 2. Translation plan

The compatibility layer must materialize a translation plan for each session.

The plan should expose:

- source API name
- backend name
- translation label
- lane contracts
- observable session state

This is the line between "metadata" and "real runtime behavior".

### 3. Session orchestration

The runtime must launch a compatibility session that owns:

- graphics lane
- audio lane
- input lane
- shims / env files / argv files / channel files

The session is responsible for:

- launch
- submit
- watch
- stop
- recovery / refusal on invalid state

### 4. Graphics execution

Graphics compatibility is implemented as a translation pipeline:

- source API request
- normalized frame script
- NGOS draw operations
- device submit / present / completion

The current NGOS graphics semantic path already uses:

- `FrameScript`
- `DrawOp`
- validation
- encoding

That is the runtime target for the compatibility layer.

## Files and Roles

### Manifest / translation layer

- [`game-compat-runtime/src/lib.rs`](/C:/Users/pocri/OneDrive/Desktop/experiment/game-compat-runtime/src/lib.rs)
  - source API parsing
  - backend parsing
  - translation plan construction
  - session plan generation

### Runtime / shell exposure

- [`userland-native/src/lib.rs`](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/lib.rs)
  - manifest rendering
  - session rendering
  - runtime observability
  - lane reporting

- [`userland-native/src/gpu_agents.rs`](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/gpu_agents.rs)
  - GPU-facing shell commands
  - driver/device inspection
  - watch / submit / lease / probe commands

- [`userland-native/src/game_agents.rs`](/C:/Users/pocri/OneDrive/Desktop/experiment/userland-native/src/game_agents.rs)
  - launch / stop / plan / simulate / watch lifecycle

### Graphics semantics

- [`gfx-translate/src/frame_script_agent.rs`](/C:/Users/pocri/OneDrive/Desktop/experiment/gfx-translate/src/frame_script_agent.rs)
  - frame scripts
  - validation
  - encoding

- [`gfx-translate/src/render_command_agent.rs`](/C:/Users/pocri/OneDrive/Desktop/experiment/gfx-translate/src/render_command_agent.rs)
  - draw operation model
  - command parsing
  - classing / validation

### Runtime execution path

- [`user-runtime`](/C:/Users/pocri/OneDrive/Desktop/experiment/user-runtime)
  - syscall runtime and process execution bridge

- [`kernel-core`](/C:/Users/pocri/OneDrive/Desktop/experiment/kernel-core)
  - device/resource/contract enforcement
  - GPU/audio/input syscall surfaces

- [`platform-hal`](/C:/Users/pocri/OneDrive/Desktop/experiment/platform-hal)
  - device platform contracts

## Non-Goals

This layer is not:

- a separate Wine project
- a separate Proton project
- a kernel rewrite
- a fake compatibility surface
- a demo-only launcher

It must remain one owned NGOS subsystem.

## Closure Criteria

This subsystem is not closed until:

- source API translation is implemented for the relevant families
- backend execution is integrated into the real runtime path
- refusal/error paths are observable
- recovery or release paths are observable
- the runtime reports the final translation state
- the real execution chain is exercised, not only a host-only preview

