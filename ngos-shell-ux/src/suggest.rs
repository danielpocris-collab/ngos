//! Shell suggest, apropos, whereami, unknown-command-feedback.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::{ShellAlias, ShellJob, ShellVariable};
use ngos_user_abi::bootstrap::SessionContext;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::catalog::{shell_guess_ux_topic, shell_is_meta_history_command, shell_ux_catalog};

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub fn shell_render_whereami<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    current_cwd: &str,
    aliases: &[ShellAlias],
    variables: &[ShellVariable],
    jobs: &[ShellJob],
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "whereami protocol={} cwd={} root={} image={} aliases={} vars={} jobs={}",
            context.protocol,
            current_cwd,
            context.root_mount_path,
            context.image_path,
            aliases.len(),
            variables.len(),
            jobs.len()
        ),
    )
}

pub fn shell_render_suggest_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    topic: Option<&str>,
    history: &[String],
) -> Result<(), ExitCode> {
    let topic = topic
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .or_else(|| {
            history
                .iter()
                .rev()
                .find(|entry| !shell_is_meta_history_command(entry))
                .map(|entry| {
                    let command = entry.split_whitespace().next().unwrap_or("session");
                    shell_guess_ux_topic(command).to_string()
                })
        })
        .unwrap_or_else(|| "session".to_string());
    let lines: &[&str] = match topic.as_str() {
        "session" => &[
            "suggest-next session whereami",
            "suggest-next session vars",
            "suggest-next session jobs",
        ],
        "pipeline" => &[
            "suggest-next pipeline session |> record-get cwd CURRENT_CWD |> value-show",
            "suggest-next pipeline mounts |> list-count MOUNT_COUNT |> value-show",
            "suggest-next pipeline process-info 1 |> record-get pid PID |> value-show",
        ],
        "process" => &[
            "suggest-next process process-info 1",
            "suggest-next process identity-of 1 |> value-show",
            "suggest-next process fd |> list-count FD_COUNT |> value-show",
        ],
        "network" => &[
            "suggest-next network netif /dev/net0",
            "suggest-next network netsock /run/net0.sock",
            "suggest-next network network-smoke",
        ],
        "storage" => &[
            "suggest-next storage storage-volume /dev/storage0",
            "suggest-next storage storage-lineage /dev/storage0",
            "suggest-next storage storage-history /dev/storage0",
            "suggest-next storage storage-history-range /dev/storage0 0 3",
            "suggest-next storage storage-history-tail /dev/storage0 3",
            "suggest-next storage storage-prepare /dev/storage0 shell-smoke persistent-shell-proof",
            "suggest-next storage storage-recover /dev/storage0",
            "suggest-next storage storage-repair /dev/storage0",
            "suggest-next storage storage-mount /dev/storage0 /shell-proof-mount",
            "suggest-next storage storage-unmount /shell-proof-mount",
            "suggest-next storage storage-history-entry /dev/storage0 0",
            "suggest-next storage storage-history-range-of /dev/storage0 0 3",
            "suggest-next storage storage-history-tail-of /dev/storage0 3",
        ],
        "review" => &[
            "suggest-next review build-diagnostics /shell-proof/build.log",
            "suggest-next review test-failures /shell-proof/test.log",
            "suggest-next review impact-summary left.rs right.rs",
        ],
        "vfs" => &[
            "suggest-next vfs mounts",
            "suggest-next vfs mount-info /",
            "suggest-next vfs vfs-smoke",
        ],
        _ => &[
            "suggest-next generic whereami",
            "suggest-next generic help-topic pipeline",
            "suggest-next generic suggest pro",
        ],
    };
    for line in lines {
        write_line(runtime, line)?;
    }
    write_line(
        runtime,
        &format!("suggest-next topic={} count={}", topic, lines.len()),
    )
}

pub fn shell_render_suggest<B: SyscallBackend>(
    runtime: &Runtime<B>,
    aliases: &[ShellAlias],
    history: &[String],
    prefix: &str,
) -> Result<(), ExitCode> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return write_line(runtime, "usage: suggest <prefix>");
    }
    let prefix_lower = prefix.to_lowercase();
    let mut matches = shell_ux_catalog(aliases, history)
        .into_iter()
        .filter(|entry| entry.to_lowercase().starts_with(&prefix_lower))
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    matches.truncate(8);
    if matches.is_empty() {
        return write_line(runtime, &format!("suggest prefix={} count=0", prefix_lower));
    }
    for entry in &matches {
        write_line(runtime, &format!("suggestion {}", entry))?;
    }
    write_line(
        runtime,
        &format!(
            "suggest prefix={} count={} first={}",
            prefix_lower,
            matches.len(),
            matches[0]
        ),
    )
}

pub fn shell_render_apropos<B: SyscallBackend>(
    runtime: &Runtime<B>,
    aliases: &[ShellAlias],
    history: &[String],
    needle: &str,
) -> Result<(), ExitCode> {
    let needle = needle.trim();
    if needle.is_empty() {
        return write_line(runtime, "usage: apropos <needle>");
    }
    let needle = needle.to_lowercase();
    let mut matches = shell_ux_catalog(aliases, history)
        .into_iter()
        .filter(|entry| entry.to_lowercase().contains(&needle))
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    matches.truncate(12);
    for entry in &matches {
        write_line(runtime, &format!("apropos-match {}", entry))?;
    }
    write_line(
        runtime,
        &format!("apropos needle={} count={}", needle, matches.len()),
    )
}

pub fn shell_render_unknown_command_feedback<B: SyscallBackend>(
    runtime: &Runtime<B>,
    aliases: &[ShellAlias],
    history: &[String],
    line: &str,
) -> Result<(), ExitCode> {
    write_line(runtime, "unknown-command")?;
    let command = line.split_whitespace().next().unwrap_or_default().trim();
    if command.is_empty() {
        return Ok(());
    }
    let mut matches = shell_ux_catalog(aliases, history)
        .into_iter()
        .filter(|entry| entry.starts_with(command.chars().next().unwrap_or_default()))
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    matches.truncate(3);
    if matches.is_empty() {
        let topic = shell_guess_ux_topic(command);
        write_line(runtime, &format!("suggest prefix={} count=0", command))?;
        write_line(runtime, &format!("unknown-topic {}", topic))?;
        write_line(runtime, &format!("unknown-next suggest-next {}", topic))?;
        return write_line(runtime, &format!("unknown-next help-topic {}", topic));
    }
    for entry in &matches {
        write_line(runtime, &format!("suggestion {}", entry))?;
    }
    write_line(
        runtime,
        &format!("suggest prefix={} count={}", command, matches.len()),
    )
}
