use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;
use crate::session_profile_loader_render_support::render_game_session_profile_loader;
use crate::session_profile_runtime_render_support::render_game_session_profile_runtime;
use crate::session_profile_state_render_support::render_game_session_profile_state;

pub fn render_game_session_profile<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    render_game_session_profile_state(runtime, session)?;
    render_game_session_profile_runtime(runtime, session)?;
    render_game_session_profile_loader(runtime, session)
}
