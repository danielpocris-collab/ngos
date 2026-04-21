//! Shell variable management operations.
//!
//! Pure operations on ShellVariable slices: lookup, set, infer semantic type.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::render::shell_render_record_value;
use crate::types::{ShellRecordField, ShellSemanticValue, ShellVariable};

/// Infer the semantic value type from a rendered string.
pub fn infer_shell_semantic_value(rendered: &str) -> Option<ShellSemanticValue> {
    if matches!(rendered, "true" | "false") {
        Some(ShellSemanticValue::Bool)
    } else if rendered.parse::<i64>().is_ok() {
        Some(ShellSemanticValue::Int)
    } else if rendered.is_empty() {
        None
    } else {
        Some(ShellSemanticValue::String)
    }
}

/// Return the type name string for a variable's semantic value.
pub fn shell_variable_type_name(variable: &ShellVariable) -> &'static str {
    match &variable.semantic {
        Some(ShellSemanticValue::String) => "string",
        Some(ShellSemanticValue::Bool) => "bool",
        Some(ShellSemanticValue::Int) => "int",
        Some(ShellSemanticValue::List(_)) => "list",
        Some(ShellSemanticValue::Record(_)) => "record",
        None => "unknown",
    }
}

/// Look up the most recently set variable with the given name.
pub fn shell_lookup_variable<'a>(variables: &'a [ShellVariable], name: &str) -> Option<&'a str> {
    variables
        .iter()
        .rev()
        .find(|variable| variable.name == name)
        .map(|variable| variable.value.as_str())
}

/// Look up the most recently set variable entry with the given name.
pub fn shell_lookup_variable_entry<'a>(
    variables: &'a [ShellVariable],
    name: &str,
) -> Option<&'a ShellVariable> {
    variables
        .iter()
        .rev()
        .find(|variable| variable.name == name)
}

/// Set or update a variable in the variable list, inferring semantic type.
pub fn shell_set_variable(variables: &mut Vec<ShellVariable>, name: &str, value: String) {
    let semantic = infer_shell_semantic_value(&value);
    if let Some(variable) = variables.iter_mut().find(|variable| variable.name == name) {
        variable.value = value;
        variable.semantic = semantic;
    } else {
        variables.push(ShellVariable {
            name: name.to_string(),
            value,
            semantic,
        });
    }
}

/// Set a record-typed variable from its fields.
pub fn shell_set_record_variable(
    variables: &mut Vec<ShellVariable>,
    name: &str,
    fields: Vec<ShellRecordField>,
) {
    let rendered = shell_render_record_value(&fields);
    if let Some(variable) = variables.iter_mut().find(|variable| variable.name == name) {
        variable.value = rendered;
        variable.semantic = Some(ShellSemanticValue::Record(fields));
    } else {
        variables.push(ShellVariable {
            name: name.to_string(),
            value: rendered,
            semantic: Some(ShellSemanticValue::Record(fields)),
        });
    }
}

/// Clone a source variable under a new name.
pub fn shell_clone_variable_as(
    variables: &mut Vec<ShellVariable>,
    name: &str,
    source: &ShellVariable,
) {
    if let Some(variable) = variables.iter_mut().find(|variable| variable.name == name) {
        variable.value = source.value.clone();
        variable.semantic = source.semantic.clone();
    } else {
        variables.push(ShellVariable {
            name: name.to_string(),
            value: source.value.clone(),
            semantic: source.semantic.clone(),
        });
    }
}
