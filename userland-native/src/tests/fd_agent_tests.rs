use super::*;

#[test]
fn fd_agent_seek_command_shares_offsets_across_dup_descriptions() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime.mkfile_path("/seek.txt").unwrap();
    let fd = runtime.open_path("/seek.txt").unwrap();
    runtime.write(fd, b"abcdef").unwrap();
    assert_eq!(runtime.seek(fd, 0, SeekWhence::Set).unwrap(), 0);
    let dup_fd = runtime.dup(fd).unwrap();

    match fd_agents::try_handle_fd_agent_command(&runtime, "seek-fd 8 set 2") {
        Some(Ok(())) => {}
        other => panic!("unexpected seek agent result: {other:?}"),
    }
    let first_fdinfo =
        String::from_utf8(read_procfs_all(&runtime, "/proc/1/fdinfo/8").unwrap()).unwrap();
    assert!(first_fdinfo.contains("pos:\t2"));

    match fd_agents::try_handle_fd_agent_command(&runtime, "seek-fd 7 end -1") {
        Some(Ok(())) => {}
        other => panic!("unexpected seek agent result: {other:?}"),
    }
    let second_fdinfo =
        String::from_utf8(read_procfs_all(&runtime, "/proc/1/fdinfo/8").unwrap()).unwrap();
    assert!(second_fdinfo.contains("pos:\t5"));

    let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
    assert!(stdout.contains("seek-fd fd=8 whence=set offset=2 pos=2"));
    assert!(stdout.contains("seek-fd fd=7 whence=end offset=-1 pos=5"));
    assert_eq!(dup_fd, 8);
}

#[test]
fn recording_backend_seek_shares_offsets_across_dup_descriptions() {
    let runtime = UserRuntime::new(RecordingBackend::default());
    runtime.mkfile_path("/seek-direct.txt").unwrap();
    let fd = runtime.open_path("/seek-direct.txt").unwrap();
    runtime.write(fd, b"abcdef").unwrap();
    assert_eq!(runtime.seek(fd, 0, SeekWhence::Set).unwrap(), 0);
    let dup_fd = runtime.dup(fd).unwrap();
    assert_eq!(runtime.seek(dup_fd, 2, SeekWhence::Set).unwrap(), 2);
    assert_eq!(runtime.seek(fd, 1, SeekWhence::Cur).unwrap(), 3);
    let fdinfo = read_procfs_all(&runtime, "/proc/1/fdinfo/8").unwrap();
    let text = String::from_utf8(fdinfo).unwrap();
    assert!(text.contains("pos:\t3"));
}
