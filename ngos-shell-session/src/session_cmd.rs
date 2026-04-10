//! Shell session command handling: alias, set, cd, source-file, session, env, history, etc.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::{
    ShellAlias, ShellJob, ShellRecordField, ShellSemanticValue, ShellVariable, parse_u64_arg,
    parse_usize_arg, resolve_shell_path, shell_lookup_variable_entry, shell_set_record_variable,
    shell_set_variable, shell_variable_type_name,
};
use ngos_shell_ux::{
    shell_is_meta_history_command, shell_render_apropos, shell_render_command_card,
    shell_render_command_explain, shell_render_examples, shell_render_help_topic,
    shell_render_help_ux, shell_render_history, shell_render_history_find,
    shell_render_history_tail, shell_render_recent_work, shell_render_suggest,
    shell_render_suggest_next, shell_render_whereami,
};
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::bootstrap::SessionContext;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::pipeline::handle_semantic_pipeline;

pub(crate) fn write_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    text: &str,
) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub fn try_handle_session_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    current_cwd: &mut String,
    aliases: &mut Vec<ShellAlias>,
    variables: &mut Vec<ShellVariable>,
    jobs: &[ShellJob],
    history: &[String],
    pending_lines: &mut Vec<String>,
    line_index: usize,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    if line.contains("|>") {
        return Some(handle_semantic_pipeline(
            runtime,
            context,
            current_cwd,
            variables,
            jobs,
            line,
        ));
    }
    if line == "aliases" {
        return Some(crate::render::shell_render_aliases(runtime, aliases).map_err(|_| 196));
    }
    if line == "vars" {
        return Some(crate::render::shell_render_variables(runtime, variables).map_err(|_| 196));
    }
    if line == "history" {
        return Some(shell_render_history(runtime, history).map_err(|_| 196));
    }
    if line == "help-ux" {
        return Some(shell_render_help_ux(runtime).map_err(|_| 196));
    }
    if line == "whereami" {
        return Some(
            shell_render_whereami(runtime, context, current_cwd, aliases, variables, jobs)
                .map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("help-topic ") {
        return Some(shell_render_help_topic(runtime, rest.trim()).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("command-card ") {
        return Some(shell_render_command_card(runtime, rest.trim()).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("examples ") {
        return Some(shell_render_examples(runtime, rest.trim()).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("history-tail ") {
        return Some(shell_render_history_tail(runtime, history, rest.trim()).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("recent-work ") {
        return Some(
            shell_render_recent_work(runtime, history, Some(rest.trim())).map_err(|_| 196),
        );
    }
    if line == "recent-work" {
        return Some(shell_render_recent_work(runtime, history, None).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("history-find ") {
        return Some(shell_render_history_find(runtime, history, rest.trim()).map_err(|_| 196));
    }
    if line == "repeat-last" {
        let replay = history
            .iter()
            .rev()
            .find(|entry| {
                let trimmed = entry.trim();
                !trimmed.is_empty() && !shell_is_meta_history_command(trimmed)
            })
            .cloned();
        let Some(replay) = replay else {
            let _ = write_line(runtime, "repeat-last source=none");
            return Some(Err(205));
        };
        pending_lines.splice(line_index..line_index, [replay.clone()]);
        return Some(
            write_line(runtime, &format!("repeat-last queued={}", replay)).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("rerun-find ") {
        let needle = rest.trim();
        if needle.is_empty() {
            let _ = write_line(runtime, "usage: rerun-find <needle>");
            return Some(Err(2));
        }
        let replay = history
            .iter()
            .rev()
            .find(|entry| {
                let trimmed = entry.trim();
                !trimmed.is_empty()
                    && !shell_is_meta_history_command(trimmed)
                    && trimmed.contains(needle)
            })
            .cloned();
        let Some(replay) = replay else {
            let _ = write_line(runtime, &format!("rerun-find needle={} count=0", needle));
            return Some(Err(205));
        };
        pending_lines.splice(line_index..line_index, [replay.clone()]);
        return Some(
            write_line(
                runtime,
                &format!("rerun-find needle={} queued={}", needle, replay),
            )
            .map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("suggest ") {
        return Some(shell_render_suggest(runtime, aliases, history, rest.trim()).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("suggest-next ") {
        return Some(
            shell_render_suggest_next(runtime, Some(rest.trim()), history).map_err(|_| 196),
        );
    }
    if line == "suggest-next" {
        return Some(shell_render_suggest_next(runtime, None, history).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("apropos ") {
        return Some(shell_render_apropos(runtime, aliases, history, rest.trim()).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("explain-command ") {
        return Some(shell_render_command_explain(runtime, rest.trim()).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("alias ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let name = match parts.next() {
            Some(name) if !name.is_empty() => name,
            _ => {
                let _ = write_line(runtime, "usage: alias <name> <command>");
                return Some(Err(2));
            }
        };
        let value = match parts.next() {
            Some(value) if !value.trim().is_empty() => value.trim(),
            _ => {
                let _ = write_line(runtime, "usage: alias <name> <command>");
                return Some(Err(2));
            }
        };
        if let Some(alias) = aliases.iter_mut().find(|alias| alias.name == name) {
            alias.value = value.to_string();
        } else {
            aliases.push(ShellAlias {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
        return Some(write_line(runtime, &format!("alias-set {name}={value}")).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("unalias ") {
        let name = rest.trim();
        if name.is_empty() {
            let _ = write_line(runtime, "usage: unalias <name>");
            return Some(Err(2));
        }
        aliases.retain(|alias| alias.name != name);
        return Some(write_line(runtime, &format!("alias-unset {name}")).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("set ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let name = match parts.next() {
            Some(name) if !name.is_empty() => name,
            _ => {
                let _ = write_line(runtime, "usage: set <name> <value>");
                return Some(Err(2));
            }
        };
        let value = match parts.next() {
            Some(value) => value.trim_start(),
            None => {
                let _ = write_line(runtime, "usage: set <name> <value>");
                return Some(Err(2));
            }
        };
        shell_set_variable(variables, name, value.to_string());
        return Some(write_line(runtime, &format!("var-set {name}={value}")).map_err(|_| 196));
    }
    if let Some(rest) = line.strip_prefix("record-set ") {
        let mut parts = rest.split_whitespace();
        let name = match parts.next() {
            Some(name) if !name.is_empty() => name,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: record-set <name> <field=value> [field=value...]",
                );
                return Some(Err(2));
            }
        };
        let mut fields = Vec::<ShellRecordField>::new();
        for entry in parts {
            let Some((key, value)) = entry.split_once('=') else {
                let _ = write_line(
                    runtime,
                    "usage: record-set <name> <field=value> [field=value...]",
                );
                return Some(Err(2));
            };
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                let _ = write_line(
                    runtime,
                    "usage: record-set <name> <field=value> [field=value...]",
                );
                return Some(Err(2));
            }
            fields.push(ShellRecordField {
                key: key.to_string(),
                value: value.to_string(),
            });
        }
        if fields.is_empty() {
            let _ = write_line(
                runtime,
                "usage: record-set <name> <field=value> [field=value...]",
            );
            return Some(Err(2));
        }
        let field_count = fields.len();
        shell_set_record_variable(variables, name, fields);
        return Some(
            write_line(
                runtime,
                &format!("record-set name={name} fields={field_count}"),
            )
            .map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("record-get ") {
        let mut parts = rest.split_whitespace();
        let name = match parts.next() {
            Some(name) if !name.is_empty() => name,
            _ => {
                let _ = write_line(runtime, "usage: record-get <name> <field> [target]");
                return Some(Err(2));
            }
        };
        let field_name = match parts.next() {
            Some(field_name) if !field_name.is_empty() => field_name,
            _ => {
                let _ = write_line(runtime, "usage: record-get <name> <field> [target]");
                return Some(Err(2));
            }
        };
        let target = parts
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let Some(variable) = shell_lookup_variable_entry(variables, name) else {
            return Some(Err(205));
        };
        let Some(ShellSemanticValue::Record(fields)) = &variable.semantic else {
            let _ = write_line(runtime, &format!("record-get-invalid name={name}"));
            return Some(Err(205));
        };
        let Some(field) = fields.iter().find(|field| field.key == field_name) else {
            let _ = write_line(
                runtime,
                &format!("record-field-missing name={name} field={field_name}"),
            );
            return Some(Err(205));
        };
        let field_value = field.value.clone();
        if let Some(target_name) = target {
            shell_set_variable(variables, target_name, field_value.clone());
            return Some(
                write_line(
                    runtime,
                    &format!(
                        "record-field name={} field={} target={} value={}",
                        name, field_name, target_name, field_value
                    ),
                )
                .map_err(|_| 196),
            );
        }
        return Some(
            write_line(
                runtime,
                &format!(
                    "record-field name={} field={} value={}",
                    name, field_name, field_value
                ),
            )
            .map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("value-type ") {
        let name = rest.trim();
        if name.is_empty() {
            let _ = write_line(runtime, "usage: value-type <name>");
            return Some(Err(2));
        }
        let Some(variable) = shell_lookup_variable_entry(variables, name) else {
            return Some(Err(205));
        };
        return Some(
            write_line(
                runtime,
                &format!(
                    "value-type name={} type={}",
                    name,
                    shell_variable_type_name(variable)
                ),
            )
            .map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("value-show ") {
        let name = rest.trim();
        if name.is_empty() {
            let _ = write_line(runtime, "usage: value-show <name>");
            return Some(Err(2));
        }
        let Some(variable) = shell_lookup_variable_entry(variables, name) else {
            return Some(Err(205));
        };
        let type_name = shell_variable_type_name(variable);
        if write_line(
            runtime,
            &format!(
                "value-show name={} type={} value={}",
                name, type_name, variable.value
            ),
        )
        .is_err()
        {
            return Some(Err(196));
        }
        if let Some(ShellSemanticValue::Record(fields)) = &variable.semantic {
            for field in fields {
                if write_line(
                    runtime,
                    &format!(
                        "value-field name={} field={} value={}",
                        name, field.key, field.value
                    ),
                )
                .is_err()
                {
                    return Some(Err(196));
                }
            }
        }
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("unset ") {
        let name = rest.trim();
        if name.is_empty() {
            let _ = write_line(runtime, "usage: unset <name>");
            return Some(Err(2));
        }
        variables.retain(|variable| variable.name != name);
        return Some(write_line(runtime, &format!("var-unset {name}")).map_err(|_| 196));
    }
    if line == "session" {
        let mut out = String::new();
        out.push_str("protocol=");
        out.push_str(&context.protocol);
        out.push_str(" process=");
        out.push_str(&context.process_name);
        out.push_str(" image=");
        out.push_str(&context.image_path);
        out.push_str(" cwd=");
        out.push_str(current_cwd);
        return Some(write_line(runtime, &out).map_err(|_| 196));
    }
    if line == "pwd" {
        return Some(write_line(runtime, current_cwd).map_err(|_| 196));
    }
    if line == "env" {
        return Some(
            crate::render::shell_render_env(runtime, context, current_cwd).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("cd ") {
        let path = rest.trim();
        if path.is_empty() {
            let _ = write_line(runtime, "usage: cd <path>");
            return Some(Err(2));
        }
        let resolved = resolve_shell_path(current_cwd, path);
        if runtime.chdir_path(&resolved).is_err() {
            return Some(Err(205));
        }
        *current_cwd = resolved.clone();
        return Some(write_line(runtime, &format!("cwd-updated path={resolved}")).map_err(|_| 205));
    }
    if let Some(rest) = line.strip_prefix("source-file ") {
        let resolved = resolve_shell_path(current_cwd, rest.trim());
        let sourced = match shell_read_file_text(runtime, &resolved) {
            Ok(text) => text,
            Err(_) => return Some(Err(205)),
        };
        let sourced_lines = sourced
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        pending_lines.splice(line_index..line_index, sourced_lines);
        return Some(
            write_line(runtime, &format!("script-loaded path={resolved}")).map_err(|_| 205),
        );
    }
    let _ = parse_u64_arg(None::<&str>);
    let _ = parse_usize_arg(None::<&str>);
    None
}
