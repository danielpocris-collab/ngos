use super::*;

pub(super) fn emit_chronoscope_report() {
    serial::write_bytes(b"== chronoscope ==\n");
    let snapshot = chronoscope_snapshot();
    if !snapshot.valid {
        serial::write_bytes(b"ngos/x86_64: chronoscope none reason=no-crash-capsule\n");
        return;
    }
    let validation = validate_bundle_invariants(&snapshot);
    if !validation.valid {
        emit_chronoscope_panic_fallback(&snapshot, ChronoscopePanicMode::PanicCaptureDegraded);
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: chronoscope schema={} generation={} signature={} root_stage={} root_path={} top_confidence={}% nodes={} edges={} checkpoints={} lineage={} runtime_events={} partial_runtime={} anomalies={} escalations={} windows={}\n",
        CHRONOSCOPE_SCHEMA_VERSION,
        snapshot.generation,
        snapshot.failure_signature_id,
        stage_name(snapshot.root_boundary.stage),
        diagnostics_path_name(snapshot.root_boundary.path),
        snapshot.top_suspect_confidence,
        snapshot.nodes.iter().filter(|node| node.valid).count(),
        snapshot.edges.iter().filter(|edge| edge.valid).count(),
        snapshot
            .checkpoints
            .iter()
            .filter(|checkpoint| checkpoint.valid)
            .count(),
        snapshot
            .lineage
            .iter()
            .filter(|record| record.valid)
            .count(),
        snapshot.runtime_events.total_events,
        snapshot.runtime_events.partial as u16,
        snapshot
            .anomalies
            .iter()
            .filter(|entry| entry.valid)
            .count(),
        snapshot
            .escalations
            .iter()
            .filter(|entry| entry.valid)
            .count(),
        snapshot
            .capture_windows
            .iter()
            .filter(|entry| entry.valid)
            .count()
    ));
    let explain = snapshot.build_explain_plan();
    serial::print(format_args!(
        "ngos/x86_64: chronoscope explain primary={} fault={} boundary={} confidence={}%\n",
        explain.primary_cause,
        explain.fault_node,
        explain.earliest_preventable_boundary,
        (explain.confidence * 100.0) as u16
    ));
    serial::print(format_args!(
        "ngos/x86_64: chronoscope temporal state_before_fault={} rewind={} divergence={} last_mutation={} temporal_confidence={}\n",
        snapshot.temporal_explain.state_before_fault.0,
        snapshot.temporal_explain.rewind_candidate.0,
        snapshot.temporal_explain.divergence_origin.0,
        snapshot.temporal_explain.last_mutation,
        snapshot.temporal_explain.temporal_confidence
    ));
    serial::print(format_args!(
        "ngos/x86_64: responsibility primary={} last_writer={} cap_origin={} responsibility_confidence={}\n",
        snapshot.primary_responsible_node(),
        snapshot.temporal_explain.last_writer,
        snapshot
            .capability_origin(snapshot.temporal_explain.capability_chain[0])
            .0,
        snapshot.temporal_explain.responsibility_confidence
    ));
    serial::print(format_args!(
        "ngos/x86_64: replay steps={} start_checkpoint={} divergence={} replay_confidence={}\n",
        snapshot.temporal_explain.replay_steps_count,
        snapshot.temporal_explain.replay_summary.start_checkpoint.0,
        snapshot.temporal_explain.first_divergence_point.0,
        snapshot.temporal_explain.replay_confidence
    ));
    serial::print(format_args!(
        "ngos/x86_64: adaptive state={} anomaly={} escalation={} candidate={} downgrade_ready={} anomaly_confidence={}\n",
        snapshot.temporal_explain.adaptive_state as u16,
        snapshot.temporal_explain.dominant_anomaly.0,
        snapshot.temporal_explain.escalation_id.0,
        snapshot.temporal_explain.candidate_node,
        snapshot.temporal_explain.downgrade_ready as u16,
        snapshot.temporal_explain.anomaly_confidence
    ));
    serial::print(format_args!(
        "ngos/x86_64: trust completeness={} reason={} replay_partial={} explain_degraded={} responsibility_partial={} capture_level={} perf_events={} perf_queries={} perf_replays={} perf_diffs={}\n",
        snapshot.trust.completeness.complete as u16,
        snapshot.trust.completeness.primary_reason as u16,
        snapshot.trust.replay_partial as u16,
        snapshot.trust.explain_degraded as u16,
        snapshot.trust.responsibility_partial as u16,
        snapshot.trust.capture_level as u16,
        snapshot.perf.events_emitted,
        snapshot.perf.query_executions,
        snapshot.perf.replay_executions,
        snapshot.perf.diff_executions
    ));
    for node in snapshot.nodes.iter().filter(|node| node.valid) {
        serial::print(format_args!(
            "ngos/x86_64: chronoscope node={} stable={:#x} kind={} seq={} cpu={} stage={} path={} req={} cmp={} irq={} score={} confidence={} severity={} distance={} evidence={}\n",
            node.node_id,
            node.stable_id,
            chronoscope_node_kind_name(node.kind),
            node.event_sequence,
            node.cpu_slot,
            stage_name(node.stage),
            diagnostics_path_name(node.path),
            node.request_id,
            node.completion_id,
            node.irq_id,
            node.score,
            (node.confidence * 100.0) as u16,
            node.severity,
            node.causal_distance_to_fault,
            node.evidence_count
        ));
    }
    for edge in snapshot.edges.iter().filter(|edge| edge.valid) {
        serial::print(format_args!(
            "ngos/x86_64: chronoscope edge={} src={} dst={} weight={}\n",
            chronoscope_edge_kind_name(edge.kind),
            edge.src_node_id,
            edge.dst_node_id,
            edge.weight
        ));
    }
    for checkpoint in snapshot.checkpoints.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: chronoscope checkpoint={} id={} stable={:#x} depth={} pred={} conf={} req={} cmp={} irq={}\n",
            chronoscope_checkpoint_kind_name(checkpoint.kind),
            checkpoint.checkpoint_id.0,
            checkpoint.stable_id,
            checkpoint.causal_depth,
            checkpoint.predecessor.0,
            checkpoint.confidence_permille,
            checkpoint.correlation.request_id,
            checkpoint.correlation.completion_id,
            checkpoint.correlation.irq_id
        ));
    }
    for anomaly in snapshot.anomalies.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: anomaly id={} kind={} severity={} confidence={} count={} core_mask={} escalation={}\n",
            anomaly.anomaly_id.0,
            chronoscope_anomaly_kind_name(anomaly.kind),
            anomaly.severity,
            anomaly.confidence_permille,
            anomaly.occurrence_count,
            anomaly.related_core_mask,
            anomaly.escalation_id.0
        ));
    }
    for escalation in snapshot.escalations.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: escalation id={} level={} reason={} target_mask={} budget={} bookmark={}\n",
            escalation.escalation_id.0,
            chronoscope_escalation_level_name(escalation.level),
            escalation.reason as u16,
            escalation.target_core_mask,
            escalation.event_budget,
            escalation.replay_bookmark.0
        ));
    }
    for window in snapshot.capture_windows.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: capture_window kind={} escalation={} start={} end={} observed={} partial={}\n",
            chronoscope_capture_window_kind_name(window.kind),
            window.escalation_id.0,
            window.start_event.0,
            window.end_event.0,
            window.observed_events,
            window.partial_history as u16
        ));
    }
    let mut lineage_printed = 0usize;
    for record in snapshot.lineage.iter().filter(|entry| entry.valid) {
        if lineage_printed >= CHRONOSCOPE_LINEAGE_SUMMARY_LIMIT {
            break;
        }
        serial::print(format_args!(
            "ngos/x86_64: chronoscope lineage={} id={} key={} prior_cp={} transition={} result_cp={} result_node={} conf={}\n",
            chronoscope_query::chronoscope_lineage_domain_name(record.domain),
            record.lineage_id.0,
            record.key,
            record.prior_checkpoint.0,
            record.transition_node,
            record.result_checkpoint.0,
            record.result_node,
            record.confidence_permille
        ));
        lineage_printed += 1;
    }
    let mut resp_index = 0usize;
    while resp_index < snapshot.temporal_explain.responsibility_ranking.len() {
        let entry = snapshot.temporal_explain.responsibility_ranking[resp_index];
        if entry.valid {
            serial::print(format_args!(
                "ngos/x86_64: responsibility rank={} node={} score={}\n",
                resp_index + 1,
                entry.node_id,
                entry.score
            ));
        }
        resp_index += 1;
    }
    let mut core = 0usize;
    while core < snapshot.runtime_events.per_core.len() {
        let summary = snapshot.runtime_events.per_core[core];
        if summary.valid {
            serial::print(format_args!(
                "ngos/x86_64: chronoscope runtime core={} buffer={} available={} overwritten={} oldest={} newest={} partial={}\n",
                summary.core_id,
                summary.buffer_id.0,
                summary.available_events,
                summary.overwritten_events,
                summary.oldest_local_sequence,
                summary.newest_local_sequence,
                summary.partial as u16
            ));
        }
        core += 1;
    }
    let mut event_index = 0usize;
    while event_index < snapshot.runtime_events.total_events as usize {
        let event = snapshot.runtime_events.events[event_index];
        if event.valid {
            serial::print(format_args!(
                "ngos/x86_64: chronoscope runtime event={} id={} cpu={} stage={} req={} cmp={} irq={} object={} parent={} flags={}\n",
                chronoscope_runtime_kind_name(event.kind),
                event.event_id.0,
                event.core_id,
                stage_name(event.stage),
                event.correlation.request_id,
                event.correlation.completion_id,
                event.correlation.irq_id,
                event.object_key,
                event.causal_parent.0,
                event.flags
            ));
        }
        event_index += 1;
    }
}

pub(super) fn emit_chronoscope_panic_fallback(
    snapshot: &ChronoscopeBundle,
    mode: ChronoscopePanicMode,
) {
    serial::print(format_args!(
        "ngos/x86_64: chronoscope panic mode={} schema={} signature={} completeness={} reason={} fault={} candidate={}\n",
        mode as u16,
        CHRONOSCOPE_SCHEMA_VERSION,
        snapshot.failure_signature_id,
        snapshot.integrity.complete as u16,
        snapshot.integrity.primary_reason as u16,
        snapshot.primary_fault_node(),
        snapshot.dominant_candidate().unwrap_or(0)
    ));
}
