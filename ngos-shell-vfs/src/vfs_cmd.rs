//! VFS mutation and inspection command dispatcher.

use ngos_shell_types::resolve_shell_path;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::io::write_line;
use crate::ops::{
    shell_append_file, shell_assert_file_contains, shell_cat_file, shell_chmod_path,
    shell_chown_path, shell_compare_files, shell_copy_file, shell_grep_file, shell_link_path,
    shell_list_path, shell_mkdir_path, shell_mkfile_path, shell_mksock_path, shell_open_path,
    shell_readlink_path, shell_rename_path, shell_render_stat_path, shell_render_statfs_path,
    shell_symlink_path, shell_truncate_file, shell_unlink_path, shell_write_file,
};

enum VfsAgentCommand<'a> {
    StatPath {
        path: &'a str,
        follow_symlink: bool,
    },
    StatFsPath {
        path: &'a str,
    },
    OpenPath {
        path: &'a str,
    },
    ReadlinkPath {
        path: &'a str,
    },
    CatFile {
        path: &'a str,
    },
    WriteFile {
        path: &'a str,
        text: &'a str,
    },
    AppendFile {
        path: &'a str,
        text: &'a str,
    },
    TruncateFile {
        path: &'a str,
        len: usize,
    },
    CopyFile {
        source: &'a str,
        destination: &'a str,
    },
    CompareFile {
        left: &'a str,
        right: &'a str,
    },
    AssertFileContains {
        path: &'a str,
        needle: &'a str,
    },
    GrepFile {
        path: &'a str,
        needle: &'a str,
    },
    MkdirPath {
        path: &'a str,
    },
    MkfilePath {
        path: &'a str,
    },
    MksockPath {
        path: &'a str,
    },
    SymlinkPath {
        path: &'a str,
        target: &'a str,
    },
    LinkPath {
        source: &'a str,
        destination: &'a str,
    },
    ChmodPath {
        path: &'a str,
        mode: &'a str,
    },
    ChownPath {
        path: &'a str,
        owner: &'a str,
        group: &'a str,
    },
    RenamePath {
        from: &'a str,
        to: &'a str,
    },
    UnlinkPath {
        path: &'a str,
    },
    ListPath {
        path: &'a str,
    },
}

impl<'a> VfsAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(path) = line.strip_prefix("stat-path ") {
            return Some(parse_single_path(path).map(|path| Self::StatPath {
                path,
                follow_symlink: true,
            }));
        }
        if let Some(path) = line.strip_prefix("lstat-path ") {
            return Some(parse_single_path(path).map(|path| Self::StatPath {
                path,
                follow_symlink: false,
            }));
        }
        if let Some(path) = line.strip_prefix("statfs-path ") {
            return Some(parse_single_path(path).map(|path| Self::StatFsPath { path }));
        }
        if let Some(path) = line.strip_prefix("open-path ") {
            return Some(parse_single_path(path).map(|path| Self::OpenPath { path }));
        }
        if let Some(path) = line.strip_prefix("readlink-path ") {
            return Some(parse_single_path(path).map(|path| Self::ReadlinkPath { path }));
        }
        if let Some(path) = line.strip_prefix("cat-file ") {
            return Some(parse_single_path(path).map(|path| Self::CatFile { path }));
        }
        if let Some(rest) = line.strip_prefix("write-file ") {
            return Some(parse_path_text(rest).map(|(path, text)| Self::WriteFile { path, text }));
        }
        if let Some(rest) = line.strip_prefix("append-file ") {
            return Some(parse_path_text(rest).map(|(path, text)| Self::AppendFile { path, text }));
        }
        if let Some(rest) = line.strip_prefix("truncate-file ") {
            return Some(
                parse_path_usize(rest).map(|(path, len)| Self::TruncateFile { path, len }),
            );
        }
        if let Some(rest) = line.strip_prefix("copy-file ") {
            return Some(
                parse_two_paths(rest).map(|(source, destination)| Self::CopyFile {
                    source,
                    destination,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("cmp-file ") {
            return Some(
                parse_two_paths(rest).map(|(left, right)| Self::CompareFile { left, right }),
            );
        }
        if let Some(rest) = line.strip_prefix("assert-file-contains ") {
            return Some(
                parse_path_text(rest)
                    .map(|(path, needle)| Self::AssertFileContains { path, needle }),
            );
        }
        if let Some(rest) = line.strip_prefix("grep-file ") {
            return Some(
                parse_path_text(rest).map(|(path, needle)| Self::GrepFile { path, needle }),
            );
        }
        if let Some(path) = line.strip_prefix("mkdir-path ") {
            return Some(parse_single_path(path).map(|path| Self::MkdirPath { path }));
        }
        if let Some(path) = line.strip_prefix("mkfile-path ") {
            return Some(parse_single_path(path).map(|path| Self::MkfilePath { path }));
        }
        if let Some(path) = line.strip_prefix("mksock-path ") {
            return Some(parse_single_path(path).map(|path| Self::MksockPath { path }));
        }
        if let Some(rest) = line.strip_prefix("symlink-path ") {
            return Some(
                parse_two_paths(rest).map(|(path, target)| Self::SymlinkPath { path, target }),
            );
        }
        if let Some(rest) = line.strip_prefix("link-path ") {
            return Some(
                parse_two_paths(rest).map(|(source, destination)| Self::LinkPath {
                    source,
                    destination,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("chmod-path ") {
            return Some(parse_path_text(rest).map(|(path, mode)| Self::ChmodPath { path, mode }));
        }
        if let Some(rest) = line.strip_prefix("chown-path ") {
            return Some(
                parse_three_tokens(rest).map(|(path, owner, group)| Self::ChownPath {
                    path,
                    owner,
                    group,
                }),
            );
        }
        if let Some(rest) = line.strip_prefix("rename-path ") {
            return Some(parse_two_paths(rest).map(|(from, to)| Self::RenamePath { from, to }));
        }
        if let Some(path) = line.strip_prefix("unlink-path ") {
            return Some(parse_single_path(path).map(|path| Self::UnlinkPath { path }));
        }
        if let Some(path) = line.strip_prefix("list-path ") {
            return Some(parse_single_path(path).map(|path| Self::ListPath { path }));
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::StatPath {
                path,
                follow_symlink,
            } => shell_render_stat_path(runtime, &resolve_shell_path(cwd, path), follow_symlink),
            Self::StatFsPath { path } => {
                shell_render_statfs_path(runtime, &resolve_shell_path(cwd, path))
            }
            Self::OpenPath { path } => shell_open_path(runtime, &resolve_shell_path(cwd, path)),
            Self::ReadlinkPath { path } => {
                shell_readlink_path(runtime, &resolve_shell_path(cwd, path))
            }
            Self::CatFile { path } => shell_cat_file(runtime, &resolve_shell_path(cwd, path)),
            Self::WriteFile { path, text } => {
                shell_write_file(runtime, &resolve_shell_path(cwd, path), text)
            }
            Self::AppendFile { path, text } => {
                shell_append_file(runtime, &resolve_shell_path(cwd, path), text)
            }
            Self::TruncateFile { path, len } => {
                shell_truncate_file(runtime, &resolve_shell_path(cwd, path), len)
            }
            Self::CopyFile {
                source,
                destination,
            } => shell_copy_file(
                runtime,
                &resolve_shell_path(cwd, source),
                &resolve_shell_path(cwd, destination),
            ),
            Self::CompareFile { left, right } => shell_compare_files(
                runtime,
                &resolve_shell_path(cwd, left),
                &resolve_shell_path(cwd, right),
            ),
            Self::AssertFileContains { path, needle } => {
                shell_assert_file_contains(runtime, &resolve_shell_path(cwd, path), needle)
            }
            Self::GrepFile { path, needle } => {
                shell_grep_file(runtime, &resolve_shell_path(cwd, path), needle)
            }
            Self::MkdirPath { path } => shell_mkdir_path(runtime, &resolve_shell_path(cwd, path)),
            Self::MkfilePath { path } => shell_mkfile_path(runtime, &resolve_shell_path(cwd, path)),
            Self::MksockPath { path } => shell_mksock_path(runtime, &resolve_shell_path(cwd, path)),
            Self::SymlinkPath { path, target } => {
                shell_symlink_path(
                    runtime,
                    &resolve_shell_path(cwd, path),
                    &resolve_shell_path(cwd, target),
                )
            }
            Self::LinkPath {
                source,
                destination,
            } => shell_link_path(
                runtime,
                &resolve_shell_path(cwd, source),
                &resolve_shell_path(cwd, destination),
            ),
            Self::ChmodPath { path, mode } => {
                shell_chmod_path(runtime, &resolve_shell_path(cwd, path), mode)
            }
            Self::ChownPath { path, owner, group } => {
                shell_chown_path(runtime, &resolve_shell_path(cwd, path), owner, group)
            }
            Self::RenamePath { from, to } => shell_rename_path(
                runtime,
                &resolve_shell_path(cwd, from),
                &resolve_shell_path(cwd, to),
            ),
            Self::UnlinkPath { path } => shell_unlink_path(runtime, &resolve_shell_path(cwd, path)),
            Self::ListPath { path } => shell_list_path(runtime, &resolve_shell_path(cwd, path)),
        }
    }

    fn usage(line: &str) -> &'static str {
        if line.starts_with("stat-path ") {
            "usage: stat-path <path>"
        } else if line.starts_with("lstat-path ") {
            "usage: lstat-path <path>"
        } else if line.starts_with("statfs-path ") {
            "usage: statfs-path <path>"
        } else if line.starts_with("open-path ") {
            "usage: open-path <path>"
        } else if line.starts_with("readlink-path ") {
            "usage: readlink-path <path>"
        } else if line.starts_with("cat-file ") {
            "usage: cat-file <path>"
        } else if line.starts_with("write-file ") {
            "usage: write-file <path> <text>"
        } else if line.starts_with("append-file ") {
            "usage: append-file <path> <text>"
        } else if line.starts_with("truncate-file ") {
            "usage: truncate-file <path> <size>"
        } else if line.starts_with("copy-file ") {
            "usage: copy-file <source> <destination>"
        } else if line.starts_with("cmp-file ") {
            "usage: cmp-file <left> <right>"
        } else if line.starts_with("assert-file-contains ") {
            "usage: assert-file-contains <path> <text>"
        } else if line.starts_with("grep-file ") {
            "usage: grep-file <path> <text>"
        } else if line.starts_with("mkdir-path ") {
            "usage: mkdir-path <path>"
        } else if line.starts_with("mkfile-path ") {
            "usage: mkfile-path <path>"
        } else if line.starts_with("mksock-path ") {
            "usage: mksock-path <path>"
        } else if line.starts_with("symlink-path ") {
            "usage: symlink-path <path> <target>"
        } else if line.starts_with("link-path ") {
            "usage: link-path <source> <destination>"
        } else if line.starts_with("chmod-path ") {
            "usage: chmod-path <path> <mode-octal>"
        } else if line.starts_with("chown-path ") {
            "usage: chown-path <path> <owner> <group>"
        } else if line.starts_with("rename-path ") {
            "usage: rename-path <from> <to>"
        } else if line.starts_with("unlink-path ") {
            "usage: unlink-path <path>"
        } else {
            "usage: list-path <path>"
        }
    }
}

fn parse_single_path(path: &str) -> Result<&str, ExitCode> {
    let path = path.trim();
    if path.is_empty() || path.contains(char::is_whitespace) {
        Err(2)
    } else {
        Ok(path)
    }
}

fn parse_path_text(rest: &str) -> Result<(&str, &str), ExitCode> {
    let mut parts = rest.splitn(2, char::is_whitespace);
    let path = parts.next().ok_or(2)?;
    let text = parts.next().map(str::trim_start).ok_or(2)?;
    if path.is_empty() || text.is_empty() {
        Err(2)
    } else {
        Ok((path, text))
    }
}

fn parse_path_usize(rest: &str) -> Result<(&str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    let len = parts.next().ok_or(2)?.parse::<usize>().map_err(|_| 2)?;
    if path.is_empty() || parts.next().is_some() {
        Err(2)
    } else {
        Ok((path, len))
    }
}

fn parse_two_paths(rest: &str) -> Result<(&str, &str), ExitCode> {
    let mut parts = rest.split_whitespace();
    let first = parts.next().ok_or(2)?;
    let second = parts.next().ok_or(2)?;
    if first.is_empty() || second.is_empty() || parts.next().is_some() {
        Err(2)
    } else {
        Ok((first, second))
    }
}

fn parse_three_tokens(rest: &str) -> Result<(&str, &str, &str), ExitCode> {
    let mut parts = rest.split_whitespace();
    let a = parts.next().ok_or(2)?;
    let b = parts.next().ok_or(2)?;
    let c = parts.next().ok_or(2)?;
    if a.is_empty() || b.is_empty() || c.is_empty() || parts.next().is_some() {
        Err(2)
    } else {
        Ok((a, b, c))
    }
}

pub fn try_handle_vfs_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match VfsAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let _ = write_line(runtime, VfsAgentCommand::usage(line));
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
