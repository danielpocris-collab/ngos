use super::*;

#[inline(never)]
pub(crate) fn run_native_bus_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    ngos_shell_bus::run_bus_boot_smoke(runtime, boot_bind_observe_contract)
}
