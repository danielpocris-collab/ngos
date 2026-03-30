use core::cell::UnsafeCell;
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::boot_locator::{
    BootLocatorKind, BootLocatorRecord, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};

const REBOOT_TRACE_MAGIC: u64 = 0x4e47_4f53_5254_5243;
const REBOOT_TRACE_VERSION: u32 = 1;
const REBOOT_TRACE_PHYS_BASE: u64 = 0x0009_f000;
const REBOOT_TRACE_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebootTraceSnapshot {
    pub valid: bool,
    pub boot_generation: u64,
    pub completed_cleanly: bool,
    pub last_record: BootLocatorRecord,
    pub records: [BootLocatorRecord; REBOOT_TRACE_CAPACITY],
}

impl RebootTraceSnapshot {
    pub const EMPTY: Self = Self {
        valid: false,
        boot_generation: 0,
        completed_cleanly: false,
        last_record: BootLocatorRecord::EMPTY,
        records: [BootLocatorRecord::EMPTY; REBOOT_TRACE_CAPACITY],
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistRecord {
    sequence: u64,
    stage: u16,
    kind: u16,
    severity: u16,
    payload0_label: u16,
    payload1_label: u16,
    _reserved0: u16,
    checkpoint: u64,
    payload0: u64,
    payload1: u64,
}

impl PersistRecord {
    const EMPTY: Self = Self {
        sequence: 0,
        stage: 0,
        kind: 0,
        severity: 0,
        payload0_label: 0,
        payload1_label: 0,
        _reserved0: 0,
        checkpoint: 0,
        payload0: 0,
        payload1: 0,
    };

    fn from_locator(record: BootLocatorRecord) -> Self {
        Self {
            sequence: record.sequence,
            stage: record.stage as u16,
            kind: record.kind as u16,
            severity: record.severity as u16,
            payload0_label: record.payload0_label as u16,
            payload1_label: record.payload1_label as u16,
            _reserved0: 0,
            checkpoint: record.checkpoint,
            payload0: record.payload0,
            payload1: record.payload1,
        }
    }

    fn into_locator(self) -> BootLocatorRecord {
        BootLocatorRecord {
            sequence: self.sequence,
            stage: decode_stage(self.stage),
            kind: decode_kind(self.kind),
            severity: decode_severity(self.severity),
            checkpoint: self.checkpoint,
            payload0_label: decode_payload_label(self.payload0_label),
            payload0: self.payload0,
            payload1_label: decode_payload_label(self.payload1_label),
            payload1: self.payload1,
        }
    }
}

#[repr(C)]
struct PersistTraceBlock {
    magic: u64,
    version: u32,
    dirty: u32,
    boot_generation: u64,
    cursor: u64,
    last_record: PersistRecord,
    records: [PersistRecord; REBOOT_TRACE_CAPACITY],
}

struct RebootTraceStorage {
    block_ptr: UnsafeCell<*mut PersistTraceBlock>,
    previous: UnsafeCell<RebootTraceSnapshot>,
}

unsafe impl Sync for RebootTraceStorage {}

static REBOOT_TRACE: RebootTraceStorage = RebootTraceStorage {
    block_ptr: UnsafeCell::new(ptr::null_mut()),
    previous: UnsafeCell::new(RebootTraceSnapshot::EMPTY),
};
static REBOOT_TRACE_READY: AtomicBool = AtomicBool::new(false);

pub fn init(physical_memory_offset: u64) {
    let Some(virtual_base) = physical_memory_offset.checked_add(REBOOT_TRACE_PHYS_BASE) else {
        return;
    };
    let block_ptr = virtual_base as *mut PersistTraceBlock;
    unsafe {
        let previous = read_snapshot(block_ptr);
        *REBOOT_TRACE.previous.get() = previous;
        ptr::write(
            block_ptr,
            PersistTraceBlock {
                magic: REBOOT_TRACE_MAGIC,
                version: REBOOT_TRACE_VERSION,
                dirty: 1,
                boot_generation: previous.boot_generation.saturating_add(1),
                cursor: 0,
                last_record: PersistRecord::EMPTY,
                records: [PersistRecord::EMPTY; REBOOT_TRACE_CAPACITY],
            },
        );
        *REBOOT_TRACE.block_ptr.get() = block_ptr;
    }
    REBOOT_TRACE_READY.store(true, Ordering::Release);
}

pub fn previous_snapshot() -> RebootTraceSnapshot {
    unsafe { *REBOOT_TRACE.previous.get() }
}

pub fn record_locator(record: BootLocatorRecord) {
    if !REBOOT_TRACE_READY.load(Ordering::Acquire) {
        return;
    }
    unsafe {
        let block_ptr = *REBOOT_TRACE.block_ptr.get();
        if block_ptr.is_null() {
            return;
        }
        let block = &mut *block_ptr;
        if block.magic != REBOOT_TRACE_MAGIC || block.version != REBOOT_TRACE_VERSION {
            return;
        }
        let next = block.cursor.saturating_add(1);
        let slot = (next as usize - 1) % REBOOT_TRACE_CAPACITY;
        let persist = PersistRecord::from_locator(record);
        ptr::write_volatile(&mut block.records[slot], persist);
        ptr::write_volatile(&mut block.last_record, persist);
        ptr::write_volatile(&mut block.cursor, next);
        ptr::write_volatile(&mut block.dirty, 1);
    }
}

pub fn mark_clean_shutdown() {
    if !REBOOT_TRACE_READY.load(Ordering::Acquire) {
        return;
    }
    unsafe {
        let block_ptr = *REBOOT_TRACE.block_ptr.get();
        if block_ptr.is_null() {
            return;
        }
        let block = &mut *block_ptr;
        if block.magic == REBOOT_TRACE_MAGIC && block.version == REBOOT_TRACE_VERSION {
            ptr::write_volatile(&mut block.dirty, 0);
        }
    }
}

pub const fn physical_base() -> u64 {
    REBOOT_TRACE_PHYS_BASE
}

#[allow(dead_code)]
pub const fn span_bytes() -> usize {
    size_of::<PersistTraceBlock>()
}

unsafe fn read_snapshot(block_ptr: *const PersistTraceBlock) -> RebootTraceSnapshot {
    let block = unsafe { &*block_ptr };
    let magic = unsafe { ptr::read_volatile(&block.magic) };
    let version = unsafe { ptr::read_volatile(&block.version) };
    if magic != REBOOT_TRACE_MAGIC || version != REBOOT_TRACE_VERSION {
        return RebootTraceSnapshot::EMPTY;
    }

    let cursor = unsafe { ptr::read_volatile(&block.cursor) };
    let boot_generation = unsafe { ptr::read_volatile(&block.boot_generation) };
    let dirty = unsafe { ptr::read_volatile(&block.dirty) != 0 };
    let persisted_records = unsafe { ptr::read_volatile(&block.records) };
    let persisted_last = unsafe { ptr::read_volatile(&block.last_record) };
    let mut records = [BootLocatorRecord::EMPTY; REBOOT_TRACE_CAPACITY];
    let available = (cursor as usize).min(REBOOT_TRACE_CAPACITY);
    if available != 0 {
        let start = cursor.saturating_sub(available as u64).saturating_add(1);
        let mut out = 0usize;
        let mut sequence = start;
        while sequence <= cursor && out < REBOOT_TRACE_CAPACITY {
            let slot = (sequence as usize - 1) % REBOOT_TRACE_CAPACITY;
            let record = persisted_records[slot];
            if record.sequence == sequence {
                records[out] = record.into_locator();
                out += 1;
            }
            sequence = sequence.saturating_add(1);
        }
    }

    RebootTraceSnapshot {
        valid: true,
        boot_generation,
        completed_cleanly: !dirty,
        last_record: persisted_last.into_locator(),
        records,
    }
}

const fn decode_stage(raw: u16) -> BootLocatorStage {
    match raw {
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

const fn decode_kind(raw: u16) -> BootLocatorKind {
    match raw {
        2 => BootLocatorKind::Transition,
        3 => BootLocatorKind::Memory,
        4 => BootLocatorKind::Contract,
        5 => BootLocatorKind::Fault,
        _ => BootLocatorKind::Progress,
    }
}

const fn decode_severity(raw: u16) -> BootLocatorSeverity {
    match raw {
        2 => BootLocatorSeverity::Info,
        3 => BootLocatorSeverity::Warn,
        4 => BootLocatorSeverity::Error,
        _ => BootLocatorSeverity::Trace,
    }
}

const fn decode_payload_label(raw: u16) -> BootPayloadLabel {
    match raw {
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

    #[test]
    fn snapshot_reads_dirty_previous_boot() {
        let mut block = PersistTraceBlock {
            magic: REBOOT_TRACE_MAGIC,
            version: REBOOT_TRACE_VERSION,
            dirty: 1,
            boot_generation: 7,
            cursor: 2,
            last_record: PersistRecord::from_locator(BootLocatorRecord {
                sequence: 2,
                stage: BootLocatorStage::Paging,
                kind: BootLocatorKind::Progress,
                severity: BootLocatorSeverity::Info,
                checkpoint: 0x340,
                payload0_label: BootPayloadLabel::Status,
                payload0: 1,
                payload1_label: BootPayloadLabel::None,
                payload1: 0,
            }),
            records: [PersistRecord::EMPTY; REBOOT_TRACE_CAPACITY],
        };
        block.records[0] = PersistRecord::from_locator(BootLocatorRecord {
            sequence: 1,
            stage: BootLocatorStage::EarlyKernel,
            kind: BootLocatorKind::Transition,
            severity: BootLocatorSeverity::Info,
            checkpoint: 0x100,
            payload0_label: BootPayloadLabel::None,
            payload0: 0,
            payload1_label: BootPayloadLabel::None,
            payload1: 0,
        });
        block.records[1] = block.last_record;

        let snapshot = unsafe { read_snapshot(&block) };
        assert!(snapshot.valid);
        assert_eq!(snapshot.boot_generation, 7);
        assert!(!snapshot.completed_cleanly);
        assert_eq!(snapshot.last_record.checkpoint, 0x340);
        assert_eq!(snapshot.records[0].checkpoint, 0x100);
        assert_eq!(snapshot.records[1].checkpoint, 0x340);
    }
}
