//! Canonical subsystem role:
//! - subsystem: native interactive edit orchestration
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing edit session control over canonical file
//!   contents
//!
//! Canonical contract families handled here:
//! - edit session contracts
//! - interactive text mutation contracts
//! - edit state management contracts
//!
//! This module may manage interactive edit state, but it must not redefine the
//! architectural ownership of the files it edits.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::{parse_usize_arg, resolve_shell_path};
use ngos_shell_vfs::{shell_read_file_text, shell_write_file};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub struct EditSessionState {
    path: Option<String>,
    lines: Vec<String>,
    dirty: bool,
}

impl EditSessionState {
    pub fn new() -> Self {
        Self {
            path: None,
            lines: Vec::new(),
            dirty: false,
        }
    }

    fn clear(&mut self) {
        self.path = None;
        self.lines.clear();
        self.dirty = false;
    }

    fn is_open(&self) -> bool {
        self.path.is_some()
    }
}

enum EditAgentCommand<'a> {
    Open { path: &'a str },
    Status,
    Show { start: usize, count: usize },
    Set { line: usize, text: &'a str },
    Insert { line: usize, text: &'a str },
    Append { text: &'a str },
    Delete { line: usize },
    Write,
    Abort,
}

impl<'a> EditAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("edit-open ") {
            let path = rest.trim();
            return Some((!path.is_empty()).then_some(Self::Open { path }).ok_or(2));
        }
        if line == "edit-status" {
            return Some(Ok(Self::Status));
        }
        if let Some(rest) = line.strip_prefix("edit-show") {
            return Some(parse_show_command(rest));
        }
        if let Some(rest) = line.strip_prefix("edit-set ") {
            return Some(
                parse_line_text_command(rest).map(|(line, text)| Self::Set { line, text }),
            );
        }
        if let Some(rest) = line.strip_prefix("edit-insert ") {
            return Some(
                parse_line_text_command(rest).map(|(line, text)| Self::Insert { line, text }),
            );
        }
        if let Some(rest) = line.strip_prefix("edit-append ") {
            let text = rest.trim();
            return Some((!text.is_empty()).then_some(Self::Append { text }).ok_or(2));
        }
        if let Some(rest) = line.strip_prefix("edit-delete ") {
            return Some(
                parse_usize_arg(Some(rest.trim()))
                    .map(|line| Self::Delete { line })
                    .ok_or(2),
            );
        }
        if line == "edit-write" {
            return Some(Ok(Self::Write));
        }
        if line == "edit-abort" {
            return Some(Ok(Self::Abort));
        }
        None
    }
}

fn parse_show_command(rest: &str) -> Result<EditAgentCommand<'_>, ExitCode> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return Ok(EditAgentCommand::Show {
            start: 1,
            count: 16,
        });
    }
    let mut parts = trimmed.split_whitespace();
    let start = parse_usize_arg(parts.next()).ok_or(2)?;
    let count = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => 16,
    };
    Ok(EditAgentCommand::Show { start, count })
}

fn parse_line_text_command(rest: &str) -> Result<(usize, &str), ExitCode> {
    let trimmed = rest.trim();
    let split = trimmed.find(char::is_whitespace).ok_or(2)?;
    let (line_raw, text_raw) = trimmed.split_at(split);
    let line = line_raw.parse::<usize>().map_err(|_| 2)?;
    let text = text_raw.trim_start();
    if text.is_empty() {
        return Err(2);
    }
    Ok((line, text))
}

pub fn try_handle_edit_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    state: &mut EditSessionState,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match EditAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("edit-open ") {
                "usage: edit-open <path>"
            } else if line.starts_with("edit-show") {
                "usage: edit-show [start] [count]"
            } else if line.starts_with("edit-set ") {
                "usage: edit-set <line> <text>"
            } else if line.starts_with("edit-insert ") {
                "usage: edit-insert <line> <text>"
            } else if line.starts_with("edit-append ") {
                "usage: edit-append <text>"
            } else if line.starts_with("edit-delete ") {
                "usage: edit-delete <line>"
            } else {
                "usage: edit-open|edit-status|edit-show|edit-set|edit-insert|edit-append|edit-delete|edit-write|edit-abort"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(execute_edit_command(runtime, cwd, state, command))
}

fn execute_edit_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    state: &mut EditSessionState,
    command: EditAgentCommand<'_>,
) -> Result<(), ExitCode> {
    match command {
        EditAgentCommand::Open { path } => {
            let resolved = resolve_shell_path(cwd, path);
            let text = shell_read_file_text(runtime, &resolved)?;
            state.path = Some(resolved.clone());
            state.lines = if text.is_empty() {
                Vec::new()
            } else {
                text.lines().map(ToString::to_string).collect()
            };
            state.dirty = false;
            write_line(
                runtime,
                &format!("edit-opened path={resolved} lines={}", state.lines.len()),
            )
        }
        EditAgentCommand::Status => {
            let path = state.path.as_deref().unwrap_or("<none>");
            write_line(
                runtime,
                &format!(
                    "edit-status path={} open={} lines={} dirty={}",
                    path,
                    if state.is_open() { "yes" } else { "no" },
                    state.lines.len(),
                    if state.dirty { "yes" } else { "no" }
                ),
            )
        }
        EditAgentCommand::Show { start, count } => {
            if !state.is_open() {
                return Err(249);
            }
            let start_index = start.saturating_sub(1);
            let end_index = state.lines.len().min(start_index.saturating_add(count));
            for (index, line) in state.lines[start_index.min(state.lines.len())..end_index]
                .iter()
                .enumerate()
            {
                write_line(
                    runtime,
                    &format!("{:>4}: {}", start_index + index + 1, line),
                )?;
            }
            write_line(
                runtime,
                &format!("edit-show lines={}..{}", start_index + 1, end_index),
            )
        }
        EditAgentCommand::Set { line, text } => {
            let target = line.checked_sub(1).ok_or(249)?;
            if state.path.is_none() || target >= state.lines.len() {
                return Err(249);
            }
            state.lines[target] = text.to_string();
            state.dirty = true;
            write_line(runtime, &format!("edit-set line={line}"))
        }
        EditAgentCommand::Insert { line, text } => {
            let target = line.checked_sub(1).ok_or(249)?;
            if state.path.is_none() || target > state.lines.len() {
                return Err(249);
            }
            state.lines.insert(target, text.to_string());
            state.dirty = true;
            write_line(runtime, &format!("edit-insert line={line}"))
        }
        EditAgentCommand::Append { text } => {
            if state.path.is_none() {
                return Err(249);
            }
            state.lines.push(text.to_string());
            state.dirty = true;
            write_line(runtime, &format!("edit-append line={}", state.lines.len()))
        }
        EditAgentCommand::Delete { line } => {
            let target = line.checked_sub(1).ok_or(249)?;
            if state.path.is_none() || target >= state.lines.len() {
                return Err(249);
            }
            state.lines.remove(target);
            state.dirty = true;
            write_line(runtime, &format!("edit-delete line={line}"))
        }
        EditAgentCommand::Write => {
            let Some(path) = state.path.as_deref() else {
                return Err(249);
            };
            let body = state.lines.join("\n");
            shell_write_file(runtime, path, &body)?;
            state.dirty = false;
            write_line(
                runtime,
                &format!("edit-written path={} lines={}", path, state.lines.len()),
            )
        }
        EditAgentCommand::Abort => {
            let path = state.path.clone().unwrap_or_else(|| "<none>".to_string());
            state.clear();
            write_line(runtime, &format!("edit-aborted path={path}"))
        }
    }
}
