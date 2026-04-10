//! Canonical subsystem role:
//! - subsystem: bus model
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: canonical peer/endpoint routing model for bus traffic on
//!   top of kernel-owned channel primitives
//!
//! Canonical contract families defined here:
//! - bus peer contracts
//! - bus endpoint contracts
//! - bus attachment contracts
//!
//! This module may define canonical bus truth. Higher layers may inspect or
//! operate it, but they must not redefine peer or endpoint ownership.

use super::*;

pub const BUS_ENDPOINT_QUEUE_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusEndpointKind {
    Channel,
}

impl BusEndpointKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Channel => "channel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BusPeer {
    pub(crate) id: BusPeerId,
    pub(crate) owner: ProcessId,
    pub(crate) domain: DomainId,
    pub(crate) name: String,
    pub(crate) attached_endpoints: Vec<BusEndpointId>,
    pub(crate) publish_count: u64,
    pub(crate) receive_count: u64,
    pub(crate) last_endpoint: Option<BusEndpointId>,
}

impl BusPeer {
    fn new_unbound(owner: ProcessId, domain: DomainId, name: impl Into<String>) -> Self {
        Self {
            id: BusPeerId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            owner,
            domain,
            name: name.into(),
            attached_endpoints: Vec::new(),
            publish_count: 0,
            receive_count: 0,
            last_endpoint: None,
        }
    }

    fn attach_id(&mut self, id: BusPeerId) {
        self.id = id;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BusEndpoint {
    pub(crate) id: BusEndpointId,
    pub(crate) domain: DomainId,
    pub(crate) resource: ResourceId,
    pub(crate) kind: BusEndpointKind,
    pub(crate) path: String,
    pub(crate) attached_peers: Vec<BusPeerId>,
    pub(crate) publish_count: u64,
    pub(crate) receive_count: u64,
    pub(crate) byte_count: u64,
    pub(crate) queue_depth: usize,
    pub(crate) queue_capacity: usize,
    pub(crate) peak_queue_depth: usize,
    pub(crate) overflow_count: u64,
    pub(crate) last_peer: Option<BusPeerId>,
}

impl BusEndpoint {
    fn new_unbound(
        domain: DomainId,
        resource: ResourceId,
        kind: BusEndpointKind,
        path: impl Into<String>,
    ) -> Self {
        Self {
            id: BusEndpointId::from_handle(ObjectHandle::new(Handle::new(0), 0)),
            domain,
            resource,
            kind,
            path: path.into(),
            attached_peers: Vec::new(),
            publish_count: 0,
            receive_count: 0,
            byte_count: 0,
            queue_depth: 0,
            queue_capacity: BUS_ENDPOINT_QUEUE_CAPACITY,
            peak_queue_depth: 0,
            overflow_count: 0,
            last_peer: None,
        }
    }

    fn attach_id(&mut self, id: BusEndpointId) {
        self.id = id;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusPeerInfo {
    pub id: BusPeerId,
    pub owner: ProcessId,
    pub domain: DomainId,
    pub name: String,
    pub attached_endpoints: Vec<BusEndpointId>,
    pub publish_count: u64,
    pub receive_count: u64,
    pub last_endpoint: Option<BusEndpointId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusEndpointInfo {
    pub id: BusEndpointId,
    pub domain: DomainId,
    pub resource: ResourceId,
    pub kind: BusEndpointKind,
    pub path: String,
    pub attached_peers: Vec<BusPeerId>,
    pub publish_count: u64,
    pub receive_count: u64,
    pub byte_count: u64,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub peak_queue_depth: usize,
    pub overflow_count: u64,
    pub last_peer: Option<BusPeerId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BusPeerTable {
    pub(crate) objects: KernelObjectTable<BusPeer>,
}

impl BusPeerTable {
    pub(crate) fn new(start: u64, end_exclusive: u64) -> Self {
        Self {
            objects: KernelObjectTable::new(start, end_exclusive),
        }
    }

    pub(crate) fn create(
        &mut self,
        processes: &ProcessTable,
        domains: &DomainTable,
        owner: ProcessId,
        domain: DomainId,
        name: impl Into<String>,
    ) -> Result<BusPeerId, NativeModelError> {
        processes
            .get(owner)
            .map_err(|_| NativeModelError::InvalidOwner)?;
        domains.get(domain)?;
        let handle = self
            .objects
            .insert(BusPeer::new_unbound(owner, domain, name))
            .map_err(NativeModelError::from_bus_peer_object_error)?;
        let id = BusPeerId::from_handle(handle);
        self.objects
            .get_mut(handle)
            .map_err(NativeModelError::from_bus_peer_object_error)?
            .attach_id(id);
        Ok(id)
    }

    pub(crate) fn get(&self, id: BusPeerId) -> Result<&BusPeer, NativeModelError> {
        self.objects
            .get(id.handle())
            .map_err(NativeModelError::from_bus_peer_object_error)
    }

    pub(crate) fn get_mut(&mut self, id: BusPeerId) -> Result<&mut BusPeer, NativeModelError> {
        self.objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_bus_peer_object_error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BusEndpointTable {
    pub(crate) objects: KernelObjectTable<BusEndpoint>,
}

impl BusEndpointTable {
    pub(crate) fn new(start: u64, end_exclusive: u64) -> Self {
        Self {
            objects: KernelObjectTable::new(start, end_exclusive),
        }
    }

    pub(crate) fn create(
        &mut self,
        domains: &DomainTable,
        resources: &ResourceTable,
        domain: DomainId,
        resource: ResourceId,
        kind: BusEndpointKind,
        path: impl Into<String>,
    ) -> Result<BusEndpointId, NativeModelError> {
        domains.get(domain)?;
        let resource_entry = resources.get(resource)?;
        if resource_entry.domain != domain {
            return Err(NativeModelError::ParentMismatch);
        }
        let handle = self
            .objects
            .insert(BusEndpoint::new_unbound(domain, resource, kind, path))
            .map_err(NativeModelError::from_bus_endpoint_object_error)?;
        let id = BusEndpointId::from_handle(handle);
        self.objects
            .get_mut(handle)
            .map_err(NativeModelError::from_bus_endpoint_object_error)?
            .attach_id(id);
        Ok(id)
    }

    pub(crate) fn get(&self, id: BusEndpointId) -> Result<&BusEndpoint, NativeModelError> {
        self.objects
            .get(id.handle())
            .map_err(NativeModelError::from_bus_endpoint_object_error)
    }

    pub(crate) fn get_mut(
        &mut self,
        id: BusEndpointId,
    ) -> Result<&mut BusEndpoint, NativeModelError> {
        self.objects
            .get_mut(id.handle())
            .map_err(NativeModelError::from_bus_endpoint_object_error)
    }
}
