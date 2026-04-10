#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", feature(alloc_error_handler))]
#![cfg_attr(target_os = "none", allow(static_mut_refs))]

#[cfg(target_os = "none")]
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate alloc;

use platform_x86_64::{BootInfo, X86_64BootRequirements, X86_64KernelLayout};

#[cfg(target_os = "none")]
use core::arch::asm;
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
use core::arch::global_asm;
#[cfg(target_os = "none")]
use core::hint::spin_loop;
#[cfg(target_os = "none")]
use core::mem::MaybeUninit;
#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use core::ptr;
#[cfg(target_os = "none")]
use platform_x86_64::highest_bootstrap_physical_address;
#[cfg(target_os = "none")]
use platform_x86_64::{PAGE_SIZE_4K, align_up};

#[cfg(target_os = "none")]
const RUNTIME_HEAP_FRAME_COUNT: usize = 12288;

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
fn enable_sse() {
    unsafe {
        let mut cr0: u64;
        asm!("mov {}, cr0", out(reg) cr0, options(nostack, preserves_flags));
        cr0 &= !(1 << 2);
        cr0 |= 1 << 1;
        asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));

        let mut cr4: u64;
        asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
        cr4 |= (1 << 9) | (1 << 10);
        asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));
    }
}

#[cfg(target_os = "none")]
mod boot_audio_runtime;
#[cfg(target_os = "none")]
mod boot_facts;
#[cfg(target_os = "none")]
mod boot_gpu_runtime;
#[cfg(any(target_os = "none", test))]
mod boot_handoff_proof;
#[cfg(target_os = "none")]
mod boot_input_runtime;
mod boot_locator;
#[cfg(target_os = "none")]
mod boot_network_runtime;
mod cpu_apic;
#[cfg(target_os = "none")]
mod cpu_extended_state_buffer;
#[cfg(target_os = "none")]
mod cpu_features;
mod cpu_handoff;
mod cpu_hardware_provider;
mod cpu_runtime_status;
#[cfg(any(target_os = "none", test))]
mod cpu_tlb;
mod diagnostics;
#[cfg(target_os = "none")]
mod fault_diag;
#[cfg(target_os = "none")]
mod framebuffer;
#[cfg(target_os = "none")]
mod gdt;
#[cfg(target_os = "none")]
mod heap;
#[cfg(target_os = "none")]
mod irq_registry;
#[cfg(target_os = "none")]
mod keyboard;
#[cfg(target_os = "none")]
mod limine;
#[cfg(target_os = "none")]
mod paging;
#[cfg(target_os = "none")]
mod phys_alloc;
#[cfg(target_os = "none")]
mod pic;
#[cfg(target_os = "none")]
mod pit;
#[cfg(target_os = "none")]
mod reboot_trace;
#[cfg(target_os = "none")]
mod runtime_kernel_stack;
mod serial;
mod smp;
#[cfg(target_os = "none")]
mod timer;
#[cfg(target_os = "none")]
mod traps;
#[cfg(target_os = "none")]
mod tty;
mod user_bridge;
#[cfg(target_os = "none")]
mod user_process;
mod user_runtime_status;
#[cfg(target_os = "none")]
mod user_syscall;
#[cfg(target_os = "none")]
mod virtio_blk_boot;
#[cfg(target_os = "none")]
mod virtio_net_boot;

#[cfg(target_os = "none")]
global_asm!(include_str!("entry.S"));
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
global_asm!(include_str!("smp_trampoline.S"));

#[cfg(target_os = "none")]
macro_rules! boot_logln {
    () => {
        $crate::serial::print(format_args!("\n"))
    };
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::serial::print(format_args!(concat!($fmt, "\n") $(, $args)*))
    };
}

#[cfg(target_os = "none")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootFailure {
    Gdt(crate::gdt::GdtBringupError),
    Heap(crate::heap::HeapInitError),
    Limine(crate::limine::LimineBootError),
    Paging(crate::paging::PagingBringupError),
    PhysicalAllocator(crate::phys_alloc::PhysAllocBringupError),
    Traps(crate::traps::TrapBringupError),
    UserProcess(crate::user_process::UserProcessError),
    UserMode(platform_x86_64::user_mode::UserModeError),
}

#[cfg(any(target_os = "none", test))]
fn boot_failure_summary(failure: BootFailure) -> (&'static str, &'static str) {
    match failure {
        BootFailure::Gdt(_) => ("gdt", "bringup-failed"),
        BootFailure::Heap(_) => ("heap", "init-failed"),
        BootFailure::Limine(error) => (error.summary_family(), error.summary_detail()),
        BootFailure::Paging(_) => ("paging", "bringup-failed"),
        BootFailure::PhysicalAllocator(_) => ("phys-alloc", "bringup-failed"),
        BootFailure::Traps(_) => ("traps", "bringup-failed"),
        BootFailure::UserProcess(_) => ("user-process", "prepare-failed"),
        BootFailure::UserMode(_) => ("user-mode", "launch-failed"),
    }
}

#[cfg(any(target_os = "none", test))]
fn boot_locator_failure_summary(
    locator: crate::boot_locator::BootLocatorRecord,
) -> (
    crate::boot_locator::BootLocatorStage,
    u64,
    &'static str,
    crate::boot_locator::BootPayloadLabel,
    u64,
    crate::boot_locator::BootPayloadLabel,
    u64,
) {
    (
        locator.stage,
        locator.checkpoint,
        crate::boot_locator::checkpoint_name(locator.stage, locator.checkpoint),
        locator.payload0_label,
        locator.payload0,
        locator.payload1_label,
        locator.payload1,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarlyBootState<'a> {
    pub boot_info: BootInfo<'a>,
    pub layout: X86_64KernelLayout,
    pub boot_requirements: X86_64BootRequirements,
    pub bootstrap_span_bytes: u64,
    pub kernel_image_len: u64,
}

#[cfg(target_os = "none")]
static mut EARLY_BOOT_STATE: MaybeUninit<EarlyBootState<'static>> = MaybeUninit::uninit();

#[cfg(target_os = "none")]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn early_boot_info() -> Option<&'static BootInfo<'static>> {
    Some(unsafe { &(*ptr::addr_of!(EARLY_BOOT_STATE).cast::<EarlyBootState<'static>>()).boot_info })
}

#[cfg(not(target_os = "none"))]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn early_boot_info() -> Option<&'static BootInfo<'static>> {
    None
}

#[cfg(target_os = "none")]
fn framebuffer_boot_trace_header() {
    if crate::framebuffer::is_available() {
        crate::framebuffer::status_banner("NGOS BOOT TRACE");
    }
}

#[cfg(target_os = "none")]
fn framebuffer_boot_trace_snapshot() {
    if !crate::framebuffer::is_available() {
        return;
    }
    let locator = crate::boot_locator::snapshot();
    let checkpoint_name = crate::boot_locator::checkpoint_name(locator.stage, locator.checkpoint);
    crate::framebuffer::print(format_args!(
        "stage={:?}\ncheckpoint={:#x}\nname={}\n\n",
        locator.stage, locator.checkpoint, checkpoint_name
    ));
}

#[cfg(target_os = "none")]
fn boot_checkpoint(
    state: &EarlyBootState<'static>,
    stage: crate::boot_locator::BootLocatorStage,
    checkpoint: u64,
) {
    crate::boot_locator::checkpoint(stage, checkpoint, 0, 0);
    maybe_halt_at_boot_checkpoint(state, stage, checkpoint);
}

#[cfg(target_os = "none")]
fn maybe_halt_at_boot_checkpoint(
    state: &EarlyBootState<'static>,
    stage: crate::boot_locator::BootLocatorStage,
    checkpoint: u64,
) {
    let Some(command_line) = state.boot_info.command_line else {
        return;
    };
    if !boot_halt_requested(command_line, stage, checkpoint) {
        return;
    }

    let checkpoint_name = crate::boot_locator::checkpoint_name(stage, checkpoint);
    boot_logln!(
        "ngos/x86_64: boot halt requested at stage={:?} checkpoint={:#x} name={}",
        stage,
        checkpoint,
        checkpoint_name
    );
    crate::framebuffer::alert_banner("BOOT HALT");
    crate::framebuffer::print(format_args!(
        "stage={:?}\ncheckpoint={:#x}\nname={}\n",
        stage, checkpoint, checkpoint_name
    ));
    halt_loop()
}

#[cfg(target_os = "none")]
fn boot_halt_requested(
    command_line: &str,
    stage: crate::boot_locator::BootLocatorStage,
    checkpoint: u64,
) -> bool {
    command_line.split_whitespace().any(|token| {
        let Some(value) = token.strip_prefix("ngos.boot.halt=") else {
            return false;
        };
        boot_halt_selector_matches(value, stage, checkpoint)
    })
}

#[cfg(target_os = "none")]
fn boot_flag_enabled(command_line: Option<&str>, flag: &str) -> bool {
    let Some(command_line) = command_line else {
        return false;
    };
    command_line.split_whitespace().any(|token| token == flag)
}

#[cfg_attr(not(test), allow(dead_code))]
fn boot_halt_selector_matches(
    selector: &str,
    stage: crate::boot_locator::BootLocatorStage,
    checkpoint: u64,
) -> bool {
    if selector.eq_ignore_ascii_case("post-paging") {
        return stage == crate::boot_locator::BootLocatorStage::Paging && checkpoint == 0x340;
    }

    if selector.eq_ignore_ascii_case(crate::boot_locator::checkpoint_name(stage, checkpoint)) {
        return true;
    }

    let Some((stage_name, checkpoint_name)) = selector.split_once(':') else {
        return parse_checkpoint_value(selector) == Some(checkpoint);
    };

    stage_name_matches(stage_name, stage)
        && (parse_checkpoint_value(checkpoint_name) == Some(checkpoint)
            || checkpoint_name
                .eq_ignore_ascii_case(crate::boot_locator::checkpoint_name(stage, checkpoint)))
}

#[cfg_attr(not(test), allow(dead_code))]
fn stage_name_matches(stage_name: &str, stage: crate::boot_locator::BootLocatorStage) -> bool {
    let expected = match stage {
        crate::boot_locator::BootLocatorStage::Reset => "reset",
        crate::boot_locator::BootLocatorStage::Stage0 => "stage0",
        crate::boot_locator::BootLocatorStage::Limine => "limine",
        crate::boot_locator::BootLocatorStage::EarlyKernel => "early-kernel",
        crate::boot_locator::BootLocatorStage::Paging => "paging",
        crate::boot_locator::BootLocatorStage::Traps => "traps",
        crate::boot_locator::BootLocatorStage::Smp => "smp",
        crate::boot_locator::BootLocatorStage::User => "user",
        crate::boot_locator::BootLocatorStage::Fault => "fault",
    };
    stage_name.eq_ignore_ascii_case(expected)
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_checkpoint_value(raw: &str) -> Option<u64> {
    raw.strip_prefix("0x")
        .map(|value| u64::from_str_radix(value, 16).ok())
        .unwrap_or_else(|| raw.parse::<u64>().ok())
}

#[cfg(target_os = "none")]
#[global_allocator]
static GLOBAL_ALLOCATOR: crate::heap::EarlyHeapAllocator = crate::heap::GLOBAL_ALLOCATOR;

#[cfg(target_os = "none")]
unsafe extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
    static __kernel_start: u8;
    static __kernel_end: u8;
    static __ngos_boot_stack_bottom: u8;
    static __ngos_boot_stack_top: u8;
}

#[unsafe(no_mangle)]
#[cfg(target_os = "none")]
pub extern "C" fn x86_64_boot_stage0() -> ! {
    serial::debug_marker(b'C');
    crate::boot_locator::reset();
    crate::boot_locator::early_reset();
    crate::boot_locator::early_event(
        crate::boot_locator::BootLocatorStage::Stage0,
        crate::boot_locator::BootLocatorKind::Progress,
        crate::boot_locator::BootLocatorSeverity::Info,
        0x10,
        crate::boot_locator::BootPayloadLabel::Status,
        0,
        crate::boot_locator::BootPayloadLabel::None,
        0,
    );
    serial::debug_marker(b'c');
    zero_bss();
    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x20,
        0,
        0,
    );
    serial::debug_marker(b'd');
    serial::init();
    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x30,
        0,
        0,
    );
    serial::debug_marker(b'e');
    enable_sse();
    serial::debug_marker(b'f');
    serial::debug_marker(b'F');
    // Stage0 runs before the rest of diagnostics state is proven safe; keep
    // locator coverage here and defer trace-ring stage recording until later.
    serial::debug_marker(b'G');
    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x43,
        0,
        0,
    );
    boot_logln!("ngos/x86_64: stage0 entered");
    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x44,
        0,
        0,
    );
    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x70,
        0,
        0,
    );
    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x45,
        0,
        0,
    );
    serial::debug_marker(b'g');

    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x46,
        0,
        0,
    );
    let kernel_start = ptr::addr_of!(__kernel_start) as u64;
    let kernel_end = ptr::addr_of!(__kernel_end) as u64;
    let kernel_image_len = align_up(kernel_end.saturating_sub(kernel_start), PAGE_SIZE_4K);
    crate::boot_locator::early_event(
        crate::boot_locator::BootLocatorStage::Stage0,
        crate::boot_locator::BootLocatorKind::Memory,
        crate::boot_locator::BootLocatorSeverity::Info,
        0x80,
        crate::boot_locator::BootPayloadLabel::Address,
        kernel_start,
        crate::boot_locator::BootPayloadLabel::Length,
        kernel_image_len,
    );
    crate::boot_locator::early_checkpoint(
        crate::boot_locator::BootLocatorStage::Stage0,
        0x47,
        kernel_start,
        kernel_image_len,
    );
    boot_logln!("ngos/x86_64: kernel image span = {:#x}", kernel_image_len);

    if let Some((name, version)) = crate::limine::bootloader_identity() {
        boot_logln!("ngos/x86_64: bootloader = {} {}", name, version);
    }

    let mut boot_info = MaybeUninit::<BootInfo<'static>>::uninit();
    crate::limine::write_boot_info(boot_info.as_mut_ptr(), kernel_image_len).unwrap_or_else(
        |error| {
            fail(BootFailure::Limine(error));
        },
    );
    crate::boot_locator::checkpoint(crate::boot_locator::BootLocatorStage::Limine, 0x90, 0, 0);
    serial::debug_marker(b'Y');
    initialize_early_boot_state(unsafe { boot_info.assume_init() }, kernel_image_len);
    serial::debug_marker(b'Z');
    serial::debug_marker(b'1');
    let state_ref = unsafe { &*ptr::addr_of!(EARLY_BOOT_STATE).cast::<EarlyBootState<'static>>() };
    early_kernel_main(state_ref)
}

#[cfg(target_os = "none")]
fn initialize_early_boot_state(boot_info: BootInfo<'static>, kernel_image_len: u64) {
    serial::debug_marker(b'2');
    let boot_stack_base = ptr::addr_of!(__ngos_boot_stack_bottom) as u64;
    let boot_stack_top = ptr::addr_of!(__ngos_boot_stack_top) as u64;
    let default_layout = X86_64KernelLayout::higher_half_default();
    let direct_map_base = boot_info.physical_memory_offset;
    let layout = X86_64KernelLayout::new(
        default_layout.kernel_base,
        direct_map_base,
        default_layout.direct_map_size,
        boot_stack_base,
        boot_stack_top.saturating_sub(boot_stack_base),
    );
    serial::debug_marker(b'3');
    let requirements = X86_64BootRequirements::baseline();
    serial::debug_marker(b'4');
    let direct_map_phys_len = highest_bootstrap_physical_address(&boot_info);
    serial::debug_marker(b'5');
    let bootstrap_span_bytes = direct_map_phys_len
        .saturating_add(kernel_image_len)
        .saturating_add(2 * 1024 * 1024);
    serial::debug_marker(b'6');
    serial::debug_marker(b'7');
    unsafe {
        let state_ptr = ptr::addr_of_mut!(EARLY_BOOT_STATE).cast::<EarlyBootState<'static>>();
        ptr::addr_of_mut!((*state_ptr).boot_info).write(boot_info);
        serial::debug_marker(b'8');
        ptr::addr_of_mut!((*state_ptr).layout.kernel_base).write(layout.kernel_base);
        ptr::addr_of_mut!((*state_ptr).layout.direct_map_base).write(layout.direct_map_base);
        ptr::addr_of_mut!((*state_ptr).layout.direct_map_size).write(layout.direct_map_size);
        ptr::addr_of_mut!((*state_ptr).layout.boot_stack_base).write(layout.boot_stack_base);
        ptr::addr_of_mut!((*state_ptr).layout.boot_stack_size).write(layout.boot_stack_size);
        serial::debug_marker(b'9');
        ptr::addr_of_mut!((*state_ptr).boot_requirements.minimum_loader_alignment)
            .write(requirements.minimum_loader_alignment);
        ptr::addr_of_mut!((*state_ptr).boot_requirements.page_table_granularity)
            .write(requirements.page_table_granularity);
        ptr::addr_of_mut!((*state_ptr).boot_requirements.stack_alignment)
            .write(requirements.stack_alignment);
        ptr::addr_of_mut!((*state_ptr).boot_requirements.nx_enabled).write(requirements.nx_enabled);
        ptr::addr_of_mut!((*state_ptr).boot_requirements.write_protect_enabled)
            .write(requirements.write_protect_enabled);
        serial::debug_marker(b'a');
        ptr::addr_of_mut!((*state_ptr).bootstrap_span_bytes).write(bootstrap_span_bytes);
        serial::debug_marker(b'b');
        ptr::addr_of_mut!((*state_ptr).kernel_image_len).write(kernel_image_len);
        serial::debug_marker(b'c');
    }
    crate::reboot_trace::init(boot_info.physical_memory_offset);
    serial::print(format_args!(
        "ngos/x86_64: early layout kernel_base={:#x} direct_map_base={:#x} default_direct_map_base={:#x}\n",
        layout.kernel_base, layout.direct_map_base, default_layout.direct_map_base
    ));
    if layout.direct_map_base != default_layout.direct_map_base {
        serial::print(format_args!(
            "ngos/x86_64: loader supplied non-default HHDM; bootstrap paging will follow loader offset\n"
        ));
    }
    if let Some(framebuffer) = boot_info.framebuffer {
        crate::framebuffer::init(framebuffer, boot_info.physical_memory_offset);
        if crate::framebuffer::is_available() {
            framebuffer_boot_trace_header();
            boot_logln!("ngos/x86_64: framebuffer console online");
            framebuffer_boot_trace_snapshot();
        }
    }
}

#[cfg(target_os = "none")]
fn early_kernel_main(state: &EarlyBootState<'static>) -> ! {
    serial::debug_marker(b'D');
    crate::boot_locator::event(
        crate::boot_locator::BootLocatorStage::EarlyKernel,
        crate::boot_locator::BootLocatorKind::Contract,
        crate::boot_locator::BootLocatorSeverity::Info,
        0x100,
        crate::boot_locator::BootPayloadLabel::Count,
        state.boot_info.modules.len() as u64,
        crate::boot_locator::BootPayloadLabel::Count,
        state.boot_info.memory_regions.len() as u64,
    );
    crate::diagnostics::record_boot_stage(
        crate::diagnostics::BootTraceStage::EarlyKernelMain,
        None,
        state.boot_info.modules.len() as u64,
    );
    let reprobe = crate::diagnostics::reprobe_policy_on_boot();
    crate::diagnostics::set_mode(reprobe.mode);
    crate::diagnostics::emit_boot_failure_history_summary();
    crate::diagnostics::trace_emit(
        crate::diagnostics::TraceKind::Transition,
        crate::diagnostics::TraceChannel::Transition,
        crate::diagnostics::BootTraceStage::EarlyKernelMain as u16,
        reprobe.mode as u64,
        reprobe.target_path as u64,
        reprobe.target_stage as u64,
        reprobe.target_checkpoint,
    );
    framebuffer_boot_trace_header();
    framebuffer_boot_trace_snapshot();
    boot_logln!("ngos/x86_64: early_kernel_main reached");
    boot_logln!(
        "ngos/x86_64: protocol={:?} hhdm={:#x} kernel_phys={:#x}..{:#x}",
        state.boot_info.protocol,
        state.boot_info.physical_memory_offset,
        state.boot_info.kernel_phys_range.start,
        state.boot_info.kernel_phys_range.end()
    );
    boot_logln!(
        "ngos/x86_64: memory_regions={} modules={} bootstrap_span={:#x}",
        state.boot_info.memory_regions.len(),
        state.boot_info.modules.len(),
        state.bootstrap_span_bytes
    );
    let previous_reboot_trace = crate::reboot_trace::previous_snapshot();
    if previous_reboot_trace.valid {
        boot_logln!(
            "ngos/x86_64: reboot-trace previous_boot={} clean_shutdown={} phys={:#x} bytes={:#x}",
            previous_reboot_trace.boot_generation,
            previous_reboot_trace.completed_cleanly,
            crate::reboot_trace::physical_base(),
            crate::reboot_trace::span_bytes()
        );
        if previous_reboot_trace.last_record.sequence != 0 {
            let last = previous_reboot_trace.last_record;
            boot_logln!(
                "ngos/x86_64: reboot-trace last stage={:?} checkpoint={:#x} name={} seq={} payload0={:#x} payload1={:#x}",
                last.stage,
                last.checkpoint,
                crate::boot_locator::checkpoint_name(last.stage, last.checkpoint),
                last.sequence,
                last.payload0,
                last.payload1
            );
        }
    }
    crate::boot_facts::emit_boot_facts(&state.boot_info);
    if let Some(command_line) = state.boot_info.command_line {
        boot_logln!("ngos/x86_64: cmdline=\"{}\"", command_line);
    }
    if let Some(framebuffer) = state.boot_info.framebuffer {
        boot_logln!(
            "ngos/x86_64: framebuffer={}x{} pitch={} bpp={}",
            framebuffer.width,
            framebuffer.height,
            framebuffer.pitch,
            framebuffer.bpp
        );
    }
    let _ = state.boot_requirements;
    let _ = state
        .layout
        .boot_stack_base
        .saturating_add(state.layout.boot_stack_size);
    let mut frame_allocator = MaybeUninit::<crate::phys_alloc::BootFrameAllocator>::uninit();
    boot_logln!("ngos/x86_64: physical allocator init start");
    serial::debug_marker(b'g');
    unsafe {
        frame_allocator
            .as_mut_ptr()
            .write(crate::phys_alloc::BootFrameAllocator::new());
    }
    serial::debug_marker(b'h');
    let frame_allocator = unsafe { &mut *frame_allocator.as_mut_ptr() };
    frame_allocator.clear();
    serial::debug_marker(b'i');
    for &region in state.boot_info.memory_regions {
        if region.kind == platform_x86_64::BootMemoryRegionKind::Usable {
            frame_allocator
                .add_usable_region(region.start, region.len)
                .unwrap_or_else(|error| fail(BootFailure::PhysicalAllocator(error.into())));
        }
    }
    serial::debug_marker(b'j');
    let initial = frame_allocator.stats();
    if initial.usable_frames == 0 {
        fail(BootFailure::PhysicalAllocator(
            platform_x86_64::FrameAllocatorError::NoUsableMemory.into(),
        ));
    }
    serial::debug_marker(b'k');
    let mut reserved_frames = frame_allocator
        .reserve_range(0, 0x10_0000)
        .unwrap_or_else(|error| fail(BootFailure::PhysicalAllocator(error.into())));
    serial::debug_marker(b'l');
    reserved_frames = reserved_frames.saturating_add(
        frame_allocator
            .reserve_range(
                state.boot_info.kernel_phys_range.start,
                state.boot_info.kernel_phys_range.len,
            )
            .unwrap_or_else(|error| fail(BootFailure::PhysicalAllocator(error.into()))),
    );
    serial::debug_marker(b'm');
    for &module in state.boot_info.modules {
        reserved_frames = reserved_frames.saturating_add(
            frame_allocator
                .reserve_range(module.physical_start, module.len)
                .unwrap_or_else(|error| fail(BootFailure::PhysicalAllocator(error.into()))),
        );
    }
    serial::debug_marker(b'n');
    if let Some(framebuffer) = state.boot_info.framebuffer {
        let framebuffer_len = (framebuffer.pitch as u64).saturating_mul(framebuffer.height as u64);
        reserved_frames = reserved_frames.saturating_add(
            frame_allocator
                .reserve_range(framebuffer.physical_start, framebuffer_len)
                .unwrap_or_else(|error| fail(BootFailure::PhysicalAllocator(error.into()))),
        );
    }
    serial::debug_marker(b'G');
    serial::print(format_args!(
        "ngos/x86_64: phys alloc init usable_regions={} usable_frames={} reserved_frames={} free_frames={}\n",
        initial.usable_regions,
        initial.usable_frames,
        reserved_frames,
        frame_allocator.stats().free_frames
    ));
    crate::diagnostics::record_boot_stage(
        crate::diagnostics::BootTraceStage::PhysAllocReady,
        None,
        frame_allocator.stats().free_frames as u64,
    );
    framebuffer_boot_trace_header();
    framebuffer_boot_trace_snapshot();
    crate::phys_alloc::log_state("ready", frame_allocator);
    let heap_frames = crate::heap::init_from_allocator(
        frame_allocator,
        state.boot_info.physical_memory_offset,
        RUNTIME_HEAP_FRAME_COUNT,
    )
    .unwrap_or_else(|error| fail(BootFailure::Heap(error)));
    boot_logln!(
        "ngos/x86_64: runtime built heap_phys={:#x} frames={} used_bytes={}",
        heap_frames.start,
        heap_frames.frame_count,
        crate::heap::allocated_bytes()
    );
    crate::boot_facts::emit_smp_facts(&state.boot_info);
    boot_logln!("ngos/x86_64: paging bring-up start");
    let paging_state = crate::paging::bring_up(state, frame_allocator).unwrap_or_else(|error| {
        fail(BootFailure::Paging(error));
    });
    crate::diagnostics::record_boot_stage(crate::diagnostics::BootTraceStage::PagingReady, None, 0);
    boot_checkpoint(state, crate::boot_locator::BootLocatorStage::Paging, 0x341);
    framebuffer_boot_trace_header();
    framebuffer_boot_trace_snapshot();
    crate::phys_alloc::log_state("post-paging", frame_allocator);
    let runtime_kernel_stack = crate::runtime_kernel_stack::allocate(
        frame_allocator,
        state.boot_info.physical_memory_offset,
    )
    .unwrap_or_else(|error| {
        fail(BootFailure::Heap(
            crate::heap::HeapInitError::AllocateFrames(error),
        ))
    });
    boot_logln!(
        "ngos/x86_64: runtime kernel stack base={:#x} top={:#x} bytes={:#x}",
        runtime_kernel_stack.base,
        runtime_kernel_stack.top,
        runtime_kernel_stack.bytes
    );
    boot_logln!("ngos/x86_64: cpu extended-state bring-up start");
    let extended_state = crate::cpu_features::enable_cpu_extended_state();
    crate::cpu_runtime_status::record(
        extended_state.sse_ready,
        extended_state.xsave_enabled,
        extended_state.save_area_bytes,
        extended_state.fsgsbase_enabled,
        extended_state.pcid_enabled,
        extended_state.invpcid_available,
        extended_state.pku_enabled,
        extended_state.smep_enabled,
        extended_state.smap_enabled,
        extended_state.umip_enabled,
        extended_state.xcr0,
    );
    let extended_state_probe =
        crate::cpu_features::probe_boot_extended_state_roundtrip(&extended_state);
    crate::cpu_runtime_status::record_probe(
        extended_state_probe.attempted,
        extended_state_probe.saved,
        extended_state_probe.restored,
        extended_state_probe.required_bytes,
        extended_state_probe.refusal as u32,
        extended_state_probe.seed_marker,
    );
    boot_logln!(
        "ngos/x86_64: cpu extended-state sse={} xsave={} save_area={} fsgsbase={} pcid={} invpcid={} pku={} smep={} smap={} umip={} xcr0={:#x} probe_attempted={} probe_saved={} probe_restored={} probe_refusal={} cr0={:#x} cr4={:#x}",
        extended_state.sse_ready,
        extended_state.xsave_enabled,
        extended_state.save_area_bytes,
        extended_state.fsgsbase_enabled,
        extended_state.pcid_enabled,
        extended_state.invpcid_available,
        extended_state.pku_enabled,
        extended_state.smep_enabled,
        extended_state.smap_enabled,
        extended_state.umip_enabled,
        extended_state.xcr0,
        extended_state_probe.attempted,
        extended_state_probe.saved,
        extended_state_probe.restored,
        extended_state_probe.refusal as u32,
        crate::cpu_features::read_cr0_local(),
        crate::cpu_features::read_cr4_local()
    );
    boot_checkpoint(state, crate::boot_locator::BootLocatorStage::Traps, 0x420);
    boot_logln!("ngos/x86_64: gdt bring-up start");
    crate::gdt::bring_up(runtime_kernel_stack.top).unwrap_or_else(|error| {
        fail(BootFailure::Gdt(error));
    });
    boot_checkpoint(state, crate::boot_locator::BootLocatorStage::Traps, 0x430);
    boot_logln!("ngos/x86_64: idt bring-up start");
    crate::traps::bring_up(runtime_kernel_stack.top).unwrap_or_else(|error| {
        fail(BootFailure::Traps(error));
    });
    boot_checkpoint(state, crate::boot_locator::BootLocatorStage::Traps, 0x440);
    boot_logln!("ngos/x86_64: traps bring-up complete");
    crate::diagnostics::record_boot_stage(
        crate::diagnostics::BootTraceStage::TrapsReady,
        None,
        runtime_kernel_stack.top,
    );
    boot_logln!("ngos/x86_64: traps stage recorded");
    if boot_flag_enabled(
        state.boot_info.command_line,
        "ngos.boot.skip_traps_framebuffer_trace",
    ) {
        boot_logln!("ngos/x86_64: traps framebuffer trace skipped by cmdline");
    } else {
        boot_logln!("ngos/x86_64: traps framebuffer header start");
        framebuffer_boot_trace_header();
        boot_logln!("ngos/x86_64: traps framebuffer header complete");
        boot_logln!("ngos/x86_64: traps framebuffer snapshot start");
        framebuffer_boot_trace_snapshot();
        boot_logln!("ngos/x86_64: traps framebuffer snapshot complete");
    }
    if boot_flag_enabled(state.boot_info.command_line, "ngos.boot.skip_int3_probe") {
        boot_logln!("ngos/x86_64: int3 probe skipped by cmdline");
    } else {
        boot_logln!("ngos/x86_64: int3 probe start");
        crate::traps::trigger_breakpoint_probe();
        boot_logln!("ngos/x86_64: breakpoint probe returned");
    }
    let timer_facts = crate::timer::init();
    boot_logln!(
        "ngos/x86_64: timer source={:?} invariant_tsc={} tsc_hz={} boot_tsc={:#x}",
        timer_facts.kind,
        timer_facts.invariant_tsc,
        timer_facts.tsc_hz,
        timer_facts.boot_tsc
    );
    let irq32_trace = boot_flag_enabled(state.boot_info.command_line, "ngos.boot.trace_irq32");
    let irq32_minimal = boot_flag_enabled(state.boot_info.command_line, "ngos.boot.irq32_minimal");
    crate::traps::configure_irq32_diagnostics(if irq32_trace { 4 } else { 0 }, irq32_minimal);
    if irq32_trace {
        boot_logln!("ngos/x86_64: irq32 trace enabled");
    }
    if irq32_minimal {
        boot_logln!("ngos/x86_64: irq32 minimal handler path enabled");
    }
    boot_logln!("ngos/x86_64: external irq bring-up start");
    let irq_facts = crate::traps::bring_up_external_irqs();
    boot_logln!(
        "ngos/x86_64: external irq ready pic_base={} pit_reload={} pit_hz={}",
        irq_facts.pic_base,
        irq_facts.pit_reload_value,
        irq_facts.pit_hz
    );
    if boot_flag_enabled(state.boot_info.command_line, "ngos.boot.mask_irq0") {
        crate::pic::mask_irq_line(0);
        boot_logln!("ngos/x86_64: irq0 masked by cmdline");
    }
    boot_logln!("ngos/x86_64: interrupts enabling");
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
    boot_logln!("ngos/x86_64: interrupts enabled");
    boot_checkpoint(state, crate::boot_locator::BootLocatorStage::Traps, 0x450);
    boot_checkpoint(state, crate::boot_locator::BootLocatorStage::Traps, 0x460);
    boot_checkpoint(state, crate::boot_locator::BootLocatorStage::Traps, 0x470);
    let paging_root = paging_state.root_phys();
    match crate::smp::prepare_bootstrap(
        &state.boot_info,
        frame_allocator,
        paging_root,
        crate::smp::x86_64_secondary_cpu_main as *const () as usize as u64,
    ) {
        Ok(Some(prepared)) => {
            crate::boot_locator::event(
                crate::boot_locator::BootLocatorStage::Smp,
                crate::boot_locator::BootLocatorKind::Contract,
                crate::boot_locator::BootLocatorSeverity::Info,
                0x400,
                crate::boot_locator::BootPayloadLabel::Count,
                prepared.targets.len() as u64,
                crate::boot_locator::BootPayloadLabel::Address,
                prepared.local_apic_address,
            );
            boot_logln!(
                "ngos/x86_64: smp bootstrap ready bsp_apic={} ap_targets={} lapic={:#x} tramp={:#x}/{} mailbox={:#x}/{} ring={:#x}/{}",
                prepared.bootstrap_apic_id,
                prepared.targets.len(),
                prepared.local_apic_address,
                prepared.layout.trampoline_base,
                prepared.layout.trampoline_len,
                prepared.layout.mailbox_base,
                prepared.layout.mailbox_len,
                prepared.layout.job_ring_base,
                prepared.layout.job_ring_len
            );
            match crate::smp::bring_up_secondary_processors(&state.boot_info, &prepared, 5_000_000)
            {
                Ok(report) => {
                    crate::boot_locator::event(
                        crate::boot_locator::BootLocatorStage::Smp,
                        crate::boot_locator::BootLocatorKind::Transition,
                        crate::boot_locator::BootLocatorSeverity::Info,
                        0x410,
                        crate::boot_locator::BootPayloadLabel::Count,
                        report.ap_online as u64,
                        crate::boot_locator::BootPayloadLabel::Count,
                        report.ap_targets as u64,
                    );
                    boot_logln!(
                        "ngos/x86_64: smp startup online={} targets={}",
                        report.ap_online,
                        report.ap_targets
                    );
                }
                Err(error) => {
                    crate::boot_locator::event(
                        crate::boot_locator::BootLocatorStage::Smp,
                        crate::boot_locator::BootLocatorKind::Fault,
                        crate::boot_locator::BootLocatorSeverity::Warn,
                        0x41f,
                        crate::boot_locator::BootPayloadLabel::Status,
                        error as u64,
                        crate::boot_locator::BootPayloadLabel::None,
                        0,
                    );
                    boot_logln!("ngos/x86_64: smp startup dispatch failed: {:?}", error);
                }
            }
        }
        Ok(None) => {
            boot_logln!("ngos/x86_64: smp bootstrap not required");
        }
        Err(error) => {
            boot_logln!("ngos/x86_64: smp bootstrap preparation failed: {:?}", error);
        }
    }
    match crate::virtio_net_boot::bring_up(state, &paging_state, frame_allocator) {
        Ok(true) => {
            boot_logln!("ngos/x86_64: virtio-net bring-up complete");
            crate::virtio_net_boot::wait_for_external_traffic(2000);
        }
        Ok(false) => {
            boot_logln!("ngos/x86_64: virtio-net not present");
        }
        Err(error) => {
            boot_logln!("ngos/x86_64: virtio-net bring-up failed: {}", error);
        }
    }
    match crate::virtio_blk_boot::bring_up(state, &paging_state, frame_allocator) {
        Ok(true) => {
            boot_logln!("ngos/x86_64: virtio-blk bring-up complete");
        }
        Ok(false) => {
            boot_logln!("ngos/x86_64: virtio-blk not present");
        }
        Err(error) => {
            boot_logln!("ngos/x86_64: virtio-blk bring-up failed: {}", error);
        }
    }
    let prepared = crate::user_process::prepare_user_launch(state).unwrap_or_else(|error| {
        fail(BootFailure::UserProcess(error));
    });
    crate::boot_locator::event(
        crate::boot_locator::BootLocatorStage::User,
        crate::boot_locator::BootLocatorKind::Contract,
        crate::boot_locator::BootLocatorSeverity::Info,
        0x500,
        crate::boot_locator::BootPayloadLabel::Rip,
        prepared.entry_point,
        crate::boot_locator::BootPayloadLabel::Count,
        prepared.plan.image_mappings.len() as u64,
    );
    boot_logln!(
        "ngos/x86_64: prepare_user_launch entry={:#x} image_mappings={} stack={:#x} bytes={} reserve={}",
        prepared.entry_point,
        prepared.plan.image_mappings.len(),
        prepared.plan.stack_mapping.vaddr,
        prepared.plan.stack_bytes.len(),
        crate::user_process::USER_STACK_RESERVE_BYTES
    );
    crate::user_runtime_status::set_boot_outcome_policy(prepared.boot_outcome_policy);
    let mut mapper = crate::user_process::mapper_for(&paging_state, frame_allocator, &prepared)
        .unwrap_or_else(|error| fail(BootFailure::UserMode(error)));
    crate::user_bridge::install_first_user_process(&mut mapper, &prepared.plan)
        .unwrap_or_else(|error| fail(BootFailure::UserMode(error)));
    drop(mapper);
    crate::boot_locator::event(
        crate::boot_locator::BootLocatorStage::User,
        crate::boot_locator::BootLocatorKind::Transition,
        crate::boot_locator::BootLocatorSeverity::Info,
        0x510,
        crate::boot_locator::BootPayloadLabel::Address,
        prepared.plan.stack_mapping.vaddr,
        crate::boot_locator::BootPayloadLabel::Length,
        prepared.plan.stack_mapping.len,
    );
    boot_logln!("ngos/x86_64: user mappings installed");
    crate::diagnostics::record_boot_stage(
        crate::diagnostics::BootTraceStage::UserLaunchReady,
        crate::timer::boot_uptime_micros(),
        prepared.entry_point,
    );
    framebuffer_boot_trace_header();
    framebuffer_boot_trace_snapshot();
    boot_logln!(
        "ngos/x86_64: entering user mode module=\"{}\"",
        prepared.module_name
    );
    crate::user_syscall::install_boot_process_exec_runtime(paging_state, unsafe {
        ptr::read(frame_allocator)
    });
    unsafe {
        crate::diagnostics::record_boot_stage(
            crate::diagnostics::BootTraceStage::EnterUserMode,
            crate::timer::boot_uptime_micros(),
            prepared.plan.registers.rip as u64,
        );
        crate::boot_locator::event(
            crate::boot_locator::BootLocatorStage::User,
            crate::boot_locator::BootLocatorKind::Transition,
            crate::boot_locator::BootLocatorSeverity::Info,
            0x520,
            crate::boot_locator::BootPayloadLabel::Rip,
            prepared.plan.registers.rip as u64,
            crate::boot_locator::BootPayloadLabel::Address,
            prepared.plan.registers.rsp as u64,
        );
        platform_x86_64::user_mode::enter_user_mode(&prepared.plan.registers);
    }
}

#[cfg(target_os = "none")]
fn zero_bss() {
    let start = ptr::addr_of!(__bss_start) as usize;
    let end = ptr::addr_of!(__bss_end) as usize;
    let len = end.saturating_sub(start);
    if len != 0 {
        unsafe {
            ptr::write_bytes(start as *mut u8, 0, len);
        }
    }
}

#[cfg(target_os = "none")]
fn fail(failure: BootFailure) -> ! {
    serial::debug_marker(b'F');
    serial::init();
    let locator = crate::boot_locator::snapshot();
    crate::reboot_trace::record_locator(locator);
    crate::framebuffer::alert_banner("BOOT FAILURE");
    framebuffer_boot_trace_snapshot();
    let (family, detail) = boot_failure_summary(failure);
    let (
        locator_stage,
        locator_checkpoint,
        locator_name,
        payload0_label,
        payload0,
        payload1_label,
        payload1,
    ) = boot_locator_failure_summary(locator);
    crate::framebuffer::print(format_args!(
        "summary: family={} detail={}\n",
        family, detail
    ));
    crate::framebuffer::print(format_args!(
        "locator: stage={:?} checkpoint={:#x} name={}\n",
        locator_stage, locator_checkpoint, locator_name
    ));
    crate::framebuffer::print(format_args!("cause: {:?}\n", failure));
    if let Some(uptime_us) = crate::timer::boot_uptime_micros() {
        boot_logln!(
            "ngos/x86_64: boot failure after {} us family={} detail={} cause={:?}",
            uptime_us,
            family,
            detail,
            failure
        );
    } else {
        boot_logln!(
            "ngos/x86_64: boot failure family={} detail={} cause={:?}",
            family,
            detail,
            failure
        );
    }
    boot_logln!(
        "ngos/x86_64: boot locator stage={:?} checkpoint={:#x} name={} payload0={:?}:{:#x} payload1={:?}:{:#x}",
        locator_stage,
        locator_checkpoint,
        locator_name,
        payload0_label,
        payload0,
        payload1_label,
        payload1
    );
    halt_loop()
}

#[cfg(target_os = "none")]
fn halt_loop() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
        spin_loop();
    }
}

#[panic_handler]
#[cfg(target_os = "none")]
fn panic(info: &PanicInfo<'_>) -> ! {
    serial::debug_marker(b'P');
    serial::init();
    crate::reboot_trace::record_locator(crate::boot_locator::snapshot());
    crate::framebuffer::alert_banner("KERNEL PANIC");
    framebuffer_boot_trace_snapshot();
    crate::framebuffer::print(format_args!("cause: {}\n", info));
    if let Some(uptime_us) = crate::timer::boot_uptime_micros() {
        boot_logln!("ngos/x86_64: panic after {} us: {}", uptime_us, info);
    } else {
        boot_logln!("ngos/x86_64: panic: {}", info);
    }
    halt_loop()
}

#[cfg(target_os = "none")]
#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    serial::debug_marker(b'A');
    serial::init();
    crate::reboot_trace::record_locator(crate::boot_locator::snapshot());
    crate::framebuffer::alert_banner("ALLOC ERROR");
    framebuffer_boot_trace_snapshot();
    crate::framebuffer::print(format_args!(
        "cause: size={} align={}\n",
        layout.size(),
        layout.align()
    ));
    if let Some(uptime_us) = crate::timer::boot_uptime_micros() {
        boot_logln!(
            "ngos/x86_64: alloc error after {} us size={} align={} used_bytes={} capacity_bytes={}",
            uptime_us,
            layout.size(),
            layout.align(),
            crate::heap::allocated_bytes(),
            crate::heap::capacity_bytes()
        );
    } else {
        boot_logln!(
            "ngos/x86_64: alloc error size={} align={} used_bytes={} capacity_bytes={}",
            layout.size(),
            layout.align(),
            crate::heap::allocated_bytes(),
            crate::heap::capacity_bytes()
        );
    }
    halt_loop()
}

#[cfg(not(target_os = "none"))]
fn main() {}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    let mut index = 0;
    while index < len {
        unsafe {
            *dst.add(index) = *src.add(index);
        }
        index += 1;
    }
    dst
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memset(dst: *mut u8, value: i32, len: usize) -> *mut u8 {
    let byte = value as u8;
    let mut index = 0;
    while index < len {
        unsafe {
            *dst.add(index) = byte;
        }
        index += 1;
    }
    dst
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    if core::ptr::eq(dst.cast_const(), src) || len == 0 {
        return dst;
    }
    if (dst as usize) < (src as usize) {
        let mut index = 0usize;
        while index < len {
            unsafe {
                *dst.add(index) = *src.add(index);
            }
            index += 1;
        }
    } else {
        let mut index = len;
        while index != 0 {
            index -= 1;
            unsafe {
                *dst.add(index) = *src.add(index);
            }
        }
    }
    dst
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(lhs: *const u8, rhs: *const u8, len: usize) -> i32 {
    let mut index = 0;
    while index < len {
        let left = unsafe { *lhs.add(index) };
        let right = unsafe { *rhs.add(index) };
        if left != right {
            return left as i32 - right as i32;
        }
        index += 1;
    }
    0
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn strlen(ptr: *const u8) -> usize {
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_x86_64::BootInfoValidationError;

    #[test]
    fn boot_halt_selector_matches_hex_checkpoint() {
        assert!(boot_halt_selector_matches(
            "traps:0x450",
            crate::boot_locator::BootLocatorStage::Traps,
            0x450
        ));
        assert!(!boot_halt_selector_matches(
            "traps:0x451",
            crate::boot_locator::BootLocatorStage::Traps,
            0x450
        ));
    }

    #[test]
    fn boot_halt_selector_matches_checkpoint_name_aliases() {
        assert!(boot_halt_selector_matches(
            "paging/handoff-ready",
            crate::boot_locator::BootLocatorStage::Paging,
            0x340
        ));
        assert!(boot_halt_selector_matches(
            "post-paging",
            crate::boot_locator::BootLocatorStage::Paging,
            0x340
        ));
    }

    #[test]
    fn parse_checkpoint_value_accepts_hex_and_decimal() {
        assert_eq!(parse_checkpoint_value("0x470"), Some(0x470));
        assert_eq!(parse_checkpoint_value("1136"), Some(1136));
        assert_eq!(parse_checkpoint_value("bad"), None);
    }

    #[test]
    fn boot_flag_enabled_matches_exact_token() {
        assert!(boot_flag_enabled(
            Some("console=ttyS0 ngos.boot.skip_int3_probe ngos.boot.halt=traps:0x450"),
            "ngos.boot.skip_int3_probe"
        ));
        assert!(!boot_flag_enabled(
            Some("console=ttyS0 ngos.boot.skip_int3_probe_extra"),
            "ngos.boot.skip_int3_probe"
        ));
        assert!(!boot_flag_enabled(None, "ngos.boot.skip_int3_probe"));
    }

    #[test]
    fn boot_failure_summary_uses_stable_tokens_for_limine_contract_refusals() {
        let summary = boot_failure_summary(BootFailure::Limine(
            crate::limine::LimineBootError::InvalidBootInfo(
                BootInfoValidationError::MemoryRegionsOverlap,
            ),
        ));
        assert_eq!(summary, ("limine", "overlapping-memory-regions"));

        let summary = boot_failure_summary(BootFailure::Limine(
            crate::limine::LimineBootError::MissingResponse("memory map"),
        ));
        assert_eq!(summary, ("limine", "missing-memory-map"));
    }

    #[test]
    fn boot_failure_summary_covers_remaining_limine_refusal_families() {
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::MissingBaseRevision,
            )),
            ("limine", "missing-base-revision")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::UnsupportedBaseRevision { loaded: Some(3) },
            )),
            ("limine", "unsupported-base-revision")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::MissingResponse("higher-half direct map"),
            )),
            ("limine", "missing-hhdm")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::MissingResponse("executable address"),
            )),
            ("limine", "missing-executable-address")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::TooManyMemoryRegions {
                    count: 257,
                    capacity: 256,
                },
            )),
            ("limine", "too-many-memory-regions")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::InvalidModulePathUtf8 { index: 0 },
            )),
            ("limine", "invalid-module-path-utf8")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::KernelRangeMustBeKernelImage,
                ),
            )),
            ("limine", "invalid-kernel-range-kind")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::KernelRangeMustBePageAligned,
                ),
            )),
            ("limine", "invalid-kernel-range-alignment")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::KernelRangeMustBeNonEmpty,
                ),
            )),
            ("limine", "empty-kernel-range")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::MemoryRegionMustBePageAligned,
                ),
            )),
            ("limine", "invalid-memory-region-alignment")
        );
        assert_eq!(
            boot_failure_summary(BootFailure::Limine(
                crate::limine::LimineBootError::InvalidBootInfo(
                    BootInfoValidationError::MemoryRegionMustBeNonEmpty,
                ),
            )),
            ("limine", "empty-memory-region")
        );
    }

    #[test]
    fn boot_locator_failure_summary_preserves_refusal_checkpoint_and_payloads() {
        let locator = crate::boot_locator::BootLocatorRecord {
            sequence: 7,
            stage: crate::boot_locator::BootLocatorStage::Limine,
            kind: crate::boot_locator::BootLocatorKind::Fault,
            severity: crate::boot_locator::BootLocatorSeverity::Error,
            checkpoint: 0x2ff,
            payload0_label: crate::boot_locator::BootPayloadLabel::Status,
            payload0: 0x21,
            payload1_label: crate::boot_locator::BootPayloadLabel::Value,
            payload1: 0,
        };

        let summary = boot_locator_failure_summary(locator);
        assert_eq!(summary.0, crate::boot_locator::BootLocatorStage::Limine);
        assert_eq!(summary.1, 0x2ff);
        assert_eq!(summary.2, "limine/contract-refusal");
        assert_eq!(summary.3, crate::boot_locator::BootPayloadLabel::Status);
        assert_eq!(summary.4, 0x21);
    }
}
