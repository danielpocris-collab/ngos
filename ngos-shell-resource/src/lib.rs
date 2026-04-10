//! Canonical subsystem role:
//! - subsystem: native resource control surface
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing resource governance actions over
//!   canonical resource contracts
//!
//! Canonical contract families handled here:
//! - resource watch contracts
//! - resource claim/release command contracts
//! - resource event inspection contracts
//!
//! This module may issue and render resource operations, but it must not
//! redefine resource truth, contract truth, or issuer/governance ownership from
//! lower layers.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use ngos_shell_types::{ShellVariable, parse_u64_arg, parse_usize_arg, shell_set_variable};
use ngos_user_abi::{
    Errno, ExitCode, NativeContractKind, NativeContractState, NativeResourceArbitrationPolicy,
    NativeResourceContractPolicy, NativeResourceGovernanceMode, NativeResourceIssuerPolicy,
    NativeResourceKind, NativeResourceState, POLLPRI, SyscallBackend,
};
use ngos_user_runtime::{ResourceClaimOutcome, ResourceReleaseOutcome, Runtime};

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

fn shell_errno_status(errno: Errno) -> ExitCode {
    257 - i32::from(errno.code())
}

fn shell_report_resource_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    operation: &str,
    contract: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "{operation}-refused contract={contract} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn shell_report_contract_target_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    operation: &str,
    contract: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "{operation}-refused contract={contract} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn shell_report_resource_target_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    operation: &str,
    resource: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "{operation}-refused resource={resource} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn shell_report_transfer_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    source: usize,
    target: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "transfer-refused source={source} target={target} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn parse_resource_kind(token: &str) -> Option<NativeResourceKind> {
    match token {
        "memory" => Some(NativeResourceKind::Memory),
        "storage" => Some(NativeResourceKind::Storage),
        "channel" => Some(NativeResourceKind::Channel),
        "device" => Some(NativeResourceKind::Device),
        "namespace" => Some(NativeResourceKind::Namespace),
        "surface" => Some(NativeResourceKind::Surface),
        _ => None,
    }
}

fn parse_contract_kind(token: &str) -> Option<NativeContractKind> {
    match token {
        "execution" => Some(NativeContractKind::Execution),
        "memory" => Some(NativeContractKind::Memory),
        "io" => Some(NativeContractKind::Io),
        "device" => Some(NativeContractKind::Device),
        "display" => Some(NativeContractKind::Display),
        "observe" => Some(NativeContractKind::Observe),
        _ => None,
    }
}

fn parse_contract_state(token: &str) -> Option<NativeContractState> {
    match token {
        "active" => Some(NativeContractState::Active),
        "suspended" => Some(NativeContractState::Suspended),
        "revoked" => Some(NativeContractState::Revoked),
        _ => None,
    }
}

fn parse_resource_arbitration(token: &str) -> Option<NativeResourceArbitrationPolicy> {
    match token {
        "fifo" => Some(NativeResourceArbitrationPolicy::Fifo),
        "lifo" => Some(NativeResourceArbitrationPolicy::Lifo),
        _ => None,
    }
}

fn parse_resource_governance(token: &str) -> Option<NativeResourceGovernanceMode> {
    match token {
        "queueing" => Some(NativeResourceGovernanceMode::Queueing),
        "exclusive-lease" => Some(NativeResourceGovernanceMode::ExclusiveLease),
        _ => None,
    }
}

fn parse_resource_contract_policy(token: &str) -> Option<NativeResourceContractPolicy> {
    match token {
        "any" => Some(NativeResourceContractPolicy::Any),
        "execution" => Some(NativeResourceContractPolicy::Execution),
        "memory" => Some(NativeResourceContractPolicy::Memory),
        "io" => Some(NativeResourceContractPolicy::Io),
        "device" => Some(NativeResourceContractPolicy::Device),
        "display" => Some(NativeResourceContractPolicy::Display),
        "observe" => Some(NativeResourceContractPolicy::Observe),
        _ => None,
    }
}

fn parse_resource_issuer_policy(token: &str) -> Option<NativeResourceIssuerPolicy> {
    match token {
        "any-issuer" => Some(NativeResourceIssuerPolicy::AnyIssuer),
        "creator-only" => Some(NativeResourceIssuerPolicy::CreatorOnly),
        "domain-owner-only" => Some(NativeResourceIssuerPolicy::DomainOwnerOnly),
        _ => None,
    }
}

fn contract_state_name(raw: u32) -> &'static str {
    match NativeContractState::from_raw(raw) {
        Some(NativeContractState::Active) => "active",
        Some(NativeContractState::Suspended) => "suspended",
        Some(NativeContractState::Revoked) => "revoked",
        None => "unknown",
    }
}

fn resource_state_name(raw: u32) -> &'static str {
    match NativeResourceState::from_raw(raw) {
        Some(NativeResourceState::Active) => "active",
        Some(NativeResourceState::Suspended) => "suspended",
        Some(NativeResourceState::Retired) => "retired",
        None => "unknown",
    }
}

fn resource_kind_name(raw: u32) -> &'static str {
    match NativeResourceKind::from_raw(raw) {
        Some(NativeResourceKind::Memory) => "memory",
        Some(NativeResourceKind::Storage) => "storage",
        Some(NativeResourceKind::Channel) => "channel",
        Some(NativeResourceKind::Device) => "device",
        Some(NativeResourceKind::Namespace) => "namespace",
        Some(NativeResourceKind::Surface) => "surface",
        None => "unknown",
    }
}

fn contract_kind_name(raw: u32) -> &'static str {
    match NativeContractKind::from_raw(raw) {
        Some(NativeContractKind::Execution) => "execution",
        Some(NativeContractKind::Memory) => "memory",
        Some(NativeContractKind::Io) => "io",
        Some(NativeContractKind::Device) => "device",
        Some(NativeContractKind::Display) => "display",
        Some(NativeContractKind::Observe) => "observe",
        None => "unknown",
    }
}

fn shell_render_domains<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = vec![0u64; 16];
    let count = runtime.list_domains(&mut ids).map_err(|_| 206)?;
    ids.truncate(count);
    let mut name = [0u8; 64];
    for id in ids {
        let copied = runtime
            .get_domain_name(id as usize, &mut name)
            .map_err(|_| 207)?;
        let label = core::str::from_utf8(&name[..copied]).map_err(|_| 208)?;
        let info = runtime.inspect_domain(id as usize).map_err(|_| 209)?;
        write_line(
            runtime,
            &format!(
                "domain id={} owner={} resources={} contracts={} name={}",
                info.id, info.owner, info.resource_count, info.contract_count, label
            ),
        )?;
    }
    Ok(())
}

fn shell_render_resources<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = vec![0u64; 16];
    let count = runtime.list_resources(&mut ids).map_err(|_| 210)?;
    ids.truncate(count);
    let mut name = [0u8; 64];
    for id in ids {
        let copied = runtime
            .get_resource_name(id as usize, &mut name)
            .map_err(|_| 211)?;
        let label = core::str::from_utf8(&name[..copied]).map_err(|_| 212)?;
        let info = runtime.inspect_resource(id as usize).map_err(|_| 213)?;
        write_line(
            runtime,
            &format!(
                "resource id={} domain={} kind={} state={} holder={} waiters={} name={}",
                info.id,
                info.domain,
                resource_kind_name(info.kind),
                resource_state_name(info.state),
                info.holder_contract,
                info.waiting_count,
                label
            ),
        )?;
    }
    Ok(())
}

fn shell_render_contracts<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = vec![0u64; 16];
    let count = runtime.list_contracts(&mut ids).map_err(|_| 214)?;
    ids.truncate(count);
    let mut label = [0u8; 64];
    for id in ids {
        let copied = runtime
            .get_contract_label(id as usize, &mut label)
            .map_err(|_| 215)?;
        let name = core::str::from_utf8(&label[..copied]).map_err(|_| 216)?;
        let info = runtime.inspect_contract(id as usize).map_err(|_| 217)?;
        write_line(
            runtime,
            &format!(
                "contract id={} domain={} resource={} issuer={} kind={} state={} label={}",
                info.id,
                info.domain,
                info.resource,
                info.issuer,
                contract_kind_name(info.kind),
                contract_state_name(info.state),
                name
            ),
        )?;
    }
    Ok(())
}

fn shell_render_domain_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let info = runtime.inspect_domain(id).map_err(|_| 220)?;
    let mut name = [0u8; 128];
    let copied = runtime.get_domain_name(id, &mut name).map_err(|_| 221)?;
    let label = core::str::from_utf8(&name[..copied]).map_err(|_| 222)?;
    write_line(
        runtime,
        &format!(
            "domain id={} owner={} parent={} resources={} contracts={} name={}",
            info.id, info.owner, info.parent, info.resource_count, info.contract_count, label
        ),
    )
}

fn shell_collect_waiters<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
) -> Result<String, ExitCode> {
    let mut ids = vec![0u64; 8];
    loop {
        let count = runtime
            .list_resource_waiters(resource, &mut ids)
            .map_err(|_| 229)?;
        if count <= ids.len() {
            ids.truncate(count);
            let rendered = if ids.is_empty() {
                String::from("-")
            } else {
                ids.into_iter()
                    .map(|id| format!("{id}"))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            return Ok(rendered);
        }
        ids.resize(count, 0);
    }
}

fn shell_render_resource_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let info = runtime.inspect_resource(id).map_err(|_| 223)?;
    let mut name = [0u8; 128];
    let copied = runtime.get_resource_name(id, &mut name).map_err(|_| 224)?;
    let label = core::str::from_utf8(&name[..copied]).map_err(|_| 225)?;
    let waiters = shell_collect_waiters(runtime, id)?;
    write_line(
        runtime,
        &format!(
            "resource id={} domain={} creator={} kind={} state={} arbitration={} governance={} contract_policy={} issuer_policy={} holder={} acquire_count={} handoff_count={} waiters={} name={}",
            info.id,
            info.domain,
            info.creator,
            resource_kind_name(info.kind),
            resource_state_name(info.state),
            resource_arbitration_name(info.arbitration),
            resource_governance_name(info.governance),
            resource_contract_policy_name(info.contract_policy),
            resource_issuer_policy_name(info.issuer_policy),
            info.holder_contract,
            info.acquire_count,
            info.handoff_count,
            waiters.as_str(),
            label
        ),
    )
}

fn shell_render_contract_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let info = runtime.inspect_contract(id).map_err(|_| 226)?;
    let mut label = [0u8; 128];
    let copied = runtime
        .get_contract_label(id, &mut label)
        .map_err(|_| 227)?;
    let text = core::str::from_utf8(&label[..copied]).map_err(|_| 228)?;
    write_line(
        runtime,
        &format!(
            "contract id={} domain={} resource={} issuer={} kind={} state={} label={}",
            info.id,
            info.domain,
            info.resource,
            info.issuer,
            contract_kind_name(info.kind),
            contract_state_name(info.state),
            text
        ),
    )
}

fn shell_render_waiters<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
) -> Result<(), ExitCode> {
    let rendered = shell_collect_waiters(runtime, resource)?;
    write_line(
        runtime,
        &format!("resource={} waiters={rendered}", resource),
    )
}

fn parse_resource_watch_kinds(
    raw: Option<&str>,
) -> Option<(bool, bool, bool, bool, bool, bool, String)> {
    let Some(raw) = raw else {
        return Some((true, true, true, true, true, true, String::from("all")));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "all" {
        return Some((true, true, true, true, true, true, String::from("all")));
    }
    let mut claimed = false;
    let mut queued = false;
    let mut canceled = false;
    let mut released = false;
    let mut handed_off = false;
    let mut revoked = false;
    for token in trimmed.split(',') {
        match token.trim() {
            "claimed" => claimed = true,
            "queued" => queued = true,
            "canceled" => canceled = true,
            "released" => released = true,
            "handed-off" => handed_off = true,
            "revoked" => revoked = true,
            _ => return None,
        }
    }
    Some((
        claimed,
        queued,
        canceled,
        released,
        handed_off,
        revoked,
        trimmed.to_string(),
    ))
}

fn shell_watch_resource_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    resource: usize,
    token: u64,
    claimed: bool,
    queued: bool,
    canceled: bool,
    released: bool,
    handed_off: bool,
    revoked: bool,
    kinds_label: &str,
) -> Result<(), ExitCode> {
    runtime
        .watch_resource_events(
            queue_fd, resource, token, claimed, queued, canceled, released, handed_off, revoked,
            POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "resource-watch queue={} resource={} token={} kinds={}",
            queue_fd, resource, token, kinds_label
        ),
    )
}

fn shell_remove_resource_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    resource: usize,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_resource_events(queue_fd, resource, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "resource-unwatch queue={} resource={} token={}",
            queue_fd, resource, token
        ),
    )
}

fn resource_arbitration_name(raw: u32) -> &'static str {
    match NativeResourceArbitrationPolicy::from_raw(raw) {
        Some(NativeResourceArbitrationPolicy::Fifo) => "fifo",
        Some(NativeResourceArbitrationPolicy::Lifo) => "lifo",
        None => "unknown",
    }
}

fn resource_governance_name(raw: u32) -> &'static str {
    match NativeResourceGovernanceMode::from_raw(raw) {
        Some(NativeResourceGovernanceMode::Queueing) => "queueing",
        Some(NativeResourceGovernanceMode::ExclusiveLease) => "exclusive-lease",
        None => "unknown",
    }
}

fn resource_contract_policy_name(raw: u32) -> &'static str {
    match NativeResourceContractPolicy::from_raw(raw) {
        Some(NativeResourceContractPolicy::Any) => "any",
        Some(NativeResourceContractPolicy::Execution) => "execution",
        Some(NativeResourceContractPolicy::Memory) => "memory",
        Some(NativeResourceContractPolicy::Io) => "io",
        Some(NativeResourceContractPolicy::Device) => "device",
        Some(NativeResourceContractPolicy::Display) => "display",
        Some(NativeResourceContractPolicy::Observe) => "observe",
        None => "unknown",
    }
}

fn resource_issuer_policy_name(raw: u32) -> &'static str {
    match NativeResourceIssuerPolicy::from_raw(raw) {
        Some(NativeResourceIssuerPolicy::AnyIssuer) => "any-issuer",
        Some(NativeResourceIssuerPolicy::CreatorOnly) => "creator-only",
        Some(NativeResourceIssuerPolicy::DomainOwnerOnly) => "domain-owner-only",
        None => "unknown",
    }
}

pub fn try_handle_resource_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    variables: &mut Vec<ShellVariable>,
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
                shell_set_variable(variables, "LAST_DOMAIN_ID", id.to_string());
                shell_set_variable(variables, "LAST_CREATED_ID", id.to_string());
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
            Ok(id) => {
                shell_set_variable(variables, "LAST_RESOURCE_ID", id.to_string());
                shell_set_variable(variables, "LAST_CREATED_ID", id.to_string());
                write_line(
                    runtime,
                    &format!(
                        "resource-created id={id} domain={domain} kind={} name={name}",
                        resource_kind_name(kind as u32)
                    ),
                )
                .map_err(|_| 197)
            }
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
        return Some(
            match runtime.create_contract(domain, resource, kind, &label) {
                Ok(id) => {
                    shell_set_variable(variables, "LAST_CONTRACT_ID", id.to_string());
                    shell_set_variable(variables, "LAST_CREATED_ID", id.to_string());
                    write_line(
                    runtime,
                    &format!(
                        "contract-created id={id} domain={domain} resource={resource} kind={} label={label}",
                        contract_kind_name(kind as u32)
                    ),
                )
                .map_err(|_| 197)
                }
                Err(_) => Err(208),
            },
        );
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
            Ok(ResourceReleaseOutcome::Released { resource }) => {
                write_line(runtime, &format!("claim-released contract={contract} resource={resource}"))
            }
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
            Err(errno) => match shell_report_resource_errno(runtime, "releaseclaim", contract, errno)
            {
                Ok(code) => {
                    *last_status = code;
                    Ok(())
                }
                Err(code) => Err(code),
            },
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
            Ok(resource) => {
                write_line(
                    runtime,
                    &format!("resource-released contract={contract} resource={resource}"),
                )
            }
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
