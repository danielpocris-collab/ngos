use alloc::format;

use ngos_user_abi::{NativeEventRecord, NativeGraphicsEventKind};

pub fn describe_graphics_queue_event(record: &NativeEventRecord) -> alloc::string::String {
    match NativeGraphicsEventKind::from_raw(record.detail1) {
        Some(NativeGraphicsEventKind::Submitted) => format!(
            "graphics device={} request={} kind=submitted",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::Completed) => format!(
            "graphics device={} request={} kind=completed",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::Failed) => format!(
            "graphics device={} request={} kind=failed",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::Drained) => format!(
            "graphics device={} request={} kind=drained",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::Canceled) => format!(
            "graphics device={} request={} kind=canceled",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::Faulted) => format!(
            "graphics device={} token={} kind=faulted",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::Recovered) => format!(
            "graphics device={} token={} kind=recovered",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::Retired) => format!(
            "graphics device={} token={} kind=retired",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::LeaseReleased) => format!(
            "graphics device={} contract={} kind=lease-released",
            record.source_arg0, record.source_arg1
        ),
        Some(NativeGraphicsEventKind::LeaseAcquired) => format!(
            "graphics device={} contract={} kind=lease-acquired",
            record.source_arg0, record.source_arg1
        ),
        None => format!(
            "graphics device={} token={} kind=unknown",
            record.source_arg0, record.source_arg1
        ),
    }
}
