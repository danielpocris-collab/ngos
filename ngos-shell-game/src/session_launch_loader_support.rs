use alloc::format;
use alloc::vec::Vec;

use ngos_game_compat_runtime::{GameCompatManifest, dll_override_mode_name};

pub fn build_loader_dll_overrides(manifest: &GameCompatManifest) -> Vec<alloc::string::String> {
    manifest
        .dll_overrides
        .iter()
        .map(|rule| format!("{}={}", rule.library, dll_override_mode_name(rule.mode)))
        .collect::<Vec<_>>()
}

pub fn build_loader_env_overrides(manifest: &GameCompatManifest) -> Vec<alloc::string::String> {
    manifest
        .env_overrides
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
}
