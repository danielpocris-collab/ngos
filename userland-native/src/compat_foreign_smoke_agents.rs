use super::*;

pub(crate) fn run_native_compat_abi_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    ngos_shell_game::run_native_compat_abi_boot_smoke(runtime)
}

pub(crate) fn run_native_compat_foreign_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    ngos_shell_game::run_native_compat_foreign_boot_smoke(runtime)
}
