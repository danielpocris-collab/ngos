//! Shell variable expansion and pipeline guard parsing.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::types::{ShellAlias, ShellCommandGuard, ShellVariable};
use crate::variable::{shell_lookup_variable, shell_set_variable};

/// Expand `$VARIABLE` references in `text` using the given variable list.
pub fn shell_expand_variables(text: &str, variables: &[ShellVariable]) -> String {
    let bytes = text.as_bytes();
    let mut expanded = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'$' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len()
                && ((bytes[end] as char).is_ascii_alphanumeric() || bytes[end] == b'_')
            {
                end += 1;
            }
            if end > start {
                let name = &text[start..end];
                if let Some(value) = shell_lookup_variable(variables, name) {
                    expanded.push_str(value);
                }
                index = end;
                continue;
            }
        }
        expanded.push(bytes[index] as char);
        index += 1;
    }
    expanded
}

/// Expand alias references at the start of a command line.
pub fn shell_expand_aliases(line: &str, aliases: &[ShellAlias]) -> String {
    let mut parts = line.splitn(2, char::is_whitespace);
    let command = match parts.next() {
        Some(command) => command,
        None => return String::new(),
    };
    let rest = parts.next().unwrap_or("").trim_start();
    if let Some(alias) = aliases.iter().rev().find(|alias| alias.name == command) {
        if rest.is_empty() {
            alias.value.clone()
        } else {
            format!("{} {}", alias.value, rest)
        }
    } else {
        line.to_string()
    }
}

/// Parse a command line into guarded segments separated by `&&` and `||`.
pub fn shell_parse_guarded_commands(line: &str) -> Vec<(ShellCommandGuard, String)> {
    let bytes = line.as_bytes();
    let mut commands = Vec::new();
    let mut start = 0usize;
    let mut guard = ShellCommandGuard::Always;
    let mut index = 0usize;
    while index < bytes.len() {
        if index + 1 < bytes.len() && bytes[index] == b'&' && bytes[index + 1] == b'&' {
            let command = line[start..index].trim();
            if !command.is_empty() {
                commands.push((guard, command.to_string()));
            }
            guard = ShellCommandGuard::OnSuccess;
            index += 2;
            start = index;
            continue;
        }
        if index + 1 < bytes.len() && bytes[index] == b'|' && bytes[index + 1] == b'|' {
            let command = line[start..index].trim();
            if !command.is_empty() {
                commands.push((guard, command.to_string()));
            }
            guard = ShellCommandGuard::OnFailure;
            index += 2;
            start = index;
            continue;
        }
        if bytes[index] == b';' {
            let command = line[start..index].trim();
            if !command.is_empty() {
                commands.push((guard, command.to_string()));
            }
            guard = ShellCommandGuard::Always;
            index += 1;
            start = index;
            continue;
        }
        index += 1;
    }
    let command = line[start..].trim();
    if !command.is_empty() {
        commands.push((guard, command.to_string()));
    }
    commands
}

/// Sync runtime-derived variables (STATUS, CWD, LAST_PID) into the variable list.
pub fn shell_sync_runtime_variables(
    variables: &mut Vec<ShellVariable>,
    last_status: i32,
    cwd: &str,
    last_pid: Option<u64>,
) {
    shell_set_variable(variables, "STATUS", last_status.to_string());
    shell_set_variable(variables, "CWD", cwd.to_string());
    shell_set_variable(
        variables,
        "LAST_PID",
        last_pid.map(|pid| pid.to_string()).unwrap_or_default(),
    );
}
