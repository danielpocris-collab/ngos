//! Core shell state types.
//!
//! Canonical subsystem role:
//! - subsystem: shell state types
//! - owner layer: Layer 3

use alloc::string::String;
use alloc::vec::Vec;

/// A background job tracked by the shell.
#[derive(Debug, Clone)]
pub struct ShellJob {
    pub pid: u64,
    pub name: String,
    pub path: String,
    pub reaped_exit: Option<i32>,
    pub signal_count: u64,
}

/// A shell alias binding a name to a replacement command string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellAlias {
    pub name: String,
    pub value: String,
}

/// A shell variable with name, rendered value, and optional semantic type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellVariable {
    pub name: String,
    pub value: String,
    pub semantic: Option<ShellSemanticValue>,
}

/// The semantic type of a shell variable's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellSemanticValue {
    String,
    Bool,
    Int,
    List(Vec<String>),
    Record(Vec<ShellRecordField>),
}

/// A single key=value field in a shell record value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRecordField {
    pub key: String,
    pub value: String,
}

/// The execution mode for shell command dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellMode {
    Direct,
    Semantic,
}

/// Guard condition controlling whether a command in a pipeline runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommandGuard {
    Always,
    OnSuccess,
    OnFailure,
}
