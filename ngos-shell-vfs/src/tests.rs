extern crate std;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

use ngos_user_abi::{
    Errno, NativeFileStatusRecord, NativeObjectKind, SYS_CLOSE, SYS_LSTAT_PATH, SYS_MKFILE_PATH,
    SYS_OPEN_PATH, SYS_READ, SYS_READLINK_PATH, SYS_STAT_PATH, SYS_SYMLINK_PATH, SYS_TRUNCATE_PATH,
    SYS_UNLINK_PATH, SYS_WRITE, SYS_WRITEV, SyscallBackend, SyscallFrame, SyscallReturn,
};
use ngos_user_runtime::Runtime;

use crate::{
    shell_read_file_text, try_handle_fd_agent_command, try_handle_path_agent_command,
    try_handle_vfs_agent_command,
};

#[derive(Clone)]
enum TestNode {
    File(Vec<u8>),
    Symlink(String),
}

#[derive(Clone, Copy)]
struct TestFd {
    path: &'static str,
    offset: usize,
}

struct TestState {
    nodes: BTreeMap<&'static str, TestNode>,
    fds: BTreeMap<usize, TestFd>,
    next_fd: usize,
    stdout: Vec<u8>,
}

struct TestBackend {
    state: RefCell<TestState>,
}

impl TestBackend {
    fn new() -> Self {
        Self {
            state: RefCell::new(TestState {
                nodes: BTreeMap::new(),
                fds: BTreeMap::new(),
                next_fd: 3,
                stdout: Vec::new(),
            }),
        }
    }

    fn insert_file(&self, path: &'static str, bytes: &[u8]) {
        self.state
            .borrow_mut()
            .nodes
            .insert(path, TestNode::File(bytes.to_vec()));
    }

    fn read_file(&self, path: &str) -> Vec<u8> {
        let state = self.state.borrow();
        match state.nodes.get(path) {
            Some(TestNode::File(bytes)) => bytes.clone(),
            Some(TestNode::Symlink(_)) => panic!("expected file at {path}"),
            None => panic!("missing file at {path}"),
        }
    }

    fn read_symlink(&self, path: &str) -> String {
        let state = self.state.borrow();
        match state.nodes.get(path) {
            Some(TestNode::Symlink(target)) => target.clone(),
            Some(TestNode::File(_)) => panic!("expected symlink at {path}"),
            None => panic!("missing symlink at {path}"),
        }
    }

    fn stdout_text(&self) -> String {
        String::from_utf8(self.state.borrow().stdout.clone()).expect("stdout should be utf8")
    }

    fn open_fd_count(&self) -> usize {
        self.state.borrow().fds.len()
    }
}

impl SyscallBackend for TestBackend {
    unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
        match frame.number {
            SYS_WRITEV => self.handle_writev(frame),
            SYS_WRITE => self.handle_write(frame),
            SYS_READ => self.handle_read(frame),
            SYS_CLOSE => self.handle_close(frame),
            SYS_OPEN_PATH => self.handle_open(frame),
            SYS_STAT_PATH | SYS_LSTAT_PATH => self.handle_stat(frame),
            SYS_MKFILE_PATH => self.handle_mkfile(frame),
            SYS_UNLINK_PATH => self.handle_unlink(frame),
            SYS_TRUNCATE_PATH => self.handle_truncate(frame),
            SYS_SYMLINK_PATH => self.handle_symlink(frame),
            SYS_READLINK_PATH => self.handle_readlink(frame),
            _ => SyscallReturn::err(Errno::NotSup),
        }
    }
}

impl TestBackend {
    unsafe fn read_str(ptr: usize, len: usize) -> &'static str {
        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        std::str::from_utf8(bytes).expect("valid utf8")
    }

    fn handle_writev(&self, frame: SyscallFrame) -> SyscallReturn {
        if frame.arg0 != 1 {
            return SyscallReturn::err(Errno::Badf);
        }
        let iovecs = unsafe {
            std::slice::from_raw_parts(frame.arg1 as *const ngos_user_abi::UserIoVec, frame.arg2)
        };
        let mut state = self.state.borrow_mut();
        let mut total = 0usize;
        for iovec in iovecs {
            let bytes = unsafe { std::slice::from_raw_parts(iovec.base as *const u8, iovec.len) };
            state.stdout.extend_from_slice(bytes);
            total += bytes.len();
        }
        SyscallReturn::ok(total)
    }

    fn handle_write(&self, frame: SyscallFrame) -> SyscallReturn {
        let bytes = unsafe { std::slice::from_raw_parts(frame.arg1 as *const u8, frame.arg2) };
        let mut state = self.state.borrow_mut();
        let Some(fd) = state.fds.get(&frame.arg0).copied() else {
            return SyscallReturn::err(Errno::Badf);
        };
        let Some(TestNode::File(contents)) = state.nodes.get_mut(fd.path) else {
            return SyscallReturn::err(Errno::Badf);
        };
        let end = fd.offset + bytes.len();
        if end > contents.len() {
            contents.resize(end, 0);
        }
        contents[fd.offset..end].copy_from_slice(bytes);
        state.fds.insert(
            frame.arg0,
            TestFd {
                path: fd.path,
                offset: end,
            },
        );
        SyscallReturn::ok(bytes.len())
    }

    fn handle_read(&self, frame: SyscallFrame) -> SyscallReturn {
        let mut state = self.state.borrow_mut();
        let Some(fd) = state.fds.get(&frame.arg0).copied() else {
            return SyscallReturn::err(Errno::Badf);
        };
        let Some(TestNode::File(contents)) = state.nodes.get(fd.path) else {
            return SyscallReturn::err(Errno::Badf);
        };
        let remaining = &contents[fd.offset.min(contents.len())..];
        let count = remaining.len().min(frame.arg2);
        let buffer = unsafe { std::slice::from_raw_parts_mut(frame.arg1 as *mut u8, frame.arg2) };
        buffer[..count].copy_from_slice(&remaining[..count]);
        state.fds.insert(
            frame.arg0,
            TestFd {
                path: fd.path,
                offset: fd.offset + count,
            },
        );
        SyscallReturn::ok(count)
    }

    fn handle_close(&self, frame: SyscallFrame) -> SyscallReturn {
        let removed = self.state.borrow_mut().fds.remove(&frame.arg0);
        if removed.is_some() {
            SyscallReturn::ok(0)
        } else {
            SyscallReturn::err(Errno::Badf)
        }
    }

    fn handle_open(&self, frame: SyscallFrame) -> SyscallReturn {
        let path = unsafe { Self::read_str(frame.arg0, frame.arg1) };
        let mut state = self.state.borrow_mut();
        if !matches!(state.nodes.get(path), Some(TestNode::File(_))) {
            return SyscallReturn::err(Errno::NoEnt);
        }
        let fd = state.next_fd;
        state.next_fd += 1;
        state.fds.insert(fd, TestFd { path, offset: 0 });
        SyscallReturn::ok(fd)
    }

    fn handle_stat(&self, frame: SyscallFrame) -> SyscallReturn {
        let path = unsafe { Self::read_str(frame.arg0, frame.arg1) };
        let Some(node) = self.state.borrow().nodes.get(path).cloned() else {
            return SyscallReturn::err(Errno::NoEnt);
        };
        let (kind, size) = match node {
            TestNode::File(bytes) => (NativeObjectKind::File as u32, bytes.len() as u64),
            TestNode::Symlink(target) => (NativeObjectKind::Symlink as u32, target.len() as u64),
        };
        let record = unsafe { &mut *(frame.arg2 as *mut NativeFileStatusRecord) };
        *record = NativeFileStatusRecord {
            inode: 1,
            link_count: 1,
            size,
            kind,
            cloexec: 0,
            nonblock: 0,
            readable: 1,
            writable: 1,
            executable: 0,
            owner_uid: 0,
            group_gid: 0,
            mode: 0o644,
        };
        SyscallReturn::ok(0)
    }

    fn handle_mkfile(&self, frame: SyscallFrame) -> SyscallReturn {
        let path = unsafe { Self::read_str(frame.arg0, frame.arg1) };
        self.state
            .borrow_mut()
            .nodes
            .insert(path, TestNode::File(Vec::new()));
        SyscallReturn::ok(0)
    }

    fn handle_unlink(&self, frame: SyscallFrame) -> SyscallReturn {
        let path = unsafe { Self::read_str(frame.arg0, frame.arg1) };
        let removed = self.state.borrow_mut().nodes.remove(path);
        if removed.is_some() {
            SyscallReturn::ok(0)
        } else {
            SyscallReturn::err(Errno::NoEnt)
        }
    }

    fn handle_truncate(&self, frame: SyscallFrame) -> SyscallReturn {
        let path = unsafe { Self::read_str(frame.arg0, frame.arg1) };
        let mut state = self.state.borrow_mut();
        let Some(TestNode::File(bytes)) = state.nodes.get_mut(path) else {
            return SyscallReturn::err(Errno::NoEnt);
        };
        bytes.resize(frame.arg2, 0);
        SyscallReturn::ok(0)
    }

    fn handle_symlink(&self, frame: SyscallFrame) -> SyscallReturn {
        let path = unsafe { Self::read_str(frame.arg0, frame.arg1) };
        let target = unsafe { Self::read_str(frame.arg2, frame.arg3) };
        self.state
            .borrow_mut()
            .nodes
            .insert(path, TestNode::Symlink(target.into()));
        SyscallReturn::ok(0)
    }

    fn handle_readlink(&self, frame: SyscallFrame) -> SyscallReturn {
        let path = unsafe { Self::read_str(frame.arg0, frame.arg1) };
        let state = self.state.borrow();
        let Some(TestNode::Symlink(target)) = state.nodes.get(path) else {
            return SyscallReturn::err(Errno::NoEnt);
        };
        let bytes = target.as_bytes();
        let count = bytes.len().min(frame.arg3);
        let buffer = unsafe { std::slice::from_raw_parts_mut(frame.arg2 as *mut u8, frame.arg3) };
        buffer[..count].copy_from_slice(&bytes[..count]);
        SyscallReturn::ok(count)
    }
}

#[test]
fn copy_file_recreates_destination_before_writing() {
    let backend = TestBackend::new();
    backend.insert_file("/src.txt", b"abc");
    backend.insert_file("/dst.txt", b"abcdefgh");
    let runtime = Runtime::new(backend);

    let handled = try_handle_vfs_agent_command(&runtime, "/", "copy-file /src.txt /dst.txt")
        .expect("vfs command should match");
    assert_eq!(handled, Ok(()));

    assert_eq!(runtime.backend().read_file("/dst.txt"), b"abc");
    assert_eq!(
        shell_read_file_text(&runtime, "/dst.txt").expect("copy destination should stay readable"),
        "abc"
    );
}

#[test]
fn symlink_path_preserves_relative_target_text() {
    let backend = TestBackend::new();
    let runtime = Runtime::new(backend);

    let handled = try_handle_vfs_agent_command(
        &runtime,
        "/workspace/app",
        "symlink-path links/config ../shared/config",
    )
    .expect("vfs command should match");
    assert_eq!(handled, Ok(()));

    assert_eq!(
        runtime
            .backend()
            .read_symlink("/workspace/app/links/config"),
        "../shared/config"
    );
}

#[test]
fn copy_file_rejects_extra_arguments_and_emits_usage() {
    let backend = TestBackend::new();
    let runtime = Runtime::new(backend);

    let handled =
        try_handle_vfs_agent_command(&runtime, "/", "copy-file /src.txt /dst.txt extra-token")
            .expect("vfs command should match");
    assert_eq!(handled, Err(2));
    assert_eq!(
        runtime.backend().stdout_text(),
        "usage: copy-file <source> <destination>\n"
    );
}

#[test]
fn seek_fd_rejects_extra_arguments_and_emits_usage() {
    let backend = TestBackend::new();
    let runtime = Runtime::new(backend);

    let handled =
        try_handle_fd_agent_command(&runtime, "seek-fd 4 set 0 trailing").expect("fd command");
    assert_eq!(handled, Err(2));
    assert_eq!(
        runtime.backend().stdout_text(),
        "usage: seek-fd <fd> <set|cur|end> <offset>\n"
    );
}

#[test]
fn tree_path_rejects_extra_arguments_and_emits_usage() {
    let backend = TestBackend::new();
    let runtime = Runtime::new(backend);

    let handled = try_handle_path_agent_command(&runtime, "/", "tree-path /tmp 2 trailing")
        .expect("path command");
    assert_eq!(handled, Err(2));
    assert_eq!(
        runtime.backend().stdout_text(),
        "usage: tree-path <path> [depth]\n"
    );
}

#[test]
fn cat_file_closes_descriptor_when_utf8_decode_fails() {
    let backend = TestBackend::new();
    backend.insert_file("/broken.bin", &[0xff, 0xfe]);
    let runtime = Runtime::new(backend);

    let handled = try_handle_vfs_agent_command(&runtime, "/", "cat-file /broken.bin")
        .expect("vfs command should match");
    assert_eq!(handled, Err(239));
    assert_eq!(runtime.backend().open_fd_count(), 0);
}

#[test]
fn copy_file_closes_source_descriptor_when_destination_open_fails() {
    let backend = TestBackend::new();
    backend.insert_file("/src.txt", b"abc");
    {
        let mut state = backend.state.borrow_mut();
        state
            .nodes
            .insert("/dst.txt", TestNode::Symlink("../somewhere".into()));
    }
    let runtime = Runtime::new(backend);

    let handled = try_handle_vfs_agent_command(&runtime, "/", "copy-file /src.txt /dst.txt")
        .expect("vfs command should match");
    assert_eq!(handled, Err(237));
    assert_eq!(runtime.backend().open_fd_count(), 0);
}

#[test]
fn assert_file_contains_success_is_routed_through_vfs_command() {
    let backend = TestBackend::new();
    backend.insert_file("/note.txt", b"hello shell world");
    let runtime = Runtime::new(backend);

    let handled =
        try_handle_vfs_agent_command(&runtime, "/", "assert-file-contains /note.txt shell")
            .expect("vfs command should match");
    assert_eq!(handled, Ok(()));
    assert_eq!(
        runtime.backend().stdout_text(),
        "assert-file-contains-ok path=/note.txt needle=shell\n"
    );
}

#[test]
fn assert_file_contains_failure_is_observable() {
    let backend = TestBackend::new();
    backend.insert_file("/note.txt", b"hello shell world");
    let runtime = Runtime::new(backend);

    let handled =
        try_handle_vfs_agent_command(&runtime, "/", "assert-file-contains /note.txt missing")
            .expect("vfs command should match");
    assert_eq!(handled, Err(248));
    assert_eq!(
        runtime.backend().stdout_text(),
        "assert-file-contains-failed path=/note.txt needle=missing\n"
    );
}

#[test]
fn assert_file_contains_usage_is_reported_for_missing_text() {
    let backend = TestBackend::new();
    let runtime = Runtime::new(backend);

    let handled = try_handle_vfs_agent_command(&runtime, "/", "assert-file-contains /note.txt")
        .expect("vfs command should match");
    assert_eq!(handled, Err(2));
    assert_eq!(
        runtime.backend().stdout_text(),
        "usage: assert-file-contains <path> <text>\n"
    );
}
