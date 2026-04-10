use alloc::string::String;

use ngos_user_abi::{ExitCode, NativeObjectKind, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn game_ensure_dir_tree<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let normalized = ngos_shell_types::normalize_shell_path(path);
    if normalized == "/" {
        return Ok(());
    }
    let mut current = String::new();
    for segment in normalized.trim_start_matches('/').split('/') {
        current.push('/');
        current.push_str(segment);
        if runtime.mkdir_path(&current).is_err() {
            let status = runtime.stat_path(&current).map_err(|_| 241)?;
            if status.kind != NativeObjectKind::Directory as u32 {
                return Err(241);
            }
        }
    }
    Ok(())
}
