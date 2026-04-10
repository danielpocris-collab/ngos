use ngos_game_compat_runtime::GameCompatManifest;
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn game_manifest_load<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<GameCompatManifest, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    GameCompatManifest::parse(&text).map_err(|_| 283)
}
