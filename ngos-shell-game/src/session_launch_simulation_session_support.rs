use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, game_launch_session, game_manifest_load, game_simulation_key,
    game_simulation_manifest_path,
};

pub fn ensure_simulation_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    target: &str,
) -> Result<usize, ExitCode> {
    let session_key = game_simulation_key(target);
    let manifest_path = game_simulation_manifest_path(current_cwd, target);
    if let Some(idx) = game_sessions.iter().position(|s| s.slug == session_key) {
        return Ok(idx);
    }
    let manifest = game_manifest_load(runtime, &manifest_path)?;
    let session = game_launch_session(runtime, &mut String::from(current_cwd), &manifest)?;
    game_sessions.push(*session);
    Ok(game_sessions.len() - 1)
}
