use ngos_user_abi::SyscallBackend;
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;

pub fn game_cleanup_session_artifacts<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) {
    for path in [
        session.runtime_env_path.as_str(),
        session.runtime_argv_path.as_str(),
        session.runtime_channel_path.as_str(),
        session.runtime_loader_path.as_str(),
        session.runtime_abi_path.as_str(),
    ] {
        let _ = runtime.unlink_path(path);
    }
}
