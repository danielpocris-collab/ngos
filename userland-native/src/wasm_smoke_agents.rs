use super::*;

pub(crate) fn run_native_wasm_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let pid = 1u64;
    let observer = "semantic-observer";
    if write_line(
        runtime,
        &format!("wasm.smoke.start component={observer} pid={pid} artifact=boot-proof"),
    )
    .is_err()
    {
        return 260;
    }

    match execute_wasm_component(
        runtime,
        WASM_BOOT_PROOF_COMPONENT,
        pid,
        &[WasmCapability::ObserveProcessCapabilityCount],
    ) {
        Err(WasmExecutionError::MissingCapability { capability, .. })
            if capability == WasmCapability::ObserveSystemProcessCount => {}
        Ok(_) => return 261,
        Err(_) => return 262,
    }
    if write_line(
        runtime,
        "wasm.smoke.refusal component=semantic-observer missing=observe-system-process-count outcome=expected",
    )
    .is_err()
    {
        return 263;
    }

    let report = match execute_wasm_component(
        runtime,
        WASM_BOOT_PROOF_COMPONENT,
        pid,
        &[
            WasmCapability::ObserveProcessCapabilityCount,
            WasmCapability::ObserveSystemProcessCount,
        ],
    ) {
        Ok(report) => report,
        Err(_) => return 264,
    };
    if write_line(
        runtime,
        "wasm.smoke.grants component=semantic-observer grants=observe-process-capability-count,observe-system-process-count",
    )
    .is_err()
    {
        return 265;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.observe component={observer} pid={} capabilities={} processes={}",
            report.observation.pid,
            report.observation.process_capability_count,
            report.observation.process_count
        ),
    )
    .is_err()
    {
        return 266;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.recovery component={observer} refusal=observe-system-process-count recovered=yes verdict={}",
            report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 267;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.result component={observer} verdict={} outcome=ok",
            report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 269;
    }
    let identity = "process-identity";
    if write_line(
        runtime,
        &format!("wasm.smoke.start component={identity} pid={pid} artifact=process-identity"),
    )
    .is_err()
    {
        return 271;
    }
    match execute_wasm_component(
        runtime,
        WASM_PROCESS_IDENTITY_COMPONENT,
        pid,
        &[WasmCapability::ObserveProcessStatusBytes],
    ) {
        Err(WasmExecutionError::MissingCapability { capability, .. })
            if capability == WasmCapability::ObserveProcessCwdRoot => {}
        Ok(_) => return 272,
        Err(_) => return 273,
    }
    if write_line(
        runtime,
        "wasm.smoke.refusal component=process-identity missing=observe-process-cwd-root outcome=expected",
    )
    .is_err()
    {
        return 274;
    }
    let identity_report = match execute_wasm_component(
        runtime,
        WASM_PROCESS_IDENTITY_COMPONENT,
        pid,
        &[
            WasmCapability::ObserveProcessStatusBytes,
            WasmCapability::ObserveProcessCwdRoot,
        ],
    ) {
        Ok(report) => report,
        Err(_) => return 275,
    };
    if write_line(
        runtime,
        "wasm.smoke.grants component=process-identity grants=observe-process-status-bytes,observe-process-cwd-root",
    )
    .is_err()
    {
        return 276;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.observe component={identity} pid={} status-bytes={} cwd-root={}",
            identity_report.observation.pid,
            identity_report.observation.process_status_bytes,
            if identity_report.observation.process_cwd_root {
                "yes"
            } else {
                "no"
            }
        ),
    )
    .is_err()
    {
        return 277;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.recovery component={identity} refusal=observe-process-cwd-root recovered=yes verdict={}",
            identity_report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 278;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.result component={identity} verdict={} outcome=ok",
            identity_report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 279;
    }
    if write_line(runtime, "wasm-smoke-ok").is_err() {
        return 270;
    }
    0
}
