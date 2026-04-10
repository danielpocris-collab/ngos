use kernel_core::{
    ResourceAgentDecisionRecord, ResourceAgentKind, VmAgentDecisionRecord, VmAgentKind,
};
use ngos_boot_x86_64::diagnostics::{
    ChronoscopeBundle, ChronoscopeCheckpointId, ChronoscopeCompletenessFlags, ChronoscopeNode,
    ChronoscopeNodeId, ChronoscopeNodeKind, ChronoscopePartialReason, ChronoscopeTrustSurface,
};

const CHRONOSCOPE_OPERATOR_PATH_LIMIT: usize = 6;
const RESOURCE_AGENT_REPORT_LIMIT: usize = 8;
const VM_AGENT_REPORT_LIMIT: usize = 8;
const VM_EPISODE_REPORT_LIMIT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChronoscopeOperatorReplayStatus {
    Unknown,
    Full,
    Partial,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronoscopeOperatorSummary {
    pub available: bool,
    pub cause_node_id: ChronoscopeNodeId,
    pub cause_kind: Option<ChronoscopeNodeKind>,
    pub responsible_node_id: ChronoscopeNodeId,
    pub last_writer_node_id: ChronoscopeNodeId,
    pub rewind_checkpoint_id: Option<ChronoscopeCheckpointId>,
    pub divergence_checkpoint_id: Option<ChronoscopeCheckpointId>,
    pub propagation: [ChronoscopeNodeId; CHRONOSCOPE_OPERATOR_PATH_LIMIT],
    pub propagation_len: usize,
    pub propagation_truncated: bool,
    pub trust_complete: Option<bool>,
    pub trust_reason: ChronoscopePartialReason,
    pub trust_flags: ChronoscopeCompletenessFlags,
    pub replay_status: ChronoscopeOperatorReplayStatus,
    pub responsibility_confidence: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceAgentReportSummary {
    pub total_decisions: usize,
    pub entries: [Option<ResourceAgentDecisionRecord>; RESOURCE_AGENT_REPORT_LIMIT],
    pub entry_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmAgentReportSummary {
    pub total_decisions: usize,
    pub entries: [Option<VmAgentDecisionRecord>; VM_AGENT_REPORT_LIMIT],
    pub entry_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmEpisodeKind {
    MapPath,
    HeapPath,
    ReclaimPath,
    FaultPath,
    Quarantine,
    RegionPath,
    PolicyPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmEpisodeReportEntry {
    pub kind: VmEpisodeKind,
    pub pid: u64,
    pub vm_object_id: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    pub quarantine_reason: u64,
    pub blocked: bool,
    pub released: bool,
    pub mapped_kind: u64,
    pub old_end: u64,
    pub new_end: u64,
    pub grew: bool,
    pub shrank: bool,
    pub evicted: bool,
    pub restored: bool,
    pub protected: bool,
    pub unmapped: bool,
    pub policy_blocked: bool,
    pub policy_state: u64,
    pub policy_operation: u64,
    pub decision_count: u32,
    pub last_agent: VmAgentKind,
    pub faulted: bool,
    pub cow: bool,
    pub bridged: bool,
    pub touched: bool,
    pub synced: bool,
    pub advised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmEpisodeReportSummary {
    pub total_episodes: usize,
    pub entries: [Option<VmEpisodeReportEntry>; VM_EPISODE_REPORT_LIMIT],
    pub entry_count: usize,
    pub truncated: bool,
}

impl ResourceAgentReportSummary {
    pub const EMPTY: Self = Self {
        total_decisions: 0,
        entries: [None; RESOURCE_AGENT_REPORT_LIMIT],
        entry_count: 0,
        truncated: false,
    };
}

impl VmAgentReportSummary {
    pub const EMPTY: Self = Self {
        total_decisions: 0,
        entries: [None; VM_AGENT_REPORT_LIMIT],
        entry_count: 0,
        truncated: false,
    };
}

impl VmEpisodeReportSummary {
    pub const EMPTY: Self = Self {
        total_episodes: 0,
        entries: [None; VM_EPISODE_REPORT_LIMIT],
        entry_count: 0,
        truncated: false,
    };
}

impl ChronoscopeOperatorSummary {
    pub const EMPTY: Self = Self {
        available: false,
        cause_node_id: 0,
        cause_kind: None,
        responsible_node_id: 0,
        last_writer_node_id: 0,
        rewind_checkpoint_id: None,
        divergence_checkpoint_id: None,
        propagation: [0; CHRONOSCOPE_OPERATOR_PATH_LIMIT],
        propagation_len: 0,
        propagation_truncated: false,
        trust_complete: None,
        trust_reason: ChronoscopePartialReason::None,
        trust_flags: ChronoscopeCompletenessFlags::EMPTY,
        replay_status: ChronoscopeOperatorReplayStatus::Unknown,
        responsibility_confidence: 0,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRuntimeNativeSessionReport {
    pub pid: u64,
    pub exit_code: i32,
    pub stdout_bytes: usize,
    pub ui_presentation_backend: &'static str,
    pub session_reported: bool,
    pub session_report_count: u64,
    pub session_status: u32,
    pub session_stage: u32,
    pub session_code: i32,
    pub session_detail: u64,
    pub domain_count: usize,
    pub resource_count: usize,
    pub contract_count: usize,
    pub chronoscope: ChronoscopeOperatorSummary,
    pub resource_agents: ResourceAgentReportSummary,
    pub vm_agents: VmAgentReportSummary,
    pub vm_episodes: VmEpisodeReportSummary,
    pub stdout: String,
}

impl HostRuntimeNativeSessionReport {
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("[native-session]\n");
        out.push_str(&format!(
            "process: pid={} exit-code={} stdout-bytes={} domains={} resources={} contracts={}\n",
            self.pid,
            self.exit_code,
            self.stdout_bytes,
            self.domain_count,
            self.resource_count,
            self.contract_count,
        ));
        out.push_str(&format!(
            "presentation: ui-backend={}\n",
            self.ui_presentation_backend,
        ));
        out.push_str(&format!(
            "session: reported={} count={} status={} stage={} code={} detail={}\n",
            self.session_reported,
            self.session_report_count,
            self.session_status,
            self.session_stage,
            self.session_code,
            self.session_detail,
        ));
        render_chronoscope_operator_summary(&mut out, &self.chronoscope);
        render_resource_agent_summary(&mut out, &self.resource_agents);
        render_vm_agent_summary(&mut out, &self.vm_agents);
        render_vm_episode_summary(&mut out, &self.vm_episodes);
        out.push_str("stdout:\n");
        out.push_str(&self.stdout);
        if !self.stdout.ends_with('\n') {
            out.push('\n');
        }
        out
    }
}

pub fn extract_resource_agent_report_summary(
    decisions: &[ResourceAgentDecisionRecord],
) -> ResourceAgentReportSummary {
    let mut summary = ResourceAgentReportSummary {
        total_decisions: decisions.len(),
        truncated: decisions.len() > RESOURCE_AGENT_REPORT_LIMIT,
        ..ResourceAgentReportSummary::EMPTY
    };
    let start = decisions.len().saturating_sub(RESOURCE_AGENT_REPORT_LIMIT);
    let mut index = start;
    while index < decisions.len() {
        summary.entries[summary.entry_count] = Some(decisions[index]);
        summary.entry_count += 1;
        index += 1;
    }
    summary
}

pub fn extract_vm_agent_report_summary(
    decisions: &[VmAgentDecisionRecord],
) -> VmAgentReportSummary {
    let mut summary = VmAgentReportSummary {
        total_decisions: decisions.len(),
        truncated: decisions.len() > VM_AGENT_REPORT_LIMIT,
        ..VmAgentReportSummary::EMPTY
    };
    let mut selected = [usize::MAX; VM_AGENT_REPORT_LIMIT];
    let mut selected_count = 0usize;
    for (index, entry) in decisions.iter().enumerate() {
        if matches!(
            entry.agent,
            VmAgentKind::QuarantineStateAgent
                | VmAgentKind::QuarantineBlockAgent
                | VmAgentKind::PolicyBlockAgent
                | VmAgentKind::PressureTriggerAgent
                | VmAgentKind::PressureVictimAgent
        ) && selected_count < VM_AGENT_REPORT_LIMIT
        {
            selected[selected_count] = index;
            selected_count += 1;
        }
    }
    let start = decisions.len().saturating_sub(VM_AGENT_REPORT_LIMIT);
    let mut index = start;
    while index < decisions.len() && selected_count < VM_AGENT_REPORT_LIMIT {
        if !selected[..selected_count].contains(&index) {
            selected[selected_count] = index;
            selected_count += 1;
        }
        index += 1;
    }
    selected[..selected_count].sort_unstable();
    let mut slot = 0usize;
    while slot < selected_count {
        summary.entries[summary.entry_count] = Some(decisions[selected[slot]]);
        summary.entry_count += 1;
        slot += 1;
    }
    summary
}

pub fn extract_vm_episode_report_summary(
    decisions: &[VmAgentDecisionRecord],
) -> VmEpisodeReportSummary {
    let episodes = build_vm_episode_entries(decisions);
    let mut summary = VmEpisodeReportSummary {
        total_episodes: episodes.len(),
        truncated: episodes.len() > VM_EPISODE_REPORT_LIMIT,
        ..VmEpisodeReportSummary::EMPTY
    };
    let mut selected = Vec::with_capacity(VM_EPISODE_REPORT_LIMIT);
    let mut index = 0usize;
    while index < episodes.len() {
        if matches!(
            episodes[index].kind,
            VmEpisodeKind::MapPath
                | VmEpisodeKind::HeapPath
                | VmEpisodeKind::ReclaimPath
                | VmEpisodeKind::PolicyPath
        ) {
            selected.push(index);
            if selected.len() == VM_EPISODE_REPORT_LIMIT {
                break;
            }
        }
        index += 1;
    }
    let start = episodes.len().saturating_sub(VM_EPISODE_REPORT_LIMIT);
    index = start;
    while index < episodes.len() && selected.len() < VM_EPISODE_REPORT_LIMIT {
        if !selected.contains(&index) {
            selected.push(index);
        }
        index += 1;
    }
    selected.sort_unstable();
    index = 0;
    while index < selected.len() {
        summary.entries[summary.entry_count] = Some(episodes[selected[index]]);
        summary.entry_count += 1;
        index += 1;
    }
    summary
}

pub fn extract_chronoscope_summary(bundle: &ChronoscopeBundle) -> ChronoscopeOperatorSummary {
    let mut summary = ChronoscopeOperatorSummary {
        available: bundle.valid,
        trust_complete: Some(bundle.trust.completeness.complete),
        trust_reason: bundle.trust.completeness.primary_reason,
        trust_flags: bundle.trust.completeness.flags,
        replay_status: replay_status_for(&bundle.trust, bundle),
        responsibility_confidence: bundle.temporal_explain.responsibility_confidence,
        ..ChronoscopeOperatorSummary::EMPTY
    };

    if !bundle.valid {
        return summary;
    }

    let cause_node_id = if bundle.temporal_explain.fault_node != 0 {
        bundle.temporal_explain.fault_node
    } else {
        bundle.primary_fault_node()
    };
    summary.cause_node_id = cause_node_id;
    summary.cause_kind = lookup_node(bundle, cause_node_id).map(|node| node.kind);
    summary.responsible_node_id = bundle.primary_responsible_node();
    summary.last_writer_node_id = bundle.temporal_explain.last_writer;
    summary.rewind_checkpoint_id = bundle.rewind_candidate();
    summary.divergence_checkpoint_id = bundle.divergence_origin();

    let mut source_path = bundle.temporal_explain.propagation_path;
    let mut source_len = 0usize;
    let mut source_truncated = false;
    if source_path.iter().all(|node_id| *node_id == 0) && summary.responsible_node_id != 0 {
        let chain = bundle.propagation_chain_to_fault(summary.responsible_node_id);
        source_len = chain.len();
        source_truncated = source_len > source_path.len();
        let mut index = 0usize;
        while index < source_path.len() && index < chain.len() {
            source_path[index] = chain[index];
            index += 1;
        }
    } else {
        while source_len < source_path.len() && source_path[source_len] != 0 {
            source_len += 1;
        }
    }
    summary.propagation_truncated =
        source_truncated || source_len > CHRONOSCOPE_OPERATOR_PATH_LIMIT;
    while summary.propagation_len < CHRONOSCOPE_OPERATOR_PATH_LIMIT
        && summary.propagation_len < source_len
    {
        summary.propagation[summary.propagation_len] = source_path[summary.propagation_len];
        summary.propagation_len += 1;
    }

    summary
}

fn lookup_node(bundle: &ChronoscopeBundle, node_id: ChronoscopeNodeId) -> Option<ChronoscopeNode> {
    if node_id == 0 {
        return None;
    }
    let mut index = 0usize;
    while index < bundle.nodes.len() {
        let node = bundle.nodes[index];
        if node.valid && node.node_id == node_id {
            return Some(node);
        }
        index += 1;
    }
    None
}

fn replay_status_for(
    trust: &ChronoscopeTrustSurface,
    bundle: &ChronoscopeBundle,
) -> ChronoscopeOperatorReplayStatus {
    let replay = bundle.temporal_explain.replay_summary;
    if !replay.valid {
        return ChronoscopeOperatorReplayStatus::Unknown;
    }
    if replay.partial || trust.replay_partial {
        return ChronoscopeOperatorReplayStatus::Partial;
    }
    if replay.deterministic {
        return ChronoscopeOperatorReplayStatus::Full;
    }
    ChronoscopeOperatorReplayStatus::Degraded
}

fn render_chronoscope_operator_summary(out: &mut String, summary: &ChronoscopeOperatorSummary) {
    out.push_str("== chronoscope ==\n");
    // CHRONOSCOPE_OPERATOR_FORMAT_V1
    out.push_str("cause: ");
    render_cause(out, summary);
    out.push('\n');
    out.push_str("responsible: ");
    render_node_id(out, summary.responsible_node_id);
    out.push('\n');
    out.push_str("last_writer: ");
    render_node_id(out, summary.last_writer_node_id);
    out.push('\n');
    out.push_str("rewind: ");
    render_checkpoint_id(out, summary.rewind_checkpoint_id);
    out.push('\n');
    out.push_str("divergence: ");
    render_checkpoint_id(out, summary.divergence_checkpoint_id);
    out.push('\n');
    out.push_str("propagation: ");
    render_propagation(out, summary);
    out.push('\n');
    out.push_str("trust: ");
    render_trust(out, summary);
    out.push('\n');
    out.push_str("replay: ");
    out.push_str(match summary.replay_status {
        ChronoscopeOperatorReplayStatus::Unknown => "unknown",
        ChronoscopeOperatorReplayStatus::Full => "full",
        ChronoscopeOperatorReplayStatus::Partial => "partial",
        ChronoscopeOperatorReplayStatus::Degraded => "degraded",
    });
    out.push('\n');
    out.push_str(&format!(
        "confidence: {:.2}\n",
        f32::from(summary.responsibility_confidence) / 100.0
    ));
}

fn render_resource_agent_summary(out: &mut String, summary: &ResourceAgentReportSummary) {
    out.push_str("== resource-agents ==\n");
    // RESOURCE_AGENT_REPORT_FORMAT_V1
    out.push_str("decisions: ");
    out.push_str(&summary.total_decisions.to_string());
    if summary.total_decisions == 0 {
        out.push_str(" none\n");
        return;
    }
    if summary.truncated {
        out.push_str(" truncated\n");
    } else {
        out.push('\n');
    }
    let mut index = 0usize;
    while index < summary.entry_count {
        let Some(entry) = summary.entries[index] else {
            index += 1;
            continue;
        };
        out.push_str("- tick=");
        out.push_str(&entry.tick.to_string());
        out.push_str(" agent=");
        out.push_str(resource_agent_kind_label(entry.agent));
        out.push_str(" resource=");
        out.push_str(&entry.resource.to_string());
        out.push_str(" contract=");
        if entry.contract == 0 {
            out.push_str("none");
        } else {
            out.push_str(&entry.contract.to_string());
        }
        out.push_str(" detail0=");
        out.push_str(&entry.detail0.to_string());
        out.push_str(" detail1=");
        out.push_str(&entry.detail1.to_string());
        out.push('\n');
        index += 1;
    }
}

fn render_vm_agent_summary(out: &mut String, summary: &VmAgentReportSummary) {
    out.push_str("== vm-agents ==\n");
    // VM_AGENT_REPORT_FORMAT_V1
    out.push_str("decisions: ");
    out.push_str(&summary.total_decisions.to_string());
    if summary.total_decisions == 0 {
        out.push_str(" none\n");
        return;
    }
    if summary.truncated {
        out.push_str(" truncated\n");
    } else {
        out.push('\n');
    }
    let mut index = 0usize;
    while index < summary.entry_count {
        let Some(entry) = summary.entries[index] else {
            index += 1;
            continue;
        };
        out.push_str("- tick=");
        out.push_str(&entry.tick.to_string());
        out.push_str(" agent=");
        out.push_str(vm_agent_kind_label(entry.agent));
        out.push_str(" pid=");
        out.push_str(&entry.pid.to_string());
        out.push_str(" vm_object=");
        out.push_str(&entry.vm_object_id.to_string());
        out.push_str(" start=");
        out.push_str(&entry.start.to_string());
        out.push_str(" len=");
        out.push_str(&entry.length.to_string());
        out.push_str(" detail0=");
        out.push_str(&entry.detail0.to_string());
        out.push_str(" detail1=");
        out.push_str(&entry.detail1.to_string());
        out.push('\n');
        index += 1;
    }
}

fn render_vm_episode_summary(out: &mut String, summary: &VmEpisodeReportSummary) {
    out.push_str("== vm-episodes ==\n");
    // VM_EPISODE_REPORT_FORMAT_V1
    out.push_str("episodes: ");
    out.push_str(&summary.total_episodes.to_string());
    if summary.total_episodes == 0 {
        out.push_str(" none\n");
        return;
    }
    if summary.truncated {
        out.push_str(" truncated\n");
    } else {
        out.push('\n');
    }
    let mut index = 0usize;
    while index < summary.entry_count {
        let Some(entry) = summary.entries[index] else {
            index += 1;
            continue;
        };
        out.push_str("- pid=");
        out.push_str(&entry.pid.to_string());
        out.push_str(" kind=");
        out.push_str(match entry.kind {
            VmEpisodeKind::MapPath => "map",
            VmEpisodeKind::HeapPath => "heap",
            VmEpisodeKind::ReclaimPath => "reclaim",
            VmEpisodeKind::FaultPath => "fault",
            VmEpisodeKind::Quarantine => "quarantine",
            VmEpisodeKind::RegionPath => "region",
            VmEpisodeKind::PolicyPath => "policy",
        });
        out.push_str(" vm_object=");
        out.push_str(&entry.vm_object_id.to_string());
        out.push_str(" start_tick=");
        out.push_str(&entry.start_tick.to_string());
        out.push_str(" end_tick=");
        out.push_str(&entry.end_tick.to_string());
        match entry.kind {
            VmEpisodeKind::MapPath => {
                out.push_str(" mapped=");
                out.push_str(match entry.mapped_kind {
                    0 => "anon",
                    1 => "file-shared",
                    2 => "file-private",
                    _ => "unknown",
                });
            }
            VmEpisodeKind::HeapPath => {
                out.push_str(" grew=");
                out.push_str(if entry.grew { "yes" } else { "no" });
                out.push_str(" shrank=");
                out.push_str(if entry.shrank { "yes" } else { "no" });
                out.push_str(" old_end=");
                out.push_str(&entry.old_end.to_string());
                out.push_str(" new_end=");
                out.push_str(&entry.new_end.to_string());
            }
            VmEpisodeKind::ReclaimPath => {
                out.push_str(" evicted=");
                out.push_str(if entry.evicted { "yes" } else { "no" });
                out.push_str(" restored=");
                out.push_str(if entry.restored { "yes" } else { "no" });
            }
            VmEpisodeKind::Quarantine => {
                out.push_str(" reason=");
                out.push_str(&entry.quarantine_reason.to_string());
                out.push_str(" blocked=");
                out.push_str(if entry.blocked { "yes" } else { "no" });
                out.push_str(" released=");
                out.push_str(if entry.released { "yes" } else { "no" });
            }
            VmEpisodeKind::FaultPath => {
                out.push_str(" faulted=");
                out.push_str(if entry.faulted { "yes" } else { "no" });
                out.push_str(" cow=");
                out.push_str(if entry.cow { "yes" } else { "no" });
                out.push_str(" bridged=");
                out.push_str(if entry.bridged { "yes" } else { "no" });
                out.push_str(" touched=");
                out.push_str(if entry.touched { "yes" } else { "no" });
                out.push_str(" synced=");
                out.push_str(if entry.synced { "yes" } else { "no" });
                out.push_str(" advised=");
                out.push_str(if entry.advised { "yes" } else { "no" });
            }
            VmEpisodeKind::RegionPath => {
                out.push_str(" protected=");
                out.push_str(if entry.protected { "yes" } else { "no" });
                out.push_str(" unmapped=");
                out.push_str(if entry.unmapped { "yes" } else { "no" });
            }
            VmEpisodeKind::PolicyPath => {
                out.push_str(" blocked=");
                out.push_str(if entry.policy_blocked { "yes" } else { "no" });
                out.push_str(" state=");
                out.push_str(&entry.policy_state.to_string());
                out.push_str(" operation=");
                out.push_str(&entry.policy_operation.to_string());
            }
        }
        out.push_str(" decisions=");
        out.push_str(&entry.decision_count.to_string());
        out.push_str(" last=");
        out.push_str(vm_agent_kind_label(entry.last_agent));
        out.push('\n');
        index += 1;
    }
}

fn resource_agent_kind_label(kind: ResourceAgentKind) -> &'static str {
    match kind {
        ResourceAgentKind::ClaimValidator => "claim-validator",
        ResourceAgentKind::CancelValidator => "cancel-validator",
        ResourceAgentKind::ReleaseValidator => "release-validator",
        ResourceAgentKind::ResourceStateTransitionAgent => "resource-state-transition",
        ResourceAgentKind::ContractStateTransitionAgent => "contract-state-transition",
    }
}

fn vm_agent_kind_label(kind: VmAgentKind) -> &'static str {
    match kind {
        VmAgentKind::MapAgent => "map",
        VmAgentKind::BrkAgent => "brk",
        VmAgentKind::ProtectAgent => "protect",
        VmAgentKind::UnmapAgent => "unmap",
        VmAgentKind::PolicyBlockAgent => "policy-block",
        VmAgentKind::PressureTriggerAgent => "pressure-trigger",
        VmAgentKind::PressureVictimAgent => "pressure-victim",
        VmAgentKind::FaultClassifierAgent => "fault-classifier",
        VmAgentKind::ShadowReuseAgent => "shadow-reuse",
        VmAgentKind::ShadowBridgeAgent => "shadow-bridge",
        VmAgentKind::CowPopulateAgent => "cow-populate",
        VmAgentKind::PageTouchAgent => "page-touch",
        VmAgentKind::SyncAgent => "sync",
        VmAgentKind::AdviceAgent => "advice",
        VmAgentKind::QuarantineStateAgent => "quarantine-state",
        VmAgentKind::QuarantineBlockAgent => "quarantine-block",
    }
}

fn build_vm_episode_entries(decisions: &[VmAgentDecisionRecord]) -> Vec<VmEpisodeReportEntry> {
    let mut open_map = Vec::<VmEpisodeReportEntry>::new();
    let mut open_heap = Vec::<VmEpisodeReportEntry>::new();
    let mut open_reclaim = Vec::<VmEpisodeReportEntry>::new();
    let mut open_quarantine = Vec::<VmEpisodeReportEntry>::new();
    let mut open_policy = Vec::<VmEpisodeReportEntry>::new();
    let mut open_fault = Vec::<VmEpisodeReportEntry>::new();
    let mut open_region = Vec::<VmEpisodeReportEntry>::new();
    let mut finished = Vec::<VmEpisodeReportEntry>::new();
    let mut index = 0usize;
    while index < decisions.len() {
        let entry = decisions[index];
        match entry.agent {
            VmAgentKind::MapAgent => {
                if let Some(slot) = find_vm_episode_slot(&open_map, entry.pid, entry.vm_object_id) {
                    let episode = &mut open_map[slot];
                    episode.end_tick = entry.tick;
                    episode.mapped_kind = entry.detail0;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                    episode.last_agent = entry.agent;
                } else {
                    let episode = VmEpisodeReportEntry {
                        kind: VmEpisodeKind::MapPath,
                        pid: entry.pid,
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: 0,
                        blocked: false,
                        released: false,
                        mapped_kind: entry.detail0,
                        old_end: 0,
                        new_end: 0,
                        grew: false,
                        shrank: false,
                        evicted: false,
                        restored: false,
                        protected: false,
                        unmapped: false,
                        policy_blocked: false,
                        policy_state: 0,
                        policy_operation: 0,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: false,
                    };
                    finished.push(episode);
                }
            }
            VmAgentKind::BrkAgent => {
                let grew = entry.detail1 > entry.detail0;
                let shrank = entry.detail1 < entry.detail0;
                if let Some(slot) = find_vm_episode_slot(&open_heap, entry.pid, entry.vm_object_id)
                {
                    let episode = &mut open_heap[slot];
                    episode.end_tick = entry.tick;
                    episode.old_end = episode.old_end.min(entry.detail0);
                    episode.new_end = entry.detail1;
                    episode.grew |= grew;
                    episode.shrank |= shrank;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                    episode.last_agent = entry.agent;
                } else {
                    open_heap.push(VmEpisodeReportEntry {
                        kind: VmEpisodeKind::HeapPath,
                        pid: entry.pid,
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: 0,
                        blocked: false,
                        released: false,
                        mapped_kind: 0,
                        old_end: entry.detail0,
                        new_end: entry.detail1,
                        grew,
                        shrank,
                        evicted: false,
                        restored: false,
                        protected: false,
                        unmapped: false,
                        policy_blocked: false,
                        policy_state: 0,
                        policy_operation: 0,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: false,
                    });
                }
            }
            VmAgentKind::QuarantineStateAgent if entry.detail1 == 1 => {
                if let Some(slot) = find_vm_episode_slot(&open_fault, entry.pid, entry.vm_object_id)
                {
                    finished.push(open_fault.remove(slot));
                }
                if let Some(slot) =
                    find_vm_episode_slot(&open_region, entry.pid, entry.vm_object_id)
                {
                    finished.push(open_region.remove(slot));
                }
                if let Some(slot) =
                    find_vm_episode_slot(&open_quarantine, entry.pid, entry.vm_object_id)
                {
                    let episode = &mut open_quarantine[slot];
                    episode.end_tick = entry.tick;
                    episode.quarantine_reason = entry.detail0;
                    episode.last_agent = entry.agent;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                } else {
                    open_quarantine.push(VmEpisodeReportEntry {
                        kind: VmEpisodeKind::Quarantine,
                        pid: entry.pid,
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: entry.detail0,
                        blocked: false,
                        released: false,
                        mapped_kind: 0,
                        old_end: 0,
                        new_end: 0,
                        grew: false,
                        shrank: false,
                        evicted: false,
                        restored: false,
                        protected: false,
                        unmapped: false,
                        policy_blocked: false,
                        policy_state: 0,
                        policy_operation: 0,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: false,
                    });
                }
            }
            VmAgentKind::QuarantineBlockAgent => {
                if let Some(slot) =
                    find_vm_episode_slot(&open_quarantine, entry.pid, entry.vm_object_id)
                {
                    let episode = &mut open_quarantine[slot];
                    episode.end_tick = entry.tick;
                    episode.blocked = true;
                    episode.last_agent = entry.agent;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                }
            }
            VmAgentKind::QuarantineStateAgent if entry.detail1 == 0 => {
                if let Some(slot) =
                    find_vm_episode_slot(&open_quarantine, entry.pid, entry.vm_object_id)
                {
                    let mut episode = open_quarantine.remove(slot);
                    episode.end_tick = entry.tick;
                    episode.released = true;
                    episode.last_agent = entry.agent;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                    finished.push(episode);
                }
            }
            VmAgentKind::AdviceAgent if entry.detail0 == 4 || entry.detail0 == 3 => {
                if let Some(slot) =
                    find_vm_episode_slot(&open_reclaim, entry.pid, entry.vm_object_id)
                {
                    let episode = &mut open_reclaim[slot];
                    episode.end_tick = entry.tick;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                    episode.last_agent = entry.agent;
                    if entry.detail0 == 4 {
                        episode.evicted = true;
                    } else {
                        episode.restored = true;
                    }
                } else {
                    open_reclaim.push(VmEpisodeReportEntry {
                        kind: VmEpisodeKind::ReclaimPath,
                        pid: entry.pid,
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: 0,
                        blocked: false,
                        released: false,
                        mapped_kind: 0,
                        old_end: 0,
                        new_end: 0,
                        grew: false,
                        shrank: false,
                        evicted: entry.detail0 == 4,
                        restored: entry.detail0 == 3,
                        protected: false,
                        unmapped: false,
                        policy_blocked: false,
                        policy_state: 0,
                        policy_operation: 0,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: true,
                    });
                }
                if entry.detail0 == 3
                    && let Some(slot) =
                        find_vm_episode_slot(&open_reclaim, entry.pid, entry.vm_object_id)
                {
                    finished.push(open_reclaim.remove(slot));
                }
            }
            VmAgentKind::ProtectAgent | VmAgentKind::UnmapAgent => {
                if let Some(slot) = find_vm_episode_slot(&open_fault, entry.pid, entry.vm_object_id)
                {
                    finished.push(open_fault.remove(slot));
                }
                if let Some(slot) =
                    find_vm_episode_slot(&open_region, entry.pid, entry.vm_object_id)
                {
                    let episode = &mut open_region[slot];
                    episode.end_tick = entry.tick;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                    episode.last_agent = entry.agent;
                    if matches!(entry.agent, VmAgentKind::ProtectAgent) {
                        episode.protected = true;
                    } else {
                        episode.unmapped = true;
                    }
                } else {
                    open_region.push(VmEpisodeReportEntry {
                        kind: VmEpisodeKind::RegionPath,
                        pid: entry.pid,
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: 0,
                        blocked: false,
                        released: false,
                        mapped_kind: 0,
                        old_end: 0,
                        new_end: 0,
                        grew: false,
                        shrank: false,
                        evicted: false,
                        restored: false,
                        protected: matches!(entry.agent, VmAgentKind::ProtectAgent),
                        unmapped: matches!(entry.agent, VmAgentKind::UnmapAgent),
                        policy_blocked: false,
                        policy_state: 0,
                        policy_operation: 0,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: false,
                    });
                }
                if matches!(entry.agent, VmAgentKind::UnmapAgent)
                    && let Some(slot) =
                        find_vm_episode_slot(&open_region, entry.pid, entry.vm_object_id)
                {
                    finished.push(open_region.remove(slot));
                }
            }
            VmAgentKind::PolicyBlockAgent => {
                open_policy.push(VmEpisodeReportEntry {
                    kind: VmEpisodeKind::PolicyPath,
                    pid: entry.pid,
                    vm_object_id: entry.vm_object_id,
                    start_tick: entry.tick,
                    end_tick: entry.tick,
                    quarantine_reason: 0,
                    blocked: false,
                    released: false,
                    mapped_kind: 0,
                    old_end: 0,
                    new_end: 0,
                    grew: false,
                    shrank: false,
                    evicted: false,
                    restored: false,
                    protected: false,
                    unmapped: false,
                    policy_blocked: true,
                    policy_state: entry.detail0,
                    policy_operation: entry.detail1,
                    decision_count: 1,
                    last_agent: entry.agent,
                    faulted: false,
                    cow: false,
                    bridged: false,
                    touched: false,
                    synced: false,
                    advised: false,
                });
            }
            VmAgentKind::PressureTriggerAgent | VmAgentKind::PressureVictimAgent => {}
            _ => {
                if let Some(slot) =
                    find_vm_episode_slot(&open_reclaim, entry.pid, entry.vm_object_id)
                {
                    let episode = &mut open_reclaim[slot];
                    episode.end_tick = entry.tick;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                    episode.last_agent = entry.agent;
                    if matches!(
                        entry.agent,
                        VmAgentKind::FaultClassifierAgent
                            | VmAgentKind::PageTouchAgent
                            | VmAgentKind::SyncAgent
                    ) {
                        episode.restored = true;
                    }
                    if matches!(
                        entry.agent,
                        VmAgentKind::PageTouchAgent | VmAgentKind::SyncAgent
                    ) && let Some(slot) =
                        find_vm_episode_slot(&open_reclaim, entry.pid, entry.vm_object_id)
                    {
                        finished.push(open_reclaim.remove(slot));
                    }
                }
                if let Some(slot) = find_vm_episode_slot(&open_fault, entry.pid, entry.vm_object_id)
                {
                    let episode = &mut open_fault[slot];
                    episode.end_tick = entry.tick;
                    episode.decision_count = episode.decision_count.saturating_add(1);
                    episode.last_agent = entry.agent;
                    mark_fault_episode_flag(episode, entry.agent);
                } else {
                    let mut episode = VmEpisodeReportEntry {
                        kind: VmEpisodeKind::FaultPath,
                        pid: entry.pid,
                        vm_object_id: entry.vm_object_id,
                        start_tick: entry.tick,
                        end_tick: entry.tick,
                        quarantine_reason: 0,
                        blocked: false,
                        released: false,
                        mapped_kind: 0,
                        old_end: 0,
                        new_end: 0,
                        grew: false,
                        shrank: false,
                        evicted: false,
                        restored: false,
                        protected: false,
                        unmapped: false,
                        policy_blocked: false,
                        policy_state: 0,
                        policy_operation: 0,
                        decision_count: 1,
                        last_agent: entry.agent,
                        faulted: false,
                        cow: false,
                        bridged: false,
                        touched: false,
                        synced: false,
                        advised: false,
                    };
                    mark_fault_episode_flag(&mut episode, entry.agent);
                    open_fault.push(episode);
                }
            }
        }
        index += 1;
    }
    finished.extend(open_map);
    finished.extend(open_heap);
    finished.extend(open_reclaim);
    finished.extend(open_quarantine);
    finished.extend(open_policy);
    finished.extend(open_fault);
    finished.extend(open_region);
    finished.sort_by_key(|entry| (entry.start_tick, entry.pid, entry.vm_object_id));
    finished
}

fn find_vm_episode_slot(
    episodes: &[VmEpisodeReportEntry],
    pid: u64,
    vm_object_id: u64,
) -> Option<usize> {
    let mut index = 0usize;
    while index < episodes.len() {
        let entry = episodes[index];
        if entry.pid == pid && entry.vm_object_id == vm_object_id {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn mark_fault_episode_flag(entry: &mut VmEpisodeReportEntry, agent: VmAgentKind) {
    match agent {
        VmAgentKind::MapAgent
        | VmAgentKind::BrkAgent
        | VmAgentKind::ProtectAgent
        | VmAgentKind::UnmapAgent
        | VmAgentKind::PolicyBlockAgent
        | VmAgentKind::PressureTriggerAgent
        | VmAgentKind::PressureVictimAgent => {}
        VmAgentKind::FaultClassifierAgent => entry.faulted = true,
        VmAgentKind::ShadowReuseAgent | VmAgentKind::CowPopulateAgent => entry.cow = true,
        VmAgentKind::ShadowBridgeAgent => entry.bridged = true,
        VmAgentKind::PageTouchAgent => entry.touched = true,
        VmAgentKind::SyncAgent => entry.synced = true,
        VmAgentKind::AdviceAgent => entry.advised = true,
        VmAgentKind::QuarantineStateAgent | VmAgentKind::QuarantineBlockAgent => {}
    }
}

fn render_cause(out: &mut String, summary: &ChronoscopeOperatorSummary) {
    if summary.cause_node_id == 0 {
        if summary.available {
            out.push_str("none");
        } else {
            out.push_str("unknown");
        }
        return;
    }
    out.push_str("node=");
    out.push_str(&summary.cause_node_id.to_string());
    out.push_str(" kind=");
    out.push_str(match summary.cause_kind {
        Some(ChronoscopeNodeKind::Observation) => "observation",
        Some(ChronoscopeNodeKind::Interpretation) => "interpretation",
        Some(ChronoscopeNodeKind::Constraint) => "constraint",
        Some(ChronoscopeNodeKind::Boundary) => "boundary",
        Some(ChronoscopeNodeKind::Outcome) => "outcome",
        None => "unknown",
    });
}

fn render_node_id(out: &mut String, node_id: ChronoscopeNodeId) {
    if node_id == 0 {
        out.push_str("none");
    } else {
        out.push_str(&node_id.to_string());
    }
}

fn render_checkpoint_id(out: &mut String, checkpoint_id: Option<ChronoscopeCheckpointId>) {
    match checkpoint_id {
        Some(id) if id != ChronoscopeCheckpointId::NONE => out.push_str(&id.0.to_string()),
        _ => out.push_str("none"),
    }
}

fn render_propagation(out: &mut String, summary: &ChronoscopeOperatorSummary) {
    if summary.propagation_len == 0 {
        out.push_str(if summary.available { "none" } else { "unknown" });
        return;
    }
    let mut index = 0usize;
    while index < summary.propagation_len {
        if index != 0 {
            out.push_str(" -> ");
        }
        out.push_str(&summary.propagation[index].to_string());
        index += 1;
    }
    if summary.propagation_truncated {
        out.push_str(" -> ...");
    }
}

fn render_trust(out: &mut String, summary: &ChronoscopeOperatorSummary) {
    match summary.trust_complete {
        Some(true) => out.push_str("full"),
        Some(false) => out.push_str("partial"),
        None => out.push_str("unknown"),
    }

    if summary.trust_complete != Some(true) {
        out.push_str(" reason=");
        out.push_str(match summary.trust_reason {
            ChronoscopePartialReason::None => "unknown",
            ChronoscopePartialReason::RingOverwrite => "ring_overwrite",
            ChronoscopePartialReason::MissingCrossCoreHistory => "missing_cross_core",
            ChronoscopePartialReason::DeepCaptureDrop => "deep_capture_drop",
            ChronoscopePartialReason::PanicTruncation => "panic_truncation",
            ChronoscopePartialReason::ReplayIncomplete => "replay_incomplete",
            ChronoscopePartialReason::DiffIncomplete => "diff_incomplete",
        });
        out.push_str(" flags=");
        let mut wrote_flag = false;
        wrote_flag |= render_flag(
            out,
            "ring_overwrite",
            summary.trust_flags.ring_overwrite,
            wrote_flag,
        );
        wrote_flag |= render_flag(
            out,
            "missing_cross_core",
            summary.trust_flags.missing_cross_core,
            wrote_flag,
        );
        wrote_flag |= render_flag(
            out,
            "deep_capture_drop",
            summary.trust_flags.deep_capture_drop,
            wrote_flag,
        );
        wrote_flag |= render_flag(
            out,
            "panic_truncated",
            summary.trust_flags.panic_truncated,
            wrote_flag,
        );
        wrote_flag |= render_flag(
            out,
            "replay_incomplete",
            summary.trust_flags.replay_incomplete,
            wrote_flag,
        );
        wrote_flag |= render_flag(
            out,
            "diff_incomplete",
            summary.trust_flags.diff_incomplete,
            wrote_flag,
        );
        if !wrote_flag {
            out.push_str("none");
        }
    }
}

fn render_flag(out: &mut String, label: &str, enabled: bool, needs_separator: bool) -> bool {
    if !enabled {
        return false;
    }
    if needs_separator {
        out.push(',');
    }
    out.push_str(label);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ngos_boot_x86_64::diagnostics::{
        ChronoscopeCaptureLevel, ChronoscopeEventId, ChronoscopeHistoryIntegrity, ReplaySummary,
        chronoscope_snapshot,
    };

    fn deterministic_empty_bundle() -> ChronoscopeBundle {
        let mut bundle = chronoscope_snapshot();
        bundle.valid = false;
        bundle.trust = ChronoscopeTrustSurface {
            schema_version: bundle.trust.schema_version,
            completeness: ChronoscopeHistoryIntegrity::COMPLETE,
            capture_level: ChronoscopeCaptureLevel::Minimal,
            replay_partial: false,
            explain_degraded: false,
            responsibility_partial: false,
        };
        bundle.temporal_explain.replay_summary = ReplaySummary {
            valid: false,
            start_checkpoint: ChronoscopeCheckpointId::NONE,
            steps_to_fault: 0,
            deterministic: false,
            partial: false,
            divergence_point: ChronoscopeEventId::NONE,
        };
        bundle.temporal_explain.responsibility_confidence = 0;
        bundle
    }

    fn report_with_summary(summary: ChronoscopeOperatorSummary) -> HostRuntimeNativeSessionReport {
        HostRuntimeNativeSessionReport {
            pid: 2,
            exit_code: 0,
            stdout_bytes: 4,
            ui_presentation_backend: "skia-host",
            session_reported: true,
            session_report_count: 1,
            session_status: 0,
            session_stage: 2,
            session_code: 0,
            session_detail: 1,
            domain_count: 1,
            resource_count: 1,
            contract_count: 1,
            chronoscope: summary,
            resource_agents: ResourceAgentReportSummary::EMPTY,
            vm_agents: VmAgentReportSummary::EMPTY,
            vm_episodes: VmEpisodeReportSummary::EMPTY,
            stdout: String::from("ok\n"),
        }
    }

    #[test]
    fn resource_agent_report_renders_none_when_empty() {
        let rendered = report_with_summary(ChronoscopeOperatorSummary::EMPTY).render();
        assert!(rendered.contains("== resource-agents =="));
        assert!(rendered.contains("decisions: 0 none"));
    }

    #[test]
    fn resource_agent_report_extracts_tail_and_marks_truncation() {
        let decisions = (1u64..=10)
            .map(|tick| ResourceAgentDecisionRecord {
                tick,
                agent: ResourceAgentKind::ClaimValidator,
                resource: 7,
                contract: tick,
                detail0: 1,
                detail1: tick + 10,
            })
            .collect::<Vec<_>>();
        let summary = extract_resource_agent_report_summary(&decisions);
        assert_eq!(summary.total_decisions, 10);
        assert_eq!(summary.entry_count, 8);
        assert!(summary.truncated);
        assert_eq!(summary.entries[0].expect("entry").tick, 3);
        assert_eq!(summary.entries[7].expect("entry").tick, 10);
    }

    #[test]
    fn resource_agent_report_renders_stable_entries() {
        let mut report = report_with_summary(ChronoscopeOperatorSummary::EMPTY);
        report.resource_agents = ResourceAgentReportSummary {
            total_decisions: 2,
            entries: [
                Some(ResourceAgentDecisionRecord {
                    tick: 4,
                    agent: ResourceAgentKind::ClaimValidator,
                    resource: 9,
                    contract: 11,
                    detail0: 1,
                    detail1: 1,
                }),
                Some(ResourceAgentDecisionRecord {
                    tick: 5,
                    agent: ResourceAgentKind::ReleaseValidator,
                    resource: 9,
                    contract: 12,
                    detail0: 2,
                    detail1: 1,
                }),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            entry_count: 2,
            truncated: false,
        };
        let rendered = report.render();
        assert!(rendered.contains("== resource-agents =="));
        assert!(rendered.contains("decisions: 2"));
        assert!(
            rendered.contains(
                "- tick=4 agent=claim-validator resource=9 contract=11 detail0=1 detail1=1"
            )
        );
        assert!(rendered.contains(
            "- tick=5 agent=release-validator resource=9 contract=12 detail0=2 detail1=1"
        ));
    }

    #[test]
    fn chronoscope_summary_extraction_handles_empty_bundle() {
        let bundle = deterministic_empty_bundle();
        let summary = extract_chronoscope_summary(&bundle);
        assert!(!summary.available);
        assert_eq!(summary.trust_complete, Some(true));
        assert_eq!(
            summary.replay_status,
            ChronoscopeOperatorReplayStatus::Unknown
        );
    }

    #[test]
    fn chronoscope_summary_extraction_marks_long_propagation_as_truncated() {
        let mut bundle = deterministic_empty_bundle();
        bundle.valid = true;
        bundle.temporal_explain.propagation_path = [10, 11, 12, 13, 14, 15, 16, 17];

        let summary = extract_chronoscope_summary(&bundle);
        assert_eq!(summary.propagation_len, CHRONOSCOPE_OPERATOR_PATH_LIMIT);
        assert_eq!(summary.propagation, [10, 11, 12, 13, 14, 15]);
        assert!(summary.propagation_truncated);
    }

    #[test]
    fn chronoscope_render_keeps_all_fields_when_partial() {
        let report = report_with_summary(ChronoscopeOperatorSummary {
            available: true,
            cause_node_id: 9,
            cause_kind: Some(ChronoscopeNodeKind::Outcome),
            responsible_node_id: 7,
            last_writer_node_id: 5,
            rewind_checkpoint_id: Some(ChronoscopeCheckpointId(3)),
            divergence_checkpoint_id: None,
            propagation: [7, 8, 9, 0, 0, 0],
            propagation_len: 3,
            propagation_truncated: false,
            trust_complete: Some(false),
            trust_reason: ChronoscopePartialReason::RingOverwrite,
            trust_flags: ChronoscopeCompletenessFlags {
                ring_overwrite: true,
                missing_cross_core: false,
                deep_capture_drop: false,
                panic_truncated: false,
                replay_incomplete: true,
                diff_incomplete: false,
            },
            replay_status: ChronoscopeOperatorReplayStatus::Partial,
            responsibility_confidence: 73,
        });

        let rendered = report.render();
        assert!(rendered.contains("== chronoscope =="));
        assert!(rendered.contains("cause: node=9 kind=outcome"));
        assert!(rendered.contains("responsible: 7"));
        assert!(rendered.contains("last_writer: 5"));
        assert!(rendered.contains("rewind: 3"));
        assert!(rendered.contains("divergence: none"));
        assert!(rendered.contains("propagation: 7 -> 8 -> 9"));
        assert!(rendered.contains(
            "trust: partial reason=ring_overwrite flags=ring_overwrite,replay_incomplete"
        ));
        assert!(rendered.contains("replay: partial"));
        assert!(rendered.contains("confidence: 0.73"));
    }

    #[test]
    fn chronoscope_render_is_deterministic_and_truncates_propagation() {
        let summary = ChronoscopeOperatorSummary {
            available: true,
            cause_node_id: 11,
            cause_kind: Some(ChronoscopeNodeKind::Outcome),
            responsible_node_id: 10,
            last_writer_node_id: 9,
            rewind_checkpoint_id: Some(ChronoscopeCheckpointId(2)),
            divergence_checkpoint_id: Some(ChronoscopeCheckpointId(4)),
            propagation: [1, 2, 3, 4, 5, 6],
            propagation_len: 6,
            propagation_truncated: true,
            trust_complete: Some(true),
            trust_reason: ChronoscopePartialReason::None,
            trust_flags: ChronoscopeCompletenessFlags::EMPTY,
            replay_status: ChronoscopeOperatorReplayStatus::Full,
            responsibility_confidence: 100,
        };
        let left = report_with_summary(summary.clone()).render();
        let right = report_with_summary(summary).render();
        assert_eq!(left, right);
        assert!(left.contains("propagation: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> ..."));
    }

    #[test]
    fn chronoscope_render_marks_unknown_and_never_panics_for_partial_bundle() {
        let rendered = report_with_summary(ChronoscopeOperatorSummary {
            trust_complete: None,
            replay_status: ChronoscopeOperatorReplayStatus::Degraded,
            ..ChronoscopeOperatorSummary::EMPTY
        })
        .render();
        assert!(rendered.contains("cause: unknown"));
        assert!(rendered.contains("responsible: none"));
        assert!(rendered.contains("propagation: unknown"));
        assert!(rendered.contains("trust: unknown reason=unknown flags=none"));
        assert!(rendered.contains("replay: degraded"));
    }
}
