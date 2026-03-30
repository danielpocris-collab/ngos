use super::*;

pub(super) struct NextMindAgentState<'a> {
    pub(super) last_snapshot: &'a mut Option<ngos_user_abi::NativeSystemSnapshotRecord>,
    pub(super) adaptive_state: &'a mut AdaptiveState,
    pub(super) context: &'a mut SemanticContext,
    pub(super) entity_epochs: &'a mut Vec<SemanticEntityEpoch>,
    pub(super) auto_state: &'a mut NextMindAutoState,
    pub(super) last_report: &'a mut Option<NextMindDecisionReport>,
    pub(super) last_status: &'a mut i32,
}

pub(super) fn try_handle_nextmind_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    state: &mut NextMindAgentState<'_>,
) -> Option<Result<(), ExitCode>> {
    if line == "nextmind.observe" {
        let controller = SystemController::new(runtime);
        let semantic_state = match controller
            .observe_semantic_state(state.last_snapshot.as_ref(), state.adaptive_state)
        {
            Ok(semantic_state) => semantic_state,
            Err(_) => return Some(Err(266)),
        };
        *state.last_snapshot = Some(semantic_state.metrics.snapshot);
        if nextmind_render_metrics(
            runtime,
            "current",
            &semantic_state.metrics,
            semantic_state.pressure,
        )
        .is_err()
        {
            return Some(Err(266));
        }
        if write_line(
            runtime,
            &format!(
                "nextmind.semantic channel={} class={} caps={} tier={:?} mode={:?} budget={} stress={} focus={} obs={}/{}/{}/{}",
                semantic_state.channel,
                semantic_class_name(semantic_state.semantic.class),
                semantic_capabilities_csv(&semantic_state.semantic),
                semantic_state.adaptive.tier,
                semantic_state.adaptive.compute_mode,
                semantic_state.adaptive.budget_points,
                semantic_state.adaptive.stress,
                semantic_state.adaptive.focus,
                semantic_state.observation.cpu_load,
                semantic_state.observation.mem_pressure,
                semantic_state.observation.anomaly_score,
                semantic_state.observation.thermal_c,
            ),
        )
        .is_err()
        {
            return Some(Err(266));
        }
        state.context.push(
            &semantic_state.channel,
            &semantic_state.semantic,
            &format!(
                "pressure={} runq={} cpu={} socket={} event={}",
                nextmind_pressure_state_label(semantic_state.pressure),
                semantic_state.metrics.run_queue_total,
                semantic_state.metrics.cpu_utilization_pct,
                semantic_state.metrics.socket_pressure_pct,
                semantic_state.metrics.event_queue_pressure_pct
            ),
            &[],
        );
        let entities = match controller.collect_semantic_entities() {
            Ok(entities) => entities,
            Err(_) => return Some(Err(266)),
        };
        for (entity, epoch) in nextmind_update_entity_epochs(state.entity_epochs, &entities) {
            if write_line(
                runtime,
                &format!(
                    "nextmind.entity kind={} subject={} class={} caps={} cpu-mask=0x{:x} policy-epoch={}",
                    semantic_entity_kind_name(entity.kind),
                    entity.subject,
                    semantic_class_name(entity.semantic.class),
                    semantic_capabilities_csv(&entity.semantic),
                    entity.policy.cpu_mask,
                    epoch,
                ),
            )
            .is_err()
            {
                return Some(Err(266));
            }
        }
        let topology = match controller.observe_topology(state.last_snapshot.as_ref()) {
            Ok(topology) => topology,
            Err(_) => return Some(Err(266)),
        };
        let loads = topology
            .entries
            .iter()
            .map(|entry| entry.load)
            .collect::<Vec<_>>();
        let selected_cpu = select_cpu(&loads, topology.online_cpus, &[]).unwrap_or(0);
        for entry in &topology.entries {
            if write_line(
                runtime,
                &format!(
                    "nextmind.cpu cpu={} apic={} online={} launched={} load={} mask=0x{:x} selected={}",
                    entry.cpu_index,
                    entry.apic_id,
                    entry.online,
                    entry.launched,
                    load_percent(&entry.load),
                    cpu_mask_for(entry.cpu_index),
                    entry.cpu_index == selected_cpu,
                ),
            )
            .is_err()
            {
                return Some(Err(266));
            }
        }
        return Some(Ok(()));
    }
    if line == "nextmind.optimize" {
        match nextmind_optimize_system(runtime, state.last_snapshot, state.adaptive_state) {
            Ok(report) => {
                if nextmind_render_metrics(runtime, "before", &report.before, report.trigger)
                    .is_err()
                    || nextmind_render_metrics(
                        runtime,
                        "after",
                        &report.after,
                        SystemController::new(runtime).classify_pressure(&report.after),
                    )
                    .is_err()
                {
                    return Some(Err(266));
                }
                if write_line(
                    runtime,
                    &format!(
                        "nextmind.semantic channel={} class={} caps={} tier={:?} mode={:?} budget={} stress={} focus={} obs={}/{}/{}/{}",
                        pressure_channel_name(report.trigger),
                        semantic_class_name(report.semantic.class),
                        semantic_capabilities_csv(&report.semantic),
                        report.adaptive.tier,
                        report.adaptive.compute_mode,
                        report.adaptive.budget_points,
                        report.adaptive.stress,
                        report.adaptive.focus,
                        report.observation.cpu_load,
                        report.observation.mem_pressure,
                        report.observation.anomaly_score,
                        report.observation.thermal_c,
                    ),
                )
                .is_err()
                {
                    return Some(Err(266));
                }
                if report.actions.is_empty() {
                    if write_line(
                        runtime,
                        "nextmind.action reason=none detail=no-direct-adjustment-required",
                    )
                    .is_err()
                    {
                        return Some(Err(266));
                    }
                } else {
                    for action in &report.actions {
                        if write_line(
                            runtime,
                            &format!(
                                "nextmind.action reason={} detail={}",
                                action.reason, action.detail
                            ),
                        )
                        .is_err()
                        {
                            return Some(Err(266));
                        }
                    }
                }
                if write_line(
                    runtime,
                    &format!("nextmind.verdict={}", semantic_verdict_name(report.verdict)),
                )
                .is_err()
                {
                    return Some(Err(266));
                }
                state.context.push(
                    pressure_channel_name(report.trigger),
                    &report.semantic,
                    &format!(
                        "verdict={} before-runq={} after-runq={}",
                        semantic_verdict_name(report.verdict),
                        report.before.run_queue_total,
                        report.after.run_queue_total
                    ),
                    &report.actions,
                );
                *state.last_report = Some(report);
                *state.last_status = 0;
            }
            Err(code) => {
                *state.last_status = code;
            }
        }
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("nextmind.auto ") {
        match rest.trim() {
            "on" => match nextmind_subscribe_auto_streams(runtime) {
                Ok(streams) => {
                    state.auto_state.enabled = true;
                    state.auto_state.streams = streams;
                    if write_line(
                        runtime,
                        &format!(
                            "nextmind.auto=on streams={}",
                            state.auto_state.streams.len()
                        ),
                    )
                    .is_err()
                    {
                        return Some(Err(195));
                    }
                    *state.last_status = 0;
                }
                Err(code) => *state.last_status = code,
            },
            "off" => {
                state.auto_state.enabled = false;
                state.auto_state.streams.clear();
                if write_line(runtime, "nextmind.auto=off").is_err() {
                    return Some(Err(195));
                }
                *state.last_status = 0;
            }
            _ => {
                *state.last_status = 2;
                if write_line(runtime, "usage: nextmind.auto <on|off>").is_err() {
                    return Some(Err(199));
                }
            }
        }
        return Some(Ok(()));
    }
    if line == "nextmind.explain last" {
        *state.last_status = match nextmind_explain_last(
            runtime,
            state.adaptive_state,
            state.context,
            state.last_report,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    None
}
