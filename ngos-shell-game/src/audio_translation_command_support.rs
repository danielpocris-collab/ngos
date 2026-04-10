use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, execute_audio_translation, parse_audio_translation_args};

pub fn handle_game_audio_translate<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let args = parse_audio_translation_args(runtime, rest)?;
    execute_audio_translation(runtime, current_cwd, args, game_sessions)
}
