use alloc::format;
use alloc::string::String;

use ngos_user_abi::{Errno, ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub struct ProcessExecSmokeReport {
    pub metadata_line: String,
    pub recovery_line: String,
    pub spawn_line: String,
    pub success_line: String,
    pub state_line: String,
    pub final_marker: &'static str,
}

pub fn emit_process_exec_smoke_report<E>(
    report: &ProcessExecSmokeReport,
    mut emit: impl FnMut(&str) -> Result<(), E>,
) -> Result<(), E> {
    emit(&report.metadata_line)?;
    emit(&report.recovery_line)?;
    emit(&report.spawn_line)?;
    emit(&report.success_line)?;
    emit(&report.state_line)?;
    emit(report.final_marker)?;
    Ok(())
}

pub fn run_process_exec_smoke_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
    executable_path: &str,
    proc_probe_arg: &str,
) -> Result<ProcessExecSmokeReport, ExitCode> {
    let metadata_pid = runtime
        .spawn_path_process("metadata-only-child", "/bin/worker")
        .map_err(|_| 530)?;
    let (metadata_line, recovery_line) = match runtime.reap_process(metadata_pid) {
        Err(Errno::Again) => {
            if runtime.send_signal(metadata_pid, 9).is_err() {
                return Err(533);
            }
            let metadata_exit = runtime.reap_process(metadata_pid).map_err(|_| 534)?;
            (
                format!(
                    "process.exec.smoke.refusal pid={metadata_pid} mode=metadata-only outcome=expected"
                ),
                format!(
                    "process.exec.smoke.recovery pid={metadata_pid} exit={metadata_exit} mode=signal outcome=ok"
                ),
            )
        }
        Ok(metadata_exit) => (
            format!(
                "process.exec.smoke.observe pid={metadata_pid} mode=metadata-only exit={metadata_exit} outcome=ok"
            ),
            format!(
                "process.exec.smoke.recovery pid={metadata_pid} exit={metadata_exit} mode=natural outcome=ok"
            ),
        ),
        _ => return Err(531),
    };

    let env_process_name = String::from("NGOS_PROCESS_NAME=proc-exec-child");
    let env_image_path = format!("NGOS_IMAGE_PATH={executable_path}");
    let env_cwd = String::from("NGOS_CWD=/");
    let env_root = String::from("NGOS_ROOT_MOUNT_PATH=/");
    let env_expect_exe = format!("NGOS_COMPAT_EXPECT_EXE={executable_path}");
    let env_expect_cwd = String::from("NGOS_COMPAT_EXPECT_CWD=/");
    let env_marker = String::from("NGOS_COMPAT_TARGET=process-exec");
    let envp = [
        env_process_name.as_str(),
        env_image_path.as_str(),
        env_cwd.as_str(),
        env_root.as_str(),
        env_expect_exe.as_str(),
        env_expect_cwd.as_str(),
        env_marker.as_str(),
    ];
    let argv = [executable_path, proc_probe_arg];
    let child_pid = runtime
        .spawn_configured_process("proc-exec-child", executable_path, "/", &argv, &envp)
        .map_err(|_| 536)?;
    let _child = runtime.inspect_process(child_pid).map_err(|_| 537)?;
    let exit_code = runtime.reap_process(child_pid).map_err(|_| 540)?;
    if exit_code != 0 {
        return Err(541);
    }
    let state_line = if runtime.inspect_process(child_pid).is_ok() {
        format!("process.exec.smoke.state pid={child_pid} present=yes reaped=yes outcome=ok")
    } else {
        format!("process.exec.smoke.state pid={child_pid} present=no outcome=ok")
    };

    Ok(ProcessExecSmokeReport {
        metadata_line,
        recovery_line,
        spawn_line: format!(
            "process.exec.smoke.spawn pid={child_pid} mode=same-image-blocking outcome=ok"
        ),
        success_line: format!(
            "process.exec.smoke.success pid={child_pid} exit={exit_code} outcome=ok"
        ),
        state_line,
        final_marker: "process-exec-smoke-ok",
    })
}

pub fn run_process_exec_boot_smoke<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    executable_path: &str,
    proc_probe_arg: &str,
    mut write_line: F,
) -> ExitCode
where
    F: FnMut(&str) -> Result<(), ExitCode>,
{
    let report = match run_process_exec_smoke_report(runtime, executable_path, proc_probe_arg) {
        Ok(report) => report,
        Err(code) => return code,
    };
    if emit_process_exec_smoke_report(&report, |line| write_line(line)).is_err() {
        return 545;
    }
    0
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::{emit_process_exec_smoke_report, ProcessExecSmokeReport};

    #[test]
    fn process_exec_smoke_report_emits_expected_lines_in_order() {
        let report = ProcessExecSmokeReport {
            metadata_line: String::from("process.exec.smoke.refusal pid=2"),
            recovery_line: String::from("process.exec.smoke.recovery pid=2"),
            spawn_line: String::from("process.exec.smoke.spawn pid=3"),
            success_line: String::from("process.exec.smoke.success pid=3"),
            state_line: String::from("process.exec.smoke.state pid=3"),
            final_marker: "process-exec-smoke-ok",
        };
        let mut lines = Vec::new();
        emit_process_exec_smoke_report(&report, |line| {
            lines.push(String::from(line));
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(
            lines,
            [
                String::from("process.exec.smoke.refusal pid=2"),
                String::from("process.exec.smoke.recovery pid=2"),
                String::from("process.exec.smoke.spawn pid=3"),
                String::from("process.exec.smoke.success pid=3"),
                String::from("process.exec.smoke.state pid=3"),
                String::from("process-exec-smoke-ok"),
            ]
        );
    }
}
