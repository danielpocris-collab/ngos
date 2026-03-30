use super::*;
#[test]
fn runtime_executes_read_write_poll_and_control_over_io_objects() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let file_cap = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(14_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "save.dat",
        )
        .unwrap();
    let device_cap = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(14_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "gpu0",
        )
        .unwrap();

    let file_fd = runtime
        .open_descriptor(owner, file_cap, ObjectKind::File, "save.dat")
        .unwrap();
    let device_fd = runtime
        .open_descriptor(owner, device_cap, ObjectKind::Device, "gpu0")
        .unwrap();

    let initial = runtime.read_io(owner, file_fd, 32).unwrap();
    assert!(String::from_utf8_lossy(&initial).contains("object:save.dat"));

    let written = runtime.write_io(owner, file_fd, b":patch").unwrap();
    assert_eq!(written, 6);

    let poll = runtime.poll_io(owner, device_fd).unwrap();
    assert!(poll.contains(IoPollEvents::PRIORITY));
    assert!(poll.contains(IoPollEvents::WRITABLE));

    let response = runtime.control_io(owner, device_fd, 0x10).unwrap();
    assert_eq!(response, 0x11);
    assert_eq!(
        runtime.inspect_io(owner, device_fd).unwrap().control_ops(),
        1
    );
}

#[test]
fn runtime_tracks_io_payload_scatter_gather_layouts() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let capability = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(14_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "save.dat",
        )
        .unwrap();

    let fd = runtime
        .open_descriptor(owner, capability, ObjectKind::File, "save.dat")
        .unwrap();

    let mut large = vec![b'x'; IO_PAYLOAD_SEGMENT_BYTES * 2 + 19];
    large[..11].copy_from_slice(b"hello-world");
    runtime.write_io(owner, fd, &large).unwrap();

    let object = runtime.inspect_io(owner, fd).unwrap();
    assert_eq!(
        object.payload().len(),
        b"object:save.dat".len() + large.len()
    );
    assert!(object.payload().starts_with(b"object:save.dathello-world"));
    assert!(object.payload_layout().segment_count() >= 3);
    assert_eq!(object.payload_layout().total_len(), object.payload().len());
    assert!(object.payload_layout().segments()[0].paddr >= IO_PAYLOAD_SEGMENT_BASE);
    let expected = object.payload().to_vec();

    let bytes = runtime.read_io(owner, fd, expected.len()).unwrap();
    assert_eq!(bytes, expected);
}

#[test]
fn runtime_supports_vectored_io_over_segmented_payloads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let capability = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(14_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "stream.sock",
        )
        .unwrap();

    let fd = runtime
        .open_descriptor(owner, capability, ObjectKind::Socket, "stream.sock")
        .unwrap();

    let written = runtime
        .write_io_vectored(
            owner,
            fd,
            &[b":alpha".to_vec(), b":beta".to_vec(), b":gamma".to_vec()],
        )
        .unwrap();
    assert_eq!(written, 17);

    let chunks = runtime.read_io_vectored(owner, fd, &[8, 8, 64]).unwrap();
    assert_eq!(chunks.len(), 3);
    let joined = chunks.concat();
    let text = String::from_utf8(joined).unwrap();
    assert!(text.contains("endpoint:stream.sock"));
    assert!(text.contains(":alpha:beta:gamma"));
}

#[test]
fn runtime_exposes_io_payload_layout_metadata() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let capability = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(14_102), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "layout.bin",
        )
        .unwrap();
    let fd = runtime
        .open_descriptor(owner, capability, ObjectKind::File, "layout.bin")
        .unwrap();

    runtime
        .write_io(owner, fd, &vec![b'z'; IO_PAYLOAD_SEGMENT_BYTES + 17])
        .unwrap();

    let layout = runtime.inspect_io_layout(owner, fd).unwrap();
    assert_eq!(
        layout.total_len,
        runtime.inspect_io(owner, fd).unwrap().payload().len()
    );
    assert_eq!(layout.segment_count, layout.segments.len());
    assert!(layout.segment_count >= 2);
    assert!(layout.segments[0].paddr >= IO_PAYLOAD_SEGMENT_BASE);
}

#[test]
fn runtime_exposes_vm_object_layout_metadata() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("app", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(14_1020), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(14_1021), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "lib",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/lib", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/lib/libvm.so", ObjectKind::File, lib)
        .unwrap();

    runtime
        .map_file_memory(
            owner,
            "/lib/libvm.so".to_string(),
            0x3000,
            0x1000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    let layouts = runtime.inspect_vm_object_layouts(owner).unwrap();
    assert!(!layouts.is_empty());
    let file_layout = layouts
        .into_iter()
        .find(|layout| {
            layout
                .segments
                .iter()
                .any(|segment| segment.byte_offset >= 0x1000)
        })
        .unwrap();
    assert!(file_layout.segment_count >= 1);
    assert_eq!(file_layout.segment_count, file_layout.segments.len());
    assert!(file_layout.segments[0].paddr >= 0x2_0000_0000);
}

#[test]
fn runtime_exposes_unified_process_introspection() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process("app", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let cap = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(14_103), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "assets",
        )
        .unwrap();
    let root = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(14_104), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(14_105), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "lib",
        )
        .unwrap();

    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/lib", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/lib/libintrospect.so", ObjectKind::File, lib)
        .unwrap();
    runtime
        .open_descriptor(app, cap, ObjectKind::Socket, "assets.sock")
        .unwrap();
    runtime
        .map_file_memory(
            app,
            "/lib/libintrospect.so".to_string(),
            0x2000,
            0,
            true,
            false,
            true,
            true,
        )
        .unwrap();

    let introspection = runtime.inspect_process(app).unwrap();
    assert_eq!(introspection.process.pid, app);
    assert_eq!(introspection.threads.len(), 1);
    assert_eq!(introspection.threads[0].owner, app);
    assert!(introspection.threads[0].is_main);
    assert_eq!(
        introspection.process.main_thread,
        Some(introspection.threads[0].tid)
    );
    assert_eq!(introspection.address_space.owner, app);
    assert_eq!(
        introspection.address_space.region_count,
        introspection.process.memory_region_count
    );
    assert_eq!(
        introspection.address_space.vm_object_count,
        introspection.process.vm_object_count
    );
    assert!(introspection.address_space.mapped_bytes >= 0x2000);
    assert!(!introspection.filedesc_entries.is_empty());
    assert!(!introspection.kinfo_file_entries.is_empty());
    assert!(!introspection.vm_object_layouts.is_empty());
}

#[test]
fn runtime_exposes_system_introspection() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = runtime
        .spawn_process_share_fds("app", Some(init), SchedulerClass::Interactive, init)
        .unwrap();
    let queue = runtime
        .create_event_queue(app, EventQueueMode::Epoll)
        .unwrap();

    let system = runtime.inspect_system();
    assert!(system.snapshot.process_count >= 2);
    assert!(system.snapshot.thread_count >= 2);
    assert!(system.processes.iter().any(|process| process.pid == init));
    assert!(system.processes.iter().any(|process| process.pid == app));
    assert!(
        system
            .address_spaces
            .iter()
            .any(|space| space.owner == init)
    );
    assert!(system.address_spaces.iter().any(|space| space.owner == app));
    assert!(system.event_queues.iter().any(|entry| entry.id == queue));
    assert!(
        system
            .fdshare_groups
            .iter()
            .any(|group| group.members.contains(&app))
    );
}

#[test]
fn runtime_tracks_native_domains_resources_and_contracts() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "graphics").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let contract = runtime
        .create_contract(
            owner,
            domain,
            resource,
            ContractKind::Display,
            "primary-scanout",
        )
        .unwrap();

    let domain_info = runtime.domain_info(domain).unwrap();
    assert_eq!(domain_info.owner, owner);
    assert_eq!(domain_info.name, "graphics");
    assert_eq!(domain_info.resource_count, 1);
    assert_eq!(domain_info.contract_count, 1);

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.domain, domain);
    assert_eq!(resource_info.kind, ResourceKind::Device);
    assert_eq!(resource_info.holder, None);
    assert_eq!(resource_info.acquire_count, 0);
    assert_eq!(resource_info.name, "gpu0");

    let contract_info = runtime.contract_info(contract).unwrap();
    assert_eq!(contract_info.domain, domain);
    assert_eq!(contract_info.resource, resource);
    assert_eq!(contract_info.kind, ContractKind::Display);
    assert_eq!(contract_info.state, ContractState::Active);
    assert_eq!(contract_info.invocation_count, 0);
    assert_eq!(contract_info.label, "primary-scanout");

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.domain_count, 1);
    assert_eq!(snapshot.resource_count, 1);
    assert_eq!(snapshot.contract_count, 1);

    let system = runtime.inspect_system();
    assert!(system.domains.iter().any(|entry| entry.id == domain));
    assert!(system.resources.iter().any(|entry| entry.id == resource));
    assert!(system.contracts.iter().any(|entry| entry.id == contract));
}

#[test]
fn runtime_rejects_cross_domain_contract_binding() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let audio = runtime.create_domain(owner, None, "audio").unwrap();
    let graphics = runtime.create_domain(owner, None, "graphics").unwrap();
    let speaker = runtime
        .create_resource(owner, audio, ResourceKind::Device, "speaker0")
        .unwrap();

    assert_eq!(
        runtime.create_contract(
            owner,
            graphics,
            speaker,
            ContractKind::Device,
            "cross-domain-bind",
        ),
        Err(RuntimeError::NativeModel(NativeModelError::ParentMismatch))
    );
}

#[test]
fn runtime_enforces_contract_state_transitions() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();

    assert_eq!(
        runtime
            .transition_contract_state(contract, ContractState::Suspended)
            .unwrap(),
        ContractState::Suspended
    );
    assert_eq!(
        runtime.contract_info(contract).unwrap().state,
        ContractState::Suspended
    );
    assert_eq!(
        runtime
            .transition_contract_state(contract, ContractState::Active)
            .unwrap(),
        ContractState::Active
    );
    assert_eq!(
        runtime
            .transition_contract_state(contract, ContractState::Revoked)
            .unwrap(),
        ContractState::Revoked
    );
    assert_eq!(
        runtime.transition_contract_state(contract, ContractState::Active),
        Err(RuntimeError::NativeModel(
            NativeModelError::InvalidStateTransition {
                from: ContractState::Revoked,
                to: ContractState::Active,
            }
        ))
    );
}

#[test]
fn runtime_invokes_only_active_contracts_and_tracks_usage_count() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();

    assert_eq!(runtime.invoke_contract(contract).unwrap(), 1);
    assert_eq!(runtime.invoke_contract(contract).unwrap(), 2);
    assert_eq!(runtime.contract_info(contract).unwrap().invocation_count, 2);

    runtime
        .transition_contract_state(contract, ContractState::Suspended)
        .unwrap();
    assert_eq!(
        runtime.invoke_contract(contract),
        Err(RuntimeError::NativeModel(
            NativeModelError::ContractNotActive {
                state: ContractState::Suspended,
            }
        ))
    );
}

#[test]
fn runtime_acquires_and_releases_resource_via_contract() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();

    assert_eq!(
        runtime.acquire_resource_via_contract(contract).unwrap(),
        (resource, 1)
    );
    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.holder, Some(contract));
    assert_eq!(resource_info.acquire_count, 1);

    assert_eq!(
        runtime.release_resource_via_contract(contract).unwrap(),
        resource
    );
    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.holder, None);
    assert_eq!(resource_info.acquire_count, 1);

    runtime
        .transition_contract_state(contract, ContractState::Suspended)
        .unwrap();
    assert_eq!(
        runtime.acquire_resource_via_contract(contract),
        Err(RuntimeError::NativeModel(
            NativeModelError::ContractNotActive {
                state: ContractState::Suspended,
            }
        ))
    );
}

#[test]
fn runtime_transfers_resource_between_active_contracts() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let source = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let target = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    runtime.acquire_resource_via_contract(source).unwrap();
    assert_eq!(
        runtime
            .transfer_resource_via_contract(source, target)
            .unwrap(),
        (resource, 2)
    );
    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.holder, Some(target));
    assert_eq!(resource_info.acquire_count, 2);
}

#[test]
fn runtime_claims_resource_with_fifo_queue_and_handoff() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();
    let recorder = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "record")
        .unwrap();

    assert_eq!(
        runtime.claim_resource_via_contract(primary).unwrap(),
        ResourceClaimResult::Acquired {
            resource,
            acquire_count: 1,
        }
    );
    assert_eq!(
        runtime.claim_resource_via_contract(mirror).unwrap(),
        ResourceClaimResult::Queued {
            resource,
            holder: primary,
            position: 1,
        }
    );
    assert_eq!(
        runtime.claim_resource_via_contract(recorder).unwrap(),
        ResourceClaimResult::Queued {
            resource,
            holder: primary,
            position: 2,
        }
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.arbitration, ResourceArbitrationPolicy::Fifo);
    assert_eq!(resource_info.holder, Some(primary));
    assert_eq!(resource_info.waiters, vec![mirror, recorder]);
    assert_eq!(resource_info.waiting_count, 2);

    assert_eq!(
        runtime
            .release_claimed_resource_via_contract(primary)
            .unwrap(),
        ResourceReleaseResult::HandedOff {
            resource,
            contract: mirror,
            acquire_count: 2,
            handoff_count: 1,
        }
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.holder, Some(mirror));
    assert_eq!(resource_info.waiters, vec![recorder]);
    assert_eq!(resource_info.acquire_count, 2);
    assert_eq!(resource_info.handoff_count, 1);
}

#[test]
fn runtime_records_resource_agent_decisions_for_claim_queue_and_handoff() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    let _ = runtime.claim_resource_via_contract(primary).unwrap();
    let _ = runtime.claim_resource_via_contract(mirror).unwrap();
    let _ = runtime
        .release_claimed_resource_via_contract(primary)
        .unwrap();

    let decisions = runtime.recent_resource_agent_decisions();
    assert_eq!(decisions.len(), 4);

    assert_eq!(decisions[0].agent, ResourceAgentKind::ClaimValidator);
    assert_eq!(decisions[0].resource, resource.raw());
    assert_eq!(decisions[0].contract, primary.raw());
    assert_eq!(decisions[0].detail0, 1);
    assert_eq!(decisions[0].detail1, 1);

    assert_eq!(decisions[1].agent, ResourceAgentKind::ClaimValidator);
    assert_eq!(decisions[1].resource, resource.raw());
    assert_eq!(decisions[1].contract, mirror.raw());
    assert_eq!(decisions[1].detail0, 2);
    assert_eq!(decisions[1].detail1, 1);

    assert_eq!(decisions[2].agent, ResourceAgentKind::ReleaseValidator);
    assert_eq!(decisions[2].resource, resource.raw());
    assert_eq!(decisions[2].contract, primary.raw());
    assert_eq!(decisions[2].detail0, 2);
    assert_eq!(decisions[2].detail1, mirror.raw());

    assert_eq!(decisions[3].agent, ResourceAgentKind::ReleaseValidator);
    assert_eq!(decisions[3].resource, resource.raw());
    assert_eq!(decisions[3].contract, mirror.raw());
    assert_eq!(decisions[3].detail0, 2);
    assert_eq!(decisions[3].detail1, 1);
}

#[test]
fn runtime_inspect_system_exports_resource_agent_decisions() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    let _ = runtime.claim_resource_via_contract(primary).unwrap();
    let _ = runtime.claim_resource_via_contract(mirror).unwrap();
    let _ = runtime
        .release_claimed_resource_via_contract(primary)
        .unwrap();

    let system = runtime.inspect_system();
    assert_eq!(system.resource_agent_decisions.len(), 4);
    assert_eq!(
        system.resource_agent_decisions,
        runtime.recent_resource_agent_decisions()
    );
    assert_eq!(
        system.resource_agent_decisions[0].agent,
        ResourceAgentKind::ClaimValidator
    );
    assert_eq!(system.resource_agent_decisions[0].resource, resource.raw());
    assert_eq!(
        system.resource_agent_decisions[3].agent,
        ResourceAgentKind::ReleaseValidator
    );
    assert_eq!(system.resource_agent_decisions[3].contract, mirror.raw());
}

#[test]
fn runtime_claims_resource_with_lifo_policy_and_skips_inactive_waiters() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();
    let recorder = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "record")
        .unwrap();

    assert_eq!(
        runtime
            .set_resource_arbitration_policy(resource, ResourceArbitrationPolicy::Lifo)
            .unwrap(),
        ResourceArbitrationPolicy::Lifo
    );
    runtime.claim_resource_via_contract(primary).unwrap();
    runtime.claim_resource_via_contract(mirror).unwrap();
    runtime.claim_resource_via_contract(recorder).unwrap();
    runtime
        .transition_contract_state(recorder, ContractState::Suspended)
        .unwrap();

    assert_eq!(
        runtime
            .release_claimed_resource_via_contract(primary)
            .unwrap(),
        ResourceReleaseResult::HandedOff {
            resource,
            contract: mirror,
            acquire_count: 2,
            handoff_count: 1,
        }
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.arbitration, ResourceArbitrationPolicy::Lifo);
    assert_eq!(resource_info.holder, Some(mirror));
    assert!(resource_info.waiters.is_empty());
    assert_eq!(resource_info.handoff_count, 1);
}

#[test]
fn runtime_suspending_waiting_contract_removes_it_from_resource_queue() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();
    let recorder = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "record")
        .unwrap();

    runtime.claim_resource_via_contract(primary).unwrap();
    runtime.claim_resource_via_contract(mirror).unwrap();
    runtime.claim_resource_via_contract(recorder).unwrap();

    assert_eq!(
        runtime
            .transition_contract_state(mirror, ContractState::Suspended)
            .unwrap(),
        ContractState::Suspended
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.holder, Some(primary));
    assert_eq!(resource_info.waiters, vec![recorder]);
    assert_eq!(resource_info.waiting_count, 1);

    assert_eq!(
        runtime
            .release_claimed_resource_via_contract(primary)
            .unwrap(),
        ResourceReleaseResult::HandedOff {
            resource,
            contract: recorder,
            acquire_count: 2,
            handoff_count: 1,
        }
    );
}

#[test]
fn runtime_can_cancel_queued_resource_claim_without_affecting_holder() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();
    let recorder = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "record")
        .unwrap();

    runtime.claim_resource_via_contract(primary).unwrap();
    runtime.claim_resource_via_contract(mirror).unwrap();
    runtime.claim_resource_via_contract(recorder).unwrap();

    assert_eq!(
        runtime.cancel_resource_claim_via_contract(mirror).unwrap(),
        (resource, 1)
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.holder, Some(primary));
    assert_eq!(resource_info.waiters, vec![recorder]);
    assert_eq!(resource_info.waiting_count, 1);
    assert_eq!(resource_info.acquire_count, 1);
}

#[test]
fn runtime_exclusive_lease_resource_rejects_queued_claims() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    assert_eq!(
        runtime
            .set_resource_governance_mode(resource, ResourceGovernanceMode::ExclusiveLease)
            .unwrap(),
        ResourceGovernanceMode::ExclusiveLease
    );
    assert_eq!(
        runtime.claim_resource_via_contract(primary).unwrap(),
        ResourceClaimResult::Acquired {
            resource,
            acquire_count: 1,
        }
    );
    assert!(matches!(
        runtime.claim_resource_via_contract(mirror),
        Err(RuntimeError::NativeModel(NativeModelError::ResourceBusy { holder }))
            if holder == primary
    ));

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(
        resource_info.governance,
        ResourceGovernanceMode::ExclusiveLease
    );
    assert_eq!(resource_info.holder, Some(primary));
    assert!(resource_info.waiters.is_empty());
}

#[test]
fn runtime_resource_contract_policy_rejects_mismatched_create_and_claims() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let display = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let io = runtime
        .create_contract(owner, domain, resource, ContractKind::Io, "writer")
        .unwrap();

    runtime.claim_resource_via_contract(display).unwrap();
    runtime.claim_resource_via_contract(io).unwrap();
    assert_eq!(
        runtime
            .set_resource_contract_policy(resource, ResourceContractPolicy::Display)
            .unwrap(),
        ResourceContractPolicy::Display
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(
        resource_info.contract_policy,
        ResourceContractPolicy::Display
    );
    assert_eq!(resource_info.holder, Some(display));
    assert!(resource_info.waiters.is_empty());

    assert!(matches!(
        runtime.claim_resource_via_contract(io),
        Err(RuntimeError::NativeModel(
            NativeModelError::ContractNotActive {
                state: ContractState::Revoked,
            }
        ))
    ));

    assert!(matches!(
        runtime.create_contract(owner, domain, resource, ContractKind::Io, "writer-2"),
        Err(RuntimeError::NativeModel(
            NativeModelError::ResourceContractKindMismatch {
                expected: ResourceContractPolicy::Display,
                actual: ContractKind::Io,
            }
        ))
    ));
}

#[test]
fn runtime_tightening_contract_policy_revokes_incompatible_holder() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let display = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let io = runtime
        .create_contract(owner, domain, resource, ContractKind::Io, "writer")
        .unwrap();

    runtime.claim_resource_via_contract(io).unwrap();

    assert_eq!(
        runtime
            .set_resource_contract_policy(resource, ResourceContractPolicy::Display)
            .unwrap(),
        ResourceContractPolicy::Display
    );

    let io_info = runtime.contract_info(io).unwrap();
    let display_info = runtime.contract_info(display).unwrap();
    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(io_info.state, ContractState::Revoked);
    assert_eq!(display_info.state, ContractState::Active);
    assert_eq!(
        resource_info.contract_policy,
        ResourceContractPolicy::Display
    );
    assert_eq!(resource_info.holder, None);
    assert!(resource_info.waiters.is_empty());
    assert_eq!(resource_info.acquire_count, 1);
    assert_eq!(resource_info.handoff_count, 0);
}

#[test]
fn runtime_resource_issuer_policy_restricts_contract_creation_to_creator() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let worker = runtime
        .spawn_process("worker", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();

    assert_eq!(
        runtime
            .set_resource_issuer_policy(resource, ResourceIssuerPolicy::CreatorOnly)
            .unwrap(),
        ResourceIssuerPolicy::CreatorOnly
    );
    assert!(
        runtime
            .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
            .is_ok()
    );
    assert!(matches!(
        runtime.create_contract(worker, domain, resource, ContractKind::Display, "mirror"),
        Err(RuntimeError::NativeModel(NativeModelError::ResourceIssuerPolicyMismatch {
            policy: ResourceIssuerPolicy::CreatorOnly,
            issuer,
        })) if issuer == worker
    ));
}

#[test]
fn runtime_tightening_issuer_policy_revokes_incompatible_holder_and_handoffs() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let worker = runtime
        .spawn_process("worker", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(worker, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let foreign = runtime
        .create_contract(worker, domain, resource, ContractKind::Display, "foreign")
        .unwrap();
    let native = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "native")
        .unwrap();

    runtime.claim_resource_via_contract(foreign).unwrap();
    runtime.claim_resource_via_contract(native).unwrap();

    assert_eq!(
        runtime
            .set_resource_issuer_policy(resource, ResourceIssuerPolicy::DomainOwnerOnly)
            .unwrap(),
        ResourceIssuerPolicy::DomainOwnerOnly
    );

    let foreign_info = runtime.contract_info(foreign).unwrap();
    let native_info = runtime.contract_info(native).unwrap();
    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(foreign_info.state, ContractState::Revoked);
    assert_eq!(native_info.state, ContractState::Active);
    assert_eq!(
        resource_info.issuer_policy,
        ResourceIssuerPolicy::DomainOwnerOnly
    );
    assert_eq!(resource_info.holder, Some(native));
    assert!(resource_info.waiters.is_empty());
    assert_eq!(resource_info.acquire_count, 2);
    assert_eq!(resource_info.handoff_count, 1);
}

#[test]
fn runtime_resource_state_suspend_blocks_use_until_reactivated() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let display = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();

    assert_eq!(
        runtime
            .transition_resource_state(resource, ResourceState::Suspended)
            .unwrap(),
        ResourceState::Suspended
    );
    assert_eq!(
        runtime.resource_info(resource).unwrap().state,
        ResourceState::Suspended
    );
    assert!(matches!(
        runtime.claim_resource_via_contract(display),
        Err(RuntimeError::NativeModel(
            NativeModelError::ResourceNotActive {
                state: ResourceState::Suspended,
            }
        ))
    ));
    assert!(matches!(
        runtime.invoke_contract(display),
        Err(RuntimeError::NativeModel(
            NativeModelError::ResourceNotActive {
                state: ResourceState::Suspended,
            }
        ))
    ));
    assert!(matches!(
        runtime.create_contract(owner, domain, resource, ContractKind::Display, "mirror"),
        Err(RuntimeError::NativeModel(
            NativeModelError::ResourceNotActive {
                state: ResourceState::Suspended,
            }
        ))
    ));

    assert_eq!(
        runtime
            .transition_resource_state(resource, ResourceState::Active)
            .unwrap(),
        ResourceState::Active
    );
    assert!(
        runtime
            .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
            .is_ok()
    );
    assert!(runtime.invoke_contract(display).is_ok());
}

#[test]
fn runtime_retiring_resource_revokes_existing_contracts_and_clears_claims() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let display = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    runtime.claim_resource_via_contract(display).unwrap();
    runtime.claim_resource_via_contract(mirror).unwrap();

    assert_eq!(
        runtime
            .transition_resource_state(resource, ResourceState::Retired)
            .unwrap(),
        ResourceState::Retired
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.state, ResourceState::Retired);
    assert_eq!(resource_info.holder, None);
    assert!(resource_info.waiters.is_empty());
    assert_eq!(
        runtime.contract_info(display).unwrap().state,
        ContractState::Revoked
    );
    assert_eq!(
        runtime.contract_info(mirror).unwrap().state,
        ContractState::Revoked
    );
    assert!(matches!(
        runtime.create_contract(owner, domain, resource, ContractKind::Display, "late"),
        Err(RuntimeError::NativeModel(
            NativeModelError::ResourceNotActive {
                state: ResourceState::Retired,
            }
        ))
    ));
}

#[test]
fn runtime_revoking_holder_contract_releases_claimed_resource() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "display").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    runtime.claim_resource_via_contract(primary).unwrap();
    runtime.claim_resource_via_contract(mirror).unwrap();

    assert_eq!(
        runtime
            .transition_contract_state(primary, ContractState::Revoked)
            .unwrap(),
        ContractState::Revoked
    );

    let resource_info = runtime.resource_info(resource).unwrap();
    assert_eq!(resource_info.holder, Some(mirror));
    assert!(resource_info.waiters.is_empty());
    assert_eq!(resource_info.acquire_count, 2);
    assert_eq!(resource_info.handoff_count, 1);
}

#[test]
fn syscall_surface_claim_and_handoff_resource_through_native_policy() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let owner = surface
        .runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(owner);
    let domain = surface
        .runtime
        .create_domain(owner, None, "display")
        .unwrap();
    let resource = surface
        .runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let primary = surface
        .runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
        .unwrap();
    let mirror = surface
        .runtime
        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::SetResourceArbitrationPolicy(SetResourceArbitrationPolicy {
                resource,
                policy: ResourceArbitrationPolicy::Fifo,
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceArbitrationPolicyChanged {
            resource: id,
            policy,
        } => {
            assert_eq!(id, resource);
            assert_eq!(policy, ResourceArbitrationPolicy::Fifo);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::SetResourceGovernanceMode(SetResourceGovernanceMode {
                resource,
                mode: ResourceGovernanceMode::ExclusiveLease,
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceGovernanceModeChanged { resource: id, mode } => {
            assert_eq!(id, resource);
            assert_eq!(mode, ResourceGovernanceMode::ExclusiveLease);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::ClaimResourceViaContract(ClaimResourceViaContract { contract: primary }),
        )
        .unwrap()
    {
        SyscallResult::ResourceClaimed {
            resource: id,
            contract,
            acquire_count,
        } => {
            assert_eq!(id, resource);
            assert_eq!(contract, primary);
            assert_eq!(acquire_count, 1);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    assert!(matches!(
        surface.dispatch(
            context.clone(),
            Syscall::ClaimResourceViaContract(ClaimResourceViaContract { contract: mirror }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::ResourceBusy { holder }
        ))) if holder == primary
    ));

    match surface
        .dispatch(
            context,
            Syscall::ReleaseClaimedResourceViaContract(ReleaseClaimedResourceViaContract {
                contract: primary,
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceClaimReleased {
            resource: id,
            contract,
        } => {
            assert_eq!(id, resource);
            assert_eq!(contract, primary);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_inspect_io_objects() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let capability = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(13_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "assets",
        )
        .unwrap();

    let fd = surface
        .runtime
        .open_descriptor(app, capability, ObjectKind::File, "doom.wad")
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectDescriptor { owner: app, fd })
        .unwrap()
    {
        SyscallResult::DescriptorInspected(object) => {
            assert_eq!(object.name(), "doom.wad");
            assert_eq!(object.kind(), ObjectKind::File);
            assert!(object.capabilities().contains(IoCapabilities::SEEK));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_executes_io_operations() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let capability = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_000), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "render.sock",
        )
        .unwrap();

    let fd = surface
        .runtime
        .open_descriptor(app, capability, ObjectKind::Socket, "render.sock")
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::WriteDescriptor {
                owner: app,
                fd,
                bytes: b":hello".to_vec(),
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorWritten(count) => assert_eq!(count, 6),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::PollDescriptor { owner: app, fd })
        .unwrap()
    {
        SyscallResult::DescriptorPolled(events) => {
            assert!(events.contains(IoPollEvents::READABLE));
            assert!(events.contains(IoPollEvents::WRITABLE));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::ReadDescriptor {
                owner: app,
                fd,
                len: 64,
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorRead(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("endpoint:render.sock"));
            assert!(text.contains(":hello"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_executes_vectored_io_operations() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let capability = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "render.sock",
        )
        .unwrap();

    let fd = surface
        .runtime
        .open_descriptor(app, capability, ObjectKind::Socket, "render.sock")
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::WriteDescriptorVectored {
                owner: app,
                fd,
                segments: vec![b":hello".to_vec(), b":vectored".to_vec()],
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorWritten(count) => assert_eq!(count, 15),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::ReadDescriptorVectored {
                owner: app,
                fd,
                segments: vec![12, 12, 64],
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorReadVectored(chunks) => {
            let text = String::from_utf8(chunks.concat()).unwrap();
            assert!(text.contains("endpoint:render.sock"));
            assert!(text.contains(":hello:vectored"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_exposes_io_payload_layout_metadata() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let capability = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "layout.sock",
        )
        .unwrap();

    let fd = surface
        .runtime
        .open_descriptor(app, capability, ObjectKind::Socket, "layout.sock")
        .unwrap();
    surface
        .runtime
        .write_io(app, fd, &vec![b'q'; IO_PAYLOAD_SEGMENT_BYTES + 33])
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectDescriptorLayout { owner: app, fd },
        )
        .unwrap()
    {
        SyscallResult::DescriptorLayoutInspected(layout) => {
            assert_eq!(layout.segment_count, layout.segments.len());
            assert!(layout.segment_count >= 2);
            assert!(layout.total_len >= IO_PAYLOAD_SEGMENT_BYTES);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::ReadDescriptorVectoredWithLayout {
                owner: app,
                fd,
                segments: vec![16, 32, IO_PAYLOAD_SEGMENT_BYTES + 64],
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorReadVectoredWithLayout { segments, layout } => {
            let text = String::from_utf8(segments.concat()).unwrap();
            assert!(text.contains("endpoint:layout.sock"));
            assert!(layout.segment_count >= 2);
            assert_eq!(layout.segment_count, layout.segments.len());
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_exposes_vm_object_layouts() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_1022), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_1023), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "lib",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/lib", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/lib/libsys.so", ObjectKind::File, lib)
        .unwrap();
    surface
        .runtime
        .map_file_memory(
            app,
            "/lib/libsys.so".to_string(),
            0x3000,
            0x1000,
            true,
            false,
            true,
            true,
        )
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: app })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            assert!(!layouts.is_empty());
            let layout = &layouts[0];
            assert_eq!(layout.segment_count, layout.segments.len());
            assert!(layout.segments[0].paddr >= 0x2_0000_0000);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_reports_nonzero_vm_object_segment_offsets() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_1031), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_1032), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "lib",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/lib", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/lib/liboffset.so", ObjectKind::File, lib)
        .unwrap();
    let mapped = surface
        .runtime
        .map_file_memory(
            app,
            "/lib/liboffset.so".to_string(),
            0x2000,
            0x3000,
            true,
            false,
            true,
            true,
        )
        .unwrap();
    surface
        .runtime
        .protect_memory(app, mapped, 0x2000, true, true, false)
        .unwrap();
    surface
        .runtime
        .touch_memory(app, mapped, 0x1000, true)
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: app })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            let layout = layouts
                .into_iter()
                .find(|layout| {
                    layout
                        .segments
                        .first()
                        .is_some_and(|segment| segment.byte_offset == 0x3000)
                        && layout
                            .segments
                            .iter()
                            .map(|segment| segment.byte_len)
                            .sum::<u64>()
                            == 0x2000
                })
                .expect("file-backed layout with non-zero offset must be present");
            assert_eq!(layout.segments[0].start_page, 0);
            assert_eq!(layout.segments[0].byte_offset, 0x3000);
            assert_eq!(layout.segments[1].byte_offset, 0x4000);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_reports_shadow_metadata_for_cow_vm_objects() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let scratch = surface
        .runtime
        .map_anonymous_memory(app, 0x2000, true, true, false, "shadow-scratch")
        .unwrap();
    let child = surface
        .runtime
        .spawn_process_copy_vm("forked", Some(bootstrap), SchedulerClass::Interactive, app)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch, 0x1000, true)
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: child })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            let cow_layout = layouts
                .into_iter()
                .find(|layout| layout.shadow_source_id.is_some())
                .expect("cow-derived vm object must expose a shadow source");
            assert_eq!(cow_layout.shadow_source_offset, 0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_reports_nested_shadow_depth_for_cow_chains() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let scratch = surface
        .runtime
        .map_anonymous_memory(app, 0x2000, true, true, false, "shadow-chain")
        .unwrap();
    let child = surface
        .runtime
        .spawn_process_copy_vm("child", Some(bootstrap), SchedulerClass::Interactive, app)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch, 0x1000, true)
        .unwrap();
    let grandchild = surface
        .runtime
        .spawn_process_copy_vm(
            "grandchild",
            Some(bootstrap),
            SchedulerClass::Interactive,
            child,
        )
        .unwrap();
    surface
        .runtime
        .touch_memory(grandchild, scratch, 0x1000, true)
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: grandchild })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            let nested = layouts
                .into_iter()
                .find(|layout| layout.shadow_depth == 2)
                .expect("nested cow object must report shadow depth 2");
            assert!(nested.shadow_source_id.is_some());
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_reports_nonzero_shadow_offsets_for_nested_cow_chains() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let scratch = surface
        .runtime
        .map_anonymous_memory(app, 0x3000, true, true, false, "shadow-offset-chain")
        .unwrap();
    let child = surface
        .runtime
        .spawn_process_copy_vm("child", Some(bootstrap), SchedulerClass::Interactive, app)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();
    let grandchild = surface
        .runtime
        .spawn_process_copy_vm(
            "grandchild",
            Some(bootstrap),
            SchedulerClass::Interactive,
            child,
        )
        .unwrap();
    surface
        .runtime
        .touch_memory(grandchild, scratch + 0x1000, 0x1000, true)
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: grandchild })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            let nested = layouts
                .into_iter()
                .find(|layout| layout.shadow_depth == 2 && layout.shadow_source_offset == 0x1000)
                .expect("nested cow object must retain the non-zero shadow offset");
            assert!(nested.shadow_source_id.is_some());
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_reuses_shadow_objects_for_adjacent_partial_cow_faults() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let scratch = surface
        .runtime
        .map_anonymous_memory(app, 0x3000, true, true, false, "shadow-reuse")
        .unwrap();
    let child = surface
        .runtime
        .spawn_process_copy_vm("child", Some(bootstrap), SchedulerClass::Interactive, app)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch, 0x1000, true)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: child })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            let shadow_layouts = layouts
                .into_iter()
                .filter(|layout| layout.shadow_source_id.is_some())
                .collect::<Vec<_>>();
            assert_eq!(shadow_layouts.len(), 1);
            assert_eq!(shadow_layouts[0].shadow_source_offset, 0);
            assert_eq!(
                shadow_layouts[0]
                    .segments
                    .iter()
                    .map(|segment| segment.byte_len)
                    .sum::<u64>(),
                0x2000
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_reuses_shadow_objects_for_reverse_adjacent_partial_cow_faults() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let scratch = surface
        .runtime
        .map_anonymous_memory(app, 0x3000, true, true, false, "shadow-reuse-reverse")
        .unwrap();
    let child = surface
        .runtime
        .spawn_process_copy_vm("child", Some(bootstrap), SchedulerClass::Interactive, app)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch, 0x1000, true)
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: child })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            let shadow_layouts = layouts
                .into_iter()
                .filter(|layout| layout.shadow_source_id.is_some())
                .collect::<Vec<_>>();
            assert_eq!(shadow_layouts.len(), 1);
            assert_eq!(shadow_layouts[0].shadow_source_offset, 0);
            assert_eq!(shadow_layouts[0].segments[0].byte_offset, 0);
            assert_eq!(
                shadow_layouts[0]
                    .segments
                    .iter()
                    .map(|segment| segment.byte_len)
                    .sum::<u64>(),
                0x2000
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_merges_shadow_objects_when_a_fault_bridges_both_sides() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let scratch = surface
        .runtime
        .map_anonymous_memory(app, 0x3000, true, true, false, "shadow-bridge")
        .unwrap();
    let child = surface
        .runtime
        .spawn_process_copy_vm("child", Some(bootstrap), SchedulerClass::Interactive, app)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch, 0x1000, true)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch + 0x2000, 0x1000, true)
        .unwrap();
    surface
        .runtime
        .touch_memory(child, scratch + 0x1000, 0x1000, true)
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectVmObjectLayouts { pid: child })
        .unwrap()
    {
        SyscallResult::VmObjectLayouts(layouts) => {
            let shadow_layouts = layouts
                .into_iter()
                .filter(|layout| layout.shadow_source_id.is_some())
                .collect::<Vec<_>>();
            assert_eq!(shadow_layouts.len(), 1);
            assert_eq!(shadow_layouts[0].shadow_source_offset, 0);
            assert_eq!(
                shadow_layouts[0]
                    .segments
                    .iter()
                    .map(|segment| segment.byte_len)
                    .sum::<u64>(),
                0x3000
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_exposes_unified_process_introspection() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let cap = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_103), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "assets",
        )
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_104), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(15_105), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "lib",
        )
        .unwrap();

    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/lib", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/lib/libbundle.so", ObjectKind::File, lib)
        .unwrap();
    surface
        .runtime
        .open_descriptor(app, cap, ObjectKind::Socket, "assets.sock")
        .unwrap();
    surface
        .runtime
        .map_file_memory(
            app,
            "/lib/libbundle.so".to_string(),
            0x2000,
            0,
            true,
            false,
            true,
            true,
        )
        .unwrap();

    match surface
        .dispatch(context, Syscall::InspectProcess { pid: app })
        .unwrap()
    {
        SyscallResult::ProcessIntrospection(introspection) => {
            assert_eq!(introspection.process.pid, app);
            assert_eq!(introspection.address_space.owner, app);
            assert!(introspection.address_space.region_count >= 1);
            assert!(!introspection.filedesc_entries.is_empty());
            assert!(!introspection.kinfo_file_entries.is_empty());
            assert!(!introspection.vm_object_layouts.is_empty());
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_exposes_system_introspection() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process_share_fds(
            "app",
            Some(bootstrap),
            SchedulerClass::Interactive,
            bootstrap,
        )
        .unwrap();
    let queue = surface
        .runtime
        .create_event_queue(app, EventQueueMode::Kqueue)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    match surface.dispatch(context, Syscall::InspectSystem).unwrap() {
        SyscallResult::SystemIntrospection(system) => {
            assert!(system.snapshot.process_count >= 2);
            assert!(
                system
                    .processes
                    .iter()
                    .any(|process| process.pid == bootstrap)
            );
            assert!(system.processes.iter().any(|process| process.pid == app));
            assert!(
                system
                    .address_spaces
                    .iter()
                    .any(|space| space.owner == bootstrap)
            );
            assert!(system.address_spaces.iter().any(|space| space.owner == app));
            assert!(system.event_queues.iter().any(|entry| entry.id == queue));
            assert!(
                system
                    .fdshare_groups
                    .iter()
                    .any(|group| group.members.contains(&app))
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn execution_contract_rebinds_scheduler_policy() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("exec-app", None, SchedulerClass::Background)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "sched").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Device, "gpu-slice")
        .unwrap();
    runtime
        .set_resource_governance_mode(resource, ResourceGovernanceMode::ExclusiveLease)
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Execution, "exec")
        .unwrap();
    runtime.bind_process_contract(owner, contract).unwrap();

    let info = runtime.process_info(owner).unwrap();
    assert_eq!(info.contract_bindings.execution, Some(contract));
    assert_eq!(info.scheduler_policy.class, SchedulerClass::LatencyCritical);
    assert_eq!(info.scheduler_policy.budget, 4);

    let scheduled = runtime.tick().unwrap();
    assert_eq!(scheduled.pid, owner);
    assert_eq!(scheduled.class, SchedulerClass::LatencyCritical);
    assert_eq!(scheduled.budget, 4);
}

#[test]
fn memory_contract_controls_vm_operations_end_to_end() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("vm-app", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "vm").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Memory, "vm-budget")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Memory)
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Memory, "vm")
        .unwrap();
    runtime.bind_process_contract(owner, contract).unwrap();

    let start = runtime
        .map_anonymous_memory(owner, 0x2000, true, true, false, "heap")
        .unwrap();
    assert!(start > 0);

    runtime
        .transition_contract_state(contract, ContractState::Suspended)
        .unwrap();
    let err = runtime
        .map_anonymous_memory(owner, 0x1000, true, true, false, "blocked")
        .unwrap_err();
    assert_eq!(
        err,
        RuntimeError::NativeModel(NativeModelError::ContractNotActive {
            state: ContractState::Suspended
        })
    );
}

#[test]
fn memory_contract_policy_blocks_all_vm_operations_and_exposes_decisions() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("vm-guarded", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "vm").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Memory, "vm-budget")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Memory)
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Memory, "vm")
        .unwrap();
    runtime.bind_process_contract(owner, contract).unwrap();
    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(18_800), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(18_801), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "lib",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/lib", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/lib/libpolicy.so", ObjectKind::File, lib)
        .unwrap();

    let mapped = runtime
        .map_anonymous_memory(owner, 0x2000, true, true, false, "policy-scratch")
        .unwrap();
    runtime.store_memory_word(owner, mapped, 41).unwrap();
    let vm_object_id = runtime.resolve_vm_object_id(owner, mapped, 0x2000).unwrap();

    runtime
        .transition_contract_state(contract, ContractState::Suspended)
        .unwrap();

    for error in [
        runtime
            .map_anonymous_memory(owner, 0x1000, true, true, false, "blocked-map")
            .map(|_| ()),
        runtime
            .map_file_memory(
                owner,
                "/lib/libpolicy.so",
                0x1000,
                0,
                true,
                false,
                true,
                true,
            )
            .map(|_| ()),
        runtime.unmap_memory(owner, mapped, 0x1000),
        runtime.protect_memory(owner, mapped, 0x1000, true, false, false),
        runtime.advise_memory(owner, mapped, 0x1000, MemoryAdvice::DontNeed),
        runtime.sync_memory(owner, mapped, 0x1000),
        runtime.quarantine_vm_object(owner, vm_object_id, 7),
        runtime.release_vm_object_quarantine(owner, vm_object_id),
    ] {
        assert_eq!(
            error,
            Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: ContractState::Suspended
                }
            ))
        );
    }

    for error in [
        runtime
            .touch_memory(owner, mapped, 0x1000, true)
            .map(|_| ()),
        runtime.load_memory_word(owner, mapped).map(|_| ()),
        runtime.compare_memory_word(owner, mapped, 41).map(|_| ()),
        runtime.store_memory_word(owner, mapped, 99).map(|_| ()),
        runtime
            .update_memory_word(owner, mapped, MemoryWordUpdateOp::Add(1))
            .map(|_| ()),
        runtime
            .set_process_break(owner, mapped + 0x4000)
            .map(|_| ()),
        runtime.reclaim_memory_pressure(owner, 1).map(|_| ()),
    ] {
        assert_eq!(
            error,
            Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: ContractState::Suspended
                }
            ))
        );
    }

    let vmdecisions = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmdecisions", owner.raw()))
            .unwrap(),
    )
    .unwrap();
    for operation in [
        "\tdetail1=0",
        "\tdetail1=1",
        "\tdetail1=2",
        "\tdetail1=3",
        "\tdetail1=4",
        "\tdetail1=5",
        "\tdetail1=6",
        "\tdetail1=7",
        "\tdetail1=8",
        "\tdetail1=9",
        "\tdetail1=10",
        "\tdetail1=11",
        "\tdetail1=12",
        "\tdetail1=13",
        "\tdetail1=14",
    ] {
        assert!(vmdecisions.contains("agent=policy-block"));
        assert!(vmdecisions.contains(operation));
    }

    let vmepisodes = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/vmepisodes", owner.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(vmepisodes.contains("kind=policy"));
    assert!(vmepisodes.contains("state=1"));
    assert!(vmepisodes.contains("operation=14"));
    assert!(vmepisodes.contains("blocked=yes"));

    let value = runtime.load_memory_word(owner, mapped).unwrap_err();
    assert_eq!(
        value,
        RuntimeError::NativeModel(NativeModelError::ContractNotActive {
            state: ContractState::Suspended
        })
    );

    runtime
        .transition_contract_state(contract, ContractState::Active)
        .unwrap();
    assert_eq!(runtime.load_memory_word(owner, mapped).unwrap(), 41);
    let info = runtime.process_info(owner).unwrap();
    let requested_brk = info.executable_image.base_addr + 0x9000;
    let new_brk = runtime.set_process_break(owner, requested_brk).unwrap();
    assert!(new_brk >= requested_brk);
}

#[test]
fn io_contract_controls_descriptor_and_path_io() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("io-app", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "io").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "io-flow")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Io)
        .unwrap();
    let contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Io, "io")
        .unwrap();
    runtime.bind_process_contract(owner, contract).unwrap();

    let root = runtime
        .grant_capability(
            owner,
            ObjectHandle::new(Handle::new(18_001), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/note", ObjectKind::File, root)
        .unwrap();

    let fd = runtime.open_path(owner, "/note").unwrap();
    assert!(runtime.write_io(owner, fd, b"hello").is_ok());

    runtime
        .transition_contract_state(contract, ContractState::Revoked)
        .unwrap();
    let err = runtime.open_path(owner, "/note").unwrap_err();
    assert_eq!(
        err,
        RuntimeError::NativeModel(NativeModelError::ContractNotActive {
            state: ContractState::Revoked
        })
    );
}

#[test]
fn observe_contract_gates_cross_process_procfs_reads_and_exposes_policy() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("target", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(observer, None, "obs").unwrap();
    let resource = runtime
        .create_resource(observer, domain, ResourceKind::Namespace, "inspect")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Observe)
        .unwrap();
    let contract = runtime
        .create_contract(observer, domain, resource, ContractKind::Observe, "observe")
        .unwrap();

    let denied = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/status", target.raw()))
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

    runtime.bind_process_contract(observer, contract).unwrap();
    let status = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/status", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(status.contains("ObserveContract:\t0"));

    let self_status = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/status", observer.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(self_status.contains(&format!("ObserveContract:\t{}", contract.raw())));
}
