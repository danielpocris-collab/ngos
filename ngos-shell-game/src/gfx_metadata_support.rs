use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub fn summarize_graphics_deep_ops(payload: &str) -> String {
    let mut ops = Vec::<String>::new();
    for line in payload.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("op=") {
            let op = rest.split_whitespace().next().unwrap_or(rest);
            if !op.is_empty() {
                ops.push(op.to_string());
            }
        }
    }
    if ops.is_empty() {
        String::from("-")
    } else {
        ops.join(",")
    }
}

pub fn gpu_request_kind_name(kind: u32) -> &'static str {
    match kind {
        0 => "read",
        1 => "write",
        2 => "control",
        _ => "unknown",
    }
}

pub fn gpu_request_state_name(state: u32) -> &'static str {
    match state {
        0 => "queued",
        1 => "inflight",
        2 => "completed",
        3 => "failed",
        4 => "canceled",
        _ => "unknown",
    }
}

pub fn parse_gfx_payload_translation_metadata(payload: &str) -> (&str, &str) {
    let mut source_api = "-";
    let mut translation = "-";
    for line in payload.lines() {
        if let Some(value) = line.strip_prefix("source-api=") {
            if !value.is_empty() {
                source_api = value;
            }
        } else if let Some(value) = line.strip_prefix("translation=") {
            if !value.is_empty() {
                translation = value;
            }
        }
    }
    (source_api, translation)
}
