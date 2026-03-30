use super::*;

pub(super) fn try_handle_resource_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    last_status: &mut i32,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("resource-watch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: resource-watch <queue-fd> <resource> <token> [all|claimed,queued,canceled,released,handed-off,revoked]",
            );
            return Some(Err(2));
        };
        let Some(resource) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: resource-watch <queue-fd> <resource> <token> [all|claimed,queued,canceled,released,handed-off,revoked]",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: resource-watch <queue-fd> <resource> <token> [all|claimed,queued,canceled,released,handed-off,revoked]",
            );
            return Some(Err(2));
        };
        let kinds = parts.next();
        let Some((claimed, queued, canceled, released, handed_off, revoked, kinds_label)) =
            parse_resource_watch_kinds(kinds)
        else {
            let _ = write_line(
                runtime,
                "usage: resource-watch <queue-fd> <resource> <token> [all|claimed,queued,canceled,released,handed-off,revoked]",
            );
            return Some(Err(2));
        };
        *last_status = match shell_watch_resource_events(
            runtime,
            queue_fd,
            resource,
            token,
            claimed,
            queued,
            canceled,
            released,
            handed_off,
            revoked,
            &kinds_label,
        ) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("resource-unwatch ") {
        let mut parts = rest.split_whitespace();
        let Some(queue_fd) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: resource-unwatch <queue-fd> <resource> <token>",
            );
            return Some(Err(2));
        };
        let Some(resource) = parse_usize_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: resource-unwatch <queue-fd> <resource> <token>",
            );
            return Some(Err(2));
        };
        let Some(token) = parse_u64_arg(parts.next()) else {
            let _ = write_line(
                runtime,
                "usage: resource-unwatch <queue-fd> <resource> <token>",
            );
            return Some(Err(2));
        };
        *last_status = match shell_remove_resource_watch(runtime, queue_fd, resource, token) {
            Ok(()) => 0,
            Err(code) => code,
        };
        return Some(Ok(()));
    }
    if line == "domains" {
        return Some(shell_render_domains(runtime).map_err(|_| 206));
    }
    if let Some(rest) = line.strip_prefix("domain ") {
        let id = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: domain <id>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_domain_detail(runtime, id).map_err(|_| 206));
    }
    if line == "resources" {
        return Some(shell_render_resources(runtime).map_err(|_| 207));
    }
    if let Some(rest) = line.strip_prefix("resource ") {
        let id = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: resource <id>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_resource_detail(runtime, id).map_err(|_| 207));
    }
    if let Some(rest) = line.strip_prefix("waiters ") {
        let resource = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: waiters <resource>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_waiters(runtime, resource).map_err(|_| 207));
    }
    if line == "contracts" {
        return Some(shell_render_contracts(runtime).map_err(|_| 208));
    }
    if let Some(rest) = line.strip_prefix("contract ") {
        let id = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: contract <id>");
                return Some(Err(2));
            }
        };
        return Some(shell_render_contract_detail(runtime, id).map_err(|_| 208));
    }
    if let Some(rest) = line.strip_prefix("mkdomain ") {
        let name = rest.trim();
        if name.is_empty() {
            let _ = write_line(runtime, "usage: mkdomain <name>");
            return Some(Err(2));
        }
        return Some(match runtime.create_domain(None, name) {
            Ok(id) => {
                write_line(runtime, &format!("domain-created id={id} name={name}")).map_err(|_| 197)
            }
            Err(_) => Err(206),
        });
    }
    if let Some(rest) = line.strip_prefix("mkresource ") {
        let mut parts = rest.split_whitespace();
        let domain = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: mkresource <domain> <kind> <name>");
                return Some(Err(2));
            }
        };
        let kind = match parts.next().and_then(parse_resource_kind) {
            Some(kind) => kind,
            None => {
                let _ = write_line(runtime, "usage: mkresource <domain> <kind> <name>");
                return Some(Err(2));
            }
        };
        let name = parts.collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            let _ = write_line(runtime, "usage: mkresource <domain> <kind> <name>");
            return Some(Err(2));
        }
        return Some(match runtime.create_resource(domain, kind, &name) {
            Ok(id) => write_line(
                runtime,
                &format!(
                    "resource-created id={id} domain={domain} kind={} name={name}",
                    resource_kind_name(kind as u32)
                ),
            )
            .map_err(|_| 197),
            Err(_) => Err(207),
        });
    }
    if let Some(rest) = line.strip_prefix("mkcontract ") {
        let mut parts = rest.split_whitespace();
        let domain = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: mkcontract <domain> <resource> <kind> <label>",
                );
                return Some(Err(2));
            }
        };
        let resource = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: mkcontract <domain> <resource> <kind> <label>",
                );
                return Some(Err(2));
            }
        };
        let kind = match parts.next().and_then(parse_contract_kind) {
            Some(kind) => kind,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: mkcontract <domain> <resource> <kind> <label>",
                );
                return Some(Err(2));
            }
        };
        let label = parts.collect::<Vec<_>>().join(" ");
        if label.is_empty() {
            let _ = write_line(
                runtime,
                "usage: mkcontract <domain> <resource> <kind> <label>",
            );
            return Some(Err(2));
        }
        return Some(match runtime.create_contract(domain, resource, kind, &label) {
            Ok(id) => write_line(
                runtime,
                &format!(
                    "contract-created id={id} domain={domain} resource={resource} kind={} label={label}",
                    contract_kind_name(kind as u32)
                ),
            )
            .map_err(|_| 197),
            Err(_) => Err(208),
        });
    }
    if let Some(rest) = line.strip_prefix("claim ") {
        let contract = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: claim <contract>");
                return Some(Err(2));
            }
        };
        return Some(match runtime.claim_resource(contract) {
            Ok(ResourceClaimOutcome::Acquired {
                resource,
                acquire_count,
            }) => write_line(
                runtime,
                &format!(
                    "claim-acquired contract={contract} resource={resource} acquire_count={acquire_count}"
                ),
            )
            .map_err(|_| 197),
            Ok(ResourceClaimOutcome::Queued {
                resource,
                holder_contract,
                position,
            }) => write_line(
                runtime,
                &format!(
                    "claim-queued contract={contract} resource={resource} holder={holder_contract} position={position}"
                ),
            )
            .map_err(|_| 197),
            Err(errno) => match shell_report_resource_errno(runtime, "claim", contract, errno) {
                Ok(code) => {
                    *last_status = code;
                    Ok(())
                }
                Err(code) => Err(code),
            },
        });
    }
    if let Some(rest) = line.strip_prefix("releaseclaim ") {
        let contract = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: releaseclaim <contract>");
                return Some(Err(2));
            }
        };
        return Some(match runtime.release_claimed_resource(contract) {
            Ok(ResourceReleaseOutcome::Released { resource }) => write_line(
                runtime,
                &format!("claim-released contract={contract} resource={resource}"),
            )
            .map_err(|_| 197),
            Ok(ResourceReleaseOutcome::HandedOff {
                resource,
                contract: handoff,
                acquire_count,
                handoff_count,
            }) => write_line(
                runtime,
                &format!(
                    "claim-handed-off resource={resource} to={handoff} acquire_count={acquire_count} handoff_count={handoff_count}"
                ),
            )
            .map_err(|_| 197),
            Err(errno) => {
                match shell_report_resource_errno(runtime, "releaseclaim", contract, errno) {
                    Ok(code) => {
                        *last_status = code;
                        Ok(())
                    }
                    Err(code) => Err(code),
                }
            }
        });
    }
    if let Some(rest) = line.strip_prefix("release ") {
        let contract = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: release <contract>");
                return Some(Err(2));
            }
        };
        return Some(match runtime.release_resource(contract) {
            Ok(resource) => write_line(
                runtime,
                &format!("resource-released contract={contract} resource={resource}"),
            )
            .map_err(|_| 197),
            Err(errno) => match shell_report_resource_errno(runtime, "release", contract, errno) {
                Ok(code) => {
                    *last_status = code;
                    Ok(())
                }
                Err(code) => Err(code),
            },
        });
    }
    if let Some(rest) = line.strip_prefix("transfer ") {
        let mut parts = rest.split_whitespace();
        let source = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: transfer <source-contract> <target-contract>",
                );
                return Some(Err(2));
            }
        };
        let target = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: transfer <source-contract> <target-contract>",
                );
                return Some(Err(2));
            }
        };
        return Some(match runtime.transfer_resource(source, target) {
            Ok(resource) => write_line(
                runtime,
                &format!(
                    "resource-transferred source={} target={} resource={}",
                    source, target, resource
                ),
            )
            .map_err(|_| 197),
            Err(errno) => match shell_report_transfer_errno(runtime, source, target, errno) {
                Ok(code) => {
                    *last_status = code;
                    Ok(())
                }
                Err(code) => Err(code),
            },
        });
    }
    if let Some(rest) = line.strip_prefix("cancelclaim ") {
        let contract = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: cancelclaim <contract>");
                return Some(Err(2));
            }
        };
        return Some(match runtime.cancel_resource_claim(contract) {
            Ok(outcome) => write_line(
                runtime,
                &format!(
                    "claim-canceled contract={contract} resource={} waiting_count={}",
                    outcome.resource, outcome.waiting_count
                ),
            )
            .map_err(|_| 197),
            Err(errno) => {
                match shell_report_resource_errno(runtime, "cancelclaim", contract, errno) {
                    Ok(code) => {
                        *last_status = code;
                        Ok(())
                    }
                    Err(code) => Err(code),
                }
            }
        });
    }
    if let Some(rest) = line.strip_prefix("invoke ") {
        let contract = match parse_usize_arg(Some(rest.trim())) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: invoke <contract>");
                return Some(Err(2));
            }
        };
        return Some(match runtime.invoke_contract(contract) {
            Ok(count) => write_line(
                runtime,
                &format!("invoked contract={contract} count={count}"),
            )
            .map_err(|_| 197),
            Err(errno) => {
                match shell_report_contract_target_errno(runtime, "invoke", contract, errno) {
                    Ok(code) => {
                        *last_status = code;
                        Ok(())
                    }
                    Err(code) => Err(code),
                }
            }
        });
    }
    if let Some(rest) = line.strip_prefix("contract-state ") {
        let mut parts = rest.split_whitespace();
        let contract = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: contract-state <contract> <active|suspended|revoked>",
                );
                return Some(Err(2));
            }
        };
        let state = match parts.next().and_then(parse_contract_state) {
            Some(state) => state,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: contract-state <contract> <active|suspended|revoked>",
                );
                return Some(Err(2));
            }
        };
        return Some(match runtime.set_contract_state(contract, state) {
            Ok(()) => write_line(
                runtime,
                &format!(
                    "contract-state-updated id={contract} state={}",
                    contract_state_name(state as u32)
                ),
            )
            .map_err(|_| 197),
            Err(errno) => {
                match shell_report_contract_target_errno(runtime, "contract-state", contract, errno)
                {
                    Ok(code) => {
                        *last_status = code;
                        Ok(())
                    }
                    Err(code) => Err(code),
                }
            }
        });
    }
    if let Some(rest) = line.strip_prefix("resource-state ") {
        let mut parts = rest.split_whitespace();
        let resource = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: resource-state <resource> <active|suspended|retired>",
                );
                return Some(Err(2));
            }
        };
        let state = match parts.next() {
            Some("active") => NativeResourceState::Active,
            Some("suspended") => NativeResourceState::Suspended,
            Some("retired") => NativeResourceState::Retired,
            _ => {
                let _ = write_line(
                    runtime,
                    "usage: resource-state <resource> <active|suspended|retired>",
                );
                return Some(Err(2));
            }
        };
        return Some(match runtime.set_resource_state(resource, state) {
            Ok(()) => write_line(
                runtime,
                &format!(
                    "resource-state-updated id={resource} state={}",
                    resource_state_name(state as u32)
                ),
            )
            .map_err(|_| 197),
            Err(errno) => {
                match shell_report_resource_target_errno(runtime, "resource-state", resource, errno)
                {
                    Ok(code) => {
                        *last_status = code;
                        Ok(())
                    }
                    Err(code) => Err(code),
                }
            }
        });
    }
    if let Some(rest) = line.strip_prefix("resource-policy ") {
        let mut parts = rest.split_whitespace();
        let resource = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(runtime, "usage: resource-policy <resource> <fifo|lifo>");
                return Some(Err(2));
            }
        };
        let policy = match parts.next().and_then(parse_resource_arbitration) {
            Some(policy) => policy,
            None => {
                let _ = write_line(runtime, "usage: resource-policy <resource> <fifo|lifo>");
                return Some(Err(2));
            }
        };
        return Some(
            match runtime.set_resource_arbitration_policy(resource, policy) {
                Ok(()) => write_line(
                    runtime,
                    &format!(
                        "resource-policy-updated id={resource} policy={}",
                        resource_arbitration_name(policy as u32)
                    ),
                )
                .map_err(|_| 197),
                Err(errno) => {
                    match shell_report_resource_target_errno(
                        runtime,
                        "resource-policy",
                        resource,
                        errno,
                    ) {
                        Ok(code) => {
                            *last_status = code;
                            Ok(())
                        }
                        Err(code) => Err(code),
                    }
                }
            },
        );
    }
    if let Some(rest) = line.strip_prefix("resource-governance ") {
        let mut parts = rest.split_whitespace();
        let resource = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: resource-governance <resource> <queueing|exclusive-lease>",
                );
                return Some(Err(2));
            }
        };
        let mode = match parts.next().and_then(parse_resource_governance) {
            Some(mode) => mode,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: resource-governance <resource> <queueing|exclusive-lease>",
                );
                return Some(Err(2));
            }
        };
        return Some(match runtime.set_resource_governance_mode(resource, mode) {
            Ok(()) => write_line(
                runtime,
                &format!(
                    "resource-governance-updated id={resource} mode={}",
                    resource_governance_name(mode as u32)
                ),
            )
            .map_err(|_| 197),
            Err(errno) => {
                match shell_report_resource_target_errno(
                    runtime,
                    "resource-governance",
                    resource,
                    errno,
                ) {
                    Ok(code) => {
                        *last_status = code;
                        Ok(())
                    }
                    Err(code) => Err(code),
                }
            }
        });
    }
    if let Some(rest) = line.strip_prefix("resource-contract-policy ") {
        let mut parts = rest.split_whitespace();
        let resource = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: resource-contract-policy <resource> <any|execution|memory|io|device|display|observe>",
                );
                return Some(Err(2));
            }
        };
        let policy = match parts.next().and_then(parse_resource_contract_policy) {
            Some(policy) => policy,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: resource-contract-policy <resource> <any|execution|memory|io|device|display|observe>",
                );
                return Some(Err(2));
            }
        };
        return Some(
            match runtime.set_resource_contract_policy(resource, policy) {
                Ok(()) => write_line(
                    runtime,
                    &format!(
                        "resource-contract-policy-updated id={resource} policy={}",
                        resource_contract_policy_name(policy as u32)
                    ),
                )
                .map_err(|_| 197),
                Err(errno) => {
                    match shell_report_resource_target_errno(
                        runtime,
                        "resource-contract-policy",
                        resource,
                        errno,
                    ) {
                        Ok(code) => {
                            *last_status = code;
                            Ok(())
                        }
                        Err(code) => Err(code),
                    }
                }
            },
        );
    }
    if let Some(rest) = line.strip_prefix("resource-issuer-policy ") {
        let mut parts = rest.split_whitespace();
        let resource = match parse_usize_arg(parts.next()) {
            Some(id) => id,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: resource-issuer-policy <resource> <any-issuer|creator-only|domain-owner-only>",
                );
                return Some(Err(2));
            }
        };
        let policy = match parts.next().and_then(parse_resource_issuer_policy) {
            Some(policy) => policy,
            None => {
                let _ = write_line(
                    runtime,
                    "usage: resource-issuer-policy <resource> <any-issuer|creator-only|domain-owner-only>",
                );
                return Some(Err(2));
            }
        };
        return Some(match runtime.set_resource_issuer_policy(resource, policy) {
            Ok(()) => write_line(
                runtime,
                &format!(
                    "resource-issuer-policy-updated id={resource} policy={}",
                    resource_issuer_policy_name(policy as u32)
                ),
            )
            .map_err(|_| 197),
            Err(errno) => {
                match shell_report_resource_target_errno(
                    runtime,
                    "resource-issuer-policy",
                    resource,
                    errno,
                ) {
                    Ok(code) => {
                        *last_status = code;
                        Ok(())
                    }
                    Err(code) => Err(code),
                }
            }
        });
    }
    None
}
