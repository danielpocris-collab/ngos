use super::*;

#[test]
fn capability_model_closes_identity_rights_refusal_recovery_and_observability() {
    let mut surface = KernelSyscallSurface::host_runtime_default();
    let bootstrap = surface
        .runtime
        .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
        .unwrap();
    let delegate = surface
        .runtime
        .spawn_process("delegate", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let recovery = surface
        .runtime
        .spawn_process("recovery", Some(bootstrap), SchedulerClass::Interactive)
        .unwrap();
    let context = SyscallContext::kernel(bootstrap);
    let target = ObjectHandle::new(Handle::new(0xCAFE), 3);

    let root = match surface
        .dispatch(
            context.clone(),
            Syscall::GrantCapability(GrantCapability {
                owner: bootstrap,
                target,
                rights: CapabilityRights::READ
                    | CapabilityRights::WRITE
                    | CapabilityRights::DUPLICATE,
                label: String::from("cap-root"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityGranted(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let delegated = match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateCapability(DuplicateCapability {
                capability: root,
                new_owner: delegate,
                rights: CapabilityRights::READ,
                label: String::from("cap-read-only"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityDuplicated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let caps = match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/caps", delegate.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => String::from_utf8(bytes).unwrap(),
        other => panic!("unexpected syscall result: {other:?}"),
    };
    assert!(
        caps.contains(&format!(
            "{}\t{}\t0x1\tcap-read-only",
            delegated.raw(),
            target.id().raw()
        )),
        "{caps}"
    );

    assert_eq!(
        surface.dispatch(
            SyscallContext::kernel(delegate),
            Syscall::DuplicateCapability(DuplicateCapability {
                capability: delegated,
                new_owner: recovery,
                rights: CapabilityRights::READ,
                label: String::from("cap-illegal"),
            }),
        ),
        Err(SyscallError::Runtime(RuntimeError::Capability(
            CapabilityError::RightDenied {
                required: CapabilityRights::DUPLICATE,
                actual: CapabilityRights::READ,
            }
        )))
    );

    let recovered = match surface
        .dispatch(
            context.clone(),
            Syscall::DuplicateCapability(DuplicateCapability {
                capability: root,
                new_owner: recovery,
                rights: CapabilityRights::READ,
                label: String::from("cap-recovered"),
            }),
        )
        .unwrap()
    {
        SyscallResult::CapabilityDuplicated(id) => id,
        other => panic!("unexpected syscall result: {other:?}"),
    };

    let recovery_caps = match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/caps", recovery.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => String::from_utf8(bytes).unwrap(),
        other => panic!("unexpected syscall result: {other:?}"),
    };
    assert!(
        recovery_caps.contains(&format!(
            "{}\t{}\t0x1\tcap-recovered",
            recovered.raw(),
            target.id().raw()
        )),
        "{recovery_caps}"
    );

    surface.runtime.revoke_capability(recovered).unwrap();

    let final_caps = match surface
        .dispatch(
            context.clone(),
            Syscall::ReadProcFs {
                path: format!("/proc/{}/caps", recovery.raw()),
            },
        )
        .unwrap()
    {
        SyscallResult::ProcFsBytes(bytes) => String::from_utf8(bytes).unwrap(),
        other => panic!("unexpected syscall result: {other:?}"),
    };
    assert!(final_caps.trim().is_empty(), "{final_caps}");

    let snapshot = match surface.dispatch(context, Syscall::Snapshot).unwrap() {
        SyscallResult::Snapshot(snapshot) => snapshot,
        other => panic!("unexpected syscall result: {other:?}"),
    };
    let report = surface.runtime.verify_core();
    assert!(report.capability_model_verified, "{report:#?}");
    assert!(report.is_verified(), "{report:#?}");
    assert!(snapshot.capability_model_verified);
    assert_eq!(snapshot.verified_core_ok, report.is_verified());
}
