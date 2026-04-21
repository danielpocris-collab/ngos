use alloc::format;
use alloc::string::{String, ToString};

use ngos_game_compat_runtime::compat_target_name;

use crate::GameCompatSession;

pub fn render_session_identity_line(session: &GameCompatSession) -> String {
    format!(
        "game.session pid={} title={} slug={} target={} domain={} process={} cwd={} exec={} gfx-api={} gfx-backend={} gfx-translation={} stopped={} exit={}",
        session.pid,
        session.title,
        session.slug,
        compat_target_name(session.target),
        session.domain_id,
        session.process_name,
        session.working_dir,
        session.executable_path,
        session.graphics_translation.source_api_name,
        session.graphics_translation.backend_name,
        session.graphics_translation.translation,
        session.stopped,
        session
            .exit_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| String::from("-"))
    )
}
