#![allow(dead_code)]
#![allow(clippy::needless_return)]

#[cfg(target_os = "none")]
use core::arch::asm;
use core::fmt::{self, Write};
#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};

const COM1_BASE: u16 = 0x3f8;
const DEBUGCON_PORT: u16 = 0x00e9;
#[cfg(target_os = "none")]
static MIRROR_SERIAL_TO_FRAMEBUFFER: AtomicBool = AtomicBool::new(true);

pub fn init() {
    #[cfg(not(target_os = "none"))]
    {
        return;
    }
    #[cfg(target_os = "none")]
    unsafe {
        outb(COM1_BASE + 1, 0x00);
        outb(COM1_BASE + 3, 0x80);
        outb(COM1_BASE, 0x03);
        outb(COM1_BASE + 1, 0x00);
        outb(COM1_BASE + 3, 0x03);
        outb(COM1_BASE + 2, 0xc7);
        outb(COM1_BASE + 4, 0x0b);
    }
}

pub fn debug_marker(byte: u8) {
    #[cfg(not(target_os = "none"))]
    {
        let _ = byte;
        return;
    }
    #[cfg(target_os = "none")]
    unsafe {
        outb(DEBUGCON_PORT, byte);
    }
}

pub fn disable_framebuffer_mirror() {
    #[cfg(target_os = "none")]
    MIRROR_SERIAL_TO_FRAMEBUFFER.store(false, Ordering::SeqCst);
}

#[cfg(test)]
pub fn enable_framebuffer_mirror_for_tests() {
    #[cfg(target_os = "none")]
    MIRROR_SERIAL_TO_FRAMEBUFFER.store(true, Ordering::SeqCst);
}

pub fn print(args: fmt::Arguments<'_>) {
    #[cfg(test)]
    {
        let rendered = std::fmt::format(args);
        write_bytes(rendered.as_bytes());
    }
    #[cfg(not(test))]
    {
        #[cfg(target_os = "none")]
        if MIRROR_SERIAL_TO_FRAMEBUFFER.load(Ordering::SeqCst) {
            crate::framebuffer::print(args);
        }
        let mut writer = SerialWriter;
        let _ = writer.write_fmt(args);
    }
}

pub fn write_bytes(bytes: &[u8]) {
    #[cfg(target_os = "none")]
    if MIRROR_SERIAL_TO_FRAMEBUFFER.load(Ordering::SeqCst) {
        crate::framebuffer::write_bytes(bytes);
    }
    #[cfg(test)]
    TEST_OUTPUT
        .lock()
        .expect("serial output mutex poisoned")
        .extend(bytes.iter().copied());
    for &byte in bytes {
        if byte == b'\n' {
            write_byte(b'\r');
        }
        write_byte(byte);
    }
}

pub fn write_stderr_bytes(bytes: &[u8]) {
    #[cfg(target_os = "none")]
    if MIRROR_SERIAL_TO_FRAMEBUFFER.load(Ordering::SeqCst) {
        crate::framebuffer::write_stderr_bytes(bytes);
    }
    #[cfg(test)]
    TEST_ERROR_OUTPUT
        .lock()
        .expect("serial stderr mutex poisoned")
        .extend(bytes.iter().copied());
    write_bytes(bytes);
}

pub fn input_ready() -> bool {
    input_ready_impl()
}

pub fn read_byte_nonblocking() -> Option<u8> {
    read_byte_nonblocking_impl()
}

struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if byte == b'\n' {
                write_byte(b'\r');
            }
            write_byte(byte);
        }
        Ok(())
    }
}

fn write_byte(byte: u8) {
    #[cfg(not(target_os = "none"))]
    {
        let _ = byte;
        return;
    }
    #[cfg(target_os = "none")]
    unsafe {
        outb(DEBUGCON_PORT, byte);
        while (inb(COM1_BASE + 5) & 0x20) == 0 {
            core::hint::spin_loop();
        }
        outb(COM1_BASE, byte);
    }
}

#[cfg(all(not(test), target_os = "none"))]
fn input_ready_impl() -> bool {
    unsafe { (inb(COM1_BASE + 5) & 0x01) != 0 }
}

#[cfg(all(not(test), target_os = "none"))]
fn read_byte_nonblocking_impl() -> Option<u8> {
    input_ready_impl().then(|| unsafe { inb(COM1_BASE) })
}

#[cfg(all(not(test), not(target_os = "none")))]
fn input_ready_impl() -> bool {
    false
}

#[cfg(all(not(test), not(target_os = "none")))]
fn read_byte_nonblocking_impl() -> Option<u8> {
    None
}

#[cfg(test)]
fn input_ready_impl() -> bool {
    !TEST_INPUT
        .lock()
        .expect("serial input mutex poisoned")
        .is_empty()
}

#[cfg(test)]
fn read_byte_nonblocking_impl() -> Option<u8> {
    TEST_INPUT
        .lock()
        .expect("serial input mutex poisoned")
        .pop_front()
}

#[cfg(test)]
static TEST_INPUT: std::sync::Mutex<std::collections::VecDeque<u8>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
#[cfg(test)]
static TEST_OUTPUT: std::sync::Mutex<std::collections::VecDeque<u8>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
#[cfg(test)]
static TEST_ERROR_OUTPUT: std::sync::Mutex<std::collections::VecDeque<u8>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
#[cfg(test)]
static TEST_IO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub fn lock_test_io() -> std::sync::MutexGuard<'static, ()> {
    TEST_IO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub fn inject_input(bytes: &[u8]) {
    let mut input = TEST_INPUT.lock().expect("serial input mutex poisoned");
    input.extend(bytes.iter().copied());
}

#[cfg(test)]
pub fn clear_input() {
    TEST_INPUT
        .lock()
        .expect("serial input mutex poisoned")
        .clear();
}

#[cfg(test)]
pub fn clear_output() {
    TEST_OUTPUT
        .lock()
        .expect("serial output mutex poisoned")
        .clear();
    TEST_ERROR_OUTPUT
        .lock()
        .expect("serial stderr mutex poisoned")
        .clear();
}

#[cfg(test)]
pub fn take_output() -> Vec<u8> {
    TEST_OUTPUT
        .lock()
        .expect("serial output mutex poisoned")
        .drain(..)
        .collect()
}

#[cfg(test)]
pub fn take_error_output() -> Vec<u8> {
    TEST_ERROR_OUTPUT
        .lock()
        .expect("serial stderr mutex poisoned")
        .drain(..)
        .collect()
}

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
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
