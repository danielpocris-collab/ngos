use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, execute_input_translation, parse_input_translation_args};

pub fn handle_game_input_translate<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let args = parse_input_translation_args(runtime, rest)?;
    execute_input_translation(runtime, current_cwd, args, game_sessions)
}
