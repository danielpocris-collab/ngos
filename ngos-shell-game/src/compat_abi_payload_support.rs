use alloc::vec::Vec;

use ngos_shell_vfs::shell_read_file_bytes;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;

pub fn game_compat_abi_payload<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<Vec<u8>, ExitCode> {
    shell_read_file_bytes(runtime, &session.runtime_abi_path)
}
