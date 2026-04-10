use alloc::format;

use ngos_user_abi::{ExitCode, NativeEventRecord, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::event_queue_describe_support::describe_queue_event;
use crate::write_line;

pub fn shell_wait_event_queue<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
) -> Result<(), ExitCode> {
    let mut records = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 16];
    let count = runtime
        .wait_event_queue(queue_fd, &mut records)
        .map_err(|_| 246)?;
    for record in &records[..count] {
        write_line(
            runtime,
            &format!(
                "queue-event queue={} token={} events=0x{:x} source={}",
                queue_fd,
                record.token,
                record.events,
                describe_queue_event(record)
            ),
        )?;
    }
    Ok(())
}
