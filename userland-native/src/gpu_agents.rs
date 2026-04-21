use super::*;

pub(super) fn try_handle_gpu_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    variables: &mut Vec<ShellVariable>,
    line: &str,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    ngos_shell_gpu::try_handle_gpu_agent_command(runtime, cwd, variables, line, last_status)
}
