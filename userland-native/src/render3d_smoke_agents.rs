use super::*;

pub(crate) fn run_native_render3d_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    match ngos_render3d::run_render3d_smoke(|line| write_line(runtime, line)) {
        Ok(()) => 0,
        Err(code) => code,
    }
}
