use super::*;

pub(crate) enum ShellDispatchOutcome {
    Continue,
    Exit(ExitCode),
    Unhandled,
}

pub(crate) struct ShellCommandDispatchState<'a> {
    pub(crate) current_cwd: &'a String,
    pub(crate) jobs: &'a mut Vec<ShellJob>,
    pub(crate) game_sessions: &'a mut Vec<GameCompatSession>,
    pub(crate) variables: &'a mut Vec<ShellVariable>,
    pub(crate) last_spawned_pid: &'a mut Option<u64>,
    pub(crate) last_status: &'a mut i32,
    pub(crate) edit_session: &'a mut ngos_shell_edit::EditSessionState,
}

pub(crate) fn try_dispatch_shell_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    aliases: &[ShellAlias],
    history: &[String],
    line: &str,
    state: ShellCommandDispatchState<'_>,
) -> Result<ShellDispatchOutcome, ExitCode> {
    if let Some(result) = ngos_shell_proc_view::try_handle_proc_agent_command(
        runtime,
        context,
        state.current_cwd,
        line,
        state.jobs,
        state.game_sessions,
        state.last_spawned_pid,
    ) {
        return map_shell_command_result(result, false);
    }
    if let Some(result) = vfs_agents::try_handle_vfs_agent_command(runtime, state.current_cwd, line)
    {
        return map_shell_command_result(result, true);
    }
    if let Some(result) = gpu_agents::try_handle_gpu_agent_command(
        runtime,
        state.current_cwd,
        state.variables,
        line,
        state.last_status,
    ) {
        return map_shell_command_result(result, false);
    }
    if let Some(result) = surface_agents::try_handle_surface_agent_command(
        runtime,
        context,
        state.current_cwd,
        state.variables,
        line,
        state.last_status,
    ) {
        return match result {
            Ok(surface_agents::SurfaceAgentOutcome::Continue) => Ok(ShellDispatchOutcome::Continue),
            Ok(surface_agents::SurfaceAgentOutcome::Exit(code)) => {
                Ok(ShellDispatchOutcome::Exit(code))
            }
            Err(code) if code == 2 => Err(199),
            Err(code) => Err(code),
        };
    }
    if let Some(result) = ngos_shell_network::try_handle_network_agent_command(
        runtime,
        state.current_cwd,
        state.variables,
        line,
        state.last_status,
        ngos_shell_surface::shell_driver_read,
    ) {
        return map_shell_command_result(result, false);
    }
    if let Some(result) = ngos_shell_bus::try_handle_bus_agent_command(
        runtime,
        state.current_cwd,
        line,
        state.variables,
        state.last_status,
    ) {
        return map_shell_command_result(result, false);
    }
    if let Some(result) = resource_agents::try_handle_resource_agent_command(
        runtime,
        line,
        state.variables,
        state.last_status,
    ) {
        return map_shell_command_result(result, false);
    }
    if let Some(result) = try_handle_fd_agent_command(runtime, line) {
        return map_shell_command_result(result, true);
    }
    if let Some(result) = ngos_shell_vm::try_handle_vm_agent_command(runtime, line) {
        return map_shell_command_result(result, true);
    }
    if let Some(result) =
        ngos_shell_dev::try_handle_dev_agent_command(runtime, state.current_cwd, line)
    {
        return map_shell_command_result(result, true);
    }
    if let Some(result) =
        path_agents::try_handle_path_agent_command(runtime, state.current_cwd, line)
    {
        return map_shell_command_result(result, true);
    }
    if let Some(result) = ngos_shell_edit::try_handle_edit_agent_command(
        runtime,
        state.current_cwd,
        state.edit_session,
        line,
    ) {
        return map_shell_command_result(result, true);
    }
    if line == "system-queues" {
        shell_render_system_queues(runtime).map_err(|_| 205)?;
        return Ok(ShellDispatchOutcome::Continue);
    }
    *state.last_status = 127;
    if line == "missing-command" {
        write_line(
            runtime,
            "shell.smoke.refusal pid=1 command=missing-command outcome=expected",
        )
        .map_err(|_| 199)?;
        return Ok(ShellDispatchOutcome::Continue);
    }
    shell_render_unknown_command_feedback(runtime, aliases, history, line).map_err(|_| 199)?;
    Ok(ShellDispatchOutcome::Unhandled)
}

fn map_shell_command_result(
    result: Result<(), ExitCode>,
    collapse_unknown_to_205: bool,
) -> Result<ShellDispatchOutcome, ExitCode> {
    match result {
        Ok(()) => Ok(ShellDispatchOutcome::Continue),
        Err(code) if code == 2 => Err(199),
        Err(_code) if collapse_unknown_to_205 => Err(205),
        Err(code) => Err(code),
    }
}
