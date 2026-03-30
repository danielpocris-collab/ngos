use super::*;

pub(super) fn try_handle_proc_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
    line: &str,
    jobs: &mut Vec<ShellJob>,
    game_sessions: &[GameCompatSession],
    last_spawned_pid: &mut Option<u64>,
) -> Option<Result<(), ExitCode>> {
    if line == "ps" {
        return Some(shell_render_ps(runtime).map_err(|_| 204));
    }
    if line == "jobs" {
        return Some(shell_render_jobs(runtime, jobs).map_err(|_| 204));
    }
    if let Some(rest) = line.strip_prefix("job-info ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: job-info <pid>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_job_info(runtime, jobs, pid).map_err(|_| 204));
    }
    if let Some(rest) = line.strip_prefix("fg ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: fg <pid>");
                return Some(Err(2));
            }
        };
        let exit_code = match runtime.reap_process(pid) {
            Ok(code) => code,
            Err(_) => return Some(Err(204)),
        };
        if let Some(job) = jobs.iter_mut().find(|job| job.pid == pid) {
            job.reaped_exit = Some(exit_code);
        }
        return Some(
            write_line(
                runtime,
                &format!("foreground-complete pid={pid} exit={exit_code}"),
            )
            .map_err(|_| 204),
        );
    }
    if let Some(rest) = line.strip_prefix("kill ") {
        let mut parts = rest.split_whitespace();
        let pid = match parse_u64_arg(parts.next()) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: kill <pid> <signal>");
                return Some(Err(2));
            }
        };
        let signal = match parts.next().and_then(|value| value.parse::<u8>().ok()) {
            Some(signal) => signal,
            None => {
                let _ = write_line(runtime, "usage: kill <pid> <signal>");
                return Some(Err(2));
            }
        };
        if shell_send_signal(runtime, pid, signal).is_err() {
            return Some(Err(204));
        }
        if let Some(job) = jobs.iter_mut().find(|job| job.pid == pid) {
            job.signal_count += 1;
        }
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("pending-signals ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: pending-signals <pid>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_pending_signals(runtime, pid, false).map_err(|_| 204));
    }
    if let Some(rest) = line.strip_prefix("blocked-signals ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: blocked-signals <pid>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_pending_signals(runtime, pid, true).map_err(|_| 204));
    }
    if let Some(rest) = line.strip_prefix("spawn-path ") {
        let mut parts = rest.split_whitespace();
        let name = match parts.next() {
            Some(name) => name,
            None => {
                let _ = write_line(runtime, "usage: spawn-path <name> <path>");
                return Some(Err(2));
            }
        };
        let path = match parts.next() {
            Some(path) => path,
            None => {
                let _ = write_line(runtime, "usage: spawn-path <name> <path>");
                return Some(Err(2));
            }
        };
        let pid = match runtime.spawn_path_process(name, path) {
            Ok(pid) => pid,
            Err(_) => return Some(Err(204)),
        };
        *last_spawned_pid = Some(pid);
        jobs.push(ShellJob {
            pid,
            name: name.to_string(),
            path: path.to_string(),
            reaped_exit: None,
            signal_count: 0,
        });
        return Some(
            write_line(
                runtime,
                &format!("process-spawned pid={pid} name={name} path={path}"),
            )
            .map_err(|_| 204),
        );
    }
    if let Some(rest) = line.strip_prefix("reap ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: reap <pid>");
                return Some(Err(2));
            }
        };
        let exit_code = match runtime.reap_process(pid) {
            Ok(code) => code,
            Err(_) => return Some(Err(204)),
        };
        if let Some(job) = jobs.iter_mut().find(|job| job.pid == pid) {
            job.reaped_exit = Some(exit_code);
        }
        return Some(
            write_line(
                runtime,
                &format!("process-reaped pid={pid} exit={exit_code}"),
            )
            .map_err(|_| 204),
        );
    }
    if let Some(rest) = line.strip_prefix("process-info ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: process-info <pid>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_process_record(runtime, pid).map_err(|_| 251));
    }
    if let Some(rest) = line.strip_prefix("proc ") {
        let mut parts = rest.split_whitespace();
        let pid = match parse_u64_arg(parts.next()) {
            Some(pid) => pid,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: proc <pid> <status|stat|cmdline|cwd|environ|exe|auxv|maps|vmobjects|vmdecisions|vmepisodes|fd|caps|queues>",
                );
                return Some(Err(2));
            }
        };
        let section = match parts.next() {
            Some(section) => section,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: proc <pid> <status|stat|cmdline|cwd|environ|exe|auxv|maps|vmobjects|vmdecisions|vmepisodes|fd|caps|queues>",
                );
                return Some(Err(2));
            }
        };
        return Some(
            shell_render_process_view(runtime, game_sessions, pid, section).map_err(|_| 205),
        );
    }
    if let Some(section) = line.strip_prefix("self ") {
        let section = section.trim();
        if is_procfs_section(section) {
            return Some(shell_render_self_view(runtime, context, cwd, section).map_err(|_| 205));
        }
    }
    if is_procfs_section(line) {
        return Some(shell_render_self_view(runtime, context, cwd, line).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("cat ") {
        return Some(shell_render_procfs_path(runtime, path.trim()).map_err(|_| 205));
    }
    None
}

fn is_procfs_section(section: &str) -> bool {
    matches!(
        section,
        "status"
            | "stat"
            | "cmdline"
            | "cwd"
            | "environ"
            | "exe"
            | "auxv"
            | "maps"
            | "vmobjects"
            | "vmdecisions"
            | "vmepisodes"
            | "fd"
            | "caps"
            | "queues"
    )
}
