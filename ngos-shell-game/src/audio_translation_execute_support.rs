use alloc::format;

use ngos_audio_translate::{AudioTranslator, MixScript};
use ngos_shell_types::resolve_shell_path;
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    AudioTranslationArgs, GameCompatSession, find_game_session, find_game_session_mut,
    game_submit_mix, write_line,
};

pub fn execute_audio_translation<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    args: AudioTranslationArgs<'_>,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let session = find_game_session(runtime, game_sessions, args.pid)?;
    if session.stopped {
        let _ = write_line(
            runtime,
            &format!(
                "game.audio.translate.refused pid={} api={} reason=session-stopped",
                args.pid, args.api_str
            ),
        );
        return Err(295);
    }
    let resolved = resolve_shell_path(current_cwd, args.path_str.trim());
    let mix_text = shell_read_file_text(runtime, &resolved)?;
    let script = MixScript::parse(&mix_text).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "game.audio.translate.refused pid={} api={} reason={}",
                args.pid,
                args.api_str,
                e.describe()
            ),
        );
        296i32
    })?;
    let translator = AudioTranslator::new(args.source_api);
    let encoded = translator.translate(&script).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "game.audio.translate.refused pid={} api={} reason={}",
                args.pid,
                args.api_str,
                e.describe()
            ),
        );
        296i32
    })?;
    let session = find_game_session_mut(runtime, game_sessions, args.pid)?;
    let (_, completion_observed) = game_submit_mix(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.audio.translate pid={} api={} translation={} stream={} ops={} submitted={} completion={} completion-observed={completion_observed}",
            args.pid,
            args.api_str,
            args.source_api.translation_label(),
            encoded.stream_tag,
            encoded.op_count,
            session.submitted_audio_batches,
            encoded.completion
        ),
    )
}
