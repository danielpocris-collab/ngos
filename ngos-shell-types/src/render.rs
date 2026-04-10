//! Shell value rendering for lists and records.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::types::ShellRecordField;

/// Render a list value as `[item1, item2, ...]`.
pub fn shell_render_list_value(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

/// Render a record value as `{key=value, key=value, ...}`.
pub fn shell_render_record_value(fields: &[ShellRecordField]) -> String {
    let parts = fields
        .iter()
        .map(|field| format!("{}={}", field.key, field.value))
        .collect::<Vec<_>>();
    format!("{{{}}}", parts.join(", "))
}
