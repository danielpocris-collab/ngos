use core::mem::size_of;

pub const IDT_ENTRY_COUNT: usize = 256;
pub const EXCEPTION_VECTOR_COUNT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionVector {
    DivisionError = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRangeExceeded = 5,
    InvalidOpcode = 6,
    DeviceNotAvailable = 7,
    DoubleFault = 8,
    CoprocessorSegmentOverrun = 9,
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    PageFault = 14,
    X87FloatingPoint = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SimdFloatingPoint = 19,
    Virtualization = 20,
    ControlProtection = 21,
    HypervisorInjection = 28,
    VmmCommunication = 29,
    Security = 30,
}

impl ExceptionVector {
    pub const fn from_u8(vector: u8) -> Option<Self> {
        match vector {
            0 => Some(Self::DivisionError),
            1 => Some(Self::Debug),
            2 => Some(Self::NonMaskableInterrupt),
            3 => Some(Self::Breakpoint),
            4 => Some(Self::Overflow),
            5 => Some(Self::BoundRangeExceeded),
            6 => Some(Self::InvalidOpcode),
            7 => Some(Self::DeviceNotAvailable),
            8 => Some(Self::DoubleFault),
            9 => Some(Self::CoprocessorSegmentOverrun),
            10 => Some(Self::InvalidTss),
            11 => Some(Self::SegmentNotPresent),
            12 => Some(Self::StackSegmentFault),
            13 => Some(Self::GeneralProtectionFault),
            14 => Some(Self::PageFault),
            16 => Some(Self::X87FloatingPoint),
            17 => Some(Self::AlignmentCheck),
            18 => Some(Self::MachineCheck),
            19 => Some(Self::SimdFloatingPoint),
            20 => Some(Self::Virtualization),
            21 => Some(Self::ControlProtection),
            28 => Some(Self::HypervisorInjection),
            29 => Some(Self::VmmCommunication),
            30 => Some(Self::Security),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::DivisionError => "division_error",
            Self::Debug => "debug",
            Self::NonMaskableInterrupt => "non_maskable_interrupt",
            Self::Breakpoint => "breakpoint",
            Self::Overflow => "overflow",
            Self::BoundRangeExceeded => "bound_range_exceeded",
            Self::InvalidOpcode => "invalid_opcode",
            Self::DeviceNotAvailable => "device_not_available",
            Self::DoubleFault => "double_fault",
            Self::CoprocessorSegmentOverrun => "coprocessor_segment_overrun",
            Self::InvalidTss => "invalid_tss",
            Self::SegmentNotPresent => "segment_not_present",
            Self::StackSegmentFault => "stack_segment_fault",
            Self::GeneralProtectionFault => "general_protection_fault",
            Self::PageFault => "page_fault",
            Self::X87FloatingPoint => "x87_floating_point",
            Self::AlignmentCheck => "alignment_check",
            Self::MachineCheck => "machine_check",
            Self::SimdFloatingPoint => "simd_floating_point",
            Self::Virtualization => "virtualization",
            Self::ControlProtection => "control_protection",
            Self::HypervisorInjection => "hypervisor_injection",
            Self::VmmCommunication => "vmm_communication",
            Self::Security => "security",
        }
    }

    pub const fn pushes_error_code(self) -> bool {
        matches!(
            self,
            Self::DoubleFault
                | Self::InvalidTss
                | Self::SegmentNotPresent
                | Self::StackSegmentFault
                | Self::GeneralProtectionFault
                | Self::PageFault
                | Self::AlignmentCheck
                | Self::ControlProtection
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IdtGateKind {
    Interrupt = 0x0e,
    Trap = 0x0f,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdtGateOptions {
    pub present: bool,
    pub privilege_level: u8,
    pub stack_index: u8,
    pub kind: IdtGateKind,
}

impl IdtGateOptions {
    pub const fn interrupt() -> Self {
        Self {
            present: true,
            privilege_level: 0,
            stack_index: 0,
            kind: IdtGateKind::Interrupt,
        }
    }

    pub const fn trap() -> Self {
        Self {
            present: true,
            privilege_level: 0,
            stack_index: 0,
            kind: IdtGateKind::Trap,
        }
    }

    pub const fn with_privilege_level(mut self, privilege_level: u8) -> Self {
        self.privilege_level = privilege_level & 0x3;
        self
    }

    pub const fn with_stack_index(mut self, stack_index: u8) -> Self {
        self.stack_index = stack_index & 0x7;
        self
    }

    pub const fn attribute_byte(self) -> u8 {
        (self.kind as u8) | ((self.privilege_level & 0x3) << 5) | ((self.present as u8) << 7)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    attributes: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            attributes: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub const fn new(offset: u64, selector: u16, options: IdtGateOptions) -> Self {
        Self {
            offset_low: (offset & 0xffff) as u16,
            selector,
            ist: options.stack_index & 0x7,
            attributes: options.attribute_byte(),
            offset_mid: ((offset >> 16) & 0xffff) as u16,
            offset_high: ((offset >> 32) & 0xffff_ffff) as u32,
            reserved: 0,
        }
    }

    pub const fn offset(self) -> u64 {
        (self.offset_low as u64)
            | ((self.offset_mid as u64) << 16)
            | ((self.offset_high as u64) << 32)
    }

    pub const fn is_present(self) -> bool {
        (self.attributes & 0x80) != 0
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IdtPointer {
    pub limit: u16,
    pub base: u64,
}

impl IdtPointer {
    pub const fn new(base: u64, bytes: usize) -> Self {
        let limit = if bytes == 0 {
            0
        } else if bytes > (u16::MAX as usize) + 1 {
            u16::MAX
        } else {
            (bytes - 1) as u16
        };
        Self { limit, base }
    }
}

#[repr(C, align(16))]
pub struct InterruptDescriptorTable {
    pub entries: [IdtEntry; IDT_ENTRY_COUNT],
}

impl InterruptDescriptorTable {
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::missing(); IDT_ENTRY_COUNT],
        }
    }

    pub fn clear(&mut self) {
        self.entries = [IdtEntry::missing(); IDT_ENTRY_COUNT];
    }

    pub fn pointer(&self) -> IdtPointer {
        IdtPointer::new(self.entries.as_ptr() as u64, size_of::<Self>())
    }
}

impl Default for InterruptDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ExceptionFrame {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub vector: u64,
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
}

impl ExceptionFrame {
    pub const fn vector_index(&self) -> u8 {
        self.vector as u8
    }

    pub const fn vector_kind(&self) -> Option<ExceptionVector> {
        ExceptionVector::from_u8(self.vector as u8)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idt_entry_encodes_offset_selector_and_attributes() {
        let entry = IdtEntry::new(
            0xffff_ffff_8123_4567,
            0x28,
            IdtGateOptions::interrupt().with_privilege_level(3),
        );

        assert_eq!(entry.offset(), 0xffff_ffff_8123_4567);
        assert!(entry.is_present());
    }

    #[test]
    fn exception_vector_reports_error_code_policy() {
        assert!(ExceptionVector::PageFault.pushes_error_code());
        assert!(ExceptionVector::DoubleFault.pushes_error_code());
        assert!(!ExceptionVector::Breakpoint.pushes_error_code());
        assert!(!ExceptionVector::InvalidOpcode.pushes_error_code());
    }

    #[test]
    fn idt_pointer_uses_table_span() {
        let idt = InterruptDescriptorTable::new();
        let pointer = idt.pointer();
        let limit = pointer.limit;
        let base = pointer.base;

        assert_eq!(limit as usize + 1, size_of::<InterruptDescriptorTable>());
        assert_ne!(base, 0);
    }
}
