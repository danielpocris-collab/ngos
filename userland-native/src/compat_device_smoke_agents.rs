use super::*;

pub(crate) fn run_native_compat_graphics_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    ngos_game_compat_runtime::run_native_compat_graphics_boot_smoke(runtime)
}

pub(crate) fn run_native_compat_audio_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    ngos_game_compat_runtime::run_native_compat_audio_boot_smoke(runtime)
}

#[inline(never)]
pub(crate) fn run_native_compat_input_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    ngos_game_compat_runtime::run_native_compat_input_boot_smoke(runtime)
}
