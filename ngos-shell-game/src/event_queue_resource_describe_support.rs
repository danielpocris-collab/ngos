use alloc::format;

use ngos_user_abi::NativeEventRecord;

pub fn describe_resource_queue_event(record: &NativeEventRecord) -> alloc::string::String {
    let kind = match record.detail0 {
        0 => "claimed",
        1 => "queued",
        2 => "canceled",
        3 => "released",
        4 => "handed-off",
        5 => "revoked",
        _ => "unknown",
    };
    format!(
        "resource id={} contract={} kind={}",
        record.source_arg0, record.source_arg1, kind
    )
}
