//! Canonical subsystem role:
//! - subsystem: native development diagnostics surface
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: operator-facing build, diff, and diagnostic inspection
//!   over canonical project artifacts
//!
//! Canonical contract families handled here:
//! - development diagnostic contracts
//! - diff and patch preview contracts
//! - test/build explanation contracts
//!
//! This module may inspect and explain development artifacts, but it must not
//! redefine subsystem truth or closure state by itself.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_shell_types::resolve_shell_path;
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

enum DevAgentCommand<'a> {
    HeadFile { path: &'a str, lines: usize },
    TailFile { path: &'a str, lines: usize },
    WcFile { path: &'a str },
    HexFile { path: &'a str, limit: usize },
    BuildDiagnostics { path: &'a str },
    DiagnosticFiles { path: &'a str },
    TestFailures { path: &'a str },
    ExplainTestFailures { path: &'a str },
    DiffFiles { left: &'a str, right: &'a str },
    PatchPreview { left: &'a str, right: &'a str },
    ExplainDiff { left: &'a str, right: &'a str },
    ImpactSummary { left: &'a str, right: &'a str },
    RollbackPreview { left: &'a str, right: &'a str },
}

impl<'a> DevAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if let Some(rest) = line.strip_prefix("head-file ") {
            return Some(
                parse_path_with_optional_count(rest, 10)
                    .map(|(path, lines)| Self::HeadFile { path, lines }),
            );
        }
        if let Some(rest) = line.strip_prefix("tail-file ") {
            return Some(
                parse_path_with_optional_count(rest, 10)
                    .map(|(path, lines)| Self::TailFile { path, lines }),
            );
        }
        if let Some(rest) = line.strip_prefix("wc-file ") {
            let path = rest.trim();
            return Some((!path.is_empty()).then_some(Self::WcFile { path }).ok_or(2));
        }
        if let Some(rest) = line.strip_prefix("hex-file ") {
            return Some(
                parse_path_with_optional_count(rest, 64)
                    .map(|(path, limit)| Self::HexFile { path, limit }),
            );
        }
        if let Some(rest) = line.strip_prefix("build-diagnostics ") {
            let path = rest.trim();
            return Some(
                (!path.is_empty())
                    .then_some(Self::BuildDiagnostics { path })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("diagnostic-files ") {
            let path = rest.trim();
            return Some(
                (!path.is_empty())
                    .then_some(Self::DiagnosticFiles { path })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("test-failures ") {
            let path = rest.trim();
            return Some(
                (!path.is_empty())
                    .then_some(Self::TestFailures { path })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("explain-test-failures ") {
            let path = rest.trim();
            return Some(
                (!path.is_empty())
                    .then_some(Self::ExplainTestFailures { path })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("diff-files ") {
            return Some(
                parse_path_pair(rest).map(|(left, right)| Self::DiffFiles { left, right }),
            );
        }
        if let Some(rest) = line.strip_prefix("patch-preview ") {
            return Some(
                parse_path_pair(rest).map(|(left, right)| Self::PatchPreview { left, right }),
            );
        }
        if let Some(rest) = line.strip_prefix("explain-diff ") {
            return Some(
                parse_path_pair(rest).map(|(left, right)| Self::ExplainDiff { left, right }),
            );
        }
        if let Some(rest) = line.strip_prefix("impact-summary ") {
            return Some(
                parse_path_pair(rest).map(|(left, right)| Self::ImpactSummary { left, right }),
            );
        }
        if let Some(rest) = line.strip_prefix("rollback-preview ") {
            return Some(
                parse_path_pair(rest).map(|(left, right)| Self::RollbackPreview { left, right }),
            );
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::HeadFile { path, lines } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let mut emitted = 0usize;
                for line in text.lines().take(lines) {
                    emitted += 1;
                    write_line(runtime, line)?;
                }
                write_line(
                    runtime,
                    &format!("head-summary path={resolved} lines={emitted}"),
                )
            }
            Self::TailFile { path, lines } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let all_lines = text.lines().collect::<Vec<_>>();
                let start = all_lines.len().saturating_sub(lines);
                let mut emitted = 0usize;
                for line in &all_lines[start..] {
                    emitted += 1;
                    write_line(runtime, line)?;
                }
                write_line(
                    runtime,
                    &format!("tail-summary path={resolved} lines={emitted}"),
                )
            }
            Self::WcFile { path } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let bytes = text.len();
                let chars = text.chars().count();
                let lines = text.lines().count();
                let words = text.split_whitespace().count();
                write_line(
                    runtime,
                    &format!(
                        "wc path={resolved} bytes={bytes} chars={chars} words={words} lines={lines}"
                    ),
                )
            }
            Self::HexFile { path, limit } => {
                let resolved = resolve_shell_path(cwd, path);
                let text = shell_read_file_text(runtime, &resolved)?;
                let bytes = text.as_bytes();
                let shown = bytes.len().min(limit);
                let mut offset = 0usize;
                while offset < shown {
                    let end = (offset + 16).min(shown);
                    let chunk = &bytes[offset..end];
                    let mut hex = String::new();
                    let mut ascii = String::new();
                    for byte in chunk {
                        hex.push_str(&format!("{byte:02x} "));
                        let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                            *byte as char
                        } else {
                            '.'
                        };
                        ascii.push(ch);
                    }
                    write_line(
                        runtime,
                        &format!("hex {offset:04x}: {:<48} {}", hex.trim_end(), ascii),
                    )?;
                    offset = end;
                }
                write_line(
                    runtime,
                    &format!(
                        "hex-summary path={resolved} shown={} total={}",
                        shown,
                        bytes.len()
                    ),
                )
            }
            Self::BuildDiagnostics { path } => {
                let resolved = resolve_shell_path(cwd, path);
                render_build_diagnostics(runtime, &resolved)
            }
            Self::DiagnosticFiles { path } => {
                let resolved = resolve_shell_path(cwd, path);
                render_diagnostic_files(runtime, &resolved)
            }
            Self::TestFailures { path } => {
                let resolved = resolve_shell_path(cwd, path);
                render_test_failures(runtime, &resolved)
            }
            Self::ExplainTestFailures { path } => {
                let resolved = resolve_shell_path(cwd, path);
                render_explain_test_failures(runtime, &resolved)
            }
            Self::DiffFiles { left, right } => {
                let resolved_left = resolve_shell_path(cwd, left);
                let resolved_right = resolve_shell_path(cwd, right);
                render_diff_files(runtime, &resolved_left, &resolved_right)
            }
            Self::PatchPreview { left, right } => {
                let resolved_left = resolve_shell_path(cwd, left);
                let resolved_right = resolve_shell_path(cwd, right);
                render_patch_preview(runtime, &resolved_left, &resolved_right)
            }
            Self::ExplainDiff { left, right } => {
                let resolved_left = resolve_shell_path(cwd, left);
                let resolved_right = resolve_shell_path(cwd, right);
                render_explain_diff(runtime, &resolved_left, &resolved_right)
            }
            Self::ImpactSummary { left, right } => {
                let resolved_left = resolve_shell_path(cwd, left);
                let resolved_right = resolve_shell_path(cwd, right);
                render_impact_summary(runtime, &resolved_left, &resolved_right)
            }
            Self::RollbackPreview { left, right } => {
                let resolved_left = resolve_shell_path(cwd, left);
                let resolved_right = resolve_shell_path(cwd, right);
                render_rollback_preview(runtime, &resolved_left, &resolved_right)
            }
        }
    }
}

struct BuildDiagnosticRecord {
    severity: &'static str,
    code: String,
    path: String,
    line: usize,
    column: usize,
    message: String,
}

struct DiagnosticFileSummary {
    path: String,
    diagnostics: usize,
    errors: usize,
    warnings: usize,
    notes: usize,
}

struct TestFailureRecord {
    name: String,
    path: String,
    line: usize,
    column: usize,
    reason: String,
}

struct FileDiffSummary {
    added: usize,
    removed: usize,
    changed: usize,
    unchanged: usize,
}

enum LineDiff<'a> {
    Unchanged,
    Added {
        right_line: usize,
        line: &'a str,
    },
    Removed {
        left_line: usize,
        line: &'a str,
    },
    Changed {
        left_line: usize,
        left: &'a str,
        right_line: usize,
        right: &'a str,
    },
}

fn render_build_diagnostics<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let diagnostics = parse_build_diagnostics(&text);
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut notes = 0usize;
    for diagnostic in &diagnostics {
        match diagnostic.severity {
            "error" => errors += 1,
            "warning" => warnings += 1,
            _ => notes += 1,
        }
        write_line(
            runtime,
            &format!(
                "build-diagnostic severity={} code={} path={} line={} column={} message={}",
                diagnostic.severity,
                if diagnostic.code.is_empty() {
                    "-"
                } else {
                    &diagnostic.code
                },
                diagnostic.path,
                diagnostic.line,
                diagnostic.column,
                diagnostic.message
            ),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "build-diagnostics-summary path={path} diagnostics={} errors={} warnings={} notes={}",
            diagnostics.len(),
            errors,
            warnings,
            notes
        ),
    )
}

fn render_test_failures<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let failures = parse_test_failures(&text);
    for failure in &failures {
        write_line(
            runtime,
            &format!(
                "test-failure name={} path={} line={} column={} reason={}",
                failure.name, failure.path, failure.line, failure.column, failure.reason
            ),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "test-failures-summary path={path} failures={}",
            failures.len()
        ),
    )
}

fn render_diagnostic_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let diagnostics = parse_build_diagnostics(&text);
    let mut files = Vec::<DiagnosticFileSummary>::new();
    for diagnostic in &diagnostics {
        let summary =
            if let Some(existing) = files.iter_mut().find(|entry| entry.path == diagnostic.path) {
                existing
            } else {
                files.push(DiagnosticFileSummary {
                    path: diagnostic.path.clone(),
                    diagnostics: 0,
                    errors: 0,
                    warnings: 0,
                    notes: 0,
                });
                files.last_mut().expect("just pushed")
            };
        summary.diagnostics += 1;
        match diagnostic.severity {
            "error" => summary.errors += 1,
            "warning" => summary.warnings += 1,
            _ => summary.notes += 1,
        }
    }
    for file in &files {
        write_line(
            runtime,
            &format!(
                "diagnostic-file path={} diagnostics={} errors={} warnings={} notes={}",
                file.path, file.diagnostics, file.errors, file.warnings, file.notes
            ),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "diagnostic-files-summary path={} files={} diagnostics={}",
            path,
            files.len(),
            diagnostics.len()
        ),
    )
}

fn render_explain_test_failures<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let failures = parse_test_failures(&text);
    if failures.is_empty() {
        return write_line(
            runtime,
            &format!(
                "explain-test-failures path={} failures=0 verdict=clean next=none",
                path
            ),
        );
    }
    let mut unique_files = Vec::<String>::new();
    for failure in &failures {
        if !unique_files.iter().any(|entry| entry == &failure.path) {
            unique_files.push(failure.path.clone());
        }
        write_line(
            runtime,
            &format!(
                "explain-test-failure name={} path={} line={} column={} kind={} hint={}",
                failure.name,
                failure.path,
                failure.line,
                failure.column,
                classify_test_failure_reason(&failure.reason),
                suggest_test_failure_hint(failure)
            ),
        )?;
    }
    let next = failures
        .first()
        .map(|failure| {
            format!(
                "inspect-{}:{}:{}",
                failure.path, failure.line, failure.column
            )
        })
        .unwrap_or_else(|| String::from("none"));
    write_line(
        runtime,
        &format!(
            "explain-test-failures path={} failures={} files={} verdict=red next={}",
            path,
            failures.len(),
            unique_files.len(),
            next
        ),
    )
}

fn parse_build_diagnostics(text: &str) -> Vec<BuildDiagnosticRecord> {
    let mut diagnostics = Vec::new();
    let lines = text.lines().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index].trim();
        let Some((severity, rest)) = parse_diagnostic_header(line) else {
            index += 1;
            continue;
        };
        let (code, message) = split_diagnostic_code(rest);
        let mut path = String::from("-");
        let mut line_no = 0usize;
        let mut column = 0usize;
        let mut lookahead = index + 1;
        while lookahead < lines.len() {
            let candidate = lines[lookahead].trim_start();
            if let Some(location) = candidate.strip_prefix("--> ") {
                if let Some((candidate_path, candidate_line, candidate_column)) =
                    parse_file_location(location)
                {
                    path = candidate_path;
                    line_no = candidate_line;
                    column = candidate_column;
                }
                break;
            }
            if parse_diagnostic_header(candidate.trim()).is_some() {
                break;
            }
            lookahead += 1;
        }
        diagnostics.push(BuildDiagnosticRecord {
            severity,
            code,
            path,
            line: line_no,
            column,
            message,
        });
        index = lookahead.max(index + 1);
    }
    diagnostics
}

fn parse_test_failures(text: &str) -> Vec<TestFailureRecord> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut failures = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index].trim();
        let Some(rest) = line
            .strip_prefix("---- ")
            .and_then(|rest| rest.strip_suffix(" ----"))
        else {
            index += 1;
            continue;
        };
        let name = rest
            .strip_suffix(" stdout")
            .unwrap_or(rest)
            .trim()
            .to_string();
        let mut path = String::from("-");
        let mut line_no = 0usize;
        let mut column = 0usize;
        let mut reason = String::from("-");
        let mut lookahead = index + 1;
        while lookahead < lines.len() {
            let candidate = lines[lookahead].trim();
            if candidate.starts_with("---- ") && candidate.ends_with(" ----") {
                break;
            }
            if let Some(rest) = candidate.strip_prefix("thread '") {
                if let Some((_, after_name)) = rest.split_once("' panicked at ") {
                    if let Some((candidate_path, candidate_line, candidate_column, panic_reason)) =
                        parse_panic_site_and_reason(after_name)
                    {
                        path = candidate_path;
                        line_no = candidate_line;
                        column = candidate_column;
                        if !panic_reason.is_empty() {
                            reason = panic_reason;
                        }
                    } else {
                        reason = after_name.trim().to_string();
                    }
                }
            } else if reason == "-" && !candidate.is_empty() && !candidate.ends_with(':') {
                reason = candidate.to_string();
            }
            lookahead += 1;
        }
        failures.push(TestFailureRecord {
            name,
            path,
            line: line_no,
            column,
            reason,
        });
        index = lookahead.max(index + 1);
    }
    failures
}

fn parse_diagnostic_header(line: &str) -> Option<(&'static str, &str)> {
    if let Some(rest) = line.strip_prefix("error") {
        return Some(("error", rest.trim_start_matches(':').trim()));
    }
    if let Some(rest) = line.strip_prefix("warning") {
        return Some(("warning", rest.trim_start_matches(':').trim()));
    }
    if let Some(rest) = line.strip_prefix("note") {
        return Some(("note", rest.trim_start_matches(':').trim()));
    }
    None
}

fn split_diagnostic_code(rest: &str) -> (String, String) {
    if let Some((code, message)) = rest.split_once(':') {
        let trimmed_code = code.trim();
        if trimmed_code.starts_with('[') && trimmed_code.ends_with(']') {
            return (
                trimmed_code
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .to_string(),
                message.trim().to_string(),
            );
        }
    }
    (String::new(), rest.trim().to_string())
}

fn parse_file_location(location: &str) -> Option<(String, usize, usize)> {
    let mut parts = location.trim().rsplitn(3, ':');
    let column = parts.next()?.parse::<usize>().ok()?;
    let line = parts.next()?.parse::<usize>().ok()?;
    let path = parts.next()?.trim().to_string();
    Some((path, line, column))
}

fn parse_panic_site_and_reason(text: &str) -> Option<(String, usize, usize, String)> {
    let mut parts = text.trim().splitn(4, ':');
    let path = parts.next()?.trim().to_string();
    let line = parts.next()?.trim().parse::<usize>().ok()?;
    let column = parts.next()?.trim().parse::<usize>().ok()?;
    let reason = parts.next()?.trim().to_string();
    Some((path, line, column, reason))
}

fn classify_test_failure_reason(reason: &str) -> &'static str {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("assertion failed") {
        "assertion"
    } else if lower.contains("called `option::unwrap()")
        || lower.contains("called `result::unwrap()")
    {
        "unwrap"
    } else if lower.contains("panicked at") {
        "panic"
    } else {
        "failure"
    }
}

fn suggest_test_failure_hint(failure: &TestFailureRecord) -> &'static str {
    match classify_test_failure_reason(&failure.reason) {
        "assertion" => "inspect-assertion-and-fixture",
        "unwrap" => "inspect-missing-value-path",
        "panic" => "inspect-panic-site",
        _ => "inspect-failure-site",
    }
}

fn render_diff_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    left: &str,
    right: &str,
) -> Result<(), ExitCode> {
    let left_text = shell_read_file_text(runtime, left)?;
    let right_text = shell_read_file_text(runtime, right)?;
    let left_lines = left_text.lines().collect::<Vec<_>>();
    let right_lines = right_text.lines().collect::<Vec<_>>();
    let diff = compute_line_diff(&left_lines, &right_lines);
    let summary = summarize_line_diff(&diff);
    for entry in diff {
        match entry {
            LineDiff::Changed {
                left_line,
                left,
                right_line,
                right,
            } => write_line(
                runtime,
                &format!(
                    "diff-change left-line={} right-line={} left={} right={}",
                    left_line,
                    right_line,
                    left.trim(),
                    right.trim()
                ),
            )?,
            LineDiff::Added { right_line, line } => write_line(
                runtime,
                &format!("diff-add right-line={} text={}", right_line, line.trim()),
            )?,
            LineDiff::Removed { left_line, line } => write_line(
                runtime,
                &format!("diff-remove left-line={} text={}", left_line, line.trim()),
            )?,
            LineDiff::Unchanged => {}
        }
    }
    write_line(
        runtime,
        &format!(
            "diff-files-summary left={} right={} changed={} added={} removed={} unchanged={}",
            left, right, summary.changed, summary.added, summary.removed, summary.unchanged
        ),
    )
}

fn render_patch_preview<B: SyscallBackend>(
    runtime: &Runtime<B>,
    left: &str,
    right: &str,
) -> Result<(), ExitCode> {
    let left_text = shell_read_file_text(runtime, left)?;
    let right_text = shell_read_file_text(runtime, right)?;
    let left_lines = left_text.lines().collect::<Vec<_>>();
    let right_lines = right_text.lines().collect::<Vec<_>>();
    let diff = compute_line_diff(&left_lines, &right_lines);
    write_line(
        runtime,
        &format!("patch-preview left={} right={}", left, right),
    )?;
    for entry in &diff {
        match entry {
            LineDiff::Changed {
                left_line,
                left,
                right_line,
                right,
            } => {
                write_line(
                    runtime,
                    &format!("@@ -{},1 +{},1 @@", left_line, right_line),
                )?;
                write_line(runtime, &format!("-{}", left))?;
                write_line(runtime, &format!("+{}", right))?;
            }
            LineDiff::Added { line, .. } => write_line(runtime, &format!("+{}", line))?,
            LineDiff::Removed { line, .. } => write_line(runtime, &format!("-{}", line))?,
            LineDiff::Unchanged => {}
        }
    }
    let summary = summarize_line_diff(&diff);
    write_line(
        runtime,
        &format!(
            "patch-preview-summary left={} right={} changed={} added={} removed={}",
            left, right, summary.changed, summary.added, summary.removed
        ),
    )
}

fn render_explain_diff<B: SyscallBackend>(
    runtime: &Runtime<B>,
    left: &str,
    right: &str,
) -> Result<(), ExitCode> {
    let left_text = shell_read_file_text(runtime, left)?;
    let right_text = shell_read_file_text(runtime, right)?;
    let left_lines = left_text.lines().collect::<Vec<_>>();
    let right_lines = right_text.lines().collect::<Vec<_>>();
    let diff = compute_line_diff(&left_lines, &right_lines);
    let summary = summarize_line_diff(&diff);
    let impact = if summary.changed != 0 {
        "behavior-edit"
    } else if summary.added != 0 && summary.removed == 0 {
        "growth"
    } else if summary.removed != 0 && summary.added == 0 {
        "shrink"
    } else if summary.added != 0 || summary.removed != 0 {
        "mixed-shape"
    } else {
        "no-op"
    };
    write_line(
        runtime,
        &format!(
            "explain-diff left={} right={} impact={} changed={} added={} removed={} unchanged={}",
            left, right, impact, summary.changed, summary.added, summary.removed, summary.unchanged
        ),
    )?;
    for entry in diff {
        match entry {
            LineDiff::Changed {
                left_line,
                right_line,
                ..
            } => write_line(
                runtime,
                &format!(
                    "explain-diff-change left-line={} right-line={} effect=replaced-line",
                    left_line, right_line
                ),
            )?,
            LineDiff::Added { right_line, .. } => write_line(
                runtime,
                &format!(
                    "explain-diff-add right-line={} effect=inserted-line",
                    right_line
                ),
            )?,
            LineDiff::Removed { left_line, .. } => write_line(
                runtime,
                &format!(
                    "explain-diff-remove left-line={} effect=deleted-line",
                    left_line
                ),
            )?,
            LineDiff::Unchanged => {}
        }
    }
    Ok(())
}

fn render_impact_summary<B: SyscallBackend>(
    runtime: &Runtime<B>,
    left: &str,
    right: &str,
) -> Result<(), ExitCode> {
    let left_text = shell_read_file_text(runtime, left)?;
    let right_text = shell_read_file_text(runtime, right)?;
    let left_lines = left_text.lines().collect::<Vec<_>>();
    let right_lines = right_text.lines().collect::<Vec<_>>();
    let diff = compute_line_diff(&left_lines, &right_lines);
    let summary = summarize_line_diff(&diff);
    let impact = if summary.changed != 0 {
        "behavior-edit"
    } else if summary.added != 0 && summary.removed == 0 {
        "growth"
    } else if summary.removed != 0 && summary.added == 0 {
        "shrink"
    } else if summary.added != 0 || summary.removed != 0 {
        "mixed-shape"
    } else {
        "no-op"
    };
    let touched = summary.changed + summary.added + summary.removed;
    let risk = if summary.changed != 0 && touched >= 3 {
        "high"
    } else if touched >= 2 {
        "medium"
    } else if touched == 1 {
        "low"
    } else {
        "none"
    };
    let review = match impact {
        "behavior-edit" => "review-replaced-lines-first",
        "growth" => "review-new-branches-and-callers",
        "shrink" => "review-removed-path-coverage",
        "mixed-shape" => "review-shape-and-behavior",
        _ => "no-review-needed",
    };
    let rollback = if touched == 0 {
        "none"
    } else {
        "rollback-preview"
    };
    write_line(
        runtime,
        &format!(
            "impact-summary left={} right={} impact={} risk={} touched={} changed={} added={} removed={} unchanged={} review={} rollback={}",
            left,
            right,
            impact,
            risk,
            touched,
            summary.changed,
            summary.added,
            summary.removed,
            summary.unchanged,
            review,
            rollback
        ),
    )
}

fn render_rollback_preview<B: SyscallBackend>(
    runtime: &Runtime<B>,
    left: &str,
    right: &str,
) -> Result<(), ExitCode> {
    let left_text = shell_read_file_text(runtime, left)?;
    let right_text = shell_read_file_text(runtime, right)?;
    let target_lines = right_text.lines().collect::<Vec<_>>();
    let source_lines = left_text.lines().collect::<Vec<_>>();
    let diff = compute_line_diff(&target_lines, &source_lines);
    write_line(
        runtime,
        &format!(
            "rollback-preview left={} right={} apply={}=>{}",
            left, right, right, left
        ),
    )?;
    for entry in &diff {
        match entry {
            LineDiff::Changed {
                left_line,
                left,
                right_line,
                right,
            } => {
                write_line(
                    runtime,
                    &format!("@@ -{},1 +{},1 @@", left_line, right_line),
                )?;
                write_line(runtime, &format!("-{}", left))?;
                write_line(runtime, &format!("+{}", right))?;
            }
            LineDiff::Added { line, .. } => write_line(runtime, &format!("+{}", line))?,
            LineDiff::Removed { line, .. } => write_line(runtime, &format!("-{}", line))?,
            LineDiff::Unchanged => {}
        }
    }
    let summary = summarize_line_diff(&diff);
    write_line(
        runtime,
        &format!(
            "rollback-preview-summary left={} right={} changed={} added={} removed={}",
            left, right, summary.changed, summary.added, summary.removed
        ),
    )
}

fn compute_line_diff<'a>(left: &'a [&'a str], right: &'a [&'a str]) -> Vec<LineDiff<'a>> {
    let mut diff = Vec::new();
    let max_len = left.len().max(right.len());
    for index in 0..max_len {
        match (left.get(index), right.get(index)) {
            (Some(left_line), Some(right_line)) if left_line == right_line => {
                diff.push(LineDiff::Unchanged);
            }
            (Some(left_line), Some(right_line)) => {
                diff.push(LineDiff::Changed {
                    left_line: index + 1,
                    left: left_line,
                    right_line: index + 1,
                    right: right_line,
                });
            }
            (Some(left_line), None) => diff.push(LineDiff::Removed {
                left_line: index + 1,
                line: left_line,
            }),
            (None, Some(right_line)) => diff.push(LineDiff::Added {
                right_line: index + 1,
                line: right_line,
            }),
            (None, None) => {}
        }
    }
    diff
}

fn summarize_line_diff(diff: &[LineDiff<'_>]) -> FileDiffSummary {
    let mut summary = FileDiffSummary {
        added: 0,
        removed: 0,
        changed: 0,
        unchanged: 0,
    };
    for entry in diff {
        match entry {
            LineDiff::Added { .. } => summary.added += 1,
            LineDiff::Removed { .. } => summary.removed += 1,
            LineDiff::Changed { .. } => summary.changed += 1,
            LineDiff::Unchanged => summary.unchanged += 1,
        }
    }
    summary
}

fn parse_path_with_optional_count<'a>(
    rest: &'a str,
    default_count: usize,
) -> Result<(&'a str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    if path.is_empty() {
        return Err(2);
    }
    let count = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => default_count,
    };
    Ok((path, count))
}

fn parse_path_pair(rest: &str) -> Result<(&str, &str), ExitCode> {
    let mut parts = rest.split_whitespace();
    let left = parts.next().ok_or(2)?;
    let right = parts.next().ok_or(2)?;
    if left.is_empty() || right.is_empty() {
        return Err(2);
    }
    Ok((left, right))
}

pub fn try_handle_dev_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match DevAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("head-file ") {
                "usage: head-file <path> [lines]"
            } else if line.starts_with("tail-file ") {
                "usage: tail-file <path> [lines]"
            } else if line.starts_with("wc-file ") {
                "usage: wc-file <path>"
            } else if line.starts_with("build-diagnostics ") {
                "usage: build-diagnostics <path>"
            } else if line.starts_with("diagnostic-files ") {
                "usage: diagnostic-files <path>"
            } else if line.starts_with("test-failures ") {
                "usage: test-failures <path>"
            } else if line.starts_with("explain-test-failures ") {
                "usage: explain-test-failures <path>"
            } else if line.starts_with("diff-files ") {
                "usage: diff-files <left> <right>"
            } else if line.starts_with("patch-preview ") {
                "usage: patch-preview <left> <right>"
            } else if line.starts_with("explain-diff ") {
                "usage: explain-diff <left> <right>"
            } else {
                "usage: hex-file <path> [bytes]"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
