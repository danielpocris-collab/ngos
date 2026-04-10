//! Canonical subsystem role:
//! - subsystem: shell language parser and interpreter
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-lang`
//! - truth path role: shell language execution for the ngos native shell
//!
//! Canonical contract families exposed from this crate:
//! - shell language command contracts (let, print, if, match, fn, call, while, for, calc)
//! - shell function definition and call-stack contracts
//! - shell language block merging contracts
//!
//! This crate is responsible for interpreting the ngos shell language.
//! It does not dispatch non-language shell commands — those belong to userland-native.

#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::{
    ShellCallFrame, ShellFunction, ShellVariable, shell_lookup_variable, shell_set_variable,
};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 190)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MatchArm<'a> {
    pattern: &'a str,
    body: &'a str,
    is_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompareOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LangValue<'a> {
    Literal(&'a str),
    Variable(&'a str),
}

impl<'a> LangValue<'a> {
    fn resolve(&self, variables: &[ShellVariable]) -> String {
        match *self {
            Self::Literal(value) => value.to_string(),
            Self::Variable(name) => shell_lookup_variable(variables, name)
                .unwrap_or_default()
                .to_string(),
        }
    }
}

/// Attempt to handle a shell language command.
///
/// Returns `Some(result)` if the line was recognized as a language construct,
/// `None` if it is a regular shell command to be dispatched elsewhere.
pub fn try_handle_shell_lang_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    variables: &mut Vec<ShellVariable>,
    functions: &mut Vec<ShellFunction>,
    call_stack: &mut Vec<ShellCallFrame>,
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Option<Result<(), ExitCode>> {
    if line == "__lang_call_end" {
        return Some(handle_call_end(runtime, variables, call_stack));
    }
    if let Some(rest) = line.strip_prefix("__lang_loop_next ") {
        return Some(handle_loop_next(runtime, rest, pending_lines, line_index));
    }
    if line == "__lang_loop_end" {
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("__lang_var_set ") {
        return Some(handle_internal_var_set(runtime, rest, variables));
    }
    if let Some(rest) = line.strip_prefix("__lang_var_restore ") {
        return Some(handle_internal_var_restore(runtime, rest, variables));
    }
    if let Some(rest) = line.strip_prefix("let ") {
        return Some(handle_let(runtime, rest, variables));
    }
    if let Some(rest) = line.strip_prefix("print ") {
        return Some(handle_print(runtime, rest, variables));
    }
    if let Some(rest) = line.strip_prefix("if ") {
        return Some(handle_if(
            runtime,
            rest,
            variables,
            pending_lines,
            line_index,
        ));
    }
    if let Some(rest) = line.strip_prefix("match ") {
        return Some(handle_match(
            runtime,
            rest,
            variables,
            pending_lines,
            line_index,
        ));
    }
    if let Some(rest) = line.strip_prefix("fn ") {
        return Some(handle_fn(runtime, rest, functions));
    }
    if let Some(rest) = line.strip_prefix("call ") {
        return Some(handle_call(
            runtime,
            rest,
            variables,
            functions,
            call_stack,
            pending_lines,
            line_index,
        ));
    }
    if let Some(rest) = line.strip_prefix("call-set ") {
        return Some(handle_call_set(
            runtime,
            rest,
            variables,
            functions,
            call_stack,
            pending_lines,
            line_index,
        ));
    }
    if let Some(rest) = line.strip_prefix("while ") {
        return Some(handle_while(
            runtime,
            rest,
            variables,
            pending_lines,
            line_index,
        ));
    }
    if let Some(rest) = line.strip_prefix("for ") {
        return Some(handle_for(
            runtime,
            rest,
            variables,
            pending_lines,
            line_index,
        ));
    }
    if let Some(rest) = line.strip_prefix("calc ") {
        return Some(handle_calc(runtime, rest, variables));
    }
    if let Some(rest) = line.strip_prefix("return ") {
        return Some(handle_return(
            runtime,
            rest,
            variables,
            pending_lines,
            line_index,
        ));
    }
    if line == "return" {
        return Some(handle_return(
            runtime,
            "",
            variables,
            pending_lines,
            line_index,
        ));
    }
    if line == "functions" {
        return Some(handle_functions(runtime, functions));
    }
    if line == "break" {
        return Some(handle_break(runtime, pending_lines, line_index));
    }
    if line == "continue" {
        return Some(handle_continue(runtime, pending_lines, line_index));
    }
    None
}

/// Merge a multi-line shell language block starting at `start_index` in `pending_lines`
/// into a single line by tracking brace balance.
pub fn merge_multiline_lang_block(pending_lines: &mut Vec<String>, start_index: usize) {
    if start_index >= pending_lines.len() {
        return;
    }
    let first_line = pending_lines[start_index].trim_start();
    if !looks_like_lang_block_start(first_line) {
        return;
    }
    let mut balance = brace_balance(&pending_lines[start_index]);
    if balance <= 0 {
        return;
    }
    let mut merged = pending_lines[start_index].clone();
    let mut end_index = start_index + 1;
    while end_index < pending_lines.len() && balance > 0 {
        merged.push('\n');
        merged.push_str(pending_lines[end_index].trim_end());
        balance += brace_balance(&pending_lines[end_index]);
        end_index += 1;
    }
    if balance == 0 && end_index > start_index + 1 {
        pending_lines[start_index] = merged;
        pending_lines.drain(start_index + 1..end_index);
    }
}

fn handle_let<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &mut Vec<ShellVariable>,
) -> Result<(), ExitCode> {
    let (name, value) = parse_assignment(rest).ok_or_else(|| {
        let _ = write_line(runtime, "usage: let <name> = <value>");
        2
    })?;
    shell_set_variable(variables, name, value.to_string());
    write_line(runtime, &format!("let {name}={value}"))
}

fn handle_print<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &[ShellVariable],
) -> Result<(), ExitCode> {
    if rest.trim().is_empty() {
        let _ = write_line(runtime, "usage: print <expr>");
        return Err(2);
    }
    let value = parse_value(rest.trim()).resolve(variables);
    write_line(runtime, &value)
}

fn handle_if<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &[ShellVariable],
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let (left, op, right, body, else_body) = parse_if(rest).ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: if <left> ==|!= <right> { <command ; ...> } [else { <command ; ...> }]",
        );
        2
    })?;
    let left_value = left.resolve(variables);
    let right_value = right.resolve(variables);
    let passed = compare_values(&left_value, op, &right_value);
    if passed {
        let injected = split_lang_body(body);
        pending_lines.splice(line_index..line_index, injected);
    } else if let Some(body) = else_body {
        let injected = split_lang_body(body);
        pending_lines.splice(line_index..line_index, injected);
    }
    write_line(
        runtime,
        &format!(
            "if-result passed={} left={} right={}",
            if passed { "yes" } else { "no" },
            left_value,
            right_value
        ),
    )
}

fn handle_match<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &[ShellVariable],
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let (subject, arms) = parse_match(rest).ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: match <value> { case <pattern> { <command ; ...> } ... }",
        );
        2
    })?;
    let subject_value = subject.resolve(variables);
    let mut matched = String::from("none");
    let mut selected_body = None;
    for arm in &arms {
        let arm_matches = arm.is_default
            || arm.pattern == "_"
            || parse_value(arm.pattern).resolve(variables) == subject_value;
        if arm_matches {
            matched = if arm.is_default {
                String::from("default")
            } else {
                arm.pattern.to_string()
            };
            selected_body = Some(arm.body);
            break;
        }
    }
    if let Some(body) = selected_body {
        pending_lines.splice(line_index..line_index, split_lang_body(body));
    }
    write_line(
        runtime,
        &format!("match-result matched={} value={}", matched, subject_value),
    )
}

fn handle_fn<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    functions: &mut Vec<ShellFunction>,
) -> Result<(), ExitCode> {
    let (name, params, body) = parse_function(rest).ok_or_else(|| {
        let _ = write_line(runtime, "usage: fn <name>([arg, ...]) { <command ; ...> }");
        2
    })?;
    let body_lines = split_lang_body(body);
    if let Some(existing) = functions.iter_mut().find(|function| function.name == name) {
        existing.params = params.iter().map(|value| value.to_string()).collect();
        existing.body = body_lines;
    } else {
        functions.push(ShellFunction {
            name: name.to_string(),
            params: params.iter().map(|value| value.to_string()).collect(),
            body: body_lines,
        });
    }
    write_line(
        runtime,
        &format!("fn-defined name={name} params={}", params.len()),
    )
}

fn handle_call<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &mut Vec<ShellVariable>,
    functions: &[ShellFunction],
    call_stack: &mut Vec<ShellCallFrame>,
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let name = parts.next().unwrap_or("").trim();
    if name.is_empty() {
        let _ = write_line(runtime, "usage: call <name> [args...]");
        return Err(2);
    }
    let args = parts.collect::<Vec<_>>();
    let Some(function) = functions.iter().find(|function| function.name == name) else {
        write_line(runtime, &format!("call-missing name={name}"))?;
        return Err(250);
    };
    let saved_variables = bind_call_arguments(variables, function, &args);
    call_stack.push(ShellCallFrame {
        function_name: function.name.clone(),
        saved_variables,
        return_target: None,
    });
    let mut injected = function.body.clone();
    injected.push("__lang_call_end".to_string());
    pending_lines.splice(line_index..line_index, injected);
    write_line(
        runtime,
        &format!(
            "call-expanded name={} lines={} args={}",
            function.name,
            function.body.len(),
            args.len()
        ),
    )
}

fn handle_call_set<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &mut Vec<ShellVariable>,
    functions: &[ShellFunction],
    call_stack: &mut Vec<ShellCallFrame>,
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let target = parts.next().unwrap_or("").trim();
    let name = parts.next().unwrap_or("").trim();
    if target.is_empty() || name.is_empty() {
        let _ = write_line(runtime, "usage: call-set <target> <name> [args...]");
        return Err(2);
    }
    let args = parts.collect::<Vec<_>>();
    let Some(function) = functions.iter().find(|function| function.name == name) else {
        write_line(runtime, &format!("call-missing name={name}"))?;
        return Err(250);
    };
    let mut saved_variables = bind_call_arguments(variables, function, &args);
    save_variable(variables, &mut saved_variables, target);
    call_stack.push(ShellCallFrame {
        function_name: function.name.clone(),
        saved_variables,
        return_target: Some(target.to_string()),
    });
    let mut injected = function.body.clone();
    injected.push("__lang_call_end".to_string());
    pending_lines.splice(line_index..line_index, injected);
    write_line(
        runtime,
        &format!(
            "call-set-expanded target={} name={} lines={} args={}",
            target,
            function.name,
            function.body.len(),
            args.len()
        ),
    )
}

fn handle_return<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &mut Vec<ShellVariable>,
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let value = if rest.trim().is_empty() {
        String::new()
    } else {
        parse_value(rest.trim()).resolve(variables)
    };
    shell_set_variable(variables, "RETURN", value.clone());
    let mut scan = line_index;
    while scan < pending_lines.len() {
        if pending_lines[scan] == "__lang_call_end" {
            pending_lines.drain(line_index..scan);
            write_line(runtime, &format!("return value={value}"))?;
            return Ok(());
        }
        scan += 1;
    }
    write_line(runtime, "return-outside-call")?;
    Err(252)
}

fn handle_call_end<B: SyscallBackend>(
    runtime: &Runtime<B>,
    variables: &mut Vec<ShellVariable>,
    call_stack: &mut Vec<ShellCallFrame>,
) -> Result<(), ExitCode> {
    let Some(frame) = call_stack.pop() else {
        return write_line(runtime, "call-end-without-frame");
    };
    let return_value = shell_lookup_variable(variables, "RETURN")
        .map(ToString::to_string)
        .unwrap_or_default();
    restore_saved_variables(variables, &frame.saved_variables);
    if let Some(target) = &frame.return_target {
        shell_set_variable(variables, target, return_value.clone());
    }
    write_line(
        runtime,
        &format!(
            "call-finished name={}{}",
            frame.function_name,
            frame
                .return_target
                .as_ref()
                .map(|target| format!(" target={} value={}", target, return_value))
                .unwrap_or_default()
        ),
    )
}

fn handle_internal_var_set<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &mut Vec<ShellVariable>,
) -> Result<(), ExitCode> {
    let mut parts = rest.splitn(2, ' ');
    let name = parts.next().unwrap_or("").trim();
    let value = parts.next().unwrap_or("").trim();
    if name.is_empty() {
        return Err(253);
    }
    shell_set_variable(variables, name, value.to_string());
    let _ = runtime;
    Ok(())
}

fn handle_internal_var_restore<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &mut Vec<ShellVariable>,
) -> Result<(), ExitCode> {
    let mut parts = rest.splitn(2, ' ');
    let name = parts.next().unwrap_or("").trim();
    let value = parts.next().unwrap_or("").trim();
    if name.is_empty() {
        return Err(253);
    }
    if value == "__lang_none" {
        if let Some(index) = variables.iter().rposition(|variable| variable.name == name) {
            variables.remove(index);
        }
    } else {
        shell_set_variable(variables, name, value.to_string());
    }
    let _ = runtime;
    Ok(())
}

fn handle_loop_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    pending_lines.insert(line_index, rest.to_string());
    let _ = runtime;
    Ok(())
}

fn handle_while<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &[ShellVariable],
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let (left, op, right, body, _) = parse_if(rest).ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: while <left> ==|!=|>|<|>=|<= <right> { <command ; ...> }",
        );
        2
    })?;
    let left_value = left.resolve(variables);
    let right_value = right.resolve(variables);
    let passed = compare_values(&left_value, op, &right_value);
    if passed {
        let mut injected = split_lang_body(body);
        injected.push(format!("__lang_loop_next while {rest}"));
        injected.push("__lang_loop_end".to_string());
        pending_lines.splice(line_index..line_index, injected);
    }
    write_line(
        runtime,
        &format!(
            "while-result passed={} left={} right={}",
            if passed { "yes" } else { "no" },
            left_value,
            right_value
        ),
    )
}

fn handle_for<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &[ShellVariable],
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let (name, values, body) = parse_for(rest).ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: for <name> in <value ...> { <command ; ...> }",
        );
        2
    })?;
    let previous_value = shell_lookup_variable(variables, name).map(ToString::to_string);
    let mut injected = Vec::new();
    for value in &values {
        injected.push(format!("__lang_var_set {name} {value}"));
        injected.extend(split_lang_body(body));
        injected.push("__lang_loop_next for".to_string());
    }
    let restore_value = previous_value.unwrap_or_else(|| String::from("__lang_none"));
    injected.push(format!("__lang_var_restore {name} {restore_value}"));
    injected.push("__lang_loop_end".to_string());
    pending_lines.splice(line_index..line_index, injected);
    write_line(
        runtime,
        &format!("for-expanded var={name} items={}", values.len()),
    )
}

fn handle_break<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let Some((_, end_index)) = find_nearest_loop_bounds(pending_lines, line_index) else {
        write_line(runtime, "break-outside-loop")?;
        return Err(254);
    };
    pending_lines.drain(line_index..=end_index);
    write_line(runtime, "break-ok")
}

fn handle_continue<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pending_lines: &mut Vec<String>,
    line_index: usize,
) -> Result<(), ExitCode> {
    let Some((next_index, _)) = find_nearest_loop_bounds(pending_lines, line_index) else {
        write_line(runtime, "continue-outside-loop")?;
        return Err(255);
    };
    pending_lines.drain(line_index..next_index);
    write_line(runtime, "continue-ok")
}

fn handle_calc<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    variables: &mut Vec<ShellVariable>,
) -> Result<(), ExitCode> {
    let (name, left, op, right) = parse_calc(rest).ok_or_else(|| {
        let _ = write_line(runtime, "usage: calc <name> = <left> +|-|*|/ <right>");
        2
    })?;
    let left_value = left.resolve(variables);
    let right_value = right.resolve(variables);
    let left_number = left_value.parse::<i64>().map_err(|_| 251)?;
    let right_number = right_value.parse::<i64>().map_err(|_| 251)?;
    let result = match op {
        MathOp::Add => left_number.saturating_add(right_number),
        MathOp::Sub => left_number.saturating_sub(right_number),
        MathOp::Mul => left_number.saturating_mul(right_number),
        MathOp::Div => {
            if right_number == 0 {
                return Err(251);
            }
            left_number / right_number
        }
    };
    shell_set_variable(variables, name, result.to_string());
    write_line(runtime, &format!("calc {name}={result}"))
}

fn handle_functions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    functions: &[ShellFunction],
) -> Result<(), ExitCode> {
    if functions.is_empty() {
        return write_line(runtime, "functions=0");
    }
    for function in functions {
        write_line(
            runtime,
            &format!(
                "fn {} params={} lines={}",
                function.name,
                function.params.len(),
                function.body.len()
            ),
        )?;
    }
    Ok(())
}

fn parse_assignment(rest: &str) -> Option<(&str, &str)> {
    let eq = rest.find('=')?;
    let name = rest[..eq].trim();
    let value = rest[eq + 1..].trim();
    if name.is_empty() || value.is_empty() {
        return None;
    }
    Some((name, value))
}

fn parse_value(token: &str) -> LangValue<'_> {
    if let Some(name) = token.strip_prefix('$') {
        LangValue::Variable(name)
    } else {
        LangValue::Literal(token)
    }
}

fn parse_if(rest: &str) -> Option<(LangValue<'_>, CompareOp, LangValue<'_>, &str, Option<&str>)> {
    let open = rest.find('{')?;
    let close = find_matching_brace(rest, open)?;
    let cond = rest[..open].trim();
    let body = rest[open + 1..close].trim();
    let (left_raw, op, right_raw) = parse_compare_condition(cond)?;
    let left = parse_value(left_raw.trim());
    let right = parse_value(right_raw.trim());
    if body.is_empty() {
        return None;
    }
    let tail = rest[close + 1..].trim();
    let else_body = if tail.is_empty() {
        None
    } else {
        let else_rest = tail.strip_prefix("else")?.trim();
        let else_open = else_rest.find('{')?;
        let else_close = find_matching_brace(else_rest, else_open)?;
        let body = else_rest[else_open + 1..else_close].trim();
        if body.is_empty() {
            return None;
        }
        Some(body)
    };
    Some((left, op, right, body, else_body))
}

fn parse_function(rest: &str) -> Option<(&str, Vec<&str>, &str)> {
    let open = rest.find('{')?;
    let close = find_matching_brace(rest, open)?;
    let header = rest[..open].trim();
    let body = rest[open + 1..close].trim();
    if header.is_empty() || body.is_empty() {
        return None;
    }
    let (name, params) = if let Some(paren) = header.find('(') {
        let end = header.rfind(')')?;
        if end <= paren {
            return None;
        }
        let name = header[..paren].trim();
        let params = header[paren + 1..end]
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        (name, params)
    } else {
        (header, Vec::new())
    };
    if name.is_empty() {
        return None;
    }
    Some((name, params, body))
}

fn parse_match(rest: &str) -> Option<(LangValue<'_>, Vec<MatchArm<'_>>)> {
    let open = rest.find('{')?;
    let close = find_matching_brace(rest, open)?;
    let subject = rest[..open].trim();
    let block = rest[open + 1..close].trim();
    if subject.is_empty() || block.is_empty() {
        return None;
    }
    let arms = parse_match_arms(block)?;
    Some((parse_value(subject), arms))
}

fn parse_for(rest: &str) -> Option<(&str, Vec<&str>, &str)> {
    let open = rest.find('{')?;
    let close = find_matching_brace(rest, open)?;
    let header = rest[..open].trim();
    let body = rest[open + 1..close].trim();
    if body.is_empty() {
        return None;
    }
    let in_index = header.find(" in ")?;
    let name = header[..in_index].trim();
    let values = header[in_index + 4..]
        .split_whitespace()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if name.is_empty() || values.is_empty() {
        return None;
    }
    Some((name, values, body))
}

fn parse_match_arms(block: &str) -> Option<Vec<MatchArm<'_>>> {
    let mut arms = Vec::new();
    let mut index = 0usize;
    while index < block.len() {
        let raw_remaining = &block[index..];
        let trimmed_len = raw_remaining.len().saturating_sub(
            raw_remaining
                .trim_start_matches(|ch: char| ch.is_whitespace() || ch == ';')
                .len(),
        );
        index += trimmed_len;
        let remaining = &block[index..];
        if remaining.is_empty() {
            break;
        }
        if let Some(rest) = remaining.strip_prefix("case ") {
            let rest_offset = block.len() - rest.len();
            let open_rel = rest.find('{')?;
            let open = rest_offset + open_rel;
            let close = find_matching_brace(block, open)?;
            let pattern = block[rest_offset..open].trim();
            let body = block[open + 1..close].trim();
            if pattern.is_empty() || body.is_empty() {
                return None;
            }
            arms.push(MatchArm {
                pattern,
                body,
                is_default: false,
            });
            index = close + 1;
            continue;
        }
        if let Some(rest) = remaining.strip_prefix("else") {
            let rest = rest.trim_start();
            if !rest.starts_with('{') {
                return None;
            }
            let open = block.len() - rest.len();
            let close = find_matching_brace(block, open)?;
            let body = block[open + 1..close].trim();
            if body.is_empty() {
                return None;
            }
            arms.push(MatchArm {
                pattern: "_",
                body,
                is_default: true,
            });
            index = close + 1;
            continue;
        }
        return None;
    }
    (!arms.is_empty()).then_some(arms)
}

fn parse_compare_condition(cond: &str) -> Option<(&str, CompareOp, &str)> {
    for (pattern, op) in [
        ("==", CompareOp::Eq),
        ("!=", CompareOp::Ne),
        (">=", CompareOp::Ge),
        ("<=", CompareOp::Le),
        (">", CompareOp::Gt),
        ("<", CompareOp::Lt),
    ] {
        if let Some(index) = cond.find(pattern) {
            return Some((&cond[..index], op, &cond[index + pattern.len()..]));
        }
    }
    None
}

fn parse_calc(rest: &str) -> Option<(&str, LangValue<'_>, MathOp, LangValue<'_>)> {
    let eq = rest.find('=')?;
    let name = rest[..eq].trim();
    let expr = rest[eq + 1..].trim();
    if name.is_empty() || expr.is_empty() {
        return None;
    }
    for (pattern, op) in [
        (" + ", MathOp::Add),
        (" - ", MathOp::Sub),
        (" * ", MathOp::Mul),
        (" / ", MathOp::Div),
    ] {
        if let Some(index) = expr.find(pattern) {
            let left = parse_value(expr[..index].trim());
            let right = parse_value(expr[index + pattern.len()..].trim());
            return Some((name, left, op, right));
        }
    }
    None
}

fn compare_values(left: &str, op: CompareOp, right: &str) -> bool {
    if let (Ok(left_number), Ok(right_number)) = (left.parse::<i64>(), right.parse::<i64>()) {
        return match op {
            CompareOp::Eq => left_number == right_number,
            CompareOp::Ne => left_number != right_number,
            CompareOp::Gt => left_number > right_number,
            CompareOp::Lt => left_number < right_number,
            CompareOp::Ge => left_number >= right_number,
            CompareOp::Le => left_number <= right_number,
        };
    }
    match op {
        CompareOp::Eq => left == right,
        CompareOp::Ne => left != right,
        CompareOp::Gt => left > right,
        CompareOp::Lt => left < right,
        CompareOp::Ge => left >= right,
        CompareOp::Le => left <= right,
    }
}

fn bind_call_arguments(
    variables: &mut Vec<ShellVariable>,
    function: &ShellFunction,
    args: &[&str],
) -> Vec<(String, Option<String>)> {
    let mut saved = Vec::new();
    save_variable(variables, &mut saved, "ARGC");
    shell_set_variable(variables, "ARGC", args.len().to_string());
    for (index, arg) in args.iter().enumerate() {
        let name = format!("ARG{}", index + 1);
        save_variable(variables, &mut saved, &name);
        shell_set_variable(variables, &name, (*arg).to_string());
    }
    for (index, param) in function.params.iter().enumerate() {
        save_variable(variables, &mut saved, param);
        let value = args.get(index).copied().unwrap_or_default();
        shell_set_variable(variables, param, value.to_string());
    }
    saved
}

fn save_variable(
    variables: &[ShellVariable],
    saved: &mut Vec<(String, Option<String>)>,
    name: &str,
) {
    if saved.iter().any(|(saved_name, _)| saved_name == name) {
        return;
    }
    let previous = shell_lookup_variable(variables, name).map(ToString::to_string);
    saved.push((name.to_string(), previous));
}

fn restore_saved_variables(variables: &mut Vec<ShellVariable>, saved: &[(String, Option<String>)]) {
    for (name, previous) in saved.iter().rev() {
        match previous {
            Some(value) => shell_set_variable(variables, name, value.clone()),
            None => {
                if let Some(index) = variables
                    .iter()
                    .rposition(|variable| variable.name == *name)
                {
                    variables.remove(index);
                }
            }
        }
    }
}

fn split_lang_body(body: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut brace_depth = 0i32;
    for ch in body.chars() {
        match ch {
            '{' => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' => {
                brace_depth -= 1;
                current.push(ch);
            }
            ';' | '\n' if brace_depth == 0 => {
                let line = current.trim();
                if !line.is_empty() {
                    lines.push(line.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let line = current.trim();
    if !line.is_empty() {
        lines.push(line.to_string());
    }
    lines
}

fn looks_like_lang_block_start(line: &str) -> bool {
    line.starts_with("if ")
        || line.starts_with("match ")
        || line.starts_with("while ")
        || line.starts_with("fn ")
        || line.starts_with("for ")
}

fn brace_balance(line: &str) -> i32 {
    let mut balance = 0i32;
    for ch in line.chars() {
        match ch {
            '{' => balance += 1,
            '}' => balance -= 1,
            _ => {}
        }
    }
    balance
}

fn find_matching_brace(text: &str, open_index: usize) -> Option<usize> {
    let mut balance = 0i32;
    for (index, ch) in text.char_indices().skip(open_index) {
        match ch {
            '{' => balance += 1,
            '}' => {
                balance -= 1;
                if balance == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_nearest_loop_bounds(
    pending_lines: &[String],
    start_index: usize,
) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut next_index = None;
    for (index, line) in pending_lines.iter().enumerate().skip(start_index) {
        if line.starts_with("__lang_loop_next ") {
            if depth == 0 && next_index.is_none() {
                next_index = Some(index);
            }
        } else if line == "__lang_loop_end" {
            if depth == 0 {
                return next_index.map(|next| (next, index));
            }
            depth = depth.saturating_sub(1);
        } else if line.starts_with("while ") || line.starts_with("for ") {
            depth += 1;
        }
    }
    None
}
