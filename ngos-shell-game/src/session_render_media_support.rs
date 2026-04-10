use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;
use crate::session_render_audio_support::render_session_audio;
use crate::session_render_gfx_support::render_session_gfx;
use crate::session_render_input_support::render_session_input;

pub fn render_session_media<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    render_session_gfx(runtime, session)?;
    render_session_audio(runtime, session)?;
    render_session_input(runtime, session)
}
