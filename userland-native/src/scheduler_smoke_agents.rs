use super::*;

pub(crate) fn run_native_scheduler_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    ngos_shell_proc::run_scheduler_boot_smoke(runtime, boot_bind_observe_contract)
}
