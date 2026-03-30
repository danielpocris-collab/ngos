use super::*;

pub(super) fn build_causal_ledger(
    capsule: &CrashCapsule,
) -> [CausalLedgerEdge; CAUSAL_LEDGER_LIMIT] {
    let focused = focused_events(capsule);
    let mut ledger = [CausalLedgerEdge::EMPTY; CAUSAL_LEDGER_LIMIT];
    let mut next = 0usize;
    for entry in focused.into_iter().flatten() {
        if next >= ledger.len() {
            break;
        }
        let kind = match dominant_path_for_trace(entry, capsule) {
            DiagnosticsPath::Syscall => CausalEdgeKind::Validation,
            DiagnosticsPath::Block => CausalEdgeKind::Submit,
            DiagnosticsPath::Irq => CausalEdgeKind::Irq,
            DiagnosticsPath::Completion => CausalEdgeKind::Completion,
            DiagnosticsPath::Fault => CausalEdgeKind::Fault,
            DiagnosticsPath::None => CausalEdgeKind::Divergence,
        };
        ledger[next] = CausalLedgerEdge {
            valid: true,
            kind,
            stage: entry.stage,
            cpu_slot: entry.cpu_slot,
            sequence: entry.sequence,
            request_id: focused_request_id(entry, &capsule.replay),
            completion_id: focused_completion_id(entry, &capsule.replay),
            irq_id: focused_irq_id(entry, &capsule.replay),
            reason: semantic_reason_for_trace(entry, capsule),
        };
        next += 1;
    }
    ledger
}

pub(super) fn build_invariant_coverage_map(capsule: &CrashCapsule) -> InvariantCoverageMap {
    let mut map = InvariantCoverageMap::EMPTY;
    map.entries[0] = invariant_entry(
        "request-validation",
        capsule,
        DiagnosticsPath::Syscall,
        semantic_reason_hit(capsule, DiagnosticsPath::Syscall),
        "write boundary remained validated",
        "request validation edge missing or weakened",
    );
    map.entries[1] = invariant_entry(
        "submit-revalidation",
        capsule,
        DiagnosticsPath::Block,
        semantic_reason_hit(capsule, DiagnosticsPath::Block),
        "submit boundary preserved request contract",
        "submit path did not preserve request contract",
    );
    map.entries[2] = invariant_entry(
        "completion-publish",
        capsule,
        DiagnosticsPath::Completion,
        semantic_reason_hit(capsule, DiagnosticsPath::Completion),
        "completion stayed observable and correlated",
        "completion publish/observe boundary diverged",
    );
    map.entries[3] = invariant_entry(
        "irq-correlation",
        capsule,
        DiagnosticsPath::Irq,
        capsule.replay.irq_id != 0,
        "irq correlation id stayed attached",
        "irq path missing or correlation lost",
    );
    map.entries[4] = invariant_entry(
        "fault-containment",
        capsule,
        DiagnosticsPath::Fault,
        capsule.fault.valid,
        "fault captured with capsule and focused trace",
        "fault path not captured cleanly",
    );
    if dominant_violation_text(capsule) != "none" {
        map.entries[5] = InvariantCoverageEntry {
            name: "memory-guarding",
            status: InvariantStatus::Violated,
            stage: capsule.failure_signature.stage,
            path: capsule.failure_signature.path,
            reason: dominant_violation_text(capsule),
        };
    } else {
        map.entries[5] = InvariantCoverageEntry {
            name: "memory-guarding",
            status: InvariantStatus::Verified,
            stage: capsule.failure_signature.stage,
            path: capsule.failure_signature.path,
            reason: "no guard/watch violation on focused path",
        };
    }
    map
}

pub(super) fn compare_differential_flows(
    current: &CrashCapsule,
    baseline: &CrashCapsule,
) -> DifferentialFlowReport {
    if !baseline.valid {
        return DifferentialFlowReport::EMPTY;
    }
    let reason = if baseline.failure_signature_id == current.failure_signature_id {
        "same-pattern-baseline"
    } else {
        "closest-available-baseline"
    };
    DifferentialFlowReport {
        has_baseline: true,
        stable_prefix: current.stable_prefix_length,
        unstable_suffix: current.unstable_suffix_length,
        first_divergence_sequence: current.first_divergence_sequence,
        baseline_signature_id: baseline.failure_signature_id,
        reason,
    }
}

pub(super) fn detect_semantic_race(capsule: &CrashCapsule) -> SemanticRaceSignal {
    let mut signal = SemanticRaceSignal::EMPTY;
    if !capsule.valid {
        return signal;
    }
    let score = 100u16.saturating_sub(consistency_score(capsule));
    if score < 35 {
        return signal;
    }
    let focused = focused_events(capsule);
    let mut cpu_a = 0u16;
    let mut cpu_b = 0u16;
    for pair in focused.windows(2) {
        if let [Some(left), Some(right)] = pair {
            if left.cpu_slot != right.cpu_slot {
                cpu_a = left.cpu_slot;
                cpu_b = right.cpu_slot;
                break;
            }
        }
    }
    signal.likely = true;
    signal.score = score;
    signal.request_id = capsule.replay.request_id;
    signal.completion_id = capsule.replay.completion_id;
    signal.irq_id = capsule.replay.irq_id;
    signal.cpu_a = cpu_a;
    signal.cpu_b = cpu_b;
    signal.reason = if cpu_a != cpu_b {
        "cross-cpu divergence and low consistency suggest semantic race"
    } else {
        "unstable suffix and low consistency suggest timing-sensitive semantic race"
    };
    signal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_race_detection_ignores_absent_crash_capsule() {
        let signal = detect_semantic_race(&CrashCapsule::EMPTY);
        assert_eq!(signal, SemanticRaceSignal::EMPTY);
    }
}

pub(super) fn replay_window_summary(capsule: &CrashCapsule) -> ReplayWindowSummary {
    ReplayWindowSummary {
        request_id: capsule.replay.request_id,
        completion_id: capsule.replay.completion_id,
        irq_id: capsule.replay.irq_id,
        sequence: capsule.replay.sequence,
        stable_prefix: capsule.stable_prefix_length,
        unstable_suffix: capsule.unstable_suffix_length,
        first_divergence_sequence: capsule.first_divergence_sequence,
        deterministic: consistency_score(capsule) >= 75,
        reason: if consistency_score(capsule) >= 75 {
            "window mostly deterministic"
        } else {
            "window carries visible nondeterminism"
        },
    }
}

pub(super) fn earliest_preventable_boundary(capsule: &CrashCapsule) -> EarliestPreventableBoundary {
    let suspect = capsule
        .suspects
        .into_iter()
        .find(|suspect| suspect.valid)
        .unwrap_or(SuspectPoint::EMPTY);
    if !suspect.valid {
        return EarliestPreventableBoundary::EMPTY;
    }
    let path = dominant_path_for_suspect(suspect, capsule);
    let (reason, action) = match path {
        DiagnosticsPath::Syscall => (
            "earliest stable suspect is request validation boundary",
            "strengthen validation before request leaves write_syscall",
        ),
        DiagnosticsPath::Block => (
            "earliest stable suspect is submit boundary",
            "revalidate request and preserve correlation before queue submit",
        ),
        DiagnosticsPath::Completion => (
            "earliest stable suspect is completion publish/observe boundary",
            "preserve completion contract before publish/read visibility",
        ),
        DiagnosticsPath::Irq => (
            "earliest stable suspect is irq-to-completion handoff",
            "retain correlation and ordering across irq completion edge",
        ),
        DiagnosticsPath::Fault => (
            "earliest stable suspect is faulting boundary itself",
            "inspect faulting boundary for missing containment invariant",
        ),
        DiagnosticsPath::None => (
            "earliest stable suspect remains on focused-path divergence",
            "inspect first focused-path divergence and restore local invariant",
        ),
    };
    EarliestPreventableBoundary {
        valid: true,
        stage: suspect.stage,
        path,
        sequence: suspect.event_sequence,
        request_id: suspect.request_id,
        completion_id: suspect.completion_id,
        irq_id: suspect.irq_id,
        reason,
        action,
    }
}

pub(super) fn semantic_reason_hit(capsule: &CrashCapsule, path: DiagnosticsPath) -> bool {
    match path {
        DiagnosticsPath::Syscall => {
            capsule.semantic_reasons.write_syscall_reject_fault != 0
                || capsule.semantic_reasons.write_syscall_reject_guard != 0
                || capsule.semantic_reasons.write_syscall_reject_watch != 0
        }
        DiagnosticsPath::Block => {
            capsule.semantic_reasons.submit_device_request_reject_fault != 0
                || capsule.semantic_reasons.submit_device_request_reject_guard != 0
                || capsule.semantic_reasons.submit_device_request_reject_watch != 0
        }
        DiagnosticsPath::Completion => {
            capsule.semantic_reasons.completion_publish_reject_fault != 0
                || capsule.semantic_reasons.completion_publish_reject_guard != 0
                || capsule.semantic_reasons.completion_publish_reject_watch != 0
                || capsule.semantic_reasons.completion_read_reject_fault != 0
                || capsule.semantic_reasons.completion_read_reject_guard != 0
                || capsule.semantic_reasons.completion_read_reject_watch != 0
        }
        DiagnosticsPath::Irq => capsule.replay.irq_id != 0,
        DiagnosticsPath::Fault => capsule.fault.valid,
        DiagnosticsPath::None => false,
    }
}

pub(super) fn invariant_entry(
    name: &'static str,
    capsule: &CrashCapsule,
    path: DiagnosticsPath,
    observed: bool,
    ok_reason: &'static str,
    fail_reason: &'static str,
) -> InvariantCoverageEntry {
    let status = if observed {
        if capsule.failure_signature.path == path || dominant_violation_text(capsule) != "none" {
            InvariantStatus::Violated
        } else {
            InvariantStatus::Verified
        }
    } else {
        InvariantStatus::Missing
    };
    InvariantCoverageEntry {
        name,
        status,
        stage: if capsule.failure_signature.stage != 0 {
            capsule.failure_signature.stage
        } else {
            capsule.window.path as u16
        },
        path,
        reason: match status {
            InvariantStatus::Verified => ok_reason,
            InvariantStatus::Violated => fail_reason,
            InvariantStatus::Missing => "no evidence this invariant executed on focused path",
        },
    }
}
