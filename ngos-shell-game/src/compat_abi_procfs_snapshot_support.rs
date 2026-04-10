use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_game_compat_runtime::CompatAbiProcProbeSnapshot;
use ngos_shell_proc::read_procfs_all;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

fn game_compat_procfs_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<String, ExitCode> {
    let bytes = read_procfs_all(runtime, path).map_err(|_| 464)?;
    core::str::from_utf8(&bytes)
        .map_err(|_| 464)
        .map(|text| text.trim().to_string())
}

pub fn game_compat_proc_probe_snapshot<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    include_environ: bool,
) -> Result<CompatAbiProcProbeSnapshot, ExitCode> {
    let proc_fd_text = game_compat_procfs_text(runtime, &format!("/proc/{pid}/fd"))?;
    let proc_fd_lines: Vec<&str> = proc_fd_text
        .lines()
        .filter(|line| !line.is_empty())
        .collect();
    let proc_environ = if include_environ {
        Some(game_compat_procfs_text(
            runtime,
            &format!("/proc/{pid}/environ"),
        )?)
    } else {
        None
    };
    Ok(CompatAbiProcProbeSnapshot {
        pid,
        fd_count: proc_fd_lines.len(),
        has_fd_0: proc_fd_lines
            .iter()
            .any(|line| line.starts_with("0\t") || line.starts_with("0 [")),
        has_fd_1: proc_fd_lines
            .iter()
            .any(|line| line.starts_with("1\t") || line.starts_with("1 [")),
        has_fd_2: proc_fd_lines
            .iter()
            .any(|line| line.starts_with("2\t") || line.starts_with("2 [")),
        cwd: game_compat_procfs_text(runtime, &format!("/proc/{pid}/cwd"))?,
        executable_path: game_compat_procfs_text(runtime, &format!("/proc/{pid}/exe"))?,
        cmdline: game_compat_procfs_text(runtime, &format!("/proc/{pid}/cmdline"))?,
        environ: proc_environ,
        invalid_fd_opened: read_procfs_all(runtime, &format!("/proc/{pid}/fd/9999")).is_ok(),
    })
}
