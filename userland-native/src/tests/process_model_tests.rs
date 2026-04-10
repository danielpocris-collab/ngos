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
        "NGOS_BOOT_CPU_XSAVE=1",
        "NGOS_BOOT_CPU_SAVE_AREA=4096",
        "NGOS_BOOT_CPU_XCR0=0xe7",
        "NGOS_BOOT_CPU_BOOT_SEED=0x12345678",
        "NGOS_BOOT_CPU_HW_PROVIDER=1",
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
fn native_shell_closes_process_model_vertical() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"process-info 1\nkill 1 9\npending-signals 1\nblocked-signals 1\nspawn-path worker /bin/worker\nprocess-info 77\njob-info 77\nfg 77\njobs\nexit 0\n",
    ));
    let bootstrap = session_bootstrap();

    let result = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();

    assert_eq!(result, 0, "stdout:\n{stdout}");
    assert!(
        stdout.contains("pid=1 name=ngos-userland-native"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("state=Running exit=0 fds=3 caps=2 env=1 regions=2 threads=1"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("thread=1"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("signal-sent pid=1 signal=9"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("pid=1 pending-signals=9"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("pid=1 blocked-pending-signals=-"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("process-spawned pid=77 name=worker path=/bin/worker"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("pid=77 name=worker image=/bin/worker cwd=/ parent=0"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("thread=77") && stdout.contains("threads=1"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(
            "job-info pid=77 name=worker path=/bin/worker state=live:Running signals=0 exit=137 pending=0"
        ),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("foreground-complete pid=77 exit=137"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("job pid=77 name=worker path=/bin/worker state=reaped:137 signals=0"),
        "stdout:\n{stdout}"
    );
}
