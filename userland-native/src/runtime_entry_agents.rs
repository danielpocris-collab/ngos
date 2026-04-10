use super::*;

pub(crate) fn write_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    text: &str,
) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 190)?;
    Ok(())
}

pub(crate) fn run_program<B: SyscallBackend>(
    runtime: &Runtime<B>,
    bootstrap: &BootstrapArgs<'_>,
) -> ExitCode {
    #[cfg(target_os = "none")]
    debug_break(ngos_user_abi::USER_DEBUG_MARKER_MAIN, 0);

    if let Err(code) = validate_program_bootstrap(bootstrap) {
        return code;
    }
    if bootstrap_has_arg(bootstrap, COMPAT_PROC_PROBE_ARG) {
        return run_native_compat_proc_probe(runtime, bootstrap);
    }
    if bootstrap_has_arg(bootstrap, COMPAT_WORKER_ARG) {
        return run_native_game_compat_worker(runtime, bootstrap);
    }
    let boot_mode = bootstrap.is_boot_mode();
    if boot_mode {
        let context = match parse_and_validate_boot_context(bootstrap) {
            Ok(context) => context,
            Err(code) => return code,
        };
        if emit_boot_cpu_contract(runtime, &context).is_err() {
            return 127;
        }
        if let Some(proof_kind) = parse_boot_proof(bootstrap) {
            return run_boot_proof(runtime, &context, proof_kind);
        }
        let desktop_code = run_boot_desktop(runtime, &context);
        if desktop_code != 0 {
            return desktop_code;
        }
        return run_native_vm_boot_smoke(runtime);
    }

    run_native_surface_smoke(runtime, true)
}
