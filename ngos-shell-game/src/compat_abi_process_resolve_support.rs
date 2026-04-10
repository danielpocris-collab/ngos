use alloc::vec::Vec;

use ngos_game_compat_runtime::compat_abi_process_image_matches;
use ngos_shell_proc::{list_process_ids, read_process_text};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn game_compat_resolve_process_pid<B: SyscallBackend>(
    runtime: &Runtime<B>,
    expected_name: &str,
    expected_image: &str,
    expected_cwd: Option<&str>,
) -> Result<u64, ExitCode> {
    let mut matches = Vec::new();
    for pid in list_process_ids(runtime)? {
        let name = read_process_text(runtime, pid, Runtime::get_process_name).map_err(|_| 464)?;
        let image =
            read_process_text(runtime, pid, Runtime::get_process_image_path).map_err(|_| 464)?;
        let cwd = if expected_cwd.is_some() {
            Some(read_process_text(runtime, pid, Runtime::get_process_cwd).map_err(|_| 464)?)
        } else {
            None
        };
        if name == expected_name
            && compat_abi_process_image_matches(&image, expected_image)
            && expected_cwd.is_none_or(|value| cwd.as_deref() == Some(value))
        {
            matches.push(pid);
        }
    }
    matches.into_iter().max().ok_or(464)
}
