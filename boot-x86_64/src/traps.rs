use core::arch::asm;
use core::arch::global_asm;
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use ngos_user_abi::{USER_DEBUG_MARKER_EXIT, USER_DEBUG_MARKER_MAIN, USER_DEBUG_MARKER_START};
use platform_x86_64::{
    EXCEPTION_VECTOR_COUNT, ExceptionFrame, ExceptionVector, IdtEntry, IdtGateOptions, IdtPointer,
    InterruptDescriptorTable,
};

use crate::fault_diag;
use crate::serial;
use crate::timer;
use crate::user_runtime_status;
use crate::{keyboard, pic, pit};

global_asm!(include_str!("traps.S"));

static mut EARLY_IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();
static IRQ32_TRACE_REMAINING: AtomicU8 = AtomicU8::new(0);
static IRQ32_MINIMAL_PATH: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
static mut __ngos_x86_64_syscall_stack_top: u64 = 0;
#[unsafe(no_mangle)]
static mut __ngos_x86_64_saved_user_rsp: u64 = 0;
#[unsafe(no_mangle)]
static mut __ngos_x86_64_saved_user_rip: u64 = 0;
#[unsafe(no_mangle)]
static mut __ngos_x86_64_saved_user_rflags: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapBringupError {
    MissingCodeSelector,
    MissingKernelStack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalIrqBringupFacts {
    pub pic_base: u8,
    pub pit_reload_value: u16,
    pub pit_hz: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
enum ExceptionDisposition {
    Return = 0,
    Halt = 1,
}

unsafe extern "C" {
    fn __ngos_x86_64_isr0();
    fn __ngos_x86_64_isr1();
    fn __ngos_x86_64_isr2();
    fn __ngos_x86_64_isr3();
    fn __ngos_x86_64_isr4();
    fn __ngos_x86_64_isr5();
    fn __ngos_x86_64_isr6();
    fn __ngos_x86_64_isr7();
    fn __ngos_x86_64_isr8();
    fn __ngos_x86_64_isr9();
    fn __ngos_x86_64_isr10();
    fn __ngos_x86_64_isr11();
    fn __ngos_x86_64_isr12();
    fn __ngos_x86_64_isr13();
    fn __ngos_x86_64_isr14();
    fn __ngos_x86_64_isr15();
    fn __ngos_x86_64_isr16();
    fn __ngos_x86_64_isr17();
    fn __ngos_x86_64_isr18();
    fn __ngos_x86_64_isr19();
    fn __ngos_x86_64_isr20();
    fn __ngos_x86_64_isr21();
    fn __ngos_x86_64_isr22();
    fn __ngos_x86_64_isr23();
    fn __ngos_x86_64_isr24();
    fn __ngos_x86_64_isr25();
    fn __ngos_x86_64_isr26();
    fn __ngos_x86_64_isr27();
    fn __ngos_x86_64_isr28();
    fn __ngos_x86_64_isr29();
    fn __ngos_x86_64_isr30();
    fn __ngos_x86_64_isr31();
    fn __ngos_x86_64_isr32();
    fn __ngos_x86_64_isr33();
    fn __ngos_x86_64_isr34();
    fn __ngos_x86_64_isr35();
    fn __ngos_x86_64_isr36();
    fn __ngos_x86_64_isr37();
    fn __ngos_x86_64_isr38();
    fn __ngos_x86_64_isr39();
    fn __ngos_x86_64_isr40();
    fn __ngos_x86_64_isr41();
    fn __ngos_x86_64_isr42();
    fn __ngos_x86_64_isr43();
    fn __ngos_x86_64_isr44();
    fn __ngos_x86_64_isr45();
    fn __ngos_x86_64_isr46();
    fn __ngos_x86_64_isr47();
    fn __ngos_x86_64_syscall_entry();
}

pub fn bring_up(kernel_stack_top: u64) -> Result<(), TrapBringupError> {
    serial::debug_marker(b'I');
    let code_selector = read_cs();
    if code_selector == 0 {
        return Err(TrapBringupError::MissingCodeSelector);
    }
    if kernel_stack_top == 0 {
        return Err(TrapBringupError::MissingKernelStack);
    }

    let stubs: [unsafe extern "C" fn(); EXCEPTION_VECTOR_COUNT] = [
        __ngos_x86_64_isr0,
        __ngos_x86_64_isr1,
        __ngos_x86_64_isr2,
        __ngos_x86_64_isr3,
        __ngos_x86_64_isr4,
        __ngos_x86_64_isr5,
        __ngos_x86_64_isr6,
        __ngos_x86_64_isr7,
        __ngos_x86_64_isr8,
        __ngos_x86_64_isr9,
        __ngos_x86_64_isr10,
        __ngos_x86_64_isr11,
        __ngos_x86_64_isr12,
        __ngos_x86_64_isr13,
        __ngos_x86_64_isr14,
        __ngos_x86_64_isr15,
        __ngos_x86_64_isr16,
        __ngos_x86_64_isr17,
        __ngos_x86_64_isr18,
        __ngos_x86_64_isr19,
        __ngos_x86_64_isr20,
        __ngos_x86_64_isr21,
        __ngos_x86_64_isr22,
        __ngos_x86_64_isr23,
        __ngos_x86_64_isr24,
        __ngos_x86_64_isr25,
        __ngos_x86_64_isr26,
        __ngos_x86_64_isr27,
        __ngos_x86_64_isr28,
        __ngos_x86_64_isr29,
        __ngos_x86_64_isr30,
        __ngos_x86_64_isr31,
    ];
    let irq_stubs: [unsafe extern "C" fn(); 16] = [
        __ngos_x86_64_isr32,
        __ngos_x86_64_isr33,
        __ngos_x86_64_isr34,
        __ngos_x86_64_isr35,
        __ngos_x86_64_isr36,
        __ngos_x86_64_isr37,
        __ngos_x86_64_isr38,
        __ngos_x86_64_isr39,
        __ngos_x86_64_isr40,
        __ngos_x86_64_isr41,
        __ngos_x86_64_isr42,
        __ngos_x86_64_isr43,
        __ngos_x86_64_isr44,
        __ngos_x86_64_isr45,
        __ngos_x86_64_isr46,
        __ngos_x86_64_isr47,
    ];

    unsafe {
        let idt_ptr = ptr::addr_of_mut!(EARLY_IDT);
        (*idt_ptr).entries = [IdtEntry::missing(); platform_x86_64::IDT_ENTRY_COUNT];
        for (vector, stub) in stubs.iter().enumerate() {
            let mut options = IdtGateOptions::interrupt();
            if vector == ExceptionVector::Breakpoint as usize {
                options = options.with_privilege_level(3);
            }
            (*idt_ptr).entries[vector] =
                IdtEntry::new(*stub as usize as u64, code_selector, options);
        }
        for (offset, stub) in irq_stubs.iter().enumerate() {
            let vector = usize::from(pic::IRQ_BASE_PRIMARY) + offset;
            (*idt_ptr).entries[vector] = IdtEntry::new(
                *stub as usize as u64,
                code_selector,
                IdtGateOptions::interrupt(),
            );
        }

        let pointer = IdtPointer::new(idt_ptr as u64, size_of::<InterruptDescriptorTable>());
        load_idt(&pointer);
        let idt_base = pointer.base;
        let idt_limit = pointer.limit;
        serial::debug_marker(b'J');
        serial::print(format_args!(
            "ngos/x86_64: idt installed base={:#x} limit={:#x} cs={:#x} exceptions={}\n",
            idt_base, idt_limit, code_selector, EXCEPTION_VECTOR_COUNT
        ));
    }
    bring_up_syscall(kernel_stack_top);
    Ok(())
}

pub fn bring_up_external_irqs() -> ExternalIrqBringupFacts {
    keyboard::init();
    pic::remap_and_unmask_timer_keyboard();
    let pit_config = pit::program_periodic(100);
    timer::set_pit_tick_rate(pit_config.actual_hz);
    let facts = ExternalIrqBringupFacts {
        pic_base: pic::IRQ_BASE_PRIMARY,
        pit_reload_value: pit_config.reload_value,
        pit_hz: pit_config.actual_hz,
    };
    serial::print(format_args!(
        "ngos/x86_64: irq timer+keyboard online pic_base={} pit_reload={} pit_hz={}\n",
        facts.pic_base, facts.pit_reload_value, facts.pit_hz
    ));
    facts
}

pub fn configure_irq32_diagnostics(trace_limit: u8, minimal_path: bool) {
    IRQ32_TRACE_REMAINING.store(trace_limit, Ordering::Relaxed);
    IRQ32_MINIMAL_PATH.store(minimal_path, Ordering::Relaxed);
}

pub fn trigger_breakpoint_probe() {
    serial::debug_marker(b'K');
    unsafe {
        asm!("int3", options(nomem, nostack));
    }
}

#[unsafe(no_mangle)]
extern "C" fn x86_64_exception_dispatch(frame: *const ExceptionFrame) -> u64 {
    let frame = unsafe { &*frame };
    let user_mode = (frame.cs & 0x3) == 0x3;
    let uptime_us = timer::boot_uptime_micros();

    if frame.vector >= u64::from(pic::IRQ_BASE_PRIMARY)
        && frame.vector < u64::from(pic::IRQ_BASE_PRIMARY + 16)
    {
        let vector = frame.vector as u8;
        let trace_irq32 = vector == pic::IRQ_TIMER && irq32_trace_enabled();
        if trace_irq32 {
            serial::print(format_args!(
                "ngos/x86_64: irq32 enter rip={:#x} cs={:#x} err={:#x}\n",
                frame.rip, frame.cs, frame.error_code
            ));
        }
        if vector == pic::IRQ_TIMER {
            if !IRQ32_MINIMAL_PATH.load(Ordering::Relaxed) {
                let ticks = timer::record_pit_tick();
                let tick_hz = timer::pit_tick_hz().max(1);
                if ticks == 1 || ticks % tick_hz == 0 {
                    let uptime = timer::boot_uptime_micros().unwrap_or(0);
                    serial::print(format_args!(
                        "ngos/x86_64: timer tick irq={} ticks={} uptime_us={}\n",
                        vector, ticks, uptime
                    ));
                }
            } else if trace_irq32 {
                serial::print(format_args!(
                    "ngos/x86_64: irq32 minimal-path body skipped\n"
                ));
            }
        } else if vector == pic::IRQ_KEYBOARD {
            keyboard::handle_irq();
        } else {
            let _ = crate::irq_registry::dispatch_irq(vector - pic::IRQ_BASE_PRIMARY);
        }
        if trace_irq32 {
            serial::print(format_args!("ngos/x86_64: irq32 before eoi\n"));
        }
        pic::end_of_interrupt(vector);
        if trace_irq32 {
            serial::print(format_args!("ngos/x86_64: irq32 after eoi\n"));
            serial::print(format_args!("ngos/x86_64: irq32 about to return\n"));
        }
        return ExceptionDisposition::Return as u64;
    }

    serial::debug_marker(b'L');
    serial::print(format_args!(
        "ngos/x86_64: exception dispatch vector={} cs={:#x} rip={:#x} err={:#x}\n",
        frame.vector, frame.cs, frame.rip, frame.error_code
    ));

    match frame.vector_kind() {
        Some(ExceptionVector::Breakpoint) => {
            if user_mode {
                match frame.rax {
                    USER_DEBUG_MARKER_START => {
                        user_runtime_status::mark_started();
                        crate::diagnostics::record_user_marker(
                            frame.rax, frame.rdi, frame.rip, uptime_us,
                        );
                        log_exception_prefix(uptime_us);
                        serial::print(format_args!(
                            "ngos/x86_64: user _start reached rip={:#x} argc={}\n",
                            frame.rip, frame.rdi
                        ));
                        return ExceptionDisposition::Return as u64;
                    }
                    USER_DEBUG_MARKER_MAIN => {
                        user_runtime_status::mark_main_reached();
                        crate::diagnostics::record_user_marker(
                            frame.rax, frame.rdi, frame.rip, uptime_us,
                        );
                        log_exception_prefix(uptime_us);
                        serial::print(format_args!(
                            "ngos/x86_64: user main reached rip={:#x}\n",
                            frame.rip
                        ));
                        return ExceptionDisposition::Return as u64;
                    }
                    USER_DEBUG_MARKER_EXIT => {
                        user_runtime_status::mark_exit(frame.rdi as i32);
                        crate::diagnostics::record_user_marker(
                            frame.rax, frame.rdi, frame.rip, uptime_us,
                        );
                        log_exception_prefix(uptime_us);
                        serial::print(format_args!(
                            "ngos/x86_64: user exit requested rip={:#x} code={}\n",
                            frame.rip, frame.rdi
                        ));
                        user_runtime_status::emit_final_report_if_terminal();
                        let _ = user_runtime_status::apply_configured_boot_outcome_policy();
                        return ExceptionDisposition::Return as u64;
                    }
                    _ => {
                        crate::diagnostics::record_user_marker(
                            frame.rax, frame.rdi, frame.rip, uptime_us,
                        );
                        log_exception_prefix(uptime_us);
                        serial::print(format_args!(
                            "ngos/x86_64: user marker rax={:#x} rdi={:#x} rip={:#x}\n",
                            frame.rax, frame.rdi, frame.rip
                        ));
                        return ExceptionDisposition::Return as u64;
                    }
                }
            }
            log_exception_prefix(uptime_us);
            serial::print(format_args!(
                "ngos/x86_64: exception breakpoint rip={:#x} cs={:#x} rflags={:#x}\n",
                frame.rip, frame.cs, frame.rflags
            ));
            ExceptionDisposition::Return as u64
        }
        Some(ExceptionVector::InvalidOpcode) => {
            if user_mode {
                user_runtime_status::mark_fault();
                fault_diag::report_exception(frame, uptime_us, None);
                user_runtime_status::emit_final_report_if_terminal();
                let _ = user_runtime_status::apply_configured_boot_outcome_policy();
                return ExceptionDisposition::Halt as u64;
            }
            fault_diag::report_exception(frame, uptime_us, None);
            ExceptionDisposition::Halt as u64
        }
        Some(ExceptionVector::GeneralProtectionFault) => {
            if user_mode {
                user_runtime_status::mark_fault();
                fault_diag::report_exception(frame, uptime_us, None);
                user_runtime_status::emit_final_report_if_terminal();
                let _ = user_runtime_status::apply_configured_boot_outcome_policy();
                return ExceptionDisposition::Halt as u64;
            }
            fault_diag::report_exception(frame, uptime_us, None);
            ExceptionDisposition::Halt as u64
        }
        Some(ExceptionVector::PageFault) => {
            let cr2 = read_cr2();
            if user_mode {
                user_runtime_status::mark_fault();
                fault_diag::report_exception(frame, uptime_us, Some(cr2));
                user_runtime_status::emit_final_report_if_terminal();
                let _ = user_runtime_status::apply_configured_boot_outcome_policy();
                return ExceptionDisposition::Halt as u64;
            }
            fault_diag::report_exception(frame, uptime_us, Some(cr2));
            ExceptionDisposition::Halt as u64
        }
        Some(ExceptionVector::DoubleFault) => {
            if user_mode {
                user_runtime_status::mark_fault();
                fault_diag::report_exception(frame, uptime_us, None);
                user_runtime_status::emit_final_report_if_terminal();
                let _ = user_runtime_status::apply_configured_boot_outcome_policy();
                return ExceptionDisposition::Halt as u64;
            }
            fault_diag::report_exception(frame, uptime_us, None);
            ExceptionDisposition::Halt as u64
        }
        Some(vector) => {
            let _ = vector;
            fault_diag::report_exception(frame, uptime_us, None);
            ExceptionDisposition::Halt as u64
        }
        None => {
            fault_diag::report_exception(frame, uptime_us, None);
            ExceptionDisposition::Halt as u64
        }
    }
}

fn irq32_trace_enabled() -> bool {
    let remaining = IRQ32_TRACE_REMAINING.load(Ordering::Relaxed);
    if remaining == 0 {
        false
    } else {
        IRQ32_TRACE_REMAINING.fetch_sub(1, Ordering::Relaxed);
        true
    }
}

fn log_exception_prefix(uptime_us: Option<u64>) {
    if let Some(uptime_us) = uptime_us {
        serial::print(format_args!("ngos/x86_64: uptime_us={} ", uptime_us));
    }
}

unsafe fn load_idt(pointer: *const IdtPointer) {
    unsafe {
        asm!(
            "lidt [{}]",
            in(reg) pointer,
            options(readonly, nostack, preserves_flags)
        );
    }
}

fn read_cs() -> u16 {
    let value: u16;
    unsafe {
        asm!("mov ax, cs", out("ax") value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn read_cr2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_runtime_status;

    fn user_breakpoint_frame(marker: u64) -> ExceptionFrame {
        ExceptionFrame {
            vector: ExceptionVector::Breakpoint as u64,
            error_code: 0,
            rip: 0x401000,
            cs: 0x33,
            rflags: 0x202,
            rsp: 0x7fff_0000,
            ss: 0x2b,
            rax: marker,
            rdi: 3,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
        }
    }

    #[test]
    fn exception_dispatch_tracks_user_debug_markers() {
        user_runtime_status::reset();

        let start = user_breakpoint_frame(USER_DEBUG_MARKER_START);
        assert_eq!(
            x86_64_exception_dispatch(&start),
            ExceptionDisposition::Return as u64
        );
        assert!(user_runtime_status::snapshot().started);

        let main = user_breakpoint_frame(USER_DEBUG_MARKER_MAIN);
        assert_eq!(
            x86_64_exception_dispatch(&main),
            ExceptionDisposition::Return as u64
        );
        assert!(user_runtime_status::snapshot().main_reached);

        let exit = user_breakpoint_frame(USER_DEBUG_MARKER_EXIT);
        assert_eq!(
            x86_64_exception_dispatch(&exit),
            ExceptionDisposition::Return as u64
        );
        let snapshot = user_runtime_status::snapshot();
        assert!(snapshot.exited);
        assert_eq!(snapshot.exit_code, 3);
    }
}

fn bring_up_syscall(kernel_stack_top: u64) {
    const IA32_EFER: u32 = 0xC000_0080;
    const IA32_STAR: u32 = 0xC000_0081;
    const IA32_LSTAR: u32 = 0xC000_0082;
    const IA32_FMASK: u32 = 0xC000_0084;
    const EFER_SCE: u64 = 1;
    const KERNEL_CS: u64 = 0x08;
    const USER_SYSRET_CS_BASE: u64 = 0x23;
    const RFLAGS_TF: u64 = 1 << 8;
    const RFLAGS_IF: u64 = 1 << 9;
    const RFLAGS_DF: u64 = 1 << 10;

    unsafe {
        __ngos_x86_64_syscall_stack_top = kernel_stack_top;
    }

    let star = (USER_SYSRET_CS_BASE << 48) | (KERNEL_CS << 32);
    let lstar = __ngos_x86_64_syscall_entry as *const () as usize as u64;
    let fmask = RFLAGS_TF | RFLAGS_IF | RFLAGS_DF;
    let efer = read_msr(IA32_EFER) | EFER_SCE;

    write_msr(IA32_STAR, star);
    write_msr(IA32_LSTAR, lstar);
    write_msr(IA32_FMASK, fmask);
    write_msr(IA32_EFER, efer);

    serial::debug_marker(b'S');
    serial::print(format_args!(
        "ngos/x86_64: syscall enabled star={:#x} lstar={:#x} fmask={:#x} stack={:#x}\n",
        star, lstar, fmask, kernel_stack_top
    ));
}

fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

fn write_msr(msr: u32, value: u64) {
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") value as u32,
            in("edx") (value >> 32) as u32,
            options(nomem, nostack, preserves_flags)
        );
    }
}
