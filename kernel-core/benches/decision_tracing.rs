use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ngos_kernel_core::{
    CapabilityRights, ContractKind, FcntlCmd, Handle, KernelRuntime, MemoryAdvice, ObjectHandle,
    ObjectKind, ProcessState, ProcessTable, ReadinessInterest, ResourceKind, Scheduler,
    SchedulerClass,
};

fn benchmark_resource_claim_handoff(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_claim_handoff");
    group.throughput(Throughput::Elements(1));
    for tracing in [false, true] {
        group.bench_function(
            BenchmarkId::new("tracing", if tracing { "on" } else { "off" }),
            |b| {
                b.iter(|| {
                    let mut runtime = KernelRuntime::host_runtime_default();
                    runtime.set_decision_tracing_enabled(tracing);
                    let owner = runtime
                        .spawn_process("owner", None, SchedulerClass::Interactive)
                        .unwrap();
                    let domain = runtime.create_domain(owner, None, "display").unwrap();
                    let resource = runtime
                        .create_resource(owner, domain, ResourceKind::Device, "gpu0")
                        .unwrap();
                    let primary = runtime
                        .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
                        .unwrap();
                    let mirror = runtime
                        .create_contract(owner, domain, resource, ContractKind::Display, "mirror")
                        .unwrap();
                    let recorder = runtime
                        .create_contract(owner, domain, resource, ContractKind::Display, "record")
                        .unwrap();
                    let _ = runtime.claim_resource_via_contract(primary).unwrap();
                    let _ = runtime.claim_resource_via_contract(mirror).unwrap();
                    let _ = runtime.claim_resource_via_contract(recorder).unwrap();
                    let _ = runtime
                        .release_claimed_resource_via_contract(primary)
                        .unwrap();
                });
            },
        );
    }
    group.finish();
}

fn benchmark_wait_wake(c: &mut Criterion) {
    let mut group = c.benchmark_group("wait_wake_requeue");
    group.throughput(Throughput::Elements(1));
    for tracing in [false, true] {
        group.bench_function(
            BenchmarkId::new("tracing", if tracing { "on" } else { "off" }),
            |b| {
                b.iter(|| {
                    let mut runtime = KernelRuntime::host_runtime_default();
                    runtime.set_decision_tracing_enabled(tracing);
                    let owner = runtime
                        .spawn_process("waiter", None, SchedulerClass::Interactive)
                        .unwrap();
                    runtime.tick().unwrap();
                    let queue = runtime.create_sleep_queue(owner).unwrap();
                    runtime.sleep_on_queue(owner, queue, 0x11, 5, None).unwrap();
                    let _ = runtime
                        .requeue_sleep_queue(owner, queue, 0x11, 0x22, 1)
                        .unwrap();
                    let _ = runtime.wake_one_sleep_queue(owner, queue, 0x22).unwrap();
                });
            },
        );
    }
    group.finish();
}

fn benchmark_descriptor_io(c: &mut Criterion) {
    let mut group = c.benchmark_group("descriptor_io");
    group.throughput(Throughput::Elements(1));
    for tracing in [false, true] {
        group.bench_function(
            BenchmarkId::new("tracing", if tracing { "on" } else { "off" }),
            |b| {
                b.iter(|| {
                    let mut runtime = KernelRuntime::host_runtime_default();
                    runtime.set_decision_tracing_enabled(tracing);
                    let owner = runtime
                        .spawn_process("io", None, SchedulerClass::Interactive)
                        .unwrap();
                    let root = runtime
                        .grant_capability(
                            owner,
                            ObjectHandle::new(Handle::new(90_000), 0),
                            CapabilityRights::READ | CapabilityRights::WRITE,
                            "root",
                        )
                        .unwrap();
                    let file = runtime
                        .grant_capability(
                            owner,
                            ObjectHandle::new(Handle::new(90_001), 0),
                            CapabilityRights::READ
                                | CapabilityRights::WRITE
                                | CapabilityRights::DUPLICATE,
                            "note",
                        )
                        .unwrap();
                    runtime
                        .create_vfs_node("/", ObjectKind::Directory, root)
                        .unwrap();
                    runtime
                        .create_vfs_node("/tmp", ObjectKind::Directory, root)
                        .unwrap();
                    runtime
                        .create_vfs_node("/tmp/note", ObjectKind::File, file)
                        .unwrap();
                    let fd = runtime.open_path(owner, "/tmp/note").unwrap();
                    let _ = runtime.write_io(owner, fd, b"payload-data").unwrap();
                    let _ = runtime
                        .fcntl(owner, fd, FcntlCmd::SetFl { nonblock: true })
                        .unwrap();
                    runtime
                        .register_readiness(
                            owner,
                            fd,
                            ReadinessInterest {
                                readable: true,
                                writable: true,
                                priority: false,
                            },
                        )
                        .unwrap();
                    let _ = runtime.collect_ready().unwrap();
                    let dup = runtime.duplicate_descriptor(owner, fd).unwrap();
                    let _ = runtime.read_io(owner, dup, 64).unwrap();
                    let _ = runtime.close_descriptor(owner, fd).unwrap();
                    let _ = runtime.close_descriptor(owner, dup).unwrap();
                });
            },
        );
    }
    group.finish();
}

fn benchmark_scheduler(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_tick_block_wake");
    group.throughput(Throughput::Elements(1));
    for tracing in [false, true] {
        group.bench_function(
            BenchmarkId::new("tracing", if tracing { "on" } else { "off" }),
            |b| {
                b.iter(|| {
                    let mut processes = ProcessTable::new(1, 64);
                    let worker = processes.spawn("worker", None).unwrap();
                    let peer = processes.spawn("peer", None).unwrap();
                    let mut scheduler = Scheduler::new(1);
                    scheduler.set_decision_tracing_enabled(tracing);
                    scheduler
                        .enqueue(&mut processes, worker, SchedulerClass::Interactive)
                        .unwrap();
                    scheduler
                        .enqueue(&mut processes, peer, SchedulerClass::BestEffort)
                        .unwrap();
                    let _ = scheduler.tick(&mut processes).unwrap();
                    let _ = scheduler.block_running(&mut processes).unwrap();
                    scheduler
                        .wake(&mut processes, worker, SchedulerClass::LatencyCritical)
                        .unwrap();
                    let _ = scheduler.tick(&mut processes).unwrap();
                    let _ = processes.get(worker).unwrap().state() == ProcessState::Running;
                });
            },
        );
    }
    group.finish();
}

fn benchmark_vm_cow_fault(c: &mut Criterion) {
    let mut group = c.benchmark_group("vm_cow_fault_bridge");
    group.throughput(Throughput::Elements(1));
    for tracing in [false, true] {
        group.bench_function(
            BenchmarkId::new("tracing", if tracing { "on" } else { "off" }),
            |b| {
                b.iter(|| {
                    let mut runtime = KernelRuntime::host_runtime_default();
                    runtime.set_decision_tracing_enabled(tracing);
                    let init = runtime
                        .spawn_process("init", None, SchedulerClass::LatencyCritical)
                        .unwrap();
                    let parent = runtime
                        .spawn_process("parent", Some(init), SchedulerClass::Interactive)
                        .unwrap();
                    let scratch = runtime
                        .map_anonymous_memory(parent, 0x3000, true, true, false, "cow-bench")
                        .unwrap();
                    let child = runtime
                        .spawn_process_copy_vm(
                            "child",
                            Some(init),
                            SchedulerClass::Interactive,
                            parent,
                        )
                        .unwrap();
                    let _ = runtime.touch_memory(child, scratch, 0x1000, true).unwrap();
                    let _ = runtime
                        .touch_memory(child, scratch + 0x2000, 0x1000, true)
                        .unwrap();
                    let _ = runtime
                        .touch_memory(child, scratch + 0x1000, 0x1000, true)
                        .unwrap();
                });
            },
        );
    }
    group.finish();
}

fn benchmark_vm_fault_reclaim(c: &mut Criterion) {
    let mut group = c.benchmark_group("vm_fault_reclaim_cycle");
    group.throughput(Throughput::Elements(1));
    for tracing in [false, true] {
        group.bench_function(
            BenchmarkId::new("tracing", if tracing { "on" } else { "off" }),
            |b| {
                b.iter(|| {
                    let mut runtime = KernelRuntime::host_runtime_default();
                    runtime.set_decision_tracing_enabled(tracing);
                    let init = runtime
                        .spawn_process("init", None, SchedulerClass::LatencyCritical)
                        .unwrap();
                    let app = runtime
                        .spawn_process("app", Some(init), SchedulerClass::Interactive)
                        .unwrap();
                    let root = runtime
                        .grant_capability(
                            app,
                            ObjectHandle::new(Handle::new(91_000), 0),
                            CapabilityRights::READ | CapabilityRights::WRITE,
                            "root",
                        )
                        .unwrap();
                    let lib = runtime
                        .grant_capability(
                            app,
                            ObjectHandle::new(Handle::new(91_001), 0),
                            CapabilityRights::READ | CapabilityRights::WRITE,
                            "lib",
                        )
                        .unwrap();
                    runtime
                        .create_vfs_node("/", ObjectKind::Directory, root)
                        .unwrap();
                    runtime
                        .create_vfs_node("/lib", ObjectKind::Directory, root)
                        .unwrap();
                    runtime
                        .create_vfs_node("/lib/libfault.so", ObjectKind::File, lib)
                        .unwrap();
                    let mapped = runtime
                        .map_file_memory(
                            app,
                            "/lib/libfault.so",
                            0x4000,
                            0x2000,
                            true,
                            false,
                            true,
                            true,
                        )
                        .unwrap();
                    runtime
                        .protect_memory(app, mapped, 0x4000, true, true, false)
                        .unwrap();
                    let _ = runtime.touch_memory(app, mapped, 0x4000, true).unwrap();
                    runtime
                        .advise_memory(app, mapped + 0x1000, 0x2000, MemoryAdvice::DontNeed)
                        .unwrap();
                    runtime
                        .advise_memory(app, mapped, 0x4000, MemoryAdvice::WillNeed)
                        .unwrap();
                    let _ = runtime.touch_memory(app, mapped, 0x4000, false).unwrap();
                    runtime.sync_memory(app, mapped, 0x4000).unwrap();
                });
            },
        );
    }
    group.finish();
}

fn benches(c: &mut Criterion) {
    benchmark_resource_claim_handoff(c);
    benchmark_wait_wake(c);
    benchmark_descriptor_io(c);
    benchmark_scheduler(c);
    benchmark_vm_cow_fault(c);
    benchmark_vm_fault_reclaim(c);
}

criterion_group!(decision_tracing_benches, benches);
criterion_main!(decision_tracing_benches);
