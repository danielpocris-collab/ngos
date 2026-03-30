# Wasm Execution Policy

## Purpose

This document defines where WebAssembly is allowed in `ngos`, where it is not
allowed, and how it must be integrated.

This document is normative.

## Core Decision

WebAssembly is an execution model for `ngos` userland extensibility.

WebAssembly is not part of the `ngos` kernel foundation.

The approved direction is:

- native kernel and platform path for system truth
- WebAssembly for user-runtime components, plugins, and sandboxed applications

## Allowed Zones

WebAssembly is allowed in these areas:

- `user-runtime`
- `userland-native`
- future plugin, extension, automation, and application surfaces above the
  kernel boundary
- semantic user-facing execution surfaces that benefit from isolation and
  capability control

The preferred modern model is:

- `Wasm 3.0` as the execution substrate
- `WASI 0.2` for system-facing contracts
- component-oriented composition for capability-based integration

## Forbidden Zones

WebAssembly is forbidden in these areas:

- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- interrupt dispatch paths
- paging, traps, scheduler core, SMP bootstrap, and other low-level execution
- hardware driver hot paths
- any code that defines the real hardware closure path of the operating system

WebAssembly must not be introduced as a substitute for native kernel or
platform implementation.

## Integration Rules

If WebAssembly is used in `ngos`, it must follow these rules:

- WebAssembly components execute under `user-runtime`, not inside the kernel
- kernel services are exposed through explicit `ngos` capabilities, not raw
  unrestricted syscall mirrors
- no component receives implicit global authority
- every granted resource must be explicit: filesystem scope, clocks, IPC,
  logging, networking, UI, or other host services
- host interfaces must be semantic and narrow, not generic "escape hatches"
- observability is mandatory: component start, stop, failure, capability grant,
  and final state must be inspectable

## First Approved Front

The first valid Wasm front in `ngos` is:

- a real `user-runtime` component host
- explicit capability binding
- observable execution
- at least one real component that performs useful work through approved host
  interfaces

The first front must not be:

- a kernel-side Wasm experiment
- a demo-only interpreter
- a fake placeholder runtime
- a generic scripting surface without capability discipline

## Non-Goals

The following are not goals of Wasm adoption in `ngos`:

- replacing the native boot path
- replacing the native kernel path
- moving hardware drivers into Wasm
- creating a second system architecture parallel to the real OS path
- treating host-runtime-only Wasm behavior as closure of the native OS path

## Summary

The approved repository policy is:

- native for boot, platform, kernel, and drivers
- Wasm for user-runtime, plugins, extensions, and sandboxed applications
- capability-based integration only
- no Wasm below the kernel boundary
