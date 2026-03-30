use super::*;

pub(super) fn try_handle_semantic_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
    last_status: &mut i32,
    semantic_learning: &mut SemanticFeedbackStore,
    nextmind_entity_epochs: &[SemanticEntityEpoch],
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("pause ") {
        let pid = match parse_u64_arg(Some(rest.trim())) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: pause <pid>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_process_action(runtime, pid, ProcessAction::Pause) {
            Ok(()) => 0,
            Err(code) => code,
        };
        shell_record_learning(
            semantic_learning,
            nextmind_entity_epochs,
            &format!("process:{pid}"),
            "pause",
            *last_status == 0,
        );
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("resume ") {
        let pid = match parse_u64_arg(Some(rest.trim())) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: resume <pid>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_process_action(runtime, pid, ProcessAction::Resume) {
            Ok(()) => 0,
            Err(code) => code,
        };
        shell_record_learning(
            semantic_learning,
            nextmind_entity_epochs,
            &format!("process:{pid}"),
            "resume",
            *last_status == 0,
        );
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("renice ") {
        let mut parts = rest.split_whitespace();
        let pid = match parse_u64_arg(parts.next()) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, "usage: renice <pid> <class> <budget>");
                return Some(Err(2));
            }
        };
        let class = match parts.next().and_then(parse_scheduler_class) {
            Some(class) => class,
            None => {
                let _ = write_line(runtime, "usage: renice <pid> <class> <budget>");
                return Some(Err(2));
            }
        };
        let budget = match parts.next().and_then(|value| value.parse::<u32>().ok()) {
            Some(budget) if budget > 0 => budget,
            _ => {
                let _ = write_line(runtime, "usage: renice <pid> <class> <budget>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_process_action(
            runtime,
            pid,
            ProcessAction::Renice { class, budget },
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        shell_record_learning(
            semantic_learning,
            nextmind_entity_epochs,
            &format!("process:{pid}"),
            "renice",
            *last_status == 0,
        );
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("semantic-watch ") {
        let mut parts = rest.split_whitespace();
        let target = parts.next().unwrap_or("");
        let result = match target {
            "process" => match parse_u64_arg(parts.next()) {
                Some(pid) => shell_semantic_watch_event(
                    runtime,
                    EventFilter::Process {
                        pid,
                        token: CapabilityToken { value: pid },
                        exited: true,
                        reaped: true,
                        poll_events: POLLPRI,
                    },
                ),
                None => Err(261),
            },
            "resource" => match parse_usize_arg(parts.next()) {
                Some(resource) => shell_semantic_watch_event(
                    runtime,
                    EventFilter::Resource {
                        resource,
                        token: CapabilityToken {
                            value: resource as u64,
                        },
                        claimed: true,
                        queued: true,
                        canceled: true,
                        released: true,
                        handed_off: true,
                        revoked: true,
                        poll_events: POLLPRI,
                    },
                ),
                None => Err(261),
            },
            "network" => match parts.next() {
                Some(path) => shell_semantic_watch_event(
                    runtime,
                    EventFilter::Network {
                        interface_path: resolve_shell_path(cwd, path),
                        socket_path: None,
                        token: CapabilityToken { value: 1 },
                        link_changed: true,
                        rx_ready: true,
                        tx_drained: true,
                        poll_events: POLLPRI,
                    },
                ),
                None => Err(261),
            },
            _ => Err(261),
        };
        match result {
            Ok(queue_fd) => {
                *last_status = 0;
                return Some(
                    write_line(
                        runtime,
                        &format!("semantic-watch fd={queue_fd} target={target}"),
                    )
                    .map_err(|_| 199),
                );
            }
            Err(code) => {
                *last_status = code;
                return Some(Ok(()));
            }
        }
    }
    if let Some(rest) = line.strip_prefix("semantic-wait ") {
        let queue_fd = match parse_usize_arg(Some(rest.trim())) {
            Some(queue_fd) => queue_fd,
            None => {
                let _ = write_line(runtime, "usage: semantic-wait <queue-fd>");
                return Some(Err(2));
            }
        };
        *last_status = match shell_semantic_wait_event(runtime, queue_fd) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    None
}
