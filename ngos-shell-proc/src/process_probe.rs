use alloc::format;
use alloc::string::{String, ToString};

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{list_process_ids, read_process_text, read_procfs_all};

pub struct CompatProcProbeSnapshot {
    pub pid: u64,
    pub fd_count: usize,
    pub has_fd_0: bool,
    pub has_fd_1: bool,
    pub has_fd_2: bool,
    pub cwd: String,
    pub executable_path: String,
    pub cmdline: String,
    pub environ: Option<String>,
    pub invalid_fd_opened: bool,
}

pub struct CompatProcProbeExpectation {
    pub cwd: String,
    pub executable_path: String,
    pub require_environ: bool,
    pub environ_marker: Option<String>,
}

pub enum CompatProcProbeMismatch {
    DescriptorSet,
    Cmdline,
    Environ,
    InvalidFdRefusal,
    Identity,
}

pub struct CompatProcProbeRequest<'a> {
    pub expected_name: &'a str,
    pub resolve_image: &'a str,
    pub expected_executable: &'a str,
    pub expected_cwd_lookup: Option<&'a str>,
    pub expected_cwd: &'a str,
    pub environ_marker: Option<&'a str>,
}

fn compat_procfs_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<String, ExitCode> {
    let bytes = read_procfs_all(runtime, path).map_err(|_| 464)?;
    core::str::from_utf8(&bytes)
        .map_err(|_| 464)
        .map(|text| text.trim().to_string())
}

fn compat_proc_probe_snapshot<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    include_environ: bool,
) -> Result<CompatProcProbeSnapshot, ExitCode> {
    let proc_fd_text = compat_procfs_text(runtime, &format!("/proc/{pid}/fd"))?;
    let proc_fd_lines = proc_fd_text.lines().filter(|line| !line.is_empty());
    let proc_environ = if include_environ {
        Some(compat_procfs_text(
            runtime,
            &format!("/proc/{pid}/environ"),
        )?)
    } else {
        None
    };
    let fd_lines = proc_fd_lines.collect::<alloc::vec::Vec<_>>();
    Ok(CompatProcProbeSnapshot {
        pid,
        fd_count: fd_lines.len(),
        has_fd_0: fd_lines
            .iter()
            .any(|line| line.starts_with("0\t") || line.starts_with("0 [")),
        has_fd_1: fd_lines
            .iter()
            .any(|line| line.starts_with("1\t") || line.starts_with("1 [")),
        has_fd_2: fd_lines
            .iter()
            .any(|line| line.starts_with("2\t") || line.starts_with("2 [")),
        cwd: compat_procfs_text(runtime, &format!("/proc/{pid}/cwd"))?,
        executable_path: compat_procfs_text(runtime, &format!("/proc/{pid}/exe"))?,
        cmdline: compat_procfs_text(runtime, &format!("/proc/{pid}/cmdline"))?,
        environ: proc_environ,
        invalid_fd_opened: read_procfs_all(runtime, &format!("/proc/{pid}/fd/9999")).is_ok(),
    })
}

fn resolve_compat_probe_pid<B: SyscallBackend>(
    runtime: &Runtime<B>,
    request: &CompatProcProbeRequest<'_>,
) -> Result<u64, ExitCode> {
    let mut matches = alloc::vec::Vec::new();
    for pid in list_process_ids(runtime)? {
        let name = read_process_text(runtime, pid, Runtime::get_process_name).map_err(|_| 464)?;
        let image =
            read_process_text(runtime, pid, Runtime::get_process_image_path).map_err(|_| 464)?;
        let cwd = if request.expected_cwd_lookup.is_some() {
            Some(read_process_text(runtime, pid, Runtime::get_process_cwd).map_err(|_| 464)?)
        } else {
            None
        };
        if name == request.expected_name
            && process_image_matches(&image, request.resolve_image)
            && request
                .expected_cwd_lookup
                .is_none_or(|value| cwd.as_deref() == Some(value))
        {
            matches.push(pid);
        }
    }
    matches.into_iter().max().ok_or(464)
}

pub fn emit_compat_proc_probe_report<E>(
    pid: u64,
    environ_marker: Option<&str>,
    snapshot: &CompatProcProbeSnapshot,
    mut emit: impl FnMut(&str) -> Result<(), E>,
) -> Result<(), E> {
    for path in [
        format!("/proc/{pid}/fd"),
        format!("/proc/{pid}/cwd"),
        format!("/proc/{pid}/exe"),
        format!("/proc/{pid}/cmdline"),
    ] {
        emit(&proc_step_line(pid, &path))?;
    }
    if let Some(marker) = environ_marker {
        emit(&proc_step_line(pid, &format!("/proc/{pid}/environ")))?;
        emit(&proc_environ_line(pid, marker))?;
    }
    emit(&proc_success_line(snapshot))?;
    emit(&proc_refusal_line(pid))?;
    emit(&proc_recovery_line(pid))?;
    Ok(())
}

pub fn run_compat_proc_probe_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
    request: &CompatProcProbeRequest<'_>,
) -> Result<(u64, CompatProcProbeSnapshot), ExitCode> {
    let pid = resolve_compat_probe_pid(runtime, request)?;
    let snapshot = compat_proc_probe_snapshot(runtime, pid, request.environ_marker.is_some())?;
    let expectation = CompatProcProbeExpectation {
        cwd: String::from(request.expected_cwd),
        executable_path: String::from(request.expected_executable),
        require_environ: request.environ_marker.is_some(),
        environ_marker: request.environ_marker.map(String::from),
    };
    if let Err(mismatch) = verify_proc_probe(&snapshot, &expectation) {
        return Err(write_failure_code(
            runtime,
            &snapshot,
            &expectation,
            &mismatch,
        ));
    }
    Ok((pid, snapshot))
}

fn write_failure_code<B: SyscallBackend>(
    runtime: &Runtime<B>,
    snapshot: &CompatProcProbeSnapshot,
    expectation: &CompatProcProbeExpectation,
    mismatch: &CompatProcProbeMismatch,
) -> ExitCode {
    let _ = runtime.writev(
        1,
        &[
            proc_failure_line(snapshot, expectation, mismatch).as_bytes(),
            b"\n",
        ],
    );
    464
}

fn process_image_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn process_image_matches(actual: &str, expected: &str) -> bool {
    actual == expected || process_image_name(actual) == process_image_name(expected)
}

fn process_cmdline_matches(cmdline: &str, expected: &str) -> bool {
    cmdline
        .split(['\0', '\n'])
        .filter(|segment| !segment.is_empty())
        .any(|segment| process_image_matches(segment, expected))
}

fn verify_proc_probe(
    snapshot: &CompatProcProbeSnapshot,
    expectation: &CompatProcProbeExpectation,
) -> Result<(), CompatProcProbeMismatch> {
    if !(snapshot.has_fd_0 && snapshot.has_fd_1 && snapshot.has_fd_2 && snapshot.fd_count >= 3) {
        return Err(CompatProcProbeMismatch::DescriptorSet);
    }
    if !process_cmdline_matches(&snapshot.cmdline, &expectation.executable_path) {
        return Err(CompatProcProbeMismatch::Cmdline);
    }
    if expectation.require_environ {
        let Some(environ) = snapshot.environ.as_deref() else {
            return Err(CompatProcProbeMismatch::Environ);
        };
        if environ.is_empty() {
            return Err(CompatProcProbeMismatch::Environ);
        }
        if expectation.environ_marker.is_some()
            && !environ.contains(expectation.environ_marker.as_deref().unwrap_or(""))
        {
            return Err(CompatProcProbeMismatch::Environ);
        }
    }
    if snapshot.invalid_fd_opened {
        return Err(CompatProcProbeMismatch::InvalidFdRefusal);
    }
    if snapshot.cwd != expectation.cwd
        || !process_image_matches(&snapshot.executable_path, &expectation.executable_path)
    {
        return Err(CompatProcProbeMismatch::Identity);
    }
    Ok(())
}

fn proc_failure_line(
    snapshot: &CompatProcProbeSnapshot,
    expectation: &CompatProcProbeExpectation,
    mismatch: &CompatProcProbeMismatch,
) -> String {
    match mismatch {
        CompatProcProbeMismatch::DescriptorSet => format!(
            "compat.abi.smoke.proc-failure pid={} fd-count={} has-0={} has-1={} has-2={} cwd={} exe={}",
            snapshot.pid,
            snapshot.fd_count,
            snapshot.has_fd_0,
            snapshot.has_fd_1,
            snapshot.has_fd_2,
            snapshot.cwd,
            snapshot.executable_path
        ),
        CompatProcProbeMismatch::Cmdline => format!(
            "compat.abi.smoke.proc-failure pid={} cmdline={}",
            snapshot.pid, snapshot.cmdline
        ),
        CompatProcProbeMismatch::Environ => format!(
            "compat.abi.smoke.proc-failure pid={} environ={}",
            snapshot.pid,
            snapshot.environ.as_deref().unwrap_or("")
        ),
        CompatProcProbeMismatch::InvalidFdRefusal => format!(
            "compat.abi.smoke.proc-failure pid={} path=/proc/{}/fd/9999 reason=unexpected-success",
            snapshot.pid, snapshot.pid
        ),
        CompatProcProbeMismatch::Identity => format!(
            "compat.abi.smoke.proc-failure pid={} cwd={} observed-cwd={} exe={} observed-exe={}",
            snapshot.pid,
            expectation.cwd,
            snapshot.cwd,
            expectation.executable_path,
            snapshot.executable_path
        ),
    }
}

fn proc_success_line(snapshot: &CompatProcProbeSnapshot) -> String {
    format!(
        "compat.abi.smoke.proc.success pid={} fd-count={} fd0=present fd1=present fd2=present cwd={} exe={} cmdline=present",
        snapshot.pid, snapshot.fd_count, snapshot.cwd, snapshot.executable_path
    )
}

fn proc_step_line(pid: u64, path: &str) -> String {
    format!("compat.abi.smoke.proc.step pid={} path={}", pid, path)
}

fn proc_environ_line(pid: u64, marker: &str) -> String {
    format!(
        "compat.abi.smoke.proc.environ pid={} outcome=ok marker={}",
        pid, marker
    )
}

fn proc_refusal_line(pid: u64) -> String {
    format!(
        "compat.abi.smoke.proc.refusal pid={} path=/proc/{}/fd/9999 outcome=expected",
        pid, pid
    )
}

fn proc_recovery_line(pid: u64) -> String {
    format!(
        "compat.abi.smoke.proc.recovery pid={} fd-list=ok outcome=ok",
        pid
    )
}

pub fn run_compat_proc_probe<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    request: &CompatProcProbeRequest<'_>,
    mut write_line: F,
) -> ExitCode
where
    F: FnMut(&str) -> Result<(), ExitCode>,
{
    let (pid, snapshot) = match run_compat_proc_probe_report(runtime, request) {
        Ok(result) => result,
        Err(code) => return code,
    };
    if emit_compat_proc_probe_report(pid, request.environ_marker, &snapshot, |line| {
        write_line(line)
    })
    .is_err()
    {
        return 464;
    }
    0
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::{
        emit_compat_proc_probe_report, process_cmdline_matches, verify_proc_probe,
        CompatProcProbeExpectation, CompatProcProbeMismatch, CompatProcProbeSnapshot,
    };

    #[test]
    fn process_cmdline_matches_nul_delimited_payloads() {
        assert!(process_cmdline_matches(
            "/kernel/ngos-userland-native\0--compat-proc-probe\0",
            "/kernel/ngos-userland-native",
        ));
    }

    #[test]
    fn process_cmdline_matches_newline_delimited_payloads() {
        assert!(process_cmdline_matches(
            "/kernel/ngos-userland-native\n--compat-proc-probe",
            "/kernel/ngos-userland-native",
        ));
    }

    #[test]
    fn verify_proc_probe_reports_identity_mismatch() {
        let snapshot = CompatProcProbeSnapshot {
            pid: 7,
            fd_count: 3,
            has_fd_0: true,
            has_fd_1: true,
            has_fd_2: true,
            cwd: String::from("/observed"),
            executable_path: String::from("/kernel/other"),
            cmdline: String::from("/kernel/ngos-userland-native"),
            environ: Some(String::from("NGOS_COMPAT_TARGET=process-exec")),
            invalid_fd_opened: false,
        };
        let expectation = CompatProcProbeExpectation {
            cwd: String::from("/expected"),
            executable_path: String::from("/kernel/ngos-userland-native"),
            require_environ: true,
            environ_marker: Some(String::from("NGOS_COMPAT_TARGET=process-exec")),
        };
        assert!(matches!(
            verify_proc_probe(&snapshot, &expectation),
            Err(CompatProcProbeMismatch::Identity)
        ));
    }

    #[test]
    fn emit_compat_proc_probe_report_emits_expected_lines_in_order() {
        let snapshot = CompatProcProbeSnapshot {
            pid: 9,
            fd_count: 3,
            has_fd_0: true,
            has_fd_1: true,
            has_fd_2: true,
            cwd: String::from("/"),
            executable_path: String::from("/kernel/ngos-userland-native"),
            cmdline: String::from("/kernel/ngos-userland-native"),
            environ: Some(String::from("NGOS_COMPAT_TARGET=process-exec")),
            invalid_fd_opened: false,
        };
        let mut lines = Vec::new();
        emit_compat_proc_probe_report(
            9,
            Some("NGOS_COMPAT_TARGET=process-exec"),
            &snapshot,
            |line| {
                lines.push(String::from(line));
                Ok::<_, ()>(())
            },
        )
        .unwrap();
        assert_eq!(
            lines,
            [
                String::from("compat.abi.smoke.proc.step pid=9 path=/proc/9/fd"),
                String::from("compat.abi.smoke.proc.step pid=9 path=/proc/9/cwd"),
                String::from("compat.abi.smoke.proc.step pid=9 path=/proc/9/exe"),
                String::from("compat.abi.smoke.proc.step pid=9 path=/proc/9/cmdline"),
                String::from("compat.abi.smoke.proc.step pid=9 path=/proc/9/environ"),
                String::from(
                    "compat.abi.smoke.proc.environ pid=9 outcome=ok marker=NGOS_COMPAT_TARGET=process-exec",
                ),
                String::from(
                    "compat.abi.smoke.proc.success pid=9 fd-count=3 fd0=present fd1=present fd2=present cwd=/ exe=/kernel/ngos-userland-native cmdline=present",
                ),
                String::from(
                    "compat.abi.smoke.proc.refusal pid=9 path=/proc/9/fd/9999 outcome=expected",
                ),
                String::from("compat.abi.smoke.proc.recovery pid=9 fd-list=ok outcome=ok"),
            ]
        );
    }
}
