use super::*;
#[test]
fn syscall_surface_dispatches_runtime_operations() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let child = match surface
        .dispatch(
            context.clone(),
            Syscall::SpawnProcess(SpawnProcess {
                name: String::from("shell"),
                parent: Some(bootstrap),
                class: SchedulerClass::Interactive,
            }),
        )
        .unwrap()
    {
        SyscallResult::ProcessSpawned(pid) => pid,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let capability = match surface
        .dispatch(
            context.clone(),
            Syscall::GrantCapability(GrantCapability {
                owner: bootstrap,
                target: ObjectHandle::new(Handle::new(321), 9),
                rights: CapabilityRights::READ
                    | CapabilityRights::WRITE
                    | CapabilityRights::DUPLICATE,
                label: String::from("vfs-root"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityGranted(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let duplicated = match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateCapability(DuplicateCapability {
                capability,
                new_owner: child,
                rights: CapabilityRights::READ,
                label: String::from("vfs-root-ro"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityDuplicated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    assert_eq!(
        surface
            .runtime()
            .capabilities()
            .get(duplicated)
            .unwrap()
            .owner(),
        child
    );

    match surface.dispatch(context.clone(), Syscall::Tick).unwrap() {
        SyscallResult::Scheduled(process) => {
            assert_eq!(process.pid, bootstrap);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface.dispatch(context, Syscall::Snapshot).unwrap() {
        SyscallResult::Snapshot(snapshot) => {
            let report = surface.runtime().verify_core();
            assert_eq!(snapshot.process_count, 2);
            assert_eq!(snapshot.capability_count, 2);
            assert_eq!(snapshot.verified_core_ok, report.is_verified());
            assert_eq!(
                snapshot.verified_core_violation_count,
                report.violations.len()
            );
            assert_eq!(
                snapshot.capability_model_verified,
                report.capability_model_verified
            );
            assert_eq!(
                snapshot.vfs_invariants_verified,
                report.vfs_invariants_verified
            );
            assert_eq!(
                snapshot.scheduler_state_machine_verified,
                report.scheduler_state_machine_verified
            );
            assert_eq!(
                snapshot.cpu_extended_state_lifecycle_verified,
                report.cpu_extended_state_lifecycle_verified
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_creates_and_inspects_native_model_entities() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("graphics"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let resource = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Device,
                name: String::from("gpu0"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let contract = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateContract(CreateContract {
                issuer: bootstrap,
                domain,
                resource,
                kind: ContractKind::Display,
                label: String::from("scanout"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ContractCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(context.clone(), Syscall::InspectDomain { id: domain })
        .unwrap()
    {
        SyscallResult::DomainInfo(info) => {
            assert_eq!(info.owner, bootstrap);
            assert_eq!(info.name, "graphics");
            assert_eq!(info.resource_count, 1);
            assert_eq!(info.contract_count, 1);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::InspectResource { id: resource })
        .unwrap()
    {
        SyscallResult::ResourceInfo(info) => {
            assert_eq!(info.domain, domain);
            assert_eq!(info.kind, ResourceKind::Device);
            assert_eq!(info.name, "gpu0");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context, Syscall::InspectContract { id: contract })
        .unwrap()
    {
        SyscallResult::ContractInfo(info) => {
            assert_eq!(info.domain, domain);
            assert_eq!(info.resource, resource);
            assert_eq!(info.kind, ContractKind::Display);
            assert_eq!(info.invocation_count, 0);
            assert_eq!(info.label, "scanout");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_routes_bus_messages_and_reports_procfs_state() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let observer = surface
        .runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
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
        .create_vfs_node("/ipc/render", ObjectKind::Channel, root)
        .unwrap();

    let peer = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("renderer"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource,
                path: String::from("/ipc/render"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerAttached {
            peer: attached_peer,
            endpoint: attached_endpoint,
        } => {
            assert_eq!(attached_peer, peer);
            assert_eq!(attached_endpoint, endpoint);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::InspectBusPeer { id: peer })
        .unwrap()
    {
        SyscallResult::BusPeerInfo(info) => {
            assert_eq!(info.owner, bootstrap);
            assert_eq!(info.domain, domain);
            assert_eq!(info.name, "renderer");
            assert_eq!(info.attached_endpoints, vec![endpoint]);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectBusEndpoint { id: endpoint },
        )
        .unwrap()
    {
        SyscallResult::BusEndpointInfo(info) => {
            assert_eq!(info.domain, domain);
            assert_eq!(info.resource, resource);
            assert_eq!(info.kind, BusEndpointKind::Channel);
            assert_eq!(info.path, "/ipc/render");
            assert_eq!(info.attached_peers, vec![peer]);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::ListBusPeers)
        .unwrap()
    {
        SyscallResult::BusPeerList(entries) => {
            assert!(entries.iter().any(|entry| entry.id == peer));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(context.clone(), Syscall::ListBusEndpoints)
        .unwrap()
    {
        SyscallResult::BusEndpointList(entries) => {
            assert!(entries.iter().any(|entry| entry.id == endpoint));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer,
                endpoint,
                bytes: b"hello-bus".to_vec(),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessagePublished {
            peer: published_peer,
            endpoint: published_endpoint,
            bytes,
        } => {
            assert_eq!(published_peer, peer);
            assert_eq!(published_endpoint, endpoint);
            assert_eq!(bytes, 9);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::ReceiveBusMessage(ReceiveBusMessage { peer, endpoint }),
        )
        .unwrap()
    {
        SyscallResult::BusMessageReceived {
            peer: received_peer,
            endpoint: received_endpoint,
            bytes,
        } => {
            assert_eq!(received_peer, peer);
            assert_eq!(received_endpoint, endpoint);
            assert_eq!(bytes, b"hello-bus");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    let observe_domain = surface
        .runtime
        .create_domain(observer, None, "obs")
        .unwrap();
    let observe_resource = surface
        .runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    surface
        .runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = surface
        .runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    surface
        .runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();

    match surface
        .dispatch(
            SyscallContext::kernel(observer),
            Syscall::ReadProcFs {
                path: String::from("/proc/system/bus"),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("bus-peers:\t1"));
            assert!(text.contains("bus-endpoints:\t1"));
            assert!(text.contains("path=/ipc/render"));
            assert!(text.contains("publishes=1"));
            assert!(text.contains("receives=1"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_bus_refuses_unattached_publish_and_recovers_after_reattach() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
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
        .create_vfs_node("/ipc/render", ObjectKind::Channel, root)
        .unwrap();

    let peer = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("renderer"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource,
                path: String::from("/ipc/render"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    surface
        .dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        )
        .unwrap();
    match surface
        .dispatch(
            context.clone(),
            Syscall::DetachBusPeer(DetachBusPeer { peer, endpoint }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerDetached {
            peer: detached_peer,
            endpoint: detached_endpoint,
        } => {
            assert_eq!(detached_peer, peer);
            assert_eq!(detached_endpoint, endpoint);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    assert_eq!(
        surface.dispatch(
            context.clone(),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer,
                endpoint,
                bytes: b"blocked".to_vec(),
            }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::BusPeerNotAttached { peer, endpoint }
        )))
    );

    surface
        .dispatch(
            context,
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        )
        .unwrap();
    match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer,
                endpoint,
                bytes: b"rebound".to_vec(),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessagePublished { bytes, .. } => assert_eq!(bytes, 7),
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_bus_io_contract_policy_refuses_until_bound_and_then_recovers() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    surface
        .runtime
        .set_resource_contract_policy(resource, ResourceContractPolicy::Io)
        .unwrap();

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
        .create_vfs_node("/ipc/policy", ObjectKind::Channel, root)
        .unwrap();

    let peer = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("renderer"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource,
                path: String::from("/ipc/policy"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    assert_eq!(
        surface.dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::ProcessContractMissing {
                kind: ContractKind::Io
            }
        )))
    );

    let foreign_resource = surface
        .runtime
        .create_resource(bootstrap, domain, ResourceKind::Channel, "wrong-bus")
        .unwrap();
    surface
        .runtime
        .set_resource_contract_policy(foreign_resource, ResourceContractPolicy::Io)
        .unwrap();
    let foreign_contract = surface
        .runtime
        .create_contract(
            bootstrap,
            domain,
            foreign_resource,
            ContractKind::Io,
            "wrong-io",
        )
        .unwrap();
    surface
        .runtime
        .bind_process_contract(bootstrap, foreign_contract)
        .unwrap();
    assert_eq!(
        surface.dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::ResourceBindingMismatch
        )))
    );

    let bus_contract = surface
        .runtime
        .create_contract(
            bootstrap,
            domain,
            resource,
            ContractKind::Io,
            "render-bus-io",
        )
        .unwrap();
    surface
        .runtime
        .bind_process_contract(bootstrap, bus_contract)
        .unwrap();
    surface
        .dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        )
        .unwrap();
    match surface
        .dispatch(
            context,
            Syscall::PublishBusMessage(PublishBusMessage {
                peer,
                endpoint,
                bytes: b"policy-ok".to_vec(),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessagePublished { bytes, .. } => assert_eq!(bytes, 9),
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_bus_endpoint_capability_delegates_and_revocation_restores_denial() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let delegate = surface
        .runtime
        .spawn_process("delegate", None, SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
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
        .create_vfs_node("/ipc/delegated", ObjectKind::Channel, root)
        .unwrap();

    let peer = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: delegate,
                domain,
                name: String::from("delegate-peer"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource,
                path: String::from("/ipc/delegated"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    assert_eq!(
        surface.dispatch(
            SyscallContext::kernel(delegate),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::BusAccessDenied {
                owner: delegate,
                endpoint,
                required: CapabilityRights::ADMIN,
            }
        )))
    );

    let root_cap = match surface
        .dispatch(
            context.clone(),
            Syscall::GrantCapability(GrantCapability {
                owner: bootstrap,
                target: endpoint.handle(),
                rights: CapabilityRights::READ
                    | CapabilityRights::WRITE
                    | CapabilityRights::DUPLICATE
                    | CapabilityRights::ADMIN,
                label: String::from("bus-endpoint-root"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityGranted(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateCapability(DuplicateCapability {
                capability: root_cap,
                new_owner: delegate,
                rights: CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::ADMIN,
                label: String::from("bus-endpoint-delegate"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityDuplicated(id) => {
            surface
                .dispatch(
                    SyscallContext::kernel(delegate),
                    Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
                )
                .unwrap();
            surface
                .dispatch(
                    SyscallContext::kernel(delegate),
                    Syscall::PublishBusMessage(PublishBusMessage {
                        peer,
                        endpoint,
                        bytes: b"delegated".to_vec(),
                    }),
                )
                .unwrap();
            surface.runtime.revoke_capability(id).unwrap();
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    assert_eq!(
        surface.dispatch(
            SyscallContext::kernel(delegate),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer,
                endpoint,
                bytes: b"blocked".to_vec(),
            }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::BusAccessDenied {
                owner: delegate,
                endpoint,
                required: CapabilityRights::WRITE,
            }
        )))
    );
}

#[test]
fn syscall_surface_bus_queue_capacity_refuses_overflow_and_recovers_after_receive() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let observer = surface
        .runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
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
        .create_vfs_node("/ipc/capacity", ObjectKind::Channel, root)
        .unwrap();

    let peer = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("renderer"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource,
                path: String::from("/ipc/capacity"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    surface
        .dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
        )
        .unwrap();
    let capacity = match surface
        .dispatch(
            context.clone(),
            Syscall::InspectBusEndpoint { id: endpoint },
        )
        .unwrap()
    {
        SyscallResult::BusEndpointInfo(info) => {
            assert_eq!(info.queue_capacity, 64);
            assert_eq!(info.peak_queue_depth, 0);
            assert_eq!(info.overflow_count, 0);
            info.queue_capacity
        }
        other => panic!("unexpected syscall result: {other:?}"),
    };
    for index in 0..capacity {
        let payload = format!("msg-{index}").into_bytes();
        match surface
            .dispatch(
                context.clone(),
                Syscall::PublishBusMessage(PublishBusMessage {
                    peer,
                    endpoint,
                    bytes: payload.clone(),
                }),
            )
            .unwrap()
        {
            SyscallResult::BusMessagePublished { bytes, .. } => assert_eq!(bytes, payload.len()),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }
    assert_eq!(
        surface.dispatch(
            context.clone(),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer,
                endpoint,
                bytes: b"overflow".to_vec(),
            }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::BusQueueFull { endpoint, capacity }
        )))
    );
    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectBusEndpoint { id: endpoint },
        )
        .unwrap()
    {
        SyscallResult::BusEndpointInfo(info) => {
            assert_eq!(info.queue_depth, info.queue_capacity);
            assert_eq!(info.peak_queue_depth, info.queue_capacity);
            assert_eq!(info.overflow_count, 1);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReceiveBusMessage(ReceiveBusMessage { peer, endpoint }),
        )
        .unwrap()
    {
        SyscallResult::BusMessageReceived { bytes, .. } => assert_eq!(bytes, b"msg-0"),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer,
                endpoint,
                bytes: b"recovered".to_vec(),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessagePublished { bytes, .. } => assert_eq!(bytes, 9),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    let observe_domain = surface
        .runtime
        .create_domain(observer, None, "obs")
        .unwrap();
    let observe_resource = surface
        .runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    surface
        .runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = surface
        .runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    surface
        .runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();
    match surface
        .dispatch(
            SyscallContext::kernel(observer),
            Syscall::ReadProcFs {
                path: String::from("/proc/system/bus"),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("path=/ipc/capacity"));
            assert!(text.contains("queue-capacity=64"));
            assert!(text.contains("queue-peak=64"));
            assert!(text.contains("overflows=1"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_bus_shared_endpoint_preserves_fifo_and_detach_isolates_one_peer() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let observer = surface
        .runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
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
        .create_vfs_node("/ipc/shared", ObjectKind::Channel, root)
        .unwrap();
    let peer_a = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("renderer-a"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let peer_b = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("renderer-b"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource,
                path: String::from("/ipc/shared"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    surface
        .dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer {
                peer: peer_a,
                endpoint,
            }),
        )
        .unwrap();
    surface
        .dispatch(
            context.clone(),
            Syscall::AttachBusPeer(AttachBusPeer {
                peer: peer_b,
                endpoint,
            }),
        )
        .unwrap();

    for (peer, bytes) in [
        (peer_a, b"a-1".to_vec()),
        (peer_b, b"b-1".to_vec()),
        (peer_a, b"a-2".to_vec()),
    ] {
        match surface
            .dispatch(
                context.clone(),
                Syscall::PublishBusMessage(PublishBusMessage {
                    peer,
                    endpoint,
                    bytes: bytes.clone(),
                }),
            )
            .unwrap()
        {
            SyscallResult::BusMessagePublished { bytes: count, .. } => {
                assert_eq!(count, bytes.len())
            }
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReceiveBusMessage(ReceiveBusMessage {
                peer: peer_b,
                endpoint,
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessageReceived { bytes, .. } => assert_eq!(bytes, b"a-1"),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReceiveBusMessage(ReceiveBusMessage {
                peer: peer_a,
                endpoint,
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessageReceived { bytes, .. } => assert_eq!(bytes, b"b-1"),
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface
        .dispatch(
            context.clone(),
            Syscall::DetachBusPeer(DetachBusPeer {
                peer: peer_a,
                endpoint,
            }),
        )
        .unwrap();
    assert_eq!(
        surface.dispatch(
            context.clone(),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer: peer_a,
                endpoint,
                bytes: b"blocked-a".to_vec(),
            }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::BusPeerNotAttached {
                peer: peer_a,
                endpoint
            }
        )))
    );
    surface
        .dispatch(
            context.clone(),
            Syscall::PublishBusMessage(PublishBusMessage {
                peer: peer_b,
                endpoint,
                bytes: b"b-2".to_vec(),
            }),
        )
        .unwrap();
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReceiveBusMessage(ReceiveBusMessage {
                peer: peer_b,
                endpoint,
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessageReceived { bytes, .. } => assert_eq!(bytes, b"a-2"),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReceiveBusMessage(ReceiveBusMessage {
                peer: peer_b,
                endpoint,
            }),
        )
        .unwrap()
    {
        SyscallResult::BusMessageReceived { bytes, .. } => assert_eq!(bytes, b"b-2"),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectBusEndpoint { id: endpoint },
        )
        .unwrap()
    {
        SyscallResult::BusEndpointInfo(info) => {
            assert_eq!(info.attached_peers, vec![peer_b]);
            assert_eq!(info.publish_count, 4);
            assert_eq!(info.receive_count, 4);
            assert_eq!(info.last_peer, Some(peer_b));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    let observe_domain = surface
        .runtime
        .create_domain(observer, None, "obs")
        .unwrap();
    let observe_resource = surface
        .runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    surface
        .runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = surface
        .runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    surface
        .runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();
    match surface
        .dispatch(
            SyscallContext::kernel(observer),
            Syscall::ReadProcFs {
                path: String::from("/proc/system/bus"),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("path=/ipc/shared"));
            assert!(text.contains(&format!("peers=[{}]", peer_b.raw())));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_bus_isolates_parallel_endpoints_for_shared_and_distinct_peers() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let peer_process = surface
        .runtime
        .spawn_process("peer-process", None, SchedulerClass::Interactive)
        .unwrap();
    let observer = surface
        .runtime
        .spawn_process("observer", None, SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("bus"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource_a = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-a"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource_b = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Channel,
                name: String::from("render-b"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
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
        .create_vfs_node("/ipc/render-a", ObjectKind::Channel, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/ipc/render-b", ObjectKind::Channel, root)
        .unwrap();

    let peer_shared = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: bootstrap,
                domain,
                name: String::from("shared"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let peer_other = match surface
        .dispatch(
            SyscallContext::kernel(peer_process),
            Syscall::CreateBusPeer(CreateBusPeer {
                owner: peer_process,
                domain,
                name: String::from("other"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusPeerCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint_a = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource: resource_a,
                path: String::from("/ipc/render-a"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint_b = match surface
        .dispatch(
            context.clone(),
            Syscall::CreateBusEndpoint(CreateBusEndpoint {
                domain,
                resource: resource_b,
                path: String::from("/ipc/render-b"),
            }),
        )
        .unwrap()
    {
        SyscallResult::BusEndpointCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint_b_root = match surface
        .dispatch(
            context.clone(),
            Syscall::GrantCapability(GrantCapability {
                owner: bootstrap,
                target: endpoint_b.handle(),
                rights: CapabilityRights::READ
                    | CapabilityRights::WRITE
                    | CapabilityRights::DUPLICATE
                    | CapabilityRights::ADMIN,
                label: String::from("render-b-root"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityGranted(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let endpoint_a_root = match surface
        .dispatch(
            context.clone(),
            Syscall::GrantCapability(GrantCapability {
                owner: bootstrap,
                target: endpoint_a.handle(),
                rights: CapabilityRights::READ
                    | CapabilityRights::WRITE
                    | CapabilityRights::ADMIN,
                label: String::from("render-a-root"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityGranted(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateCapability(DuplicateCapability {
                capability: endpoint_b_root,
                new_owner: peer_process,
                rights: CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::ADMIN,
                label: String::from("render-b-delegate"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityDuplicated(_) => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    for (caller, peer, endpoint) in [
        (bootstrap, peer_shared, endpoint_a),
        (bootstrap, peer_shared, endpoint_b),
        (peer_process, peer_other, endpoint_b),
    ] {
        surface
            .dispatch(
                SyscallContext::kernel(caller),
                Syscall::AttachBusPeer(AttachBusPeer { peer, endpoint }),
            )
            .unwrap();
    }
    for (caller, peer, endpoint, bytes) in [
        (bootstrap, peer_shared, endpoint_a, b"a-1".to_vec()),
        (bootstrap, peer_shared, endpoint_b, b"b-1".to_vec()),
        (peer_process, peer_other, endpoint_b, b"b-2".to_vec()),
    ] {
        match surface
            .dispatch(
                SyscallContext::kernel(caller),
                Syscall::PublishBusMessage(PublishBusMessage {
                    peer,
                    endpoint,
                    bytes: bytes.clone(),
                }),
            )
            .unwrap()
        {
            SyscallResult::BusMessagePublished { bytes: count, .. } => {
                assert_eq!(count, bytes.len())
            }
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }
    for (caller, peer, endpoint, expected) in [
        (bootstrap, peer_shared, endpoint_a, b"a-1".as_slice()),
        (bootstrap, peer_shared, endpoint_a, b"".as_slice()),
        (peer_process, peer_other, endpoint_b, b"b-1".as_slice()),
        (bootstrap, peer_shared, endpoint_b, b"b-2".as_slice()),
    ] {
        match surface
            .dispatch(
                SyscallContext::kernel(caller),
                Syscall::ReceiveBusMessage(ReceiveBusMessage { peer, endpoint }),
            )
            .unwrap()
        {
            SyscallResult::BusMessageReceived { bytes, .. } => assert_eq!(bytes, expected),
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectBusEndpoint { id: endpoint_a },
        )
        .unwrap()
    {
        SyscallResult::BusEndpointInfo(info) => {
            assert_eq!(info.publish_count, 1);
            assert_eq!(info.receive_count, 2);
            assert_eq!(info.queue_depth, 0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::InspectBusEndpoint { id: endpoint_b },
        )
        .unwrap()
    {
        SyscallResult::BusEndpointInfo(info) => {
            assert_eq!(info.publish_count, 2);
            assert_eq!(info.receive_count, 2);
            assert_eq!(info.queue_depth, 0);
            assert_eq!(info.attached_peers, vec![peer_shared, peer_other]);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    let observe_domain = surface
        .runtime
        .create_domain(observer, None, "obs")
        .unwrap();
    let observe_resource = surface
        .runtime
        .create_resource(
            observer,
            observe_domain,
            ResourceKind::Namespace,
            "inspect-bus",
        )
        .unwrap();
    surface
        .runtime
        .set_resource_contract_policy(observe_resource, ResourceContractPolicy::Observe)
        .unwrap();
    let observe_contract = surface
        .runtime
        .create_contract(
            observer,
            observe_domain,
            observe_resource,
            ContractKind::Observe,
            "observe-bus",
        )
        .unwrap();
    surface
        .runtime
        .bind_process_contract(observer, observe_contract)
        .unwrap();
    match surface
        .dispatch(
            SyscallContext::kernel(observer),
            Syscall::ReadProcFs {
                path: String::from("/proc/system/bus"),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("path=/ipc/render-a"));
            assert!(text.contains("path=/ipc/render-b"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_records_dispatch_trace_and_refusal_paths() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let full_context = SyscallContext::kernel(bootstrap);
    let read_only_context = SyscallContext {
        caller: bootstrap,
        tid: ThreadId::from_process_id(bootstrap),
        authority: CapabilityRights::READ,
    };
    let write_only_context = SyscallContext {
        caller: bootstrap,
        tid: ThreadId::from_process_id(bootstrap),
        authority: CapabilityRights::WRITE,
    };

    let domain = match surface
        .dispatch(
            full_context.clone(),
            Syscall::CreateDomain(CreateDomain {
                owner: bootstrap,
                parent: None,
                name: String::from("trace"),
            }),
        )
        .unwrap()
    {
        SyscallResult::DomainCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let resource = match surface
        .dispatch(
            full_context.clone(),
            Syscall::CreateResource(CreateResource {
                creator: bootstrap,
                domain,
                kind: ResourceKind::Namespace,
                name: String::from("inspect"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ResourceCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let contract = match surface
        .dispatch(
            full_context.clone(),
            Syscall::CreateContract(CreateContract {
                issuer: bootstrap,
                domain,
                resource,
                kind: ContractKind::Observe,
                label: String::from("trace"),
            }),
        )
        .unwrap()
    {
        SyscallResult::ContractCreated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(full_context.clone(), Syscall::Tick)
        .unwrap()
    {
        SyscallResult::Scheduled(process) => assert_eq!(process.pid, bootstrap),
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(full_context.clone(), Syscall::ListDomains)
        .unwrap()
    {
        SyscallResult::DomainList(domains) => {
            assert!(domains.iter().any(|entry| entry.id == domain));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(full_context.clone(), Syscall::InspectDomain { id: domain })
        .unwrap()
    {
        SyscallResult::DomainInfo(info) => {
            assert_eq!(info.id, domain);
            assert_eq!(info.name, "trace");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(full_context.clone(), Syscall::Snapshot)
        .unwrap()
    {
        SyscallResult::Snapshot(snapshot) => {
            let report = surface.runtime().verify_core();
            assert!(snapshot.domain_count >= 1);
            assert!(snapshot.resource_count >= 1);
            assert!(snapshot.contract_count >= 1);
            assert_eq!(snapshot.verified_core_ok, report.is_verified());
            assert_eq!(
                snapshot.verified_core_violation_count,
                report.violations.len()
            );
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    assert_eq!(
        surface.dispatch(read_only_context, Syscall::Tick),
        Err(SyscallError::AccessDenied)
    );
    assert_eq!(
        surface.dispatch(
            write_only_context.clone(),
            Syscall::InspectProcess { pid: bootstrap }
        ),
        Err(SyscallError::AccessDenied)
    );
    assert_eq!(
        surface.dispatch(
            SyscallContext {
                caller: bootstrap,
                tid: ThreadId::from_process_id(bootstrap),
                authority: CapabilityRights::READ,
            },
            Syscall::SetContractState(SetContractState {
                id: contract,
                state: ContractState::Suspended,
            }),
        ),
        Err(SyscallError::AccessDenied)
    );
    assert_eq!(
        surface.dispatch(write_only_context.clone(), Syscall::InspectSystem),
        Err(SyscallError::AccessDenied)
    );

    let system = match surface
        .dispatch(full_context, Syscall::InspectSystem)
        .unwrap()
    {
        SyscallResult::SystemIntrospection(system) => system,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("CreateDomain") && entry.outcome == "ok")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("CreateResource") && entry.outcome == "ok")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("CreateContract") && entry.outcome == "ok")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("ListDomains") && entry.outcome == "ok")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("InspectDomain") && entry.outcome == "ok")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("Snapshot") && entry.outcome == "ok")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("Tick") && entry.outcome == "ok")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("Tick") && entry.outcome == "access-denied")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("InspectProcess")
                && entry.outcome == "access-denied")
    );
    assert!(
        system
            .syscall_dispatches
            .iter()
            .any(|entry| entry.syscall.contains("InspectSystem") && entry.outcome == "ok")
    );
    assert!(
        system.syscall_dispatches.iter().any(
            |entry| entry.syscall.contains("InspectSystem") && entry.outcome == "access-denied"
        )
    );
    assert!(system.syscall_dispatches.iter().any(
        |entry| entry.syscall.contains("SetContractState") && entry.outcome == "access-denied"
    ));
}

#[test]
fn syscall_surface_lists_native_model_entities_and_system_snapshot_counts() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let domain = surface
        .runtime
        .create_domain(bootstrap, None, "storage")
        .unwrap();
    let resource = surface
        .runtime
        .create_resource(bootstrap, domain, ResourceKind::Storage, "nvme0")
        .unwrap();
    let contract = surface
        .runtime
        .create_contract(bootstrap, domain, resource, ContractKind::Io, "journal")
        .unwrap();

    match surface
        .dispatch(context.clone(), Syscall::ListDomains)
        .unwrap()
    {
        SyscallResult::DomainList(domains) => {
            assert!(domains.iter().any(|entry| entry.id == domain));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::ListResources)
        .unwrap()
    {
        SyscallResult::ResourceList(resources) => {
            assert!(resources.iter().any(|entry| entry.id == resource));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::ListContracts)
        .unwrap()
    {
        SyscallResult::ContractList(contracts) => {
            assert!(contracts.iter().any(|entry| entry.id == contract));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface.dispatch(context, Syscall::InspectSystem).unwrap() {
        SyscallResult::SystemIntrospection(system) => {
            let report = surface.runtime().verify_core();
            assert!(system.snapshot.domain_count >= 1);
            assert!(system.snapshot.resource_count >= 1);
            assert!(system.snapshot.contract_count >= 1);
            assert_eq!(system.snapshot.verified_core_ok, report.is_verified());
            assert_eq!(
                system.snapshot.verified_core_violation_count,
                report.violations.len()
            );
            assert!(system.domains.iter().any(|entry| entry.id == domain));
            assert!(system.resources.iter().any(|entry| entry.id == resource));
            assert!(system.contracts.iter().any(|entry| entry.id == contract));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_updates_contract_state() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let domain = surface
        .runtime
        .create_domain(bootstrap, None, "display")
        .unwrap();
    let resource = surface
        .runtime
        .create_resource(bootstrap, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let contract = surface
        .runtime
        .create_contract(
            bootstrap,
            domain,
            resource,
            ContractKind::Display,
            "scanout",
        )
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::SetContractState(SetContractState {
                id: contract,
                state: ContractState::Suspended,
            }),
        )
        .unwrap()
    {
        SyscallResult::ContractStateChanged { id, state } => {
            assert_eq!(id, contract);
            assert_eq!(state, ContractState::Suspended);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context, Syscall::InspectContract { id: contract })
        .unwrap()
    {
        SyscallResult::ContractInfo(info) => {
            assert_eq!(info.state, ContractState::Suspended);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_acquires_and_releases_resource_via_contract() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let domain = surface
        .runtime
        .create_domain(bootstrap, None, "display")
        .unwrap();
    let resource = surface
        .runtime
        .create_resource(bootstrap, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let contract = surface
        .runtime
        .create_contract(
            bootstrap,
            domain,
            resource,
            ContractKind::Display,
            "scanout",
        )
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::AcquireResourceViaContract(AcquireResourceViaContract { contract }),
        )
        .unwrap()
    {
        SyscallResult::ResourceAcquired {
            resource: acquired,
            contract: holder,
            acquire_count,
        } => {
            assert_eq!(acquired, resource);
            assert_eq!(holder, contract);
            assert_eq!(acquire_count, 1);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::InspectResource { id: resource })
        .unwrap()
    {
        SyscallResult::ResourceInfo(info) => {
            assert_eq!(info.holder, Some(contract));
            assert_eq!(info.acquire_count, 1);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::ReleaseResourceViaContract(ReleaseResourceViaContract { contract }),
        )
        .unwrap()
    {
        SyscallResult::ResourceReleased {
            resource: released,
            contract: holder,
        } => {
            assert_eq!(released, resource);
            assert_eq!(holder, contract);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_transfers_resource_between_contracts() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let domain = surface
        .runtime
        .create_domain(bootstrap, None, "display")
        .unwrap();
    let resource = surface
        .runtime
        .create_resource(bootstrap, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let source = surface
        .runtime
        .create_contract(
            bootstrap,
            domain,
            resource,
            ContractKind::Display,
            "scanout",
        )
        .unwrap();
    let target = surface
        .runtime
        .create_contract(bootstrap, domain, resource, ContractKind::Display, "mirror")
        .unwrap();

    surface
        .dispatch(
            context.clone(),
            Syscall::AcquireResourceViaContract(AcquireResourceViaContract { contract: source }),
        )
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::TransferResourceViaContract(TransferResourceViaContract { source, target }),
        )
        .unwrap()
    {
        SyscallResult::ResourceTransferred {
            resource: transferred,
            from,
            to,
            acquire_count,
        } => {
            assert_eq!(transferred, resource);
            assert_eq!(from, source);
            assert_eq!(to, target);
            assert_eq!(acquire_count, 2);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context, Syscall::InspectResource { id: resource })
        .unwrap()
    {
        SyscallResult::ResourceInfo(info) => {
            assert_eq!(info.holder, Some(target));
            assert_eq!(info.acquire_count, 2);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_invokes_only_active_contracts() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let domain = surface
        .runtime
        .create_domain(bootstrap, None, "display")
        .unwrap();
    let resource = surface
        .runtime
        .create_resource(bootstrap, domain, ResourceKind::Device, "gpu0")
        .unwrap();
    let contract = surface
        .runtime
        .create_contract(
            bootstrap,
            domain,
            resource,
            ContractKind::Display,
            "scanout",
        )
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::InvokeContract(InvokeContract { id: contract }),
        )
        .unwrap()
    {
        SyscallResult::ContractInvoked {
            id,
            invocation_count,
        } => {
            assert_eq!(id, contract);
            assert_eq!(invocation_count, 1);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface
        .dispatch(
            context.clone(),
            Syscall::SetContractState(SetContractState {
                id: contract,
                state: ContractState::Suspended,
            }),
        )
        .unwrap();

    assert_eq!(
        surface.dispatch(
            context,
            Syscall::InvokeContract(InvokeContract { id: contract }),
        ),
        Err(SyscallError::Runtime(RuntimeError::NativeModel(
            NativeModelError::ContractNotActive {
                state: ContractState::Suspended,
            }
        )))
    );
}

#[test]
fn syscall_surface_exposes_process_introspection() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    surface
        .runtime
        .set_process_args(
            app,
            vec![
                String::from("app"),
                String::from("--mode"),
                String::from("debug"),
            ],
        )
        .unwrap();
    surface
        .runtime
        .set_process_env(app, vec![String::from("USER=app")])
        .unwrap();
    let cap = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(11_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "asset",
        )
        .unwrap();
    let _fd = surface
        .runtime
        .open_descriptor(app, cap, ObjectKind::File, "/tmp/app.log")
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    match surface
        .dispatch(context.clone(), Syscall::InspectProcess { pid: app })
        .unwrap()
    {
        SyscallResult::ProcessIntrospection(introspection) => {
            let info = introspection.process;
            assert_eq!(info.pid, app);
            assert_eq!(info.parent, Some(bootstrap));
            assert_eq!(info.image_path, "app");
            assert_eq!(info.executable_image.path, "app");
            assert_eq!(info.cwd, "/");
            assert_eq!(info.descriptor_count, 1);
            assert_eq!(info.capability_count, 1);
            assert_eq!(info.environment_count, 1);
            assert_eq!(info.auxiliary_vector_count, 6);
            assert_eq!(info.memory_region_count, 5);
            assert_eq!(info.vm_object_count, 5);
            assert_eq!(introspection.filedesc_entries.len(), 1);
            assert_eq!(introspection.kinfo_file_entries.len(), 1);
            assert_eq!(introspection.vm_object_layouts.len(), 5);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(context.clone(), Syscall::ListProcesses)
        .unwrap()
    {
        SyscallResult::ProcessList(processes) => {
            assert!(processes.iter().any(|process| process.pid == bootstrap));
            assert!(processes.iter().any(|process| process.pid == app));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/status", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("Name:\tapp"));
            assert!(text.contains("Auxv:\t6"));
            assert!(text.contains("Maps:\t5"));
            assert!(text.contains("VmObjects:\t5"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/cmdline", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            assert_eq!(bytes, b"app\0--mode\0debug\0");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/environ", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            assert_eq!(bytes, b"USER=app\0");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/cwd", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            assert_eq!(bytes, b"/");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/exe", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            assert_eq!(bytes, b"app");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/auxv", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("3\t0x"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/maps", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("r-xp 00000000 normal app"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_handles_descriptor_lifecycle() {
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
            ObjectHandle::new(Handle::new(8_000), 1),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "game-device",
        )
        .unwrap();

    let fd0 = match surface
        .dispatch(
            context.clone(),
            Syscall::OpenDescriptor {
                owner: app,
                capability,
                kind: ObjectKind::Device,
                name: String::from("gpu0"),
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorOpened(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let fd1 = match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateDescriptor {
                owner: app,
                fd: fd0,
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorDuplicated(fd) => fd,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    surface
        .runtime
        .set_descriptor_cloexec(app, fd0, true)
        .unwrap();

    match surface
        .dispatch(context.clone(), Syscall::ExecTransition { owner: app })
        .unwrap()
    {
        SyscallResult::ExecTransitioned(closed) => {
            assert_eq!(closed.len(), 1);
            assert_eq!(closed[0].fd(), fd0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::CloseDescriptor {
                owner: app,
                fd: fd1,
            },
        )
        .unwrap()
    {
        SyscallResult::DescriptorClosed(descriptor) => {
            assert_eq!(descriptor.name(), "gpu0");
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_execs_processes_with_image_updates() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_100), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "root",
        )
        .unwrap();
    let bin = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_101), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "bin",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/bin", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/srv", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/bin/editor", ObjectKind::File, bin)
        .unwrap();
    surface.runtime.set_process_cwd(app, "/srv").unwrap();
    let fd0 = surface.runtime.open_path(app, "/bin/editor").unwrap();
    surface.runtime.duplicate_descriptor(app, fd0).unwrap();
    surface
        .runtime
        .set_descriptor_cloexec(app, fd0, true)
        .unwrap();

    match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::ExecProcess(ExecProcess {
                pid: app,
                path: String::from("/bin/editor"),
                argv: vec![String::from("editor"), String::from("notes.txt")],
                envp: vec![String::from("EDITOR=vi")],
            }),
        )
        .unwrap()
    {
        SyscallResult::ExecTransitioned(closed) => {
            assert_eq!(closed.len(), 1);
            assert_eq!(closed[0].fd(), fd0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    assert_eq!(
        surface.runtime.descriptors_for(app).unwrap(),
        vec![Descriptor::new(0), Descriptor::new(1), Descriptor::new(2)]
    );
    let info = surface.runtime.process_info(app).unwrap();
    assert_eq!(info.name, "editor");
    assert_eq!(info.image_path, "/bin/editor");
    assert_eq!(info.executable_image.path, "/bin/editor");
    assert_eq!(info.cwd, "/srv");
    assert_eq!(info.environment_count, 1);
    assert_eq!(info.auxiliary_vector_count, 6);
    assert_eq!(info.memory_region_count, 5);
}

#[test]
fn syscall_surface_supports_vm_mapping_operations() {
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

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapAnonymousMemory(MapAnonymousMemory {
                pid: app,
                length: 0x3000,
                readable: true,
                writable: true,
                executable: false,
                label: String::from("jit-cache"),
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    match surface
        .dispatch(
            context.clone(),
            Syscall::TouchMemory(TouchMemory {
                pid: app,
                start: mapped,
                length: 0x2000,
                write: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryTouched(stats) => {
            assert_eq!(stats.pages_touched, 2);
            assert_eq!(stats.faulted_pages, 2);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::UnmapMemory(UnmapMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryUnmapped => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    let new_brk = match surface
        .dispatch(
            context.clone(),
            Syscall::SetProcessBreak(SetProcessBreak {
                pid: app,
                new_end: 0x0041_5000,
            }),
        )
        .unwrap()
    {
        SyscallResult::ProcessBreak(end) => end,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    assert!(new_brk >= 0x0041_5000);

    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/maps", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("rw-p 00000000 normal [heap]"));
            assert!(!text.contains("[anon:jit-cache]"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_supports_file_mapping_and_mprotect() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_500), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_501), 0),
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
        .create_vfs_node("/lib/libgpu.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/libgpu.so"),
                length: 0x3000,
                file_offset: 0,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::ProtectMemory(ProtectMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                readable: true,
                writable: true,
                executable: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::TouchMemory(TouchMemory {
                pid: app,
                start: mapped,
                length: 0x2000,
                write: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryTouched(stats) => {
            assert_eq!(stats.pages_touched, 2);
            assert_eq!(stats.faulted_pages, 2);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/maps", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("rw-p 00000000 normal /lib/libgpu.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_splits_regions_for_partial_vm_operations() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_700), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_701), 0),
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
        .create_vfs_node("/lib/libpartial.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/libpartial.so"),
                length: 0x3000,
                file_offset: 0x7000,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::ProtectMemory(ProtectMemory {
                pid: app,
                start: mapped + 0x1000,
                length: 0x1000,
                readable: true,
                writable: true,
                executable: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::AdviseMemory(AdviseMemory {
                pid: app,
                start: mapped + 0x2000,
                length: 0x1000,
                advice: MemoryAdvice::Sequential,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/maps", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("r-xp 00007000 normal /lib/libpartial.so"));
            assert!(text.contains("rw-p 00008000 normal /lib/libpartial.so"));
            assert!(text.contains("r-xp 00009000 seq /lib/libpartial.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_coalesces_regions_after_restoring_vm_semantics() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_704), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_705), 0),
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
        .create_vfs_node("/lib/libmerge.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/libmerge.so"),
                length: 0x3000,
                file_offset: 0x5000,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    for (writable, executable) in [(true, false), (false, true)] {
        match surface
            .dispatch(
                context.clone(),
                Syscall::ProtectMemory(ProtectMemory {
                    pid: app,
                    start: mapped + 0x1000,
                    length: 0x1000,
                    readable: true,
                    writable,
                    executable,
                }),
            )
            .unwrap()
        {
            SyscallResult::DescriptorFlagsUpdated => {}
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    for advice in [MemoryAdvice::Sequential, MemoryAdvice::Normal] {
        match surface
            .dispatch(
                context.clone(),
                Syscall::AdviseMemory(AdviseMemory {
                    pid: app,
                    start: mapped + 0x1000,
                    length: 0x1000,
                    advice,
                }),
            )
            .unwrap()
        {
            SyscallResult::DescriptorFlagsUpdated => {}
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::SyncMemory(SyncMemory {
                pid: app,
                start: mapped + 0x1000,
                length: 0x1000,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/maps", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert_eq!(text.matches("/lib/libmerge.so").count(), 1);
            assert!(text.contains("r-xp 00005000 normal /lib/libmerge.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_vm_range_operations_span_split_regions() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_706), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_707), 0),
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
        .create_vfs_node("/lib/librange.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/librange.so"),
                length: 0x3000,
                file_offset: 0x6000,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(
            context.clone(),
            Syscall::ProtectMemory(ProtectMemory {
                pid: app,
                start: mapped + 0x1000,
                length: 0x1000,
                readable: true,
                writable: true,
                executable: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::ProtectMemory(ProtectMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                readable: true,
                writable: true,
                executable: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::AdviseMemory(AdviseMemory {
                pid: app,
                start: mapped + 0x1000,
                length: 0x1000,
                advice: MemoryAdvice::DontNeed,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::AdviseMemory(AdviseMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                advice: MemoryAdvice::Normal,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    surface
        .runtime
        .touch_memory(app, mapped + 0x1000, 0x1000, true)
        .unwrap();
    match surface
        .dispatch(
            context.clone(),
            Syscall::SyncMemory(SyncMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/maps", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert_eq!(text.matches("/lib/librange.so").count(), 1);
            assert!(text.contains("rw-p 00006000 normal /lib/librange.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/vmobjects", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("dirty=0"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_touch_memory_spans_split_regions_and_aggregates_stats() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_708), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_709), 0),
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
        .create_vfs_node("/lib/libtouch.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/libtouch.so"),
                length: 0x3000,
                file_offset: 0x7000,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    for start in [mapped + 0x1000, mapped] {
        let length = if start == mapped { 0x3000 } else { 0x1000 };
        match surface
            .dispatch(
                context.clone(),
                Syscall::ProtectMemory(ProtectMemory {
                    pid: app,
                    start,
                    length,
                    readable: true,
                    writable: true,
                    executable: false,
                }),
            )
            .unwrap()
        {
            SyscallResult::DescriptorFlagsUpdated => {}
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::TouchMemory(TouchMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                write: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryTouched(stats) => {
            assert_eq!(stats.pages_touched, 3);
            assert_eq!(stats.faulted_pages, 3);
            assert_eq!(stats.cow_faulted_pages, 0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/vmobjects", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("committed=3\tresident=3\tdirty=3\taccessed=3"));
            assert!(text.contains("faults=3(r=0,w=3,cow=0)\t/lib/libtouch.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_touch_memory_spans_split_cow_regions_and_aggregates_stats() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let parent = surface
        .runtime
        .spawn_process("parent", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let scratch = surface
        .runtime
        .map_anonymous_memory(parent, 0x3000, true, true, false, "cow-touch-range")
        .unwrap();
    surface
        .runtime
        .protect_memory(parent, scratch + 0x1000, 0x1000, true, false, false)
        .unwrap();
    surface
        .runtime
        .protect_memory(parent, scratch, 0x3000, true, true, false)
        .unwrap();
    let child = surface
        .runtime
        .spawn_process_copy_vm(
            "child",
            Some(bootstrap),
            SchedulerClass::Interactive,
            parent,
        )
        .unwrap();

    match surface
        .dispatch(
            context.clone(),
            Syscall::TouchMemory(TouchMemory {
                pid: child,
                start: scratch,
                length: 0x3000,
                write: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryTouched(stats) => {
            assert_eq!(stats.pages_touched, 3);
            assert_eq!(stats.faulted_pages, 3);
            assert_eq!(stats.cow_faulted_pages, 3);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
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
fn syscall_surface_mixed_faults_across_split_regions_preserve_read_write_counters() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_710), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_711), 0),
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
        .create_vfs_node("/lib/libmixed.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/libmixed.so"),
                length: 0x3000,
                file_offset: 0x8000,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    for start in [mapped + 0x1000, mapped] {
        let length = if start == mapped { 0x3000 } else { 0x1000 };
        match surface
            .dispatch(
                context.clone(),
                Syscall::ProtectMemory(ProtectMemory {
                    pid: app,
                    start,
                    length,
                    readable: true,
                    writable: true,
                    executable: false,
                }),
            )
            .unwrap()
        {
            SyscallResult::DescriptorFlagsUpdated => {}
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::TouchMemory(TouchMemory {
                pid: app,
                start: mapped,
                length: 0x1000,
                write: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryTouched(stats) => {
            assert_eq!(stats.faulted_pages, 1);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::TouchMemory(TouchMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                write: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryTouched(stats) => {
            assert_eq!(stats.pages_touched, 3);
            assert_eq!(stats.faulted_pages, 2);
            assert_eq!(stats.cow_faulted_pages, 0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/vmobjects", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("committed=3\tresident=3\tdirty=1\taccessed=3"));
            assert!(text.contains("faults=3(r=2,w=1,cow=0)\t/lib/libmixed.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_madvise_dontneed_evicts_pages_and_willneed_prefaults_them() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_712), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_713), 0),
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
        .create_vfs_node("/lib/libadvise.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/libadvise.so"),
                length: 0x3000,
                file_offset: 0x9000,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    match surface
        .dispatch(
            context.clone(),
            Syscall::ProtectMemory(ProtectMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                readable: true,
                writable: true,
                executable: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    surface
        .runtime
        .touch_memory(app, mapped, 0x3000, true)
        .unwrap();

    for advice in [MemoryAdvice::DontNeed, MemoryAdvice::WillNeed] {
        let (start, length) = if advice == MemoryAdvice::DontNeed {
            (mapped + 0x1000, 0x1000)
        } else {
            (mapped, 0x3000)
        };
        match surface
            .dispatch(
                context.clone(),
                Syscall::AdviseMemory(AdviseMemory {
                    pid: app,
                    start,
                    length,
                    advice,
                }),
            )
            .unwrap()
        {
            SyscallResult::DescriptorFlagsUpdated => {}
            other => panic!("unexpected syscall result: {other:?}"),
        }
    }

    match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/vmobjects", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("committed=3\tresident=3\tdirty=2\taccessed=3"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::AdviseMemory(AdviseMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                advice: MemoryAdvice::DontNeed,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::AdviseMemory(AdviseMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                advice: MemoryAdvice::WillNeed,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context.clone(),
            Syscall::TouchMemory(TouchMemory {
                pid: app,
                start: mapped,
                length: 0x3000,
                write: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryTouched(stats) => {
            assert_eq!(stats.faulted_pages, 0);
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/vmobjects", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("committed=3\tresident=3\tdirty=0\taccessed=3"));
            assert!(text.contains("faults=3(r=0,w=3,cow=0)\t/lib/libadvise.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_mprotect_does_not_dirty_pages_without_writes() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_714), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let lib = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_715), 0),
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
        .create_vfs_node("/lib/libprot.so", ObjectKind::File, lib)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let mapped = match surface
        .dispatch(
            context.clone(),
            Syscall::MapFileMemory(MapFileMemory {
                pid: app,
                path: String::from("/lib/libprot.so"),
                length: 0x2000,
                file_offset: 0xa000,
                readable: true,
                writable: false,
                executable: true,
                private: true,
            }),
        )
        .unwrap()
    {
        SyscallResult::MemoryMapped(start) => start,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    match surface
        .dispatch(
            context.clone(),
            Syscall::ProtectMemory(ProtectMemory {
                pid: app,
                start: mapped,
                length: 0x2000,
                readable: true,
                writable: true,
                executable: false,
            }),
        )
        .unwrap()
    {
        SyscallResult::DescriptorFlagsUpdated => {}
        other => panic!("unexpected syscall result: {other:?}"),
    }
    match surface
        .dispatch(
            context,
            Syscall::ReadProcFs {
                path: format!("/proc/{}/vmobjects", app.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("committed=2\tresident=0\tdirty=0\taccessed=0"));
            assert!(text.contains("faults=0(r=0,w=0,cow=0)\t/lib/libprot.so"));
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
}

#[test]
fn syscall_surface_can_spawn_processes_with_copied_vm_state() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let root = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_800), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "root",
        )
        .unwrap();
    let bin = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_801), 0),
            CapabilityRights::READ | CapabilityRights::WRITE,
            "bin",
        )
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/bin", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/work", ObjectKind::Directory, root)
        .unwrap();
    surface
        .runtime
        .create_vfs_node("/bin/app", ObjectKind::File, bin)
        .unwrap();
    surface.runtime.set_process_cwd(app, "/work").unwrap();
    surface
        .runtime
        .exec_process(
            app,
            "/bin/app",
            vec![String::from("app"), String::from("--spawned")],
            vec![String::from("MODE=test")],
        )
        .unwrap();
    surface
        .runtime
        .map_anonymous_memory(app, 0x2000, true, true, false, "vm-copy")
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);

    let child = match surface
        .dispatch(
            context.clone(),
            Syscall::SpawnProcessCopyVm(SpawnProcessWithVm {
                name: String::from("child"),
                parent: Some(bootstrap),
                class: SchedulerClass::Interactive,
                source: app,
            }),
        )
        .unwrap()
    {
        SyscallResult::ProcessSpawned(pid) => pid,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    match surface
        .dispatch(context, Syscall::InspectProcess { pid: child })
        .unwrap()
    {
        SyscallResult::ProcessIntrospection(introspection) => {
            let info = introspection.process;
            assert_eq!(info.image_path, "/bin/app");
            assert_eq!(info.cwd, "/work");
            assert!(info.shared_memory_region_count >= 1);
            assert!(info.copy_on_write_region_count >= 1);
            assert!(!introspection.vm_object_layouts.is_empty());
        }
        other => panic!("unexpected syscall result: {other:?}"),
    }
    let maps = String::from_utf8(
        surface
            .runtime
            .read_procfs_path(&format!("/proc/{}/maps", child.raw()))
            .unwrap(),
    )
    .unwrap();
    assert!(maps.contains("/bin/app"));
    assert!(maps.contains("[anon:vm-copy]"));
    assert!(maps.contains("refs=2"));
    assert!(maps.contains("cow"));
}

#[test]
fn syscall_surface_can_spawn_processes_from_source_with_combined_modes() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let app = surface
        .runtime
        .spawn_process("app", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let cap = surface
        .runtime
        .grant_capability(
            app,
            ObjectHandle::new(Handle::new(12_850), 0),
            CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
            "share",
        )
        .unwrap();
    let fd = surface
        .runtime
        .open_descriptor(app, cap, ObjectKind::Socket, "/run/syscall-share.sock")
        .unwrap();
    let mapped = surface
        .runtime
        .map_anonymous_memory(app, 0x2000, true, true, false, "syscall-combined")
        .unwrap();

    let child = match surface
        .dispatch(
            SyscallContext::kernel(bootstrap),
            Syscall::SpawnProcessFromSource(SpawnProcessFromSource {
                name: String::from("forked"),
                parent: Some(bootstrap),
                class: SchedulerClass::Interactive,
                source: app,
                filedesc_mode: SpawnFiledescMode::Copy,
                vm_mode: SpawnVmMode::Copy,
            }),
        )
        .unwrap()
    {
        SyscallResult::ProcessSpawned(pid) => pid,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let info = surface.runtime.process_info(child).unwrap();
    assert!(info.copy_on_write_region_count >= 1);
    assert_eq!(surface.runtime.filedesc_entries(child).unwrap().len(), 1);
    let touch = surface
        .runtime
        .touch_memory(child, mapped, 0x1000, true)
        .unwrap();
    assert_eq!(touch.cow_faulted_pages, 1);

    surface.runtime.close_descriptor(app, fd).unwrap();
    assert_eq!(surface.runtime.filedesc_entries(child).unwrap().len(), 1);
}

#[test]
fn syscall_surface_enforces_authority() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let limited = SyscallContext {
        caller: bootstrap,
        tid: ThreadId::from_process_id(bootstrap),
        authority: CapabilityRights::READ,
    };

    assert_eq!(
        surface.dispatch(
            limited,
            Syscall::SpawnProcess(SpawnProcess {
                name: String::from("forbidden"),
                parent: Some(bootstrap),
                class: SchedulerClass::BestEffort,
            }),
        ),
        Err(SyscallError::AccessDenied)
    );
}
