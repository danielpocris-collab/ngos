use super::*;

#[path = "sleep_queue_runtime/lifecycle.rs"]
mod lifecycle;
#[path = "sleep_queue_runtime/wait_ops.rs"]
mod wait_ops;

pub(crate) use lifecycle::*;
pub(crate) use wait_ops::*;

pub(crate) fn record_wait_agent_decision(
    runtime: &mut KernelRuntime,
    agent: WaitAgentKind,
    owner: ProcessId,
    queue: SleepQueueId,
    channel: u64,
    detail0: u64,
    detail1: u64,
) {
    if !runtime.decision_tracing_enabled {
        return;
    }
    if runtime.wait_agent_decisions.len() == 64 {
        runtime.wait_agent_decisions.remove(0);
    }
    runtime.wait_agent_decisions.push(WaitAgentDecisionRecord {
        tick: runtime.current_tick,
        agent,
        owner: owner.raw(),
        queue: queue.0,
        channel,
        detail0,
        detail1,
    });
}

pub(crate) fn sleep_queue_mut_by_binding(
    runtime: &mut KernelRuntime,
    binding: QueueDescriptorTarget,
) -> Result<&mut RuntimeSleepQueue, RuntimeError> {
    match binding {
        QueueDescriptorTarget::Sleep { owner, queue } => runtime
            .sleep_queues
            .iter_mut()
            .find(|candidate| candidate.id == queue && candidate.owner == owner)
            .ok_or(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound)),
        QueueDescriptorTarget::Event { .. } => {
            Err(RuntimeError::SleepQueue(SleepQueueError::WaiterNotFound))
        }
    }
}
