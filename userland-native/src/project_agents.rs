use super::*;

const PROJECT_AGENT_DEFAULT_DEPTH: usize = 3;
const PROJECT_AGENT_MAX_DEPTH: usize = 12;

enum ProjectAgentCommand<'a> {
    WorkspaceSummary,
    WorkspaceMembers,
    WorkspaceTopology,
    CrateInfo {
        name: &'a str,
    },
    CrateFiles {
        name: &'a str,
        depth: usize,
    },
    CrateDeps {
        name: &'a str,
    },
    DocsList,
    DocShow {
        name: &'a str,
        start: usize,
        count: usize,
    },
    DocSearch {
        needle: &'a str,
    },
    ManifestShow {
        name: &'a str,
    },
    RustFiles {
        path: &'a str,
        depth: usize,
    },
}

impl<'a> ProjectAgentCommand<'a> {
    fn parse(line: &'a str) -> Option<Result<Self, ExitCode>> {
        if line == "workspace-summary" {
            return Some(Ok(Self::WorkspaceSummary));
        }
        if line == "workspace-members" {
            return Some(Ok(Self::WorkspaceMembers));
        }
        if line == "workspace-topology" {
            return Some(Ok(Self::WorkspaceTopology));
        }
        if let Some(rest) = line.strip_prefix("crate-info ") {
            let name = rest.trim();
            return Some(
                (!name.is_empty())
                    .then_some(Self::CrateInfo { name })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("crate-files ") {
            return Some(parse_crate_files(rest));
        }
        if let Some(rest) = line.strip_prefix("crate-deps ") {
            let name = rest.trim();
            return Some(
                (!name.is_empty())
                    .then_some(Self::CrateDeps { name })
                    .ok_or(2),
            );
        }
        if line == "docs-list" {
            return Some(Ok(Self::DocsList));
        }
        if let Some(rest) = line.strip_prefix("doc-show ") {
            return Some(parse_doc_show(rest));
        }
        if let Some(rest) = line.strip_prefix("doc-search ") {
            let needle = rest.trim();
            return Some(
                (!needle.is_empty())
                    .then_some(Self::DocSearch { needle })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("manifest-show ") {
            let name = rest.trim();
            return Some(
                (!name.is_empty())
                    .then_some(Self::ManifestShow { name })
                    .ok_or(2),
            );
        }
        if let Some(rest) = line.strip_prefix("rust-files ") {
            return Some(parse_rust_files(rest));
        }
        None
    }

    fn execute<B: SyscallBackend>(&self, runtime: &Runtime<B>, cwd: &str) -> Result<(), ExitCode> {
        match *self {
            Self::WorkspaceSummary => render_workspace_summary(runtime),
            Self::WorkspaceMembers => render_workspace_members(runtime),
            Self::WorkspaceTopology => render_workspace_topology(runtime),
            Self::CrateInfo { name } => render_crate_info(runtime, name),
            Self::CrateFiles { name, depth } => render_crate_files(runtime, name, depth),
            Self::CrateDeps { name } => render_crate_deps(runtime, name),
            Self::DocsList => render_docs_list(runtime),
            Self::DocShow { name, start, count } => render_doc_show(runtime, name, start, count),
            Self::DocSearch { needle } => render_doc_search(runtime, needle),
            Self::ManifestShow { name } => render_manifest_show(runtime, name),
            Self::RustFiles { path, depth } => render_rust_files(runtime, cwd, path, depth),
        }
    }
}

fn parse_crate_files(rest: &str) -> Result<ProjectAgentCommand<'_>, ExitCode> {
    let mut parts = rest.split_whitespace();
    let name = parts.next().ok_or(2)?;
    if name.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => PROJECT_AGENT_DEFAULT_DEPTH,
    };
    Ok(ProjectAgentCommand::CrateFiles {
        name,
        depth: depth.min(PROJECT_AGENT_MAX_DEPTH),
    })
}

fn parse_rust_files(rest: &str) -> Result<ProjectAgentCommand<'_>, ExitCode> {
    let mut parts = rest.split_whitespace();
    let path = parts.next().ok_or(2)?;
    if path.is_empty() {
        return Err(2);
    }
    let depth = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => PROJECT_AGENT_DEFAULT_DEPTH,
    };
    Ok(ProjectAgentCommand::RustFiles {
        path,
        depth: depth.min(PROJECT_AGENT_MAX_DEPTH),
    })
}

fn parse_doc_show(rest: &str) -> Result<ProjectAgentCommand<'_>, ExitCode> {
    let mut parts = rest.split_whitespace();
    let name = parts.next().ok_or(2)?;
    if name.is_empty() {
        return Err(2);
    }
    let start = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => 1,
    };
    let count = match parts.next() {
        Some(raw) => raw.parse::<usize>().map_err(|_| 2)?,
        None => 32,
    };
    Ok(ProjectAgentCommand::DocShow { name, start, count })
}

fn render_workspace_summary<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let members = parse_workspace_members(&cargo);
    let metadata = parse_workspace_metadata(&cargo);
    write_line(
        runtime,
        &format!(
            "workspace-summary name={} product={} codename={} members={} edition={} version={}",
            metadata.workspace_name,
            metadata.product_name,
            metadata.codename,
            members.len(),
            metadata.edition,
            metadata.version
        ),
    )
}

fn render_workspace_members<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let members = parse_workspace_members(&cargo);
    for member in &members {
        write_line(runtime, &format!("workspace-member path={member}"))?;
    }
    write_line(
        runtime,
        &format!("workspace-members-summary count={}", members.len()),
    )
}

fn render_workspace_topology<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let members = parse_workspace_members(&cargo);
    for member in &members {
        let manifest_path = format!("/{member}/Cargo.toml");
        let manifest = shell_read_file_text(runtime, &manifest_path)?;
        let package = parse_package_field(&manifest, "name").unwrap_or_else(|| member.clone());
        let deps = parse_dependency_names(&manifest);
        let src_root = format!("/{member}/src");
        let (rust_files, directories) = count_rust_files(runtime, &src_root, 3)?;
        write_line(
            runtime,
            &format!(
                "workspace-crate path=/{member} package={} rust-files={} directories={} deps={}",
                package,
                rust_files,
                directories,
                deps.len()
            ),
        )?;
        for dep in deps {
            write_line(
                runtime,
                &format!("workspace-edge crate={} dep={}", package, dep),
            )?;
        }
    }
    write_line(
        runtime,
        &format!("workspace-topology-summary crates={}", members.len()),
    )
}

fn render_crate_info<B: SyscallBackend>(runtime: &Runtime<B>, name: &str) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let members = parse_workspace_members(&cargo);
    let Some(member) = members.into_iter().find(|member| member.ends_with(name)) else {
        write_line(runtime, &format!("crate-info-miss name={name}"))?;
        return Err(249);
    };
    let manifest_path = format!("/{member}/Cargo.toml");
    let manifest = shell_read_file_text(runtime, &manifest_path)?;
    let package_name = parse_package_field(&manifest, "name").unwrap_or_else(|| String::from("?"));
    let edition =
        parse_package_field(&manifest, "edition").unwrap_or_else(|| String::from("workspace"));
    let version =
        parse_package_field(&manifest, "version").unwrap_or_else(|| String::from("workspace"));
    let src_root = format!("/{member}/src");
    let (rust_files, directories) = count_rust_files(runtime, &src_root, 3)?;
    write_line(
        runtime,
        &format!(
            "crate-info path=/{member} package={} edition={} version={} rust-files={} directories={}",
            package_name, edition, version, rust_files, directories
        ),
    )
}

fn render_crate_deps<B: SyscallBackend>(runtime: &Runtime<B>, name: &str) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let Some(member) = parse_workspace_members(&cargo)
        .into_iter()
        .find(|member| member.ends_with(name))
    else {
        write_line(runtime, &format!("crate-deps-miss name={name}"))?;
        return Err(249);
    };
    let manifest_path = format!("/{member}/Cargo.toml");
    let manifest = shell_read_file_text(runtime, &manifest_path)?;
    let package = parse_package_field(&manifest, "name").unwrap_or_else(|| member.clone());
    let deps = parse_dependency_names(&manifest);
    for dep in &deps {
        write_line(runtime, &format!("crate-dep crate={} dep={}", package, dep))?;
    }
    write_line(
        runtime,
        &format!("crate-deps-summary crate={} count={}", package, deps.len()),
    )
}

fn render_docs_list<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let entries = list_project_entries(runtime, "/docs")?;
    for entry in &entries {
        write_line(runtime, &format!("doc path=/docs/{entry}"))?;
    }
    write_line(
        runtime,
        &format!("docs-list-summary count={}", entries.len()),
    )
}

fn render_doc_search<B: SyscallBackend>(
    runtime: &Runtime<B>,
    needle: &str,
) -> Result<(), ExitCode> {
    let entries = list_project_entries(runtime, "/docs")?;
    let mut matches = 0usize;
    for entry in entries {
        let path = format!("/docs/{entry}");
        let text = shell_read_file_text(runtime, &path)?;
        for (index, line) in text.lines().enumerate() {
            if line.contains(needle) {
                matches += 1;
                write_line(
                    runtime,
                    &format!("doc-match {}:{} {}", path, index + 1, line),
                )?;
            }
        }
    }
    write_line(
        runtime,
        &format!("doc-search-summary needle={} matches={}", needle, matches),
    )
}

fn render_doc_show<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    start: usize,
    count: usize,
) -> Result<(), ExitCode> {
    let path = format!("/docs/{name}");
    let text = shell_read_file_text(runtime, &path)?;
    let lines = text.lines().collect::<Vec<_>>();
    let start_index = start.saturating_sub(1).min(lines.len());
    let end_index = start_index.saturating_add(count).min(lines.len());
    for (index, line) in lines[start_index..end_index].iter().enumerate() {
        write_line(
            runtime,
            &format!("{:>5}: {}", start_index + index + 1, line),
        )?;
    }
    write_line(
        runtime,
        &format!(
            "doc-show-summary path={path} start={} shown={}",
            start_index + 1,
            end_index.saturating_sub(start_index)
        ),
    )
}

fn render_manifest_show<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let Some(member) = parse_workspace_members(&cargo)
        .into_iter()
        .find(|member| member.ends_with(name))
    else {
        write_line(runtime, &format!("manifest-show-miss name={name}"))?;
        return Err(249);
    };
    let path = format!("/{member}/Cargo.toml");
    let manifest = shell_read_file_text(runtime, &path)?;
    for line in manifest.lines() {
        write_line(runtime, line)?;
    }
    write_line(runtime, &format!("manifest-show-summary path={path}"))
}

fn render_rust_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    path: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let root = resolve_shell_path(cwd, path);
    let mut visited = 0usize;
    let mut matched = 0usize;
    walk_rust_files(runtime, &root, depth, 0, &mut visited, &mut matched)?;
    write_line(
        runtime,
        &format!(
            "rust-files-summary path={root} depth={depth} visited={visited} matches={matched}"
        ),
    )
}

fn render_crate_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    name: &str,
    depth: usize,
) -> Result<(), ExitCode> {
    let cargo = shell_read_file_text(runtime, "/Cargo.toml")?;
    let Some(member) = parse_workspace_members(&cargo)
        .into_iter()
        .find(|member| member.ends_with(name))
    else {
        write_line(runtime, &format!("crate-files-miss name={name}"))?;
        return Err(249);
    };
    let root = format!("/{member}/src");
    let mut visited = 0usize;
    let mut matched = 0usize;
    walk_rust_files(runtime, &root, depth, 0, &mut visited, &mut matched)?;
    write_line(
        runtime,
        &format!(
            "crate-files-summary crate={name} path={root} depth={depth} visited={visited} matches={matched}"
        ),
    )
}

fn walk_rust_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    max_depth: usize,
    depth: usize,
    visited: &mut usize,
    matched: &mut usize,
) -> Result<(), ExitCode> {
    let status = runtime.stat_path(path).map_err(|_| 231)?;
    *visited += 1;
    match NativeObjectKind::from_raw(status.kind) {
        Some(NativeObjectKind::Directory) if depth < max_depth => {
            for entry in list_project_entries(runtime, path)? {
                let child = join_project_path(path, &entry);
                walk_rust_files(runtime, &child, max_depth, depth + 1, visited, matched)?;
            }
        }
        Some(NativeObjectKind::File) if path.ends_with(".rs") => {
            *matched += 1;
            write_line(
                runtime,
                &format!("rust-file path={path} size={}", status.size),
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn count_rust_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    depth: usize,
) -> Result<(usize, usize), ExitCode> {
    let mut rust_files = 0usize;
    let mut directories = 0usize;
    count_rust_files_inner(runtime, path, depth, 0, &mut rust_files, &mut directories)?;
    Ok((rust_files, directories))
}

fn count_rust_files_inner<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    max_depth: usize,
    depth: usize,
    rust_files: &mut usize,
    directories: &mut usize,
) -> Result<(), ExitCode> {
    let status = runtime.stat_path(path).map_err(|_| 231)?;
    match NativeObjectKind::from_raw(status.kind) {
        Some(NativeObjectKind::Directory) => {
            *directories += 1;
            if depth >= max_depth {
                return Ok(());
            }
            for entry in list_project_entries(runtime, path)? {
                let child = join_project_path(path, &entry);
                count_rust_files_inner(
                    runtime,
                    &child,
                    max_depth,
                    depth + 1,
                    rust_files,
                    directories,
                )?;
            }
        }
        Some(NativeObjectKind::File) if path.ends_with(".rs") => {
            *rust_files += 1;
        }
        _ => {}
    }
    Ok(())
}

fn list_project_entries<B: SyscallBackend>(
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

fn join_project_path(base: &str, leaf: &str) -> String {
    if base == "/" {
        format!("/{leaf}")
    } else {
        format!("{base}/{leaf}")
    }
}

struct WorkspaceMetadata {
    product_name: String,
    codename: String,
    workspace_name: String,
    edition: String,
    version: String,
}

fn parse_workspace_members(cargo: &str) -> Vec<String> {
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

fn parse_workspace_metadata(cargo: &str) -> WorkspaceMetadata {
    WorkspaceMetadata {
        product_name: parse_metadata_field(cargo, "product_name")
            .unwrap_or_else(|| String::from("?")),
        codename: parse_metadata_field(cargo, "codename").unwrap_or_else(|| String::from("?")),
        workspace_name: parse_metadata_field(cargo, "workspace_name")
            .unwrap_or_else(|| String::from("?")),
        edition: parse_section_field(cargo, "[workspace.package]", "edition")
            .unwrap_or_else(|| String::from("?")),
        version: parse_section_field(cargo, "[workspace.package]", "version")
            .unwrap_or_else(|| String::from("?")),
    }
}

fn parse_metadata_field(cargo: &str, name: &str) -> Option<String> {
    parse_section_field(cargo, "[workspace.metadata.ngos]", name)
}

fn parse_package_field(manifest: &str, name: &str) -> Option<String> {
    parse_section_field(manifest, "[package]", name)
}

fn parse_dependency_names(manifest: &str) -> Vec<String> {
    let mut deps = Vec::new();
    collect_section_dependency_names(manifest, "[dependencies]", &mut deps);
    collect_section_dependency_names(manifest, "[dev-dependencies]", &mut deps);
    collect_section_dependency_names(manifest, "[build-dependencies]", &mut deps);
    deps
}

fn collect_section_dependency_names(manifest: &str, section: &str, deps: &mut Vec<String>) {
    let Some(start) = manifest.find(section) else {
        return;
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

fn parse_section_field(text: &str, section: &str, field: &str) -> Option<String> {
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

pub(super) fn try_handle_project_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    cwd: &str,
    line: &str,
) -> Option<Result<(), ExitCode>> {
    let command = match ProjectAgentCommand::parse(line)? {
        Ok(command) => command,
        Err(code) => {
            let usage = if line.starts_with("crate-info ") {
                "usage: crate-info <name>"
            } else if line.starts_with("crate-files ") {
                "usage: crate-files <name> [depth]"
            } else if line.starts_with("crate-deps ") {
                "usage: crate-deps <name>"
            } else if line.starts_with("doc-show ") {
                "usage: doc-show <name> [start] [count]"
            } else if line.starts_with("doc-search ") {
                "usage: doc-search <needle>"
            } else if line.starts_with("manifest-show ") {
                "usage: manifest-show <name>"
            } else if line.starts_with("rust-files ") {
                "usage: rust-files <path> [depth]"
            } else {
                "usage: workspace-summary|workspace-members|workspace-topology|docs-list"
            };
            let _ = write_line(runtime, usage);
            return Some(Err(code));
        }
    };
    Some(command.execute(runtime, cwd))
}
