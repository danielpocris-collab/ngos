use super::*;

#[test]
fn native_shell_nextmind_observe_reports_verified_core_state() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let argv = ["ngos-userland-native"];
    let envp = [
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
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    let session = parse_session_context(&bootstrap).unwrap();
    assert_eq!(
        run_session_shell_script(
            &runtime,
            &session,
            "mode semantic\nnextmind.observe\nexit 0\n"
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("nextmind.metrics label=current state="));
    assert!(stdout.contains("nextmind.semantic channel=proc::"));
    assert!(stdout.contains("verified-core=true"));
    assert!(stdout.contains("violations=0"));
}

#[test]
fn native_shell_nextmind_observe_reports_bus_entities() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let argv = ["ngos-userland-native"];
    let envp = [
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
    ];
    let auxv = [
        ngos_user_abi::AuxvEntry {
            key: AT_PAGESZ,
            value: 4096,
        },
        ngos_user_abi::AuxvEntry {
            key: AT_ENTRY,
            value: 0x401000,
        },
    ];
    let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
    let session = parse_session_context(&bootstrap).unwrap();
    runtime.mkdir_path("/ipc").unwrap();
    runtime.mkchan_path("/ipc/render").unwrap();
    assert_eq!(
        run_session_shell_script(
            &runtime,
            &session,
            "mkdomain render\nset BUS_DOMAIN $LAST_DOMAIN_ID\nmkresource $BUS_DOMAIN channel render-bus\nset BUS_RESOURCE $LAST_RESOURCE_ID\nmkbuspeer $BUS_DOMAIN renderer\nmkbusendpoint $BUS_DOMAIN $BUS_RESOURCE /ipc/render\nmode semantic\nnextmind.observe\nexit 0\n",
        ),
        0
    );
    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("nextmind.entity kind=bus-peer subject=bus-peer:"));
    assert!(stdout.contains("nextmind.entity kind=bus-endpoint subject=bus-endpoint:"));
    assert!(stdout.contains("class=process"));
    assert!(stdout.contains("caps=observe,signal"));
}

#[test]
fn nextmind_auto_triggered_emits_once_for_initial_watch_events_then_quiets() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let controller = SystemController::new(&runtime);
    let auto_state = NextMindAutoState {
        enabled: true,
        streams: vec![
            controller
                .subscribe(EventFilter::Network {
                    interface_path: String::from("/dev/net0"),
                    socket_path: None,
                    token: CapabilityToken { value: 1 },
                    link_changed: true,
                    rx_ready: true,
                    tx_drained: true,
                    poll_events: POLLPRI,
                })
                .unwrap(),
        ],
    };
    assert!(nextmind_auto_triggered(&runtime, &auto_state).unwrap());
    assert!(!nextmind_auto_triggered(&runtime, &auto_state).unwrap());
}

#[test]
fn nextmind_auto_subscribes_bus_endpoints_and_triggers_on_bus_events() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let domain = runtime.create_domain(None, "render").unwrap();
    let resource = runtime
        .create_resource(domain, NativeResourceKind::Channel, "render-bus")
        .unwrap();
    runtime.mkdir_path("/ipc").unwrap();
    runtime.mkchan_path("/ipc/render").unwrap();
    let peer = runtime.create_bus_peer(domain, "renderer").unwrap();
    let endpoint = runtime
        .create_bus_endpoint(domain, resource, "/ipc/render")
        .unwrap();

    let streams = nextmind_subscribe_auto_streams(&runtime).unwrap();
    assert!(streams.iter().any(|stream| {
        matches!(
            stream.filter,
            EventFilter::Bus {
                endpoint: watched,
                attached: true,
                detached: true,
                published: true,
                received: true,
                ..
            } if watched == endpoint
        )
    }));

    let auto_state = NextMindAutoState {
        enabled: true,
        streams,
    };
    assert!(nextmind_auto_triggered(&runtime, &auto_state).unwrap());
    assert!(!nextmind_auto_triggered(&runtime, &auto_state).unwrap());

    runtime.attach_bus_peer(peer, endpoint).unwrap();
    assert!(nextmind_auto_triggered(&runtime, &auto_state).unwrap());
    assert!(!nextmind_auto_triggered(&runtime, &auto_state).unwrap());
}

#[test]
fn recording_backend_supports_nextmind_controller_flow() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let controller = SystemController::new(&runtime);
    let mut adaptive = AdaptiveState::new();

    let semantic = controller
        .observe_semantic_state(None, &mut adaptive)
        .unwrap();
    assert!(semantic.metrics.verified_core_ok);

    let mut pids = vec![0u64; 64];
    let process_count = runtime.list_processes(&mut pids).unwrap();
    assert!(process_count >= 2);
    let devices = controller.enumerate_devices().unwrap();
    assert!(!devices.is_empty());
    let resources = controller.query_resources().unwrap();
    assert!(!resources.is_empty());
    let mut contracts = vec![0u64; 64];
    let contract_count = runtime.list_contracts(&mut contracts).unwrap();
    assert!(contract_count >= 1);

    let facts = controller.collect_facts().unwrap();
    assert!(!facts.is_empty());

    let entities = controller.collect_semantic_entities().unwrap();
    assert!(!entities.is_empty());

    let topology = controller
        .observe_topology(Some(&semantic.metrics.snapshot))
        .unwrap();
    assert!(topology.online_cpus >= 1);
    assert_eq!(topology.entries.len(), topology.online_cpus);
}
