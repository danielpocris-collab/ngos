use super::*;

pub(crate) fn run_native_surface_core<B: SyscallBackend>(
    runtime: &Runtime<B>,
    validate_stdio: bool,
) -> ExitCode {
    if validate_stdio && runtime.fcntl(1, FcntlCmd::GetFl).unwrap_or(usize::MAX) != 0 {
        return 2;
    }
    if validate_stdio && runtime.poll(1, POLLOUT).unwrap_or(0) & POLLOUT == 0 {
        return 3;
    }
    let dup_fd = match runtime.dup(1) {
        Ok(fd) => fd,
        Err(_) => return 4,
    };
    debug_break(0x4e47_4f53_4253_3030, dup_fd as u64);
    if runtime
        .fcntl(dup_fd, FcntlCmd::SetFd { cloexec: true })
        .unwrap_or(usize::MAX)
        != 2
    {
        return 5;
    }
    debug_break(0x4e47_4f53_4253_3041, dup_fd as u64);
    if runtime.fcntl(dup_fd, FcntlCmd::GetFd).unwrap_or(usize::MAX) != 2 {
        return 6;
    }
    debug_break(0x4e47_4f53_4253_3042, dup_fd as u64);
    if runtime.close(dup_fd).is_err() {
        return 7;
    }
    debug_break(0x4e47_4f53_4253_3043, dup_fd as u64);

    debug_break(0x4e47_4f53_4253_3031, 0);
    let storage = match runtime.inspect_device("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 131,
    };
    debug_break(0x4e47_4f53_4253_3032, storage.block_size as u64);
    if storage.block_size != 512 || storage.capacity_bytes < 512 {
        return 132;
    }
    debug_break(0x4e47_4f53_4253_3033, storage.capacity_bytes);
    let driver = match runtime.inspect_driver("/drv/storage0") {
        Ok(record) => record,
        Err(_) => return 133,
    };
    debug_break(0x4e47_4f53_4253_3034, driver.bound_device_count);
    if driver.bound_device_count != 1 {
        return 134;
    }
    let block_fd = match runtime.open_path("/dev/storage0") {
        Ok(fd) => fd,
        Err(_) => return 135,
    };
    debug_break(0x4e47_4f53_4253_3035, block_fd as u64);
    let rights = ngos_user_abi::BlockRightsMask::READ.union(ngos_user_abi::BlockRightsMask::SUBMIT);
    let (capability, label, provenance, integrity) =
        default_block_request_security(0x5354_4f52_4147_4530, 1, rights);
    let request = NativeBlockIoRequest {
        magic: NATIVE_BLOCK_IO_MAGIC,
        version: NATIVE_BLOCK_IO_VERSION,
        op: NATIVE_BLOCK_IO_OP_READ,
        sector: 0,
        sector_count: 1,
        block_size: storage.block_size,
        rights,
        capability,
        label,
        provenance,
        integrity,
    };
    debug_break(0x4e47_4f53_4253_3036, request.block_size as u64);
    let request_bytes = encode_block_request_bytes(&request);
    debug_break(0x4e47_4f53_4253_3037, request_bytes.len() as u64);
    if runtime.write(block_fd, request_bytes).is_err() {
        let _ = runtime.close(block_fd);
        return 136;
    }
    debug_break(0x4e47_4f53_4253_3038, 0);
    let driver_fd = match runtime.open_path("/drv/storage0") {
        Ok(fd) => fd,
        Err(_) => {
            let _ = runtime.close(block_fd);
            return 137;
        }
    };
    debug_break(0x4e47_4f53_4253_3039, driver_fd as u64);
    if runtime.poll(driver_fd, POLLIN).unwrap_or(0) != POLLIN {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 138;
    }
    debug_break(0x4e47_4f53_4253_3044, driver_fd as u64);
    let mut driver_bytes = [0u8; 512];
    let driver_read = match runtime.read(driver_fd, &mut driver_bytes) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.close(driver_fd);
            let _ = runtime.close(block_fd);
            return 139;
        }
    };
    debug_break(0x4e47_4f53_4253_3045, driver_read as u64);
    if driver_read == 0 {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 140;
    }
    let driver_payload = &driver_bytes[..driver_read];
    let driver_prefix_len = driver_payload
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(driver_payload.len());
    let driver_request = match try_decode_block_request(&driver_payload[driver_prefix_len..]) {
        Some(request) => request,
        None => {
            let _ = runtime.close(driver_fd);
            let _ = runtime.close(block_fd);
            return 141;
        }
    };
    debug_break(0x4e47_4f53_4253_3046, driver_request.sector_count as u64);
    if driver_request.op != NATIVE_BLOCK_IO_OP_READ
        || driver_request.sector != 0
        || driver_request.sector_count != 1
    {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 142;
    }
    let completion_payload = b"sector0:eb58904d5357494e";
    if runtime.write(driver_fd, completion_payload).is_err() {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 143;
    }
    if runtime.poll(block_fd, POLLIN).unwrap_or(0) != POLLIN {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 144;
    }
    debug_break(0x4e47_4f53_4253_3047, block_fd as u64);
    let mut sector0 = [0u8; 512];
    let sector0_read = match runtime.read(block_fd, &mut sector0) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.close(driver_fd);
            let _ = runtime.close(block_fd);
            return 145;
        }
    };
    debug_break(0x4e47_4f53_4253_3048, sector0_read as u64);
    let _ = runtime.close(driver_fd);
    let _ = runtime.close(block_fd);
    let completion = &sector0[..sector0_read];
    if completion != completion_payload {
        return 146;
    }

    let domain = match runtime.create_domain(None, "graphics") {
        Ok(id) => id,
        Err(_) => return 8,
    };
    let resource = match runtime.create_resource(domain, NativeResourceKind::Device, "gpu0") {
        Ok(id) => id,
        Err(_) => return 9,
    };
    let contract =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "scanout") {
            Ok(id) => id,
            Err(_) => return 10,
        };
    let mirror =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "mirror") {
            Ok(id) => id,
            Err(_) => return 11,
        };
    let recorder =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "record") {
            Ok(id) => id,
            Err(_) => return 12,
        };

    let mut domain_ids = [0u64; 4];
    let domain_count = match runtime.list_domains(&mut domain_ids) {
        Ok(count) if count >= 1 && domain_ids[0] as usize == domain => count,
        _ => return 12,
    };
    debug_break(0x4e47_4f53_4253_3049, domain_count as u64);
    let mut resource_ids = [0u64; 4];
    if !matches!(
        runtime.list_resources(&mut resource_ids),
        Ok(count) if count >= 1 && resource_ids[0] as usize == resource
    ) {
        return 13;
    }
    debug_break(0x4e47_4f53_4253_3050, resource_ids[0]);
    let mut contract_ids = [0u64; 4];
    if !matches!(
        runtime.list_contracts(&mut contract_ids),
        Ok(count) if count >= 3
            && contract_ids[0] as usize == contract
            && contract_ids[1] as usize == mirror
            && contract_ids[2] as usize == recorder
    ) {
        return 14;
    }
    debug_break(0x4e47_4f53_4253_3051, contract_ids[2]);

    let domain_info = match runtime.inspect_domain(domain) {
        Ok(info) => info,
        Err(_) => return 14,
    };
    debug_break(0x4e47_4f53_4253_3052, domain_info.contract_count);
    if domain_info.id as usize != domain
        || domain_info.resource_count != 1
        || domain_info.contract_count != 3
        || domain_info.owner == 0
        || domain_count < 1
    {
        return 15;
    }

    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 16,
    };
    debug_break(0x4e47_4f53_4253_3053, resource_info.acquire_count);
    if resource_info.id as usize != resource
        || resource_info.domain as usize != domain
        || resource_info.kind != NativeResourceKind::Device as u32
        || resource_info.arbitration != NativeResourceArbitrationPolicy::Fifo as u32
        || resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 0
        || resource_info.handoff_count != 0
    {
        return 17;
    }

    let contract_info = match runtime.inspect_contract(contract) {
        Ok(info) => info,
        Err(_) => return 18,
    };
    debug_break(0x4e47_4f53_4253_3054, contract_info.state as u64);
    if contract_info.id as usize != contract
        || contract_info.domain as usize != domain
        || contract_info.resource as usize != resource
        || contract_info.kind != NativeContractKind::Display as u32
        || contract_info.state != NativeContractState::Active as u32
    {
        return 19;
    }

    debug_break(0x4e47_4f53_4253_3059, contract as u64);
    if runtime
        .set_contract_state(contract, NativeContractState::Suspended)
        .is_err()
    {
        return 20;
    }
    debug_break(0x4e47_4f53_4253_3055, contract as u64);
    let contract_info = match runtime.inspect_contract(contract) {
        Ok(info) => info,
        Err(_) => return 21,
    };
    debug_break(0x4e47_4f53_4253_3561, contract_info.state as u64);
    if contract_info.state != NativeContractState::Suspended as u32 {
        return 22;
    }
    debug_break(0x4e47_4f53_4253_3062, contract as u64);
    if runtime.invoke_contract(contract) != Err(ngos_user_abi::Errno::Access) {
        return 23;
    }
    debug_break(0x4e47_4f53_4253_3056, contract as u64);
    debug_break(0x4e47_4f53_4253_3060, contract as u64);
    if runtime
        .set_contract_state(contract, NativeContractState::Active)
        .is_err()
    {
        return 24;
    }
    let contract_info = match runtime.inspect_contract(contract) {
        Ok(info) => info,
        Err(_) => return 25,
    };
    if contract_info.state != NativeContractState::Active as u32 {
        return 26;
    }
    let invocation_count = match runtime.invoke_contract(contract) {
        Ok(count) => count,
        Err(_) => return 27,
    };
    debug_break(0x4e47_4f53_4253_3057, invocation_count as u64);
    if invocation_count != 1 {
        return 28;
    }
    if runtime
        .set_resource_arbitration_policy(resource, NativeResourceArbitrationPolicy::Fifo)
        .is_err()
    {
        return 29;
    }
    match runtime.claim_resource(contract) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 1,
        }) if id == resource => {}
        _ => return 30,
    }
    debug_break(0x4e47_4f53_4253_3058, resource as u64);
    match runtime.claim_resource(mirror) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 1,
        }) if id == resource && holder_contract == contract => {}
        _ => return 31,
    }
    match runtime.claim_resource(recorder) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 2,
        }) if id == resource && holder_contract == contract => {}
        _ => return 32,
    }
    let mut waiters = [0u64; 4];
    if !matches!(
        runtime.list_resource_waiters(resource, &mut waiters),
        Ok(2) if waiters[0] as usize == mirror && waiters[1] as usize == recorder
    ) {
        return 33;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.queue resource={} holder={} waiters={}:{} acquires=1 handoffs=0",
            resource, contract, mirror, recorder
        ),
    )
    .is_err()
    {
        return 102;
    }
    match runtime.cancel_resource_claim(mirror) {
        Ok(ResourceCancelOutcome {
            resource: id,
            waiting_count: 1,
        }) if id == resource => {}
        _ => return 34,
    }
    if !matches!(
        runtime.list_resource_waiters(resource, &mut waiters),
        Ok(1) if waiters[0] as usize == recorder
    ) {
        return 35;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.cancel resource={} canceled={} waiters={} acquires=1 handoffs=0",
            resource, mirror, recorder
        ),
    )
    .is_err()
    {
        return 103;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 36,
    };
    if resource_info.arbitration != NativeResourceArbitrationPolicy::Fifo as u32
        || resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract as usize != contract
        || resource_info.waiting_count != 1
        || resource_info.acquire_count != 1
        || resource_info.handoff_count != 0
    {
        return 37;
    }
    match runtime.release_claimed_resource(contract) {
        Ok(ResourceReleaseOutcome::HandedOff {
            resource: id,
            contract: handoff,
            acquire_count: 2,
            handoff_count: 1,
        }) if id == resource && handoff == recorder => {}
        _ => return 38,
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 39,
    };
    if resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract as usize != recorder
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 2
        || resource_info.handoff_count != 1
    {
        return 40;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.handoff resource={} from={} to={} waiters=0 acquires=2 handoffs=1",
            resource, contract, recorder
        ),
    )
    .is_err()
    {
        return 104;
    }
    let transferred = match runtime.transfer_resource(recorder, mirror) {
        Ok(id) => id,
        Err(_) => return 41,
    };
    if transferred != resource {
        return 42;
    }
    if !matches!(runtime.list_resource_waiters(resource, &mut waiters), Ok(0)) {
        return 43;
    }
    let released = match runtime.release_resource(mirror) {
        Ok(id) => id,
        Err(_) => return 44,
    };
    if released != resource {
        return 45;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 46,
    };
    if resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 3
        || resource_info.handoff_count != 2
    {
        return 47;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.transfer resource={} from={} to={} holder=0 acquires=3 handoffs=2",
            resource, recorder, mirror
        ),
    )
    .is_err()
    {
        return 105;
    }

    match runtime.claim_resource(mirror) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 4,
        }) if id == resource => {}
        _ => return 48,
    }
    match runtime.claim_resource(recorder) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 1,
        }) if id == resource && holder_contract == mirror => {}
        _ => return 49,
    }
    if runtime
        .set_contract_state(recorder, NativeContractState::Suspended)
        .is_err()
    {
        return 50;
    }
    if !matches!(runtime.list_resource_waiters(resource, &mut waiters), Ok(0)) {
        return 51;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 52,
    };
    if resource_info.holder_contract as usize != mirror
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 4
        || resource_info.handoff_count != 2
    {
        return 53;
    }
    match runtime.release_claimed_resource(mirror) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) if id == resource => {}
        _ => return 54,
    }
    debug_break(0x4e47_4f53_4253_3633, resource as u64);
    if runtime.claim_resource(recorder) != Err(ngos_user_abi::Errno::Access) {
        return 55;
    }
    if runtime
        .set_contract_state(recorder, NativeContractState::Active)
        .is_err()
    {
        return 56;
    }
    match runtime.claim_resource(recorder) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 5,
        }) if id == resource => {}
        _ => return 57,
    }
    if runtime
        .set_contract_state(recorder, NativeContractState::Revoked)
        .is_err()
    {
        return 58;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 59,
    };
    if resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 5
        || resource_info.handoff_count != 2
    {
        return 60;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.revoked resource={} contract={} holder=0 waiters=0 acquires=5 handoffs=2",
            resource, recorder
        ),
    )
    .is_err()
    {
        return 106;
    }
    if runtime
        .set_resource_governance_mode(resource, NativeResourceGovernanceMode::ExclusiveLease)
        .is_err()
    {
        return 61;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 62,
    };
    if resource_info.governance != NativeResourceGovernanceMode::ExclusiveLease as u32 {
        return 63;
    }
    match runtime.claim_resource(contract) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 6,
        }) if id == resource => {}
        _ => return 64,
    }
    if runtime.claim_resource(mirror) != Err(ngos_user_abi::Errno::Busy) {
        return 65;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 66,
    };
    if resource_info.holder_contract as usize != contract
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 6
        || resource_info.handoff_count != 2
    {
        return 67;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.exclusive resource={} holder={} refused={} reason=busy acquires=6 handoffs=2",
            resource, contract, mirror
        ),
    )
    .is_err()
    {
        return 107;
    }
    let released = match runtime.release_claimed_resource(contract) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) => id,
        _ => return 68,
    };
    if released != resource {
        return 69;
    }
    let writer = match runtime.create_contract(domain, resource, NativeContractKind::Io, "writer") {
        Ok(id) => id,
        Err(_) => return 70,
    };
    if runtime
        .set_resource_contract_policy(resource, NativeResourceContractPolicy::Io)
        .is_err()
    {
        return 71;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 72,
    };
    if resource_info.contract_policy != NativeResourceContractPolicy::Io as u32
        || resource_info.governance != NativeResourceGovernanceMode::ExclusiveLease as u32
    {
        return 73;
    }
    match runtime.claim_resource(writer) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 7,
        }) if id == resource => {}
        _ => return 74,
    }
    if runtime.claim_resource(contract) != Err(ngos_user_abi::Errno::Access) {
        return 75;
    }
    match runtime.release_claimed_resource(writer) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) if id == resource => {}
        _ => return 76,
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.contract-policy resource={} policy=io acquired={} refused={} reason=access",
            resource, writer, contract
        ),
    )
    .is_err()
    {
        return 108;
    }
    debug_break(0x4e47_4f53_4253_3634, resource as u64);
    if runtime.create_contract(domain, resource, NativeContractKind::Display, "overlay")
        != Err(ngos_user_abi::Errno::Access)
    {
        return 77;
    }
    if runtime
        .set_resource_issuer_policy(resource, NativeResourceIssuerPolicy::CreatorOnly)
        .is_err()
    {
        return 78;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 79,
    };
    if resource_info.issuer_policy != NativeResourceIssuerPolicy::CreatorOnly as u32 {
        return 80;
    }
    if runtime
        .create_contract(domain, resource, NativeContractKind::Io, "writer-2")
        .is_err()
    {
        return 81;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.issuer-policy resource={} policy=creator-only allowed_domain={} outcome=ok",
            resource, domain
        ),
    )
    .is_err()
    {
        return 109;
    }
    if runtime
        .set_resource_state(resource, NativeResourceState::Suspended)
        .is_err()
    {
        return 82;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 83,
    };
    if resource_info.state != NativeResourceState::Suspended as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
    {
        return 84;
    }
    if runtime.claim_resource(writer) != Err(ngos_user_abi::Errno::Access) {
        return 85;
    }
    if runtime.invoke_contract(contract) != Err(ngos_user_abi::Errno::Access) {
        return 86;
    }
    if runtime.create_contract(domain, resource, NativeContractKind::Io, "writer-3")
        != Err(ngos_user_abi::Errno::Access)
    {
        return 87;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.suspended resource={} claims=refused invoke=refused create-contract=refused",
            resource
        ),
    )
    .is_err()
    {
        return 110;
    }
    if runtime
        .set_resource_state(resource, NativeResourceState::Active)
        .is_err()
    {
        return 88;
    }
    let writer3 =
        match runtime.create_contract(domain, resource, NativeContractKind::Io, "writer-3") {
            Ok(id) => id,
            Err(_) => return 89,
        };
    if runtime
        .set_resource_state(resource, NativeResourceState::Retired)
        .is_err()
    {
        return 90;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 91,
    };
    if resource_info.state != NativeResourceState::Retired as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
    {
        return 92;
    }
    let writer3_info = match runtime.inspect_contract(writer3) {
        Ok(info) => info,
        Err(_) => return 93,
    };
    if writer3_info.state != NativeContractState::Revoked as u32 {
        return 94;
    }
    if runtime.create_contract(domain, resource, NativeContractKind::Io, "writer-4")
        != Err(ngos_user_abi::Errno::Access)
    {
        return 95;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.retired resource={} writer={} state=retired revoked=true create-contract=refused",
            resource, writer3
        ),
    )
    .is_err()
    {
        return 111;
    }

    let mut text = [0u8; 16];
    let copied = match runtime.get_domain_name(domain, &mut text) {
        Ok(count) => count,
        Err(_) => return 96,
    };
    if &text[..copied] != b"graphics" {
        return 97;
    }

    let copied = match runtime.get_resource_name(resource, &mut text) {
        Ok(count) => count,
        Err(_) => return 98,
    };
    if &text[..copied] != b"gpu0" {
        return 99;
    }

    let copied = match runtime.get_contract_label(contract, &mut text) {
        Ok(count) => count,
        Err(_) => return 100,
    };
    if &text[..copied] != b"scanout" {
        return 101;
    }

    if write_line(
        runtime,
        &format!(
            "resource.core.final resource={} state=retired governance=exclusive-lease contract-policy=io issuer-policy=creator-only holder=0 waiters=0 acquires=7 handoffs=2",
            resource
        ),
    )
    .is_err()
    {
        return 112;
    }

    0
}

pub(crate) fn run_native_surface_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
    validate_stdio: bool,
) -> ExitCode {
    let core_code = run_native_surface_core(runtime, validate_stdio);
    if core_code != 0 {
        return core_code;
    }
    debug_break(0x4e47_4f53_5653_3030, 0);
    let eventing_code = run_native_eventing_resource_smoke(runtime);
    if eventing_code != 0 {
        return eventing_code;
    }
    debug_break(0x4e47_4f53_5653_3032, 0);
    let vm_smoke_code = run_native_vm_boot_smoke(runtime);
    if vm_smoke_code != 0 {
        return vm_smoke_code;
    }
    debug_break(0x4e47_4f53_5653_3031, 0);
    debug_break(0x4e47_4f53_4753_3030, 0);
    let game_smoke_code = run_native_game_stack_smoke(runtime);
    if game_smoke_code != 0 {
        return game_smoke_code;
    }
    debug_break(0x4e47_4f53_4753_3031, 0);
    let render3d_code = run_native_render3d_smoke(runtime);
    if render3d_code != 0 {
        return render3d_code;
    }
    debug_break(0x4e47_4f53_5233_4430, 0);
    let payload = b"ngos-userland-native: native abi ok\n";
    match runtime.write(1, payload) {
        Ok(wrote) if wrote == payload.len() => 0,
        _ => 102,
    }
}

#[cfg(target_os = "none")]
pub(crate) fn debug_break(marker: u64, value: u64) {
    unsafe {
        core::arch::asm!(
            "mov rax, {marker}",
            "mov rdi, {value}",
            "int3",
            marker = in(reg) marker,
            value = in(reg) value,
            options(nostack)
        );
    }
}

#[cfg(not(target_os = "none"))]
pub(crate) fn debug_break(_marker: u64, _value: u64) {}
