use super::{
    CAUSAL_LEDGER_LIMIT, ChronoscopeEventId, CorrelationKey, DiagnosticsPath,
    FOCUSED_TRACE_HISTORY, MemoryOverlapClass, SUSPECT_LIMIT, VIOLATION_TAIL,
};

pub const CHRONOSCOPE_NODE_LIMIT: usize =
    FOCUSED_TRACE_HISTORY + VIOLATION_TAIL + SUSPECT_LIMIT + 12;
pub const CHRONOSCOPE_EDGE_LIMIT: usize = CAUSAL_LEDGER_LIMIT * 2 + 12;
pub const CHRONOSCOPE_CHECKPOINT_LIMIT: usize = 8;
pub const CHRONOSCOPE_CHECKPOINT_NODE_LIMIT: usize = 4;
pub const CHRONOSCOPE_STATE_FRAGMENT_LIMIT: usize = 4;
pub const CHRONOSCOPE_LINEAGE_LIMIT: usize = 24;
pub const CHRONOSCOPE_LINEAGE_SUMMARY_LIMIT: usize = 4;
pub const CHRONOSCOPE_TEMPORAL_PATH_LIMIT: usize = 8;
pub const CHRONOSCOPE_LAST_WRITER_LIMIT: usize = 16;
pub const CHRONOSCOPE_LAST_WRITER_INDEX_LIMIT: usize = 32;
pub const CHRONOSCOPE_CAPABILITY_LIMIT: usize = 16;
pub const CHRONOSCOPE_CAPABILITY_INDEX_LIMIT: usize = 32;
pub const CHRONOSCOPE_PROPAGATION_LIMIT: usize = 32;
pub const CHRONOSCOPE_RESPONSIBILITY_LIMIT: usize = 8;
pub const CHRONOSCOPE_INVALID_INDEX: u16 = u16::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeNodeKind {
    Observation = 1,
    Interpretation = 2,
    Constraint = 3,
    Boundary = 4,
    Outcome = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeEdgeKind {
    Caused = 1,
    ObservedBefore = 2,
    LeadsTo = 3,
    Violates = 4,
    PreventableAt = 5,
    DivergedFrom = 6,
    Explains = 7,
    Supports = 8,
    CompetesWith = 9,
}

pub type ChronoscopeNodeId = u64;
pub type ChronoscopeStableNodeId = u128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChronoscopeCheckpointId(pub u16);

impl ChronoscopeCheckpointId {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChronoscopeLineageId(pub u16);

impl ChronoscopeLineageId {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(pub u64);

impl CapabilityId {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChronoscopeAnomalyId(pub u16);

impl ChronoscopeAnomalyId {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChronoscopeEscalationId(pub u16);

impl ChronoscopeEscalationId {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeAnomalyKind {
    SchedulerStall = 1,
    RepeatedDivergence = 2,
    ContractStateOscillation = 3,
    ResourceWaitInflation = 4,
    ViolationBurst = 5,
    SuspectPromotionBurst = 6,
    FaultNearPrecursor = 7,
    AbnormalReplayDivergence = 8,
    RepeatedOverwriteHistoryLoss = 9,
    CapabilityMisuseHint = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeEscalationLevel {
    None = 0,
    LocalStandard = 1,
    LocalDeep = 2,
    GlobalStandard = 3,
    GlobalDeep = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeEscalationReason {
    None = 0,
    ViolationBurst = 1,
    DivergenceBurst = 2,
    WaitInflation = 3,
    ContractOscillation = 4,
    OverwritePressure = 5,
    FaultNearPrecursor = 6,
    CapabilityMisuse = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeCaptureWindowKind {
    LocalCore = 1,
    Correlation = 2,
    DomainKey = 3,
    Global = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeAdaptiveState {
    Normal = 0,
    Watching = 1,
    EscalatedLocal = 2,
    EscalatedGlobal = 3,
    CoolingDown = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeAdaptiveTransition {
    pub valid: bool,
    pub from: ChronoscopeAdaptiveState,
    pub to: ChronoscopeAdaptiveState,
    pub reason: ChronoscopeEscalationReason,
    pub at_tick: u64,
    pub anomaly_id: ChronoscopeAnomalyId,
}

impl ChronoscopeAdaptiveTransition {
    pub const EMPTY: Self = Self {
        valid: false,
        from: ChronoscopeAdaptiveState::Normal,
        to: ChronoscopeAdaptiveState::Normal,
        reason: ChronoscopeEscalationReason::None,
        at_tick: 0,
        anomaly_id: ChronoscopeAnomalyId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeAnomalyRecord {
    pub valid: bool,
    pub anomaly_id: ChronoscopeAnomalyId,
    pub kind: ChronoscopeAnomalyKind,
    pub first_event: ChronoscopeEventId,
    pub first_node: ChronoscopeNodeId,
    pub first_checkpoint: ChronoscopeCheckpointId,
    pub domain: Option<ChronoscopeLineageDomain>,
    pub key: u64,
    pub correlation: CorrelationKey,
    pub severity: u8,
    pub confidence_permille: u16,
    pub first_seen_tick: u64,
    pub last_seen_tick: u64,
    pub occurrence_count: u16,
    pub related_core_mask: u16,
    pub escalation_id: ChronoscopeEscalationId,
}

impl ChronoscopeAnomalyRecord {
    pub const EMPTY: Self = Self {
        valid: false,
        anomaly_id: ChronoscopeAnomalyId::NONE,
        kind: ChronoscopeAnomalyKind::SchedulerStall,
        first_event: ChronoscopeEventId::NONE,
        first_node: 0,
        first_checkpoint: ChronoscopeCheckpointId::NONE,
        domain: None,
        key: 0,
        correlation: CorrelationKey {
            request_id: 0,
            completion_id: 0,
            irq_id: 0,
        },
        severity: 0,
        confidence_permille: 0,
        first_seen_tick: 0,
        last_seen_tick: 0,
        occurrence_count: 0,
        related_core_mask: 0,
        escalation_id: ChronoscopeEscalationId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeEscalationRecord {
    pub valid: bool,
    pub escalation_id: ChronoscopeEscalationId,
    pub level: ChronoscopeEscalationLevel,
    pub reason: ChronoscopeEscalationReason,
    pub anomaly_id: ChronoscopeAnomalyId,
    pub target_core_mask: u16,
    pub correlation: CorrelationKey,
    pub domain: Option<ChronoscopeLineageDomain>,
    pub key: u64,
    pub start_tick: u64,
    pub event_budget: u16,
    pub checkpoint_triggered: bool,
    pub replay_bookmark: ChronoscopeCheckpointId,
}

impl ChronoscopeEscalationRecord {
    pub const EMPTY: Self = Self {
        valid: false,
        escalation_id: ChronoscopeEscalationId::NONE,
        level: ChronoscopeEscalationLevel::None,
        reason: ChronoscopeEscalationReason::None,
        anomaly_id: ChronoscopeAnomalyId::NONE,
        target_core_mask: 0,
        correlation: CorrelationKey {
            request_id: 0,
            completion_id: 0,
            irq_id: 0,
        },
        domain: None,
        key: 0,
        start_tick: 0,
        event_budget: 0,
        checkpoint_triggered: false,
        replay_bookmark: ChronoscopeCheckpointId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeCaptureWindow {
    pub valid: bool,
    pub kind: ChronoscopeCaptureWindowKind,
    pub escalation_id: ChronoscopeEscalationId,
    pub anomaly_id: ChronoscopeAnomalyId,
    pub start_event: ChronoscopeEventId,
    pub end_event: ChronoscopeEventId,
    pub start_tick: u64,
    pub end_tick: u64,
    pub target_core_mask: u16,
    pub correlation: CorrelationKey,
    pub domain: Option<ChronoscopeLineageDomain>,
    pub key: u64,
    pub observed_events: u16,
    pub partial_history: bool,
}

impl ChronoscopeCaptureWindow {
    pub const EMPTY: Self = Self {
        valid: false,
        kind: ChronoscopeCaptureWindowKind::Global,
        escalation_id: ChronoscopeEscalationId::NONE,
        anomaly_id: ChronoscopeAnomalyId::NONE,
        start_event: ChronoscopeEventId::NONE,
        end_event: ChronoscopeEventId::NONE,
        start_tick: 0,
        end_tick: 0,
        target_core_mask: 0,
        correlation: CorrelationKey {
            request_id: 0,
            completion_id: 0,
            irq_id: 0,
        },
        domain: None,
        key: 0,
        observed_events: 0,
        partial_history: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeRootCauseCandidate {
    pub valid: bool,
    pub anomaly_id: ChronoscopeAnomalyId,
    pub node_id: ChronoscopeNodeId,
    pub first_bad_transition: ChronoscopeNodeId,
    pub last_safe_checkpoint: ChronoscopeCheckpointId,
    pub last_writer: ChronoscopeNodeId,
    pub linkage_score: u16,
}

impl ChronoscopeRootCauseCandidate {
    pub const EMPTY: Self = Self {
        valid: false,
        anomaly_id: ChronoscopeAnomalyId::NONE,
        node_id: 0,
        first_bad_transition: 0,
        last_safe_checkpoint: ChronoscopeCheckpointId::NONE,
        last_writer: 0,
        linkage_score: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeCandidateSet {
    pub dominant_candidate: ChronoscopeNodeId,
    pub candidates: [ChronoscopeRootCauseCandidate; super::CHRONOSCOPE_CANDIDATE_LIMIT],
}

impl ChronoscopeCandidateSet {
    pub const EMPTY: Self = Self {
        dominant_candidate: 0,
        candidates: [ChronoscopeRootCauseCandidate::EMPTY; super::CHRONOSCOPE_CANDIDATE_LIMIT],
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChronoscopeNode {
    pub valid: bool,
    pub node_id: ChronoscopeNodeId,
    pub stable_id: ChronoscopeStableNodeId,
    pub event_sequence: u64,
    pub kind: ChronoscopeNodeKind,
    pub cpu_slot: u16,
    pub stage: u16,
    pub path: DiagnosticsPath,
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
    pub score: u16,
    pub confidence: f32,
    pub severity: u8,
    pub causal_distance_to_fault: u32,
    pub evidence_count: u32,
}

impl ChronoscopeNode {
    pub const EMPTY: Self = Self {
        valid: false,
        node_id: 0,
        stable_id: 0,
        event_sequence: 0,
        kind: ChronoscopeNodeKind::Observation,
        cpu_slot: 0,
        stage: 0,
        path: DiagnosticsPath::None,
        request_id: 0,
        completion_id: 0,
        irq_id: 0,
        score: 0,
        confidence: 0.0,
        severity: 0,
        causal_distance_to_fault: u32::MAX,
        evidence_count: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeEdge {
    pub valid: bool,
    pub src_node_id: ChronoscopeNodeId,
    pub dst_node_id: ChronoscopeNodeId,
    pub kind: ChronoscopeEdgeKind,
    pub weight: u16,
}

impl ChronoscopeEdge {
    pub const EMPTY: Self = Self {
        valid: false,
        src_node_id: 0,
        dst_node_id: 0,
        kind: ChronoscopeEdgeKind::ObservedBefore,
        weight: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeCheckpointKind {
    FaultAdjacent = 1,
    PreBoundary = 2,
    Divergence = 3,
    LastKnownGood = 4,
    RewindCandidate = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChronoscopeStateFragment {
    None = 0,
    ContractState {
        stage: u16,
        path: DiagnosticsPath,
        key: u64,
        status: u16,
    } = 1,
    ResourceWaiterState {
        stage: u16,
        cpu_slot: u16,
        key: u64,
        waiters: u16,
    } = 2,
    RequestPathState {
        stage: u16,
        path: DiagnosticsPath,
        request_id: u64,
        completion_id: u64,
    } = 3,
    ViolationState {
        stage: u16,
        overlap: MemoryOverlapClass,
        descriptor_id: u64,
        flags: u16,
    } = 4,
    SuspectState {
        stage: u16,
        reason_code: u16,
        event_kind: u16,
        score: u16,
    } = 5,
    DivergenceState {
        stage: u16,
        cpu_a: u16,
        cpu_b: u16,
        sequence: u64,
    } = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeCheckpoint {
    pub valid: bool,
    pub checkpoint_id: ChronoscopeCheckpointId,
    pub stable_id: u128,
    pub kind: ChronoscopeCheckpointKind,
    pub related_nodes: [ChronoscopeNodeId; CHRONOSCOPE_CHECKPOINT_NODE_LIMIT],
    pub correlation: CorrelationKey,
    pub causal_depth: u32,
    pub confidence_permille: u16,
    pub predecessor: ChronoscopeCheckpointId,
    pub replay_start_index: u16,
    pub fragments: [ChronoscopeStateFragment; CHRONOSCOPE_STATE_FRAGMENT_LIMIT],
}

impl ChronoscopeCheckpoint {
    pub const EMPTY: Self = Self {
        valid: false,
        checkpoint_id: ChronoscopeCheckpointId::NONE,
        stable_id: 0,
        kind: ChronoscopeCheckpointKind::FaultAdjacent,
        related_nodes: [0; CHRONOSCOPE_CHECKPOINT_NODE_LIMIT],
        correlation: CorrelationKey {
            request_id: 0,
            completion_id: 0,
            irq_id: 0,
        },
        causal_depth: u32::MAX,
        confidence_permille: 0,
        predecessor: ChronoscopeCheckpointId::NONE,
        replay_start_index: CHRONOSCOPE_INVALID_INDEX,
        fragments: [ChronoscopeStateFragment::None; CHRONOSCOPE_STATE_FRAGMENT_LIMIT],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ChronoscopeLineageDomain {
    ContractState = 1,
    ResourceState = 2,
    RequestPath = 3,
    ViolationState = 4,
    SuspectState = 5,
    CoreDivergenceState = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeLineageRecord {
    pub valid: bool,
    pub lineage_id: ChronoscopeLineageId,
    pub stable_id: u128,
    pub domain: ChronoscopeLineageDomain,
    pub key: u64,
    pub prior_checkpoint: ChronoscopeCheckpointId,
    pub prior_node: ChronoscopeNodeId,
    pub transition_node: ChronoscopeNodeId,
    pub result_checkpoint: ChronoscopeCheckpointId,
    pub result_node: ChronoscopeNodeId,
    pub confidence_permille: u16,
}

impl ChronoscopeLineageRecord {
    pub const EMPTY: Self = Self {
        valid: false,
        lineage_id: ChronoscopeLineageId::NONE,
        stable_id: 0,
        domain: ChronoscopeLineageDomain::RequestPath,
        key: 0,
        prior_checkpoint: ChronoscopeCheckpointId::NONE,
        prior_node: 0,
        transition_node: 0,
        result_checkpoint: ChronoscopeCheckpointId::NONE,
        result_node: 0,
        confidence_permille: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastWriterRecord {
    pub valid: bool,
    pub domain: ChronoscopeLineageDomain,
    pub key: u64,
    pub last_writer_node: ChronoscopeNodeId,
    pub predecessor_writer_node: ChronoscopeNodeId,
    pub event_id: ChronoscopeEventId,
    pub capability_id: CapabilityId,
}

impl LastWriterRecord {
    pub const EMPTY: Self = Self {
        valid: false,
        domain: ChronoscopeLineageDomain::RequestPath,
        key: 0,
        last_writer_node: 0,
        predecessor_writer_node: 0,
        event_id: ChronoscopeEventId::NONE,
        capability_id: CapabilityId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityDerivationRecord {
    pub valid: bool,
    pub parent: CapabilityId,
    pub derived: CapabilityId,
    pub node_id: ChronoscopeNodeId,
    pub event_id: ChronoscopeEventId,
    pub rights_mask: u64,
    pub confidence_permille: u16,
}

impl CapabilityDerivationRecord {
    pub const EMPTY: Self = Self {
        valid: false,
        parent: CapabilityId::NONE,
        derived: CapabilityId::NONE,
        node_id: 0,
        event_id: ChronoscopeEventId::NONE,
        rights_mask: 0,
        confidence_permille: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PropagationType {
    DataFlow = 1,
    ControlFlow = 2,
    CapabilityFlow = 3,
    CausalAmplification = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropagationRecord {
    pub valid: bool,
    pub source_node: ChronoscopeNodeId,
    pub target_node: ChronoscopeNodeId,
    pub propagation_type: PropagationType,
    pub correlation: CorrelationKey,
}

impl PropagationRecord {
    pub const EMPTY: Self = Self {
        valid: false,
        source_node: 0,
        target_node: 0,
        propagation_type: PropagationType::ControlFlow,
        correlation: CorrelationKey {
            request_id: 0,
            completion_id: 0,
            irq_id: 0,
        },
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponsibilityEntry {
    pub valid: bool,
    pub node_id: ChronoscopeNodeId,
    pub score: u16,
}

impl ResponsibilityEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        node_id: 0,
        score: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastWriterIndexEntry {
    pub valid: bool,
    pub domain: ChronoscopeLineageDomain,
    pub key: u64,
    pub record_index: u16,
}

impl LastWriterIndexEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        domain: ChronoscopeLineageDomain::RequestPath,
        key: 0,
        record_index: CHRONOSCOPE_INVALID_INDEX,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityParentIndexEntry {
    pub valid: bool,
    pub capability: CapabilityId,
    pub record_index: u16,
}

impl CapabilityParentIndexEntry {
    pub const EMPTY: Self = Self {
        valid: false,
        capability: CapabilityId::NONE,
        record_index: CHRONOSCOPE_INVALID_INDEX,
    };
}
