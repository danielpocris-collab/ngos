use super::*;
#[test]
fn native_model_user_syscalls_create_domain_resource_and_contract() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanout")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();

    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    let contract = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    let system = runtime.inspect_system();
    assert!(
        system
            .domains
            .iter()
            .any(|entry| entry.id.raw() as usize == domain)
    );
    assert!(
        system
            .resources
            .iter()
            .any(|entry| entry.id.raw() as usize == resource)
    );
    assert!(
        system
            .contracts
            .iter()
            .any(|entry| entry.id.raw() as usize == contract)
    );
}

#[test]
fn native_model_user_syscalls_reject_invalid_kind_and_utf8() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, &[0xff, 0xfe])
        .unwrap();

    let invalid_utf8 = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 2, 0, 0, 0]),
    );
    assert_eq!(invalid_utf8, SyscallReturn::err(Errno::Inval));

    runtime
        .copy_to_user(pid, mapped as usize, b"storage")
        .unwrap();
    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 7, 0, 0, 0]),
        )
        .into_result()
        .unwrap();

    let invalid_kind = runtime.dispatch_user_syscall_frame(
        pid,
        SyscallFrame::new(SYS_CREATE_RESOURCE, [domain, 99, mapped as usize, 7, 0, 0]),
    );
    assert_eq!(invalid_kind, SyscallReturn::err(Errno::Inval));
}

#[test]
fn native_model_user_syscalls_list_and_inspect_entities() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"storagenvmejournal")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 7, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Storage as usize,
                    mapped as usize + 7,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let contract = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Io as usize,
                    mapped as usize + 11,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    let ids_ptr = mapped as usize + 0x80;
    let listed_domains = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_LIST_DOMAINS, [ids_ptr, 4, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert!(listed_domains >= 1);
    let domain_id_bytes = runtime.copy_from_user(pid, ids_ptr, 8).unwrap();
    assert_eq!(
        u64::from_ne_bytes(domain_id_bytes.try_into().unwrap()) as usize,
        domain
    );

    let listed_resources = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_LIST_RESOURCES, [ids_ptr, 4, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert!(listed_resources >= 1);

    let listed_contracts = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_LIST_CONTRACTS, [ids_ptr, 4, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert!(listed_contracts >= 1);

    let domain_record_ptr = mapped as usize + 0x100;
    let resource_record_ptr = mapped as usize + 0x180;
    let contract_record_ptr = mapped as usize + 0x200;

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_INSPECT_DOMAIN, [domain, domain_record_ptr, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_CONTRACT,
                [contract, contract_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let domain_bytes = runtime
        .copy_from_user(
            pid,
            domain_record_ptr,
            core::mem::size_of::<NativeDomainRecord>(),
        )
        .unwrap();
    let domain_record =
        unsafe { core::ptr::read_unaligned(domain_bytes.as_ptr().cast::<NativeDomainRecord>()) };
    assert_eq!(domain_record.id as usize, domain);
    assert_eq!(domain_record.resource_count, 1);
    assert_eq!(domain_record.contract_count, 1);

    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(resource_record.id as usize, resource);
    assert_eq!(resource_record.domain as usize, domain);
    assert_eq!(resource_record.kind, NativeResourceKind::Storage as u32);
    assert_eq!(resource_record.holder_contract, 0);
    assert_eq!(resource_record.acquire_count, 0);

    let contract_bytes = runtime
        .copy_from_user(
            pid,
            contract_record_ptr,
            core::mem::size_of::<NativeContractRecord>(),
        )
        .unwrap();
    let contract_record = unsafe {
        core::ptr::read_unaligned(contract_bytes.as_ptr().cast::<NativeContractRecord>())
    };
    assert_eq!(contract_record.id as usize, contract);
    assert_eq!(contract_record.domain as usize, domain);
    assert_eq!(contract_record.resource as usize, resource);
    assert_eq!(contract_record.kind, NativeContractKind::Io as u32);
    assert_eq!(contract_record.state, NativeContractState::Active as u32);
}

#[test]
fn native_model_user_syscalls_bind_process_contract_and_reject_foreign_issuer() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"rendergpuio")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 6, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 6,
                    3,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let contract = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Io as usize,
                    mapped as usize + 9,
                    2,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_BIND_PROCESS_CONTRACT, [contract, 0, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime
            .inspect_process(pid)
            .unwrap()
            .process
            .contract_bindings
            .io
            .map(|id| id.raw()),
        Some(contract as u64)
    );

    let foreign = runtime
        .spawn_process("foreign", None, SchedulerClass::Interactive)
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            foreign,
            SyscallFrame::new(SYS_BIND_PROCESS_CONTRACT, [contract, 0, 0, 0, 0, 0]),
        ),
        SyscallReturn::err(Errno::Inval)
    );
    assert_eq!(
        runtime
            .inspect_process(foreign)
            .unwrap()
            .process
            .contract_bindings
            .io,
        None
    );
}

#[test]
fn native_model_user_syscalls_export_text_metadata() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanout")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let contract = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    let name_ptr = mapped as usize + 0x280;
    let copied = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_GET_DOMAIN_NAME, [domain, name_ptr, 16, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert_eq!(copied, 8);
    assert_eq!(
        runtime.copy_from_user(pid, name_ptr, copied).unwrap(),
        b"graphics"
    );

    let copied = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_GET_RESOURCE_NAME, [resource, name_ptr, 16, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert_eq!(copied, 4);
    assert_eq!(
        runtime.copy_from_user(pid, name_ptr, copied).unwrap(),
        b"gpu0"
    );

    let copied = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_GET_CONTRACT_LABEL, [contract, name_ptr, 16, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    assert_eq!(copied, 7);
    assert_eq!(
        runtime.copy_from_user(pid, name_ptr, copied).unwrap(),
        b"scanout"
    );
}

#[test]
fn native_model_user_syscalls_update_contract_state() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanout")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let contract = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_CONTRACT_STATE,
                [
                    contract,
                    NativeContractState::Suspended as usize,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let contract_record_ptr = mapped as usize + 0x200;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_CONTRACT,
                [contract, contract_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let contract_bytes = runtime
        .copy_from_user(
            pid,
            contract_record_ptr,
            core::mem::size_of::<NativeContractRecord>(),
        )
        .unwrap();
    let contract_record = unsafe {
        core::ptr::read_unaligned(contract_bytes.as_ptr().cast::<NativeContractRecord>())
    };
    assert_eq!(contract_record.state, NativeContractState::Suspended as u32);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_CONTRACT_STATE,
                [contract, NativeContractState::Revoked as usize, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_CONTRACT_STATE,
                [contract, NativeContractState::Active as usize, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::err(Errno::Inval)
    );
}

#[test]
fn native_model_user_syscalls_invoke_contract_and_reject_inactive_state() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanout")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let contract = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime
            .dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(SYS_INVOKE_CONTRACT, [contract, 0, 0, 0, 0, 0]),
            )
            .into_result()
            .unwrap(),
        1
    );
    assert_eq!(
        runtime
            .dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(SYS_INVOKE_CONTRACT, [contract, 0, 0, 0, 0, 0]),
            )
            .into_result()
            .unwrap(),
        2
    );

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_CONTRACT_STATE,
                [
                    contract,
                    NativeContractState::Suspended as usize,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_INVOKE_CONTRACT, [contract, 0, 0, 0, 0, 0]),
        ),
        SyscallReturn::err(Errno::Access)
    );
}

#[test]
fn native_model_user_syscalls_acquire_and_release_resource() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanout")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let contract = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime
            .dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(SYS_ACQUIRE_RESOURCE, [contract, 0, 0, 0, 0, 0]),
            )
            .into_result()
            .unwrap(),
        resource
    );

    let resource_record_ptr = mapped as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(resource_record.holder_contract as usize, contract);
    assert_eq!(resource_record.acquire_count, 1);

    assert_eq!(
        runtime
            .dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(SYS_RELEASE_RESOURCE, [contract, 0, 0, 0, 0, 0]),
            )
            .into_result()
            .unwrap(),
        resource
    );
}

#[test]
fn native_model_user_syscalls_state_change_updates_claim_queue_and_holder() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"displaygpu0scanoutmirrorrecord")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 7, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 7,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let primary = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 11,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let mirror = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 18,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let recorder = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 24,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    let claim_ptr = mapped as usize + 0x200;
    for contract in [primary, mirror, recorder] {
        assert_eq!(
            runtime.dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(SYS_CLAIM_RESOURCE, [contract, claim_ptr, 0, 0, 0, 0]),
            ),
            SyscallReturn::ok(0)
        );
    }

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_CONTRACT_STATE,
                [mirror, NativeContractState::Suspended as usize, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let waiters_ptr = mapped as usize + 0x280;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_LIST_RESOURCE_WAITERS,
                [resource, waiters_ptr, 4, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(1)
    );
    let waiter_bytes = runtime.copy_from_user(pid, waiters_ptr, 8).unwrap();
    let waiter = u64::from_ne_bytes(waiter_bytes[..8].try_into().unwrap());
    assert_eq!(waiter as usize, recorder);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_RELEASE_CLAIMED_RESOURCE,
                [primary, mapped as usize + 0x300, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_CONTRACT_STATE,
                [recorder, NativeContractState::Revoked as usize, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let resource_record_ptr = mapped as usize + 0x380;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(resource_record.holder_contract, 0);
    assert_eq!(resource_record.waiting_count, 0);
    assert_eq!(resource_record.acquire_count, 2);
    assert_eq!(resource_record.handoff_count, 1);
}

#[test]
fn native_model_user_syscalls_can_cancel_queued_resource_claim() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"displaygpu0scanoutmirrorrecord")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 7, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 7,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let primary = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 11,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let mirror = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 18,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let recorder = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 24,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    let claim_ptr = mapped as usize + 0x200;
    for contract in [primary, mirror, recorder] {
        assert_eq!(
            runtime.dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(SYS_CLAIM_RESOURCE, [contract, claim_ptr, 0, 0, 0, 0]),
            ),
            SyscallReturn::ok(0)
        );
    }

    let cancel_ptr = mapped as usize + 0x280;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CANCEL_RESOURCE_CLAIM, [mirror, cancel_ptr, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    let cancel_bytes = runtime
        .copy_from_user(
            pid,
            cancel_ptr,
            core::mem::size_of::<NativeResourceCancelRecord>(),
        )
        .unwrap();
    let cancel_record = unsafe {
        core::ptr::read_unaligned(cancel_bytes.as_ptr().cast::<NativeResourceCancelRecord>())
    };
    assert_eq!(cancel_record.resource as usize, resource);
    assert_eq!(cancel_record.waiting_count, 1);

    let waiters_ptr = mapped as usize + 0x300;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_LIST_RESOURCE_WAITERS,
                [resource, waiters_ptr, 4, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(1)
    );
    let waiter_bytes = runtime.copy_from_user(pid, waiters_ptr, 8).unwrap();
    let waiter = u64::from_ne_bytes(waiter_bytes[..8].try_into().unwrap());
    assert_eq!(waiter as usize, recorder);
}

#[test]
fn native_model_user_syscalls_can_enable_exclusive_lease_governance() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"displaygpu0scanoutmirror")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 7, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 7,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let primary = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 11,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let mirror = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 18,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_RESOURCE_GOVERNANCE,
                [
                    resource,
                    NativeResourceGovernanceMode::ExclusiveLease as usize,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let claim_ptr = mapped as usize + 0x200;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CLAIM_RESOURCE, [primary, claim_ptr, 0, 0, 0, 0]),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CLAIM_RESOURCE, [mirror, claim_ptr, 0, 0, 0, 0]),
        ),
        SyscallReturn::err(Errno::Busy)
    );

    let resource_record_ptr = mapped as usize + 0x280;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(
        resource_record.governance,
        NativeResourceGovernanceMode::ExclusiveLease as u32
    );
    assert_eq!(resource_record.holder_contract as usize, primary);
    assert_eq!(resource_record.waiting_count, 0);
}

#[test]
fn native_model_user_syscalls_can_enforce_resource_contract_policy() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanoutwriteroverlay")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let display = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let writer = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Io as usize,
                    mapped as usize + 19,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_RESOURCE_CONTRACT_POLICY,
                [
                    resource,
                    NativeResourceContractPolicy::Display as usize,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [display, mapped as usize + 0x100, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [writer, mapped as usize + 0x140, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::err(Errno::Access)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Io as usize,
                    mapped as usize + 25,
                    7,
                    0,
                ],
            ),
        ),
        SyscallReturn::err(Errno::Access)
    );

    let resource_record_ptr = mapped as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(
        resource_record.contract_policy,
        NativeResourceContractPolicy::Display as u32
    );
}

#[test]
fn native_model_user_syscalls_tighten_contract_policy_and_revoke_incompatible_holder() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanoutwriter")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let display = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let writer = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Io as usize,
                    mapped as usize + 19,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [writer, mapped as usize + 0x100, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_RESOURCE_CONTRACT_POLICY,
                [
                    resource,
                    NativeResourceContractPolicy::Display as usize,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let resource_record_ptr = mapped as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(
        resource_record.contract_policy,
        NativeResourceContractPolicy::Display as u32
    );
    assert_eq!(resource_record.holder_contract, 0);

    let contract_record_ptr = mapped as usize + 0x220;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_CONTRACT,
                [writer, contract_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let contract_bytes = runtime
        .copy_from_user(
            pid,
            contract_record_ptr,
            core::mem::size_of::<NativeContractRecord>(),
        )
        .unwrap();
    let contract_record = unsafe {
        core::ptr::read_unaligned(contract_bytes.as_ptr().cast::<NativeContractRecord>())
    };
    assert_eq!(contract_record.state, NativeContractState::Revoked as u32);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [display, mapped as usize + 0x260, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
}

#[test]
fn native_model_user_syscalls_can_suspend_and_reactivate_resource() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanoutmirror")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let display = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_RESOURCE_STATE,
                [
                    resource,
                    NativeResourceState::Suspended as usize,
                    0,
                    0,
                    0,
                    0
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [display, mapped as usize + 0x100, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::err(Errno::Access)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_INVOKE_CONTRACT, [display, 0, 0, 0, 0, 0]),
        ),
        SyscallReturn::err(Errno::Access)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 19,
                    6,
                    0,
                ],
            ),
        ),
        SyscallReturn::err(Errno::Access)
    );

    let resource_record_ptr = mapped as usize + 0x140;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(resource_record.state, NativeResourceState::Suspended as u32);

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_RESOURCE_STATE,
                [resource, NativeResourceState::Active as usize, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert!(
        runtime
            .dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(
                    SYS_CREATE_CONTRACT,
                    [
                        domain,
                        resource,
                        NativeContractKind::Display as usize,
                        mapped as usize + 19,
                        6,
                        0,
                    ],
                ),
            )
            .into_result()
            .is_ok()
    );
}

#[test]
fn native_model_user_syscalls_retiring_resource_revokes_existing_contracts() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanoutmirror")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let display = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let mirror = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 19,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [display, mapped as usize + 0x100, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [mirror, mapped as usize + 0x140, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_SET_RESOURCE_STATE,
                [resource, NativeResourceState::Retired as usize, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let resource_record_ptr = mapped as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(resource_record.state, NativeResourceState::Retired as u32);
    assert_eq!(resource_record.holder_contract, 0);
    assert_eq!(resource_record.waiting_count, 0);

    let contract_record_ptr = mapped as usize + 0x1c0;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_CONTRACT,
                [display, contract_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let contract_bytes = runtime
        .copy_from_user(
            pid,
            contract_record_ptr,
            core::mem::size_of::<NativeContractRecord>(),
        )
        .unwrap();
    let contract_record = unsafe {
        core::ptr::read_unaligned(contract_bytes.as_ptr().cast::<NativeContractRecord>())
    };
    assert_eq!(contract_record.state, NativeContractState::Revoked as u32);
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 19,
                    6,
                    0,
                ],
            ),
        ),
        SyscallReturn::err(Errno::Access)
    );
}

#[test]
fn native_model_user_syscalls_can_enforce_resource_issuer_policy() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let worker = runtime
        .spawn_process("worker", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let mapped_owner = runtime
        .map_anonymous_memory(owner, 0x1000, true, true, false, "owner-syscall")
        .unwrap();
    let mapped_worker = runtime
        .map_anonymous_memory(worker, 0x1000, true, true, false, "worker-syscall")
        .unwrap();
    runtime
        .copy_to_user(owner, mapped_owner as usize, b"graphicsgpu0scanoutmirror")
        .unwrap();
    runtime
        .copy_to_user(worker, mapped_worker as usize, b"mirror")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped_owner as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped_owner as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(
                SYS_SET_RESOURCE_ISSUER_POLICY,
                [
                    resource,
                    NativeResourceIssuerPolicy::CreatorOnly as usize,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    assert!(
        runtime
            .dispatch_user_syscall_frame(
                owner,
                SyscallFrame::new(
                    SYS_CREATE_CONTRACT,
                    [
                        domain,
                        resource,
                        NativeContractKind::Display as usize,
                        mapped_owner as usize + 12,
                        7,
                        0,
                    ],
                ),
            )
            .into_result()
            .is_ok()
    );

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            worker,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped_worker as usize,
                    6,
                    0,
                ],
            ),
        ),
        SyscallReturn::err(Errno::Access)
    );

    let resource_record_ptr = mapped_owner as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            owner,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(
        resource_record.issuer_policy,
        NativeResourceIssuerPolicy::CreatorOnly as u32
    );
}

#[test]
fn native_model_user_syscalls_tighten_issuer_policy_and_revoke_foreign_holder() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let worker = runtime
        .spawn_process("worker", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let mapped_owner = runtime
        .map_anonymous_memory(owner, 0x1000, true, true, false, "owner-syscall")
        .unwrap();
    let mapped_worker = runtime
        .map_anonymous_memory(worker, 0x1000, true, true, false, "worker-syscall")
        .unwrap();
    runtime
        .copy_to_user(owner, mapped_owner as usize, b"displaynative")
        .unwrap();
    runtime
        .copy_to_user(worker, mapped_worker as usize, b"gpu0foreign")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped_owner as usize, 7, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            worker,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped_worker as usize,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let foreign = runtime
        .dispatch_user_syscall_frame(
            worker,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped_worker as usize + 4,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let native = runtime
        .dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped_owner as usize + 7,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            worker,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [foreign, mapped_worker as usize + 0x100, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(
                SYS_CLAIM_RESOURCE,
                [native, mapped_owner as usize + 0x100, 0, 0, 0, 0]
            ),
        ),
        SyscallReturn::ok(0)
    );
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(
                SYS_SET_RESOURCE_ISSUER_POLICY,
                [
                    resource,
                    NativeResourceIssuerPolicy::DomainOwnerOnly as usize,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
        ),
        SyscallReturn::ok(0)
    );

    let resource_record_ptr = mapped_owner as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            owner,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            owner,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(
        resource_record.issuer_policy,
        NativeResourceIssuerPolicy::DomainOwnerOnly as u32
    );
    assert_eq!(resource_record.holder_contract as usize, native);
    assert_eq!(resource_record.waiting_count, 0);
    assert_eq!(resource_record.handoff_count, 1);

    let foreign_record_ptr = mapped_worker as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            worker,
            SyscallFrame::new(
                SYS_INSPECT_CONTRACT,
                [foreign, foreign_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let foreign_bytes = runtime
        .copy_from_user(
            worker,
            foreign_record_ptr,
            core::mem::size_of::<NativeContractRecord>(),
        )
        .unwrap();
    let foreign_record =
        unsafe { core::ptr::read_unaligned(foreign_bytes.as_ptr().cast::<NativeContractRecord>()) };
    assert_eq!(foreign_record.state, NativeContractState::Revoked as u32);
}

#[test]
fn native_model_user_syscalls_transfer_resource_between_contracts() {
    let (mut runtime, pid, mapped) = setup_runtime_with_user_process();
    runtime
        .copy_to_user(pid, mapped as usize, b"graphicsgpu0scanoutmirror")
        .unwrap();

    let domain = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_CREATE_DOMAIN, [0, mapped as usize, 8, 0, 0, 0]),
        )
        .into_result()
        .unwrap();
    let resource = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_RESOURCE,
                [
                    domain,
                    NativeResourceKind::Device as usize,
                    mapped as usize + 8,
                    4,
                    0,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let source = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 12,
                    7,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();
    let target = runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_CREATE_CONTRACT,
                [
                    domain,
                    resource,
                    NativeContractKind::Display as usize,
                    mapped as usize + 19,
                    6,
                    0,
                ],
            ),
        )
        .into_result()
        .unwrap();

    runtime
        .dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(SYS_ACQUIRE_RESOURCE, [source, 0, 0, 0, 0, 0]),
        )
        .into_result()
        .unwrap();

    assert_eq!(
        runtime
            .dispatch_user_syscall_frame(
                pid,
                SyscallFrame::new(SYS_TRANSFER_RESOURCE, [source, target, 0, 0, 0, 0]),
            )
            .into_result()
            .unwrap(),
        resource
    );

    let resource_record_ptr = mapped as usize + 0x180;
    assert_eq!(
        runtime.dispatch_user_syscall_frame(
            pid,
            SyscallFrame::new(
                SYS_INSPECT_RESOURCE,
                [resource, resource_record_ptr, 0, 0, 0, 0],
            ),
        ),
        SyscallReturn::ok(0)
    );
    let resource_bytes = runtime
        .copy_from_user(
            pid,
            resource_record_ptr,
            core::mem::size_of::<NativeResourceRecord>(),
        )
        .unwrap();
    let resource_record = unsafe {
        core::ptr::read_unaligned(resource_bytes.as_ptr().cast::<NativeResourceRecord>())
    };
    assert_eq!(resource_record.holder_contract as usize, target);
    assert_eq!(resource_record.acquire_count, 2);
}
