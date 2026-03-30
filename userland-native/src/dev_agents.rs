use super::*;

enum DevAgentCommand<'a> {
    HeadFile { path: &'a str, lines: usize },
    TailFile { path: &'a str, lines: usize },
    WcFile { path: &'a str },
    HexFile { path: &'a str, limit: usize },
}

impl<'a> DevAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("head-file ") {
            return Some(
                parse_path_with_optional_count(rest, 10)
                    .map(|(path, lines)| Self::HeadFile { path, lines }),
            );
        }
        if let Some(rest) = line.strip_prefix("tail-file ") {
            return Some(
                parse_path_with_optional_count(rest, 10)
                    .map(|(path, lines)| Self::TailFile { path, lines }),
            );
        }
        if let Some(rest) = line.strip_prefix("wc-file ") {
            let path = rest.trim();
            return Some((!path.is_empty()).then_some(Self::WcFile { path }).ok_or(2));
        }
        if let Some(rest) = line.strip_prefix("hex-file ") {
            return Some(
                parse_path_with_optional_count(rest, 64)
                    .map(|(path, limit)| Self::HexFile { path, limit }),
            );
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::HeadFile { path, lines } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let mut emitted = 0usize;
                for line in text.lines().take(lines) {
                    emitted += 1;
                    write_line(runtime, line)?;
                }
                write_line(
                    runtime,
                    &format!("head-summary path={resolved} lines={emitted}"),
                )
            }
            Self::TailFile { path, lines } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let all_lines = text.lines().collect::<Vec<_>>();
                let start = all_lines.len().saturating_sub(lines);
                let mut emitted = 0usize;
                for line in &all_lines[start..] {
                    emitted += 1;
                    write_line(runtime, line)?;
                }
                write_line(
                    runtime,
                    &format!("tail-summary path={resolved} lines={emitted}"),
                )
            }
            Self::WcFile { path } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let bytes = text.len();
                let chars = text.chars().count();
                let lines = text.lines().count();
                let words = text.split_whitespace().count();
                write_line(
                    runtime,
                    &format!(
                        "wc path={resolved} bytes={bytes} chars={chars} words={words} lines={lines}"
                    ),
                )
            }
            Self::HexFile { path, limit } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let bytes = text.as_bytes();
                let shown = bytes.len().min(limit);
                let mut offset = 0usize;
                while offset < shown {
                    let end = (offset + 16).min(shown);
                    let chunk = &bytes[offset..end];
                    let mut hex = String::new();
                    let mut ascii = String::new();
                    for byte in chunk {
                        hex.push_str(&format!("{byte:02x} "));
                        let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                            *byte as char
                        } else {
                            '.'
                        };
                        ascii.push(ch);
                    }
                    write_line(
                        runtime,
                        &format!("hex {offset:04x}: {:<48} {}", hex.trim_end(), ascii),
                    )?;
                    offset = end;
                }
                write_line(
                    runtime,
                    &format!(
                        "hex-summary path={resolved} shown={} total={}",
                        shown,
                        bytes.len()
                    ),
                )
            }
        }
    }
}

fn parse_path_with_optional_count<'a>(
    rest: &'a str,
    default_count: usize,
) -> Result<(&'a str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    if path.is_empty() {
        return Err(2);
    }
    let count = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => default_count,
    };
    Ok((path, count))
}

pub(super) fn try_handle_dev_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match DevAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("head-file ") {
                "usage: head-file <path> [lines]"
            } else if line.starts_with("tail-file ") {
                "usage: tail-file <path> [lines]"
            } else if line.starts_with("wc-file ") {
                "usage: wc-file <path>"
            } else {
                "usage: hex-file <path> [bytes]"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
