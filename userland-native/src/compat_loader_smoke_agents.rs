use super::*;

pub(crate) fn run_native_compat_loader_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    ngos_shell_game::run_native_compat_loader_boot_smoke(runtime)
}
