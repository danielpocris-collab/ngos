use super::*;

pub(crate) fn run_session_shell_script<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    text: &str,
) -> ExitCode {
    if write_line(runtime, "ngos shell").is_err() {
        return 192;
    }
    let line_count = text.lines().count();
    let mut last_status = 0;
    let mut current_cwd = context.cwd.clone();
    let mut jobs = Vec::<ShellJob>::with_capacity(8);
    let mut aliases = Vec::<ShellAlias>::with_capacity(16);
    let mut variables = Vec::<ShellVariable>::with_capacity(line_count.saturating_add(64));
    let mut history = Vec::<String>::with_capacity(line_count.saturating_add(32));
    let mut shell_mode = ShellMode::Direct;
    let mut semantic_learning = SemanticFeedbackStore::new();
    let mut nextmind_auto_state = NextMindAutoState {
        enabled: false,
        streams: Vec::new(),
    };
    let mut nextmind_last_report = None::<NextMindDecisionReport>;
    let mut nextmind_last_snapshot = None::<ngos_user_abi::NativeSystemSnapshotRecord>;
    let mut nextmind_adaptive_state = AdaptiveState::new();
    let mut repair_ai_state = ngos_shell_repair::RepairAiState::new();
    let mut nextmind_context = SemanticContext::new();
    let mut nextmind_entity_epochs = Vec::<SemanticEntityEpoch>::new();
    let mut game_sessions = Vec::<GameCompatSession>::new();
    let mut compat_abi = ngos_shell_compat_abi::CompatAbiShellState::new();
    let mut edit_session = ngos_shell_edit::EditSessionState::new();
    let mut shell_functions = Vec::<ShellFunction>::new();
    let mut shell_call_stack = Vec::<ShellCallFrame>::new();
    let mut last_spawned_pid = None::<u64>;
    shell_sync_runtime_variables(&mut variables, last_status, &current_cwd, last_spawned_pid);
    let mut pending_lines = Vec::<String>::with_capacity(line_count.saturating_add(32));
    pending_lines.extend(text.lines().map(|line| line.to_string()));
    let mut line_index = 0usize;
    while line_index < pending_lines.len() {
        merge_multiline_lang_block(&mut pending_lines, line_index);
        let raw_line = pending_lines[line_index].clone();
        line_index += 1;
        for (guard, command) in shell_parse_guarded_commands(&raw_line) {
            match guard {
                ShellCommandGuard::Always => {}
                ShellCommandGuard::OnSuccess if last_status != 0 => continue,
                ShellCommandGuard::OnFailure if last_status == 0 => continue,
                ShellCommandGuard::OnSuccess | ShellCommandGuard::OnFailure => {}
            }
            if command.is_empty() || command.starts_with('#') {
                continue;
            }
            history.push(command.to_string());
            if nextmind_drain_auto_events(
                runtime,
                &nextmind_auto_state,
                &mut nextmind_last_snapshot,
                &mut nextmind_adaptive_state,
                &mut nextmind_last_report,
            )
            .is_err()
            {
                return 267;
            }
            shell_sync_runtime_variables(
                &mut variables,
                last_status,
                &current_cwd,
                last_spawned_pid,
            );
            let expanded_command = shell_expand_aliases(&command, &aliases);
            let lang_candidate = expanded_command.trim();
            let line = shell_expand_variables(&expanded_command, &variables);
            if line.is_empty() {
                continue;
            }
            let previous_status = last_status;
            last_status = 0;
            match try_dispatch_shell_front_command(
                runtime,
                lang_candidate,
                &line,
                ShellFrontDispatchState {
                    variables: &mut variables,
                    shell_functions: &mut shell_functions,
                    shell_call_stack: &mut shell_call_stack,
                    pending_lines: &mut pending_lines,
                    line_index,
                    last_status: &mut last_status,
                },
            ) {
                Ok(ShellFrontDispatchOutcome::Continue) => continue,
                Ok(ShellFrontDispatchOutcome::Unhandled) => {}
                Err(code) => return code,
            }
            match try_dispatch_shell_semantic_command(
                runtime,
                context,
                &line,
                ShellSemanticDispatchState {
                    current_cwd: &mut current_cwd,
                    jobs: &mut jobs,
                    aliases: &mut aliases,
                    variables: &mut variables,
                    history: &history,
                    pending_lines: &mut pending_lines,
                    line_index,
                    shell_mode: &mut shell_mode,
                    semantic_learning: &mut semantic_learning,
                    nextmind_entity_epochs: &mut nextmind_entity_epochs,
                    nextmind_auto_state: &mut nextmind_auto_state,
                    nextmind_last_report: &mut nextmind_last_report,
                    nextmind_last_snapshot: &mut nextmind_last_snapshot,
                    nextmind_adaptive_state: &mut nextmind_adaptive_state,
                    nextmind_context: &mut nextmind_context,
                    repair_ai_state: &mut repair_ai_state,
                    game_sessions: &mut game_sessions,
                    compat_abi: &mut compat_abi,
                    last_spawned_pid: &mut last_spawned_pid,
                    last_status: &mut last_status,
                    previous_status,
                },
            ) {
                Ok(ShellSemanticDispatchOutcome::Continue) => continue,
                Ok(ShellSemanticDispatchOutcome::Unhandled) => {}
                Err(code) => return code,
            }
            match try_dispatch_shell_command(
                runtime,
                context,
                &aliases,
                &history,
                &line,
                ShellCommandDispatchState {
                    current_cwd: &current_cwd,
                    jobs: &mut jobs,
                    game_sessions: &mut game_sessions,
                    variables: &mut variables,
                    last_spawned_pid: &mut last_spawned_pid,
                    last_status: &mut last_status,
                    edit_session: &mut edit_session,
                },
            ) {
                Ok(ShellDispatchOutcome::Continue) => continue,
                Ok(ShellDispatchOutcome::Exit(code)) => {
                    shell_cleanup_game_sessions(runtime, &mut game_sessions, &mut jobs);
                    return code;
                }
                Ok(ShellDispatchOutcome::Unhandled) => continue,
                Err(code) => return code,
            }
        }
    }
    shell_cleanup_game_sessions(runtime, &mut game_sessions, &mut jobs);
    last_status
}

pub(crate) fn run_session_shell<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
) -> ExitCode {
    let mut script = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let read = match runtime.read(0, &mut chunk) {
            Ok(read) => read,
            Err(_) => return 193,
        };
        if read == 0 {
            break;
        }
        script.extend_from_slice(&chunk[..read]);
    }

    let text = match core::str::from_utf8(&script) {
        Ok(text) => text,
        Err(_) => return 194,
    };

    run_session_shell_script(runtime, context, text)
}
