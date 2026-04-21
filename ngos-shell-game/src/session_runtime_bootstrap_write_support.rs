use ngos_game_compat_runtime::{GameCompatManifest, GameSessionPlan};
use ngos_shell_vfs::shell_write_all;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::session_runtime_bootstrap_text_support::runtime_bootstrap_texts;
use crate::session_runtime_dir_support::game_ensure_dir_tree;
use crate::session_runtime_path_support::runtime_bootstrap_paths;

pub fn game_write_runtime_bootstrap<B: SyscallBackend>(
    runtime: &Runtime<B>,
    plan: &GameSessionPlan,
    manifest: &GameCompatManifest,
) -> Result<
    (
        alloc::string::String,
        alloc::string::String,
        alloc::string::String,
        alloc::string::String,
        alloc::string::String,
    ),
    ExitCode,
> {
    if manifest.shims.prefix != "/" {
        game_ensure_dir_tree(runtime, &manifest.shims.prefix)?;
    }
    if manifest.shims.saves != "/" {
        game_ensure_dir_tree(runtime, &manifest.shims.saves)?;
    }
    if manifest.shims.cache != "/" {
        game_ensure_dir_tree(runtime, &manifest.shims.cache)?;
    }
    let (env_path, argv_path, channel_path, loader_path, abi_path) =
        runtime_bootstrap_paths(manifest);
    let (env_text, argv_text, loader_text, abi_text) =
        runtime_bootstrap_texts(plan, manifest, &channel_path);
    let _ = runtime.mkfile_path(&env_path);
    let _ = runtime.mkfile_path(&argv_path);
    let _ = runtime.mkfile_path(&loader_path);
    let _ = runtime.mkfile_path(&abi_path);
    let _ = runtime.mkchan_path(&channel_path);
    let env_fd = runtime.open_path(&env_path).map_err(|_| 237)?;
    shell_write_all(runtime, env_fd, env_text.as_bytes())?;
    runtime.close(env_fd).map_err(|_| 240)?;
    let argv_fd = runtime.open_path(&argv_path).map_err(|_| 237)?;
    shell_write_all(runtime, argv_fd, argv_text.as_bytes())?;
    runtime.close(argv_fd).map_err(|_| 240)?;
    let loader_fd = runtime.open_path(&loader_path).map_err(|_| 237)?;
    shell_write_all(runtime, loader_fd, loader_text.as_bytes())?;
    runtime.close(loader_fd).map_err(|_| 240)?;
    let abi_fd = runtime.open_path(&abi_path).map_err(|_| 237)?;
    shell_write_all(runtime, abi_fd, abi_text.as_bytes())?;
    runtime.close(abi_fd).map_err(|_| 240)?;
    Ok((env_path, argv_path, channel_path, loader_path, abi_path))
}
