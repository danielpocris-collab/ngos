use super::*;

pub(super) fn build_memory_lineage_export() -> [ExportMemoryLineageEntry; 8] {
    let lineage = unsafe { *TRACE_STORAGE.memory_lineage.get() };
    let mut ordered = [MemoryLineageEntry::EMPTY; MEMORY_LINEAGE_CAPACITY];
    let mut count = 0usize;
    for entry in lineage.iter().copied().filter(|entry| entry.valid) {
        let mut insert = count;
        while insert > 0 && ordered[insert - 1].version_id < entry.version_id {
            ordered[insert] = ordered[insert - 1];
            insert -= 1;
        }
        if insert < ordered.len() {
            ordered[insert] = entry;
            count = (count + 1).min(ordered.len());
        }
    }
    let mut out = [ExportMemoryLineageEntry::EMPTY; 8];
    let mut next = 0usize;
    while next < out.len() && next < count {
        let entry = ordered[next];
        out[next] = ExportMemoryLineageEntry {
            valid: true,
            version_id: entry.version_id,
            parent_version_id: entry.parent_version_id,
            sequence: entry.sequence,
            address_space_id: entry.address_space_id,
            base: entry.base,
            len: entry.len,
            object_id: entry.object_id,
            request_id: entry.request_id,
            completion_id: entry.completion_id,
            irq_id: entry.irq_id,
            cpu_slot: entry.cpu_slot,
            stage: entry.stage,
            kind: entry.kind,
            bytes_changed: entry.bytes_changed,
            digest: entry.digest,
        };
        next += 1;
    }
    out
}

pub(super) fn build_export_bundle(capsule: &CrashCapsule) -> DiagnosticsExportBundle {
    if !capsule.valid {
        return DiagnosticsExportBundle::EMPTY;
    }
    let focused_events = focused_events(capsule);
    let mut focused = [ExportFocusedEntry::EMPTY; FOCUSED_TRACE_HISTORY];
    let mut focused_next = 0usize;
    for entry in focused_events.into_iter().flatten() {
        if focused_next >= focused.len() {
            break;
        }
        focused[focused_next] = ExportFocusedEntry {
            valid: true,
            sequence: entry.sequence,
            cpu_slot: entry.cpu_slot,
            stage: entry.stage,
            kind: entry.kind,
            path: dominant_path_for_trace(entry, capsule),
            request_id: focused_request_id(entry, &capsule.replay),
            completion_id: focused_completion_id(entry, &capsule.replay),
            irq_id: focused_irq_id(entry, &capsule.replay),
            result: focused_result(entry),
            reason: focused_reason(entry),
        };
        focused_next += 1;
    }
    let mut suspects = [ExportSuspectEntry::EMPTY; SUSPECT_LIMIT];
    for (index, suspect) in capsule
        .suspects
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, s)| s.valid)
    {
        suspects[index] = ExportSuspectEntry {
            valid: true,
            rank: index as u8 + 1,
            stage: suspect.stage,
            cpu_slot: suspect.cpu_slot,
            request_id: suspect.request_id,
            completion_id: suspect.completion_id,
            irq_id: suspect.irq_id,
            score: suspect.score,
            confidence: suspect_confidence(suspect, capsule),
            reason: suspect_reason_name(suspect.reason_code),
            local_violation: local_violation_for_suspect(suspect, capsule),
        };
    }
    let failure_history = build_failure_history_export();
    let patterns = build_pattern_export();
    let smp = build_smp_timeline_from_trace(capsule);
    DiagnosticsExportBundle {
        valid: true,
        generation: capsule.generation,
        failure_signature_id: capsule.failure_signature_id,
        stable_prefix_length: capsule.stable_prefix_length,
        unstable_suffix_length: capsule.unstable_suffix_length,
        first_divergence_sequence: capsule.first_divergence_sequence,
        memory: build_memory_violation_summary(capsule),
        memory_lineage: build_memory_lineage_summary(),
        focused,
        memory_lineage_tail: build_memory_lineage_export(),
        suspects,
        failure_history,
        patterns,
        causal: build_causal_ledger(capsule),
        reprobe: capsule.reprobe,
        smp_divergence: build_smp_divergence_summary(&smp),
        smp,
    }
}

pub(super) fn build_failure_history_export() -> [ExportFailureHistoryEntry; FAILURE_HISTORY_CAPACITY]
{
    let history = unsafe { *TRACE_STORAGE.crash_history.get() };
    let mut out = [ExportFailureHistoryEntry::EMPTY; FAILURE_HISTORY_CAPACITY];
    let mut next = 0usize;
    for entry in history.iter().copied().filter(|entry| entry.valid) {
        if next >= out.len() {
            break;
        }
        let capsule = crash_capsule_from_history(entry, &crash_capsule());
        out[next] = ExportFailureHistoryEntry {
            valid: true,
            generation: entry.generation,
            signature_id: entry.signature_id,
            request_id: entry.replay.request_id,
            completion_id: entry.replay.completion_id,
            irq_id: entry.replay.irq_id,
            stage: entry.fault.stage,
            path: dominant_failure_path(&capsule).0,
            stable_prefix_length: entry.stable_prefix_length,
            unstable_suffix_length: entry.unstable_suffix_length,
            first_bad_sequence: entry.first_bad_sequence,
            first_divergence_sequence: entry.first_divergence_sequence,
        };
        next += 1;
    }
    out
}

pub(super) fn build_pattern_export() -> [ExportPatternEntry; 3] {
    let patterns = unsafe { *TRACE_STORAGE.pattern_history.get() };
    let mut top = [FailurePatternSummary::EMPTY; 3];
    for entry in patterns.iter().copied().filter(|entry| entry.valid) {
        insert_top_pattern(&mut top, entry);
    }
    let mut out = [ExportPatternEntry::EMPTY; 3];
    for (index, entry) in top
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, entry)| entry.valid)
    {
        out[index] = ExportPatternEntry {
            valid: true,
            rank: index as u8 + 1,
            signature_id: entry.signature_id,
            frequency: entry.frequency,
            last_generation: entry.last_generation,
            last_seen_sequence: entry.last_seen_sequence,
            path: entry.path,
            stage: entry.most_common_stage,
            first_bad_kind: entry.most_common_first_bad_kind,
        };
    }
    out
}

pub(super) fn build_smp_timeline_from_trace(
    capsule: &CrashCapsule,
) -> [SmpTimelineCpuSummary; MAX_TRACE_CPUS] {
    let mut out = [SmpTimelineCpuSummary::EMPTY; MAX_TRACE_CPUS];
    let mut cpu_slot = 0usize;
    while cpu_slot < MAX_TRACE_CPUS {
        let mut first = TraceRecord::EMPTY;
        let mut last = TraceRecord::EMPTY;
        let mut count = 0u16;
        let mut request_id = 0u64;
        let mut completion_id = 0u64;
        let mut irq_id = 0u64;
        let mut dominant_path = DiagnosticsPath::None;
        let mut last_path = DiagnosticsPath::None;
        for entry in capsule.trace_tail[cpu_slot]
            .iter()
            .copied()
            .filter(|entry| entry.sequence != 0)
        {
            if count == 0 {
                first = entry;
            }
            last = entry;
            if request_id == 0 {
                request_id = focused_request_id(&entry, &capsule.replay);
            }
            if completion_id == 0 {
                completion_id = focused_completion_id(&entry, &capsule.replay);
            }
            if irq_id == 0 {
                irq_id = focused_irq_id(&entry, &capsule.replay);
            }
            let path = dominant_path_for_trace(&entry, capsule);
            if dominant_path == DiagnosticsPath::None {
                dominant_path = path;
            }
            last_path = path;
            count = count.saturating_add(1);
        }
        if count != 0 {
            out[cpu_slot] = SmpTimelineCpuSummary {
                valid: true,
                cpu_slot: cpu_slot as u16,
                apic_id: last.apic_id,
                first_sequence: first.sequence,
                last_sequence: last.sequence,
                event_count: count,
                first_stage: first.stage,
                last_stage: last.stage,
                request_id,
                completion_id,
                irq_id,
                dominant_path,
                divergence_suspected: dominant_path != last_path,
            };
        }
        cpu_slot += 1;
    }
    out
}

pub(super) fn build_smp_divergence_summary(
    smp: &[SmpTimelineCpuSummary; MAX_TRACE_CPUS],
) -> SmpDivergenceSummary {
    let mut best = SmpDivergenceSummary::EMPTY;
    let mut cpu_a = 0usize;
    while cpu_a < MAX_TRACE_CPUS {
        let left = smp[cpu_a];
        if !left.valid {
            cpu_a += 1;
            continue;
        }
        let mut cpu_b = cpu_a + 1;
        while cpu_b < MAX_TRACE_CPUS {
            let right = smp[cpu_b];
            if !right.valid {
                cpu_b += 1;
                continue;
            }
            let shared_req = left.request_id != 0 && left.request_id == right.request_id;
            let shared_cmp = left.completion_id != 0 && left.completion_id == right.completion_id;
            let shared_irq = left.irq_id != 0 && left.irq_id == right.irq_id;
            if (shared_req || shared_cmp || shared_irq)
                && (left.dominant_path != right.dominant_path
                    || left.last_stage != right.last_stage)
            {
                let gap = left.last_sequence.abs_diff(right.last_sequence);
                if !best.valid || gap > best.sequence_gap {
                    best = SmpDivergenceSummary {
                        valid: true,
                        cpu_a: left.cpu_slot,
                        cpu_b: right.cpu_slot,
                        sequence_gap: gap,
                        stage_a: left.last_stage,
                        stage_b: right.last_stage,
                        path_a: left.dominant_path,
                        path_b: right.dominant_path,
                        request_id: if shared_req { left.request_id } else { 0 },
                        completion_id: if shared_cmp { left.completion_id } else { 0 },
                        irq_id: if shared_irq { left.irq_id } else { 0 },
                    };
                }
            }
            cpu_b += 1;
        }
        cpu_a += 1;
    }
    best
}

pub(super) fn emit_export_bundle() {
    serial::write_bytes(b"== export-bundle ==\n");
    let bundle = build_export_bundle(&crash_capsule());
    if !bundle.valid {
        serial::write_bytes(b"ngos/x86_64: export-bundle none reason=no-crash-capsule\n");
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: export meta generation={} signature={} stable_prefix={} unstable_suffix={} divergence_seq={} dominant_overlap={} memory_total={}\n",
        bundle.generation,
        bundle.failure_signature_id,
        bundle.stable_prefix_length,
        bundle.unstable_suffix_length,
        bundle.first_divergence_sequence,
        memory_overlap_name(bundle.memory.dominant_overlap),
        bundle.memory.total
    ));
    for entry in bundle.focused.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: export focused seq={} cpu={} stage={} stage_name={} event={} path={} req={} cmp={} irq={} result={} reason={}\n",
            entry.sequence,
            entry.cpu_slot,
            entry.stage,
            stage_name(entry.stage),
            trace_kind_name(entry.kind),
            diagnostics_path_name(entry.path),
            entry.request_id,
            entry.completion_id,
            entry.irq_id,
            entry.result,
            entry.reason
        ));
    }
    for entry in bundle.suspects.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: export suspect rank={} cpu={} stage={} stage_name={} req={} cmp={} irq={} score={} confidence={}% reason={} violation={}\n",
            entry.rank,
            entry.cpu_slot,
            entry.stage,
            stage_name(entry.stage),
            entry.request_id,
            entry.completion_id,
            entry.irq_id,
            entry.score,
            entry.confidence,
            entry.reason,
            entry.local_violation
        ));
    }
    for edge in bundle.causal.iter().filter(|edge| edge.valid) {
        serial::print(format_args!(
            "ngos/x86_64: export causal seq={} cpu={} stage={} stage_name={} kind={} req={} cmp={} irq={} reason={}\n",
            edge.sequence,
            edge.cpu_slot,
            edge.stage,
            stage_name(edge.stage),
            causal_edge_name(edge.kind),
            edge.request_id,
            edge.completion_id,
            edge.irq_id,
            edge.reason
        ));
    }
    for entry in bundle.failure_history.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: export history generation={} signature={} req={} cmp={} irq={} stage={} path={} stable_prefix={} unstable_suffix={} first_bad={} divergence={}\n",
            entry.generation,
            entry.signature_id,
            entry.request_id,
            entry.completion_id,
            entry.irq_id,
            stage_name(entry.stage),
            diagnostics_path_name(entry.path),
            entry.stable_prefix_length,
            entry.unstable_suffix_length,
            entry.first_bad_sequence,
            entry.first_divergence_sequence
        ));
    }
    for entry in bundle.patterns.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: export pattern rank={} signature={} frequency={} last_generation={} last_seen={} path={} stage={} first_bad={}\n",
            entry.rank,
            entry.signature_id,
            entry.frequency,
            entry.last_generation,
            entry.last_seen_sequence,
            diagnostics_path_name(entry.path),
            stage_name(entry.stage),
            trace_kind_name(entry.first_bad_kind)
        ));
    }
    serial::print(format_args!(
        "ngos/x86_64: export reprobe mode={:?} path={} stage={} stage_name={} checkpoint={:#x} escalation={} crashes={}\n",
        bundle.reprobe.mode,
        diagnostics_path_name(bundle.reprobe.target_path),
        bundle.reprobe.target_stage,
        stage_name(bundle.reprobe.target_stage),
        bundle.reprobe.target_checkpoint,
        bundle.reprobe.escalation,
        bundle.reprobe.crash_count
    ));
}
