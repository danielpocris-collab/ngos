use super::*;

pub(crate) fn register_readiness(
    runtime: &mut KernelRuntime,
    owner: ProcessId,
    fd: Descriptor,
    interest: ReadinessInterest,
) -> Result<(), RuntimeError> {
    let kind = runtime
        .namespace(owner)?
        .get(fd)
        .map_err(RuntimeError::from)?
        .kind();
    runtime
        .readiness
        .retain(|registration| !(registration.owner == owner && registration.fd == fd));
    runtime.readiness.push(ReadinessRegistration {
        owner,
        fd,
        interest,
    });
    if !runtime.decision_tracing_enabled {
        return Ok(());
    }
    if runtime.io_agent_decisions.len() == 64 {
        runtime.io_agent_decisions.remove(0);
    }
    runtime.io_agent_decisions.push(IoAgentDecisionRecord {
        tick: runtime.current_tick,
        agent: IoAgentKind::ReadinessAgent,
        owner: owner.raw(),
        fd: u64::from(fd.raw()),
        kind: kind as u64,
        detail0: u64::from(interest.readable)
            | (u64::from(interest.writable) << 1)
            | (u64::from(interest.priority) << 2),
        detail1: 0,
    });
    Ok(())
}

pub(crate) fn collect_ready(
    runtime: &KernelRuntime,
) -> Result<Vec<ReadinessRegistration>, RuntimeError> {
    let mut ready = Vec::new();
    for registration in &runtime.readiness {
        let events = poll_io(runtime, registration.owner, registration.fd)?;
        let matches = (registration.interest.readable && events.contains(IoPollEvents::READABLE))
            || (registration.interest.writable && events.contains(IoPollEvents::WRITABLE))
            || (registration.interest.priority && events.contains(IoPollEvents::PRIORITY));
        if matches {
            ready.push(*registration);
        }
    }
    Ok(ready)
}
