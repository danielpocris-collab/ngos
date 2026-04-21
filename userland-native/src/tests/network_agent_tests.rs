use super::*;

const NETWORK_TEST_ARGV: [&str; 1] = ["ngos-userland-native"];
const NETWORK_TEST_ENVP: [&str; 13] = [
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
];
const NETWORK_TEST_AUXV: [ngos_user_abi::AuxvEntry; 2] = [
    ngos_user_abi::AuxvEntry {
        key: AT_PAGESZ,
        value: 4096,
    },
    ngos_user_abi::AuxvEntry {
        key: AT_ENTRY,
        value: 0x401000,
    },
];

fn session_bootstrap() -> BootstrapArgs<'static> {
    BootstrapArgs::new(&NETWORK_TEST_ARGV, &NETWORK_TEST_ENVP, &NETWORK_TEST_AUXV)
}

#[test]
fn native_shell_resolves_relative_network_interface_paths_for_admin_and_link_commands() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"net-admin dev/net0 1400 6 5 3 up promisc\nnet-link dev/net0 down\nnetif /dev/net0\nexit 0\n",
    ));
    let bootstrap = session_bootstrap();

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains(
        "net-admin path=/dev/net0 mtu=1400 tx-cap=6 rx-cap=5 inflight-limit=3 admin=up promisc=on"
    ));
    assert!(stdout.contains("netif-link path=/dev/net0 state=down"));
    assert!(stdout.contains(
        "netif path=/dev/net0 admin=up link=down promisc=on mtu=1400 tx-cap=6 rx-cap=5 inflight-limit=3"
    ));
}

#[test]
fn native_shell_resolves_relative_network_watch_paths_and_unwatches_cleanly() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(
        b"mkdir-path /run\nmksock-path /run/net0.sock\nqueue-create epoll\nnet-watch $LAST_QUEUE_FD dev/net0 700 run/net0.sock\nnet-link dev/net0 down\nqueue-wait $LAST_QUEUE_FD\nnet-unwatch $LAST_QUEUE_FD dev/net0 700 run/net0.sock\nnonblock-fd $LAST_QUEUE_FD on\nqueue-wait $LAST_QUEUE_FD\nlast-status\nexit 0\n",
    ));
    let bootstrap = session_bootstrap();

    let code = main(&runtime, &bootstrap);
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("queue-created fd="));
    assert!(stdout.contains("net-watch queue="));
    assert!(stdout.contains("device=/dev/net0"));
    assert!(stdout.contains("socket=/run/net0.sock"));
    assert!(stdout.contains("token=700"));
    assert!(stdout.contains("queue-event queue="));
    assert!(stdout.contains("source=network iface=99"));
    assert!(stdout.contains("kind=link-changed"));
    assert!(stdout.contains("net-unwatch queue="));
    assert!(stdout.contains("nonblock-fd fd="));
    assert!(runtime.backend().network_event_watches.borrow().is_empty());
    assert!(
        runtime
            .backend()
            .event_queue_pending
            .borrow()
            .iter()
            .all(|(_, records)| records.is_empty())
    );
}
