use core::arch::asm;
use core::mem::size_of;

use crate::serial;

const KERNEL_CODE_SELECTOR: u16 = 0x08;
const KERNEL_DATA_SELECTOR: u16 = 0x10;
const TSS_SELECTOR: u16 = 0x18;

const KERNEL_CODE_DESCRIPTOR: u64 = 0x00af_9a00_0000_ffff;
const KERNEL_DATA_DESCRIPTOR: u64 = 0x00af_9200_0000_ffff;
const USER_DATA_DESCRIPTOR: u64 = 0x00af_f200_0000_ffff;
const USER_CODE_DESCRIPTOR: u64 = 0x00af_fa00_0000_ffff;

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
struct TaskStateSegment {
    reserved0: u32,
    rsp: [u64; 3],
    reserved1: u64,
    ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    iomap_base: u16,
}

impl TaskStateSegment {
    const fn new() -> Self {
        Self {
            reserved0: 0,
            rsp: [0; 3],
            reserved1: 0,
            ist: [0; 7],
            reserved2: 0,
            reserved3: 0,
            iomap_base: size_of::<Self>() as u16,
        }
    }
}

static mut EARLY_TSS: TaskStateSegment = TaskStateSegment::new();
static mut EARLY_GDT: [u64; 7] = [0; 7];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GdtBringupError {
    MissingKernelStack,
}

pub fn bring_up(kernel_stack_top: u64) -> Result<(), GdtBringupError> {
    unsafe {
        asm!(
            "mov dx, 0xe9",
            "mov al, 'g'",
            "out dx, al",
            lateout("dx") _,
            lateout("al") _,
            options(nostack, preserves_flags)
        );
    }
    if kernel_stack_top == 0 {
        return Err(GdtBringupError::MissingKernelStack);
    }

    unsafe {
        EARLY_TSS.rsp[0] = kernel_stack_top;
        EARLY_TSS.iomap_base = size_of::<TaskStateSegment>() as u16;

        EARLY_GDT[0] = 0;
        EARLY_GDT[1] = KERNEL_CODE_DESCRIPTOR;
        EARLY_GDT[2] = KERNEL_DATA_DESCRIPTOR;
        let (tss_low, tss_high) = tss_descriptor((&raw const EARLY_TSS) as u64);
        EARLY_GDT[3] = tss_low;
        EARLY_GDT[4] = tss_high;
        EARLY_GDT[5] = USER_DATA_DESCRIPTOR;
        EARLY_GDT[6] = USER_CODE_DESCRIPTOR;

        let pointer = DescriptorTablePointer {
            limit: (size_of::<[u64; 7]>() - 1) as u16,
            base: (&raw const EARLY_GDT) as u64,
        };
        load_gdt_and_tss(&pointer);
        serial::debug_marker(b'j');
        let base = pointer.base;
        let limit = pointer.limit;
        serial::print(format_args!(
            "ngos/x86_64: gdt installed base={:#x} limit={:#x} tss={:#x} rsp0={:#x}\n",
            base,
            limit,
            (&raw const EARLY_TSS) as u64,
            kernel_stack_top
        ));
    }

    Ok(())
}

unsafe fn load_gdt_and_tss(pointer: *const DescriptorTablePointer) {
    unsafe {
        asm!(
            "lgdt [{pointer}]",
            "mov dx, 0xe9",
            "mov al, 'h'",
            "out dx, al",
            "push {code}",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            "mov dx, 0xe9",
            "mov al, 'i'",
            "out dx, al",
            "mov ax, {data}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            "mov ax, {tss}",
            "ltr ax",
            "mov dx, 0xe9",
            "mov al, 'k'",
            "out dx, al",
            pointer = in(reg) pointer,
            code = const KERNEL_CODE_SELECTOR as u64,
            data = const KERNEL_DATA_SELECTOR,
            tss = const TSS_SELECTOR,
            lateout("rax") _,
            lateout("rdx") _,
            options(preserves_flags)
        );
    }
}

const fn tss_descriptor(base: u64) -> (u64, u64) {
    let limit = (size_of::<TaskStateSegment>() - 1) as u64;
    let low = (limit & 0xffff)
        | ((base & 0x00ff_ffff) << 16)
        | (0x89u64 << 40)
        | (((limit >> 16) & 0xf) << 48)
        | (((base >> 24) & 0xff) << 56);
    let high = base >> 32;
    (low, high)
}
