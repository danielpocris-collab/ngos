use alloc::format;
use alloc::string::String;

use ngos_user_abi::{Errno, ExitCode, NativeSchedulerClass, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{native_process_state_label, read_procfs_all};

fn scheduler_procfs_contains_all_markers(text: &str, markers: &[&str]) -> bool {
    markers.iter().all(|marker| text.contains(marker))
}

fn scheduler_procfs_contains_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    markers: &[&str],
) -> Result<bool, ExitCode> {
    let scheduler = read_procfs_all(runtime, "/proc/system/scheduler")?;
    let text = core::str::from_utf8(&scheduler).map_err(|_| 470)?;
    Ok(scheduler_procfs_contains_all_markers(text, markers))
}

fn scheduler_text_contains_pid(text: &str, pid: u64) -> bool {
    let pid_marker = format!("pid={pid}");
    let tids_exact = format!("tids=[{pid}]");
    let tids_prefix = format!("tids=[{pid},");
    let tids_suffix = format!(",{pid}]");
    let tids_middle = format!(",{pid},");
    text.contains(&pid_marker)
        || text.contains(&tids_exact)
        || text.contains(&tids_prefix)
        || text.contains(&tids_suffix)
        || text.contains(&tids_middle)
}

fn scheduler_cpu_queue_contains_pid(text: &str, cpu: usize, pid: u64) -> bool {
    let prefix = format!("cpu-queue\tindex={cpu}\t");
    text.lines()
        .filter(|line| line.starts_with(&prefix))
        .any(|line| scheduler_text_contains_pid(line, pid))
}

fn scheduler_procfs_contains_pid<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<bool, ExitCode> {
    let scheduler = read_procfs_all(runtime, "/proc/system/scheduler")?;
    let text = core::str::from_utf8(&scheduler).map_err(|_| 470)?;
    Ok(scheduler_text_contains_pid(text, pid))
}

fn scheduler_cpu_count(text: &str) -> Option<usize> {
    let summary = text
        .lines()
        .find(|line| line.starts_with("cpu-summary:\t"))?;
    summary.split('\t').find_map(|field| {
        field
            .strip_prefix("count=")
            .and_then(|value| value.parse::<usize>().ok())
    })
}

fn scheduler_cpu_summary_u64(text: &str, key: &str) -> Option<u64> {
    let summary = text
        .lines()
        .find(|line| line.starts_with("cpu-summary:\t"))?;
    summary.split('\t').find_map(|field| {
        field
            .strip_prefix(key)
            .and_then(|value| value.parse::<u64>().ok())
    })
}

fn scheduler_rebalance_migrations(text: &str) -> Option<u64> {
    scheduler_cpu_summary_u64(text, "rebalance-migrations=")
}

fn scheduler_last_rebalance(text: &str) -> Option<u64> {
    scheduler_cpu_summary_u64(text, "last-rebalance=")
}

fn scheduler_rebalance_operations(text: &str) -> Option<u64> {
    scheduler_cpu_summary_u64(text, "rebalance-ops=")
}

fn scheduler_episodes_contains_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    markers: &[&str],
) -> Result<bool, ExitCode> {
    let episodes = read_procfs_all(runtime, "/proc/system/schedulerepisodes")?;
    let text = core::str::from_utf8(&episodes).map_err(|_| 470)?;
    Ok(markers.iter().all(|marker| text.contains(marker)))
}

fn reap_spawned_process_with_retry<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<i32, ExitCode> {
    for _ in 0..256 {
        match runtime.reap_process(pid) {
            Ok(exit_code) => return Ok(exit_code),
            Err(Errno::Again) => {
                let _ = runtime.inspect_process(pid);
            }
            Err(_) => return Err(471),
        }
    }
    Err(471)
}

fn emit_scheduler_smoke_report<E>(
    report: &SchedulerSmokeReport,
    mut emit: impl FnMut(&str) -> Result<(), E>,
) -> Result<(), E> {
    for line in [
        report.observe_line,
        report.spawn_line.as_str(),
        report.balance_line.as_str(),
        report.affinity_refusal_line.as_str(),
        report.affinity_line.as_str(),
        report.renice_line.as_str(),
        report.pause_line.as_str(),
        report.resume_line.as_str(),
        report.queue_line.as_str(),
        report.fairness_line,
        report.cpu_line.as_str(),
        report.episodes_line,
        report.recovery_line.as_str(),
        report.state_line.as_str(),
        report.final_marker,
    ] {
        emit(line)?;
    }
    Ok(())
}

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerSmokeReport {
    pub observe_line: &'static str,
    pub spawn_line: String,
    pub balance_line: String,
    pub affinity_refusal_line: String,
    pub affinity_line: String,
    pub renice_line: String,
    pub pause_line: String,
    pub resume_line: String,
    pub queue_line: String,
    pub fairness_line: &'static str,
    pub cpu_line: String,
    pub episodes_line: &'static str,
    pub recovery_line: String,
    pub state_line: String,
    pub final_marker: &'static str,
}

pub fn run_scheduler_smoke_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<SchedulerSmokeReport, ExitCode> {
    let baseline_markers = [
        "current-tick:",
        "decision-tracing:",
        "running:\t",
        "cpu-summary:\t",
        "cpu-queue\tindex=0\tclass=",
        "rebalance-ops=",
        "rebalance-migrations=",
        "last-rebalance=",
        "cpu\tindex=0\tapic-id=",
        "inferred-topology=true",
        "queued-load=",
        "tokens=",
        "wait-ticks=",
        "lag-debt=",
        "dispatches=",
        "runtime-ticks=",
        "fairness-dispatch-total:",
        "fairness-runtime-total:",
        "fairness-runtime-imbalance:",
        "decision\ttick=",
    ];
    if !scheduler_procfs_contains_all(runtime, &baseline_markers)? {
        return Err(473);
    }
    let baseline_scheduler = read_procfs_all(runtime, "/proc/system/scheduler")?;
    let baseline_scheduler_text = core::str::from_utf8(&baseline_scheduler).map_err(|_| 470)?;
    let cpu_count = scheduler_cpu_count(baseline_scheduler_text).ok_or(473)?;

    let worker_pid = runtime
        .spawn_path_process("scheduler-worker", "/bin/worker")
        .map_err(|_| 475)?;
    let report = (|| {
        let spawned = runtime.inspect_process(worker_pid).map_err(|_| 476)?;
        if spawned.scheduler_class != NativeSchedulerClass::Interactive as u32 {
            return Err(477);
        }
        let balance_pid = if cpu_count >= 2 {
            Some(
                runtime
                    .spawn_path_process("scheduler-balance", "/bin/worker")
                    .map_err(|_| 476)?,
            )
        } else {
            None
        };
        let (balance_line, pre_affinity_migrations, pre_affinity_last_rebalance) = if let Some(
            balance_pid,
        ) =
            balance_pid
        {
            let balance_scheduler = read_procfs_all(runtime, "/proc/system/scheduler")?;
            let balance_scheduler_text =
                core::str::from_utf8(&balance_scheduler).map_err(|_| 470)?;
            let rebalance_migrations =
                scheduler_rebalance_migrations(balance_scheduler_text).ok_or(493)?;
            let last_rebalance = scheduler_last_rebalance(balance_scheduler_text).ok_or(493)?;
            let rebalanced_pid =
                if scheduler_cpu_queue_contains_pid(balance_scheduler_text, 1, balance_pid) {
                    balance_pid
                } else if scheduler_cpu_queue_contains_pid(balance_scheduler_text, 1, worker_pid) {
                    worker_pid
                } else {
                    return Err(493);
                };
            if rebalance_migrations == 0
                || last_rebalance == 0
                || !balance_scheduler_text.contains("agent=RebalanceAgent")
            {
                return Err(493);
            }
            let episodes = read_procfs_all(runtime, "/proc/system/schedulerepisodes")?;
            let episodes_text = core::str::from_utf8(&episodes).map_err(|_| 470)?;
            if !episodes_text.contains("episode\tkind=rebalance")
                || !episodes_text.contains("causal=queued-moved")
            {
                return Err(493);
            }
            (
                    format!(
                        "scheduler.smoke.balance pid={rebalanced_pid} cpu=1 auto=yes migrations={rebalance_migrations} last-rebalance={last_rebalance} outcome=ok"
                    ),
                    rebalance_migrations,
                    last_rebalance,
                )
        } else {
            (
                    String::from(
                        "scheduler.smoke.balance pid=0 cpu=0 auto=skipped migrations=0 last-rebalance=0 outcome=ok",
                    ),
                    0,
                    0,
                )
        };

        match runtime.set_process_affinity(worker_pid, 0) {
            Err(Errno::Inval) => {}
            _ => return Err(478),
        }
        let expected_cpu = if cpu_count >= 2 { 1usize } else { 0usize };
        let affinity_mask = 1u64 << expected_cpu;
        runtime
            .set_process_affinity(worker_pid, affinity_mask)
            .map_err(|_| 478)?;
        if !scheduler_procfs_contains_all(
            runtime,
            &["agent=AffinityAgent", "meaning=affinity cpu-mask=0x"],
        )? {
            return Err(478);
        }

        runtime
            .renice_process(worker_pid, NativeSchedulerClass::Background, 1)
            .map_err(|_| 479)?;
        let reniced = runtime.inspect_process(worker_pid).map_err(|_| 480)?;
        if reniced.scheduler_class != NativeSchedulerClass::Background as u32
            || reniced.scheduler_budget != 1
        {
            return Err(481);
        }

        runtime.pause_process(worker_pid).map_err(|_| 483)?;
        let paused = runtime.inspect_process(worker_pid).map_err(|_| 484)?;
        if paused.state != 3 {
            return Err(485);
        }

        runtime.resume_process(worker_pid).map_err(|_| 487)?;
        let resumed = runtime.inspect_process(worker_pid).map_err(|_| 488)?;
        if resumed.state != 1 && resumed.state != 2 {
            return Err(489);
        }

        let queue_markers = [
            "queue\tclass=background",
            "tokens=",
            "wait-ticks=",
            "lag-debt=",
            "dispatches=",
            "runtime-ticks=",
        ];
        if !scheduler_procfs_contains_all(runtime, &queue_markers)? {
            return Err(491);
        }
        if !scheduler_procfs_contains_pid(runtime, worker_pid)? {
            return Err(492);
        }

        let fairness_markers = [
            "fairness-dispatch-total:",
            "fairness-runtime-total:",
            "fairness-runtime-imbalance:",
        ];
        if !scheduler_procfs_contains_all(runtime, &fairness_markers)? {
            return Err(493);
        }

        let cpu_markers = [
            "cpu-summary:\t",
            "cpu-queue\tindex=0\tclass=",
            "rebalance-ops=",
            "rebalance-migrations=",
            "last-rebalance=",
            "cpu\tindex=0\tapic-id=",
            "inferred-topology=true",
            "queued-load=",
        ];
        if !scheduler_procfs_contains_all(runtime, &cpu_markers)? {
            return Err(493);
        }
        let cpu_scheduler = read_procfs_all(runtime, "/proc/system/scheduler")?;
        let cpu_scheduler_text = core::str::from_utf8(&cpu_scheduler).map_err(|_| 470)?;
        let rebalance_operations = scheduler_rebalance_operations(cpu_scheduler_text).ok_or(493)?;
        let rebalance_migrations = scheduler_rebalance_migrations(cpu_scheduler_text).ok_or(493)?;
        let last_rebalance = scheduler_last_rebalance(cpu_scheduler_text).ok_or(493)?;
        if cpu_count >= 2
            && (rebalance_operations == 0 || rebalance_migrations == 0 || last_rebalance == 0)
        {
            return Err(493);
        }
        let episode_markers = [
            "episodes:\t",
            "episode\tkind=affinity",
            "causal=cpu-mask-updated",
            "episode\tkind=dispatch",
            "episode\tkind=rebalance",
            "causal=queued-moved",
        ];
        if !scheduler_episodes_contains_all(runtime, &episode_markers)? {
            return Err(493);
        }

        runtime.send_signal(worker_pid, 15).map_err(|_| 494)?;
        if let Some(balance_pid) = balance_pid {
            runtime.send_signal(balance_pid, 15).map_err(|_| 494)?;
        }
        let exit_code = reap_spawned_process_with_retry(runtime, worker_pid)?;
        if let Some(balance_pid) = balance_pid {
            let _ = reap_spawned_process_with_retry(runtime, balance_pid);
        }
        if scheduler_procfs_contains_pid(runtime, worker_pid)? {
            return Err(496);
        }

        Ok(SchedulerSmokeReport {
            observe_line: "scheduler.smoke.observe path=/proc/system/scheduler tokens=yes wait-ticks=yes lag=yes fairness=yes decisions=yes running=yes cpu=yes cpu-topology=yes cpu-queue=yes rebalance=yes outcome=ok",
            spawn_line: format!(
                "scheduler.smoke.spawn pid={worker_pid} class=interactive outcome=ok"
            ),
            balance_line,
            affinity_refusal_line: format!(
                "scheduler.smoke.affinity.refusal pid={worker_pid} cpu-mask=0x0 errno=EINVAL outcome=expected"
            ),
            affinity_line: format!(
                "scheduler.smoke.affinity pid={worker_pid} cpu-mask=0x{affinity_mask:x} cpu={expected_cpu} visible=yes outcome=ok"
            ),
            renice_line: format!(
                "scheduler.smoke.renice pid={worker_pid} class=background budget=1 outcome=ok"
            ),
            pause_line: format!(
                "scheduler.smoke.pause pid={worker_pid} state=Blocked outcome=ok"
            ),
            resume_line: format!(
                "scheduler.smoke.resume pid={worker_pid} state={} outcome=ok",
                native_process_state_label(resumed.state)
            ),
            queue_line: format!(
                "scheduler.smoke.queue pid={worker_pid} class=background visible=yes outcome=ok"
            ),
            fairness_line: "scheduler.smoke.fairness dispatch=yes runtime=yes imbalance=yes outcome=ok",
            cpu_line: format!(
                "scheduler.smoke.cpu count={cpu_count} running=yes load=yes cpu-topology=yes cpu-queue=yes rebalance=yes migrations={rebalance_migrations} last-rebalance={last_rebalance} auto-migrations={pre_affinity_migrations} auto-last-rebalance={pre_affinity_last_rebalance} outcome=ok"
            ),
            episodes_line: "scheduler.smoke.episodes affinity=yes dispatch=yes causal=yes outcome=ok",
            recovery_line: format!(
                "scheduler.smoke.recovery pid={worker_pid} exit={exit_code} outcome=ok"
            ),
            state_line: format!("scheduler.smoke.state pid={worker_pid} present=no outcome=ok"),
            final_marker: "scheduler-smoke-ok",
        })
    })();

    if report.is_err() {
        let _ = runtime.send_signal(worker_pid, 15);
        let _ = reap_spawned_process_with_retry(runtime, worker_pid);
        let _ = runtime.send_signal(worker_pid.saturating_add(1), 15);
        let _ = reap_spawned_process_with_retry(runtime, worker_pid.saturating_add(1));
    }
    report
}

pub fn run_scheduler_boot_smoke<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    bind_observe_contract: F,
) -> ExitCode
where
    F: FnOnce(&Runtime<B>) -> Result<(), ExitCode>,
{
    let mut probe = [0u8; 64];
    if runtime
        .read_procfs("/proc/system/scheduler", &mut probe)
        .is_ok()
    {
        return 470;
    }
    if write_line(
        runtime,
        "scheduler.smoke.refusal path=/proc/system/scheduler contract=observe outcome=expected",
    )
    .is_err()
    {
        return 472;
    }

    if let Err(code) = bind_observe_contract(runtime) {
        return code;
    }

    let report = match run_scheduler_smoke_report(runtime) {
        Ok(report) => report,
        Err(code) => return code,
    };
    if emit_scheduler_smoke_report(&report, |line| write_line(runtime, line)).is_err() {
        return 498;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::{
        emit_scheduler_smoke_report, scheduler_procfs_contains_all_markers,
        scheduler_text_contains_pid, SchedulerSmokeReport,
    };
    use alloc::string::{String, ToString};
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn scheduler_procfs_pid_matching_handles_pid_and_tid_forms() {
        assert!(scheduler_text_contains_pid("pid=41\n", 41));
        assert!(scheduler_text_contains_pid("tids=[41]\n", 41));
        assert!(scheduler_text_contains_pid("tids=[7,41,9]\n", 41));
        assert!(!scheduler_text_contains_pid("pid=42\ntids=[42]\n", 41));
    }

    #[test]
    fn scheduler_marker_scan_requires_every_expected_marker() {
        let text = "current-tick:\nrebalance-ops=\nqueued-load=\n";
        assert!(scheduler_procfs_contains_all_markers(
            text,
            &["current-tick:", "queued-load="]
        ));
        assert!(!scheduler_procfs_contains_all_markers(
            text,
            &["current-tick:", "fairness-runtime-total:"]
        ));
    }

    #[test]
    fn scheduler_smoke_report_emits_expected_lines_in_order() {
        let report = SchedulerSmokeReport {
            observe_line: "scheduler.smoke.observe path=/proc/system/scheduler",
            spawn_line: String::from("scheduler.smoke.spawn pid=7"),
            balance_line: String::from(
                "scheduler.smoke.balance pid=8 cpu=1 auto=yes migrations=1 last-rebalance=1 outcome=ok",
            ),
            affinity_refusal_line: String::from(
                "scheduler.smoke.affinity.refusal pid=7 cpu-mask=0x0 errno=EINVAL outcome=expected",
            ),
            affinity_line: String::from(
                "scheduler.smoke.affinity pid=7 cpu-mask=0x2 cpu=1 visible=yes outcome=ok",
            ),
            renice_line: String::from("scheduler.smoke.renice pid=7"),
            pause_line: String::from("scheduler.smoke.pause pid=7"),
            resume_line: String::from("scheduler.smoke.resume pid=7"),
            queue_line: String::from("scheduler.smoke.queue pid=7"),
            fairness_line: "scheduler.smoke.fairness outcome=ok",
            cpu_line: String::from(
                "scheduler.smoke.cpu count=2 running=yes load=yes cpu-topology=yes cpu-queue=yes rebalance=yes migrations=1 last-rebalance=1 auto-migrations=1 auto-last-rebalance=1 outcome=ok",
            ),
            episodes_line: "scheduler.smoke.episodes affinity=yes dispatch=yes causal=yes outcome=ok",
            recovery_line: String::from("scheduler.smoke.recovery pid=7"),
            state_line: String::from("scheduler.smoke.state pid=7"),
            final_marker: "scheduler-smoke-ok",
        };
        let mut lines = Vec::new();

        emit_scheduler_smoke_report(&report, |line| {
            lines.push(line.to_string());
            Ok::<(), ()>(())
        })
        .unwrap();

        assert_eq!(
            lines,
            vec![
                String::from("scheduler.smoke.observe path=/proc/system/scheduler"),
                String::from("scheduler.smoke.spawn pid=7"),
                String::from(
                    "scheduler.smoke.balance pid=8 cpu=1 auto=yes migrations=1 last-rebalance=1 outcome=ok",
                ),
                String::from(
                    "scheduler.smoke.affinity.refusal pid=7 cpu-mask=0x0 errno=EINVAL outcome=expected",
                ),
                String::from(
                    "scheduler.smoke.affinity pid=7 cpu-mask=0x2 cpu=1 visible=yes outcome=ok",
                ),
                String::from("scheduler.smoke.renice pid=7"),
                String::from("scheduler.smoke.pause pid=7"),
                String::from("scheduler.smoke.resume pid=7"),
                String::from("scheduler.smoke.queue pid=7"),
                String::from("scheduler.smoke.fairness outcome=ok"),
                String::from(
                    "scheduler.smoke.cpu count=2 running=yes load=yes cpu-topology=yes cpu-queue=yes rebalance=yes migrations=1 last-rebalance=1 auto-migrations=1 auto-last-rebalance=1 outcome=ok",
                ),
                String::from("scheduler.smoke.episodes affinity=yes dispatch=yes causal=yes outcome=ok"),
                String::from("scheduler.smoke.recovery pid=7"),
                String::from("scheduler.smoke.state pid=7"),
                String::from("scheduler-smoke-ok"),
            ]
        );
    }
}
