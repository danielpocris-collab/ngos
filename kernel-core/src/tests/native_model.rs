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
fn procfs_system_resources_renders_state_and_recovery_flow() {
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

    let system_before =
        String::from_utf8(runtime.read_procfs_path("/proc/system/resources").unwrap()).unwrap();
    assert!(system_before.contains("resources:\t1"));
    assert!(system_before.contains("resource\tid="));
    assert!(system_before.contains("holder=-"));
    assert!(system_before.contains("waiting=0"));

    runtime.claim_resource_via_contract(primary).unwrap();
    runtime.claim_resource_via_contract(mirror).unwrap();

    let queued =
        String::from_utf8(runtime.read_procfs_path("/proc/system/resources").unwrap()).unwrap();
    assert!(queued.contains("queued:\t1"));
    assert!(queued.contains(&format!("holder={}", primary.raw())));
    assert!(queued.contains(&format!("waiters=[{}]", mirror.raw())));
    assert!(queued.contains("decision\ttick="));

    runtime
        .release_claimed_resource_via_contract(primary)
        .unwrap();

    let recovered =
        String::from_utf8(runtime.read_procfs_path("/proc/system/resources").unwrap()).unwrap();
    assert!(recovered.contains(&format!("holder={}", mirror.raw())));
    assert!(recovered.contains("handoffs=1"));
    assert!(recovered.contains("resource-decisions:"));
}

#[test]
fn observe_contract_gates_system_resources_procfs_reads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("target", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();

    let denied = runtime
        .read_procfs_path_for(observer, "/proc/system/resources")
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

    let denied = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/resources", target.raw()))
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

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
    runtime.bind_process_contract(observer, contract).unwrap();

    let system = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/resources")
            .unwrap(),
    )
    .unwrap();
    assert!(system.contains("resources:"));
    assert!(system.contains("resource-decisions:"));

    let target_view = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/resources", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(target_view.contains(&format!("pid:\t{}", target.raw())));
}

#[test]
fn runtime_bus_routes_channel_messages_and_reports_procfs_state() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();

    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Io)
        .unwrap();
    let bus_contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Io, "render-bus-io")
        .unwrap();
    runtime.bind_process_contract(owner, bus_contract).unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render", ObjectKind::Channel, root)
        .unwrap();

    let peer = runtime.create_bus_peer(owner, domain, "renderer").unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/render")
        .unwrap();
    runtime.attach_bus_peer(peer, endpoint).unwrap();
    assert_eq!(
        runtime.bus_publish(peer, endpoint, b"hello-bus").unwrap(),
        9
    );
    assert_eq!(runtime.bus_receive(peer, endpoint).unwrap(), b"hello-bus");

    let observe_domain = runtime.create_domain(observer, None, "obs").unwrap();
    let observe_resource = runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();

    let procfs = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/bus")
            .unwrap(),
    )
    .unwrap();
    assert!(procfs.contains("bus-peers:\t1"));
    assert!(procfs.contains("bus-endpoints:\t1"));
    assert!(procfs.contains("peer\tid="));
    assert!(procfs.contains("endpoint\tid="));
    assert!(procfs.contains("path=/ipc/render"));
    assert!(procfs.contains("contract-policy=io"));
    assert!(procfs.contains("publishes=1"));
    assert!(procfs.contains("receives=1"));
}

#[test]
fn observe_contract_gates_bus_procfs_and_bus_rejects_unattached_peers() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render", ObjectKind::Channel, root)
        .unwrap();
    let peer = runtime.create_bus_peer(owner, domain, "renderer").unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/render")
        .unwrap();

    let denied = runtime
        .read_procfs_path_for(observer, "/proc/system/bus")
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

    let publish_denied = runtime.bus_publish(peer, endpoint, b"blocked").unwrap_err();
    assert_eq!(
        publish_denied,
        RuntimeError::NativeModel(NativeModelError::BusPeerNotAttached { peer, endpoint })
    );
}

#[test]
fn runtime_bus_detach_is_reversible_and_restores_refusal_state() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Io)
        .unwrap();
    let bus_contract = runtime
        .create_contract(owner, domain, resource, ContractKind::Io, "render-bus-io")
        .unwrap();
    runtime.bind_process_contract(owner, bus_contract).unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render", ObjectKind::Channel, root)
        .unwrap();
    let peer = runtime.create_bus_peer(owner, domain, "renderer").unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/render")
        .unwrap();

    runtime.attach_bus_peer(peer, endpoint).unwrap();
    runtime.bus_publish(peer, endpoint, b"one").unwrap();
    runtime.detach_bus_peer(peer, endpoint).unwrap();

    let denied = runtime.bus_publish(peer, endpoint, b"two").unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::BusPeerNotAttached { peer, endpoint })
    );

    runtime.attach_bus_peer(peer, endpoint).unwrap();
    assert_eq!(runtime.bus_receive(peer, endpoint).unwrap(), b"one");
}

#[test]
fn runtime_bus_io_contract_policy_gates_attach_publish_and_recovers_after_binding() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let client = runtime
        .spawn_process("client", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Io)
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/policy", ObjectKind::Channel, root)
        .unwrap();
    let peer = runtime.create_bus_peer(client, domain, "renderer").unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/policy")
        .unwrap();

    assert_eq!(
        runtime.attach_bus_peer(peer, endpoint),
        Err(RuntimeError::NativeModel(
            NativeModelError::ProcessContractMissing {
                kind: ContractKind::Io
            }
        ))
    );

    let foreign_resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "wrong-bus")
        .unwrap();
    runtime
        .set_resource_contract_policy(foreign_resource, ResourceContractPolicy::Io)
        .unwrap();
    let foreign_contract = runtime
        .create_contract(
            client,
            domain,
            foreign_resource,
            ContractKind::Io,
            "wrong-io",
        )
        .unwrap();
    runtime
        .bind_process_contract(client, foreign_contract)
        .unwrap();
    assert_eq!(
        runtime.attach_bus_peer(peer, endpoint),
        Err(RuntimeError::NativeModel(
            NativeModelError::ResourceBindingMismatch
        ))
    );

    let bus_contract = runtime
        .create_contract(client, domain, resource, ContractKind::Io, "render-bus-io")
        .unwrap();
    runtime.bind_process_contract(client, bus_contract).unwrap();
    runtime.attach_bus_peer(peer, endpoint).unwrap();
    assert_eq!(
        runtime.bus_publish(peer, endpoint, b"policy-ok").unwrap(),
        9
    );
    assert_eq!(runtime.bus_receive(peer, endpoint).unwrap(), b"policy-ok");
}

#[test]
fn runtime_bus_endpoint_capability_delegates_and_revocation_restores_denial() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let delegate = runtime
        .spawn_process("delegate", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/delegated", ObjectKind::Channel, root)
        .unwrap();
    let peer = runtime
        .create_bus_peer(delegate, domain, "delegate-peer")
        .unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/delegated")
        .unwrap();

    assert_eq!(
        runtime.attach_bus_peer(peer, endpoint),
        Err(RuntimeError::NativeModel(
            NativeModelError::BusAccessDenied {
                owner: delegate,
                endpoint,
                required: CapabilityRights::ADMIN,
            }
        ))
    );

    let endpoint_cap = runtime
        .grant_capability(
            owner,
            endpoint.handle(),
            CapabilityRights::READ
                | CapabilityRights::WRITE
                | CapabilityRights::ADMIN
                | CapabilityRights::DUPLICATE,
            "bus-endpoint-root",
        )
        .unwrap();
    let delegated_io = runtime
        .duplicate_capability(
            endpoint_cap,
            delegate,
            CapabilityRights::READ | CapabilityRights::WRITE,
            "bus-endpoint-delegate-io",
        )
        .unwrap();
    assert_eq!(
        runtime.attach_bus_peer(peer, endpoint),
        Err(RuntimeError::NativeModel(
            NativeModelError::BusAccessDenied {
                owner: delegate,
                endpoint,
                required: CapabilityRights::ADMIN,
            }
        ))
    );
    let delegated = runtime
        .duplicate_capability(
            endpoint_cap,
            delegate,
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::ADMIN,
            "bus-endpoint-delegate-admin",
        )
        .unwrap();

    runtime.attach_bus_peer(peer, endpoint).unwrap();
    assert_eq!(
        runtime.bus_publish(peer, endpoint, b"delegated").unwrap(),
        9
    );
    assert_eq!(runtime.bus_receive(peer, endpoint).unwrap(), b"delegated");
    let delegated_cap_count_before = runtime
        .capabilities
        .objects
        .iter()
        .filter(|(_, capability)| capability.target() == endpoint.handle())
        .count();
    assert!(delegated_cap_count_before >= 2);

    let observe_domain = runtime.create_domain(observer, None, "obs").unwrap();
    let observe_resource = runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();
    let procfs = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/bus")
            .unwrap(),
    )
    .unwrap();
    assert!(procfs.contains("path=/ipc/delegated"));
    assert!(
        procfs.contains("delegated-caps="),
        "procfs bus view did not expose delegation count: {procfs}"
    );

    runtime.revoke_capability(delegated_io).unwrap();
    runtime.revoke_capability(delegated).unwrap();
    let delegated_cap_count_after = runtime
        .capabilities
        .objects
        .iter()
        .filter(|(_, capability)| capability.target() == endpoint.handle())
        .count();
    assert_eq!(delegated_cap_count_after + 2, delegated_cap_count_before);
    assert_eq!(
        runtime.bus_publish(peer, endpoint, b"blocked"),
        Err(RuntimeError::NativeModel(
            NativeModelError::BusAccessDenied {
                owner: delegate,
                endpoint,
                required: CapabilityRights::WRITE,
            }
        ))
    );
    let procfs = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/bus")
            .unwrap(),
    )
    .unwrap();
    assert!(
        procfs.contains("delegated-caps="),
        "procfs bus view did not expose delegation count after revoke: {procfs}"
    );
}

#[test]
fn runtime_bus_requires_admin_rights_for_attach_and_detach_but_allows_io_after_attach() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let delegate = runtime
        .spawn_process("delegate", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/admin-rights", ObjectKind::Channel, root)
        .unwrap();
    let peer = runtime
        .create_bus_peer(delegate, domain, "delegate-peer")
        .unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/admin-rights")
        .unwrap();

    let endpoint_cap = runtime
        .grant_capability(
            owner,
            endpoint.handle(),
            CapabilityRights::READ
                | CapabilityRights::WRITE
                | CapabilityRights::ADMIN
                | CapabilityRights::DUPLICATE,
            "bus-endpoint-root",
        )
        .unwrap();
    let io_only = runtime
        .duplicate_capability(
            endpoint_cap,
            delegate,
            CapabilityRights::READ | CapabilityRights::WRITE,
            "bus-endpoint-io-only",
        )
        .unwrap();

    assert_eq!(
        runtime.attach_bus_peer(peer, endpoint),
        Err(RuntimeError::NativeModel(
            NativeModelError::BusAccessDenied {
                owner: delegate,
                endpoint,
                required: CapabilityRights::ADMIN,
            }
        ))
    );

    runtime
        .revoke_capability(io_only)
        .expect("io-only capability should revoke cleanly");
    let admin_cap = runtime
        .duplicate_capability(
            endpoint_cap,
            delegate,
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::ADMIN,
            "bus-endpoint-admin",
        )
        .unwrap();

    runtime.attach_bus_peer(peer, endpoint).unwrap();
    assert_eq!(runtime.bus_publish(peer, endpoint, b"admin-ok").unwrap(), 8);
    assert_eq!(runtime.bus_receive(peer, endpoint).unwrap(), b"admin-ok");

    runtime
        .revoke_capability(admin_cap)
        .expect("admin capability should revoke cleanly");
    assert_eq!(
        runtime.detach_bus_peer(peer, endpoint),
        Err(RuntimeError::NativeModel(
            NativeModelError::BusAccessDenied {
                owner: delegate,
                endpoint,
                required: CapabilityRights::ADMIN,
            }
        ))
    );

    let recovery_cap = runtime
        .duplicate_capability(
            endpoint_cap,
            delegate,
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::ADMIN,
            "bus-endpoint-admin-recovery",
        )
        .unwrap();
    runtime.detach_bus_peer(peer, endpoint).unwrap();
    assert_eq!(
        runtime.bus_publish(peer, endpoint, b"blocked"),
        Err(RuntimeError::NativeModel(
            NativeModelError::BusPeerNotAttached { peer, endpoint }
        ))
    );
    runtime
        .revoke_capability(recovery_cap)
        .expect("recovery capability should revoke cleanly");
}

#[test]
fn runtime_bus_queue_capacity_refuses_overflow_and_recovers_after_receive() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/capacity", ObjectKind::Channel, root)
        .unwrap();

    let peer = runtime.create_bus_peer(owner, domain, "renderer").unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/capacity")
        .unwrap();
    runtime.attach_bus_peer(peer, endpoint).unwrap();
    let endpoint_info = runtime.bus_endpoint_info(endpoint).unwrap();
    assert_eq!(endpoint_info.queue_capacity, 64);

    for index in 0..endpoint_info.queue_capacity {
        let payload = format!("msg-{index}");
        assert_eq!(
            runtime
                .bus_publish(peer, endpoint, payload.as_bytes())
                .unwrap(),
            payload.len()
        );
    }
    let filled = runtime.bus_endpoint_info(endpoint).unwrap();
    assert_eq!(filled.queue_depth, filled.queue_capacity);
    assert_eq!(filled.peak_queue_depth, filled.queue_capacity);
    assert_eq!(filled.overflow_count, 0);
    assert_eq!(
        runtime.bus_publish(peer, endpoint, b"overflow"),
        Err(RuntimeError::NativeModel(NativeModelError::BusQueueFull {
            endpoint,
            capacity: filled.queue_capacity,
        }))
    );
    let still_filled = runtime.bus_endpoint_info(endpoint).unwrap();
    assert_eq!(still_filled.queue_depth, still_filled.queue_capacity);
    assert_eq!(still_filled.peak_queue_depth, still_filled.queue_capacity);
    assert_eq!(still_filled.overflow_count, 1);

    let first = runtime.bus_receive(peer, endpoint).unwrap();
    assert_eq!(first, b"msg-0");
    let drained = runtime.bus_endpoint_info(endpoint).unwrap();
    assert_eq!(drained.queue_depth + 1, drained.queue_capacity);
    assert_eq!(
        runtime.bus_publish(peer, endpoint, b"recovered").unwrap(),
        9
    );

    let observe_domain = runtime.create_domain(observer, None, "obs").unwrap();
    let observe_resource = runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();
    let procfs = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/bus")
            .unwrap(),
    )
    .unwrap();
    assert!(procfs.contains("path=/ipc/capacity"));
    assert!(procfs.contains("queue-capacity=64"));
    assert!(procfs.contains("queue-peak=64"));
    assert!(procfs.contains("overflows=1"));
}

#[test]
fn runtime_bus_shared_endpoint_preserves_fifo_and_detach_isolates_one_peer() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-bus")
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/shared", ObjectKind::Channel, root)
        .unwrap();

    let peer_a = runtime
        .create_bus_peer(owner, domain, "renderer-a")
        .unwrap();
    let peer_b = runtime
        .create_bus_peer(owner, domain, "renderer-b")
        .unwrap();
    let endpoint = runtime
        .create_bus_channel_endpoint(domain, resource, "/ipc/shared")
        .unwrap();
    runtime.attach_bus_peer(peer_a, endpoint).unwrap();
    runtime.attach_bus_peer(peer_b, endpoint).unwrap();

    assert_eq!(runtime.bus_publish(peer_a, endpoint, b"a-1").unwrap(), 3);
    assert_eq!(runtime.bus_publish(peer_b, endpoint, b"b-1").unwrap(), 3);
    assert_eq!(runtime.bus_publish(peer_a, endpoint, b"a-2").unwrap(), 3);
    assert_eq!(runtime.bus_receive(peer_b, endpoint).unwrap(), b"a-1");
    assert_eq!(runtime.bus_receive(peer_a, endpoint).unwrap(), b"b-1");

    runtime.detach_bus_peer(peer_a, endpoint).unwrap();
    assert_eq!(
        runtime.bus_publish(peer_a, endpoint, b"blocked-a"),
        Err(RuntimeError::NativeModel(
            NativeModelError::BusPeerNotAttached {
                peer: peer_a,
                endpoint,
            }
        ))
    );
    assert_eq!(runtime.bus_publish(peer_b, endpoint, b"b-2").unwrap(), 3);
    assert_eq!(runtime.bus_receive(peer_b, endpoint).unwrap(), b"a-2");
    assert_eq!(runtime.bus_receive(peer_b, endpoint).unwrap(), b"b-2");

    let endpoint_info = runtime.bus_endpoint_info(endpoint).unwrap();
    assert_eq!(endpoint_info.attached_peers, vec![peer_b]);
    assert_eq!(endpoint_info.publish_count, 4);
    assert_eq!(endpoint_info.receive_count, 4);
    assert_eq!(endpoint_info.last_peer, Some(peer_b));

    let observe_domain = runtime.create_domain(observer, None, "obs").unwrap();
    let observe_resource = runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();
    let procfs = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/bus")
            .unwrap(),
    )
    .unwrap();
    assert!(procfs.contains("path=/ipc/shared"));
    assert!(procfs.contains(&format!("peers=[{}]", peer_b.raw())));
}

#[test]
fn runtime_bus_isolates_parallel_endpoints_for_shared_and_distinct_peers() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let owner = runtime
        .spawn_process("owner", None, SchedulerClass::Interactive)
        .unwrap();
    let peer_process = runtime
        .spawn_process("peer-process", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let domain = runtime.create_domain(owner, None, "bus").unwrap();
    let resource_a = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-a")
        .unwrap();
    let resource_b = runtime
        .create_resource(owner, domain, ResourceKind::Channel, "render-b")
        .unwrap();
    let root = runtime
        .grant_capability(
            owner,
            owner.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render-a", ObjectKind::Channel, root)
        .unwrap();
    runtime
        .create_vfs_node("/ipc/render-b", ObjectKind::Channel, root)
        .unwrap();

    let peer_shared = runtime.create_bus_peer(owner, domain, "shared").unwrap();
    let peer_other = runtime
        .create_bus_peer(peer_process, domain, "other")
        .unwrap();
    let endpoint_a = runtime
        .create_bus_channel_endpoint(domain, resource_a, "/ipc/render-a")
        .unwrap();
    let endpoint_b = runtime
        .create_bus_channel_endpoint(domain, resource_b, "/ipc/render-b")
        .unwrap();
    let endpoint_b_cap = runtime
        .grant_capability(
            owner,
            endpoint_b.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "render-b-root",
        )
        .unwrap();
    runtime
        .duplicate_capability(
            endpoint_b_cap,
            peer_process,
            CapabilityRights::READ | CapabilityRights::WRITE,
            "render-b-delegate",
        )
        .unwrap();
    runtime.attach_bus_peer(peer_shared, endpoint_a).unwrap();
    runtime.attach_bus_peer(peer_shared, endpoint_b).unwrap();
    runtime.attach_bus_peer(peer_other, endpoint_b).unwrap();

    assert_eq!(
        runtime
            .bus_publish(peer_shared, endpoint_a, b"a-1")
            .unwrap(),
        3
    );
    assert_eq!(
        runtime
            .bus_publish(peer_shared, endpoint_b, b"b-1")
            .unwrap(),
        3
    );
    assert_eq!(
        runtime.bus_publish(peer_other, endpoint_b, b"b-2").unwrap(),
        3
    );

    assert_eq!(
        runtime.bus_receive(peer_shared, endpoint_a).unwrap(),
        b"a-1"
    );
    assert_eq!(runtime.bus_receive(peer_shared, endpoint_a).unwrap(), b"");
    assert_eq!(runtime.bus_receive(peer_other, endpoint_b).unwrap(), b"b-1");
    assert_eq!(
        runtime.bus_receive(peer_shared, endpoint_b).unwrap(),
        b"b-2"
    );

    let endpoint_a_info = runtime.bus_endpoint_info(endpoint_a).unwrap();
    assert_eq!(endpoint_a_info.queue_depth, 0);
    assert_eq!(endpoint_a_info.publish_count, 1);
    assert_eq!(endpoint_a_info.receive_count, 2);
    assert_eq!(endpoint_a_info.last_peer, Some(peer_shared));

    let endpoint_b_info = runtime.bus_endpoint_info(endpoint_b).unwrap();
    assert_eq!(endpoint_b_info.queue_depth, 0);
    assert_eq!(endpoint_b_info.publish_count, 2);
    assert_eq!(endpoint_b_info.receive_count, 2);
    assert_eq!(
        endpoint_b_info.attached_peers,
        vec![peer_shared, peer_other]
    );
    assert_eq!(endpoint_b_info.last_peer, Some(peer_shared));

    let observe_domain = runtime.create_domain(observer, None, "obs").unwrap();
    let observe_resource = runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();
    let procfs = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/bus")
            .unwrap(),
    )
    .unwrap();
    assert!(procfs.contains("path=/ipc/render-a"));
    assert!(procfs.contains("path=/ipc/render-b"));
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

    let root = runtime
        .grant_capability(
            target,
            ObjectHandle::new(Handle::new(2_601), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let cap = runtime
        .grant_capability(
            target,
            ObjectHandle::new(Handle::new(2_602), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "inspect-cap",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/tmp", ObjectKind::Directory, root)
        .unwrap();
    runtime
        .create_vfs_node("/tmp/observe.txt", ObjectKind::File, cap)
        .unwrap();
    let fd = runtime.open_path(target, "/tmp/observe.txt").unwrap();

    let denied = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/status", target.raw()))
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );
    for path in [
        "stat", "cmdline", "cwd", "environ", "exe", "auxv", "maps", "fd", "caps",
    ] {
        let denied = runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/{}", target.raw(), path))
            .unwrap_err();
        assert_eq!(
            denied,
            RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
                kind: ContractKind::Observe
            })
        );
    }
    let denied = runtime
        .read_procfs_path_for(
            observer,
            &format!("/proc/{}/fdinfo/{}", target.raw(), fd.raw()),
        )
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

    let stat = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/stat", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(stat.contains(&format!("{} ({})", target.raw(), "target")));

    let cmdline = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/cmdline", target.raw()))
        .unwrap();
    assert!(!cmdline.is_empty());

    let cwd = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/cwd", target.raw()))
        .unwrap();
    assert!(!cwd.is_empty());

    let _environ = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/environ", target.raw()))
        .unwrap();

    let exe = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/exe", target.raw()))
        .unwrap();
    assert!(!exe.is_empty());

    let auxv = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/auxv", target.raw()))
        .unwrap();
    assert!(!auxv.is_empty());

    let maps = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/maps", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(!maps.is_empty());

    let fd_view = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/fd", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(fd_view.contains("/tmp/observe.txt"));
    assert!(fd_view.contains(&format!("{}\t", fd.raw())));

    let fdinfo = String::from_utf8(
        runtime
            .read_procfs_path_for(
                observer,
                &format!("/proc/{}/fdinfo/{}", target.raw(), fd.raw()),
            )
            .unwrap(),
    )
    .unwrap();
    assert!(fdinfo.contains("path:\t/tmp/observe.txt"));

    let caps = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/caps", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(caps.contains("inspect-cap"));

    let self_status = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/status", observer.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(self_status.contains(&format!("ObserveContract:\t{}", contract.raw())));
}

#[test]
fn procfs_system_scheduler_renders_trace_and_queue_state() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("init", None, SchedulerClass::BestEffort)
        .unwrap();
    let ui = runtime
        .spawn_process("ui", Some(init), SchedulerClass::Interactive)
        .unwrap();
    let _ = runtime.tick().unwrap();

    let scheduler =
        String::from_utf8(runtime.read_procfs_path("/proc/system/scheduler").unwrap()).unwrap();
    assert!(scheduler.contains("current-tick:"));
    assert!(scheduler.contains("queued-total:"));
    assert!(scheduler.contains("queue\tclass=interactive"));
    assert!(scheduler.contains("policy\tclass=interactive"));
    assert!(scheduler.contains("urgent="));
    assert!(scheduler.contains("starved="));
    assert!(scheduler.contains("starvation-guard="));
    assert!(scheduler.contains("lag-debt="));
    assert!(scheduler.contains("dispatches="));
    assert!(scheduler.contains("runtime-ticks="));
    assert!(scheduler.contains("fairness-dispatch-total:"));
    assert!(scheduler.contains("fairness-runtime-total:"));
    assert!(scheduler.contains("fairness-runtime-imbalance:"));
    assert!(scheduler.contains("meaning="));
    assert!(scheduler.contains("tokens="));
    assert!(scheduler.contains("wait-ticks="));
    assert!(scheduler.contains("decision\ttick="));
    assert!(scheduler.contains(&format!("pid={}", ui.raw())));
}

#[test]
fn procfs_system_scheduler_renders_urgent_policy_after_wake() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let a = runtime
        .spawn_process("wake-a", None, SchedulerClass::Interactive)
        .unwrap();
    let b = runtime
        .spawn_process("wake-b", None, SchedulerClass::Interactive)
        .unwrap();
    let _ = runtime.tick().unwrap();
    assert_eq!(runtime.block_running().unwrap(), a);
    runtime
        .wake_process(b, SchedulerClass::Interactive)
        .unwrap_err();
    runtime
        .wake_process(a, SchedulerClass::Interactive)
        .unwrap();

    let scheduler =
        String::from_utf8(runtime.read_procfs_path("/proc/system/scheduler").unwrap()).unwrap();
    assert!(scheduler.contains("policy\tclass=interactive\turgent=1"));
    assert!(scheduler.contains("starved=false"));
    assert!(scheduler.contains("starvation-guard=8"));
    assert!(scheduler.contains("lag-debt="));
    assert!(scheduler.contains("meaning=wake urgent-requeue=true"));

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.queued_interactive, 2);
    assert_eq!(snapshot.queued_urgent_interactive, 1);
    assert_ne!(snapshot.lag_debt_interactive, 0);
    assert!(snapshot.dispatch_count_interactive >= 1);
    assert_eq!(snapshot.runtime_ticks_interactive, 0);
    assert!(snapshot.scheduler_dispatch_total >= 1);
    assert_eq!(snapshot.scheduler_runtime_ticks_total, 0);
    assert!(!snapshot.starved_interactive);
}

#[test]
fn procfs_system_scheduler_renders_cpu_placement_and_balancing() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.scheduler_logical_cpu_count = 2;
    let mut runtime = KernelRuntime::new(policy);
    let a = runtime
        .spawn_process("cpu-a", None, SchedulerClass::BestEffort)
        .unwrap();
    let b = runtime
        .spawn_process("cpu-b", None, SchedulerClass::BestEffort)
        .unwrap();
    runtime
        .scheduler
        .set_thread_affinity(
            runtime.processes.get(a).unwrap().main_thread().unwrap(),
            0b01,
        )
        .unwrap();
    runtime
        .scheduler
        .set_thread_affinity(
            runtime.processes.get(b).unwrap().main_thread().unwrap(),
            0b10,
        )
        .unwrap();
    let c = runtime
        .spawn_process("cpu-c", None, SchedulerClass::BestEffort)
        .unwrap();
    let d = runtime
        .spawn_process("cpu-d", None, SchedulerClass::BestEffort)
        .unwrap();
    runtime
        .scheduler
        .set_thread_affinity(
            runtime.processes.get(c).unwrap().main_thread().unwrap(),
            0b01,
        )
        .unwrap();
    runtime
        .scheduler
        .set_thread_affinity(
            runtime.processes.get(d).unwrap().main_thread().unwrap(),
            0b10,
        )
        .unwrap();
    let _ = runtime.tick().unwrap();

    let scheduler =
        String::from_utf8(runtime.read_procfs_path("/proc/system/scheduler").unwrap()).unwrap();
    assert!(scheduler.contains("cpu-summary:\tcount=2"));
    assert!(scheduler.contains("rebalance-ops="));
    assert!(scheduler.contains("rebalance-migrations="));
    assert!(scheduler.contains("last-rebalance="));
    assert!(scheduler.contains(
        "cpu\tindex=0\tapic-id=0\tpackage=0\tcore-group=0\tsibling-group=0\tinferred-topology=true\tqueued-load="
    ));
    assert!(scheduler.contains(
        "cpu\tindex=1\tapic-id=1\tpackage=0\tcore-group=0\tsibling-group=1\tinferred-topology=true\tqueued-load="
    ));
    assert!(scheduler.contains("cpu-queue\tindex=0\tclass=best-effort"));
    assert!(scheduler.contains("cpu-queue\tindex=1\tclass=best-effort"));
    assert!(scheduler.contains("tids=["));

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.scheduler_cpu_count, 2);
    assert!(snapshot.scheduler_cpu_load_imbalance <= 1);
}

#[test]
fn procfs_system_scheduler_renders_runtime_policy_topology_handoff() {
    let mut policy = RuntimePolicy::host_runtime_default();
    policy.apply_scheduler_cpu_topology(vec![
        SchedulerCpuTopologyEntry {
            apic_id: 17,
            package_id: 2,
            core_group: 8,
            sibling_group: 0,
            inferred: false,
        },
        SchedulerCpuTopologyEntry {
            apic_id: 29,
            package_id: 2,
            core_group: 8,
            sibling_group: 1,
            inferred: false,
        },
    ]);
    let mut runtime = KernelRuntime::new(policy);
    let _ = runtime
        .spawn_process("topology-a", None, SchedulerClass::BestEffort)
        .unwrap();
    let _ = runtime.tick().unwrap();

    let scheduler =
        String::from_utf8(runtime.read_procfs_path("/proc/system/scheduler").unwrap()).unwrap();
    assert!(scheduler.contains(
        "cpu\tindex=0\tapic-id=17\tpackage=2\tcore-group=8\tsibling-group=0\tinferred-topology=false\tqueued-load="
    ));
    assert!(scheduler.contains(
        "cpu\tindex=1\tapic-id=29\tpackage=2\tcore-group=8\tsibling-group=1\tinferred-topology=false\tqueued-load="
    ));
}

#[test]
fn procfs_system_schedulerepisodes_renders_causal_scheduler_flow() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let a = runtime
        .spawn_process("episode-a", None, SchedulerClass::BestEffort)
        .unwrap();
    let b = runtime
        .spawn_process("episode-b", None, SchedulerClass::Interactive)
        .unwrap();
    let _ = runtime.tick().unwrap();
    runtime
        .wake_process(b, SchedulerClass::Interactive)
        .unwrap_err();
    runtime
        .renice_process(a, SchedulerClass::LatencyCritical, 2)
        .unwrap();
    runtime.set_process_affinity(a, 0b1).unwrap();
    let blocked = runtime.block_running().unwrap();
    runtime
        .wake_process(blocked, SchedulerClass::Interactive)
        .unwrap();
    let _ = runtime.tick().unwrap();

    let episodes = String::from_utf8(
        runtime
            .read_procfs_path("/proc/system/schedulerepisodes")
            .unwrap(),
    )
    .unwrap();
    assert!(episodes.contains("episodes:\t"));
    assert!(episodes.contains("episode\tkind=block"));
    assert!(episodes.contains("causal=running-blocked"));
    assert!(episodes.contains("episode\tkind=wake"));
    assert!(episodes.contains("causal=urgent-requeue"));
    assert!(episodes.contains("episode\tkind=rebind"));
    assert!(
        episodes.contains("causal=running-updated") || episodes.contains("causal=queued-moved")
    );
    assert!(episodes.contains("episode\tkind=affinity"));
    assert!(episodes.contains("causal=cpu-mask-updated"));
    assert!(episodes.contains("episode\tkind=dispatch"));
    assert!(episodes.contains("causal=selected-next-runnable"));
}

#[test]
fn observe_contract_gates_system_scheduler_procfs_reads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("target", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let denied = runtime
        .read_procfs_path_for(observer, "/proc/system/scheduler")
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

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
    runtime.bind_process_contract(observer, contract).unwrap();

    let scheduler = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/scheduler")
            .unwrap(),
    )
    .unwrap();
    assert!(scheduler.contains("decision-tracing:"));
    assert!(scheduler.contains("running:"));
    assert!(scheduler.contains("tokens="));
    assert!(scheduler.contains(&format!("pid={}", target.raw())));
}

#[test]
fn procfs_system_cpu_renders_extended_state_and_observe_contract_gates_reads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    runtime.apply_cpu_extended_state_handoff(CpuExtendedStateHandoff {
        xsave_managed: true,
        save_area_bytes: 4096,
        xcr0_mask: 0xe7,
        boot_probed: true,
        boot_seed_marker: 0xface_cafe,
    });
    let target = runtime
        .spawn_process("target-cpu", None, SchedulerClass::Interactive)
        .unwrap();
    let target_tid = runtime
        .processes()
        .get(target)
        .unwrap()
        .main_thread()
        .unwrap();
    runtime
        .processes
        .mark_thread_cpu_extended_state_saved(target_tid, 12)
        .unwrap();

    let cpu = String::from_utf8(runtime.read_procfs_path("/proc/system/cpu").unwrap()).unwrap();
    assert!(cpu.contains("current-tick:"));
    assert!(cpu.contains("threads:"));
    assert!(cpu.contains(&format!("pid={}", target.raw())));
    assert!(cpu.contains("xsave-managed=true"));
    assert!(cpu.contains("save-area=4096"));
    assert!(cpu.contains("boot-seed=0xfacecafe"));

    let observer = runtime
        .spawn_process("observer-cpu", None, SchedulerClass::Interactive)
        .unwrap();
    let denied = runtime
        .read_procfs_path_for(observer, "/proc/system/cpu")
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

    let domain = runtime.create_domain(observer, None, "obs-cpu").unwrap();
    let resource = runtime
        .create_resource(observer, domain, ResourceKind::Namespace, "inspect-cpu")
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Observe)
        .unwrap();
    let contract = runtime
        .create_contract(
            observer,
            domain,
            resource,
            ContractKind::Observe,
            "observe-cpu",
        )
        .unwrap();
    runtime.bind_process_contract(observer, contract).unwrap();

    let allowed = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/cpu")
            .unwrap(),
    )
    .unwrap();
    assert!(allowed.contains("process\tpid="));
    assert!(allowed.contains("thread\tpid="));
    assert!(allowed.contains("xcr0=0xe7"));
}

#[test]
fn procfs_signals_renders_delivery_and_recovery_state() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("signaled", None, SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_signal_disposition(target, 9, Some(SignalDisposition::Catch), 0, false)
        .unwrap();
    runtime
        .set_signal_mask(target, SignalMaskHow::Block, 1u64 << (9 - 1))
        .unwrap();
    runtime
        .send_signal(
            PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
            target,
            9,
        )
        .unwrap();

    let signals = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/signals", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(signals.contains("blocked-pending:"));
    assert!(signals.contains("pending:\t[9]"));
    assert!(signals.contains("blocked:\t[9]"));
    assert!(signals.contains("signal=9"));

    assert_eq!(
        runtime
            .take_pending_signal(target, 1u64 << (9 - 1), true)
            .unwrap(),
        Some(9)
    );
    let cleared = String::from_utf8(
        runtime
            .read_procfs_path(&format!("/proc/{}/signals", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(cleared.contains("pending:\t[]"));
    assert!(cleared.contains("blocked-pending:\t[]"));
}

#[test]
fn observe_contract_gates_system_signals_procfs_reads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("target", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let denied = runtime
        .read_procfs_path_for(observer, "/proc/system/signals")
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

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
    runtime.bind_process_contract(observer, contract).unwrap();

    let signals = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/signals")
            .unwrap(),
    )
    .unwrap();
    assert!(signals.contains("pid="));
    assert!(signals.contains("wait-mask=0x"));
    assert!(signals.contains("mask=0x"));
    assert!(signals.contains(&format!("pid={}", target.raw())));
}

#[test]
fn observe_contract_gates_process_signals_procfs_reads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let target = runtime
        .spawn_process("signaled", None, SchedulerClass::Interactive)
        .unwrap();
    runtime
        .set_signal_disposition(target, 9, Some(SignalDisposition::Catch), 0, false)
        .unwrap();
    runtime
        .set_signal_mask(target, SignalMaskHow::Block, 1u64 << (9 - 1))
        .unwrap();
    runtime
        .send_signal(
            PendingSignalSender {
                pid: target,
                tid: ThreadId::from_process_id(target),
            },
            target,
            9,
        )
        .unwrap();

    let observer = runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let denied = runtime
        .read_procfs_path_for(observer, &format!("/proc/{}/signals", target.raw()))
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

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
    runtime.bind_process_contract(observer, contract).unwrap();

    let signals = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, &format!("/proc/{}/signals", target.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(signals.contains(&format!("pid:\t{}", target.raw())));
    assert!(signals.contains("pending:\t[9]"));
    assert!(signals.contains("blocked:\t[9]"));
    assert!(signals.contains("blocked-pending:"));
}

#[test]
fn procfs_system_verified_core_renders_report_and_observe_contract_gates_reads() {
    let mut runtime = KernelRuntime::host_runtime_default();
    let init = runtime
        .spawn_process("verified-init", None, SchedulerClass::Interactive)
        .unwrap();
    let root = runtime
        .grant_capability(
            init,
            ObjectHandle::new(Handle::new(8_200), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();

    let verified = String::from_utf8(
        runtime
            .read_procfs_path("/proc/system/verified-core")
            .unwrap(),
    )
    .unwrap();
    assert!(verified.contains("verified:\ttrue"));
    assert!(verified.contains("capability-model:\ttrue"));
    assert!(verified.contains("vfs-invariants:\ttrue"));
    assert!(verified.contains("scheduler-state-machine:\ttrue"));
    assert!(verified.contains("cpu-extended-state-lifecycle:\ttrue"));
    assert!(verified.contains("bus-integrity:\ttrue"));

    let observer = runtime
        .spawn_process("verified-observer", None, SchedulerClass::Interactive)
        .unwrap();
    let denied = runtime
        .read_procfs_path_for(observer, "/proc/system/verified-core")
        .unwrap_err();
    assert_eq!(
        denied,
        RuntimeError::NativeModel(NativeModelError::ProcessContractMissing {
            kind: ContractKind::Observe
        })
    );

    let domain = runtime
        .create_domain(observer, None, "verified-obs")
        .unwrap();
    let resource = runtime
        .create_resource(
            observer,
            domain,
            ResourceKind::Namespace,
            "verified-inspect",
        )
        .unwrap();
    runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Observe)
        .unwrap();
    let contract = runtime
        .create_contract(
            observer,
            domain,
            resource,
            ContractKind::Observe,
            "verified-observe",
        )
        .unwrap();
    runtime.bind_process_contract(observer, contract).unwrap();

    let allowed = String::from_utf8(
        runtime
            .read_procfs_path_for(observer, "/proc/system/verified-core")
            .unwrap(),
    )
    .unwrap();
    assert!(allowed.contains("violations:\t0"));
}

#[test]
fn bus_stress_rapid_publish_receive_cycle() {
    let mut surface = crate::syscall_surface::KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bus-stress", None, SchedulerClass::Interactive)
        .unwrap();

    let domain = match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("stress"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected: {other:?}"),
    };

    let resource = match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("stress-chan"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected: {other:?}"),
    };

    let root = surface
        .runtime
        .grant_capability(
            bootstrap,
            bootstrap.handle(),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "ipc-root",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/ipc", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/ipc/stress", ObjectKind::Channel, root)
        .unwrap();

    let peer = match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("stress-peer"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected: {other:?}"),
    };

    let endpoint = match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource,
                path: String::from("/ipc/stress"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected: {other:?}"),
    };

    surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        )
        .unwrap();

    const MESSAGE_COUNT: usize = 100;
    const BATCH_SIZE: usize = 50;
    let ctx = SyscallContext::kernel(bootstrap);

    for batch in 0..(MESSAGE_COUNT / BATCH_SIZE) {
        for i in 0..BATCH_SIZE {
            let msg_idx = batch * BATCH_SIZE + i;
            let bytes = format!("msg-{}", msg_idx).into_bytes();
            surface
                .dispatch(
                    ctx.clone(),
                    Syscall::PublishBusMessage(PublishBusMessage {
                        peer,
                        endpoint,
                        bytes,
                    }),
                )
                .unwrap();
        }

        for i in 0..BATCH_SIZE {
            let msg_idx = batch * BATCH_SIZE + i;
            let msg = match surface
                .dispatch(
                    ctx.clone(),
                    Syscall::ReceiveBusMessage(ReceiveBusMessage {
                        peer,
                        endpoint,
                    }),
                )
                .unwrap()
            {
                SyscallResult::BusMessageReceived { bytes, .. } => bytes,
                other => panic!("unexpected: {other:?}"),
            };
            let expected = format!("msg-{}", msg_idx).into_bytes();
            assert_eq!(msg, expected);
        }
    }
}
