//! Shell history render functions.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::catalog::shell_is_meta_history_command;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub fn shell_render_history<B: SyscallBackend>(
    runtime: &Runtime<B>,
    history: &[String],
) -> Result<(), ExitCode> {
    if history.is_empty() {
        return write_line(runtime, "history=0");
    }
    for (index, entry) in history.iter().enumerate() {
        write_line(runtime, &format!("history {} {}", index + 1, entry))?;
    }
    Ok(())
}

pub fn shell_render_history_find<B: SyscallBackend>(
    runtime: &Runtime<B>,
    history: &[String],
    needle: &str,
) -> Result<(), ExitCode> {
    if needle.is_empty() {
        return write_line(runtime, "usage: history-find <needle>");
    }
    let mut count = 0usize;
    for (index, entry) in history.iter().enumerate() {
        if entry.contains(needle) {
            count += 1;
            write_line(runtime, &format!("history-match {} {}", index + 1, entry))?;
        }
    }
    write_line(
        runtime,
        &format!("history-find needle={} count={count}", needle),
    )
}

pub fn shell_render_history_tail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    history: &[String],
    count: &str,
) -> Result<(), ExitCode> {
    let count = count.trim();
    if count.is_empty() {
        return write_line(runtime, "usage: history-tail <count>");
    }
    let Ok(count) = count.parse::<usize>() else {
        return write_line(runtime, "usage: history-tail <count>");
    };
    let start = history.len().saturating_sub(count);
    let mut emitted = 0usize;
    for (index, entry) in history.iter().enumerate().skip(start) {
        emitted += 1;
        write_line(runtime, &format!("history-tail {} {}", index + 1, entry))?;
    }
    write_line(
        runtime,
        &format!("history-tail count={emitted} requested={count}"),
    )
}

pub fn shell_render_recent_work<B: SyscallBackend>(
    runtime: &Runtime<B>,
    history: &[String],
    count: Option<&str>,
) -> Result<(), ExitCode> {
    let requested = match count.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => value
            .parse::<usize>()
            .ok()
            .filter(|value| *value != 0)
            .unwrap_or(5),
        None => 5,
    };
    let mut entries = history
        .iter()
        .enumerate()
        .filter(|(_, entry)| !shell_is_meta_history_command(entry))
        .collect::<Vec<_>>();
    let start = entries.len().saturating_sub(requested);
    let slice = entries.split_off(start);
    let mut emitted = 0usize;
    for (index, entry) in slice {
        emitted += 1;
        write_line(runtime, &format!("recent-work {} {}", index + 1, entry))?;
    }
    write_line(
        runtime,
        &format!("recent-work count={emitted} requested={requested}"),
    )
}
