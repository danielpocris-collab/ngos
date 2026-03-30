use super::*;

#[path = "event_queue_runtime/control.rs"]
mod control;
#[path = "event_queue_runtime/emit.rs"]
mod emit;

pub(crate) use control::*;
pub(crate) use emit::*;

pub(crate) fn event_queue_mut_by_binding(
    runtime: &mut KernelRuntime,
    binding: QueueDescriptorTarget,
) -> Result<&mut EventQueue, RuntimeError> {
    match binding {
        QueueDescriptorTarget::Event { owner, queue, .. } => runtime
            .event_queues
            .iter_mut()
            .find(|candidate| candidate.id == queue && candidate.owner == owner)
            .ok_or(EventQueueError::InvalidQueue.into()),
        QueueDescriptorTarget::Sleep { .. } => {
            Err(RuntimeError::EventQueue(EventQueueError::InvalidQueue))
        }
    }
}
