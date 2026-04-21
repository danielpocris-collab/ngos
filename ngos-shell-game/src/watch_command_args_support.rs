use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{parse_game_lane_kind, write_line};

pub fn parse_game_pid_lane_args<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    usage: &str,
) -> Result<(u64, CompatLaneKind), ExitCode> {
    let mut parts = rest.split_whitespace();
    let pid = match parts.next().and_then(|value| value.parse::<u64>().ok()) {
        Some(pid) => pid,
        None => {
            let _ = write_line(runtime, usage);
            return Err(2);
        }
    };
    let Some(kind) = parts.next().and_then(parse_game_lane_kind) else {
        let _ = write_line(runtime, usage);
        return Err(2);
    };
    Ok((pid, kind))
}
