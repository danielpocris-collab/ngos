#![cfg_attr(not(test), allow(dead_code))]

use super::*;

pub(super) fn build_memory_violation_summary(capsule: &CrashCapsule) -> MemoryViolationSummary {
    let mut summary = MemoryViolationSummary::EMPTY;
    let mut dominant_counts = [0u16; 8];
    let mut hottest_descriptor_hits = 0u16;
    let mut hottest_descriptor_id = 0u64;
    for violation in capsule
        .watch_tail
        .iter()
        .filter(|entry| entry.sequence != 0)
    {
        summary.total = summary.total.saturating_add(1);
        match violation.kind {
            ViolationKind::Guard => summary.guard_hits = summary.guard_hits.saturating_add(1),
            ViolationKind::Watch => summary.watch_hits = summary.watch_hits.saturating_add(1),
        }
        if violation.suspicion_flags & MEMORY_SUSPECT_REPEATED != 0 {
            summary.repeated_hits = summary.repeated_hits.saturating_add(1);
        }
        if violation.suspicion_flags & MEMORY_SUSPECT_UNDERRUN != 0 {
            summary.underrun_hits = summary.underrun_hits.saturating_add(1);
        }
        if violation.suspicion_flags & MEMORY_SUSPECT_OVERRUN != 0 {
            summary.overrun_hits = summary.overrun_hits.saturating_add(1);
        }
        if violation.suspicion_flags & MEMORY_SUSPECT_INTERIOR != 0 {
            summary.interior_hits = summary.interior_hits.saturating_add(1);
        }
        if matches!(violation.overlap, MemoryOverlapClass::Span) {
            summary.span_hits = summary.span_hits.saturating_add(1);
        }
        let idx = violation.overlap as usize;
        if idx < dominant_counts.len() {
            dominant_counts[idx] = dominant_counts[idx].saturating_add(1);
        }
        let descriptor_hits = capsule
            .watch_tail
            .iter()
            .filter(|entry| entry.sequence != 0 && entry.descriptor_id == violation.descriptor_id)
            .count() as u16;
        if descriptor_hits > hottest_descriptor_hits {
            hottest_descriptor_hits = descriptor_hits;
            hottest_descriptor_id = violation.descriptor_id;
        }
    }
    summary.hottest_descriptor_id = hottest_descriptor_id;
    summary.hottest_descriptor_hits = hottest_descriptor_hits;
    let mut dominant_overlap = MemoryOverlapClass::None;
    let mut dominant_hits = 0u16;
    for class in [
        MemoryOverlapClass::Exact,
        MemoryOverlapClass::Interior,
        MemoryOverlapClass::Prefix,
        MemoryOverlapClass::Suffix,
        MemoryOverlapClass::Span,
        MemoryOverlapClass::LeftRedZone,
        MemoryOverlapClass::RightRedZone,
    ] {
        let hits = dominant_counts[class as usize];
        if hits > dominant_hits {
            dominant_hits = hits;
            dominant_overlap = class;
        }
    }
    summary.dominant_overlap = dominant_overlap;
    summary
}

pub(super) fn emit_memory_debug_summary() {
    serial::write_bytes(b"== memory-debug ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: memory-debug none reason=no-crash-capsule\n");
        return;
    }
    let summary = build_memory_violation_summary(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: memory-debug total={} guards={} watches={} repeated={} underrun={} overrun={} interior={} span={} hottest_descriptor={} hottest_hits={} dominant_overlap={}\n",
        summary.total,
        summary.guard_hits,
        summary.watch_hits,
        summary.repeated_hits,
        summary.underrun_hits,
        summary.overrun_hits,
        summary.interior_hits,
        summary.span_hits,
        summary.hottest_descriptor_id,
        summary.hottest_descriptor_hits,
        memory_overlap_name(summary.dominant_overlap)
    ));
}

pub(super) fn emit_memory_lineage_summary() {
    serial::write_bytes(b"== memory-lineage ==\n");
    let summary = build_memory_lineage_summary();
    serial::print(format_args!(
        "ngos/x86_64: memory-lineage total_versions={} writes={} copies={} zeros={} dmas={} frees={} latest_version={} latest_parent={} latest_digest={:#x} hottest_object={} hottest_versions={}\n",
        summary.total_versions,
        summary.writes,
        summary.copies,
        summary.zeros,
        summary.dmas,
        summary.frees,
        summary.latest_version_id,
        summary.latest_parent_version_id,
        summary.latest_digest,
        summary.hottest_object_id,
        summary.hottest_object_versions
    ));
}

pub(super) fn emit_smp_timeline_summary() {
    serial::write_bytes(b"== smp-timeline ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: smp-timeline none reason=no-crash-capsule\n");
        return;
    }
    let summary = build_smp_timeline_from_trace(&capsule);
    for cpu in summary.iter().filter(|entry| entry.valid) {
        serial::print(format_args!(
            "ngos/x86_64: smp cpu={} apic={} first_seq={} last_seq={} events={} first_stage={} last_stage={} req={} cmp={} irq={} path={} divergence={}\n",
            cpu.cpu_slot,
            cpu.apic_id,
            cpu.first_sequence,
            cpu.last_sequence,
            cpu.event_count,
            stage_name(cpu.first_stage),
            stage_name(cpu.last_stage),
            cpu.request_id,
            cpu.completion_id,
            cpu.irq_id,
            diagnostics_path_name(cpu.dominant_path),
            cpu.divergence_suspected as u16
        ));
    }
    let divergence = build_smp_divergence_summary(&summary);
    if divergence.valid {
        serial::print(format_args!(
            "ngos/x86_64: smp-divergence cpu_a={} cpu_b={} gap={} stage_a={} stage_b={} path_a={} path_b={} req={} cmp={} irq={}\n",
            divergence.cpu_a,
            divergence.cpu_b,
            divergence.sequence_gap,
            stage_name(divergence.stage_a),
            stage_name(divergence.stage_b),
            diagnostics_path_name(divergence.path_a),
            diagnostics_path_name(divergence.path_b),
            divergence.request_id,
            divergence.completion_id,
            divergence.irq_id
        ));
    }
}

pub(super) fn emit_active_window_summary() {
    serial::write_bytes(b"== active-window ==\n");
    let window = unsafe { *TRACE_STORAGE.current_window.get() };
    serial::print(format_args!(
        "ngos/x86_64: active-window valid={} syscall={} fd={} op={} device={} state={} path={} req={} cmp={}\n",
        window.valid as u16,
        window.syscall_id,
        window.fd,
        window.request_op,
        window.device_id,
        window.completion_state,
        diagnostics_path_name(window.path),
        window.request_id,
        window.completion_id
    ));
}

pub(super) fn emit_syscall_frontier_summary() {
    serial::write_bytes(b"== syscall-frontier ==\n");
    let last_enter = unsafe { *TRACE_STORAGE.last_syscall_enter.get() };
    let last_exit = unsafe { *TRACE_STORAGE.last_syscall_exit.get() };
    if !last_enter.valid {
        serial::write_bytes(
            b"ngos/x86_64: syscall-frontier none result=ok reason=no-syscall-recorded\n",
        );
        return;
    }
    serial::print(format_args!(
        "ngos/x86_64: last-confirmed-syscall valid={} syscall={} stage_name={} cpu={} req={} cmp={} irq={} result={} errno={} a0={:#x} a1={:#x} a2={:#x}\n",
        last_exit.valid,
        last_exit.syscall_id,
        stage_name(last_exit.stage),
        last_exit.cpu_slot,
        last_exit.request_id,
        last_exit.completion_id,
        last_exit.irq_id,
        if last_exit.result_ok { "ok" } else { "fail" },
        last_exit.errno,
        last_exit.arg0,
        last_exit.arg1,
        last_exit.arg2
    ));
}

pub(super) fn emit_exact_localization_summary() {
    serial::write_bytes(b"== exact-localization ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: exact-localization none reason=no-crash-capsule\n");
        return;
    }
    let boundary = earliest_preventable_boundary(&capsule);
    if boundary.valid {
        serial::print(format_args!(
            "ngos/x86_64: exact-localization stage={} path={} sequence={} req={} cmp={} irq={} reason={} action={}\n",
            stage_name(boundary.stage),
            diagnostics_path_name(boundary.path),
            boundary.sequence,
            boundary.request_id,
            boundary.completion_id,
            boundary.irq_id,
            boundary.reason,
            boundary.action
        ));
    } else {
        serial::write_bytes(b"ngos/x86_64: exact-localization none reason=no-boundary\n");
    }
}

pub(super) fn emit_fault_summary() {
    serial::write_bytes(b"== fault-summary ==\n");
    let fault = last_fault();
    serial::print(format_args!(
        "ngos/x86_64: fault valid={} vector={} stage={} cpu={} cr2={:#x} code={:#x} rip={:#x}\n",
        fault.valid as u16,
        fault.vector,
        stage_name(fault.stage),
        fault.cpu_slot,
        fault.cr2,
        fault.error_code,
        fault.rip
    ));
}

pub(super) fn emit_reason_summary() {
    serial::write_bytes(b"== semantic-reasons ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: reason none reason=no-crash-capsule\n");
        return;
    }
    emit_semantic_reason_bucket(
        "syscall",
        &[
            (
                "write_syscall_reject_fault",
                capsule.semantic_reasons.write_syscall_reject_fault,
            ),
            (
                "write_syscall_reject_guard",
                capsule.semantic_reasons.write_syscall_reject_guard,
            ),
            (
                "write_syscall_reject_watch",
                capsule.semantic_reasons.write_syscall_reject_watch,
            ),
        ],
    );
    emit_semantic_reason_bucket(
        "block",
        &[
            (
                "submit_device_request_reject_fault",
                capsule.semantic_reasons.submit_device_request_reject_fault,
            ),
            (
                "submit_device_request_reject_guard",
                capsule.semantic_reasons.submit_device_request_reject_guard,
            ),
            (
                "submit_device_request_reject_watch",
                capsule.semantic_reasons.submit_device_request_reject_watch,
            ),
        ],
    );
    emit_semantic_reason_bucket(
        "completion",
        &[
            (
                "completion_publish_reject_fault",
                capsule.semantic_reasons.completion_publish_reject_fault,
            ),
            (
                "completion_publish_reject_guard",
                capsule.semantic_reasons.completion_publish_reject_guard,
            ),
            (
                "completion_publish_reject_watch",
                capsule.semantic_reasons.completion_publish_reject_watch,
            ),
        ],
    );
}

pub(super) fn emit_violation_summary() {
    serial::write_bytes(b"== violation-summary ==\n");
    let capsule = crash_capsule();
    if !capsule.valid {
        serial::write_bytes(b"ngos/x86_64: violation none reason=no-crash-capsule\n");
        return;
    }
    let dominant = dominant_violation_text(&capsule);
    let summary = build_memory_violation_summary(&capsule);
    serial::print(format_args!(
        "ngos/x86_64: violation dominant={} total={} guards={} watches={} repeated={} dominant_overlap={}\n",
        dominant,
        summary.total,
        summary.guard_hits,
        summary.watch_hits,
        summary.repeated_hits,
        memory_overlap_name(summary.dominant_overlap)
    ));
}

pub(super) fn emit_reprobe_summary() {
    serial::write_bytes(b"== reprobe-state ==\n");
    let reprobe = reprobe_policy_on_boot();
    serial::print(format_args!(
        "ngos/x86_64: reprobe mode={:?} target_path={:?} target_stage={} stage_name={} checkpoint={:#x} escalation={} crashes={}\n",
        reprobe.mode,
        reprobe.target_path,
        reprobe.target_stage,
        stage_name(reprobe.target_stage),
        reprobe.target_checkpoint,
        reprobe.escalation,
        reprobe.crash_count
    ));
}

pub(super) fn emit_trace_summary() {
    serial::write_bytes(b"== trace-summary ==\n");
    let replay = replay_ids();
    let window = unsafe { *TRACE_STORAGE.current_window.get() };
    serial::print(format_args!(
        "ngos/x86_64: trace-summary replay_seq={} request_id={} completion_id={} irq_id={} active_window={} syscall={} fd={} op={} device={} state={} path={:?} mode={:?}\n",
        replay.sequence,
        replay.request_id,
        replay.completion_id,
        replay.irq_id,
        window.valid,
        window.syscall_id,
        window.fd,
        window.request_op,
        window.device_id,
        window.completion_state,
        window.path,
        mode()
    ));
}

#[cfg_attr(test, allow(dead_code))]
pub(super) fn emit_boot_memory_summary() {
    serial::write_bytes(b"== boot-memory ==\n");
    let snapshot = boot_locator::early_snapshot();
    serial::print(format_args!(
        "ngos/x86_64: boot-memory magic={:#x} cursor={} wrapped={} last_seq={}\n",
        snapshot.magic, snapshot.cursor, snapshot.wrapped as u16, snapshot.last.sequence
    ));
}

#[cfg_attr(test, allow(dead_code))]
pub(super) fn emit_boot_locator_summary() {
    serial::write_bytes(b"== boot-locator ==\n");
    for record in boot_locator::recent(16)
        .iter()
        .copied()
        .filter(|entry| entry.sequence != 0)
    {
        serial::print(format_args!(
            "ngos/x86_64: boot-locator sequence={} stage={} kind={} severity={} checkpoint={:#x} payload0_label={:?} payload0={:#x} payload1_label={:?} payload1={:#x}\n",
            record.sequence,
            stage_name(record.stage as u16),
            record.kind as u16,
            record.severity as u16,
            record.checkpoint,
            record.payload0_label,
            record.payload0,
            record.payload1_label,
            record.payload1
        ));
    }
}
