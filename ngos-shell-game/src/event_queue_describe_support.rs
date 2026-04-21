use alloc::format;
use alloc::string::ToString;

use ngos_user_abi::{NativeEventRecord, NativeEventSourceKind};

pub fn describe_queue_event(record: &NativeEventRecord) -> alloc::string::String {
    match NativeEventSourceKind::from_raw(record.source_kind) {
        Some(NativeEventSourceKind::Resource) => {
            crate::event_queue_resource_describe_support::describe_resource_queue_event(record)
        }
        Some(NativeEventSourceKind::Network) => {
            crate::event_queue_network_describe_support::describe_network_queue_event(record)
        }
        Some(NativeEventSourceKind::Graphics) => {
            crate::event_queue_graphics_describe_support::describe_graphics_queue_event(record)
        }
        Some(NativeEventSourceKind::Bus) => {
            crate::event_queue_bus_describe_support::describe_bus_queue_event(record)
        }
        Some(kind) => format!("other:{kind:?}"),
        None => "unknown".to_string(),
    }
}
