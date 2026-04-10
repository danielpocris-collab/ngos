//! Canonical subsystem role:
//! - subsystem: shell process control and inspection crate
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-proc`
//! - truth path role: process and job-control command surfaces for the ngos native shell

#![no_std]

extern crate alloc;

mod infra;
mod proc_cmd;
mod proc_contract;
mod process_exec_smoke;
mod process_probe;
mod render;
mod scheduler_smoke;

pub use infra::{
    fixed_text_field, list_process_ids, native_process_state_label, read_process_text,
    read_procfs_all, scheduler_class_label,
};
pub use proc_cmd::try_handle_proc_agent_command;
pub use process_exec_smoke::{
    emit_process_exec_smoke_report, run_process_exec_boot_smoke, run_process_exec_smoke_report,
    ProcessExecSmokeReport,
};
pub use process_probe::{
    emit_compat_proc_probe_report, run_compat_proc_probe, run_compat_proc_probe_report,
    CompatProcProbeRequest,
};
pub use render::{
    shell_process_compat_record_value, shell_process_record, shell_render_job_info,
    shell_render_jobs, shell_render_pending_signals, shell_render_process_compat_record,
    shell_render_process_record, shell_render_procfs_path, shell_render_ps, shell_render_self_view,
    shell_resolve_self_pid, shell_send_signal,
};
pub use scheduler_smoke::{
    run_scheduler_boot_smoke, run_scheduler_smoke_report, SchedulerSmokeReport,
};
