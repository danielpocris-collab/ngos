//! Canonical subsystem role:
//! - subsystem: native kernel object and contract model
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical native resource, contract, and governance
//!   object model
//!
//! Canonical contract families defined here:
//! - resource contracts
//! - domain and contract state contracts
//! - governance and arbitration contracts
//!
//! This module may define canonical native object truth for `ngos`. Higher
//! layers may inspect or serialize it, but they must not redefine it.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Memory,
    Storage,
    Channel,
    Device,
    Namespace,
    Surface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractKind {
    Execution,
    Memory,
    Io,
    Device,
    Display,
    Observe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractState {
    Active,
    Suspended,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Domain {
    pub(crate) id: DomainId,
    pub(crate) owner: ProcessId,
    pub(crate) parent: Option<DomainId>,
    pub(crate) name: String,
}

impl Domain {
    fn new_unbound(owner: ProcessId, parent: Option<DomainId>, name: impl Into<String>) -> Self {
        Self {
            id: DomainId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            owner,
            parent,
            name: name.into(),
        }
    }

    fn attach_id(&mut self, id: DomainId) {
        self.id = id;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resource {
    pub(crate) id: ResourceId,
    pub(crate) domain: DomainId,
    pub(crate) creator: ProcessId,
    pub(crate) kind: ResourceKind,
    pub(crate) state: ResourceState,
    pub(crate) arbitration: ResourceArbitrationPolicy,
    pub(crate) governance: ResourceGovernanceMode,
    pub(crate) contract_policy: ResourceContractPolicy,
    pub(crate) issuer_policy: ResourceIssuerPolicy,
    pub(crate) holder: Option<ContractId>,
    pub(crate) waiters: Vec<ContractId>,
    pub(crate) acquire_count: u64,
    pub(crate) handoff_count: u64,
    pub(crate) name: String,
}

impl Resource {
    fn new_unbound(
        domain: DomainId,
        creator: ProcessId,
        kind: ResourceKind,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: ResourceId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            domain,
            creator,
            kind,
            state: ResourceState::Active,
            arbitration: ResourceArbitrationPolicy::Fifo,
            governance: ResourceGovernanceMode::Queueing,
            contract_policy: ResourceContractPolicy::Any,
            issuer_policy: ResourceIssuerPolicy::AnyIssuer,
            holder: None,
            waiters: Vec::new(),
            acquire_count: 0,
            handoff_count: 0,
            name: name.into(),
        }
    }

    fn attach_id(&mut self, id: ResourceId) {
        self.id = id;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Contract {
    pub(crate) id: ContractId,
    pub(crate) domain: DomainId,
    pub(crate) resource: ResourceId,
    pub(crate) issuer: ProcessId,
    pub(crate) kind: ContractKind,
    pub(crate) state: ContractState,
    pub(crate) invocation_count: u64,
    pub(crate) label: String,
}

impl Contract {
    fn new_unbound(
        domain: DomainId,
        resource: ResourceId,
        issuer: ProcessId,
        kind: ContractKind,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: ContractId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            domain,
            resource,
            issuer,
            kind,
            state: ContractState::Active,
            invocation_count: 0,
            label: label.into(),
        }
    }

    fn attach_id(&mut self, id: ContractId) {
        self.id = id;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainInfo {
    pub id: DomainId,
    pub owner: ProcessId,
    pub parent: Option<DomainId>,
    pub name: String,
    pub resource_count: usize,
    pub contract_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceInfo {
    pub id: ResourceId,
    pub domain: DomainId,
    pub creator: ProcessId,
    pub kind: ResourceKind,
    pub state: ResourceState,
    pub arbitration: ResourceArbitrationPolicy,
    pub governance: ResourceGovernanceMode,
    pub contract_policy: ResourceContractPolicy,
    pub issuer_policy: ResourceIssuerPolicy,
    pub holder: Option<ContractId>,
    pub waiters: Vec<ContractId>,
    pub waiting_count: usize,
    pub acquire_count: u64,
    pub handoff_count: u64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractInfo {
    pub id: ContractId,
    pub domain: DomainId,
    pub resource: ResourceId,
    pub issuer: ProcessId,
    pub kind: ContractKind,
    pub state: ContractState,
    pub invocation_count: u64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceClaimResult {
    Acquired {
        resource: ResourceId,
        acquire_count: u64,
    },
    Queued {
        resource: ResourceId,
        holder: ContractId,
        position: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceReleaseResult {
    Released {
        resource: ResourceId,
    },
    HandedOff {
        resource: ResourceId,
        contract: ContractId,
        acquire_count: u64,
        handoff_count: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DomainTable {
    pub(crate) objects: KernelObjectTable<Domain>,
}

impl DomainTable {
    pub(crate) fn new(start: u64, end_exclusive: u64) -> Self {
        Self {
            objects: KernelObjectTable::new(start, end_exclusive),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        &mut self,
        processes: &ProcessTable,
        owner: ProcessId,
        parent: Option<DomainId>,
        name: impl Into<String>,
    ) -> Result<DomainId, NativeModelError> {
        processes
            .get(owner)
            .map_err(|_| NativeModelError::InvalidOwner)?;
        if let Some(parent) = parent {
            self.get(parent)?;
        }

        let handle = self
            .objects
            .insert(Domain::new_unbound(owner, parent, name))
            .map_err(NativeModelError::from_domain_object_error)?;
        let id = DomainId::from_handle(handle);
        self.objects
            .get_mut(handle)
            .map_err(NativeModelError::from_domain_object_error)?
            .attach_id(id);
        Ok(id)
    }

    pub(crate) fn get(&self, id: DomainId) -> Result<&Domain, NativeModelError> {
        self.objects
            .get(id.handle())
            .map_err(NativeModelError::from_domain_object_error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResourceTable {
    pub(crate) objects: KernelObjectTable<Resource>,
}

impl ResourceTable {
    pub(crate) fn new(start: u64, end_exclusive: u64) -> Self {
        Self {
            objects: KernelObjectTable::new(start, end_exclusive),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        &mut self,
        processes: &ProcessTable,
        domains: &DomainTable,
        creator: ProcessId,
        domain: DomainId,
        kind: ResourceKind,
        name: impl Into<String>,
    ) -> Result<ResourceId, NativeModelError> {
        processes
            .get(creator)
            .map_err(|_| NativeModelError::InvalidOwner)?;
        domains.get(domain)?;

        let handle = self
            .objects
            .insert(Resource::new_unbound(domain, creator, kind, name))
            .map_err(NativeModelError::from_resource_object_error)?;
        let id = ResourceId::from_handle(handle);
        self.objects
            .get_mut(handle)
            .map_err(NativeModelError::from_resource_object_error)?
            .attach_id(id);
        Ok(id)
    }

    pub(crate) fn get(&self, id: ResourceId) -> Result<&Resource, NativeModelError> {
        self.objects
            .get(id.handle())
            .map_err(NativeModelError::from_resource_object_error)
    }

    pub(crate) fn acquire_with_contract(
        &mut self,
        id: ResourceId,
        contract: ContractId,
    ) -> Result<u64, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        match resource.holder {
            Some(holder) if holder != contract => {
                return Err(NativeModelError::ResourceBusy { holder });
            }
            Some(_) => {}
            None => {
                resource.holder = Some(contract);
            }
        }
        resource.acquire_count = resource.acquire_count.saturating_add(1);
        Ok(resource.acquire_count)
    }

    pub(crate) fn set_arbitration_policy(
        &mut self,
        id: ResourceId,
        policy: ResourceArbitrationPolicy,
    ) -> Result<ResourceArbitrationPolicy, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        resource.arbitration = policy;
        Ok(resource.arbitration)
    }

    pub(crate) fn set_governance_mode(
        &mut self,
        id: ResourceId,
        mode: ResourceGovernanceMode,
    ) -> Result<ResourceGovernanceMode, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        resource.governance = mode;
        if mode == ResourceGovernanceMode::ExclusiveLease {
            resource.waiters.clear();
        }
        Ok(resource.governance)
    }

    pub(crate) fn set_state(
        &mut self,
        id: ResourceId,
        state: ResourceState,
    ) -> Result<ResourceState, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        resource.state = state;
        Ok(resource.state)
    }

    pub(crate) fn set_contract_policy(
        &mut self,
        id: ResourceId,
        policy: ResourceContractPolicy,
    ) -> Result<ResourceContractPolicy, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        resource.contract_policy = policy;
        Ok(resource.contract_policy)
    }

    pub(crate) fn set_issuer_policy(
        &mut self,
        id: ResourceId,
        policy: ResourceIssuerPolicy,
    ) -> Result<ResourceIssuerPolicy, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        resource.issuer_policy = policy;
        Ok(resource.issuer_policy)
    }

    pub(crate) fn claim_with_contract(
        &mut self,
        id: ResourceId,
        contract: ContractId,
    ) -> Result<ResourceClaimResult, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        if resource.state != ResourceState::Active {
            return Err(NativeModelError::ResourceNotActive {
                state: resource.state,
            });
        }
        if resource.holder.is_none() {
            resource.holder = Some(contract);
            resource.acquire_count = resource.acquire_count.saturating_add(1);
            return Ok(ResourceClaimResult::Acquired {
                resource: id,
                acquire_count: resource.acquire_count,
            });
        }
        if resource.holder == Some(contract) {
            return Ok(ResourceClaimResult::Acquired {
                resource: id,
                acquire_count: resource.acquire_count,
            });
        }
        if resource.governance == ResourceGovernanceMode::ExclusiveLease {
            return Err(NativeModelError::ResourceBusy {
                holder: resource.holder.expect("checked above"),
            });
        }
        if let Some(position) = resource.waiters.iter().position(|entry| *entry == contract) {
            return Ok(ResourceClaimResult::Queued {
                resource: id,
                holder: resource.holder.expect("checked above"),
                position: position + 1,
            });
        }
        match resource.arbitration {
            ResourceArbitrationPolicy::Fifo => resource.waiters.push(contract),
            ResourceArbitrationPolicy::Lifo => resource.waiters.insert(0, contract),
        }
        let position = resource
            .waiters
            .iter()
            .position(|entry| *entry == contract)
            .expect("waiter inserted")
            + 1;
        Ok(ResourceClaimResult::Queued {
            resource: id,
            holder: resource.holder.expect("checked above"),
            position,
        })
    }

    pub(crate) fn release_with_contract(
        &mut self,
        id: ResourceId,
        contract: ContractId,
    ) -> Result<(), NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        match resource.holder {
            Some(holder) if holder == contract => {
                resource.holder = None;
                Ok(())
            }
            _ => Err(NativeModelError::ResourceNotHeld { resource: id }),
        }
    }

    pub(crate) fn transfer_between_contracts(
        &mut self,
        id: ResourceId,
        source: ContractId,
        target: ContractId,
    ) -> Result<u64, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        match resource.holder {
            Some(holder) if holder == source => {
                resource.waiters.retain(|waiter| *waiter != target);
                resource.holder = Some(target);
                resource.acquire_count = resource.acquire_count.saturating_add(1);
                resource.handoff_count = resource.handoff_count.saturating_add(1);
                Ok(resource.acquire_count)
            }
            _ => Err(NativeModelError::ResourceNotHeld { resource: id }),
        }
    }

    pub(crate) fn pop_next_waiter(
        &mut self,
        id: ResourceId,
    ) -> Result<Option<ContractId>, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        if resource.waiters.is_empty() {
            return Ok(None);
        }
        Ok(Some(resource.waiters.remove(0)))
    }

    pub(crate) fn remove_waiter(
        &mut self,
        id: ResourceId,
        contract: ContractId,
    ) -> Result<bool, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        let before = resource.waiters.len();
        resource.waiters.retain(|waiter| *waiter != contract);
        Ok(resource.waiters.len() != before)
    }

    pub(crate) fn cancel_claim_with_contract(
        &mut self,
        id: ResourceId,
        contract: ContractId,
    ) -> Result<usize, NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        if resource.holder == Some(contract) {
            return Err(NativeModelError::ResourceBusy { holder: contract });
        }
        let before = resource.waiters.len();
        resource.waiters.retain(|waiter| *waiter != contract);
        if resource.waiters.len() == before {
            return Err(NativeModelError::ResourceClaimNotQueued { resource: id });
        }
        Ok(resource.waiters.len())
    }

    pub(crate) fn complete_handoff(
        &mut self,
        id: ResourceId,
        contract: ContractId,
    ) -> Result<(u64, u64), NativeModelError> {
        let resource = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_resource_object_error)?;
        resource.holder = Some(contract);
        resource.acquire_count = resource.acquire_count.saturating_add(1);
        resource.handoff_count = resource.handoff_count.saturating_add(1);
        Ok((resource.acquire_count, resource.handoff_count))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContractTable {
    pub(crate) objects: KernelObjectTable<Contract>,
}

impl ContractTable {
    pub(crate) fn new(start: u64, end_exclusive: u64) -> Self {
        Self {
            objects: KernelObjectTable::new(start, end_exclusive),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        &mut self,
        processes: &ProcessTable,
        domains: &DomainTable,
        resources: &ResourceTable,
        issuer: ProcessId,
        domain: DomainId,
        resource: ResourceId,
        kind: ContractKind,
        label: impl Into<String>,
    ) -> Result<ContractId, NativeModelError> {
        processes
            .get(issuer)
            .map_err(|_| NativeModelError::InvalidOwner)?;
        domains.get(domain)?;
        let resource_entry = resources.get(resource)?;
        if resource_entry.state != ResourceState::Active {
            return Err(NativeModelError::ResourceNotActive {
                state: resource_entry.state,
            });
        }
        if resource_entry.domain != domain {
            return Err(NativeModelError::ParentMismatch);
        }
        if !resource_entry.contract_policy.allows(kind) {
            return Err(NativeModelError::ResourceContractKindMismatch {
                expected: resource_entry.contract_policy,
                actual: kind,
            });
        }
        let domain_entry = domains.get(domain)?;
        let issuer_allowed = match resource_entry.issuer_policy {
            ResourceIssuerPolicy::AnyIssuer => true,
            ResourceIssuerPolicy::CreatorOnly => issuer == resource_entry.creator,
            ResourceIssuerPolicy::DomainOwnerOnly => issuer == domain_entry.owner,
        };
        if !issuer_allowed {
            return Err(NativeModelError::ResourceIssuerPolicyMismatch {
                policy: resource_entry.issuer_policy,
                issuer,
            });
        }

        let handle = self
            .objects
            .insert(Contract::new_unbound(domain, resource, issuer, kind, label))
            .map_err(NativeModelError::from_contract_object_error)?;
        let id = ContractId::from_handle(handle);
        self.objects
            .get_mut(handle)
            .map_err(NativeModelError::from_contract_object_error)?
            .attach_id(id);
        Ok(id)
    }

    pub(crate) fn get(&self, id: ContractId) -> Result<&Contract, NativeModelError> {
        self.objects
            .get(id.handle())
            .map_err(NativeModelError::from_contract_object_error)
    }

    pub(crate) fn transition_state(
        &mut self,
        id: ContractId,
        next: ContractState,
    ) -> Result<ContractState, NativeModelError> {
        let contract = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_contract_object_error)?;
        let current = contract.state;
        if current == next {
            return Ok(current);
        }

        let allowed = matches!(
            (current, next),
            (ContractState::Active, ContractState::Suspended)
                | (ContractState::Suspended, ContractState::Active)
                | (ContractState::Active, ContractState::Revoked)
                | (ContractState::Suspended, ContractState::Revoked)
        );
        if !allowed {
            return Err(NativeModelError::InvalidStateTransition {
                from: current,
                to: next,
            });
        }

        contract.state = next;
        Ok(next)
    }

    pub(crate) fn invoke(&mut self, id: ContractId) -> Result<u64, NativeModelError> {
        let contract = self
            .objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_contract_object_error)?;
        if contract.state != ContractState::Active {
            return Err(NativeModelError::ContractNotActive {
                state: contract.state,
            });
        }
        contract.invocation_count = contract.invocation_count.saturating_add(1);
        Ok(contract.invocation_count)
    }
}
