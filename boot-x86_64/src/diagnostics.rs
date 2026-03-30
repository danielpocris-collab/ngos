#![cfg_attr(not(test), allow(dead_code))]
#![allow(
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::explicit_counter_loop,
    clippy::manual_checked_ops,
    clippy::manual_contains,
    clippy::manual_swap,
    clippy::needless_lifetimes,
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    clippy::unnecessary_to_owned,
    clippy::write_with_newline
)]

use alloc::vec::Vec;
#[cfg(target_os = "none")]
use core::arch::asm;
use core::cell::UnsafeCell;
use core::fmt::{self, Write};
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use platform_x86_64::ExceptionFrame;

use crate::boot_locator::{self, BootLocatorRecord, BootPayloadLabel, EarlyBootSnapshot};
use crate::serial;
use crate::user_runtime_status::FirstUserProcessStatus;

mod chronoscope_graph;
mod chronoscope_query;
mod chronoscope_render;
mod chronoscope_replay;
mod chronoscope_types;
mod crash_history;
mod crash_types;
mod developer_guidance;
mod export_bundle;
mod failure_analysis;

#[cfg(test)]
fn ensure_test_thread_lock() {
    use std::cell::RefCell;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    thread_local! {
        static GUARD: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
    }

    GUARD.with(|slot| {
        if slot.borrow().is_none() {
            let guard = LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *slot.borrow_mut() = Some(guard);
        }
    });
}
mod runtime_summaries;

pub use chronoscope_graph::*;
use chronoscope_render::*;
use chronoscope_replay::*;
pub use chronoscope_types::*;
pub use crash_history::*;
pub use crash_types::*;
use developer_guidance::*;
use export_bundle::*;
use failure_analysis::*;
use runtime_summaries::*;

const TRACE_CAPACITY: usize = 128;
const MAX_TRACE_CPUS: usize = 16;
const INVALID_APIC_ID: u64 = u64::MAX;
const TRACE_SEQUENCE_CPU_BITS: u32 = 4;
const TRACE_SEQUENCE_CPU_MASK: u64 = (1u64 << TRACE_SEQUENCE_CPU_BITS) - 1;
const GUARD_CAPACITY: usize = 32;
const WATCH_CAPACITY: usize = 32;
const CRASH_TRACE_TAIL: usize = 16;
const VIOLATION_TAIL: usize = 8;
const FAILURE_HISTORY_CAPACITY: usize = 8;
const FAILURE_PATTERN_CAPACITY: usize = 16;
const FOCUSED_TRACE_HISTORY: usize = 8;
const SUSPECT_LIMIT: usize = 3;
const MEMORY_LINEAGE_CAPACITY: usize = 64;
const MEMORY_LINEAGE_HINT_CAPACITY: usize = 64;
const VIOLATION_HINT_CAPACITY: usize = 32;
const GUARD_ID_TAG: u64 = 0x4700_0000_0000_0000;
const WATCH_ID_TAG: u64 = 0x5700_0000_0000_0000;
const RUNTIME_EVENT_CAPACITY: usize = 64;
const RUNTIME_EVENT_EXPORT_LIMIT: usize = MAX_TRACE_CPUS * 8;
const CHRONOSCOPE_ANOMALY_LIMIT: usize = 16;
const CHRONOSCOPE_ESCALATION_LIMIT: usize = 8;
const CHRONOSCOPE_CAPTURE_WINDOW_LIMIT: usize = 8;
const CHRONOSCOPE_CANDIDATE_LIMIT: usize = 4;
const CHRONOSCOPE_ADAPTIVE_TRANSITION_LIMIT: usize = 16;
const CHRONOSCOPE_SCHEMA_VERSION: u16 = 8;
const CHRONOSCOPE_MAX_QUERY_ROWS: u16 = 64;
const CHRONOSCOPE_MAX_REPORT_PATH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallFrontierRecord {
    pub valid: bool,
    pub sequence: u64,
    pub syscall_id: u64,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub stage: u16,
    pub cpu_slot: u16,
    pub apic_id: u32,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub result_ok: bool,
    pub errno: u16,
}

impl SyscallFrontierRecord {
    const EMPTY: Self = Self {
        valid: false,
        sequence: 0,
        syscall_id: 0,
        arg0: 0,
        arg1: 0,
        arg2: 0,
        stage: 0,
        cpu_slot: 0,
        apic_id: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        result_ok: false,
        errno: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionTraceRecord {
    pub valid: bool,
    pub sequence: u64,
    pub function_id: u64,
    pub checkpoint_id: u64,
    pub step_id: u64,
    pub object0: u64,
    pub object1: u64,
    pub stage: u16,
    pub cpu_slot: u16,
    pub apic_id: u32,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub result_ok: bool,
    pub errno: u16,
}

impl FunctionTraceRecord {
    const EMPTY: Self = Self {
        valid: false,
        sequence: 0,
        function_id: 0,
        checkpoint_id: 0,
        step_id: 0,
        object0: 0,
        object1: 0,
        stage: 0,
        cpu_slot: 0,
        apic_id: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        result_ok: false,
        errno: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DiagnosticsMode {
    Off = 0,
    Light = 1,
    Targeted = 2,
    CrashFollowup = 3,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DiagnosticsPath {
    None = 0,
    Syscall = 1,
    Block = 2,
    Irq = 3,
    Completion = 4,
    Fault = 5,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum GuardKind {
    Stack = 1,
    RequestBuffer = 2,
    CompletionBuffer = 3,
    QueueMetadata = 4,
    Ring = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum WatchKind {
    Read = 1,
    Write = 2,
    Touch = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ViolationKind {
    Guard = 1,
    Watch = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MemoryOverlapClass {
    None = 0,
    Exact = 1,
    Interior = 2,
    Prefix = 3,
    Suffix = 4,
    Span = 5,
    LeftRedZone = 6,
    RightRedZone = 7,
}

const MEMORY_SUSPECT_UNDERRUN: u16 = 1 << 0;
const MEMORY_SUSPECT_OVERRUN: u16 = 1 << 1;
const MEMORY_SUSPECT_INTERIOR: u16 = 1 << 2;
const MEMORY_SUSPECT_EXACT: u16 = 1 << 3;
const MEMORY_SUSPECT_REPEATED: u16 = 1 << 4;
const MEMORY_SUSPECT_CROSS_REQUEST: u16 = 1 << 5;
const MEMORY_SUSPECT_CROSS_COMPLETION: u16 = 1 << 6;
const MEMORY_SUSPECT_WIDE_SPAN: u16 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayCorrelationIds {
    pub sequence: u64,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
}

impl ReplayCorrelationIds {
    pub const EMPTY: Self = Self {
        sequence: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChronoscopeEventId(pub u64);

impl ChronoscopeEventId {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChronoscopeBufferId(pub u16);

impl ChronoscopeBufferId {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeCaptureLevel {
    Minimal = 1,
    Standard = 2,
    Deep = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeConfig {
    pub max_query_rows: u16,
    pub max_report_path_len: u16,
    pub panic_export_limit: u16,
    pub anomaly_threshold: u16,
    pub escalation_budget: u16,
}

impl ChronoscopeConfig {
    pub const DEFAULT: Self = Self {
        max_query_rows: CHRONOSCOPE_MAX_QUERY_ROWS,
        max_report_path_len: CHRONOSCOPE_MAX_REPORT_PATH as u16,
        panic_export_limit: 32,
        anomaly_threshold: 3,
        escalation_budget: 24,
    };

    pub fn validate(&self) -> bool {
        self.max_query_rows != 0
            && self.max_query_rows <= 256
            && self.max_report_path_len != 0
            && self.max_report_path_len <= 64
            && self.panic_export_limit != 0
            && self.panic_export_limit <= 128
            && self.anomaly_threshold != 0
            && self.anomaly_threshold <= 64
            && self.escalation_budget != 0
            && self.escalation_budget <= 256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeRuntimeEventKind {
    SchedulerSwitch = 1,
    SchedulerWake = 2,
    SchedulerBlock = 3,
    RequestStart = 4,
    RequestComplete = 5,
    IrqEnter = 6,
    IrqExit = 7,
    ContractTransition = 8,
    ResourceClaim = 9,
    ResourceRelease = 10,
    ResourceWait = 11,
    ViolationObserved = 12,
    SuspectPromoted = 13,
    FocusedTraceMarker = 14,
    FaultMarker = 15,
    DivergenceHint = 16,
    CapabilityDerive = 17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeRuntimePayload {
    None = 0,
    Scheduler {
        from_tid: u32,
        to_tid: u32,
        reason: u16,
    } = 1,
    Request {
        op: u16,
        status: u16,
        device_id: u32,
    } = 2,
    Irq {
        vector: u16,
        phase: u16,
        reserved: u32,
    } = 3,
    Contract {
        contract_id: u32,
        from_state: u16,
        to_state: u16,
    } = 4,
    Resource {
        resource_id: u32,
        actor: u16,
        state: u16,
    } = 5,
    Violation {
        violation_kind: u16,
        descriptor_kind: u16,
        score: u16,
        flags: u16,
    } = 6,
    Suspect {
        reason_code: u16,
        event_kind: u16,
        score: u16,
        reserved: u16,
    } = 7,
    Fault {
        vector: u16,
        stage: u16,
        reserved: u32,
    } = 8,
    Divergence {
        other_cpu: u16,
        divergence_stage: u16,
        divergence_path: u16,
        reserved: u16,
    } = 9,
    Capability {
        parent_lo: u16,
        parent_hi: u16,
        rights_hint: u16,
        reserved: u16,
    } = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeRuntimeEventRecord {
    pub valid: bool,
    pub event_id: ChronoscopeEventId,
    pub buffer_id: ChronoscopeBufferId,
    pub local_sequence: u64,
    pub core_id: u16,
    pub stage: u16,
    pub uptime_us: u64,
    pub kind: ChronoscopeRuntimeEventKind,
    pub correlation: ReplayCorrelationIds,
    pub object_key: u64,
    pub capability_id: CapabilityId,
    pub parent_capability_id: CapabilityId,
    pub rights_mask: u64,
    pub causal_parent: ChronoscopeEventId,
    pub flags: u16,
    pub payload: ChronoscopeRuntimePayload,
}

impl ChronoscopeRuntimeEventRecord {
    const EMPTY: Self = Self {
        valid: false,
        event_id: ChronoscopeEventId::NONE,
        buffer_id: ChronoscopeBufferId::NONE,
        local_sequence: 0,
        core_id: 0,
        stage: 0,
        uptime_us: 0,
        kind: ChronoscopeRuntimeEventKind::FocusedTraceMarker,
        correlation: ReplayCorrelationIds::EMPTY,
        object_key: 0,
        capability_id: CapabilityId::NONE,
        parent_capability_id: CapabilityId::NONE,
        rights_mask: 0,
        causal_parent: ChronoscopeEventId::NONE,
        flags: 0,
        payload: ChronoscopeRuntimePayload::None,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeRuntimeCoreSummary {
    pub valid: bool,
    pub buffer_id: ChronoscopeBufferId,
    pub core_id: u16,
    pub available_events: u16,
    pub overwritten_events: u64,
    pub oldest_local_sequence: u64,
    pub newest_local_sequence: u64,
    pub partial: bool,
    pub high_water_mark: u16,
}

impl ChronoscopeRuntimeCoreSummary {
    const EMPTY: Self = Self {
        valid: false,
        buffer_id: ChronoscopeBufferId::NONE,
        core_id: 0,
        available_events: 0,
        overwritten_events: 0,
        oldest_local_sequence: 0,
        newest_local_sequence: 0,
        partial: false,
        high_water_mark: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeHotPathCostClass {
    AppendOnly = 1,
    AppendEscalationBranch = 2,
    AppendCheckpointBranch = 3,
    AppendOverwritePath = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopePartialReason {
    None = 0,
    RingOverwrite = 1,
    MissingCrossCoreHistory = 2,
    DeepCaptureDrop = 3,
    PanicTruncation = 4,
    ReplayIncomplete = 5,
    DiffIncomplete = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeCompletenessFlags {
    pub ring_overwrite: bool,
    pub missing_cross_core: bool,
    pub deep_capture_drop: bool,
    pub panic_truncated: bool,
    pub replay_incomplete: bool,
    pub diff_incomplete: bool,
}

impl ChronoscopeCompletenessFlags {
    pub const EMPTY: Self = Self {
        ring_overwrite: false,
        missing_cross_core: false,
        deep_capture_drop: false,
        panic_truncated: false,
        replay_incomplete: false,
        diff_incomplete: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeHistoryIntegrity {
    pub complete: bool,
    pub primary_reason: ChronoscopePartialReason,
    pub flags: ChronoscopeCompletenessFlags,
}

impl ChronoscopeHistoryIntegrity {
    pub const COMPLETE: Self = Self {
        complete: true,
        primary_reason: ChronoscopePartialReason::None,
        flags: ChronoscopeCompletenessFlags::EMPTY,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeCorePerfCounters {
    pub events_emitted: u64,
    pub events_overwritten: u64,
    pub high_water_mark: u16,
    pub append_only_count: u64,
    pub escalation_branch_count: u64,
    pub checkpoint_branch_count: u64,
    pub overwrite_path_count: u64,
}

impl ChronoscopeCorePerfCounters {
    const EMPTY: Self = Self {
        events_emitted: 0,
        events_overwritten: 0,
        high_water_mark: 0,
        append_only_count: 0,
        escalation_branch_count: 0,
        checkpoint_branch_count: 0,
        overwrite_path_count: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeCaptureStats {
    pub average_events_per_capture: u16,
    pub peak_events_per_capture: u16,
    pub partial_captures: u16,
}

impl ChronoscopeCaptureStats {
    const EMPTY: Self = Self {
        average_events_per_capture: 0,
        peak_events_per_capture: 0,
        partial_captures: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopePerfCounters {
    pub schema_version: u16,
    pub events_emitted: u64,
    pub events_dropped_or_overwritten: u64,
    pub replay_executions: u64,
    pub replay_steps: u64,
    pub query_executions: u64,
    pub escalations: u64,
    pub checkpoints: u64,
    pub lineage_records: u64,
    pub diff_executions: u64,
    pub captures: ChronoscopeCaptureStats,
    pub per_core: [ChronoscopeCorePerfCounters; MAX_TRACE_CPUS],
}

impl ChronoscopePerfCounters {
    const EMPTY: Self = Self {
        schema_version: CHRONOSCOPE_SCHEMA_VERSION,
        events_emitted: 0,
        events_dropped_or_overwritten: 0,
        replay_executions: 0,
        replay_steps: 0,
        query_executions: 0,
        escalations: 0,
        checkpoints: 0,
        lineage_records: 0,
        diff_executions: 0,
        captures: ChronoscopeCaptureStats::EMPTY,
        per_core: [ChronoscopeCorePerfCounters::EMPTY; MAX_TRACE_CPUS],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeTrustSurface {
    pub schema_version: u16,
    pub completeness: ChronoscopeHistoryIntegrity,
    pub capture_level: ChronoscopeCaptureLevel,
    pub replay_partial: bool,
    pub explain_degraded: bool,
    pub responsibility_partial: bool,
}

impl ChronoscopeTrustSurface {
    const EMPTY: Self = Self {
        schema_version: CHRONOSCOPE_SCHEMA_VERSION,
        completeness: ChronoscopeHistoryIntegrity::COMPLETE,
        capture_level: ChronoscopeCaptureLevel::Minimal,
        replay_partial: false,
        explain_degraded: false,
        responsibility_partial: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopePanicMode {
    MinimalPanicCapture = 1,
    PanicCaptureDegraded = 2,
    FullCaptureUnavailable = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeRuntimeEventWindow {
    pub valid: bool,
    pub partial: bool,
    pub total_events: u16,
    pub integrity: ChronoscopeHistoryIntegrity,
    pub events: [ChronoscopeRuntimeEventRecord; RUNTIME_EVENT_EXPORT_LIMIT],
    pub per_core: [ChronoscopeRuntimeCoreSummary; MAX_TRACE_CPUS],
}

impl ChronoscopeRuntimeEventWindow {
    const EMPTY: Self = Self {
        valid: false,
        partial: false,
        total_events: 0,
        integrity: ChronoscopeHistoryIntegrity::COMPLETE,
        events: [ChronoscopeRuntimeEventRecord::EMPTY; RUNTIME_EVENT_EXPORT_LIMIT],
        per_core: [ChronoscopeRuntimeCoreSummary::EMPTY; MAX_TRACE_CPUS],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardDescriptor {
    pub active: bool,
    pub id: u64,
    pub kind: GuardKind,
    pub registration_sequence: u64,
    pub stage: u16,
    pub path: DiagnosticsPath,
    pub request_id: u64,
    pub completion_id: u64,
    pub address: u64,
    pub length: u64,
    pub red_zone: u64,
    pub canary: u64,
}

impl GuardDescriptor {
    const EMPTY: Self = Self {
        active: false,
        id: 0,
        kind: GuardKind::RequestBuffer,
        registration_sequence: 0,
        stage: 0,
        path: DiagnosticsPath::None,
        request_id: 0,
        completion_id: 0,
        address: 0,
        length: 0,
        red_zone: 0,
        canary: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchDescriptor {
    pub active: bool,
    pub id: u64,
    pub kind: WatchKind,
    pub registration_sequence: u64,
    pub stage: u16,
    pub path: DiagnosticsPath,
    pub request_id: u64,
    pub completion_id: u64,
    pub address: u64,
    pub length: u64,
}

impl WatchDescriptor {
    const EMPTY: Self = Self {
        active: false,
        id: 0,
        kind: WatchKind::Touch,
        registration_sequence: 0,
        stage: 0,
        path: DiagnosticsPath::None,
        request_id: 0,
        completion_id: 0,
        address: 0,
        length: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViolationRecord {
    pub sequence: u64,
    pub kind: ViolationKind,
    pub descriptor_id: u64,
    pub descriptor_kind: u16,
    pub address: u64,
    pub length: u64,
    pub descriptor_address: u64,
    pub descriptor_length: u64,
    pub overlap: MemoryOverlapClass,
    pub suspicion_flags: u16,
    pub relative_start: i64,
    pub relative_end: i64,
    pub stage: u16,
    pub path: DiagnosticsPath,
    pub request_id: u64,
    pub completion_id: u64,
    pub cpu_slot: u16,
    pub apic_id: u32,
}

impl ViolationRecord {
    const EMPTY: Self = Self {
        sequence: 0,
        kind: ViolationKind::Guard,
        descriptor_id: 0,
        descriptor_kind: 0,
        address: 0,
        length: 0,
        descriptor_address: 0,
        descriptor_length: 0,
        overlap: MemoryOverlapClass::None,
        suspicion_flags: 0,
        relative_start: 0,
        relative_end: 0,
        stage: 0,
        path: DiagnosticsPath::None,
        request_id: 0,
        completion_id: 0,
        cpu_slot: 0,
        apic_id: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MemoryLineageKind {
    Snapshot = 1,
    Write = 2,
    Copy = 3,
    Zero = 4,
    Dma = 5,
    Free = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLineageEntry {
    pub valid: bool,
    pub sequence: u64,
    pub version_id: u64,
    pub parent_version_id: u64,
    pub address_space_id: u64,
    pub base: u64,
    pub len: u64,
    pub object_id: u64,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub cpu_slot: u16,
    pub stage: u16,
    pub kind: MemoryLineageKind,
    pub bytes_changed: u32,
    pub digest: u64,
}

impl MemoryLineageEntry {
    const EMPTY: Self = Self {
        valid: false,
        sequence: 0,
        version_id: 0,
        parent_version_id: 0,
        address_space_id: 0,
        base: 0,
        len: 0,
        object_id: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        cpu_slot: 0,
        stage: 0,
        kind: MemoryLineageKind::Snapshot,
        bytes_changed: 0,
        digest: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemoryLineageHint {
    valid: bool,
    address_space_id: u64,
    base: u64,
    len: u64,
    latest_version_id: u64,
}

impl MemoryLineageHint {
    const EMPTY: Self = Self {
        valid: false,
        address_space_id: 0,
        base: 0,
        len: 0,
        latest_version_id: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ViolationHint {
    valid: bool,
    descriptor_id: u64,
    kind: ViolationKind,
    request_id: u64,
    completion_id: u64,
}

impl ViolationHint {
    const EMPTY: Self = Self {
        valid: false,
        descriptor_id: 0,
        kind: ViolationKind::Guard,
        request_id: 0,
        completion_id: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestWindowState {
    pub valid: bool,
    pub syscall_id: u64,
    pub fd: u64,
    pub request_op: u64,
    pub device_id: u64,
    pub completion_state: u64,
    pub path: DiagnosticsPath,
    pub request_id: u64,
    pub completion_id: u64,
}

impl RequestWindowState {
    const EMPTY: Self = Self {
        valid: false,
        syscall_id: 0,
        fd: 0,
        request_op: 0,
        device_id: 0,
        completion_state: 0,
        path: DiagnosticsPath::None,
        request_id: 0,
        completion_id: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReprobePolicyState {
    pub mode: DiagnosticsMode,
    pub target_path: DiagnosticsPath,
    pub target_stage: u16,
    pub target_checkpoint: u64,
    pub escalation: u8,
    pub crash_count: u32,
}

impl ReprobePolicyState {
    const EMPTY: Self = Self {
        mode: DiagnosticsMode::Light,
        target_path: DiagnosticsPath::None,
        target_stage: 0,
        target_checkpoint: 0,
        escalation: 0,
        crash_count: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticReasonAggregate {
    pub write_syscall_reject_fault: u16,
    pub write_syscall_reject_guard: u16,
    pub write_syscall_reject_watch: u16,
    pub submit_device_request_reject_fault: u16,
    pub submit_device_request_reject_guard: u16,
    pub submit_device_request_reject_watch: u16,
    pub completion_publish_reject_fault: u16,
    pub completion_publish_reject_guard: u16,
    pub completion_publish_reject_watch: u16,
    pub completion_read_reject_fault: u16,
    pub completion_read_reject_guard: u16,
    pub completion_read_reject_watch: u16,
    pub reprobe_escalation_reason_fault: u16,
    pub reprobe_escalation_reason_watch_guard: u16,
}

impl SemanticReasonAggregate {
    const EMPTY: Self = Self {
        write_syscall_reject_fault: 0,
        write_syscall_reject_guard: 0,
        write_syscall_reject_watch: 0,
        submit_device_request_reject_fault: 0,
        submit_device_request_reject_guard: 0,
        submit_device_request_reject_watch: 0,
        completion_publish_reject_fault: 0,
        completion_publish_reject_guard: 0,
        completion_publish_reject_watch: 0,
        completion_read_reject_fault: 0,
        completion_read_reject_guard: 0,
        completion_read_reject_watch: 0,
        reprobe_escalation_reason_fault: 0,
        reprobe_escalation_reason_watch_guard: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureSignature {
    pub valid: bool,
    pub id: u64,
    pub path: DiagnosticsPath,
    pub stage: u16,
    pub fault_vector: u64,
    pub dominant_violation: ViolationKind,
    pub last_good_kind: TraceKind,
    pub last_good_stage: u16,
    pub first_bad_kind: TraceKind,
    pub first_bad_stage: u16,
    pub divergence_kind: TraceKind,
    pub divergence_stage: u16,
    pub chain_shape: u16,
}

impl FailureSignature {
    const EMPTY: Self = Self {
        valid: false,
        id: 0,
        path: DiagnosticsPath::None,
        stage: 0,
        fault_vector: 0,
        dominant_violation: ViolationKind::Guard,
        last_good_kind: TraceKind::BootStage,
        last_good_stage: 0,
        first_bad_kind: TraceKind::BootStage,
        first_bad_stage: 0,
        divergence_kind: TraceKind::BootStage,
        divergence_stage: 0,
        chain_shape: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuspectPoint {
    pub valid: bool,
    pub score: u16,
    pub stage: u16,
    pub cpu_slot: u16,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub event_sequence: u64,
    pub event_kind: TraceKind,
    pub reason_code: u16,
}

impl SuspectPoint {
    const EMPTY: Self = Self {
        valid: false,
        score: 0,
        stage: 0,
        cpu_slot: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        event_sequence: 0,
        event_kind: TraceKind::BootStage,
        reason_code: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeDiffSummary {
    pub common_nodes: u16,
    pub new_nodes: u16,
    pub missing_nodes: u16,
    pub changed_paths: u16,
    pub common_checkpoints: u16,
    pub new_checkpoints: u16,
    pub missing_checkpoints: u16,
    pub common_lineage: u16,
    pub new_lineage: u16,
    pub missing_lineage: u16,
    pub changed_last_writer: u16,
    pub changed_capability_lineage: u16,
    pub changed_propagation: u16,
    pub changed_responsibility: u16,
    pub changed_anomalies: u16,
    pub changed_escalations: u16,
    pub changed_capture_windows: u16,
    pub changed_candidates: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TraceKind {
    BootStage = 1,
    UserMarker = 2,
    UserStatus = 3,
    Fault = 4,
    Irq = 5,
    Memory = 6,
    Device = 7,
    Transition = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TraceChannel {
    Boot = 1,
    User = 2,
    Fault = 3,
    Irq = 4,
    Memory = 5,
    Device = 6,
    Transition = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BootTraceStage {
    Stage0 = 1,
    EarlyKernelMain = 2,
    PhysAllocReady = 3,
    PagingReady = 4,
    TrapsReady = 5,
    SyscallReady = 6,
    UserLaunchReady = 7,
    EnterUserMode = 8,
    SmpTopologyReady = 9,
    SmpDispatchReady = 10,
    SecondaryCpuOnline = 11,
    DeviceBringup = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceRecord {
    pub sequence: u64,
    pub uptime_us: u64,
    pub apic_id: u32,
    pub cpu_slot: u16,
    pub kind: TraceKind,
    pub channel: TraceChannel,
    pub stage: u16,
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: u64,
}

impl TraceRecord {
    const EMPTY: Self = Self {
        sequence: 0,
        uptime_us: 0,
        apic_id: 0,
        cpu_slot: 0,
        kind: TraceKind::BootStage,
        channel: TraceChannel::Boot,
        stage: 0,
        a: 0,
        b: 0,
        c: 0,
        d: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlRegisterSnapshot {
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub efer: u64,
}

impl ControlRegisterSnapshot {
    #[allow(dead_code)]
    const EMPTY: Self = Self {
        cr0: 0,
        cr2: 0,
        cr3: 0,
        cr4: 0,
        efer: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultRecord {
    pub valid: bool,
    pub uptime_us: u64,
    pub apic_id: u32,
    pub cpu_slot: u16,
    pub stage: u16,
    pub vector: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub error_code: u64,
    pub cr2: u64,
    pub cr0: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub efer: u64,
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
    pub locator_sequence: u64,
    pub locator_stage: u16,
    pub locator_checkpoint: u64,
    pub locator_payload0: u64,
    pub locator_payload1: u64,
}

impl FaultRecord {
    const EMPTY: Self = Self {
        valid: false,
        uptime_us: 0,
        apic_id: 0,
        cpu_slot: 0,
        stage: 0,
        vector: 0,
        rip: 0,
        cs: 0,
        rflags: 0,
        error_code: 0,
        cr2: 0,
        cr0: 0,
        cr3: 0,
        cr4: 0,
        efer: 0,
        rax: 0,
        rbx: 0,
        rcx: 0,
        rdx: 0,
        rsi: 0,
        rdi: 0,
        rbp: 0,
        r8: 0,
        r9: 0,
        r10: 0,
        r11: 0,
        r12: 0,
        r13: 0,
        r14: 0,
        r15: 0,
        locator_sequence: 0,
        locator_stage: 0,
        locator_checkpoint: 0,
        locator_payload0: 0,
        locator_payload1: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolLandmark {
    pub name: &'static str,
    pub base: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSymbol {
    pub name: &'static str,
    pub base: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CpuTraceContext {
    cpu_slot: usize,
    apic_id: u32,
    stage: u16,
}

struct TraceStorage {
    records: UnsafeCell<[[TraceRecord; TRACE_CAPACITY]; MAX_TRACE_CPUS]>,
    runtime_events:
        UnsafeCell<[[ChronoscopeRuntimeEventRecord; RUNTIME_EVENT_CAPACITY]; MAX_TRACE_CPUS]>,
    last_faults: UnsafeCell<[FaultRecord; MAX_TRACE_CPUS]>,
    last_user_status: UnsafeCell<MaybeUninit<FirstUserProcessStatus>>,
    last_user_status_valid: AtomicBool,
    guards: UnsafeCell<[GuardDescriptor; GUARD_CAPACITY]>,
    watches: UnsafeCell<[WatchDescriptor; WATCH_CAPACITY]>,
    violations: UnsafeCell<[ViolationRecord; VIOLATION_TAIL]>,
    crash_capsule: UnsafeCell<CrashCapsule>,
    previous_crash_capsule: UnsafeCell<CrashCapsule>,
    crash_history: UnsafeCell<[CrashHistoryEntry; FAILURE_HISTORY_CAPACITY]>,
    pattern_history: UnsafeCell<[FailurePatternSummary; FAILURE_PATTERN_CAPACITY]>,
    memory_lineage: UnsafeCell<[MemoryLineageEntry; MEMORY_LINEAGE_CAPACITY]>,
    memory_lineage_hints: UnsafeCell<[MemoryLineageHint; MEMORY_LINEAGE_HINT_CAPACITY]>,
    violation_hints: UnsafeCell<[ViolationHint; VIOLATION_HINT_CAPACITY]>,
    replay_ids: UnsafeCell<ReplayCorrelationIds>,
    current_window: UnsafeCell<RequestWindowState>,
    last_syscall_enter: UnsafeCell<SyscallFrontierRecord>,
    last_syscall_exit: UnsafeCell<SyscallFrontierRecord>,
    last_function_enter: UnsafeCell<FunctionTraceRecord>,
    last_function_checkpoint: UnsafeCell<FunctionTraceRecord>,
    last_function_exit: UnsafeCell<FunctionTraceRecord>,
    reprobe_policy: UnsafeCell<ReprobePolicyState>,
    active_capability: UnsafeCell<CapabilityId>,
    active_parent_capability: UnsafeCell<CapabilityId>,
    active_rights_mask: UnsafeCell<u64>,
}

unsafe impl Sync for TraceStorage {}

static TRACE_STORAGE: TraceStorage = TraceStorage {
    records: UnsafeCell::new([[TraceRecord::EMPTY; TRACE_CAPACITY]; MAX_TRACE_CPUS]),
    runtime_events: UnsafeCell::new(
        [[ChronoscopeRuntimeEventRecord::EMPTY; RUNTIME_EVENT_CAPACITY]; MAX_TRACE_CPUS],
    ),
    last_faults: UnsafeCell::new([FaultRecord::EMPTY; MAX_TRACE_CPUS]),
    last_user_status: UnsafeCell::new(MaybeUninit::uninit()),
    last_user_status_valid: AtomicBool::new(false),
    guards: UnsafeCell::new([GuardDescriptor::EMPTY; GUARD_CAPACITY]),
    watches: UnsafeCell::new([WatchDescriptor::EMPTY; WATCH_CAPACITY]),
    violations: UnsafeCell::new([ViolationRecord::EMPTY; VIOLATION_TAIL]),
    crash_capsule: UnsafeCell::new(CrashCapsule::EMPTY),
    previous_crash_capsule: UnsafeCell::new(CrashCapsule::EMPTY),
    crash_history: UnsafeCell::new([CrashHistoryEntry::EMPTY; FAILURE_HISTORY_CAPACITY]),
    pattern_history: UnsafeCell::new([FailurePatternSummary::EMPTY; FAILURE_PATTERN_CAPACITY]),
    memory_lineage: UnsafeCell::new([MemoryLineageEntry::EMPTY; MEMORY_LINEAGE_CAPACITY]),
    memory_lineage_hints: UnsafeCell::new([MemoryLineageHint::EMPTY; MEMORY_LINEAGE_HINT_CAPACITY]),
    violation_hints: UnsafeCell::new([ViolationHint::EMPTY; VIOLATION_HINT_CAPACITY]),
    replay_ids: UnsafeCell::new(ReplayCorrelationIds::EMPTY),
    current_window: UnsafeCell::new(RequestWindowState::EMPTY),
    last_syscall_enter: UnsafeCell::new(SyscallFrontierRecord::EMPTY),
    last_syscall_exit: UnsafeCell::new(SyscallFrontierRecord::EMPTY),
    last_function_enter: UnsafeCell::new(FunctionTraceRecord::EMPTY),
    last_function_checkpoint: UnsafeCell::new(FunctionTraceRecord::EMPTY),
    last_function_exit: UnsafeCell::new(FunctionTraceRecord::EMPTY),
    reprobe_policy: UnsafeCell::new(ReprobePolicyState::EMPTY),
    active_capability: UnsafeCell::new(CapabilityId::NONE),
    active_parent_capability: UnsafeCell::new(CapabilityId::NONE),
    active_rights_mask: UnsafeCell::new(0),
};

static PERF_EVENTS_EMITTED: AtomicU64 = AtomicU64::new(0);
static PERF_EVENTS_OVERWRITTEN: AtomicU64 = AtomicU64::new(0);
static PERF_REPLAY_EXECUTIONS: AtomicU64 = AtomicU64::new(0);
static PERF_REPLAY_STEPS: AtomicU64 = AtomicU64::new(0);
static PERF_QUERY_EXECUTIONS: AtomicU64 = AtomicU64::new(0);
static PERF_ESCALATIONS: AtomicU64 = AtomicU64::new(0);
static PERF_CHECKPOINTS: AtomicU64 = AtomicU64::new(0);
static PERF_LINEAGE_RECORDS: AtomicU64 = AtomicU64::new(0);
static PERF_DIFF_EXECUTIONS: AtomicU64 = AtomicU64::new(0);
static PERF_CAPTURE_COUNT: AtomicU64 = AtomicU64::new(0);
static PERF_CAPTURE_EVENT_TOTAL: AtomicU64 = AtomicU64::new(0);
static PERF_CAPTURE_EVENT_PEAK: AtomicU64 = AtomicU64::new(0);
static PERF_PARTIAL_CAPTURES: AtomicU64 = AtomicU64::new(0);
static CPU_RUNTIME_HIGH_WATER: [AtomicU32; MAX_TRACE_CPUS] =
    [const { AtomicU32::new(0) }; MAX_TRACE_CPUS];
static CPU_APPEND_ONLY_COUNT: [AtomicU64; MAX_TRACE_CPUS] =
    [const { AtomicU64::new(0) }; MAX_TRACE_CPUS];
static CPU_APPEND_ESCALATION_COUNT: [AtomicU64; MAX_TRACE_CPUS] =
    [const { AtomicU64::new(0) }; MAX_TRACE_CPUS];
static CPU_APPEND_CHECKPOINT_COUNT: [AtomicU64; MAX_TRACE_CPUS] =
    [const { AtomicU64::new(0) }; MAX_TRACE_CPUS];
static CPU_APPEND_OVERWRITE_COUNT: [AtomicU64; MAX_TRACE_CPUS] =
    [const { AtomicU64::new(0) }; MAX_TRACE_CPUS];
static DIAGNOSTICS_MODE: AtomicU32 = AtomicU32::new(DiagnosticsMode::Light as u32);
static GUARD_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static WATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static REPLAY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static COMPLETION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static IRQ_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static MEMORY_VERSION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static RUNTIME_CAPTURE_LEVEL: AtomicU32 = AtomicU32::new(ChronoscopeCaptureLevel::Standard as u32);
static CPU_APIC_IDS: [AtomicU64; MAX_TRACE_CPUS] = [
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
    AtomicU64::new(INVALID_APIC_ID),
];
static CPU_RUNTIME_EVENT_SEQUENCES: [AtomicU64; MAX_TRACE_CPUS] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];
static CPU_RUNTIME_OVERWRITES: [AtomicU64; MAX_TRACE_CPUS] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];
static CPU_CURRENT_STAGE: [AtomicU32; MAX_TRACE_CPUS] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];
static CPU_TRACE_SEQUENCES: [AtomicU64; MAX_TRACE_CPUS] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

#[cfg(target_os = "none")]
unsafe extern "C" {
    fn _start();
    fn x86_64_boot_stage0();
    fn x86_64_exception_dispatch(frame: *const ExceptionFrame) -> u64;
    static __kernel_start: u8;
    static __kernel_end: u8;
    static __bss_start: u8;
    static __bss_end: u8;
}

#[allow(dead_code)]
pub fn reset() {
    #[cfg(test)]
    ensure_test_thread_lock();
    unsafe {
        ptr::write_bytes(
            TRACE_STORAGE.records.get().cast::<u8>(),
            0,
            core::mem::size_of::<[[TraceRecord; TRACE_CAPACITY]; MAX_TRACE_CPUS]>(),
        );
        ptr::write(
            TRACE_STORAGE.runtime_events.get(),
            [[ChronoscopeRuntimeEventRecord::EMPTY; RUNTIME_EVENT_CAPACITY]; MAX_TRACE_CPUS],
        );
        ptr::write_bytes(
            TRACE_STORAGE.last_faults.get().cast::<u8>(),
            0,
            core::mem::size_of::<[FaultRecord; MAX_TRACE_CPUS]>(),
        );
        let mut cpu = 0usize;
        while cpu < MAX_TRACE_CPUS {
            CPU_APIC_IDS[cpu].store(INVALID_APIC_ID, Ordering::SeqCst);
            CPU_CURRENT_STAGE[cpu].store(0, Ordering::SeqCst);
            CPU_TRACE_SEQUENCES[cpu].store(0, Ordering::Relaxed);
            CPU_RUNTIME_EVENT_SEQUENCES[cpu].store(0, Ordering::Relaxed);
            CPU_RUNTIME_OVERWRITES[cpu].store(0, Ordering::Relaxed);
            CPU_RUNTIME_HIGH_WATER[cpu].store(0, Ordering::Relaxed);
            CPU_APPEND_ONLY_COUNT[cpu].store(0, Ordering::Relaxed);
            CPU_APPEND_ESCALATION_COUNT[cpu].store(0, Ordering::Relaxed);
            CPU_APPEND_CHECKPOINT_COUNT[cpu].store(0, Ordering::Relaxed);
            CPU_APPEND_OVERWRITE_COUNT[cpu].store(0, Ordering::Relaxed);
            cpu += 1;
        }
        PERF_EVENTS_EMITTED.store(0, Ordering::Relaxed);
        PERF_EVENTS_OVERWRITTEN.store(0, Ordering::Relaxed);
        PERF_REPLAY_EXECUTIONS.store(0, Ordering::Relaxed);
        PERF_REPLAY_STEPS.store(0, Ordering::Relaxed);
        PERF_QUERY_EXECUTIONS.store(0, Ordering::Relaxed);
        PERF_ESCALATIONS.store(0, Ordering::Relaxed);
        PERF_CHECKPOINTS.store(0, Ordering::Relaxed);
        PERF_LINEAGE_RECORDS.store(0, Ordering::Relaxed);
        PERF_DIFF_EXECUTIONS.store(0, Ordering::Relaxed);
        PERF_CAPTURE_COUNT.store(0, Ordering::Relaxed);
        PERF_CAPTURE_EVENT_TOTAL.store(0, Ordering::Relaxed);
        PERF_CAPTURE_EVENT_PEAK.store(0, Ordering::Relaxed);
        PERF_PARTIAL_CAPTURES.store(0, Ordering::Relaxed);
        ptr::write(
            TRACE_STORAGE.guards.get(),
            [GuardDescriptor::EMPTY; GUARD_CAPACITY],
        );
        ptr::write(
            TRACE_STORAGE.watches.get(),
            [WatchDescriptor::EMPTY; WATCH_CAPACITY],
        );
        ptr::write(
            TRACE_STORAGE.violations.get(),
            [ViolationRecord::EMPTY; VIOLATION_TAIL],
        );
        ptr::write(TRACE_STORAGE.crash_capsule.get(), CrashCapsule::EMPTY);
        ptr::write(
            TRACE_STORAGE.previous_crash_capsule.get(),
            CrashCapsule::EMPTY,
        );
        ptr::write(
            TRACE_STORAGE.crash_history.get(),
            [CrashHistoryEntry::EMPTY; FAILURE_HISTORY_CAPACITY],
        );
        ptr::write(
            TRACE_STORAGE.pattern_history.get(),
            [FailurePatternSummary::EMPTY; FAILURE_PATTERN_CAPACITY],
        );
        ptr::write(
            TRACE_STORAGE.memory_lineage.get(),
            [MemoryLineageEntry::EMPTY; MEMORY_LINEAGE_CAPACITY],
        );
        ptr::write(
            TRACE_STORAGE.memory_lineage_hints.get(),
            [MemoryLineageHint::EMPTY; MEMORY_LINEAGE_HINT_CAPACITY],
        );
        ptr::write(
            TRACE_STORAGE.violation_hints.get(),
            [ViolationHint::EMPTY; VIOLATION_HINT_CAPACITY],
        );
        ptr::write(TRACE_STORAGE.replay_ids.get(), ReplayCorrelationIds::EMPTY);
        ptr::write(
            TRACE_STORAGE.current_window.get(),
            RequestWindowState::EMPTY,
        );
        ptr::write(
            TRACE_STORAGE.last_syscall_enter.get(),
            SyscallFrontierRecord::EMPTY,
        );
        ptr::write(
            TRACE_STORAGE.last_syscall_exit.get(),
            SyscallFrontierRecord::EMPTY,
        );
        ptr::write(
            TRACE_STORAGE.last_function_enter.get(),
            FunctionTraceRecord::EMPTY,
        );
        ptr::write(
            TRACE_STORAGE.last_function_checkpoint.get(),
            FunctionTraceRecord::EMPTY,
        );
        ptr::write(
            TRACE_STORAGE.last_function_exit.get(),
            FunctionTraceRecord::EMPTY,
        );
        ptr::write(
            TRACE_STORAGE.reprobe_policy.get(),
            ReprobePolicyState::EMPTY,
        );
        ptr::write(TRACE_STORAGE.active_capability.get(), CapabilityId::NONE);
        ptr::write(
            TRACE_STORAGE.active_parent_capability.get(),
            CapabilityId::NONE,
        );
        ptr::write(TRACE_STORAGE.active_rights_mask.get(), 0);
    }
    GUARD_SEQUENCE.store(0, Ordering::SeqCst);
    WATCH_SEQUENCE.store(0, Ordering::SeqCst);
    REPLAY_SEQUENCE.store(0, Ordering::SeqCst);
    REQUEST_SEQUENCE.store(0, Ordering::SeqCst);
    COMPLETION_SEQUENCE.store(0, Ordering::SeqCst);
    IRQ_SEQUENCE.store(0, Ordering::SeqCst);
    MEMORY_VERSION_SEQUENCE.store(0, Ordering::Relaxed);
    RUNTIME_CAPTURE_LEVEL.store(ChronoscopeCaptureLevel::Standard as u32, Ordering::Relaxed);
}

pub fn trace_emit(
    kind: TraceKind,
    channel: TraceChannel,
    stage: u16,
    a: u64,
    b: u64,
    c: u64,
    d: u64,
) {
    let context = current_trace_context();
    push_with_context(kind, channel, stage, context, current_uptime(), a, b, c, d);
}

pub fn set_mode(mode: DiagnosticsMode) {
    DIAGNOSTICS_MODE.store(mode as u32, Ordering::Relaxed);
}

pub fn mode() -> DiagnosticsMode {
    match DIAGNOSTICS_MODE.load(Ordering::Relaxed) as u16 {
        1 => DiagnosticsMode::Light,
        2 => DiagnosticsMode::Targeted,
        3 => DiagnosticsMode::CrashFollowup,
        _ => DiagnosticsMode::Off,
    }
}

pub fn set_runtime_capture_level(level: ChronoscopeCaptureLevel) {
    RUNTIME_CAPTURE_LEVEL.store(level as u32, Ordering::Relaxed);
}

pub fn set_active_capability(capability_id: CapabilityId, rights_mask: u64) {
    unsafe {
        *TRACE_STORAGE.active_capability.get() = capability_id;
        *TRACE_STORAGE.active_parent_capability.get() = CapabilityId::NONE;
        *TRACE_STORAGE.active_rights_mask.get() = rights_mask;
    }
}

pub fn clear_active_capability() {
    unsafe {
        *TRACE_STORAGE.active_capability.get() = CapabilityId::NONE;
        *TRACE_STORAGE.active_parent_capability.get() = CapabilityId::NONE;
        *TRACE_STORAGE.active_rights_mask.get() = 0;
    }
}

pub fn record_capability_derivation(
    parent: CapabilityId,
    derived: CapabilityId,
    rights_mask: u64,
) -> ChronoscopeEventId {
    let context = current_trace_context();
    unsafe {
        *TRACE_STORAGE.active_capability.get() = derived;
        *TRACE_STORAGE.active_parent_capability.get() = parent;
        *TRACE_STORAGE.active_rights_mask.get() = rights_mask;
    }
    emit_runtime_event_with_context(
        context,
        ChronoscopeRuntimeEventKind::CapabilityDerive,
        parent.0,
        ChronoscopeEventId::NONE,
        0,
        ChronoscopeRuntimePayload::Capability {
            parent_lo: (parent.0 & 0xffff) as u16,
            parent_hi: ((parent.0 >> 16) & 0xffff) as u16,
            rights_hint: (rights_mask & 0xffff) as u16,
            reserved: 0,
        },
    )
}

pub fn runtime_capture_level() -> ChronoscopeCaptureLevel {
    match RUNTIME_CAPTURE_LEVEL.load(Ordering::Relaxed) as u16 {
        1 => ChronoscopeCaptureLevel::Minimal,
        3 => ChronoscopeCaptureLevel::Deep,
        _ => ChronoscopeCaptureLevel::Standard,
    }
}

fn chronoscope_runtime_kind_name(kind: ChronoscopeRuntimeEventKind) -> &'static str {
    match kind {
        ChronoscopeRuntimeEventKind::SchedulerSwitch => "sched-switch",
        ChronoscopeRuntimeEventKind::SchedulerWake => "sched-wake",
        ChronoscopeRuntimeEventKind::SchedulerBlock => "sched-block",
        ChronoscopeRuntimeEventKind::RequestStart => "request-start",
        ChronoscopeRuntimeEventKind::RequestComplete => "request-complete",
        ChronoscopeRuntimeEventKind::IrqEnter => "irq-enter",
        ChronoscopeRuntimeEventKind::IrqExit => "irq-exit",
        ChronoscopeRuntimeEventKind::ContractTransition => "contract",
        ChronoscopeRuntimeEventKind::ResourceClaim => "resource-claim",
        ChronoscopeRuntimeEventKind::ResourceRelease => "resource-release",
        ChronoscopeRuntimeEventKind::ResourceWait => "resource-wait",
        ChronoscopeRuntimeEventKind::ViolationObserved => "violation",
        ChronoscopeRuntimeEventKind::SuspectPromoted => "suspect",
        ChronoscopeRuntimeEventKind::FocusedTraceMarker => "focused",
        ChronoscopeRuntimeEventKind::FaultMarker => "fault",
        ChronoscopeRuntimeEventKind::DivergenceHint => "divergence",
        ChronoscopeRuntimeEventKind::CapabilityDerive => "cap-derive",
    }
}

fn should_capture_runtime_event(kind: ChronoscopeRuntimeEventKind) -> bool {
    match runtime_capture_level() {
        ChronoscopeCaptureLevel::Minimal => matches!(
            kind,
            ChronoscopeRuntimeEventKind::RequestStart
                | ChronoscopeRuntimeEventKind::RequestComplete
                | ChronoscopeRuntimeEventKind::IrqEnter
                | ChronoscopeRuntimeEventKind::IrqExit
                | ChronoscopeRuntimeEventKind::ViolationObserved
                | ChronoscopeRuntimeEventKind::FaultMarker
                | ChronoscopeRuntimeEventKind::DivergenceHint
        ),
        ChronoscopeCaptureLevel::Standard => !matches!(
            kind,
            ChronoscopeRuntimeEventKind::SchedulerWake
                | ChronoscopeRuntimeEventKind::FocusedTraceMarker
        ),
        ChronoscopeCaptureLevel::Deep => true,
    }
}

#[inline(always)]
fn compose_runtime_event_id(cpu_slot: usize, local_sequence: u64) -> ChronoscopeEventId {
    ChronoscopeEventId(compose_event_sequence(cpu_slot, local_sequence))
}

#[inline(always)]
fn emit_runtime_event_with_context(
    context: CpuTraceContext,
    kind: ChronoscopeRuntimeEventKind,
    object_key: u64,
    causal_parent: ChronoscopeEventId,
    flags: u16,
    payload: ChronoscopeRuntimePayload,
) -> ChronoscopeEventId {
    #[cfg(test)]
    ensure_test_thread_lock();
    if !should_capture_runtime_event(kind) {
        return ChronoscopeEventId::NONE;
    }
    let local = CPU_RUNTIME_EVENT_SEQUENCES[context.cpu_slot].fetch_add(1, Ordering::Relaxed) + 1;
    PERF_EVENTS_EMITTED.fetch_add(1, Ordering::Relaxed);
    CPU_APPEND_ONLY_COUNT[context.cpu_slot].fetch_add(1, Ordering::Relaxed);
    if matches!(
        runtime_capture_level(),
        ChronoscopeCaptureLevel::Standard | ChronoscopeCaptureLevel::Deep
    ) {
        CPU_APPEND_ESCALATION_COUNT[context.cpu_slot].fetch_add(1, Ordering::Relaxed);
    }
    if matches!(
        kind,
        ChronoscopeRuntimeEventKind::FaultMarker | ChronoscopeRuntimeEventKind::ViolationObserved
    ) {
        CPU_APPEND_CHECKPOINT_COUNT[context.cpu_slot].fetch_add(1, Ordering::Relaxed);
    }
    if local > RUNTIME_EVENT_CAPACITY as u64 {
        CPU_RUNTIME_OVERWRITES[context.cpu_slot].fetch_add(1, Ordering::Relaxed);
        PERF_EVENTS_OVERWRITTEN.fetch_add(1, Ordering::Relaxed);
        CPU_APPEND_OVERWRITE_COUNT[context.cpu_slot].fetch_add(1, Ordering::Relaxed);
    }
    let occupancy = local.min(RUNTIME_EVENT_CAPACITY as u64) as u32;
    let mut previous = CPU_RUNTIME_HIGH_WATER[context.cpu_slot].load(Ordering::Relaxed);
    while occupancy > previous {
        match CPU_RUNTIME_HIGH_WATER[context.cpu_slot].compare_exchange_weak(
            previous,
            occupancy,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(current) => previous = current,
        }
    }
    let event_id = compose_runtime_event_id(context.cpu_slot, local);
    let slot = ((local - 1) as usize) % RUNTIME_EVENT_CAPACITY;
    unsafe {
        let capability_id = *TRACE_STORAGE.active_capability.get();
        let parent_capability_id = *TRACE_STORAGE.active_parent_capability.get();
        let rights_mask = *TRACE_STORAGE.active_rights_mask.get();
        (*TRACE_STORAGE.runtime_events.get())[context.cpu_slot][slot] =
            ChronoscopeRuntimeEventRecord {
                valid: true,
                event_id,
                buffer_id: ChronoscopeBufferId(context.cpu_slot as u16 + 1),
                local_sequence: local,
                core_id: context.cpu_slot as u16,
                stage: context.stage,
                uptime_us: current_uptime().unwrap_or(0),
                kind,
                correlation: replay_ids(),
                object_key,
                capability_id,
                parent_capability_id,
                rights_mask,
                causal_parent,
                flags,
                payload,
            };
    }
    event_id
}

pub fn emit_sched_event(
    from_tid: u32,
    to_tid: u32,
    reason: u16,
    blocked: bool,
) -> ChronoscopeEventId {
    let context = current_trace_context();
    emit_runtime_event_with_context(
        context,
        if blocked {
            ChronoscopeRuntimeEventKind::SchedulerBlock
        } else {
            ChronoscopeRuntimeEventKind::SchedulerSwitch
        },
        ((from_tid as u64) << 32) | (to_tid as u64),
        ChronoscopeEventId::NONE,
        0,
        ChronoscopeRuntimePayload::Scheduler {
            from_tid,
            to_tid,
            reason,
        },
    )
}

pub fn emit_irq_event(vector: u16, enter: bool) -> ChronoscopeEventId {
    let context = current_trace_context();
    emit_runtime_event_with_context(
        context,
        if enter {
            ChronoscopeRuntimeEventKind::IrqEnter
        } else {
            ChronoscopeRuntimeEventKind::IrqExit
        },
        vector as u64,
        ChronoscopeEventId::NONE,
        0,
        ChronoscopeRuntimePayload::Irq {
            vector,
            phase: if enter { 1 } else { 2 },
            reserved: 0,
        },
    )
}

pub fn emit_contract_event(
    contract_id: u32,
    from_state: u16,
    to_state: u16,
    object_key: u64,
) -> ChronoscopeEventId {
    let context = current_trace_context();
    emit_runtime_event_with_context(
        context,
        ChronoscopeRuntimeEventKind::ContractTransition,
        object_key,
        ChronoscopeEventId::NONE,
        0,
        ChronoscopeRuntimePayload::Contract {
            contract_id,
            from_state,
            to_state,
        },
    )
}

pub fn emit_resource_event(
    kind: ChronoscopeRuntimeEventKind,
    resource_id: u32,
    actor: u16,
    state: u16,
) -> ChronoscopeEventId {
    let context = current_trace_context();
    emit_runtime_event_with_context(
        context,
        kind,
        resource_id as u64,
        ChronoscopeEventId::NONE,
        0,
        ChronoscopeRuntimePayload::Resource {
            resource_id,
            actor,
            state,
        },
    )
}

pub fn emit_fault_event(vector: u16, stage: u16) -> ChronoscopeEventId {
    let context = current_trace_context();
    emit_runtime_event_with_context(
        context,
        ChronoscopeRuntimeEventKind::FaultMarker,
        vector as u64,
        ChronoscopeEventId::NONE,
        1,
        ChronoscopeRuntimePayload::Fault {
            vector,
            stage,
            reserved: 0,
        },
    )
}

fn runtime_event_correlation_matches(
    event: ChronoscopeRuntimeEventRecord,
    replay: ReplayCorrelationIds,
) -> bool {
    (replay.request_id != 0 && event.correlation.request_id == replay.request_id)
        || (replay.completion_id != 0 && event.correlation.completion_id == replay.completion_id)
        || (replay.irq_id != 0 && event.correlation.irq_id == replay.irq_id)
}

fn snapshot_runtime_events(replay: ReplayCorrelationIds) -> ChronoscopeRuntimeEventWindow {
    #[cfg(test)]
    ensure_test_thread_lock();
    let mut out = ChronoscopeRuntimeEventWindow::EMPTY;
    let records = unsafe { *TRACE_STORAGE.runtime_events.get() };
    let mut cpu = 0usize;
    let mut out_index = 0usize;
    while cpu < MAX_TRACE_CPUS {
        let latest = CPU_RUNTIME_EVENT_SEQUENCES[cpu].load(Ordering::Relaxed);
        let overwrites = CPU_RUNTIME_OVERWRITES[cpu].load(Ordering::Relaxed);
        let available = if latest > RUNTIME_EVENT_CAPACITY as u64 {
            RUNTIME_EVENT_CAPACITY as u16
        } else {
            latest as u16
        };
        let oldest = if available == 0 {
            0
        } else {
            latest.saturating_sub(available as u64).saturating_add(1)
        };
        out.per_core[cpu] = ChronoscopeRuntimeCoreSummary {
            valid: available != 0 || overwrites != 0,
            buffer_id: ChronoscopeBufferId(cpu as u16 + 1),
            core_id: cpu as u16,
            available_events: available,
            overwritten_events: overwrites,
            oldest_local_sequence: oldest,
            newest_local_sequence: latest,
            partial: overwrites != 0,
            high_water_mark: CPU_RUNTIME_HIGH_WATER[cpu].load(Ordering::Relaxed) as u16,
        };
        if overwrites != 0 {
            out.partial = true;
            out.integrity.complete = false;
            out.integrity.primary_reason = ChronoscopePartialReason::RingOverwrite;
            out.integrity.flags.ring_overwrite = true;
        }
        let mut local = oldest;
        while local <= latest && local != 0 {
            let slot = ((local - 1) as usize) % RUNTIME_EVENT_CAPACITY;
            let record = records[cpu][slot];
            if record.valid
                && record.local_sequence == local
                && (runtime_event_correlation_matches(record, replay)
                    || matches!(
                        record.kind,
                        ChronoscopeRuntimeEventKind::FaultMarker
                            | ChronoscopeRuntimeEventKind::ViolationObserved
                            | ChronoscopeRuntimeEventKind::DivergenceHint
                    ))
            {
                if out_index < out.events.len() {
                    out.events[out_index] = record;
                    out_index += 1;
                } else {
                    out.partial = true;
                    out.integrity.complete = false;
                    out.integrity.flags.deep_capture_drop = true;
                    if matches!(out.integrity.primary_reason, ChronoscopePartialReason::None) {
                        out.integrity.primary_reason = ChronoscopePartialReason::DeepCaptureDrop;
                    }
                }
            }
            local += 1;
        }
        cpu += 1;
    }
    out.valid = out_index != 0;
    out.total_events = out_index as u16;
    PERF_CAPTURE_COUNT.fetch_add(1, Ordering::Relaxed);
    PERF_CAPTURE_EVENT_TOTAL.fetch_add(out.total_events as u64, Ordering::Relaxed);
    if out.partial {
        PERF_PARTIAL_CAPTURES.fetch_add(1, Ordering::Relaxed);
    }
    let mut peak = PERF_CAPTURE_EVENT_PEAK.load(Ordering::Relaxed);
    while (out.total_events as u64) > peak {
        match PERF_CAPTURE_EVENT_PEAK.compare_exchange_weak(
            peak,
            out.total_events as u64,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(current) => peak = current,
        }
    }
    out
}

fn snapshot_perf_counters() -> ChronoscopePerfCounters {
    let capture_count = PERF_CAPTURE_COUNT.load(Ordering::Relaxed);
    let capture_total = PERF_CAPTURE_EVENT_TOTAL.load(Ordering::Relaxed);
    let mut per_core = [ChronoscopeCorePerfCounters::EMPTY; MAX_TRACE_CPUS];
    let mut cpu = 0usize;
    while cpu < MAX_TRACE_CPUS {
        per_core[cpu] = ChronoscopeCorePerfCounters {
            events_emitted: CPU_RUNTIME_EVENT_SEQUENCES[cpu].load(Ordering::Relaxed),
            events_overwritten: CPU_RUNTIME_OVERWRITES[cpu].load(Ordering::Relaxed),
            high_water_mark: CPU_RUNTIME_HIGH_WATER[cpu].load(Ordering::Relaxed) as u16,
            append_only_count: CPU_APPEND_ONLY_COUNT[cpu].load(Ordering::Relaxed),
            escalation_branch_count: CPU_APPEND_ESCALATION_COUNT[cpu].load(Ordering::Relaxed),
            checkpoint_branch_count: CPU_APPEND_CHECKPOINT_COUNT[cpu].load(Ordering::Relaxed),
            overwrite_path_count: CPU_APPEND_OVERWRITE_COUNT[cpu].load(Ordering::Relaxed),
        };
        cpu += 1;
    }
    ChronoscopePerfCounters {
        schema_version: CHRONOSCOPE_SCHEMA_VERSION,
        events_emitted: PERF_EVENTS_EMITTED.load(Ordering::Relaxed),
        events_dropped_or_overwritten: PERF_EVENTS_OVERWRITTEN.load(Ordering::Relaxed),
        replay_executions: PERF_REPLAY_EXECUTIONS.load(Ordering::Relaxed),
        replay_steps: PERF_REPLAY_STEPS.load(Ordering::Relaxed),
        query_executions: PERF_QUERY_EXECUTIONS.load(Ordering::Relaxed),
        escalations: PERF_ESCALATIONS.load(Ordering::Relaxed),
        checkpoints: PERF_CHECKPOINTS.load(Ordering::Relaxed),
        lineage_records: PERF_LINEAGE_RECORDS.load(Ordering::Relaxed),
        diff_executions: PERF_DIFF_EXECUTIONS.load(Ordering::Relaxed),
        captures: ChronoscopeCaptureStats {
            average_events_per_capture: if capture_count == 0 {
                0
            } else {
                (capture_total / capture_count).min(u16::MAX as u64) as u16
            },
            peak_events_per_capture: PERF_CAPTURE_EVENT_PEAK
                .load(Ordering::Relaxed)
                .min(u16::MAX as u64) as u16,
            partial_captures: PERF_PARTIAL_CAPTURES
                .load(Ordering::Relaxed)
                .min(u16::MAX as u64) as u16,
        },
        per_core,
    }
}

fn chronoscope_collect_diff_entries(
    bundle: &ChronoscopeBundle,
) -> ([ChronoscopeDiffEntry; CHRONOSCOPE_NODE_LIMIT], usize) {
    let mut out = [ChronoscopeDiffEntry::EMPTY; CHRONOSCOPE_NODE_LIMIT];
    let mut next = 0usize;
    let mut index = 0usize;
    while index < bundle.nodes.len() {
        let node = bundle.nodes[index];
        if node.valid {
            if next >= out.len() {
                break;
            }
            out[next] = ChronoscopeDiffEntry {
                stable_id: node.stable_id,
                path: node.path,
            };
            next += 1;
        }
        index += 1;
    }
    (out, next)
}

fn chronoscope_sort_diff_entries(
    entries: &mut [ChronoscopeDiffEntry; CHRONOSCOPE_NODE_LIMIT],
    len: usize,
) {
    if len <= 1 {
        return;
    }
    let mut scratch = [ChronoscopeDiffEntry::EMPTY; CHRONOSCOPE_NODE_LIMIT];
    let mut width = 1usize;
    while width < len {
        let mut start = 0usize;
        while start < len {
            let mid = (start + width).min(len);
            let end = (start + width * 2).min(len);
            let mut left = start;
            let mut right = mid;
            let mut out = start;
            while left < mid && right < end {
                if entries[left].stable_id <= entries[right].stable_id {
                    scratch[out] = entries[left];
                    left += 1;
                } else {
                    scratch[out] = entries[right];
                    right += 1;
                }
                out += 1;
            }
            while left < mid {
                scratch[out] = entries[left];
                left += 1;
                out += 1;
            }
            while right < end {
                scratch[out] = entries[right];
                right += 1;
                out += 1;
            }
            let mut copy = start;
            while copy < end {
                entries[copy] = scratch[copy];
                copy += 1;
            }
            start = end;
        }
        width *= 2;
    }
}

pub fn next_request_id() -> u64 {
    let id = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    unsafe {
        (*TRACE_STORAGE.replay_ids.get()).request_id = id;
        (*TRACE_STORAGE.replay_ids.get()).sequence =
            REPLAY_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    }
    id
}

pub fn next_completion_id() -> u64 {
    let id = COMPLETION_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    unsafe {
        (*TRACE_STORAGE.replay_ids.get()).completion_id = id;
        (*TRACE_STORAGE.replay_ids.get()).sequence =
            REPLAY_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    }
    id
}

pub fn next_irq_id() -> u64 {
    let id = IRQ_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    unsafe {
        (*TRACE_STORAGE.replay_ids.get()).irq_id = id;
        (*TRACE_STORAGE.replay_ids.get()).sequence =
            REPLAY_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    }
    id
}

pub fn replay_ids() -> ReplayCorrelationIds {
    unsafe { *TRACE_STORAGE.replay_ids.get() }
}

pub fn set_active_window(
    syscall_id: u64,
    fd: u64,
    request_op: u64,
    device_id: u64,
    completion_state: u64,
    path: DiagnosticsPath,
    request_id: u64,
    completion_id: u64,
) {
    let context = current_trace_context();
    unsafe {
        *TRACE_STORAGE.current_window.get() = RequestWindowState {
            valid: true,
            syscall_id,
            fd,
            request_op,
            device_id,
            completion_state,
            path,
            request_id,
            completion_id,
        };
    }
    let _ = emit_runtime_event_with_context(
        context,
        ChronoscopeRuntimeEventKind::RequestStart,
        device_id,
        ChronoscopeEventId::NONE,
        0,
        ChronoscopeRuntimePayload::Request {
            op: request_op as u16,
            status: completion_state as u16,
            device_id: device_id as u32,
        },
    );
}

pub fn clear_active_window() {
    let context = current_trace_context();
    let window = unsafe { *TRACE_STORAGE.current_window.get() };
    unsafe {
        *TRACE_STORAGE.current_window.get() = RequestWindowState::EMPTY;
    }
    if window.valid {
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::RequestComplete,
            window.device_id,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::Request {
                op: window.request_op as u16,
                status: window.completion_state as u16,
                device_id: window.device_id as u32,
            },
        );
    }
}

pub fn record_syscall_enter(syscall_id: u64, arg0: u64, arg1: u64, arg2: u64) {
    let context = current_trace_context();
    let replay = replay_ids();
    unsafe {
        *TRACE_STORAGE.last_syscall_enter.get() = SyscallFrontierRecord {
            valid: true,
            sequence: latest_event_sequence(context.cpu_slot),
            syscall_id,
            arg0,
            arg1,
            arg2,
            stage: context.stage,
            cpu_slot: context.cpu_slot as u16,
            apic_id: context.apic_id,
            request_id: replay.request_id,
            completion_id: replay.completion_id,
            irq_id: replay.irq_id,
            result_ok: false,
            errno: 0,
        };
    }
}

pub fn record_syscall_exit(
    syscall_id: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    result_ok: bool,
    errno: u16,
) {
    let context = current_trace_context();
    let replay = replay_ids();
    unsafe {
        *TRACE_STORAGE.last_syscall_exit.get() = SyscallFrontierRecord {
            valid: true,
            sequence: latest_event_sequence(context.cpu_slot),
            syscall_id,
            arg0,
            arg1,
            arg2,
            stage: context.stage,
            cpu_slot: context.cpu_slot as u16,
            apic_id: context.apic_id,
            request_id: replay.request_id,
            completion_id: replay.completion_id,
            irq_id: replay.irq_id,
            result_ok,
            errno,
        };
    }
}

pub fn record_function_enter(function_id: u64, step_id: u64, object0: u64, object1: u64) {
    let context = current_trace_context();
    let replay = replay_ids();
    unsafe {
        *TRACE_STORAGE.last_function_enter.get() = FunctionTraceRecord {
            valid: true,
            sequence: latest_event_sequence(context.cpu_slot),
            function_id,
            checkpoint_id: 0,
            step_id,
            object0,
            object1,
            stage: context.stage,
            cpu_slot: context.cpu_slot as u16,
            apic_id: context.apic_id,
            request_id: replay.request_id,
            completion_id: replay.completion_id,
            irq_id: replay.irq_id,
            result_ok: false,
            errno: 0,
        };
    }
}

pub fn record_function_checkpoint(
    function_id: u64,
    checkpoint_id: u64,
    step_id: u64,
    object0: u64,
    object1: u64,
) {
    let context = current_trace_context();
    let replay = replay_ids();
    unsafe {
        *TRACE_STORAGE.last_function_checkpoint.get() = FunctionTraceRecord {
            valid: true,
            sequence: latest_event_sequence(context.cpu_slot),
            function_id,
            checkpoint_id,
            step_id,
            object0,
            object1,
            stage: context.stage,
            cpu_slot: context.cpu_slot as u16,
            apic_id: context.apic_id,
            request_id: replay.request_id,
            completion_id: replay.completion_id,
            irq_id: replay.irq_id,
            result_ok: false,
            errno: 0,
        };
    }
}

pub fn record_function_exit(
    function_id: u64,
    checkpoint_id: u64,
    step_id: u64,
    object0: u64,
    object1: u64,
    result_ok: bool,
    errno: u16,
) {
    let context = current_trace_context();
    let replay = replay_ids();
    unsafe {
        *TRACE_STORAGE.last_function_exit.get() = FunctionTraceRecord {
            valid: true,
            sequence: latest_event_sequence(context.cpu_slot),
            function_id,
            checkpoint_id,
            step_id,
            object0,
            object1,
            stage: context.stage,
            cpu_slot: context.cpu_slot as u16,
            apic_id: context.apic_id,
            request_id: replay.request_id,
            completion_id: replay.completion_id,
            irq_id: replay.irq_id,
            result_ok,
            errno,
        };
    }
}

pub fn guard_register(
    kind: GuardKind,
    path: DiagnosticsPath,
    address: u64,
    length: u64,
    red_zone: u64,
    request_id: u64,
    completion_id: u64,
) -> u64 {
    let context = current_trace_context();
    let sequence = GUARD_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    let id = GUARD_ID_TAG | sequence;
    let slot = (sequence as usize - 1) % GUARD_CAPACITY;
    let canary = address ^ length ^ red_zone ^ request_id ^ completion_id ^ 0x4744_5244_5f4e_474f;
    unsafe {
        (*TRACE_STORAGE.guards.get())[slot] = GuardDescriptor {
            active: true,
            id,
            kind,
            registration_sequence: latest_event_sequence(context.cpu_slot),
            stage: context.stage,
            path,
            request_id,
            completion_id,
            address,
            length,
            red_zone,
            canary,
        };
    }
    trace_emit(
        TraceKind::Memory,
        TraceChannel::Memory,
        context.stage,
        id,
        address,
        length,
        red_zone,
    );
    id
}

pub fn watch_register(
    kind: WatchKind,
    path: DiagnosticsPath,
    address: u64,
    length: u64,
    request_id: u64,
    completion_id: u64,
) -> u64 {
    let context = current_trace_context();
    let sequence = WATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    let id = WATCH_ID_TAG | sequence;
    let slot = (sequence as usize - 1) % WATCH_CAPACITY;
    unsafe {
        (*TRACE_STORAGE.watches.get())[slot] = WatchDescriptor {
            active: true,
            id,
            kind,
            registration_sequence: latest_event_sequence(context.cpu_slot),
            stage: context.stage,
            path,
            request_id,
            completion_id,
            address,
            length,
        };
    }
    trace_emit(
        TraceKind::Memory,
        TraceChannel::Memory,
        context.stage,
        id,
        address,
        length,
        kind as u64,
    );
    id
}

pub fn guard_check(address: u64, length: u64) -> bool {
    let guards = unsafe { &*TRACE_STORAGE.guards.get() };
    for descriptor in guards.iter().copied().filter(|entry| entry.active) {
        let start = descriptor.address.saturating_sub(descriptor.red_zone);
        let end = descriptor
            .address
            .saturating_add(descriptor.length)
            .saturating_add(descriptor.red_zone);
        let access_end = address.saturating_add(length);
        if address < end && access_end > start {
            record_violation(
                ViolationKind::Guard,
                descriptor.id,
                descriptor.kind as u16,
                address,
                length,
                descriptor.address,
                descriptor.length,
                descriptor.red_zone,
                descriptor.stage,
                descriptor.path,
                descriptor.request_id,
                descriptor.completion_id,
            );
            return false;
        }
    }
    true
}

pub fn watch_touch(kind: WatchKind, address: u64, length: u64) {
    let watches = unsafe { &*TRACE_STORAGE.watches.get() };
    for descriptor in watches.iter().copied().filter(|entry| entry.active) {
        let matches_kind = descriptor.kind == WatchKind::Touch
            || descriptor.kind == kind
            || kind == WatchKind::Touch;
        let access_end = address.saturating_add(length);
        let watch_end = descriptor.address.saturating_add(descriptor.length);
        if matches_kind && address < watch_end && access_end > descriptor.address {
            record_violation(
                ViolationKind::Watch,
                descriptor.id,
                descriptor.kind as u16,
                address,
                length,
                descriptor.address,
                descriptor.length,
                0,
                descriptor.stage,
                descriptor.path,
                descriptor.request_id,
                descriptor.completion_id,
            );
        }
    }
}

pub fn reprobe_policy_on_boot() -> ReprobePolicyState {
    unsafe {
        let state = &mut *TRACE_STORAGE.reprobe_policy.get();
        if state.crash_count > 0 {
            state.mode = DiagnosticsMode::CrashFollowup;
        }
        *state
    }
}

pub fn reprobe_policy_on_crash(
    path: DiagnosticsPath,
    stage: u16,
    checkpoint: u64,
) -> ReprobePolicyState {
    unsafe {
        let state = &mut *TRACE_STORAGE.reprobe_policy.get();
        state.target_path = path;
        state.target_stage = stage;
        state.target_checkpoint = checkpoint;
        state.crash_count = state.crash_count.saturating_add(1);
        state.escalation = state.escalation.saturating_add(1).min(3);
        state.mode = match state.escalation {
            0 | 1 => DiagnosticsMode::Targeted,
            _ => DiagnosticsMode::CrashFollowup,
        };
        *state
    }
}

pub fn record_boot_stage(stage: BootTraceStage, uptime_us: Option<u64>, detail: u64) {
    let mut context = current_trace_context();
    CPU_CURRENT_STAGE[context.cpu_slot].store(stage as u32, Ordering::Relaxed);
    context.stage = stage as u16;
    push_with_context(
        TraceKind::BootStage,
        TraceChannel::Boot,
        stage as u16,
        context,
        uptime_us,
        stage as u64,
        detail,
        0,
        0,
    );
}

pub fn record_user_marker(marker: u64, value: u64, rip: u64, uptime_us: Option<u64>) {
    let context = current_trace_context();
    push_with_context(
        TraceKind::UserMarker,
        TraceChannel::User,
        context.stage,
        context,
        uptime_us,
        marker,
        value,
        rip,
        0,
    );
}

pub fn record_user_status(snapshot: FirstUserProcessStatus) {
    #[cfg(test)]
    ensure_test_thread_lock();
    unsafe {
        ptr::write(
            TRACE_STORAGE.last_user_status.get(),
            MaybeUninit::new(snapshot),
        );
    }
    TRACE_STORAGE
        .last_user_status_valid
        .store(true, Ordering::Relaxed);
    let flags = (snapshot.started as u64)
        | ((snapshot.main_reached as u64) << 1)
        | ((snapshot.exited as u64) << 2)
        | ((snapshot.faulted as u64) << 3)
        | ((snapshot.boot_reported as u64) << 4);
    let context = current_trace_context();
    push_with_context(
        TraceKind::UserStatus,
        TraceChannel::User,
        context.stage,
        context,
        None,
        flags,
        snapshot.exit_code as i64 as u64,
        snapshot.syscall_count,
        ((snapshot.boot_report_stage as u64) << 32) | snapshot.boot_report_status as u64,
    );
}

pub fn record_irq_event(line: u8, dispatch_count: usize, handled: bool) {
    let context = current_trace_context();
    push_with_context(
        TraceKind::Irq,
        TraceChannel::Irq,
        context.stage,
        context,
        current_uptime(),
        u64::from(line),
        dispatch_count as u64,
        handled as u64,
        0,
    );
    let _ = emit_runtime_event_with_context(
        context,
        if handled {
            ChronoscopeRuntimeEventKind::IrqExit
        } else {
            ChronoscopeRuntimeEventKind::IrqEnter
        },
        u64::from(line),
        ChronoscopeEventId::NONE,
        handled as u16,
        ChronoscopeRuntimePayload::Irq {
            vector: line as u16,
            phase: if handled { 2 } else { 1 },
            reserved: dispatch_count as u32,
        },
    );
}

pub fn record_memory_window(tag: u64, start: u64, len: u64, detail: u64) {
    let context = current_trace_context();
    push_with_context(
        TraceKind::Memory,
        TraceChannel::Memory,
        context.stage,
        context,
        None,
        tag,
        start,
        len,
        detail,
    );
}

#[allow(dead_code)]
pub fn record_memory_overlap(tag: u64, left_start: u64, left_len: u64, right_start: u64) {
    let context = current_trace_context();
    push_with_context(
        TraceKind::Memory,
        TraceChannel::Memory,
        context.stage,
        context,
        None,
        tag,
        left_start,
        left_len,
        right_start,
    );
}

pub fn record_memory_lineage(
    kind: MemoryLineageKind,
    address_space_id: u64,
    base: u64,
    len: u64,
    object_id: u64,
    bytes_changed: u32,
    digest_seed: u64,
) -> u64 {
    let context = current_trace_context();
    let replay = replay_ids();
    let version_id = MEMORY_VERSION_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    let parent_version_id = last_memory_version_for_range(address_space_id, base, len);
    let digest = memory_lineage_digest(
        parent_version_id,
        address_space_id,
        base,
        len,
        object_id,
        u64::from(bytes_changed),
        digest_seed,
        kind as u64,
    );
    let slot = (version_id as usize - 1) % MEMORY_LINEAGE_CAPACITY;
    unsafe {
        (*TRACE_STORAGE.memory_lineage.get())[slot] = MemoryLineageEntry {
            valid: true,
            sequence: latest_event_sequence(context.cpu_slot),
            version_id,
            parent_version_id,
            address_space_id,
            base,
            len,
            object_id,
            request_id: replay.request_id,
            completion_id: replay.completion_id,
            irq_id: replay.irq_id,
            cpu_slot: context.cpu_slot as u16,
            stage: context.stage,
            kind,
            bytes_changed,
            digest,
        };
        update_memory_lineage_hint(address_space_id, base, len, version_id);
    }
    push_with_context(
        TraceKind::Memory,
        TraceChannel::Memory,
        context.stage,
        context,
        None,
        version_id,
        base,
        len,
        digest,
    );
    version_id
}

#[allow(dead_code)]
pub fn record_device_event(kind: u64, a: u64, b: u64, c: u64) {
    let context = current_trace_context();
    push_with_context(
        TraceKind::Device,
        TraceChannel::Device,
        context.stage,
        context,
        current_uptime(),
        kind,
        a,
        b,
        c,
    );
}

pub fn record_transition(kind: u64, a: u64, b: u64, c: u64) {
    let context = current_trace_context();
    push_with_context(
        TraceKind::Transition,
        TraceChannel::Transition,
        context.stage,
        context,
        current_uptime(),
        kind,
        a,
        b,
        c,
    );
}

pub fn record_fault(frame: &ExceptionFrame, uptime_us: Option<u64>, cr2: Option<u64>) {
    let (cpu_slot, apic_id) = current_cpu_context();
    let stage = current_stage_for(cpu_slot);
    let controls = capture_control_registers(cr2.unwrap_or(0));
    let locator = boot_locator::snapshot();
    let record = FaultRecord {
        valid: true,
        uptime_us: uptime_us.unwrap_or(0),
        apic_id,
        cpu_slot: cpu_slot as u16,
        stage,
        vector: frame.vector,
        rip: frame.rip,
        cs: frame.cs,
        rflags: frame.rflags,
        error_code: frame.error_code,
        cr2: controls.cr2,
        cr0: controls.cr0,
        cr3: controls.cr3,
        cr4: controls.cr4,
        efer: controls.efer,
        rax: frame.rax,
        rbx: frame.rbx,
        rcx: frame.rcx,
        rdx: frame.rdx,
        rsi: frame.rsi,
        rdi: frame.rdi,
        rbp: frame.rbp,
        r8: frame.r8,
        r9: frame.r9,
        r10: frame.r10,
        r11: frame.r11,
        r12: frame.r12,
        r13: frame.r13,
        r14: frame.r14,
        r15: frame.r15,
        locator_sequence: locator.sequence,
        locator_stage: locator.stage as u16,
        locator_checkpoint: locator.checkpoint,
        locator_payload0: locator.payload0,
        locator_payload1: locator.payload1,
    };
    unsafe {
        (*TRACE_STORAGE.last_faults.get())[cpu_slot] = record;
    }
    let _ = emit_fault_event(frame.vector as u16, stage);
    let mut crash_path = DiagnosticsPath::Fault;
    let mut crash_checkpoint = locator.checkpoint;
    let window = unsafe { *TRACE_STORAGE.current_window.get() };
    if window.valid {
        crash_path = window.path;
        crash_checkpoint = window.request_id.max(window.completion_id);
    }
    let _ = reprobe_policy_on_crash(crash_path, stage, crash_checkpoint);
    push_with_context(
        TraceKind::Fault,
        TraceChannel::Fault,
        stage,
        CpuTraceContext {
            cpu_slot,
            apic_id,
            stage,
        },
        uptime_us,
        frame.vector,
        frame.rip,
        frame.error_code,
        controls.cr2,
    );
    if controls.cr2 != 0 {
        let _ = guard_check(controls.cr2, 1);
        watch_touch(WatchKind::Touch, controls.cr2, 1);
    }
    crash_capsule_capture();
}

pub fn emit_report() {
    serial::write_bytes(b"NGOS DIAGNOSTICS REPORT\n");
    emit_chronoscope_report();
    emit_failure_explanation();
    emit_causal_chain();
    emit_causal_ledger();
    emit_earliest_preventable_boundary();
    emit_invariant_coverage();
    emit_differential_flow();
    emit_semantic_race_report();
    emit_replay_window_summary();
    emit_patch_suggestions();
    emit_reprobe_summary();
    emit_active_window_summary();
    emit_syscall_frontier_summary();
    emit_exact_localization_summary();
    emit_fault_summary();
    emit_reason_summary();
    emit_pattern_path_narrowing();
    emit_failure_comparison();
    emit_same_pattern_stability_report();
    emit_violation_summary();
    emit_memory_debug_summary();
    emit_memory_lineage_summary();
    emit_focused_path_view();
    emit_suspect_evidence_bundles();
    emit_export_bundle();
    emit_smp_timeline_summary();
    emit_trace_summary();
    dump_crash_capsule();
    dump_failure_history();
    if let Some(status) = last_user_status() {
        serial::print(format_args!(
            "ngos/x86_64: diag last_user started={} main={} exited={} faulted={} exit_code={} syscalls={} boot_stage={} boot_status={}\n",
            status.started,
            status.main_reached,
            status.exited,
            status.faulted,
            status.exit_code,
            status.syscall_count,
            status.boot_report_stage,
            status.boot_report_status
        ));
    }
}

#[allow(dead_code)]
pub fn emit_report_compact() {
    serial::write_bytes(b"NGOS DIAGNOSTICS COMPACT REPORT\n");
    emit_failure_explanation();
    emit_failure_story_compact();
    emit_earliest_preventable_boundary_compact();
    emit_top_suspect_compact();
    emit_failure_classification();
    emit_developer_hint();
    emit_syscall_frontier_summary();
    emit_exact_localization_summary();
}

#[allow(dead_code)]
pub fn emit_patch_suggestion_compact() {
    serial::write_bytes(b"NGOS PATCH SUGGESTION COMPACT\n");
    emit_likely_bug_class();
    emit_top_patch_target_compact();
    emit_patch_shape_summary();
    emit_check_first_summary();
    emit_do_not_touch_first_summary();
}

pub fn dump_crash_capsule() {
    serial::write_bytes(b"== crash-capsule ==\n");
    emit_crash_capsule();
}

pub fn chronoscope_snapshot() -> ChronoscopeBundle {
    build_chronoscope_bundle(&crash_capsule())
}

fn chronoscope_payload_hash(a: u64, b: u64, c: u64, d: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    hash = fnv_mix(hash, a);
    hash = fnv_mix(hash, b);
    hash = fnv_mix(hash, c);
    fnv_mix(hash, d)
}

#[inline(always)]
fn chronoscope_mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn chronoscope_stable_node_id(
    kind: ChronoscopeNodeKind,
    cpu_slot: u16,
    request_id: u64,
    completion_id: u64,
    irq_id: u64,
    semantic_hash: u64,
) -> ChronoscopeStableNodeId {
    let lane0_seed = chronoscope_payload_hash(
        kind as u64,
        ((cpu_slot as u64) << 48) ^ request_id,
        completion_id,
        irq_id ^ semantic_hash.rotate_left(17),
    );
    let lane1_seed = chronoscope_payload_hash(
        semantic_hash ^ 0x9e37_79b9_7f4a_7c15,
        request_id.rotate_left(13) ^ completion_id.rotate_right(7),
        irq_id.rotate_left(29) ^ (kind as u64),
        (cpu_slot as u64) ^ 0xd6e8_feb8_6659_fd93,
    );
    let lo = chronoscope_mix64(lane0_seed ^ semantic_hash.rotate_right(11));
    let hi = chronoscope_mix64(lane1_seed ^ semantic_hash.rotate_left(9));
    ((hi as u128) << 64) | (lo as u128)
}

fn chronoscope_stable_checkpoint_id(
    kind: ChronoscopeCheckpointKind,
    correlation: CorrelationKey,
    causal_depth: u32,
    semantic_hash: u64,
) -> u128 {
    let lane0 = chronoscope_payload_hash(
        kind as u64,
        correlation.request_id,
        correlation.completion_id,
        correlation.irq_id ^ semantic_hash,
    );
    let lane1 = chronoscope_payload_hash(
        semantic_hash.rotate_left(7),
        causal_depth as u64,
        correlation.request_id.rotate_left(11) ^ correlation.completion_id.rotate_right(3),
        correlation.irq_id ^ 0x517c_c1a0_5eed_1234,
    );
    ((chronoscope_mix64(lane1) as u128) << 64) | (chronoscope_mix64(lane0) as u128)
}

fn chronoscope_stable_lineage_id(
    domain: ChronoscopeLineageDomain,
    key: u64,
    prior: ChronoscopeCheckpointId,
    transition_node: ChronoscopeNodeId,
    result: ChronoscopeCheckpointId,
) -> u128 {
    let lane0 = chronoscope_payload_hash(domain as u64, key, prior.0 as u64, transition_node);
    let lane1 = chronoscope_payload_hash(
        result.0 as u64,
        transition_node.rotate_left(13),
        key.rotate_right(7),
        0xa076_1d64_78bd_642f,
    );
    ((chronoscope_mix64(lane1) as u128) << 64) | (chronoscope_mix64(lane0) as u128)
}

fn chronoscope_finalize_bundle(bundle: &mut ChronoscopeBundle) {
    let fault_node_id = bundle.primary_fault_node();
    for index in 0..bundle.nodes.len() {
        if !bundle.nodes[index].valid {
            continue;
        }
        let node_id = bundle.nodes[index].node_id;
        bundle.nodes[index].causal_distance_to_fault =
            chronoscope_distance_to_node(bundle, node_id, fault_node_id);
        bundle.nodes[index].evidence_count = bundle
            .edges
            .iter()
            .filter(|edge| edge.valid && edge.dst_node_id == node_id)
            .count() as u32;
    }
    bundle.dominant_suspect_node_id = chronoscope_select_dominant_suspect(bundle);
    bundle.top_suspect_confidence = bundle
        .node_by_id(bundle.dominant_suspect_node_id)
        .map(|node| (node.confidence * 100.0) as u16)
        .unwrap_or(bundle.top_suspect_confidence);
    bundle.strongest_chain =
        chronoscope_build_chain_to_fault(bundle, bundle.dominant_suspect_node_id);
}

fn chronoscope_select_dominant_suspect(bundle: &ChronoscopeBundle) -> ChronoscopeNodeId {
    let candidates: [Option<&ChronoscopeNode>; CHRONOSCOPE_NODE_LIMIT] =
        core::array::from_fn(|index| {
            let node = &bundle.nodes[index];
            if node.valid && node.kind == ChronoscopeNodeKind::Interpretation {
                Some(node)
            } else {
                None
            }
        });
    let max_evidence = candidates
        .iter()
        .flatten()
        .map(|node| node.evidence_count)
        .max()
        .unwrap_or(1);
    let max_distance = candidates
        .iter()
        .flatten()
        .filter_map(|node| {
            if node.causal_distance_to_fault == u32::MAX {
                None
            } else {
                Some(node.causal_distance_to_fault)
            }
        })
        .max()
        .unwrap_or(1);
    candidates
        .iter()
        .flatten()
        .max_by(|left, right| {
            let left_score = chronoscope_rank_score(left, max_evidence, max_distance);
            let right_score = chronoscope_rank_score(right, max_evidence, max_distance);
            left_score
                .partial_cmp(&right_score)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then_with(|| left.stable_id.cmp(&right.stable_id))
        })
        .map(|node| node.node_id)
        .unwrap_or(0)
}

fn chronoscope_rank_score(node: &ChronoscopeNode, max_evidence: u32, max_distance: u32) -> f32 {
    let confidence = node.confidence.clamp(0.0, 1.0);
    let severity = (node.severity.min(5) as f32) / 5.0;
    let evidence = if max_evidence == 0 {
        0.0
    } else {
        (node.evidence_count.min(max_evidence) as f32) / (max_evidence as f32)
    };
    let proximity = if node.causal_distance_to_fault == u32::MAX || max_distance == 0 {
        0.0
    } else {
        1.0 - ((node.causal_distance_to_fault.min(max_distance) as f32) / (max_distance as f32))
    };
    // Balanced normalized score. No component dominates due to raw scale.
    confidence * 0.40 + severity * 0.20 + evidence * 0.20 + proximity * 0.20
}

fn chronoscope_distance_to_node(
    bundle: &ChronoscopeBundle,
    start: ChronoscopeNodeId,
    goal: ChronoscopeNodeId,
) -> u32 {
    if start == 0 || goal == 0 {
        return u32::MAX;
    }
    if start == goal {
        return 0;
    }
    let mut queue = [0u64; CHRONOSCOPE_NODE_LIMIT];
    let mut dist = [u32::MAX; CHRONOSCOPE_NODE_LIMIT];
    let mut head = 0usize;
    let mut tail = 0usize;
    queue[tail] = start;
    tail += 1;
    if let Some(index) = bundle.node_index_by_id(start) {
        dist[index] = 0;
    }
    while head < tail {
        let current = queue[head];
        head += 1;
        let current_idx = match bundle.node_index_by_id(current) {
            Some(index) => index,
            None => continue,
        };
        let current_dist = dist[current_idx];
        let mut edge_index = 0usize;
        while edge_index < bundle.edges.len() {
            let edge = bundle.edges[edge_index];
            edge_index += 1;
            if !edge.valid || edge.src_node_id != current {
                continue;
            }
            if let Some(next_idx) = bundle.node_index_by_id(edge.dst_node_id) {
                if dist[next_idx] != u32::MAX {
                    continue;
                }
                dist[next_idx] = current_dist.saturating_add(1);
                if edge.dst_node_id == goal {
                    return dist[next_idx];
                }
                if tail < queue.len() {
                    queue[tail] = edge.dst_node_id;
                    tail += 1;
                }
            }
        }
    }
    u32::MAX
}

fn chronoscope_build_chain_to_fault(
    bundle: &ChronoscopeBundle,
    start: ChronoscopeNodeId,
) -> [ChronoscopeNodeId; 8] {
    let fault = bundle.primary_fault_node();
    let mut out = [0u64; 8];
    if start == 0 || fault == 0 {
        return out;
    }
    out[0] = start;
    let mut cursor = start;
    let mut slot = 1usize;
    while slot < out.len() && cursor != fault {
        let mut next = 0u64;
        let mut next_distance = u32::MAX;
        let mut edge_index = 0usize;
        while edge_index < bundle.edges.len() {
            let edge = bundle.edges[edge_index];
            edge_index += 1;
            if !edge.valid || edge.src_node_id != cursor {
                continue;
            }
            let distance = bundle
                .node_by_id(edge.dst_node_id)
                .map(|node| node.causal_distance_to_fault)
                .unwrap_or(u32::MAX);
            if distance < next_distance {
                next_distance = distance;
                next = edge.dst_node_id;
            }
        }
        if next == 0 || next == cursor {
            break;
        }
        out[slot] = next;
        cursor = next;
        slot += 1;
    }
    out
}

impl ChronoscopeBundle {
    #[inline(always)]
    fn direct_node_index(&self, node_id: ChronoscopeNodeId) -> Option<usize> {
        if node_id == 0 {
            return None;
        }
        let index = (node_id - 1) as usize;
        if index < self.nodes.len() {
            let node = self.nodes[index];
            if node.valid && node.node_id == node_id {
                return Some(index);
            }
        }
        None
    }

    fn node_index_by_id(&self, node_id: ChronoscopeNodeId) -> Option<usize> {
        if let Some(index) = self.direct_node_index(node_id) {
            return Some(index);
        }
        let mut index = 0usize;
        while index < self.nodes.len() {
            let node = self.nodes[index];
            if node.valid && node.node_id == node_id {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn node_by_id(&self, node_id: ChronoscopeNodeId) -> Option<&ChronoscopeNode> {
        if let Some(index) = self.direct_node_index(node_id) {
            return Some(&self.nodes[index]);
        }
        let mut index = 0usize;
        while index < self.nodes.len() {
            let node = &self.nodes[index];
            if node.valid && node.node_id == node_id {
                return Some(node);
            }
            index += 1;
        }
        None
    }

    pub fn primary_fault_node(&self) -> ChronoscopeNodeId {
        let mut index = 0usize;
        while index < self.nodes.len() {
            let node = self.nodes[index];
            if node.valid && node.kind == ChronoscopeNodeKind::Outcome {
                return node.node_id;
            }
            index += 1;
        }
        0
    }

    pub fn dominant_suspect_chain(&self) -> ChronoscopeNodeList<8> {
        let mut out = ChronoscopeNodeList::EMPTY;
        let mut index = 0usize;
        while index < self.strongest_chain.len() {
            out.push(self.strongest_chain[index]);
            index += 1;
        }
        out
    }

    pub fn earliest_preventable_boundary(&self) -> Option<ChronoscopeNodeId> {
        let mut index = 0usize;
        while index < self.nodes.len() {
            let node = self.nodes[index];
            if node.valid
                && node.kind == ChronoscopeNodeKind::Boundary
                && node.path == self.root_boundary.path
                && node.stage == self.root_boundary.stage
            {
                return Some(node.node_id);
            }
            index += 1;
        }
        None
    }

    pub fn cross_core_divergence_chain(
        &self,
        key: CorrelationKey,
    ) -> ChronoscopeNodeList<CHRONOSCOPE_NODE_LIMIT> {
        let mut out = ChronoscopeNodeList::EMPTY;
        let mut index = 0usize;
        while index < self.nodes.len() {
            let node = self.nodes[index];
            index += 1;
            if !node.valid {
                continue;
            }
            if !((key.request_id != 0 && node.request_id == key.request_id)
                || (key.completion_id != 0 && node.completion_id == key.completion_id)
                || (key.irq_id != 0 && node.irq_id == key.irq_id))
            {
                continue;
            }
            if node.kind != ChronoscopeNodeKind::Observation
                && node.kind != ChronoscopeNodeKind::Interpretation
            {
                continue;
            }
            out.push(node.node_id);
        }
        out
    }

    pub fn supporting_nodes(
        &self,
        node: ChronoscopeNodeId,
    ) -> ChronoscopeNodeList<CHRONOSCOPE_EDGE_LIMIT> {
        let mut out = ChronoscopeNodeList::EMPTY;
        let mut index = 0usize;
        while index < self.edges.len() {
            let edge = self.edges[index];
            index += 1;
            if !edge.valid || edge.dst_node_id != node {
                continue;
            }
            if edge.kind != ChronoscopeEdgeKind::Supports
                && edge.kind != ChronoscopeEdgeKind::Explains
                && edge.kind != ChronoscopeEdgeKind::ObservedBefore
            {
                continue;
            }
            out.push(edge.src_node_id);
        }
        out
    }

    pub fn diff_against(&self, previous: &ChronoscopeBundle) -> ChronoscopeDiff {
        PERF_DIFF_EXECUTIONS.fetch_add(1, Ordering::Relaxed);
        let mut diff = ChronoscopeDiff::EMPTY;
        let (mut current, current_len) = chronoscope_collect_diff_entries(self);
        let (mut baseline, baseline_len) = chronoscope_collect_diff_entries(previous);
        chronoscope_sort_diff_entries(&mut current, current_len);
        chronoscope_sort_diff_entries(&mut baseline, baseline_len);

        let mut left = 0usize;
        let mut right = 0usize;
        while left < current_len && right < baseline_len {
            let current_entry = current[left];
            let baseline_entry = baseline[right];
            if current_entry.stable_id == baseline_entry.stable_id {
                let common_slot = diff.summary.common_nodes as usize;
                if common_slot < diff.common_nodes.len() {
                    diff.common_nodes[common_slot] = current_entry.stable_id;
                    diff.summary.common_nodes = diff.summary.common_nodes.saturating_add(1);
                }
                if current_entry.path != baseline_entry.path {
                    let changed_slot = diff.summary.changed_paths as usize;
                    if changed_slot < diff.changed_path_heads.len() {
                        diff.changed_path_heads[changed_slot] = current_entry.stable_id;
                        diff.summary.changed_paths = diff.summary.changed_paths.saturating_add(1);
                    }
                }
                left += 1;
                right += 1;
            } else if current_entry.stable_id < baseline_entry.stable_id {
                let new_slot = diff.summary.new_nodes as usize;
                if new_slot < diff.new_nodes.len() {
                    if diff.summary.new_nodes == 0 {
                        diff.first_divergence_stable_id = current_entry.stable_id;
                    }
                    diff.new_nodes[new_slot] = current_entry.stable_id;
                    diff.summary.new_nodes = diff.summary.new_nodes.saturating_add(1);
                }
                left += 1;
            } else {
                let missing_slot = diff.summary.missing_nodes as usize;
                if missing_slot < diff.missing_nodes.len() {
                    diff.missing_nodes[missing_slot] = baseline_entry.stable_id;
                    diff.summary.missing_nodes = diff.summary.missing_nodes.saturating_add(1);
                }
                right += 1;
            }
        }
        while left < current_len {
            let new_slot = diff.summary.new_nodes as usize;
            if new_slot < diff.new_nodes.len() {
                if diff.summary.new_nodes == 0 {
                    diff.first_divergence_stable_id = current[left].stable_id;
                }
                diff.new_nodes[new_slot] = current[left].stable_id;
                diff.summary.new_nodes = diff.summary.new_nodes.saturating_add(1);
            }
            left += 1;
        }
        while right < baseline_len {
            let missing_slot = diff.summary.missing_nodes as usize;
            if missing_slot < diff.missing_nodes.len() {
                diff.missing_nodes[missing_slot] = baseline[right].stable_id;
                diff.summary.missing_nodes = diff.summary.missing_nodes.saturating_add(1);
            }
            right += 1;
        }
        let checkpoint_ids: [u128; CHRONOSCOPE_CHECKPOINT_LIMIT] =
            core::array::from_fn(|index| self.checkpoints[index].stable_id);
        let prev_checkpoint_ids: [u128; CHRONOSCOPE_CHECKPOINT_LIMIT] =
            core::array::from_fn(|index| previous.checkpoints[index].stable_id);
        let (mut checkpoints, checkpoints_len) =
            chronoscope_collect_temporal_entries(&checkpoint_ids);
        let (mut prev_checkpoints, prev_checkpoints_len) =
            chronoscope_collect_temporal_entries(&prev_checkpoint_ids);
        chronoscope_sort_temporal_entries(&mut checkpoints, checkpoints_len);
        chronoscope_sort_temporal_entries(&mut prev_checkpoints, prev_checkpoints_len);
        let mut cleft = 0usize;
        let mut cright = 0usize;
        while cleft < checkpoints_len && cright < prev_checkpoints_len {
            let current_entry = checkpoints[cleft];
            let baseline_entry = prev_checkpoints[cright];
            if current_entry.stable_id == baseline_entry.stable_id {
                let slot = diff.summary.common_checkpoints as usize;
                if slot < diff.common_checkpoints.len() {
                    diff.common_checkpoints[slot] = current_entry.stable_id;
                    diff.summary.common_checkpoints =
                        diff.summary.common_checkpoints.saturating_add(1);
                }
                cleft += 1;
                cright += 1;
            } else if current_entry.stable_id < baseline_entry.stable_id {
                let slot = diff.summary.new_checkpoints as usize;
                if slot < diff.new_checkpoints.len() {
                    diff.new_checkpoints[slot] = current_entry.stable_id;
                    diff.summary.new_checkpoints = diff.summary.new_checkpoints.saturating_add(1);
                }
                if diff.first_temporal_divergence == 0 {
                    diff.first_temporal_divergence = current_entry.stable_id;
                }
                cleft += 1;
            } else {
                let slot = diff.summary.missing_checkpoints as usize;
                if slot < diff.missing_checkpoints.len() {
                    diff.missing_checkpoints[slot] = baseline_entry.stable_id;
                    diff.summary.missing_checkpoints =
                        diff.summary.missing_checkpoints.saturating_add(1);
                }
                if diff.first_temporal_divergence == 0 {
                    diff.first_temporal_divergence = baseline_entry.stable_id;
                }
                cright += 1;
            }
        }
        while cleft < checkpoints_len {
            let slot = diff.summary.new_checkpoints as usize;
            if slot < diff.new_checkpoints.len() {
                diff.new_checkpoints[slot] = checkpoints[cleft].stable_id;
                diff.summary.new_checkpoints = diff.summary.new_checkpoints.saturating_add(1);
            }
            if diff.first_temporal_divergence == 0 {
                diff.first_temporal_divergence = checkpoints[cleft].stable_id;
            }
            cleft += 1;
        }
        while cright < prev_checkpoints_len {
            let slot = diff.summary.missing_checkpoints as usize;
            if slot < diff.missing_checkpoints.len() {
                diff.missing_checkpoints[slot] = prev_checkpoints[cright].stable_id;
                diff.summary.missing_checkpoints =
                    diff.summary.missing_checkpoints.saturating_add(1);
            }
            if diff.first_temporal_divergence == 0 {
                diff.first_temporal_divergence = prev_checkpoints[cright].stable_id;
            }
            cright += 1;
        }

        let lineage_ids: [u128; CHRONOSCOPE_LINEAGE_LIMIT] =
            core::array::from_fn(|index| self.lineage[index].stable_id);
        let prev_lineage_ids: [u128; CHRONOSCOPE_LINEAGE_LIMIT] =
            core::array::from_fn(|index| previous.lineage[index].stable_id);
        let (mut lineage, lineage_len) = chronoscope_collect_temporal_entries(&lineage_ids);
        let (mut prev_lineage, prev_lineage_len) =
            chronoscope_collect_temporal_entries(&prev_lineage_ids);
        chronoscope_sort_temporal_entries(&mut lineage, lineage_len);
        chronoscope_sort_temporal_entries(&mut prev_lineage, prev_lineage_len);
        let mut lleft = 0usize;
        let mut lright = 0usize;
        while lleft < lineage_len && lright < prev_lineage_len {
            let current_entry = lineage[lleft];
            let baseline_entry = prev_lineage[lright];
            if current_entry.stable_id == baseline_entry.stable_id {
                let slot = diff.summary.common_lineage as usize;
                if slot < diff.common_lineage.len() {
                    diff.common_lineage[slot] = current_entry.stable_id;
                    diff.summary.common_lineage = diff.summary.common_lineage.saturating_add(1);
                }
                lleft += 1;
                lright += 1;
            } else if current_entry.stable_id < baseline_entry.stable_id {
                let slot = diff.summary.new_lineage as usize;
                if slot < diff.new_lineage.len() {
                    diff.new_lineage[slot] = current_entry.stable_id;
                    diff.summary.new_lineage = diff.summary.new_lineage.saturating_add(1);
                }
                lleft += 1;
            } else {
                let slot = diff.summary.missing_lineage as usize;
                if slot < diff.missing_lineage.len() {
                    diff.missing_lineage[slot] = baseline_entry.stable_id;
                    diff.summary.missing_lineage = diff.summary.missing_lineage.saturating_add(1);
                }
                lright += 1;
            }
        }
        while lleft < lineage_len {
            let slot = diff.summary.new_lineage as usize;
            if slot < diff.new_lineage.len() {
                diff.new_lineage[slot] = lineage[lleft].stable_id;
                diff.summary.new_lineage = diff.summary.new_lineage.saturating_add(1);
            }
            lleft += 1;
        }
        while lright < prev_lineage_len {
            let slot = diff.summary.missing_lineage as usize;
            if slot < diff.missing_lineage.len() {
                diff.missing_lineage[slot] = prev_lineage[lright].stable_id;
                diff.summary.missing_lineage = diff.summary.missing_lineage.saturating_add(1);
            }
            lright += 1;
        }

        diff.changed_rewind_candidate = self.rewind_checkpoint != previous.rewind_checkpoint;
        diff.changed_mutation_path = self.temporal_explain.last_mutation
            != previous.temporal_explain.last_mutation
            || self.strongest_chain != previous.strongest_chain;
        diff.changed_last_writer =
            self.temporal_explain.last_writer != previous.temporal_explain.last_writer;
        diff.changed_capability_lineage =
            self.temporal_explain.capability_chain != previous.temporal_explain.capability_chain;
        diff.changed_propagation_path =
            self.temporal_explain.propagation_path != previous.temporal_explain.propagation_path;
        diff.changed_responsibility_ranking = self.temporal_explain.responsibility_ranking
            != previous.temporal_explain.responsibility_ranking;
        diff.changed_adaptive_state = self.adaptive_state != previous.adaptive_state;
        if self.perf.schema_version != previous.perf.schema_version {
            diff.first_adaptive_divergence = CHRONOSCOPE_SCHEMA_VERSION as u64;
        }
        diff.changed_escalation_target = self
            .escalations
            .iter()
            .copied()
            .find(|entry| entry.valid)
            .map(|entry| entry.target_core_mask)
            != previous
                .escalations
                .iter()
                .copied()
                .find(|entry| entry.valid)
                .map(|entry| entry.target_core_mask);
        diff.first_adaptive_divergence = self
            .anomalies
            .iter()
            .copied()
            .find(|entry| entry.valid)
            .map(|entry| entry.first_event.0)
            .filter(|left| {
                Some(*left)
                    != previous
                        .anomalies
                        .iter()
                        .copied()
                        .find(|entry| entry.valid)
                        .map(|entry| entry.first_event.0)
            })
            .unwrap_or(0);
        diff.summary.changed_last_writer = diff.changed_last_writer as u16;
        diff.summary.changed_capability_lineage = diff.changed_capability_lineage as u16;
        diff.summary.changed_propagation = diff.changed_propagation_path as u16;
        diff.summary.changed_responsibility = diff.changed_responsibility_ranking as u16;
        diff.summary.changed_anomalies = (self.anomalies != previous.anomalies) as u16;
        diff.summary.changed_escalations = (self.escalations != previous.escalations) as u16;
        diff.summary.changed_capture_windows =
            (self.capture_windows != previous.capture_windows) as u16;
        diff.summary.changed_candidates = (self.candidates != previous.candidates) as u16;
        if !self.integrity.complete || !previous.integrity.complete {
            diff.changed_mutation_path = true;
            diff.changed_propagation_path = true;
        }
        diff
    }

    pub fn build_explain_plan(&self) -> ExplainPlan {
        let fault_node = self.primary_fault_node();
        let primary_cause = self.dominant_suspect_node_id;
        let mut competing = [0u64; SUSPECT_LIMIT];
        let mut next = 0usize;
        let mut index = 0usize;
        while index < self.nodes.len() && next < competing.len() {
            let node = self.nodes[index];
            index += 1;
            if node.valid
                && node.kind == ChronoscopeNodeKind::Interpretation
                && node.node_id != primary_cause
            {
                competing[next] = node.node_id;
                next += 1;
            }
        }
        let mut divergence = [0u64; 4];
        let mut dnext = 0usize;
        let mut dindex = 0usize;
        while dindex < self.nodes.len() && dnext < divergence.len() {
            let node = self.nodes[dindex];
            dindex += 1;
            if node.valid
                && node.kind == ChronoscopeNodeKind::Observation
                && self.supporting_nodes(node.node_id).len > 1
            {
                divergence[dnext] = node.node_id;
                dnext += 1;
            }
        }
        ExplainPlan {
            valid: fault_node != 0,
            primary_cause,
            fault_node,
            earliest_preventable_boundary: self.earliest_preventable_boundary().unwrap_or(0),
            causal_chain: self.strongest_chain,
            competing_suspects: competing,
            cross_core_divergence: divergence,
            confidence: self
                .node_by_id(primary_cause)
                .map(|node| node.confidence)
                .unwrap_or(0.0),
            state_before_fault: ChronoscopeCheckpointId::NONE,
            last_mutation: 0,
            rewind_candidate: ChronoscopeCheckpointId::NONE,
            divergence_origin: ChronoscopeCheckpointId::NONE,
            lineage_summary: [ChronoscopeLineageId::NONE; CHRONOSCOPE_LINEAGE_SUMMARY_LIMIT],
            temporal_confidence: 0,
            last_writer: 0,
            writer_chain: [0; CHRONOSCOPE_TEMPORAL_PATH_LIMIT],
            capability_chain: [CapabilityId::NONE; CHRONOSCOPE_LINEAGE_SUMMARY_LIMIT],
            propagation_path: [0; CHRONOSCOPE_TEMPORAL_PATH_LIMIT],
            responsibility_ranking: [ResponsibilityEntry::EMPTY; 3],
            responsibility_confidence: 0,
            replay_summary: ReplaySummary::EMPTY,
            replay_steps_count: 0,
            first_divergence_point: ChronoscopeEventId::NONE,
            replay_confidence: 0,
            dominant_anomaly: ChronoscopeAnomalyId::NONE,
            adaptive_state: ChronoscopeAdaptiveState::Normal,
            escalation_id: ChronoscopeEscalationId::NONE,
            candidate_node: 0,
            downgrade_ready: false,
            anomaly_confidence: 0,
        }
    }

    fn checkpoint_by_id(
        &self,
        checkpoint_id: ChronoscopeCheckpointId,
    ) -> Option<&ChronoscopeCheckpoint> {
        if checkpoint_id == ChronoscopeCheckpointId::NONE {
            return None;
        }
        let mut index = 0usize;
        while index < self.checkpoints.len() {
            let checkpoint = &self.checkpoints[index];
            if checkpoint.valid && checkpoint.checkpoint_id == checkpoint_id {
                return Some(checkpoint);
            }
            index += 1;
        }
        None
    }

    fn checkpoint_index_by_id(&self, checkpoint_id: ChronoscopeCheckpointId) -> Option<usize> {
        if checkpoint_id == ChronoscopeCheckpointId::NONE {
            return None;
        }
        let mut index = 0usize;
        while index < self.checkpoints.len() {
            let checkpoint = self.checkpoints[index];
            if checkpoint.valid && checkpoint.checkpoint_id == checkpoint_id {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    pub fn state_before_fault(&self) -> Option<ChronoscopeCheckpointId> {
        self.checkpoint_by_id(self.primary_fault_checkpoint)
            .map(|checkpoint| checkpoint.predecessor)
            .filter(|checkpoint| *checkpoint != ChronoscopeCheckpointId::NONE)
    }

    pub fn rewind_candidate(&self) -> Option<ChronoscopeCheckpointId> {
        if self.rewind_checkpoint == ChronoscopeCheckpointId::NONE {
            None
        } else {
            Some(self.rewind_checkpoint)
        }
    }

    pub fn divergence_origin(&self) -> Option<ChronoscopeCheckpointId> {
        if self.divergence_checkpoint == ChronoscopeCheckpointId::NONE {
            None
        } else {
            Some(self.divergence_checkpoint)
        }
    }

    pub fn predecessor_state(
        &self,
        checkpoint: ChronoscopeCheckpointId,
    ) -> Option<ChronoscopeCheckpointId> {
        self.checkpoint_by_id(checkpoint)
            .map(|entry| entry.predecessor)
            .filter(|entry| *entry != ChronoscopeCheckpointId::NONE)
    }

    pub fn last_mutation_of(
        &self,
        domain: ChronoscopeLineageDomain,
        key: u64,
    ) -> Option<ChronoscopeNodeId> {
        let mut best_node = 0u64;
        let mut best_seq = 0u64;
        let mut index = 0usize;
        while index < self.lineage.len() {
            let record = self.lineage[index];
            index += 1;
            if !record.valid || record.domain != domain || record.key != key {
                continue;
            }
            if let Some(node) = self.node_by_id(record.transition_node) {
                if node.event_sequence >= best_seq {
                    best_seq = node.event_sequence;
                    best_node = node.node_id;
                }
            }
        }
        if best_node == 0 {
            None
        } else {
            Some(best_node)
        }
    }

    pub fn minimal_transition_path_to_fault(&self) -> Vec<ChronoscopeNodeId> {
        let mut out = Vec::new();
        let mut index = 0usize;
        while index < self.strongest_chain.len() {
            let node_id = self.strongest_chain[index];
            if node_id != 0 {
                out.push(node_id);
            }
            index += 1;
        }
        out
    }

    pub fn lineage_for_node(&self, node: ChronoscopeNodeId) -> Vec<ChronoscopeLineageId> {
        let mut out = Vec::new();
        let mut index = 0usize;
        while index < self.lineage.len() {
            let record = self.lineage[index];
            index += 1;
            if !record.valid {
                continue;
            }
            if record.prior_node == node
                || record.transition_node == node
                || record.result_node == node
            {
                out.push(record.lineage_id);
            }
        }
        out
    }

    pub fn capability_for_node(&self, node: ChronoscopeNodeId) -> Option<CapabilityId> {
        if let Some(index) = self.direct_node_index(node) {
            let capability = self.node_capabilities[index];
            if capability != CapabilityId::NONE {
                return Some(capability);
            }
        }
        None
    }

    pub fn last_writer_of(
        &self,
        domain: ChronoscopeLineageDomain,
        key: u64,
    ) -> Option<ChronoscopeNodeId> {
        let mut probe = ((key ^ ((domain as u64) << 32)) as usize) % self.last_writer_index.len();
        let mut steps = 0usize;
        while steps < self.last_writer_index.len() {
            let entry = self.last_writer_index[probe];
            if !entry.valid {
                return None;
            }
            if entry.domain == domain && entry.key == key {
                let record_index = entry.record_index as usize;
                if record_index < self.last_writers.len() {
                    let record = self.last_writers[record_index];
                    if record.valid {
                        return Some(record.last_writer_node);
                    }
                }
                return None;
            }
            probe = (probe + 1) % self.last_writer_index.len();
            steps += 1;
        }
        None
    }

    pub fn writer_chain_to_fault(
        &self,
        domain: ChronoscopeLineageDomain,
        key: u64,
    ) -> Vec<ChronoscopeNodeId> {
        let mut out = Vec::new();
        let mut current = self.last_writer_of(domain, key).unwrap_or(0);
        let mut guard = 0usize;
        while current != 0 {
            out.push(current);
            current = self
                .direct_node_index(current)
                .map(|index| self.writer_predecessor_by_node[index])
                .unwrap_or(0);
            guard += 1;
            if guard >= CHRONOSCOPE_NODE_LIMIT {
                break;
            }
        }
        let fault = self.primary_fault_node();
        if fault != 0 && out.last().copied().unwrap_or(0) != fault {
            out.push(fault);
        }
        out
    }

    pub fn capability_origin(&self, cap: CapabilityId) -> CapabilityId {
        if cap == CapabilityId::NONE {
            return CapabilityId::NONE;
        }
        let mut current = cap;
        loop {
            let parent = self.capability_parent(current);
            if parent == CapabilityId::NONE {
                return current;
            }
            current = parent;
        }
    }

    pub fn capability_chain(&self, cap: CapabilityId) -> Vec<CapabilityId> {
        let mut out = Vec::new();
        if cap == CapabilityId::NONE {
            return out;
        }
        let mut chain = [CapabilityId::NONE; CHRONOSCOPE_CAPABILITY_LIMIT];
        let mut len = 0usize;
        let mut current = cap;
        while current != CapabilityId::NONE && len < chain.len() {
            chain[len] = current;
            len += 1;
            current = self.capability_parent(current);
        }
        let mut index = len;
        while index > 0 {
            index -= 1;
            out.push(chain[index]);
        }
        out
    }

    pub fn capability_usage(&self, cap: CapabilityId) -> Vec<ChronoscopeNodeId> {
        let mut out = Vec::new();
        let mut index = 0usize;
        while index < self.last_writers.len() {
            let entry = self.last_writers[index];
            index += 1;
            if entry.valid && entry.capability_id == cap {
                out.push(entry.last_writer_node);
            }
        }
        out
    }

    pub fn propagation_chain_to_fault(&self, node_id: ChronoscopeNodeId) -> Vec<ChronoscopeNodeId> {
        let mut out = Vec::new();
        if node_id == 0 {
            return out;
        }
        out.push(node_id);
        let fault = self.primary_fault_node();
        let mut current = node_id;
        let mut guard = 0usize;
        while current != fault && guard < CHRONOSCOPE_PROPAGATION_LIMIT {
            let mut next = 0u64;
            let mut next_distance = u32::MAX;
            let mut edge_index = self
                .direct_node_index(current)
                .map(|index| self.propagation_heads[index])
                .unwrap_or(CHRONOSCOPE_INVALID_INDEX);
            while edge_index != CHRONOSCOPE_INVALID_INDEX {
                let entry = self.propagation[edge_index as usize];
                let distance = self
                    .node_by_id(entry.target_node)
                    .map(|node| node.causal_distance_to_fault)
                    .unwrap_or(u32::MAX);
                if distance < next_distance {
                    next_distance = distance;
                    next = entry.target_node;
                }
                edge_index = self.propagation_next[edge_index as usize];
            }
            if next == 0 || next == current {
                break;
            }
            out.push(next);
            current = next;
            guard += 1;
        }
        out
    }

    pub fn dominant_propagation_path(&self) -> Vec<ChronoscopeNodeId> {
        self.propagation_chain_to_fault(self.primary_responsible_node())
    }

    pub fn primary_responsible_node(&self) -> ChronoscopeNodeId {
        let mut best_node = 0;
        let mut best_score = 0u16;
        let mut index = 0usize;
        while index < self.responsibility.len() {
            let entry = self.responsibility[index];
            index += 1;
            if !entry.valid {
                continue;
            }
            if entry.score > best_score
                || (entry.score == best_score && (best_node == 0 || entry.node_id < best_node))
            {
                best_score = entry.score;
                best_node = entry.node_id;
            }
        }
        if best_node != 0 {
            best_node
        } else {
            self.dominant_suspect_node_id
        }
    }

    pub fn responsibility_ranking(&self) -> Vec<(ChronoscopeNodeId, u16)> {
        let mut out = Vec::new();
        let mut index = 0usize;
        while index < self.responsibility.len() {
            let entry = self.responsibility[index];
            index += 1;
            if entry.valid {
                out.push((entry.node_id, entry.score));
            }
        }
        out.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        out
    }

    #[inline(always)]
    fn capability_parent(&self, cap: CapabilityId) -> CapabilityId {
        if cap == CapabilityId::NONE {
            return CapabilityId::NONE;
        }
        let mut probe = (cap.0 as usize) % self.capability_parent_index.len();
        let mut steps = 0usize;
        while steps < self.capability_parent_index.len() {
            let entry = self.capability_parent_index[probe];
            if !entry.valid {
                return CapabilityId::NONE;
            }
            if entry.capability == cap {
                let record_index = entry.record_index as usize;
                if record_index < self.capability_derivations.len() {
                    let record = self.capability_derivations[record_index];
                    if record.valid {
                        return record.parent;
                    }
                }
                return CapabilityId::NONE;
            }
            probe = (probe + 1) % self.capability_parent_index.len();
            steps += 1;
        }
        CapabilityId::NONE
    }

    pub fn replay_sequence(&self) -> ReplaySequence {
        replay_sequence_from_runtime(&self.runtime_events)
    }

    pub fn rewind_to(&self, checkpoint_id: ChronoscopeCheckpointId) -> ReplayCursor {
        let sequence = self.replay_sequence();
        if !sequence.valid {
            return ReplayCursor::EMPTY;
        }
        let checkpoint = self.checkpoint_by_id(checkpoint_id);
        let start_index = checkpoint
            .map(|checkpoint| checkpoint.replay_start_index)
            .filter(|index| *index != CHRONOSCOPE_INVALID_INDEX)
            .unwrap_or(0);
        let mut cursor = ReplayCursor {
            valid: true,
            sequence,
            global_position: 0,
            per_core_positions: [0; MAX_TRACE_CPUS],
            state: ReplaySemanticState::EMPTY,
            checkpoint_id,
        };
        while cursor.global_position < start_index {
            if self.step(&mut cursor).is_none() {
                break;
            }
        }
        cursor
    }

    pub fn step(&self, cursor: &mut ReplayCursor) -> Option<ReplayEvent> {
        if !cursor.valid || cursor.global_position as usize >= cursor.sequence.total_events as usize
        {
            return None;
        }
        let event = cursor.sequence.events[cursor.global_position as usize];
        if !event.valid {
            return None;
        }
        debug_assert!((event.core_id as usize) < cursor.per_core_positions.len());
        debug_assert!(
            cursor.per_core_positions[event.core_id as usize] as u64 <= event.local_sequence
        );
        apply_replay_event(&mut cursor.state, event);
        cursor.per_core_positions[event.core_id as usize] =
            cursor.per_core_positions[event.core_id as usize].saturating_add(1);
        cursor.global_position = cursor.global_position.saturating_add(1);
        Some(event)
    }

    pub fn run_until<P>(&self, cursor: &mut ReplayCursor, predicate: P) -> ReplayResult
    where
        P: Fn(ReplayEvent, &ReplaySemanticState) -> bool,
    {
        let mut result = ReplayResult::EMPTY;
        while let Some(event) = self.step(cursor) {
            result.valid = true;
            result.last_event = event;
            result.steps = result.steps.saturating_add(1);
            result.state = cursor.state;
            if predicate(event, &cursor.state) {
                break;
            }
        }
        result
    }

    pub fn run_n_steps(&self, cursor: &mut ReplayCursor, n: usize) -> ReplayResult {
        let mut result = ReplayResult::EMPTY;
        let mut steps = 0usize;
        while steps < n {
            let event = match self.step(cursor) {
                Some(event) => event,
                None => break,
            };
            result.valid = true;
            result.last_event = event;
            result.steps = result.steps.saturating_add(1);
            result.state = cursor.state;
            steps += 1;
        }
        result
    }

    pub fn replay_to_fault(&self) -> ReplayTrace {
        let mut cursor = self.rewind_to(
            self.state_before_fault()
                .unwrap_or(ChronoscopeCheckpointId::NONE),
        );
        let mut trace = ReplayTrace::EMPTY;
        trace.valid = cursor.valid;
        trace.partial = self.runtime_events.partial;
        while let Some(event) = self.step(&mut cursor) {
            let slot = trace.total_steps as usize;
            if slot >= trace.events.len() {
                trace.partial = true;
                break;
            }
            trace.events[slot] = event;
            trace.total_steps = trace.total_steps.saturating_add(1);
            if event.kind == ChronoscopeRuntimeEventKind::FaultMarker {
                break;
            }
        }
        trace.final_state = cursor.state;
        trace
    }

    pub fn replay_from_checkpoint(&self, checkpoint_id: ChronoscopeCheckpointId) -> ReplayTrace {
        let mut cursor = self.rewind_to(checkpoint_id);
        let mut trace = ReplayTrace::EMPTY;
        trace.valid = cursor.valid;
        trace.partial = self.runtime_events.partial;
        while let Some(event) = self.step(&mut cursor) {
            let slot = trace.total_steps as usize;
            if slot >= trace.events.len() {
                trace.partial = true;
                break;
            }
            trace.events[slot] = event;
            trace.total_steps = trace.total_steps.saturating_add(1);
        }
        trace.final_state = cursor.state;
        trace
    }

    pub fn replay_until_fault(&self) -> ReplayResult {
        let mut cursor = self.rewind_to(
            self.state_before_fault()
                .unwrap_or(ChronoscopeCheckpointId::NONE),
        );
        self.run_until(&mut cursor, |event, _| {
            event.kind == ChronoscopeRuntimeEventKind::FaultMarker
        })
    }

    pub fn replay_until_violation(&self) -> ReplayResult {
        let mut cursor = self.rewind_to(
            self.rewind_candidate()
                .unwrap_or(ChronoscopeCheckpointId::NONE),
        );
        self.run_until(&mut cursor, |event, state| {
            event.kind == ChronoscopeRuntimeEventKind::ViolationObserved
                || state.violation_flags != 0
        })
    }

    pub fn replay_path_for_node(&self, node_id: ChronoscopeNodeId) -> ReplayTrace {
        let mut trace = ReplayTrace::EMPTY;
        let node = match self.node_by_id(node_id) {
            Some(node) => *node,
            None => return trace,
        };
        let mut cursor = self.rewind_to(
            self.rewind_candidate()
                .unwrap_or(ChronoscopeCheckpointId::NONE),
        );
        trace.valid = cursor.valid;
        trace.partial = self.runtime_events.partial;
        while let Some(event) = self.step(&mut cursor) {
            if event.correlation.request_id == node.request_id
                && event.correlation.completion_id == node.completion_id
                && event.correlation.irq_id == node.irq_id
            {
                let slot = trace.total_steps as usize;
                if slot >= trace.events.len() {
                    trace.partial = true;
                    break;
                }
                trace.events[slot] = event;
                trace.total_steps = trace.total_steps.saturating_add(1);
            }
            if event.event_id.0 >= node.event_sequence {
                break;
            }
        }
        trace.final_state = cursor.state;
        trace
    }

    pub fn detect_divergence(&self, left: &ReplayTrace, right: &ReplayTrace) -> DivergenceResult {
        let mut result = DivergenceResult::EMPTY;
        let max = left.total_steps.min(right.total_steps) as usize;
        let mut index = 0usize;
        while index < max {
            let le = left.events[index];
            let re = right.events[index];
            if le.event_id != re.event_id || le.kind != re.kind || le.core_id != re.core_id {
                result.valid = true;
                result.first_event_mismatch = le.event_id;
                result.first_state_mismatch = index as u16;
                return result;
            }
            index += 1;
        }
        if left.final_state != right.final_state {
            result.valid = true;
            result.first_state_mismatch = max as u16;
            if left.final_state.violation_flags != right.final_state.violation_flags {
                result.first_violation_mismatch = max as u16;
            }
        }
        result
    }
}

fn chronoscope_checkpoint_kind_name(kind: ChronoscopeCheckpointKind) -> &'static str {
    match kind {
        ChronoscopeCheckpointKind::FaultAdjacent => "fault-adjacent",
        ChronoscopeCheckpointKind::PreBoundary => "pre-boundary",
        ChronoscopeCheckpointKind::Divergence => "divergence",
        ChronoscopeCheckpointKind::LastKnownGood => "last-known-good",
        ChronoscopeCheckpointKind::RewindCandidate => "rewind-candidate",
    }
}

fn chronoscope_temporal_fragment_for_node(
    node: &ChronoscopeNode,
    capsule: &CrashCapsule,
) -> ChronoscopeStateFragment {
    match node.kind {
        ChronoscopeNodeKind::Observation => ChronoscopeStateFragment::RequestPathState {
            stage: node.stage,
            path: node.path,
            request_id: node.request_id,
            completion_id: node.completion_id,
        },
        ChronoscopeNodeKind::Interpretation => {
            let mut reason_code = 0u16;
            let mut event_kind = 0u16;
            let mut index = 0usize;
            while index < capsule.suspects.len() {
                let suspect = capsule.suspects[index];
                index += 1;
                if suspect.valid && suspect.event_sequence == node.event_sequence {
                    reason_code = suspect.reason_code;
                    event_kind = suspect.event_kind as u16;
                    break;
                }
            }
            ChronoscopeStateFragment::SuspectState {
                stage: node.stage,
                reason_code,
                event_kind,
                score: node.score,
            }
        }
        ChronoscopeNodeKind::Constraint => {
            let mut descriptor_id = 0u64;
            let mut overlap = MemoryOverlapClass::None;
            let mut flags = 0u16;
            let mut index = 0usize;
            while index < capsule.watch_tail.len() {
                let violation = capsule.watch_tail[index];
                index += 1;
                if violation.sequence == node.event_sequence {
                    descriptor_id = violation.descriptor_id;
                    overlap = violation.overlap;
                    flags = violation.suspicion_flags;
                    break;
                }
            }
            ChronoscopeStateFragment::ViolationState {
                stage: node.stage,
                overlap,
                descriptor_id,
                flags,
            }
        }
        ChronoscopeNodeKind::Boundary => ChronoscopeStateFragment::ContractState {
            stage: node.stage,
            path: node.path,
            key: node.request_id ^ node.completion_id ^ node.irq_id,
            status: 1,
        },
        ChronoscopeNodeKind::Outcome => ChronoscopeStateFragment::ResourceWaiterState {
            stage: node.stage,
            cpu_slot: node.cpu_slot,
            key: node.request_id ^ node.completion_id ^ node.irq_id,
            waiters: 1,
        },
    }
}

fn chronoscope_add_checkpoint(
    bundle: &mut ChronoscopeBundle,
    next: &mut usize,
    kind: ChronoscopeCheckpointKind,
    primary_node: ChronoscopeNodeId,
    secondary_node: ChronoscopeNodeId,
    predecessor: ChronoscopeCheckpointId,
    causal_depth: u32,
    confidence_permille: u16,
    capsule: &CrashCapsule,
) -> ChronoscopeCheckpointId {
    if *next >= bundle.checkpoints.len() || primary_node == 0 {
        return ChronoscopeCheckpointId::NONE;
    }
    let primary = match bundle.node_by_id(primary_node) {
        Some(node) => *node,
        None => return ChronoscopeCheckpointId::NONE,
    };
    let checkpoint_id = ChronoscopeCheckpointId((*next as u16) + 1);
    let correlation = CorrelationKey {
        request_id: primary.request_id,
        completion_id: primary.completion_id,
        irq_id: primary.irq_id,
    };
    let semantic_hash = chronoscope_payload_hash(
        primary.event_sequence,
        primary.stage as u64,
        primary.path as u64,
        secondary_node ^ ((kind as u64) << 32),
    );
    let mut fragments = [ChronoscopeStateFragment::None; CHRONOSCOPE_STATE_FRAGMENT_LIMIT];
    fragments[0] = chronoscope_temporal_fragment_for_node(&primary, capsule);
    if secondary_node != 0 {
        if let Some(other) = bundle.node_by_id(secondary_node) {
            fragments[1] = chronoscope_temporal_fragment_for_node(other, capsule);
            if kind == ChronoscopeCheckpointKind::Divergence {
                fragments[2] = ChronoscopeStateFragment::DivergenceState {
                    stage: primary.stage,
                    cpu_a: primary.cpu_slot,
                    cpu_b: other.cpu_slot,
                    sequence: primary.event_sequence.min(other.event_sequence),
                };
            }
        }
    }
    bundle.checkpoints[*next] = ChronoscopeCheckpoint {
        valid: true,
        checkpoint_id,
        stable_id: chronoscope_stable_checkpoint_id(kind, correlation, causal_depth, semantic_hash),
        kind,
        related_nodes: [primary_node, secondary_node, 0, 0],
        correlation,
        causal_depth,
        confidence_permille,
        predecessor,
        replay_start_index: CHRONOSCOPE_INVALID_INDEX,
        fragments,
    };
    *next += 1;
    checkpoint_id
}

fn chronoscope_find_last_known_good_node(bundle: &ChronoscopeBundle) -> ChronoscopeNodeId {
    let fault = bundle.primary_fault_node();
    let fault_seq = bundle
        .node_by_id(fault)
        .map(|node| node.event_sequence)
        .unwrap_or(u64::MAX);
    let mut best = 0u64;
    let mut best_seq = 0u64;
    let mut index = 0usize;
    while index < bundle.nodes.len() {
        let node = bundle.nodes[index];
        index += 1;
        if !node.valid || node.kind != ChronoscopeNodeKind::Observation {
            continue;
        }
        if node.event_sequence >= fault_seq {
            continue;
        }
        if node.event_sequence >= best_seq {
            best_seq = node.event_sequence;
            best = node.node_id;
        }
    }
    best
}

fn chronoscope_find_divergence_pair(
    bundle: &ChronoscopeBundle,
) -> (ChronoscopeNodeId, ChronoscopeNodeId) {
    let mut left = 0usize;
    while left < bundle.nodes.len() {
        let a = bundle.nodes[left];
        left += 1;
        if !a.valid || a.kind != ChronoscopeNodeKind::Observation {
            continue;
        }
        let mut right = left;
        while right < bundle.nodes.len() {
            let b = bundle.nodes[right];
            right += 1;
            if !b.valid || b.kind != ChronoscopeNodeKind::Observation {
                continue;
            }
            if a.cpu_slot == b.cpu_slot {
                continue;
            }
            let same_corr = (a.request_id != 0 && a.request_id == b.request_id)
                || (a.completion_id != 0 && a.completion_id == b.completion_id)
                || (a.irq_id != 0 && a.irq_id == b.irq_id);
            if same_corr && (a.path != b.path || a.stage != b.stage) {
                if a.event_sequence <= b.event_sequence {
                    return (a.node_id, b.node_id);
                }
                return (b.node_id, a.node_id);
            }
        }
    }
    (0, 0)
}

fn chronoscope_infer_temporal_checkpoints(bundle: &mut ChronoscopeBundle, capsule: &CrashCapsule) {
    let fault_node = bundle.primary_fault_node();
    let boundary_node = bundle.earliest_preventable_boundary().unwrap_or(0);
    let last_known_good = chronoscope_find_last_known_good_node(bundle);
    let (divergence_a, divergence_b) = chronoscope_find_divergence_pair(bundle);
    let mut next = 0usize;

    let last_known_good_checkpoint = chronoscope_add_checkpoint(
        bundle,
        &mut next,
        ChronoscopeCheckpointKind::LastKnownGood,
        last_known_good,
        0,
        ChronoscopeCheckpointId::NONE,
        bundle
            .node_by_id(last_known_good)
            .map(|node| node.causal_distance_to_fault)
            .unwrap_or(u32::MAX),
        900,
        capsule,
    );

    let pre_boundary_checkpoint = chronoscope_add_checkpoint(
        bundle,
        &mut next,
        ChronoscopeCheckpointKind::PreBoundary,
        boundary_node,
        0,
        last_known_good_checkpoint,
        bundle
            .node_by_id(boundary_node)
            .map(|node| node.causal_distance_to_fault)
            .unwrap_or(u32::MAX),
        if boundary_node != 0 { 960 } else { 0 },
        capsule,
    );

    let divergence_checkpoint = chronoscope_add_checkpoint(
        bundle,
        &mut next,
        ChronoscopeCheckpointKind::Divergence,
        divergence_a,
        divergence_b,
        if pre_boundary_checkpoint != ChronoscopeCheckpointId::NONE {
            pre_boundary_checkpoint
        } else {
            last_known_good_checkpoint
        },
        bundle
            .node_by_id(divergence_a)
            .map(|node| node.causal_distance_to_fault)
            .unwrap_or(u32::MAX),
        if divergence_a != 0 { 875 } else { 0 },
        capsule,
    );

    let predecessor = if pre_boundary_checkpoint != ChronoscopeCheckpointId::NONE {
        pre_boundary_checkpoint
    } else if divergence_checkpoint != ChronoscopeCheckpointId::NONE {
        divergence_checkpoint
    } else {
        last_known_good_checkpoint
    };
    let fault_checkpoint = chronoscope_add_checkpoint(
        bundle,
        &mut next,
        ChronoscopeCheckpointKind::FaultAdjacent,
        fault_node,
        0,
        predecessor,
        0,
        1000,
        capsule,
    );
    bundle.primary_fault_checkpoint = fault_checkpoint;

    let rewind_node = if pre_boundary_checkpoint != ChronoscopeCheckpointId::NONE {
        boundary_node
    } else {
        last_known_good
    };
    bundle.rewind_checkpoint = chronoscope_add_checkpoint(
        bundle,
        &mut next,
        ChronoscopeCheckpointKind::RewindCandidate,
        rewind_node,
        0,
        if rewind_node == boundary_node && pre_boundary_checkpoint != ChronoscopeCheckpointId::NONE
        {
            last_known_good_checkpoint
        } else {
            ChronoscopeCheckpointId::NONE
        },
        bundle
            .node_by_id(rewind_node)
            .map(|node| node.causal_distance_to_fault)
            .unwrap_or(u32::MAX),
        if rewind_node != 0 { 940 } else { 0 },
        capsule,
    );
    bundle.divergence_checkpoint = divergence_checkpoint;
}

fn chronoscope_attach_replay_positions(bundle: &mut ChronoscopeBundle) {
    let sequence = replay_sequence_from_runtime(&bundle.runtime_events);
    if !sequence.valid {
        return;
    }
    let mut checkpoint_index = 0usize;
    while checkpoint_index < bundle.checkpoints.len() {
        if !bundle.checkpoints[checkpoint_index].valid {
            checkpoint_index += 1;
            continue;
        }
        let checkpoint = bundle.checkpoints[checkpoint_index];
        let mut target_request = checkpoint.correlation.request_id;
        let mut target_completion = checkpoint.correlation.completion_id;
        let mut target_irq = checkpoint.correlation.irq_id;
        if target_request == 0 && target_completion == 0 && target_irq == 0 {
            if let Some(node) = bundle.node_by_id(checkpoint.related_nodes[0]) {
                target_request = node.request_id;
                target_completion = node.completion_id;
                target_irq = node.irq_id;
            }
        }
        let mut event_index = 0usize;
        let mut found = CHRONOSCOPE_INVALID_INDEX;
        while event_index < sequence.total_events as usize {
            let event = sequence.events[event_index];
            if event.valid
                && ((target_request != 0 && event.correlation.request_id == target_request)
                    || (target_completion != 0
                        && event.correlation.completion_id == target_completion)
                    || (target_irq != 0 && event.correlation.irq_id == target_irq))
            {
                found = event_index as u16;
                break;
            }
            event_index += 1;
        }
        bundle.checkpoints[checkpoint_index].replay_start_index = found;
        checkpoint_index += 1;
    }
}

fn chronoscope_lineage_domain_for_node(node: &ChronoscopeNode) -> ChronoscopeLineageDomain {
    match node.kind {
        ChronoscopeNodeKind::Observation => ChronoscopeLineageDomain::RequestPath,
        ChronoscopeNodeKind::Interpretation => ChronoscopeLineageDomain::SuspectState,
        ChronoscopeNodeKind::Constraint => ChronoscopeLineageDomain::ViolationState,
        ChronoscopeNodeKind::Boundary => ChronoscopeLineageDomain::ContractState,
        ChronoscopeNodeKind::Outcome => ChronoscopeLineageDomain::ResourceState,
    }
}

fn chronoscope_lineage_key_for_node(node: &ChronoscopeNode) -> u64 {
    let correlation = if node.request_id != 0 {
        node.request_id
    } else if node.completion_id != 0 {
        node.completion_id
    } else if node.irq_id != 0 {
        node.irq_id
    } else {
        node.event_sequence
    };
    correlation ^ ((node.stage as u64) << 48) ^ ((node.path as u64) << 32)
}

fn chronoscope_add_lineage(
    bundle: &mut ChronoscopeBundle,
    next: &mut usize,
    domain: ChronoscopeLineageDomain,
    key: u64,
    prior_checkpoint: ChronoscopeCheckpointId,
    prior_node: ChronoscopeNodeId,
    transition_node: ChronoscopeNodeId,
    result_checkpoint: ChronoscopeCheckpointId,
    result_node: ChronoscopeNodeId,
    confidence_permille: u16,
) {
    if *next >= bundle.lineage.len() || transition_node == 0 {
        return;
    }
    let lineage_id = ChronoscopeLineageId((*next as u16) + 1);
    bundle.lineage[*next] = ChronoscopeLineageRecord {
        valid: true,
        lineage_id,
        stable_id: chronoscope_stable_lineage_id(
            domain,
            key,
            prior_checkpoint,
            transition_node,
            result_checkpoint,
        ),
        domain,
        key,
        prior_checkpoint,
        prior_node,
        transition_node,
        result_checkpoint,
        result_node,
        confidence_permille,
    };
    *next += 1;
}

fn chronoscope_build_lineage(bundle: &mut ChronoscopeBundle) {
    let mut next = 0usize;
    let fault_node = bundle.primary_fault_node();
    let fault_checkpoint = bundle.primary_fault_checkpoint;
    let rewind_checkpoint = bundle.rewind_checkpoint;
    let mut previous_observation = 0u64;
    let mut index = 0usize;
    while index < bundle.nodes.len() {
        let node = bundle.nodes[index];
        index += 1;
        if !node.valid {
            continue;
        }
        match node.kind {
            ChronoscopeNodeKind::Observation => {
                chronoscope_add_lineage(
                    bundle,
                    &mut next,
                    ChronoscopeLineageDomain::RequestPath,
                    chronoscope_lineage_key_for_node(&node),
                    if previous_observation == 0 {
                        rewind_checkpoint
                    } else {
                        ChronoscopeCheckpointId::NONE
                    },
                    previous_observation,
                    node.node_id,
                    ChronoscopeCheckpointId::NONE,
                    0,
                    800,
                );
                previous_observation = node.node_id;
            }
            ChronoscopeNodeKind::Interpretation => {
                chronoscope_add_lineage(
                    bundle,
                    &mut next,
                    ChronoscopeLineageDomain::SuspectState,
                    chronoscope_lineage_key_for_node(&node),
                    rewind_checkpoint,
                    previous_observation,
                    node.node_id,
                    fault_checkpoint,
                    fault_node,
                    900,
                );
            }
            ChronoscopeNodeKind::Constraint => {
                chronoscope_add_lineage(
                    bundle,
                    &mut next,
                    ChronoscopeLineageDomain::ViolationState,
                    chronoscope_lineage_key_for_node(&node),
                    rewind_checkpoint,
                    previous_observation,
                    node.node_id,
                    fault_checkpoint,
                    fault_node,
                    950,
                );
            }
            ChronoscopeNodeKind::Boundary => {
                chronoscope_add_lineage(
                    bundle,
                    &mut next,
                    ChronoscopeLineageDomain::ContractState,
                    chronoscope_lineage_key_for_node(&node),
                    bundle
                        .predecessor_state(bundle.rewind_checkpoint)
                        .unwrap_or(ChronoscopeCheckpointId::NONE),
                    previous_observation,
                    node.node_id,
                    bundle.rewind_checkpoint,
                    0,
                    920,
                );
            }
            ChronoscopeNodeKind::Outcome => {
                chronoscope_add_lineage(
                    bundle,
                    &mut next,
                    ChronoscopeLineageDomain::ResourceState,
                    chronoscope_lineage_key_for_node(&node),
                    bundle
                        .state_before_fault()
                        .unwrap_or(ChronoscopeCheckpointId::NONE),
                    previous_observation,
                    node.node_id,
                    fault_checkpoint,
                    node.node_id,
                    1000,
                );
            }
        }
    }
    if bundle.divergence_checkpoint != ChronoscopeCheckpointId::NONE {
        if let Some(checkpoint) = bundle.checkpoint_by_id(bundle.divergence_checkpoint) {
            chronoscope_add_lineage(
                bundle,
                &mut next,
                ChronoscopeLineageDomain::CoreDivergenceState,
                checkpoint.correlation.request_id
                    ^ checkpoint.correlation.completion_id
                    ^ checkpoint.correlation.irq_id,
                checkpoint.predecessor,
                checkpoint.related_nodes[0],
                checkpoint.related_nodes[1],
                bundle.divergence_checkpoint,
                fault_node,
                checkpoint.confidence_permille,
            );
        }
    }
}

fn chronoscope_build_temporal_explain_plan(bundle: &ChronoscopeBundle) -> ExplainPlan {
    let mut plan = bundle.build_explain_plan();
    let primary = plan.primary_cause;
    let primary_node = bundle.node_by_id(primary);
    let primary_domain = primary_node
        .map(chronoscope_lineage_domain_for_node)
        .unwrap_or(ChronoscopeLineageDomain::RequestPath);
    let primary_key = primary_node
        .map(chronoscope_lineage_key_for_node)
        .unwrap_or(0);
    plan.state_before_fault = bundle
        .state_before_fault()
        .unwrap_or(ChronoscopeCheckpointId::NONE);
    plan.rewind_candidate = bundle
        .rewind_candidate()
        .unwrap_or(ChronoscopeCheckpointId::NONE);
    plan.divergence_origin = bundle
        .divergence_origin()
        .unwrap_or(ChronoscopeCheckpointId::NONE);
    plan.last_mutation = bundle
        .last_mutation_of(primary_domain, primary_key)
        .unwrap_or(0);
    let mut next = 0usize;
    let mut index = 0usize;
    while index < bundle.lineage.len() && next < plan.lineage_summary.len() {
        let record = bundle.lineage[index];
        index += 1;
        if !record.valid {
            continue;
        }
        if record.transition_node == primary || record.result_node == plan.fault_node {
            plan.lineage_summary[next] = record.lineage_id;
            next += 1;
        }
    }
    plan.temporal_confidence = bundle
        .checkpoint_by_id(plan.state_before_fault)
        .map(|checkpoint| checkpoint.confidence_permille)
        .unwrap_or(0);
    plan.last_writer = bundle
        .last_writer_of(primary_domain, primary_key)
        .unwrap_or(0);
    let writer_chain = bundle.writer_chain_to_fault(primary_domain, primary_key);
    let mut writer_index = 0usize;
    while writer_index < writer_chain.len() && writer_index < plan.writer_chain.len() {
        plan.writer_chain[writer_index] = writer_chain[writer_index];
        writer_index += 1;
    }
    let primary_cap = bundle
        .capability_for_node(primary)
        .unwrap_or(CapabilityId::NONE);
    let capability_chain = bundle.capability_chain(primary_cap);
    let mut capability_index = 0usize;
    while capability_index < capability_chain.len()
        && capability_index < plan.capability_chain.len()
    {
        plan.capability_chain[capability_index] = capability_chain[capability_index];
        capability_index += 1;
    }
    let propagation = bundle.dominant_propagation_path();
    let mut prop_index = 0usize;
    while prop_index < propagation.len() && prop_index < plan.propagation_path.len() {
        plan.propagation_path[prop_index] = propagation[prop_index];
        prop_index += 1;
    }
    let ranking = bundle.responsibility_ranking();
    let mut rank_index = 0usize;
    while rank_index < ranking.len() && rank_index < plan.responsibility_ranking.len() {
        plan.responsibility_ranking[rank_index] = ResponsibilityEntry {
            valid: true,
            node_id: ranking[rank_index].0,
            score: ranking[rank_index].1,
        };
        rank_index += 1;
    }
    plan.responsibility_confidence = ranking.first().map(|entry| entry.1).unwrap_or(0);
    let replay = bundle.replay_to_fault();
    plan.replay_steps_count = replay.total_steps;
    plan.replay_summary = ReplaySummary {
        valid: replay.valid,
        start_checkpoint: bundle
            .state_before_fault()
            .unwrap_or(ChronoscopeCheckpointId::NONE),
        steps_to_fault: replay.total_steps,
        deterministic: replay.valid
            && !replay.partial
            && validate_replay_invariants(&replay.final_state)
            && validate_replay_sequence(&bundle.replay_sequence()),
        partial: replay.partial,
        divergence_point: ChronoscopeEventId::NONE,
    };
    plan.first_divergence_point = ChronoscopeEventId::NONE;
    plan.replay_confidence = if replay.valid && !replay.partial {
        900
    } else if replay.valid {
        450
    } else {
        0
    };
    debug_assert!(plan.fault_node == 0 || bundle.node_by_id(plan.fault_node).is_some());
    debug_assert!(
        plan.last_writer == 0
            || plan
                .writer_chain
                .iter()
                .any(|entry| *entry == plan.last_writer)
    );
    plan.dominant_anomaly = bundle
        .anomalies
        .iter()
        .copied()
        .find(|anomaly| anomaly.valid)
        .map(|anomaly| anomaly.anomaly_id)
        .unwrap_or(ChronoscopeAnomalyId::NONE);
    plan.adaptive_state = bundle.adaptive_state;
    plan.escalation_id = bundle
        .escalations
        .iter()
        .copied()
        .find(|record| record.valid)
        .map(|record| record.escalation_id)
        .unwrap_or(ChronoscopeEscalationId::NONE);
    plan.candidate_node = bundle.dominant_candidate().unwrap_or(0);
    plan.downgrade_ready = matches!(bundle.adaptive_state, ChronoscopeAdaptiveState::CoolingDown);
    plan.anomaly_confidence = bundle
        .anomalies
        .iter()
        .copied()
        .find(|anomaly| anomaly.valid)
        .map(|anomaly| anomaly.confidence_permille)
        .unwrap_or(0);
    plan
}

fn chronoscope_event_id_for_node(
    bundle: &ChronoscopeBundle,
    node: ChronoscopeNodeId,
) -> ChronoscopeEventId {
    let mut index = 0usize;
    while index < bundle.runtime_events.total_events as usize {
        let event = bundle.runtime_events.events[index];
        if event.valid
            && event.event_id.0
                == bundle
                    .node_by_id(node)
                    .map(|n| n.event_sequence)
                    .unwrap_or(0)
        {
            return event.event_id;
        }
        index += 1;
    }
    ChronoscopeEventId::NONE
}

fn chronoscope_capability_for_node_from_events(
    bundle: &ChronoscopeBundle,
    node: ChronoscopeNodeId,
) -> CapabilityId {
    let target = match bundle.node_by_id(node) {
        Some(node) => node,
        None => return CapabilityId::NONE,
    };
    let mut index = 0usize;
    while index < bundle.runtime_events.total_events as usize {
        let event = bundle.runtime_events.events[index];
        if event.valid
            && event.correlation.request_id == target.request_id
            && event.correlation.completion_id == target.completion_id
            && event.correlation.irq_id == target.irq_id
            && event.capability_id != CapabilityId::NONE
        {
            return event.capability_id;
        }
        index += 1;
    }
    CapabilityId::NONE
}

fn chronoscope_build_last_writers(bundle: &mut ChronoscopeBundle) {
    let mut next = 0usize;
    bundle.last_writer_index = [LastWriterIndexEntry::EMPTY; CHRONOSCOPE_LAST_WRITER_INDEX_LIMIT];
    bundle.writer_predecessor_by_node = [0; CHRONOSCOPE_NODE_LIMIT];
    bundle.node_capabilities = [CapabilityId::NONE; CHRONOSCOPE_NODE_LIMIT];
    let mut index = 0usize;
    while index < bundle.nodes.len() && next < bundle.last_writers.len() {
        let node = bundle.nodes[index];
        index += 1;
        if !node.valid {
            continue;
        }
        let domain = chronoscope_lineage_domain_for_node(&node);
        let key = chronoscope_lineage_key_for_node(&node);
        let mut slot = None;
        let mut probe = ((key ^ ((domain as u64) << 32)) as usize) % bundle.last_writer_index.len();
        let mut steps = 0usize;
        while steps < bundle.last_writer_index.len() {
            let entry = bundle.last_writer_index[probe];
            if !entry.valid {
                break;
            }
            if entry.domain == domain && entry.key == key {
                slot = Some(entry.record_index as usize);
                break;
            }
            probe = (probe + 1) % bundle.last_writer_index.len();
            steps += 1;
        }
        let capability_id = chronoscope_capability_for_node_from_events(bundle, node.node_id);
        bundle.node_capabilities[index - 1] = capability_id;
        match slot {
            Some(existing) => {
                let previous = bundle.last_writers[existing].last_writer_node;
                bundle.last_writers[existing] = LastWriterRecord {
                    valid: true,
                    domain,
                    key,
                    last_writer_node: node.node_id,
                    predecessor_writer_node: previous,
                    event_id: chronoscope_event_id_for_node(bundle, node.node_id),
                    capability_id,
                };
                if let Some(node_slot) = bundle.direct_node_index(node.node_id) {
                    bundle.writer_predecessor_by_node[node_slot] = previous;
                }
            }
            None => {
                bundle.last_writers[next] = LastWriterRecord {
                    valid: true,
                    domain,
                    key,
                    last_writer_node: node.node_id,
                    predecessor_writer_node: 0,
                    event_id: chronoscope_event_id_for_node(bundle, node.node_id),
                    capability_id,
                };
                let mut insert =
                    ((key ^ ((domain as u64) << 32)) as usize) % bundle.last_writer_index.len();
                while bundle.last_writer_index[insert].valid {
                    insert = (insert + 1) % bundle.last_writer_index.len();
                }
                bundle.last_writer_index[insert] = LastWriterIndexEntry {
                    valid: true,
                    domain,
                    key,
                    record_index: next as u16,
                };
                next += 1;
            }
        }
    }
}

fn chronoscope_build_capability_derivations(bundle: &mut ChronoscopeBundle) {
    let mut next = 0usize;
    bundle.capability_parent_index =
        [CapabilityParentIndexEntry::EMPTY; CHRONOSCOPE_CAPABILITY_INDEX_LIMIT];
    let mut event_index = 0usize;
    while event_index < bundle.runtime_events.total_events as usize
        && next < bundle.capability_derivations.len()
    {
        let event = bundle.runtime_events.events[event_index];
        event_index += 1;
        if !event.valid || event.capability_id == CapabilityId::NONE {
            continue;
        }
        if event.kind == ChronoscopeRuntimeEventKind::CapabilityDerive
            || event.parent_capability_id != CapabilityId::NONE
        {
            let mut node_id = 0u64;
            let mut node_index = 0usize;
            while node_index < bundle.nodes.len() {
                let node = bundle.nodes[node_index];
                node_index += 1;
                if node.valid
                    && node.request_id == event.correlation.request_id
                    && node.completion_id == event.correlation.completion_id
                    && node.irq_id == event.correlation.irq_id
                {
                    node_id = node.node_id;
                    break;
                }
            }
            bundle.capability_derivations[next] = CapabilityDerivationRecord {
                valid: true,
                parent: event.parent_capability_id,
                derived: event.capability_id,
                node_id,
                event_id: event.event_id,
                rights_mask: event.rights_mask,
                confidence_permille: if event.kind == ChronoscopeRuntimeEventKind::CapabilityDerive
                {
                    1000
                } else {
                    750
                },
            };
            let mut insert =
                (event.capability_id.0 as usize) % bundle.capability_parent_index.len();
            while bundle.capability_parent_index[insert].valid {
                insert = (insert + 1) % bundle.capability_parent_index.len();
            }
            bundle.capability_parent_index[insert] = CapabilityParentIndexEntry {
                valid: true,
                capability: event.capability_id,
                record_index: next as u16,
            };
            next += 1;
        }
    }
}

fn chronoscope_build_propagation(bundle: &mut ChronoscopeBundle) {
    let mut next = 0usize;
    bundle.propagation_heads = [CHRONOSCOPE_INVALID_INDEX; CHRONOSCOPE_NODE_LIMIT];
    bundle.propagation_next = [CHRONOSCOPE_INVALID_INDEX; CHRONOSCOPE_PROPAGATION_LIMIT];
    let mut edge_index = 0usize;
    while edge_index < bundle.edges.len() && next < bundle.propagation.len() {
        let edge = bundle.edges[edge_index];
        edge_index += 1;
        if !edge.valid {
            continue;
        }
        let source = match bundle.node_by_id(edge.src_node_id) {
            Some(node) => node,
            None => continue,
        };
        let propagation_type = match edge.kind {
            ChronoscopeEdgeKind::Supports | ChronoscopeEdgeKind::Explains => {
                PropagationType::DataFlow
            }
            ChronoscopeEdgeKind::ObservedBefore
            | ChronoscopeEdgeKind::LeadsTo
            | ChronoscopeEdgeKind::Caused => PropagationType::ControlFlow,
            ChronoscopeEdgeKind::Violates => PropagationType::CausalAmplification,
            _ => {
                if chronoscope_capability_for_node_from_events(bundle, edge.src_node_id)
                    != CapabilityId::NONE
                    || chronoscope_capability_for_node_from_events(bundle, edge.dst_node_id)
                        != CapabilityId::NONE
                {
                    PropagationType::CapabilityFlow
                } else {
                    PropagationType::ControlFlow
                }
            }
        };
        bundle.propagation[next] = PropagationRecord {
            valid: true,
            source_node: edge.src_node_id,
            target_node: edge.dst_node_id,
            propagation_type,
            correlation: CorrelationKey {
                request_id: source.request_id,
                completion_id: source.completion_id,
                irq_id: source.irq_id,
            },
        };
        if let Some(source_index) = bundle.direct_node_index(edge.src_node_id) {
            bundle.propagation_next[next] = bundle.propagation_heads[source_index];
            bundle.propagation_heads[source_index] = next as u16;
        }
        next += 1;
    }
}

fn chronoscope_build_responsibility(bundle: &mut ChronoscopeBundle) {
    let mut entries = [ResponsibilityEntry::EMPTY; CHRONOSCOPE_RESPONSIBILITY_LIMIT];
    let mut count = 0usize;
    let mut index = 0usize;
    while index < bundle.nodes.len() {
        let node = bundle.nodes[index];
        index += 1;
        if !node.valid {
            continue;
        }
        let mut propagated = 0u16;
        let mut prop_index = 0usize;
        while prop_index < bundle.propagation.len() {
            let propagation = bundle.propagation[prop_index];
            prop_index += 1;
            if propagation.valid && propagation.source_node == node.node_id {
                propagated = propagated.saturating_add(1);
            }
        }
        let authority = if chronoscope_capability_for_node_from_events(bundle, node.node_id)
            != CapabilityId::NONE
        {
            20
        } else {
            0
        };
        let violation = if node.kind == ChronoscopeNodeKind::Constraint {
            25
        } else {
            0
        };
        let proximity = if node.causal_distance_to_fault == u32::MAX {
            0
        } else {
            40u16.saturating_sub((node.causal_distance_to_fault.min(40)) as u16)
        };
        let score = proximity
            .saturating_add(propagated.saturating_mul(10))
            .saturating_add(authority)
            .saturating_add(violation)
            .saturating_add(node.score.min(15));
        if count < entries.len() {
            entries[count] = ResponsibilityEntry {
                valid: true,
                node_id: node.node_id,
                score,
            };
            count += 1;
        } else {
            let mut worst = 0usize;
            let mut candidate = 1usize;
            while candidate < entries.len() {
                if entries[candidate].score < entries[worst].score {
                    worst = candidate;
                }
                candidate += 1;
            }
            if score > entries[worst].score {
                entries[worst] = ResponsibilityEntry {
                    valid: true,
                    node_id: node.node_id,
                    score,
                };
            }
        }
    }
    let mut outer = 0usize;
    while outer < entries.len() {
        let mut best = outer;
        let mut inner = outer + 1;
        while inner < entries.len() {
            if entries[inner].score > entries[best].score {
                best = inner;
            }
            inner += 1;
        }
        if best != outer {
            let tmp = entries[outer];
            entries[outer] = entries[best];
            entries[best] = tmp;
        }
        outer += 1;
    }
    bundle.responsibility = entries;
}

fn chronoscope_event_to_domain(
    event: &ChronoscopeRuntimeEventRecord,
) -> Option<ChronoscopeLineageDomain> {
    match event.kind {
        ChronoscopeRuntimeEventKind::ContractTransition => {
            Some(ChronoscopeLineageDomain::ContractState)
        }
        ChronoscopeRuntimeEventKind::ResourceClaim
        | ChronoscopeRuntimeEventKind::ResourceRelease
        | ChronoscopeRuntimeEventKind::ResourceWait => {
            Some(ChronoscopeLineageDomain::ResourceState)
        }
        ChronoscopeRuntimeEventKind::RequestStart
        | ChronoscopeRuntimeEventKind::RequestComplete => {
            Some(ChronoscopeLineageDomain::RequestPath)
        }
        ChronoscopeRuntimeEventKind::ViolationObserved => {
            Some(ChronoscopeLineageDomain::ViolationState)
        }
        ChronoscopeRuntimeEventKind::SuspectPromoted => {
            Some(ChronoscopeLineageDomain::SuspectState)
        }
        ChronoscopeRuntimeEventKind::DivergenceHint => {
            Some(ChronoscopeLineageDomain::CoreDivergenceState)
        }
        _ => None,
    }
}

fn chronoscope_find_checkpoint_for_node(
    bundle: &ChronoscopeBundle,
    node_id: ChronoscopeNodeId,
) -> ChronoscopeCheckpointId {
    let mut index = 0usize;
    while index < bundle.checkpoints.len() {
        let checkpoint = bundle.checkpoints[index];
        index += 1;
        if !checkpoint.valid {
            continue;
        }
        let mut related = 0usize;
        while related < checkpoint.related_nodes.len() {
            if checkpoint.related_nodes[related] == node_id {
                return checkpoint.checkpoint_id;
            }
            related += 1;
        }
    }
    ChronoscopeCheckpointId::NONE
}

fn chronoscope_find_node_for_event(
    bundle: &ChronoscopeBundle,
    event: ChronoscopeRuntimeEventRecord,
) -> ChronoscopeNodeId {
    let mut index = 0usize;
    while index < bundle.nodes.len() {
        let node = bundle.nodes[index];
        index += 1;
        if !node.valid {
            continue;
        }
        if node.cpu_slot == event.core_id
            && node.stage == event.stage
            && node.request_id == event.correlation.request_id
            && node.completion_id == event.correlation.completion_id
            && node.irq_id == event.correlation.irq_id
        {
            return node.node_id;
        }
    }
    0
}

fn chronoscope_anomaly_kind_name(kind: ChronoscopeAnomalyKind) -> &'static str {
    match kind {
        ChronoscopeAnomalyKind::SchedulerStall => "scheduler-stall",
        ChronoscopeAnomalyKind::RepeatedDivergence => "repeated-divergence",
        ChronoscopeAnomalyKind::ContractStateOscillation => "contract-oscillation",
        ChronoscopeAnomalyKind::ResourceWaitInflation => "wait-inflation",
        ChronoscopeAnomalyKind::ViolationBurst => "violation-burst",
        ChronoscopeAnomalyKind::SuspectPromotionBurst => "suspect-burst",
        ChronoscopeAnomalyKind::FaultNearPrecursor => "fault-precursor",
        ChronoscopeAnomalyKind::AbnormalReplayDivergence => "replay-divergence",
        ChronoscopeAnomalyKind::RepeatedOverwriteHistoryLoss => "history-loss",
        ChronoscopeAnomalyKind::CapabilityMisuseHint => "cap-misuse",
    }
}

fn chronoscope_escalation_level_name(level: ChronoscopeEscalationLevel) -> &'static str {
    match level {
        ChronoscopeEscalationLevel::None => "none",
        ChronoscopeEscalationLevel::LocalStandard => "local-standard",
        ChronoscopeEscalationLevel::LocalDeep => "local-deep",
        ChronoscopeEscalationLevel::GlobalStandard => "global-standard",
        ChronoscopeEscalationLevel::GlobalDeep => "global-deep",
    }
}

fn chronoscope_capture_window_kind_name(kind: ChronoscopeCaptureWindowKind) -> &'static str {
    match kind {
        ChronoscopeCaptureWindowKind::LocalCore => "local-core",
        ChronoscopeCaptureWindowKind::Correlation => "correlation",
        ChronoscopeCaptureWindowKind::DomainKey => "domain-key",
        ChronoscopeCaptureWindowKind::Global => "global",
    }
}

fn chronoscope_detect_adaptive_state(
    bundle: &ChronoscopeBundle,
    anomaly_count: usize,
    escalated_global: bool,
    escalated_local: bool,
) -> ChronoscopeAdaptiveState {
    if escalated_global {
        ChronoscopeAdaptiveState::EscalatedGlobal
    } else if escalated_local {
        ChronoscopeAdaptiveState::EscalatedLocal
    } else if anomaly_count != 0 {
        if bundle.temporal_explain.replay_confidence > 850
            && bundle.temporal_explain.responsibility_confidence > 700
        {
            ChronoscopeAdaptiveState::CoolingDown
        } else {
            ChronoscopeAdaptiveState::Watching
        }
    } else {
        ChronoscopeAdaptiveState::Normal
    }
}

fn chronoscope_build_anomaly_and_adaptive(bundle: &mut ChronoscopeBundle) {
    let mut next_anomaly = 0usize;
    let mut next_escalation = 0usize;
    let mut next_window = 0usize;
    let next_transition = 0usize;
    let previous_state = bundle.adaptive_state;
    bundle.anomalies = [ChronoscopeAnomalyRecord::EMPTY; CHRONOSCOPE_ANOMALY_LIMIT];
    bundle.escalations = [ChronoscopeEscalationRecord::EMPTY; CHRONOSCOPE_ESCALATION_LIMIT];
    bundle.capture_windows = [ChronoscopeCaptureWindow::EMPTY; CHRONOSCOPE_CAPTURE_WINDOW_LIMIT];
    bundle.adaptive_transitions =
        [ChronoscopeAdaptiveTransition::EMPTY; CHRONOSCOPE_ADAPTIVE_TRANSITION_LIMIT];
    bundle.candidates = ChronoscopeCandidateSet::EMPTY;

    let mut per_core_violation = [0u16; MAX_TRACE_CPUS];
    let mut per_core_overwrite = [0u64; MAX_TRACE_CPUS];
    let mut last_wait_key = 0u64;
    let mut wait_count = 0u16;
    let mut last_contract_key = 0u64;
    let mut contract_flips = 0u16;
    let mut last_divergence = CorrelationKey {
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
    };
    let mut divergence_count = 0u16;
    let mut last_suspect_corr = CorrelationKey {
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
    };
    let mut suspect_count = 0u16;
    let mut event_index = 0usize;
    while event_index < bundle.runtime_events.total_events as usize
        && next_anomaly < bundle.anomalies.len()
    {
        let event = bundle.runtime_events.events[event_index];
        event_index += 1;
        if !event.valid {
            continue;
        }
        let node_id = chronoscope_find_node_for_event(bundle, event);
        let checkpoint_id = chronoscope_find_checkpoint_for_node(bundle, node_id);
        match event.kind {
            ChronoscopeRuntimeEventKind::ViolationObserved => {
                let core_index = event.core_id as usize;
                if core_index < per_core_violation.len() {
                    per_core_violation[core_index] =
                        per_core_violation[core_index].saturating_add(1);
                    if per_core_violation[core_index] >= 3 {
                        bundle.anomalies[next_anomaly] = ChronoscopeAnomalyRecord {
                            valid: true,
                            anomaly_id: ChronoscopeAnomalyId(next_anomaly as u16 + 1),
                            kind: ChronoscopeAnomalyKind::ViolationBurst,
                            first_event: event.event_id,
                            first_node: node_id,
                            first_checkpoint: checkpoint_id,
                            domain: Some(ChronoscopeLineageDomain::ViolationState),
                            key: event.object_key,
                            correlation: CorrelationKey {
                                request_id: event.correlation.request_id,
                                completion_id: event.correlation.completion_id,
                                irq_id: event.correlation.irq_id,
                            },
                            severity: 4,
                            confidence_permille: 880,
                            first_seen_tick: event.uptime_us,
                            last_seen_tick: event.uptime_us,
                            occurrence_count: per_core_violation[core_index],
                            related_core_mask: 1u16 << event.core_id.min(15),
                            escalation_id: ChronoscopeEscalationId::NONE,
                        };
                        next_anomaly += 1;
                    }
                }
            }
            ChronoscopeRuntimeEventKind::ResourceWait => {
                if event.object_key == last_wait_key {
                    wait_count = wait_count.saturating_add(1);
                } else {
                    last_wait_key = event.object_key;
                    wait_count = 1;
                }
                if wait_count >= 3 && next_anomaly < bundle.anomalies.len() {
                    bundle.anomalies[next_anomaly] = ChronoscopeAnomalyRecord {
                        valid: true,
                        anomaly_id: ChronoscopeAnomalyId(next_anomaly as u16 + 1),
                        kind: ChronoscopeAnomalyKind::ResourceWaitInflation,
                        first_event: event.event_id,
                        first_node: node_id,
                        first_checkpoint: checkpoint_id,
                        domain: Some(ChronoscopeLineageDomain::ResourceState),
                        key: event.object_key,
                        correlation: CorrelationKey {
                            request_id: event.correlation.request_id,
                            completion_id: event.correlation.completion_id,
                            irq_id: event.correlation.irq_id,
                        },
                        severity: 3,
                        confidence_permille: 820,
                        first_seen_tick: event.uptime_us,
                        last_seen_tick: event.uptime_us,
                        occurrence_count: wait_count,
                        related_core_mask: 1u16 << event.core_id.min(15),
                        escalation_id: ChronoscopeEscalationId::NONE,
                    };
                    next_anomaly += 1;
                }
            }
            ChronoscopeRuntimeEventKind::ContractTransition => {
                if event.object_key == last_contract_key {
                    contract_flips = contract_flips.saturating_add(1);
                } else {
                    last_contract_key = event.object_key;
                    contract_flips = 1;
                }
                if contract_flips >= 4 && next_anomaly < bundle.anomalies.len() {
                    bundle.anomalies[next_anomaly] = ChronoscopeAnomalyRecord {
                        valid: true,
                        anomaly_id: ChronoscopeAnomalyId(next_anomaly as u16 + 1),
                        kind: ChronoscopeAnomalyKind::ContractStateOscillation,
                        first_event: event.event_id,
                        first_node: node_id,
                        first_checkpoint: checkpoint_id,
                        domain: Some(ChronoscopeLineageDomain::ContractState),
                        key: event.object_key,
                        correlation: CorrelationKey {
                            request_id: event.correlation.request_id,
                            completion_id: event.correlation.completion_id,
                            irq_id: event.correlation.irq_id,
                        },
                        severity: 3,
                        confidence_permille: 790,
                        first_seen_tick: event.uptime_us,
                        last_seen_tick: event.uptime_us,
                        occurrence_count: contract_flips,
                        related_core_mask: 1u16 << event.core_id.min(15),
                        escalation_id: ChronoscopeEscalationId::NONE,
                    };
                    next_anomaly += 1;
                }
            }
            ChronoscopeRuntimeEventKind::DivergenceHint => {
                let corr = CorrelationKey {
                    request_id: event.correlation.request_id,
                    completion_id: event.correlation.completion_id,
                    irq_id: event.correlation.irq_id,
                };
                if corr == last_divergence {
                    divergence_count = divergence_count.saturating_add(1);
                } else {
                    last_divergence = corr;
                    divergence_count = 1;
                }
                if divergence_count >= 2 && next_anomaly < bundle.anomalies.len() {
                    bundle.anomalies[next_anomaly] = ChronoscopeAnomalyRecord {
                        valid: true,
                        anomaly_id: ChronoscopeAnomalyId(next_anomaly as u16 + 1),
                        kind: ChronoscopeAnomalyKind::RepeatedDivergence,
                        first_event: event.event_id,
                        first_node: node_id,
                        first_checkpoint: checkpoint_id,
                        domain: Some(ChronoscopeLineageDomain::CoreDivergenceState),
                        key: event.object_key,
                        correlation: corr,
                        severity: 4,
                        confidence_permille: 860,
                        first_seen_tick: event.uptime_us,
                        last_seen_tick: event.uptime_us,
                        occurrence_count: divergence_count,
                        related_core_mask: 1u16 << event.core_id.min(15),
                        escalation_id: ChronoscopeEscalationId::NONE,
                    };
                    next_anomaly += 1;
                }
            }
            ChronoscopeRuntimeEventKind::SuspectPromoted => {
                let corr = CorrelationKey {
                    request_id: event.correlation.request_id,
                    completion_id: event.correlation.completion_id,
                    irq_id: event.correlation.irq_id,
                };
                if corr == last_suspect_corr {
                    suspect_count = suspect_count.saturating_add(1);
                } else {
                    last_suspect_corr = corr;
                    suspect_count = 1;
                }
                if suspect_count >= 2 && next_anomaly < bundle.anomalies.len() {
                    bundle.anomalies[next_anomaly] = ChronoscopeAnomalyRecord {
                        valid: true,
                        anomaly_id: ChronoscopeAnomalyId(next_anomaly as u16 + 1),
                        kind: ChronoscopeAnomalyKind::SuspectPromotionBurst,
                        first_event: event.event_id,
                        first_node: node_id,
                        first_checkpoint: checkpoint_id,
                        domain: Some(ChronoscopeLineageDomain::SuspectState),
                        key: event.object_key,
                        correlation: corr,
                        severity: 2,
                        confidence_permille: 700,
                        first_seen_tick: event.uptime_us,
                        last_seen_tick: event.uptime_us,
                        occurrence_count: suspect_count,
                        related_core_mask: 1u16 << event.core_id.min(15),
                        escalation_id: ChronoscopeEscalationId::NONE,
                    };
                    next_anomaly += 1;
                }
            }
            ChronoscopeRuntimeEventKind::FaultMarker => {
                if next_anomaly < bundle.anomalies.len() {
                    bundle.anomalies[next_anomaly] = ChronoscopeAnomalyRecord {
                        valid: true,
                        anomaly_id: ChronoscopeAnomalyId(next_anomaly as u16 + 1),
                        kind: ChronoscopeAnomalyKind::FaultNearPrecursor,
                        first_event: event.event_id,
                        first_node: node_id,
                        first_checkpoint: checkpoint_id,
                        domain: chronoscope_event_to_domain(&event),
                        key: event.object_key,
                        correlation: CorrelationKey {
                            request_id: event.correlation.request_id,
                            completion_id: event.correlation.completion_id,
                            irq_id: event.correlation.irq_id,
                        },
                        severity: 5,
                        confidence_permille: 940,
                        first_seen_tick: event.uptime_us,
                        last_seen_tick: event.uptime_us,
                        occurrence_count: 1,
                        related_core_mask: 1u16 << event.core_id.min(15),
                        escalation_id: ChronoscopeEscalationId::NONE,
                    };
                    next_anomaly += 1;
                }
            }
            _ => {}
        }
    }

    let mut core_index = 0usize;
    while core_index < bundle.runtime_events.per_core.len() && next_anomaly < bundle.anomalies.len()
    {
        let core = bundle.runtime_events.per_core[core_index];
        if core.valid && core.overwritten_events > 0 {
            per_core_overwrite[core_index] = core.overwritten_events;
            bundle.anomalies[next_anomaly] = ChronoscopeAnomalyRecord {
                valid: true,
                anomaly_id: ChronoscopeAnomalyId(next_anomaly as u16 + 1),
                kind: ChronoscopeAnomalyKind::RepeatedOverwriteHistoryLoss,
                first_event: ChronoscopeEventId::NONE,
                first_node: 0,
                first_checkpoint: bundle
                    .state_before_fault()
                    .unwrap_or(ChronoscopeCheckpointId::NONE),
                domain: None,
                key: core.oldest_local_sequence,
                correlation: CorrelationKey {
                    request_id: 0,
                    completion_id: 0,
                    irq_id: 0,
                },
                severity: 3,
                confidence_permille: 910,
                first_seen_tick: core.oldest_local_sequence,
                last_seen_tick: core.newest_local_sequence,
                occurrence_count: core.overwritten_events.min(u16::MAX as u64) as u16,
                related_core_mask: 1u16 << core.core_id.min(15),
                escalation_id: ChronoscopeEscalationId::NONE,
            };
            next_anomaly += 1;
        }
        core_index += 1;
    }

    let mut anomaly_index = 0usize;
    let mut escalated_local = false;
    let mut escalated_global = false;
    while anomaly_index < bundle.anomalies.len()
        && next_escalation < bundle.escalations.len()
        && next_window < bundle.capture_windows.len()
    {
        let anomaly = bundle.anomalies[anomaly_index];
        anomaly_index += 1;
        if !anomaly.valid {
            continue;
        }
        let (level, reason, budget) = if anomaly.severity >= 5
            || anomaly.kind == ChronoscopeAnomalyKind::RepeatedOverwriteHistoryLoss
        {
            (
                ChronoscopeEscalationLevel::GlobalDeep,
                ChronoscopeEscalationReason::OverwritePressure,
                24u16,
            )
        } else if anomaly.severity >= 4 {
            (
                ChronoscopeEscalationLevel::LocalDeep,
                match anomaly.kind {
                    ChronoscopeAnomalyKind::ViolationBurst => {
                        ChronoscopeEscalationReason::ViolationBurst
                    }
                    ChronoscopeAnomalyKind::RepeatedDivergence => {
                        ChronoscopeEscalationReason::DivergenceBurst
                    }
                    ChronoscopeAnomalyKind::FaultNearPrecursor => {
                        ChronoscopeEscalationReason::FaultNearPrecursor
                    }
                    _ => ChronoscopeEscalationReason::CapabilityMisuse,
                },
                16u16,
            )
        } else {
            (
                ChronoscopeEscalationLevel::LocalStandard,
                match anomaly.kind {
                    ChronoscopeAnomalyKind::ResourceWaitInflation => {
                        ChronoscopeEscalationReason::WaitInflation
                    }
                    ChronoscopeAnomalyKind::ContractStateOscillation => {
                        ChronoscopeEscalationReason::ContractOscillation
                    }
                    _ => ChronoscopeEscalationReason::CapabilityMisuse,
                },
                8u16,
            )
        };
        if level == ChronoscopeEscalationLevel::GlobalDeep {
            escalated_global = true;
        } else {
            escalated_local = true;
        }
        let escalation_id = ChronoscopeEscalationId(next_escalation as u16 + 1);
        let replay_bookmark = if anomaly.first_checkpoint != ChronoscopeCheckpointId::NONE {
            anomaly.first_checkpoint
        } else {
            bundle
                .rewind_candidate()
                .unwrap_or(ChronoscopeCheckpointId::NONE)
        };
        bundle.escalations[next_escalation] = ChronoscopeEscalationRecord {
            valid: true,
            escalation_id,
            level,
            reason,
            anomaly_id: anomaly.anomaly_id,
            target_core_mask: if anomaly.related_core_mask != 0 {
                anomaly.related_core_mask
            } else {
                0xffff
            },
            correlation: anomaly.correlation,
            domain: anomaly.domain,
            key: anomaly.key,
            start_tick: anomaly.first_seen_tick,
            event_budget: budget,
            checkpoint_triggered: replay_bookmark != ChronoscopeCheckpointId::NONE,
            replay_bookmark,
        };
        bundle.capture_windows[next_window] = ChronoscopeCaptureWindow {
            valid: true,
            kind: if anomaly.correlation.request_id != 0
                || anomaly.correlation.completion_id != 0
                || anomaly.correlation.irq_id != 0
            {
                ChronoscopeCaptureWindowKind::Correlation
            } else if anomaly.domain.is_some() {
                ChronoscopeCaptureWindowKind::DomainKey
            } else if anomaly.related_core_mask != 0 && anomaly.related_core_mask.count_ones() == 1
            {
                ChronoscopeCaptureWindowKind::LocalCore
            } else {
                ChronoscopeCaptureWindowKind::Global
            },
            escalation_id,
            anomaly_id: anomaly.anomaly_id,
            start_event: anomaly.first_event,
            end_event: ChronoscopeEventId(anomaly.first_event.0.saturating_add(budget as u64)),
            start_tick: anomaly.first_seen_tick,
            end_tick: anomaly.last_seen_tick.saturating_add(budget as u64),
            target_core_mask: bundle.escalations[next_escalation].target_core_mask,
            correlation: anomaly.correlation,
            domain: anomaly.domain,
            key: anomaly.key,
            observed_events: budget,
            partial_history: bundle.runtime_events.partial,
        };
        let update_index = anomaly_index - 1;
        bundle.anomalies[update_index].escalation_id = escalation_id;
        next_escalation += 1;
        next_window += 1;
    }

    bundle.adaptive_state =
        chronoscope_detect_adaptive_state(bundle, next_anomaly, escalated_global, escalated_local);
    if next_transition < bundle.adaptive_transitions.len()
        && bundle.adaptive_state != previous_state
    {
        bundle.adaptive_transitions[next_transition] = ChronoscopeAdaptiveTransition {
            valid: true,
            from: previous_state,
            to: bundle.adaptive_state,
            reason: if next_escalation != 0 {
                bundle.escalations[0].reason
            } else {
                ChronoscopeEscalationReason::None
            },
            at_tick: if next_anomaly != 0 {
                bundle.anomalies[0].first_seen_tick
            } else {
                0
            },
            anomaly_id: if next_anomaly != 0 {
                bundle.anomalies[0].anomaly_id
            } else {
                ChronoscopeAnomalyId::NONE
            },
        };
    }
}

fn chronoscope_build_candidates(bundle: &mut ChronoscopeBundle) {
    let ranking = bundle.responsibility_ranking();
    let state_before_fault = bundle
        .state_before_fault()
        .unwrap_or(ChronoscopeCheckpointId::NONE);
    let mut candidate_index = 0usize;
    while candidate_index < bundle.candidates.candidates.len() {
        let node_id = if candidate_index < ranking.len() {
            ranking[candidate_index].0
        } else {
            0
        };
        if node_id == 0 {
            break;
        }
        let node = bundle.node_by_id(node_id);
        let domain = node
            .map(chronoscope_lineage_domain_for_node)
            .unwrap_or(ChronoscopeLineageDomain::RequestPath);
        let key = node.map(chronoscope_lineage_key_for_node).unwrap_or(0);
        let anomaly_id = if bundle.anomalies[candidate_index].valid {
            bundle.anomalies[candidate_index].anomaly_id
        } else {
            ChronoscopeAnomalyId::NONE
        };
        let last_writer = bundle.last_writer_of(domain, key).unwrap_or(0);
        let path = bundle.propagation_chain_to_fault(node_id);
        bundle.candidates.candidates[candidate_index] = ChronoscopeRootCauseCandidate {
            valid: true,
            anomaly_id,
            node_id,
            first_bad_transition: if path.len() > 1 { path[1] } else { node_id },
            last_safe_checkpoint: state_before_fault,
            last_writer,
            linkage_score: ranking[candidate_index].1,
        };
        candidate_index += 1;
    }
    bundle.candidates.dominant_candidate = bundle
        .candidates
        .candidates
        .iter()
        .copied()
        .find(|candidate| candidate.valid)
        .map(|candidate| candidate.node_id)
        .unwrap_or(0);
}

fn chronoscope_build_trust_surface(bundle: &mut ChronoscopeBundle) {
    bundle.integrity = bundle.runtime_events.integrity;
    if bundle.temporal_explain.replay_summary.valid
        && !bundle.temporal_explain.replay_summary.deterministic
    {
        bundle.integrity.complete = false;
        bundle.integrity.flags.replay_incomplete = true;
        if matches!(
            bundle.integrity.primary_reason,
            ChronoscopePartialReason::None
        ) {
            bundle.integrity.primary_reason = ChronoscopePartialReason::ReplayIncomplete;
        }
    }
    bundle.perf = snapshot_perf_counters();
    bundle.trust = ChronoscopeTrustSurface {
        schema_version: CHRONOSCOPE_SCHEMA_VERSION,
        completeness: bundle.integrity,
        capture_level: runtime_capture_level(),
        replay_partial: bundle.integrity.flags.replay_incomplete
            || bundle.runtime_events.partial
            || bundle.temporal_explain.replay_summary.partial,
        explain_degraded: !bundle.integrity.complete || bundle.temporal_explain.candidate_node == 0,
        responsibility_partial: !bundle.integrity.complete
            || bundle.temporal_explain.responsibility_confidence < 500,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChronoscopeValidationError {
    InvalidSchemaVersion,
    InvalidCheckpointReference,
    InvalidLineageReference,
    InvalidAdaptiveLinkage,
    NonMonotonicRuntimeSequence,
    InvalidResponsibilityOrdering,
    InconsistentExplainPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronoscopeValidationResult {
    pub valid: bool,
    pub errors: Vec<ChronoscopeValidationError>,
}

fn validate_event_fabric(window: &ChronoscopeRuntimeEventWindow) -> ChronoscopeValidationResult {
    let mut result = ChronoscopeValidationResult {
        valid: true,
        errors: Vec::new(),
    };
    let mut cpu = 0usize;
    while cpu < window.per_core.len() {
        let mut last = 0u64;
        let mut index = 0usize;
        while index < window.total_events as usize {
            let event = window.events[index];
            index += 1;
            if !event.valid || event.core_id as usize != cpu {
                continue;
            }
            if event.local_sequence < last {
                result.valid = false;
                result
                    .errors
                    .push(ChronoscopeValidationError::NonMonotonicRuntimeSequence);
                return result;
            }
            last = event.local_sequence;
        }
        cpu += 1;
    }
    result
}

fn validate_lineage(bundle: &ChronoscopeBundle) -> ChronoscopeValidationResult {
    let mut result = ChronoscopeValidationResult {
        valid: true,
        errors: Vec::new(),
    };
    let mut index = 0usize;
    while index < bundle.lineage.len() {
        let record = bundle.lineage[index];
        index += 1;
        if !record.valid {
            continue;
        }
        if record.transition_node != 0 && bundle.node_by_id(record.transition_node).is_none() {
            result.valid = false;
            result
                .errors
                .push(ChronoscopeValidationError::InvalidLineageReference);
            break;
        }
        if record.result_checkpoint != ChronoscopeCheckpointId::NONE
            && bundle.checkpoint_by_id(record.result_checkpoint).is_none()
        {
            result.valid = false;
            result
                .errors
                .push(ChronoscopeValidationError::InvalidLineageReference);
            break;
        }
    }
    result
}

fn validate_adaptive_state(bundle: &ChronoscopeBundle) -> ChronoscopeValidationResult {
    let mut result = ChronoscopeValidationResult {
        valid: true,
        errors: Vec::new(),
    };
    let mut index = 0usize;
    while index < bundle.escalations.len() {
        let escalation = bundle.escalations[index];
        index += 1;
        if !escalation.valid {
            continue;
        }
        if bundle
            .anomalies
            .iter()
            .all(|anomaly| !anomaly.valid || anomaly.anomaly_id != escalation.anomaly_id)
        {
            result.valid = false;
            result
                .errors
                .push(ChronoscopeValidationError::InvalidAdaptiveLinkage);
            break;
        }
    }
    result
}

fn validate_bundle_invariants(bundle: &ChronoscopeBundle) -> ChronoscopeValidationResult {
    let mut result = ChronoscopeValidationResult {
        valid: true,
        errors: Vec::new(),
    };
    if bundle.perf.schema_version != CHRONOSCOPE_SCHEMA_VERSION {
        result.valid = false;
        result
            .errors
            .push(ChronoscopeValidationError::InvalidSchemaVersion);
    }
    let fabric = validate_event_fabric(&bundle.runtime_events);
    if !fabric.valid {
        result.valid = false;
        result.errors.extend(fabric.errors);
    }
    let lineage = validate_lineage(bundle);
    if !lineage.valid {
        result.valid = false;
        result.errors.extend(lineage.errors);
    }
    let adaptive = validate_adaptive_state(bundle);
    if !adaptive.valid {
        result.valid = false;
        result.errors.extend(adaptive.errors);
    }
    if bundle
        .temporal_explain
        .responsibility_ranking
        .windows(2)
        .any(|pair| pair[0].score < pair[1].score)
    {
        result.valid = false;
        result
            .errors
            .push(ChronoscopeValidationError::InvalidResponsibilityOrdering);
    }
    if bundle.temporal_explain.valid {
        let fault_node = bundle.temporal_explain.fault_node;
        if fault_node != 0 && bundle.node_by_id(fault_node).is_none() {
            result.valid = false;
            result
                .errors
                .push(ChronoscopeValidationError::InconsistentExplainPlan);
        }
        let last_writer = bundle.temporal_explain.last_writer;
        if last_writer != 0
            && !bundle
                .temporal_explain
                .writer_chain
                .iter()
                .any(|entry| *entry == last_writer)
        {
            result.valid = false;
            result
                .errors
                .push(ChronoscopeValidationError::InconsistentExplainPlan);
        }
    }
    result
}

fn chronoscope_runtime_event_semantic_hash(event: ChronoscopeRuntimeEventRecord) -> u64 {
    chronoscope_payload_hash(
        event.local_sequence,
        event.kind as u64,
        event.object_key,
        event.correlation.request_id ^ event.correlation.completion_id ^ event.correlation.irq_id,
    )
}

fn chronoscope_runtime_event_path(event: ChronoscopeRuntimeEventRecord) -> DiagnosticsPath {
    match event.kind {
        ChronoscopeRuntimeEventKind::IrqEnter
        | ChronoscopeRuntimeEventKind::IrqExit
        | ChronoscopeRuntimeEventKind::DivergenceHint => DiagnosticsPath::Irq,
        ChronoscopeRuntimeEventKind::RequestStart => DiagnosticsPath::Block,
        ChronoscopeRuntimeEventKind::RequestComplete => DiagnosticsPath::Completion,
        ChronoscopeRuntimeEventKind::FaultMarker => DiagnosticsPath::Fault,
        _ => DiagnosticsPath::Completion,
    }
}

fn chronoscope_runtime_event_kind_to_node_kind(
    kind: ChronoscopeRuntimeEventKind,
) -> ChronoscopeNodeKind {
    match kind {
        ChronoscopeRuntimeEventKind::ViolationObserved => ChronoscopeNodeKind::Constraint,
        ChronoscopeRuntimeEventKind::DivergenceHint => ChronoscopeNodeKind::Boundary,
        ChronoscopeRuntimeEventKind::FaultMarker => ChronoscopeNodeKind::Outcome,
        _ => ChronoscopeNodeKind::Observation,
    }
}

fn chronoscope_bridge_runtime_events(
    bundle: &mut ChronoscopeBundle,
    next_node: &mut usize,
    next_edge: &mut usize,
) {
    let total = bundle.runtime_events.total_events as usize;
    if total == 0 {
        return;
    }
    let start = total.saturating_sub(8);
    let mut first_runtime_node = 0u64;
    let mut previous_runtime_node = 0u64;
    let mut index = start;
    while index < total && *next_node < bundle.nodes.len() {
        let event = bundle.runtime_events.events[index];
        if !event.valid {
            index += 1;
            continue;
        }
        let node_id = *next_node as u64 + 1;
        let kind = chronoscope_runtime_event_kind_to_node_kind(event.kind);
        bundle.nodes[*next_node] = ChronoscopeNode {
            valid: true,
            node_id,
            stable_id: chronoscope_stable_node_id(
                kind,
                event.core_id,
                event.correlation.request_id,
                event.correlation.completion_id,
                event.correlation.irq_id,
                chronoscope_runtime_event_semantic_hash(event),
            ),
            event_sequence: event.event_id.0,
            kind,
            cpu_slot: event.core_id,
            stage: event.stage,
            path: chronoscope_runtime_event_path(event),
            request_id: event.correlation.request_id,
            completion_id: event.correlation.completion_id,
            irq_id: event.correlation.irq_id,
            score: match event.kind {
                ChronoscopeRuntimeEventKind::FaultMarker => 100,
                ChronoscopeRuntimeEventKind::ViolationObserved => 95,
                ChronoscopeRuntimeEventKind::DivergenceHint => 85,
                _ => 55,
            },
            confidence: match event.kind {
                ChronoscopeRuntimeEventKind::FaultMarker => 1.0,
                ChronoscopeRuntimeEventKind::ViolationObserved => 0.95,
                ChronoscopeRuntimeEventKind::DivergenceHint => 0.85,
                _ => 0.60,
            },
            severity: match event.kind {
                ChronoscopeRuntimeEventKind::FaultMarker => 5,
                ChronoscopeRuntimeEventKind::ViolationObserved => 5,
                ChronoscopeRuntimeEventKind::DivergenceHint => 4,
                _ => 2,
            },
            causal_distance_to_fault: u32::MAX,
            evidence_count: 1,
        };
        if first_runtime_node == 0 {
            first_runtime_node = node_id;
        }
        if previous_runtime_node != 0 && *next_edge < bundle.edges.len() {
            bundle.edges[*next_edge] = ChronoscopeEdge {
                valid: true,
                src_node_id: previous_runtime_node,
                dst_node_id: node_id,
                kind: ChronoscopeEdgeKind::ObservedBefore,
                weight: 100,
            };
            *next_edge += 1;
        }
        previous_runtime_node = node_id;
        *next_node += 1;
        index += 1;
    }
    if first_runtime_node != 0 && *next_edge < bundle.edges.len() {
        let mut obs = 0usize;
        while obs < bundle.nodes.len() {
            let node = bundle.nodes[obs];
            if node.valid
                && node.kind == ChronoscopeNodeKind::Observation
                && node.node_id != first_runtime_node
            {
                bundle.edges[*next_edge] = ChronoscopeEdge {
                    valid: true,
                    src_node_id: first_runtime_node,
                    dst_node_id: node.node_id,
                    kind: ChronoscopeEdgeKind::Supports,
                    weight: 60,
                };
                *next_edge += 1;
                break;
            }
            obs += 1;
        }
    }
}

fn chronoscope_collect_temporal_entries<const N: usize>(
    source: &[u128; N],
) -> ([ChronoscopeTemporalDiffEntry; N], usize) {
    let mut out = [ChronoscopeTemporalDiffEntry::EMPTY; N];
    let mut next = 0usize;
    let mut index = 0usize;
    while index < source.len() {
        let stable_id = source[index];
        index += 1;
        if stable_id == 0 {
            continue;
        }
        out[next] = ChronoscopeTemporalDiffEntry { stable_id };
        next += 1;
    }
    (out, next)
}

fn chronoscope_sort_temporal_entries<const N: usize>(
    entries: &mut [ChronoscopeTemporalDiffEntry; N],
    len: usize,
) {
    if len <= 1 {
        return;
    }
    let mut scratch = [ChronoscopeTemporalDiffEntry::EMPTY; N];
    let mut width = 1usize;
    while width < len {
        let mut start = 0usize;
        while start < len {
            let mid = (start + width).min(len);
            let end = (start + width * 2).min(len);
            let mut left = start;
            let mut right = mid;
            let mut out = start;
            while left < mid && right < end {
                if entries[left].stable_id <= entries[right].stable_id {
                    scratch[out] = entries[left];
                    left += 1;
                } else {
                    scratch[out] = entries[right];
                    right += 1;
                }
                out += 1;
            }
            while left < mid {
                scratch[out] = entries[left];
                left += 1;
                out += 1;
            }
            while right < end {
                scratch[out] = entries[right];
                right += 1;
                out += 1;
            }
            let mut copy = start;
            while copy < end {
                entries[copy] = scratch[copy];
                copy += 1;
            }
            start = end;
        }
        width *= 2;
    }
}

fn build_chronoscope_bundle(capsule: &CrashCapsule) -> ChronoscopeBundle {
    if !capsule.valid {
        return ChronoscopeBundle::EMPTY;
    }
    let mut bundle = ChronoscopeBundle {
        valid: true,
        generation: capsule.generation,
        failure_signature_id: capsule.failure_signature_id,
        top_suspect_confidence: capsule
            .suspects
            .iter()
            .copied()
            .find(|suspect| suspect.valid)
            .map(|suspect| suspect_confidence(suspect, capsule))
            .unwrap_or(0),
        root_boundary: earliest_preventable_boundary(capsule),
        dominant_suspect_node_id: 0,
        strongest_chain: [0; 8],
        nodes: [ChronoscopeNode::EMPTY; CHRONOSCOPE_NODE_LIMIT],
        edges: [ChronoscopeEdge::EMPTY; CHRONOSCOPE_EDGE_LIMIT],
        runtime_events: snapshot_runtime_events(capsule.replay),
        checkpoints: [ChronoscopeCheckpoint::EMPTY; CHRONOSCOPE_CHECKPOINT_LIMIT],
        lineage: [ChronoscopeLineageRecord::EMPTY; CHRONOSCOPE_LINEAGE_LIMIT],
        last_writers: [LastWriterRecord::EMPTY; CHRONOSCOPE_LAST_WRITER_LIMIT],
        last_writer_index: [LastWriterIndexEntry::EMPTY; CHRONOSCOPE_LAST_WRITER_INDEX_LIMIT],
        writer_predecessor_by_node: [0; CHRONOSCOPE_NODE_LIMIT],
        capability_derivations: [CapabilityDerivationRecord::EMPTY; CHRONOSCOPE_CAPABILITY_LIMIT],
        capability_parent_index: [CapabilityParentIndexEntry::EMPTY;
            CHRONOSCOPE_CAPABILITY_INDEX_LIMIT],
        node_capabilities: [CapabilityId::NONE; CHRONOSCOPE_NODE_LIMIT],
        propagation: [PropagationRecord::EMPTY; CHRONOSCOPE_PROPAGATION_LIMIT],
        propagation_heads: [CHRONOSCOPE_INVALID_INDEX; CHRONOSCOPE_NODE_LIMIT],
        propagation_next: [CHRONOSCOPE_INVALID_INDEX; CHRONOSCOPE_PROPAGATION_LIMIT],
        responsibility: [ResponsibilityEntry::EMPTY; CHRONOSCOPE_RESPONSIBILITY_LIMIT],
        anomalies: [ChronoscopeAnomalyRecord::EMPTY; CHRONOSCOPE_ANOMALY_LIMIT],
        escalations: [ChronoscopeEscalationRecord::EMPTY; CHRONOSCOPE_ESCALATION_LIMIT],
        capture_windows: [ChronoscopeCaptureWindow::EMPTY; CHRONOSCOPE_CAPTURE_WINDOW_LIMIT],
        candidates: ChronoscopeCandidateSet::EMPTY,
        adaptive_state: ChronoscopeAdaptiveState::Normal,
        adaptive_transitions: [ChronoscopeAdaptiveTransition::EMPTY;
            CHRONOSCOPE_ADAPTIVE_TRANSITION_LIMIT],
        integrity: ChronoscopeHistoryIntegrity::COMPLETE,
        perf: ChronoscopePerfCounters::EMPTY,
        trust: ChronoscopeTrustSurface::EMPTY,
        temporal_explain: ExplainPlan::EMPTY,
        primary_fault_checkpoint: ChronoscopeCheckpointId::NONE,
        rewind_checkpoint: ChronoscopeCheckpointId::NONE,
        divergence_checkpoint: ChronoscopeCheckpointId::NONE,
    };
    let mut next_node = 0usize;
    let mut next_edge = 0usize;
    let mut last_trace_node = 0u64;
    for entry in focused_events(capsule).into_iter().flatten() {
        if next_node >= bundle.nodes.len() {
            break;
        }
        let node_id = next_node as u64 + 1;
        bundle.nodes[next_node] = ChronoscopeNode {
            valid: true,
            node_id,
            stable_id: chronoscope_stable_node_id(
                ChronoscopeNodeKind::Observation,
                entry.cpu_slot,
                focused_request_id(entry, &capsule.replay),
                focused_completion_id(entry, &capsule.replay),
                focused_irq_id(entry, &capsule.replay),
                chronoscope_payload_hash(
                    entry.sequence,
                    entry.kind as u64,
                    dominant_path_for_trace(entry, capsule) as u64,
                    entry.a ^ entry.b ^ entry.c ^ entry.d,
                ),
            ),
            event_sequence: entry.sequence,
            kind: ChronoscopeNodeKind::Observation,
            cpu_slot: entry.cpu_slot,
            stage: entry.stage,
            path: dominant_path_for_trace(entry, capsule),
            request_id: focused_request_id(entry, &capsule.replay),
            completion_id: focused_completion_id(entry, &capsule.replay),
            irq_id: focused_irq_id(entry, &capsule.replay),
            score: 0,
            confidence: 0.55,
            severity: 1,
            causal_distance_to_fault: u32::MAX,
            evidence_count: 1,
        };
        if last_trace_node != 0 && next_edge < bundle.edges.len() {
            bundle.edges[next_edge] = ChronoscopeEdge {
                valid: true,
                src_node_id: last_trace_node,
                dst_node_id: node_id,
                kind: ChronoscopeEdgeKind::ObservedBefore,
                weight: 100,
            };
            next_edge += 1;
        }
        last_trace_node = node_id;
        next_node += 1;
    }
    chronoscope_bridge_runtime_events(&mut bundle, &mut next_node, &mut next_edge);
    for suspect in capsule
        .suspects
        .iter()
        .copied()
        .filter(|suspect| suspect.valid)
    {
        if next_node >= bundle.nodes.len() {
            break;
        }
        let node_id = next_node as u64 + 1;
        bundle.nodes[next_node] = ChronoscopeNode {
            valid: true,
            node_id,
            stable_id: chronoscope_stable_node_id(
                ChronoscopeNodeKind::Interpretation,
                suspect.cpu_slot,
                suspect.request_id,
                suspect.completion_id,
                suspect.irq_id,
                chronoscope_payload_hash(
                    suspect.event_sequence,
                    suspect.reason_code as u64,
                    suspect.event_kind as u64,
                    suspect.score as u64,
                ),
            ),
            event_sequence: suspect.event_sequence,
            kind: ChronoscopeNodeKind::Interpretation,
            cpu_slot: suspect.cpu_slot,
            stage: suspect.stage,
            path: dominant_path_for_suspect(suspect, capsule),
            request_id: suspect.request_id,
            completion_id: suspect.completion_id,
            irq_id: suspect.irq_id,
            score: suspect.score,
            confidence: suspect_confidence(suspect, capsule) as f32 / 100.0,
            severity: 4,
            causal_distance_to_fault: u32::MAX,
            evidence_count: 1,
        };
        if let Some(trace_node) = bundle.nodes.iter().find(|node| {
            node.valid
                && node.kind == ChronoscopeNodeKind::Observation
                && node.event_sequence == suspect.event_sequence
        }) {
            if next_edge < bundle.edges.len() {
                bundle.edges[next_edge] = ChronoscopeEdge {
                    valid: true,
                    src_node_id: trace_node.node_id,
                    dst_node_id: node_id,
                    kind: ChronoscopeEdgeKind::Supports,
                    weight: suspect.score,
                };
                next_edge += 1;
            }
        }
        next_node += 1;
    }
    for violation in capsule
        .watch_tail
        .iter()
        .copied()
        .filter(|entry| entry.sequence != 0)
    {
        if next_node >= bundle.nodes.len() {
            break;
        }
        let node_id = next_node as u64 + 1;
        bundle.nodes[next_node] = ChronoscopeNode {
            valid: true,
            node_id,
            stable_id: chronoscope_stable_node_id(
                ChronoscopeNodeKind::Constraint,
                violation.cpu_slot,
                violation.request_id,
                violation.completion_id,
                capsule.replay.irq_id,
                chronoscope_payload_hash(
                    violation.sequence,
                    violation.descriptor_id,
                    violation.overlap as u64,
                    violation.suspicion_flags as u64,
                ),
            ),
            event_sequence: violation.sequence,
            kind: ChronoscopeNodeKind::Constraint,
            cpu_slot: violation.cpu_slot,
            stage: violation.stage,
            path: violation.path,
            request_id: violation.request_id,
            completion_id: violation.completion_id,
            irq_id: capsule.replay.irq_id,
            score: if violation.kind == ViolationKind::Guard {
                95
            } else {
                80
            },
            confidence: if violation.kind == ViolationKind::Guard {
                0.95
            } else {
                0.80
            },
            severity: if violation.kind == ViolationKind::Guard {
                5
            } else {
                4
            },
            causal_distance_to_fault: u32::MAX,
            evidence_count: 1,
        };
        if let Some(suspect_node) = bundle.nodes.iter().find(|node| {
            node.valid
                && node.kind == ChronoscopeNodeKind::Interpretation
                && (node.request_id == violation.request_id
                    || node.completion_id == violation.completion_id)
        }) {
            if next_edge < bundle.edges.len() {
                bundle.edges[next_edge] = ChronoscopeEdge {
                    valid: true,
                    src_node_id: node_id,
                    dst_node_id: suspect_node.node_id,
                    kind: ChronoscopeEdgeKind::Violates,
                    weight: bundle.nodes[next_node].score,
                };
                next_edge += 1;
            }
        }
        next_node += 1;
    }
    if capsule.fault.valid && next_node < bundle.nodes.len() {
        let fault_node_id = next_node as u64 + 1;
        bundle.nodes[next_node] = ChronoscopeNode {
            valid: true,
            node_id: fault_node_id,
            stable_id: chronoscope_stable_node_id(
                ChronoscopeNodeKind::Outcome,
                capsule.fault.cpu_slot,
                capsule.replay.request_id,
                capsule.replay.completion_id,
                capsule.replay.irq_id,
                chronoscope_payload_hash(
                    capsule.fault.rip,
                    capsule.fault.vector,
                    capsule.fault.error_code,
                    capsule.fault.cr2,
                ),
            ),
            event_sequence: latest_event_sequence(capsule.fault.cpu_slot as usize),
            kind: ChronoscopeNodeKind::Outcome,
            cpu_slot: capsule.fault.cpu_slot,
            stage: capsule.fault.stage,
            path: DiagnosticsPath::Fault,
            request_id: capsule.replay.request_id,
            completion_id: capsule.replay.completion_id,
            irq_id: capsule.replay.irq_id,
            score: 100,
            confidence: 1.0,
            severity: 5,
            causal_distance_to_fault: 0,
            evidence_count: 1,
        };
        if last_trace_node != 0 && next_edge < bundle.edges.len() {
            bundle.edges[next_edge] = ChronoscopeEdge {
                valid: true,
                src_node_id: last_trace_node,
                dst_node_id: fault_node_id,
                kind: ChronoscopeEdgeKind::LeadsTo,
                weight: 100,
            };
            next_edge += 1;
        }
        next_node += 1;
    }
    if bundle.root_boundary.valid && next_node < bundle.nodes.len() {
        let node_id = next_node as u64 + 1;
        bundle.nodes[next_node] = ChronoscopeNode {
            valid: true,
            node_id,
            stable_id: chronoscope_stable_node_id(
                ChronoscopeNodeKind::Boundary,
                0,
                bundle.root_boundary.request_id,
                bundle.root_boundary.completion_id,
                bundle.root_boundary.irq_id,
                chronoscope_payload_hash(
                    bundle.root_boundary.sequence,
                    bundle.root_boundary.stage as u64,
                    bundle.root_boundary.path as u64,
                    0,
                ),
            ),
            event_sequence: bundle.root_boundary.sequence,
            kind: ChronoscopeNodeKind::Boundary,
            cpu_slot: 0,
            stage: bundle.root_boundary.stage,
            path: bundle.root_boundary.path,
            request_id: bundle.root_boundary.request_id,
            completion_id: bundle.root_boundary.completion_id,
            irq_id: bundle.root_boundary.irq_id,
            score: 100,
            confidence: 0.9,
            severity: 4,
            causal_distance_to_fault: u32::MAX,
            evidence_count: 1,
        };
        if let Some(suspect_node) = bundle
            .nodes
            .iter()
            .find(|node| node.valid && node.kind == ChronoscopeNodeKind::Interpretation)
        {
            if next_edge < bundle.edges.len() {
                bundle.edges[next_edge] = ChronoscopeEdge {
                    valid: true,
                    src_node_id: node_id,
                    dst_node_id: suspect_node.node_id,
                    kind: ChronoscopeEdgeKind::PreventableAt,
                    weight: 100,
                };
            }
        }
    }
    chronoscope_finalize_bundle(&mut bundle);
    chronoscope_infer_temporal_checkpoints(&mut bundle, capsule);
    PERF_CHECKPOINTS.store(
        bundle
            .checkpoints
            .iter()
            .filter(|entry| entry.valid)
            .count() as u64,
        Ordering::Relaxed,
    );
    chronoscope_attach_replay_positions(&mut bundle);
    chronoscope_build_lineage(&mut bundle);
    PERF_LINEAGE_RECORDS.store(
        bundle.lineage.iter().filter(|entry| entry.valid).count() as u64,
        Ordering::Relaxed,
    );
    chronoscope_build_last_writers(&mut bundle);
    chronoscope_build_capability_derivations(&mut bundle);
    chronoscope_build_propagation(&mut bundle);
    chronoscope_build_responsibility(&mut bundle);
    chronoscope_build_anomaly_and_adaptive(&mut bundle);
    PERF_ESCALATIONS.store(
        bundle
            .escalations
            .iter()
            .filter(|entry| entry.valid)
            .count() as u64,
        Ordering::Relaxed,
    );
    chronoscope_build_candidates(&mut bundle);
    bundle.temporal_explain = chronoscope_build_temporal_explain_plan(&bundle);
    chronoscope_build_trust_surface(&mut bundle);
    bundle
}

fn build_memory_lineage_summary() -> MemoryLineageSummary {
    let lineage = unsafe { *TRACE_STORAGE.memory_lineage.get() };
    let mut summary = MemoryLineageSummary::EMPTY;
    for entry in lineage.iter().copied().filter(|entry| entry.valid) {
        summary.total_versions = summary.total_versions.saturating_add(1);
        match entry.kind {
            MemoryLineageKind::Snapshot => {}
            MemoryLineageKind::Write => summary.writes = summary.writes.saturating_add(1),
            MemoryLineageKind::Copy => summary.copies = summary.copies.saturating_add(1),
            MemoryLineageKind::Zero => summary.zeros = summary.zeros.saturating_add(1),
            MemoryLineageKind::Dma => summary.dmas = summary.dmas.saturating_add(1),
            MemoryLineageKind::Free => summary.frees = summary.frees.saturating_add(1),
        }
        if entry.version_id >= summary.latest_version_id {
            summary.latest_version_id = entry.version_id;
            summary.latest_parent_version_id = entry.parent_version_id;
            summary.latest_digest = entry.digest;
        }
        let object_versions = lineage
            .iter()
            .filter(|other| {
                other.valid && other.object_id != 0 && other.object_id == entry.object_id
            })
            .count() as u16;
        if object_versions > summary.hottest_object_versions {
            summary.hottest_object_versions = object_versions;
            summary.hottest_object_id = entry.object_id;
        }
    }
    summary
}

fn last_memory_version_for_range(address_space_id: u64, base: u64, len: u64) -> u64 {
    let hint_slot = memory_lineage_hint_slot(address_space_id, base);
    let hints = unsafe { &*TRACE_STORAGE.memory_lineage_hints.get() };
    let hint = hints[hint_slot];
    if hint.valid
        && hint.address_space_id == address_space_id
        && ranges_overlap(base, len, hint.base, hint.len)
    {
        return hint.latest_version_id;
    }
    let lineage = unsafe { *TRACE_STORAGE.memory_lineage.get() };
    let mut best = 0u64;
    for entry in lineage.iter().copied().filter(|entry| entry.valid) {
        let overlap = entry.address_space_id == address_space_id
            && ranges_overlap(base, len, entry.base, entry.len);
        if overlap && entry.version_id > best {
            best = entry.version_id;
        }
    }
    best
}

#[inline(always)]
fn ranges_overlap(left_base: u64, left_len: u64, right_base: u64, right_len: u64) -> bool {
    left_base < right_base.saturating_add(right_len)
        && left_base.saturating_add(left_len) > right_base
}

#[inline(always)]
fn memory_lineage_hint_slot(address_space_id: u64, base: u64) -> usize {
    ((address_space_id ^ (base >> 12)) as usize) % MEMORY_LINEAGE_HINT_CAPACITY
}

fn update_memory_lineage_hint(address_space_id: u64, base: u64, len: u64, version_id: u64) {
    let slot = memory_lineage_hint_slot(address_space_id, base);
    unsafe {
        (*TRACE_STORAGE.memory_lineage_hints.get())[slot] = MemoryLineageHint {
            valid: true,
            address_space_id,
            base,
            len,
            latest_version_id: version_id,
        };
    }
}

fn memory_lineage_digest(
    parent_version_id: u64,
    address_space_id: u64,
    base: u64,
    len: u64,
    object_id: u64,
    bytes_changed: u64,
    digest_seed: u64,
    kind_tag: u64,
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    hash = fnv_mix(hash, parent_version_id);
    hash = fnv_mix(hash, address_space_id);
    hash = fnv_mix(hash, base);
    hash = fnv_mix(hash, len);
    hash = fnv_mix(hash, object_id);
    hash = fnv_mix(hash, bytes_changed);
    hash = fnv_mix(hash, digest_seed);
    fnv_mix(hash, kind_tag)
}

#[inline(always)]
fn violation_hint_slot(descriptor_id: u64) -> usize {
    (descriptor_id as usize) % VIOLATION_HINT_CAPACITY
}

#[inline(always)]
fn last_violation_hint(descriptor_id: u64) -> ViolationHint {
    unsafe { (*TRACE_STORAGE.violation_hints.get())[violation_hint_slot(descriptor_id)] }
}

fn update_violation_hint(
    descriptor_id: u64,
    kind: ViolationKind,
    request_id: u64,
    completion_id: u64,
) {
    let slot = violation_hint_slot(descriptor_id);
    unsafe {
        (*TRACE_STORAGE.violation_hints.get())[slot] = ViolationHint {
            valid: true,
            descriptor_id,
            kind,
            request_id,
            completion_id,
        };
    }
}

fn emit_causal_ledger() {
    serial::write_bytes(b"== causal-ledger ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: causal-ledger none reason=no-crash-capsule\n");
        return;
    }
    for (index, edge) in build_causal_ledger(&capsule)
        .into_iter()
        .enumerate()
        .filter(|(_, edge)| edge.valid)
    {
        serial::print(format_args!(
            "ngos/x86_64: causal-edge rank={} kind={} stage={} cpu={} req={} cmp={} irq={} seq={} reason={}\n",
            index + 1,
            causal_edge_name(edge.kind),
            stage_name(edge.stage),
            edge.cpu_slot,
            edge.request_id,
            edge.completion_id,
            edge.irq_id,
            edge.sequence,
            edge.reason
        ));
    }
}

fn emit_earliest_preventable_boundary() {
    serial::write_bytes(b"== earliest-preventable-boundary ==\n");
    let capsule = crash_capsule();
    let boundary = earliest_preventable_boundary(&capsule);
    if !boundary.valid {
        serial::write_bytes(b"ngos/x86_64: earliest-preventable none reason=no-suspect\n");
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: earliest-preventable stage={} path={} seq={} req={} cmp={} irq={} reason={} action={}\n",
        stage_name(boundary.stage),
        diagnostics_path_name(boundary.path),
        boundary.sequence,
        boundary.request_id,
        boundary.completion_id,
        boundary.irq_id,
        boundary.reason,
        boundary.action
    ));
}

fn emit_earliest_preventable_boundary_compact() {
    let capsule = crash_capsule();
    let boundary = earliest_preventable_boundary(&capsule);
    if !boundary.valid {
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: earliest-preventable stage={} path={} action={}\n",
        stage_name(boundary.stage),
        diagnostics_path_name(boundary.path),
        boundary.action
    ));
}

fn emit_invariant_coverage() {
    serial::write_bytes(b"== invariant-coverage ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: invariant-coverage none reason=no-crash-capsule\n");
        return;
    }
    for entry in build_invariant_coverage_map(&capsule)
        .entries
        .into_iter()
        .filter(|entry| !entry.name.is_empty())
    {
        serial::print(format_args!(
            "ngos/x86_64: invariant name={} status={} stage={} path={} reason={}\n",
            entry.name,
            invariant_status_name(entry.status),
            stage_name(entry.stage),
            diagnostics_path_name(entry.path),
            entry.reason
        ));
    }
}

fn emit_differential_flow() {
    serial::write_bytes(b"== differential-flow ==\n");
    let current = crash_capsule();
    let baseline = same_pattern_baseline(&current).unwrap_or_else(previous_crash_capsule);
    let report = compare_differential_flows(&current, &baseline);
    if !report.has_baseline {
        serial::write_bytes(b"ngos/x86_64: differential-flow none reason=no-baseline\n");
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: differential-flow baseline_pattern={} stable_prefix={} unstable_suffix={} divergence_seq={} reason={}\n",
        report.baseline_signature_id,
        report.stable_prefix,
        report.unstable_suffix,
        report.first_divergence_sequence,
        report.reason
    ));
}

fn emit_semantic_race_report() {
    serial::write_bytes(b"== semantic-race ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: semantic-race none reason=no-crash-capsule\n");
        return;
    }
    let race = detect_semantic_race(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: semantic-race likely={} score={} req={} cmp={} irq={} cpu_a={} cpu_b={} reason={}\n",
        race.likely,
        race.score,
        race.request_id,
        race.completion_id,
        race.irq_id,
        race.cpu_a,
        race.cpu_b,
        if race.reason.is_empty() {
            "none"
        } else {
            race.reason
        }
    ));
}

fn emit_replay_window_summary() {
    serial::write_bytes(b"== replay-window ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: replay-window none reason=no-crash-capsule\n");
        return;
    }
    let window = replay_window_summary(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: replay-window seq={} req={} cmp={} irq={} stable_prefix={} unstable_suffix={} divergence_seq={} deterministic={} reason={}\n",
        window.sequence,
        window.request_id,
        window.completion_id,
        window.irq_id,
        window.stable_prefix,
        window.unstable_suffix,
        window.first_divergence_sequence,
        window.deterministic,
        window.reason
    ));
}

pub fn snapshot_trace() -> [[TraceRecord; TRACE_CAPACITY]; MAX_TRACE_CPUS] {
    unsafe { *TRACE_STORAGE.records.get() }
}

pub fn last_fault() -> FaultRecord {
    last_faults()
        .into_iter()
        .find(|fault| fault.valid)
        .unwrap_or(FaultRecord::EMPTY)
}

pub fn last_faults() -> [FaultRecord; MAX_TRACE_CPUS] {
    unsafe { *TRACE_STORAGE.last_faults.get() }
}

pub fn last_user_status() -> Option<FirstUserProcessStatus> {
    if !TRACE_STORAGE.last_user_status_valid.load(Ordering::SeqCst) {
        return None;
    }
    Some(unsafe { (*TRACE_STORAGE.last_user_status.get()).assume_init_read() })
}

pub fn boot_locator_snapshot() -> BootLocatorRecord {
    boot_locator::snapshot()
}

#[allow(dead_code)]
pub fn early_boot_locator_snapshot() -> EarlyBootSnapshot {
    boot_locator::early_snapshot()
}

#[allow(dead_code)]
pub fn recent_boot_locator(limit: usize) -> [BootLocatorRecord; 64] {
    boot_locator::recent(limit)
}

fn record_violation(
    kind: ViolationKind,
    descriptor_id: u64,
    descriptor_kind: u16,
    address: u64,
    length: u64,
    descriptor_address: u64,
    descriptor_length: u64,
    red_zone: u64,
    stage: u16,
    path: DiagnosticsPath,
    request_id: u64,
    completion_id: u64,
) {
    let context = current_trace_context();
    let local_sequence = next_local_trace_sequence(context.cpu_slot);
    let sequence = compose_event_sequence(context.cpu_slot, local_sequence);
    let slot = (local_sequence as usize - 1) % VIOLATION_TAIL;
    let (overlap, mut suspicion_flags, relative_start, relative_end) = classify_memory_overlap(
        descriptor_address,
        descriptor_length,
        red_zone,
        address,
        length,
    );
    let violation_hint = last_violation_hint(descriptor_id);
    if violation_hint.valid
        && violation_hint.descriptor_id == descriptor_id
        && violation_hint.kind == kind
        && violation_hint.request_id == request_id
        && violation_hint.completion_id == completion_id
    {
        suspicion_flags |= MEMORY_SUSPECT_REPEATED;
    }
    if violation_hint.valid
        && violation_hint.descriptor_id == descriptor_id
        && violation_hint.request_id != 0
        && request_id != 0
        && violation_hint.request_id != request_id
    {
        suspicion_flags |= MEMORY_SUSPECT_CROSS_REQUEST;
    }
    if violation_hint.valid
        && violation_hint.descriptor_id == descriptor_id
        && violation_hint.completion_id != 0
        && completion_id != 0
        && violation_hint.completion_id != completion_id
    {
        suspicion_flags |= MEMORY_SUSPECT_CROSS_COMPLETION;
    }
    unsafe {
        (*TRACE_STORAGE.violations.get())[slot] = ViolationRecord {
            sequence,
            kind,
            descriptor_id,
            descriptor_kind,
            address,
            length,
            descriptor_address,
            descriptor_length,
            overlap,
            suspicion_flags,
            relative_start,
            relative_end,
            stage,
            path,
            request_id,
            completion_id,
            cpu_slot: context.cpu_slot as u16,
            apic_id: context.apic_id,
        };
        update_violation_hint(descriptor_id, kind, request_id, completion_id);
    }
    let _ = emit_runtime_event_with_context(
        context,
        ChronoscopeRuntimeEventKind::ViolationObserved,
        descriptor_id,
        ChronoscopeEventId::NONE,
        suspicion_flags,
        ChronoscopeRuntimePayload::Violation {
            violation_kind: kind as u16,
            descriptor_kind,
            score: if kind == ViolationKind::Guard { 95 } else { 80 },
            flags: suspicion_flags,
        },
    );
    push_with_context(
        TraceKind::Memory,
        TraceChannel::Memory,
        stage,
        context,
        current_uptime(),
        descriptor_id,
        address,
        length,
        kind as u64,
    );
}

fn classify_memory_overlap(
    descriptor_address: u64,
    descriptor_length: u64,
    red_zone: u64,
    access_address: u64,
    access_length: u64,
) -> (MemoryOverlapClass, u16, i64, i64) {
    let descriptor_start = descriptor_address;
    let descriptor_end = descriptor_address.saturating_add(descriptor_length);
    let access_end = access_address.saturating_add(access_length);
    let relative_start = access_address as i128 - descriptor_start as i128;
    let relative_end = access_end as i128 - descriptor_end as i128;
    let mut flags = 0u16;
    if access_address < descriptor_start {
        flags |= MEMORY_SUSPECT_UNDERRUN;
    }
    if access_end > descriptor_end {
        flags |= MEMORY_SUSPECT_OVERRUN;
    }
    if access_address == descriptor_start && access_end == descriptor_end {
        flags |= MEMORY_SUSPECT_EXACT;
        return (
            MemoryOverlapClass::Exact,
            flags,
            relative_start as i64,
            relative_end as i64,
        );
    }
    if access_address >= descriptor_start && access_end <= descriptor_end {
        flags |= MEMORY_SUSPECT_INTERIOR;
        return (
            MemoryOverlapClass::Interior,
            flags,
            relative_start as i64,
            relative_end as i64,
        );
    }
    if access_address < descriptor_start && access_end > descriptor_end {
        flags |= MEMORY_SUSPECT_WIDE_SPAN;
        return (
            MemoryOverlapClass::Span,
            flags,
            relative_start as i64,
            relative_end as i64,
        );
    }
    if access_address < descriptor_start {
        let class = if red_zone != 0 && access_end <= descriptor_start.saturating_add(red_zone) {
            MemoryOverlapClass::LeftRedZone
        } else {
            MemoryOverlapClass::Prefix
        };
        return (class, flags, relative_start as i64, relative_end as i64);
    }
    let class = if red_zone != 0 && access_address >= descriptor_end.saturating_sub(red_zone) {
        MemoryOverlapClass::RightRedZone
    } else {
        MemoryOverlapClass::Suffix
    };
    (class, flags, relative_start as i64, relative_end as i64)
}

pub fn resolve_address(address: u64) -> Option<ResolvedSymbol> {
    let landmarks = symbol_landmarks();
    let mut best = None::<SymbolLandmark>;
    for landmark in landmarks.iter().copied() {
        if address >= landmark.base {
            match best {
                Some(current) if current.base >= landmark.base => {}
                _ => best = Some(landmark),
            }
        }
    }
    best.map(|landmark| ResolvedSymbol {
        name: landmark.name,
        base: landmark.base,
        offset: address.saturating_sub(landmark.base),
    })
}

#[inline(always)]
fn push_with_context(
    kind: TraceKind,
    channel: TraceChannel,
    stage: u16,
    context: CpuTraceContext,
    uptime_us: Option<u64>,
    a: u64,
    b: u64,
    c: u64,
    d: u64,
) {
    let local_sequence = next_local_trace_sequence(context.cpu_slot);
    let sequence = compose_event_sequence(context.cpu_slot, local_sequence);
    let slot = (local_sequence as usize - 1) % TRACE_CAPACITY;
    unsafe {
        write_trace_record(
            core::ptr::addr_of_mut!((*TRACE_STORAGE.records.get())[context.cpu_slot][slot]),
            sequence,
            uptime_us.unwrap_or(0),
            context.apic_id,
            context.cpu_slot as u16,
            kind,
            channel,
            stage,
            a,
            b,
            c,
            d,
        );
    }
}

unsafe fn write_trace_record(
    target: *mut TraceRecord,
    sequence: u64,
    uptime_us: u64,
    apic_id: u32,
    cpu_slot: u16,
    kind: TraceKind,
    channel: TraceChannel,
    stage: u16,
    a: u64,
    b: u64,
    c: u64,
    d: u64,
) {
    unsafe {
        core::ptr::write(core::ptr::addr_of_mut!((*target).sequence), sequence);
        core::ptr::write(core::ptr::addr_of_mut!((*target).uptime_us), uptime_us);
        core::ptr::write(core::ptr::addr_of_mut!((*target).apic_id), apic_id);
        core::ptr::write(core::ptr::addr_of_mut!((*target).cpu_slot), cpu_slot);
        core::ptr::write(core::ptr::addr_of_mut!((*target).kind), kind);
        core::ptr::write(core::ptr::addr_of_mut!((*target).channel), channel);
        core::ptr::write(core::ptr::addr_of_mut!((*target).stage), stage);
        core::ptr::write(core::ptr::addr_of_mut!((*target).a), a);
        core::ptr::write(core::ptr::addr_of_mut!((*target).b), b);
        core::ptr::write(core::ptr::addr_of_mut!((*target).c), c);
        core::ptr::write(core::ptr::addr_of_mut!((*target).d), d);
    }
}

fn current_stage_for(cpu_slot: usize) -> u16 {
    CPU_CURRENT_STAGE[cpu_slot].load(Ordering::Relaxed) as u16
}

#[inline(always)]
fn compose_event_sequence(cpu_slot: usize, local_sequence: u64) -> u64 {
    (local_sequence << TRACE_SEQUENCE_CPU_BITS) | ((cpu_slot as u64) & TRACE_SEQUENCE_CPU_MASK)
}

#[inline(always)]
fn next_local_trace_sequence(cpu_slot: usize) -> u64 {
    CPU_TRACE_SEQUENCES[cpu_slot].fetch_add(1, Ordering::Relaxed) + 1
}

#[cfg_attr(test, allow(dead_code))]
#[inline(always)]
fn next_event_sequence(cpu_slot: usize) -> u64 {
    let local = next_local_trace_sequence(cpu_slot);
    compose_event_sequence(cpu_slot, local)
}

#[inline(always)]
fn latest_event_sequence(cpu_slot: usize) -> u64 {
    let local = CPU_TRACE_SEQUENCES[cpu_slot].load(Ordering::Relaxed);
    if local == 0 {
        0
    } else {
        compose_event_sequence(cpu_slot, local)
    }
}

#[inline(always)]
fn current_trace_context() -> CpuTraceContext {
    let apic_id = current_apic_id();
    let cpu_slot = register_cpu_slot(apic_id);
    CpuTraceContext {
        cpu_slot,
        apic_id,
        stage: current_stage_for(cpu_slot),
    }
}

fn current_cpu_context() -> (usize, u32) {
    let apic_id = current_apic_id();
    let slot = register_cpu_slot(apic_id);
    (slot, apic_id)
}

fn register_cpu_slot(apic_id: u32) -> usize {
    let apic_id = u64::from(apic_id);
    let mut slot = 0usize;
    while slot < MAX_TRACE_CPUS {
        let current = CPU_APIC_IDS[slot].load(Ordering::Acquire);
        if current == apic_id {
            return slot;
        }
        if current == INVALID_APIC_ID
            && CPU_APIC_IDS[slot]
                .compare_exchange(
                    INVALID_APIC_ID,
                    apic_id,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
        {
            return slot;
        }
        slot += 1;
    }
    0
}

fn current_apic_id() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        let leaf = core::arch::x86_64::__cpuid(1);
        (leaf.ebx >> 24) & 0xff
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

fn capture_control_registers(_cr2_fallback: u64) -> ControlRegisterSnapshot {
    #[cfg(target_os = "none")]
    {
        let mut cr0: u64;
        let mut cr2: u64;
        let mut cr3: u64;
        let mut cr4: u64;
        let mut efer_lo: u32;
        let mut efer_hi: u32;
        unsafe {
            asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
            asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
            asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
            asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
            asm!(
                "rdmsr",
                in("ecx") 0xc000_0080u32,
                out("eax") efer_lo,
                out("edx") efer_hi,
                options(nostack, preserves_flags)
            );
        }
        ControlRegisterSnapshot {
            cr0,
            cr2,
            cr3,
            cr4,
            efer: ((efer_hi as u64) << 32) | u64::from(efer_lo),
        }
    }
    #[cfg(not(target_os = "none"))]
    {
        let _ = _cr2_fallback;
        ControlRegisterSnapshot::EMPTY
    }
}

fn current_uptime() -> Option<u64> {
    #[cfg(target_os = "none")]
    {
        crate::timer::boot_uptime_micros()
    }
    #[cfg(not(target_os = "none"))]
    {
        None
    }
}

#[cfg_attr(test, allow(dead_code))]
fn emit_fault_packet(record: &FaultRecord) {
    serial::print(format_args!(
        "ngos/x86_64: fault cpu_slot={} apic={} stage={} stage_name={} vector={} vector_name={} rip={:#x} rip_sym={} cs={:#x} rflags={:#x} error={:#x} locator_seq={} locator_stage={} locator_checkpoint={:#x} locator_p0={:#x} locator_p1={:#x}\n",
        record.cpu_slot,
        record.apic_id,
        record.stage,
        stage_name(record.stage),
        record.vector,
        fault_vector_name(record.vector),
        record.rip,
        render_symbol(record.rip),
        record.cs,
        record.rflags,
        record.error_code,
        record.locator_sequence,
        record.locator_stage,
        record.locator_checkpoint,
        record.locator_payload0,
        record.locator_payload1
    ));
    serial::print(format_args!(
        "ngos/x86_64: diag control cr0={:#x} cr2={:#x} cr3={:#x} cr4={:#x} efer={:#x}\n",
        record.cr0, record.cr2, record.cr3, record.cr4, record.efer
    ));
    serial::print(format_args!(
        "ngos/x86_64: diag regs rax={:#x} rbx={:#x} rcx={:#x} rdx={:#x} rsi={:#x} rdi={:#x} rbp={:#x}\n",
        record.rax, record.rbx, record.rcx, record.rdx, record.rsi, record.rdi, record.rbp
    ));
    serial::print(format_args!(
        "ngos/x86_64: diag regs r8={:#x} r9={:#x} r10={:#x} r11={:#x} r12={:#x} r13={:#x} r14={:#x} r15={:#x}\n",
        record.r8,
        record.r9,
        record.r10,
        record.r11,
        record.r12,
        record.r13,
        record.r14,
        record.r15
    ));
}

fn emit_crash_capsule() {
    let capsule = crash_capsule();
    if !capsule.valid {
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: crash-capsule generation={} pattern={} closest_prior={} mode={:?} replay(seq={} req={} cmp={} irq={}) window(valid={} syscall={} fd={} op={} device={} state={} path={:?} req={} cmp={}) reprobe(mode={:?} path={:?} stage={} stage_name={} checkpoint={:#x} escalation={} crashes={}) stable_prefix={} unstable_suffix={} divergence_seq={}\n",
        capsule.generation,
        capsule.failure_signature_id,
        capsule.closest_prior_pattern_id,
        capsule.mode,
        capsule.replay.sequence,
        capsule.replay.request_id,
        capsule.replay.completion_id,
        capsule.replay.irq_id,
        capsule.window.valid,
        capsule.window.syscall_id,
        capsule.window.fd,
        capsule.window.request_op,
        capsule.window.device_id,
        capsule.window.completion_state,
        capsule.window.path,
        capsule.window.request_id,
        capsule.window.completion_id,
        capsule.reprobe.mode,
        capsule.reprobe.target_path,
        capsule.reprobe.target_stage,
        stage_name(capsule.reprobe.target_stage),
        capsule.reprobe.target_checkpoint,
        capsule.reprobe.escalation,
        capsule.reprobe.crash_count,
        capsule.stable_prefix_length,
        capsule.unstable_suffix_length,
        capsule.first_divergence_sequence
    ));
    serial::print(format_args!(
        "ngos/x86_64: crash-capsule fault-summary rip={:#x} sym={} vector={} vector_name={} error={:#x} cr2={:#x} first_bad={} last_good={}\n",
        capsule.fault.rip,
        render_symbol(capsule.fault.rip),
        capsule.fault.vector,
        fault_vector_name(capsule.fault.vector),
        capsule.fault.error_code,
        capsule.fault.cr2,
        trace_kind_name(capsule.failure_signature.first_bad_kind),
        trace_kind_name(capsule.failure_signature.last_good_kind)
    ));
    for (index, suspect) in capsule
        .suspects
        .iter()
        .enumerate()
        .filter(|(_, suspect)| suspect.valid)
    {
        serial::print(format_args!(
            "ngos/x86_64: crash-capsule suspect#{} stage={} cpu={} req={} cmp={} irq={} seq={} event={} score={} reason={}\n",
            index + 1,
            stage_name(suspect.stage),
            suspect.cpu_slot,
            suspect.request_id,
            suspect.completion_id,
            suspect.irq_id,
            suspect.event_sequence,
            trace_kind_name(suspect.event_kind),
            suspect.score,
            suspect_reason_name(suspect.reason_code)
        ));
    }
    for violation in capsule
        .watch_tail
        .iter()
        .filter(|entry| entry.sequence != 0)
    {
        serial::print(format_args!(
            "ngos/x86_64: violation seq={} type={} descriptor={} descriptor_kind={} addr={:#x} len={} base={:#x} base_len={} overlap={} flags={:#x} rel_start={} rel_end={} stage={} stage_name={} path={:?} req={} cmp={} cpu_slot={} apic={} addr_sym={}\n",
            violation.sequence,
            violation_kind_name(violation.kind),
            violation.descriptor_id,
            violation.descriptor_kind,
            violation.address,
            violation.length,
            violation.descriptor_address,
            violation.descriptor_length,
            memory_overlap_name(violation.overlap),
            violation.suspicion_flags,
            violation.relative_start,
            violation.relative_end,
            violation.stage,
            stage_name(violation.stage),
            violation.path,
            violation.request_id,
            violation.completion_id,
            violation.cpu_slot,
            violation.apic_id,
            render_symbol(violation.address)
        ));
    }
    for (cpu_slot, entries) in capsule.trace_tail.iter().enumerate() {
        for entry in entries.iter().filter(|entry| entry.sequence != 0) {
            serial::print(format_args!(
                "ngos/x86_64: crash-trace cpu_slot={} seq={} event={} channel={} stage={} stage_name={} a={:#x} b={:#x} c={:#x} d={:#x}\n",
                cpu_slot,
                entry.sequence,
                trace_kind_name(entry.kind),
                trace_channel_name(entry.channel),
                entry.stage,
                stage_name(entry.stage),
                entry.a,
                entry.b,
                entry.c,
                entry.d
            ));
        }
    }
}

#[cfg_attr(test, allow(dead_code))]
fn function_name(function_id: u64) -> &'static str {
    match function_id {
        1 => "claim_resource_syscall",
        2 => "release_claimed_resource_syscall",
        3 => "set_resource_governance_syscall",
        4 => "set_resource_contract_policy_syscall",
        5 => "set_resource_state_syscall",
        6 => "create_contract_syscall",
        7 => "set_resource_issuer_policy_syscall",
        _ => "<unknown-function>",
    }
}

#[cfg_attr(test, allow(dead_code))]
fn checkpoint_name(checkpoint_id: u64) -> &'static str {
    match checkpoint_id {
        1 => "lookup",
        2 => "state-check",
        3 => "policy-check",
        4 => "mutation",
        5 => "copyout",
        6 => "return",
        _ => "<unknown-checkpoint>",
    }
}

#[cfg_attr(test, allow(dead_code))]
fn semantic_step_name(function_id: u64, step_id: u64) -> &'static str {
    match (function_id, step_id) {
        (1, 0) => "claim-resource",
        (1, 1) => "claim-resource/acquire",
        (1, 2) => "claim-resource/queue",
        (2, 0) => "release-claimed-resource",
        (3, 0) => "set-resource-governance",
        (3, 1) => "resource-governance/exclusive-lease",
        (4, 0) => "set-resource-contract-policy",
        (4, 3) => "resource-contract-policy/io",
        (5, 0) => "set-resource-state",
        (5, 1) => "resource-state/suspended",
        (5, 2) => "resource-state/retired",
        (6, 0) => "create-contract",
        (6, 2) => "create-contract/io",
        (6, 4) => "create-contract/display",
        (7, 0) => "set-resource-issuer-policy",
        (7, 1) => "resource-issuer-policy/creator-only",
        _ => "<unknown-step>",
    }
}

fn emit_focused_path_view() {
    serial::write_bytes(b"== focused-path ==\n");
    let capsule = crash_capsule();
    let previous = previous_crash_capsule();
    let trace = capsule.trace_tail;
    let divergence = first_divergence_point(&capsule, &previous);
    let last_good = last_confirmed_good(&capsule);
    let first_bad = first_bad_event(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: focused stable_prefix={} unstable_suffix={} first_bad_seq={} last_good_seq={} divergence_seq={}\n",
        capsule.stable_prefix_length,
        capsule.unstable_suffix_length,
        first_bad.map(|entry| entry.sequence).unwrap_or(0),
        last_good.map(|entry| entry.sequence).unwrap_or(0),
        divergence.map(|(entry, _)| entry.sequence).unwrap_or(0)
    ));
    for (cpu_slot, entries) in trace.iter().enumerate() {
        let mut cpu_header_emitted = false;
        for entry in entries.iter().filter(|entry| entry.sequence != 0) {
            if !trace_belongs_to_window(entry, &capsule.window, &capsule.replay) {
                continue;
            }
            if !cpu_header_emitted {
                serial::print(format_args!("ngos/x86_64: cpu{}:\n", cpu_slot));
                cpu_header_emitted = true;
            }
            serial::print(format_args!(
                "ngos/x86_64: path seq={} cpu={} event={} channel={} stage_name={} req={} cmp={} irq={} result={} reason={} marker={}\n",
                entry.sequence,
                cpu_slot,
                trace_kind_name(entry.kind),
                trace_channel_name(entry.channel),
                stage_name(entry.stage),
                focused_request_id(entry, &capsule.replay),
                focused_completion_id(entry, &capsule.replay),
                focused_irq_id(entry, &capsule.replay),
                focused_result(entry),
                focused_reason(entry),
                focused_marker(entry, last_good, first_bad, divergence)
            ));
        }
    }
}

fn emit_failure_comparison() {
    serial::write_bytes(b"== compare-last-failures ==\n");
    let current = crash_capsule();
    let previous = previous_crash_capsule();
    if !current.valid {
        serial::write_bytes(
            b"ngos/x86_64: compare current=none previous=none result=ok reason=no-crash-capsule\n",
        );
        return;
    }
    if !previous.valid {
        serial::write_bytes(
            b"ngos/x86_64: compare current=present previous=none result=ok reason=first-capsule\n",
        );
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: compare ids req={}=>{} cmp={}=>{} irq={}=>{} stage={}=>{} fault={}=>{} pattern={}=>{}\n",
        previous.replay.request_id,
        current.replay.request_id,
        previous.replay.completion_id,
        current.replay.completion_id,
        previous.replay.irq_id,
        current.replay.irq_id,
        stage_name(previous.fault.stage),
        stage_name(current.fault.stage),
        fault_vector_name(previous.fault.vector),
        fault_vector_name(current.fault.vector),
        previous.failure_signature_id,
        current.failure_signature_id
    ));
    let divergence = first_divergence_point(&current, &previous);
    serial::print(format_args!(
        "ngos/x86_64: compare stable_prefix previous={} current={} same-pattern={} closest-prior={}\n",
        previous.stable_prefix_length,
        current.stable_prefix_length,
        (previous.failure_signature_id == current.failure_signature_id) as u8,
        current.closest_prior_pattern_id
    ));
    if let Some((current_entry, previous_entry)) = divergence {
        serial::print(format_args!(
            "ngos/x86_64: compare divergence current_seq={} current_stage={} current_event={} previous_seq={} previous_stage={} previous_event={} result=fail reason=focused-path-diverged\n",
            current_entry.sequence,
            stage_name(current_entry.stage),
            trace_kind_name(current_entry.kind),
            previous_entry.sequence,
            stage_name(previous_entry.stage),
            trace_kind_name(previous_entry.kind)
        ));
    } else {
        serial::write_bytes(
            b"ngos/x86_64: compare divergence none result=ok reason=focused-path-stable\n",
        );
    }
}

fn emit_failure_explanation() {
    serial::write_bytes(b"== failure-explanation ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: explanation none result=ok reason=no-crash-capsule\n");
        return;
    }
    let (path, score, path_reason) = dominant_failure_path(&capsule);
    let top_suspect = capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid);
    let classification = classify_failure(&capsule);
    let consistency = consistency_score(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: explanation pattern={} similar_to={} path={} similarity={} consistency={}({})\n",
        capsule.failure_signature_id,
        capsule.closest_prior_pattern_id,
        dominant_path_name(path),
        path_reason,
        consistency,
        consistency_band_name(consistency)
    ));
    serial::print(format_args!(
        "ngos/x86_64: explanation break stage={} event={} fault={} path_score={}\n",
        stage_name(
            capsule
                .failure_signature
                .first_bad_stage
                .max(capsule.failure_signature.stage)
        ),
        trace_kind_name(capsule.failure_signature.first_bad_kind),
        fault_vector_name(capsule.failure_signature.fault_vector),
        score
    ));
    serial::print(format_args!(
        "ngos/x86_64: explanation before stage={} event={} stable_prefix={} unstable_suffix={}\n",
        stage_name(capsule.failure_signature.last_good_stage),
        trace_kind_name(capsule.failure_signature.last_good_kind),
        capsule.stable_prefix_length,
        capsule.unstable_suffix_length
    ));
    serial::print(format_args!(
        "ngos/x86_64: explanation suspect reason={} violation={} divergence={} classification={}\n",
        top_suspect
            .map(|suspect| suspect_reason_name(suspect.reason_code))
            .unwrap_or("none"),
        top_suspect
            .map(|suspect| local_violation_for_suspect(suspect, &capsule))
            .unwrap_or("none"),
        trace_kind_name(capsule.failure_signature.divergence_kind),
        classification_name(classification)
    ));
    serial::print(format_args!(
        "ngos/x86_64: explanation disposition={}\n",
        explanation_disposition(&capsule, classification, consistency)
    ));
}

#[allow(dead_code)]
fn emit_top_suspect_compact() {
    let capsule = crash_capsule();
    if let Some(suspect) = capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid)
    {
        let confidence = suspect_confidence(suspect, &capsule);
        serial::print(format_args!(
            "ngos/x86_64: compact suspect stage={} cpu={} req={} cmp={} irq={} confidence={}% explanation={}\n",
            stage_name(suspect.stage),
            suspect.cpu_slot,
            suspect.request_id,
            suspect.completion_id,
            suspect.irq_id,
            confidence,
            suspect_confidence_explanation(suspect, &capsule, confidence)
        ));
    } else {
        serial::write_bytes(
            b"ngos/x86_64: compact suspect none confidence=0% explanation=no-suspect\n",
        );
    }
}

#[allow(dead_code)]
fn emit_failure_classification() {
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(
            b"ngos/x86_64: classification none confidence=0 reason=no-crash-capsule\n",
        );
        return;
    }
    let class = classify_failure(&capsule);
    let consistency = consistency_score(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: classification class={} consistency={} band={} reason={}\n",
        classification_name(class),
        consistency,
        consistency_band_name(consistency),
        classification_reason(&capsule, class)
    ));
}

#[allow(dead_code)]
fn emit_developer_hint() {
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: developer-hint none\n");
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: developer-hint {}\n",
        developer_hint(&capsule)
    ));
}

#[allow(dead_code)]
fn emit_top_patch_target_compact() {
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: compact patch-target none\n");
        return;
    }
    let target = top_patch_targets(&capsule)[0];
    serial::print(format_args!(
        "ngos/x86_64: compact patch-target area={} zone={} confidence={} reason={}\n",
        target.file_area, target.function_zone, target.confidence, target.reason
    ));
}

fn emit_pattern_path_narrowing() {
    serial::write_bytes(b"== pattern-path ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: pattern-path none result=ok reason=no-crash-capsule\n");
        return;
    }
    let (path, score, reason) = dominant_failure_path(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: pattern={} dominant_path={} score={} reason={} stage={} first_bad={} divergence={} fault={}\n",
        capsule.failure_signature_id,
        dominant_path_name(path),
        score,
        reason,
        stage_name(capsule.failure_signature.stage),
        trace_kind_name(capsule.failure_signature.first_bad_kind),
        trace_kind_name(capsule.failure_signature.divergence_kind),
        fault_vector_name(capsule.failure_signature.fault_vector)
    ));
}

fn emit_suspect_evidence_bundles() {
    serial::write_bytes(b"== suspect-evidence ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(
            b"ngos/x86_64: suspect-evidence none result=ok reason=no-crash-capsule\n",
        );
        return;
    }
    for (index, suspect) in capsule
        .suspects
        .iter()
        .enumerate()
        .filter(|(_, suspect)| suspect.valid)
    {
        let confidence = suspect_confidence(*suspect, &capsule);
        let reason = local_semantic_reason_for_suspect(*suspect, &capsule);
        let violation = local_violation_for_suspect(*suspect, &capsule);
        let (before, current, after) = local_event_window_for_suspect(*suspect, &capsule);
        serial::print(format_args!(
            "ngos/x86_64: suspect-bundle rank={} stage={} cpu={} req={} cmp={} irq={} score={} confidence={}% reason={} symbol={} local_reason={} local_violation={} diff_prev={}\n",
            index + 1,
            stage_name(suspect.stage),
            suspect.cpu_slot,
            suspect.request_id,
            suspect.completion_id,
            suspect.irq_id,
            suspect.score,
            confidence,
            suspect_reason_name(suspect.reason_code),
            suspect_symbol_hint(*suspect, &capsule),
            reason,
            violation,
            suspect_difference_from_previous(*suspect, &capsule)
        ));
        serial::print(format_args!(
            "ngos/x86_64: suspect-confidence explanation={}\n",
            suspect_confidence_explanation(*suspect, &capsule, confidence)
        ));
        emit_neighbor_event("before", before);
        emit_neighbor_event("current", current);
        emit_neighbor_event("after", after);
    }
}

fn trace_belongs_to_window(
    entry: &TraceRecord,
    window: &RequestWindowState,
    replay: &ReplayCorrelationIds,
) -> bool {
    if !window.valid {
        return true;
    }
    entry.a == replay.request_id
        || entry.a == replay.completion_id
        || entry.a == replay.irq_id
        || entry.b == replay.request_id
        || entry.b == replay.completion_id
        || entry.b == replay.irq_id
}

fn emit_semantic_reason_bucket(label: &str, reasons: &[(&str, u16)]) {
    serial::print(format_args!("ngos/x86_64: reason-group path={}\n", label));
    for (name, count) in reasons.iter().copied() {
        serial::print(format_args!(
            "ngos/x86_64: reason code={} count={}\n",
            name, count
        ));
    }
}

fn diagnostics_path_name(path: DiagnosticsPath) -> &'static str {
    match path {
        DiagnosticsPath::None => "none",
        DiagnosticsPath::Syscall => "write_syscall",
        DiagnosticsPath::Block => "submit_device_request",
        DiagnosticsPath::Irq => "irq",
        DiagnosticsPath::Completion => "completion",
        DiagnosticsPath::Fault => "fault",
    }
}

fn causal_edge_name(kind: CausalEdgeKind) -> &'static str {
    match kind {
        CausalEdgeKind::Validation => "validation",
        CausalEdgeKind::Submit => "submit",
        CausalEdgeKind::Irq => "irq",
        CausalEdgeKind::Completion => "completion",
        CausalEdgeKind::Fault => "fault",
        CausalEdgeKind::Divergence => "divergence",
    }
}

fn invariant_status_name(status: InvariantStatus) -> &'static str {
    match status {
        InvariantStatus::Missing => "missing",
        InvariantStatus::Verified => "verified",
        InvariantStatus::Violated => "violated",
    }
}

fn dominant_path_for_trace(entry: &TraceRecord, capsule: &CrashCapsule) -> DiagnosticsPath {
    if entry.kind == TraceKind::Fault || entry.channel == TraceChannel::Fault {
        DiagnosticsPath::Fault
    } else if entry.kind == TraceKind::Irq || entry.channel == TraceChannel::Irq {
        DiagnosticsPath::Irq
    } else if entry.stage >= BootTraceStage::UserLaunchReady as u16
        && entry.stage <= BootTraceStage::EnterUserMode as u16
    {
        if capsule.window.path != DiagnosticsPath::None {
            capsule.window.path
        } else {
            DiagnosticsPath::Completion
        }
    } else {
        capsule.failure_signature.path
    }
}

fn semantic_reason_for_trace(entry: &TraceRecord, capsule: &CrashCapsule) -> &'static str {
    let suspect = capsule
        .suspects
        .into_iter()
        .find(|suspect| suspect.valid && suspect.event_sequence == entry.sequence);
    if let Some(suspect) = suspect {
        return local_semantic_reason_for_suspect(suspect, capsule);
    }
    match dominant_path_for_trace(entry, capsule) {
        DiagnosticsPath::Syscall => "write boundary remained on focused path",
        DiagnosticsPath::Block => "submit boundary remained on focused path",
        DiagnosticsPath::Completion => "completion boundary remained on focused path",
        DiagnosticsPath::Irq => "irq boundary remained on focused path",
        DiagnosticsPath::Fault => "fault boundary is on focused path",
        DiagnosticsPath::None => "focused-path divergence boundary",
    }
}

fn dominant_violation_text(capsule: &CrashCapsule) -> &'static str {
    if capsule
        .watch_tail
        .iter()
        .any(|entry| entry.kind == ViolationKind::Guard && entry.sequence != 0)
    {
        "guard-hit"
    } else if capsule
        .watch_tail
        .iter()
        .any(|entry| entry.kind == ViolationKind::Watch && entry.sequence != 0)
    {
        "watch-hit"
    } else {
        "none"
    }
}

fn same_pattern_baseline(capsule: &CrashCapsule) -> Option<CrashCapsule> {
    let history = unsafe { *TRACE_STORAGE.crash_history.get() };
    let mut best = None;
    let mut best_generation = 0u64;
    for entry in history.iter().copied().filter(|entry| {
        entry.valid
            && entry.signature_id == capsule.failure_signature_id
            && entry.generation != capsule.generation
    }) {
        if entry.generation >= best_generation {
            best_generation = entry.generation;
            best = Some(crash_capsule_from_history(entry, capsule));
        }
    }
    best
}

fn crash_capsule_from_history(entry: CrashHistoryEntry, current: &CrashCapsule) -> CrashCapsule {
    let mut capsule = *current;
    capsule.valid = entry.valid;
    capsule.generation = entry.generation;
    capsule.failure_signature_id = entry.signature_id;
    capsule.replay = entry.replay;
    capsule.window = entry.window;
    capsule.fault = entry.fault;
    capsule.stable_prefix_length = entry.stable_prefix_length;
    capsule.unstable_suffix_length = entry.unstable_suffix_length;
    capsule.first_divergence_sequence = entry.first_divergence_sequence;
    let mut tail = [[TraceRecord::EMPTY; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS];
    let mut cpu0 = [TraceRecord::EMPTY; CRASH_TRACE_TAIL];
    for (index, trace) in entry.focused_trace.into_iter().enumerate() {
        if index < cpu0.len() {
            cpu0[index] = trace;
        }
    }
    tail[0] = cpu0;
    capsule.trace_tail = tail;
    capsule
}

fn focused_events<'a>(
    capsule: &'a CrashCapsule,
) -> [Option<&'a TraceRecord>; CRASH_TRACE_TAIL * MAX_TRACE_CPUS] {
    focused_events_from_parts(&capsule.trace_tail, &capsule.window, &capsule.replay)
}

fn focused_events_from_parts<'a>(
    trace_tail: &'a [[TraceRecord; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS],
    window: &RequestWindowState,
    replay: &ReplayCorrelationIds,
) -> [Option<&'a TraceRecord>; CRASH_TRACE_TAIL * MAX_TRACE_CPUS] {
    let mut out: [Option<&'a TraceRecord>; CRASH_TRACE_TAIL * MAX_TRACE_CPUS] =
        [None; CRASH_TRACE_TAIL * MAX_TRACE_CPUS];
    let mut count = 0usize;
    for entries in trace_tail.iter() {
        for entry in entries.iter().filter(|entry| entry.sequence != 0) {
            if trace_belongs_to_window(entry, window, replay) {
                out[count] = Some(entry);
                count += 1;
            }
        }
    }
    let mut i = 0usize;
    while i < count {
        let mut j = i + 1;
        while j < count {
            if out[j].unwrap().sequence < out[i].unwrap().sequence {
                out.swap(i, j);
            }
            j += 1;
        }
        i += 1;
    }
    out
}

fn same_trace_signature(left: &TraceRecord, right: &TraceRecord) -> bool {
    left.kind == right.kind
        && left.channel == right.channel
        && left.stage == right.stage
        && left.a == right.a
        && left.b == right.b
        && left.c == right.c
        && left.d == right.d
}

fn first_divergence_point_from_events<'a>(
    current: &[Option<&'a TraceRecord>; CRASH_TRACE_TAIL * MAX_TRACE_CPUS],
    previous: &[Option<&'a TraceRecord>; CRASH_TRACE_TAIL * MAX_TRACE_CPUS],
) -> Option<(&'a TraceRecord, &'a TraceRecord)> {
    let mut index = 0usize;
    while index < current.len() {
        match (current[index], previous[index]) {
            (Some(current_entry), Some(previous_entry)) => {
                if !same_trace_signature(current_entry, previous_entry) {
                    return Some((current_entry, previous_entry));
                }
            }
            (Some(current_entry), None) => return Some((current_entry, current_entry)),
            _ => break,
        }
        index += 1;
    }
    None
}

fn first_divergence_point<'a>(
    current: &'a CrashCapsule,
    previous: &'a CrashCapsule,
) -> Option<(&'a TraceRecord, &'a TraceRecord)> {
    if !current.valid || !previous.valid {
        return None;
    }
    first_divergence_point_from_events(&focused_events(current), &focused_events(previous))
}

fn last_confirmed_good(capsule: &CrashCapsule) -> Option<&TraceRecord> {
    focused_last_good(&focused_events(capsule))
}

fn first_bad_event(capsule: &CrashCapsule) -> Option<&TraceRecord> {
    focused_first_bad(&focused_events(capsule))
}

fn focused_last_good<'a>(
    events: &[Option<&'a TraceRecord>; CRASH_TRACE_TAIL * MAX_TRACE_CPUS],
) -> Option<&'a TraceRecord> {
    let mut last_good = None;
    for entry in events.iter().flatten() {
        if focused_result(entry) == "ok" {
            last_good = Some(*entry);
        } else {
            break;
        }
    }
    last_good
}

fn focused_first_bad<'a>(
    events: &[Option<&'a TraceRecord>; CRASH_TRACE_TAIL * MAX_TRACE_CPUS],
) -> Option<&'a TraceRecord> {
    for entry in events.iter().flatten() {
        if focused_result(entry) == "fail" {
            return Some(*entry);
        }
    }
    None
}

fn focused_marker(
    entry: &TraceRecord,
    last_good: Option<&TraceRecord>,
    first_bad: Option<&TraceRecord>,
    divergence: Option<(&TraceRecord, &TraceRecord)>,
) -> &'static str {
    if let Some(good) = last_good {
        if good.sequence == entry.sequence {
            return "last-good";
        }
    }
    if let Some(bad) = first_bad {
        if bad.sequence == entry.sequence {
            return "first-bad";
        }
    }
    if let Some((current, _)) = divergence {
        if current.sequence == entry.sequence {
            return "divergence";
        }
    }
    "none"
}

fn focused_request_id(entry: &TraceRecord, replay: &ReplayCorrelationIds) -> u64 {
    if entry.a == replay.request_id || entry.b == replay.request_id {
        replay.request_id
    } else {
        0
    }
}

fn focused_completion_id(entry: &TraceRecord, replay: &ReplayCorrelationIds) -> u64 {
    if entry.a == replay.completion_id || entry.b == replay.completion_id {
        replay.completion_id
    } else {
        0
    }
}

fn focused_irq_id(entry: &TraceRecord, replay: &ReplayCorrelationIds) -> u64 {
    if entry.a == replay.irq_id || entry.b == replay.irq_id {
        replay.irq_id
    } else {
        0
    }
}

fn focused_result(entry: &TraceRecord) -> &'static str {
    match entry.kind {
        TraceKind::Fault => "fail",
        TraceKind::Memory => "fail",
        _ => "ok",
    }
}

fn focused_reason(entry: &TraceRecord) -> &'static str {
    match entry.kind {
        TraceKind::Fault => fault_vector_name(entry.a),
        TraceKind::Memory => "guard-or-watch",
        TraceKind::Irq => "irq-edge",
        TraceKind::Transition => "path-transition",
        TraceKind::Device => "device-event",
        TraceKind::BootStage => "boot-stage",
        TraceKind::UserMarker => "user-marker",
        TraceKind::UserStatus => "user-status",
    }
}

fn suspect_reason_name(code: u16) -> &'static str {
    match code {
        1 => "first-bad-event",
        2 => "cross-run-divergence",
        3 => "guard-violation",
        4 => "watch-violation",
        5 => "terminal-fault",
        6 => "last-good-boundary",
        _ => "unknown",
    }
}

fn fnv_mix(state: u64, value: u64) -> u64 {
    let mixed = state ^ value;
    mixed.wrapping_mul(0x100000001b3)
}

#[allow(dead_code)]
fn build_failure_signature(capsule: &CrashCapsule) -> FailureSignature {
    build_failure_signature_from_parts(
        &capsule.fault,
        &capsule.window,
        &capsule.replay,
        &capsule.watch_tail,
        &capsule.trace_tail,
        &previous_crash_capsule(),
    )
}

fn build_failure_signature_from_parts(
    fault: &FaultRecord,
    window: &RequestWindowState,
    replay: &ReplayCorrelationIds,
    violations: &[ViolationRecord; VIOLATION_TAIL],
    trace_tail: &[[TraceRecord; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS],
    previous: &CrashCapsule,
) -> FailureSignature {
    let focused = focused_events_from_parts(trace_tail, window, replay);
    let last_good = focused_last_good(&focused);
    let first_bad = focused_first_bad(&focused);
    let divergence = first_divergence_point_from_events(&focused, &focused_events(previous));
    let dominant_violation = dominant_violation_kind(violations, window.path);
    let chain_shape = ((replay.request_id != 0) as u16)
        | (((replay.completion_id != 0) as u16) << 1)
        | (((replay.irq_id != 0) as u16) << 2)
        | (((window.fd != 0) as u16) << 3);
    let mut hash = 0xcbf29ce484222325u64;
    hash = fnv_mix(hash, window.path as u64);
    hash = fnv_mix(hash, fault.stage as u64);
    hash = fnv_mix(hash, fault.vector);
    hash = fnv_mix(hash, dominant_violation as u64);
    hash = fnv_mix(hash, last_good.map(|entry| entry.kind as u64).unwrap_or(0));
    hash = fnv_mix(hash, last_good.map(|entry| entry.stage as u64).unwrap_or(0));
    hash = fnv_mix(hash, first_bad.map(|entry| entry.kind as u64).unwrap_or(0));
    hash = fnv_mix(hash, first_bad.map(|entry| entry.stage as u64).unwrap_or(0));
    hash = fnv_mix(
        hash,
        divergence.map(|(entry, _)| entry.kind as u64).unwrap_or(0),
    );
    hash = fnv_mix(
        hash,
        divergence.map(|(entry, _)| entry.stage as u64).unwrap_or(0),
    );
    hash = fnv_mix(hash, chain_shape as u64);
    FailureSignature {
        valid: fault.valid || window.valid,
        id: hash,
        path: window.path,
        stage: fault.stage,
        fault_vector: fault.vector,
        dominant_violation,
        last_good_kind: last_good
            .map(|entry| entry.kind)
            .unwrap_or(TraceKind::BootStage),
        last_good_stage: last_good.map(|entry| entry.stage).unwrap_or(0),
        first_bad_kind: first_bad
            .map(|entry| entry.kind)
            .unwrap_or(TraceKind::BootStage),
        first_bad_stage: first_bad.map(|entry| entry.stage).unwrap_or(0),
        divergence_kind: divergence
            .map(|(entry, _)| entry.kind)
            .unwrap_or(TraceKind::BootStage),
        divergence_stage: divergence.map(|(entry, _)| entry.stage).unwrap_or(0),
        chain_shape,
    }
}

fn compare_failure_patterns_from_parts(
    window: &RequestWindowState,
    replay: &ReplayCorrelationIds,
    signature_id: u64,
    trace_tail: &[[TraceRecord; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS],
    previous: &CrashCapsule,
) -> (u16, u16, u64, u64) {
    let current = focused_events_from_parts(trace_tail, window, replay);
    let previous_events = focused_events(previous);
    let mut stable = 0u16;
    let mut divergence_sequence = 0u64;
    let mut index = 0usize;
    while index < current.len() {
        match (current[index], previous_events[index]) {
            (Some(current_entry), Some(previous_entry))
                if same_trace_signature(current_entry, previous_entry) =>
            {
                stable = stable.saturating_add(1);
            }
            (Some(current_entry), Some(_)) | (Some(current_entry), None) => {
                divergence_sequence = current_entry.sequence;
                break;
            }
            _ => break,
        }
        index += 1;
    }
    let total = current.iter().flatten().count() as u16;
    let unstable = total.saturating_sub(stable);
    let closest_prior = closest_prior_pattern(signature_id);
    (
        stable,
        unstable,
        divergence_sequence,
        closest_prior.signature_id,
    )
}

fn rank_suspects_from_parts(
    fault: &FaultRecord,
    window: &RequestWindowState,
    replay: &ReplayCorrelationIds,
    violations: &[ViolationRecord; VIOLATION_TAIL],
    trace_tail: &[[TraceRecord; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS],
    previous: &CrashCapsule,
) -> [SuspectPoint; SUSPECT_LIMIT] {
    let focused = focused_events_from_parts(trace_tail, window, replay);
    let last_good = focused_last_good(&focused);
    let first_bad = focused_first_bad(&focused);
    let divergence = first_divergence_point_from_events(&focused, &focused_events(previous));
    let mut suspects = [SuspectPoint::EMPTY; SUSPECT_LIMIT];
    if let Some(entry) = first_bad {
        insert_suspect(
            &mut suspects,
            SuspectPoint {
                valid: true,
                score: 100,
                stage: entry.stage,
                cpu_slot: entry.cpu_slot,
                request_id: focused_request_id(entry, replay),
                completion_id: focused_completion_id(entry, replay),
                irq_id: focused_irq_id(entry, replay),
                event_sequence: entry.sequence,
                event_kind: entry.kind,
                reason_code: 1,
            },
        );
    }
    if let Some((entry, _)) = divergence {
        insert_suspect(
            &mut suspects,
            SuspectPoint {
                valid: true,
                score: 90,
                stage: entry.stage,
                cpu_slot: entry.cpu_slot,
                request_id: focused_request_id(entry, replay),
                completion_id: focused_completion_id(entry, replay),
                irq_id: focused_irq_id(entry, replay),
                event_sequence: entry.sequence,
                event_kind: entry.kind,
                reason_code: 2,
            },
        );
    }
    if let Some(violation) = violations
        .iter()
        .filter(|entry| entry.sequence != 0)
        .max_by_key(|entry| entry.sequence)
    {
        insert_suspect(
            &mut suspects,
            SuspectPoint {
                valid: true,
                score: if violation.kind == ViolationKind::Guard {
                    95
                } else {
                    85
                },
                stage: violation.stage,
                cpu_slot: violation.cpu_slot,
                request_id: violation.request_id,
                completion_id: violation.completion_id,
                irq_id: replay.irq_id,
                event_sequence: violation.sequence,
                event_kind: TraceKind::Memory,
                reason_code: if violation.kind == ViolationKind::Guard {
                    3
                } else {
                    4
                },
            },
        );
    }
    if fault.valid {
        insert_suspect(
            &mut suspects,
            SuspectPoint {
                valid: true,
                score: 88,
                stage: fault.stage,
                cpu_slot: fault.cpu_slot,
                request_id: replay.request_id,
                completion_id: replay.completion_id,
                irq_id: replay.irq_id,
                event_sequence: latest_event_sequence(fault.cpu_slot as usize),
                event_kind: TraceKind::Fault,
                reason_code: 5,
            },
        );
    }
    if let Some(entry) = last_good {
        insert_suspect(
            &mut suspects,
            SuspectPoint {
                valid: true,
                score: 70,
                stage: entry.stage,
                cpu_slot: entry.cpu_slot,
                request_id: focused_request_id(entry, replay),
                completion_id: focused_completion_id(entry, replay),
                irq_id: focused_irq_id(entry, replay),
                event_sequence: entry.sequence,
                event_kind: entry.kind,
                reason_code: 6,
            },
        );
    }
    suspects
}

fn summarize_semantic_reasons(
    fault: &FaultRecord,
    window: &RequestWindowState,
    violations: &[ViolationRecord; VIOLATION_TAIL],
    reprobe: ReprobePolicyState,
) -> SemanticReasonAggregate {
    let mut reasons = SemanticReasonAggregate::EMPTY;
    if fault.valid {
        match window.path {
            DiagnosticsPath::Syscall => reasons.write_syscall_reject_fault += 1,
            DiagnosticsPath::Block => reasons.submit_device_request_reject_fault += 1,
            DiagnosticsPath::Completion => {
                reasons.completion_publish_reject_fault += 1;
                reasons.completion_read_reject_fault += 1;
            }
            _ => {}
        }
        reasons.reprobe_escalation_reason_fault += 1;
    }
    for violation in violations.iter().filter(|entry| entry.sequence != 0) {
        match (violation.path, violation.kind) {
            (DiagnosticsPath::Syscall, ViolationKind::Guard) => {
                reasons.write_syscall_reject_guard += 1
            }
            (DiagnosticsPath::Syscall, ViolationKind::Watch) => {
                reasons.write_syscall_reject_watch += 1
            }
            (DiagnosticsPath::Block, ViolationKind::Guard) => {
                reasons.submit_device_request_reject_guard += 1
            }
            (DiagnosticsPath::Block, ViolationKind::Watch) => {
                reasons.submit_device_request_reject_watch += 1
            }
            (DiagnosticsPath::Completion, ViolationKind::Guard) => {
                reasons.completion_publish_reject_guard += 1;
                reasons.completion_read_reject_guard += 1;
            }
            (DiagnosticsPath::Completion, ViolationKind::Watch) => {
                reasons.completion_publish_reject_watch += 1;
                reasons.completion_read_reject_watch += 1;
            }
            _ => {}
        }
        reasons.reprobe_escalation_reason_watch_guard += 1;
    }
    if reprobe.escalation != 0
        && reasons.reprobe_escalation_reason_fault == 0
        && reasons.reprobe_escalation_reason_watch_guard == 0
    {
        reasons.reprobe_escalation_reason_fault = 1;
    }
    reasons
}

fn dominant_violation_kind(
    violations: &[ViolationRecord; VIOLATION_TAIL],
    path: DiagnosticsPath,
) -> ViolationKind {
    let mut guard = 0u16;
    let mut watch = 0u16;
    for violation in violations
        .iter()
        .filter(|entry| entry.sequence != 0 && entry.path == path)
    {
        match violation.kind {
            ViolationKind::Guard => guard += 1,
            ViolationKind::Watch => watch += 1,
        }
    }
    if watch > guard {
        ViolationKind::Watch
    } else {
        ViolationKind::Guard
    }
}

fn insert_suspect(suspects: &mut [SuspectPoint; SUSPECT_LIMIT], candidate: SuspectPoint) {
    if suspects.iter().any(|entry| {
        entry.valid
            && entry.event_sequence == candidate.event_sequence
            && entry.reason_code == candidate.reason_code
    }) {
        return;
    }
    let mut index = 0usize;
    while index < suspects.len() {
        if !suspects[index].valid || candidate.score > suspects[index].score {
            let mut shift = suspects.len() - 1;
            while shift > index {
                suspects[shift] = suspects[shift - 1];
                shift -= 1;
            }
            suspects[index] = candidate;
            return;
        }
        index += 1;
    }
}

fn dominant_failure_path(capsule: &CrashCapsule) -> (DiagnosticsPath, u16, &'static str) {
    let mut best_path = capsule.window.path;
    let mut best_score = path_score(capsule, DiagnosticsPath::Syscall);
    let mut best_reason = dominant_path_reason(capsule, DiagnosticsPath::Syscall);
    for path in [
        DiagnosticsPath::Block,
        DiagnosticsPath::Completion,
        DiagnosticsPath::Fault,
    ] {
        let score = path_score(capsule, path);
        if score > best_score {
            best_score = score;
            best_path = path;
            best_reason = dominant_path_reason(capsule, path);
        }
    }
    (best_path, best_score, best_reason)
}

fn path_score(capsule: &CrashCapsule, path: DiagnosticsPath) -> u16 {
    let mut score = 0u16;
    if capsule.window.path == path {
        score = score.saturating_add(60);
    }
    match path {
        DiagnosticsPath::Syscall => {
            score = score
                .saturating_add(capsule.semantic_reasons.write_syscall_reject_fault)
                .saturating_add(capsule.semantic_reasons.write_syscall_reject_guard)
                .saturating_add(capsule.semantic_reasons.write_syscall_reject_watch);
        }
        DiagnosticsPath::Block => {
            score = score
                .saturating_add(capsule.semantic_reasons.submit_device_request_reject_fault)
                .saturating_add(capsule.semantic_reasons.submit_device_request_reject_guard)
                .saturating_add(capsule.semantic_reasons.submit_device_request_reject_watch);
        }
        DiagnosticsPath::Completion => {
            score = score
                .saturating_add(capsule.semantic_reasons.completion_publish_reject_fault)
                .saturating_add(capsule.semantic_reasons.completion_publish_reject_guard)
                .saturating_add(capsule.semantic_reasons.completion_publish_reject_watch)
                .saturating_add(capsule.semantic_reasons.completion_read_reject_fault)
                .saturating_add(capsule.semantic_reasons.completion_read_reject_guard)
                .saturating_add(capsule.semantic_reasons.completion_read_reject_watch);
        }
        DiagnosticsPath::Fault => {
            if capsule.fault.valid {
                score = score.saturating_add(40);
            }
        }
        _ => {}
    }
    for violation in capsule
        .watch_tail
        .iter()
        .filter(|entry| entry.sequence != 0 && entry.path == path)
    {
        score = score.saturating_add(match violation.kind {
            ViolationKind::Guard => 20,
            ViolationKind::Watch => 12,
        });
    }
    score
}

fn dominant_path_reason(capsule: &CrashCapsule, path: DiagnosticsPath) -> &'static str {
    if capsule.window.path == path {
        return "active-window";
    }
    if capsule.watch_tail.iter().any(|entry| {
        entry.sequence != 0 && entry.path == path && entry.kind == ViolationKind::Guard
    }) {
        return "guard-dominant";
    }
    if capsule.watch_tail.iter().any(|entry| {
        entry.sequence != 0 && entry.path == path && entry.kind == ViolationKind::Watch
    }) {
        return "watch-dominant";
    }
    if capsule.fault.valid && path == DiagnosticsPath::Fault {
        return "terminal-fault";
    }
    "semantic-reason-count"
}

fn dominant_path_name(path: DiagnosticsPath) -> &'static str {
    match path {
        DiagnosticsPath::Syscall => "write_syscall",
        DiagnosticsPath::Block => "submit_device_request",
        DiagnosticsPath::Completion => "completion_publish_or_read",
        DiagnosticsPath::Fault => "fault_or_trap",
        DiagnosticsPath::Irq => "irq",
        DiagnosticsPath::None => "none",
    }
}

fn local_semantic_reason_for_suspect(
    suspect: SuspectPoint,
    capsule: &CrashCapsule,
) -> &'static str {
    match dominant_path_for_suspect(suspect, capsule) {
        DiagnosticsPath::Syscall if capsule.semantic_reasons.write_syscall_reject_guard != 0 => {
            "write_syscall_reject_guard"
        }
        DiagnosticsPath::Syscall if capsule.semantic_reasons.write_syscall_reject_watch != 0 => {
            "write_syscall_reject_watch"
        }
        DiagnosticsPath::Syscall if capsule.semantic_reasons.write_syscall_reject_fault != 0 => {
            "write_syscall_reject_fault"
        }
        DiagnosticsPath::Block
            if capsule.semantic_reasons.submit_device_request_reject_guard != 0 =>
        {
            "submit_device_request_reject_guard"
        }
        DiagnosticsPath::Block
            if capsule.semantic_reasons.submit_device_request_reject_watch != 0 =>
        {
            "submit_device_request_reject_watch"
        }
        DiagnosticsPath::Block
            if capsule.semantic_reasons.submit_device_request_reject_fault != 0 =>
        {
            "submit_device_request_reject_fault"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_publish_reject_guard != 0 =>
        {
            "completion_publish_reject_guard"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_read_reject_guard != 0 =>
        {
            "completion_read_reject_guard"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_publish_reject_watch != 0 =>
        {
            "completion_publish_reject_watch"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_read_reject_watch != 0 =>
        {
            "completion_read_reject_watch"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_publish_reject_fault != 0 =>
        {
            "completion_publish_reject_fault"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_read_reject_fault != 0 =>
        {
            "completion_read_reject_fault"
        }
        DiagnosticsPath::Fault => fault_vector_name(capsule.fault.vector),
        _ => "none",
    }
}

fn dominant_path_for_suspect(suspect: SuspectPoint, capsule: &CrashCapsule) -> DiagnosticsPath {
    if let Some(violation) = capsule
        .watch_tail
        .iter()
        .find(|entry| entry.sequence == suspect.event_sequence)
    {
        return violation.path;
    }
    if suspect.event_kind == TraceKind::Fault {
        return DiagnosticsPath::Fault;
    }
    capsule.window.path
}

fn local_violation_for_suspect(suspect: SuspectPoint, capsule: &CrashCapsule) -> &'static str {
    for violation in capsule
        .watch_tail
        .iter()
        .filter(|entry| entry.sequence != 0)
    {
        if violation.cpu_slot == suspect.cpu_slot
            && (violation.request_id == suspect.request_id
                || violation.completion_id == suspect.completion_id)
        {
            return violation_kind_name(violation.kind);
        }
    }
    "none"
}

fn local_event_window_for_suspect(
    suspect: SuspectPoint,
    capsule: &CrashCapsule,
) -> (
    Option<&TraceRecord>,
    Option<&TraceRecord>,
    Option<&TraceRecord>,
) {
    let events = focused_events(capsule);
    let mut previous = None;
    let mut current = None;
    let mut next = None;
    let mut index = 0usize;
    while index < events.len() {
        if let Some(entry) = events[index] {
            if entry.sequence == suspect.event_sequence {
                current = Some(entry);
                if index > 0 {
                    previous = events[index - 1];
                }
                if index + 1 < events.len() {
                    next = events[index + 1];
                }
                break;
            }
        }
        index += 1;
    }
    (previous, current, next)
}

fn emit_neighbor_event(label: &str, event: Option<&TraceRecord>) {
    if let Some(event) = event {
        serial::print(format_args!(
            "ngos/x86_64: suspect-event slot={} seq={} cpu={} stage={} event={} channel={} req={} cmp={} irq={} reason={}\n",
            label,
            event.sequence,
            event.cpu_slot,
            stage_name(event.stage),
            trace_kind_name(event.kind),
            trace_channel_name(event.channel),
            event.a,
            event.b,
            event.c,
            focused_reason(event)
        ));
    } else {
        serial::print(format_args!(
            "ngos/x86_64: suspect-event slot={} seq=0 result=ok reason=none\n",
            label
        ));
    }
}

fn suspect_symbol_hint(suspect: SuspectPoint, capsule: &CrashCapsule) -> SymbolRender {
    if suspect.event_kind == TraceKind::Fault {
        return render_symbol(capsule.fault.rip);
    }
    if let Some(violation) = capsule
        .watch_tail
        .iter()
        .find(|entry| entry.sequence == suspect.event_sequence)
    {
        return render_symbol(violation.address);
    }
    render_symbol(0)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureClassification {
    LogicError,
    SecurityViolation,
    MemoryCorruption,
    TimingRace,
    DriverContractBreak,
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatchBugClass {
    MissingValidation,
    WrongRightsCheck,
    LabelDowngrade,
    ProvenanceChainBreak,
    IntegrityMismatch,
    BufferLifetimeBug,
    RequestCompletionMismatch,
    IrqOrderingRace,
    CompletionPublishRace,
    MemoryCorruption,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PatchTargetSuggestion {
    rank: u8,
    file_area: &'static str,
    stage: u16,
    function_zone: &'static str,
    confidence: u16,
    reason: &'static str,
}

fn consistency_score(capsule: &CrashCapsule) -> u16 {
    let history = unsafe { *TRACE_STORAGE.crash_history.get() };
    let mut matches = 0u16;
    let mut stable_acc = 0u16;
    let mut varying_stage = 0u16;
    let mut varying_ids = 0u16;
    for entry in history
        .iter()
        .filter(|entry| entry.valid && entry.signature_id == capsule.failure_signature_id)
    {
        matches += 1;
        stable_acc = stable_acc.saturating_add(entry.stable_prefix_length.min(100));
        if entry.fault.stage != capsule.fault.stage {
            varying_stage += 1;
        }
        if entry.replay.request_id != capsule.replay.request_id
            || entry.replay.completion_id != capsule.replay.completion_id
            || entry.replay.irq_id != capsule.replay.irq_id
        {
            varying_ids += 1;
        }
    }
    if matches <= 1 {
        return 60;
    }
    let avg_stable = stable_acc / matches;
    let instability_penalty = varying_stage.saturating_mul(10) + varying_ids.saturating_mul(6);
    avg_stable
        .saturating_add(40)
        .saturating_sub(instability_penalty)
        .min(100)
}

fn consistency_band_name(score: u16) -> &'static str {
    match score {
        85..=100 => "stable",
        60..=84 => "mostly-stable",
        _ => "unstable",
    }
}

fn classify_failure(capsule: &CrashCapsule) -> FailureClassification {
    if capsule.watch_tail.iter().any(|entry| {
        entry.sequence != 0 && matches!(entry.kind, ViolationKind::Guard | ViolationKind::Watch)
    }) {
        return FailureClassification::MemoryCorruption;
    }
    if dominant_path_name(dominant_failure_path(capsule).0) == "completion_publish_or_read"
        && capsule.failure_signature.first_bad_kind == TraceKind::Device
    {
        return FailureClassification::DriverContractBreak;
    }
    if consistency_score(capsule) < 60
        || capsule.unstable_suffix_length > capsule.stable_prefix_length
    {
        return FailureClassification::TimingRace;
    }
    if capsule.window.path == DiagnosticsPath::Block
        || capsule.window.path == DiagnosticsPath::Completion
    {
        return FailureClassification::DriverContractBreak;
    }
    if capsule.fault.valid {
        return FailureClassification::LogicError;
    }
    FailureClassification::Unknown
}

fn classification_name(class: FailureClassification) -> &'static str {
    match class {
        FailureClassification::LogicError => "LOGIC_ERROR",
        FailureClassification::SecurityViolation => "SECURITY_VIOLATION",
        FailureClassification::MemoryCorruption => "MEMORY_CORRUPTION",
        FailureClassification::TimingRace => "TIMING_RACE",
        FailureClassification::DriverContractBreak => "DRIVER_CONTRACT_BREAK",
        FailureClassification::Unknown => "UNKNOWN",
    }
}

fn classification_reason(capsule: &CrashCapsule, class: FailureClassification) -> &'static str {
    match class {
        FailureClassification::MemoryCorruption => "guard-or-watch-hit-present",
        FailureClassification::TimingRace => "cross-run-instability",
        FailureClassification::DriverContractBreak => "device-or-completion-path-dominant",
        FailureClassification::LogicError => fault_vector_name(capsule.fault.vector),
        FailureClassification::SecurityViolation => "semantic-security-reason",
        FailureClassification::Unknown => "insufficient-signal",
    }
}

fn suspect_confidence(suspect: SuspectPoint, capsule: &CrashCapsule) -> u16 {
    let mut confidence = suspect.score.min(100);
    confidence = confidence.saturating_add(consistency_score(capsule) / 5);
    if local_violation_for_suspect(suspect, capsule) != "none" {
        confidence = confidence.saturating_add(10);
    }
    if local_semantic_reason_for_suspect(suspect, capsule) != "none" {
        confidence = confidence.saturating_add(8);
    }
    if suspect.reason_code == 1 || suspect.reason_code == 2 {
        confidence = confidence.saturating_add(8);
    }
    confidence.min(100)
}

fn suspect_confidence_explanation(
    suspect: SuspectPoint,
    capsule: &CrashCapsule,
    confidence: u16,
) -> &'static str {
    let has_violation = local_violation_for_suspect(suspect, capsule) != "none";
    let consistent = consistency_score(capsule) >= 85;
    match (
        suspect.reason_code,
        has_violation,
        consistent,
        confidence >= 90,
    ) {
        (1, true, true, true) => "first-bad + violation + stable across runs",
        (2, true, _, _) => "first divergence + violation",
        (1, false, true, _) => "first-bad + stable across runs",
        (5, true, _, _) => "terminal fault aligned with violation",
        (5, false, _, _) => "terminal fault near failure boundary",
        (_, _, false, _) => "suspect aligns but run consistency is lower",
        _ => "suspect aligns with local evidence",
    }
}

fn explanation_disposition(
    _capsule: &CrashCapsule,
    class: FailureClassification,
    consistency: u16,
) -> &'static str {
    match class {
        FailureClassification::MemoryCorruption => "probable memory / guard failure",
        FailureClassification::TimingRace => "timing-sensitive bug",
        _ if consistency >= 85 => "deterministic bug",
        _ => "mixed-signal failure",
    }
}

fn developer_hint(capsule: &CrashCapsule) -> &'static str {
    let top = capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid);
    let (path, _, _) = dominant_failure_path(capsule);
    let class = classify_failure(capsule);
    match (
        path,
        class,
        top.map(|suspect| suspect.reason_code).unwrap_or(0),
    ) {
        (DiagnosticsPath::Syscall, _, 1 | 2) => "probabil bug in write_syscall validation path",
        (DiagnosticsPath::Block, _, 1 | 2) => "probabil bug in submit_device_request path",
        (DiagnosticsPath::Completion, FailureClassification::DriverContractBreak, _) => {
            "likely mismatch in completion publish/read contract"
        }
        (_, FailureClassification::MemoryCorruption, _) => {
            "buffer corruption or invalid access before terminal fault"
        }
        (_, FailureClassification::TimingRace, _) => {
            "likely race between irq/completion or cross-core timing edge"
        }
        (_, FailureClassification::LogicError, _) => {
            "investigate first-bad event and stage-local invariants"
        }
        _ => "investigate top suspect and stable prefix boundary first",
    }
}

fn likely_patch_bug_class(capsule: &CrashCapsule) -> PatchBugClass {
    let class = classify_failure(capsule);
    match class {
        FailureClassification::MemoryCorruption => {
            if consistency_score(capsule) < 60 {
                PatchBugClass::BufferLifetimeBug
            } else {
                PatchBugClass::MemoryCorruption
            }
        }
        FailureClassification::TimingRace => {
            if capsule.window.path == DiagnosticsPath::Completion {
                PatchBugClass::CompletionPublishRace
            } else {
                PatchBugClass::IrqOrderingRace
            }
        }
        FailureClassification::DriverContractBreak => {
            if capsule.window.path == DiagnosticsPath::Completion {
                PatchBugClass::RequestCompletionMismatch
            } else {
                PatchBugClass::MissingValidation
            }
        }
        FailureClassification::LogicError => PatchBugClass::MissingValidation,
        FailureClassification::SecurityViolation => PatchBugClass::WrongRightsCheck,
        FailureClassification::Unknown => PatchBugClass::Unknown,
    }
}

fn patch_bug_class_name(class: PatchBugClass) -> &'static str {
    match class {
        PatchBugClass::MissingValidation => "MISSING_VALIDATION",
        PatchBugClass::WrongRightsCheck => "WRONG_RIGHTS_CHECK",
        PatchBugClass::LabelDowngrade => "LABEL_DOWNGRADE",
        PatchBugClass::ProvenanceChainBreak => "PROVENANCE_CHAIN_BREAK",
        PatchBugClass::IntegrityMismatch => "INTEGRITY_MISMATCH",
        PatchBugClass::BufferLifetimeBug => "BUFFER_LIFETIME_BUG",
        PatchBugClass::RequestCompletionMismatch => "REQUEST_COMPLETION_MISMATCH",
        PatchBugClass::IrqOrderingRace => "IRQ_ORDERING_RACE",
        PatchBugClass::CompletionPublishRace => "COMPLETION_PUBLISH_RACE",
        PatchBugClass::MemoryCorruption => "MEMORY_CORRUPTION",
        PatchBugClass::Unknown => "UNKNOWN",
    }
}

fn bug_class_confidence(capsule: &CrashCapsule, class: PatchBugClass) -> u16 {
    let mut confidence = consistency_score(capsule);
    if capsule.watch_tail.iter().any(|entry| entry.sequence != 0) {
        confidence = confidence.saturating_add(10);
    }
    if matches!(
        class,
        PatchBugClass::BufferLifetimeBug
            | PatchBugClass::MemoryCorruption
            | PatchBugClass::RequestCompletionMismatch
    ) {
        confidence = confidence.saturating_add(8);
    }
    confidence.min(100)
}

fn bug_class_reason(capsule: &CrashCapsule, class: PatchBugClass) -> &'static str {
    match class {
        PatchBugClass::BufferLifetimeBug => "guard/watch present with unstable execution",
        PatchBugClass::MemoryCorruption => "guard/watch present on stable pattern",
        PatchBugClass::RequestCompletionMismatch => "completion or driver contract dominates",
        PatchBugClass::IrqOrderingRace => "cross-run instability before stable completion",
        PatchBugClass::CompletionPublishRace => "completion path unstable across runs",
        PatchBugClass::MissingValidation => "first-bad aligns with validation boundary",
        PatchBugClass::WrongRightsCheck => "semantic security reasons dominate",
        PatchBugClass::LabelDowngrade => "completion label path dominates",
        PatchBugClass::ProvenanceChainBreak => "provenance path mismatch inferred",
        PatchBugClass::IntegrityMismatch => "integrity check mismatch inferred",
        PatchBugClass::Unknown => classification_reason(capsule, classify_failure(capsule)),
    }
}

fn bug_class_history_relation(capsule: &CrashCapsule, class: PatchBugClass) -> &'static str {
    let previous = previous_crash_capsule();
    if !previous.valid {
        return "first-suggestion";
    }
    let previous_class = likely_patch_bug_class(&previous);
    if previous_class == class {
        if consistency_score(capsule) > consistency_score(&previous) {
            "stronger-than-previous"
        } else {
            "same-as-previous"
        }
    } else {
        "changed-due-to-new-divergence"
    }
}

fn top_patch_targets(capsule: &CrashCapsule) -> [PatchTargetSuggestion; 3] {
    let (path, _, _) = dominant_failure_path(capsule);
    let bug_class = likely_patch_bug_class(capsule);
    let top = capsule.suspects;
    let base_conf = bug_class_confidence(capsule, bug_class);
    [
        PatchTargetSuggestion {
            rank: 1,
            file_area: patch_target_file_area(path),
            stage: top[0].stage.max(capsule.failure_signature.stage),
            function_zone: patch_target_zone(path, bug_class, 1),
            confidence: base_conf,
            reason: patch_target_primary_reason(capsule, path, 1),
        },
        PatchTargetSuggestion {
            rank: 2,
            file_area: patch_target_file_area_secondary(path),
            stage: top[1].stage.max(capsule.failure_signature.first_bad_stage),
            function_zone: patch_target_zone(path, bug_class, 2),
            confidence: base_conf.saturating_sub(10),
            reason: patch_target_primary_reason(capsule, path, 2),
        },
        PatchTargetSuggestion {
            rank: 3,
            file_area: patch_target_file_area_tertiary(path),
            stage: top[2].stage.max(capsule.failure_signature.last_good_stage),
            function_zone: patch_target_zone(path, bug_class, 3),
            confidence: base_conf.saturating_sub(20),
            reason: patch_target_primary_reason(capsule, path, 3),
        },
    ]
}

fn patch_target_file_area(path: DiagnosticsPath) -> &'static str {
    match path {
        DiagnosticsPath::Syscall => "boot-x86_64/src/user_syscall.rs",
        DiagnosticsPath::Block => "boot-x86_64/src/virtio_blk_boot.rs",
        DiagnosticsPath::Completion => "boot-x86_64/src/virtio_blk_boot.rs",
        DiagnosticsPath::Fault => "boot-x86_64/src/user_syscall.rs",
        DiagnosticsPath::Irq => "boot-x86_64/src/virtio_blk_boot.rs",
        DiagnosticsPath::None => "boot-x86_64/src/diagnostics.rs",
    }
}

fn patch_target_file_area_secondary(path: DiagnosticsPath) -> &'static str {
    match path {
        DiagnosticsPath::Syscall => "boot-x86_64/src/virtio_blk_boot.rs",
        DiagnosticsPath::Block => "boot-x86_64/src/user_syscall.rs",
        DiagnosticsPath::Completion => "boot-x86_64/src/user_syscall.rs",
        DiagnosticsPath::Fault => "boot-x86_64/src/virtio_blk_boot.rs",
        DiagnosticsPath::Irq => "boot-x86_64/src/user_syscall.rs",
        DiagnosticsPath::None => "boot-x86_64/src/diagnostics.rs",
    }
}

fn patch_target_file_area_tertiary(path: DiagnosticsPath) -> &'static str {
    match path {
        DiagnosticsPath::Completion | DiagnosticsPath::Irq => "completion publish/read path",
        DiagnosticsPath::Block => "completion publish/read path",
        DiagnosticsPath::Syscall => "completion publish/read path",
        DiagnosticsPath::Fault => "boot-x86_64/src/diagnostics.rs",
        DiagnosticsPath::None => "boot-x86_64/src/diagnostics.rs",
    }
}

fn patch_target_zone(path: DiagnosticsPath, bug_class: PatchBugClass, rank: u8) -> &'static str {
    match (path, bug_class, rank) {
        (DiagnosticsPath::Syscall, _, 1) => "write_syscall validation block",
        (DiagnosticsPath::Block, _, 1) => "submit_device_request queue/build path",
        (DiagnosticsPath::Completion, PatchBugClass::CompletionPublishRace, 1) => {
            "completion publish ordering block"
        }
        (DiagnosticsPath::Completion, _, 1) => "completion validation/preservation block",
        (_, PatchBugClass::BufferLifetimeBug, 2) => "buffer lifetime / request buffer handoff",
        (_, PatchBugClass::IrqOrderingRace, 2) => "IRQ/completion ordering block",
        (_, _, 2) => "defensive revalidation boundary",
        (_, _, _) => "correlation/preservation boundary",
    }
}

fn patch_target_primary_reason(
    capsule: &CrashCapsule,
    path: DiagnosticsPath,
    rank: u8,
) -> &'static str {
    match (path, rank) {
        (DiagnosticsPath::Syscall, 1) => "first-bad before submit + stable path",
        (DiagnosticsPath::Block, 1) => "submit-stage divergence + repeated block semantic reason",
        (DiagnosticsPath::Completion, 1) => "completion path dominates + mismatch/race evidence",
        (_, 2) if capsule.watch_tail.iter().any(|entry| entry.sequence != 0) => {
            "guard/watch evidence links to propagation path"
        }
        (_, 2) => "causal chain shows propagation through boundary",
        _ => "retain correlation and preserve invariant across boundary",
    }
}

fn patch_target_because(capsule: &CrashCapsule, target: PatchTargetSuggestion) -> &'static str {
    let _ = target;
    if let Some(suspect) = capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid)
    {
        return suspect_confidence_explanation(
            suspect,
            capsule,
            suspect_confidence(suspect, capsule),
        );
    }
    "no-suspect"
}

fn dominant_patch_semantic_reason(
    capsule: &CrashCapsule,
    target: PatchTargetSuggestion,
) -> &'static str {
    let path = if target.rank == 1 {
        dominant_failure_path(capsule).0
    } else if target.rank == 2 {
        patch_target_path_secondary(capsule)
    } else {
        DiagnosticsPath::Completion
    };
    match path {
        DiagnosticsPath::Syscall if capsule.semantic_reasons.write_syscall_reject_guard != 0 => {
            "write_syscall_reject_guard"
        }
        DiagnosticsPath::Syscall if capsule.semantic_reasons.write_syscall_reject_watch != 0 => {
            "write_syscall_reject_watch"
        }
        DiagnosticsPath::Syscall if capsule.semantic_reasons.write_syscall_reject_fault != 0 => {
            "write_syscall_reject_fault"
        }
        DiagnosticsPath::Block
            if capsule.semantic_reasons.submit_device_request_reject_guard != 0 =>
        {
            "submit_device_request_reject_guard"
        }
        DiagnosticsPath::Block
            if capsule.semantic_reasons.submit_device_request_reject_watch != 0 =>
        {
            "submit_device_request_reject_watch"
        }
        DiagnosticsPath::Block
            if capsule.semantic_reasons.submit_device_request_reject_fault != 0 =>
        {
            "submit_device_request_reject_fault"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_publish_reject_guard != 0 =>
        {
            "completion_publish_reject_guard"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_publish_reject_watch != 0 =>
        {
            "completion_publish_reject_watch"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_read_reject_guard != 0 =>
        {
            "completion_read_reject_guard"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_read_reject_watch != 0 =>
        {
            "completion_read_reject_watch"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_publish_reject_fault != 0 =>
        {
            "completion_publish_reject_fault"
        }
        DiagnosticsPath::Completion
            if capsule.semantic_reasons.completion_read_reject_fault != 0 =>
        {
            "completion_read_reject_fault"
        }
        _ => "none",
    }
}

fn patch_target_path_secondary(capsule: &CrashCapsule) -> DiagnosticsPath {
    match dominant_failure_path(capsule).0 {
        DiagnosticsPath::Syscall => DiagnosticsPath::Block,
        DiagnosticsPath::Block => DiagnosticsPath::Completion,
        DiagnosticsPath::Completion => DiagnosticsPath::Block,
        other => other,
    }
}

fn dominant_patch_violation(capsule: &CrashCapsule, target: PatchTargetSuggestion) -> &'static str {
    let target_path = if target.rank == 1 {
        dominant_failure_path(capsule).0
    } else {
        patch_target_path_secondary(capsule)
    };
    if let Some(violation) = capsule
        .watch_tail
        .iter()
        .find(|entry| entry.sequence != 0 && entry.path == target_path)
    {
        return violation_kind_name(violation.kind);
    }
    "none"
}

fn causal_step_for_target(capsule: &CrashCapsule, target: PatchTargetSuggestion) -> &'static str {
    match target.rank {
        1 => probable_root_cause(capsule),
        2 => probable_propagation(capsule),
        _ => probable_bad_boundary(capsule),
    }
}

fn patch_target_stability(capsule: &CrashCapsule, _target: PatchTargetSuggestion) -> &'static str {
    consistency_band_name(consistency_score(capsule))
}

fn missing_invariant_hints(
    capsule: &CrashCapsule,
) -> [(&'static str, &'static str, &'static str); 3] {
    let path = dominant_failure_path(capsule).0;
    match (path, likely_patch_bug_class(capsule)) {
        (DiagnosticsPath::Syscall, _) => [
            (
                "request validation not enforced early enough",
                "write_syscall validation block",
                "request validity + operation semantics",
            ),
            (
                "request/completion correlation id likely not preserved",
                "write_syscall -> submit boundary",
                "request_id/completion_id",
            ),
            (
                "defensive submit precondition missing",
                "submit_device_request entry",
                "validated request state",
            ),
        ],
        (
            DiagnosticsPath::Block,
            PatchBugClass::BufferLifetimeBug | PatchBugClass::MemoryCorruption,
        ) => [
            (
                "buffer lifetime invariant not enforced before queue submission",
                "submit_device_request queue/build path",
                "buffer address/length/liveness",
            ),
            (
                "request shape not defensively revalidated",
                "submit_device_request pre-queue block",
                "request fields + payload length",
            ),
            (
                "completion correlation not preserved across queue boundary",
                "queue/build -> completion path",
                "request_id/completion_id/irq_id",
            ),
        ],
        (DiagnosticsPath::Completion, _) => [
            (
                "effective completion state not preserved across publish path",
                "completion validation/preservation block",
                "completion state + ids",
            ),
            (
                "request/completion correlation id likely lost before publish/read",
                "completion publish/read path",
                "request_id/completion_id",
            ),
            (
                "ordering/barrier invariant likely weak",
                "completion publish ordering block",
                "publish order / visibility",
            ),
        ],
        _ => [
            (
                "stage-local invariant likely missing",
                "first-bad stage boundary",
                "stage transition assumptions",
            ),
            (
                "faulting boundary not defensively revalidated",
                "fault-adjacent path",
                "current window + request state",
            ),
            (
                "correlation invariant likely lost before terminal fault",
                "focused-path divergence boundary",
                "request/completion/irq ids",
            ),
        ],
    }
}

fn check_first_list(capsule: &CrashCapsule) -> [(&'static str, &'static str, &'static str); 3] {
    let targets = top_patch_targets(capsule);
    [
        (
            targets[0].function_zone,
            "verify preconditions and data shape exactly at first-bad boundary",
            targets[0].reason,
        ),
        (
            targets[1].function_zone,
            "verify propagation state and correlation ids are still preserved",
            targets[1].reason,
        ),
        (
            targets[2].function_zone,
            "verify defensive revalidation or publish ordering assumptions",
            targets[2].reason,
        ),
    ]
}

fn do_not_touch_first_list(capsule: &CrashCapsule) -> [(&'static str, &'static str); 3] {
    let path = dominant_failure_path(capsule).0;
    match path {
        DiagnosticsPath::Syscall => [
            (
                "IRQ path",
                "stable prefix shows failure begins before notify/irq",
            ),
            (
                "global diagnostics",
                "crash pattern points to request validation, not instrumentation",
            ),
            (
                "completion helpers",
                "divergence starts before completion publish/read",
            ),
        ],
        DiagnosticsPath::Block => [
            (
                "global diagnostics",
                "pattern localizes to submit/build path",
            ),
            (
                "top-level boot flow",
                "stable prefix reaches block submit boundary consistently",
            ),
            (
                "completion read path",
                "failure begins before final completion observation",
            ),
        ],
        DiagnosticsPath::Completion => [
            (
                "write_syscall validation",
                "stable prefix reaches completion path first",
            ),
            (
                "global diagnostics",
                "evidence bundles point to completion preservation/ordering",
            ),
            (
                "boot stage transitions",
                "divergence starts after submit, not in boot bringup",
            ),
        ],
        _ => [
            (
                "global diagnostics",
                "pattern localizes to runtime path, not reporting",
            ),
            (
                "unrelated stages",
                "focused path narrows failure to dominant path",
            ),
            (
                "broad refactors",
                "same-pattern stability suggests local fix first",
            ),
        ],
    }
}

fn patch_shape_suggestion(capsule: &CrashCapsule) -> (&'static str, u16, &'static str) {
    let bug = likely_patch_bug_class(capsule);
    match bug {
        PatchBugClass::MissingValidation => (
            "add defensive revalidation before submit",
            bug_class_confidence(capsule, bug),
            "stable pattern + first-bad near validation boundary",
        ),
        PatchBugClass::BufferLifetimeBug => (
            "preserve buffer lifetime across path boundary",
            bug_class_confidence(capsule, bug),
            "guard/watch evidence + unstable execution around submit",
        ),
        PatchBugClass::MemoryCorruption => (
            "strengthen memory lifetime invariant and reject invalid transition earlier",
            bug_class_confidence(capsule, bug),
            "stable memory violation close to first-bad event",
        ),
        PatchBugClass::RequestCompletionMismatch => (
            "retain correlation id and strengthen completion invariant",
            bug_class_confidence(capsule, bug),
            "completion path dominates + repeated mismatch evidence",
        ),
        PatchBugClass::CompletionPublishRace | PatchBugClass::IrqOrderingRace => (
            "fix ordering / publish barrier",
            bug_class_confidence(capsule, bug),
            "same-pattern instability + divergence after stable prefix",
        ),
        _ => (
            "move validation earlier",
            bug_class_confidence(capsule, bug),
            bug_class_reason(capsule, bug),
        ),
    }
}

fn probable_root_cause(capsule: &CrashCapsule) -> &'static str {
    match likely_patch_bug_class(capsule) {
        PatchBugClass::BufferLifetimeBug => "buffer lifetime violation before submit",
        PatchBugClass::MemoryCorruption => "memory corruption at watched/guarded boundary",
        PatchBugClass::RequestCompletionMismatch => "request/completion contract mismatch",
        PatchBugClass::CompletionPublishRace | PatchBugClass::IrqOrderingRace => {
            "ordering race around publish/irq"
        }
        PatchBugClass::MissingValidation => "missing validation before boundary crossing",
        PatchBugClass::WrongRightsCheck => "wrong or missing rights check",
        _ => "root cause remains local to first-bad boundary",
    }
}

fn probable_symptom(capsule: &CrashCapsule) -> &'static str {
    match classify_failure(capsule) {
        FailureClassification::MemoryCorruption => "terminal fault after corrupted state use",
        FailureClassification::TimingRace => "late divergence surfaced as unstable failure",
        FailureClassification::DriverContractBreak => {
            "contract break observed at publish/read edge"
        }
        FailureClassification::SecurityViolation => {
            "rejection or fault after invalid security state"
        }
        FailureClassification::LogicError => "terminal failure at semantic boundary",
        FailureClassification::Unknown => "symptom remains aligned with terminal fault",
    }
}

fn probable_propagation(capsule: &CrashCapsule) -> &'static str {
    match dominant_failure_path(capsule).0 {
        DiagnosticsPath::Syscall => {
            "invalid request propagated from syscall validation into submit path"
        }
        DiagnosticsPath::Block => "invalid request state propagated through queue/build path",
        DiagnosticsPath::Completion => "bad completion state propagated into publish/read path",
        DiagnosticsPath::Fault => {
            "fault surfaced after prior runtime corruption or invalid state use"
        }
        _ => "propagation remained within focused path",
    }
}

fn probable_bad_boundary(capsule: &CrashCapsule) -> &'static str {
    match dominant_failure_path(capsule).0 {
        DiagnosticsPath::Syscall => "write_syscall -> submit boundary",
        DiagnosticsPath::Block => "submit -> device queue boundary",
        DiagnosticsPath::Completion => "completion publish -> completion read boundary",
        DiagnosticsPath::Fault => "terminal fault boundary",
        _ => "focused-path divergence boundary",
    }
}

fn first_concrete_inspection_step(capsule: &CrashCapsule) -> &'static str {
    top_patch_targets(capsule)[0].function_zone
}

fn root_cause_confidence(suspect: SuspectPoint, capsule: &CrashCapsule) -> u16 {
    let mut confidence = suspect_confidence(suspect, capsule);
    if local_violation_for_suspect(suspect, capsule) != "none" {
        confidence = confidence.saturating_add(6);
    }
    if suspect.reason_code == 1 || suspect.reason_code == 2 {
        confidence = confidence.saturating_add(4);
    }
    confidence.min(100)
}

fn top_root_cause_confidence(capsule: &CrashCapsule) -> u16 {
    capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid)
        .map(|suspect| root_cause_confidence(suspect, capsule))
        .unwrap_or(consistency_score(capsule) / 2)
}

fn root_cause_reason(capsule: &CrashCapsule) -> &'static str {
    if let Some(suspect) = capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid)
    {
        if local_violation_for_suspect(suspect, capsule) != "none" {
            return "first-bad aligns with local violation";
        }
        if consistency_score(capsule) >= 85 {
            return "same first-bad boundary remains stable across runs";
        }
        if capsule.first_divergence_sequence != 0 {
            return "root cause inferred from earliest divergence after stable prefix";
        }
    }
    "root cause inferred from focused-path and dominant semantic path"
}

fn failure_story_request_state(capsule: &CrashCapsule) -> &'static str {
    if local_violation_for_top_suspect(capsule) != "none" {
        return "request became invalid near watched/guarded boundary";
    }
    if capsule.stable_prefix_length != 0 {
        return "request remained valid through stable prefix";
    }
    match dominant_failure_path(capsule).0 {
        DiagnosticsPath::Syscall => "request failed during syscall-side validation or handoff",
        DiagnosticsPath::Block => "request failed during submit/build transition",
        DiagnosticsPath::Completion => "request survived submit and failed during completion flow",
        DiagnosticsPath::Fault => "request path ended in terminal fault state",
        _ => "request state changed inside focused path",
    }
}

fn local_violation_for_top_suspect(capsule: &CrashCapsule) -> &'static str {
    capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid)
        .map(|suspect| local_violation_for_suspect(suspect, capsule))
        .unwrap_or("none")
}

fn local_semantic_reason_for_top_suspect(capsule: &CrashCapsule) -> &'static str {
    capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid)
        .map(|suspect| local_semantic_reason_for_suspect(suspect, capsule))
        .unwrap_or("none")
}

fn render_trace_evidence(entry: Option<&TraceRecord>) -> &'static str {
    match entry {
        Some(record) => trace_evidence_name(*record),
        None => "none",
    }
}

fn render_divergence_evidence(entry: Option<(&TraceRecord, &TraceRecord)>) -> &'static str {
    match entry {
        Some((current, _previous)) => trace_evidence_name(*current),
        None => "none",
    }
}

fn trace_evidence_name(record: TraceRecord) -> &'static str {
    match (record.stage, record.kind) {
        (stage, TraceKind::Fault) => stage_name(stage),
        (stage, TraceKind::Transition) => stage_name(stage),
        (stage, TraceKind::Device) => stage_name(stage),
        (stage, TraceKind::Memory) => stage_name(stage),
        (stage, TraceKind::Irq) => stage_name(stage),
        (stage, TraceKind::BootStage) => stage_name(stage),
        (stage, TraceKind::UserMarker) => stage_name(stage),
        (stage, TraceKind::UserStatus) => stage_name(stage),
    }
}

fn suspect_difference_from_previous(suspect: SuspectPoint, capsule: &CrashCapsule) -> &'static str {
    let previous = previous_crash_capsule();
    if !previous.valid {
        return "no-previous-run";
    }
    if previous.suspects.iter().any(|entry| {
        entry.valid && entry.reason_code == suspect.reason_code && entry.stage == suspect.stage
    }) {
        return "same-shape";
    }
    if previous.failure_signature_id == capsule.failure_signature_id {
        return "same-pattern-different-point";
    }
    "new-vs-previous"
}

#[allow(dead_code)]
fn emit_symbol_table() {
    for landmark in symbol_landmarks() {
        serial::print(format_args!(
            "ngos/x86_64: diag symbol {}={:#x}\n",
            landmark.name, landmark.base
        ));
    }
}

#[allow(dead_code)]
fn emit_boot_locator_ring() {
    let early = boot_locator::early_snapshot();
    for record in early.records.iter().filter(|record| record.sequence != 0) {
        serial::print(format_args!(
            "ngos/x86_64: diag early-locator seq={} stage={:?} kind={:?} severity={:?} checkpoint={:#x} name={} {}={:#x} {}={:#x}\n",
            record.sequence,
            record.stage,
            record.kind,
            record.severity,
            record.checkpoint,
            boot_locator::checkpoint_name(record.stage, record.checkpoint),
            render_payload_label(record.payload0_label),
            record.payload0,
            render_payload_label(record.payload1_label),
            record.payload1
        ));
    }
    let ring = boot_locator::ring_snapshot();
    for record in ring.iter().filter(|record| record.sequence != 0) {
        serial::print(format_args!(
            "ngos/x86_64: diag locator seq={} stage={:?} kind={:?} severity={:?} checkpoint={:#x} name={} {}={:#x} {}={:#x}\n",
            record.sequence,
            record.stage,
            record.kind,
            record.severity,
            record.checkpoint,
            boot_locator::checkpoint_name(record.stage, record.checkpoint),
            render_payload_label(record.payload0_label),
            record.payload0,
            render_payload_label(record.payload1_label),
            record.payload1
        ));
    }
}

#[allow(dead_code)]
fn emit_boot_locator_recent(limit: usize) {
    let early = boot_locator::early_recent(limit);
    for record in early.iter().filter(|record| record.sequence != 0) {
        serial::print(format_args!(
            "ngos/x86_64: diag early-locator-recent seq={} stage={:?} kind={:?} severity={:?} checkpoint={:#x} name={} {}={:#x} {}={:#x}\n",
            record.sequence,
            record.stage,
            record.kind,
            record.severity,
            record.checkpoint,
            boot_locator::checkpoint_name(record.stage, record.checkpoint),
            render_payload_label(record.payload0_label),
            record.payload0,
            render_payload_label(record.payload1_label),
            record.payload1
        ));
    }
    let ring = boot_locator::recent(limit);
    for record in ring.iter().filter(|record| record.sequence != 0) {
        serial::print(format_args!(
            "ngos/x86_64: diag locator-recent seq={} stage={:?} kind={:?} severity={:?} checkpoint={:#x} name={} {}={:#x} {}={:#x}\n",
            record.sequence,
            record.stage,
            record.kind,
            record.severity,
            record.checkpoint,
            boot_locator::checkpoint_name(record.stage, record.checkpoint),
            render_payload_label(record.payload0_label),
            record.payload0,
            render_payload_label(record.payload1_label),
            record.payload1
        ));
    }
}

#[allow(dead_code)]
fn render_payload_label(label: BootPayloadLabel) -> &'static str {
    match label {
        BootPayloadLabel::Address => "addr",
        BootPayloadLabel::Length => "len",
        BootPayloadLabel::Count => "count",
        BootPayloadLabel::Rip => "rip",
        BootPayloadLabel::Value => "value",
        BootPayloadLabel::Status => "status",
        BootPayloadLabel::None => "none",
    }
}

fn trace_kind_name(kind: TraceKind) -> &'static str {
    match kind {
        TraceKind::BootStage => "boot-stage",
        TraceKind::UserMarker => "user-marker",
        TraceKind::UserStatus => "user-status",
        TraceKind::Fault => "fault",
        TraceKind::Irq => "irq",
        TraceKind::Memory => "memory",
        TraceKind::Device => "device",
        TraceKind::Transition => "transition",
    }
}

fn trace_channel_name(channel: TraceChannel) -> &'static str {
    match channel {
        TraceChannel::Boot => "boot",
        TraceChannel::User => "user",
        TraceChannel::Fault => "fault",
        TraceChannel::Irq => "irq",
        TraceChannel::Memory => "memory",
        TraceChannel::Device => "device",
        TraceChannel::Transition => "transition",
    }
}

fn violation_kind_name(kind: ViolationKind) -> &'static str {
    match kind {
        ViolationKind::Guard => "guard-hit",
        ViolationKind::Watch => "watch-hit",
    }
}

fn memory_overlap_name(class: MemoryOverlapClass) -> &'static str {
    match class {
        MemoryOverlapClass::None => "none",
        MemoryOverlapClass::Exact => "exact",
        MemoryOverlapClass::Interior => "interior",
        MemoryOverlapClass::Prefix => "prefix",
        MemoryOverlapClass::Suffix => "suffix",
        MemoryOverlapClass::Span => "span",
        MemoryOverlapClass::LeftRedZone => "left-red-zone",
        MemoryOverlapClass::RightRedZone => "right-red-zone",
    }
}

#[cfg_attr(test, allow(dead_code))]
fn memory_lineage_kind_name(kind: MemoryLineageKind) -> &'static str {
    match kind {
        MemoryLineageKind::Snapshot => "snapshot",
        MemoryLineageKind::Write => "write",
        MemoryLineageKind::Copy => "copy",
        MemoryLineageKind::Zero => "zero",
        MemoryLineageKind::Dma => "dma",
        MemoryLineageKind::Free => "free",
    }
}

fn chronoscope_node_kind_name(kind: ChronoscopeNodeKind) -> &'static str {
    match kind {
        ChronoscopeNodeKind::Observation => "observation",
        ChronoscopeNodeKind::Interpretation => "interpretation",
        ChronoscopeNodeKind::Constraint => "constraint",
        ChronoscopeNodeKind::Outcome => "outcome",
        ChronoscopeNodeKind::Boundary => "boundary",
    }
}

fn chronoscope_edge_kind_name(kind: ChronoscopeEdgeKind) -> &'static str {
    match kind {
        ChronoscopeEdgeKind::Caused => "caused",
        ChronoscopeEdgeKind::ObservedBefore => "observed-before",
        ChronoscopeEdgeKind::LeadsTo => "leads-to",
        ChronoscopeEdgeKind::Violates => "violates",
        ChronoscopeEdgeKind::PreventableAt => "preventable-at",
        ChronoscopeEdgeKind::DivergedFrom => "diverged-from",
        ChronoscopeEdgeKind::Explains => "explains",
        ChronoscopeEdgeKind::Supports => "supports",
        ChronoscopeEdgeKind::CompetesWith => "competes-with",
    }
}

fn stage_name(stage: u16) -> &'static str {
    match stage {
        x if x == BootTraceStage::Stage0 as u16 => "stage0",
        x if x == BootTraceStage::EarlyKernelMain as u16 => "early-kernel-main",
        x if x == BootTraceStage::PhysAllocReady as u16 => "phys-alloc-ready",
        x if x == BootTraceStage::PagingReady as u16 => "paging-ready",
        x if x == BootTraceStage::TrapsReady as u16 => "traps-ready",
        x if x == BootTraceStage::SyscallReady as u16 => "syscall-ready",
        x if x == BootTraceStage::UserLaunchReady as u16 => "user-launch-ready",
        x if x == BootTraceStage::EnterUserMode as u16 => "enter-user-mode",
        x if x == BootTraceStage::SmpTopologyReady as u16 => "smp-topology-ready",
        x if x == BootTraceStage::SmpDispatchReady as u16 => "smp-dispatch-ready",
        x if x == BootTraceStage::SecondaryCpuOnline as u16 => "secondary-cpu-online",
        x if x == BootTraceStage::DeviceBringup as u16 => "device-bringup",
        _ => "<unknown-stage>",
    }
}

fn fault_vector_name(vector: u64) -> &'static str {
    match vector {
        6 => "invalid-opcode",
        8 => "double-fault",
        10 => "invalid-tss",
        11 => "segment-not-present",
        12 => "stack-segment-fault",
        13 => "general-protection",
        14 => "page-fault",
        17 => "alignment-check",
        21 => "control-protection",
        _ => "<unknown-vector>",
    }
}

fn render_symbol(address: u64) -> SymbolRender {
    SymbolRender(resolve_address(address))
}

struct SymbolRender(Option<ResolvedSymbol>);

impl core::fmt::Display for SymbolRender {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            Some(symbol) => write!(f, "{}+{:#x}", symbol.name, symbol.offset),
            None => write!(f, "<unknown>"),
        }
    }
}

fn symbol_landmarks() -> [SymbolLandmark; 8] {
    #[cfg(target_os = "none")]
    {
        [
            SymbolLandmark {
                name: "_start",
                base: _start as *const () as usize as u64,
            },
            SymbolLandmark {
                name: "x86_64_boot_stage0",
                base: x86_64_boot_stage0 as *const () as usize as u64,
            },
            SymbolLandmark {
                name: "early_kernel_main",
                base: crate::early_kernel_main as *const () as usize as u64,
            },
            SymbolLandmark {
                name: "gdt::bring_up",
                base: crate::gdt::bring_up as *const () as usize as u64,
            },
            SymbolLandmark {
                name: "traps::bring_up",
                base: crate::traps::bring_up as *const () as usize as u64,
            },
            SymbolLandmark {
                name: "traps::x86_64_exception_dispatch",
                base: x86_64_exception_dispatch as *const () as usize as u64,
            },
            SymbolLandmark {
                name: "smp::x86_64_ap_long_mode_entry",
                base: crate::smp::x86_64_ap_long_mode_entry as *const () as usize as u64,
            },
            SymbolLandmark {
                name: "smp::x86_64_secondary_cpu_main",
                base: crate::smp::x86_64_secondary_cpu_main as *const () as usize as u64,
            },
        ]
    }
    #[cfg(not(target_os = "none"))]
    {
        [SymbolLandmark {
            name: "host",
            base: 0,
        }; 8]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronoscope_query::parse_chronoscope_query;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn test_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        match LOCK.get_or_init(|| Mutex::new(())).lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[test]
    fn trace_ring_records_and_wraps() {
        let _guard = test_lock();
        reset();
        for index in 0..(TRACE_CAPACITY as u64 + 4) {
            record_boot_stage(BootTraceStage::Stage0, Some(index), index);
        }
        let trace = snapshot_trace();
        let cpu0_nonzero = trace[0].iter().filter(|entry| entry.sequence != 0).count();
        assert!(cpu0_nonzero != 0);
        let last_detail = trace[0]
            .iter()
            .filter(|entry| entry.sequence != 0)
            .map(|entry| entry.b)
            .max()
            .unwrap();
        assert_eq!(last_detail, TRACE_CAPACITY as u64 + 3);
    }

    #[test]
    fn fault_record_captures_register_state() {
        let _guard = test_lock();
        reset();
        let frame = ExceptionFrame {
            rax: 1,
            rbx: 2,
            rcx: 3,
            rdx: 4,
            rsi: 5,
            rdi: 6,
            rbp: 7,
            r8: 8,
            r9: 9,
            r10: 10,
            r11: 11,
            r12: 12,
            r13: 13,
            r14: 14,
            r15: 15,
            vector: 14,
            error_code: 5,
            rip: 0x401000,
            cs: 0x33,
            rflags: 0x202,
        };
        record_fault(&frame, Some(44), Some(0xdead));
        let snapshot = last_fault();
        assert!(snapshot.valid);
        assert_eq!(snapshot.rip, 0x401000);
        assert_eq!(snapshot.cr2, 0);
        assert_eq!(snapshot.r15, 15);
    }

    #[test]
    fn classify_memory_overlap_reports_red_zone_and_span() {
        let (class, flags, rel_start, rel_end) = classify_memory_overlap(0x1000, 64, 16, 0x0ff8, 8);
        assert_eq!(class, MemoryOverlapClass::LeftRedZone);
        assert_ne!(flags & MEMORY_SUSPECT_UNDERRUN, 0);
        assert!(rel_start < 0);
        assert!(rel_end <= 0);

        let (class, flags, _, _) = classify_memory_overlap(0x1000, 64, 16, 0x0ff0, 96);
        assert_eq!(class, MemoryOverlapClass::Span);
        assert_ne!(flags & MEMORY_SUSPECT_WIDE_SPAN, 0);
    }

    #[test]
    fn guard_and_watch_violations_capture_overlap_metadata() {
        let _guard = test_lock();
        reset();
        let guard_id = guard_register(
            GuardKind::RequestBuffer,
            DiagnosticsPath::Block,
            0x2000,
            64,
            16,
            7,
            11,
        );
        watch_register(
            WatchKind::Write,
            DiagnosticsPath::Completion,
            0x3000,
            32,
            7,
            12,
        );
        assert!(!guard_check(0x1ff8, 8));
        watch_touch(WatchKind::Write, 0x3000, 32);

        let violations = unsafe { *TRACE_STORAGE.violations.get() };
        let guard = violations
            .iter()
            .filter(|entry| {
                entry.sequence != 0
                    && entry.kind == ViolationKind::Guard
                    && entry.descriptor_kind == GuardKind::RequestBuffer as u16
            })
            .max_by_key(|entry| entry.sequence)
            .copied()
            .unwrap();
        assert_eq!(guard.descriptor_id, guard_id);
        assert_eq!(guard.overlap, MemoryOverlapClass::LeftRedZone);
        assert_ne!(guard.suspicion_flags & MEMORY_SUSPECT_UNDERRUN, 0);

        let watch = violations
            .iter()
            .find(|entry| entry.sequence != 0 && entry.kind == ViolationKind::Watch)
            .copied()
            .unwrap();
        assert_eq!(watch.overlap, MemoryOverlapClass::Exact);
        assert_ne!(watch.suspicion_flags & MEMORY_SUSPECT_EXACT, 0);
    }

    #[test]
    fn export_bundle_and_smp_timeline_are_deterministic() {
        let _guard = test_lock();
        let trace_tail = {
            let mut tail = [[TraceRecord::EMPTY; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS];
            tail[0][0] = TraceRecord {
                sequence: 10,
                uptime_us: 1,
                apic_id: 1,
                cpu_slot: 0,
                kind: TraceKind::BootStage,
                channel: TraceChannel::Boot,
                stage: BootTraceStage::SmpTopologyReady as u16,
                a: 41,
                b: 0,
                c: 0,
                d: 0,
            };
            tail[0][1] = TraceRecord {
                sequence: 11,
                uptime_us: 2,
                apic_id: 1,
                cpu_slot: 0,
                kind: TraceKind::Transition,
                channel: TraceChannel::Transition,
                stage: BootTraceStage::SecondaryCpuOnline as u16,
                a: 41,
                b: 99,
                c: 0,
                d: 0,
            };
            tail[1][0] = TraceRecord {
                sequence: 20,
                uptime_us: 3,
                apic_id: 2,
                cpu_slot: 1,
                kind: TraceKind::Transition,
                channel: TraceChannel::Transition,
                stage: BootTraceStage::SecondaryCpuOnline as u16,
                a: 41,
                b: 99,
                c: 77,
                d: 0,
            };
            tail
        };
        let capsule = CrashCapsule {
            valid: true,
            generation: 3,
            mode: DiagnosticsMode::CrashFollowup,
            failure_signature: FailureSignature::EMPTY,
            failure_signature_id: 77,
            closest_prior_pattern_id: 0,
            stable_prefix_length: 2,
            unstable_suffix_length: 1,
            first_divergence_sequence: 11,
            semantic_reasons: SemanticReasonAggregate::EMPTY,
            suspects: [SuspectPoint::EMPTY; SUSPECT_LIMIT],
            replay: ReplayCorrelationIds::EMPTY,
            fault: FaultRecord::EMPTY,
            reprobe: ReprobePolicyState {
                mode: DiagnosticsMode::CrashFollowup,
                target_path: DiagnosticsPath::Irq,
                target_stage: BootTraceStage::SecondaryCpuOnline as u16,
                target_checkpoint: 0x44,
                escalation: 2,
                crash_count: 5,
            },
            window: RequestWindowState::EMPTY,
            watch_tail: [ViolationRecord::EMPTY; VIOLATION_TAIL],
            trace_tail,
        };
        let timeline = build_smp_timeline_from_trace(&capsule);
        assert!(timeline[0].valid);
        assert_eq!(timeline[0].first_sequence, 10);
        assert_eq!(timeline[0].last_sequence, 11);
        assert_eq!(timeline[0].event_count, 2);
        let timeline_again = build_smp_timeline_from_trace(&capsule);
        assert_eq!(timeline, timeline_again);
        assert!(timeline[1].valid);
        assert_eq!(timeline[1].apic_id, 2);

        let bundle = build_export_bundle(&capsule);
        let bundle_again = build_export_bundle(&capsule);
        assert!(bundle.valid);
        assert_eq!(bundle, bundle_again);
        assert_eq!(bundle.generation, 3);
        assert_eq!(bundle.failure_signature_id, 77);
        assert_eq!(bundle.smp[0].last_sequence, 11);
        assert!(bundle.smp_divergence.valid || bundle.smp[1].valid);
        assert_eq!(bundle.reprobe.crash_count, 5);
    }

    #[test]
    fn failure_history_export_keeps_path_and_correlation_ids() {
        let _guard = test_lock();
        reset();
        unsafe {
            (*TRACE_STORAGE.crash_history.get())[0] = CrashHistoryEntry {
                valid: true,
                generation: 9,
                signature_id: 1234,
                replay: ReplayCorrelationIds {
                    sequence: 55,
                    request_id: 4,
                    completion_id: 8,
                    irq_id: 12,
                },
                window: RequestWindowState {
                    valid: true,
                    syscall_id: 1,
                    fd: 2,
                    request_op: 3,
                    device_id: 4,
                    completion_state: 5,
                    path: DiagnosticsPath::Completion,
                    request_id: 4,
                    completion_id: 8,
                },
                fault: FaultRecord {
                    valid: true,
                    stage: BootTraceStage::EnterUserMode as u16,
                    ..FaultRecord::EMPTY
                },
                stable_prefix_length: 6,
                unstable_suffix_length: 2,
                first_divergence_sequence: 51,
                first_bad_sequence: 52,
                last_good_sequence: 50,
                focused_trace: [TraceRecord::EMPTY; FOCUSED_TRACE_HISTORY],
            };
            *TRACE_STORAGE.crash_capsule.get() = CrashCapsule {
                valid: true,
                generation: 10,
                mode: DiagnosticsMode::CrashFollowup,
                failure_signature: FailureSignature::EMPTY,
                failure_signature_id: 1234,
                closest_prior_pattern_id: 0,
                stable_prefix_length: 6,
                unstable_suffix_length: 2,
                first_divergence_sequence: 51,
                semantic_reasons: SemanticReasonAggregate::EMPTY,
                suspects: [SuspectPoint::EMPTY; SUSPECT_LIMIT],
                replay: ReplayCorrelationIds {
                    sequence: 55,
                    request_id: 4,
                    completion_id: 8,
                    irq_id: 12,
                },
                fault: FaultRecord {
                    valid: true,
                    stage: BootTraceStage::EnterUserMode as u16,
                    ..FaultRecord::EMPTY
                },
                reprobe: ReprobePolicyState::EMPTY,
                window: RequestWindowState {
                    valid: true,
                    syscall_id: 1,
                    fd: 2,
                    request_op: 3,
                    device_id: 4,
                    completion_state: 5,
                    path: DiagnosticsPath::Completion,
                    request_id: 4,
                    completion_id: 8,
                },
                watch_tail: [ViolationRecord::EMPTY; VIOLATION_TAIL],
                trace_tail: [[TraceRecord::EMPTY; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS],
            };
        }
        let export = build_failure_history_export();
        assert!(export[0].valid);
        assert_eq!(export[0].signature_id, 1234);
        assert_eq!(export[0].request_id, 4);
        assert_eq!(export[0].completion_id, 8);
        assert_eq!(export[0].irq_id, 12);
        assert_eq!(export[0].path, DiagnosticsPath::Completion);
    }

    #[test]
    fn pattern_export_orders_top_entries_by_frequency() {
        let _guard = test_lock();
        reset();
        unsafe {
            let patterns = &mut *TRACE_STORAGE.pattern_history.get();
            patterns[0] = FailurePatternSummary {
                valid: true,
                signature_id: 10,
                frequency: 2,
                last_generation: 4,
                last_seen_sequence: 40,
                path: DiagnosticsPath::Block,
                most_common_stage: BootTraceStage::DeviceBringup as u16,
                most_common_first_bad_kind: TraceKind::Device,
            };
            patterns[1] = FailurePatternSummary {
                valid: true,
                signature_id: 20,
                frequency: 7,
                last_generation: 8,
                last_seen_sequence: 70,
                path: DiagnosticsPath::Completion,
                most_common_stage: BootTraceStage::EnterUserMode as u16,
                most_common_first_bad_kind: TraceKind::Memory,
            };
        }
        let export = build_pattern_export();
        assert!(export[0].valid);
        assert_eq!(export[0].signature_id, 20);
        assert_eq!(export[0].rank, 1);
        assert_eq!(export[1].signature_id, 10);
    }

    #[test]
    fn smp_divergence_summary_detects_cross_core_path_split() {
        let mut smp = [SmpTimelineCpuSummary::EMPTY; MAX_TRACE_CPUS];
        smp[0] = SmpTimelineCpuSummary {
            valid: true,
            cpu_slot: 0,
            apic_id: 1,
            first_sequence: 10,
            last_sequence: 20,
            event_count: 4,
            first_stage: BootTraceStage::SmpTopologyReady as u16,
            last_stage: BootTraceStage::SecondaryCpuOnline as u16,
            request_id: 9,
            completion_id: 0,
            irq_id: 0,
            dominant_path: DiagnosticsPath::Block,
            divergence_suspected: true,
        };
        smp[1] = SmpTimelineCpuSummary {
            valid: true,
            cpu_slot: 1,
            apic_id: 2,
            first_sequence: 11,
            last_sequence: 31,
            event_count: 3,
            first_stage: BootTraceStage::SmpTopologyReady as u16,
            last_stage: BootTraceStage::EnterUserMode as u16,
            request_id: 9,
            completion_id: 0,
            irq_id: 0,
            dominant_path: DiagnosticsPath::Completion,
            divergence_suspected: true,
        };
        let divergence = build_smp_divergence_summary(&smp);
        assert!(divergence.valid);
        assert_eq!(divergence.cpu_a, 0);
        assert_eq!(divergence.cpu_b, 1);
        assert_eq!(divergence.request_id, 9);
        assert_eq!(divergence.path_a, DiagnosticsPath::Block);
        assert_eq!(divergence.path_b, DiagnosticsPath::Completion);
    }

    #[test]
    fn chronoscope_snapshot_builds_nodes_and_edges_from_capsule() {
        let mut suspects = [SuspectPoint::EMPTY; SUSPECT_LIMIT];
        suspects[0] = SuspectPoint {
            valid: true,
            score: 91,
            stage: BootTraceStage::EnterUserMode as u16,
            cpu_slot: 0,
            request_id: 7,
            completion_id: 8,
            irq_id: 9,
            event_sequence: 11,
            event_kind: TraceKind::Memory,
            reason_code: 3,
        };
        let mut watch_tail = [ViolationRecord::EMPTY; VIOLATION_TAIL];
        watch_tail[0] = ViolationRecord {
            sequence: 11,
            kind: ViolationKind::Guard,
            descriptor_id: 1,
            descriptor_kind: GuardKind::RequestBuffer as u16,
            address: 0x1000,
            length: 8,
            descriptor_address: 0x1008,
            descriptor_length: 32,
            overlap: MemoryOverlapClass::LeftRedZone,
            suspicion_flags: MEMORY_SUSPECT_UNDERRUN,
            relative_start: -8,
            relative_end: -24,
            stage: BootTraceStage::EnterUserMode as u16,
            path: DiagnosticsPath::Completion,
            request_id: 7,
            completion_id: 8,
            cpu_slot: 0,
            apic_id: 1,
        };
        let mut trace_tail = [[TraceRecord::EMPTY; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS];
        trace_tail[0][0] = TraceRecord {
            sequence: 10,
            uptime_us: 1,
            apic_id: 1,
            cpu_slot: 0,
            kind: TraceKind::Device,
            channel: TraceChannel::Device,
            stage: BootTraceStage::EnterUserMode as u16,
            a: 7,
            b: 8,
            c: 9,
            d: 0,
        };
        trace_tail[0][1] = TraceRecord {
            sequence: 11,
            uptime_us: 2,
            apic_id: 1,
            cpu_slot: 0,
            kind: TraceKind::Memory,
            channel: TraceChannel::Memory,
            stage: BootTraceStage::EnterUserMode as u16,
            a: 7,
            b: 8,
            c: 9,
            d: 0,
        };
        let capsule = CrashCapsule {
            valid: true,
            generation: 1,
            mode: DiagnosticsMode::CrashFollowup,
            failure_signature: FailureSignature {
                valid: true,
                id: 99,
                path: DiagnosticsPath::Completion,
                stage: BootTraceStage::EnterUserMode as u16,
                fault_vector: 14,
                dominant_violation: ViolationKind::Guard,
                last_good_kind: TraceKind::Device,
                last_good_stage: BootTraceStage::EnterUserMode as u16,
                first_bad_kind: TraceKind::Memory,
                first_bad_stage: BootTraceStage::EnterUserMode as u16,
                divergence_kind: TraceKind::Memory,
                divergence_stage: BootTraceStage::EnterUserMode as u16,
                chain_shape: 1,
            },
            failure_signature_id: 99,
            closest_prior_pattern_id: 0,
            stable_prefix_length: 1,
            unstable_suffix_length: 1,
            first_divergence_sequence: 11,
            semantic_reasons: SemanticReasonAggregate::EMPTY,
            suspects,
            replay: ReplayCorrelationIds {
                sequence: 11,
                request_id: 7,
                completion_id: 8,
                irq_id: 9,
            },
            fault: FaultRecord {
                valid: true,
                cpu_slot: 0,
                stage: BootTraceStage::EnterUserMode as u16,
                vector: 14,
                ..FaultRecord::EMPTY
            },
            reprobe: ReprobePolicyState::EMPTY,
            window: RequestWindowState {
                valid: true,
                syscall_id: 1,
                fd: 1,
                request_op: 1,
                device_id: 1,
                completion_state: 1,
                path: DiagnosticsPath::Completion,
                request_id: 7,
                completion_id: 8,
            },
            watch_tail,
            trace_tail,
        };
        let snapshot = build_chronoscope_bundle(&capsule);
        assert!(snapshot.valid);
        assert_eq!(snapshot.failure_signature_id, 99);
        assert_eq!(snapshot.top_suspect_confidence, 100);
        assert!(
            snapshot
                .nodes
                .iter()
                .any(|node| node.valid && node.kind == ChronoscopeNodeKind::Interpretation)
        );
        assert!(
            snapshot
                .edges
                .iter()
                .any(|edge| edge.valid && edge.kind == ChronoscopeEdgeKind::Supports)
        );
        assert!(
            snapshot
                .edges
                .iter()
                .any(|edge| edge.valid && edge.kind == ChronoscopeEdgeKind::LeadsTo)
        );
        assert_ne!(
            snapshot
                .nodes
                .iter()
                .find(|node| node.valid)
                .unwrap()
                .stable_id,
            0
        );
        assert_ne!(snapshot.primary_fault_node(), 0);
        assert_ne!(snapshot.dominant_suspect_chain().len, 0);
        assert!(snapshot.earliest_preventable_boundary().is_some());
        assert!(snapshot.build_explain_plan().valid);
    }

    #[test]
    fn chronoscope_diff_tracks_new_and_common_nodes_by_stable_id() {
        let mut left = ChronoscopeBundle::EMPTY;
        left.valid = true;
        left.nodes[0] = ChronoscopeNode {
            valid: true,
            node_id: 1,
            stable_id: 11,
            kind: ChronoscopeNodeKind::Observation,
            ..ChronoscopeNode::EMPTY
        };
        let mut right = left.clone();
        right.nodes[1] = ChronoscopeNode {
            valid: true,
            node_id: 2,
            stable_id: 22,
            kind: ChronoscopeNodeKind::Interpretation,
            ..ChronoscopeNode::EMPTY
        };
        let diff = right.diff_against(&left);
        assert_eq!(diff.summary.common_nodes, 1);
        assert_eq!(diff.summary.new_nodes, 1);
        assert_eq!(diff.first_divergence_stable_id, 22);
    }

    #[test]
    fn chronoscope_supporting_nodes_returns_graph_predecessors() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.generation = 1;
        bundle.failure_signature_id = 1;
        bundle.top_suspect_confidence = 90;
        bundle.dominant_suspect_node_id = 2;
        bundle.strongest_chain = [2, 3, 0, 0, 0, 0, 0, 0];
        bundle.nodes[0] = ChronoscopeNode {
            valid: true,
            node_id: 1,
            stable_id: 101,
            kind: ChronoscopeNodeKind::Observation,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[1] = ChronoscopeNode {
            valid: true,
            node_id: 2,
            stable_id: 102,
            kind: ChronoscopeNodeKind::Interpretation,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[2] = ChronoscopeNode {
            valid: true,
            node_id: 3,
            stable_id: 103,
            kind: ChronoscopeNodeKind::Outcome,
            ..ChronoscopeNode::EMPTY
        };
        bundle.edges[0] = ChronoscopeEdge {
            valid: true,
            src_node_id: 1,
            dst_node_id: 2,
            kind: ChronoscopeEdgeKind::Supports,
            weight: 80,
        };
        bundle.edges[1] = ChronoscopeEdge {
            valid: true,
            src_node_id: 2,
            dst_node_id: 3,
            kind: ChronoscopeEdgeKind::LeadsTo,
            weight: 100,
        };
        let supporting = bundle.supporting_nodes(2);
        assert_eq!(supporting.len, 1);
        assert_eq!(supporting.nodes[0], 1);
        let explain = bundle.build_explain_plan();
        assert_eq!(explain.primary_cause, 2);
        assert_eq!(explain.fault_node, 3);
    }

    #[test]
    fn chronoscope_dominant_suspect_scoring_is_normalized_and_less_biased() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.nodes[0] = ChronoscopeNode {
            valid: true,
            node_id: 1,
            stable_id: 1001,
            kind: ChronoscopeNodeKind::Interpretation,
            confidence: 0.95,
            severity: 4,
            causal_distance_to_fault: 1,
            evidence_count: 2,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[1] = ChronoscopeNode {
            valid: true,
            node_id: 2,
            stable_id: 1002,
            kind: ChronoscopeNodeKind::Interpretation,
            confidence: 0.55,
            severity: 4,
            causal_distance_to_fault: 6,
            evidence_count: 20,
            ..ChronoscopeNode::EMPTY
        };
        let dominant = chronoscope_select_dominant_suspect(&bundle);
        assert_eq!(dominant, 1);
    }

    #[test]
    fn chronoscope_stable_node_id_is_fast_and_collision_resistant_for_nearby_inputs() {
        let a = chronoscope_stable_node_id(
            ChronoscopeNodeKind::Observation,
            1,
            10,
            20,
            30,
            chronoscope_payload_hash(1, 2, 3, 4),
        );
        let b = chronoscope_stable_node_id(
            ChronoscopeNodeKind::Observation,
            1,
            10,
            20,
            30,
            chronoscope_payload_hash(1, 2, 3, 5),
        );
        let c = chronoscope_stable_node_id(
            ChronoscopeNodeKind::Observation,
            2,
            10,
            20,
            30,
            chronoscope_payload_hash(1, 2, 3, 4),
        );
        assert_ne!(a, 0);
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    fn synthetic_temporal_capsule() -> CrashCapsule {
        let mut suspects = [SuspectPoint::EMPTY; SUSPECT_LIMIT];
        suspects[0] = SuspectPoint {
            valid: true,
            score: 91,
            cpu_slot: 0,
            stage: BootTraceStage::EnterUserMode as u16,
            event_sequence: 12,
            request_id: 7,
            completion_id: 8,
            irq_id: 9,
            event_kind: TraceKind::Memory,
            reason_code: 5,
        };
        let mut watch_tail = [ViolationRecord::EMPTY; VIOLATION_TAIL];
        watch_tail[0] = ViolationRecord {
            sequence: 13,
            kind: ViolationKind::Guard,
            descriptor_id: 1,
            descriptor_kind: GuardKind::RequestBuffer as u16,
            address: 0x1000,
            length: 8,
            descriptor_address: 0x1008,
            descriptor_length: 32,
            overlap: MemoryOverlapClass::Span,
            suspicion_flags: MEMORY_SUSPECT_OVERRUN | MEMORY_SUSPECT_REPEATED,
            relative_start: 0,
            relative_end: 16,
            stage: BootTraceStage::EnterUserMode as u16,
            path: DiagnosticsPath::Completion,
            request_id: 7,
            completion_id: 8,
            cpu_slot: 0,
            apic_id: 1,
        };
        let mut trace_tail = [[TraceRecord::EMPTY; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS];
        trace_tail[0][0] = TraceRecord {
            sequence: 10,
            uptime_us: 1,
            apic_id: 1,
            cpu_slot: 0,
            kind: TraceKind::Device,
            channel: TraceChannel::Device,
            stage: BootTraceStage::EnterUserMode as u16,
            a: 7,
            b: 8,
            c: 9,
            d: 0,
        };
        trace_tail[0][1] = TraceRecord {
            sequence: 12,
            uptime_us: 2,
            apic_id: 1,
            cpu_slot: 0,
            kind: TraceKind::Memory,
            channel: TraceChannel::Memory,
            stage: BootTraceStage::EnterUserMode as u16,
            a: 7,
            b: 8,
            c: 9,
            d: 1,
        };
        trace_tail[1][0] = TraceRecord {
            sequence: 11,
            uptime_us: 2,
            apic_id: 2,
            cpu_slot: 1,
            kind: TraceKind::Device,
            channel: TraceChannel::Device,
            stage: BootTraceStage::EnterUserMode as u16,
            a: 7,
            b: 8,
            c: 9,
            d: 2,
        };
        CrashCapsule {
            valid: true,
            generation: 2,
            mode: DiagnosticsMode::CrashFollowup,
            failure_signature: FailureSignature {
                valid: true,
                id: 100,
                path: DiagnosticsPath::Completion,
                stage: BootTraceStage::EnterUserMode as u16,
                fault_vector: 14,
                dominant_violation: ViolationKind::Guard,
                last_good_kind: TraceKind::Device,
                last_good_stage: BootTraceStage::EnterUserMode as u16,
                first_bad_kind: TraceKind::Memory,
                first_bad_stage: BootTraceStage::EnterUserMode as u16,
                divergence_kind: TraceKind::Memory,
                divergence_stage: BootTraceStage::EnterUserMode as u16,
                chain_shape: 1,
            },
            failure_signature_id: 100,
            closest_prior_pattern_id: 0,
            stable_prefix_length: 1,
            unstable_suffix_length: 2,
            first_divergence_sequence: 11,
            semantic_reasons: SemanticReasonAggregate::EMPTY,
            suspects,
            replay: ReplayCorrelationIds {
                sequence: 13,
                request_id: 7,
                completion_id: 8,
                irq_id: 9,
            },
            fault: FaultRecord {
                valid: true,
                cpu_slot: 0,
                stage: BootTraceStage::EnterUserMode as u16,
                vector: 14,
                ..FaultRecord::EMPTY
            },
            reprobe: ReprobePolicyState::EMPTY,
            window: RequestWindowState {
                valid: true,
                syscall_id: 1,
                fd: 1,
                request_op: 1,
                device_id: 1,
                completion_state: 1,
                path: DiagnosticsPath::Completion,
                request_id: 7,
                completion_id: 8,
            },
            watch_tail,
            trace_tail,
        }
    }

    #[test]
    fn chronoscope_gen3_infers_temporal_checkpoints_from_synthetic_capsule() {
        let snapshot = build_chronoscope_bundle(&synthetic_temporal_capsule());
        assert!(
            snapshot
                .checkpoints
                .iter()
                .any(|entry| entry.valid && entry.kind == ChronoscopeCheckpointKind::FaultAdjacent)
        );
        assert!(
            snapshot.checkpoints.iter().any(
                |entry| entry.valid && entry.kind == ChronoscopeCheckpointKind::RewindCandidate
            )
        );
        assert!(snapshot.state_before_fault().is_some());
        assert!(snapshot.rewind_candidate().is_some());
    }

    #[test]
    fn chronoscope_gen3_predecessor_lookup_tracks_checkpoint_chain() {
        let snapshot = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let rewind = snapshot.rewind_candidate().unwrap();
        assert_ne!(rewind.0, 0);
        let predecessor = snapshot.predecessor_state(rewind);
        assert!(predecessor.is_some() || snapshot.state_before_fault().is_some());
    }

    #[test]
    fn chronoscope_gen3_builds_lineage_and_last_mutation() {
        let snapshot = build_chronoscope_bundle(&synthetic_temporal_capsule());
        assert!(snapshot.lineage.iter().any(|entry| entry.valid));
        let primary = snapshot.dominant_suspect_node_id;
        let lineage = snapshot.lineage_for_node(primary);
        assert!(!lineage.is_empty());
        let mutation = snapshot.last_mutation_of(
            ChronoscopeLineageDomain::SuspectState,
            snapshot
                .node_by_id(primary)
                .map(chronoscope_lineage_key_for_node)
                .unwrap_or(0),
        );
        assert_eq!(mutation, Some(primary));
    }

    #[test]
    fn chronoscope_gen3_temporal_diff_detects_first_divergence() {
        let left = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let mut right = build_chronoscope_bundle(&synthetic_temporal_capsule());
        if let Some(slot) = right.checkpoint_index_by_id(right.rewind_checkpoint) {
            right.checkpoints[slot].stable_id ^= 0x55;
        }
        right.temporal_explain = chronoscope_build_temporal_explain_plan(&right);
        let diff = right.diff_against(&left);
        assert_ne!(diff.first_temporal_divergence, 0);
        assert!(
            diff.changed_rewind_candidate
                || diff.summary.new_checkpoints != 0
                || diff.summary.missing_checkpoints != 0
        );
    }

    #[test]
    fn chronoscope_gen3_temporal_explain_carries_checkpoint_and_mutation_state() {
        let snapshot = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let explain = snapshot.temporal_explain.clone();
        assert!(explain.valid);
        assert_ne!(explain.state_before_fault.0, 0);
        assert_ne!(explain.rewind_candidate.0, 0);
        assert_ne!(explain.last_mutation, 0);
    }

    #[test]
    fn runtime_event_fabric_preserves_per_core_append_order() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 11,
                completion_id: 22,
                irq_id: 33,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let first = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::RequestStart,
            1,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let second = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::RequestComplete,
            1,
            first,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let window = snapshot_runtime_events(replay_ids());
        assert_eq!(window.total_events, 2);
        assert_eq!(window.events[0].event_id, first);
        assert_eq!(window.events[1].event_id, second);
        assert!(window.events[1].local_sequence > window.events[0].local_sequence);
    }

    #[test]
    fn runtime_event_fabric_overwrites_oldest_entries_when_full() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 99,
                completion_id: 0,
                irq_id: 0,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let mut index = 0usize;
        while index < (RUNTIME_EVENT_CAPACITY + 4) {
            let _ = emit_runtime_event_with_context(
                context,
                ChronoscopeRuntimeEventKind::RequestStart,
                index as u64,
                ChronoscopeEventId::NONE,
                0,
                ChronoscopeRuntimePayload::None,
            );
            index += 1;
        }
        let window = snapshot_runtime_events(replay_ids());
        assert!(window.partial);
        assert_eq!(window.per_core[0].overwritten_events, 4);
        assert_eq!(window.per_core[0].oldest_local_sequence, 5);
    }

    #[test]
    fn runtime_event_capture_policy_filters_non_critical_events() {
        let _guard = test_lock();
        reset();
        set_runtime_capture_level(ChronoscopeCaptureLevel::Minimal);
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 55,
                completion_id: 66,
                irq_id: 77,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let skipped = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::SchedulerWake,
            0,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let kept = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::FaultMarker,
            14,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::Fault {
                vector: 14,
                stage: context.stage,
                reserved: 0,
            },
        );
        let window = snapshot_runtime_events(replay_ids());
        assert_eq!(skipped, ChronoscopeEventId::NONE);
        assert_ne!(kept, ChronoscopeEventId::NONE);
        assert_eq!(window.total_events, 1);
    }

    #[test]
    fn runtime_event_snapshot_marks_partial_history() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 77,
                completion_id: 88,
                irq_id: 99,
            };
        }
        CPU_RUNTIME_OVERWRITES[1].store(3, Ordering::Relaxed);
        let window = snapshot_runtime_events(replay_ids());
        assert!(window.partial);
        assert!(window.per_core[1].partial);
    }

    #[test]
    fn runtime_event_bridge_augments_chronoscope_bundle() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 7,
                completion_id: 8,
                irq_id: 9,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::RequestStart,
            42,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::Request {
                op: 1,
                status: 1,
                device_id: 42,
            },
        );
        let snapshot = build_chronoscope_bundle(&synthetic_temporal_capsule());
        assert!(snapshot.runtime_events.total_events >= 1);
        assert!(snapshot.nodes.iter().any(|node| {
            node.valid
                && node.request_id == 7
                && node.event_sequence == snapshot.runtime_events.events[0].event_id.0
        }));
    }

    #[test]
    fn chronoscope_gen5_last_writer_and_writer_chain_are_reconstructed() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.nodes[0] = ChronoscopeNode {
            valid: true,
            node_id: 1,
            stable_id: 1,
            event_sequence: 10,
            kind: ChronoscopeNodeKind::Observation,
            request_id: 7,
            stage: BootTraceStage::EnterUserMode as u16,
            path: DiagnosticsPath::Completion,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[1] = ChronoscopeNode {
            valid: true,
            node_id: 2,
            stable_id: 2,
            event_sequence: 11,
            kind: ChronoscopeNodeKind::Observation,
            request_id: 7,
            stage: BootTraceStage::EnterUserMode as u16,
            path: DiagnosticsPath::Completion,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[2] = ChronoscopeNode {
            valid: true,
            node_id: 3,
            stable_id: 3,
            event_sequence: 12,
            kind: ChronoscopeNodeKind::Outcome,
            request_id: 7,
            ..ChronoscopeNode::EMPTY
        };
        chronoscope_build_last_writers(&mut bundle);
        let key = chronoscope_lineage_key_for_node(&bundle.nodes[1]);
        assert_eq!(
            bundle.last_writer_of(ChronoscopeLineageDomain::RequestPath, key),
            Some(2)
        );
        let chain = bundle.writer_chain_to_fault(ChronoscopeLineageDomain::RequestPath, key);
        assert_eq!(chain[0], 2);
        assert_eq!(chain.last().copied(), Some(3));
    }

    #[test]
    fn chronoscope_gen5_capability_derivation_chain_is_deterministic() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.capability_derivations[0] = CapabilityDerivationRecord {
            valid: true,
            parent: CapabilityId(1),
            derived: CapabilityId(2),
            node_id: 10,
            event_id: ChronoscopeEventId(10),
            rights_mask: 0xff,
            confidence_permille: 1000,
        };
        bundle.capability_derivations[1] = CapabilityDerivationRecord {
            valid: true,
            parent: CapabilityId(2),
            derived: CapabilityId(3),
            node_id: 11,
            event_id: ChronoscopeEventId(11),
            rights_mask: 0x0f,
            confidence_permille: 1000,
        };
        bundle.capability_parent_index
            [(CapabilityId(2).0 as usize) % CHRONOSCOPE_CAPABILITY_INDEX_LIMIT] =
            CapabilityParentIndexEntry {
                valid: true,
                capability: CapabilityId(2),
                record_index: 0,
            };
        bundle.capability_parent_index
            [(CapabilityId(3).0 as usize) % CHRONOSCOPE_CAPABILITY_INDEX_LIMIT] =
            CapabilityParentIndexEntry {
                valid: true,
                capability: CapabilityId(3),
                record_index: 1,
            };
        assert_eq!(bundle.capability_origin(CapabilityId(3)), CapabilityId(1));
        assert_eq!(
            bundle.capability_chain(CapabilityId(3)),
            vec![CapabilityId(1), CapabilityId(2), CapabilityId(3)]
        );
    }

    #[test]
    fn chronoscope_gen5_propagation_path_tracks_to_fault() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.nodes[0] = ChronoscopeNode {
            valid: true,
            node_id: 1,
            stable_id: 1,
            kind: ChronoscopeNodeKind::Interpretation,
            causal_distance_to_fault: 2,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[1] = ChronoscopeNode {
            valid: true,
            node_id: 2,
            stable_id: 2,
            kind: ChronoscopeNodeKind::Constraint,
            causal_distance_to_fault: 1,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[2] = ChronoscopeNode {
            valid: true,
            node_id: 3,
            stable_id: 3,
            kind: ChronoscopeNodeKind::Outcome,
            causal_distance_to_fault: 0,
            ..ChronoscopeNode::EMPTY
        };
        bundle.propagation[0] = PropagationRecord {
            valid: true,
            source_node: 1,
            target_node: 2,
            propagation_type: PropagationType::DataFlow,
            correlation: CorrelationKey {
                request_id: 1,
                completion_id: 0,
                irq_id: 0,
            },
        };
        bundle.propagation[1] = PropagationRecord {
            valid: true,
            source_node: 2,
            target_node: 3,
            propagation_type: PropagationType::CausalAmplification,
            correlation: CorrelationKey {
                request_id: 1,
                completion_id: 0,
                irq_id: 0,
            },
        };
        bundle.propagation_heads[0] = 0;
        bundle.propagation_heads[1] = 1;
        bundle.propagation_next[0] = CHRONOSCOPE_INVALID_INDEX;
        bundle.propagation_next[1] = CHRONOSCOPE_INVALID_INDEX;
        assert_eq!(bundle.propagation_chain_to_fault(1), vec![1, 2, 3]);
    }

    #[test]
    fn chronoscope_gen5_responsibility_ranking_is_stable() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.nodes[0] = ChronoscopeNode {
            valid: true,
            node_id: 1,
            stable_id: 1,
            kind: ChronoscopeNodeKind::Constraint,
            score: 10,
            causal_distance_to_fault: 1,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[1] = ChronoscopeNode {
            valid: true,
            node_id: 2,
            stable_id: 2,
            kind: ChronoscopeNodeKind::Interpretation,
            score: 5,
            causal_distance_to_fault: 3,
            ..ChronoscopeNode::EMPTY
        };
        bundle.propagation[0] = PropagationRecord {
            valid: true,
            source_node: 1,
            target_node: 2,
            propagation_type: PropagationType::ControlFlow,
            correlation: CorrelationKey {
                request_id: 0,
                completion_id: 0,
                irq_id: 0,
            },
        };
        chronoscope_build_responsibility(&mut bundle);
        assert_eq!(bundle.primary_responsible_node(), 1);
        assert_eq!(bundle.responsibility_ranking()[0].0, 1);
    }

    #[test]
    fn chronoscope_gen5_diff_detects_responsibility_changes() {
        let mut left = ChronoscopeBundle::EMPTY;
        let mut right = ChronoscopeBundle::EMPTY;
        left.valid = true;
        right.valid = true;
        left.temporal_explain.last_writer = 1;
        right.temporal_explain.last_writer = 2;
        left.temporal_explain.responsibility_ranking[0] = ResponsibilityEntry {
            valid: true,
            node_id: 1,
            score: 70,
        };
        right.temporal_explain.responsibility_ranking[0] = ResponsibilityEntry {
            valid: true,
            node_id: 2,
            score: 80,
        };
        let diff = right.diff_against(&left);
        assert!(diff.changed_last_writer);
        assert!(diff.changed_responsibility_ranking);
    }

    #[test]
    fn chronoscope_gen5_last_writer_and_propagation_indices_avoid_linear_lookup_paths() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.nodes[0] = ChronoscopeNode {
            valid: true,
            node_id: 1,
            stable_id: 1,
            event_sequence: 10,
            kind: ChronoscopeNodeKind::Observation,
            request_id: 42,
            stage: BootTraceStage::EnterUserMode as u16,
            path: DiagnosticsPath::Completion,
            causal_distance_to_fault: 2,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[1] = ChronoscopeNode {
            valid: true,
            node_id: 2,
            stable_id: 2,
            event_sequence: 11,
            kind: ChronoscopeNodeKind::Constraint,
            request_id: 42,
            stage: BootTraceStage::EnterUserMode as u16,
            path: DiagnosticsPath::Completion,
            causal_distance_to_fault: 1,
            ..ChronoscopeNode::EMPTY
        };
        bundle.nodes[2] = ChronoscopeNode {
            valid: true,
            node_id: 3,
            stable_id: 3,
            event_sequence: 12,
            kind: ChronoscopeNodeKind::Outcome,
            request_id: 42,
            causal_distance_to_fault: 0,
            ..ChronoscopeNode::EMPTY
        };
        chronoscope_build_last_writers(&mut bundle);
        let key = chronoscope_lineage_key_for_node(&bundle.nodes[1]);
        assert!(bundle.last_writer_index.iter().any(|entry| {
            entry.valid
                && entry.domain == ChronoscopeLineageDomain::ViolationState
                && entry.key == key
        }));
        bundle.edges[0] = ChronoscopeEdge {
            valid: true,
            src_node_id: 1,
            dst_node_id: 2,
            kind: ChronoscopeEdgeKind::Supports,
            weight: 1,
        };
        bundle.edges[1] = ChronoscopeEdge {
            valid: true,
            src_node_id: 2,
            dst_node_id: 3,
            kind: ChronoscopeEdgeKind::LeadsTo,
            weight: 1,
        };
        chronoscope_build_propagation(&mut bundle);
        assert_ne!(bundle.propagation_heads[0], CHRONOSCOPE_INVALID_INDEX);
        assert_eq!(bundle.propagation_chain_to_fault(1), vec![1, 2, 3]);
    }

    #[test]
    fn chronoscope_gen6_replay_is_deterministic_for_same_runtime_window() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 101,
                completion_id: 202,
                irq_id: 303,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::RequestStart,
            1,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::ViolationObserved,
            2,
            ChronoscopeEventId::NONE,
            1,
            ChronoscopeRuntimePayload::None,
        );
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::FaultMarker,
            14,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let left = bundle.replay_to_fault();
        let right = bundle.replay_to_fault();
        assert_eq!(left.total_steps, right.total_steps);
        assert_eq!(left.events[0].event_id, right.events[0].event_id);
        assert_eq!(left.final_state, right.final_state);
    }

    #[test]
    fn chronoscope_gen6_rewind_to_checkpoint_uses_replay_start_index() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 7,
                completion_id: 8,
                irq_id: 9,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::RequestStart,
            1,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let checkpoint = bundle
            .rewind_candidate()
            .unwrap_or(ChronoscopeCheckpointId::NONE);
        let cursor = bundle.rewind_to(checkpoint);
        assert!(cursor.valid || checkpoint == ChronoscopeCheckpointId::NONE);
        if checkpoint != ChronoscopeCheckpointId::NONE {
            assert_eq!(cursor.checkpoint_id, checkpoint);
        }
    }

    #[test]
    fn chronoscope_gen6_replay_until_fault_reaches_fault_marker() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 111,
                completion_id: 222,
                irq_id: 333,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::RequestStart,
            1,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::FaultMarker,
            14,
            ChronoscopeEventId::NONE,
            0,
            ChronoscopeRuntimePayload::None,
        );
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let result = bundle.replay_until_fault();
        assert!(result.valid);
        assert_eq!(
            result.last_event.kind,
            ChronoscopeRuntimeEventKind::FaultMarker
        );
    }

    #[test]
    fn chronoscope_gen6_divergence_detection_finds_first_mismatch() {
        let mut left = ReplayTrace::EMPTY;
        left.valid = true;
        left.total_steps = 2;
        left.events[0] = ReplayEvent {
            valid: true,
            event_id: ChronoscopeEventId(1),
            core_id: 0,
            local_sequence: 1,
            kind: ChronoscopeRuntimeEventKind::RequestStart,
            correlation: CorrelationKey {
                request_id: 1,
                completion_id: 0,
                irq_id: 0,
            },
            capability_id: CapabilityId::NONE,
            object_key: 0,
            causal_parent: ChronoscopeEventId::NONE,
            flags: 0,
        };
        left.events[1] = ReplayEvent {
            event_id: ChronoscopeEventId(2),
            kind: ChronoscopeRuntimeEventKind::FaultMarker,
            ..left.events[0]
        };
        let mut right = left;
        right.events[1].event_id = ChronoscopeEventId(3);
        let bundle = ChronoscopeBundle::EMPTY;
        let diff = bundle.detect_divergence(&left, &right);
        assert!(diff.valid);
        assert_eq!(diff.first_event_mismatch, ChronoscopeEventId(2));
    }

    #[test]
    fn chronoscope_gen6_state_reconstruction_is_consistent() {
        let mut state = ReplaySemanticState::EMPTY;
        apply_replay_event(
            &mut state,
            ReplayEvent {
                valid: true,
                event_id: ChronoscopeEventId(1),
                core_id: 0,
                local_sequence: 1,
                kind: ChronoscopeRuntimeEventKind::RequestStart,
                correlation: CorrelationKey {
                    request_id: 7,
                    completion_id: 8,
                    irq_id: 9,
                },
                capability_id: CapabilityId(1),
                object_key: 42,
                causal_parent: ChronoscopeEventId::NONE,
                flags: 0,
            },
        );
        apply_replay_event(
            &mut state,
            ReplayEvent {
                kind: ChronoscopeRuntimeEventKind::ViolationObserved,
                flags: 1,
                ..ReplayEvent {
                    valid: true,
                    event_id: ChronoscopeEventId(2),
                    core_id: 0,
                    local_sequence: 2,
                    kind: ChronoscopeRuntimeEventKind::RequestStart,
                    correlation: CorrelationKey {
                        request_id: 7,
                        completion_id: 8,
                        irq_id: 9,
                    },
                    capability_id: CapabilityId(1),
                    object_key: 42,
                    causal_parent: ChronoscopeEventId::NONE,
                    flags: 0,
                }
            },
        );
        assert!(
            state
                .requests
                .iter()
                .any(|entry| entry.valid && entry.request_id == 7)
        );
        assert_eq!(state.violation_flags, 1);
    }

    #[test]
    fn chronoscope_gen7_parser_accepts_basic_queries() {
        let query = parse_chronoscope_query("last-writer request 0x2a").unwrap();
        assert_eq!(query.kind, ChronoscopeQueryKind::LastWriter);
        assert_eq!(query.domain, Some(ChronoscopeLineageDomain::RequestPath));
        assert_eq!(query.key, 0x2a);
    }

    #[test]
    fn chronoscope_gen7_parser_rejects_invalid_queries() {
        let error = parse_chronoscope_query("last-writer nonsense nope").unwrap_err();
        assert_eq!(error.reason, "invalid-key");
    }

    #[test]
    fn chronoscope_gen7_query_executes_last_writer() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.last_writers[0] = LastWriterRecord {
            valid: true,
            domain: ChronoscopeLineageDomain::RequestPath,
            key: 42,
            last_writer_node: 7,
            predecessor_writer_node: 0,
            event_id: ChronoscopeEventId::NONE,
            capability_id: CapabilityId::NONE,
        };
        bundle.last_writer_index[10] = LastWriterIndexEntry {
            valid: true,
            domain: ChronoscopeLineageDomain::RequestPath,
            key: 42,
            record_index: 0,
        };
        let result = chronoscope_query::run_query(&bundle, "last-writer request 42");
        assert_eq!(result.kind, ChronoscopeQueryResultKind::Scalar);
        assert_eq!(result.rows[0].value, 7);
    }

    #[test]
    fn chronoscope_gen7_query_executes_capability_chain() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.capability_derivations[0] = CapabilityDerivationRecord {
            valid: true,
            parent: CapabilityId(1),
            derived: CapabilityId(2),
            node_id: 0,
            event_id: ChronoscopeEventId::NONE,
            rights_mask: 0,
            confidence_permille: 1000,
        };
        bundle.capability_parent_index[2] = CapabilityParentIndexEntry {
            valid: true,
            capability: CapabilityId(2),
            record_index: 0,
        };
        let result = chronoscope_query::run_query(&bundle, "cap-chain 2");
        assert_eq!(result.kind, ChronoscopeQueryResultKind::CapabilityList);
        assert_eq!(result.capabilities, vec![CapabilityId(1), CapabilityId(2)]);
    }

    #[test]
    fn chronoscope_gen7_query_executes_replay_until_violation() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 5,
                completion_id: 6,
                irq_id: 7,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let _ = emit_runtime_event_with_context(
            context,
            ChronoscopeRuntimeEventKind::ViolationObserved,
            1,
            ChronoscopeEventId::NONE,
            1,
            ChronoscopeRuntimePayload::None,
        );
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let result = chronoscope_query::run_query(&bundle, "replay until violation");
        assert_eq!(result.kind, ChronoscopeQueryResultKind::ReplaySummary);
        assert!(result.replay.unwrap().steps >= 1);
    }

    #[test]
    fn chronoscope_gen7_query_executes_explain_fault() {
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let result = chronoscope_query::run_query(&bundle, "explain fault");
        assert_eq!(result.kind, ChronoscopeQueryResultKind::ExplainSummary);
        assert!(result.explain.unwrap().plan.valid);
    }

    #[test]
    fn chronoscope_gen7_query_limit_is_applied() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.responsibility[0] = ResponsibilityEntry {
            valid: true,
            node_id: 1,
            score: 90,
        };
        bundle.responsibility[1] = ResponsibilityEntry {
            valid: true,
            node_id: 2,
            score: 80,
        };
        let result = chronoscope_query::run_query(&bundle, "responsibility top limit 1");
        assert_eq!(result.kind, ChronoscopeQueryResultKind::Scalar);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn chronoscope_gen7_query_diff_requires_baseline() {
        let bundle = ChronoscopeBundle::EMPTY;
        let result = chronoscope_query::run_query(&bundle, "diff first-divergence");
        assert_eq!(result.kind, ChronoscopeQueryResultKind::Error);
        assert_eq!(result.error.unwrap().reason, "missing-baseline");
    }

    #[test]
    fn memory_lineage_chains_parent_versions_by_overlap() {
        let _guard = test_lock();
        reset();
        let v1 = record_memory_lineage(
            MemoryLineageKind::Snapshot,
            1,
            0x1000,
            0x1000,
            55,
            4096,
            0xaaaa,
        );
        let v2 = record_memory_lineage(MemoryLineageKind::Write, 1, 0x1800, 0x100, 55, 64, 0xbbbb);
        let lineage = unsafe { *TRACE_STORAGE.memory_lineage.get() };
        let second = lineage
            .iter()
            .find(|entry| entry.valid && entry.version_id == v2)
            .copied()
            .unwrap();
        assert_eq!(v1, 1);
        assert_eq!(second.parent_version_id, v1);
        assert_eq!(second.object_id, 55);
        assert_eq!(second.kind, MemoryLineageKind::Write);
    }

    #[test]
    fn memory_lineage_summary_tracks_hot_object_and_latest_version() {
        let _guard = test_lock();
        reset();
        let _ = record_memory_lineage(MemoryLineageKind::Snapshot, 2, 0x2000, 0x1000, 99, 4096, 1);
        let v2 = record_memory_lineage(MemoryLineageKind::Write, 2, 0x2000, 0x80, 99, 32, 2);
        let _ = record_memory_lineage(MemoryLineageKind::Dma, 2, 0x4000, 0x200, 11, 128, 3);
        let summary = build_memory_lineage_summary();
        assert_eq!(summary.total_versions, 3);
        assert_eq!(summary.writes, 1);
        assert_eq!(summary.dmas, 1);
        assert_eq!(summary.latest_version_id, v2.max(3));
        assert_eq!(summary.hottest_object_id, 99);
        assert_eq!(summary.hottest_object_versions, 2);
    }

    #[test]
    fn memory_lineage_export_orders_latest_versions_first() {
        let _guard = test_lock();
        reset();
        let _ = record_memory_lineage(MemoryLineageKind::Snapshot, 3, 0x3000, 0x1000, 1, 4096, 7);
        let v2 = record_memory_lineage(MemoryLineageKind::Write, 3, 0x3000, 0x40, 1, 16, 8);
        let export = build_memory_lineage_export();
        assert!(export[0].valid);
        assert_eq!(export[0].version_id, v2);
        assert_eq!(export[0].parent_version_id, 1);
    }

    #[test]
    fn memory_lineage_hint_returns_latest_parent_without_semantic_change() {
        let _guard = test_lock();
        reset();
        let _ = record_memory_lineage(MemoryLineageKind::Snapshot, 4, 0x4000, 0x1000, 2, 4096, 10);
        let v2 = record_memory_lineage(MemoryLineageKind::Write, 4, 0x4400, 0x80, 2, 32, 11);
        let parent = last_memory_version_for_range(4, 0x4400, 0x40);
        assert_eq!(parent, v2);
    }

    #[test]
    fn violation_hint_marks_repeated_descriptor_without_semantic_change() {
        let _guard = test_lock();
        reset();
        let _ = guard_register(
            GuardKind::RequestBuffer,
            DiagnosticsPath::Block,
            0x5000,
            64,
            16,
            1,
            2,
        );
        assert!(!guard_check(0x4ff8, 8));
        assert!(!guard_check(0x4ff0, 8));
        let violations = unsafe { *TRACE_STORAGE.violations.get() };
        let repeated = violations
            .iter()
            .filter(|entry| entry.sequence != 0)
            .max_by_key(|entry| entry.sequence)
            .copied()
            .unwrap();
        assert_ne!(repeated.suspicion_flags & MEMORY_SUSPECT_REPEATED, 0);
    }

    #[test]
    fn memory_violation_summary_tracks_repeated_descriptor_hits() {
        let mut capsule = CrashCapsule::EMPTY;
        capsule.valid = true;
        capsule.watch_tail[0] = ViolationRecord {
            sequence: 1,
            kind: ViolationKind::Guard,
            descriptor_id: 9,
            descriptor_kind: GuardKind::RequestBuffer as u16,
            address: 0x2000,
            length: 4,
            descriptor_address: 0x2004,
            descriptor_length: 32,
            overlap: MemoryOverlapClass::LeftRedZone,
            suspicion_flags: MEMORY_SUSPECT_UNDERRUN | MEMORY_SUSPECT_REPEATED,
            relative_start: -4,
            relative_end: -28,
            stage: 1,
            path: DiagnosticsPath::Block,
            request_id: 1,
            completion_id: 0,
            cpu_slot: 0,
            apic_id: 1,
        };
        capsule.watch_tail[1] = ViolationRecord {
            sequence: 2,
            kind: ViolationKind::Guard,
            descriptor_id: 9,
            descriptor_kind: GuardKind::RequestBuffer as u16,
            address: 0x2020,
            length: 24,
            descriptor_address: 0x2004,
            descriptor_length: 32,
            overlap: MemoryOverlapClass::Suffix,
            suspicion_flags: MEMORY_SUSPECT_OVERRUN | MEMORY_SUSPECT_REPEATED,
            relative_start: 28,
            relative_end: 20,
            stage: 1,
            path: DiagnosticsPath::Block,
            request_id: 1,
            completion_id: 0,
            cpu_slot: 0,
            apic_id: 1,
        };
        let summary = build_memory_violation_summary(&capsule);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.guard_hits, 2);
        assert_eq!(summary.repeated_hits, 2);
        assert_eq!(summary.hottest_descriptor_id, 9);
        assert_eq!(summary.hottest_descriptor_hits, 2);
    }

    #[test]
    fn chronoscope_gen8_detects_violation_burst_and_escalates() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 7,
                request_id: 700,
                completion_id: 0,
                irq_id: 0,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        for parent in [
            ChronoscopeEventId::NONE,
            ChronoscopeEventId(1),
            ChronoscopeEventId(2),
        ] {
            let _ = emit_runtime_event_with_context(
                context,
                ChronoscopeRuntimeEventKind::ViolationObserved,
                0x100,
                parent,
                3,
                ChronoscopeRuntimePayload::Violation {
                    violation_kind: 1,
                    descriptor_kind: 1,
                    score: 10,
                    flags: 1,
                },
            );
        }
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        assert!(
            bundle
                .anomalies
                .iter()
                .any(|entry| entry.valid && entry.kind == ChronoscopeAnomalyKind::ViolationBurst)
        );
        assert!(bundle.escalations.iter().any(|entry| entry.valid));
    }

    #[test]
    fn chronoscope_gen8_creates_capture_window_and_candidate() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 8,
                request_id: 800,
                completion_id: 0,
                irq_id: 0,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        for parent in [
            ChronoscopeEventId::NONE,
            ChronoscopeEventId(1),
            ChronoscopeEventId(2),
        ] {
            let _ = emit_runtime_event_with_context(
                context,
                ChronoscopeRuntimeEventKind::ViolationObserved,
                0x55,
                parent,
                3,
                ChronoscopeRuntimePayload::Violation {
                    violation_kind: 1,
                    descriptor_kind: 1,
                    score: 10,
                    flags: 1,
                },
            );
        }
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        assert!(
            bundle.capture_windows.iter().any(|entry| entry.valid)
                || bundle.escalations.iter().any(|entry| entry.valid)
        );
        assert_ne!(bundle.dominant_candidate().unwrap_or(0), 0);
    }

    #[test]
    fn chronoscope_gen8_query_parser_accepts_adaptive_queries() {
        let query = parse_chronoscope_query("anomalies top limit 2").unwrap();
        assert_eq!(query.kind, ChronoscopeQueryKind::AnomaliesTop);
        assert_eq!(query.limit, 2);
        let query = parse_chronoscope_query("candidate for-anomaly 1").unwrap();
        assert_eq!(query.kind, ChronoscopeQueryKind::CandidateForAnomaly);
        assert_eq!(query.anomaly_id, ChronoscopeAnomalyId(1));
    }

    #[test]
    fn chronoscope_gen8_diff_detects_adaptive_changes() {
        let mut left = ChronoscopeBundle::EMPTY;
        left.valid = true;
        left.adaptive_state = ChronoscopeAdaptiveState::Watching;
        left.anomalies[0] = ChronoscopeAnomalyRecord {
            valid: true,
            anomaly_id: ChronoscopeAnomalyId(1),
            kind: ChronoscopeAnomalyKind::ViolationBurst,
            first_event: ChronoscopeEventId(11),
            severity: 4,
            ..ChronoscopeAnomalyRecord::EMPTY
        };
        let mut right = left.clone();
        right.adaptive_state = ChronoscopeAdaptiveState::EscalatedLocal;
        right.escalations[0] = ChronoscopeEscalationRecord {
            valid: true,
            escalation_id: ChronoscopeEscalationId(1),
            level: ChronoscopeEscalationLevel::LocalDeep,
            target_core_mask: 1,
            ..ChronoscopeEscalationRecord::EMPTY
        };
        let diff = right.diff_against(&left);
        assert!(diff.changed_adaptive_state);
        assert_eq!(diff.summary.changed_escalations, 1);
    }

    #[test]
    fn chronoscope_config_validation_rejects_invalid_limits() {
        assert!(ChronoscopeConfig::DEFAULT.validate());
        let invalid = ChronoscopeConfig {
            max_query_rows: 0,
            ..ChronoscopeConfig::DEFAULT
        };
        assert!(!invalid.validate());
    }

    #[test]
    fn chronoscope_runtime_window_marks_integrity_on_overwrite() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 1,
                request_id: 1,
                completion_id: 0,
                irq_id: 0,
            };
        }
        let context = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let mut i = 0usize;
        while i < RUNTIME_EVENT_CAPACITY + 2 {
            let _ = emit_runtime_event_with_context(
                context,
                ChronoscopeRuntimeEventKind::RequestStart,
                i as u64,
                ChronoscopeEventId::NONE,
                0,
                ChronoscopeRuntimePayload::None,
            );
            i += 1;
        }
        let window = snapshot_runtime_events(replay_ids());
        assert!(window.partial);
        assert!(!window.integrity.complete);
        assert_eq!(
            window.integrity.primary_reason,
            ChronoscopePartialReason::RingOverwrite
        );
    }

    #[test]
    fn chronoscope_validation_catches_bad_lineage_reference() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.perf.schema_version = CHRONOSCOPE_SCHEMA_VERSION;
        bundle.lineage[0] = ChronoscopeLineageRecord {
            valid: true,
            lineage_id: ChronoscopeLineageId(1),
            stable_id: 1,
            domain: ChronoscopeLineageDomain::RequestPath,
            key: 1,
            prior_checkpoint: ChronoscopeCheckpointId::NONE,
            prior_node: 0,
            transition_node: 999,
            result_checkpoint: ChronoscopeCheckpointId::NONE,
            result_node: 0,
            confidence_permille: 1,
        };
        let validation = validate_bundle_invariants(&bundle);
        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .contains(&ChronoscopeValidationError::InvalidLineageReference)
        );
    }

    #[test]
    fn chronoscope_query_formatting_is_stable_and_sorted() {
        let mut result = ChronoscopeQueryResult::empty();
        result.kind = ChronoscopeQueryResultKind::Scalar;
        result.rows.push(ChronoscopeQueryRow { key: "z", value: 2 });
        result.rows.push(ChronoscopeQueryRow { key: "a", value: 1 });
        let mut out = alloc::string::String::new();
        chronoscope_query::emit_query_result(&result, &mut out).unwrap();
        assert_eq!(out, "a=1\nz=2\n");
    }

    #[test]
    fn chronoscope_trust_surface_reflects_partial_history() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.runtime_events.integrity = ChronoscopeHistoryIntegrity {
            complete: false,
            primary_reason: ChronoscopePartialReason::RingOverwrite,
            flags: ChronoscopeCompletenessFlags {
                ring_overwrite: true,
                ..ChronoscopeCompletenessFlags::EMPTY
            },
        };
        bundle.temporal_explain.responsibility_confidence = 400;
        chronoscope_build_trust_surface(&mut bundle);
        assert!(!bundle.trust.completeness.complete);
        assert!(bundle.trust.responsibility_partial);
    }

    #[test]
    fn chronoscope_replay_sequence_remains_deterministic_after_multi_swap_sort() {
        let mut window = ChronoscopeRuntimeEventWindow::EMPTY;
        window.valid = true;
        window.total_events = 4;
        window.events[0] = ChronoscopeRuntimeEventRecord {
            valid: true,
            event_id: ChronoscopeEventId(40),
            core_id: 1,
            local_sequence: 5,
            uptime_us: 50,
            kind: ChronoscopeRuntimeEventKind::RequestComplete,
            ..ChronoscopeRuntimeEventRecord::EMPTY
        };
        window.events[1] = ChronoscopeRuntimeEventRecord {
            valid: true,
            event_id: ChronoscopeEventId(10),
            core_id: 0,
            local_sequence: 1,
            uptime_us: 10,
            kind: ChronoscopeRuntimeEventKind::RequestStart,
            ..ChronoscopeRuntimeEventRecord::EMPTY
        };
        window.events[2] = ChronoscopeRuntimeEventRecord {
            valid: true,
            event_id: ChronoscopeEventId(20),
            core_id: 0,
            local_sequence: 2,
            uptime_us: 20,
            kind: ChronoscopeRuntimeEventKind::ContractTransition,
            ..ChronoscopeRuntimeEventRecord::EMPTY
        };
        window.events[3] = ChronoscopeRuntimeEventRecord {
            valid: true,
            event_id: ChronoscopeEventId(30),
            core_id: 1,
            local_sequence: 1,
            uptime_us: 30,
            kind: ChronoscopeRuntimeEventKind::IrqEnter,
            ..ChronoscopeRuntimeEventRecord::EMPTY
        };
        let first = replay_sequence_from_runtime(&window);
        let second = replay_sequence_from_runtime(&window);
        assert!(validate_replay_sequence(&first));
        assert_eq!(first, second);
        assert_eq!(first.events[0].event_id, ChronoscopeEventId(10));
        assert_eq!(first.events[1].event_id, ChronoscopeEventId(20));
        assert_eq!(first.events[2].event_id, ChronoscopeEventId(30));
        assert_eq!(first.events[3].event_id, ChronoscopeEventId(40));
    }

    #[test]
    fn chronoscope_responsibility_selection_and_ranking_are_stable() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.responsibility[0] = ResponsibilityEntry {
            valid: true,
            node_id: 9,
            score: 400,
        };
        bundle.responsibility[1] = ResponsibilityEntry {
            valid: true,
            node_id: 3,
            score: 900,
        };
        bundle.responsibility[2] = ResponsibilityEntry {
            valid: true,
            node_id: 2,
            score: 900,
        };
        assert_eq!(bundle.primary_responsible_node(), 2);
        assert_eq!(
            bundle.responsibility_ranking(),
            vec![(2, 900), (3, 900), (9, 400)]
        );
    }

    #[test]
    fn chronoscope_replay_marks_partial_when_history_is_overwritten() {
        let _guard = test_lock();
        reset();
        unsafe {
            *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                sequence: 99,
                request_id: 55,
                completion_id: 0,
                irq_id: 0,
            };
        }
        let core0 = CpuTraceContext {
            cpu_slot: 0,
            apic_id: 1,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let core1 = CpuTraceContext {
            cpu_slot: 1,
            apic_id: 2,
            stage: BootTraceStage::EnterUserMode as u16,
        };
        let mut i = 0usize;
        while i < RUNTIME_EVENT_CAPACITY + 6 {
            let context = if i & 1 == 0 { core0 } else { core1 };
            let _ = emit_runtime_event_with_context(
                context,
                ChronoscopeRuntimeEventKind::RequestStart,
                i as u64,
                ChronoscopeEventId::NONE,
                0,
                ChronoscopeRuntimePayload::None,
            );
            i += 1;
        }
        let replay = replay_sequence_from_runtime(&snapshot_runtime_events(replay_ids()));
        assert!(replay.valid);
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.runtime_events.partial = true;
        bundle.runtime_events.integrity = ChronoscopeHistoryIntegrity {
            complete: false,
            primary_reason: ChronoscopePartialReason::RingOverwrite,
            flags: ChronoscopeCompletenessFlags {
                ring_overwrite: true,
                ..ChronoscopeCompletenessFlags::EMPTY
            },
        };
        bundle.temporal_explain.replay_summary.valid = true;
        bundle.temporal_explain.replay_summary.partial = true;
        bundle.temporal_explain.replay_summary.deterministic = false;
        chronoscope_build_trust_surface(&mut bundle);
        assert!(bundle.trust.replay_partial);
        assert_eq!(
            bundle.integrity.primary_reason,
            ChronoscopePartialReason::RingOverwrite
        );
    }

    #[test]
    fn chronoscope_validation_catches_inconsistent_explain_plan() {
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.perf.schema_version = CHRONOSCOPE_SCHEMA_VERSION;
        bundle.temporal_explain.valid = true;
        bundle.temporal_explain.fault_node = 1234;
        bundle.temporal_explain.last_writer = 77;
        let validation = validate_bundle_invariants(&bundle);
        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .contains(&ChronoscopeValidationError::InconsistentExplainPlan)
        );
    }

    #[test]
    fn chronoscope_multicore_event_storm_preserves_monotonic_sequences_and_partial_flags() {
        let _guard = test_lock();
        reset();
        let mut cpu = 0usize;
        while cpu < 4 {
            unsafe {
                *TRACE_STORAGE.replay_ids.get() = ReplayCorrelationIds {
                    sequence: cpu as u64 + 1,
                    request_id: 0x100 + cpu as u64,
                    completion_id: 0x200 + cpu as u64,
                    irq_id: 0x300 + cpu as u64,
                };
            }
            let context = CpuTraceContext {
                cpu_slot: cpu,
                apic_id: cpu as u32 + 1,
                stage: BootTraceStage::EnterUserMode as u16,
            };
            let mut i = 0usize;
            while i < RUNTIME_EVENT_CAPACITY + 4 {
                let _ = emit_runtime_event_with_context(
                    context,
                    ChronoscopeRuntimeEventKind::ViolationObserved,
                    (cpu * 1000 + i) as u64,
                    ChronoscopeEventId::NONE,
                    1,
                    ChronoscopeRuntimePayload::Violation {
                        violation_kind: 1,
                        descriptor_kind: 1,
                        score: 1,
                        flags: 1,
                    },
                );
                i += 1;
            }
            cpu += 1;
        }
        let bundle = build_chronoscope_bundle(&synthetic_temporal_capsule());
        let validation = validate_event_fabric(&bundle.runtime_events);
        assert!(validation.valid);
        assert!(bundle.runtime_events.partial);
        assert!(
            bundle
                .runtime_events
                .per_core
                .iter()
                .take(4)
                .any(|summary| summary.overwritten_events != 0)
        );
        assert!(
            bundle
                .runtime_events
                .per_core
                .iter()
                .take(4)
                .all(|summary| summary.high_water_mark as usize >= RUNTIME_EVENT_CAPACITY)
        );
    }

    #[test]
    fn chronoscope_panic_fallback_does_not_recurse_on_invalid_bundle() {
        let _guard = test_lock();
        let mut bundle = ChronoscopeBundle::EMPTY;
        bundle.valid = true;
        bundle.failure_signature_id = 0xdead_beef;
        bundle.integrity.complete = false;
        bundle.integrity.primary_reason = ChronoscopePartialReason::PanicTruncation;
        emit_chronoscope_panic_fallback(&bundle, ChronoscopePanicMode::PanicCaptureDegraded);
    }

    #[test]
    fn semantic_race_report_skips_invalid_crash_capsule() {
        let _guard = test_lock();
        let _io = serial::lock_test_io();
        reset();
        serial::clear_output();

        emit_semantic_race_report();

        let output =
            String::from_utf8(serial::take_output()).expect("serial output must stay utf8");
        assert!(output.contains("== semantic-race =="));
        assert!(output.contains("semantic-race none reason=no-crash-capsule"));
        assert!(!output.contains("semantic-race likely=true"));
    }
}
