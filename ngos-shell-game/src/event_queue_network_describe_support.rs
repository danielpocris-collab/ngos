use alloc::format;
use alloc::string::ToString;

use ngos_user_abi::{NativeEventRecord, NativeNetworkEventKind};

pub fn describe_network_queue_event(record: &NativeEventRecord) -> alloc::string::String {
    let kind = match NativeNetworkEventKind::from_raw(record.detail1) {
        Some(NativeNetworkEventKind::LinkChanged) => "link-changed",
        Some(NativeNetworkEventKind::RxReady) => "rx-ready",
        Some(NativeNetworkEventKind::TxDrained) => "tx-drained",
        None => "unknown",
    };
    format!(
        "network iface={} socket={} kind={}",
        record.source_arg0,
        if record.detail0 != 0 {
            record.source_arg1.to_string()
        } else {
            "-".to_string()
        },
        kind
    )
}
