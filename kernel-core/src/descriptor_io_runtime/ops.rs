use super::*;

fn record_io_runtime_decision(
    runtime: &mut KernelRuntime,
    agent: IoAgentKind,
    owner: ProcessId,
    fd: Descriptor,
    detail0: u64,
    detail1: u64,
) -> Result<(), RuntimeError> {
    if !runtime.decision_tracing_enabled {
        return Ok(());
    }
    let kind = runtime
        .namespace(owner)?
        .get(fd)
        .map_err(RuntimeError::from)?
        .kind();
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
    Ok(())
}

pub(crate) fn read_io(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    len: usize,
) -> Result<Vec<u8>, RuntimeError> {
    if let Some(bytes) = runtime.endpoint_read_io(owner, fd)? {
        let read_len = bytes.len().min(len);
        runtime.sync_fdshare_group_io_from(owner);
        runtime.notify_descriptor_ready(owner, fd)?;
        record_io_runtime_decision(
            runtime,
            IoAgentKind::ReadAgent,
            owner,
            fd,
            read_len as u64,
            1,
        )?;
        return Ok(bytes.into_iter().take(len).collect());
    }
    let bytes = runtime
        .io_registry
        .read(owner, fd, len)
        .map_err(map_runtime_io_error)?;
    runtime.sync_fdshare_group_io_from(owner);
    runtime.notify_descriptor_ready(owner, fd)?;
    record_io_runtime_decision(
        runtime,
        IoAgentKind::ReadAgent,
        owner,
        fd,
        bytes.len() as u64,
        0,
    )?;
    Ok(bytes)
}

pub(crate) fn read_io_vectored(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    segments: &[usize],
) -> Result<Vec<Vec<u8>>, RuntimeError> {
    let bytes = runtime
        .io_registry
        .read_vectored(owner, fd, segments)
        .map_err(map_runtime_io_error)?;
    runtime.sync_fdshare_group_io_from(owner);
    runtime.notify_descriptor_ready(owner, fd)?;
    Ok(bytes)
}

pub(crate) fn read_io_vectored_with_layout(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    segments: &[usize],
) -> Result<(Vec<Vec<u8>>, IoPayloadLayoutInfo), RuntimeError> {
    let layout = inspect_io_layout(runtime, owner, fd)?;
    let bytes = read_io_vectored(runtime, owner, fd, segments)?;
    Ok((bytes, layout))
}

pub(crate) fn write_io(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    bytes: &[u8],
) -> Result<usize, RuntimeError> {
    if let Some(written) = runtime.endpoint_write_io(owner, fd, bytes)? {
        runtime.sync_fdshare_group_io_from(owner);
        runtime.notify_descriptor_ready(owner, fd)?;
        record_io_runtime_decision(
            runtime,
            IoAgentKind::WriteAgent,
            owner,
            fd,
            written as u64,
            1,
        )?;
        return Ok(written);
    }
    let written = runtime
        .io_registry
        .write(owner, fd, bytes)
        .map_err(map_runtime_io_error)?;
    sync_vfs_file_payload(runtime, owner, fd)?;
    runtime.sync_fdshare_group_io_from(owner);
    runtime.notify_descriptor_ready(owner, fd)?;
    record_io_runtime_decision(
        runtime,
        IoAgentKind::WriteAgent,
        owner,
        fd,
        written as u64,
        0,
    )?;
    Ok(written)
}

pub(crate) fn write_io_vectored(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    segments: &[Vec<u8>],
) -> Result<usize, RuntimeError> {
    let written = runtime
        .io_registry
        .write_vectored(owner, fd, segments)
        .map_err(map_runtime_io_error)?;
    sync_vfs_file_payload(runtime, owner, fd)?;
    runtime.sync_fdshare_group_io_from(owner);
    runtime.notify_descriptor_ready(owner, fd)?;
    record_io_runtime_decision(
        runtime,
        IoAgentKind::WriteAgent,
        owner,
        fd,
        written as u64,
        2,
    )?;
    Ok(written)
}

pub(crate) fn control_io(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    opcode: u32,
) -> Result<u32, RuntimeError> {
    if let Some(response) = runtime.endpoint_control_io(owner, fd, opcode)? {
        runtime.sync_fdshare_group_io_from(owner);
        runtime.notify_descriptor_ready(owner, fd)?;
        return Ok(response);
    }
    let response = runtime
        .io_registry
        .control(owner, fd, opcode)
        .map_err(map_runtime_io_error)?;
    runtime.sync_fdshare_group_io_from(owner);
    runtime.notify_descriptor_ready(owner, fd)?;
    Ok(response)
}

fn sync_vfs_file_payload(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
) -> Result<(), RuntimeError> {
    let (kind, path, payload) = {
        let descriptor = runtime
            .namespace(owner)?
            .get(fd)
            .map_err(RuntimeError::from)?;
        let io = runtime.inspect_io(owner, fd)?;
        (
            descriptor.kind(),
            descriptor.name().to_string(),
            io.payload().to_vec(),
        )
    };
    if kind != ObjectKind::File {
        return Ok(());
    }
    if let Ok(node) = runtime.vfs.node_mut(&path) {
        node.set_content(payload);
    }
    Ok(())
}

pub(crate) fn fcntl(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    cmd: FcntlCmd,
) -> Result<FcntlResult, RuntimeError> {
    let result = match cmd {
        FcntlCmd::GetFl | FcntlCmd::GetFd => {
            FcntlResult::Flags(runtime.descriptor_flags(owner, fd)?)
        }
        FcntlCmd::SetFl { nonblock } => {
            runtime.set_descriptor_nonblock(owner, fd, nonblock)?;
            FcntlResult::Updated(runtime.descriptor_flags(owner, fd)?)
        }
        FcntlCmd::SetFd { cloexec } => {
            runtime.set_descriptor_cloexec(owner, fd, cloexec)?;
            FcntlResult::Updated(runtime.descriptor_flags(owner, fd)?)
        }
    };
    let detail0 = match cmd {
        FcntlCmd::GetFl => 0,
        FcntlCmd::GetFd => 1,
        FcntlCmd::SetFl { nonblock } => 2 | (u64::from(nonblock) << 8),
        FcntlCmd::SetFd { cloexec } => 3 | (u64::from(cloexec) << 8),
    };
    record_io_runtime_decision(runtime, IoAgentKind::FcntlAgent, owner, fd, detail0, 0)?;
    Ok(result)
}
