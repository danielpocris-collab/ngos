use super::*;

pub(super) fn emit_causal_chain() {
    serial::write_bytes(b"== causal-chain ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: causal-chain none reason=no-crash-capsule\n");
        return;
    }
    emit_root_cause_summary(&capsule);
    emit_symptom_vs_root_summary(&capsule);
    emit_propagation_tracking(&capsule);
    emit_propagation_evidence(&capsule);
    emit_failure_story(&capsule);
    emit_causal_action_summary(&capsule);
}

pub(super) fn emit_root_cause_summary(capsule: &CrashCapsule) {
    let top = capsule
        .suspects
        .iter()
        .copied()
        .find(|suspect| suspect.valid);
    let confidence = top
        .map(|suspect| root_cause_confidence(suspect, capsule))
        .unwrap_or(consistency_score(capsule) / 2);
    serial::print(format_args!(
        "ngos/x86_64: root-cause candidate={} symptom={} confidence={} reason={}\n",
        probable_root_cause(capsule),
        probable_symptom(capsule),
        confidence,
        root_cause_reason(capsule)
    ));
}

pub(super) fn emit_symptom_vs_root_summary(capsule: &CrashCapsule) {
    serial::print(format_args!(
        "ngos/x86_64: symptom-vs-root root={} symptom={} class={} consistency={} band={}\n",
        probable_root_cause(capsule),
        probable_symptom(capsule),
        classification_name(classify_failure(capsule)),
        consistency_score(capsule),
        consistency_band_name(consistency_score(capsule))
    ));
}

pub(super) fn emit_propagation_tracking(capsule: &CrashCapsule) {
    let last_good = focused_last_good(&focused_events(capsule));
    let first_bad = focused_first_bad(&focused_events(capsule));
    serial::print(format_args!(
        "ngos/x86_64: propagation last_valid={} first_invalid={} boundary={} req={} cmp={} irq={}\n",
        last_good
            .map(|entry| stage_name(entry.stage))
            .unwrap_or("none"),
        first_bad
            .map(|entry| stage_name(entry.stage))
            .unwrap_or(stage_name(capsule.failure_signature.first_bad_stage)),
        probable_bad_boundary(capsule),
        capsule.replay.request_id,
        capsule.replay.completion_id,
        capsule.replay.irq_id
    ));
    serial::print(format_args!(
        "ngos/x86_64: propagation path={} stable_prefix={} unstable_suffix={} divergence={}\n",
        dominant_path_name(dominant_failure_path(capsule).0),
        capsule.stable_prefix_length,
        capsule.unstable_suffix_length,
        trace_kind_name(capsule.failure_signature.divergence_kind)
    ));
}

pub(super) fn emit_propagation_evidence(capsule: &CrashCapsule) {
    let events = focused_events(capsule);
    let last_good = focused_last_good(&events);
    let first_bad = focused_first_bad(&events);
    let previous = previous_crash_capsule();
    let divergence = first_divergence_point_from_events(&events, &focused_events(&previous));
    serial::print(format_args!(
        "ngos/x86_64: propagation-evidence last_good={} first_bad={} divergence={} top_violation={} semantic_reason={}\n",
        render_trace_evidence(last_good),
        render_trace_evidence(first_bad),
        render_divergence_evidence(divergence),
        local_violation_for_top_suspect(capsule),
        local_semantic_reason_for_top_suspect(capsule)
    ));
}

pub(super) fn emit_failure_story(capsule: &CrashCapsule) {
    serial::print(format_args!(
        "ngos/x86_64: failure-story request_state={} propagation={} failure_point={} inspect_first={}\n",
        failure_story_request_state(capsule),
        probable_propagation(capsule),
        probable_bad_boundary(capsule),
        first_concrete_inspection_step(capsule)
    ));
}

pub(super) fn emit_causal_action_summary(capsule: &CrashCapsule) {
    let target = top_patch_targets(capsule)[0];
    let class = likely_patch_bug_class(capsule);
    let confidence = top_root_cause_confidence(capsule);
    serial::print(format_args!(
        "ngos/x86_64: causal-action bug_class={} target={} zone={} confidence={} inspect={} avoid={}\n",
        patch_bug_class_name(class),
        target.file_area,
        target.function_zone,
        confidence,
        first_concrete_inspection_step(capsule),
        do_not_touch_first_list(capsule)[0].0
    ));
    serial::print(format_args!(
        "ngos/x86_64: causal-action why={} patch_shape={} history={}\n",
        root_cause_reason(capsule),
        patch_shape_suggestion(capsule).0,
        bug_class_history_relation(capsule, class)
    ));
}

#[allow(dead_code)]
pub(super) fn emit_failure_story_compact() {
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: compact story none\n");
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: compact story root={} propagation={} boundary={}\n",
        probable_root_cause(&capsule),
        probable_propagation(&capsule),
        probable_bad_boundary(&capsule)
    ));
}

pub(super) fn emit_patch_suggestions() {
    serial::write_bytes(b"== patch-suggestions ==\n");
    emit_likely_bug_class();
    emit_patch_targets();
    emit_missing_invariant_hints();
    emit_check_first_summary();
    emit_do_not_touch_first_summary();
    emit_patch_shape_summary();
    emit_developer_action_summary();
}

pub(super) fn emit_likely_bug_class() {
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: bug-class none confidence=0 reason=no-crash-capsule\n");
        return;
    }
    let class = likely_patch_bug_class(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: bug-class class={} confidence={} history={} reason={}\n",
        patch_bug_class_name(class),
        bug_class_confidence(&capsule, class),
        bug_class_history_relation(&capsule, class),
        bug_class_reason(&capsule, class)
    ));
}

pub(super) fn emit_patch_targets() {
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(
            b"ngos/x86_64: patch-target none confidence=0 reason=no-crash-capsule\n",
        );
        return;
    }
    let targets = top_patch_targets(&capsule);
    for target in targets.iter() {
        serial::print(format_args!(
            "ngos/x86_64: patch-target rank={} area={} stage={} zone={} confidence={} reason={}\n",
            target.rank,
            target.file_area,
            stage_name(target.stage),
            target.function_zone,
            target.confidence,
            target.reason
        ));
        serial::print(format_args!(
            "ngos/x86_64: patch-evidence rank={} because={} semantic_reason={} violation={} causal_step={} stability={}\n",
            target.rank,
            patch_target_because(&capsule, *target),
            dominant_patch_semantic_reason(&capsule, *target),
            dominant_patch_violation(&capsule, *target),
            causal_step_for_target(&capsule, *target),
            patch_target_stability(&capsule, *target)
        ));
    }
}

pub(super) fn emit_missing_invariant_hints() {
    serial::write_bytes(b"== missing-invariant-hints ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: invariant none reason=no-crash-capsule\n");
        return;
    }
    for hint in missing_invariant_hints(&capsule).iter() {
        serial::print(format_args!(
            "ngos/x86_64: invariant missing={} enforce_at={} carry_and_check={}\n",
            hint.0, hint.1, hint.2
        ));
    }
}

pub(super) fn emit_check_first_summary() {
    serial::write_bytes(b"== check-first ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: check-first none reason=no-crash-capsule\n");
        return;
    }
    for (index, item) in check_first_list(&capsule).iter().enumerate() {
        serial::print(format_args!(
            "ngos/x86_64: check-first rank={} zone={} condition={} reason={}\n",
            index + 1,
            item.0,
            item.1,
            item.2
        ));
    }
}

pub(super) fn emit_do_not_touch_first_summary() {
    serial::write_bytes(b"== do-not-touch-first ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: do-not-touch none reason=no-crash-capsule\n");
        return;
    }
    for item in do_not_touch_first_list(&capsule).iter() {
        serial::print(format_args!(
            "ngos/x86_64: do-not-touch area={} reason={}\n",
            item.0, item.1
        ));
    }
}

pub(super) fn emit_patch_shape_summary() {
    serial::write_bytes(b"== patch-shape ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(
            b"ngos/x86_64: patch-shape likely=none confidence=0 reason=no-crash-capsule\n",
        );
        return;
    }
    let shape = patch_shape_suggestion(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: patch-shape likely={} confidence={} reason={}\n",
        shape.0, shape.1, shape.2
    ));
}

pub(super) fn emit_developer_action_summary() {
    serial::write_bytes(b"== developer-action-summary ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: developer-action none\n");
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: developer-action root_cause={} propagation={} bad_boundary={} inspect_first={}\n",
        probable_root_cause(&capsule),
        probable_propagation(&capsule),
        probable_bad_boundary(&capsule),
        first_concrete_inspection_step(&capsule)
    ));
}

pub(super) fn emit_same_pattern_stability_report() {
    serial::write_bytes(b"== same-pattern-stability ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(
            b"ngos/x86_64: pattern-stability none result=ok reason=no-crash-capsule\n",
        );
        return;
    }
    let history = unsafe { *TRACE_STORAGE.crash_history.get() };
    let mut matches = [CrashHistoryEntry::EMPTY; FAILURE_HISTORY_CAPACITY];
    let mut count = 0usize;
    for entry in history
        .iter()
        .copied()
        .filter(|entry| entry.valid && entry.signature_id == capsule.failure_signature_id)
    {
        if count < matches.len() {
            matches[count] = entry;
            count += 1;
        }
    }
    if count < 2 {
        serial::write_bytes(
            b"ngos/x86_64: pattern-stability count=1 result=ok reason=no-prior-same-pattern\n",
        );
        return;
    }
    let mut stable_min = u16::MAX;
    let mut stable_max = 0u16;
    let mut varying_stage = false;
    let mut varying_ids = false;
    let mut nondeterminism = 0u16;
    let baseline = matches[0];
    let mut index = 0usize;
    while index < count {
        let entry = matches[index];
        stable_min = stable_min.min(entry.stable_prefix_length);
        stable_max = stable_max.max(entry.stable_prefix_length);
        if entry.fault.stage != baseline.fault.stage {
            varying_stage = true;
        }
        if entry.replay.request_id != baseline.replay.request_id
            || entry.replay.completion_id != baseline.replay.completion_id
            || entry.replay.irq_id != baseline.replay.irq_id
        {
            varying_ids = true;
        }
        if entry.window.path == DiagnosticsPath::Completion
            || entry.replay.irq_id != baseline.replay.irq_id
            || entry.fault.cpu_slot != baseline.fault.cpu_slot
        {
            nondeterminism = nondeterminism.saturating_add(1);
        }
        index += 1;
    }
    serial::print(format_args!(
        "ngos/x86_64: pattern-stability pattern={} runs={} stable_prefix_min={} stable_prefix_max={} varying_stage={} varying_ids={} nondeterminism_score={} stable_until_stage={} result={}\n",
        capsule.failure_signature_id,
        count,
        stable_min,
        stable_max,
        varying_stage as u8,
        varying_ids as u8,
        nondeterminism,
        stage_name(capsule.failure_signature.last_good_stage),
        if nondeterminism > 1 {
            "timing-sensitive"
        } else {
            "mostly-stable"
        }
    ));
}
