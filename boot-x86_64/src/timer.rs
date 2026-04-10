//! Canonical subsystem role:
//! - subsystem: boot timer and clocksource discovery
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-stage discovery and recording of timer facts for
//!   the real x86 path
//!
//! Canonical contract families handled here:
//! - clocksource discovery contracts
//! - TSC/PIT fact contracts
//! - boot timer status contracts
//!
//! This module may discover and record boot timer facts, but it must not
//! redefine long-term scheduler or runtime timekeeping ownership.

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, __cpuid_count, _rdtsc};
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClocksourceKind {
    TscLeaf15,
    TscLeaf16,
    Pit,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerFacts {
    pub kind: ClocksourceKind,
    pub tsc_hz: u64,
    pub boot_tsc: u64,
    pub invariant_tsc: bool,
}

static CLOCKSOURCE_KIND: AtomicU64 = AtomicU64::new(ClocksourceKind::Unavailable as u64);
static TSC_HZ: AtomicU64 = AtomicU64::new(0);
static BOOT_TSC: AtomicU64 = AtomicU64::new(0);
static PIT_TICK_HZ: AtomicU64 = AtomicU64::new(0);
static PIT_TICKS: AtomicU64 = AtomicU64::new(0);

pub fn init() -> TimerFacts {
    #[cfg(target_arch = "x86_64")]
    {
        let invariant_tsc = has_invariant_tsc();
        let (mut kind, tsc_hz) = detect_tsc_hz();
        if tsc_hz == 0 && pit_tick_hz() != 0 {
            kind = ClocksourceKind::Pit;
        }
        let boot_tsc = if tsc_hz != 0 { unsafe { _rdtsc() } } else { 0 };
        CLOCKSOURCE_KIND.store(kind as u64, Ordering::Relaxed);
        TSC_HZ.store(tsc_hz, Ordering::Relaxed);
        BOOT_TSC.store(boot_tsc, Ordering::Relaxed);
        return TimerFacts {
            kind,
            tsc_hz,
            boot_tsc,
            invariant_tsc,
        };
    }

    #[allow(unreachable_code)]
    TimerFacts {
        kind: ClocksourceKind::Unavailable,
        tsc_hz: 0,
        boot_tsc: 0,
        invariant_tsc: false,
    }
}

#[allow(dead_code)]
pub fn clocksource_kind() -> ClocksourceKind {
    match CLOCKSOURCE_KIND.load(Ordering::Relaxed) {
        0 => ClocksourceKind::TscLeaf15,
        1 => ClocksourceKind::TscLeaf16,
        2 => ClocksourceKind::Pit,
        _ => ClocksourceKind::Unavailable,
    }
}

pub fn tsc_hz() -> u64 {
    TSC_HZ.load(Ordering::Relaxed)
}

pub fn set_pit_tick_rate(hz: u32) {
    PIT_TICK_HZ.store(hz as u64, Ordering::Relaxed);
    PIT_TICKS.store(0, Ordering::Relaxed);
}

pub fn record_pit_tick() -> u64 {
    PIT_TICKS.fetch_add(1, Ordering::Relaxed) + 1
}

pub fn pit_tick_count() -> u64 {
    PIT_TICKS.load(Ordering::Relaxed)
}

pub fn pit_tick_hz() -> u64 {
    PIT_TICK_HZ.load(Ordering::Relaxed)
}

pub fn boot_uptime_from_pit_micros() -> Option<u64> {
    let hz = pit_tick_hz();
    if hz == 0 {
        return None;
    }
    Some(ticks_to_micros(pit_tick_count(), hz))
}

pub fn boot_uptime_micros() -> Option<u64> {
    if let Some(micros) = boot_uptime_from_pit_micros() {
        return Some(micros);
    }
    let hz = tsc_hz();
    if hz != 0 {
        #[cfg(target_arch = "x86_64")]
        {
            let now = unsafe { _rdtsc() };
            let boot = BOOT_TSC.load(Ordering::Relaxed);
            return Some(ticks_to_micros(now.saturating_sub(boot), hz));
        }
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(target_arch = "x86_64")]
fn detect_tsc_hz() -> (ClocksourceKind, u64) {
    let max_basic = __cpuid(0).eax;
    if max_basic >= 0x15 {
        let leaf = __cpuid_count(0x15, 0);
        if let Some(hz) = tsc_hz_from_leaf15(leaf.eax, leaf.ebx, leaf.ecx) {
            return (ClocksourceKind::TscLeaf15, hz);
        }
    }
    if max_basic >= 0x16 {
        let leaf = __cpuid_count(0x16, 0);
        if let Some(hz) = tsc_hz_from_leaf16(leaf.eax) {
            return (ClocksourceKind::TscLeaf16, hz);
        }
    }
    (ClocksourceKind::Unavailable, 0)
}

#[cfg(target_arch = "x86_64")]
fn has_invariant_tsc() -> bool {
    let max_extended = __cpuid(0x8000_0000).eax;
    if max_extended < 0x8000_0007 {
        return false;
    }
    let leaf = __cpuid(0x8000_0007);
    (leaf.edx & (1 << 8)) != 0
}

pub const fn tsc_hz_from_leaf15(denominator: u32, numerator: u32, crystal_hz: u32) -> Option<u64> {
    if denominator == 0 || numerator == 0 || crystal_hz == 0 {
        return None;
    }
    Some((crystal_hz as u64).saturating_mul(numerator as u64) / denominator as u64)
}

pub const fn tsc_hz_from_leaf16(base_mhz: u32) -> Option<u64> {
    if base_mhz == 0 {
        None
    } else {
        Some((base_mhz as u64).saturating_mul(1_000_000))
    }
}

pub const fn ticks_to_micros(ticks: u64, hz: u64) -> u64 {
    if hz == 0 {
        0
    } else {
        ticks.saturating_mul(1_000_000) / hz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaf15_frequency_uses_crystal_ratio() {
        assert_eq!(tsc_hz_from_leaf15(3, 200, 24_000_000), Some(1_600_000_000));
        assert_eq!(tsc_hz_from_leaf15(0, 1, 24_000_000), None);
    }

    #[test]
    fn leaf16_frequency_uses_base_mhz() {
        assert_eq!(tsc_hz_from_leaf16(3200), Some(3_200_000_000));
        assert_eq!(tsc_hz_from_leaf16(0), None);
    }

    #[test]
    fn tick_conversion_reports_microseconds() {
        assert_eq!(ticks_to_micros(3_200_000, 3_200_000_000), 1000);
        assert_eq!(ticks_to_micros(0, 3_200_000_000), 0);
    }

    #[test]
    fn clocksource_kind_decodes_pit() {
        CLOCKSOURCE_KIND.store(ClocksourceKind::Pit as u64, Ordering::Relaxed);
        assert_eq!(clocksource_kind(), ClocksourceKind::Pit);
    }
}
