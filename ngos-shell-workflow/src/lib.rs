//! Workflow command surface: touch-file, truncate-file, move-path, grep-tree, copy-tree, mirror-tree.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use ngos_shell_types::resolve_shell_path;
use ngos_shell_vfs::{shell_mkdir_path, shell_mkfile_path, shell_read_file_text, shell_write_file};
use ngos_user_abi::{ExitCode, NativeObjectKind, SyscallBackend};
use ngos_user_runtime::Runtime;

const WORKFLOW_AGENT_DEFAULT_DEPTH: usize = 4;
const WORKFLOW_AGENT_MAX_DEPTH: usize = 16;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

enum WorkflowAgentCommand<'a> {
    TouchFile {
        path: &'a str,
    },
    TruncateFile {
        path: &'a str,
    },
    MovePath {
        from: &'a str,
        to: &'a str,
    },
    GrepTree {
        path: &'a str,
        needle: &'a str,
        depth: usize,
    },
    CopyTree {
        source: &'a str,
        destination: &'a str,
        depth: usize,
    },
    MirrorTree {
        source: &'a str,
        destination: &'a str,
        depth: usize,
    },
}

impl<'a> WorkflowAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("touch-file ") {
            let path = rest.trim();
            return Some(
                (!path.is_empty())
                    .then_some(Self::TouchFile { path })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("truncate-file ") {
            let path = rest.trim();
            return Some(
                (!path.is_empty())
                    .then_some(Self::TruncateFile { path })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("move-path ") {
            return Some(parse_pair_command(rest).map(|(from, to)| Self::MovePath { from, to }));
        }
        if let Some(rest) = line.strip_prefix("grep-tree ") {
            return Some(
                parse_tree_workflow_command(rest).map(|(path, needle, depth)| Self::GrepTree {
                    path,
                    needle,
                    depth,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("copy-tree ") {
            return Some(
                parse_tree_pair_command(rest).map(|(source, destination, depth)| Self::CopyTree {
                    source,
                    destination,
                    depth,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("mirror-tree ") {
            return Some(
                parse_tree_pair_command(rest).map(|(source, destination, depth)| {
                    Self::MirrorTree {
                        source,
                        destination,
                        depth,
                    }
                }),
            );
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::TouchFile { path } => touch_file(runtime, cwd, path),
            Self::TruncateFile { path } => truncate_file(runtime, cwd, path),
            Self::MovePath { from, to } => move_path(runtime, cwd, from, to),
            Self::GrepTree {
                path,
                needle,
                depth,
            } => grep_tree(runtime, cwd, path, needle, depth),
            Self::CopyTree {
                source,
                destination,
                depth,
            } => copy_tree(runtime, cwd, source, destination, depth, false),
            Self::MirrorTree {
                source,
                destination,
                depth,
            } => copy_tree(runtime, cwd, source, destination, depth, true),
        }
    }
}

fn parse_pair_command(rest: &str) -> Result<(&str, &str), ExitCode> {
    let mut parts = rest.split_whitespace();
    let first = parts.next().ok_or(2)?;
    let second = parts.next().ok_or(2)?;
    if first.is_empty() || second.is_empty() {
        return Err(2);
    }
    Ok((first, second))
}

fn parse_tree_workflow_command(rest: &str) -> Result<(&str, &str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    let needle = parts.next().ok_or(2)?;
    if path.is_empty() || needle.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => WORKFLOW_AGENT_DEFAULT_DEPTH,
    };
    Ok((path, needle, depth.min(WORKFLOW_AGENT_MAX_DEPTH)))
}

fn parse_tree_pair_command(rest: &str) -> Result<(&str, &str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let source = parts.next().ok_or(2)?;
    let destination = parts.next().ok_or(2)?;
    if source.is_empty() || destination.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => WORKFLOW_AGENT_DEFAULT_DEPTH,
    };
    Ok((source, destination, depth.min(WORKFLOW_AGENT_MAX_DEPTH)))
}

fn touch_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    if runtime.stat_path(&resolved).is_err() {
        shell_mkfile_path(runtime, &resolved)?;
    }
    write_line(runtime, &format!("touch-file-ok path={resolved}"))
}

fn truncate_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(cwd, path);
    shell_write_file(runtime, &resolved, "")?;
    write_line(runtime, &format!("truncate-file-ok path={resolved}"))
}

fn move_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    from: &str,
    to: &str,
) -> Result<(), ExitCode> {
    let resolved_from = resolve_shell_path(cwd, from);
    let resolved_to = resolve_shell_path(cwd, to);
    runtime
        .rename_path(&resolved_from, &resolved_to)
        .map_err(|_| 205)?;
    write_line(
        runtime,
        &format!("move-path-ok from={resolved_from} to={resolved_to}"),
    )
}

fn grep_tree<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    needle: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let mut visited = 0usize;
    let mut matches = 0usize;
    walk_tree_grep(runtime, &root, needle, depth, 0, &mut visited, &mut matches)?;
    write_line(
        runtime,
        &format!(
            "grep-tree-summary path={root} needle={needle} depth={depth} visited={visited} matches={matches}"
        ),
    )
}

fn walk_tree_grep<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
    max_depth: usize,
    depth: usize,
    visited: &mut usize,
    matches: &mut usize,
) -> Result<(), ExitCode> {
    let status = runtime.lstat_path(path).map_err(|_| 231)?;
    *visited += 1;
    match NativeObjectKind::from_raw(status.kind) {
        Some(NativeObjectKind::Directory) if depth < max_depth => {
            for entry in list_workflow_entries(runtime, path)? {
                let child = join_workflow_path(path, &entry);
                walk_tree_grep(
                    runtime,
                    &child,
                    needle,
                    max_depth,
                    depth + 1,
                    visited,
                    matches,
                )?;
            }
        }
        Some(NativeObjectKind::File) => {
            let text = shell_read_file_text(runtime, path)?;
            for (index, line) in text.lines().enumerate() {
                if line.contains(needle) {
                    *matches += 1;
                    write_line(
                        runtime,
                        &format!("grep-match {}:{} {}", path, index + 1, line),
                    )?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn copy_tree<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    source: &str,
    destination: &str,
    depth: usize,
    overwrite_files: bool,
) -> Result<(), ExitCode> {
    let resolved_source = resolve_shell_path(cwd, source);
    let resolved_destination = resolve_shell_path(cwd, destination);
    if resolved_source == resolved_destination
        || resolved_destination.starts_with(&(resolved_source.clone() + "/"))
    {
        return Err(205);
    }
    let mut copied = 0usize;
    let mut symlinks = 0usize;
    let mut pruned = 0usize;
    copy_tree_inner(
        runtime,
        &resolved_source,
        &resolved_destination,
        depth,
        0,
        overwrite_files,
        &mut copied,
        &mut symlinks,
        &mut pruned,
    )?;
    let _ = (
        runtime,
        resolved_source,
        resolved_destination,
        depth,
        copied,
        symlinks,
        pruned,
    );
    Ok(())
}

fn copy_tree_inner<B: SyscallBackend>(
    runtime: &Runtime<B>,
    source: &str,
    destination: &str,
    max_depth: usize,
    depth: usize,
    overwrite_files: bool,
    copied: &mut usize,
    symlinks: &mut usize,
    pruned: &mut usize,
) -> Result<(), ExitCode> {
    let status = runtime.lstat_path(source).map_err(|_| 231)?;
    match NativeObjectKind::from_raw(status.kind) {
        Some(NativeObjectKind::Directory) => {
            ensure_destination_kind(runtime, destination, NativeObjectKind::Directory)?;
            if runtime.stat_path(destination).is_err() {
                shell_mkdir_path(runtime, destination)?;
            }
            if depth >= max_depth {
                return Ok(());
            }
            if overwrite_files {
                prune_mirror_destination(runtime, source, destination, max_depth, depth, pruned)?;
            }
            for entry in list_workflow_entries(runtime, source)? {
                let child_source = join_workflow_path(source, &entry);
                let child_destination = join_workflow_path(destination, &entry);
                copy_tree_inner(
                    runtime,
                    &child_source,
                    &child_destination,
                    max_depth,
                    depth + 1,
                    overwrite_files,
                    copied,
                    symlinks,
                    pruned,
                )?;
            }
        }
        Some(NativeObjectKind::File) => {
            ensure_destination_kind(runtime, destination, NativeObjectKind::File)?;
            if overwrite_files || runtime.stat_path(destination).is_err() {
                let text = shell_read_file_text(runtime, source)?;
                shell_write_file(runtime, destination, &text)?;
                *copied += 1;
                let _ = (runtime, source, destination);
            }
        }
        Some(NativeObjectKind::Symlink) => {
            ensure_destination_kind(runtime, destination, NativeObjectKind::Symlink)?;
            if overwrite_files || runtime.lstat_path(destination).is_err() {
                let mut target = vec![0u8; 256];
                let count = runtime
                    .readlink_path(source, &mut target)
                    .map_err(|_| 235)?;
                let link_target = core::str::from_utf8(&target[..count]).map_err(|_| 247)?;
                if runtime.lstat_path(destination).is_err() {
                    runtime
                        .symlink_path(destination, link_target)
                        .map_err(|_| 205)?;
                } else {
                    runtime.unlink_path(destination).map_err(|_| 245)?;
                    runtime
                        .symlink_path(destination, link_target)
                        .map_err(|_| 205)?;
                }
                *symlinks += 1;
                let _ = (runtime, source, destination, link_target);
            }
        }
        _ => {}
    }
    Ok(())
}

fn ensure_destination_kind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    destination: &str,
    source_kind: NativeObjectKind,
) -> Result<(), ExitCode> {
    let destination_status = match runtime.lstat_path(destination) {
        Ok(status) => status,
        Err(_) => return Ok(()),
    };
    let Some(destination_kind) = NativeObjectKind::from_raw(destination_status.kind) else {
        return Ok(());
    };
    if destination_kind == source_kind {
        return Ok(());
    }
    remove_tree_path(runtime, destination)?;
    Ok(())
}

fn prune_mirror_destination<B: SyscallBackend>(
    runtime: &Runtime<B>,
    source: &str,
    destination: &str,
    max_depth: usize,
    depth: usize,
    pruned: &mut usize,
) -> Result<(), ExitCode> {
    if depth >= max_depth {
        return Ok(());
    }
    let source_entries = list_workflow_entries(runtime, source)?;
    let destination_entries = list_workflow_entries(runtime, destination)?;
    for entry in destination_entries {
        if source_entries.iter().any(|candidate| candidate == &entry) {
            continue;
        }
        let stale_path = join_workflow_path(destination, &entry);
        remove_tree_path(runtime, &stale_path)?;
        *pruned += 1;
        let _ = (runtime, stale_path);
    }
    Ok(())
}

fn remove_tree_path<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    let status = runtime.lstat_path(path).map_err(|_| 232)?;
    if NativeObjectKind::from_raw(status.kind) == Some(NativeObjectKind::Directory) {
        for entry in list_workflow_entries(runtime, path)? {
            let child = join_workflow_path(path, &entry);
            remove_tree_path(runtime, &child)?;
        }
    }
    runtime.unlink_path(path).map_err(|_| 245)
}

fn list_workflow_entries<B: SyscallBackend>(
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

fn join_workflow_path(base: &str, leaf: &str) -> String {
    if base == "/" {
        format!("/{leaf}")
    } else {
        format!("{base}/{leaf}")
    }
}

pub fn try_handle_workflow_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match WorkflowAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("touch-file") {
                "usage: touch-file <path>"
            } else if line.starts_with("truncate-file") {
                "usage: truncate-file <path>"
            } else if line.starts_with("move-path") {
                "usage: move-path <from> <to>"
            } else if line.starts_with("grep-tree") {
                "usage: grep-tree <path> <needle> [depth]"
            } else if line.starts_with("copy-tree") {
                "usage: copy-tree <source> <destination> [depth]"
            } else {
                "usage: mirror-tree <source> <destination> [depth]"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
