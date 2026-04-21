//! Shell language function and call-frame types.
//!
//! These types support the shell language interpreter's function definition
//! and call stack mechanics.

use alloc::string::String;
use alloc::vec::Vec;

/// A user-defined shell function with parameter names and a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<String>,
}

/// A saved call frame used to restore variable state on function return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCallFrame {
    pub function_name: String,
    pub saved_variables: Vec<(String, Option<String>)>,
    pub return_target: Option<String>,
}
