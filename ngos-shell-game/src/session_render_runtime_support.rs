use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;
use crate::session_render_identity_support::render_session_identity;
use crate::session_render_lane_support::render_session_lanes;
use crate::session_render_media_support::render_session_media;

pub fn game_render_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    render_session_identity(runtime, session)?;
    render_session_media(runtime, session)?;
    render_session_lanes(runtime, session)
}
