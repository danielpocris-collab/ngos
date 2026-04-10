//! Canonical subsystem role:
//! - subsystem: native surface and readiness control
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing orchestration over readiness, file
//!   descriptor, and surface-related contracts
//!
//! Canonical contract families handled here:
//! - surface command contracts
//! - readiness and watch command contracts
//! - fd/surface inspection contracts
//!
//! This module may orchestrate surface-level commands, but it must not
//! redefine readiness truth, descriptor truth, or lower-layer ownership.

use super::*;

pub(super) enum SurfaceAgentOutcome {
    Continue,
    Exit(ExitCode),
}

pub(super) fn try_handle_surface_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
    variables: &mut Vec<ShellVariable>,
    line: &str,
    last_status: &mut i32,
) -> Option<Result<SurfaceAgentOutcome, ExitCode>> {
    if let Some(result) = ngos_shell_surface::try_handle_surface_front_command(
        runtime,
        cwd,
        variables,
        line,
        last_status,
    ) {
        return Some(result.map(|()| SurfaceAgentOutcome::Continue));
    }
    if let Some(rest) = line.strip_prefix("fdinfo ") {
        let pid = match shell_resolve_self_pid(runtime, context, cwd) {
            Ok(pid) => pid,
            Err(code) => return Some(Err(code)),
        };
        let fd = match parse_u64_arg(Some(rest.trim())) {
            Some(fd) => fd,
            None => {
                let _ = write_line(runtime, "usage: fdinfo <fd>");
                return Some(Err(2));
            }
        };
        return Some(
            shell_render_procfs_path(runtime, &format!("/proc/{pid}/fdinfo/{fd}"))
                .map(|_| SurfaceAgentOutcome::Continue)
                .map_err(|_| 205),
        );
    }
    if let Some(result) = try_handle_surface_smoke_command(runtime, context, line) {
        return Some(result.map(|()| SurfaceAgentOutcome::Continue));
    }
    if line == "exit" {
        return Some(Ok(SurfaceAgentOutcome::Exit(*last_status)));
    }
    if let Some(rest) = line.strip_prefix("exit ") {
        return Some(Ok(SurfaceAgentOutcome::Exit(parse_exit_code(Some(rest)))));
    }
    None
}
