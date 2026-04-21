//! Shared IO helpers: line output, multi-line emit, buffered write, file read.

use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, NativeObjectKind, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 190)?;
    Ok(())
}

pub fn shell_emit_lines<B: SyscallBackend>(
    runtime: &Runtime<B>,
    text: &str,
) -> Result<(), ExitCode> {
    if text.is_empty() {
        return Ok(());
    }
    for line in text.lines() {
        write_line(runtime, line)?;
    }
    if text.as_bytes().last().is_some_and(|byte| *byte != b'\n') {
        write_line(runtime, "")?;
    }
    Ok(())
}

pub fn shell_write_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
    bytes: &[u8],
) -> Result<(), ExitCode> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        let written = runtime.write(fd, &bytes[offset..]).map_err(|_| 240)?;
        if written == 0 {
            return Err(240);
        }
        offset += written;
    }
    Ok(())
}

pub fn shell_read_file_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<String, ExitCode> {
    let bytes = shell_read_file_bytes(runtime, path)?;
    String::from_utf8(bytes).map_err(|_| 239)
}

pub fn shell_read_file_bytes<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<u8>, ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 256];
    loop {
        let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    runtime.close(fd).map_err(|_| 240)?;
    Ok(bytes)
}

pub fn object_kind_name(raw: u32) -> &'static str {
    match NativeObjectKind::from_raw(raw) {
        Some(NativeObjectKind::File) => "file",
        Some(NativeObjectKind::Directory) => "directory",
        Some(NativeObjectKind::Symlink) => "symlink",
        Some(NativeObjectKind::Socket) => "socket",
        Some(NativeObjectKind::Device) => "device",
        Some(NativeObjectKind::Driver) => "driver",
        Some(NativeObjectKind::Process) => "process",
        Some(NativeObjectKind::Memory) => "memory",
        Some(NativeObjectKind::Channel) => "channel",
        Some(NativeObjectKind::EventQueue) => "event-queue",
        Some(NativeObjectKind::SleepQueue) => "sleep-queue",
        None => "unknown",
    }
}
