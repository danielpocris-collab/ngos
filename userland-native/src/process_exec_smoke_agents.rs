use super::*;

pub(crate) fn run_native_process_exec_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let image_path = native_game_smoke_image_path(runtime);
    ngos_shell_proc::run_process_exec_boot_smoke(
        runtime,
        image_path.as_str(),
        COMPAT_PROC_PROBE_ARG,
        |line| write_line(runtime, line),
    )
}
