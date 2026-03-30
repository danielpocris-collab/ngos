use super::*;

pub(super) fn try_handle_shell_state_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
    shell_mode: &mut ShellMode,
    semantic_learning: &SemanticFeedbackStore,
    previous_status: i32,
    pending_lines: &mut Vec<String>,
    line_index: usize,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("mode ") {
        *shell_mode = match rest.trim() {
            "direct" => ShellMode::Direct,
            "semantic" => ShellMode::Semantic,
            _ => {
                let _ = write_line(runtime, "usage: mode <direct|semantic>");
                return Some(Err(2));
            }
        };
        return Some(
            write_line(
                runtime,
                &format!(
                    "mode={}",
                    match shell_mode {
                        ShellMode::Direct => "direct",
                        ShellMode::Semantic => "semantic",
                    }
                ),
            )
            .map_err(|_| 195),
        );
    }
    if line == "observe" || line == "observe system" || line == "observe facts" {
        *last_status = match shell_render_semantic_facts(runtime) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if line == "learn" {
        if semantic_learning.entries().is_empty() {
            return Some(write_line(runtime, "learn entries=0").map_err(|_| 195));
        }
        for entry in semantic_learning.entries() {
            if write_line(
                runtime,
                &format!(
                    "learn subject={} action={} policy-epoch={} success={} failure={}",
                    entry.subject,
                    entry.action,
                    entry.policy_epoch,
                    entry.success_count,
                    entry.failure_count
                ),
            )
            .is_err()
            {
                return Some(Err(195));
            }
        }
        return Some(Ok(()));
    }
    if line == "last-status" {
        return Some(
            write_line(runtime, &format!("last-status={previous_status}")).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("fcntl-getfl ") {
        let Some(fd) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: fcntl-getfl <fd>");
            return Some(Err(2));
        };
        *last_status = match shell_get_fd_status_flags(runtime, fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("fcntl-getfd ") {
        let Some(fd) = parse_usize_arg(Some(rest.trim())) else {
            let _ = write_line(runtime, "usage: fcntl-getfd <fd>");
            return Some(Err(2));
        };
        *last_status = match shell_get_fd_descriptor_flags(runtime, fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("nonblock-fd ") {
        let mut parts = rest.split_whitespace();
        let Some(fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: nonblock-fd <fd> <on|off>");
            return Some(Err(2));
        };
        let nonblock = match parts.next() {
            Some("on") => true,
            Some("off") => false,
            _ => {
                let _ = write_line(runtime, "usage: nonblock-fd <fd> <on|off>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_set_fd_nonblock(runtime, fd, nonblock) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("cloexec-fd ") {
        let mut parts = rest.split_whitespace();
        let Some(fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(runtime, "usage: cloexec-fd <fd> <on|off>");
            return Some(Err(2));
        };
        let cloexec = match parts.next() {
            Some("on") => true,
            Some("off") => false,
            _ => {
                let _ = write_line(runtime, "usage: cloexec-fd <fd> <on|off>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_set_fd_cloexec(runtime, fd, cloexec) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if line == "true" {
        return Some(Ok(()));
    }
    if line == "false" {
        *last_status = 1;
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("repeat ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let count = match parts.next().and_then(|value| value.parse::<usize>().ok()) {
            Some(count) => count,
            None => {
                let _ = write_line(runtime, "usage: repeat <count> <command>");
                return Some(Err(2));
            }
        };
        let body = match parts.next().map(str::trim_start) {
            Some(body) if !body.is_empty() => body,
            _ => {
                let _ = write_line(runtime, "usage: repeat <count> <command>");
                return Some(Err(2));
            }
        };
        let repeated = (0..count).map(|_| body.to_string()).collect::<Vec<_>>();
        pending_lines.splice(line_index..line_index, repeated);
        return Some(
            write_line(runtime, &format!("repeat-expanded count={count}")).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("assert-status ") {
        let expected = match rest.trim().parse::<i32>().ok() {
            Some(expected) => expected,
            None => {
                let _ = write_line(runtime, "usage: assert-status <code>");
                return Some(Err(2));
            }
        };
        if previous_status != expected {
            *last_status = 1;
            return Some(
                write_line(
                    runtime,
                    &format!("assert-status-failed expected={expected} actual={previous_status}"),
                )
                .map_err(|_| 196),
            );
        }
        return Some(
            write_line(runtime, &format!("assert-status-ok expected={expected}")).map_err(|_| 196),
        );
    }
    if let Some(rest) = line.strip_prefix("assert-file-contains ") {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let path = match parts.next() {
            Some(path) if !path.is_empty() => path,
            _ => {
                let _ = write_line(runtime, "usage: assert-file-contains <path> <text>");
                return Some(Err(2));
            }
        };
        let needle = match parts.next().map(str::trim_start) {
            Some(needle) if !needle.is_empty() => needle,
            _ => {
                let _ = write_line(runtime, "usage: assert-file-contains <path> <text>");
                return Some(Err(2));
            }
        };
        let resolved = resolve_shell_path(cwd, path);
        if shell_assert_file_contains(runtime, &resolved, needle).is_err() {
            *last_status = 1;
            return Some(
                write_line(
                    runtime,
                    &format!("assert-file-contains-failed path={resolved} needle={needle}"),
                )
                .map_err(|_| 205),
            );
        }
        return Some(Ok(()));
    }
    None
}
