use super::*;

const RUST_AGENT_DEFAULT_DEPTH: usize = 4;
const RUST_AGENT_MAX_DEPTH: usize = 12;

enum RustAgentCommand<'a> {
    RustSymbols { path: &'a str, depth: usize },
    CrateSymbols { name: &'a str, depth: usize },
    UnsafeAudit { path: &'a str, depth: usize },
    TodoRust { path: &'a str, depth: usize },
}

impl<'a> RustAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("rust-symbols ") {
            return Some(
                parse_path_depth_command(rest)
                    .map(|(path, depth)| Self::RustSymbols { path, depth }),
            );
        }
        if let Some(rest) = line.strip_prefix("crate-symbols ") {
            return Some(
                parse_name_depth_command(rest)
                    .map(|(name, depth)| Self::CrateSymbols { name, depth }),
            );
        }
        if let Some(rest) = line.strip_prefix("unsafe-audit ") {
            return Some(
                parse_path_depth_command(rest)
                    .map(|(path, depth)| Self::UnsafeAudit { path, depth }),
            );
        }
        if let Some(rest) = line.strip_prefix("todo-rust ") {
            return Some(
                parse_path_depth_command(rest).map(|(path, depth)| Self::TodoRust { path, depth }),
            );
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::RustSymbols { path, depth } => render_rust_symbols(runtime, cwd, path, depth),
            Self::CrateSymbols { name, depth } => render_crate_symbols(runtime, name, depth),
            Self::UnsafeAudit { path, depth } => render_unsafe_audit(runtime, cwd, path, depth),
            Self::TodoRust { path, depth } => render_todo_rust(runtime, cwd, path, depth),
        }
    }
}

fn parse_path_depth_command(rest: &str) -> Result<(&str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    if path.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => RUST_AGENT_DEFAULT_DEPTH,
    };
    Ok((path, depth.min(RUST_AGENT_MAX_DEPTH)))
}

fn parse_name_depth_command(rest: &str) -> Result<(&str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let name = parts.next().ok_or(2)?;
    if name.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => RUST_AGENT_DEFAULT_DEPTH,
    };
    Ok((name, depth.min(RUST_AGENT_MAX_DEPTH)))
}

fn render_rust_symbols<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let mut visited = 0usize;
    let mut matched = 0usize;
    walk_rust_tree(
        runtime,
        &root,
        depth,
        0,
        &mut visited,
        &mut |runtime, path| {
            matched += render_symbols_in_file(runtime, path)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "rust-symbols-summary path={root} depth={depth} visited={visited} matches={matched}"
        ),
    )
}

fn render_crate_symbols<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let Some(member) = parse_rust_workspace_members(&cargo)
        .into_iter()
        .find(|member| member.ends_with(name))
    else {
        write_line(runtime, &format!("crate-symbols-miss name={name}"))?;
        return Err(249);
    };
    let root = format!("/{member}/src");
    let mut visited = 0usize;
    let mut matched = 0usize;
    walk_rust_tree(
        runtime,
        &root,
        depth,
        0,
        &mut visited,
        &mut |runtime, path| {
            matched += render_symbols_in_file(runtime, path)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "crate-symbols-summary crate={name} path={root} depth={depth} visited={visited} matches={matched}"
        ),
    )
}

fn render_unsafe_audit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let mut visited = 0usize;
    let mut matches = 0usize;
    walk_rust_tree(
        runtime,
        &root,
        depth,
        0,
        &mut visited,
        &mut |runtime, path| {
            matches += render_matching_lines(runtime, path, &["unsafe "], "unsafe-line")?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "unsafe-audit-summary path={root} depth={depth} visited={visited} matches={matches}"
        ),
    )
}

fn render_todo_rust<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let mut visited = 0usize;
    let mut matches = 0usize;
    walk_rust_tree(
        runtime,
        &root,
        depth,
        0,
        &mut visited,
        &mut |runtime, path| {
            matches +=
                render_matching_lines(runtime, path, &["TODO", "FIXME", "XXX"], "todo-line")?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!("todo-rust-summary path={root} depth={depth} visited={visited} matches={matches}"),
    )
}

fn walk_rust_tree<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    path: &str,
    max_depth: usize,
    depth: usize,
    visited: &mut usize,
    on_file: &mut F,
) -> Result<(), ExitCode>
where
    F: FnMut(&Runtime<B>, &str) -> Result<(), ExitCode>,
{
    let status = runtime.stat_path(path).map_err(|_| 231)?;
    *visited += 1;
    match NativeObjectKind::from_raw(status.kind) {
        Some(NativeObjectKind::Directory) if depth < max_depth => {
            for entry in list_rust_entries(runtime, path)? {
                let child = join_rust_path(path, &entry);
                walk_rust_tree(runtime, &child, max_depth, depth + 1, visited, on_file)?;
            }
        }
        Some(NativeObjectKind::File) if path.ends_with(".rs") => on_file(runtime, path)?,
        _ => {}
    }
    Ok(())
}

fn render_symbols_in_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<usize, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let mut matches = 0usize;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let kind = if let Some(rest) = trimmed.strip_prefix("pub fn ") {
            Some(("fn", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("fn ") {
            Some(("fn", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("pub struct ") {
            Some(("struct", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("struct ") {
            Some(("struct", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("pub enum ") {
            Some(("enum", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("enum ") {
            Some(("enum", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("pub trait ") {
            Some(("trait", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("trait ") {
            Some(("trait", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("impl ") {
            Some(("impl", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("pub mod ") {
            Some(("mod", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("mod ") {
            Some(("mod", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("pub const ") {
            Some(("const", parse_symbol_name(rest)))
        } else if let Some(rest) = trimmed.strip_prefix("const ") {
            Some(("const", parse_symbol_name(rest)))
        } else {
            None
        };
        if let Some((kind, name)) = kind {
            matches += 1;
            write_line(
                runtime,
                &format!(
                    "rust-symbol kind={} path={} line={} name={}",
                    kind,
                    path,
                    index + 1,
                    name
                ),
            )?;
        }
    }
    Ok(matches)
}

fn parse_symbol_name(rest: &str) -> String {
    rest.chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == ':' || *ch == '<')
        .collect()
}

fn parse_rust_workspace_members(cargo: &str) -> Vec<String> {
    let Some(start) = cargo.find("members = [") else {
        return Vec::new();
    };
    let rest = &cargo[start + "members = [".len()..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('"') && line.ends_with("\","))
        .map(|line| line.trim_matches(',').trim_matches('"').to_string())
        .collect()
}

fn render_matching_lines<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needles: &[&str],
    label: &str,
) -> Result<usize, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let mut matches = 0usize;
    for (index, line) in text.lines().enumerate() {
        if needles.iter().any(|needle| line.contains(needle)) {
            matches += 1;
            write_line(
                runtime,
                &format!("{} {}:{} {}", label, path, index + 1, line.trim()),
            )?;
        }
    }
    Ok(matches)
}

fn list_rust_entries<B: SyscallBackend>(
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

fn join_rust_path(base: &str, leaf: &str) -> String {
    if base == "/" {
        format!("/{leaf}")
    } else {
        format!("{base}/{leaf}")
    }
}

pub(super) fn try_handle_rust_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match RustAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("rust-symbols ") {
                "usage: rust-symbols <path> [depth]"
            } else if line.starts_with("crate-symbols ") {
                "usage: crate-symbols <name> [depth]"
            } else if line.starts_with("unsafe-audit ") {
                "usage: unsafe-audit <path> [depth]"
            } else {
                "usage: todo-rust <path> [depth]"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
