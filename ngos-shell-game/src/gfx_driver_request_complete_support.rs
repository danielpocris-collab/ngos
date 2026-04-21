use alloc::format;

use ngos_shell_vfs::shell_write_all;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn complete_graphics_driver_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let encoded = format!("request:{request_id}\n{payload}");
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encoded.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)
}
