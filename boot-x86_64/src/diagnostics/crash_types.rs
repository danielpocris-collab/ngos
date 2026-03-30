use super::*;

pub const REPLAY_STATE_SLOT_LIMIT: usize = 8;
pub const CAUSAL_LEDGER_LIMIT: usize = 8;
pub const INVARIANT_LIMIT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailurePatternSummary {
    pub valid: bool,
    pub signature_id: u64,
    pub frequency: u32,
    pub last_generation: u64,
    pub last_seen_sequence: u64,
    pub path: DiagnosticsPath,
    pub most_common_stage: u16,
    pub most_common_first_bad_kind: TraceKind,
}

impl FailurePatternSummary {
    pub const EMPTY: Self = Self {
        valid: false,
        signature_id: 0,
        frequency: 0,
        last_generation: 0,
        last_seen_sequence: 0,
        path: DiagnosticsPath::None,
        most_common_stage: 0,
        most_common_first_bad_kind: TraceKind::BootStage,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashHistoryEntry {
    pub valid: bool,
    pub generation: u64,
    pub signature_id: u64,
    pub replay: ReplayCorrelationIds,
    pub window: RequestWindowState,
    pub fault: FaultRecord,
    pub stable_prefix_length: u16,
    pub unstable_suffix_length: u16,
    pub first_divergence_sequence: u64,
    pub first_bad_sequence: u64,
    pub last_good_sequence: u64,
    pub focused_trace: [TraceRecord; FOCUSED_TRACE_HISTORY],
}

impl CrashHistoryEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        generation: 0,
        signature_id: 0,
        replay: ReplayCorrelationIds::EMPTY,
        window: RequestWindowState::EMPTY,
        fault: FaultRecord::EMPTY,
        stable_prefix_length: 0,
        unstable_suffix_length: 0,
        first_divergence_sequence: 0,
        first_bad_sequence: 0,
        last_good_sequence: 0,
        focused_trace: [TraceRecord::EMPTY; FOCUSED_TRACE_HISTORY],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashCapsule {
    pub valid: bool,
    pub generation: u64,
    pub mode: DiagnosticsMode,
    pub failure_signature: FailureSignature,
    pub failure_signature_id: u64,
    pub closest_prior_pattern_id: u64,
    pub stable_prefix_length: u16,
    pub unstable_suffix_length: u16,
    pub first_divergence_sequence: u64,
    pub semantic_reasons: SemanticReasonAggregate,
    pub suspects: [SuspectPoint; SUSPECT_LIMIT],
    pub replay: ReplayCorrelationIds,
    pub fault: FaultRecord,
    pub reprobe: ReprobePolicyState,
    pub window: RequestWindowState,
    pub watch_tail: [ViolationRecord; VIOLATION_TAIL],
    pub trace_tail: [[TraceRecord; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS],
}

impl CrashCapsule {
    pub const EMPTY: Self = Self {
        valid: false,
        generation: 0,
        mode: DiagnosticsMode::Light,
        failure_signature: FailureSignature::EMPTY,
        failure_signature_id: 0,
        closest_prior_pattern_id: 0,
        stable_prefix_length: 0,
        unstable_suffix_length: 0,
        first_divergence_sequence: 0,
        semantic_reasons: SemanticReasonAggregate::EMPTY,
        suspects: [SuspectPoint::EMPTY; SUSPECT_LIMIT],
        replay: ReplayCorrelationIds::EMPTY,
        fault: FaultRecord::EMPTY,
        reprobe: ReprobePolicyState::EMPTY,
        window: RequestWindowState::EMPTY,
        watch_tail: [ViolationRecord::EMPTY; VIOLATION_TAIL],
        trace_tail: [[TraceRecord::EMPTY; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryViolationSummary {
    pub total: u16,
    pub guard_hits: u16,
    pub watch_hits: u16,
    pub repeated_hits: u16,
    pub underrun_hits: u16,
    pub overrun_hits: u16,
    pub interior_hits: u16,
    pub span_hits: u16,
    pub hottest_descriptor_id: u64,
    pub hottest_descriptor_hits: u16,
    pub dominant_overlap: MemoryOverlapClass,
}

impl MemoryViolationSummary {
    pub const EMPTY: Self = Self {
        total: 0,
        guard_hits: 0,
        watch_hits: 0,
        repeated_hits: 0,
        underrun_hits: 0,
        overrun_hits: 0,
        interior_hits: 0,
        span_hits: 0,
        hottest_descriptor_id: 0,
        hottest_descriptor_hits: 0,
        dominant_overlap: MemoryOverlapClass::None,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLineageSummary {
    pub total_versions: u16,
    pub writes: u16,
    pub copies: u16,
    pub zeros: u16,
    pub dmas: u16,
    pub frees: u16,
    pub latest_version_id: u64,
    pub latest_parent_version_id: u64,
    pub latest_digest: u64,
    pub hottest_object_id: u64,
    pub hottest_object_versions: u16,
}

impl MemoryLineageSummary {
    pub const EMPTY: Self = Self {
        total_versions: 0,
        writes: 0,
        copies: 0,
        zeros: 0,
        dmas: 0,
        frees: 0,
        latest_version_id: 0,
        latest_parent_version_id: 0,
        latest_digest: 0,
        hottest_object_id: 0,
        hottest_object_versions: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportMemoryLineageEntry {
    pub valid: bool,
    pub version_id: u64,
    pub parent_version_id: u64,
    pub sequence: u64,
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

impl ExportMemoryLineageEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        version_id: 0,
        parent_version_id: 0,
        sequence: 0,
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
pub struct ExportFocusedEntry {
    pub valid: bool,
    pub sequence: u64,
    pub cpu_slot: u16,
    pub stage: u16,
    pub kind: TraceKind,
    pub path: DiagnosticsPath,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub result: &'static str,
    pub reason: &'static str,
}

impl ExportFocusedEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        sequence: 0,
        cpu_slot: 0,
        stage: 0,
        kind: TraceKind::BootStage,
        path: DiagnosticsPath::None,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        result: "",
        reason: "",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportSuspectEntry {
    pub valid: bool,
    pub rank: u8,
    pub stage: u16,
    pub cpu_slot: u16,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub score: u16,
    pub confidence: u16,
    pub reason: &'static str,
    pub local_violation: &'static str,
}

impl ExportSuspectEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        rank: 0,
        stage: 0,
        cpu_slot: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        score: 0,
        confidence: 0,
        reason: "",
        local_violation: "",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportFailureHistoryEntry {
    pub valid: bool,
    pub generation: u64,
    pub signature_id: u64,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub stage: u16,
    pub path: DiagnosticsPath,
    pub stable_prefix_length: u16,
    pub unstable_suffix_length: u16,
    pub first_bad_sequence: u64,
    pub first_divergence_sequence: u64,
}

impl ExportFailureHistoryEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        generation: 0,
        signature_id: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        stage: 0,
        path: DiagnosticsPath::None,
        stable_prefix_length: 0,
        unstable_suffix_length: 0,
        first_bad_sequence: 0,
        first_divergence_sequence: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportPatternEntry {
    pub valid: bool,
    pub rank: u8,
    pub signature_id: u64,
    pub frequency: u32,
    pub last_generation: u64,
    pub last_seen_sequence: u64,
    pub path: DiagnosticsPath,
    pub stage: u16,
    pub first_bad_kind: TraceKind,
}

impl ExportPatternEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        rank: 0,
        signature_id: 0,
        frequency: 0,
        last_generation: 0,
        last_seen_sequence: 0,
        path: DiagnosticsPath::None,
        stage: 0,
        first_bad_kind: TraceKind::BootStage,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmpDivergenceSummary {
    pub valid: bool,
    pub cpu_a: u16,
    pub cpu_b: u16,
    pub sequence_gap: u64,
    pub stage_a: u16,
    pub stage_b: u16,
    pub path_a: DiagnosticsPath,
    pub path_b: DiagnosticsPath,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
}

impl SmpDivergenceSummary {
    pub const EMPTY: Self = Self {
        valid: false,
        cpu_a: 0,
        cpu_b: 0,
        sequence_gap: 0,
        stage_a: 0,
        stage_b: 0,
        path_a: DiagnosticsPath::None,
        path_b: DiagnosticsPath::None,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmpTimelineCpuSummary {
    pub valid: bool,
    pub cpu_slot: u16,
    pub apic_id: u32,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub event_count: u16,
    pub first_stage: u16,
    pub last_stage: u16,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub dominant_path: DiagnosticsPath,
    pub divergence_suspected: bool,
}

impl SmpTimelineCpuSummary {
    pub const EMPTY: Self = Self {
        valid: false,
        cpu_slot: 0,
        apic_id: 0,
        first_sequence: 0,
        last_sequence: 0,
        event_count: 0,
        first_stage: 0,
        last_stage: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        dominant_path: DiagnosticsPath::None,
        divergence_suspected: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticsExportBundle {
    pub valid: bool,
    pub generation: u64,
    pub failure_signature_id: u64,
    pub stable_prefix_length: u16,
    pub unstable_suffix_length: u16,
    pub first_divergence_sequence: u64,
    pub memory: MemoryViolationSummary,
    pub memory_lineage: MemoryLineageSummary,
    pub focused: [ExportFocusedEntry; FOCUSED_TRACE_HISTORY],
    pub memory_lineage_tail: [ExportMemoryLineageEntry; 8],
    pub suspects: [ExportSuspectEntry; SUSPECT_LIMIT],
    pub failure_history: [ExportFailureHistoryEntry; FAILURE_HISTORY_CAPACITY],
    pub patterns: [ExportPatternEntry; 3],
    pub causal: [CausalLedgerEdge; CAUSAL_LEDGER_LIMIT],
    pub reprobe: ReprobePolicyState,
    pub smp: [SmpTimelineCpuSummary; MAX_TRACE_CPUS],
    pub smp_divergence: SmpDivergenceSummary,
}

impl DiagnosticsExportBundle {
    pub const EMPTY: Self = Self {
        valid: false,
        generation: 0,
        failure_signature_id: 0,
        stable_prefix_length: 0,
        unstable_suffix_length: 0,
        first_divergence_sequence: 0,
        memory: MemoryViolationSummary::EMPTY,
        memory_lineage: MemoryLineageSummary::EMPTY,
        focused: [ExportFocusedEntry::EMPTY; FOCUSED_TRACE_HISTORY],
        memory_lineage_tail: [ExportMemoryLineageEntry::EMPTY; 8],
        suspects: [ExportSuspectEntry::EMPTY; SUSPECT_LIMIT],
        failure_history: [ExportFailureHistoryEntry::EMPTY; FAILURE_HISTORY_CAPACITY],
        patterns: [ExportPatternEntry::EMPTY; 3],
        causal: [CausalLedgerEdge::EMPTY; CAUSAL_LEDGER_LIMIT],
        reprobe: ReprobePolicyState::EMPTY,
        smp: [SmpTimelineCpuSummary::EMPTY; MAX_TRACE_CPUS],
        smp_divergence: SmpDivergenceSummary::EMPTY,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CausalEdgeKind {
    Validation = 1,
    Submit = 2,
    Irq = 3,
    Completion = 4,
    Fault = 5,
    Divergence = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CausalLedgerEdge {
    pub valid: bool,
    pub kind: CausalEdgeKind,
    pub stage: u16,
    pub cpu_slot: u16,
    pub sequence: u64,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub reason: &'static str,
}

impl CausalLedgerEdge {
    pub const EMPTY: Self = Self {
        valid: false,
        kind: CausalEdgeKind::Validation,
        stage: 0,
        cpu_slot: 0,
        sequence: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        reason: "",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarliestPreventableBoundary {
    pub valid: bool,
    pub stage: u16,
    pub path: DiagnosticsPath,
    pub sequence: u64,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub reason: &'static str,
    pub action: &'static str,
}

impl EarliestPreventableBoundary {
    pub const EMPTY: Self = Self {
        valid: false,
        stage: 0,
        path: DiagnosticsPath::None,
        sequence: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        reason: "",
        action: "",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum InvariantStatus {
    Missing = 0,
    Verified = 1,
    Violated = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantCoverageEntry {
    pub name: &'static str,
    pub status: InvariantStatus,
    pub stage: u16,
    pub path: DiagnosticsPath,
    pub reason: &'static str,
}

impl InvariantCoverageEntry {
    pub const EMPTY: Self = Self {
        name: "",
        status: InvariantStatus::Missing,
        stage: 0,
        path: DiagnosticsPath::None,
        reason: "",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantCoverageMap {
    pub entries: [InvariantCoverageEntry; INVARIANT_LIMIT],
}

impl InvariantCoverageMap {
    pub const EMPTY: Self = Self {
        entries: [InvariantCoverageEntry::EMPTY; INVARIANT_LIMIT],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentialFlowReport {
    pub has_baseline: bool,
    pub stable_prefix: u16,
    pub unstable_suffix: u16,
    pub first_divergence_sequence: u64,
    pub baseline_signature_id: u64,
    pub reason: &'static str,
}

impl DifferentialFlowReport {
    pub const EMPTY: Self = Self {
        has_baseline: false,
        stable_prefix: 0,
        unstable_suffix: 0,
        first_divergence_sequence: 0,
        baseline_signature_id: 0,
        reason: "no-baseline",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticRaceSignal {
    pub likely: bool,
    pub score: u16,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub cpu_a: u16,
    pub cpu_b: u16,
    pub reason: &'static str,
}

impl SemanticRaceSignal {
    pub const EMPTY: Self = Self {
        likely: false,
        score: 0,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        cpu_a: 0,
        cpu_b: 0,
        reason: "",
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayWindowSummary {
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub sequence: u64,
    pub stable_prefix: u16,
    pub unstable_suffix: u16,
    pub first_divergence_sequence: u64,
    pub deterministic: bool,
    pub reason: &'static str,
}

impl ReplayWindowSummary {
    pub const EMPTY: Self = Self {
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        sequence: 0,
        stable_prefix: 0,
        unstable_suffix: 0,
        first_divergence_sequence: 0,
        deterministic: false,
        reason: "",
    };
}
