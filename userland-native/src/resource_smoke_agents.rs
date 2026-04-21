use super::*;

pub(crate) fn run_native_resource_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let domain = match runtime.create_domain(None, "resource-boot") {
        Ok(id) => id,
        Err(_) => return 300,
    };
    let resource = match runtime.create_resource(domain, NativeResourceKind::Device, "gpu1") {
        Ok(id) => id,
        Err(_) => return 301,
    };
    let primary =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "primary") {
            Ok(id) => id,
            Err(_) => return 302,
        };
    let mirror =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "mirror") {
            Ok(id) => id,
            Err(_) => return 303,
        };

    if runtime
        .set_contract_state(primary, NativeContractState::Suspended)
        .is_ok()
    {
        let _ = write_line(
            runtime,
            &format!("contract-state-updated id={primary} state=suspended"),
        );
    } else {
        return 304;
    }
    if runtime.invoke_contract(primary).is_ok() {
        return 305;
    }
    if runtime
        .set_contract_state(primary, NativeContractState::Active)
        .is_err()
    {
        return 306;
    }
    let _ = write_line(
        runtime,
        &format!("contract-state-updated id={primary} state=active"),
    );

    if runtime
        .set_resource_state(resource, NativeResourceState::Suspended)
        .is_ok()
    {
        let _ = write_line(
            runtime,
            &format!("resource-state-updated id={resource} state=suspended"),
        );
    } else {
        return 307;
    }
    if runtime.claim_resource(primary).is_ok() {
        return 308;
    }
    if runtime
        .set_resource_state(resource, NativeResourceState::Active)
        .is_err()
    {
        return 309;
    }
    let _ = write_line(
        runtime,
        &format!("resource-state-updated id={resource} state=active"),
    );

    match runtime.claim_resource(primary) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: acquired,
            acquire_count,
        }) if acquired == resource && acquire_count == 1 => {
            let _ = write_line(
                runtime,
                &format!(
                    "claim-acquired contract={primary} resource={resource} acquire_count={acquire_count}"
                ),
            );
        }
        _ => return 310,
    }
    match runtime.claim_resource(mirror) {
        Ok(ResourceClaimOutcome::Queued {
            resource: queued,
            holder_contract,
            position,
        }) if queued == resource && holder_contract == primary && position == 1 => {
            let _ = write_line(
                runtime,
                &format!(
                    "claim-queued contract={mirror} resource={resource} holder={holder_contract} position={position}"
                ),
            );
        }
        _ => return 311,
    }
    match runtime.release_claimed_resource(primary) {
        Ok(ResourceReleaseOutcome::HandedOff {
            resource: handed,
            contract,
            acquire_count,
            handoff_count,
        }) if handed == resource
            && contract == mirror
            && acquire_count == 2
            && handoff_count == 1 =>
        {
            let _ = write_line(
                runtime,
                &format!(
                    "claim-handed-off resource={resource} to={mirror} acquire_count={acquire_count} handoff_count={handoff_count}"
                ),
            );
        }
        _ => return 312,
    }
    match runtime.release_claimed_resource(mirror) {
        Ok(ResourceReleaseOutcome::Released { resource: released }) if released == resource => {
            let _ = write_line(
                runtime,
                &format!("claim-released contract={mirror} resource={resource}"),
            );
        }
        _ => return 313,
    }
    let domain_info = match runtime.inspect_domain(domain) {
        Ok(info) => info,
        Err(_) => return 314,
    };
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 315,
    };
    let contract_info = match runtime.inspect_contract(primary) {
        Ok(info) => info,
        Err(_) => return 316,
    };
    if write_line(
        runtime,
        &format!(
            "resource.smoke.final domain={} resource={} contract={} resources={} contracts={} holder={} waiters={} acquires={} handoffs={} outcome=ok",
            domain_info.id,
            resource_info.id,
            contract_info.id,
            domain_info.resource_count,
            domain_info.contract_count,
            resource_info.holder_contract,
            resource_info.waiting_count,
            resource_info.acquire_count,
            resource_info.handoff_count,
        ),
    )
    .is_err()
    {
        return 317;
    }
    if write_line(runtime, "resource-smoke-ok").is_err() {
        return 318;
    }
    0
}

pub(crate) fn run_native_eventing_resource_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let domain = match runtime.create_domain(None, "eventing-smoke") {
        Ok(id) => id,
        Err(_) => return 200,
    };
    let resource = match runtime.create_resource(domain, NativeResourceKind::Device, "queue0") {
        Ok(id) => id,
        Err(_) => return 201,
    };
    let primary =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "primary") {
            Ok(id) => id,
            Err(_) => return 202,
        };
    let mirror =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "mirror") {
            Ok(id) => id,
            Err(_) => return 203,
        };
    let queue_fd = match runtime.create_event_queue(NativeEventQueueMode::Epoll) {
        Ok(fd) => fd,
        Err(_) => return 204,
    };
    if runtime
        .fcntl(queue_fd, FcntlCmd::SetFl { nonblock: true })
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 205;
    }
    let watch_token = ((resource as u64) << 32) | 0x515;
    if runtime
        .watch_resource_events(
            queue_fd,
            resource,
            watch_token,
            false,
            true,
            false,
            false,
            true,
            true,
            POLLPRI,
        )
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 206;
    }

    let mut events = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 4];
    if runtime.wait_event_queue(queue_fd, &mut events) != Err(Errno::Again) {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 207;
    }

    match runtime.claim_resource(primary) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 1,
        }) if id == resource => {}
        _ => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 208;
        }
    }
    match runtime.claim_resource(mirror) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 1,
        }) if id == resource && holder_contract == primary => {}
        _ => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 209;
        }
    }

    let queued_count = match runtime.wait_event_queue(queue_fd, &mut events) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 210;
        }
    };
    if queued_count != 1
        || events[0].token != watch_token
        || events[0].events != POLLPRI
        || events[0].source_kind != NativeEventSourceKind::Resource as u32
        || events[0].source_arg0 as usize != resource
        || events[0].source_arg1 as usize != mirror
        || events[0].detail0 != 1
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 211;
    }
    if write_line(
        runtime,
        &format!(
            "resource.smoke.queued queue={} resource={} contract={} holder={}",
            queue_fd, resource, mirror, primary
        ),
    )
    .is_err()
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 212;
    }

    match runtime.release_claimed_resource(primary) {
        Ok(ResourceReleaseOutcome::HandedOff {
            resource: id,
            contract: handoff,
            acquire_count: 2,
            handoff_count: 1,
        }) if id == resource && handoff == mirror => {}
        _ => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 213;
        }
    }
    let handed_off_count = match runtime.wait_event_queue(queue_fd, &mut events) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 214;
        }
    };
    if handed_off_count != 1
        || events[0].token != watch_token
        || events[0].events != POLLPRI
        || events[0].source_kind != NativeEventSourceKind::Resource as u32
        || events[0].source_arg0 as usize != resource
        || events[0].source_arg1 as usize != mirror
        || events[0].detail0 != 4
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 215;
    }
    if write_line(
        runtime,
        &format!(
            "resource.smoke.handoff queue={} resource={} contract={} handoff_count=1",
            queue_fd, resource, mirror
        ),
    )
    .is_err()
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 216;
    }

    if runtime
        .remove_resource_events(queue_fd, resource, watch_token)
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 217;
    }
    match runtime.release_claimed_resource(mirror) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) if id == resource => {}
        _ => {
            let _ = runtime.close(queue_fd);
            return 218;
        }
    }
    if runtime.wait_event_queue(queue_fd, &mut events) != Err(Errno::Again) {
        let _ = runtime.close(queue_fd);
        return 219;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => {
            let _ = runtime.close(queue_fd);
            return 220;
        }
    };
    if resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 2
        || resource_info.handoff_count != 1
    {
        let _ = runtime.close(queue_fd);
        return 221;
    }
    if write_line(
        runtime,
        &format!(
            "resource.smoke.final resource={} holder=0 waiters=0 acquires=2 handoffs=1",
            resource
        ),
    )
    .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 222;
    }
    if runtime.close(queue_fd).is_err() {
        return 223;
    }
    0
}
