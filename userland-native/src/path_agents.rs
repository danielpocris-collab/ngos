use super::*;

const PATH_AGENT_DEFAULT_DEPTH: usize = 4;
const PATH_AGENT_MAX_DEPTH: usize = 16;

enum PathAgentCommand<'a> {
    TreePath {
        path: &'a str,
        depth: usize,
    },
    FindPath {
        path: &'a str,
        needle: &'a str,
        depth: usize,
    },
}

impl<'a> PathAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("tree-path ") {
            return Some(parse_tree_command(rest));
        }
        if let Some(rest) = line.strip_prefix("find-path ") {
            return Some(parse_find_command(rest));
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::TreePath { path, depth } => render_tree(runtime, cwd, path, depth),
            Self::FindPath {
                path,
                needle,
                depth,
            } => render_find(runtime, cwd, path, needle, depth),
        }
    }
}

fn parse_tree_command(rest: &str) -> Result<PathAgentCommand<'_>, ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    if path.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => PATH_AGENT_DEFAULT_DEPTH,
    };
    Ok(PathAgentCommand::TreePath {
        path,
        depth: depth.min(PATH_AGENT_MAX_DEPTH),
    })
}

fn parse_find_command(rest: &str) -> Result<PathAgentCommand<'_>, ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    let needle = parts.next().ok_or(2)?;
    if path.is_empty() || needle.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => PATH_AGENT_DEFAULT_DEPTH,
    };
    Ok(PathAgentCommand::FindPath {
        path,
        needle,
        depth: depth.min(PATH_AGENT_MAX_DEPTH),
    })
}

fn render_tree<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    write_line(runtime, &format!("tree-root path={root} depth={depth}"))?;
    let mut visited = 0usize;
    walk_tree(runtime, &root, depth, 0, &mut visited, None)?;
    write_line(
        runtime,
        &format!("tree-summary path={root} visited={visited}"),
    )
}

fn render_find<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    needle: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let mut visited = 0usize;
    let mut matched = 0usize;
    walk_find(runtime, &root, needle, depth, 0, &mut visited, &mut matched)?;
    write_line(
        runtime,
        &format!("find-summary path={root} needle={needle} visited={visited} matches={matched}"),
    )
}

fn walk_tree<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    max_depth: usize,
    depth: usize,
    visited: &mut usize,
    name_hint: Option<&str>,
) -> Result<(), ExitCode> {
    let status = runtime.stat_path(path).map_err(|_| 231)?;
    *visited += 1;
    let label = name_hint.unwrap_or(path);
    write_line(
        runtime,
        &format!(
            "tree {}{} kind={} size={}",
            "  ".repeat(depth),
            label,
            object_kind_name(status.kind),
            status.size
        ),
    )?;
    if depth >= max_depth
        || NativeObjectKind::from_raw(status.kind) != Some(NativeObjectKind::Directory)
    {
        return Ok(());
    }
    let entries = list_directory_entries(runtime, path)?;
    for entry in entries {
        let child = join_shell_path(path, &entry);
        walk_tree(runtime, &child, max_depth, depth + 1, visited, Some(&entry))?;
    }
    Ok(())
}

fn walk_find<B: SyscallBackend>(
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
    if path.contains(needle) {
        *matched += 1;
        write_line(
            runtime,
            &format!(
                "find path={} kind={} size={}",
                path,
                object_kind_name(status.kind),
                status.size
            ),
        )?;
    }
    if depth >= max_depth
        || NativeObjectKind::from_raw(status.kind) != Some(NativeObjectKind::Directory)
    {
        return Ok(());
    }
    let entries = list_directory_entries(runtime, path)?;
    for entry in entries {
        let child = join_shell_path(path, &entry);
        walk_find(
            runtime,
            &child,
            needle,
            max_depth,
            depth + 1,
            visited,
            matched,
        )?;
    }
    Ok(())
}

fn list_directory_entries<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<String>, ExitCode> {
    let mut buffer = vec![0u8; 512];
    loop {
        let count = runtime.list_path(path, &mut buffer).map_err(|_| 246)?;
        if count < buffer.len() {
            let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 247)?;
            let entries = text
                .lines()
                .map(str::trim)
                .filter(|entry| !entry.is_empty() && *entry != "." && *entry != "..")
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            return Ok(entries);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

fn join_shell_path(base: &str, leaf: &str) -> String {
    if base == "/" {
        format!("/{leaf}")
    } else {
        format!("{base}/{leaf}")
    }
}

pub(super) fn try_handle_path_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match PathAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("tree-path ") {
                "usage: tree-path <path> [depth]"
            } else {
                "usage: find-path <path> <needle> [depth]"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
