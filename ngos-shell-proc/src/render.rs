//! Process render functions: ps, jobs, job-info, process records, pending signals, procfs.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::{
    shell_render_record_value, ShellJob, ShellRecordField, ShellSemanticValue, ShellVariable,
};
use ngos_user_abi::bootstrap::SessionContext;
use ngos_user_abi::{ExitCode, NativeProcessCompatRecord, NativeProcessRecord, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::infra::{
    fixed_text_field, list_process_ids, native_process_state_label, read_process_text,
    read_procfs_all, scheduler_class_label,
};

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map(|_| ())
        .map_err(|_| 1)
}

fn shell_emit_lines<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    if text.is_empty() {
        return Ok(());
    }
    for line in text.lines() {
        write_line(runtime, line)?;
    }
    if text.as_bytes().last().is_some_and(|byte| *byte != b'\n') {
        write_line(runtime, "")?;
    }
    Ok(())
}

pub fn shell_resolve_self_pid<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
) -> Result<u64, ExitCode> {
    let mut matches = Vec::new();
    for pid in list_process_ids(runtime)? {
        let name = read_process_text(runtime, pid, Runtime::get_process_name).map_err(|_| 218)?;
        let process_cwd =
            read_process_text(runtime, pid, Runtime::get_process_cwd).map_err(|_| 218)?;
        let image =
            read_process_text(runtime, pid, Runtime::get_process_image_path).map_err(|_| 218)?;
        if name == context.process_name && process_cwd == cwd && image == context.image_path {
            matches.push(pid);
        }
    }
    matches.into_iter().max().ok_or(219)
}

pub fn shell_send_signal<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    signal: u8,
) -> Result<(), ExitCode> {
    runtime.send_signal(pid, signal).map_err(|_| 248)?;
    write_line(runtime, &format!("signal-sent pid={pid} signal={signal}"))
}

pub fn shell_render_ps<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    for pid in list_process_ids(runtime)? {
        let process = runtime.inspect_process(pid).map_err(|_| 221)?;
        let name = read_process_text(runtime, pid, Runtime::get_process_name)
            .unwrap_or_else(|_| String::from("unknown"));
        let cwd = read_process_text(runtime, pid, Runtime::get_process_cwd)
            .unwrap_or_else(|_| String::from("?"));
        write_line(
            runtime,
            &format!(
                "pid={pid} name={name} state={} cwd={cwd}",
                native_process_state_label(process.state)
            ),
        )?;
    }
    Ok(())
}

pub fn shell_render_jobs<B: SyscallBackend>(
    runtime: &Runtime<B>,
    jobs: &[ShellJob],
) -> Result<(), ExitCode> {
    if jobs.is_empty() {
        return write_line(runtime, "jobs=0");
    }
    for job in jobs {
        let state = if let Some(exit) = job.reaped_exit {
            format!("reaped:{exit}")
        } else {
            let status = runtime
                .inspect_process(job.pid)
                .ok()
                .map(|record| native_process_state_label(record.state).to_string())
                .unwrap_or_else(|| String::from("unknown"));
            format!("live:{status}")
        };
        write_line(
            runtime,
            &format!(
                "job pid={} name={} path={} state={} signals={}",
                job.pid, job.name, job.path, state, job.signal_count
            ),
        )?;
    }
    Ok(())
}

pub fn shell_render_job_info<B: SyscallBackend>(
    runtime: &Runtime<B>,
    jobs: &[ShellJob],
    pid: u64,
) -> Result<(), ExitCode> {
    let process = runtime.inspect_process(pid).map_err(|_| 252)?;
    let Some(job) = jobs.iter().find(|job| job.pid == pid) else {
        return Err(252);
    };
    let state = if let Some(exit) = job.reaped_exit {
        format!("reaped:{exit}")
    } else {
        format!("live:{}", native_process_state_label(process.state))
    };
    write_line(
        runtime,
        &format!(
            "job-info pid={} name={} path={} state={} signals={} exit={} pending={}",
            job.pid,
            job.name,
            job.path,
            state,
            job.signal_count,
            process.exit_code,
            process.pending_signal_count
        ),
    )
}

pub fn shell_render_process_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    let record: NativeProcessRecord = runtime.inspect_process(pid).map_err(|_| 251)?;
    let name = read_process_text(runtime, pid, Runtime::get_process_name).map_err(|_| 251)?;
    let image =
        read_process_text(runtime, pid, Runtime::get_process_image_path).map_err(|_| 251)?;
    let cwd = read_process_text(runtime, pid, Runtime::get_process_cwd).map_err(|_| 251)?;
    write_line(
        runtime,
        &format!(
            "pid={} name={} image={} cwd={} parent={} address-space={} thread={} state={} exit={} fds={} caps={} env={} regions={} threads={} pending={} session-reported={} session-status={} session-stage={} scheduler-class={} scheduler-budget={}",
            record.pid,
            name,
            image,
            cwd,
            record.parent,
            record.address_space,
            record.main_thread,
            native_process_state_label(record.state),
            record.exit_code,
            record.descriptor_count,
            record.capability_count,
            record.environment_count,
            record.memory_region_count,
            record.thread_count,
            record.pending_signal_count,
            record.session_reported,
            record.session_status,
            record.session_stage,
            scheduler_class_label(record.scheduler_class),
            record.scheduler_budget,
        ),
    )?;
    match runtime.inspect_process_compat(pid) {
        Ok(compat) => write_line(
            runtime,
            &format!(
                "pid={} compat target={} route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={} prefix={} exec={} cwd={}",
                compat.pid,
                fixed_text_field(&compat.target),
                fixed_text_field(&compat.route_class),
                fixed_text_field(&compat.handle_profile),
                fixed_text_field(&compat.path_profile),
                fixed_text_field(&compat.scheduler_profile),
                fixed_text_field(&compat.sync_profile),
                fixed_text_field(&compat.timer_profile),
                fixed_text_field(&compat.module_profile),
                fixed_text_field(&compat.event_profile),
                compat.requires_kernel_abi_shims,
                fixed_text_field(&compat.prefix),
                fixed_text_field(&compat.executable_path),
                fixed_text_field(&compat.working_dir),
            ),
        ),
        Err(_) => write_line(runtime, &format!("pid={pid} compat status=unavailable")),
    }
}

pub fn shell_render_process_compat_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    let record: NativeProcessCompatRecord = runtime.inspect_process_compat(pid).map_err(|_| 251)?;
    write_line(
        runtime,
        &format!(
            "pid={} target={} route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={} prefix={} exec={} cwd={} loader-route={} loader-mode={} loader-entry={} loader-requires-shims={}",
            record.pid,
            fixed_text_field(&record.target),
            fixed_text_field(&record.route_class),
            fixed_text_field(&record.handle_profile),
            fixed_text_field(&record.path_profile),
            fixed_text_field(&record.scheduler_profile),
            fixed_text_field(&record.sync_profile),
            fixed_text_field(&record.timer_profile),
            fixed_text_field(&record.module_profile),
            fixed_text_field(&record.event_profile),
            record.requires_kernel_abi_shims,
            fixed_text_field(&record.prefix),
            fixed_text_field(&record.executable_path),
            fixed_text_field(&record.working_dir),
            fixed_text_field(&record.loader_route_class),
            fixed_text_field(&record.loader_launch_mode),
            fixed_text_field(&record.loader_entry_profile),
            record.loader_requires_compat_shims,
        ),
    )
}

pub fn shell_render_pending_signals<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    blocked_only: bool,
) -> Result<(), ExitCode> {
    let mut buffer = [0u8; 64];
    let count = if blocked_only {
        runtime
            .blocked_pending_signals(pid, &mut buffer)
            .map_err(|_| 249)?
    } else {
        runtime.pending_signals(pid, &mut buffer).map_err(|_| 250)?
    };
    let rendered = if count == 0 {
        String::from("-")
    } else {
        buffer[..count]
            .iter()
            .map(|signal| format!("{signal}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    write_line(
        runtime,
        &format!(
            "pid={pid} {}={rendered}",
            if blocked_only {
                "blocked-pending-signals"
            } else {
                "pending-signals"
            }
        ),
    )
}

pub fn shell_render_procfs_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let bytes = read_procfs_all(runtime, path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    shell_emit_lines(runtime, text)
}

pub fn shell_render_self_view<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
    section: &str,
) -> Result<(), ExitCode> {
    let pid = shell_resolve_self_pid(runtime, context, cwd)?;
    shell_render_procfs_path(runtime, &format!("/proc/{pid}/{section}"))
}

pub fn shell_process_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<ShellVariable, ExitCode> {
    let record: NativeProcessRecord = runtime.inspect_process(pid).map_err(|_| 251)?;
    let name = read_process_text(runtime, pid, Runtime::get_process_name).map_err(|_| 251)?;
    let image =
        read_process_text(runtime, pid, Runtime::get_process_image_path).map_err(|_| 251)?;
    let cwd = read_process_text(runtime, pid, Runtime::get_process_cwd).map_err(|_| 251)?;
    let mut fields = Vec::with_capacity(12);
    fields.push(ShellRecordField {
        key: String::from("pid"),
        value: record.pid.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("name"),
        value: name,
    });
    fields.push(ShellRecordField {
        key: String::from("image"),
        value: image,
    });
    fields.push(ShellRecordField {
        key: String::from("cwd"),
        value: cwd,
    });
    fields.push(ShellRecordField {
        key: String::from("parent"),
        value: record.parent.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("state"),
        value: native_process_state_label(record.state).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("exit"),
        value: record.exit_code.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("fds"),
        value: record.descriptor_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("caps"),
        value: record.capability_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("pending"),
        value: record.pending_signal_count.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("scheduler_class"),
        value: scheduler_class_label(record.scheduler_class).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("scheduler_budget"),
        value: record.scheduler_budget.to_string(),
    });
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}

pub fn shell_process_compat_record_value<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<ShellVariable, ExitCode> {
    let record: NativeProcessCompatRecord = runtime.inspect_process_compat(pid).map_err(|_| 251)?;
    let mut fields = Vec::with_capacity(18);
    fields.push(ShellRecordField {
        key: String::from("pid"),
        value: record.pid.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("target"),
        value: fixed_text_field(&record.target).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("route"),
        value: fixed_text_field(&record.route_class).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("handles"),
        value: fixed_text_field(&record.handle_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("paths"),
        value: fixed_text_field(&record.path_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("scheduler"),
        value: fixed_text_field(&record.scheduler_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("sync"),
        value: fixed_text_field(&record.sync_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("timer"),
        value: fixed_text_field(&record.timer_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("module"),
        value: fixed_text_field(&record.module_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("event"),
        value: fixed_text_field(&record.event_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("requires_shims"),
        value: record.requires_kernel_abi_shims.to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("prefix"),
        value: fixed_text_field(&record.prefix).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("exec"),
        value: fixed_text_field(&record.executable_path).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("cwd"),
        value: fixed_text_field(&record.working_dir).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("loader_route"),
        value: fixed_text_field(&record.loader_route_class).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("loader_mode"),
        value: fixed_text_field(&record.loader_launch_mode).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("loader_entry"),
        value: fixed_text_field(&record.loader_entry_profile).to_string(),
    });
    fields.push(ShellRecordField {
        key: String::from("loader_requires_shims"),
        value: record.loader_requires_compat_shims.to_string(),
    });
    Ok(ShellVariable {
        name: String::from("_PIPE"),
        value: shell_render_record_value(&fields),
        semantic: Some(ShellSemanticValue::Record(fields)),
    })
}
