use super::*;

#[test]
fn native_shell_can_watch_and_unwatch_bus_events_through_queue_interface() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let mut variables = Vec::new();
    let mut last_status = 0;

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        "bus-watch 7 22 900 all",
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected bus-watch result: {other:?}"),
    }
    assert_eq!(last_status, 0);

    shell_wait_event_queue(&runtime, 7).unwrap();

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        "bus-unwatch 7 22 900",
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected bus-unwatch result: {other:?}"),
    }
    assert_eq!(last_status, 0);

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("bus-watch queue=7 endpoint=22 token=900 kinds=all"));
    assert!(stdout.contains("queue-event queue=7"));
    assert!(stdout.contains("source=bus peer=11 endpoint=22 kind=attached"));
    assert!(stdout.contains("bus-unwatch queue=7 endpoint=22 token=900"));
}

#[test]
fn native_shell_bus_agents_create_and_list_entities_directly() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let mut variables = Vec::new();
    let mut last_status = 0;

    runtime.mkdir_path("/ipc").unwrap();
    runtime.mkchan_path("/ipc/render").unwrap();

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        "mkbuspeer 41 renderer",
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected mkbuspeer result: {other:?}"),
    }
    assert_eq!(last_status, 0);

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        "mkbusendpoint 41 42 /ipc/render",
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected mkbusendpoint result: {other:?}"),
    }
    assert_eq!(last_status, 0);

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        "bus-peers",
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected bus-peers result: {other:?}"),
    }
    assert_eq!(last_status, 0);

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        "bus-endpoints",
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected bus-endpoints result: {other:?}"),
    }
    assert_eq!(last_status, 0);

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("bus-peer-created id="));
    assert!(stdout.contains("bus-endpoint-created id="));
    assert!(stdout.contains("path=/ipc/render"));
    assert!(stdout.contains("bus-peers count=1"));
    assert!(stdout.contains("bus-endpoints count=1"));
    assert!(stdout.contains("bus-peer id="));
    assert!(stdout.contains("bus-endpoint id="));
}

#[test]
fn native_shell_bus_resolves_relative_endpoint_paths_from_cwd() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let mut variables = Vec::new();
    let mut last_status = 0;

    runtime.mkdir_path("/ipc").unwrap();
    runtime.mkchan_path("/ipc/render").unwrap();

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/ipc",
        "mkbusendpoint 41 42 render",
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected mkbusendpoint result: {other:?}"),
    }
    assert_eq!(last_status, 0);

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("bus-endpoint-created id="));
    assert!(stdout.contains("path=/ipc/render"));
}

#[test]
fn native_shell_bus_controls_full_lifecycle_and_traffic() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let mut variables = Vec::new();
    let mut last_status = 0;

    let domain = runtime.create_domain(None, "bus").unwrap();
    let resource = runtime
        .create_resource(domain, NativeResourceKind::Channel, "render-bus")
        .unwrap();
    runtime.mkdir_path("/ipc").unwrap();
    runtime.mkchan_path("/ipc/render").unwrap();

    for command in [
        format!("mkbuspeer {domain} renderer"),
        format!("mkbusendpoint {domain} {resource} /ipc/render"),
    ] {
        match ngos_shell_bus::try_handle_bus_agent_command(
            &runtime,
            "/",
            &command,
            &mut variables,
            &mut last_status,
        ) {
            Some(Ok(())) => {}
            other => panic!("unexpected bus command result for `{command}`: {other:?}"),
        }
        assert_eq!(last_status, 0);
    }

    let peer = variables
        .iter()
        .find(|variable| variable.name == "LAST_BUS_PEER_ID")
        .map(|variable| variable.value.clone())
        .unwrap();
    let endpoint = variables
        .iter()
        .find(|variable| variable.name == "LAST_BUS_ENDPOINT_ID")
        .map(|variable| variable.value.clone())
        .unwrap();
    for command in [
        format!("bus-attach {peer} {endpoint}"),
        format!("bus-send {peer} {endpoint} hello-bus"),
        format!("bus-recv {peer} {endpoint}"),
        format!("bus-peer {peer}"),
        format!("bus-endpoint {endpoint}"),
        String::from("bus-peers"),
        String::from("bus-endpoints"),
    ] {
        match ngos_shell_bus::try_handle_bus_agent_command(
            &runtime,
            "/",
            &command,
            &mut variables,
            &mut last_status,
        ) {
            Some(Ok(())) => {}
            other => panic!("unexpected bus command result for `{command}`: {other:?}"),
        }
        assert_eq!(last_status, 0);
    }

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("bus-peer-created id="));
    assert!(stdout.contains("bus-endpoint-created id="));
    assert!(stdout.contains("bus-attached peer="));
    assert!(stdout.contains("bus-published peer="));
    assert!(stdout.contains("payload=hello-bus"));
    assert!(stdout.contains("bus-received peer="));
    assert!(stdout.contains("bus-peer-detail id="));
    assert!(stdout.contains("bus-endpoint-detail id="));
    assert!(stdout.contains("bus-peers count=1"));
    assert!(stdout.contains("bus-endpoints count=1"));
    assert!(stdout.contains("payload=hello-bus"));
}

#[test]
fn native_shell_rejects_invalid_bus_watch_kind_list() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let mut variables = Vec::new();
    let mut last_status = 0;

    let result = ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        "bus-watch 7 22 900 invalid-kind",
        &mut variables,
        &mut last_status,
    );
    assert_eq!(result, Some(Err(2)));

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains(
        "usage: bus-watch <queue-fd> <endpoint> <token> [all|attached,detached,published,received]"
    ));
}

#[test]
fn native_shell_bus_attachment_rights_enforce_refusal_and_recovery() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    let mut variables = Vec::new();
    let mut last_status = 0;

    let domain = runtime.create_domain(None, "bus").unwrap();
    let resource = runtime
        .create_resource(domain, NativeResourceKind::Channel, "render-bus")
        .unwrap();
    runtime.mkdir_path("/ipc").unwrap();
    runtime.mkchan_path("/ipc/render").unwrap();

    for command in [
        format!("mkbuspeer {domain} renderer"),
        format!("mkbusendpoint {domain} {resource} /ipc/render"),
    ] {
        match ngos_shell_bus::try_handle_bus_agent_command(
            &runtime,
            "/",
            &command,
            &mut variables,
            &mut last_status,
        ) {
            Some(Ok(())) => {}
            other => panic!("unexpected bus command result for `{command}`: {other:?}"),
        }
        assert_eq!(last_status, 0);
    }

    let peer = variables
        .iter()
        .find(|variable| variable.name == "LAST_BUS_PEER_ID")
        .map(|variable| variable.value.clone())
        .unwrap();
    let endpoint = variables
        .iter()
        .find(|variable| variable.name == "LAST_BUS_ENDPOINT_ID")
        .map(|variable| variable.value.clone())
        .unwrap();

    for command in [
        format!("bus-attach-rights {peer} {endpoint} write"),
        format!("bus-send {peer} {endpoint} rights-only-write"),
        format!("bus-endpoint {endpoint}"),
        format!("bus-peer {peer}"),
    ] {
        match ngos_shell_bus::try_handle_bus_agent_command(
            &runtime,
            "/",
            &command,
            &mut variables,
            &mut last_status,
        ) {
            Some(Ok(())) => {}
            other => panic!("unexpected bus command result for `{command}`: {other:?}"),
        }
    }
    assert_eq!(last_status, 0);

    match ngos_shell_bus::try_handle_bus_agent_command(
        &runtime,
        "/",
        &format!("bus-recv {peer} {endpoint}"),
        &mut variables,
        &mut last_status,
    ) {
        Some(Ok(())) => {}
        other => panic!("unexpected bus receive refusal result: {other:?}"),
    }
    assert_eq!(last_status, 246);

    for command in [
        format!("bus-attach-rights {peer} {endpoint} readwrite"),
        format!("bus-recv {peer} {endpoint}"),
        format!("bus-endpoint {endpoint}"),
        format!("bus-peer {peer}"),
    ] {
        match ngos_shell_bus::try_handle_bus_agent_command(
            &runtime,
            "/",
            &command,
            &mut variables,
            &mut last_status,
        ) {
            Some(Ok(())) => {}
            other => panic!("unexpected bus recovery command result for `{command}`: {other:?}"),
        }
    }
    assert_eq!(last_status, 0);

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("bus-attached peer="));
    assert!(stdout.contains("rights=write"));
    assert!(stdout.contains("bus-endpoint-detail id="));
    assert!(stdout.contains("readers=0 writers=1"));
    assert!(stdout.contains("bus-peer-detail id="));
    assert!(stdout.contains("readable=0 writable=1"));
    assert!(stdout.contains("rights=readwrite"));
    assert!(stdout.contains("payload=rights-only-write"));
    assert!(stdout.contains("readers=1 writers=1"));
    assert!(stdout.contains("readable=1 writable=1"));
}
