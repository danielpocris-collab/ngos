use super::*;

pub(super) fn try_handle_session_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    current_cwd: &mut String,
    aliases: &mut Vec<ShellAlias>,
    variables: &mut Vec<ShellVariable>,
    history: &[String],
    pending_lines: &mut Vec<String>,
    line_index: usize,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    if line == "aliases" {
        return Some(shell_render_aliases(runtime, aliases).map_err(|_| 196));
    }
    if line == "vars" {
        return Some(shell_render_variables(runtime, variables).map_err(|_| 196));
    }
    if line == "history" {
        return Some(shell_render_history(runtime, history).map_err(|_| 196));
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
        if let Some(variable) = variables.iter_mut().find(|variable| variable.name == name) {
            variable.value = value.to_string();
        } else {
            variables.push(ShellVariable {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
        return Some(write_line(runtime, &format!("var-set {name}={value}")).map_err(|_| 196));
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
        return Some(shell_render_env(runtime, context, current_cwd).map_err(|_| 196));
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
    None
}
