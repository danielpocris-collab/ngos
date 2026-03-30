#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", allow(static_mut_refs))]

#[cfg(target_os = "none")]
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate alloc;

use platform_x86_64::{BootInfo, X86_64BootRequirements, X86_64KernelLayout};

#[cfg(target_os = "none")]
pub mod boot_facts;
pub mod boot_locator;
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
unsafe extern "Rust" {
    pub fn early_kernel_main();
}
