#![cfg_attr(not(test), allow(dead_code))]

use alloc::vec::Vec;

use super::{
    CHRONOSCOPE_ANOMALY_LIMIT, CHRONOSCOPE_CAPABILITY_INDEX_LIMIT, CHRONOSCOPE_CAPABILITY_LIMIT,
    CHRONOSCOPE_CAPTURE_WINDOW_LIMIT, CHRONOSCOPE_CHECKPOINT_LIMIT, CHRONOSCOPE_EDGE_LIMIT,
    CHRONOSCOPE_ESCALATION_LIMIT, CHRONOSCOPE_INVALID_INDEX, CHRONOSCOPE_LAST_WRITER_INDEX_LIMIT,
    CHRONOSCOPE_LAST_WRITER_LIMIT, CHRONOSCOPE_LINEAGE_LIMIT, CHRONOSCOPE_LINEAGE_SUMMARY_LIMIT,
    CHRONOSCOPE_NODE_LIMIT, CHRONOSCOPE_PROPAGATION_LIMIT, CHRONOSCOPE_RESPONSIBILITY_LIMIT,
    CHRONOSCOPE_TEMPORAL_PATH_LIMIT, CapabilityDerivationRecord, CapabilityId, CausalEdgeKind,
    ChronoscopeAdaptiveState, ChronoscopeAdaptiveTransition, ChronoscopeAnomalyId,
    ChronoscopeAnomalyRecord, ChronoscopeCandidateSet, ChronoscopeCaptureWindow,
    ChronoscopeCheckpoint, ChronoscopeCheckpointId, ChronoscopeDiffSummary, ChronoscopeEventId,
    ChronoscopeHistoryIntegrity, ChronoscopeLineageDomain, ChronoscopeLineageId,
    ChronoscopeLineageRecord, ChronoscopeNode, ChronoscopeNodeId, ChronoscopePerfCounters,
    ChronoscopeRootCauseCandidate, ChronoscopeRuntimeEventKind, ChronoscopeRuntimeEventWindow,
    ChronoscopeStableNodeId, ChronoscopeTrustSurface, DiagnosticsPath, EarliestPreventableBoundary,
    LastWriterIndexEntry, LastWriterRecord, MAX_TRACE_CPUS, PropagationRecord,
    RUNTIME_EVENT_EXPORT_LIMIT, ResponsibilityEntry, SUSPECT_LIMIT,
};

const REPLAY_STATE_SLOT_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct ChronoscopeBundle {
    pub valid: bool,
    pub generation: u64,
    pub failure_signature_id: u64,
    pub top_suspect_confidence: u16,
    pub root_boundary: EarliestPreventableBoundary,
    pub dominant_suspect_node_id: ChronoscopeNodeId,
    pub strongest_chain: [ChronoscopeNodeId; 8],
    pub nodes: [ChronoscopeNode; CHRONOSCOPE_NODE_LIMIT],
    pub edges: [super::ChronoscopeEdge; CHRONOSCOPE_EDGE_LIMIT],
    pub runtime_events: ChronoscopeRuntimeEventWindow,
    pub checkpoints: [ChronoscopeCheckpoint; CHRONOSCOPE_CHECKPOINT_LIMIT],
    pub lineage: [ChronoscopeLineageRecord; CHRONOSCOPE_LINEAGE_LIMIT],
    pub last_writers: [LastWriterRecord; CHRONOSCOPE_LAST_WRITER_LIMIT],
    pub last_writer_index: [LastWriterIndexEntry; CHRONOSCOPE_LAST_WRITER_INDEX_LIMIT],
    pub writer_predecessor_by_node: [ChronoscopeNodeId; CHRONOSCOPE_NODE_LIMIT],
    pub capability_derivations: [CapabilityDerivationRecord; CHRONOSCOPE_CAPABILITY_LIMIT],
    pub capability_parent_index:
        [super::CapabilityParentIndexEntry; CHRONOSCOPE_CAPABILITY_INDEX_LIMIT],
    pub node_capabilities: [CapabilityId; CHRONOSCOPE_NODE_LIMIT],
    pub propagation: [PropagationRecord; CHRONOSCOPE_PROPAGATION_LIMIT],
    pub propagation_heads: [u16; CHRONOSCOPE_NODE_LIMIT],
    pub propagation_next: [u16; CHRONOSCOPE_PROPAGATION_LIMIT],
    pub responsibility: [ResponsibilityEntry; CHRONOSCOPE_RESPONSIBILITY_LIMIT],
    pub anomalies: [ChronoscopeAnomalyRecord; CHRONOSCOPE_ANOMALY_LIMIT],
    pub escalations: [super::ChronoscopeEscalationRecord; CHRONOSCOPE_ESCALATION_LIMIT],
    pub capture_windows: [ChronoscopeCaptureWindow; CHRONOSCOPE_CAPTURE_WINDOW_LIMIT],
    pub candidates: ChronoscopeCandidateSet,
    pub adaptive_state: ChronoscopeAdaptiveState,
    pub adaptive_transitions:
        [ChronoscopeAdaptiveTransition; super::CHRONOSCOPE_ADAPTIVE_TRANSITION_LIMIT],
    pub integrity: ChronoscopeHistoryIntegrity,
    pub perf: ChronoscopePerfCounters,
    pub trust: ChronoscopeTrustSurface,
    pub temporal_explain: ExplainPlan,
    pub primary_fault_checkpoint: ChronoscopeCheckpointId,
    pub rewind_checkpoint: ChronoscopeCheckpointId,
    pub divergence_checkpoint: ChronoscopeCheckpointId,
}

impl ChronoscopeBundle {
    pub const EMPTY: Self = Self {
        valid: false,
        generation: 0,
        failure_signature_id: 0,
        top_suspect_confidence: 0,
        root_boundary: EarliestPreventableBoundary::EMPTY,
        dominant_suspect_node_id: 0,
        strongest_chain: [0; 8],
        nodes: [ChronoscopeNode::EMPTY; CHRONOSCOPE_NODE_LIMIT],
        edges: [super::ChronoscopeEdge::EMPTY; CHRONOSCOPE_EDGE_LIMIT],
        runtime_events: ChronoscopeRuntimeEventWindow::EMPTY,
        checkpoints: [ChronoscopeCheckpoint::EMPTY; CHRONOSCOPE_CHECKPOINT_LIMIT],
        lineage: [ChronoscopeLineageRecord::EMPTY; CHRONOSCOPE_LINEAGE_LIMIT],
        last_writers: [LastWriterRecord::EMPTY; CHRONOSCOPE_LAST_WRITER_LIMIT],
        last_writer_index: [LastWriterIndexEntry::EMPTY; CHRONOSCOPE_LAST_WRITER_INDEX_LIMIT],
        writer_predecessor_by_node: [0; CHRONOSCOPE_NODE_LIMIT],
        capability_derivations: [CapabilityDerivationRecord::EMPTY; CHRONOSCOPE_CAPABILITY_LIMIT],
        capability_parent_index: [super::CapabilityParentIndexEntry::EMPTY;
            CHRONOSCOPE_CAPABILITY_INDEX_LIMIT],
        node_capabilities: [CapabilityId::NONE; CHRONOSCOPE_NODE_LIMIT],
        propagation: [PropagationRecord::EMPTY; CHRONOSCOPE_PROPAGATION_LIMIT],
        propagation_heads: [CHRONOSCOPE_INVALID_INDEX; CHRONOSCOPE_NODE_LIMIT],
        propagation_next: [CHRONOSCOPE_INVALID_INDEX; CHRONOSCOPE_PROPAGATION_LIMIT],
        responsibility: [ResponsibilityEntry::EMPTY; CHRONOSCOPE_RESPONSIBILITY_LIMIT],
        anomalies: [ChronoscopeAnomalyRecord::EMPTY; CHRONOSCOPE_ANOMALY_LIMIT],
        escalations: [super::ChronoscopeEscalationRecord::EMPTY; CHRONOSCOPE_ESCALATION_LIMIT],
        capture_windows: [ChronoscopeCaptureWindow::EMPTY; CHRONOSCOPE_CAPTURE_WINDOW_LIMIT],
        candidates: ChronoscopeCandidateSet::EMPTY,
        adaptive_state: ChronoscopeAdaptiveState::Normal,
        adaptive_transitions: [ChronoscopeAdaptiveTransition::EMPTY;
            super::CHRONOSCOPE_ADAPTIVE_TRANSITION_LIMIT],
        integrity: ChronoscopeHistoryIntegrity::COMPLETE,
        perf: ChronoscopePerfCounters::EMPTY,
        trust: ChronoscopeTrustSurface::EMPTY,
        temporal_explain: ExplainPlan::EMPTY,
        primary_fault_checkpoint: ChronoscopeCheckpointId::NONE,
        rewind_checkpoint: ChronoscopeCheckpointId::NONE,
        divergence_checkpoint: ChronoscopeCheckpointId::NONE,
    };

    pub fn anomaly_candidates(&self, anomaly_id: ChronoscopeAnomalyId) -> ChronoscopeCandidateSet {
        let mut set = ChronoscopeCandidateSet::EMPTY;
        let mut next = 0usize;
        let mut index = 0usize;
        while index < self.candidates.candidates.len() && next < set.candidates.len() {
            let candidate = self.candidates.candidates[index];
            index += 1;
            if candidate.valid && candidate.anomaly_id == anomaly_id {
                set.candidates[next] = candidate;
                if set.dominant_candidate == 0 {
                    set.dominant_candidate = candidate.node_id;
                }
                next += 1;
            }
        }
        set
    }

    pub fn dominant_candidate(&self) -> Option<ChronoscopeNodeId> {
        if self.candidates.dominant_candidate != 0 {
            Some(self.candidates.dominant_candidate)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorrelationKey {
    pub request_id: u64,
    pub completion_id: u64,
    pub irq_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronoscopeDiff {
    pub summary: ChronoscopeDiffSummary,
    pub common_nodes: [ChronoscopeStableNodeId; CHRONOSCOPE_NODE_LIMIT],
    pub new_nodes: [ChronoscopeStableNodeId; CHRONOSCOPE_NODE_LIMIT],
    pub missing_nodes: [ChronoscopeStableNodeId; CHRONOSCOPE_NODE_LIMIT],
    pub first_divergence_stable_id: ChronoscopeStableNodeId,
    pub changed_path_heads: [ChronoscopeStableNodeId; CHRONOSCOPE_NODE_LIMIT],
    pub common_checkpoints: [u128; CHRONOSCOPE_CHECKPOINT_LIMIT],
    pub new_checkpoints: [u128; CHRONOSCOPE_CHECKPOINT_LIMIT],
    pub missing_checkpoints: [u128; CHRONOSCOPE_CHECKPOINT_LIMIT],
    pub common_lineage: [u128; CHRONOSCOPE_LINEAGE_LIMIT],
    pub new_lineage: [u128; CHRONOSCOPE_LINEAGE_LIMIT],
    pub missing_lineage: [u128; CHRONOSCOPE_LINEAGE_LIMIT],
    pub first_temporal_divergence: u128,
    pub changed_rewind_candidate: bool,
    pub changed_mutation_path: bool,
    pub changed_last_writer: bool,
    pub changed_capability_lineage: bool,
    pub changed_propagation_path: bool,
    pub changed_responsibility_ranking: bool,
    pub changed_adaptive_state: bool,
    pub first_adaptive_divergence: u64,
    pub changed_escalation_target: bool,
}

impl ChronoscopeDiff {
    pub const EMPTY: Self = Self {
        summary: ChronoscopeDiffSummary {
            common_nodes: 0,
            new_nodes: 0,
            missing_nodes: 0,
            changed_paths: 0,
            common_checkpoints: 0,
            new_checkpoints: 0,
            missing_checkpoints: 0,
            common_lineage: 0,
            new_lineage: 0,
            missing_lineage: 0,
            changed_last_writer: 0,
            changed_capability_lineage: 0,
            changed_propagation: 0,
            changed_responsibility: 0,
            changed_anomalies: 0,
            changed_escalations: 0,
            changed_capture_windows: 0,
            changed_candidates: 0,
        },
        common_nodes: [0; CHRONOSCOPE_NODE_LIMIT],
        new_nodes: [0; CHRONOSCOPE_NODE_LIMIT],
        missing_nodes: [0; CHRONOSCOPE_NODE_LIMIT],
        first_divergence_stable_id: 0,
        changed_path_heads: [0; CHRONOSCOPE_NODE_LIMIT],
        common_checkpoints: [0; CHRONOSCOPE_CHECKPOINT_LIMIT],
        new_checkpoints: [0; CHRONOSCOPE_CHECKPOINT_LIMIT],
        missing_checkpoints: [0; CHRONOSCOPE_CHECKPOINT_LIMIT],
        common_lineage: [0; CHRONOSCOPE_LINEAGE_LIMIT],
        new_lineage: [0; CHRONOSCOPE_LINEAGE_LIMIT],
        missing_lineage: [0; CHRONOSCOPE_LINEAGE_LIMIT],
        first_temporal_divergence: 0,
        changed_rewind_candidate: false,
        changed_mutation_path: false,
        changed_last_writer: false,
        changed_capability_lineage: false,
        changed_propagation_path: false,
        changed_responsibility_ranking: false,
        changed_adaptive_state: false,
        first_adaptive_divergence: 0,
        changed_escalation_target: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeDiffEntry {
    pub stable_id: ChronoscopeStableNodeId,
    pub path: DiagnosticsPath,
}

impl ChronoscopeDiffEntry {
    pub const EMPTY: Self = Self {
        stable_id: 0,
        path: DiagnosticsPath::None,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeTemporalDiffEntry {
    pub stable_id: u128,
}

impl ChronoscopeTemporalDiffEntry {
    pub const EMPTY: Self = Self { stable_id: 0 };
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExplainPlan {
    pub valid: bool,
    pub primary_cause: ChronoscopeNodeId,
    pub fault_node: ChronoscopeNodeId,
    pub earliest_preventable_boundary: ChronoscopeNodeId,
    pub causal_chain: [ChronoscopeNodeId; 8],
    pub competing_suspects: [ChronoscopeNodeId; SUSPECT_LIMIT],
    pub cross_core_divergence: [ChronoscopeNodeId; 4],
    pub confidence: f32,
    pub state_before_fault: ChronoscopeCheckpointId,
    pub last_mutation: ChronoscopeNodeId,
    pub rewind_candidate: ChronoscopeCheckpointId,
    pub divergence_origin: ChronoscopeCheckpointId,
    pub lineage_summary: [ChronoscopeLineageId; CHRONOSCOPE_LINEAGE_SUMMARY_LIMIT],
    pub temporal_confidence: u16,
    pub last_writer: ChronoscopeNodeId,
    pub writer_chain: [ChronoscopeNodeId; CHRONOSCOPE_TEMPORAL_PATH_LIMIT],
    pub capability_chain: [CapabilityId; CHRONOSCOPE_LINEAGE_SUMMARY_LIMIT],
    pub propagation_path: [ChronoscopeNodeId; CHRONOSCOPE_TEMPORAL_PATH_LIMIT],
    pub responsibility_ranking: [ResponsibilityEntry; 3],
    pub responsibility_confidence: u16,
    pub replay_summary: ReplaySummary,
    pub replay_steps_count: u16,
    pub first_divergence_point: ChronoscopeEventId,
    pub replay_confidence: u16,
    pub dominant_anomaly: ChronoscopeAnomalyId,
    pub adaptive_state: ChronoscopeAdaptiveState,
    pub escalation_id: super::ChronoscopeEscalationId,
    pub candidate_node: ChronoscopeNodeId,
    pub downgrade_ready: bool,
    pub anomaly_confidence: u16,
}

impl ExplainPlan {
    pub const EMPTY: Self = Self {
        valid: false,
        primary_cause: 0,
        fault_node: 0,
        earliest_preventable_boundary: 0,
        causal_chain: [0; 8],
        competing_suspects: [0; SUSPECT_LIMIT],
        cross_core_divergence: [0; 4],
        confidence: 0.0,
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
        escalation_id: super::ChronoscopeEscalationId::NONE,
        candidate_node: 0,
        downgrade_ready: false,
        anomaly_confidence: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayEvent {
    pub valid: bool,
    pub event_id: ChronoscopeEventId,
    pub core_id: u16,
    pub local_sequence: u64,
    pub kind: ChronoscopeRuntimeEventKind,
    pub correlation: CorrelationKey,
    pub capability_id: CapabilityId,
    pub object_key: u64,
    pub causal_parent: ChronoscopeEventId,
    pub flags: u16,
}

impl ReplayEvent {
    pub const EMPTY: Self = Self {
        valid: false,
        event_id: ChronoscopeEventId::NONE,
        core_id: 0,
        local_sequence: 0,
        kind: ChronoscopeRuntimeEventKind::FocusedTraceMarker,
        correlation: CorrelationKey {
            request_id: 0,
            completion_id: 0,
            irq_id: 0,
        },
        capability_id: CapabilityId::NONE,
        object_key: 0,
        causal_parent: ChronoscopeEventId::NONE,
        flags: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaySequence {
    pub valid: bool,
    pub total_events: u16,
    pub per_core_counts: [u16; MAX_TRACE_CPUS],
    pub events: [ReplayEvent; RUNTIME_EVENT_EXPORT_LIMIT],
}

impl ReplaySequence {
    pub const EMPTY: Self = Self {
        valid: false,
        total_events: 0,
        per_core_counts: [0; MAX_TRACE_CPUS],
        events: [ReplayEvent::EMPTY; RUNTIME_EVENT_EXPORT_LIMIT],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayContractState {
    pub valid: bool,
    pub key: u64,
    pub stage: u16,
    pub status: u16,
}

impl ReplayContractState {
    pub const EMPTY: Self = Self {
        valid: false,
        key: 0,
        stage: 0,
        status: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayResourceState {
    pub valid: bool,
    pub key: u64,
    pub owner: u16,
    pub state: u16,
}

impl ReplayResourceState {
    pub const EMPTY: Self = Self {
        valid: false,
        key: 0,
        owner: 0,
        state: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayRequestState {
    pub valid: bool,
    pub request_id: u64,
    pub phase: u16,
    pub completion_id: u64,
}

impl ReplayRequestState {
    pub const EMPTY: Self = Self {
        valid: false,
        request_id: 0,
        phase: 0,
        completion_id: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaySemanticState {
    pub contracts: [ReplayContractState; REPLAY_STATE_SLOT_LIMIT],
    pub resources: [ReplayResourceState; REPLAY_STATE_SLOT_LIMIT],
    pub requests: [ReplayRequestState; REPLAY_STATE_SLOT_LIMIT],
    pub violation_flags: u16,
    pub suspect_flags: u16,
    pub last_writer: ChronoscopeNodeId,
    pub last_capability: CapabilityId,
}

impl ReplaySemanticState {
    pub const EMPTY: Self = Self {
        contracts: [ReplayContractState::EMPTY; REPLAY_STATE_SLOT_LIMIT],
        resources: [ReplayResourceState::EMPTY; REPLAY_STATE_SLOT_LIMIT],
        requests: [ReplayRequestState::EMPTY; REPLAY_STATE_SLOT_LIMIT],
        violation_flags: 0,
        suspect_flags: 0,
        last_writer: 0,
        last_capability: CapabilityId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayCursor {
    pub valid: bool,
    pub sequence: ReplaySequence,
    pub global_position: u16,
    pub per_core_positions: [u16; MAX_TRACE_CPUS],
    pub state: ReplaySemanticState,
    pub checkpoint_id: ChronoscopeCheckpointId,
}

impl ReplayCursor {
    pub const EMPTY: Self = Self {
        valid: false,
        sequence: ReplaySequence::EMPTY,
        global_position: 0,
        per_core_positions: [0; MAX_TRACE_CPUS],
        state: ReplaySemanticState::EMPTY,
        checkpoint_id: ChronoscopeCheckpointId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayResult {
    pub valid: bool,
    pub last_event: ReplayEvent,
    pub steps: u16,
    pub state: ReplaySemanticState,
}

impl ReplayResult {
    pub const EMPTY: Self = Self {
        valid: false,
        last_event: ReplayEvent::EMPTY,
        steps: 0,
        state: ReplaySemanticState::EMPTY,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayTrace {
    pub valid: bool,
    pub partial: bool,
    pub total_steps: u16,
    pub events: [ReplayEvent; RUNTIME_EVENT_EXPORT_LIMIT],
    pub final_state: ReplaySemanticState,
}

impl ReplayTrace {
    pub const EMPTY: Self = Self {
        valid: false,
        partial: false,
        total_steps: 0,
        events: [ReplayEvent::EMPTY; RUNTIME_EVENT_EXPORT_LIMIT],
        final_state: ReplaySemanticState::EMPTY,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivergenceResult {
    pub valid: bool,
    pub first_event_mismatch: ChronoscopeEventId,
    pub first_state_mismatch: u16,
    pub first_violation_mismatch: u16,
}

impl DivergenceResult {
    pub const EMPTY: Self = Self {
        valid: false,
        first_event_mismatch: ChronoscopeEventId::NONE,
        first_state_mismatch: CHRONOSCOPE_INVALID_INDEX,
        first_violation_mismatch: CHRONOSCOPE_INVALID_INDEX,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaySummary {
    pub valid: bool,
    pub start_checkpoint: ChronoscopeCheckpointId,
    pub steps_to_fault: u16,
    pub deterministic: bool,
    pub partial: bool,
    pub divergence_point: ChronoscopeEventId,
}

impl ReplaySummary {
    pub const EMPTY: Self = Self {
        valid: false,
        start_checkpoint: ChronoscopeCheckpointId::NONE,
        steps_to_fault: 0,
        deterministic: false,
        partial: false,
        divergence_point: ChronoscopeEventId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChronoscopeQueryKind {
    PathToFault,
    PathFromNodeToFault,
    ExplainFault,
    ExplainNode,
    LastWriter,
    WriterChain,
    LineageNode,
    LineageDomain,
    CapabilityOrigin,
    CapabilityChain,
    CapabilityUsage,
    CheckpointBeforeFault,
    RewindCandidate,
    ReplayFromCheckpointToFault,
    ReplayUntilViolation,
    DivergenceOrigin,
    DiffFirstDivergence,
    DiffResponsibility,
    ResponsibilityTop,
    ResponsibilityNode,
    PropagationPathToFault,
    AnomaliesTop,
    AnomalyById,
    AdaptiveState,
    CaptureWindowActive,
    CaptureWindowRecent,
    EscalationHistory,
    CandidateTop,
    CandidateForAnomaly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronoscopeQuery {
    pub kind: ChronoscopeQueryKind,
    pub node_id: ChronoscopeNodeId,
    pub checkpoint_id: ChronoscopeCheckpointId,
    pub capability_id: CapabilityId,
    pub domain: Option<ChronoscopeLineageDomain>,
    pub key: u64,
    pub limit: u16,
    pub anomaly_id: ChronoscopeAnomalyId,
}

impl ChronoscopeQuery {
    pub const EMPTY: Self = Self {
        kind: ChronoscopeQueryKind::ExplainFault,
        node_id: 0,
        checkpoint_id: ChronoscopeCheckpointId::NONE,
        capability_id: CapabilityId::NONE,
        domain: None,
        key: 0,
        limit: 0,
        anomaly_id: ChronoscopeAnomalyId::NONE,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeQueryParseError {
    pub token_index: u8,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChronoscopeQueryResultKind {
    Empty,
    Scalar,
    NodeList,
    CapabilityList,
    CheckpointList,
    ReplaySummary,
    DiffSummary,
    ExplainSummary,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeQueryRow {
    pub key: &'static str,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChronoscopeQueryPath {
    pub nodes: Vec<ChronoscopeNodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChronoscopeQueryExplain {
    pub plan: ExplainPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeQueryReplaySummary {
    pub steps: u16,
    pub checkpoint: ChronoscopeCheckpointId,
    pub last_event: ChronoscopeEventId,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChronoscopeQueryResult {
    pub kind: ChronoscopeQueryResultKind,
    pub rows: Vec<ChronoscopeQueryRow>,
    pub nodes: Vec<ChronoscopeNodeId>,
    pub capabilities: Vec<CapabilityId>,
    pub checkpoints: Vec<ChronoscopeCheckpointId>,
    pub replay: Option<ChronoscopeQueryReplaySummary>,
    pub diff: Option<ChronoscopeDiff>,
    pub explain: Option<ChronoscopeQueryExplain>,
    pub path: Option<ChronoscopeQueryPath>,
    pub error: Option<ChronoscopeQueryParseError>,
}

impl ChronoscopeQueryResult {
    pub fn empty() -> Self {
        Self {
            kind: ChronoscopeQueryResultKind::Empty,
            rows: Vec::new(),
            nodes: Vec::new(),
            capabilities: Vec::new(),
            checkpoints: Vec::new(),
            replay: None,
            diff: None,
            explain: None,
            path: None,
            error: None,
        }
    }

    pub fn parse_error(token_index: u8, reason: &'static str) -> Self {
        let mut result = Self::empty();
        result.kind = ChronoscopeQueryResultKind::Error;
        result.error = Some(ChronoscopeQueryParseError {
            token_index,
            reason,
        });
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChronoscopeNodeList<const N: usize> {
    pub len: usize,
    pub nodes: [ChronoscopeNodeId; N],
}

impl<const N: usize> ChronoscopeNodeList<N> {
    pub const EMPTY: Self = Self {
        len: 0,
        nodes: [0; N],
    };

    pub fn push(&mut self, node_id: ChronoscopeNodeId) {
        if node_id == 0 || self.len >= N {
            return;
        }
        self.nodes[self.len] = node_id;
        self.len += 1;
    }
}

const _: fn() = || {
    let _ = ChronoscopeNodeList::<8>::EMPTY;
    let _ = CausalEdgeKind::Validation;
    let _ = ChronoscopeRootCauseCandidate::EMPTY;
};
