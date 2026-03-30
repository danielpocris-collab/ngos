#![cfg_attr(not(test), allow(dead_code))]

use super::*;

pub(super) fn chronoscope_lineage_domain_name(domain: ChronoscopeLineageDomain) -> &'static str {
    match domain {
        ChronoscopeLineageDomain::ContractState => "contract",
        ChronoscopeLineageDomain::ResourceState => "resource",
        ChronoscopeLineageDomain::RequestPath => "request-path",
        ChronoscopeLineageDomain::ViolationState => "violation",
        ChronoscopeLineageDomain::SuspectState => "suspect",
        ChronoscopeLineageDomain::CoreDivergenceState => "core-divergence",
    }
}

fn parse_u64_token(token: &str) -> Option<u64> {
    if let Some(hex) = token.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        token.parse::<u64>().ok()
    }
}

fn parse_query_domain(token: &str) -> Option<ChronoscopeLineageDomain> {
    match token {
        "contract" | "contractstate" => Some(ChronoscopeLineageDomain::ContractState),
        "resource" | "resourcestate" => Some(ChronoscopeLineageDomain::ResourceState),
        "request" | "requestpath" => Some(ChronoscopeLineageDomain::RequestPath),
        "violation" | "violationstate" => Some(ChronoscopeLineageDomain::ViolationState),
        "suspect" | "suspectstate" => Some(ChronoscopeLineageDomain::SuspectState),
        "divergence" | "coredivergence" => Some(ChronoscopeLineageDomain::CoreDivergenceState),
        _ => None,
    }
}

fn parse_query_limit(tokens: &[&str]) -> Result<(usize, u16), ChronoscopeQueryParseError> {
    if tokens.len() >= 2 && tokens[tokens.len() - 2] == "limit" {
        let value =
            parse_u64_token(tokens[tokens.len() - 1]).ok_or(ChronoscopeQueryParseError {
                token_index: (tokens.len() - 1) as u8,
                reason: "invalid-limit",
            })?;
        Ok((tokens.len() - 2, value.min(u16::MAX as u64) as u16))
    } else {
        Ok((tokens.len(), 0))
    }
}

pub(crate) fn parse_chronoscope_query(
    input: &str,
) -> Result<ChronoscopeQuery, ChronoscopeQueryParseError> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(ChronoscopeQueryParseError {
            token_index: 0,
            reason: "empty-query",
        });
    }
    let (base_len, limit) = parse_query_limit(&tokens)?;
    let tokens = &tokens[..base_len];
    let mut query = ChronoscopeQuery::EMPTY;
    query.limit = limit;
    match tokens {
        ["path", "to", "fault"] => query.kind = ChronoscopeQueryKind::PathToFault,
        ["path", "from", "node", node, "to", "fault"] => {
            query.kind = ChronoscopeQueryKind::PathFromNodeToFault;
            query.node_id = parse_u64_token(node).ok_or(ChronoscopeQueryParseError {
                token_index: 3,
                reason: "invalid-node-id",
            })?;
        }
        ["explain", "fault"] => query.kind = ChronoscopeQueryKind::ExplainFault,
        ["explain", "node", node] => {
            query.kind = ChronoscopeQueryKind::ExplainNode;
            query.node_id = parse_u64_token(node).ok_or(ChronoscopeQueryParseError {
                token_index: 2,
                reason: "invalid-node-id",
            })?;
        }
        ["last-writer", domain, key] => {
            query.kind = ChronoscopeQueryKind::LastWriter;
            query.domain = parse_query_domain(domain);
            query.key = parse_u64_token(key).ok_or(ChronoscopeQueryParseError {
                token_index: 2,
                reason: "invalid-key",
            })?;
        }
        ["writer-chain", domain, key] => {
            query.kind = ChronoscopeQueryKind::WriterChain;
            query.domain = parse_query_domain(domain);
            query.key = parse_u64_token(key).ok_or(ChronoscopeQueryParseError {
                token_index: 2,
                reason: "invalid-key",
            })?;
        }
        ["lineage", "node", node] => {
            query.kind = ChronoscopeQueryKind::LineageNode;
            query.node_id = parse_u64_token(node).ok_or(ChronoscopeQueryParseError {
                token_index: 2,
                reason: "invalid-node-id",
            })?;
        }
        ["lineage", "domain", domain, key] => {
            query.kind = ChronoscopeQueryKind::LineageDomain;
            query.domain = parse_query_domain(domain);
            query.key = parse_u64_token(key).ok_or(ChronoscopeQueryParseError {
                token_index: 3,
                reason: "invalid-key",
            })?;
        }
        ["cap-origin", cap] => {
            query.kind = ChronoscopeQueryKind::CapabilityOrigin;
            query.capability_id =
                CapabilityId(parse_u64_token(cap).ok_or(ChronoscopeQueryParseError {
                    token_index: 1,
                    reason: "invalid-capability-id",
                })?);
        }
        ["cap-chain", cap] => {
            query.kind = ChronoscopeQueryKind::CapabilityChain;
            query.capability_id =
                CapabilityId(parse_u64_token(cap).ok_or(ChronoscopeQueryParseError {
                    token_index: 1,
                    reason: "invalid-capability-id",
                })?);
        }
        ["cap-usage", cap] => {
            query.kind = ChronoscopeQueryKind::CapabilityUsage;
            query.capability_id =
                CapabilityId(parse_u64_token(cap).ok_or(ChronoscopeQueryParseError {
                    token_index: 1,
                    reason: "invalid-capability-id",
                })?);
        }
        ["checkpoint", "before", "fault"] => {
            query.kind = ChronoscopeQueryKind::CheckpointBeforeFault
        }
        ["rewind-candidate"] => query.kind = ChronoscopeQueryKind::RewindCandidate,
        ["replay", "from", checkpoint, "to", "fault"] => {
            query.kind = ChronoscopeQueryKind::ReplayFromCheckpointToFault;
            query.checkpoint_id = ChronoscopeCheckpointId(parse_u64_token(checkpoint).ok_or(
                ChronoscopeQueryParseError {
                    token_index: 2,
                    reason: "invalid-checkpoint-id",
                },
            )? as u16);
        }
        ["replay", "until", "violation"] => query.kind = ChronoscopeQueryKind::ReplayUntilViolation,
        ["divergence-origin"] => query.kind = ChronoscopeQueryKind::DivergenceOrigin,
        ["diff", "first-divergence"] => query.kind = ChronoscopeQueryKind::DiffFirstDivergence,
        ["diff", "responsibility"] => query.kind = ChronoscopeQueryKind::DiffResponsibility,
        ["responsibility", "top"] => query.kind = ChronoscopeQueryKind::ResponsibilityTop,
        ["responsibility", "node", node] => {
            query.kind = ChronoscopeQueryKind::ResponsibilityNode;
            query.node_id = parse_u64_token(node).ok_or(ChronoscopeQueryParseError {
                token_index: 2,
                reason: "invalid-node-id",
            })?;
        }
        ["anomalies", "top"] => query.kind = ChronoscopeQueryKind::AnomaliesTop,
        ["anomaly", id] => {
            query.kind = ChronoscopeQueryKind::AnomalyById;
            query.anomaly_id =
                ChronoscopeAnomalyId(parse_u64_token(id).ok_or(ChronoscopeQueryParseError {
                    token_index: 1,
                    reason: "invalid-anomaly-id",
                })? as u16);
        }
        ["adaptive-state"] => query.kind = ChronoscopeQueryKind::AdaptiveState,
        ["capture-window", "active"] => query.kind = ChronoscopeQueryKind::CaptureWindowActive,
        ["capture-window", "recent"] => query.kind = ChronoscopeQueryKind::CaptureWindowRecent,
        ["escalation", "history"] => query.kind = ChronoscopeQueryKind::EscalationHistory,
        ["candidate", "top"] => query.kind = ChronoscopeQueryKind::CandidateTop,
        ["candidate", "for-anomaly", id] => {
            query.kind = ChronoscopeQueryKind::CandidateForAnomaly;
            query.anomaly_id =
                ChronoscopeAnomalyId(parse_u64_token(id).ok_or(ChronoscopeQueryParseError {
                    token_index: 2,
                    reason: "invalid-anomaly-id",
                })? as u16);
        }
        ["propagation", "path", "to", "fault", "from", "node", node] => {
            query.kind = ChronoscopeQueryKind::PropagationPathToFault;
            query.node_id = parse_u64_token(node).ok_or(ChronoscopeQueryParseError {
                token_index: 6,
                reason: "invalid-node-id",
            })?;
        }
        _ => {
            return Err(ChronoscopeQueryParseError {
                token_index: 0,
                reason: "unknown-query",
            });
        }
    }
    Ok(query)
}

fn apply_query_limit<T>(items: &mut Vec<T>, limit: u16) {
    if limit != 0 && items.len() > limit as usize {
        items.truncate(limit as usize);
    }
}

fn explain_for_node(
    bundle: &ChronoscopeBundle,
    node_id: ChronoscopeNodeId,
) -> ChronoscopeQueryExplain {
    let mut plan = bundle.temporal_explain.clone();
    if node_id != 0 {
        plan.primary_cause = node_id;
    }
    ChronoscopeQueryExplain { plan }
}

pub(super) fn execute_query_with_baseline(
    bundle: &ChronoscopeBundle,
    baseline: Option<&ChronoscopeBundle>,
    query: &ChronoscopeQuery,
) -> ChronoscopeQueryResult {
    PERF_QUERY_EXECUTIONS.fetch_add(1, Ordering::Relaxed);
    let mut result = ChronoscopeQueryResult::empty();
    match query.kind {
        ChronoscopeQueryKind::PathToFault => {
            result.kind = ChronoscopeQueryResultKind::NodeList;
            result.nodes = bundle.dominant_propagation_path();
            if result.nodes.is_empty() {
                result.nodes = bundle
                    .strongest_chain
                    .iter()
                    .copied()
                    .filter(|node| *node != 0)
                    .collect();
            }
            apply_query_limit(&mut result.nodes, query.limit);
            result.path = Some(ChronoscopeQueryPath {
                nodes: result.nodes.clone(),
            });
        }
        ChronoscopeQueryKind::PathFromNodeToFault
        | ChronoscopeQueryKind::PropagationPathToFault => {
            result.kind = ChronoscopeQueryResultKind::NodeList;
            result.nodes = bundle.propagation_chain_to_fault(query.node_id);
            apply_query_limit(&mut result.nodes, query.limit);
            result.path = Some(ChronoscopeQueryPath {
                nodes: result.nodes.clone(),
            });
        }
        ChronoscopeQueryKind::ExplainFault => {
            result.kind = ChronoscopeQueryResultKind::ExplainSummary;
            result.explain = Some(ChronoscopeQueryExplain {
                plan: bundle.temporal_explain.clone(),
            });
        }
        ChronoscopeQueryKind::ExplainNode => {
            result.kind = ChronoscopeQueryResultKind::ExplainSummary;
            result.explain = Some(explain_for_node(bundle, query.node_id));
        }
        ChronoscopeQueryKind::LastWriter => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            let writer = bundle
                .last_writer_of(query.domain.unwrap(), query.key)
                .unwrap_or(0);
            result.rows.push(ChronoscopeQueryRow {
                key: "last_writer",
                value: writer,
            });
        }
        ChronoscopeQueryKind::WriterChain => {
            result.kind = ChronoscopeQueryResultKind::NodeList;
            result.nodes = bundle.writer_chain_to_fault(query.domain.unwrap(), query.key);
            apply_query_limit(&mut result.nodes, query.limit);
            result.path = Some(ChronoscopeQueryPath {
                nodes: result.nodes.clone(),
            });
        }
        ChronoscopeQueryKind::LineageNode => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            for id in bundle.lineage_for_node(query.node_id) {
                result.rows.push(ChronoscopeQueryRow {
                    key: "lineage_id",
                    value: id.0 as u64,
                });
            }
            apply_query_limit(&mut result.rows, query.limit);
        }
        ChronoscopeQueryKind::LineageDomain => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            let writer = bundle
                .last_writer_of(query.domain.unwrap(), query.key)
                .unwrap_or(0);
            for id in bundle.lineage_for_node(writer) {
                result.rows.push(ChronoscopeQueryRow {
                    key: "lineage_id",
                    value: id.0 as u64,
                });
            }
            apply_query_limit(&mut result.rows, query.limit);
        }
        ChronoscopeQueryKind::CapabilityOrigin => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            result.rows.push(ChronoscopeQueryRow {
                key: "cap_origin",
                value: bundle.capability_origin(query.capability_id).0,
            });
        }
        ChronoscopeQueryKind::CapabilityChain => {
            result.kind = ChronoscopeQueryResultKind::CapabilityList;
            result.capabilities = bundle.capability_chain(query.capability_id);
            apply_query_limit(&mut result.capabilities, query.limit);
        }
        ChronoscopeQueryKind::CapabilityUsage => {
            result.kind = ChronoscopeQueryResultKind::NodeList;
            result.nodes = bundle.capability_usage(query.capability_id);
            apply_query_limit(&mut result.nodes, query.limit);
        }
        ChronoscopeQueryKind::CheckpointBeforeFault => {
            result.kind = ChronoscopeQueryResultKind::CheckpointList;
            if let Some(checkpoint) = bundle.state_before_fault() {
                result.checkpoints.push(checkpoint);
            }
        }
        ChronoscopeQueryKind::RewindCandidate => {
            result.kind = ChronoscopeQueryResultKind::CheckpointList;
            if let Some(checkpoint) = bundle.rewind_candidate() {
                result.checkpoints.push(checkpoint);
            }
        }
        ChronoscopeQueryKind::ReplayFromCheckpointToFault => {
            let trace = bundle.replay_from_checkpoint(query.checkpoint_id);
            result.kind = ChronoscopeQueryResultKind::ReplaySummary;
            result.replay = Some(ChronoscopeQueryReplaySummary {
                steps: trace.total_steps,
                checkpoint: query.checkpoint_id,
                last_event: if trace.total_steps != 0 {
                    trace.events[trace.total_steps as usize - 1].event_id
                } else {
                    ChronoscopeEventId::NONE
                },
                partial: trace.partial,
            });
        }
        ChronoscopeQueryKind::ReplayUntilViolation => {
            let replay = bundle.replay_until_violation();
            result.kind = ChronoscopeQueryResultKind::ReplaySummary;
            result.replay = Some(ChronoscopeQueryReplaySummary {
                steps: replay.steps,
                checkpoint: bundle
                    .rewind_candidate()
                    .unwrap_or(ChronoscopeCheckpointId::NONE),
                last_event: replay.last_event.event_id,
                partial: bundle.runtime_events.partial,
            });
        }
        ChronoscopeQueryKind::DivergenceOrigin => {
            result.kind = ChronoscopeQueryResultKind::CheckpointList;
            if let Some(checkpoint) = bundle.divergence_origin() {
                result.checkpoints.push(checkpoint);
            }
        }
        ChronoscopeQueryKind::DiffFirstDivergence | ChronoscopeQueryKind::DiffResponsibility => {
            let baseline = match baseline {
                Some(baseline) => baseline,
                None => return ChronoscopeQueryResult::parse_error(0, "missing-baseline"),
            };
            result.kind = ChronoscopeQueryResultKind::DiffSummary;
            result.diff = Some(bundle.diff_against(baseline));
        }
        ChronoscopeQueryKind::ResponsibilityTop => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            let mut ranking = bundle.responsibility_ranking();
            if query.limit != 0 && ranking.len() > query.limit as usize {
                ranking.truncate(query.limit as usize);
            }
            for (node, score) in ranking {
                result.rows.push(ChronoscopeQueryRow {
                    key: "node",
                    value: node,
                });
                result.rows.push(ChronoscopeQueryRow {
                    key: "score",
                    value: score as u64,
                });
            }
        }
        ChronoscopeQueryKind::ResponsibilityNode => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            let score = bundle
                .responsibility_ranking()
                .into_iter()
                .find(|entry| entry.0 == query.node_id)
                .map(|entry| entry.1 as u64)
                .unwrap_or(0);
            result.rows.push(ChronoscopeQueryRow {
                key: "responsibility_score",
                value: score,
            });
        }
        ChronoscopeQueryKind::AnomaliesTop => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            for anomaly in bundle.anomalies.iter().copied() {
                if anomaly.valid {
                    result.rows.push(ChronoscopeQueryRow {
                        key: chronoscope_anomaly_kind_name(anomaly.kind),
                        value: anomaly.anomaly_id.0 as u64,
                    });
                }
            }
            apply_query_limit(&mut result.rows, query.limit);
        }
        ChronoscopeQueryKind::AnomalyById => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            for anomaly in bundle.anomalies.iter().copied() {
                if anomaly.valid && anomaly.anomaly_id == query.anomaly_id {
                    result.rows.push(ChronoscopeQueryRow {
                        key: "severity",
                        value: anomaly.severity as u64,
                    });
                    result.rows.push(ChronoscopeQueryRow {
                        key: "count",
                        value: anomaly.occurrence_count as u64,
                    });
                    result.rows.push(ChronoscopeQueryRow {
                        key: "confidence",
                        value: anomaly.confidence_permille as u64,
                    });
                    break;
                }
            }
        }
        ChronoscopeQueryKind::AdaptiveState => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            result.rows.push(ChronoscopeQueryRow {
                key: "adaptive_state",
                value: bundle.adaptive_state as u64,
            });
        }
        ChronoscopeQueryKind::CaptureWindowActive | ChronoscopeQueryKind::CaptureWindowRecent => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            for window in bundle.capture_windows.iter().copied() {
                if window.valid {
                    result.rows.push(ChronoscopeQueryRow {
                        key: "capture_window",
                        value: window.escalation_id.0 as u64,
                    });
                }
            }
            apply_query_limit(&mut result.rows, query.limit);
        }
        ChronoscopeQueryKind::EscalationHistory => {
            result.kind = ChronoscopeQueryResultKind::Scalar;
            for escalation in bundle.escalations.iter().copied() {
                if escalation.valid {
                    result.rows.push(ChronoscopeQueryRow {
                        key: chronoscope_escalation_level_name(escalation.level),
                        value: escalation.escalation_id.0 as u64,
                    });
                }
            }
            apply_query_limit(&mut result.rows, query.limit);
        }
        ChronoscopeQueryKind::CandidateTop => {
            result.kind = ChronoscopeQueryResultKind::NodeList;
            for candidate in bundle.candidates.candidates.iter().copied() {
                if candidate.valid {
                    result.nodes.push(candidate.node_id);
                }
            }
            apply_query_limit(&mut result.nodes, query.limit);
            result.path = Some(ChronoscopeQueryPath {
                nodes: result.nodes.clone(),
            });
        }
        ChronoscopeQueryKind::CandidateForAnomaly => {
            result.kind = ChronoscopeQueryResultKind::NodeList;
            let candidates = bundle.anomaly_candidates(query.anomaly_id);
            for candidate in candidates.candidates.iter().copied() {
                if candidate.valid {
                    result.nodes.push(candidate.node_id);
                }
            }
            apply_query_limit(&mut result.nodes, query.limit);
            result.path = Some(ChronoscopeQueryPath {
                nodes: result.nodes.clone(),
            });
        }
    }
    result
}

pub(super) fn execute_query(
    bundle: &ChronoscopeBundle,
    query: &ChronoscopeQuery,
) -> ChronoscopeQueryResult {
    execute_query_with_baseline(bundle, None, query)
}

pub(super) fn run_query(bundle: &ChronoscopeBundle, input: &str) -> ChronoscopeQueryResult {
    match parse_chronoscope_query(input) {
        Ok(query) => execute_query(bundle, &query),
        Err(error) => ChronoscopeQueryResult::parse_error(error.token_index, error.reason),
    }
}

#[cfg_attr(test, allow(dead_code))]
pub(super) fn run_query_with_baseline(
    bundle: &ChronoscopeBundle,
    baseline: &ChronoscopeBundle,
    input: &str,
) -> ChronoscopeQueryResult {
    match parse_chronoscope_query(input) {
        Ok(query) => execute_query_with_baseline(bundle, Some(baseline), &query),
        Err(error) => ChronoscopeQueryResult::parse_error(error.token_index, error.reason),
    }
}

pub(super) fn emit_query_result(
    result: &ChronoscopeQueryResult,
    out: &mut impl Write,
) -> fmt::Result {
    match result.kind {
        ChronoscopeQueryResultKind::Empty => out.write_str("empty\n"),
        ChronoscopeQueryResultKind::Error => {
            let error = result.error.unwrap();
            write!(
                out,
                "error token={} reason={}\n",
                error.token_index, error.reason
            )
        }
        ChronoscopeQueryResultKind::Scalar => {
            let mut rows = result.rows.clone();
            rows.sort_by(|left, right| {
                left.key
                    .cmp(right.key)
                    .then_with(|| left.value.cmp(&right.value))
            });
            let mut emitted = 0usize;
            for row in &rows {
                if emitted >= ChronoscopeConfig::DEFAULT.max_query_rows as usize {
                    out.write_str("omitted=1\n")?;
                    break;
                }
                writeln!(out, "{}={}", row.key, row.value)?;
                emitted += 1;
            }
            Ok(())
        }
        ChronoscopeQueryResultKind::NodeList => {
            for node in &result.nodes {
                writeln!(out, "node={}", node)?;
            }
            Ok(())
        }
        ChronoscopeQueryResultKind::CapabilityList => {
            for cap in &result.capabilities {
                writeln!(out, "cap={}", cap.0)?;
            }
            Ok(())
        }
        ChronoscopeQueryResultKind::CheckpointList => {
            for checkpoint in &result.checkpoints {
                writeln!(out, "checkpoint={}", checkpoint.0)?;
            }
            Ok(())
        }
        ChronoscopeQueryResultKind::ReplaySummary => {
            if let Some(replay) = result.replay {
                writeln!(
                    out,
                    "replay steps={} checkpoint={} last_event={} partial={}",
                    replay.steps, replay.checkpoint.0, replay.last_event.0, replay.partial as u16
                )
            } else {
                out.write_str("replay none\n")
            }
        }
        ChronoscopeQueryResultKind::DiffSummary => {
            if let Some(diff) = &result.diff {
                writeln!(
                    out,
                    "diff first_temporal_divergence={:#x} changed_last_writer={} changed_responsibility={}",
                    diff.first_temporal_divergence,
                    diff.changed_last_writer as u16,
                    diff.changed_responsibility_ranking as u16
                )
            } else {
                out.write_str("diff none\n")
            }
        }
        ChronoscopeQueryResultKind::ExplainSummary => {
            if let Some(explain) = &result.explain {
                writeln!(
                    out,
                    "explain primary={} fault={} last_writer={} replay_steps={}",
                    explain.plan.primary_cause,
                    explain.plan.fault_node,
                    explain.plan.last_writer,
                    explain.plan.replay_steps_count
                )
            } else {
                out.write_str("explain none\n")
            }
        }
    }
}

const _: fn() = || {
    let _ = chronoscope_lineage_domain_name(ChronoscopeLineageDomain::RequestPath);
};
