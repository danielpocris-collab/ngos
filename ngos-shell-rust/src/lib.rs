//! Canonical subsystem role:
//! - subsystem: native Rust symbol inspection surface
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing Rust code structure inspection over
//!   canonical repository contents
//!
//! Canonical contract families handled here:
//! - symbol inspection contracts
//! - Rust structure discovery contracts
//! - crate symbol search contracts
//!
//! This module may inspect Rust source structure, but it must not redefine
//! subsystem semantics or architectural ownership.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use ngos_shell_types::resolve_shell_path;
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::{ExitCode, NativeObjectKind, SyscallBackend};
use ngos_user_runtime::Runtime;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

const RUST_AGENT_DEFAULT_DEPTH: usize = 4;
const RUST_AGENT_MAX_DEPTH: usize = 12;

enum RustAgentCommand<'a> {
    RustSymbols {
        path: &'a str,
        depth: usize,
    },
    CrateSymbols {
        name: &'a str,
        depth: usize,
    },
    FindSymbol {
        path: &'a str,
        needle: &'a str,
        depth: usize,
    },
    CrateFindSymbol {
        name: &'a str,
        needle: &'a str,
        depth: usize,
    },
    Refs {
        path: &'a str,
        needle: &'a str,
        depth: usize,
    },
    CrateRefs {
        name: &'a str,
        needle: &'a str,
        depth: usize,
    },
    Outline {
        path: &'a str,
        depth: usize,
    },
    CrateOutline {
        name: &'a str,
        depth: usize,
    },
    UnsafeAudit {
        path: &'a str,
        depth: usize,
    },
    TodoRust {
        path: &'a str,
        depth: usize,
    },
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
        if let Some(rest) = line.strip_prefix("find-symbol ") {
            return Some(
                parse_path_needle_depth_command(rest).map(|(path, needle, depth)| {
                    Self::FindSymbol {
                        path,
                        needle,
                        depth,
                    }
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("crate-find-symbol ") {
            return Some(
                parse_name_needle_depth_command(rest).map(|(name, needle, depth)| {
                    Self::CrateFindSymbol {
                        name,
                        needle,
                        depth,
                    }
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("refs ") {
            return Some(
                parse_path_needle_depth_command(rest).map(|(path, needle, depth)| Self::Refs {
                    path,
                    needle,
                    depth,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("crate-refs ") {
            return Some(
                parse_name_needle_depth_command(rest).map(|(name, needle, depth)| {
                    Self::CrateRefs {
                        name,
                        needle,
                        depth,
                    }
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("outline ") {
            return Some(
                parse_path_depth_command(rest).map(|(path, depth)| Self::Outline { path, depth }),
            );
        }
        if let Some(rest) = line.strip_prefix("crate-outline ") {
            return Some(
                parse_name_depth_command(rest)
                    .map(|(name, depth)| Self::CrateOutline { name, depth }),
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
            Self::FindSymbol {
                path,
                needle,
                depth,
            } => render_find_symbol(runtime, cwd, path, needle, depth),
            Self::CrateFindSymbol {
                name,
                needle,
                depth,
            } => render_crate_find_symbol(runtime, name, needle, depth),
            Self::Refs {
                path,
                needle,
                depth,
            } => render_refs(runtime, cwd, path, needle, depth),
            Self::CrateRefs {
                name,
                needle,
                depth,
            } => render_crate_refs(runtime, name, needle, depth),
            Self::Outline { path, depth } => render_outline(runtime, cwd, path, depth),
            Self::CrateOutline { name, depth } => render_crate_outline(runtime, name, depth),
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

fn parse_path_needle_depth_command(rest: &str) -> Result<(&str, &str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    let needle = parts.next().ok_or(2)?;
    if path.is_empty() || needle.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => RUST_AGENT_DEFAULT_DEPTH,
    };
    Ok((path, needle, depth.min(RUST_AGENT_MAX_DEPTH)))
}

fn parse_name_needle_depth_command(rest: &str) -> Result<(&str, &str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let name = parts.next().ok_or(2)?;
    let needle = parts.next().ok_or(2)?;
    if name.is_empty() || needle.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => RUST_AGENT_DEFAULT_DEPTH,
    };
    Ok((name, needle, depth.min(RUST_AGENT_MAX_DEPTH)))
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
    let root = resolve_crate_rust_root(runtime, name, "crate-symbols-miss")?;
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

fn render_find_symbol<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    needle: &str,
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
            matches += render_symbol_matches_in_file(runtime, path, needle)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "find-symbol-summary path={root} needle={needle} depth={depth} visited={visited} matches={matches}"
        ),
    )
}

fn render_crate_find_symbol<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    needle: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_crate_rust_root(runtime, name, "crate-find-symbol-miss")?;
    let mut visited = 0usize;
    let mut matches = 0usize;
    walk_rust_tree(
        runtime,
        &root,
        depth,
        0,
        &mut visited,
        &mut |runtime, path| {
            matches += render_symbol_matches_in_file(runtime, path, needle)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "crate-find-symbol-summary crate={name} path={root} needle={needle} depth={depth} visited={visited} matches={matches}"
        ),
    )
}

fn render_refs<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    needle: &str,
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
            matches += render_refs_in_file(runtime, path, needle)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "refs-summary path={root} needle={needle} depth={depth} visited={visited} matches={matches}"
        ),
    )
}

fn render_crate_refs<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    needle: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_crate_rust_root(runtime, name, "crate-refs-miss")?;
    let mut visited = 0usize;
    let mut matches = 0usize;
    walk_rust_tree(
        runtime,
        &root,
        depth,
        0,
        &mut visited,
        &mut |runtime, path| {
            matches += render_refs_in_file(runtime, path, needle)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "crate-refs-summary crate={name} path={root} needle={needle} depth={depth} visited={visited} matches={matches}"
        ),
    )
}

fn render_outline<B: SyscallBackend>(
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
            matches += render_outline_in_file(runtime, path)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!("outline-summary path={root} depth={depth} visited={visited} matches={matches}"),
    )
}

fn render_crate_outline<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_crate_rust_root(runtime, name, "crate-outline-miss")?;
    let mut visited = 0usize;
    let mut matches = 0usize;
    walk_rust_tree(
        runtime,
        &root,
        depth,
        0,
        &mut visited,
        &mut |runtime, path| {
            matches += render_outline_in_file(runtime, path)?;
            Ok(())
        },
    )?;
    write_line(
        runtime,
        &format!(
            "crate-outline-summary crate={name} path={root} depth={depth} visited={visited} matches={matches}"
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
        if let Some(symbol) = parse_rust_symbol_line(line) {
            matches += 1;
            write_line(
                runtime,
                &format!(
                    "rust-symbol kind={} path={} line={} name={}",
                    symbol.kind,
                    path,
                    index + 1,
                    symbol.name
                ),
            )?;
        }
    }
    Ok(matches)
}

fn render_symbol_matches_in_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
) -> Result<usize, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let mut matches = 0usize;
    for (index, line) in text.lines().enumerate() {
        let Some(symbol) = parse_rust_symbol_line(line) else {
            continue;
        };
        if !symbol.name.contains(needle) {
            continue;
        }
        matches += 1;
        write_line(
            runtime,
            &format!(
                "symbol-match kind={} path={} line={} name={}",
                symbol.kind,
                path,
                index + 1,
                symbol.name
            ),
        )?;
    }
    Ok(matches)
}

fn render_refs_in_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
) -> Result<usize, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let mut matches = 0usize;
    for (index, line) in text.lines().enumerate() {
        if !contains_rust_identifier_token(line, needle) {
            continue;
        }
        let kind = parse_rust_symbol_line(line)
            .filter(|symbol| symbol.name == needle)
            .map(|_| "definition")
            .unwrap_or("reference");
        matches += 1;
        write_line(
            runtime,
            &format!(
                "rust-ref kind={} path={} line={} text={}",
                kind,
                path,
                index + 1,
                line.trim()
            ),
        )?;
    }
    Ok(matches)
}

fn render_outline_in_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<usize, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let mut matches = 0usize;
    for (index, line) in text.lines().enumerate() {
        let Some(symbol) = parse_rust_symbol_line(line) else {
            continue;
        };
        let indent = line.len().saturating_sub(line.trim_start().len()) / 4;
        matches += 1;
        write_line(
            runtime,
            &format!(
                "outline-symbol path={} line={} indent={} kind={} name={}",
                path,
                index + 1,
                indent,
                symbol.kind,
                symbol.name
            ),
        )?;
    }
    Ok(matches)
}

fn parse_symbol_name(rest: &str) -> String {
    rest.chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == ':' || *ch == '<')
        .collect()
}

struct RustSymbol<'a> {
    kind: &'a str,
    name: String,
}

fn parse_rust_symbol_line(line: &str) -> Option<RustSymbol<'static>> {
    let trimmed = line.trim_start();
    let (kind, rest) = if let Some(rest) = trimmed.strip_prefix("pub fn ") {
        ("fn", rest)
    } else if let Some(rest) = trimmed.strip_prefix("fn ") {
        ("fn", rest)
    } else if let Some(rest) = trimmed.strip_prefix("pub struct ") {
        ("struct", rest)
    } else if let Some(rest) = trimmed.strip_prefix("struct ") {
        ("struct", rest)
    } else if let Some(rest) = trimmed.strip_prefix("pub enum ") {
        ("enum", rest)
    } else if let Some(rest) = trimmed.strip_prefix("enum ") {
        ("enum", rest)
    } else if let Some(rest) = trimmed.strip_prefix("pub trait ") {
        ("trait", rest)
    } else if let Some(rest) = trimmed.strip_prefix("trait ") {
        ("trait", rest)
    } else if let Some(rest) = trimmed.strip_prefix("impl ") {
        ("impl", rest)
    } else if let Some(rest) = trimmed.strip_prefix("pub mod ") {
        ("mod", rest)
    } else if let Some(rest) = trimmed.strip_prefix("mod ") {
        ("mod", rest)
    } else if let Some(rest) = trimmed.strip_prefix("pub const ") {
        ("const", rest)
    } else if let Some(rest) = trimmed.strip_prefix("const ") {
        ("const", rest)
    } else {
        return None;
    };
    Some(RustSymbol {
        kind,
        name: parse_symbol_name(rest),
    })
}

fn contains_rust_identifier_token(line: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let haystack = line.as_bytes();
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .enumerate()
        .any(|(index, candidate)| {
            if candidate != needle {
                return false;
            }
            let before_ok = index == 0 || !is_identifier_byte(haystack[index - 1]);
            let after = index + needle.len();
            let after_ok = after == haystack.len() || !is_identifier_byte(haystack[after]);
            before_ok && after_ok
        })
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn resolve_crate_rust_root<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    miss_label: &str,
) -> Result<String, ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let Some(member) = parse_rust_workspace_members(&cargo)
        .into_iter()
        .find(|member| member.ends_with(name))
    else {
        write_line(runtime, &format!("{miss_label} name={name}"))?;
        return Err(249);
    };
    Ok(format!("/{member}/src"))
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
                .filter_map(|entry| entry.split_once('\t').map(|(name, _)| name).or(Some(entry)))
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

pub fn try_handle_rust_agent_command<B: SyscallBackend>(
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
            } else if line.starts_with("find-symbol ") {
                "usage: find-symbol <path> <needle> [depth]"
            } else if line.starts_with("crate-find-symbol ") {
                "usage: crate-find-symbol <name> <needle> [depth]"
            } else if line.starts_with("refs ") {
                "usage: refs <path> <needle> [depth]"
            } else if line.starts_with("crate-refs ") {
                "usage: crate-refs <name> <needle> [depth]"
            } else if line.starts_with("outline ") {
                "usage: outline <path> [depth]"
            } else if line.starts_with("crate-outline ") {
                "usage: crate-outline <name> [depth]"
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
