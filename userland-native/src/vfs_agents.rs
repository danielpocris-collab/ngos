use super::*;

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
    CopyFile {
        source: &'a str,
        destination: &'a str,
    },
    CompareFile {
        left: &'a str,
        right: &'a str,
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
            return Some(
                parse_path_text_command(rest).map(|(path, text)| Self::WriteFile { path, text }),
            );
        }
        if let Some(rest) = line.strip_prefix("append-file ") {
            return Some(
                parse_path_text_command(rest).map(|(path, text)| Self::AppendFile { path, text }),
            );
        }
        if let Some(rest) = line.strip_prefix("copy-file ") {
            return Some(parse_two_path_command(rest).map(|(source, destination)| {
                Self::CopyFile {
                    source,
                    destination,
                }
            }));
        }
        if let Some(rest) = line.strip_prefix("cmp-file ") {
            return Some(
                parse_two_path_command(rest).map(|(left, right)| Self::CompareFile { left, right }),
            );
        }
        if let Some(rest) = line.strip_prefix("grep-file ") {
            return Some(
                parse_path_text_command(rest).map(|(path, needle)| Self::GrepFile { path, needle }),
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
                parse_two_path_command(rest)
                    .map(|(path, target)| Self::SymlinkPath { path, target }),
            );
        }
        if let Some(rest) = line.strip_prefix("rename-path ") {
            return Some(
                parse_two_path_command(rest).map(|(from, to)| Self::RenamePath { from, to }),
            );
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
            Self::GrepFile { path, needle } => {
                shell_grep_file(runtime, &resolve_shell_path(cwd, path), needle)
            }
            Self::MkdirPath { path } => shell_mkdir_path(runtime, &resolve_shell_path(cwd, path)),
            Self::MkfilePath { path } => shell_mkfile_path(runtime, &resolve_shell_path(cwd, path)),
            Self::MksockPath { path } => shell_mksock_path(runtime, &resolve_shell_path(cwd, path)),
            Self::SymlinkPath { path, target } => shell_symlink_path(
                runtime,
                &resolve_shell_path(cwd, path),
                &resolve_shell_path(cwd, target),
            ),
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
        } else if line.starts_with("copy-file ") {
            "usage: copy-file <source> <destination>"
        } else if line.starts_with("cmp-file ") {
            "usage: cmp-file <left> <right>"
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
    if path.is_empty() {
        return Err(2);
    }
    Ok(path)
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

fn parse_two_path_command(rest: &str) -> Result<(&str, &str), ExitCode> {
    let mut parts = rest.split_whitespace();
    let first = parts.next().ok_or(2)?;
    let second = parts.next().ok_or(2)?;
    if first.is_empty() || second.is_empty() {
        return Err(2);
    }
    Ok((first, second))
}

pub(super) fn try_handle_vfs_agent_command<B: SyscallBackend>(
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
