//! Process command dispatcher (ps, jobs, kill, spawn, self-view, procfs).
//! Handles all proc commands that do NOT require game session context.
//! The `proc <pid> <section>` command (which uses GameCompatSession) is handled
//! by the dedicated proc-view owner in `ngos-shell-proc-view`.

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

use ngos_shell_types::{parse_u64_arg, ShellJob};
use ngos_user_abi::bootstrap::SessionContext;
use ngos_user_abi::{Errno, ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::proc_contract::{
    is_self_procfs_section, BLOCKED_SIGNALS_USAGE, FG_USAGE, JOB_INFO_USAGE, KILL_USAGE,
    PENDING_SIGNALS_USAGE, PROCESS_COMPAT_STATUS_USAGE, PROCESS_INFO_USAGE, REAP_USAGE,
    SPAWN_PATH_USAGE,
};
use crate::render::{
    shell_render_job_info, shell_render_jobs, shell_render_pending_signals,
    shell_render_process_compat_record, shell_render_process_record, shell_render_procfs_path,
    shell_render_ps, shell_render_self_view, shell_send_signal,
};

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map(|_| ())
        .map_err(|_| 1)
}

pub fn try_handle_proc_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
    line: &str,
    jobs: &mut Vec<ShellJob>,
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
                let _ = write_line(runtime, JOB_INFO_USAGE);
                return Some(Err(2));
            }
        };
        return Some(shell_render_job_info(runtime, jobs, pid).map_err(|_| 204));
    }
    if let Some(rest) = line.strip_prefix("fg ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, FG_USAGE);
                return Some(Err(2));
            }
        };
        let mut exit_code = None;
        for _ in 0..256 {
            match runtime.reap_process(pid) {
                Ok(code) => {
                    exit_code = Some(code);
                    break;
                }
                Err(Errno::Again) => {
                    let _ = runtime.inspect_process(pid);
                }
                Err(_) => return Some(Err(204)),
            }
        }
        let Some(exit_code) = exit_code else {
            return Some(Err(204));
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
                let _ = write_line(runtime, KILL_USAGE);
                return Some(Err(2));
            }
        };
        let signal = match parts.next().and_then(|value| value.parse::<u8>().ok()) {
            Some(signal) => signal,
            None => {
                let _ = write_line(runtime, KILL_USAGE);
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
                let _ = write_line(runtime, PENDING_SIGNALS_USAGE);
                return Some(Err(2));
            }
        };
        return Some(shell_render_pending_signals(runtime, pid, false).map_err(|_| 204));
    }
    if let Some(rest) = line.strip_prefix("blocked-signals ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, BLOCKED_SIGNALS_USAGE);
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
                let _ = write_line(runtime, SPAWN_PATH_USAGE);
                return Some(Err(2));
            }
        };
        let path = match parts.next() {
            Some(path) => path,
            None => {
                let _ = write_line(runtime, SPAWN_PATH_USAGE);
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
                let _ = write_line(runtime, REAP_USAGE);
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
                let _ = write_line(runtime, PROCESS_INFO_USAGE);
                return Some(Err(2));
            }
        };
        return Some(shell_render_process_record(runtime, pid).map_err(|_| 251));
    }
    if let Some(rest) = line.strip_prefix("process-compat-status ") {
        let pid = match rest.trim().parse::<u64>().ok() {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, PROCESS_COMPAT_STATUS_USAGE);
                return Some(Err(2));
            }
        };
        return Some(shell_render_process_compat_record(runtime, pid).map_err(|_| 251));
    }
    if let Some(section) = line.strip_prefix("self ") {
        let section = section.trim();
        if is_self_procfs_section(section) {
            return Some(shell_render_self_view(runtime, context, cwd, section).map_err(|_| 205));
        }
    }
    if is_self_procfs_section(line) {
        return Some(shell_render_self_view(runtime, context, cwd, line).map_err(|_| 205));
    }
    if let Some(path) = line.strip_prefix("cat ") {
        return Some(shell_render_procfs_path(runtime, path.trim()).map_err(|_| 205));
    }
    if line == "cpu-info" {
        return Some(shell_render_procfs_path(runtime, "/proc/system/cpu").map_err(|_| 205));
    }
    if line == "cpu-topology" {
        return Some(shell_render_procfs_path(runtime, "/proc/system/scheduler").map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("cpu-online ") {
        let cpu = match rest.trim().parse::<u64>() {
            Ok(cpu) => cpu,
            Err(_) => {
                let _ = write_line(runtime, "usage: cpu-online <cpu-index>");
                return Some(Err(2));
            }
        };
        return Some(runtime.set_cpu_online(cpu as usize, true).map(|_| ()).map_err(|_| 251));
    }
    if let Some(rest) = line.strip_prefix("cpu-offline ") {
        let cpu = match rest.trim().parse::<u64>() {
            Ok(cpu) => cpu,
            Err(_) => {
                let _ = write_line(runtime, "usage: cpu-offline <cpu-index>");
                return Some(Err(2));
            }
        };
        return Some(runtime.set_cpu_online(cpu as usize, false).map(|_| ()).map_err(|_| 251));
    }
    None
}
