use super::*;

const ANALYSIS_AGENT_DEFAULT_DEPTH: usize = 4;
const ANALYSIS_AGENT_MAX_DEPTH: usize = 12;

enum AnalysisAgentCommand<'a> {
    WorkspaceAudit,
    CrateAudit { name: &'a str },
    SourceHotspots { path: &'a str, depth: usize },
    CrateHotspots { name: &'a str, depth: usize },
    DocLinks { name: &'a str },
}

impl<'a> AnalysisAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if line == "workspace-audit" {
            return Some(Ok(Self::WorkspaceAudit));
        }
        if let Some(rest) = line.strip_prefix("crate-audit ") {
            let name = rest.trim();
            return Some(
                (!name.is_empty())
                    .then_some(Self::CrateAudit { name })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("source-hotspots ") {
            return Some(
                parse_path_depth(rest).map(|(path, depth)| Self::SourceHotspots { path, depth }),
            );
        }
        if let Some(rest) = line.strip_prefix("crate-hotspots ") {
            return Some(
                parse_name_depth(rest).map(|(name, depth)| Self::CrateHotspots { name, depth }),
            );
        }
        if let Some(rest) = line.strip_prefix("doc-links ") {
            let name = rest.trim();
            return Some(
                (!name.is_empty())
                    .then_some(Self::DocLinks { name })
                    .ok_or(2),
            );
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::WorkspaceAudit => render_workspace_audit(runtime),
            Self::CrateAudit { name } => render_crate_audit(runtime, name),
            Self::SourceHotspots { path, depth } => {
                render_source_hotspots(runtime, cwd, path, depth)
            }
            Self::CrateHotspots { name, depth } => render_crate_hotspots(runtime, name, depth),
            Self::DocLinks { name } => render_doc_links(runtime, name),
        }
    }
}

fn parse_path_depth(rest: &str) -> Result<(&str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    if path.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => ANALYSIS_AGENT_DEFAULT_DEPTH,
    };
    Ok((path, depth.min(ANALYSIS_AGENT_MAX_DEPTH)))
}

fn parse_name_depth(rest: &str) -> Result<(&str, usize), ExitCode> {
    let mut parts = rest.split_whitespace();
    let name = parts.next().ok_or(2)?;
    if name.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => ANALYSIS_AGENT_DEFAULT_DEPTH,
    };
    Ok((name, depth.min(ANALYSIS_AGENT_MAX_DEPTH)))
}

fn render_workspace_audit<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let members = parse_analysis_workspace_members(&cargo);
    let mut total_rust_files = 0usize;
    let mut total_lines = 0usize;
    let mut total_unsafe = 0usize;
    let mut total_todo = 0usize;
    for member in &members {
        let src_root = format!("/{member}/src");
        let stats = analyze_rust_tree(runtime, &src_root, 4)?;
        total_rust_files += stats.files;
        total_lines += stats.lines;
        total_unsafe += stats.unsafe_hits;
        total_todo += stats.todo_hits;
        write_line(
            runtime,
            &format!(
                "workspace-audit-crate path=/{member} rust-files={} lines={} unsafe={} todo={}",
                stats.files, stats.lines, stats.unsafe_hits, stats.todo_hits
            ),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "workspace-audit-summary crates={} rust-files={} lines={} unsafe={} todo={}",
            members.len(),
            total_rust_files,
            total_lines,
            total_unsafe,
            total_todo
        ),
    )
}

fn render_crate_audit<B: SyscallBackend>(runtime: &Runtime<B>, name: &str) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let Some(member) = parse_analysis_workspace_members(&cargo)
        .into_iter()
        .find(|member| member.ends_with(name))
    else {
        write_line(runtime, &format!("crate-audit-miss name={name}"))?;
        return Err(249);
    };
    let manifest_path = format!("/{member}/Cargo.toml");
    let manifest = shell_read_file_text(runtime, &manifest_path)?;
    let package = parse_analysis_section_field(&manifest, "[package]", "name")
        .unwrap_or_else(|| member.clone());
    let edition = parse_analysis_section_field(&manifest, "[package]", "edition")
        .unwrap_or_else(|| String::from("workspace"));
    let version = parse_analysis_section_field(&manifest, "[package]", "version")
        .unwrap_or_else(|| String::from("workspace"));
    let deps = parse_analysis_dependency_names(&manifest);
    let src_root = format!("/{member}/src");
    let stats = analyze_rust_tree(runtime, &src_root, 6)?;
    write_line(
        runtime,
        &format!(
            "crate-audit path=/{member} package={} edition={} version={} deps={} rust-files={} lines={} unsafe={} todo={} symbols={}",
            package,
            edition,
            version,
            deps.len(),
            stats.files,
            stats.lines,
            stats.unsafe_hits,
            stats.todo_hits,
            stats.symbols
        ),
    )?;
    for dep in deps {
        write_line(
            runtime,
            &format!("crate-audit-dep crate={} dep={}", package, dep),
        )?;
    }
    Ok(())
}

fn render_source_hotspots<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let hotspots = collect_hotspots(runtime, &root, depth)?;
    for hotspot in hotspots.iter().take(12) {
        write_line(
            runtime,
            &format!(
                "hotspot path={} lines={} nonempty={} unsafe={} todo={}",
                hotspot.path,
                hotspot.lines,
                hotspot.nonempty,
                hotspot.unsafe_hits,
                hotspot.todo_hits
            ),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "source-hotspots-summary path={root} entries={}",
            hotspots.len()
        ),
    )
}

fn render_crate_hotspots<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let Some(member) = parse_analysis_workspace_members(&cargo)
        .into_iter()
        .find(|member| member.ends_with(name))
    else {
        write_line(runtime, &format!("crate-hotspots-miss name={name}"))?;
        return Err(249);
    };
    let root = format!("/{member}/src");
    let hotspots = collect_hotspots(runtime, &root, depth)?;
    for hotspot in hotspots.iter().take(12) {
        write_line(
            runtime,
            &format!(
                "crate-hotspot crate={} path={} lines={} nonempty={} unsafe={} todo={}",
                name,
                hotspot.path,
                hotspot.lines,
                hotspot.nonempty,
                hotspot.unsafe_hits,
                hotspot.todo_hits
            ),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "crate-hotspots-summary crate={} path={} entries={}",
            name,
            root,
            hotspots.len()
        ),
    )
}

fn render_doc_links<B: SyscallBackend>(runtime: &Runtime<B>, name: &str) -> Result<(), ExitCode> {
    let path = format!("/docs/{name}");
    let text = shell_read_file_text(runtime, &path)?;
    let mut matches = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line.contains(".md") || line.contains(".rs") || line.contains("Cargo.toml") {
            matches += 1;
            write_line(
                runtime,
                &format!("doc-link {}:{} {}", path, index + 1, line.trim()),
            )?;
        }
    }
    write_line(
        runtime,
        &format!("doc-links-summary path={} matches={}", path, matches),
    )
}

#[derive(Default)]
struct RustTreeStats {
    files: usize,
    lines: usize,
    unsafe_hits: usize,
    todo_hits: usize,
    symbols: usize,
}

struct HotspotEntry {
    path: String,
    lines: usize,
    nonempty: usize,
    unsafe_hits: usize,
    todo_hits: usize,
}

fn analyze_rust_tree<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    depth: usize,
) -> Result<RustTreeStats, ExitCode> {
    let mut stats = RustTreeStats::default();
    walk_analysis_tree(runtime, path, depth, 0, &mut |runtime, path| {
        let text = shell_read_file_text(runtime, path)?;
        stats.files += 1;
        for line in text.lines() {
            stats.lines += 1;
            if line.contains("unsafe ") {
                stats.unsafe_hits += 1;
            }
            if line.contains("TODO") || line.contains("FIXME") || line.contains("XXX") {
                stats.todo_hits += 1;
            }
            let trimmed = line.trim_start();
            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("mod ")
                || trimmed.starts_with("pub mod ")
            {
                stats.symbols += 1;
            }
        }
        Ok(())
    })?;
    Ok(stats)
}

fn collect_hotspots<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    depth: usize,
) -> Result<Vec<HotspotEntry>, ExitCode> {
    let mut hotspots = Vec::new();
    walk_analysis_tree(runtime, path, depth, 0, &mut |runtime, path| {
        let text = shell_read_file_text(runtime, path)?;
        let mut entry = HotspotEntry {
            path: path.to_string(),
            lines: 0,
            nonempty: 0,
            unsafe_hits: 0,
            todo_hits: 0,
        };
        for line in text.lines() {
            entry.lines += 1;
            if !line.trim().is_empty() {
                entry.nonempty += 1;
            }
            if line.contains("unsafe ") {
                entry.unsafe_hits += 1;
            }
            if line.contains("TODO") || line.contains("FIXME") || line.contains("XXX") {
                entry.todo_hits += 1;
            }
        }
        hotspots.push(entry);
        Ok(())
    })?;
    hotspots.sort_by(|left, right| {
        right
            .lines
            .cmp(&left.lines)
            .then(right.nonempty.cmp(&left.nonempty))
            .then(left.path.cmp(&right.path))
    });
    Ok(hotspots)
}

fn walk_analysis_tree<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    path: &str,
    max_depth: usize,
    depth: usize,
    on_file: &mut F,
) -> Result<(), ExitCode>
where
    F: FnMut(&Runtime<B>, &str) -> Result<(), ExitCode>,
{
    let status = runtime.stat_path(path).map_err(|_| 231)?;
    match NativeObjectKind::from_raw(status.kind) {
        Some(NativeObjectKind::Directory) if depth < max_depth => {
            for entry in list_analysis_entries(runtime, path)? {
                let child = join_analysis_path(path, &entry);
                walk_analysis_tree(runtime, &child, max_depth, depth + 1, on_file)?;
            }
        }
        Some(NativeObjectKind::File) if path.ends_with(".rs") => on_file(runtime, path)?,
        _ => {}
    }
    Ok(())
}

fn list_analysis_entries<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<String>, ExitCode> {
    let mut buffer = vec![0u8; 512];
    loop {
        let count = runtime.list_path(path, &mut buffer).map_err(|_| 246)?;
        if count < buffer.len() {
            let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 247)?;
            return Ok(text
                .lines()
                .map(str::trim)
                .filter(|entry| !entry.is_empty() && *entry != "." && *entry != "..")
                .map(ToString::to_string)
                .collect());
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

fn join_analysis_path(base: &str, leaf: &str) -> String {
    if base == "/" {
        format!("/{leaf}")
    } else {
        format!("{base}/{leaf}")
    }
}

fn parse_analysis_workspace_members(cargo: &str) -> Vec<String> {
    let Some(start) = cargo.find("members = [") else {
        return Vec::new();
    };
    let rest = &cargo[start + "members = [".len()..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('"') && line.ends_with("\","))
        .map(|line| line.trim_matches(',').trim_matches('"').to_string())
        .collect()
}

fn parse_analysis_section_field(text: &str, section: &str, field: &str) -> Option<String> {
    let start = text.find(section)?;
    let rest = &text[start + section.len()..];
    for line in rest.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            break;
        }
        if let Some(value) = trimmed.strip_prefix(&format!("{field} = ")) {
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

fn parse_analysis_dependency_names(manifest: &str) -> Vec<String> {
    let mut deps = Vec::new();
    for section in [
        "[dependencies]",
        "[dev-dependencies]",
        "[build-dependencies]",
    ] {
        let Some(start) = manifest.find(section) else {
            continue;
        };
        let rest = &manifest[start + section.len()..];
        for line in rest.lines().skip(1) {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                break;
            }
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(eq) = trimmed.find('=') {
                let name = trimmed[..eq].trim();
                if !name.is_empty() && !deps.iter().any(|existing| existing == name) {
                    deps.push(name.to_string());
                }
            }
        }
    }
    deps
}

pub(super) fn try_handle_analysis_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match AnalysisAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("crate-audit ") {
                "usage: crate-audit <name>"
            } else if line.starts_with("source-hotspots ") {
                "usage: source-hotspots <path> [depth]"
            } else if line.starts_with("crate-hotspots ") {
                "usage: crate-hotspots <name> [depth]"
            } else if line.starts_with("doc-links ") {
                "usage: doc-links <name>"
            } else {
                "usage: workspace-audit"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
