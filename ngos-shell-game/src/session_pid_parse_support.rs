use alloc::string::String;

use ngos_shell_types::resolve_shell_path;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::write_line;

pub fn parse_game_pid_arg<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    usage: &str,
) -> Result<u64, ExitCode> {
    match rest.trim().parse::<u64>().ok() {
        Some(pid) => Ok(pid),
        None => {
            let _ = write_line(runtime, usage);
            Err(2)
        }
    }
}

pub fn parse_game_pid_script_args<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    usage: &str,
) -> Result<(u64, String), ExitCode> {
    let mut parts = rest.split_whitespace();
    let pid = match parts.next().and_then(|value| value.parse::<u64>().ok()) {
        Some(pid) => pid,
        None => {
            let _ = write_line(runtime, usage);
            return Err(2);
        }
    };
    let Some(script_path) = parts.next() else {
        let _ = write_line(runtime, usage);
        return Err(2);
    };
    Ok((pid, resolve_shell_path(current_cwd, script_path)))
}
