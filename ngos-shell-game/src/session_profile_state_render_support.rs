use alloc::format;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn render_game_session_profile_state<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    let pid = session.pid;
    if session.stopped {
        let exit_code = session.exit_code.unwrap_or(-1);
        write_line(
            runtime,
            &format!(
                "game.session.profile pid={pid} slug={} state=stopped exit-code={exit_code}",
                session.slug
            ),
        )
    } else {
        write_line(
            runtime,
            &format!(
                "game.session.profile pid={pid} slug={} state=running",
                session.slug
            ),
        )
    }
}
