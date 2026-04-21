use super::*;

pub(crate) fn run_native_device_runtime_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let graphics_code = run_native_compat_graphics_boot_smoke(runtime);
    if graphics_code != 0 {
        return graphics_code;
    }

    let audio_code = run_native_compat_audio_boot_smoke(runtime);
    if audio_code != 0 {
        return audio_code;
    }

    let input_code = run_native_compat_input_boot_smoke(runtime);
    if input_code != 0 {
        return input_code;
    }

    let network_code = run_native_network_boot_smoke(runtime);
    if network_code != 0 {
        return network_code;
    }

    let storage_code = run_native_storage_commit_boot_smoke(runtime);
    if storage_code != 0 {
        return storage_code;
    }

    let report = match build_device_runtime_smoke_report(runtime) {
        Ok(report) => report,
        Err(code) => return code,
    };
    for line in [
        report.graphics_line.as_str(),
        report.audio_line.as_str(),
        report.input_line.as_str(),
        report.network_interface_line.as_str(),
        report.network_socket_line.as_str(),
        report.storage_line.as_str(),
        report.final_marker,
    ] {
        if write_line(runtime, line).is_err() {
            return 183;
        }
    }
    0
}
