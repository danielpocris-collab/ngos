use super::*;

pub(crate) fn inspect_io(
    runtime: &KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<&IoObject, RuntimeError> {
    runtime
        .io_registry
        .inspect(owner, fd)
        .map_err(|_| RuntimeError::Descriptor(DescriptorError::InvalidDescriptor))
}

pub(crate) fn inspect_io_layout(
    runtime: &KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<IoPayloadLayoutInfo, RuntimeError> {
    Ok(inspect_io(runtime, owner, fd)?.payload_layout_info())
}

pub(crate) fn poll_io(
    runtime: &KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<IoPollEvents, RuntimeError> {
    if let Some(events) = runtime.endpoint_poll_io(owner, fd)? {
        return Ok(events);
    }
    runtime
        .io_registry
        .poll(owner, fd)
        .map_err(map_runtime_io_error)
}
