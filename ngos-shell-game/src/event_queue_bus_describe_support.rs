use alloc::format;

use ngos_user_abi::NativeEventRecord;

pub fn describe_bus_queue_event(record: &NativeEventRecord) -> alloc::string::String {
    let kind = match record.detail0 {
        0 => "attached",
        1 => "detached",
        2 => "published",
        3 => "received",
        _ => "unknown",
    };
    format!(
        "bus peer={} endpoint={} kind={}",
        record.source_arg0, record.source_arg1, kind
    )
}
