use alloc::format;

use ngos_shell_vfs::shell_write_all;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn game_publish_runtime_payload<B: SyscallBackend>(
    runtime: &Runtime<B>,
    channel_path: &str,
    kind: &str,
    tag: &str,
    payload: &[u8],
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(channel_path).map_err(|_| 237)?;
    let mut envelope = format!("kind={kind} tag={tag}\n").into_bytes();
    envelope.extend_from_slice(payload);
    let result = shell_write_all(runtime, fd, &envelope);
    let _ = runtime.close(fd);
    result
}
