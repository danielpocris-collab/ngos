use alloc::format;

use ngos_game_compat_runtime::{
    GameCompatManifest, audio_backend_name, compat_target_name, graphics_api_name,
    graphics_backend_name, graphics_translation_name, input_backend_name,
};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::write_line;

pub fn game_render_manifest<B: SyscallBackend>(
    runtime: &Runtime<B>,
    manifest_path: &str,
    manifest: &GameCompatManifest,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.manifest path={} target={} title={} slug={} exec={} cwd={} argv={}",
            manifest_path,
            compat_target_name(manifest.target),
            manifest.title,
            manifest.slug,
            manifest.executable_path,
            manifest.working_dir,
            manifest.argv.join(" ")
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.gfx backend={} profile={} api={} translation={}",
            graphics_backend_name(manifest.graphics.backend),
            manifest.graphics.profile,
            graphics_api_name(manifest.graphics.source_api),
            graphics_translation_name(manifest.graphics.source_api, manifest.graphics.backend)
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.audio backend={} profile={}",
            audio_backend_name(manifest.audio.backend),
            manifest.audio.profile
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.input backend={} profile={}",
            input_backend_name(manifest.input.backend),
            manifest.input.profile
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.shim prefix={} saves={} cache={}",
            manifest.shims.prefix, manifest.shims.saves, manifest.shims.cache
        ),
    )
}
