use super::*;

fn session_bootstrap() -> BootstrapArgs<'static> {
    let argv = Box::leak(Box::new(["ngos-userland-native"]));
    let envp = Box::leak(Box::new([
        "NGOS_SESSION=1",
        "NGOS_SESSION_PROTOCOL=kernel-launch",
        "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
        "NGOS_PROCESS_NAME=ngos-userland-native",
        "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
        "NGOS_CWD=/",
        "NGOS_ROOT_MOUNT_PATH=/",
        "NGOS_ROOT_MOUNT_NAME=rootfs",
        "NGOS_IMAGE_BASE=0x400000",
        "NGOS_STACK_TOP=0x7fffffff0000",
        "NGOS_PHDR=0x40",
        "NGOS_PHENT=56",
        "NGOS_PHNUM=2",
    ]));
    let auxv = Box::leak(Box::new([
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ]));
    BootstrapArgs::new(argv, envp, auxv)
}

#[test]
fn native_shell_closes_domain_resource_contract_model_vertical() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdomain closure\nset DOMAIN_ID $LAST_DOMAIN_ID\nmkresource $DOMAIN_ID device closure-gpu\nset RESOURCE_ID $LAST_RESOURCE_ID\nmkcontract $DOMAIN_ID $RESOURCE_ID display closure-primary\nset PRIMARY_ID $LAST_CONTRACT_ID\nmkcontract $DOMAIN_ID $RESOURCE_ID display closure-mirror\nset MIRROR_ID $LAST_CONTRACT_ID\ndomains\ndomain $DOMAIN_ID\nresources\nresource $RESOURCE_ID\ncontracts\ncontract $PRIMARY_ID\nclaim $PRIMARY_ID\nclaim $MIRROR_ID\nwaiters $RESOURCE_ID\nreleaseclaim $PRIMARY_ID\nresource $RESOURCE_ID\ncontract-state $PRIMARY_ID suspended\ninvoke $PRIMARY_ID\nlast-status\ncontract-state $PRIMARY_ID active\ninvoke $PRIMARY_ID\nrelease $MIRROR_ID\nresource $RESOURCE_ID\nexit 0\n",
    ));
    let bootstrap = session_bootstrap();

    let result = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();

    assert_eq!(result, 0, "stdout:\n{stdout}");
    assert!(stdout.contains("domain-created id="), "stdout:\n{stdout}");
    assert!(stdout.contains("resource-created id="), "stdout:\n{stdout}");
    assert!(stdout.contains("contract-created id="), "stdout:\n{stdout}");
    assert!(
        stdout.contains("domain id=41 owner=1 resources=1 contracts=3 name=graphics"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(
            "resource id=42 domain=41 creator=1 kind=device state=active arbitration=fifo governance=queueing"
        ),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(
            "contract id=43 domain=41 resource=42 issuer=1 kind=display state=active label=scanout"
        ),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("claim-queued contract=44 resource=42 holder=43 position=1"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("resource=42 waiters=44"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("claim-handed-off resource=42 to=44 acquire_count=2 handoff_count=1"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(
            "resource id=42 domain=41 creator=1 kind=device state=active arbitration=fifo governance=queueing"
        ) && stdout.contains("holder=44 acquire_count=2 handoff_count=1 waiters=- name=gpu0"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("contract-state-updated id=43 state=suspended"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("invoke-refused contract=43 errno=EACCES code=13"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("last-status=244"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("contract-state-updated id=43 state=active"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("invoked contract=43 count=1"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("resource-released contract=44 resource=42"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("resource-released contract=44 resource=42")
            && stdout.contains("holder=0 acquire_count=2 handoff_count=1 waiters=- name=gpu0"),
        "stdout:\n{stdout}"
    );
}
