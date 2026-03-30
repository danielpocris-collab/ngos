use super::*;

pub(super) fn try_handle_intent_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    shell_mode: ShellMode,
    line: &str,
    last_status: &mut i32,
    semantic_learning: &mut SemanticFeedbackStore,
    nextmind_entity_epochs: &[SemanticEntityEpoch],
) -> Option<Result<(), ExitCode>> {
    let rest = line.strip_prefix("intent ")?;
    let mut parts = rest.split_whitespace();
    let kind = parts.next().unwrap_or("");
    let subject = parts.next().unwrap_or("");
    match (kind, subject) {
        ("optimize", "process") => {
            let Some(pid) = parse_u64_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent optimize process <pid>");
                return Some(Err(2));
            };
            *last_status = match shell_semantic_process_action(
                runtime,
                pid,
                ProcessAction::Renice {
                    class: NativeSchedulerClass::LatencyCritical,
                    budget: 4,
                },
            ) {
                Ok(()) => 0,
                Err(code) => code,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("process:{pid}"),
                "optimize",
                *last_status == 0,
            );
        }
        ("throttle", "process") => {
            let Some(pid) = parse_u64_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent throttle process <pid>");
                return Some(Err(2));
            };
            *last_status = match shell_semantic_process_action(
                runtime,
                pid,
                ProcessAction::Renice {
                    class: NativeSchedulerClass::Background,
                    budget: 1,
                },
            ) {
                Ok(()) => 0,
                Err(code) => code,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("process:{pid}"),
                "throttle",
                *last_status == 0,
            );
        }
        ("restart", "process") => {
            let Some(pid) = parse_u64_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent restart process <pid>");
                return Some(Err(2));
            };
            *last_status = match shell_semantic_process_action(
                runtime,
                pid,
                ProcessAction::Kill { signal: 9 },
            ) {
                Ok(()) => 0,
                Err(code) => code,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("process:{pid}"),
                "restart",
                *last_status == 0,
            );
        }
        ("activate", "resource") => {
            let Some(contract) = parse_usize_arg(parts.next()) else {
                let _ = write_line(runtime, "usage: intent activate resource <contract>");
                return Some(Err(2));
            };
            *last_status =
                match shell_semantic_resource_update(runtime, contract, ResourceUpdate::Activate) {
                    Ok(()) => 0,
                    Err(code) => code,
                };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("resource:{contract}"),
                "activate",
                *last_status == 0,
            );
        }
        ("stabilize", "network") => {
            let Some(path) = parts.next() else {
                let _ = write_line(runtime, "usage: intent stabilize network <device>");
                return Some(Err(2));
            };
            let device = DeviceHandle {
                path: resolve_shell_path(cwd, path),
            };
            let controller = SystemController::new(runtime);
            *last_status = match controller.device_stats(&device) {
                Ok(stats) => match stats.record {
                    Some(record) => match controller.configure_interface_admin(
                        &device,
                        record.mtu as usize,
                        record.tx_capacity.max(record.tx_inflight_limit + 1) as usize,
                        record.rx_capacity as usize,
                        record.tx_inflight_limit.max(2) as usize,
                        true,
                        false,
                    ) {
                        Ok(()) => 0,
                        Err(_) => 265,
                    },
                    None => 265,
                },
                Err(_) => 265,
            };
            shell_record_learning(
                semantic_learning,
                nextmind_entity_epochs,
                &format!("device:{}", device.path),
                "stabilize",
                *last_status == 0,
            );
        }
        _ if matches!(shell_mode, ShellMode::Semantic) => {
            let _ = write_line(
                runtime,
                "usage: intent <optimize|throttle|restart|activate|stabilize> <process|resource|network> ...",
            );
            return Some(Err(2));
        }
        _ => {}
    }
    Some(Ok(()))
}
