use core::arch::asm;

pub const PIT_INPUT_HZ: u32 = 1_193_182;
const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL0: u16 = 0x40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PitConfig {
    pub requested_hz: u32,
    pub reload_value: u16,
    pub actual_hz: u32,
}

pub fn program_periodic(requested_hz: u32) -> PitConfig {
    let divisor = divisor_for_rate(requested_hz);
    let actual_hz = actual_rate_for_divisor(divisor);
    unsafe {
        outb(PIT_COMMAND, 0x36);
        outb(PIT_CHANNEL0, (divisor & 0xff) as u8);
        outb(PIT_CHANNEL0, (divisor >> 8) as u8);
    }
    PitConfig {
        requested_hz,
        reload_value: divisor,
        actual_hz,
    }
}

pub const fn divisor_for_rate(requested_hz: u32) -> u16 {
    if requested_hz == 0 {
        return u16::MAX;
    }
    let mut divisor = PIT_INPUT_HZ / requested_hz;
    if divisor == 0 {
        divisor = 1;
    }
    if divisor > u16::MAX as u32 {
        u16::MAX
    } else {
        divisor as u16
    }
}

pub const fn actual_rate_for_divisor(divisor: u16) -> u32 {
    let divisor = if divisor == 0 { 65_536 } else { divisor as u32 };
    PIT_INPUT_HZ / divisor
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisor_rounds_to_nonzero_value() {
        assert_eq!(divisor_for_rate(100), 11_931);
        assert_eq!(divisor_for_rate(0), u16::MAX);
    }

    #[test]
    fn actual_rate_uses_programmed_divisor() {
        assert_eq!(actual_rate_for_divisor(11_931), 100);
        assert!(actual_rate_for_divisor(1) >= PIT_INPUT_HZ / 2);
    }
}
