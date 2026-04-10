//! Shell path normalization and resolution.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Normalize a path by resolving `.` and `..` segments.
pub fn normalize_shell_path(path: &str) -> String {
    let mut parts = Vec::<&str>::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            let _ = parts.pop();
            continue;
        }
        parts.push(segment);
    }
    if parts.is_empty() {
        return String::from("/");
    }
    format!("/{}", parts.join("/"))
}

/// Resolve a path relative to a current working directory.
pub fn resolve_shell_path(cwd: &str, path: &str) -> String {
    if path.is_empty() {
        return normalize_shell_path(cwd);
    }
    if path.starts_with('/') {
        return normalize_shell_path(path);
    }
    if cwd == "/" {
        return normalize_shell_path(&format!("/{}", path));
    }
    normalize_shell_path(&format!("{cwd}/{path}"))
}
