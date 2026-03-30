#![cfg_attr(not(test), allow(dead_code))]

use super::*;

pub(super) fn replay_event_priority(kind: ChronoscopeRuntimeEventKind) -> u16 {
    match kind {
        ChronoscopeRuntimeEventKind::RequestStart => 1,
        ChronoscopeRuntimeEventKind::IrqEnter => 2,
        ChronoscopeRuntimeEventKind::ContractTransition => 3,
        ChronoscopeRuntimeEventKind::ResourceClaim => 4,
        ChronoscopeRuntimeEventKind::ResourceWait => 5,
        ChronoscopeRuntimeEventKind::ViolationObserved => 6,
        ChronoscopeRuntimeEventKind::SuspectPromoted => 7,
        ChronoscopeRuntimeEventKind::RequestComplete => 8,
        ChronoscopeRuntimeEventKind::IrqExit => 9,
        ChronoscopeRuntimeEventKind::FaultMarker => 10,
        ChronoscopeRuntimeEventKind::DivergenceHint => 11,
        ChronoscopeRuntimeEventKind::CapabilityDerive => 12,
        _ => 13,
    }
}

#[cfg_attr(test, allow(dead_code))]
pub(super) fn replay_event_precedes(
    left: ChronoscopeRuntimeEventRecord,
    right: ChronoscopeRuntimeEventRecord,
) -> bool {
    if left.core_id == right.core_id {
        return left.local_sequence < right.local_sequence;
    }
    if left.event_id == right.causal_parent {
        return true;
    }
    if right.event_id == left.causal_parent {
        return false;
    }
    let left_corr =
        left.correlation.request_id ^ left.correlation.completion_id ^ left.correlation.irq_id;
    let right_corr =
        right.correlation.request_id ^ right.correlation.completion_id ^ right.correlation.irq_id;
    if left_corr != 0 && left_corr == right_corr {
        let left_priority = replay_event_priority(left.kind);
        let right_priority = replay_event_priority(right.kind);
        if left_priority != right_priority {
            return left_priority < right_priority;
        }
    }
    if left.uptime_us != right.uptime_us {
        return left.uptime_us < right.uptime_us;
    }
    if left.core_id != right.core_id {
        return left.core_id < right.core_id;
    }
    left.local_sequence < right.local_sequence
}

pub(super) fn replay_trace_event_precedes(left: ReplayEvent, right: ReplayEvent) -> bool {
    if left.core_id == right.core_id {
        return left.local_sequence < right.local_sequence;
    }
    if left.event_id == right.causal_parent {
        return true;
    }
    if right.event_id == left.causal_parent {
        return false;
    }
    let left_corr =
        left.correlation.request_id ^ left.correlation.completion_id ^ left.correlation.irq_id;
    let right_corr =
        right.correlation.request_id ^ right.correlation.completion_id ^ right.correlation.irq_id;
    if left_corr != 0 && left_corr == right_corr {
        let left_priority = replay_event_priority(left.kind);
        let right_priority = replay_event_priority(right.kind);
        if left_priority != right_priority {
            return left_priority < right_priority;
        }
    }
    if left.core_id != right.core_id {
        return left.core_id < right.core_id;
    }
    left.local_sequence < right.local_sequence
}

pub(super) fn replay_sequence_from_runtime(
    window: &ChronoscopeRuntimeEventWindow,
) -> ReplaySequence {
    PERF_REPLAY_EXECUTIONS.fetch_add(1, Ordering::Relaxed);
    if !window.valid || window.total_events == 0 {
        return ReplaySequence::EMPTY;
    }
    let mut sequence = ReplaySequence::EMPTY;
    sequence.valid = true;
    sequence.total_events = window.total_events;
    let mut index = 0usize;
    while index < window.total_events as usize {
        let event = window.events[index];
        sequence.events[index] = ReplayEvent {
            valid: event.valid,
            event_id: event.event_id,
            core_id: event.core_id,
            local_sequence: event.local_sequence,
            kind: event.kind,
            correlation: CorrelationKey {
                request_id: event.correlation.request_id,
                completion_id: event.correlation.completion_id,
                irq_id: event.correlation.irq_id,
            },
            capability_id: event.capability_id,
            object_key: event.object_key,
            causal_parent: event.causal_parent,
            flags: event.flags,
        };
        sequence.per_core_counts[event.core_id as usize] =
            sequence.per_core_counts[event.core_id as usize].saturating_add(1);
        index += 1;
    }
    let mut outer = 0usize;
    while outer < sequence.total_events as usize {
        let mut best = outer;
        let mut inner = outer + 1;
        while inner < sequence.total_events as usize {
            let left = sequence.events[inner];
            let right = sequence.events[best];
            if left.valid && right.valid && replay_trace_event_precedes(left, right) {
                best = inner;
            }
            inner += 1;
        }
        if best != outer {
            let event = sequence.events[outer];
            sequence.events[outer] = sequence.events[best];
            sequence.events[best] = event;
        }
        outer += 1;
    }
    debug_assert!(validate_replay_sequence(&sequence));
    PERF_REPLAY_STEPS.fetch_add(sequence.total_events as u64, Ordering::Relaxed);
    sequence
}

pub(super) fn validate_replay_sequence(sequence: &ReplaySequence) -> bool {
    let mut last_per_core = [0u64; MAX_TRACE_CPUS];
    let mut seen_per_core = [false; MAX_TRACE_CPUS];
    let mut index = 0usize;
    while index < sequence.total_events as usize {
        let event = sequence.events[index];
        if !event.valid {
            return false;
        }
        let core = event.core_id as usize;
        if core >= MAX_TRACE_CPUS {
            return false;
        }
        if seen_per_core[core] && event.local_sequence < last_per_core[core] {
            return false;
        }
        seen_per_core[core] = true;
        last_per_core[core] = event.local_sequence;
        index += 1;
    }
    true
}

pub(super) fn replay_state_slot_for_contract(
    slots: &mut [ReplayContractState; REPLAY_STATE_SLOT_LIMIT],
    key: u64,
) -> &mut ReplayContractState {
    let mut empty = 0usize;
    let mut index = 0usize;
    while index < slots.len() {
        if slots[index].valid && slots[index].key == key {
            return &mut slots[index];
        }
        if !slots[index].valid {
            empty = index;
            break;
        }
        index += 1;
    }
    slots[empty].valid = true;
    slots[empty].key = key;
    &mut slots[empty]
}

pub(super) fn replay_state_slot_for_resource(
    slots: &mut [ReplayResourceState; REPLAY_STATE_SLOT_LIMIT],
    key: u64,
) -> &mut ReplayResourceState {
    let mut empty = 0usize;
    let mut index = 0usize;
    while index < slots.len() {
        if slots[index].valid && slots[index].key == key {
            return &mut slots[index];
        }
        if !slots[index].valid {
            empty = index;
            break;
        }
        index += 1;
    }
    slots[empty].valid = true;
    slots[empty].key = key;
    &mut slots[empty]
}

pub(super) fn replay_state_slot_for_request(
    slots: &mut [ReplayRequestState; REPLAY_STATE_SLOT_LIMIT],
    request_id: u64,
) -> &mut ReplayRequestState {
    let mut empty = 0usize;
    let mut index = 0usize;
    while index < slots.len() {
        if slots[index].valid && slots[index].request_id == request_id {
            return &mut slots[index];
        }
        if !slots[index].valid {
            empty = index;
            break;
        }
        index += 1;
    }
    slots[empty].valid = true;
    slots[empty].request_id = request_id;
    &mut slots[empty]
}

pub(super) fn apply_replay_event(state: &mut ReplaySemanticState, event: ReplayEvent) {
    state.last_capability = event.capability_id;
    match event.kind {
        ChronoscopeRuntimeEventKind::RequestStart => {
            let slot =
                replay_state_slot_for_request(&mut state.requests, event.correlation.request_id);
            slot.phase = 1;
            slot.completion_id = event.correlation.completion_id;
        }
        ChronoscopeRuntimeEventKind::RequestComplete => {
            let slot =
                replay_state_slot_for_request(&mut state.requests, event.correlation.request_id);
            slot.phase = 2;
            slot.completion_id = event.correlation.completion_id;
        }
        ChronoscopeRuntimeEventKind::ContractTransition => {
            let slot = replay_state_slot_for_contract(&mut state.contracts, event.object_key);
            slot.stage = event.core_id;
            slot.status = replay_event_priority(event.kind);
        }
        ChronoscopeRuntimeEventKind::ResourceClaim
        | ChronoscopeRuntimeEventKind::ResourceRelease
        | ChronoscopeRuntimeEventKind::ResourceWait => {
            let slot = replay_state_slot_for_resource(&mut state.resources, event.object_key);
            slot.owner = event.core_id;
            slot.state = replay_event_priority(event.kind);
        }
        ChronoscopeRuntimeEventKind::ViolationObserved => {
            state.violation_flags |= event.flags;
        }
        ChronoscopeRuntimeEventKind::SuspectPromoted => {
            state.suspect_flags |= 1;
        }
        ChronoscopeRuntimeEventKind::FaultMarker => {
            state.last_writer = event.event_id.0;
        }
        _ => {}
    }
}

pub(super) fn validate_replay_invariants(state: &ReplaySemanticState) -> bool {
    state.violation_flags == 0
        || state.suspect_flags != 0
        || state.last_capability != CapabilityId::NONE
}
