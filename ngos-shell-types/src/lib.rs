//! Canonical subsystem role:
//! - subsystem: shell state types
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-types`
//! - truth path role: shared type foundation for all ngos shell agent crates
//!
//! Canonical contract families exposed from this crate:
//! - shell state type contracts
//! - shell variable management contracts
//! - shell path normalization contracts
//! - shell value typing and rendering contracts
//!
//! This crate defines only pure data types and type-only operations.
//! It has no dependency on Runtime or SyscallBackend.
//! All shell agent crates depend on this crate for shared types.

#![no_std]

extern crate alloc;

mod expand;
mod lang_types;
mod parse;
mod path;
mod render;
mod types;
mod variable;

pub use expand::{
    shell_expand_aliases, shell_expand_variables, shell_parse_guarded_commands,
    shell_sync_runtime_variables,
};
pub use lang_types::{ShellCallFrame, ShellFunction};
pub use parse::{parse_i64_arg, parse_u16_arg, parse_u64_arg, parse_usize_arg};
pub use path::{normalize_shell_path, resolve_shell_path};
pub use render::{shell_render_list_value, shell_render_record_value};
pub use types::{
    ShellAlias, ShellCommandGuard, ShellJob, ShellMode, ShellRecordField, ShellSemanticValue,
    ShellVariable,
};
pub use variable::{
    infer_shell_semantic_value, shell_clone_variable_as, shell_lookup_variable,
    shell_lookup_variable_entry, shell_set_record_variable, shell_set_variable,
    shell_variable_type_name,
};
