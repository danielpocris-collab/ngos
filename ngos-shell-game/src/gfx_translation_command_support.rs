use alloc::format;

use ngos_gfx_translate::{ForeignFrameScript, GfxTranslator};
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, find_game_session_mut, game_submit_frame, graphics_api_name,
    parse_game_pid_script_args, summarize_graphics_deep_ops, write_line,
};

pub fn handle_game_gfx_translate<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-gfx-translate <pid> <foreign-cmd-file>",
    )?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    let source_api = match session.graphics_source_api.to_source_api() {
        Some(api) => api,
        None => {
            write_line(
                runtime,
                &format!(
                    "game.gfx.translate.refused pid={} api={} reason=unsupported-api",
                    pid,
                    graphics_api_name(session.graphics_source_api)
                ),
            )?;
            return Err(294);
        }
    };
    let text = shell_read_file_text(runtime, &resolved)?;
    let foreign = ForeignFrameScript::parse_for_api(Some(source_api), &text).map_err(|_| 291)?;
    let translator = GfxTranslator::new(source_api);
    let script = translator.translate(&foreign).map_err(|_| 291)?;
    let encoded = script.encode_translated(
        &session.graphics_profile,
        source_api.name(),
        source_api.translation_label(),
    );
    let (presented, completion_observed) = game_submit_frame(runtime, session, &encoded)?;
    let deep_ops = summarize_graphics_deep_ops(&encoded.payload);
    write_line(
        runtime,
        &format!(
            "game.gfx.translate pid={} frame={} ops={} bytes={} deep-ops={} api={} translation={} submitted={} presented={} present-ok={} queue={} present-mode={} completion={} completion-observed={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            deep_ops,
            source_api.name(),
            source_api.translation_label(),
            session.submitted_frames,
            session.presented_frames,
            presented,
            encoded.queue,
            encoded.present_mode,
            encoded.completion,
            completion_observed
        ),
    )
}
