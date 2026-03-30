use super::*;
use crate::device_runtime::sync_endpoint_io_state;

fn record_io_agent_decision(
    runtime: &mut KernelRuntime,
    agent: IoAgentKind,
    owner: ProcessId,
    fd: Descriptor,
    kind: ObjectKind,
    detail0: u64,
    detail1: u64,
) {
    if !runtime.decision_tracing_enabled {
        return;
    }
    if runtime.io_agent_decisions.len() == 64 {
        runtime.io_agent_decisions.remove(0);
    }
    runtime.io_agent_decisions.push(IoAgentDecisionRecord {
        tick: runtime.current_tick,
        agent,
        owner: owner.raw(),
        fd: u64::from(fd.raw()),
        kind: kind as u64,
        detail0,
        detail1,
    });
}

pub(crate) fn open_runtime_queue_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    kind: ObjectKind,
    binding: QueueDescriptorTarget,
    name: String,
) -> Result<Descriptor, RuntimeError> {
    let capability = runtime.grant_capability(
        owner,
        owner.handle(),
        CapabilityRights::READ | CapabilityRights::DUPLICATE,
        name.clone(),
    )?;
    open_descriptor_bound(runtime, owner, capability, kind, name, Some(binding))
}

pub(crate) fn finalize_queue_descriptor_close(
    runtime: &mut KernelRuntime,
    descriptor: &ObjectDescriptor,
) -> Result<(), RuntimeError> {
    let Some(target) = descriptor.queue_binding() else {
        return Ok(());
    };
    if queue_descriptor_reference_count(runtime, target) != 0 {
        return Ok(());
    }
    match target {
        QueueDescriptorTarget::Event { owner, queue, .. } => {
            match runtime.remove_event_queue_record(owner, queue) {
                Ok(()) | Err(RuntimeError::EventQueue(EventQueueError::InvalidQueue)) => Ok(()),
                Err(err) => Err(err),
            }
        }
        QueueDescriptorTarget::Sleep { owner, queue } => {
            match runtime.remove_sleep_queue_record(owner, queue) {
                Ok(_) | Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound)) => Ok(()),
                Err(err) => Err(err),
            }
        }
    }
}

pub(crate) fn queue_descriptor_reference_count(
    runtime: &KernelRuntime,
    binding: QueueDescriptorTarget,
) -> usize {
    runtime
        .namespaces
        .iter()
        .map(|(_, namespace)| {
            namespace
                .descriptors
                .iter()
                .flatten()
                .filter(|descriptor| descriptor.queue_binding() == Some(binding))
                .count()
        })
        .sum()
}

pub(crate) fn open_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    capability: CapabilityId,
    kind: ObjectKind,
    name: impl Into<String>,
) -> Result<Descriptor, RuntimeError> {
    open_descriptor_bound(runtime, owner, capability, kind, name, None)
}

pub(crate) fn open_descriptor_bound(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    capability: CapabilityId,
    kind: ObjectKind,
    name: impl Into<String>,
    queue_binding: Option<QueueDescriptorTarget>,
) -> Result<Descriptor, RuntimeError> {
    runtime.processes.get(owner)?;
    runtime.ensure_namespace(owner);
    let index = runtime
        .namespaces
        .iter()
        .position(|(pid, _)| *pid == owner)
        .ok_or(RuntimeError::Descriptor(DescriptorError::InvalidOwner))?;
    let processes = &runtime.processes;
    let capabilities = &runtime.capabilities;
    let namespace = &mut runtime.namespaces[index].1;
    let fd = namespace
        .open_bound(
            processes,
            capabilities,
            owner,
            capability,
            kind,
            name,
            queue_binding,
        )
        .map_err(RuntimeError::from)?;
    let descriptor = namespace.get(fd).map_err(RuntimeError::from)?.clone();
    runtime.io_registry.register(&descriptor);
    let _ = sync_endpoint_io_state(runtime, owner, fd);
    runtime.sync_fdshare_group_from(owner)?;
    record_io_agent_decision(runtime, IoAgentKind::OpenPathAgent, owner, fd, kind, 0, 0);
    Ok(fd)
}

pub(crate) fn duplicate_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<Descriptor, RuntimeError> {
    runtime.processes.get(owner)?;
    let index = runtime
        .namespaces
        .iter()
        .position(|(pid, _)| *pid == owner)
        .ok_or(RuntimeError::Descriptor(DescriptorError::InvalidOwner))?;
    let processes = &runtime.processes;
    let capabilities = &runtime.capabilities;
    let namespace = &mut runtime.namespaces[index].1;
    let new_fd = namespace
        .dup(processes, capabilities, fd)
        .map_err(RuntimeError::from)?;
    let descriptor = namespace.get(new_fd).map_err(RuntimeError::from)?.clone();
    runtime
        .io_registry
        .duplicate(owner, fd, &descriptor)
        .map_err(map_runtime_io_error)?;
    runtime.sync_fdshare_group_from(owner)?;
    record_io_agent_decision(
        runtime,
        IoAgentKind::DuplicateDescriptorAgent,
        owner,
        new_fd,
        descriptor.kind(),
        u64::from(fd.raw()),
        0,
    );
    Ok(new_fd)
}

pub(crate) fn duplicate_descriptor_to(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    target: Descriptor,
) -> Result<Descriptor, RuntimeError> {
    runtime.processes.get(owner)?;
    let index = runtime
        .namespaces
        .iter()
        .position(|(pid, _)| *pid == owner)
        .ok_or(RuntimeError::Descriptor(DescriptorError::InvalidOwner))?;
    let processes = &runtime.processes;
    let capabilities = &runtime.capabilities;
    let (replaced, descriptor) = {
        let namespace = &mut runtime.namespaces[index].1;
        let replaced = namespace
            .dup_to(processes, capabilities, fd, target)
            .map_err(RuntimeError::from)?;
        let descriptor = namespace.get(target).map_err(RuntimeError::from)?.clone();
        (replaced, descriptor)
    };
    if replaced.is_some() {
        let _ = runtime.io_registry.close(owner, target);
    }
    if let Some(replaced) = &replaced {
        finalize_queue_descriptor_close(runtime, replaced)?;
    }
    runtime
        .io_registry
        .duplicate(owner, fd, &descriptor)
        .map_err(map_runtime_io_error)?;
    runtime.sync_fdshare_group_from(owner)?;
    record_io_agent_decision(
        runtime,
        IoAgentKind::DuplicateDescriptorAgent,
        owner,
        target,
        descriptor.kind(),
        u64::from(fd.raw()),
        u64::from(replaced.is_some()),
    );
    Ok(target)
}

pub(crate) fn close_descriptor(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<ObjectDescriptor, RuntimeError> {
    let descriptor = runtime
        .namespace_mut(owner)?
        .close(fd)
        .map_err(RuntimeError::from)?;
    let _ = runtime.io_registry.close(owner, fd);
    runtime.purge_descriptor_runtime_state(owner, |candidate| candidate == fd);
    finalize_queue_descriptor_close(runtime, &descriptor)?;
    runtime.sync_fdshare_group_from(owner)?;
    record_io_agent_decision(
        runtime,
        IoAgentKind::CloseDescriptorAgent,
        owner,
        fd,
        descriptor.kind(),
        0,
        0,
    );
    Ok(descriptor)
}

pub(crate) fn exec_transition(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
) -> Result<Vec<ObjectDescriptor>, RuntimeError> {
    let closed = runtime.namespace_mut(owner)?.close_on_exec(owner);
    let _ = runtime.io_registry.close_many(owner, &closed);
    runtime.purge_descriptor_runtime_state(owner, |candidate| {
        closed.iter().any(|descriptor| descriptor.fd() == candidate)
    });
    for descriptor in &closed {
        finalize_queue_descriptor_close(runtime, descriptor)?;
    }
    runtime.sync_fdshare_group_from(owner)?;
    Ok(closed)
}

pub(crate) fn open_path(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    path: &str,
) -> Result<Descriptor, RuntimeError> {
    runtime.processes.get(owner)?;
    let node = runtime
        .vfs
        .resolve_node(path, 0)
        .map_err(RuntimeError::from)?;
    let kind = node.kind();
    let node_path = node.path().to_string();
    let capability = runtime.grant_capability(
        owner,
        owner.handle(),
        CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
        node_path.clone(),
    )?;
    let fd = open_descriptor(runtime, owner, capability, kind, node_path.clone())?;
    if kind == ObjectKind::File {
        let payload = runtime
            .vfs
            .node(&node_path)
            .map(|node| node.content().to_vec())
            .map_err(RuntimeError::from)?;
        runtime
            .io_registry
            .replace_payload(owner, fd, &payload)
            .map_err(map_runtime_io_error)?;
    }
    let _ = sync_endpoint_io_state(runtime, owner, fd);
    record_io_agent_decision(
        runtime,
        IoAgentKind::OpenPathAgent,
        owner,
        fd,
        kind,
        u64::from(kind == ObjectKind::File),
        0,
    );
    Ok(fd)
}
