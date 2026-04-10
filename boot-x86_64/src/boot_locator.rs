#![cfg_attr(not(target_os = "none"), allow(dead_code))]
#![allow(clippy::too_many_arguments)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU64, Ordering};

const EARLY_BLACKBOX_CAPACITY: usize = 256;
const LOCATOR_RING_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BootLocatorStage {
    Reset = 1,
    Stage0 = 2,
    Limine = 3,
    EarlyKernel = 4,
    Paging = 5,
    Traps = 6,
    Smp = 7,
    User = 8,
    Fault = 9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BootLocatorKind {
    Progress = 1,
    Transition = 2,
    Memory = 3,
    Contract = 4,
    Fault = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BootLocatorSeverity {
    Trace = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BootPayloadLabel {
    None = 0,
    Address = 1,
    Length = 2,
    Count = 3,
    Rip = 4,
    Value = 5,
    Status = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootLocatorRecord {
    pub sequence: u64,
    pub stage: BootLocatorStage,
    pub kind: BootLocatorKind,
    pub severity: BootLocatorSeverity,
    pub checkpoint: u64,
    pub payload0_label: BootPayloadLabel,
    pub payload0: u64,
    pub payload1_label: BootPayloadLabel,
    pub payload1: u64,
}

impl BootLocatorRecord {
    pub const EMPTY: Self = Self {
        sequence: 0,
        stage: BootLocatorStage::Reset,
        kind: BootLocatorKind::Progress,
        severity: BootLocatorSeverity::Trace,
        checkpoint: 0,
        payload0_label: BootPayloadLabel::None,
        payload0: 0,
        payload1_label: BootPayloadLabel::None,
        payload1: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarlyBootRecord {
    pub sequence: u32,
    pub stage: BootLocatorStage,
    pub kind: BootLocatorKind,
    pub severity: BootLocatorSeverity,
    pub checkpoint: u64,
    pub payload0_label: BootPayloadLabel,
    pub payload0: u64,
    pub payload1_label: BootPayloadLabel,
    pub payload1: u64,
}

impl EarlyBootRecord {
    pub const EMPTY: Self = Self {
        sequence: 0,
        stage: BootLocatorStage::Reset,
        kind: BootLocatorKind::Progress,
        severity: BootLocatorSeverity::Trace,
        checkpoint: 0,
        payload0_label: BootPayloadLabel::None,
        payload0: 0,
        payload1_label: BootPayloadLabel::None,
        payload1: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarlyBootSnapshot {
    pub magic: u64,
    pub cursor: u32,
    pub wrapped: bool,
    pub last: EarlyBootRecord,
    pub records: [EarlyBootRecord; EARLY_BLACKBOX_CAPACITY],
}

impl EarlyBootSnapshot {
    #[allow(dead_code)]
    pub const EMPTY: Self = Self {
        magic: 0,
        cursor: 0,
        wrapped: false,
        last: EarlyBootRecord::EMPTY,
        records: [EarlyBootRecord::EMPTY; EARLY_BLACKBOX_CAPACITY],
    };
}

#[repr(C)]
struct EarlyBootBlackbox {
    magic: u64,
    cursor: u32,
    wrapped: u32,
    last: EarlyBootRecord,
    records: [EarlyBootRecord; EARLY_BLACKBOX_CAPACITY],
}

impl EarlyBootBlackbox {
    const MAGIC: u64 = 0x4e47_4f53_4742_5053;

    const EMPTY: Self = Self {
        magic: 0,
        cursor: 0,
        wrapped: 0,
        last: EarlyBootRecord::EMPTY,
        records: [EarlyBootRecord::EMPTY; EARLY_BLACKBOX_CAPACITY],
    };
}

struct BootLocatorStorage {
    records: UnsafeCell<[BootLocatorRecord; LOCATOR_RING_CAPACITY]>,
    early_blackbox: UnsafeCell<EarlyBootBlackbox>,
}

unsafe impl Sync for BootLocatorStorage {}

static LOCATOR_STORAGE: BootLocatorStorage = BootLocatorStorage {
    records: UnsafeCell::new([BootLocatorRecord::EMPTY; LOCATOR_RING_CAPACITY]),
    early_blackbox: UnsafeCell::new(EarlyBootBlackbox::EMPTY),
};
static LOCATOR_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static LAST_STAGE: AtomicU64 = AtomicU64::new(BootLocatorStage::Reset as u64);
static LAST_KIND: AtomicU64 = AtomicU64::new(BootLocatorKind::Progress as u64);
static LAST_SEVERITY: AtomicU64 = AtomicU64::new(BootLocatorSeverity::Trace as u64);
static LAST_CHECKPOINT: AtomicU64 = AtomicU64::new(0);
static LAST_PAYLOAD0_LABEL: AtomicU64 = AtomicU64::new(BootPayloadLabel::None as u64);
static LAST_PAYLOAD0: AtomicU64 = AtomicU64::new(0);
static LAST_PAYLOAD1_LABEL: AtomicU64 = AtomicU64::new(BootPayloadLabel::None as u64);
static LAST_PAYLOAD1: AtomicU64 = AtomicU64::new(0);

pub fn reset() {
    LOCATOR_SEQUENCE.store(0, Ordering::SeqCst);
    LAST_STAGE.store(BootLocatorStage::Reset as u64, Ordering::SeqCst);
    LAST_KIND.store(BootLocatorKind::Progress as u64, Ordering::SeqCst);
    LAST_SEVERITY.store(BootLocatorSeverity::Trace as u64, Ordering::SeqCst);
    LAST_CHECKPOINT.store(0, Ordering::SeqCst);
    LAST_PAYLOAD0_LABEL.store(BootPayloadLabel::None as u64, Ordering::SeqCst);
    LAST_PAYLOAD0.store(0, Ordering::SeqCst);
    LAST_PAYLOAD1_LABEL.store(BootPayloadLabel::None as u64, Ordering::SeqCst);
    LAST_PAYLOAD1.store(0, Ordering::SeqCst);
    unsafe {
        core::ptr::write_bytes(
            LOCATOR_STORAGE.records.get().cast::<u8>(),
            0,
            core::mem::size_of::<[BootLocatorRecord; LOCATOR_RING_CAPACITY]>(),
        );
        core::ptr::write(
            LOCATOR_STORAGE.early_blackbox.get(),
            EarlyBootBlackbox::EMPTY,
        );
    }
}

pub fn early_reset() {
    unsafe {
        let blackbox = &mut *LOCATOR_STORAGE.early_blackbox.get();
        core::ptr::write(blackbox, EarlyBootBlackbox::EMPTY);
        core::ptr::write_volatile(&mut blackbox.magic, EarlyBootBlackbox::MAGIC);
    }
}

pub fn checkpoint(stage: BootLocatorStage, checkpoint: u64, payload0: u64, payload1: u64) {
    event(
        stage,
        BootLocatorKind::Progress,
        BootLocatorSeverity::Info,
        checkpoint,
        BootPayloadLabel::Value,
        payload0,
        BootPayloadLabel::Value,
        payload1,
    );
}

pub fn early_checkpoint(stage: BootLocatorStage, checkpoint: u64, payload0: u64, payload1: u64) {
    unsafe {
        let blackbox = &mut *LOCATOR_STORAGE.early_blackbox.get();
        let cursor = core::ptr::read_volatile(&blackbox.cursor);
        let slot = (cursor as usize) % EARLY_BLACKBOX_CAPACITY;
        let sequence = cursor.wrapping_add(1);
        write_early_record(
            &mut blackbox.records[slot],
            sequence,
            stage,
            BootLocatorKind::Progress,
            BootLocatorSeverity::Info,
            checkpoint,
            BootPayloadLabel::Value,
            payload0,
            BootPayloadLabel::Value,
            payload1,
        );
        write_early_record(
            &mut blackbox.last,
            sequence,
            stage,
            BootLocatorKind::Progress,
            BootLocatorSeverity::Info,
            checkpoint,
            BootPayloadLabel::Value,
            payload0,
            BootPayloadLabel::Value,
            payload1,
        );
        core::ptr::write_volatile(&mut blackbox.cursor, sequence);
        if sequence as usize > EARLY_BLACKBOX_CAPACITY {
            core::ptr::write_volatile(&mut blackbox.wrapped, 1);
        }
    }
}

pub fn event(
    stage: BootLocatorStage,
    kind: BootLocatorKind,
    severity: BootLocatorSeverity,
    checkpoint: u64,
    payload0_label: BootPayloadLabel,
    payload0: u64,
    payload1_label: BootPayloadLabel,
    payload1: u64,
) {
    let sequence = LOCATOR_SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1;
    let slot = (sequence as usize - 1) % LOCATOR_RING_CAPACITY;
    unsafe {
        write_record(
            core::ptr::addr_of_mut!((*LOCATOR_STORAGE.records.get())[slot]),
            sequence,
            stage,
            kind,
            severity,
            checkpoint,
            payload0_label,
            payload0,
            payload1_label,
            payload1,
        );
    }
    LAST_STAGE.store(stage as u64, Ordering::SeqCst);
    LAST_KIND.store(kind as u64, Ordering::SeqCst);
    LAST_SEVERITY.store(severity as u64, Ordering::SeqCst);
    LAST_CHECKPOINT.store(checkpoint, Ordering::SeqCst);
    LAST_PAYLOAD0_LABEL.store(payload0_label as u64, Ordering::SeqCst);
    LAST_PAYLOAD0.store(payload0, Ordering::SeqCst);
    LAST_PAYLOAD1_LABEL.store(payload1_label as u64, Ordering::SeqCst);
    LAST_PAYLOAD1.store(payload1, Ordering::SeqCst);
    #[cfg(target_os = "none")]
    crate::reboot_trace::record_locator(BootLocatorRecord {
        sequence,
        stage,
        kind,
        severity,
        checkpoint,
        payload0_label,
        payload0,
        payload1_label,
        payload1,
    });
}

pub fn early_event(
    stage: BootLocatorStage,
    kind: BootLocatorKind,
    severity: BootLocatorSeverity,
    checkpoint: u64,
    payload0_label: BootPayloadLabel,
    payload0: u64,
    payload1_label: BootPayloadLabel,
    payload1: u64,
) {
    unsafe {
        let blackbox = &mut *LOCATOR_STORAGE.early_blackbox.get();
        let cursor = core::ptr::read_volatile(&blackbox.cursor);
        let slot = (cursor as usize) % EARLY_BLACKBOX_CAPACITY;
        let sequence = cursor.wrapping_add(1);
        write_early_record(
            &mut blackbox.records[slot],
            sequence,
            stage,
            kind,
            severity,
            checkpoint,
            payload0_label,
            payload0,
            payload1_label,
            payload1,
        );
        write_early_record(
            &mut blackbox.last,
            sequence,
            stage,
            kind,
            severity,
            checkpoint,
            payload0_label,
            payload0,
            payload1_label,
            payload1,
        );
        core::ptr::write_volatile(&mut blackbox.cursor, sequence);
        if sequence as usize > EARLY_BLACKBOX_CAPACITY {
            core::ptr::write_volatile(&mut blackbox.wrapped, 1);
        }
    }
}

unsafe fn write_early_record(
    target: *mut EarlyBootRecord,
    sequence: u32,
    stage: BootLocatorStage,
    kind: BootLocatorKind,
    severity: BootLocatorSeverity,
    checkpoint: u64,
    payload0_label: BootPayloadLabel,
    payload0: u64,
    payload1_label: BootPayloadLabel,
    payload1: u64,
) {
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*target).sequence), sequence);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*target).stage), stage);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*target).kind), kind);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*target).severity), severity);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*target).checkpoint), checkpoint);
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!((*target).payload0_label),
            payload0_label,
        );
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*target).payload0), payload0);
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!((*target).payload1_label),
            payload1_label,
        );
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*target).payload1), payload1);
    }
}

unsafe fn write_record(
    target: *mut BootLocatorRecord,
    sequence: u64,
    stage: BootLocatorStage,
    kind: BootLocatorKind,
    severity: BootLocatorSeverity,
    checkpoint: u64,
    payload0_label: BootPayloadLabel,
    payload0: u64,
    payload1_label: BootPayloadLabel,
    payload1: u64,
) {
    unsafe {
        core::ptr::write(core::ptr::addr_of_mut!((*target).sequence), sequence);
        core::ptr::write(core::ptr::addr_of_mut!((*target).stage), stage);
        core::ptr::write(core::ptr::addr_of_mut!((*target).kind), kind);
        core::ptr::write(core::ptr::addr_of_mut!((*target).severity), severity);
        core::ptr::write(core::ptr::addr_of_mut!((*target).checkpoint), checkpoint);
        core::ptr::write(
            core::ptr::addr_of_mut!((*target).payload0_label),
            payload0_label,
        );
        core::ptr::write(core::ptr::addr_of_mut!((*target).payload0), payload0);
        core::ptr::write(
            core::ptr::addr_of_mut!((*target).payload1_label),
            payload1_label,
        );
        core::ptr::write(core::ptr::addr_of_mut!((*target).payload1), payload1);
    }
}

pub fn snapshot() -> BootLocatorRecord {
    BootLocatorRecord {
        sequence: LOCATOR_SEQUENCE.load(Ordering::SeqCst),
        stage: decode_stage(LAST_STAGE.load(Ordering::SeqCst)),
        kind: decode_kind(LAST_KIND.load(Ordering::SeqCst)),
        severity: decode_severity(LAST_SEVERITY.load(Ordering::SeqCst)),
        checkpoint: LAST_CHECKPOINT.load(Ordering::SeqCst),
        payload0_label: decode_payload_label(LAST_PAYLOAD0_LABEL.load(Ordering::SeqCst)),
        payload0: LAST_PAYLOAD0.load(Ordering::SeqCst),
        payload1_label: decode_payload_label(LAST_PAYLOAD1_LABEL.load(Ordering::SeqCst)),
        payload1: LAST_PAYLOAD1.load(Ordering::SeqCst),
    }
}

pub fn ring_snapshot() -> [BootLocatorRecord; LOCATOR_RING_CAPACITY] {
    unsafe { *LOCATOR_STORAGE.records.get() }
}

pub fn early_snapshot() -> EarlyBootSnapshot {
    unsafe {
        let blackbox = &*LOCATOR_STORAGE.early_blackbox.get();
        EarlyBootSnapshot {
            magic: core::ptr::read_volatile(&blackbox.magic),
            cursor: core::ptr::read_volatile(&blackbox.cursor),
            wrapped: core::ptr::read_volatile(&blackbox.wrapped) != 0,
            last: core::ptr::read_volatile(&blackbox.last),
            records: core::ptr::read_volatile(&blackbox.records),
        }
    }
}

pub fn early_recent(limit: usize) -> [EarlyBootRecord; EARLY_BLACKBOX_CAPACITY] {
    let snapshot = early_snapshot();
    let mut ordered = [EarlyBootRecord::EMPTY; EARLY_BLACKBOX_CAPACITY];
    let capped = limit.min(EARLY_BLACKBOX_CAPACITY);
    if capped == 0 || snapshot.cursor == 0 {
        return ordered;
    }

    let available = (snapshot.cursor as usize).min(EARLY_BLACKBOX_CAPACITY);
    let count = capped.min(available);
    let start_sequence = snapshot.cursor.saturating_sub(count as u32) + 1;

    let mut out = 0usize;
    let mut sequence = start_sequence;
    while sequence <= snapshot.cursor && out < EARLY_BLACKBOX_CAPACITY {
        let slot = (sequence as usize - 1) % EARLY_BLACKBOX_CAPACITY;
        let record = snapshot.records[slot];
        if record.sequence == sequence {
            ordered[out] = record;
            out += 1;
        }
        sequence = sequence.wrapping_add(1);
    }

    ordered
}

pub fn recent(limit: usize) -> [BootLocatorRecord; LOCATOR_RING_CAPACITY] {
    let ring = ring_snapshot();
    let mut ordered = [BootLocatorRecord::EMPTY; LOCATOR_RING_CAPACITY];
    let capped = limit.min(LOCATOR_RING_CAPACITY);
    if capped == 0 {
        return ordered;
    }

    let current = LOCATOR_SEQUENCE.load(Ordering::SeqCst) as usize;
    if current == 0 {
        return ordered;
    }

    let available = current.min(LOCATOR_RING_CAPACITY);
    let count = capped.min(available);
    let start_sequence = current.saturating_sub(count) + 1;

    let mut out = 0usize;
    let mut sequence = start_sequence;
    while sequence <= current && out < LOCATOR_RING_CAPACITY {
        let slot = (sequence - 1) % LOCATOR_RING_CAPACITY;
        let record = ring[slot];
        if record.sequence == sequence as u64 {
            ordered[out] = record;
            out += 1;
        }
        sequence += 1;
    }

    ordered
}

pub fn checkpoint_name(stage: BootLocatorStage, checkpoint: u64) -> &'static str {
    match (stage, checkpoint) {
        (BootLocatorStage::Stage0, 0x10) => "stage0/entry",
        (BootLocatorStage::Stage0, 0x20) => "stage0/bss-cleared",
        (BootLocatorStage::Stage0, 0x30) => "stage0/serial-ready",
        (BootLocatorStage::Stage0, 0x40) => "stage0/diagnostics-reset",
        (BootLocatorStage::Stage0, 0x41) => "stage0/post-reset",
        (BootLocatorStage::Stage0, 0x42) => "stage0/pre-logln",
        (BootLocatorStage::Stage0, 0x43) => "stage0/post-logln",
        (BootLocatorStage::Stage0, 0x44) => "stage0/pre-log-checkpoint",
        (BootLocatorStage::Stage0, 0x45) => "stage0/post-log-checkpoint",
        (BootLocatorStage::Stage0, 0x46) => "stage0/pre-kernel-span",
        (BootLocatorStage::Stage0, 0x47) => "stage0/post-kernel-span",
        (BootLocatorStage::Stage0, 0x50) => "stage0/pre-trace-stage0",
        (BootLocatorStage::Stage0, 0x60) => "stage0/stage0-traced",
        (BootLocatorStage::Stage0, 0x70) => "stage0/log-online",
        (BootLocatorStage::Stage0, 0x80) => "stage0/kernel-span-known",
        (BootLocatorStage::Limine, 0x90) => "limine/bootinfo-ready",
        (BootLocatorStage::Limine, 0x200) => "limine/write-bootinfo-entry",
        (BootLocatorStage::Limine, 0x210) => "limine/memory-map-ready",
        (BootLocatorStage::Limine, 0x220) => "limine/hhdm-ready",
        (BootLocatorStage::Limine, 0x230) => "limine/modules-enumerated",
        (BootLocatorStage::Limine, 0x240) => "limine/rsdp-captured",
        (BootLocatorStage::Limine, 0x250) => "limine/kernel-range-written",
        (BootLocatorStage::Limine, 0x2ff) => "limine/contract-refusal",
        (BootLocatorStage::EarlyKernel, 0x100) => "early-kernel/entry",
        (BootLocatorStage::Paging, 0x300) => "paging/bringup-entry",
        (BootLocatorStage::Paging, 0x310) => "paging/windows-sized",
        (BootLocatorStage::Paging, 0x320) => "paging/arena-allocated",
        (BootLocatorStage::Paging, 0x330) => "paging/cr3-reloaded",
        (BootLocatorStage::Paging, 0x340) => "paging/handoff-ready",
        (BootLocatorStage::Paging, 0x341) => "paging/post-handoff",
        (BootLocatorStage::Traps, 0x420) => "traps/sse-ready",
        (BootLocatorStage::Traps, 0x430) => "traps/gdt-ready",
        (BootLocatorStage::Traps, 0x440) => "traps/idt-ready",
        (BootLocatorStage::Traps, 0x450) => "traps/interrupts-enabled",
        (BootLocatorStage::Traps, 0x460) => "traps/int3-returned",
        (BootLocatorStage::Traps, 0x470) => "traps/timer-ready",
        (BootLocatorStage::Smp, 0x400) => "smp/bootstrap-prepared",
        (BootLocatorStage::Smp, 0x410) => "smp/online-report",
        (BootLocatorStage::Smp, 0x41f) => "smp/dispatch-failed",
        (BootLocatorStage::Smp, 0x700) => "smp/bootstrap-apic-known",
        (BootLocatorStage::Smp, 0x710) => "smp/topology-known",
        (BootLocatorStage::Smp, 0x720) => "smp/targets-built",
        (BootLocatorStage::Smp, 0x730) => "smp/bringup-entry",
        (BootLocatorStage::Smp, 0x740) => "smp/startup-ipi-sent",
        (BootLocatorStage::Smp, 0x750) => "smp/wait-online",
        (BootLocatorStage::User, 0x500) => "user/launch-prepared",
        (BootLocatorStage::User, 0x510) => "user/mappings-installed",
        (BootLocatorStage::User, 0x520) => "user/enter-user-mode",
        (BootLocatorStage::User, 0x540) => "user/process-prepare-entry",
        (BootLocatorStage::User, 0x550) => "user/module-window-ready",
        (BootLocatorStage::User, 0x560) => "user/start-frame-ready",
        (BootLocatorStage::User, 0x565) => "user/bootstrap-cpu-contract-ready",
        (BootLocatorStage::User, 0x570) => "user/bridge-install-entry",
        (BootLocatorStage::User, 0x571) => "user/cpu-provider-installed",
        (BootLocatorStage::User, 0x572) => "user/cpu-provider-skipped",
        (BootLocatorStage::User, 0x600) => "virtio/dma-window-entry",
        (BootLocatorStage::User, 0x610) => "virtio/devices-enumerated",
        (BootLocatorStage::User, 0x620) => "virtio/driver-online",
        (BootLocatorStage::User, 0x630) => "virtio/tx-complete",
        (BootLocatorStage::User, 0x640) => "virtio/rx-complete",
        _ => "<unknown-checkpoint>",
    }
}

fn decode_stage(raw: u64) -> BootLocatorStage {
    match raw as u16 {
        2 => BootLocatorStage::Stage0,
        3 => BootLocatorStage::Limine,
        4 => BootLocatorStage::EarlyKernel,
        5 => BootLocatorStage::Paging,
        6 => BootLocatorStage::Traps,
        7 => BootLocatorStage::Smp,
        8 => BootLocatorStage::User,
        9 => BootLocatorStage::Fault,
        _ => BootLocatorStage::Reset,
    }
}

fn decode_kind(raw: u64) -> BootLocatorKind {
    match raw as u16 {
        2 => BootLocatorKind::Transition,
        3 => BootLocatorKind::Memory,
        4 => BootLocatorKind::Contract,
        5 => BootLocatorKind::Fault,
        _ => BootLocatorKind::Progress,
    }
}

fn decode_severity(raw: u64) -> BootLocatorSeverity {
    match raw as u16 {
        2 => BootLocatorSeverity::Info,
        3 => BootLocatorSeverity::Warn,
        4 => BootLocatorSeverity::Error,
        _ => BootLocatorSeverity::Trace,
    }
}

fn decode_payload_label(raw: u64) -> BootPayloadLabel {
    match raw as u16 {
        1 => BootPayloadLabel::Address,
        2 => BootPayloadLabel::Length,
        3 => BootPayloadLabel::Count,
        4 => BootPayloadLabel::Rip,
        5 => BootPayloadLabel::Value,
        6 => BootPayloadLabel::Status,
        _ => BootPayloadLabel::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static TEST_GUARD: Mutex<()> = Mutex::new(());

    fn lock_test_state() -> MutexGuard<'static, ()> {
        TEST_GUARD.lock().expect("boot locator test mutex poisoned")
    }

    #[test]
    fn locator_recent_orders_records_oldest_to_newest() {
        let _guard = lock_test_state();
        reset();
        event(
            BootLocatorStage::Stage0,
            BootLocatorKind::Progress,
            BootLocatorSeverity::Info,
            0x10,
            BootPayloadLabel::Status,
            1,
            BootPayloadLabel::None,
            0,
        );
        event(
            BootLocatorStage::Limine,
            BootLocatorKind::Transition,
            BootLocatorSeverity::Warn,
            0x20,
            BootPayloadLabel::Count,
            2,
            BootPayloadLabel::None,
            0,
        );
        event(
            BootLocatorStage::Paging,
            BootLocatorKind::Memory,
            BootLocatorSeverity::Error,
            0x30,
            BootPayloadLabel::Address,
            0x1000,
            BootPayloadLabel::Length,
            0x2000,
        );

        let recent = recent(2);
        assert_eq!(recent[0].sequence, 2);
        assert_eq!(recent[0].stage, BootLocatorStage::Limine);
        assert_eq!(recent[1].sequence, 3);
        assert_eq!(recent[1].stage, BootLocatorStage::Paging);
    }

    #[test]
    fn locator_recent_wraps_ring_without_losing_order() {
        let _guard = lock_test_state();
        reset();
        for index in 0..(LOCATOR_RING_CAPACITY as u64 + 4) {
            event(
                BootLocatorStage::Stage0,
                BootLocatorKind::Progress,
                BootLocatorSeverity::Info,
                0x100 + index,
                BootPayloadLabel::Value,
                index,
                BootPayloadLabel::None,
                0,
            );
        }

        let recent = recent(4);
        assert_eq!(recent[0].sequence, LOCATOR_RING_CAPACITY as u64 + 1);
        assert_eq!(recent[3].sequence, LOCATOR_RING_CAPACITY as u64 + 4);
        assert_eq!(
            recent[3].checkpoint,
            0x100 + LOCATOR_RING_CAPACITY as u64 + 3
        );
    }

    #[test]
    fn early_blackbox_records_without_atomic_locator_path() {
        let _guard = lock_test_state();
        early_reset();
        early_checkpoint(BootLocatorStage::Stage0, 0x10, 1, 2);
        early_event(
            BootLocatorStage::Stage0,
            BootLocatorKind::Transition,
            BootLocatorSeverity::Warn,
            0x20,
            BootPayloadLabel::Address,
            0x1000,
            BootPayloadLabel::Status,
            7,
        );

        let snapshot = early_snapshot();
        assert_eq!(snapshot.magic, EarlyBootBlackbox::MAGIC);
        assert_eq!(snapshot.cursor, 2);
        assert_eq!(snapshot.last.checkpoint, 0x20);
        assert_eq!(snapshot.records[0].checkpoint, 0x10);
        assert_eq!(snapshot.records[1].payload0, 0x1000);
    }

    #[test]
    fn early_recent_orders_records_oldest_to_newest() {
        let _guard = lock_test_state();
        early_reset();
        early_checkpoint(BootLocatorStage::Stage0, 0x10, 0, 0);
        early_checkpoint(BootLocatorStage::Stage0, 0x20, 0, 0);
        early_checkpoint(BootLocatorStage::Stage0, 0x30, 0, 0);

        let recent = early_recent(2);
        assert_eq!(recent[0].sequence, 2);
        assert_eq!(recent[0].checkpoint, 0x20);
        assert_eq!(recent[1].sequence, 3);
        assert_eq!(recent[1].checkpoint, 0x30);
    }
}
