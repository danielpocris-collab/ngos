use super::*;

pub(crate) enum ShellSemanticDispatchOutcome {
    Continue,
    Unhandled,
}

pub(crate) struct ShellSemanticDispatchState<'a> {
    pub(crate) current_cwd: &'a mut String,
    pub(crate) jobs: &'a mut Vec<ShellJob>,
    pub(crate) aliases: &'a mut Vec<ShellAlias>,
    pub(crate) variables: &'a mut Vec<ShellVariable>,
    pub(crate) history: &'a [String],
    pub(crate) pending_lines: &'a mut Vec<String>,
    pub(crate) line_index: usize,
    pub(crate) shell_mode: &'a mut ShellMode,
    pub(crate) semantic_learning: &'a mut SemanticFeedbackStore,
    pub(crate) nextmind_entity_epochs: &'a mut Vec<SemanticEntityEpoch>,
    pub(crate) nextmind_auto_state: &'a mut NextMindAutoState,
    pub(crate) nextmind_last_report: &'a mut Option<NextMindDecisionReport>,
    pub(crate) nextmind_last_snapshot: &'a mut Option<ngos_user_abi::NativeSystemSnapshotRecord>,
    pub(crate) nextmind_adaptive_state: &'a mut AdaptiveState,
    pub(crate) nextmind_context: &'a mut SemanticContext,
    pub(crate) repair_ai_state: &'a mut ngos_shell_repair::RepairAiState,
    pub(crate) game_sessions: &'a mut Vec<GameCompatSession>,
    pub(crate) compat_abi: &'a mut ngos_shell_compat_abi::CompatAbiShellState,
    pub(crate) last_spawned_pid: &'a mut Option<u64>,
    pub(crate) last_status: &'a mut i32,
    pub(crate) previous_status: i32,
}

pub(crate) fn try_dispatch_shell_semantic_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    line: &str,
    state: ShellSemanticDispatchState<'_>,
) -> Result<ShellSemanticDispatchOutcome, ExitCode> {
    if let Some(result) = try_handle_game_agent_command(
        runtime,
        state.current_cwd,
        line,
        state.game_sessions,
        state.jobs,
        state.last_spawned_pid,
        state.last_status,
    ) {
        return map_semantic_dispatch_result(result, false);
    }
    if let Some(result) = ngos_shell_compat_abi::try_handle_compat_abi_command(
        runtime,
        line,
        state.compat_abi,
        state.last_status,
    ) {
        return map_semantic_dispatch_result(result, false);
    }
    if let Some(result) =
        ngos_shell_code::try_handle_code_agent_command(runtime, state.current_cwd, line)
    {
        return map_semantic_dispatch_result(result, true);
    }
    if let Some(result) = try_handle_workflow_agent_command(runtime, state.current_cwd, line) {
        return map_semantic_dispatch_result(result, true);
    }
    if let Some(result) =
        ngos_shell_project::try_handle_project_agent_command(runtime, state.current_cwd, line)
    {
        return map_semantic_dispatch_result(result, true);
    }
    if let Some(result) =
        ngos_shell_rust::try_handle_rust_agent_command(runtime, state.current_cwd, line)
    {
        return map_semantic_dispatch_result(result, true);
    }
    if let Some(result) = try_handle_analysis_agent_command(runtime, state.current_cwd, line) {
        return map_semantic_dispatch_result(result, true);
    }
    if let Some(result) = nextmind_agents::try_handle_nextmind_agent_command(
        runtime,
        line,
        &mut nextmind_agents::NextMindAgentState {
            last_snapshot: state.nextmind_last_snapshot,
            adaptive_state: state.nextmind_adaptive_state,
            context: state.nextmind_context,
            entity_epochs: state.nextmind_entity_epochs,
            auto_state: state.nextmind_auto_state,
            last_report: state.nextmind_last_report,
            last_status: state.last_status,
        },
    ) {
        return match result {
            Ok(()) => Ok(ShellSemanticDispatchOutcome::Continue),
            Err(code) if code == 2 => Err(199),
            Err(code) => Err(code),
        };
    }
    if let Some(result) = ngos_shell_repair::try_handle_repair_agent_command(
        runtime,
        line,
        state.repair_ai_state,
        state.nextmind_adaptive_state,
        state.last_status,
    ) {
        return match result {
            Ok(()) => Ok(ShellSemanticDispatchOutcome::Continue),
            Err(code) if code == 2 => Err(199),
            Err(code) => Err(code),
        };
    }
    if let Some(result) = shell_state_agents::try_handle_shell_state_agent_command(
        runtime,
        state.current_cwd,
        line,
        state.shell_mode,
        state.semantic_learning,
        state.previous_status,
        state.pending_lines,
        state.line_index,
        state.last_status,
    ) {
        return match result {
            Ok(()) => Ok(ShellSemanticDispatchOutcome::Continue),
            Err(code) if code == 2 => Err(199),
            Err(code) => Err(code),
        };
    }
    if let Some(result) = session_agents::try_handle_session_agent_command(
        runtime,
        context,
        state.current_cwd,
        state.aliases,
        state.variables,
        state.jobs,
        state.history,
        state.pending_lines,
        state.line_index,
        line,
    ) {
        return match result {
            Ok(()) => Ok(ShellSemanticDispatchOutcome::Continue),
            Err(code) if code == 2 => Err(199),
            Err(code) => Err(code),
        };
    }
    if let Some(result) = semantic_agents::try_handle_semantic_agent_command(
        runtime,
        state.current_cwd,
        line,
        state.last_status,
        state.semantic_learning,
        state.nextmind_entity_epochs,
    ) {
        return match result {
            Ok(()) => Ok(ShellSemanticDispatchOutcome::Continue),
            Err(code) if code == 2 => Err(199),
            Err(code) => Err(code),
        };
    }
    if let Some(result) = intent_agents::try_handle_intent_agent_command(
        runtime,
        state.current_cwd,
        *state.shell_mode,
        line,
        state.last_status,
        state.semantic_learning,
        state.nextmind_entity_epochs,
    ) {
        return match result {
            Ok(()) => Ok(ShellSemanticDispatchOutcome::Continue),
            Err(code) if code == 2 => Err(199),
            Err(code) => Err(code),
        };
    }

    Ok(ShellSemanticDispatchOutcome::Unhandled)
}

fn map_semantic_dispatch_result(
    result: Result<(), ExitCode>,
    collapse_unknown_to_205: bool,
) -> Result<ShellSemanticDispatchOutcome, ExitCode> {
    match result {
        Ok(()) => Ok(ShellSemanticDispatchOutcome::Continue),
        Err(code) if code == 2 => Err(199),
        Err(_code) if collapse_unknown_to_205 => Err(205),
        Err(code) => Err(code),
    }
}
