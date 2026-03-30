use core::arch::asm;

pub const IRQ_BASE_PRIMARY: u8 = 32;
pub const IRQ_BASE_SECONDARY: u8 = 40;
pub const IRQ_TIMER: u8 = IRQ_BASE_PRIMARY;
pub const IRQ_KEYBOARD: u8 = IRQ_BASE_PRIMARY + 1;

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xa0;
const PIC2_DATA: u16 = 0xa1;
const PIC_EOI: u8 = 0x20;

pub fn remap_and_unmask_timer_keyboard() {
    unsafe {
        outb(PIC1_COMMAND, 0x11);
        io_wait();
        outb(PIC2_COMMAND, 0x11);
        io_wait();

        outb(PIC1_DATA, IRQ_BASE_PRIMARY);
        io_wait();
        outb(PIC2_DATA, IRQ_BASE_SECONDARY);
        io_wait();

        outb(PIC1_DATA, 4);
        io_wait();
        outb(PIC2_DATA, 2);
        io_wait();

        outb(PIC1_DATA, 0x01);
        io_wait();
        outb(PIC2_DATA, 0x01);
        io_wait();

        outb(PIC1_DATA, 0xfc);
        io_wait();
        outb(PIC2_DATA, 0xff);
        io_wait();
    }
}

pub fn unmask_irq_line(line: u8) {
    if line >= 16 {
        return;
    }
    unsafe {
        if line >= 8 {
            let mut secondary_mask = inb(PIC2_DATA);
            secondary_mask &= !(1u8 << (line - 8));
            outb(PIC2_DATA, secondary_mask);

            let mut primary_mask = inb(PIC1_DATA);
            primary_mask &= !(1u8 << 2);
            outb(PIC1_DATA, primary_mask);
        } else {
            let mut primary_mask = inb(PIC1_DATA);
            primary_mask &= !(1u8 << line);
            outb(PIC1_DATA, primary_mask);
        }
        io_wait();
    }
}

pub fn mask_irq_line(line: u8) {
    if line >= 16 {
        return;
    }
    unsafe {
        if line >= 8 {
            let mut secondary_mask = inb(PIC2_DATA);
            secondary_mask |= 1u8 << (line - 8);
            outb(PIC2_DATA, secondary_mask);
        } else {
            let mut primary_mask = inb(PIC1_DATA);
            primary_mask |= 1u8 << line;
            outb(PIC1_DATA, primary_mask);
        }
        io_wait();
    }
}

pub fn end_of_interrupt(vector: u8) {
    unsafe {
        if vector >= IRQ_BASE_SECONDARY {
            outb(PIC2_COMMAND, PIC_EOI);
        }
        outb(PIC1_COMMAND, PIC_EOI);
    }
}

unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

unsafe fn io_wait() {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") 0x80u16,
            in("al") 0u8,
            options(nomem, nostack, preserves_flags)
        );
    }
}
