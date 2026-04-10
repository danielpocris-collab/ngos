use super::*;
use crate::eventing_model::{BusEventKind, GraphicsEventKind};

fn emit_graphics_resource_event(
    runtime: &mut KernelRuntime,
    resource: ResourceId,
    contract: ContractId,
    kind: GraphicsEventKind,
) -> Result<(), RuntimeError> {
    let info = runtime.resource_info(resource)?;
    if info.kind != ResourceKind::Device {
        return Ok(());
    }
    let device_path = format!("/dev/{}", info.name);
    let Ok(device) = runtime.device_info_by_path(&device_path) else {
        return Ok(());
    };
    if device.class != DeviceClass::Graphics {
        return Ok(());
    }
    let device_inode = runtime.stat_path(&device_path)?.inode;
    event_queue_runtime::emit_graphics_events(runtime, device_inode, contract.raw(), kind)
}

fn cancel_graphics_resource_requests(
    runtime: &mut KernelRuntime,
    resource: ResourceId,
    contract: ContractId,
) -> Result<(), RuntimeError> {
    let resource_info = runtime.resource_info(resource)?;
    if resource_info.kind != ResourceKind::Device {
        return Ok(());
    }
    let device_path = format!("/dev/{}", resource_info.name);
    let device = match runtime.device_info_by_path(&device_path) {
        Ok(device) => device,
        Err(_) => return Ok(()),
    };
    if device.class != DeviceClass::Graphics {
        return Ok(());
    }
    let contract_info = runtime.contract_info(contract)?;
    let _ = runtime.cancel_graphics_requests_for_issuer(&device_path, contract_info.issuer)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceClaimPlan {
    Claim {
        resource: ResourceId,
        contract: ContractId,
    },
    Queue {
        resource: ResourceId,
        contract: ContractId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceReleasePlan {
    ReleaseOnly {
        resource: ResourceId,
        contract: ContractId,
    },
    ReleaseAndHandoff {
        resource: ResourceId,
        contract: ContractId,
        next: ContractId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResourceStateTransitionPlan {
    Activate {
        resource: ResourceId,
    },
    Suspend {
        resource: ResourceId,
        holder: Option<ContractId>,
        waiters: Vec<ContractId>,
    },
    Retire {
        resource: ResourceId,
        revoke_contracts: Vec<ContractId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContractStateTransitionPlan {
    Activate {
        contract: ContractId,
    },
    Deactivate {
        contract: ContractId,
        resource: ResourceId,
        was_holder: bool,
        was_waiter: bool,
        emit_revoked: bool,
    },
}

struct ClaimValidator;
struct CancelValidator;
struct ReleaseValidator;
struct ResourceStateTransitionAgent;
struct ContractStateTransitionAgent;

impl KernelRuntime {
    fn has_bus_endpoint_capability(
        &self,
        owner: ProcessId,
        endpoint: BusEndpointId,
        required: CapabilityRights,
    ) -> bool {
        self.capabilities.objects.iter().any(|(_, capability)| {
            capability.owner() == owner
                && capability.target() == endpoint.handle()
                && capability.rights().contains(required)
        })
    }

    fn enforce_bus_resource_policy(
        &self,
        owner: ProcessId,
        endpoint: BusEndpointId,
        required: CapabilityRights,
    ) -> Result<(), RuntimeError> {
        let endpoint_info = self.bus_endpoint_info(endpoint)?;
        let resource_info = self.resource_info(endpoint_info.resource)?;
        match resource_info.contract_policy {
            ResourceContractPolicy::Io => {
                let Some((contract_info, _)) =
                    self.require_process_contract(owner, ContractKind::Io)?
                else {
                    return Err(RuntimeError::NativeModel(
                        NativeModelError::ProcessContractMissing {
                            kind: ContractKind::Io,
                        },
                    ));
                };
                if contract_info.resource != endpoint_info.resource {
                    return Err(RuntimeError::NativeModel(
                        NativeModelError::ResourceBindingMismatch,
                    ));
                }
                Ok(())
            }
            ResourceContractPolicy::Any
            | ResourceContractPolicy::Execution
            | ResourceContractPolicy::Memory
            | ResourceContractPolicy::Device
            | ResourceContractPolicy::Display
            | ResourceContractPolicy::Observe => {
                if resource_info.creator == owner
                    || self.has_bus_endpoint_capability(owner, endpoint, required)
                {
                    Ok(())
                } else {
                    Err(RuntimeError::NativeModel(
                        NativeModelError::BusAccessDenied {
                            owner,
                            endpoint,
                            required,
                        },
                    ))
                }
            }
        }
    }

    pub fn create_bus_peer(
        &mut self,
        owner: ProcessId,
        domain: DomainId,
        name: impl Into<String>,
    ) -> Result<BusPeerId, RuntimeError> {
        self.bus_peers
            .create(&self.processes, &self.domains, owner, domain, name)
            .map_err(Into::into)
    }

    pub fn create_bus_channel_endpoint(
        &mut self,
        domain: DomainId,
        resource: ResourceId,
        path: impl Into<String>,
    ) -> Result<BusEndpointId, RuntimeError> {
        let path = path.into();
        let resource_info = self.resource_info(resource)?;
        if resource_info.kind != ResourceKind::Channel {
            return Err(RuntimeError::NativeModel(
                NativeModelError::BusEndpointKindMismatch,
            ));
        }
        let status = self.stat_path(&path)?;
        if status.kind != ObjectKind::Channel {
            return Err(RuntimeError::NativeModel(
                NativeModelError::BusEndpointKindMismatch,
            ));
        }
        self.bus_endpoints
            .create(
                &self.domains,
                &self.resources,
                domain,
                resource,
                BusEndpointKind::Channel,
                path,
            )
            .map_err(Into::into)
    }

    pub fn attach_bus_peer(
        &mut self,
        peer: BusPeerId,
        endpoint: BusEndpointId,
    ) -> Result<(), RuntimeError> {
        let endpoint_domain = self.bus_endpoints.get(endpoint)?.domain;
        let peer_owner = self.bus_peers.get(peer)?.owner;
        let peer_domain = self.bus_peers.get(peer)?.domain;
        if peer_domain != endpoint_domain {
            return Err(RuntimeError::NativeModel(NativeModelError::ParentMismatch));
        }
        self.enforce_bus_resource_policy(peer_owner, endpoint, CapabilityRights::ADMIN)?;
        let peer_entry = self.bus_peers.get_mut(peer)?;
        if !peer_entry.attached_endpoints.contains(&endpoint) {
            peer_entry.attached_endpoints.push(endpoint);
        }
        let endpoint_entry = self.bus_endpoints.get_mut(endpoint)?;
        if !endpoint_entry.attached_peers.contains(&peer) {
            endpoint_entry.attached_peers.push(peer);
        }
        event_queue_runtime::emit_bus_events(self, peer, endpoint, BusEventKind::Attached)?;
        Ok(())
    }

    pub fn detach_bus_peer(
        &mut self,
        peer: BusPeerId,
        endpoint: BusEndpointId,
    ) -> Result<(), RuntimeError> {
        let peer_owner = self.bus_peers.get(peer)?.owner;
        self.enforce_bus_resource_policy(peer_owner, endpoint, CapabilityRights::ADMIN)?;
        let peer_entry = self.bus_peers.get_mut(peer)?;
        peer_entry
            .attached_endpoints
            .retain(|candidate| *candidate != endpoint);
        let endpoint_entry = self.bus_endpoints.get_mut(endpoint)?;
        endpoint_entry
            .attached_peers
            .retain(|candidate| *candidate != peer);
        event_queue_runtime::emit_bus_events(self, peer, endpoint, BusEventKind::Detached)?;
        Ok(())
    }

    pub fn bus_publish(
        &mut self,
        peer: BusPeerId,
        endpoint: BusEndpointId,
        bytes: &[u8],
    ) -> Result<usize, RuntimeError> {
        let endpoint_info = self.bus_endpoint_info(endpoint)?;
        let peer_owner = self.bus_peers.get(peer)?.owner;
        if endpoint_info.kind != BusEndpointKind::Channel {
            return Err(RuntimeError::NativeModel(
                NativeModelError::BusEndpointKindMismatch,
            ));
        }
        self.enforce_bus_resource_policy(peer_owner, endpoint, CapabilityRights::WRITE)?;
        let attached = self
            .bus_peers
            .get(peer)?
            .attached_endpoints
            .contains(&endpoint);
        if !attached {
            return Err(RuntimeError::NativeModel(
                NativeModelError::BusPeerNotAttached { peer, endpoint },
            ));
        }
        if let Some(channel) = self
            .runtime_channels
            .iter_mut()
            .find(|channel| channel.path == endpoint_info.path)
        {
            if channel.messages.len() >= endpoint_info.queue_capacity {
                let endpoint_entry = self.bus_endpoints.get_mut(endpoint)?;
                endpoint_entry.overflow_count = endpoint_entry.overflow_count.saturating_add(1);
                return Err(RuntimeError::NativeModel(NativeModelError::BusQueueFull {
                    endpoint,
                    capacity: endpoint_info.queue_capacity,
                }));
            }
            channel.messages.push(bytes.to_vec());
        } else {
            self.runtime_channels.push(RuntimeChannel {
                path: endpoint_info.path.clone(),
                messages: vec![bytes.to_vec()],
            });
        }
        let peer_entry = self.bus_peers.get_mut(peer)?;
        peer_entry.publish_count = peer_entry.publish_count.saturating_add(1);
        peer_entry.last_endpoint = Some(endpoint);
        let endpoint_entry = self.bus_endpoints.get_mut(endpoint)?;
        endpoint_entry.publish_count = endpoint_entry.publish_count.saturating_add(1);
        endpoint_entry.byte_count = endpoint_entry.byte_count.saturating_add(bytes.len() as u64);
        endpoint_entry.queue_depth = endpoint_entry.queue_depth.saturating_add(1);
        endpoint_entry.peak_queue_depth = endpoint_entry
            .peak_queue_depth
            .max(endpoint_entry.queue_depth);
        endpoint_entry.last_peer = Some(peer);
        event_queue_runtime::emit_bus_events(self, peer, endpoint, BusEventKind::Published)?;
        Ok(bytes.len())
    }

    pub fn bus_receive(
        &mut self,
        peer: BusPeerId,
        endpoint: BusEndpointId,
    ) -> Result<Vec<u8>, RuntimeError> {
        let endpoint_info = self.bus_endpoint_info(endpoint)?;
        let peer_owner = self.bus_peers.get(peer)?.owner;
        self.enforce_bus_resource_policy(peer_owner, endpoint, CapabilityRights::READ)?;
        let attached = self
            .bus_peers
            .get(peer)?
            .attached_endpoints
            .contains(&endpoint);
        if !attached {
            return Err(RuntimeError::NativeModel(
                NativeModelError::BusPeerNotAttached { peer, endpoint },
            ));
        }
        let Some(channel) = self
            .runtime_channels
            .iter_mut()
            .find(|channel| channel.path == endpoint_info.path)
        else {
            return Ok(Vec::new());
        };
        let bytes = if channel.messages.is_empty() {
            Vec::new()
        } else {
            channel.messages.remove(0)
        };
        let remaining = channel.messages.len();
        let peer_entry = self.bus_peers.get_mut(peer)?;
        peer_entry.receive_count = peer_entry.receive_count.saturating_add(1);
        peer_entry.last_endpoint = Some(endpoint);
        let endpoint_entry = self.bus_endpoints.get_mut(endpoint)?;
        endpoint_entry.receive_count = endpoint_entry.receive_count.saturating_add(1);
        endpoint_entry.queue_depth = remaining;
        endpoint_entry.last_peer = Some(peer);
        event_queue_runtime::emit_bus_events(self, peer, endpoint, BusEventKind::Received)?;
        Ok(bytes)
    }

    pub fn bus_peer_info(&self, peer: BusPeerId) -> Result<BusPeerInfo, RuntimeError> {
        let peer = self.bus_peers.get(peer)?;
        Ok(BusPeerInfo {
            id: peer.id,
            owner: peer.owner,
            domain: peer.domain,
            name: peer.name.clone(),
            attached_endpoints: peer.attached_endpoints.clone(),
            publish_count: peer.publish_count,
            receive_count: peer.receive_count,
            last_endpoint: peer.last_endpoint,
        })
    }

    pub fn bus_endpoint_info(
        &self,
        endpoint: BusEndpointId,
    ) -> Result<BusEndpointInfo, RuntimeError> {
        let endpoint = self.bus_endpoints.get(endpoint)?;
        Ok(BusEndpointInfo {
            id: endpoint.id,
            domain: endpoint.domain,
            resource: endpoint.resource,
            kind: endpoint.kind,
            path: endpoint.path.clone(),
            attached_peers: endpoint.attached_peers.clone(),
            publish_count: endpoint.publish_count,
            receive_count: endpoint.receive_count,
            byte_count: endpoint.byte_count,
            queue_depth: endpoint.queue_depth,
            queue_capacity: endpoint.queue_capacity,
            peak_queue_depth: endpoint.peak_queue_depth,
            overflow_count: endpoint.overflow_count,
            last_peer: endpoint.last_peer,
        })
    }

    pub fn bus_peers(&self) -> Vec<BusPeerInfo> {
        self.bus_peers
            .objects
            .iter()
            .map(|(_, peer)| BusPeerInfo {
                id: peer.id,
                owner: peer.owner,
                domain: peer.domain,
                name: peer.name.clone(),
                attached_endpoints: peer.attached_endpoints.clone(),
                publish_count: peer.publish_count,
                receive_count: peer.receive_count,
                last_endpoint: peer.last_endpoint,
            })
            .collect()
    }

    pub fn bus_endpoints(&self) -> Vec<BusEndpointInfo> {
        self.bus_endpoints
            .objects
            .iter()
            .map(|(_, endpoint)| BusEndpointInfo {
                id: endpoint.id,
                domain: endpoint.domain,
                resource: endpoint.resource,
                kind: endpoint.kind,
                path: endpoint.path.clone(),
                attached_peers: endpoint.attached_peers.clone(),
                publish_count: endpoint.publish_count,
                receive_count: endpoint.receive_count,
                byte_count: endpoint.byte_count,
                queue_depth: endpoint.queue_depth,
                queue_capacity: endpoint.queue_capacity,
                peak_queue_depth: endpoint.peak_queue_depth,
                overflow_count: endpoint.overflow_count,
                last_peer: endpoint.last_peer,
            })
            .collect()
    }

    fn record_resource_agent_decision(
        &mut self,
        agent: ResourceAgentKind,
        resource: ResourceId,
        contract: Option<ContractId>,
        detail0: u64,
        detail1: u64,
    ) {
        if !self.decision_tracing_enabled {
            return;
        }
        if self.resource_agent_decisions.len() == RESOURCE_AGENT_DECISION_LIMIT {
            self.resource_agent_decisions.remove(0);
        }
        self.resource_agent_decisions
            .push(ResourceAgentDecisionRecord {
                tick: self.current_tick,
                agent,
                resource: resource.raw(),
                contract: contract.map(ContractId::raw).unwrap_or(0),
                detail0,
                detail1,
            });
    }

    fn emit_resource_contract_event(
        &mut self,
        resource: ResourceId,
        contract: ContractId,
        kind: ResourceEventKind,
    ) -> Result<(), RuntimeError> {
        event_queue_runtime::emit_resource_events(self, resource, contract, kind)?;
        let issuer = self.contract_info(contract)?.issuer;
        let state = self.processes.get(issuer)?.state();
        if !matches!(state, ProcessState::Running | ProcessState::Exited) {
            let policy = self.scheduler_policy_for_process(issuer)?;
            let _ = self.scheduler.wake_with_budget(
                &mut self.processes,
                issuer,
                policy.class,
                policy.budget,
            );
        }
        Ok(())
    }

    fn scheduler_policy_for_resource(
        &self,
        resource: ResourceId,
    ) -> Result<SchedulerPolicyInfo, RuntimeError> {
        let info = self.resource_info(resource)?;
        let mut class = match info.kind {
            ResourceKind::Device | ResourceKind::Surface => SchedulerClass::LatencyCritical,
            ResourceKind::Channel => SchedulerClass::Interactive,
            ResourceKind::Storage | ResourceKind::Namespace => SchedulerClass::BestEffort,
            ResourceKind::Memory => SchedulerClass::Background,
        };
        let mut budget = match class {
            SchedulerClass::LatencyCritical => 4,
            SchedulerClass::Interactive => 3,
            SchedulerClass::BestEffort => 2,
            SchedulerClass::Background => 1,
        };
        if info.governance == ResourceGovernanceMode::ExclusiveLease {
            class = SchedulerClass::LatencyCritical;
            budget = budget.max(4);
        }
        if info.arbitration == ResourceArbitrationPolicy::Lifo
            && matches!(
                class,
                SchedulerClass::BestEffort | SchedulerClass::Background
            )
        {
            class = SchedulerClass::Interactive;
            budget = budget.max(3);
        }
        Ok(SchedulerPolicyInfo { class, budget })
    }

    pub fn scheduler_policy_for_process(
        &self,
        pid: ProcessId,
    ) -> Result<SchedulerPolicyInfo, RuntimeError> {
        if let Some(override_policy) = self.processes.get(pid)?.scheduler_override() {
            return Ok(override_policy);
        }
        let bindings = self.processes.contract_bindings(pid)?;
        let Some(execution) = bindings.execution else {
            return Ok(SchedulerPolicyInfo {
                class: SchedulerClass::Interactive,
                budget: 2,
            });
        };
        let contract = self.contract_info(execution)?;
        self.scheduler_policy_for_resource(contract.resource)
    }

    fn refresh_process_scheduler_policy(&mut self, pid: ProcessId) -> Result<(), RuntimeError> {
        let policy = self.scheduler_policy_for_process(pid)?;
        self.scheduler
            .rebind_process(&self.processes, pid, policy.class, policy.budget)?;
        Ok(())
    }

    fn require_process_contract(
        &self,
        pid: ProcessId,
        kind: ContractKind,
    ) -> Result<Option<(ContractInfo, ResourceInfo)>, RuntimeError> {
        let bindings = self.processes.contract_bindings(pid)?;
        let bound = match kind {
            ContractKind::Execution => bindings.execution,
            ContractKind::Memory => bindings.memory,
            ContractKind::Io => bindings.io,
            ContractKind::Observe => bindings.observe,
            ContractKind::Device | ContractKind::Display => None,
        };
        let Some(contract) = bound else {
            return Ok(None);
        };
        let contract_info = self.contract_info(contract)?;
        if contract_info.state != ContractState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: contract_info.state,
                },
            ));
        }
        if contract_info.issuer != pid {
            return Err(RuntimeError::NativeModel(NativeModelError::InvalidOwner));
        }
        let resource_info = self.resource_info(contract_info.resource)?;
        if resource_info.state != ResourceState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceNotActive {
                    state: resource_info.state,
                },
            ));
        }
        if !resource_info.contract_policy.allows(kind) {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceContractKindMismatch {
                    expected: resource_info.contract_policy,
                    actual: kind,
                },
            ));
        }
        Ok(Some((contract_info, resource_info)))
    }

    pub fn enforce_process_memory_contract(&self, pid: ProcessId) -> Result<(), RuntimeError> {
        let _ = self.require_process_contract(pid, ContractKind::Memory)?;
        Ok(())
    }

    pub fn enforce_process_io_contract(&self, pid: ProcessId) -> Result<(), RuntimeError> {
        let _ = self.require_process_contract(pid, ContractKind::Io)?;
        Ok(())
    }

    pub fn enforce_process_observe_contract(&self, pid: ProcessId) -> Result<(), RuntimeError> {
        if self
            .require_process_contract(pid, ContractKind::Observe)?
            .is_none()
        {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ProcessContractMissing {
                    kind: ContractKind::Observe,
                },
            ));
        }
        Ok(())
    }

    pub fn bind_process_contract(
        &mut self,
        pid: ProcessId,
        contract: ContractId,
    ) -> Result<(), RuntimeError> {
        let contract_info = self.contract_info(contract)?;
        if contract_info.issuer != pid {
            return Err(RuntimeError::NativeModel(NativeModelError::InvalidOwner));
        }
        self.processes
            .bind_contract(pid, contract_info.kind, contract)?;
        if contract_info.kind == ContractKind::Execution {
            self.refresh_process_scheduler_policy(pid)?;
        }
        Ok(())
    }

    pub fn create_domain(
        &mut self,
        owner: ProcessId,
        parent: Option<DomainId>,
        name: impl Into<String>,
    ) -> Result<DomainId, RuntimeError> {
        self.domains
            .create(&self.processes, owner, parent, name)
            .map_err(Into::into)
    }

    pub fn create_resource(
        &mut self,
        creator: ProcessId,
        domain: DomainId,
        kind: ResourceKind,
        name: impl Into<String>,
    ) -> Result<ResourceId, RuntimeError> {
        self.resources
            .create(&self.processes, &self.domains, creator, domain, kind, name)
            .map_err(Into::into)
    }

    pub fn create_contract(
        &mut self,
        issuer: ProcessId,
        domain: DomainId,
        resource: ResourceId,
        kind: ContractKind,
        label: impl Into<String>,
    ) -> Result<ContractId, RuntimeError> {
        self.contracts
            .create(
                &self.processes,
                &self.domains,
                &self.resources,
                issuer,
                domain,
                resource,
                kind,
                label,
            )
            .map_err(Into::into)
    }

    pub fn set_resource_arbitration_policy(
        &mut self,
        resource: ResourceId,
        policy: ResourceArbitrationPolicy,
    ) -> Result<ResourceArbitrationPolicy, RuntimeError> {
        self.resources
            .set_arbitration_policy(resource, policy)
            .map_err(Into::into)
    }

    pub fn set_resource_governance_mode(
        &mut self,
        resource: ResourceId,
        mode: ResourceGovernanceMode,
    ) -> Result<ResourceGovernanceMode, RuntimeError> {
        self.resources
            .set_governance_mode(resource, mode)
            .map_err(Into::into)
    }

    pub fn transition_resource_state(
        &mut self,
        resource: ResourceId,
        next: ResourceState,
    ) -> Result<ResourceState, RuntimeError> {
        let plan = ResourceStateTransitionAgent::plan(self, resource, next)?;
        let detail1 = match &plan {
            ResourceStateTransitionPlan::Activate { .. } => 0,
            ResourceStateTransitionPlan::Suspend { waiters, .. } => waiters.len() as u64,
            ResourceStateTransitionPlan::Retire {
                revoke_contracts, ..
            } => revoke_contracts.len() as u64,
        };
        self.record_resource_agent_decision(
            ResourceAgentKind::ResourceStateTransitionAgent,
            resource,
            None,
            next as u64,
            detail1,
        );
        self.resources
            .set_state(resource, next)
            .map_err(RuntimeError::from)?;
        match plan {
            ResourceStateTransitionPlan::Activate { .. } => {}
            ResourceStateTransitionPlan::Suspend {
                resource,
                holder,
                waiters,
            } => {
                for waiter in waiters {
                    let _ = self.resources.remove_waiter(resource, waiter);
                }
                if let Some(holder) = holder {
                    self.resources
                        .release_with_contract(resource, holder)
                        .map_err(RuntimeError::from)?;
                }
            }
            ResourceStateTransitionPlan::Retire {
                revoke_contracts, ..
            } => {
                for contract in revoke_contracts {
                    let info = self.contract_info(contract)?;
                    if info.state != ContractState::Revoked {
                        let _ = self.transition_contract_state(contract, ContractState::Revoked)?;
                    }
                }
            }
        }
        Ok(next)
    }

    pub fn set_resource_contract_policy(
        &mut self,
        resource: ResourceId,
        policy: ResourceContractPolicy,
    ) -> Result<ResourceContractPolicy, RuntimeError> {
        self.resources
            .set_contract_policy(resource, policy)
            .map_err(RuntimeError::from)?;
        let contract_ids = self
            .contract_list()
            .into_iter()
            .filter(|contract| contract.resource == resource)
            .map(|contract| contract.id)
            .collect::<Vec<_>>();
        for contract in contract_ids {
            let info = self.contract_info(contract)?;
            if !policy.allows(info.kind) && info.state != ContractState::Revoked {
                let _ = self.transition_contract_state(contract, ContractState::Revoked)?;
            }
        }
        Ok(policy)
    }

    pub fn set_resource_issuer_policy(
        &mut self,
        resource: ResourceId,
        policy: ResourceIssuerPolicy,
    ) -> Result<ResourceIssuerPolicy, RuntimeError> {
        self.resources
            .set_issuer_policy(resource, policy)
            .map_err(RuntimeError::from)?;
        let resource_info = self.resource_info(resource)?;
        let contract_ids = self
            .contract_list()
            .into_iter()
            .filter(|contract| contract.resource == resource)
            .map(|contract| contract.id)
            .collect::<Vec<_>>();
        for contract in contract_ids {
            let info = self.contract_info(contract)?;
            let domain = self.domain_info(info.domain)?;
            let allowed = match policy {
                ResourceIssuerPolicy::AnyIssuer => true,
                ResourceIssuerPolicy::CreatorOnly => info.issuer == resource_info.creator,
                ResourceIssuerPolicy::DomainOwnerOnly => info.issuer == domain.owner,
            };
            if !allowed && info.state != ContractState::Revoked {
                let _ = self.transition_contract_state(contract, ContractState::Revoked)?;
            }
        }
        Ok(policy)
    }

    pub fn transition_contract_state(
        &mut self,
        contract: ContractId,
        next: ContractState,
    ) -> Result<ContractState, RuntimeError> {
        let contract_info = self.contract_info(contract)?;
        let previous_state = contract_info.state;
        let plan = ContractStateTransitionAgent::plan(self, contract, next)?;
        let detail1 = match &plan {
            ContractStateTransitionPlan::Activate { .. } => 0,
            ContractStateTransitionPlan::Deactivate {
                was_holder,
                was_waiter,
                emit_revoked,
                ..
            } => {
                u64::from(*was_holder)
                    | (u64::from(*was_waiter) << 1)
                    | (u64::from(*emit_revoked) << 2)
            }
        };
        self.record_resource_agent_decision(
            ResourceAgentKind::ContractStateTransitionAgent,
            contract_info.resource,
            Some(contract),
            next as u64,
            detail1,
        );
        let state = self
            .contracts
            .transition_state(contract, next)
            .map_err(RuntimeError::from)?;
        match plan {
            ContractStateTransitionPlan::Activate { .. } => {}
            ContractStateTransitionPlan::Deactivate {
                contract,
                resource,
                was_holder,
                was_waiter,
                emit_revoked,
            } => {
                if was_holder {
                    let _ = self.release_claimed_resource_via_contract(contract)?;
                } else if was_waiter {
                    let _ = self
                        .resources
                        .remove_waiter(resource, contract)
                        .map_err(RuntimeError::from)?;
                }
                if emit_revoked
                    && previous_state != ContractState::Revoked
                    && state == ContractState::Revoked
                {
                    self.emit_resource_contract_event(
                        resource,
                        contract,
                        ResourceEventKind::Revoked,
                    )?;
                }
            }
        }
        Ok(state)
    }

    pub fn invoke_contract(&mut self, contract: ContractId) -> Result<u64, RuntimeError> {
        let contract_info = self.contract_info(contract)?;
        let resource_info = self.resource_info(contract_info.resource)?;
        if resource_info.state != ResourceState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceNotActive {
                    state: resource_info.state,
                },
            ));
        }
        self.contracts.invoke(contract).map_err(Into::into)
    }

    pub fn acquire_resource_via_contract(
        &mut self,
        contract: ContractId,
    ) -> Result<(ResourceId, u64), RuntimeError> {
        let contract_info = self.contract_info(contract)?;
        if contract_info.state != ContractState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: contract_info.state,
                },
            ));
        }
        let resource_info = self.resource_info(contract_info.resource)?;
        if resource_info.state != ResourceState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceNotActive {
                    state: resource_info.state,
                },
            ));
        }
        if !resource_info.contract_policy.allows(contract_info.kind) {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceContractKindMismatch {
                    expected: resource_info.contract_policy,
                    actual: contract_info.kind,
                },
            ));
        }
        let acquire_count = self
            .resources
            .acquire_with_contract(contract_info.resource, contract)
            .map_err(RuntimeError::from)?;
        self.emit_resource_contract_event(
            contract_info.resource,
            contract,
            ResourceEventKind::Claimed,
        )?;
        Ok((contract_info.resource, acquire_count))
    }

    pub fn claim_resource_via_contract(
        &mut self,
        contract: ContractId,
    ) -> Result<ResourceClaimResult, RuntimeError> {
        let plan = ClaimValidator::plan(self, contract)?;
        let result = match plan {
            ResourceClaimPlan::Claim { resource, contract }
            | ResourceClaimPlan::Queue { resource, contract } => self
                .resources
                .claim_with_contract(resource, contract)
                .map_err(RuntimeError::from)?,
        };
        match result {
            ResourceClaimResult::Acquired {
                resource,
                acquire_count,
            } => {
                self.record_resource_agent_decision(
                    ResourceAgentKind::ClaimValidator,
                    resource,
                    Some(contract),
                    1,
                    acquire_count,
                );
                self.emit_resource_contract_event(resource, contract, ResourceEventKind::Claimed)?
            }
            ResourceClaimResult::Queued {
                resource, position, ..
            } => {
                self.record_resource_agent_decision(
                    ResourceAgentKind::ClaimValidator,
                    resource,
                    Some(contract),
                    2,
                    position as u64,
                );
                self.emit_resource_contract_event(resource, contract, ResourceEventKind::Queued)?
            }
        }
        Ok(result)
    }

    pub fn cancel_resource_claim_via_contract(
        &mut self,
        contract: ContractId,
    ) -> Result<(ResourceId, usize), RuntimeError> {
        let resource = CancelValidator::plan(self, contract)?;
        let waiting_count = self
            .resources
            .cancel_claim_with_contract(resource, contract)
            .map_err(RuntimeError::from)?;
        self.record_resource_agent_decision(
            ResourceAgentKind::CancelValidator,
            resource,
            Some(contract),
            waiting_count as u64,
            0,
        );
        self.emit_resource_contract_event(resource, contract, ResourceEventKind::Canceled)?;
        Ok((resource, waiting_count))
    }

    pub fn release_resource_via_contract(
        &mut self,
        contract: ContractId,
    ) -> Result<ResourceId, RuntimeError> {
        let contract_info = self.contract_info(contract)?;
        self.resources
            .release_with_contract(contract_info.resource, contract)
            .map_err(RuntimeError::from)?;
        self.emit_resource_contract_event(
            contract_info.resource,
            contract,
            ResourceEventKind::Released,
        )?;
        Ok(contract_info.resource)
    }

    pub fn release_claimed_resource_via_contract(
        &mut self,
        contract: ContractId,
    ) -> Result<ResourceReleaseResult, RuntimeError> {
        match ReleaseValidator::plan(self, contract)? {
            ResourceReleasePlan::ReleaseOnly { resource, contract } => {
                self.record_resource_agent_decision(
                    ResourceAgentKind::ReleaseValidator,
                    resource,
                    Some(contract),
                    1,
                    0,
                );
                self.resources
                    .release_with_contract(resource, contract)
                    .map_err(RuntimeError::from)?;
                cancel_graphics_resource_requests(self, resource, contract)?;
                self.emit_resource_contract_event(resource, contract, ResourceEventKind::Released)?;
                emit_graphics_resource_event(
                    self,
                    resource,
                    contract,
                    GraphicsEventKind::LeaseReleased,
                )?;
                Ok(ResourceReleaseResult::Released { resource })
            }
            ResourceReleasePlan::ReleaseAndHandoff {
                resource,
                contract,
                next,
            } => {
                self.record_resource_agent_decision(
                    ResourceAgentKind::ReleaseValidator,
                    resource,
                    Some(contract),
                    2,
                    next.raw(),
                );
                self.resources
                    .release_with_contract(resource, contract)
                    .map_err(RuntimeError::from)?;
                cancel_graphics_resource_requests(self, resource, contract)?;
                self.emit_resource_contract_event(resource, contract, ResourceEventKind::Released)?;
                emit_graphics_resource_event(
                    self,
                    resource,
                    contract,
                    GraphicsEventKind::LeaseReleased,
                )?;
                loop {
                    let popped = self
                        .resources
                        .pop_next_waiter(resource)
                        .map_err(RuntimeError::from)?;
                    let Some(candidate) = popped else {
                        break;
                    };
                    if candidate != next {
                        continue;
                    }
                    let (acquire_count, handoff_count) = self
                        .resources
                        .complete_handoff(resource, next)
                        .map_err(RuntimeError::from)?;
                    self.record_resource_agent_decision(
                        ResourceAgentKind::ReleaseValidator,
                        resource,
                        Some(next),
                        acquire_count,
                        handoff_count,
                    );
                    self.emit_resource_contract_event(
                        resource,
                        next,
                        ResourceEventKind::HandedOff,
                    )?;
                    emit_graphics_resource_event(
                        self,
                        resource,
                        next,
                        GraphicsEventKind::LeaseAcquired,
                    )?;
                    return Ok(ResourceReleaseResult::HandedOff {
                        resource,
                        contract: next,
                        acquire_count,
                        handoff_count,
                    });
                }
                Ok(ResourceReleaseResult::Released { resource })
            }
        }
    }

    pub fn transfer_resource_via_contract(
        &mut self,
        source: ContractId,
        target: ContractId,
    ) -> Result<(ResourceId, u64), RuntimeError> {
        let source_info = self.contract_info(source)?;
        let target_info = self.contract_info(target)?;
        if source_info.resource != target_info.resource {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceBindingMismatch,
            ));
        }
        if source_info.state != ContractState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: source_info.state,
                },
            ));
        }
        if target_info.state != ContractState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: target_info.state,
                },
            ));
        }
        let resource_info = self.resource_info(source_info.resource)?;
        if resource_info.state != ResourceState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceNotActive {
                    state: resource_info.state,
                },
            ));
        }
        if !resource_info.contract_policy.allows(target_info.kind) {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceContractKindMismatch {
                    expected: resource_info.contract_policy,
                    actual: target_info.kind,
                },
            ));
        }
        let acquire_count = self
            .resources
            .transfer_between_contracts(source_info.resource, source, target)
            .map_err(RuntimeError::from)?;
        Ok((source_info.resource, acquire_count))
    }

    pub fn domain_info(&self, id: DomainId) -> Result<DomainInfo, RuntimeError> {
        let domain = self.domains.get(id)?;
        Ok(DomainInfo {
            id: domain.id,
            owner: domain.owner,
            parent: domain.parent,
            name: domain.name.clone(),
            resource_count: self
                .resources
                .objects
                .iter()
                .filter(|(_, resource)| resource.domain == id)
                .count(),
            contract_count: self
                .contracts
                .objects
                .iter()
                .filter(|(_, contract)| contract.domain == id)
                .count(),
        })
    }

    pub fn resource_info(&self, id: ResourceId) -> Result<ResourceInfo, RuntimeError> {
        let resource = self.resources.get(id)?;
        Ok(ResourceInfo {
            id: resource.id,
            domain: resource.domain,
            creator: resource.creator,
            kind: resource.kind,
            state: resource.state,
            arbitration: resource.arbitration,
            governance: resource.governance,
            contract_policy: resource.contract_policy,
            issuer_policy: resource.issuer_policy,
            holder: resource.holder,
            waiters: resource.waiters.clone(),
            waiting_count: resource.waiters.len(),
            acquire_count: resource.acquire_count,
            handoff_count: resource.handoff_count,
            name: resource.name.clone(),
        })
    }

    pub fn contract_info(&self, id: ContractId) -> Result<ContractInfo, RuntimeError> {
        let contract = self.contracts.get(id)?;
        Ok(ContractInfo {
            id: contract.id,
            domain: contract.domain,
            resource: contract.resource,
            issuer: contract.issuer,
            kind: contract.kind,
            state: contract.state,
            invocation_count: contract.invocation_count,
            label: contract.label.clone(),
        })
    }

    pub fn domain_list(&self) -> Vec<DomainInfo> {
        let mut domains = self
            .domains
            .objects
            .iter()
            .map(|(handle, _)| DomainId::from_handle(handle))
            .filter_map(|id| self.domain_info(id).ok())
            .collect::<Vec<_>>();
        domains.sort_by_key(|domain| domain.id.raw());
        domains
    }

    pub fn resource_list(&self) -> Vec<ResourceInfo> {
        let mut resources = self
            .resources
            .objects
            .iter()
            .map(|(handle, _)| ResourceId::from_handle(handle))
            .filter_map(|id| self.resource_info(id).ok())
            .collect::<Vec<_>>();
        resources.sort_by_key(|resource| resource.id.raw());
        resources
    }

    pub fn contract_list(&self) -> Vec<ContractInfo> {
        let mut contracts = self
            .contracts
            .objects
            .iter()
            .map(|(handle, _)| ContractId::from_handle(handle))
            .filter_map(|id| self.contract_info(id).ok())
            .collect::<Vec<_>>();
        contracts.sort_by_key(|contract| contract.id.raw());
        contracts
    }
}

impl ClaimValidator {
    fn plan(
        runtime: &KernelRuntime,
        contract: ContractId,
    ) -> Result<ResourceClaimPlan, RuntimeError> {
        let contract_info = runtime.contract_info(contract)?;
        if contract_info.state != ContractState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: contract_info.state,
                },
            ));
        }
        let resource_info = runtime.resource_info(contract_info.resource)?;
        if resource_info.state != ResourceState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceNotActive {
                    state: resource_info.state,
                },
            ));
        }
        if !resource_info.contract_policy.allows(contract_info.kind) {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ResourceContractKindMismatch {
                    expected: resource_info.contract_policy,
                    actual: contract_info.kind,
                },
            ));
        }
        Ok(
            if resource_info.holder.is_none() || resource_info.holder == Some(contract) {
                ResourceClaimPlan::Claim {
                    resource: contract_info.resource,
                    contract,
                }
            } else {
                ResourceClaimPlan::Queue {
                    resource: contract_info.resource,
                    contract,
                }
            },
        )
    }
}

impl CancelValidator {
    fn plan(runtime: &KernelRuntime, contract: ContractId) -> Result<ResourceId, RuntimeError> {
        let contract_info = runtime.contract_info(contract)?;
        if contract_info.state != ContractState::Active {
            return Err(RuntimeError::NativeModel(
                NativeModelError::ContractNotActive {
                    state: contract_info.state,
                },
            ));
        }
        Ok(contract_info.resource)
    }
}

impl ReleaseValidator {
    fn plan(
        runtime: &KernelRuntime,
        contract: ContractId,
    ) -> Result<ResourceReleasePlan, RuntimeError> {
        let contract_info = runtime.contract_info(contract)?;
        let resource_info = runtime.resource_info(contract_info.resource)?;
        let next = resource_info.waiters.into_iter().find(|waiter| {
            runtime
                .contract_info(*waiter)
                .map(|info| info.state == ContractState::Active)
                .unwrap_or(false)
        });
        Ok(match next {
            Some(next) => ResourceReleasePlan::ReleaseAndHandoff {
                resource: contract_info.resource,
                contract,
                next,
            },
            None => ResourceReleasePlan::ReleaseOnly {
                resource: contract_info.resource,
                contract,
            },
        })
    }
}

impl ResourceStateTransitionAgent {
    fn plan(
        runtime: &KernelRuntime,
        resource: ResourceId,
        next: ResourceState,
    ) -> Result<ResourceStateTransitionPlan, RuntimeError> {
        let resource_info = runtime.resource_info(resource)?;
        Ok(match next {
            ResourceState::Active => ResourceStateTransitionPlan::Activate { resource },
            ResourceState::Suspended => ResourceStateTransitionPlan::Suspend {
                resource,
                holder: resource_info.holder,
                waiters: resource_info.waiters,
            },
            ResourceState::Retired => {
                let mut revoke_contracts = runtime
                    .contract_list()
                    .into_iter()
                    .filter(|contract| contract.resource == resource)
                    .map(|contract| contract.id)
                    .collect::<Vec<_>>();
                revoke_contracts.sort_by_key(|contract| contract.raw());
                revoke_contracts.dedup();
                ResourceStateTransitionPlan::Retire {
                    resource,
                    revoke_contracts,
                }
            }
        })
    }
}

impl ContractStateTransitionAgent {
    fn plan(
        runtime: &KernelRuntime,
        contract: ContractId,
        next: ContractState,
    ) -> Result<ContractStateTransitionPlan, RuntimeError> {
        let contract_info = runtime.contract_info(contract)?;
        if next == ContractState::Active {
            return Ok(ContractStateTransitionPlan::Activate { contract });
        }
        let resource_info = runtime.resource_info(contract_info.resource)?;
        Ok(ContractStateTransitionPlan::Deactivate {
            contract,
            resource: contract_info.resource,
            was_holder: resource_info.holder == Some(contract),
            was_waiter: resource_info.waiters.contains(&contract),
            emit_revoked: next == ContractState::Revoked,
        })
    }
}
