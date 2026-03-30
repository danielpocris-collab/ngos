use super::*;

enum FdAgentCommand {
    Duplicate { fd: usize },
    Close { fd: usize },
    GetStatusFlags { fd: usize },
    GetDescriptorFlags { fd: usize },
    SetNonblock { fd: usize, enabled: bool },
    SetCloexec { fd: usize, enabled: bool },
}

impl FdAgentCommand {
    fn parse(line: &str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("dup-fd ") {
            return Some(
                parse_usize_arg(Some(rest.trim()))
                    .map(|fd| Self::Duplicate { fd })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("close-fd ") {
            return Some(
                parse_usize_arg(Some(rest.trim()))
                    .map(|fd| Self::Close { fd })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("fcntl-getfl ") {
            return Some(
                parse_usize_arg(Some(rest.trim()))
                    .map(|fd| Self::GetStatusFlags { fd })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("fcntl-getfd ") {
            return Some(
                parse_usize_arg(Some(rest.trim()))
                    .map(|fd| Self::GetDescriptorFlags { fd })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("nonblock-fd ") {
            return Some(
                parse_toggle_command(rest).map(|(fd, enabled)| Self::SetNonblock { fd, enabled }),
            );
        }
        if let Some(rest) = line.strip_prefix("cloexec-fd ") {
            return Some(
                parse_toggle_command(rest).map(|(fd, enabled)| Self::SetCloexec { fd, enabled }),
            );
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>) -> Result<(), ExitCode> {
        match *self {
            Self::Duplicate { fd } => {
                let duplicated = runtime.dup(fd).map_err(|_| 234)?;
                write_line(runtime, &format!("fd-duplicated from={fd} to={duplicated}"))
            }
            Self::Close { fd } => {
                runtime.close(fd).map_err(|_| 240)?;
                write_line(runtime, &format!("fd-closed fd={fd}"))
            }
            Self::GetStatusFlags { fd } => {
                let flags = runtime.fcntl(fd, FcntlCmd::GetFl).map_err(|_| 234)?;
                write_line(runtime, &format!("fcntl-getfl fd={fd} flags=0x{flags:x}"))
            }
            Self::GetDescriptorFlags { fd } => {
                let flags = runtime.fcntl(fd, FcntlCmd::GetFd).map_err(|_| 234)?;
                write_line(runtime, &format!("fcntl-getfd fd={fd} flags=0x{flags:x}"))
            }
            Self::SetNonblock { fd, enabled } => {
                let flags = runtime
                    .fcntl(fd, FcntlCmd::SetFl { nonblock: enabled })
                    .map_err(|_| 234)?;
                write_line(
                    runtime,
                    &format!(
                        "fd-nonblock fd={fd} state={} flags=0x{flags:x}",
                        if enabled { "on" } else { "off" }
                    ),
                )
            }
            Self::SetCloexec { fd, enabled } => {
                let flags = runtime
                    .fcntl(fd, FcntlCmd::SetFd { cloexec: enabled })
                    .map_err(|_| 234)?;
                write_line(
                    runtime,
                    &format!(
                        "fd-cloexec fd={fd} state={} flags=0x{flags:x}",
                        if enabled { "on" } else { "off" }
                    ),
                )
            }
        }
    }
}

fn parse_toggle_command(rest: &str) -> Result<(usize, bool), ExitCode> {
    let mut parts = rest.split_whitespace();
    let fd = parse_usize_arg(parts.next()).ok_or(2)?;
    let enabled = match parts.next() {
        Some("on") => true,
        Some("off") => false,
        _ => return Err(2),
    };
    Ok((fd, enabled))
}

pub(super) fn try_handle_fd_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match FdAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("dup-fd ") {
                "usage: dup-fd <fd>"
            } else if line.starts_with("close-fd ") {
                "usage: close-fd <fd>"
            } else if line.starts_with("fcntl-getfl ") {
                "usage: fcntl-getfl <fd>"
            } else if line.starts_with("fcntl-getfd ") {
                "usage: fcntl-getfd <fd>"
            } else if line.starts_with("nonblock-fd ") {
                "usage: nonblock-fd <fd> <on|off>"
            } else {
                "usage: cloexec-fd <fd> <on|off>"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime))
}
