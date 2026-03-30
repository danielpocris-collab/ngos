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
    if let Some(rest) = line.strip_prefix("fd-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(path) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: fd-watch <path> <read|write|priority|readwrite|readpriority|writepriority|all>",
            );
            return Some(Err(2));
        };
        let Some(mode) = parts.next() else {
            let _ = write_line(
                runtime,
                "usage: fd-watch <path> <read|write|priority|readwrite|readpriority|writepriority|all>",
            );
            return Some(Err(2));
        };
        let Some((readable, writable, priority)) = parse_readiness_interest(mode) else {
            let _ = write_line(
                runtime,
                "usage: fd-watch <path> <read|write|priority|readwrite|readpriority|writepriority|all>",
            );
            return Some(Err(2));
        };
        *last_status = match shell_watch_fd_readiness(
            runtime,
            &resolve_shell_path(cwd, path),
            readable,
            writable,
            priority,
        ) {
            Ok(fd) => {
                shell_set_variable(variables, "LAST_WATCH_FD", fd.to_string());
                0
            }
            Err(code) => code,
        };
        return Some(Ok(SurfaceAgentOutcome::Continue));
    }
    if line == "fd-ready" {
        *last_status = match shell_collect_readiness(runtime) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(SurfaceAgentOutcome::Continue));
    }
    if let Some(rest) = line.strip_prefix("blk-read ") {
        let mut parts = rest.split_whitespace();
        let Some(device_path) = parts.next() else {
            let _ = write_line(runtime, "usage: blk-read <device> <sector> [sector-count]");
            return Some(Err(2));
        };
        let Some(sector) = parse_u64_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: blk-read <device> <sector> [sector-count]");
            return Some(Err(2));
        };
        let sector_count = parts
            .next()
            .and_then(|token| parse_u64_arg(Some(token)))
            .map(|count| count as u32)
            .unwrap_or(1);
        *last_status = match shell_submit_block_read(
            runtime,
            &resolve_shell_path(cwd, device_path),
            sector,
            sector_count,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(SurfaceAgentOutcome::Continue));
    }
    if let Some(path) = line.strip_prefix("driver-read ") {
        let resolved = resolve_shell_path(cwd, path.trim());
        return Some(
            shell_driver_read(runtime, &resolved)
                .map(|_| SurfaceAgentOutcome::Continue)
                .map_err(|_| 205),
        );
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
    if let Some(rest) = line.strip_prefix("echo ") {
        return Some(
            write_line(runtime, rest)
                .map(|_| SurfaceAgentOutcome::Continue)
                .map_err(|_| 197),
        );
    }
    if line == "smoke" {
        let code = run_native_surface_smoke(runtime, false);
        if code != 0 {
            return Some(Err(code));
        }
        return Some(
            write_line(runtime, "smoke-ok")
                .map(|_| SurfaceAgentOutcome::Continue)
                .map_err(|_| 198),
        );
    }
    if line == "vfs-smoke" {
        let code = run_native_vfs_boot_smoke(runtime);
        if code != 0 {
            return Some(Err(code));
        }
        return Some(
            write_line(runtime, "vfs-smoke-ok")
                .map(|_| SurfaceAgentOutcome::Continue)
                .map_err(|_| 198),
        );
    }
    if line == "wasm-smoke" {
        let code = run_native_wasm_boot_smoke(runtime);
        if code != 0 {
            return Some(Err(code));
        }
        return Some(
            write_line(runtime, "wasm-smoke-ok")
                .map(|_| SurfaceAgentOutcome::Continue)
                .map_err(|_| 198),
        );
    }
    if line == "exit" {
        return Some(Ok(SurfaceAgentOutcome::Exit(*last_status)));
    }
    if let Some(rest) = line.strip_prefix("exit ") {
        return Some(Ok(SurfaceAgentOutcome::Exit(parse_exit_code(Some(rest)))));
    }
    None
}
