use super::*;

#[inline(never)]
pub(crate) fn run_native_network_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    ngos_shell_network::run_network_boot_smoke(runtime)
}
