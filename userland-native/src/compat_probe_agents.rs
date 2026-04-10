use super::*;

pub(crate) fn run_native_compat_proc_probe<B: SyscallBackend>(
    runtime: &Runtime<B>,
    bootstrap: &BootstrapArgs<'_>,
) -> ExitCode {
    let expected_name = bootstrap_env_value(bootstrap, "NGOS_PROCESS_NAME").unwrap_or(PROGRAM_NAME);
    let expected_cwd_lookup = bootstrap_env_value(bootstrap, "NGOS_CWD");
    let expected_image = bootstrap_env_value(bootstrap, "NGOS_IMAGE_PATH")
        .or_else(|| bootstrap.argv.first().copied())
        .unwrap_or(PROGRAM_NAME);
    let expected_exe = bootstrap_env_value(bootstrap, "NGOS_COMPAT_EXPECT_EXE")
        .or_else(|| bootstrap_env_value(bootstrap, "NGOS_IMAGE_PATH"))
        .unwrap_or(PROGRAM_NAME);
    let expected_cwd = bootstrap_env_value(bootstrap, "NGOS_COMPAT_EXPECT_CWD")
        .or_else(|| bootstrap_env_value(bootstrap, "NGOS_CWD"))
        .unwrap_or("/");
    let environ_marker = bootstrap_env_value(bootstrap, "NGOS_COMPAT_TARGET")
        .map(|target| format!("NGOS_COMPAT_TARGET={target}"));
    let request = ngos_shell_proc::CompatProcProbeRequest {
        expected_name,
        resolve_image: expected_image,
        expected_executable: expected_exe,
        expected_cwd_lookup,
        expected_cwd,
        environ_marker: environ_marker.as_deref(),
    };

    ngos_shell_proc::run_compat_proc_probe(runtime, &request, |line| write_line(runtime, line))
}
