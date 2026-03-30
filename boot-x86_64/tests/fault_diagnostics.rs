#![allow(dead_code)]
#![allow(clippy::needless_return)]

mod serial {
    pub fn print(_args: core::fmt::Arguments<'_>) {}
}

mod platform_x86_64 {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum ExceptionVector {
        GeneralProtectionFault = 13,
        PageFault = 14,
    }

    impl ExceptionVector {
        pub const fn from_u8(vector: u8) -> Option<Self> {
            match vector {
                13 => Some(Self::GeneralProtectionFault),
                14 => Some(Self::PageFault),
                _ => None,
            }
        }

        pub const fn name(self) -> &'static str {
            match self {
                Self::GeneralProtectionFault => "general_protection_fault",
                Self::PageFault => "page_fault",
            }
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
        pub const fn vector_kind(&self) -> Option<ExceptionVector> {
            ExceptionVector::from_u8(self.vector as u8)
        }
    }
}

#[path = "../src/fault_diag.rs"]
mod fault_diag;

#[test]
fn page_fault_decoder_reports_full_access_context() {
    let decoded =
        fault_diag::decode_page_fault_error((1 << 0) | (1 << 1) | (1 << 2) | (1 << 4) | (1 << 6));
    assert!(decoded.present);
    assert!(decoded.write);
    assert!(decoded.user);
    assert!(decoded.instruction_fetch);
    assert!(decoded.shadow_stack);
    assert!(!decoded.protection_key);
}

#[test]
fn selector_error_decoder_reports_table_origin() {
    let decoded = fault_diag::decode_selector_error((9 << 3) | (2 << 1));
    assert!(!decoded.external);
    assert_eq!(decoded.table, fault_diag::SelectorTable::Ldt);
    assert_eq!(decoded.index, 9);
}
