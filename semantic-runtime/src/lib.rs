#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use ngos_user_abi::NativeSystemSnapshotRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureState {
    Stable,
    HighSchedulerPressure,
    NetworkBackpressure,
    MixedPressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticClass {
    Dialog,
    Process,
    Memory,
    Interrupt,
    Power,
    Device,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticCapability {
    Converse,
    Schedule,
    Signal,
    Protect,
    Throttle,
    Observe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSemantic {
    pub class: SemanticClass,
    pub capabilities: Vec<SemanticCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitiveTier {
    Reflex,
    Operational,
    Reasoning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeMode {
    Economy,
    Balanced,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticObservation {
    pub cpu_load: u16,
    pub mem_pressure: u16,
    pub anomaly_score: u16,
    pub thermal_c: i16,
}

impl SemanticObservation {
    pub const fn critical(&self) -> bool {
        self.anomaly_score > 70 || self.thermal_c > 90
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptiveStateSnapshot {
    pub stress: u16,
    pub focus: u16,
    pub tier: CognitiveTier,
    pub compute_mode: ComputeMode,
    pub budget_points: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptiveState {
    stress: u16,
    focus: u16,
}

impl AdaptiveState {
    pub const fn new() -> Self {
        Self {
            stress: 10,
            focus: 50,
        }
    }

    pub fn record(&mut self, observation: &SemanticObservation) {
        let delta = if observation.critical() {
            12
        } else {
            observation.cpu_load / 10 + observation.mem_pressure / 10
        };
        self.stress = self
            .stress
            .saturating_add(delta)
            .saturating_sub(self.stress.min(5))
            .min(100);

        let adjust = if observation.anomaly_score < 20 { 8 } else { 3 };
        self.focus = self
            .focus
            .saturating_add(adjust)
            .saturating_sub(observation.mem_pressure / 15)
            .clamp(10, 100);
    }

    pub const fn tier(&self) -> CognitiveTier {
        if self.stress > 70 || self.focus < 20 {
            CognitiveTier::Reasoning
        } else if self.stress > 35 || self.focus < 40 {
            CognitiveTier::Operational
        } else {
            CognitiveTier::Reflex
        }
    }

    pub const fn compute_mode(&self) -> ComputeMode {
        if self.stress > 75 || self.focus < 25 {
            ComputeMode::Economy
        } else if self.stress > 40 || self.focus < 45 {
            ComputeMode::Balanced
        } else {
            ComputeMode::Deep
        }
    }

    pub const fn budget_points(&self) -> u16 {
        match self.compute_mode() {
            ComputeMode::Economy => 25,
            ComputeMode::Balanced => 55,
            ComputeMode::Deep => 90,
        }
    }

    pub const fn snapshot(&self) -> AdaptiveStateSnapshot {
        AdaptiveStateSnapshot {
            stress: self.stress,
            focus: self.focus,
            tier: self.tier(),
            compute_mode: self.compute_mode(),
            budget_points: self.budget_points(),
        }
    }
}

impl Default for AdaptiveState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPressureMetrics {
    pub snapshot: NativeSystemSnapshotRecord,
    pub cpu_utilization_pct: u32,
    pub run_queue_total: u64,
    pub run_queue_latency_critical: u64,
    pub run_queue_interactive: u64,
    pub run_queue_normal: u64,
    pub run_queue_background: u64,
    pub socket_pressure_pct: u32,
    pub event_queue_pressure_pct: u32,
    pub tx_drop_delta: u64,
    pub rx_drop_delta: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSystemState {
    pub metrics: SystemPressureMetrics,
    pub pressure: PressureState,
    pub channel: String,
    pub semantic: EventSemantic,
    pub observation: SemanticObservation,
    pub adaptive: AdaptiveStateSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticEntityKind {
    Process,
    Device,
    Socket,
    Resource,
    Contract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticPolicyView {
    pub cpu_mask: u64,
    pub policy_fingerprint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEntity {
    pub kind: SemanticEntityKind,
    pub subject: String,
    pub semantic: EventSemantic,
    pub policy: SemanticPolicyView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticActionRecord {
    pub reason: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticVerdict {
    Improved,
    NoChange,
    Worse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDecisionPlan {
    pub trigger: PressureState,
    pub semantic: EventSemantic,
    pub observation: SemanticObservation,
    pub adaptive: AdaptiveStateSnapshot,
    pub before: SystemPressureMetrics,
    pub actions: Vec<SemanticActionRecord>,
}

const CONTEXT_CAPACITY: usize = 16;
const CONTEXT_ENTRY_MAX_CHARS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticContextEntry {
    pub sequence: u64,
    pub channel: String,
    pub semantic_class: SemanticClass,
    pub payload: String,
    pub action_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticContext {
    next_sequence: u64,
    entries: Vec<SemanticContextEntry>,
}

impl SemanticContext {
    pub const fn new() -> Self {
        Self {
            next_sequence: 1,
            entries: Vec::new(),
        }
    }

    pub fn push(
        &mut self,
        channel: &str,
        semantic: &EventSemantic,
        payload: &str,
        actions: &[SemanticActionRecord],
    ) {
        let action_summary = if actions.is_empty() {
            String::from("no-action")
        } else {
            actions
                .iter()
                .map(|action| format!("{}:{}", action.reason, action.detail))
                .collect::<Vec<_>>()
                .join(" | ")
        };
        if self.entries.len() == CONTEXT_CAPACITY {
            self.entries.remove(0);
        }
        self.entries.push(SemanticContextEntry {
            sequence: self.next_sequence,
            channel: channel.to_string(),
            semantic_class: semantic.class,
            payload: truncate_ascii(payload.to_string(), CONTEXT_ENTRY_MAX_CHARS),
            action_summary: truncate_ascii(action_summary, CONTEXT_ENTRY_MAX_CHARS),
        });
        self.next_sequence = self.next_sequence.saturating_add(1);
    }

    pub fn tail(&self, mode: ComputeMode) -> String {
        let window = match mode {
            ComputeMode::Economy => 3,
            ComputeMode::Balanced => 6,
            ComputeMode::Deep => 10,
        };
        let start = self.entries.len().saturating_sub(window);
        let mut text = String::new();
        for entry in self.entries.iter().skip(start) {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&format!(
                "#{} class={} channel={} payload={} -> {}",
                entry.sequence,
                semantic_class_name(entry.semantic_class),
                entry.channel,
                entry.payload,
                entry.action_summary
            ));
        }
        text
    }

    pub const fn event_count(&self) -> u64 {
        self.next_sequence.saturating_sub(1)
    }
}

impl Default for SemanticContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDiagnostics {
    pub stress: u16,
    pub focus: u16,
    pub tier: CognitiveTier,
    pub compute_mode: ComputeMode,
    pub budget_points: u16,
    pub event_count: u64,
    pub context_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticFeedbackEntry {
    pub subject: String,
    pub action: String,
    pub policy_epoch: u32,
    pub success_count: u64,
    pub failure_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticFeedbackStore {
    entries: Vec<SemanticFeedbackEntry>,
}

impl SemanticFeedbackStore {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, subject: &str, action: &str, policy_epoch: u32, success: bool) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.subject == subject && entry.action == action)
        {
            entry.policy_epoch = policy_epoch;
            if success {
                entry.success_count += 1;
            } else {
                entry.failure_count += 1;
            }
            return;
        }
        self.entries.push(SemanticFeedbackEntry {
            subject: subject.to_string(),
            action: action.to_string(),
            policy_epoch,
            success_count: u64::from(success),
            failure_count: u64::from(!success),
        });
    }

    pub fn entries(&self) -> &[SemanticFeedbackEntry] {
        &self.entries
    }
}

impl Default for SemanticFeedbackStore {
    fn default() -> Self {
        Self::new()
    }
}

pub type CpuMask = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuLoadStats {
    pub run_events: u64,
    pub idle_events: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticCpuTopologyEntry {
    pub cpu_index: usize,
    pub apic_id: u32,
    pub launched: bool,
    pub online: bool,
    pub load: CpuLoadStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTopologySnapshot {
    pub online_cpus: usize,
    pub entries: Vec<SemanticCpuTopologyEntry>,
}

pub const fn pressure_channel_name(state: PressureState) -> &'static str {
    match state {
        PressureState::Stable => "proc::steady",
        PressureState::HighSchedulerPressure => "proc::pressure",
        PressureState::NetworkBackpressure => "dev::network-pressure",
        PressureState::MixedPressure => "proc::network-pressure",
    }
}

pub fn semantic_for_channel(channel: &str) -> EventSemantic {
    match channel {
        c if c.starts_with("dialog::") => EventSemantic {
            class: SemanticClass::Dialog,
            capabilities: vec![SemanticCapability::Converse, SemanticCapability::Observe],
        },
        c if c.starts_with("proc::") => EventSemantic {
            class: SemanticClass::Process,
            capabilities: vec![SemanticCapability::Schedule, SemanticCapability::Observe],
        },
        c if c.starts_with("vm::") || c.starts_with("mm::") => EventSemantic {
            class: SemanticClass::Memory,
            capabilities: vec![SemanticCapability::Protect, SemanticCapability::Signal],
        },
        c if c.contains("thermal") => EventSemantic {
            class: SemanticClass::Power,
            capabilities: vec![SemanticCapability::Throttle, SemanticCapability::Observe],
        },
        c if c.starts_with("irq::") => EventSemantic {
            class: SemanticClass::Interrupt,
            capabilities: vec![SemanticCapability::Observe],
        },
        c if c.starts_with("dma::") || c.starts_with("dev::") => EventSemantic {
            class: SemanticClass::Device,
            capabilities: vec![SemanticCapability::Observe, SemanticCapability::Throttle],
        },
        _ => EventSemantic {
            class: SemanticClass::Unknown,
            capabilities: vec![SemanticCapability::Observe],
        },
    }
}

pub const fn semantic_class_name(class: SemanticClass) -> &'static str {
    match class {
        SemanticClass::Dialog => "dialog",
        SemanticClass::Process => "process",
        SemanticClass::Memory => "memory",
        SemanticClass::Interrupt => "interrupt",
        SemanticClass::Power => "power",
        SemanticClass::Device => "device",
        SemanticClass::Unknown => "unknown",
    }
}

pub const fn semantic_capability_name(capability: SemanticCapability) -> &'static str {
    match capability {
        SemanticCapability::Converse => "converse",
        SemanticCapability::Schedule => "schedule",
        SemanticCapability::Signal => "signal",
        SemanticCapability::Protect => "protect",
        SemanticCapability::Throttle => "throttle",
        SemanticCapability::Observe => "observe",
    }
}

pub fn semantic_capabilities_csv(semantic: &EventSemantic) -> String {
    let mut text = String::new();
    for (index, capability) in semantic.capabilities.iter().enumerate() {
        if index != 0 {
            text.push(',');
        }
        text.push_str(semantic_capability_name(*capability));
    }
    text
}

pub const fn semantic_entity_kind_name(kind: SemanticEntityKind) -> &'static str {
    match kind {
        SemanticEntityKind::Process => "process",
        SemanticEntityKind::Device => "device",
        SemanticEntityKind::Socket => "socket",
        SemanticEntityKind::Resource => "resource",
        SemanticEntityKind::Contract => "contract",
    }
}

pub const fn semantic_verdict_name(verdict: SemanticVerdict) -> &'static str {
    match verdict {
        SemanticVerdict::Improved => "improved",
        SemanticVerdict::NoChange => "no_change",
        SemanticVerdict::Worse => "worse",
    }
}

pub const fn cpu_mask_for(index: usize) -> CpuMask {
    if index >= 64 {
        CpuMask::MAX
    } else {
        1u64 << index
    }
}

pub fn select_cpu(stats: &[CpuLoadStats], online: usize, excludes: &[usize]) -> Option<usize> {
    if stats.is_empty() {
        return None;
    }
    let limit = core::cmp::min(online.max(1), stats.len());
    let mut best: Option<(usize, u64)> = None;
    for (cpu, stat) in stats.iter().copied().enumerate().take(limit) {
        if excludes.contains(&cpu) {
            continue;
        }
        let total = stat.run_events.saturating_add(stat.idle_events).max(1);
        let ratio = stat.run_events.saturating_mul(1_000_000) / total;
        match best {
            None => best = Some((cpu, ratio)),
            Some((_, best_ratio)) if ratio < best_ratio => best = Some((cpu, ratio)),
            _ => {}
        }
    }
    best.map(|(cpu, _)| cpu)
}

pub fn load_percent(stat: &CpuLoadStats) -> u64 {
    let total = stat.run_events.saturating_add(stat.idle_events).max(1);
    stat.run_events.saturating_mul(100) / total
}

fn truncate_ascii(mut text: String, limit: usize) -> String {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit.saturating_sub(3);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push_str("...");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_channel_maps_to_schedule_and_observe() {
        let semantic = semantic_for_channel("proc::blocked");
        assert_eq!(semantic.class, SemanticClass::Process);
        assert!(
            semantic
                .capabilities
                .contains(&SemanticCapability::Schedule)
        );
        assert!(semantic.capabilities.contains(&SemanticCapability::Observe));
    }

    #[test]
    fn adaptive_state_promotes_tier_for_critical_observation() {
        let mut state = AdaptiveState::new();
        for _ in 0..4 {
            state.record(&SemanticObservation {
                cpu_load: 95,
                mem_pressure: 90,
                anomaly_score: 85,
                thermal_c: 96,
            });
        }
        let snapshot = state.snapshot();
        assert!(matches!(
            snapshot.tier,
            CognitiveTier::Operational | CognitiveTier::Reasoning
        ));
        assert!(snapshot.budget_points <= 55);
    }

    #[test]
    fn load_helpers_select_lowest_load_cpu() {
        let stats = [
            CpuLoadStats {
                run_events: 90,
                idle_events: 10,
            },
            CpuLoadStats {
                run_events: 40,
                idle_events: 60,
            },
            CpuLoadStats {
                run_events: 70,
                idle_events: 30,
            },
        ];
        assert_eq!(select_cpu(&stats, 3, &[]), Some(1));
        assert_eq!(select_cpu(&stats, 3, &[1]), Some(2));
        assert_eq!(load_percent(&stats[1]), 40);
        assert_eq!(cpu_mask_for(1), 0x2);
    }

    #[test]
    fn semantic_context_tail_respects_compute_mode_window() {
        let semantic = semantic_for_channel("proc::blocked");
        let mut context = SemanticContext::new();
        for index in 0..8u64 {
            context.push(
                "proc::blocked",
                &semantic,
                &format!("evt-{index}"),
                &[SemanticActionRecord {
                    reason: String::from("observe"),
                    detail: format!("detail-{index}"),
                }],
            );
        }
        let tail = context.tail(ComputeMode::Economy);
        assert!(tail.contains("evt-7"));
        assert!(tail.contains("evt-6"));
        assert!(!tail.contains("evt-1"));
    }
}
