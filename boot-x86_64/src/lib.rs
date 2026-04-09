#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", allow(static_mut_refs))]

//! Canonical subsystem role:
//! - subsystem: boot and diagnostics
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path: `boot-x86_64 -> platform-x86_64 -> kernel-core -> user-runtime -> userland-native -> QEMU`
//!
//! Canonical contract families produced here:
//! - boot contracts
//! - CPU/runtime bring-up contracts
//! - boot diagnostics contracts
//! - hardware handoff contracts
//!
//! This crate may activate hardware-facing mechanisms and produce facts,
//! diagnostics, and handoff state. It must not become the long-term semantic
//! owner of kernel subsystems that belong to `kernel-core`.

#[cfg(target_os = "none")]
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate alloc;

use platform_x86_64::{BootInfo, X86_64BootRequirements, X86_64KernelLayout};

#[cfg(any(target_os = "none", test))]
pub mod boot_audio_runtime;
#[cfg(target_os = "none")]
pub mod boot_facts;
#[cfg(any(target_os = "none", test))]
pub mod boot_gpu_runtime;
#[cfg(any(target_os = "none", test))]
pub mod boot_handoff_proof;
#[cfg(any(target_os = "none", test))]
pub mod boot_input_runtime;
pub mod boot_locator;
#[cfg(any(target_os = "none", test))]
pub mod boot_network_runtime;
pub mod cpu_apic;
#[cfg(target_os = "none")]
pub mod cpu_extended_state_buffer;
#[cfg(target_os = "none")]
pub mod cpu_features;
pub mod cpu_handoff;
pub mod cpu_hardware_provider;
pub mod cpu_runtime_status;
#[cfg(any(target_os = "none", test))]
pub mod cpu_tlb;
pub mod diagnostics;
#[cfg(target_os = "none")]
pub mod fault_diag;
#[cfg(target_os = "none")]
pub mod framebuffer;
#[cfg(target_os = "none")]
pub mod gdt;
#[cfg(any(target_os = "none", test))]
pub mod heap;
#[cfg(any(target_os = "none", test))]
pub mod irq_registry;
#[cfg(any(target_os = "none", test))]
pub mod keyboard;
#[cfg(target_os = "none")]
pub mod limine;
#[cfg(any(target_os = "none", test))]
pub mod paging;
#[cfg(any(target_os = "none", test))]
pub mod phys_alloc;
#[cfg(any(target_os = "none", test))]
pub mod pic;
#[cfg(target_os = "none")]
pub mod pit;
#[cfg(any(target_os = "none", test))]
pub mod reboot_trace;
pub mod serial;
pub mod smp;
#[cfg(target_os = "none")]
pub mod timer;
#[cfg(target_os = "none")]
pub mod traps;
#[cfg(any(target_os = "none", test))]
pub mod tty;
#[cfg(target_os = "none")]
pub mod ui_framebuffer;
#[cfg(any(target_os = "none", test))]
pub mod ui_presenter;
#[cfg(target_os = "none")]
pub mod ui_renderer;
#[cfg(not(target_os = "none"))]
pub mod ui_skia_preview;
pub mod user_bridge;
#[cfg(target_os = "none")]
pub mod user_process;
pub mod user_runtime_status;
#[cfg(any(target_os = "none", test))]
pub mod user_syscall;
#[cfg(any(target_os = "none", test))]
pub mod virtio_blk_boot;
#[cfg(target_os = "none")]
pub mod virtio_net_boot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarlyBootState<'a> {
    pub boot_info: BootInfo<'a>,
    pub layout: X86_64KernelLayout,
    pub boot_requirements: X86_64BootRequirements,
    pub bootstrap_span_bytes: u64,
    pub kernel_image_len: u64,
}

#[cfg(target_os = "none")]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn early_boot_info() -> Option<&'static BootInfo<'static>> {
    None
}

#[cfg(not(target_os = "none"))]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn early_boot_info() -> Option<&'static BootInfo<'static>> {
    None
}

#[cfg(target_os = "none")]
unsafe extern "Rust" {
    pub fn early_kernel_main();
}
