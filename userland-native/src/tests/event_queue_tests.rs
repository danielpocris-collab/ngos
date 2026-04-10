use super::*;

#[test]
fn shell_wait_event_queue_renders_bus_events() {
    let runtime = UserRuntime::new(RecordingBackend::with_stdin(b""));
    runtime.backend().push_event_queue_record(
        77,
        NativeEventRecord {
            token: 940,
            events: 0x1,
            source_kind: NativeEventSourceKind::Bus as u32,
            source_arg0: 11,
            source_arg1: 22,
            source_arg2: 0,
            detail0: 2,
            detail1: 0,
        },
    );

    shell_wait_event_queue(&runtime, 77).unwrap();

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("queue-event queue=77"));
    assert!(stdout.contains("token=940"));
    assert!(stdout.contains("source=bus peer=11 endpoint=22 kind=published"));
}
