//! Canonical subsystem role:
//! - subsystem: native repair and modernization control
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing repair orchestration on top of canonical
//!   system truth
//!
//! Canonical contract families handled here:
//! - repair command contracts
//! - modernization command contracts
//! - repair AI diagnosis / planning contracts
//!
//! This module may plan and orchestrate corrective actions from canonical
//! signals, but it must not redefine kernel truth, verified-core truth, or
//! scheduler truth.

#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use ngos_semantic_runtime::{
    semantic_verdict_name, PressureState, SemanticVerdict, SystemPressureMetrics,
};
use ngos_shell_proc::scheduler_class_label;
use ngos_shell_vfs::shell_write_file;
use ngos_user_abi::{ExitCode, NativeNetworkInterfaceRecord, NativeSchedulerClass, SyscallBackend};
use ngos_user_runtime::{
    system_control::{
        AdaptiveState, DeviceHandle, ProcessAction, ProcessEntity, ProcessHandle, SystemController,
        SystemFact,
    },
    Runtime,
};

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Local semantic helpers (private to this crate)
// ---------------------------------------------------------------------------

fn nextmind_pressure_state_label(state: PressureState) -> &'static str {
    match state {
        PressureState::Stable => "stable",
        PressureState::HighSchedulerPressure => "high-scheduler-pressure",
        PressureState::NetworkBackpressure => "network-backpressure",
        PressureState::MixedPressure => "mixed-pressure",
    }
}

fn nextmind_metrics_score(metrics: &SystemPressureMetrics) -> u64 {
    metrics.run_queue_total.saturating_mul(100)
        + metrics.run_queue_urgent_total().saturating_mul(80)
        + metrics.scheduler_lag_debt_total.max(0) as u64 * 25
        + metrics.scheduler_dispatch_total.saturating_mul(5)
        + metrics.scheduler_runtime_ticks_total.saturating_mul(5)
        + metrics.scheduler_runtime_imbalance.saturating_mul(20)
        + metrics.scheduler_cpu_load_imbalance.saturating_mul(40)
        + metrics.bus_endpoint_count.saturating_mul(5)
        + metrics.saturated_bus_endpoint_count.saturating_mul(100)
        + metrics.bus_queue_depth_total.saturating_mul(15)
        + metrics.bus_overflow_total.saturating_mul(120)
        + u64::from(metrics.bus_pressure_pct)
        + metrics.snapshot.saturated_socket_count.saturating_mul(100)
        + u64::from(metrics.cpu_utilization_pct)
        + u64::from(metrics.socket_pressure_pct)
        + u64::from(metrics.event_queue_pressure_pct)
        + metrics.snapshot.blocked_process_count.saturating_mul(5)
        + if metrics.scheduler_starved { 250 } else { 0 }
        + if metrics.verified_core_ok {
            0
        } else {
            500 + metrics.verified_core_violation_count.saturating_mul(50)
        }
}

fn nextmind_collect_process_entities(facts: &[SystemFact]) -> Vec<ProcessEntity> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Process(process) => Some(process.clone()),
            _ => None,
        })
        .collect()
}

fn nextmind_collect_device_entities(
    facts: &[SystemFact],
) -> Vec<(DeviceHandle, NativeNetworkInterfaceRecord)> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Device(device) => {
                device.record.map(|record| (device.handle.clone(), record))
            }
            _ => None,
        })
        .collect()
}

fn nextmind_protected_process(process: &ProcessEntity) -> bool {
    matches!(
        NativeSchedulerClass::from_raw(process.record.scheduler_class),
        Some(NativeSchedulerClass::LatencyCritical | NativeSchedulerClass::Interactive)
    ) || process.handle.pid == 1
}

fn nextmind_candidate_processes(processes: &[ProcessEntity]) -> Vec<ProcessEntity> {
    let mut candidates = processes
        .iter()
        .filter(|process| {
            !nextmind_protected_process(process) && matches!(process.record.state, 1 | 2)
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .record
            .cpu_runtime_ticks
            .cmp(&left.record.cpu_runtime_ticks)
            .then(
                right
                    .record
                    .scheduler_budget
                    .cmp(&left.record.scheduler_budget),
            )
            .then(left.handle.pid.cmp(&right.handle.pid))
    });
    candidates
}

// ---------------------------------------------------------------------------
// Internal repair types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum MaintenanceStrategy {
    BalancedRepair,
    SchedulerFirst,
    NetworkFirst,
    Modernize,
}

impl MaintenanceStrategy {
    fn label(self) -> &'static str {
        match self {
            Self::BalancedRepair => "balanced-repair",
            Self::SchedulerFirst => "scheduler-first",
            Self::NetworkFirst => "network-first",
            Self::Modernize => "modernize",
        }
    }
}

#[derive(Clone, Copy)]
pub struct RepairAiState {
    next_episode_id: u64,
    memory: [Option<RepairEpisode>; 8],
}

impl RepairAiState {
    pub const fn new() -> Self {
        Self {
            next_episode_id: 1,
            memory: [None; 8],
        }
    }

    fn record(&mut self, episode: RepairEpisode) {
        let index = self
            .memory
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| (episode.id as usize) % self.memory.len());
        self.memory[index] = Some(episode);
        self.next_episode_id = self.next_episode_id.saturating_add(1);
    }

    fn episodes(&self) -> impl Iterator<Item = &RepairEpisode> {
        self.memory.iter().filter_map(|entry| entry.as_ref())
    }

    fn replace_with(&mut self, episodes: &[RepairEpisode]) {
        self.memory = [None; 8];
        self.next_episode_id = 1;
        for episode in episodes.iter().copied().take(self.memory.len()) {
            self.record(episode);
        }
        if let Some(max_id) = episodes.iter().map(|episode| episode.id).max() {
            self.next_episode_id = max_id.saturating_add(1);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct IncidentSignature {
    verified_core_ok: bool,
    violation_count: u8,
    runq_bucket: u8,
    cpu_bucket: u8,
    socket_bucket: u8,
    event_bucket: u8,
    drop_bucket: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct RepairEpisode {
    id: u64,
    strategy: MaintenanceStrategy,
    verdict: SemanticVerdict,
    before_score: u32,
    after_score: u32,
    signature: IncidentSignature,
}

#[derive(Clone)]
struct ProcessRepairRollback {
    handle: ProcessHandle,
    class: NativeSchedulerClass,
    budget: u32,
}

#[derive(Clone)]
struct DeviceRepairRollback {
    handle: DeviceHandle,
    record: NativeNetworkInterfaceRecord,
}

#[derive(Clone)]
struct MaintenanceReport {
    label: &'static str,
    strategy: &'static str,
    before: SystemPressureMetrics,
    after: SystemPressureMetrics,
    trigger: PressureState,
    reclaimed_pages: u64,
    actions: Vec<String>,
    verdict: SemanticVerdict,
}

#[derive(Clone)]
struct CandidateStrategyScore {
    strategy: MaintenanceStrategy,
    score: i32,
    reason: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IncidentNodeKind {
    VerifiedCore,
    Scheduler,
    Network,
    Eventing,
    Memory,
}

impl IncidentNodeKind {
    fn label(self) -> &'static str {
        match self {
            Self::VerifiedCore => "verified-core",
            Self::Scheduler => "scheduler",
            Self::Network => "network",
            Self::Eventing => "eventing",
            Self::Memory => "memory",
        }
    }
}

#[derive(Clone)]
struct IncidentNode {
    kind: IncidentNodeKind,
    severity: u32,
    detail: String,
}

#[derive(Clone)]
struct IncidentEdge {
    from: IncidentNodeKind,
    to: IncidentNodeKind,
    relation: &'static str,
}

#[derive(Clone)]
struct IncidentGraph {
    nodes: Vec<IncidentNode>,
    edges: Vec<IncidentEdge>,
}

#[derive(Clone)]
struct RepairHypothesis {
    strategy: MaintenanceStrategy,
    confidence: u32,
    rationale: String,
}

#[derive(Clone)]
struct RepairCritique {
    winner: MaintenanceStrategy,
    loser: MaintenanceStrategy,
    score_delta: i32,
    reason: String,
}

#[derive(Clone)]
struct VerifiedCoreFamilyStatus {
    family: &'static str,
    verified: bool,
}

#[derive(Clone)]
struct VerifiedCoreViolationView {
    family: String,
    code: String,
    detail: String,
}

#[derive(Clone)]
struct VerifiedCoreContext {
    verified: bool,
    source: &'static str,
    family_statuses: Vec<VerifiedCoreFamilyStatus>,
    violations: Vec<VerifiedCoreViolationView>,
}

// ---------------------------------------------------------------------------
// Core analysis helpers
// ---------------------------------------------------------------------------

fn pressure_bucket(value: u32) -> u8 {
    match value {
        0..=19 => 0,
        20..=49 => 1,
        50..=79 => 2,
        _ => 3,
    }
}

fn incident_signature(metrics: &SystemPressureMetrics) -> IncidentSignature {
    IncidentSignature {
        verified_core_ok: metrics.verified_core_ok,
        violation_count: metrics.verified_core_violation_count.min(255) as u8,
        runq_bucket: metrics.run_queue_total.min(255) as u8,
        cpu_bucket: pressure_bucket(metrics.cpu_utilization_pct),
        socket_bucket: pressure_bucket(metrics.socket_pressure_pct),
        event_bucket: pressure_bucket(metrics.event_queue_pressure_pct),
        drop_bucket: pressure_bucket(
            (metrics.tx_drop_delta.saturating_add(metrics.rx_drop_delta)).min(100) as u32,
        ),
    }
}

fn signature_distance(left: IncidentSignature, right: IncidentSignature) -> i32 {
    i32::from(left.verified_core_ok != right.verified_core_ok) * 4
        + (left.violation_count as i32 - right.violation_count as i32).abs()
        + (left.runq_bucket as i32 - right.runq_bucket as i32).abs()
        + (left.cpu_bucket as i32 - right.cpu_bucket as i32).abs()
        + (left.socket_bucket as i32 - right.socket_bucket as i32).abs()
        + (left.event_bucket as i32 - right.event_bucket as i32).abs()
        + (left.drop_bucket as i32 - right.drop_bucket as i32).abs()
}

fn default_verified_core_family_statuses(verified: bool) -> Vec<VerifiedCoreFamilyStatus> {
    vec![
        VerifiedCoreFamilyStatus {
            family: "capability-model",
            verified,
        },
        VerifiedCoreFamilyStatus {
            family: "vfs-invariants",
            verified,
        },
        VerifiedCoreFamilyStatus {
            family: "scheduler-state-machine",
            verified,
        },
        VerifiedCoreFamilyStatus {
            family: "cpu-extended-state-lifecycle",
            verified,
        },
    ]
}

fn parse_verified_core_context<B: SyscallBackend>(
    runtime: &Runtime<B>,
    metrics: &SystemPressureMetrics,
) -> VerifiedCoreContext {
    use ngos_shell_proc::read_procfs_all;

    let fallback = || VerifiedCoreContext {
        verified: metrics.verified_core_ok,
        source: "snapshot-fallback",
        family_statuses: default_verified_core_family_statuses(metrics.verified_core_ok),
        violations: if metrics.verified_core_ok {
            Vec::new()
        } else {
            vec![VerifiedCoreViolationView {
                family: String::from("unknown"),
                code: String::from("verified-core-broken"),
                detail: format!(
                    "snapshot-violations={}",
                    metrics.verified_core_violation_count
                ),
            }]
        },
    };

    let Ok(bytes) = read_procfs_all(runtime, "/proc/system/verified-core") else {
        return fallback();
    };
    let Ok(text) = core::str::from_utf8(&bytes) else {
        return fallback();
    };
    let mut verified = metrics.verified_core_ok;
    let mut family_statuses = Vec::new();
    let mut violations = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(value) = line.strip_prefix("verified:\t") {
            verified = value == "true";
            continue;
        }
        for family in [
            "capability-model",
            "vfs-invariants",
            "scheduler-state-machine",
            "cpu-extended-state-lifecycle",
        ] {
            if let Some(value) = line.strip_prefix(&format!("{family}:\t")) {
                family_statuses.push(VerifiedCoreFamilyStatus {
                    family,
                    verified: value == "true",
                });
                continue;
            }
        }
        if let Some(payload) = line.strip_prefix("violation\t") {
            let mut family = String::from("unknown");
            let mut code = String::from("unknown");
            let mut detail = String::new();
            for field in payload.split('\t') {
                if let Some(value) = field.strip_prefix("family=") {
                    family = value.to_string();
                } else if let Some(value) = field.strip_prefix("code=") {
                    code = value.to_string();
                } else if let Some(value) = field.strip_prefix("detail=") {
                    detail = value.to_string();
                }
            }
            violations.push(VerifiedCoreViolationView {
                family,
                code,
                detail,
            });
        }
    }
    if family_statuses.is_empty() {
        family_statuses = default_verified_core_family_statuses(verified);
    }
    VerifiedCoreContext {
        verified,
        source: "procfs",
        family_statuses,
        violations,
    }
}

fn primary_verified_core_family(context: &VerifiedCoreContext) -> &'static str {
    context
        .family_statuses
        .iter()
        .find(|status| !status.verified)
        .map(|status| status.family)
        .unwrap_or("stable")
}

fn build_incident_graph(
    metrics: &SystemPressureMetrics,
    trigger: PressureState,
    verified_core: &VerifiedCoreContext,
) -> IncidentGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    if !verified_core.verified {
        nodes.push(IncidentNode {
            kind: IncidentNodeKind::VerifiedCore,
            severity: 100,
            detail: format!(
                "violations={} focus={} source={}",
                verified_core
                    .violations
                    .len()
                    .max(metrics.verified_core_violation_count as usize),
                primary_verified_core_family(verified_core),
                verified_core.source
            ),
        });
    }
    if matches!(
        trigger,
        PressureState::HighSchedulerPressure | PressureState::MixedPressure
    ) || metrics.run_queue_total >= 3
    {
        nodes.push(IncidentNode {
            kind: IncidentNodeKind::Scheduler,
            severity: metrics
                .cpu_utilization_pct
                .max((metrics.run_queue_total as u32) * 10),
            detail: format!(
                "runq={} cpu={}",
                metrics.run_queue_total, metrics.cpu_utilization_pct
            ),
        });
    }
    if matches!(
        trigger,
        PressureState::NetworkBackpressure | PressureState::MixedPressure
    ) || metrics.socket_pressure_pct >= 80
    {
        nodes.push(IncidentNode {
            kind: IncidentNodeKind::Network,
            severity: metrics.socket_pressure_pct,
            detail: format!(
                "socket={} drops={}",
                metrics.socket_pressure_pct,
                metrics.tx_drop_delta.saturating_add(metrics.rx_drop_delta)
            ),
        });
    }
    if metrics.event_queue_pressure_pct >= 50 {
        nodes.push(IncidentNode {
            kind: IncidentNodeKind::Eventing,
            severity: metrics.event_queue_pressure_pct,
            detail: format!("event={}", metrics.event_queue_pressure_pct),
        });
    }
    if metrics.tx_drop_delta.saturating_add(metrics.rx_drop_delta) > 0 {
        nodes.push(IncidentNode {
            kind: IncidentNodeKind::Memory,
            severity: 40 + (metrics.tx_drop_delta.saturating_add(metrics.rx_drop_delta) as u32),
            detail: String::from("reclaimable-pressure-present"),
        });
    }

    let has = |kind: IncidentNodeKind| nodes.iter().any(|node| node.kind == kind);
    if has(IncidentNodeKind::VerifiedCore) && has(IncidentNodeKind::Scheduler) {
        edges.push(IncidentEdge {
            from: IncidentNodeKind::VerifiedCore,
            to: IncidentNodeKind::Scheduler,
            relation: "degrades-policy-safety",
        });
    }
    if has(IncidentNodeKind::Scheduler) && has(IncidentNodeKind::Eventing) {
        edges.push(IncidentEdge {
            from: IncidentNodeKind::Scheduler,
            to: IncidentNodeKind::Eventing,
            relation: "backs-up-waiters",
        });
    }
    if has(IncidentNodeKind::Network) && has(IncidentNodeKind::Eventing) {
        edges.push(IncidentEdge {
            from: IncidentNodeKind::Network,
            to: IncidentNodeKind::Eventing,
            relation: "amplifies-queue-pressure",
        });
    }
    if has(IncidentNodeKind::Network) && has(IncidentNodeKind::Memory) {
        edges.push(IncidentEdge {
            from: IncidentNodeKind::Network,
            to: IncidentNodeKind::Memory,
            relation: "drives-buffer-churn",
        });
    }

    IncidentGraph { nodes, edges }
}

fn build_repair_hypotheses(
    metrics: &SystemPressureMetrics,
    ai_state: &RepairAiState,
    verified_core: &VerifiedCoreContext,
) -> Vec<RepairHypothesis> {
    let mut candidates = [
        score_candidate(metrics, MaintenanceStrategy::BalancedRepair, ai_state),
        score_candidate(metrics, MaintenanceStrategy::SchedulerFirst, ai_state),
        score_candidate(metrics, MaintenanceStrategy::NetworkFirst, ai_state),
        score_candidate(metrics, MaintenanceStrategy::Modernize, ai_state),
    ];
    candidates.sort_by(|left, right| right.score.cmp(&left.score));
    candidates
        .into_iter()
        .map(|candidate| RepairHypothesis {
            strategy: candidate.strategy,
            confidence: candidate.score.max(0).min(100) as u32,
            rationale: if verified_core.verified {
                candidate.reason
            } else {
                format!(
                    "{}+kernel-family:{}",
                    candidate.reason,
                    primary_verified_core_family(verified_core)
                )
            },
        })
        .collect()
}

fn ranked_candidates(
    metrics: &SystemPressureMetrics,
    ai_state: &RepairAiState,
) -> Vec<CandidateStrategyScore> {
    let mut candidates = vec![
        score_candidate(metrics, MaintenanceStrategy::BalancedRepair, ai_state),
        score_candidate(metrics, MaintenanceStrategy::SchedulerFirst, ai_state),
        score_candidate(metrics, MaintenanceStrategy::NetworkFirst, ai_state),
        score_candidate(metrics, MaintenanceStrategy::Modernize, ai_state),
    ];
    candidates.sort_by(|left, right| right.score.cmp(&left.score));
    candidates
}

fn build_repair_critique(candidates: &[CandidateStrategyScore]) -> Vec<RepairCritique> {
    let Some(winner) = candidates.first() else {
        return Vec::new();
    };
    candidates
        .iter()
        .skip(1)
        .map(|candidate| RepairCritique {
            winner: winner.strategy,
            loser: candidate.strategy,
            score_delta: winner.score.saturating_sub(candidate.score),
            reason: format!("winner={} loser={}", winner.reason, candidate.reason),
        })
        .collect()
}

fn emit_verified_core_context<B: SyscallBackend>(
    runtime: &Runtime<B>,
    verified_core: &VerifiedCoreContext,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "repair-ai.kernel verified={} source={} families={} violations={}",
            verified_core.verified,
            verified_core.source,
            verified_core.family_statuses.len(),
            verified_core.violations.len()
        ),
    )?;
    for family in &verified_core.family_statuses {
        write_line(
            runtime,
            &format!(
                "repair-ai.kernel-family family={} verified={}",
                family.family, family.verified
            ),
        )?;
    }
    for violation in &verified_core.violations {
        write_line(
            runtime,
            &format!(
                "repair-ai.violation family={} code={} detail={}",
                violation.family, violation.code, violation.detail
            ),
        )?;
    }
    Ok(())
}

fn emit_candidate_scores<B: SyscallBackend>(
    runtime: &Runtime<B>,
    candidates: &[CandidateStrategyScore],
) -> Result<(), ExitCode> {
    for candidate in candidates {
        write_line(
            runtime,
            &format!(
                "repair-ai.candidate strategy={} score={} reason={}",
                candidate.strategy.label(),
                candidate.score,
                candidate.reason
            ),
        )?;
    }
    Ok(())
}

fn emit_repair_critique<B: SyscallBackend>(
    runtime: &Runtime<B>,
    critique: &[RepairCritique],
) -> Result<(), ExitCode> {
    for item in critique {
        write_line(
            runtime,
            &format!(
                "repair-ai.critic winner={} over={} delta={} reason={}",
                item.winner.label(),
                item.loser.label(),
                item.score_delta,
                item.reason
            ),
        )?;
    }
    Ok(())
}

fn read_path_all<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<Vec<u8>, ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 256];
    loop {
        let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    runtime.close(fd).map_err(|_| 240)?;
    Ok(bytes)
}

fn parse_strategy_label(label: &str) -> Option<MaintenanceStrategy> {
    match label {
        "balanced-repair" => Some(MaintenanceStrategy::BalancedRepair),
        "scheduler-first" => Some(MaintenanceStrategy::SchedulerFirst),
        "network-first" => Some(MaintenanceStrategy::NetworkFirst),
        "modernize" => Some(MaintenanceStrategy::Modernize),
        _ => None,
    }
}

fn parse_verdict_label(label: &str) -> Option<SemanticVerdict> {
    match label {
        "improved" => Some(SemanticVerdict::Improved),
        "no-change" => Some(SemanticVerdict::NoChange),
        "worse" => Some(SemanticVerdict::Worse),
        _ => None,
    }
}

fn serialize_repair_episode(episode: &RepairEpisode) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        episode.id,
        episode.strategy.label(),
        semantic_verdict_name(episode.verdict),
        episode.before_score,
        episode.after_score,
        episode.signature.verified_core_ok as u8,
        episode.signature.violation_count,
        episode.signature.runq_bucket,
        episode.signature.cpu_bucket,
        episode.signature.socket_bucket,
        episode.signature.event_bucket,
        episode.signature.drop_bucket
    )
}

fn deserialize_repair_episode(line: &str) -> Option<RepairEpisode> {
    let mut fields = line.split('|');
    let id = fields.next()?.parse().ok()?;
    let strategy = parse_strategy_label(fields.next()?)?;
    let verdict = parse_verdict_label(fields.next()?)?;
    let before_score = fields.next()?.parse().ok()?;
    let after_score = fields.next()?.parse().ok()?;
    let verified_core_ok = match fields.next()? {
        "0" => false,
        "1" => true,
        _ => return None,
    };
    let violation_count = fields.next()?.parse().ok()?;
    let runq_bucket = fields.next()?.parse().ok()?;
    let cpu_bucket = fields.next()?.parse().ok()?;
    let socket_bucket = fields.next()?.parse().ok()?;
    let event_bucket = fields.next()?.parse().ok()?;
    let drop_bucket = fields.next()?.parse().ok()?;
    Some(RepairEpisode {
        id,
        strategy,
        verdict,
        before_score,
        after_score,
        signature: IncidentSignature {
            verified_core_ok,
            violation_count,
            runq_bucket,
            cpu_bucket,
            socket_bucket,
            event_bucket,
            drop_bucket,
        },
    })
}

fn save_repair_ai_memory<B: SyscallBackend>(
    runtime: &Runtime<B>,
    ai_state: &RepairAiState,
    path: &str,
) -> Result<(), ExitCode> {
    let payload = ai_state
        .episodes()
        .map(serialize_repair_episode)
        .collect::<Vec<_>>()
        .join("\n");
    shell_write_file(runtime, path, &payload)?;
    write_line(
        runtime,
        &format!(
            "repair-ai.memory.saved path={} entries={}",
            path,
            ai_state.episodes().count()
        ),
    )
}

fn load_repair_ai_memory<B: SyscallBackend>(
    runtime: &Runtime<B>,
    ai_state: &mut RepairAiState,
    path: &str,
) -> Result<(), ExitCode> {
    let bytes = read_path_all(runtime, path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 266)?;
    let mut episodes = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some(episode) = deserialize_repair_episode(line) else {
            return Err(269);
        };
        episodes.push(episode);
    }
    ai_state.replace_with(&episodes);
    write_line(
        runtime,
        &format!(
            "repair-ai.memory.loaded path={} entries={}",
            path,
            episodes.len()
        ),
    )
}

fn render_maintenance_metrics<B: SyscallBackend>(
    runtime: &Runtime<B>,
    label: &str,
    metrics: &SystemPressureMetrics,
    trigger: PressureState,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "system.{label}.metrics state={} runq={} cpu={} socket={} event={} verified-core={} violations={} reclaimed={}/{}",
            nextmind_pressure_state_label(trigger),
            metrics.run_queue_total,
            metrics.cpu_utilization_pct,
            metrics.socket_pressure_pct,
            metrics.event_queue_pressure_pct,
            metrics.verified_core_ok,
            metrics.verified_core_violation_count,
            metrics.tx_drop_delta,
            metrics.rx_drop_delta,
        ),
    )
}

fn maintenance_verdict(
    before: &SystemPressureMetrics,
    after: &SystemPressureMetrics,
) -> SemanticVerdict {
    match nextmind_metrics_score(after).cmp(&nextmind_metrics_score(before)) {
        core::cmp::Ordering::Less => SemanticVerdict::Improved,
        core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
        core::cmp::Ordering::Greater => SemanticVerdict::Worse,
    }
}

fn score_candidate(
    metrics: &SystemPressureMetrics,
    strategy: MaintenanceStrategy,
    ai_state: &RepairAiState,
) -> CandidateStrategyScore {
    let mut score = 0i32;
    let mut reason = Vec::new();
    if !metrics.verified_core_ok {
        score -= 1000;
        reason.push("verified-core-degraded");
    }
    match strategy {
        MaintenanceStrategy::BalancedRepair => {
            score += 20;
            reason.push("balanced-default");
        }
        MaintenanceStrategy::SchedulerFirst => {
            score += (metrics.run_queue_total as i32) * 8;
            score += (metrics.cpu_utilization_pct as i32) / 4;
            reason.push("scheduler-pressure");
        }
        MaintenanceStrategy::NetworkFirst => {
            score += (metrics.socket_pressure_pct as i32) / 2;
            score += (metrics.event_queue_pressure_pct as i32) / 3;
            score += (metrics.tx_drop_delta.saturating_add(metrics.rx_drop_delta) as i32) * 3;
            reason.push("network-pressure");
        }
        MaintenanceStrategy::Modernize => {
            score += 15;
            score += (100 - metrics.cpu_utilization_pct.min(100) as i32) / 5;
            reason.push("proactive-modernize");
        }
    }

    let signature = incident_signature(metrics);
    for episode in ai_state.episodes() {
        if episode.strategy != strategy {
            continue;
        }
        let distance = signature_distance(signature, episode.signature);
        let memory_weight = (12 - distance).max(0);
        let outcome_weight = match episode.verdict {
            SemanticVerdict::Improved => 18,
            SemanticVerdict::NoChange => 3,
            SemanticVerdict::Worse => -18,
        };
        score += memory_weight * outcome_weight;
    }

    CandidateStrategyScore {
        strategy,
        score,
        reason: reason.join("+"),
    }
}

fn choose_ai_strategy(
    metrics: &SystemPressureMetrics,
    ai_state: &RepairAiState,
) -> CandidateStrategyScore {
    let mut candidates = [
        score_candidate(metrics, MaintenanceStrategy::BalancedRepair, ai_state),
        score_candidate(metrics, MaintenanceStrategy::SchedulerFirst, ai_state),
        score_candidate(metrics, MaintenanceStrategy::NetworkFirst, ai_state),
        score_candidate(metrics, MaintenanceStrategy::Modernize, ai_state),
    ];
    candidates.sort_by(|left, right| right.score.cmp(&left.score));
    candidates[0].clone()
}

fn emit_incident_graph<B: SyscallBackend>(
    runtime: &Runtime<B>,
    graph: &IncidentGraph,
) -> Result<(), ExitCode> {
    if graph.nodes.is_empty() {
        return write_line(runtime, "repair-ai.graph nodes=0 edges=0");
    }
    write_line(
        runtime,
        &format!(
            "repair-ai.graph nodes={} edges={}",
            graph.nodes.len(),
            graph.edges.len()
        ),
    )?;
    for node in &graph.nodes {
        write_line(
            runtime,
            &format!(
                "repair-ai.node kind={} severity={} detail={}",
                node.kind.label(),
                node.severity,
                node.detail
            ),
        )?;
    }
    for edge in &graph.edges {
        write_line(
            runtime,
            &format!(
                "repair-ai.edge from={} to={} relation={}",
                edge.from.label(),
                edge.to.label(),
                edge.relation
            ),
        )?;
    }
    Ok(())
}

fn emit_repair_hypotheses<B: SyscallBackend>(
    runtime: &Runtime<B>,
    hypotheses: &[RepairHypothesis],
) -> Result<(), ExitCode> {
    for hypothesis in hypotheses {
        write_line(
            runtime,
            &format!(
                "repair-ai.hypothesis strategy={} confidence={} rationale={}",
                hypothesis.strategy.label(),
                hypothesis.confidence,
                hypothesis.rationale
            ),
        )?;
    }
    Ok(())
}

fn apply_repair_actions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    label: &'static str,
    before: &SystemPressureMetrics,
    trigger: PressureState,
    strategy: MaintenanceStrategy,
) -> Result<
    (
        Vec<String>,
        Vec<ProcessRepairRollback>,
        Vec<DeviceRepairRollback>,
        u64,
    ),
    ExitCode,
> {
    let controller = SystemController::new(runtime);
    let facts = controller.collect_facts().map_err(|_| 266)?;
    let processes = nextmind_collect_process_entities(&facts);
    let devices = nextmind_collect_device_entities(&facts);
    let mut actions = Vec::new();
    let mut process_rollbacks = Vec::new();
    let mut device_rollbacks = Vec::new();

    if matches!(
        strategy,
        MaintenanceStrategy::BalancedRepair | MaintenanceStrategy::SchedulerFirst
    ) && matches!(
        trigger,
        PressureState::HighSchedulerPressure | PressureState::MixedPressure
    ) {
        for process in nextmind_candidate_processes(&processes).into_iter().take(2) {
            let class = scheduler_class_label(process.record.scheduler_class);
            if process.record.scheduler_class == NativeSchedulerClass::Background as u32
                && process.record.scheduler_budget <= 1
            {
                continue;
            }
            process_rollbacks.push(ProcessRepairRollback {
                handle: process.handle,
                class: NativeSchedulerClass::from_raw(process.record.scheduler_class)
                    .unwrap_or(NativeSchedulerClass::Interactive),
                budget: process.record.scheduler_budget,
            });
            controller
                .act_on_process(
                    process.handle,
                    ProcessAction::Renice {
                        class: NativeSchedulerClass::Background,
                        budget: 1,
                    },
                )
                .map_err(|_| 266)?;
            actions.push(format!(
                "{label}.action kind=renice pid={} from={class}/{} to=background/1",
                process.handle.pid, process.record.scheduler_budget
            ));
        }
    }

    if matches!(
        strategy,
        MaintenanceStrategy::BalancedRepair | MaintenanceStrategy::NetworkFirst
    ) && matches!(
        trigger,
        PressureState::NetworkBackpressure | PressureState::MixedPressure
    ) {
        for (handle, record) in devices {
            let socket_pressure = if before.snapshot.total_socket_rx_limit == 0 {
                0
            } else {
                before.socket_pressure_pct
            };
            if record.rx_dropped == 0
                && record.tx_dropped == 0
                && socket_pressure < 80
                && record.tx_inflight_depth < record.tx_inflight_limit
            {
                continue;
            }
            device_rollbacks.push(DeviceRepairRollback {
                handle: handle.clone(),
                record,
            });
            let new_tx_capacity = (record.tx_capacity as usize)
                .saturating_add((record.tx_capacity as usize / 2).max(1));
            let new_rx_capacity = (record.rx_capacity as usize)
                .saturating_add((record.rx_capacity as usize / 2).max(1));
            let new_tx_inflight_limit = (record.tx_inflight_limit as usize)
                .saturating_add((record.tx_inflight_limit as usize / 2).max(1))
                .min(new_tx_capacity.max(1));
            controller
                .configure_interface_admin(
                    &handle,
                    record.mtu as usize,
                    new_tx_capacity,
                    new_rx_capacity,
                    new_tx_inflight_limit,
                    record.admin_up != 0,
                    false,
                )
                .map_err(|_| 266)?;
            actions.push(format!(
                "{label}.action kind=net-admin iface={} tx={}->{} rx={}->{} inflight={}->{}",
                handle.path,
                record.tx_capacity,
                new_tx_capacity,
                record.rx_capacity,
                new_rx_capacity,
                record.tx_inflight_limit,
                new_tx_inflight_limit
            ));
        }
    }

    let reclaim_target = match strategy {
        MaintenanceStrategy::Modernize => 4,
        MaintenanceStrategy::SchedulerFirst => 2,
        _ => 3,
    };
    let reclaimed_pages = runtime
        .reclaim_memory_pressure_global(reclaim_target)
        .map_err(|_| 266)?;
    actions.push(format!(
        "{label}.action kind=vm-reclaim target-pages={reclaim_target} reclaimed-pages={reclaimed_pages}"
    ));

    Ok((
        actions,
        process_rollbacks,
        device_rollbacks,
        reclaimed_pages,
    ))
}

fn rollback_repair_actions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    label: &'static str,
    process_rollbacks: &[ProcessRepairRollback],
    device_rollbacks: &[DeviceRepairRollback],
    actions: &mut Vec<String>,
) -> Result<(), ExitCode> {
    let controller = SystemController::new(runtime);
    for rollback in process_rollbacks {
        controller
            .act_on_process(
                rollback.handle,
                ProcessAction::Renice {
                    class: rollback.class,
                    budget: rollback.budget,
                },
            )
            .map_err(|_| 266)?;
        actions.push(format!(
            "{label}.rollback kind=renice pid={} restored={}/{}",
            rollback.handle.pid,
            scheduler_class_label(rollback.class as u32),
            rollback.budget
        ));
    }
    for rollback in device_rollbacks {
        controller
            .configure_interface_admin(
                &rollback.handle,
                rollback.record.mtu as usize,
                rollback.record.tx_capacity as usize,
                rollback.record.rx_capacity as usize,
                rollback.record.tx_inflight_limit as usize,
                rollback.record.admin_up != 0,
                rollback.record.promiscuous != 0,
            )
            .map_err(|_| 266)?;
        actions.push(format!(
            "{label}.rollback kind=net-admin iface={} restored={}/{}/{}",
            rollback.handle.path,
            rollback.record.tx_capacity,
            rollback.record.rx_capacity,
            rollback.record.tx_inflight_limit
        ));
    }
    Ok(())
}

fn run_system_repair<B: SyscallBackend>(
    runtime: &Runtime<B>,
    _adaptive_state: &mut AdaptiveState,
) -> Result<MaintenanceReport, ExitCode> {
    let controller = SystemController::new(runtime);
    let before = controller.observe_pressure(None).map_err(|_| 266)?;
    let trigger = controller.classify_pressure(&before);
    if !before.verified_core_ok {
        return Err(268);
    }
    let (mut actions, process_rollbacks, device_rollbacks, reclaimed_pages) = apply_repair_actions(
        runtime,
        "repair",
        &before,
        trigger,
        MaintenanceStrategy::BalancedRepair,
    )?;
    let mut after = controller
        .observe_pressure(Some(&before.snapshot))
        .map_err(|_| 266)?;
    let mut verdict = maintenance_verdict(&before, &after);
    if matches!(verdict, SemanticVerdict::Worse) {
        rollback_repair_actions(
            runtime,
            "repair",
            &process_rollbacks,
            &device_rollbacks,
            &mut actions,
        )?;
        after = controller
            .observe_pressure(Some(&before.snapshot))
            .map_err(|_| 266)?;
        verdict = maintenance_verdict(&before, &after);
    }
    Ok(MaintenanceReport {
        label: "repair",
        strategy: "balanced-repair",
        before,
        after,
        trigger,
        reclaimed_pages,
        actions,
        verdict,
    })
}

fn run_system_modernize<B: SyscallBackend>(
    runtime: &Runtime<B>,
    _adaptive_state: &mut AdaptiveState,
) -> Result<MaintenanceReport, ExitCode> {
    let controller = SystemController::new(runtime);
    let before = controller.observe_pressure(None).map_err(|_| 266)?;
    let trigger = controller.classify_pressure(&before);
    if !before.verified_core_ok {
        return Err(268);
    }
    let facts = controller.collect_facts().map_err(|_| 266)?;
    let devices = nextmind_collect_device_entities(&facts);
    let mut actions = Vec::new();
    let mut device_rollbacks = Vec::new();
    for (handle, record) in devices {
        let target_tx = (record.tx_capacity as usize).max(8);
        let target_rx = (record.rx_capacity as usize).max(8);
        let target_inflight = (record.tx_inflight_limit as usize).max(4).min(target_tx);
        if target_tx == record.tx_capacity as usize
            && target_rx == record.rx_capacity as usize
            && target_inflight == record.tx_inflight_limit as usize
            && record.admin_up != 0
            && record.promiscuous == 0
        {
            continue;
        }
        device_rollbacks.push(DeviceRepairRollback {
            handle: handle.clone(),
            record,
        });
        controller
            .configure_interface_admin(
                &handle,
                record.mtu as usize,
                target_tx,
                target_rx,
                target_inflight,
                true,
                false,
            )
            .map_err(|_| 266)?;
        actions.push(format!(
            "modernize.action kind=net-profile iface={} admin=up tx={target_tx} rx={target_rx} inflight={target_inflight}",
            handle.path
        ));
    }
    let reclaimed_pages = runtime.reclaim_memory_pressure_global(4).map_err(|_| 266)?;
    actions.push(format!(
        "modernize.action kind=vm-reclaim target-pages=4 reclaimed-pages={reclaimed_pages}"
    ));

    let mut after = controller
        .observe_pressure(Some(&before.snapshot))
        .map_err(|_| 266)?;
    let mut verdict = maintenance_verdict(&before, &after);
    if matches!(verdict, SemanticVerdict::Worse) {
        rollback_repair_actions(runtime, "modernize", &[], &device_rollbacks, &mut actions)?;
        after = controller
            .observe_pressure(Some(&before.snapshot))
            .map_err(|_| 266)?;
        verdict = maintenance_verdict(&before, &after);
    }
    Ok(MaintenanceReport {
        label: "modernize",
        strategy: "modernize",
        before,
        after,
        trigger,
        reclaimed_pages,
        actions,
        verdict,
    })
}

fn diagnose_ai_incident<B: SyscallBackend>(
    runtime: &Runtime<B>,
    ai_state: &RepairAiState,
) -> Result<
    (
        SystemPressureMetrics,
        PressureState,
        CandidateStrategyScore,
        VerifiedCoreContext,
        IncidentGraph,
        Vec<RepairHypothesis>,
        Vec<CandidateStrategyScore>,
        Vec<RepairCritique>,
    ),
    ExitCode,
> {
    let controller = SystemController::new(runtime);
    let metrics = controller.observe_pressure(None).map_err(|_| 266)?;
    let trigger = controller.classify_pressure(&metrics);
    let verified_core = parse_verified_core_context(runtime, &metrics);
    let ranked = ranked_candidates(&metrics, ai_state);
    let candidate = ranked
        .first()
        .cloned()
        .unwrap_or_else(|| choose_ai_strategy(&metrics, ai_state));
    let graph = build_incident_graph(&metrics, trigger, &verified_core);
    let hypotheses = build_repair_hypotheses(&metrics, ai_state, &verified_core);
    let critique = build_repair_critique(&ranked);
    Ok((
        metrics,
        trigger,
        candidate,
        verified_core,
        graph,
        hypotheses,
        ranked,
        critique,
    ))
}

fn run_ai_repair<B: SyscallBackend>(
    runtime: &Runtime<B>,
    ai_state: &mut RepairAiState,
    adaptive_state: &mut AdaptiveState,
) -> Result<MaintenanceReport, ExitCode> {
    let (before, trigger, candidate, verified_core, graph, hypotheses, ranked, critique) =
        diagnose_ai_incident(runtime, ai_state)?;
    if !before.verified_core_ok {
        return Err(268);
    }
    let label = "repair-ai";
    let (mut actions, process_rollbacks, device_rollbacks, reclaimed_pages) =
        apply_repair_actions(runtime, label, &before, trigger, candidate.strategy)?;
    actions.insert(
        0,
        format!(
            "repair-ai.plan strategy={} score={} reason={} signature=vc:{} rq:{} cpu:{} sock:{} evt:{} drop:{}",
            candidate.strategy.label(),
            candidate.score,
            candidate.reason,
            before.verified_core_violation_count,
            before.run_queue_total,
            before.cpu_utilization_pct,
            before.socket_pressure_pct,
            before.event_queue_pressure_pct,
            before.tx_drop_delta.saturating_add(before.rx_drop_delta),
        ),
    );
    actions.insert(
        1,
        format!(
            "repair-ai.model nodes={} edges={} hypotheses={} candidates={} critique={} kernel-family={} kernel-source={}",
            graph.nodes.len(),
            graph.edges.len(),
            hypotheses.len(),
            ranked.len(),
            critique.len(),
            primary_verified_core_family(&verified_core),
            verified_core.source
        ),
    );
    if let Some(top_hypothesis) = hypotheses.first() {
        actions.insert(
            2,
            format!(
                "repair-ai.choice strategy={} confidence={} rationale={}",
                top_hypothesis.strategy.label(),
                top_hypothesis.confidence,
                top_hypothesis.rationale
            ),
        );
    }
    let controller = SystemController::new(runtime);
    let mut after = controller
        .observe_pressure(Some(&before.snapshot))
        .map_err(|_| 266)?;
    let mut verdict = maintenance_verdict(&before, &after);
    if matches!(verdict, SemanticVerdict::Worse) {
        rollback_repair_actions(
            runtime,
            label,
            &process_rollbacks,
            &device_rollbacks,
            &mut actions,
        )?;
        after = controller
            .observe_pressure(Some(&before.snapshot))
            .map_err(|_| 266)?;
        verdict = maintenance_verdict(&before, &after);
    }
    ai_state.record(RepairEpisode {
        id: ai_state.next_episode_id,
        strategy: candidate.strategy,
        verdict,
        before_score: nextmind_metrics_score(&before) as u32,
        after_score: nextmind_metrics_score(&after) as u32,
        signature: incident_signature(&before),
    });
    let _ = adaptive_state;
    Ok(MaintenanceReport {
        label,
        strategy: candidate.strategy.label(),
        before,
        after,
        trigger,
        reclaimed_pages,
        actions,
        verdict,
    })
}

fn emit_maintenance_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
    report: &MaintenanceReport,
) -> Result<(), ExitCode> {
    render_maintenance_metrics(
        runtime,
        &format!("{}.before", report.label),
        &report.before,
        report.trigger,
    )?;
    for action in &report.actions {
        write_line(runtime, action)?;
    }
    render_maintenance_metrics(
        runtime,
        &format!("{}.after", report.label),
        &report.after,
        SystemController::new(runtime).classify_pressure(&report.after),
    )?;
    write_line(
        runtime,
        &format!(
            "system.{}.verdict={} strategy={} reclaimed-pages={} verified-core={} violations={}",
            report.label,
            semantic_verdict_name(report.verdict),
            report.strategy,
            report.reclaimed_pages,
            report.after.verified_core_ok,
            report.after.verified_core_violation_count
        ),
    )
}

pub fn try_handle_repair_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    ai_state: &mut RepairAiState,
    adaptive_state: &mut AdaptiveState,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if line == "repair-system" {
        match run_system_repair(runtime, adaptive_state) {
            Ok(report) => {
                *last_status = 0;
                return Some(emit_maintenance_report(runtime, &report));
            }
            Err(268) => {
                *last_status = 1;
                return Some(
                    write_line(
                        runtime,
                        "system.repair.refusal reason=verified-core-degraded action=manual-kernel-repair-required",
                    )
                    .map_err(|_| 266),
                );
            }
            Err(code) => {
                *last_status = code;
                return Some(Ok(()));
            }
        }
    }
    if line == "modernize-system" {
        match run_system_modernize(runtime, adaptive_state) {
            Ok(report) => {
                *last_status = 0;
                return Some(emit_maintenance_report(runtime, &report));
            }
            Err(268) => {
                *last_status = 1;
                return Some(
                    write_line(
                        runtime,
                        "system.modernize.refusal reason=verified-core-degraded action=repair-system-first",
                    )
                    .map_err(|_| 266),
                );
            }
            Err(code) => {
                *last_status = code;
                return Some(Ok(()));
            }
        }
    }
    if line == "repair-ai.diagnose" {
        match diagnose_ai_incident(runtime, ai_state) {
            Ok((
                metrics,
                trigger,
                candidate,
                verified_core,
                graph,
                hypotheses,
                ranked,
                critique,
            )) => {
                *last_status = 0;
                let result = (|| -> Result<(), ExitCode> {
                    write_line(
                        runtime,
                        &format!(
                            "repair-ai.diagnose state={} strategy={} score={} reason={} verified-core={} violations={} runq={} cpu={} socket={} event={}",
                            nextmind_pressure_state_label(trigger),
                            candidate.strategy.label(),
                            candidate.score,
                            candidate.reason,
                            metrics.verified_core_ok,
                            metrics.verified_core_violation_count,
                            metrics.run_queue_total,
                            metrics.cpu_utilization_pct,
                            metrics.socket_pressure_pct,
                            metrics.event_queue_pressure_pct
                        ),
                    )?;
                    emit_verified_core_context(runtime, &verified_core)?;
                    emit_incident_graph(runtime, &graph)?;
                    emit_repair_hypotheses(runtime, &hypotheses)?;
                    emit_candidate_scores(runtime, &ranked)?;
                    emit_repair_critique(runtime, &critique)?;
                    Ok(())
                })();
                return Some(result);
            }
            Err(code) => {
                *last_status = code;
                return Some(Ok(()));
            }
        }
    }
    if line == "repair-ai.repair" {
        match run_ai_repair(runtime, ai_state, adaptive_state) {
            Ok(report) => {
                *last_status = 0;
                return Some(emit_maintenance_report(runtime, &report));
            }
            Err(268) => {
                *last_status = 1;
                return Some(
                    write_line(
                        runtime,
                        "repair-ai.refusal reason=verified-core-degraded action=manual-kernel-repair-required",
                    )
                    .map_err(|_| 266),
                );
            }
            Err(code) => {
                *last_status = code;
                return Some(Ok(()));
            }
        }
    }
    if line == "repair-ai.memory" {
        *last_status = 0;
        if ai_state.episodes().next().is_none() {
            return Some(write_line(runtime, "repair-ai.memory entries=0").map_err(|_| 266));
        }
        for episode in ai_state.episodes() {
            if write_line(
                runtime,
                &format!(
                    "repair-ai.memory id={} strategy={} verdict={} score={}->{} signature=vc:{} rq:{} cpu:{} sock:{} evt:{} drop:{}",
                    episode.id,
                    episode.strategy.label(),
                    semantic_verdict_name(episode.verdict),
                    episode.before_score,
                    episode.after_score,
                    episode.signature.violation_count,
                    episode.signature.runq_bucket,
                    episode.signature.cpu_bucket,
                    episode.signature.socket_bucket,
                    episode.signature.event_bucket,
                    episode.signature.drop_bucket,
                ),
            )
            .is_err()
            {
                return Some(Err(266));
            }
        }
        return Some(Ok(()));
    }
    if let Some(path) = line.strip_prefix("repair-ai.save ").map(str::trim) {
        *last_status = 0;
        return Some(save_repair_ai_memory(runtime, ai_state, path));
    }
    if let Some(path) = line.strip_prefix("repair-ai.load ").map(str::trim) {
        match load_repair_ai_memory(runtime, ai_state, path) {
            Ok(()) => {
                *last_status = 0;
                return Some(Ok(()));
            }
            Err(269) => {
                *last_status = 1;
                return Some(
                    write_line(
                        runtime,
                        "repair-ai.memory.refusal reason=corrupt-memory-image action=discard-or-rebuild",
                    )
                    .map_err(|_| 266),
                );
            }
            Err(code) => {
                *last_status = code;
                return Some(Ok(()));
            }
        }
    }
    None
}
