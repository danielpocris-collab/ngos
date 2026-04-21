//! Shell language dispatcher — thin re-export bridge.
//!
//! This module re-exports the canonical shell language parser and interpreter
//! from `ngos-shell-lang` and shared types from `ngos-shell-types`.
//!
//! All implementation lives in those dedicated crates.
//! This file is a compatibility shim only.

pub(super) use ngos_shell_lang::{merge_multiline_lang_block, try_handle_shell_lang_command};
