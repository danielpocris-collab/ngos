use alloc::format;

use crate::GameCompatSession;

pub fn render_session_shim_line(session: &GameCompatSession) -> alloc::string::String {
    format!(
        "game.session.shim prefix={} saves={} cache={} env-file={} argv-file={} channel-file={} loader-file={} abi-file={}",
        session.prefix_path,
        session.saves_path,
        session.cache_path,
        session.runtime_env_path,
        session.runtime_argv_path,
        session.runtime_channel_path,
        session.runtime_loader_path,
        session.runtime_abi_path,
    )
}
