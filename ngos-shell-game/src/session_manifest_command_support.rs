use ngos_shell_types::resolve_shell_path;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{game_manifest_load, game_render_manifest, game_render_plan};

pub fn handle_game_manifest<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    path: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(current_cwd, path);
    let manifest = game_manifest_load(runtime, &resolved)?;
    game_render_manifest(runtime, &resolved, &manifest)
}

pub fn handle_game_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    path: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(current_cwd, path);
    let manifest = game_manifest_load(runtime, &resolved)?;
    let plan = manifest.session_plan();
    game_render_plan(runtime, &plan)
}
