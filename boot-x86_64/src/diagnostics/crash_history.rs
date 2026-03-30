use super::*;

pub(super) fn emit_failure_history() {
    let patterns = unsafe { *TRACE_STORAGE.pattern_history.get() };
    let history = unsafe { *TRACE_STORAGE.crash_history.get() };
    let mut top = [FailurePatternSummary::EMPTY; 3];
    for entry in patterns.iter().copied().filter(|entry| entry.valid) {
        insert_top_pattern(&mut top, entry);
    }
    for (index, entry) in top.iter().enumerate().filter(|(_, entry)| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: recurring-failure rank={} pattern={} frequency={} last_generation={} stage={} first_bad={} path={}\n",
            index + 1,
            entry.signature_id,
            entry.frequency,
            entry.last_generation,
            stage_name(entry.most_common_stage),
            trace_kind_name(entry.most_common_first_bad_kind),
            diagnostics_path_name(entry.path)
        ));
    }
    for entry in history.iter().filter(|entry| entry.valid) {
        let capsule = crash_capsule_from_history(*entry, &crash_capsule());
        serial::print(format_args!(
            "ngos/x86_64: failure-history generation={} pattern={} path={} req={} cmp={} irq={} stage={} stable_prefix={} unstable_suffix={} first_bad_seq={} divergence_seq={}\n",
            entry.generation,
            entry.signature_id,
            diagnostics_path_name(dominant_failure_path(&capsule).0),
            entry.replay.request_id,
            entry.replay.completion_id,
            entry.replay.irq_id,
            stage_name(entry.fault.stage),
            entry.stable_prefix_length,
            entry.unstable_suffix_length,
            entry.first_bad_sequence,
            entry.first_divergence_sequence
        ));
    }
}

pub fn crash_capsule_capture() {
    let fault = last_fault();
    let mut tail = [[TraceRecord::EMPTY; CRASH_TRACE_TAIL]; MAX_TRACE_CPUS];
    let trace = snapshot_trace();
    for cpu in 0..MAX_TRACE_CPUS {
        let mut kept = [TraceRecord::EMPTY; CRASH_TRACE_TAIL];
        let mut kept_count = 0usize;
        for entry in trace[cpu].iter().filter(|entry| entry.sequence != 0) {
            if kept_count < CRASH_TRACE_TAIL {
                kept[kept_count] = *entry;
                kept_count += 1;
            } else {
                let mut shift = 1usize;
                while shift < CRASH_TRACE_TAIL {
                    kept[shift - 1] = kept[shift];
                    shift += 1;
                }
                kept[CRASH_TRACE_TAIL - 1] = *entry;
            }
        }
        let mut index = 0usize;
        while index < kept_count {
            tail[cpu][index] = kept[index];
            index += 1;
        }
    }
    let violations = unsafe { *TRACE_STORAGE.violations.get() };
    let replay = replay_ids();
    let reprobe = unsafe { *TRACE_STORAGE.reprobe_policy.get() };
    let window = unsafe { *TRACE_STORAGE.current_window.get() };
    let previous = previous_crash_capsule();
    let signature =
        build_failure_signature_from_parts(&fault, &window, &replay, &violations, &tail, &previous);
    let semantic_reasons = summarize_semantic_reasons(&fault, &window, &violations, reprobe);
    let suspects =
        rank_suspects_from_parts(&fault, &window, &replay, &violations, &tail, &previous);
    let (
        stable_prefix_length,
        unstable_suffix_length,
        first_divergence_sequence,
        closest_prior_pattern_id,
    ) = compare_failure_patterns_from_parts(&window, &replay, signature.id, &tail, &previous);
    unsafe {
        let capsule = &mut *TRACE_STORAGE.crash_capsule.get();
        if capsule.valid {
            *TRACE_STORAGE.previous_crash_capsule.get() = *capsule;
        }
        *capsule = CrashCapsule {
            valid: true,
            generation: capsule.generation.saturating_add(1),
            mode: mode(),
            failure_signature: signature,
            failure_signature_id: signature.id,
            closest_prior_pattern_id,
            stable_prefix_length,
            unstable_suffix_length,
            first_divergence_sequence,
            semantic_reasons,
            suspects,
            replay,
            fault,
            reprobe,
            window,
            watch_tail: violations,
            trace_tail: tail,
        };
        record_failure_history(*capsule);
    }
}

pub fn crash_capsule() -> CrashCapsule {
    unsafe { *TRACE_STORAGE.crash_capsule.get() }
}

pub fn previous_crash_capsule() -> CrashCapsule {
    unsafe { *TRACE_STORAGE.previous_crash_capsule.get() }
}

pub fn dump_failure_history() {
    serial::write_bytes(b"== failure-history ==\n");
    emit_failure_history();
}

pub fn emit_boot_failure_history_summary() {
    let patterns = unsafe { *TRACE_STORAGE.pattern_history.get() };
    let history = unsafe { *TRACE_STORAGE.crash_history.get() };
    let mut top = [FailurePatternSummary::EMPTY; 2];
    for entry in patterns.iter().copied().filter(|entry| entry.valid) {
        insert_top_pattern(&mut top, entry);
    }
    for (index, entry) in top.iter().enumerate().filter(|(_, entry)| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: boot-history rank={} pattern={} frequency={} stage={} first_bad={} path={:?}\n",
            index + 1,
            entry.signature_id,
            entry.frequency,
            stage_name(entry.most_common_stage),
            trace_kind_name(entry.most_common_first_bad_kind),
            entry.path
        ));
    }
    let mut suspect_stage = [0u16; 2];
    let mut suspect_count = [0u16; 2];
    for crash in history.iter().copied().filter(|entry| entry.valid) {
        let stage = crash.fault.stage;
        if suspect_count[0] == 0 || suspect_stage[0] == stage {
            suspect_stage[0] = stage;
            suspect_count[0] = suspect_count[0].saturating_add(1);
        } else if suspect_count[1] == 0 || suspect_stage[1] == stage {
            suspect_stage[1] = stage;
            suspect_count[1] = suspect_count[1].saturating_add(1);
        }
    }
    for index in 0..2 {
        if suspect_count[index] != 0 {
            serial::print(format_args!(
                "ngos/x86_64: boot-top-stage rank={} stage={} count={}\n",
                index + 1,
                stage_name(suspect_stage[index]),
                suspect_count[index]
            ));
        }
    }
}

pub(super) fn record_failure_history(capsule: CrashCapsule) {
    unsafe {
        let history = &mut *TRACE_STORAGE.crash_history.get();
        let slot = (capsule.generation as usize - 1) % FAILURE_HISTORY_CAPACITY;
        history[slot] = CrashHistoryEntry {
            valid: true,
            generation: capsule.generation,
            signature_id: capsule.failure_signature_id,
            replay: capsule.replay,
            window: capsule.window,
            fault: capsule.fault,
            stable_prefix_length: capsule.stable_prefix_length,
            unstable_suffix_length: capsule.unstable_suffix_length,
            first_divergence_sequence: capsule.first_divergence_sequence,
            first_bad_sequence: first_bad_event(&capsule)
                .map(|entry| entry.sequence)
                .unwrap_or(0),
            last_good_sequence: last_confirmed_good(&capsule)
                .map(|entry| entry.sequence)
                .unwrap_or(0),
            focused_trace: compact_focused_trace(&capsule),
        };
        let patterns = &mut *TRACE_STORAGE.pattern_history.get();
        for entry in patterns.iter_mut() {
            if entry.valid && entry.signature_id == capsule.failure_signature_id {
                entry.frequency = entry.frequency.saturating_add(1);
                entry.last_generation = capsule.generation;
                entry.last_seen_sequence = capsule.replay.sequence;
                entry.path = capsule.window.path;
                entry.most_common_stage = capsule.failure_signature.stage;
                entry.most_common_first_bad_kind = capsule.failure_signature.first_bad_kind;
                return;
            }
        }
        let slot = (capsule.generation as usize - 1) % FAILURE_PATTERN_CAPACITY;
        patterns[slot] = FailurePatternSummary {
            valid: true,
            signature_id: capsule.failure_signature_id,
            frequency: 1,
            last_generation: capsule.generation,
            last_seen_sequence: capsule.replay.sequence,
            path: capsule.window.path,
            most_common_stage: capsule.failure_signature.stage,
            most_common_first_bad_kind: capsule.failure_signature.first_bad_kind,
        };
    }
}

fn compact_focused_trace(capsule: &CrashCapsule) -> [TraceRecord; FOCUSED_TRACE_HISTORY] {
    let mut out = [TraceRecord::EMPTY; FOCUSED_TRACE_HISTORY];
    let events = focused_events(capsule);
    let mut index = 0usize;
    for entry in events.iter().flatten() {
        if index >= FOCUSED_TRACE_HISTORY {
            break;
        }
        out[index] = **entry;
        index += 1;
    }
    out
}

pub(super) fn closest_prior_pattern(signature_id: u64) -> FailurePatternSummary {
    let patterns = unsafe { *TRACE_STORAGE.pattern_history.get() };
    let mut exact = FailurePatternSummary::EMPTY;
    let mut closest = FailurePatternSummary::EMPTY;
    for entry in patterns.iter().copied().filter(|entry| entry.valid) {
        if entry.signature_id == signature_id {
            exact = entry;
            break;
        }
        if !closest.valid || entry.frequency > closest.frequency {
            closest = entry;
        }
    }
    if exact.valid { exact } else { closest }
}

pub(super) fn insert_top_pattern(
    top: &mut [FailurePatternSummary],
    candidate: FailurePatternSummary,
) {
    let mut index = 0usize;
    while index < top.len() {
        if !top[index].valid || candidate.frequency > top[index].frequency {
            let mut shift = top.len() - 1;
            while shift > index {
                top[shift] = top[shift - 1];
                shift -= 1;
            }
            top[index] = candidate;
            return;
        }
        index += 1;
    }
}
