use alloc::format;

use ngos_input_translate::{InputScript, InputTranslator};
use ngos_shell_types::resolve_shell_path;
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, InputTranslationArgs, find_game_session, find_game_session_mut,
    game_submit_input, write_line,
};

pub fn execute_input_translation<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    args: InputTranslationArgs<'_>,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let session = find_game_session(runtime, game_sessions, args.pid)?;
    if session.stopped {
        let _ = write_line(
            runtime,
            &format!(
                "game.input.translate.refused pid={} api={} reason=session-stopped",
                args.pid, args.api_str
            ),
        );
        return Err(295);
    }
    let resolved = resolve_shell_path(current_cwd, args.path_str.trim());
    let input_text = shell_read_file_text(runtime, &resolved)?;
    let script = InputScript::parse(&input_text).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "game.input.translate.refused pid={} api={} reason={}",
                args.pid,
                args.api_str,
                e.describe()
            ),
        );
        297i32
    })?;
    let translator = InputTranslator::new(args.source_api);
    let encoded = translator.translate(&script).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "game.input.translate.refused pid={} api={} reason={}",
                args.pid,
                args.api_str,
                e.describe()
            ),
        );
        297i32
    })?;
    let session = find_game_session_mut(runtime, game_sessions, args.pid)?;
    let (_, delivery_observed) = game_submit_input(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.input.translate pid={} api={} translation={} frame={} ops={} submitted={} delivery={} delivery-observed={delivery_observed}",
            args.pid,
            args.api_str,
            args.source_api.translation_label(),
            encoded.frame_tag,
            encoded.op_count,
            session.submitted_input_batches,
            encoded.delivery
        ),
    )
}
