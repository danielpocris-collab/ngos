//! Low-level process infrastructure: ID enumeration, procfs read, process text.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use ngos_user_abi::{Errno, ExitCode, NativeSchedulerClass, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn list_process_ids<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<Vec<u64>, ExitCode> {
    let mut ids = vec![0u64; 16];
    loop {
        let count = runtime.list_processes(&mut ids).map_err(|_| 200)?;
        if count <= ids.len() {
            ids.truncate(count);
            return Ok(ids);
        }
        ids.resize(count, 0);
    }
}

pub fn read_procfs_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<u8>, ExitCode> {
    let mut buffer = vec![0u8; 1024];
    loop {
        let count = runtime.read_procfs(path, &mut buffer).map_err(|_| 201)?;
        if count < buffer.len() {
            buffer.truncate(count);
            return Ok(buffer);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

pub fn read_process_text<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    pid: u64,
    loader: F,
) -> Result<String, ExitCode>
where
    F: Fn(&Runtime<B>, u64, &mut [u8]) -> Result<usize, Errno>,
{
    let mut buffer = vec![0u8; 256];
    loop {
        let count = loader(runtime, pid, &mut buffer).map_err(|_| 202)?;
        if count < buffer.len() {
            buffer.truncate(count);
            return String::from_utf8(buffer).map_err(|_| 202);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

pub fn native_process_state_label(state: u32) -> &'static str {
    match state {
        0 => "Created",
        1 => "Ready",
        2 => "Running",
        3 => "Blocked",
        4 => "Exited",
        _ => "Unknown",
    }
}

pub fn scheduler_class_label(raw: u32) -> &'static str {
    match NativeSchedulerClass::from_raw(raw) {
        Some(NativeSchedulerClass::LatencyCritical) => "latency-critical",
        Some(NativeSchedulerClass::Interactive) => "interactive",
        Some(NativeSchedulerClass::BestEffort) => "best-effort",
        Some(NativeSchedulerClass::Background) => "background",
        None => "unknown",
    }
}

pub fn fixed_text_field(bytes: &[u8]) -> &str {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..end]).unwrap_or("")
}
