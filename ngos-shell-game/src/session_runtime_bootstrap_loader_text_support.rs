use alloc::format;
use alloc::vec::Vec;

use ngos_game_compat_runtime::{
    GameCompatManifest, compat_target_name, graphics_api_name, graphics_backend_name,
    graphics_translation_name,
};

use crate::PROGRAM_NAME;

pub fn runtime_bootstrap_loader_text(manifest: &GameCompatManifest) -> alloc::string::String {
    let loader_routing = manifest.loader_routing_plan();
    [
        format!("route-class={}", loader_routing.route_class),
        format!("launch-mode={}", loader_routing.launch_mode),
        format!("entry-profile={}", loader_routing.entry_profile),
        format!("bootstrap-profile={}", loader_routing.bootstrap_profile),
        format!("entrypoint={}", loader_routing.entrypoint),
        format!(
            "requires-compat-shims={}",
            if loader_routing.requires_compat_shims {
                "1"
            } else {
                "0"
            }
        ),
        format!("target={}", compat_target_name(manifest.target)),
        format!(
            "gfx-api={}",
            graphics_api_name(manifest.graphics.source_api)
        ),
        format!(
            "gfx-backend={}",
            graphics_backend_name(manifest.graphics.backend)
        ),
        format!(
            "gfx-translation={}",
            graphics_translation_name(manifest.graphics.source_api, manifest.graphics.backend)
        ),
        format!("preloads={}", manifest.shim_preloads.join(";")),
        format!(
            "dll-overrides={}",
            manifest
                .dll_overrides
                .iter()
                .map(|rule| format!(
                    "{}={}",
                    rule.library,
                    ngos_game_compat_runtime::dll_override_mode_name(rule.mode)
                ))
                .collect::<Vec<_>>()
                .join(";")
        ),
        format!(
            "env-overrides={}",
            manifest
                .env_overrides
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(";")
        ),
        format!("producer={PROGRAM_NAME}"),
    ]
    .join("\n")
}
