//! Canonical subsystem role:
//! - subsystem: verified core
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path role: hard invariant verification surface for the kernel truth
//!
//! Canonical contract families verified here:
//! - capability model contracts
//! - VFS invariant contracts
//! - scheduler state-machine contracts
//! - CPU extended-state lifecycle contracts
//!
//! This module may judge whether the hard kernel core remains valid. Higher
//! layers may inspect or react to this report, but they must not redefine the
//! verification criteria that live here.

use super::*;
use crate::vfs_model::parent_path;
use alloc::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedCoreFamily {
    CapabilityModel,
    VfsInvariants,
    SchedulerStateMachine,
    CpuExtendedStateLifecycle,
    BusIntegrity,
}

impl VerifiedCoreFamily {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CapabilityModel => "capability-model",
            Self::VfsInvariants => "vfs-invariants",
            Self::SchedulerStateMachine => "scheduler-state-machine",
            Self::CpuExtendedStateLifecycle => "cpu-extended-state-lifecycle",
            Self::BusIntegrity => "bus-integrity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCoreViolation {
    pub family: VerifiedCoreFamily,
    pub code: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCoreReport {
    pub capability_model_verified: bool,
    pub vfs_invariants_verified: bool,
    pub scheduler_state_machine_verified: bool,
    pub cpu_extended_state_lifecycle_verified: bool,
    pub bus_integrity_verified: bool,
    pub violations: Vec<VerifiedCoreViolation>,
}

impl VerifiedCoreReport {
    pub fn is_verified(&self) -> bool {
        self.capability_model_verified
            && self.vfs_invariants_verified
            && self.scheduler_state_machine_verified
            && self.cpu_extended_state_lifecycle_verified
            && self.bus_integrity_verified
            && self.violations.is_empty()
    }
}

impl KernelRuntime {
    pub fn verify_core(&self) -> VerifiedCoreReport {
        let mut violations = Vec::new();

        self.verify_capability_model(&mut violations);
        self.verify_vfs_invariants(&mut violations);
        self.verify_scheduler_state_machine(&mut violations);
        self.verify_cpu_extended_state_lifecycle(&mut violations);
        self.verify_bus_integrity(&mut violations);

        VerifiedCoreReport {
            capability_model_verified: !violations
                .iter()
                .any(|entry| entry.family == VerifiedCoreFamily::CapabilityModel),
            vfs_invariants_verified: !violations
                .iter()
                .any(|entry| entry.family == VerifiedCoreFamily::VfsInvariants),
            scheduler_state_machine_verified: !violations
                .iter()
                .any(|entry| entry.family == VerifiedCoreFamily::SchedulerStateMachine),
            cpu_extended_state_lifecycle_verified: !violations
                .iter()
                .any(|entry| entry.family == VerifiedCoreFamily::CpuExtendedStateLifecycle),
            bus_integrity_verified: !violations
                .iter()
                .any(|entry| entry.family == VerifiedCoreFamily::BusIntegrity),
            violations,
        }
    }

    fn verify_capability_model(&self, violations: &mut Vec<VerifiedCoreViolation>) {
        for (_, capability) in self.capabilities.objects.iter() {
            if !self.processes.contains(capability.owner()) {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::CapabilityModel,
                    code: "cap-owner-missing",
                    detail: format!(
                        "cap={} owner={} target={}",
                        capability.id().raw(),
                        capability.owner().raw(),
                        capability.target().id().raw()
                    ),
                });
            }
            if capability.rights().bits() == 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::CapabilityModel,
                    code: "cap-rights-empty",
                    detail: format!(
                        "cap={} owner={}",
                        capability.id().raw(),
                        capability.owner().raw()
                    ),
                });
            }
            if capability.label().trim().is_empty() {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::CapabilityModel,
                    code: "cap-label-empty",
                    detail: format!(
                        "cap={} owner={}",
                        capability.id().raw(),
                        capability.owner().raw()
                    ),
                });
            }
            if capability.target().id().raw() == 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::CapabilityModel,
                    code: "cap-target-unbound",
                    detail: format!(
                        "cap={} owner={}",
                        capability.id().raw(),
                        capability.owner().raw()
                    ),
                });
            }
        }
    }

    fn verify_vfs_invariants(&self, violations: &mut Vec<VerifiedCoreViolation>) {
        if self.vfs.mounts().is_empty() || self.vfs.mounts()[0].mount_path() != "/" {
            violations.push(VerifiedCoreViolation {
                family: VerifiedCoreFamily::VfsInvariants,
                code: "vfs-root-mount-missing",
                detail: String::from("expected root mount at /"),
            });
        }

        let mut mount_paths = BTreeSet::<String>::new();
        for mount in self.vfs.mounts() {
            match normalize_path(mount.mount_path()) {
                Some(normalized) if normalized == mount.mount_path() => {}
                _ => violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::VfsInvariants,
                    code: "vfs-mount-path-invalid",
                    detail: mount.mount_path().to_string(),
                }),
            }
            if !mount_paths.insert(mount.mount_path().to_string()) {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::VfsInvariants,
                    code: "vfs-mount-duplicate",
                    detail: mount.mount_path().to_string(),
                });
            }
        }

        let mut node_paths = BTreeSet::<String>::new();
        let mut node_inodes = BTreeSet::<u64>::new();
        for node in self.vfs.nodes() {
            match normalize_path(node.path()) {
                Some(normalized) if normalized == node.path() => {}
                _ => violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::VfsInvariants,
                    code: "vfs-node-path-invalid",
                    detail: node.path().to_string(),
                }),
            }
            if !node_paths.insert(node.path().to_string()) {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::VfsInvariants,
                    code: "vfs-node-duplicate-path",
                    detail: node.path().to_string(),
                });
            }
            if !node_inodes.insert(node.inode()) {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::VfsInvariants,
                    code: "vfs-node-duplicate-inode",
                    detail: format!("inode={} path={}", node.inode(), node.path()),
                });
            }
            if node.path() != "/" {
                let Some(parent) = parent_path(node.path()) else {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::VfsInvariants,
                        code: "vfs-parent-invalid",
                        detail: node.path().to_string(),
                    });
                    continue;
                };
                if parent != "/"
                    && self
                        .vfs
                        .nodes()
                        .iter()
                        .find(|candidate| candidate.path() == parent)
                        .is_none_or(|candidate| candidate.kind() != ObjectKind::Directory)
                {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::VfsInvariants,
                        code: "vfs-parent-missing-directory",
                        detail: format!("path={} parent={}", node.path(), parent),
                    });
                }
            }
            if self.capabilities.get(node.capability()).is_err() {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::VfsInvariants,
                    code: "vfs-capability-missing",
                    detail: format!("path={} cap={}", node.path(), node.capability().raw()),
                });
            }
            if node.kind() == ObjectKind::Symlink
                && node.link_target().and_then(normalize_path).is_none()
            {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::VfsInvariants,
                    code: "vfs-symlink-target-invalid",
                    detail: node.path().to_string(),
                });
            }
        }
    }

    fn verify_scheduler_state_machine(&self, violations: &mut Vec<VerifiedCoreViolation>) {
        let queued = self.scheduler.queued_threads_by_class();
        let urgent = self.scheduler.queued_urgent_len_by_class();
        let lag_debt = self.scheduler.class_lag_debt();
        let dispatch_counts = self.scheduler.class_dispatch_counts();
        let runtime_ticks = self.scheduler.class_runtime_ticks();
        let wait_ticks = self.scheduler.class_wait_ticks();
        let starved = self.scheduler.starved_classes();
        let cpu_queued_loads = self.scheduler.cpu_queued_loads();
        let logical_cpu_count = self.scheduler.logical_cpu_count();
        let mut seen = BTreeSet::<u64>::new();
        let mut total = 0usize;
        for (tid_raw, assigned_cpu, affinity_mask) in self.scheduler.queued_thread_assignments() {
            if affinity_mask == 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-thread-affinity-empty",
                    detail: format!("tid={tid_raw}"),
                });
            }
            if assigned_cpu >= logical_cpu_count {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-thread-assigned-cpu-invalid",
                    detail: format!(
                        "tid={} cpu={} cpu-count={}",
                        tid_raw, assigned_cpu, logical_cpu_count
                    ),
                });
            } else if (affinity_mask & (1u64 << assigned_cpu)) == 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-thread-affinity-mismatch",
                    detail: format!(
                        "tid={} cpu={} affinity=0x{:x}",
                        tid_raw, assigned_cpu, affinity_mask
                    ),
                });
            }
        }
        for (index, class_queue) in queued.iter().enumerate() {
            total = total.saturating_add(class_queue.len());
            if class_queue.is_empty() && wait_ticks[index] != 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-empty-class-has-wait-ticks",
                    detail: format!("class-index={} wait-ticks={}", index, wait_ticks[index]),
                });
            }
            if class_queue.is_empty() && lag_debt[index] != 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-empty-class-has-lag-debt",
                    detail: format!("class-index={} lag-debt={}", index, lag_debt[index]),
                });
            }
            if dispatch_counts[index] == 0 && runtime_ticks[index] > 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-runtime-without-dispatch",
                    detail: format!(
                        "class-index={} runtime-ticks={}",
                        index, runtime_ticks[index]
                    ),
                });
            }
            if runtime_ticks[index]
                > dispatch_counts[index].saturating_mul(self.scheduler.default_budget() as u64)
            {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-runtime-exceeds-dispatch-budget",
                    detail: format!(
                        "class-index={} runtime-ticks={} dispatches={} budget={}",
                        index,
                        runtime_ticks[index],
                        dispatch_counts[index],
                        self.scheduler.default_budget()
                    ),
                });
            }
            if urgent[index] > class_queue.len() {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-urgent-count-exceeds-queue",
                    detail: format!(
                        "class-index={} urgent={} queued={}",
                        index,
                        urgent[index],
                        class_queue.len()
                    ),
                });
            }
            if starved[index]
                && (class_queue.is_empty()
                    || wait_ticks[index] < self.scheduler.starvation_guard_ticks())
            {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-starvation-flag-invalid",
                    detail: format!(
                        "class-index={} queued={} wait-ticks={}",
                        index,
                        class_queue.len(),
                        wait_ticks[index]
                    ),
                });
            }
            if !starved[index] && class_queue.is_empty() && lag_debt[index] < 0 {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-idle-class-negative-lag-debt",
                    detail: format!("class-index={} lag-debt={}", index, lag_debt[index]),
                });
            }
            for tid in class_queue {
                if !seen.insert(tid.raw()) {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::SchedulerStateMachine,
                        code: "scheduler-duplicate-queued-thread",
                        detail: format!("tid={} class-index={}", tid.raw(), index),
                    });
                }
                let Ok(thread) = self.processes.get_thread(*tid) else {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::SchedulerStateMachine,
                        code: "scheduler-queued-thread-missing",
                        detail: format!("tid={}", tid.raw()),
                    });
                    continue;
                };
                let Ok(process) = self.processes.get(thread.owner()) else {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::SchedulerStateMachine,
                        code: "scheduler-queued-owner-missing",
                        detail: format!("tid={} owner={}", tid.raw(), thread.owner().raw()),
                    });
                    continue;
                };
                if process.state() != ProcessState::Ready {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::SchedulerStateMachine,
                        code: "scheduler-queued-process-not-ready",
                        detail: format!(
                            "tid={} pid={} state={:?}",
                            tid.raw(),
                            thread.owner().raw(),
                            process.state()
                        ),
                    });
                }
            }
        }
        if total != self.scheduler.queued_len() {
            violations.push(VerifiedCoreViolation {
                family: VerifiedCoreFamily::SchedulerStateMachine,
                code: "scheduler-queued-len-mismatch",
                detail: format!(
                    "reported={} enumerated={}",
                    self.scheduler.queued_len(),
                    total
                ),
            });
        }
        if cpu_queued_loads.iter().copied().sum::<usize>() != self.scheduler.queued_len() {
            violations.push(VerifiedCoreViolation {
                family: VerifiedCoreFamily::SchedulerStateMachine,
                code: "scheduler-cpu-load-mismatch",
                detail: format!(
                    "reported={} per-cpu={}",
                    self.scheduler.queued_len(),
                    cpu_queued_loads.iter().copied().sum::<usize>()
                ),
            });
        }
        let cpu_class_loads = self.scheduler.cpu_class_queued_loads();
        for (cpu, class_loads) in cpu_class_loads.iter().enumerate() {
            let per_cpu_total = class_loads.iter().copied().sum::<usize>();
            if per_cpu_total != cpu_queued_loads.get(cpu).copied().unwrap_or(0) {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-cpu-class-load-mismatch",
                    detail: format!(
                        "cpu={} queued-load={} class-total={}",
                        cpu,
                        cpu_queued_loads.get(cpu).copied().unwrap_or(0),
                        per_cpu_total
                    ),
                });
            }
        }
        if let Some(running) = self.scheduler.running() {
            if running.cpu >= logical_cpu_count {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-running-cpu-invalid",
                    detail: format!(
                        "pid={} tid={} cpu={} cpu-count={}",
                        running.pid.raw(),
                        running.tid.raw(),
                        running.cpu,
                        logical_cpu_count
                    ),
                });
            }
            if let Some((assigned_cpu, affinity_mask)) =
                self.scheduler.thread_assignment(running.tid)
                && ((running.cpu >= logical_cpu_count)
                    || ((affinity_mask & (1u64 << running.cpu)) == 0)
                    || assigned_cpu >= logical_cpu_count)
            {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-running-affinity-mismatch",
                    detail: format!(
                        "pid={} tid={} cpu={} assigned={} affinity=0x{:x}",
                        running.pid.raw(),
                        running.tid.raw(),
                        running.cpu,
                        assigned_cpu,
                        affinity_mask
                    ),
                });
            }
            if seen.contains(&running.tid.raw()) {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-running-also-queued",
                    detail: format!("pid={} tid={}", running.pid.raw(), running.tid.raw()),
                });
            }
            match self.processes.get(running.pid) {
                Ok(process) if process.state() == ProcessState::Running => {}
                Ok(process) => violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-running-process-state-invalid",
                    detail: format!(
                        "pid={} tid={} state={:?}",
                        running.pid.raw(),
                        running.tid.raw(),
                        process.state()
                    ),
                }),
                Err(_) => violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::SchedulerStateMachine,
                    code: "scheduler-running-process-missing",
                    detail: format!("pid={} tid={}", running.pid.raw(), running.tid.raw()),
                }),
            }
        }
    }

    fn verify_cpu_extended_state_lifecycle(&self, violations: &mut Vec<VerifiedCoreViolation>) {
        let mut active_threads = Vec::<ThreadId>::new();
        for (_, thread) in self.processes.threads.iter() {
            let profile = thread.cpu_extended_state();
            if profile.xsave_managed {
                if profile.save_area_bytes == 0 || profile.xcr0_mask == 0 {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                        code: "cpu-xsave-profile-incomplete",
                        detail: format!(
                            "tid={} save-area={} xcr0=0x{:x}",
                            thread.tid().raw(),
                            profile.save_area_bytes,
                            profile.xcr0_mask
                        ),
                    });
                }
                match thread.cpu_extended_state_image() {
                    Ok(image) => {
                        if image.bytes.len() != profile.save_area_bytes as usize {
                            violations.push(VerifiedCoreViolation {
                                family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                                code: "cpu-save-area-size-mismatch",
                                detail: format!(
                                    "tid={} profile={} image={}",
                                    thread.tid().raw(),
                                    profile.save_area_bytes,
                                    image.bytes.len()
                                ),
                            });
                        }
                        if !image.bytes.is_empty() && !image.bytes.is_aligned() {
                            violations.push(VerifiedCoreViolation {
                                family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                                code: "cpu-save-area-unaligned",
                                detail: format!("tid={}", thread.tid().raw()),
                            });
                        }
                    }
                    Err(_) => violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                        code: "cpu-save-area-missing",
                        detail: format!(
                            "tid={} save-area={}",
                            thread.tid().raw(),
                            profile.save_area_bytes
                        ),
                    }),
                }
            }
            if profile.active_in_cpu {
                active_threads.push(thread.tid());
            }
        }
        if active_threads.len() > 1 {
            violations.push(VerifiedCoreViolation {
                family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                code: "cpu-multiple-active-threads",
                detail: format!(
                    "tids={}",
                    active_threads
                        .iter()
                        .map(|tid| tid.raw().to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            });
        }
        match (
            &self.active_cpu_extended_state,
            active_threads.first().copied(),
        ) {
            (Some(slot), Some(active_tid)) => {
                if slot.owner_tid != active_tid {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                        code: "cpu-active-slot-thread-mismatch",
                        detail: format!(
                            "slot={} thread={}",
                            slot.owner_tid.raw(),
                            active_tid.raw()
                        ),
                    });
                }
                if !slot.image.profile.xsave_managed {
                    violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                        code: "cpu-active-slot-unmanaged",
                        detail: format!("tid={}", slot.owner_tid.raw()),
                    });
                }
            }
            (Some(slot), None) => violations.push(VerifiedCoreViolation {
                family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                code: "cpu-active-slot-without-thread",
                detail: format!("slot-tid={}", slot.owner_tid.raw()),
            }),
            (None, Some(active_tid)) => violations.push(VerifiedCoreViolation {
                family: VerifiedCoreFamily::CpuExtendedStateLifecycle,
                code: "cpu-thread-active-without-slot",
                detail: format!("tid={}", active_tid.raw()),
            }),
            (None, None) => {}
        }
    }

    fn verify_bus_integrity(&self, violations: &mut Vec<VerifiedCoreViolation>) {
        for (_, peer) in self.bus_peers.objects.iter() {
            if !self.processes.contains(peer.owner) {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::BusIntegrity,
                    code: "bus-peer-owner-missing",
                    detail: format!("peer={} owner={}", peer.id.raw(), peer.owner.raw()),
                });
            }
            if self.domains.get(peer.domain).is_err() {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::BusIntegrity,
                    code: "bus-peer-domain-missing",
                    detail: format!("peer={} domain={}", peer.id.raw(), peer.domain.raw()),
                });
            }
            for endpoint in &peer.attached_endpoints {
                match self.bus_endpoints.get(*endpoint) {
                    Ok(endpoint_info) => {
                        if endpoint_info.domain != peer.domain {
                            violations.push(VerifiedCoreViolation {
                                family: VerifiedCoreFamily::BusIntegrity,
                                code: "bus-peer-endpoint-domain-mismatch",
                                detail: format!(
                                    "peer={} endpoint={} peer-domain={} endpoint-domain={}",
                                    peer.id.raw(),
                                    endpoint.raw(),
                                    peer.domain.raw(),
                                    endpoint_info.domain.raw()
                                ),
                            });
                        }
                        if !endpoint_info.attached_peers.contains(&peer.id) {
                            violations.push(VerifiedCoreViolation {
                                family: VerifiedCoreFamily::BusIntegrity,
                                code: "bus-peer-attachment-not-reciprocated",
                                detail: format!(
                                    "peer={} endpoint={}",
                                    peer.id.raw(),
                                    endpoint.raw()
                                ),
                            });
                        }
                    }
                    Err(_) => violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::BusIntegrity,
                        code: "bus-peer-endpoint-missing",
                        detail: format!("peer={} endpoint={}", peer.id.raw(), endpoint.raw()),
                    }),
                }
            }
        }

        for (_, endpoint) in self.bus_endpoints.objects.iter() {
            if self.domains.get(endpoint.domain).is_err() {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::BusIntegrity,
                    code: "bus-endpoint-domain-missing",
                    detail: format!(
                        "endpoint={} domain={}",
                        endpoint.id.raw(),
                        endpoint.domain.raw()
                    ),
                });
            }
            let Ok(resource) = self.resources.get(endpoint.resource) else {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::BusIntegrity,
                    code: "bus-endpoint-resource-missing",
                    detail: format!(
                        "endpoint={} resource={}",
                        endpoint.id.raw(),
                        endpoint.resource.raw()
                    ),
                });
                continue;
            };
            if resource.domain != endpoint.domain {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::BusIntegrity,
                    code: "bus-endpoint-resource-domain-mismatch",
                    detail: format!(
                        "endpoint={} resource={} endpoint-domain={} resource-domain={}",
                        endpoint.id.raw(),
                        endpoint.resource.raw(),
                        endpoint.domain.raw(),
                        resource.domain.raw()
                    ),
                });
            }
            if resource.kind != ResourceKind::Channel {
                violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::BusIntegrity,
                    code: "bus-endpoint-resource-kind-mismatch",
                    detail: format!(
                        "endpoint={} resource={} kind={:?}",
                        endpoint.id.raw(),
                        endpoint.resource.raw(),
                        resource.kind
                    ),
                });
            }
            match self.stat_path(&endpoint.path) {
                Ok(status) => {
                    if status.kind != ObjectKind::Channel {
                        violations.push(VerifiedCoreViolation {
                            family: VerifiedCoreFamily::BusIntegrity,
                            code: "bus-endpoint-vfs-kind-mismatch",
                            detail: format!(
                                "endpoint={} path={}",
                                endpoint.id.raw(),
                                endpoint.path
                            ),
                        });
                    }
                }
                Err(_) => violations.push(VerifiedCoreViolation {
                    family: VerifiedCoreFamily::BusIntegrity,
                    code: "bus-endpoint-path-missing",
                    detail: format!("endpoint={} path={}", endpoint.id.raw(), endpoint.path),
                }),
            }
            for peer in &endpoint.attached_peers {
                match self.bus_peers.get(*peer) {
                    Ok(peer_info) => {
                        if !peer_info.attached_endpoints.contains(&endpoint.id) {
                            violations.push(VerifiedCoreViolation {
                                family: VerifiedCoreFamily::BusIntegrity,
                                code: "bus-endpoint-attachment-not-reciprocated",
                                detail: format!(
                                    "endpoint={} peer={}",
                                    endpoint.id.raw(),
                                    peer.raw()
                                ),
                            });
                        }
                    }
                    Err(_) => violations.push(VerifiedCoreViolation {
                        family: VerifiedCoreFamily::BusIntegrity,
                        code: "bus-endpoint-peer-missing",
                        detail: format!("endpoint={} peer={}", endpoint.id.raw(), peer.raw()),
                    }),
                }
            }
        }
    }
}
