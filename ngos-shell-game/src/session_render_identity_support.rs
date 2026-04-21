use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, render_session_abi_line, render_session_identity_line,
    render_session_loader_line, render_session_shim_line, write_line,
};

pub fn render_session_identity<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    write_line(runtime, &render_session_identity_line(session))?;
    write_line(runtime, &render_session_shim_line(session))?;
    write_line(runtime, &render_session_loader_line(session))?;
    write_line(runtime, &render_session_abi_line(session))
}
