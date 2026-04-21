use alloc::format;
use alloc::string::String;

use ngos_shell_types::resolve_shell_path;

pub fn game_simulation_key(target: &str) -> &str {
    let without_manifest = target.strip_suffix(".manifest").unwrap_or(target);
    without_manifest
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(without_manifest)
}

pub fn game_simulation_manifest_path(current_cwd: &str, target: &str) -> String {
    if target.ends_with(".manifest") {
        return resolve_shell_path(current_cwd, target);
    }
    let without_trailing = target.trim_end_matches('/');
    let with_manifest = format!("{without_trailing}.manifest");
    resolve_shell_path(current_cwd, &with_manifest)
}
