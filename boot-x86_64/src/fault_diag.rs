use platform_x86_64::{ExceptionFrame, ExceptionVector};

#[cfg(not(test))]
use crate::boot_locator::BootLocatorRecord;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootLocatorRecord {
    sequence: u64,
    stage: u16,
    checkpoint: u64,
    payload0: u64,
    payload1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFaultError {
    pub present: bool,
    pub write: bool,
    pub user: bool,
    pub reserved_write: bool,
    pub instruction_fetch: bool,
    pub protection_key: bool,
    pub shadow_stack: bool,
    pub software_guard: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorTable {
    Gdt,
    Idt,
    Ldt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectorError {
    pub external: bool,
    pub table: SelectorTable,
    pub index: u16,
}

pub fn decode_page_fault_error(error_code: u64) -> PageFaultError {
    PageFaultError {
        present: (error_code & (1 << 0)) != 0,
        write: (error_code & (1 << 1)) != 0,
        user: (error_code & (1 << 2)) != 0,
        reserved_write: (error_code & (1 << 3)) != 0,
        instruction_fetch: (error_code & (1 << 4)) != 0,
        protection_key: (error_code & (1 << 5)) != 0,
        shadow_stack: (error_code & (1 << 6)) != 0,
        software_guard: (error_code & (1 << 15)) != 0,
    }
}

pub fn decode_selector_error(error_code: u64) -> SelectorError {
    let table = match (error_code >> 1) & 0x3 {
        1 => SelectorTable::Idt,
        2 | 3 => SelectorTable::Ldt,
        _ => SelectorTable::Gdt,
    };
    SelectorError {
        external: (error_code & 1) != 0,
        table,
        index: ((error_code >> 3) & 0x1fff) as u16,
    }
}

pub fn report_exception(frame: &ExceptionFrame, uptime_us: Option<u64>, cr2: Option<u64>) {
    record_fault_in_diagnostics(frame, uptime_us, cr2);
    activate_fault_console(frame);
    print_header(frame, uptime_us, cr2);
    print_registers(frame);
    print_decoded_error(frame, cr2);
}

#[cfg(not(test))]
fn record_fault_in_diagnostics(frame: &ExceptionFrame, uptime_us: Option<u64>, cr2: Option<u64>) {
    crate::diagnostics::record_fault(frame, uptime_us, cr2);
}

#[cfg(test)]
fn record_fault_in_diagnostics(
    _frame: &ExceptionFrame,
    _uptime_us: Option<u64>,
    _cr2: Option<u64>,
) {
}

fn activate_fault_console(frame: &ExceptionFrame) {
    #[cfg(test)]
    {
        let _ = frame;
        return;
    }

    #[cfg(not(test))]
    let title = match frame.vector_kind() {
        Some(vector) => match vector {
            ExceptionVector::PageFault => "PAGE FAULT",
            ExceptionVector::GeneralProtectionFault => "GENERAL PROTECTION FAULT",
            ExceptionVector::DoubleFault => "DOUBLE FAULT",
            ExceptionVector::InvalidOpcode => "INVALID OPCODE",
            ExceptionVector::StackSegmentFault => "STACK SEGMENT FAULT",
            ExceptionVector::SegmentNotPresent => "SEGMENT NOT PRESENT",
            ExceptionVector::InvalidTss => "INVALID TSS",
            ExceptionVector::AlignmentCheck => "ALIGNMENT CHECK",
            ExceptionVector::ControlProtection => "CONTROL PROTECTION FAULT",
            _ => "CPU EXCEPTION",
        },
        None => "CPU EXCEPTION",
    };

    #[cfg(not(test))]
    crate::framebuffer::alert_banner(title);
}

fn print_header(frame: &ExceptionFrame, uptime_us: Option<u64>, cr2: Option<u64>) {
    let privilege = if (frame.cs & 0x3) == 0x3 {
        "user"
    } else {
        "kernel"
    };
    let uptime = uptime_us.unwrap_or(0);
    if let Some(vector) = frame.vector_kind() {
        if let Some(cr2) = cr2 {
            crate::serial::print(format_args!(
                "ngos/x86_64: fault kind={} vector={}({}) uptime_us={} rip={:#x} cs={:#x} rflags={:#x} error={:#x} cr2={:#x}\n",
                privilege,
                frame.vector,
                vector.name(),
                uptime,
                frame.rip,
                frame.cs,
                frame.rflags,
                frame.error_code,
                cr2
            ));
        } else {
            crate::serial::print(format_args!(
                "ngos/x86_64: fault kind={} vector={}({}) uptime_us={} rip={:#x} cs={:#x} rflags={:#x} error={:#x}\n",
                privilege,
                frame.vector,
                vector.name(),
                uptime,
                frame.rip,
                frame.cs,
                frame.rflags,
                frame.error_code
            ));
        }
    } else {
        crate::serial::print(format_args!(
            "ngos/x86_64: fault kind={} vector={} uptime_us={} rip={:#x} cs={:#x} rflags={:#x} error={:#x}\n",
            privilege, frame.vector, uptime, frame.rip, frame.cs, frame.rflags, frame.error_code
        ));
    }
    let locator = boot_locator_snapshot();
    crate::serial::print(format_args!(
        "ngos/x86_64: fault locator seq={} stage={:?} checkpoint={:#x} payload0={:#x} payload1={:#x}\n",
        locator.sequence, locator.stage, locator.checkpoint, locator.payload0, locator.payload1
    ));
}

#[cfg(not(test))]
fn boot_locator_snapshot() -> BootLocatorRecord {
    crate::diagnostics::boot_locator_snapshot()
}

#[cfg(test)]
fn boot_locator_snapshot() -> BootLocatorRecord {
    BootLocatorRecord {
        sequence: 0,
        stage: 0,
        checkpoint: 0,
        payload0: 0,
        payload1: 0,
    }
}

fn print_registers(frame: &ExceptionFrame) {
    crate::serial::print(format_args!(
        "ngos/x86_64: regs rax={:#x} rbx={:#x} rcx={:#x} rdx={:#x}\n",
        frame.rax, frame.rbx, frame.rcx, frame.rdx
    ));
    crate::serial::print(format_args!(
        "ngos/x86_64: regs rsi={:#x} rdi={:#x} rbp={:#x} r8={:#x}\n",
        frame.rsi, frame.rdi, frame.rbp, frame.r8
    ));
    crate::serial::print(format_args!(
        "ngos/x86_64: regs r9={:#x} r10={:#x} r11={:#x} r12={:#x}\n",
        frame.r9, frame.r10, frame.r11, frame.r12
    ));
    crate::serial::print(format_args!(
        "ngos/x86_64: regs r13={:#x} r14={:#x} r15={:#x}\n",
        frame.r13, frame.r14, frame.r15
    ));
}

fn print_decoded_error(frame: &ExceptionFrame, cr2: Option<u64>) {
    match frame.vector_kind() {
        Some(ExceptionVector::PageFault) => {
            let decoded = decode_page_fault_error(frame.error_code);
            crate::serial::print(format_args!(
                "ngos/x86_64: page_fault access={} privilege={} present={} reserved={} ifetch={} pkey={} shadow_stack={} sgx={}\n",
                if decoded.write { "write" } else { "read" },
                if decoded.user { "user" } else { "kernel" },
                decoded.present,
                decoded.reserved_write,
                decoded.instruction_fetch,
                decoded.protection_key,
                decoded.shadow_stack,
                decoded.software_guard
            ));
            if let Some(cr2) = cr2 {
                crate::serial::print(format_args!(
                    "ngos/x86_64: page_fault linear_address={:#x}\n",
                    cr2
                ));
            }
        }
        Some(ExceptionVector::GeneralProtectionFault)
        | Some(ExceptionVector::InvalidTss)
        | Some(ExceptionVector::SegmentNotPresent)
        | Some(ExceptionVector::StackSegmentFault)
        | Some(ExceptionVector::AlignmentCheck)
        | Some(ExceptionVector::ControlProtection) => {
            let decoded = decode_selector_error(frame.error_code);
            crate::serial::print(format_args!(
                "ngos/x86_64: selector_error external={} table={:?} index={:#x}\n",
                decoded.external, decoded.table, decoded.index
            ));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_fault_error_bits_decode_all_flags() {
        let decoded = decode_page_fault_error((1 << 0) | (1 << 1) | (1 << 4) | (1 << 15));
        assert!(decoded.present);
        assert!(decoded.write);
        assert!(decoded.instruction_fetch);
        assert!(decoded.software_guard);
        assert!(!decoded.user);
    }

    #[test]
    fn selector_error_decodes_table_and_index() {
        let decoded = decode_selector_error((5 << 3) | (1 << 1) | 1);
        assert!(decoded.external);
        assert_eq!(decoded.table, SelectorTable::Idt);
        assert_eq!(decoded.index, 5);
    }
}
