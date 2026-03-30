use super::*;

const CODE_AGENT_DEFAULT_DEPTH: usize = 4;
const CODE_AGENT_MAX_DEPTH: usize = 16;
const CODE_AGENT_DEFAULT_WINDOW: usize = 24;

enum CodeAgentCommand<'a> {
    CatNumbered {
        path: &'a str,
        start: usize,
        count: usize,
    },
    FindText {
        path: &'a str,
        needle: &'a str,
    },
    FindTreeText {
        path: &'a str,
        needle: &'a str,
        depth: usize,
    },
    ReplaceText {
        path: &'a str,
        from: &'a str,
        to: &'a str,
    },
    ReplaceLine {
        path: &'a str,
        line: usize,
        text: &'a str,
    },
    InsertLine {
        path: &'a str,
        line: usize,
        text: &'a str,
    },
    DeleteLine {
        path: &'a str,
        line: usize,
    },
    AppendLine {
        path: &'a str,
        text: &'a str,
    },
    InsertBefore {
        path: &'a str,
        needle: &'a str,
        text: &'a str,
    },
    InsertAfter {
        path: &'a str,
        needle: &'a str,
        text: &'a str,
    },
}

impl<'a> CodeAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("cat-numbered ") {
            return Some(parse_cat_numbered(rest));
        }
        if let Some(rest) = line.strip_prefix("find-text ") {
            return Some(
                parse_find_text(rest).map(|(path, needle)| Self::FindText { path, needle }),
            );
        }
        if let Some(rest) = line.strip_prefix("find-tree-text ") {
            return Some(parse_find_tree_text(rest));
        }
        if let Some(rest) = line.strip_prefix("replace-text ") {
            return Some(
                parse_three_part_command(rest).map(|(path, from, to)| Self::ReplaceText {
                    path,
                    from,
                    to,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("replace-line ") {
            return Some(
                parse_path_line_text_command(rest).map(|(path, line, text)| Self::ReplaceLine {
                    path,
                    line,
                    text,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("insert-line ") {
            return Some(
                parse_path_line_text_command(rest).map(|(path, line, text)| Self::InsertLine {
                    path,
                    line,
                    text,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("delete-line ") {
            return Some(
                parse_path_line_command(rest).map(|(path, line)| Self::DeleteLine { path, line }),
            );
        }
        if let Some(rest) = line.strip_prefix("append-line ") {
            return Some(
                parse_path_text_command(rest).map(|(path, text)| Self::AppendLine { path, text }),
            );
        }
        if let Some(rest) = line.strip_prefix("insert-before ") {
            return Some(
                parse_three_part_command(rest).map(|(path, needle, text)| Self::InsertBefore {
                    path,
                    needle,
                    text,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("insert-after ") {
            return Some(
                parse_three_part_command(rest).map(|(path, needle, text)| Self::InsertAfter {
                    path,
                    needle,
                    text,
                }),
            );
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::CatNumbered { path, start, count } => {
                render_numbered_file(runtime, cwd, path, start, count)
            }
            Self::FindText { path, needle } => render_find_text(runtime, cwd, path, needle),
            Self::FindTreeText {
                path,
                needle,
                depth,
            } => render_find_tree_text(runtime, cwd, path, needle, depth),
            Self::ReplaceText { path, from, to } => replace_text(runtime, cwd, path, from, to),
            Self::ReplaceLine { path, line, text } => replace_line(runtime, cwd, path, line, text),
            Self::InsertLine { path, line, text } => insert_line(runtime, cwd, path, line, text),
            Self::DeleteLine { path, line } => delete_line(runtime, cwd, path, line),
            Self::AppendLine { path, text } => append_line(runtime, cwd, path, text),
            Self::InsertBefore { path, needle, text } => {
                insert_relative(runtime, cwd, path, needle, text, true)
            }
            Self::InsertAfter { path, needle, text } => {
                insert_relative(runtime, cwd, path, needle, text, false)
            }
        }
    }
}

fn parse_cat_numbered(rest: &str) -> Result<CodeAgentCommand<'_>, ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    if path.is_empty() {
        return Err(2);
    }
    let start = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => 1,
    };
    let count = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => CODE_AGENT_DEFAULT_WINDOW,
    };
    Ok(CodeAgentCommand::CatNumbered { path, start, count })
}

fn parse_find_text(rest: &str) -> Result<(&str, &str), ExitCode> {
    let mut parts = rest.splitn(2, char::is_whitespace);
    let path = parts.next().ok_or(2)?;
    let needle = parts.next().map(str::trim_start).ok_or(2)?;
    if path.is_empty() || needle.is_empty() {
        return Err(2);
    }
    Ok((path, needle))
}

fn parse_find_tree_text(rest: &str) -> Result<CodeAgentCommand<'_>, ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    let needle = parts.next().ok_or(2)?;
    if path.is_empty() || needle.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => CODE_AGENT_DEFAULT_DEPTH,
    };
    Ok(CodeAgentCommand::FindTreeText {
        path,
        needle,
        depth: depth.min(CODE_AGENT_MAX_DEPTH),
    })
}

fn parse_path_text_command(rest: &str) -> Result<(&str, &str), ExitCode> {
    let mut parts = rest.splitn(2, char::is_whitespace);
    let path = parts.next().ok_or(2)?;
    let text = parts.next().map(str::trim_start).ok_or(2)?;
    if path.is_empty() || text.is_empty() {
        return Err(2);
    }
    Ok((path, text))
}

fn parse_path_line_command(rest: &str) -> Result<(&str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    let line = parts.next().ok_or(2)?.parse::<usize>().map_err(|_| 2)?;
    if path.is_empty() || line == 0 {
        return Err(2);
    }
    Ok((path, line))
}

fn parse_path_line_text_command(rest: &str) -> Result<(&str, usize, &str), ExitCode> {
    let mut parts = rest.splitn(3, char::is_whitespace);
    let path = parts.next().ok_or(2)?;
    let line = parts.next().ok_or(2)?.parse::<usize>().map_err(|_| 2)?;
    let text = parts.next().map(str::trim_start).ok_or(2)?;
    if path.is_empty() || line == 0 || text.is_empty() {
        return Err(2);
    }
    Ok((path, line, text))
}

fn parse_three_part_command(rest: &str) -> Result<(&str, &str, &str), ExitCode> {
    let mut parts = rest.splitn(3, char::is_whitespace);
    let path = parts.next().ok_or(2)?;
    let first = parts.next().ok_or(2)?;
    let second = parts.next().map(str::trim_start).ok_or(2)?;
    if path.is_empty() || first.is_empty() || second.is_empty() {
        return Err(2);
    }
    Ok((path, first, second))
}

fn render_numbered_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    start: usize,
    count: usize,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let text = shell_read_file_text(runtime, &resolved)?;
    let lines = text.lines().collect::<Vec<_>>();
    let start_index = start.saturating_sub(1).min(lines.len());
    let end_index = start_index.saturating_add(count).min(lines.len());
    for (index, line) in lines[start_index..end_index].iter().enumerate() {
        write_line(
            runtime,
            &format!("{:>5}: {}", start_index + index + 1, line),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "cat-numbered-summary path={resolved} start={} shown={}",
            start_index + 1,
            end_index.saturating_sub(start_index)
        ),
    )
}

fn render_find_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    needle: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let text = shell_read_file_text(runtime, &resolved)?;
    let mut matches = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line.contains(needle) {
            matches += 1;
            write_line(
                runtime,
                &format!("match {}:{} {}", resolved, index + 1, line),
            )?;
        }
    }
    write_line(
        runtime,
        &format!("find-text-summary path={resolved} needle={needle} matches={matches}"),
    )
}

fn render_find_tree_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    needle: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let mut visited = 0usize;
    let mut matched = 0usize;
    walk_tree_text(runtime, &root, needle, depth, 0, &mut visited, &mut matched)?;
    write_line(
        runtime,
        &format!(
            "find-tree-text-summary path={root} needle={needle} depth={depth} visited={visited} matches={matched}"
        ),
    )
}

fn walk_tree_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
    max_depth: usize,
    depth: usize,
    visited: &mut usize,
    matched: &mut usize,
) -> Result<(), ExitCode> {
    let status = runtime.stat_path(path).map_err(|_| 231)?;
    *visited += 1;
    match NativeObjectKind::from_raw(status.kind) {
        Some(NativeObjectKind::Directory) if depth < max_depth => {
            for entry in list_code_directory_entries(runtime, path)? {
                let child = if path == "/" {
                    format!("/{entry}")
                } else {
                    format!("{path}/{entry}")
                };
                walk_tree_text(
                    runtime,
                    &child,
                    needle,
                    max_depth,
                    depth + 1,
                    visited,
                    matched,
                )?;
            }
        }
        Some(NativeObjectKind::File) => {
            let text = shell_read_file_text(runtime, path)?;
            for (index, line) in text.lines().enumerate() {
                if line.contains(needle) {
                    *matched += 1;
                    write_line(runtime, &format!("match {}:{} {}", path, index + 1, line))?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn replace_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    from: &str,
    to: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let text = shell_read_file_text(runtime, &resolved)?;
    let matches = text.matches(from).count();
    if matches == 0 {
        write_line(
            runtime,
            &format!("replace-text-noop path={resolved} needle={from}"),
        )?;
        return Ok(());
    }
    let updated = text.replace(from, to);
    shell_write_file(runtime, &resolved, &updated)?;
    write_line(
        runtime,
        &format!("replace-text-ok path={resolved} replacements={matches}"),
    )
}

fn replace_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    line: usize,
    text: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let mut lines = load_lines(runtime, &resolved)?;
    let index = line.checked_sub(1).ok_or(249)?;
    if index >= lines.len() {
        return Err(249);
    }
    lines[index] = text.to_string();
    store_lines(runtime, &resolved, &lines)?;
    write_line(
        runtime,
        &format!("replace-line-ok path={resolved} line={line}"),
    )
}

fn insert_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    line: usize,
    text: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let mut lines = load_lines(runtime, &resolved)?;
    let index = line.checked_sub(1).ok_or(249)?;
    if index > lines.len() {
        return Err(249);
    }
    lines.insert(index, text.to_string());
    store_lines(runtime, &resolved, &lines)?;
    write_line(
        runtime,
        &format!("insert-line-ok path={resolved} line={line}"),
    )
}

fn delete_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    line: usize,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let mut lines = load_lines(runtime, &resolved)?;
    let index = line.checked_sub(1).ok_or(249)?;
    if index >= lines.len() {
        return Err(249);
    }
    lines.remove(index);
    store_lines(runtime, &resolved, &lines)?;
    write_line(
        runtime,
        &format!("delete-line-ok path={resolved} line={line}"),
    )
}

fn append_line<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    text: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let mut lines = load_lines(runtime, &resolved)?;
    lines.push(text.to_string());
    store_lines(runtime, &resolved, &lines)?;
    write_line(
        runtime,
        &format!("append-line-ok path={resolved} line={}", lines.len()),
    )
}

fn insert_relative<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    needle: &str,
    text: &str,
    before: bool,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    let mut lines = load_lines(runtime, &resolved)?;
    let Some(index) = lines.iter().position(|line| line.contains(needle)) else {
        write_line(
            runtime,
            &format!("insert-relative-miss path={resolved} needle={needle}"),
        )?;
        return Err(249);
    };
    let target = if before { index } else { index + 1 };
    lines.insert(target, text.to_string());
    store_lines(runtime, &resolved, &lines)?;
    write_line(
        runtime,
        &format!(
            "{}-ok path={} anchor-line={} inserted-line={}",
            if before {
                "insert-before"
            } else {
                "insert-after"
            },
            resolved,
            index + 1,
            target + 1
        ),
    )
}

fn load_lines<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<String>, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    if text.is_empty() {
        return Ok(Vec::new());
    }
    Ok(text.lines().map(ToString::to_string).collect())
}

fn store_lines<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    lines: &[String],
) -> Result<(), ExitCode> {
    let body = lines.join("\n");
    shell_write_file(runtime, path, &body)
}

fn list_code_directory_entries<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<String>, ExitCode> {
    let mut buffer = vec![0u8; 512];
    loop {
        let count = runtime.list_path(path, &mut buffer).map_err(|_| 246)?;
        if count < buffer.len() {
            let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 247)?;
            return Ok(text
                .lines()
                .map(str::trim)
                .filter(|entry| !entry.is_empty() && *entry != "." && *entry != "..")
                .map(ToString::to_string)
                .collect());
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

pub(super) fn try_handle_code_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match CodeAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("cat-numbered ") {
                "usage: cat-numbered <path> [start] [count]"
            } else if line.starts_with("find-text ") {
                "usage: find-text <path> <needle>"
            } else if line.starts_with("find-tree-text ") {
                "usage: find-tree-text <path> <needle> [depth]"
            } else if line.starts_with("replace-text ") {
                "usage: replace-text <path> <from> <to>"
            } else if line.starts_with("replace-line ") {
                "usage: replace-line <path> <line> <text>"
            } else if line.starts_with("insert-line ") {
                "usage: insert-line <path> <line> <text>"
            } else if line.starts_with("delete-line ") {
                "usage: delete-line <path> <line>"
            } else if line.starts_with("append-line ") {
                "usage: append-line <path> <text>"
            } else if line.starts_with("insert-before ") {
                "usage: insert-before <path> <needle> <text>"
            } else {
                "usage: insert-after <path> <needle> <text>"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
